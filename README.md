# Tihulu Clipboard Manager

A security-first COSMIC panel clipboard manager applet for Pop!_OS / COSMIC.

The main design goal is simple: **Erase All must be visible in the main popup** so clipboard history does not accumulate silently.

## Security-first feature set

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
- Clear unpinned items while keeping pinned entries
- Delete individual entries
- Pin / unpin entries

## Current status

The security storage layer and popup actions are implemented in the scaffold.

Still needed before daily use:

- Connect the real Wayland data-control clipboard watcher
- Implement click-to-copy by setting the Wayland clipboard
- Test against the current COSMIC/libcosmic API on Pop!_OS
- Add per-application ignore rules if COSMIC/Wayland exposes source app metadata
- Run `cargo check`, `cargo fmt`, `cargo clippy`, and `cargo audit` on a COSMIC development machine

## Build locally

Install Rust and `just`, then run:

```bash
cargo check
just run
```

Install system-wide for testing:

```bash
just build-release
sudo just install
```

## Security notes

Clipboard managers are sensitive software. Treat this applet like a password-adjacent tool.

Do not test development builds with real passwords, recovery phrases, API keys, SSH keys, or personal documents until the Wayland watcher and click-to-copy code has had a second security review.

Read [`SECURITY.md`](SECURITY.md) before testing.

## App identity

- Applet name: **Tihulu Clipboard Manager**
- Binary name: `tihulu-clipboard-manager`
- App ID: `io.github.tihulu.ClipboardManager`
- License: GPL-3.0-or-later
