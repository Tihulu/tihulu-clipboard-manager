// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ClipboardPayload {
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ClipboardEntry {
    pub id: u64,
    pub payload: ClipboardPayload,
    pub pinned: bool,
    pub created_at_unix: u64,
}

impl ClipboardEntry {
    pub fn preview(&self) -> String {
        match &self.payload {
            ClipboardPayload::Text(text) => text.chars().take(120).collect(),
        }
    }
}
