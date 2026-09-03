from pathlib import Path

p = Path('src/storage.rs')
s = p.read_text()

s = s.replace('    path::{Path, PathBuf},\n    thread,\n', '    path::{Path, PathBuf},\n    sync::atomic::{AtomicBool, Ordering},\n    thread,\n')
s = s.replace('const STORAGE_LOCK_RETRY_DELAY: Duration = Duration::from_millis(20);\n', 'const STORAGE_LOCK_RETRY_DELAY: Duration = Duration::from_millis(20);\nstatic STORAGE_SESSION_LOCKED: AtomicBool = AtomicBool::new(false);\n')
s = s.replace('    pub fn encryption_state() -> EncryptionState {\n        let encrypted_path = Self::data_path(true);', '    pub fn encryption_state() -> EncryptionState {\n        if STORAGE_SESSION_LOCKED.load(Ordering::Acquire) {\n            return EncryptionState::Error;\n        }\n\n        let encrypted_path = Self::data_path(true);', 1)

old = '''    pub fn load_or_default(_config: &Config) -> Self {
        if let Ok(contents) = fs::read_to_string(Self::data_path(true)) {
            return Self::load_encrypted(&contents).unwrap_or_default();
        }

        fs::read_to_string(Self::data_path(false))
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }'''
new = '''    pub fn load_or_default(_config: &Config) -> Self {
        let encrypted_path = Self::data_path(true);
        if encrypted_path.exists() {
            return match fs::read_to_string(&encrypted_path).and_then(|contents| Self::load_encrypted(&contents)) {
                Ok(store) => {
                    STORAGE_SESSION_LOCKED.store(false, Ordering::Release);
                    store
                }
                Err(_) => {
                    STORAGE_SESSION_LOCKED.store(true, Ordering::Release);
                    Self::default()
                }
            };
        }

        fs::read_to_string(Self::data_path(false))
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }'''
assert old in s
s = s.replace(old, new, 1)

s = s.replace('    pub fn save(&self, config: &Config) -> io::Result<()> {\n        let path = Self::data_path(true);', '    pub fn save(&self, config: &Config) -> io::Result<()> {\n        if STORAGE_SESSION_LOCKED.load(Ordering::Acquire) {\n            return Err(io::Error::new(\n                io::ErrorKind::PermissionDenied,\n                "encrypted history is locked for this session; restart or reset after fixing keyring access",\n            ));\n        }\n\n        let path = Self::data_path(true);', 1)
s = s.replace('        let store = Self::default();\n        rotate_history_key()?;\n        store.save_unlocked(config)?;\n        Ok(store)', '        let store = Self::default();\n        rotate_history_key()?;\n        store.save_unlocked(config)?;\n        STORAGE_SESSION_LOCKED.store(false, Ordering::Release);\n        Ok(store)', 1)

old = '''    pub fn delete_plain_history_file() -> io::Result<()> {
        create_private_dir(&storage_base_dir())?;
        let _lock = acquire_storage_lock()?;

        match fs::remove_file(Self::data_path(false)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }'''
new = '''    pub fn delete_plain_history_file() -> io::Result<()> {
        create_private_dir(&storage_base_dir())?;
        let _lock = acquire_storage_lock()?;

        let plain_path = Self::data_path(false);
        if !plain_path.exists() {
            return Ok(());
        }

        let encrypted_path = Self::data_path(true);
        let encrypted_contents = fs::read_to_string(&encrypted_path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("refusing to delete plaintext history before encrypted history is readable: {error}"),
            )
        })?;
        Self::load_encrypted(&encrypted_contents)?;

        match fs::remove_file(plain_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }'''
assert old in s
s = s.replace(old, new, 1)

old = '''        let key = get_or_create_history_key()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key[..]));
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "failed to decrypt history"))?;

        serde_json::from_slice(&plaintext)'''
