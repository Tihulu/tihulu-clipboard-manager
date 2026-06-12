# Tihulu Clipboard Manager

A security-first clipboard history project for COSMIC/Wayland and GNOME/Ubuntu.

Tihulu focuses on privacy, visible clear controls, encrypted local storage, and text/image clipboard history.

## Which version should I install?

| Desktop/session | Recommended path | Notes |
| --- | --- | --- |
| GNOME on Ubuntu / Pop!_OS / Debian | GNOME/Ubuntu installer | Uses a GNOME Shell extension plus a `systemd --user` background service. Works on X11/Xorg with `xclip`; Wayland uses `wl-clipboard`. |
| COSMIC / Wayland | COSMIC applet installer | Native COSMIC applet project. Uses `wl-paste`/`wl-copy` for the current backend. |
| Other desktops | Not packaged yet | Manual testing only. |

## GNOME / Ubuntu quick install

Install dependencies first:

```bash
sudo apt update
sudo apt install -y git cargo gnome-shell-extensions xclip wl-clipboard coreutils build-essential pkg-config libssl-dev
```

Install from GitHub:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/gnome/scripts/quick-install.sh | bash
```

On GNOME X11/Xorg, reload GNOME Shell after install:

```text
Alt + F2 → r → Enter
```

On GNOME Wayland, log out and log back in.

The GNOME install creates:

```text
~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
~/.local/bin/tihulu-gnome-clipboard-helper
~/.config/systemd/user/tihulu-gnome-clipboard-daemon.service
```

Check the background service:

```bash
systemctl --user status tihulu-gnome-clipboard-daemon.service --no-pager
```

Follow logs:

```bash
journalctl --user -u tihulu-gnome-clipboard-daemon.service -f
```

Clean install / reset history:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/gnome/scripts/quick-install.sh -o /tmp/tihulu-gnome-install.sh
RESET_HISTORY=1 bash /tmp/tihulu-gnome-install.sh
```

Uninstall:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/gnome/scripts/uninstall.sh | bash
```

Remove history too:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/gnome/scripts/uninstall.sh -o /tmp/tihulu-gnome-uninstall.sh
REMOVE_HISTORY=1 bash /tmp/tihulu-gnome-uninstall.sh
```

GNOME docs:

- [`gnome/README.md`](gnome/README.md)
- [`gnome/RELEASE_NOTES.md`](gnome/RELEASE_NOTES.md)
- [`gnome/CHECKS.md`](gnome/CHECKS.md)

## GNOME release packaging

GNOME release packages are built by:

```text
.github/workflows/gnome-ubuntu-release.yml
```

To create a GNOME release from a clean local clone:

```bash
git checkout main
git pull
git tag -a gnome-v2.3.0 -m "Tihulu GNOME Ubuntu v2.3.0"
git push origin gnome-v2.3.0
```

The GitHub Actions workflow packages the GNOME extension, helper, service, install script, uninstall script, and docs into a zip release asset.

## COSMIC / Wayland quick install

The COSMIC quick installer clones the repository, installs common Pop!_OS/Ubuntu build dependencies, installs Rust/`just` if needed, runs checks, builds the release binary, and installs the applet.

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

The project is in active development.

The GNOME/Ubuntu path has a daemon-backed installer and has been tested locally on GNOME X11 during development. The COSMIC path still needs more validation on a real COSMIC development machine.

Before daily use, verify the target build with:

```bash
cargo check
cargo test --all-targets --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
```

## Features

- Text clipboard history
- Image clipboard history for PNG, JPEG, WebP, and GIF payloads
- Click-to-copy old entries
- Image click-to-copy while preserving MIME type
- Visible **Clear All / Erase All** controls
- Confirmation before destructive history deletion
- Encrypted history at rest by default
- `ChaCha20Poly1305` authenticated encryption for history storage
- OS keyring/local key fallback depending on the target helper
- Private mode to stop storing new clipboard items
- Unique session mode
- Maximum history size
- Maximum history age, default 30 days
- Sensitive-content filter for common passwords, API keys, private keys, tokens, OTPs, and recovery phrases
- Oversized text entry protection
- Image clipboard size limit
- Clear unpinned items while keeping pinned entries
- Delete individual entries
- Pin / unpin entries

## Image clipboard behavior

Tihulu supports image clipboard entries for:

- `image/png`
- `image/jpeg`
- `image/webp`
- `image/gif`

By default, image history is limited to 25 MiB per entry. MIME allowlisting, private mode, duplicate detection, and encryption still apply.

## Runtime dependencies

COSMIC/Wayland currently uses `wl-clipboard`:

```bash
sudo apt install wl-clipboard
```

GNOME X11/Xorg uses `xclip`:

```bash
sudo apt install xclip
```

GNOME Wayland uses `wl-clipboard`:

```bash
sudo apt install wl-clipboard
```

## Build locally

Install Rust and `just`, then run:

```bash
cargo check
cargo test --all-targets --all-features
just run
```

Install system-wide for COSMIC testing:

```bash
just build-release
sudo just install
```

## Security notes

Clipboard managers are sensitive software. Treat this applet like a password-adjacent tool.

Do not test development builds with real passwords, recovery phrases, API keys, SSH keys, personal documents, private screenshots, or QR codes containing secrets until the watcher and click-to-copy code has had a second security review on the target desktop/session.

No-size-cap image mode can store very large screenshots or photos. Keep limited mode enabled unless you specifically need larger image clipboard entries.

Read [`SECURITY.md`](SECURITY.md) before testing.

## Roadmap

- Dedicated native GNOME daemon command for release builds
- Native Wayland data-control backend
- Per-application ignore rules if the desktop/session exposes source app metadata
- Persistent settings UI for security and image options
- Native COSMIC styling pass
- Release packaging after CI and runtime validation pass

## App identity

- Applet name: **Tihulu Clipboard Manager**
- COSMIC binary name: `tihulu-clipboard-manager`
- GNOME helper binary name: `tihulu-gnome-clipboard-helper`
- App ID: `io.github.tihulu.ClipboardManager`
- License: GPL-3.0-or-later
