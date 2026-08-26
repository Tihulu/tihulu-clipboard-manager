// SPDX-License-Identifier: GPL-3.0-or-later

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};

pub const SAFE_CORE_MAX_ENTRIES: usize = 50;
pub const SAFE_CORE_MAX_AGE_DAYS: u64 = 7;
pub const SAFE_CORE_MAX_TEXT_BYTES: usize = 64 * 1024;
pub const SAFE_CORE_MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 7]
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
    pub safe_core: bool,
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
            safe_core: true,
        }
    }
}

impl Config {
    pub fn hardened(mut self) -> Self {
        self.encrypt_history = true;
        self
    }

    pub fn effective_image_clipboard(&self) -> bool {
        self.image_clipboard
    }

    pub fn effective_limit_image_size(&self) -> bool {
        self.limit_image_size || self.safe_core
    }

    pub fn effective_max_entries(&self) -> usize {
        if self.safe_core {
            self.max_entries.min(SAFE_CORE_MAX_ENTRIES)
        } else {
            self.max_entries
        }
    }

    pub fn effective_max_age_days(&self) -> u64 {
        if self.safe_core {
            if self.max_age_days == 0 {
                SAFE_CORE_MAX_AGE_DAYS
            } else {
                self.max_age_days.min(SAFE_CORE_MAX_AGE_DAYS)
            }
        } else {
            self.max_age_days
        }
    }

    pub fn effective_max_text_bytes(&self) -> usize {
        if self.safe_core {
            self.max_text_bytes.min(SAFE_CORE_MAX_TEXT_BYTES)
        } else {
            self.max_text_bytes
        }
    }

    pub fn effective_max_image_bytes(&self) -> usize {
        if self.safe_core {
            self.max_image_bytes.min(SAFE_CORE_MAX_IMAGE_BYTES)
        } else {
            self.max_image_bytes
        }
    }

    pub fn image_previews_enabled(&self) -> bool {
        !self.safe_core
    }
}
