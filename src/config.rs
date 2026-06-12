// SPDX-License-Identifier: GPL-3.0-or-later

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 4]
pub struct Config {
    pub confirm_before_clear_all: bool,
    pub max_entries: usize,
    pub max_age_days: u64,
    pub keep_pinned_on_clear_unpinned: bool,
    pub private_mode: bool,
    pub unique_session: bool,
    pub encrypt_history: bool,
    pub sensitive_filter: bool,
    pub max_text_bytes: usize,
    pub image_clipboard: bool,
    pub limit_image_size: bool,
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
