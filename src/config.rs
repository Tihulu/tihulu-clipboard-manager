// SPDX-License-Identifier: GPL-3.0-or-later

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 4]
pub struct Config {
    pub confirm_before_clear_all: bool,
    pub max_entries: usize,
    /// Maximum age in days for non-pinned entries. 0 disables age pruning.
    pub max_age_days: u64,
    pub keep_pinned_on_clear_unpinned: bool,
    /// Stop storing newly captured clipboard contents.
    pub private_mode: bool,
    /// Delete all history when the applet starts.
    pub unique_session: bool,
    /// Encrypt the history file at rest using an OS keyring-backed key.
    pub encrypt_history: bool,
    /// Skip entries that look like passwords, tokens, keys, or recovery material.
    pub sensitive_filter: bool,
    /// Maximum text payload size to store, in bytes. 0 disables text storage.
    pub max_text_bytes: usize,
    /// Store common image clipboard payloads.
    pub image_clipboard: bool,
    /// Enforce max_image_bytes when image_clipboard is enabled.
    pub limit_image_size: bool,
    /// Maximum image payload size to store, in bytes when limit_image_size is true.
    pub max_image_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            confirm_before_clear_all: true,
            max_entries: 200,
            max_age_days: 30,
            keep_pinned_on_clear_unpinned: true,
            private_mode: false,
            unique_session: false,
            encrypt_history: true,
            sensitive_filter: true,
            max_text_bytes: 256 * 1024,
            image_clipboard: true,
            limit_image_size: true,
            max_image_bytes: 25 * 1024 * 1024,
        }
    }
}
