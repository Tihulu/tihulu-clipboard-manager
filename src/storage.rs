// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config::Config;
use crate::model::{ClipboardEntry, ClipboardPayload, encode_image_payload};
use crate::sensitive::looks_sensitive;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const KEYRING_SERVICE: &str = "io.github.tihulu.ClipboardManager";
const KEYRING_USER: &str = "history-encryption-key-v1";
const LOCAL_KEY_FILE: &str = "history.key";
const ENCRYPTED_FORMAT_VERSION: u8 = 1;
const ALLOWED_IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];
const STORAGE_LOCK_DIR: &str = "history.lock";
const KEY_LOCK_DIR: &str = "history-key.lock";
const STORAGE_LOCK_RETRIES: usize = 50;
const STORAGE_LOCK_RETRY_DELAY: Duration = Duration::from_millis(20);
static STORAGE_SESSION_LOCKED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EncryptionState {
    Ready,
    Encrypted,
    Plaintext,
    Error,
}

impl EncryptionState {
    pub fn is_secure(self) -> bool {
        matches!(self, Self::Ready | Self::Encrypted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardStore {
    entries: Vec<ClipboardEntry>,
    next_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedStoreFile {
    version: u8,
    algorithm: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

struct StorageLock {
    path: PathBuf,
}

impl Drop for StorageLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl Default for ClipboardStore {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }
}

impl ClipboardStore {
    pub fn encryption_state() -> EncryptionState {
        if STORAGE_SESSION_LOCKED.load(Ordering::Acquire) {
            return EncryptionState::Error;
        }

        let encrypted_path = Self::data_path(true);
        let plain_path = Self::data_path(false);

        if encrypted_path.exists() {
            return match fs::read_to_string(&encrypted_path) {
                Ok(contents) => match Self::load_encrypted(&contents) {
                    Ok(_) => EncryptionState::Encrypted,
                    Err(_) => EncryptionState::Error,
                },
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    if plain_path.exists() {
                        EncryptionState::Plaintext
                    } else if get_or_create_history_key().is_ok() {
                        EncryptionState::Ready
                    } else {
                        EncryptionState::Error
                    }
                }
                Err(_) => EncryptionState::Error,
            };
        }

        if plain_path.exists() {
            EncryptionState::Plaintext
        } else if get_or_create_history_key().is_ok() {
            EncryptionState::Ready
        } else {
            EncryptionState::Error
        }
    }

    pub fn load_or_default(_config: &Config) -> Self {
        let encrypted_path = Self::data_path(true);
        if encrypted_path.exists() {
            return match fs::read_to_string(&encrypted_path)
                .and_then(|contents| Self::load_encrypted(&contents))
            {
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
    }

    pub fn save(&self, config: &Config) -> io::Result<()> {
        if STORAGE_SESSION_LOCKED.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "encrypted history is locked for this session; restart or reset after fixing keyring access",
            ));
        }

        let path = Self::data_path(true);
        if let Some(parent) = path.parent() {
            create_private_dir(parent)?;
        }

        let _lock = acquire_storage_lock()?;
        self.save_unlocked(config)
    }

    pub fn reset_encrypted_history(config: &Config) -> io::Result<Self> {
        create_private_dir(&storage_base_dir())?;
        let _lock = acquire_storage_lock()?;

        for path in [Self::data_path(false), Self::data_path(true)] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }

        let store = Self::default();
        let _key_lock = acquire_key_lock()?;
        rotate_history_key()?;
        store.save_unlocked(config)?;
        STORAGE_SESSION_LOCKED.store(false, Ordering::Release);
        Ok(store)
    }

    pub fn delete_persisted_files() -> io::Result<()> {
        create_private_dir(&storage_base_dir())?;
        let _lock = acquire_storage_lock()?;

        for path in [Self::data_path(false), Self::data_path(true)] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn delete_plain_history_file() -> io::Result<()> {
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
    }

    pub fn entries(&self) -> &[ClipboardEntry] {
        &self.entries
    }

    pub fn add_text(&mut self, text: impl Into<String>, config: &Config) -> AddContentResult {
        let config = config.clone().hardened();
        if config.private_mode {
            return AddContentResult::SkippedPrivateMode;
        }

        if config.effective_max_text_bytes() == 0 {
            return AddContentResult::SkippedTooLarge;
        }

        let text = text.into();
        if text.trim().is_empty() {
            return AddContentResult::SkippedEmpty;
        }

        if text.len() > config.effective_max_text_bytes() {
            return AddContentResult::SkippedTooLarge;
        }

        if config.sensitive_filter && looks_sensitive(&text) {
            return AddContentResult::SkippedSensitive;
        }

        if self.entries.iter().any(
            |entry| matches!(&entry.payload, ClipboardPayload::Text(existing) if existing == &text),
        ) {
            return AddContentResult::SkippedDuplicate;
        }

        let entry = ClipboardEntry {
            id: self.next_id,
            payload: ClipboardPayload::Text(text),
            pinned: false,
            created_at_unix: unix_now(),
        };
        self.next_id += 1;
        self.entries.insert(0, entry);
        self.prune(&config);
        AddContentResult::Added
    }

    pub fn add_image(
        &mut self,
        mime: impl Into<String>,
        bytes: &[u8],
        config: &Config,
    ) -> AddContentResult {
        let config = config.clone().hardened();
        if config.private_mode {
            return AddContentResult::SkippedPrivateMode;
        }

        if !config.effective_image_clipboard() {
            return AddContentResult::SkippedUnsupportedMime;
        }

        let mime = mime.into();
        if !is_allowed_image_mime(&mime) {
            return AddContentResult::SkippedUnsupportedMime;
        }

        if bytes.is_empty() {
            return AddContentResult::SkippedEmpty;
        }

        if config.effective_limit_image_size() && config.effective_max_image_bytes() == 0 {
            return AddContentResult::SkippedTooLarge;
        }

        if config.effective_limit_image_size() && bytes.len() > config.effective_max_image_bytes() {
            return AddContentResult::SkippedTooLarge;
        }

        if self.entries.iter().any(|entry| {
            matches!(&entry.payload, ClipboardPayload::Image { mime: existing_mime, size_bytes, .. } if existing_mime == &mime && *size_bytes == bytes.len())
                && entry.image().is_some_and(|(_, existing_bytes)| existing_bytes == bytes)
        }) {
            return AddContentResult::SkippedDuplicate;
        }

        let entry = ClipboardEntry {
            id: self.next_id,
            payload: encode_image_payload(mime, bytes),
            pinned: false,
            created_at_unix: unix_now(),
        };
        self.next_id += 1;
        self.entries.insert(0, entry);
        self.prune(&config);
        AddContentResult::Added
    }

    pub fn prune(&mut self, config: &Config) {
        let config = config.clone().hardened();
        self.prune_to_max_age(config.effective_max_age_days());
        self.prune_to_max_entries(config.effective_max_entries());
    }

    pub fn prune_to_max_age(&mut self, max_age_days: u64) {
        if max_age_days == 0 {
            return;
        }

        let cutoff = unix_now().saturating_sub(max_age_days.saturating_mul(24 * 60 * 60));
        self.entries
            .retain(|entry| entry.pinned || entry.created_at_unix >= cutoff);
    }

    pub fn prune_to_max_entries(&mut self, max_entries: usize) {
        if max_entries == 0 {
            self.entries.retain(|entry| entry.pinned);
            return;
        }

        let pinned_count = self.entries.iter().filter(|entry| entry.pinned).count();
        let mut unpinned_budget = max_entries.saturating_sub(pinned_count);

        self.entries.retain(|entry| {
            if entry.pinned {
                true
            } else if unpinned_budget > 0 {
                unpinned_budget -= 1;
                true
            } else {
                false
            }
        });
    }

    pub fn delete(&mut self, id: u64) {
        self.entries.retain(|entry| entry.id != id);
    }

    pub fn toggle_pin(&mut self, id: u64) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            entry.pinned = !entry.pinned;
        }
    }

