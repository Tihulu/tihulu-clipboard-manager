from pathlib import Path
import runpy

# Apply the original migration first, then layer fixes found by CI/security review.
runpy.run_path('scripts/apply-keyring-only-migration.py', run_name='__main__')

p = Path('src/storage.rs')
s = p.read_text()

# rustc E0283: avoid ambiguous AsRef implementations pulled in through libcosmic/palette.
s = s.replace('if verified.as_ref() != key {', 'if &verified[..] != key {')

# Serialize first-time/migration key initialization across multiple COSMIC panel
# processes. The history lock cannot be reused here because save() already holds it.
s = s.replace(
    'const STORAGE_LOCK_DIR: &str = "history.lock";\n',
    'const STORAGE_LOCK_DIR: &str = "history.lock";\nconst KEY_LOCK_DIR: &str = "history-key.lock";\n',
    1,
)

old = '''fn migrate_legacy_or_create_history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    match read_local_history_key() {'''
new = '''fn migrate_legacy_or_create_history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    let _key_lock = acquire_key_lock()?;

    // Another panel process may have initialized the key while we waited.
    match read_keyring_secret() {
        Ok(secret) => return decode_history_key(&secret),
        Err(keyring::Error::NoEntry) => {}
        Err(error) => return Err(io::Error::other(error)),
    }

    match read_local_history_key() {'''
assert old in s
s = s.replace(old, new, 1)

# Explicit reset is destructive by design, but key rotation itself must still be
# serialized with concurrent applet processes.
old = '''        let store = Self::default();
        rotate_history_key()?;
        store.save_unlocked(config)?;'''
new = '''        let store = Self::default();
        let _key_lock = acquire_key_lock()?;
        rotate_history_key()?;
        store.save_unlocked(config)?;'''
assert old in s
s = s.replace(old, new, 1)

# If a stale/wrong keyring key exists but the legacy key still decrypts the file,
# serialize the repair before replacing the keyring entry and removing history.key.
old = '''                persist_keyring_key(&legacy_key)?;
                remove_local_history_key()?;
                plaintext'''
new = '''                let _key_lock = acquire_key_lock()?;
                persist_keyring_key(&legacy_key)?;
                remove_local_history_key()?;
                plaintext'''
assert old in s
s = s.replace(old, new, 1)

old = '''fn acquire_storage_lock() -> io::Result<StorageLock> {
    let lock_path = storage_base_dir().join(STORAGE_LOCK_DIR);'''
new = '''fn acquire_storage_lock() -> io::Result<StorageLock> {
    acquire_named_lock(STORAGE_LOCK_DIR)
}

fn acquire_key_lock() -> io::Result<StorageLock> {
    acquire_named_lock(KEY_LOCK_DIR)
}

fn acquire_named_lock(lock_dir: &str) -> io::Result<StorageLock> {
    let lock_path = storage_base_dir().join(lock_dir);'''
assert old in s
s = s.replace(old, new, 1)

# Keep the lock timeout diagnostic useful for either lock.
s = s.replace(
    '"timed out waiting for clipboard history storage lock",',
    'format!("timed out waiting for clipboard storage lock: {lock_dir}"),',
    1,
)

# Document the multi-process key initialization hardening.
changelog = Path('CHANGELOG.md')
t = changelog.read_text()
needle = '- Refuse to delete plaintext history until the encrypted history can be read successfully.\n'
addition = '- Serialize key initialization and migration across concurrent COSMIC panel processes.\n'
if addition not in t:
    t = t.replace(needle, needle + addition, 1)
changelog.write_text(t)

p.write_text(s)
