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
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const KEYRING_SERVICE: &str = "io.github.tihulu.ClipboardManager";
const KEYRING_USER: &str = "history-encryption-key-v1";
const ENCRYPTED_FORMAT_VERSION: u8 = 1;
const ALLOWED_IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

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

impl Default for ClipboardStore {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }
}

impl ClipboardStore {
    pub fn load_or_default(config: &Config) -> Self {
        let path = Self::data_path(config.encrypt_history);
        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };

        if config.encrypt_history {
            Self::load_encrypted(&contents).unwrap_or_default()
        } else {
            serde_json::from_str(&contents).unwrap_or_default()
        }
    }

    pub fn save(&self, config: &Config) -> io::Result<()> {
        let path = Self::data_path(config.encrypt_history);
        if let Some(parent) = path.parent() {
            create_private_dir(parent)?;
        }

        let plaintext = Zeroizing::new(
            serde_json::to_vec_pretty(self)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
        );

        let bytes = if config.encrypt_history {
            Self::encrypt_store(&plaintext)?
        } else {
            plaintext.to_vec()
        };

        write_private_file(&path, &bytes)
    }

    pub fn delete_persisted_files() -> io::Result<()> {
        for path in [Self::data_path(false), Self::data_path(true)] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn entries(&self) -> &[ClipboardEntry] {
        &self.entries
    }

    pub fn add_text(&mut self, text: impl Into<String>, config: &Config) -> AddContentResult {
        if config.private_mode {
            return AddContentResult::SkippedPrivateMode;
        }

        if config.max_text_bytes == 0 {
            return AddContentResult::SkippedTooLarge;
        }

        let text = text.into();
        if text.trim().is_empty() {
            return AddContentResult::SkippedEmpty;
        }

        if text.len() > config.max_text_bytes {
            return AddContentResult::SkippedTooLarge;
        }

        if config.sensitive_filter && looks_sensitive(&text) {
            return AddContentResult::SkippedSensitive;
        }

        if self.entries.iter().any(|entry| {
            matches!(&entry.payload, ClipboardPayload::Text(existing) if existing == &text)
        }) {
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
        self.prune(config);
        AddContentResult::Added
    }

    pub fn add_image(
        &mut self,
        mime: impl Into<String>,
        bytes: &[u8],
        config: &Config,
    ) -> AddContentResult {
        if config.private_mode {
            return AddContentResult::SkippedPrivateMode;
        }

        if !config.image_clipboard {
            return AddContentResult::SkippedUnsupportedMime;
        }

        let mime = mime.into();
        if !is_allowed_image_mime(&mime) {
            return AddContentResult::SkippedUnsupportedMime;
        }

        if bytes.is_empty() {
            return AddContentResult::SkippedEmpty;
        }

        if config.limit_image_size && config.max_image_bytes == 0 {
            return AddContentResult::SkippedTooLarge;
        }

        if config.limit_image_size && bytes.len() > config.max_image_bytes {
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
        self.prune(config);
        AddContentResult::Added
    }

    pub fn prune(&mut self, config: &Config) {
        self.prune_to_max_age(config.max_age_days);
        self.prune_to_max_entries(config.max_entries);
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
        let ciphertext = B64
            .decode(file.ciphertext_b64)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        let key = get_or_create_history_key()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key[..]));
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "failed to decrypt history"))?;

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
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".local/share"))
            })
            .unwrap_or_else(|| PathBuf::from("."));

        let filename = if encrypted {
            "history.enc.json"
        } else {
            "history.json"
        };

        base.join("tihulu-clipboard-manager").join(filename)
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
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(io::Error::other)?;

    if let Ok(secret) = entry.get_password() {
        let bytes = B64
            .decode(secret)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid key length"))?;
        return Ok(Zeroizing::new(key));
    }

    let mut key = Zeroizing::new([0u8; 32]);
    OsRng.fill_bytes(key.as_mut());
    entry
        .set_password(&B64.encode(*key))
        .map_err(io::Error::other)?;

    Ok(key)
}

fn create_private_dir(path: &std::path::Path) -> io::Result<()> {
    fs::create_dir_all(path)?;

    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;

    Ok(())
}

fn write_private_file(path: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        return Ok(());
    }

    #[cfg(not(unix))]
    {
        fs::write(path, bytes)
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{AddContentResult, ClipboardStore, is_allowed_image_mime};
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
    fn image_can_be_added_and_deduplicated() {
        let config = Config::default();
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
}