    pub fn clear_all(&mut self) {
        self.entries.clear();
    }

    pub fn clear_unpinned(&mut self) {
        self.entries.retain(|entry| entry.pinned);
    }

    fn save_unlocked(&self, config: &Config) -> io::Result<()> {
        let config = config.clone().hardened();
        let path = Self::data_path(true);
        if let Some(parent) = path.parent() {
            create_private_dir(parent)?;
        }

        let mut clone = self.clone();
        clone.prune(&config);

        let plaintext = Zeroizing::new(
            serde_json::to_vec_pretty(&clone)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
        );
        let bytes = Self::encrypt_store(&plaintext)?;

        write_private_file(&path, &bytes)
    }

    fn load_encrypted(contents: &str) -> io::Result<Self> {
        let file: EncryptedStoreFile = serde_json::from_str(contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        if file.version != ENCRYPTED_FORMAT_VERSION || file.algorithm != "ChaCha20Poly1305" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported encrypted history format",
            ));
        }

        let nonce_bytes = B64
            .decode(file.nonce_b64)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let nonce: [u8; 12] = nonce_bytes
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid nonce length"))?;
        let ciphertext = B64
            .decode(file.ciphertext_b64)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        let key = get_or_create_history_key()?;
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
                    .map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, "failed to decrypt history")
                    })?;
                let _key_lock = acquire_key_lock()?;
                persist_keyring_key(&legacy_key)?;
                remove_local_history_key()?;
                plaintext
            }
        };

        serde_json::from_slice(&plaintext)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    fn encrypt_store(plaintext: &[u8]) -> io::Result<Vec<u8>> {
        let key = get_or_create_history_key()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key[..]));

        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);

        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|_| io::Error::other("failed to encrypt history"))?;

        let file = EncryptedStoreFile {
            version: ENCRYPTED_FORMAT_VERSION,
            algorithm: "ChaCha20Poly1305".to_string(),
            nonce_b64: B64.encode(nonce),
            ciphertext_b64: B64.encode(ciphertext),
        };

        serde_json::to_vec_pretty(&file)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    fn data_path(encrypted: bool) -> PathBuf {
        let filename = if encrypted {
            "history.enc.json"
        } else {
            "history.json"
        };

        storage_base_dir().join(filename)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AddContentResult {
    Added,
    SkippedEmpty,
    SkippedDuplicate,
    SkippedPrivateMode,
    SkippedSensitive,
    SkippedTooLarge,
    SkippedUnsupportedMime,
}

pub fn is_allowed_image_mime(mime: &str) -> bool {
    ALLOWED_IMAGE_MIME_TYPES.contains(&mime)
}

fn get_or_create_history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    match read_keyring_secret() {
        Ok(secret) => decode_history_key(&secret),
        Err(keyring::Error::NoEntry) => migrate_legacy_or_create_history_key(),
        Err(error) => Err(io::Error::other(error)),
    }
}

