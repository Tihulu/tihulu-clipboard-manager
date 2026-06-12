// SPDX-License-Identifier: GPL-3.0-or-later

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};
use rand_core::{OsRng, RngCore};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

const SERVICE: &str = "io.github.tihulu.ClipboardManager.GNOME";
const KEY_NAME: &str = "history-v1";
const FORMAT_VERSION: u8 = 1;
const IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    confirm_before_clear_all: bool,
    max_entries: usize,
    max_age_days: u64,
    keep_pinned_on_clear_unpinned: bool,
    private_mode: bool,
    unique_session: bool,
    encrypt_history: bool,
    sensitive_filter: bool,
    max_text_bytes: usize,
    image_clipboard: bool,
    limit_image_size: bool,
    max_image_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            confirm_before_clear_all: true,
            max_entries: 100,
            max_age_days: 30,
            keep_pinned_on_clear_unpinned: true,
            private_mode: false,
            unique_session: false,
            encrypt_history: true,
            sensitive_filter: true,
            max_text_bytes: 1024 * 1024,
            image_clipboard: true,
            limit_image_size: true,
            max_image_bytes: 25 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Payload {
    Text {
        text: String,
    },
    Image {
        mime: String,
        bytes_b64: String,
        size_bytes: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: u64,
    payload: Payload,
    pinned: bool,
    created_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Store {
    entries: Vec<Entry>,
    next_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncFile {
    version: u8,
    algorithm: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ViewEntry {
    id: u64,
    kind: String,
    preview: String,
    mime: Option<String>,
    size_bytes: Option<usize>,
    pinned: bool,
    created_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct State {
    config: Config,
    entries: Vec<ViewEntry>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "state".to_string());
    let mut config = load_config();

    if config.unique_session && command == "capture" {
        let store = Store::default();
        save_store(&store, &config)?;
        print_state(&store, &config)?;
        return Ok(());
    }

    let mut store = load_store(&config);

    match command.as_str() {
        "state" => {}
        "capture" => {
            capture_clipboard(&mut store, &config)?;
            prune(&mut store, &config);
            save_store(&store, &config)?;
        }
        "copy" => {
            let id = parse_id(args.next())?;
            copy_entry(&store, id)?;
        }
        "delete" => {
            let id = parse_id(args.next())?;
            store.entries.retain(|entry| entry.id != id);
            save_store(&store, &config)?;
        }
        "toggle-pin" => {
            let id = parse_id(args.next())?;
            if let Some(entry) = store.entries.iter_mut().find(|entry| entry.id == id) {
                entry.pinned = !entry.pinned;
            }
            save_store(&store, &config)?;
        }
        "clear-all" => {
            store.entries.clear();
            save_store(&store, &config)?;
        }
        "clear-unpinned" => {
            store.entries.retain(|entry| entry.pinned);
            save_store(&store, &config)?;
        }
        "set" => {
            let key = args
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing config key"))?;
            let value = args.next().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "missing config value")
            })?;
            set_config_value(&mut config, &key, &value)?;
            save_config(&config)?;
            prune(&mut store, &config);
            save_store(&store, &config)?;
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown command",
            ));
        }
    }

    print_state(&store, &config)
}

fn parse_id(value: Option<String>) -> io::Result<u64> {
    value
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing entry id"))?
        .parse::<u64>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn set_config_value(config: &mut Config, key: &str, value: &str) -> io::Result<()> {
    let bool_value = || match value {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected boolean value",
        )),
    };

    match key {
        "confirmBeforeClearAll" => config.confirm_before_clear_all = bool_value()?,
        "keepPinnedOnClearUnpinned" => config.keep_pinned_on_clear_unpinned = bool_value()?,
        "privateMode" => config.private_mode = bool_value()?,
        "uniqueSession" => config.unique_session = bool_value()?,
        "encryptHistory" => config.encrypt_history = bool_value()?,
        "sensitiveFilter" => config.sensitive_filter = bool_value()?,
        "imageClipboard" => config.image_clipboard = bool_value()?,
        "limitImageSize" => config.limit_image_size = bool_value()?,
        "maxEntries" => config.max_entries = value.parse().map_err(io::Error::other)?,
        "maxAgeDays" => config.max_age_days = value.parse().map_err(io::Error::other)?,
        "maxTextBytes" => config.max_text_bytes = value.parse().map_err(io::Error::other)?,
        "maxImageBytes" => config.max_image_bytes = value.parse().map_err(io::Error::other)?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown config key",
            ));
        }
    }

    Ok(())
}

