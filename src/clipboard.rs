// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::Message;
use futures::{channel::mpsc::Sender, SinkExt};
use std::{io, process::Stdio, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    process::Command,
    time::{sleep, timeout},
};

const POLL_INTERVAL: Duration = Duration::from_millis(750);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);

/// Watch the Wayland text clipboard using `wl-paste`.
///
/// This is the first daily-use backend because it is simple to test on COSMIC/Wayland.
/// A native data-control backend can replace this module later without touching the UI/store.
pub async fn watch_text_clipboard(mut sender: Sender<Message>) {
    let mut last_seen: Option<String> = None;

    loop {
        match read_text_clipboard().await {
            Ok(Some(text)) => {
                if last_seen.as_deref() != Some(text.as_str()) {
                    last_seen = Some(text.clone());
                    if sender.send(Message::ClipboardChanged(text)).await.is_err() {
                        return;
                    }
                }
            }
            Ok(None) => {}
            Err(error) => {
                let _ = sender
                    .send(Message::ClipboardBackendWarning(format!(
                        "wl-paste clipboard watcher failed: {error}"
                    )))
                    .await;
                sleep(Duration::from_secs(5)).await;
            }
        }

        sleep(POLL_INTERVAL).await;
    }
}

pub async fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start wl-copy: {error}"))?;

    let Some(mut stdin) = child.stdin.take() else {
        return Err("failed to open wl-copy stdin".to_string());
    };

    stdin
        .write_all(text.as_bytes())
        .await
        .map_err(|error| format!("failed to write clipboard text: {error}"))?;
    drop(stdin);

    let status = timeout(COMMAND_TIMEOUT, child.wait())
        .await
        .map_err(|_| "wl-copy timed out".to_string())?
        .map_err(|error| format!("failed to wait for wl-copy: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("wl-copy exited with status {status}"))
    }
}

async fn read_text_clipboard() -> io::Result<Option<String>> {
    let output = timeout(
        COMMAND_TIMEOUT,
        Command::new("wl-paste")
            .args(["--no-newline", "--type", "text"])
            .output(),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "wl-paste timed out"))??;

    if !output.status.success() {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}
