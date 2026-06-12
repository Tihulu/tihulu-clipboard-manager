# Tihulu Clipboard Manager

A security-first COSMIC panel clipboard manager applet for Pop!_OS / COSMIC.

Tihulu Clipboard Manager focuses on privacy, clear history controls, encrypted local storage, and text/image clipboard history.

## Quick install from GitHub

The quick installer clones the repository, installs common Pop!_OS/Ubuntu build dependencies, installs Rust/`just` if needed, runs `cargo check` and `cargo test`, builds the release binary, and installs the applet under `/usr`.

Review the script first:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/quick-install.sh -o /tmp/tihulu-quick-install.sh
less /tmp/tihulu-quick-install.sh
bash /tmp/tihulu-quick-install.sh
```

One-line install:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/quick-install.sh | bash
```

Optional environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `BRANCH` | `main` | Git branch to install from |
| `PREFIX` | `/usr` | Install prefix passed to `just install` |
| `KEEP_BUILD_DIR` | `0` | Set to `1` to keep the temporary build directory |

Example:

```bash
BRANCH=main PREFIX=/usr/local KEEP_BUILD_DIR=1 bash /tmp/tihulu-quick-install.sh
```

## Current status

The project is in active development. The security storage layer, popup actions, text/image clipboard watcher, image size switch, and click-to-copy path are implemented in the scaffold.

Before daily use, this still needs to be verified on a real COSMIC development machine with:

```bash
cargo check
cargo test --all-targets --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
```

## Features

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

## Tihulu vs COSMIC Clipboard Manager

This comparison is based on the current Tihulu implementation and the public `cosmic-utils/clipboard-manager` project.

### General comparison

| Topic | COSMIC Clipboard Manager | Tihulu Clipboard Manager |
| --- | --- | --- |
| Maturity | More mature, existing working applet | New project, active development |
| Target | General clipboard history applet | Security-first clipboard history applet |
| Clipboard watcher | Native Wayland/data-control backend | `wl-paste` backend for the first working version |
| Click-to-copy | Native applet behavior | `wl-copy` backend |
| Text clipboard | Supported | Supported |
| Image clipboard | Supported through multi-content storage | Supported for PNG, JPEG, WebP, and GIF |
| Native data-control | Yes | Planned later |
| Runtime dependency | No extra `wl-clipboard` dependency expected | Requires `wl-clipboard` for the current backend |
| Daily-use readiness | More ready today | Needs compile/runtime testing first |

### Security and storage comparison

| Topic | COSMIC Clipboard Manager | Tihulu Clipboard Manager |
| --- | --- | --- |
| Storage backend | SQLite | Encrypted JSON history file |
| At-rest encryption | Not visible in the public code reviewed | Enabled by default |
| Encryption algorithm | Not visible | `ChaCha20Poly1305` |
| Encryption key storage | Not visible | OS keyring |
| Text history on disk | Stored in app database | Stored inside encrypted history by default |
| Image history on disk | Stored in app database | Stored inside encrypted history by default |
| File permissions | No explicit app-level hardening noted in the reviewed storage path | Unix directory `0700`, file `0600` |
| Private mode | Available | Available |
| Unique session | Available | Available |
| Max age | Default 30 days | Default 30 days |
| Max entries | Default 500 | Default 200 |
| Sensitive text filter | Not visible in the public code reviewed | Enabled by default |
| Image MIME allowlist | Multi-MIME storage | PNG, JPEG, WebP, GIF only |
| Image size control | Not the focus of reviewed comparison | Limited 25 MiB by default; no-size-cap mode available |
| Clear behavior | Clear keeps favorites | Erase All removes all history; Clear Unpinned is separate |

### Practical decision table

| Use case | Better choice today | Why |
| --- | --- | --- |
| Stable daily use right now | COSMIC Clipboard Manager | It is more mature and already integrated natively |
| Stronger privacy/storage design | Tihulu Clipboard Manager | Encryption, keyring, sensitive filter, and stricter erase behavior |
| Native Wayland backend | COSMIC Clipboard Manager | Tihulu currently uses `wl-clipboard` as a practical first backend |
| Clear all history with no favorites left behind | Tihulu Clipboard Manager | Erase All is designed to delete all entries |
| Encrypted image clipboard history | Tihulu Clipboard Manager | Images are stored inside the encrypted history file by default |
| Large image clipboard entries | Tihulu Clipboard Manager | 25 MiB limited mode by default, no-size-cap mode available |

## Image clipboard behavior

Tihulu supports image clipboard entries for:

- `image/png`
- `image/jpeg`
- `image/webp`
- `image/gif`

By default, image history is limited to 25 MiB per entry. The main popup includes an image size switch:

| Mode | Behavior |
| --- | --- |
| Limited: 25 MiB | Rejects images above 25 MiB |
| No size cap | Skips the image size check |

MIME allowlisting, private mode, duplicate detection, and encryption still apply in both modes.

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

Do not test development builds with real passwords, recovery phrases, API keys, SSH keys, personal documents, private screenshots, or QR codes containing secrets until the watcher and click-to-copy code has had a second security review on the target COSMIC system.

No-size-cap image mode can store very large screenshots or photos. Keep limited mode enabled unless you specifically need larger image clipboard entries.

Read [`SECURITY.md`](SECURITY.md) before testing.

## Roadmap

- Native Wayland data-control backend
- Per-application ignore rules if COSMIC/Wayland exposes source app metadata
- Persistent settings UI for security and image options
- Native COSMIC styling pass
- Release packaging after `cargo check`, tests, clippy, audit, and runtime validation pass

## App identity

- Applet name: **Tihulu Clipboard Manager**
- Binary name: `tihulu-clipboard-manager`
- App ID: `io.github.tihulu.ClipboardManager`
- License: GPL-3.0-or-later
