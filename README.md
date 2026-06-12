# Tihulu Clipboard Manager

A security-first COSMIC panel clipboard manager applet for Pop!_OS / COSMIC.

Tihulu Clipboard Manager focuses on privacy, clear history controls, and encrypted local storage for clipboard history.

## Security-first feature set

- Text clipboard watcher for COSMIC/Wayland via `wl-paste`
- Image clipboard watcher for PNG, JPEG, WebP, and GIF payloads
- Click-to-copy via `wl-copy`
- Image click-to-copy while preserving the MIME type
- Visible **Clear All / Erase All** button in the main popup
- Confirmation before destructive history deletion
- Real Erase All removes plaintext and encrypted persisted history files
- Encrypted history at rest by default
- OS keyring-backed random encryption key
- `ChaCha20Poly1305` authenticated encryption for history storage
- Private mode to stop storing new clipboard items
- Unique session mode to clear persisted history at applet startup
- Maximum history size
- Maximum history age, default 30 days
- Sensitive-content filter for common passwords, API keys, private keys, tokens, OTPs, and recovery phrases
- Oversized text entry protection
- Image clipboard size switch: limited mode defaults to 25 MiB, no-size-cap mode skips the image size check
- Clear unpinned items while keeping pinned entries
- Delete individual entries
- Pin / unpin entries

## Current status

The security storage layer, popup actions, text/image clipboard watcher, image limit switch, and click-to-copy path are implemented in the scaffold.

Still needed before daily use:

- Test against the current COSMIC/libcosmic API on Pop!_OS
- Install and verify `wl-clipboard` on the target system
- Add per-application ignore rules if COSMIC/Wayland exposes source app metadata
- Run `cargo check`, `cargo test`, `cargo fmt`, `cargo clippy`, and `cargo audit` on a COSMIC development machine
- Replace the `wl-clipboard` backend with a native Wayland data-control backend later if needed

## Runtime dependency

The first working clipboard backend uses `wl-clipboard`:

```bash
sudo apt install wl-clipboard
```

It uses:

- `wl-paste` to watch text clipboard payloads
- `wl-paste` to watch PNG, JPEG, WebP, and GIF clipboard payloads
- `wl-copy` to restore/copy a selected history item

## Build locally

Install Rust and `just`, then run:

```bash
cargo check
cargo test --all-targets --all-features
just run
```

Install system-wide for testing:

```bash
just build-release
sudo just install
```

## Security notes

Clipboard managers are sensitive software. Treat this applet like a password-adjacent tool.

Do not test development builds with real passwords, recovery phrases, API keys, SSH keys, or personal documents until the watcher and click-to-copy code has had a second security review on the target COSMIC system.

No-size-cap image mode can store very large screenshots or photos. Keep limited mode enabled unless you specifically need larger image clipboard entries.

Read [`SECURITY.md`](SECURITY.md) before testing.

## App identity

- Applet name: **Tihulu Clipboard Manager**
- Binary name: `tihulu-clipboard-manager`
- App ID: `io.github.tihulu.ClipboardManager`
- License: GPL-3.0-or-later