fn migrate_legacy_or_create_history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    let _key_lock = acquire_key_lock()?;

    // Another panel process may have initialized the key while we waited.
    match read_keyring_secret() {
        Ok(secret) => return decode_history_key(&secret),
        Err(keyring::Error::NoEntry) => {}
        Err(error) => return Err(io::Error::other(error)),
    }

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
    if &verified[..] != key {
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

fn decode_history_key(secret: &str) -> io::Result<Zeroizing<[u8; 32]>> {
    let bytes = B64
        .decode(secret)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid key length"))?;
    Ok(Zeroizing::new(key))
}

fn storage_base_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/share"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tihulu-clipboard-manager")
}

fn local_key_path() -> PathBuf {
    storage_base_dir().join(LOCAL_KEY_FILE)
}

fn acquire_storage_lock() -> io::Result<StorageLock> {
    acquire_named_lock(STORAGE_LOCK_DIR)
}

fn acquire_key_lock() -> io::Result<StorageLock> {
    acquire_named_lock(KEY_LOCK_DIR)
}

fn acquire_named_lock(lock_dir: &str) -> io::Result<StorageLock> {
    let lock_path = storage_base_dir().join(lock_dir);
    for _ in 0..STORAGE_LOCK_RETRIES {
        match fs::create_dir(&lock_path) {
            Ok(()) => {
                let _ = fs::write(lock_path.join("pid"), std::process::id().to_string());
                return Ok(StorageLock { path: lock_path });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if !lock_owner_is_running(&lock_path) {
                    let _ = fs::remove_dir_all(&lock_path);
                    continue;
                }
                thread::sleep(STORAGE_LOCK_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        format!("timed out waiting for clipboard storage lock: {lock_dir}"),
    ))
}

fn lock_owner_is_running(lock_path: &Path) -> bool {
    let Ok(pid_text) = fs::read_to_string(lock_path.join("pid")) else {
        return false;
    };
    let Ok(pid) = pid_text.trim().parse::<u32>() else {
        return false;
    };

    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!("/proc/{pid}")).exists()
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn create_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;

    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;

    Ok(())
}

fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "history path has no parent"))?;
    create_private_dir(parent)?;

    let tmp_path = parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("history.enc.json"),
        std::process::id(),
        unix_now_nanos()
    ));

    #[cfg(unix)]
    {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
        fs::rename(&tmp_path, path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        sync_parent_dir(parent)?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        fs::write(&tmp_path, bytes)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }
}