fn capture_clipboard(store: &mut Store, config: &Config) -> io::Result<()> {
    if config.private_mode {
        return Ok(());
    }

    if let Some(text) = read_text()? {
        add_text(store, text, config);
        return Ok(());
    }

    if config.image_clipboard
        && let Some((mime, bytes)) = read_image()?
    {
        add_image(store, mime, &bytes, config);
    }

    Ok(())
}

fn add_text(store: &mut Store, text: String, config: &Config) {
    if text.trim().is_empty() || text.len() > config.max_text_bytes {
        return;
    }

    if config.sensitive_filter && looks_sensitive(&text) {
        return;
    }

    if store.entries.iter().any(
        |entry| matches!(&entry.payload, Payload::Text { text: existing } if existing == &text),
    ) {
        return;
    }

    push_entry(store, Payload::Text { text }, config);
}

fn add_image(store: &mut Store, mime: String, bytes: &[u8], config: &Config) {
    if !IMAGE_MIME_TYPES.contains(&mime.as_str()) || bytes.is_empty() {
        return;
    }

    if config.limit_image_size && bytes.len() > config.max_image_bytes {
        return;
    }

    if store.entries.iter().any(|entry| match &entry.payload {
        Payload::Image {
            mime: existing_mime,
            bytes_b64,
            size_bytes,
        } => {
            existing_mime == &mime
                && *size_bytes == bytes.len()
                && B64
                    .decode(bytes_b64)
                    .is_ok_and(|existing| existing == bytes)
        }
        Payload::Text { .. } => false,
    }) {
        return;
    }

    push_entry(
        store,
        Payload::Image {
            mime,
            bytes_b64: B64.encode(bytes),
            size_bytes: bytes.len(),
        },
        config,
    );
}

fn push_entry(store: &mut Store, payload: Payload, config: &Config) {
    if store.next_id == 0 {
        store.next_id = 1;
    }

    store.entries.insert(
        0,
        Entry {
            id: store.next_id,
            payload,
            pinned: false,
            created_at_unix: unix_now(),
        },
    );
    store.next_id += 1;
    prune(store, config);
}

fn prune(store: &mut Store, config: &Config) {
    if config.max_age_days > 0 {
        let cutoff = unix_now().saturating_sub(config.max_age_days.saturating_mul(24 * 60 * 60));
        store
            .entries
            .retain(|entry| entry.pinned || entry.created_at_unix >= cutoff);
    }

    let pinned_count = store.entries.iter().filter(|entry| entry.pinned).count();
    let mut unpinned_budget = config.max_entries.saturating_sub(pinned_count);
    store.entries.retain(|entry| {
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

fn read_text() -> io::Result<Option<String>> {
    let Some(bytes) = read_clipboard_mime("text/plain", true)? else {
        return Ok(None);
    };
    let text = String::from_utf8_lossy(&bytes).to_string();
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

fn read_image() -> io::Result<Option<(String, Vec<u8>)>> {
    for mime in IMAGE_MIME_TYPES {
        if let Some(bytes) = read_clipboard_mime(mime, false)?
            && !bytes.is_empty()
        {
            return Ok(Some(((*mime).to_string(), bytes)));
        }
    }
    Ok(None)
}

fn read_clipboard_mime(mime: &str, no_newline: bool) -> io::Result<Option<Vec<u8>>> {
    if is_wayland_session()
        && command_exists("wl-paste")
        && let Some(bytes) = read_wl_clipboard_mime(mime, no_newline)?
    {
        return Ok(Some(bytes));
    }

    if command_exists("xclip") {
        return read_xclip_clipboard_mime(mime, no_newline);
    }

    Ok(None)
}

fn read_wl_clipboard_mime(mime: &str, no_newline: bool) -> io::Result<Option<Vec<u8>>> {
    let mut command = Command::new("wl-paste");
    if no_newline {
        command.arg("--no-newline");
    }
    command.args(["--type", mime]);
    let output = command.output()?;
    if !output.status.success() || output.stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(output.stdout))
    }
}

fn read_xclip_clipboard_mime(mime: &str, no_newline: bool) -> io::Result<Option<Vec<u8>>> {
    let output = Command::new("xclip")
        .args([
            "-selection",
            "clipboard",
            "-out",
            "-target",
            xclip_target(mime),
        ])
        .output()?;

    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }

    let mut bytes = output.stdout;
    if no_newline {
        while matches!(bytes.last(), Some(b'\n' | b'\r')) {
            bytes.pop();
        }
    }

    Ok(Some(bytes))
}

fn write_clipboard(mime: &str, bytes: Vec<u8>) -> io::Result<()> {
    if is_wayland_session() && command_exists("wl-copy") {
        return write_wl_clipboard(mime, bytes);
    }

    if command_exists("xclip") {
        return write_xclip_clipboard(mime, bytes);
    }

    if command_exists("wl-copy") {
        return write_wl_clipboard(mime, bytes);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no supported clipboard backend found; install xclip on X11 or wl-clipboard on Wayland",
    ))
}

