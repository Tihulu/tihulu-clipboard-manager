// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::Message;
use futures::{SinkExt, channel::mpsc::Sender};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io,
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::{sleep, timeout},
};

const TEXT_POLL_INTERVAL: Duration = Duration::from_secs(2);
const IMAGE_POLL_INTERVAL: Duration = Duration::from_secs(10);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
const TEXT_READ_MAX_BYTES: usize = 256 * 1024;
const IMAGE_READ_MAX_BYTES: usize = 25 * 1024 * 1024;
const MIME_LIST_MAX_BYTES: usize = 8 * 1024;
const READ_CHUNK_BYTES: usize = 8 * 1024;
const IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

pub async fn watch_text_clipboard(mut sender: Sender<Message>) {
    let mut last_seen_text: Option<String> = None;

    loop {
        if let Ok(Some(text)) = read_text_clipboard().await
            && last_seen_text.as_deref() != Some(text.as_str())
        {
            last_seen_text = Some(text.clone());
            if sender.send(Message::ClipboardChanged(text)).await.is_err() {
                return;
            }
        }

        sleep(TEXT_POLL_INTERVAL).await;
    }
}

pub async fn watch_image_clipboard(mut sender: Sender<Message>) {
    let mut last_seen_image: Option<(String, u64)> = None;

    loop {
        if let Ok(Some((mime, bytes))) = read_image_clipboard().await {
            let hash = hash_payload(&mime, &bytes);
            if last_seen_image.as_ref() != Some(&(mime.clone(), hash)) {
                last_seen_image = Some((mime.clone(), hash));
                if sender
                    .send(Message::ClipboardImageChanged {
                        mime,
                        bytes: bytes.into_boxed_slice(),
                    })
                    .await
                    .is_err()
                {
                    return;
                }
            }
        }

        sleep(IMAGE_POLL_INTERVAL).await;
    }
}

pub async fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    write_clipboard("text/plain", text.into_bytes()).await
}

pub async fn copy_image_to_clipboard(mime: String, bytes: Box<[u8]>) -> Result<(), String> {
    write_clipboard(&mime, bytes.into_vec()).await
}

async fn write_clipboard(mime: &str, bytes: Vec<u8>) -> Result<(), String> {
    let mut command = Command::new("wl-copy");
    command
        .args(["--type", mime])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to start wl-copy: {error}"))?;

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill().await;
        let _ = child.wait().await;
        return Err("failed to open wl-copy stdin".to_string());
    };

    if let Err(error) = stdin.write_all(&bytes).await {
        let _ = child.kill().await;
        let _ = child.wait().await;
        return Err(format!("failed to write clipboard payload: {error}"));
    }
    drop(stdin);

    let status = match timeout(COMMAND_TIMEOUT, child.wait()).await {
        Ok(result) => result.map_err(|error| format!("failed to wait for wl-copy: {error}"))?,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err("wl-copy timed out".to_string());
        }
    };

    if status.success() {
        Ok(())
    } else {
        Err(format!("wl-copy exited with status {status}"))
    }
}

async fn read_text_clipboard() -> io::Result<Option<String>> {
    let Some(bytes) = read_clipboard_mime("text/plain", true, TEXT_READ_MAX_BYTES).await? else {
        return Ok(None);
    };

    let text = String::from_utf8_lossy(&bytes).to_string();
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

async fn read_image_clipboard() -> io::Result<Option<(String, Vec<u8>)>> {
    let available_types = list_clipboard_types().await.unwrap_or_default();

    for mime in IMAGE_MIME_TYPES {
        if !available_types.iter().any(|available| available == mime) {
            continue;
        }

        if let Some(bytes) = read_clipboard_mime(mime, false, IMAGE_READ_MAX_BYTES).await?
            && !bytes.is_empty()
        {
            return Ok(Some(((*mime).to_string(), bytes)));
        }
    }

    Ok(None)
}

async fn list_clipboard_types() -> io::Result<Vec<String>> {
    let mut command = Command::new("wl-paste");
    command.arg("--list-types");

    let Some(bytes) = capture_command_output(command, MIME_LIST_MAX_BYTES).await? else {
        return Ok(Vec::new());
    };

    Ok(String::from_utf8_lossy(&bytes)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

async fn read_clipboard_mime(
    mime: &str,
    no_newline: bool,
    max_bytes: usize,
) -> io::Result<Option<Vec<u8>>> {
    let mut command = Command::new("wl-paste");
    if no_newline {
        command.arg("--no-newline");
    }
    command.args(["--type", mime]);

    capture_command_output(command, max_bytes).await
}

async fn capture_command_output(
    mut command: Command,
    max_bytes: usize,
) -> io::Result<Option<Vec<u8>>> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    let mut child = command.spawn()?;
    let Some(mut stdout) = child.stdout.take() else {
        return Err(io::Error::other("failed to open command stdout"));
    };

    let output = timeout(COMMAND_TIMEOUT, async move {
        let mut bytes = Vec::new();
        let mut buffer = vec![0u8; READ_CHUNK_BYTES];

        loop {
            let read = stdout.read(&mut buffer).await?;
            if read == 0 {
                break;
            }

            if bytes.len().saturating_add(read) > max_bytes {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "clipboard payload is too large",
                ));
            }

            bytes.extend_from_slice(&buffer[..read]);
        }

        let status = child.wait().await?;
        Ok::<_, io::Error>((status, bytes))
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "clipboard command timed out"))??;

    let (status, bytes) = output;
    if !status.success() || bytes.is_empty() {
        return Ok(None);
    }

    Ok(Some(bytes))
}

fn hash_payload(mime: &str, bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    mime.hash(&mut hasher);
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::hash_payload;

    #[test]
    fn image_hash_includes_mime_and_bytes() {
        assert_eq!(
            hash_payload("image/png", &[1, 2]),
            hash_payload("image/png", &[1, 2])
        );
        assert_ne!(
            hash_payload("image/png", &[1, 2]),
            hash_payload("image/jpeg", &[1, 2])
        );
        assert_ne!(
            hash_payload("image/png", &[1, 2]),
            hash_payload("image/png", &[1, 3])
        );
    }
}