#[cfg(unix)]
fn sync_parent_dir(path: &Path) -> io::Result<()> {
    OpenOptions::new().read(true).open(path)?.sync_all()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn unix_now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{AddContentResult, ClipboardStore, EncryptionState, is_allowed_image_mime};
    use crate::config::Config;

    #[test]
    fn image_mime_allowlist_is_strict() {
        assert!(is_allowed_image_mime("image/png"));
        assert!(is_allowed_image_mime("image/jpeg"));
        assert!(!is_allowed_image_mime("image/svg+xml"));
        assert!(!is_allowed_image_mime("text/html"));
    }

    #[test]
    fn image_size_limit_is_enforced_when_enabled() {
        let config = Config {
            safe_core: false,
            image_clipboard: true,
            limit_image_size: true,
            max_image_bytes: 3,
            ..Config::default()
        };
        let mut store = ClipboardStore::default();

        assert_eq!(
            store.add_image("image/png", &[1, 2, 3, 4], &config),
            AddContentResult::SkippedTooLarge
        );
    }

    #[test]
    fn image_size_limit_can_be_disabled() {
        let config = Config {
            safe_core: false,
            image_clipboard: true,
            limit_image_size: false,
            max_image_bytes: 3,
            ..Config::default()
        };
        let mut store = ClipboardStore::default();

        assert_eq!(
            store.add_image("image/png", &[1, 2, 3, 4], &config),
            AddContentResult::Added
        );
    }

    #[test]
    fn safe_core_keeps_image_history_but_disables_previews() {
        let config = Config {
            safe_core: true,
            image_clipboard: true,
            ..Config::default()
        };
        let mut store = ClipboardStore::default();

        assert_eq!(
            store.add_image("image/png", &[1, 2, 3, 4], &config),
            AddContentResult::Added
        );
        assert!(config.effective_image_clipboard());
        assert!(!config.image_previews_enabled());
        assert!(config.effective_limit_image_size());
        assert_eq!(config.effective_max_image_bytes(), 5 * 1024 * 1024);
    }

    #[test]
    fn default_config_uses_panel_safe_image_settings() {
        let config = Config::default();
        assert!(config.safe_core);
        assert!(config.effective_image_clipboard());
        assert!(!config.image_previews_enabled());
        assert!(config.effective_limit_image_size());
    }

    #[test]
    fn image_can_be_added_and_deduplicated() {
        let config = Config {
            safe_core: false,
            image_clipboard: true,
            ..Config::default()
        };
        let mut store = ClipboardStore::default();

        assert_eq!(
            store.add_image("image/png", &[1, 2, 3, 4], &config),
            AddContentResult::Added
        );
        assert_eq!(
            store.add_image("image/png", &[1, 2, 3, 4], &config),
            AddContentResult::SkippedDuplicate
        );
    }

    #[test]
    fn encrypted_and_ready_states_are_secure() {
        assert!(EncryptionState::Ready.is_secure());
        assert!(EncryptionState::Encrypted.is_secure());
        assert!(!EncryptionState::Plaintext.is_secure());
        assert!(!EncryptionState::Error.is_secure());
    }
}