fn write_wl_clipboard(mime: &str, bytes: Vec<u8>) -> io::Result<()> {
    let mut child = Command::new("wl-copy")
        .args(["--type", mime])
        .stdin(Stdio::piped())
        .spawn()?;
    write_child_stdin(&mut child, &bytes)?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("wl-copy failed"))
    }
}

fn write_xclip_clipboard(mime: &str, bytes: Vec<u8>) -> io::Result<()> {
    let mut child = Command::new("xclip")
        .args([
            "-selection",
            "clipboard",
            "-in",
            "-target",
            xclip_target(mime),
        ])
        .stdin(Stdio::piped())
        .spawn()?;
    write_child_stdin(&mut child, &bytes)?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("xclip failed"))
    }
}

fn write_child_stdin(child: &mut std::process::Child, bytes: &[u8]) -> io::Result<()> {
    let Some(mut stdin) = child.stdin.take() else {
        return Err(io::Error::other("failed to open clipboard backend stdin"));
    };
    stdin.write_all(bytes)?;
    drop(stdin);
    Ok(())
}

fn xclip_target(mime: &str) -> &str {
    if mime == "text/plain" {
        "UTF8_STRING"
    } else {
        mime
    }
}

fn is_wayland_session() -> bool {
    env::var("XDG_SESSION_TYPE").is_ok_and(|session| session.eq_ignore_ascii_case("wayland"))
        || env::var_os("WAYLAND_DISPLAY").is_some()
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .is_ok_and(|status| status.success())
}

fn copy_entry(store: &Store, id: u64) -> io::Result<()> {
    let entry = store
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "entry not found"))?;

    match &entry.payload {
        Payload::Text { text } => write_clipboard("text/plain", text.as_bytes().to_vec()),
        Payload::Image {
            mime, bytes_b64, ..
        } => {
            let bytes = B64.decode(bytes_b64).map_err(io::Error::other)?;
            write_clipboard(mime, bytes)
        }
    }
}

fn looks_sensitive(text: &str) -> bool {
    let patterns = [
        concat!("(?i)-----BEGIN [A-Z ]*", "PRIVATE", " ", "KEY", "-----"),
        concat!(
            "(?i)\\b(pass",
            "word|passwd|pwd|tok",
            "en|sec",
            "ret|api[_-]?key)\\s*[:=]\\s*\\S+"
        ),
        concat!("\\b", "AK", "IA", "[0-9A-Z]{16}\\b"),
        concat!("\\b", "gh", "[pousr]_[A-Za-z0-9_]{30,}\\b"),
    ];
    patterns
        .iter()
        .any(|pattern| Regex::new(pattern).is_ok_and(|regex| regex.is_match(text)))
}