new = '''        let key = get_or_create_history_key()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key[..]));
        let plaintext = match cipher.decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref()) {
            Ok(plaintext) => plaintext,
            Err(_) => {
                let legacy_key = read_local_history_key().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "failed to decrypt history")
                })?;
                let legacy_cipher = ChaCha20Poly1305::new(Key::from_slice(&legacy_key[..]));
                let plaintext = legacy_cipher
                    .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "failed to decrypt history"))?;
                persist_keyring_key(&legacy_key)?;
                remove_local_history_key()?;
                plaintext
            }
        };

        serde_json::from_slice(&plaintext)'''
assert old in s
s = s.replace(old, new, 1)

start = s.index('fn get_or_create_history_key()')
end = s.index('fn decode_history_key', start)
replacement = r'''fn get_or_create_history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    match read_keyring_secret() {
        Ok(secret) => decode_history_key(&secret),
        Err(keyring::Error::NoEntry) => migrate_legacy_or_create_history_key(),
        Err(error) => Err(io::Error::other(error)),
    }
}

fn migrate_legacy_or_create_history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    match read_local_history_key() {
        Ok(key) => {
            persist_keyring_key(&key)?;
            remove_local_history_key()?;
            Ok(key)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if ClipboardStore::data_path(true).exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "encrypted history exists but its key is missing from the OS keyring",
                ));
            }
            rotate_history_key()
        }
        Err(error) => Err(error),
    }
}

fn rotate_history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0u8; 32]);
    OsRng.fill_bytes(key.as_mut());
    persist_keyring_key(&key)?;
    remove_local_history_key()?;
    Ok(key)
}

fn persist_keyring_key(key: &[u8; 32]) -> io::Result<()> {
    write_keyring_secret(key).map_err(io::Error::other)?;
    let round_trip = read_keyring_secret().map_err(io::Error::other)?;
    let verified = decode_history_key(&round_trip)?;
    if verified.as_ref() != key {
        return Err(io::Error::other("OS keyring verification failed"));
    }
    Ok(())
}

fn read_local_history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    let secret = fs::read_to_string(local_key_path())?;
    decode_history_key(secret.trim())
}

fn remove_local_history_key() -> io::Result<()> {
    match fs::remove_file(local_key_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn read_keyring_secret() -> Result<String, keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    entry.get_password()
}

fn write_keyring_secret(key: &[u8; 32]) -> Result<(), keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    entry.set_password(&B64.encode(key))
}

'''
s = s[:start] + replacement + s[end:]
p.write_text(s)

cargo = Path('Cargo.toml')
cargo.write_text(cargo.read_text().replace('version = "0.1.0"', 'version = "0.2.1"', 1))

sec = Path('SECURITY.md')
t = sec.read_text().replace('- The encryption key is generated randomly and stored in the OS keyring.', '- The encryption key is generated randomly and stored only in the OS keyring. Legacy `history.key` files are migrated into the keyring and removed after a verified round trip.')
t = t.replace('- The OS keyring must be available; if it is unavailable, encrypted history load/save will fail rather than silently falling back to plaintext.', '- The OS keyring must be available; if it is unavailable, encrypted history load/save fails closed rather than silently falling back to plaintext or a local key file. A failed encrypted-history load locks persistence for that applet session until restart or an explicit encrypted-history reset.')
sec.write_text(t)

readme = Path('README.md')
readme.write_text(readme.read_text().replace('- Local `0600` encryption key with best-effort OS keyring mirror', '- OS keyring-only encryption key storage with automatic migration from legacy `history.key` files'))

changelog = Path('CHANGELOG.md')
t = changelog.read_text()
entry = '''## v0.2.1 - Keyring-only encryption hardening\n\n- Store the ChaCha20Poly1305 history key only in the OS keyring.\n- Migrate legacy `history.key` material into the keyring and remove the local key only after a verified keyring round trip.\n- Fail closed when Secret Service/keyring access is unavailable or locked.\n- Never generate a replacement key when encrypted history already exists but the key is missing.\n- Lock persistence for the current applet session after encrypted-history load failure to prevent accidental overwrite after transient keyring recovery.\n- Refuse to delete plaintext history until the encrypted history can be read successfully.\n\n'''
if '## v0.2.1 - Keyring-only encryption hardening' not in t:
    changelog.write_text(entry + t)
