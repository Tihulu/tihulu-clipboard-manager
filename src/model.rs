// SPDX-License-Identifier: GPL-3.0-or-later

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ClipboardPayload {
    Text(String),
    Image {
        mime: String,
        bytes_b64: String,
        size_bytes: usize,
    },
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
            ClipboardPayload::Image {
                mime, size_bytes, ..
            } => format!("Image: {mime} ({})", human_size(*size_bytes)),
        }
    }

    pub fn text(&self) -> Option<&str> {
        match &self.payload {
            ClipboardPayload::Text(text) => Some(text),
            ClipboardPayload::Image { .. } => None,
        }
    }

    pub fn image(&self) -> Option<(&str, Vec<u8>)> {
        match &self.payload {
            ClipboardPayload::Text(_) => None,
            ClipboardPayload::Image {
                mime, bytes_b64, ..
            } => B64.decode(bytes_b64).ok().map(|bytes| (mime.as_str(), bytes)),
        }
    }
}

pub fn encode_image_payload(mime: impl Into<String>, bytes: &[u8]) -> ClipboardPayload {
    ClipboardPayload::Image {
        mime: mime.into(),
        bytes_b64: B64.encode(bytes),
        size_bytes: bytes.len(),
    }
}

fn human_size(bytes: usize) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;

    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / MIB)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::{encode_image_payload, ClipboardEntry, ClipboardPayload};

    #[test]
    fn image_payload_roundtrips() {
        let payload = encode_image_payload("image/png", &[1, 2, 3, 4]);
        let entry = ClipboardEntry {
            id: 1,
            payload,
            pinned: false,
            created_at_unix: 0,
        };

        let Some((mime, bytes)) = entry.image() else {
            panic!("expected image payload");
        };

        assert_eq!(mime, "image/png");
        assert_eq!(bytes, vec![1, 2, 3, 4]);
        assert!(entry.preview().contains("image/png"));
    }

    #[test]
    fn text_payload_has_no_image() {
        let entry = ClipboardEntry {
            id: 1,
            payload: ClipboardPayload::Text("hello".to_string()),
            pinned: false,
            created_at_unix: 0,
        };

        assert_eq!(entry.text(), Some("hello"));
        assert!(entry.image().is_none());
    }
}