fn load_config() -> Config {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

fn save_config(config: &Config) -> io::Result<()> {
    write_private_file(
        &config_path(),
        &serde_json::to_vec_pretty(config).map_err(io::Error::other)?,
    )
}

fn load_store(config: &Config) -> Store {
    let path = store_path(config.encrypt_history);
    let Ok(contents) = fs::read_to_string(path) else {
        return Store {
            entries: Vec::new(),
            next_id: 1,
        };
    };

    if config.encrypt_history {
        load_encrypted(&contents).unwrap_or_else(|_| Store {
            entries: Vec::new(),
            next_id: 1,
        })
    } else {
        serde_json::from_str(&contents).unwrap_or_else(|_| Store {
            entries: Vec::new(),
            next_id: 1,
        })
    }
}

fn save_store(store: &Store, config: &Config) -> io::Result<()> {
    let plaintext = serde_json::to_vec_pretty(store).map_err(io::Error::other)?;
    let bytes = if config.encrypt_history {
        encrypt_bytes(&plaintext)?
    } else {
        plaintext
    };
    write_private_file(&store_path(config.encrypt_history), &bytes)
}

fn load_encrypted(contents: &str) -> io::Result<Store> {
    let file: EncFile = serde_json::from_str(contents).map_err(io::Error::other)?;
    if file.version != FORMAT_VERSION || file.algorithm != "ChaCha20Poly1305" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported format",
        ));
    }

    let nonce_bytes = B64.decode(file.nonce_b64).map_err(io::Error::other)?;
    let nonce: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid nonce"))?;
    let ciphertext = B64.decode(file.ciphertext_b64).map_err(io::Error::other)?;
    let key = history_key()?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key[..]));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "decrypt failed"))?;
    serde_json::from_slice(&plaintext).map_err(io::Error::other)
}

fn encrypt_bytes(plaintext: &[u8]) -> io::Result<Vec<u8>> {
    let key = history_key()?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key[..]));
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| io::Error::other("encrypt failed"))?;
    serde_json::to_vec_pretty(&EncFile {
        version: FORMAT_VERSION,
        algorithm: "ChaCha20Poly1305".to_string(),
        nonce_b64: B64.encode(nonce),
        ciphertext_b64: B64.encode(ciphertext),
    })
    .map_err(io::Error::other)
}

fn history_key() -> io::Result<Zeroizing<[u8; 32]>> {
    let entry = keyring::Entry::new(SERVICE, KEY_NAME).map_err(io::Error::other)?;
    if let Ok(encoded) = entry.get_password() {
        let bytes = B64.decode(encoded).map_err(io::Error::other)?;
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

fn print_state(store: &Store, config: &Config) -> io::Result<()> {
    let entries = store.entries.iter().map(view_entry).collect::<Vec<_>>();
    let state = State {
        config: config.clone(),
        entries,
    };
    println!(
        "{}",
        serde_json::to_string(&state).map_err(io::Error::other)?
    );
    Ok(())
}

fn view_entry(entry: &Entry) -> ViewEntry {
    match &entry.payload {
        Payload::Text { text } => ViewEntry {
            id: entry.id,
            kind: "text".to_string(),
            preview: text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(80)
                .collect(),
            mime: None,
            size_bytes: None,
            pinned: entry.pinned,
            created_at_unix: entry.created_at_unix,
        },
        Payload::Image {
            mime, size_bytes, ..
        } => ViewEntry {
            id: entry.id,
            kind: "image".to_string(),
            preview: format!("Image · {mime} · {} bytes", size_bytes),
            mime: Some(mime.clone()),
            size_bytes: Some(*size_bytes),
            pinned: entry.pinned,
            created_at_unix: entry.created_at_unix,
        },
    }
}

fn data_dir() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/share"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tihulu-clipboard-manager-gnome")
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

fn store_path(encrypted: bool) -> PathBuf {
    data_dir().join(if encrypted {
        "history.enc.json"
    } else {
        "history.json"
    })
}

fn write_private_file(path: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
