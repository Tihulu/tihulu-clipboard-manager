// SPDX-License-Identifier: GPL-3.0-or-later

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    pub confirm_before_clear_all: bool,
    pub max_entries: usize,
    pub keep_pinned_on_clear_unpinned: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            confirm_before_clear_all: true,
            max_entries: 200,
            keep_pinned_on_clear_unpinned: true,
        }
    }
}
