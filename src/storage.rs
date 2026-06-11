// SPDX-License-Identifier: GPL-3.0-or-later

use crate::model::{ClipboardEntry, ClipboardPayload};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardStore {
    entries: Vec<ClipboardEntry>,
    next_id: u64,
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
    pub fn load_or_default() -> Self {
        let path = Self::data_path();
        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };

        serde_json::from_str(&contents).unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::data_path();
        if let Some(parent) = path.parent() {
            create_private_dir(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        write_private_file(&path, contents.as_bytes())
    }

    pub fn entries(&self) -> &[ClipboardEntry] {
        &self.entries
    }

    pub fn add_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text.trim().is_empty() {
            return;
        }

        if self.entries.iter().any(|entry| {
            matches!(&entry.payload, ClipboardPayload::Text(existing) if existing == &text)
        }) {
            return;
        }

        let entry = ClipboardEntry {
            id: self.next_id,
            payload: ClipboardPayload::Text(text),
            pinned: false,
            created_at_unix: unix_now(),
        };
        self.next_id += 1;
        self.entries.insert(0, entry);
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

    fn data_path() -> PathBuf {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".local/share"))
            })
            .unwrap_or_else(|| PathBuf::from("."));

        base.join("tihulu-clipboard-manager/history.json")
    }
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
