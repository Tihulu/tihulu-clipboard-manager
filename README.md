# Tihulu Clipboard Manager

A security-first clipboard history project for COSMIC/Wayland and GNOME/Ubuntu.

Tihulu focuses on privacy, visible clear controls, encrypted local storage, text/image clipboard history, and stable panel behavior.

## Stable one-line install

For Pop!_OS 24.04 COSMIC Wayland, install the latest stable release from the pinned `v0.2.0` tag:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/v0.2.0/scripts/quick-install.sh | BRANCH=v0.2.0 bash
```

The installer builds from the stable `v0.2.0` release tag, runs Rust checks/tests, installs the COSMIC desktop entry, and asks you to restart COSMIC Shell or log out/in if the applet does not appear immediately.

To install development `main` instead:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/quick-install.sh | bash
```

## Which version should I install?

| Desktop/session | Recommended path | Notes |
| --- | --- | --- |
| GNOME on Ubuntu / Pop!_OS / Debian | GNOME/Ubuntu installer | Uses a GNOME Shell extension plus a native helper. |
| COSMIC / Wayland | COSMIC applet installer | Native COSMIC applet for Pop!_OS 24.04 COSMIC Wayland. Uses `wl-paste`/`wl-copy` for the current clipboard backend. |
| Other desktops | Not packaged yet | Manual testing only. |

## COSMIC / Wayland quick install

This is the recommended path for Pop!_OS 24.04 COSMIC on Wayland.

Review the stable release script first:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/v0.2.0/scripts/quick-install.sh -o /tmp/tihulu-quick-install.sh
less /tmp/tihulu-quick-install.sh
BRANCH=v0.2.0 bash /tmp/tihulu-quick-install.sh
```

One-line stable install from GitHub:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/v0.2.0/scripts/quick-install.sh | BRANCH=v0.2.0 bash
```

Optional environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `BRANCH` | `main` | Git branch or tag to install from |
| `PREFIX` | `/usr` | Install prefix passed to `just install` |
| `KEEP_BUILD_DIR` | `0` | Set to `1` to keep the temporary build directory |

Example development install:

```bash
BRANCH=main PREFIX=/usr/local KEEP_BUILD_DIR=1 bash /tmp/tihulu-quick-install.sh
```

## Update from GitHub

For an already-installed system, use the updater:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/update-from-github.sh | bash
```

Force COSMIC:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/update-from-github.sh -o /tmp/tihulu-update.sh
bash /tmp/tihulu-update.sh --cosmic
```

Force GNOME/Ubuntu:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/update-from-github.sh -o /tmp/tihulu-update.sh
bash /tmp/tihulu-update.sh --gnome
```

## Pop!_OS COSMIC Wayland notes

The COSMIC applet does **not** use screencopy, thumbnails, PipeWire, MPRIS, live window previews, or background screenshot capture. Clipboard access is currently done through `wl-paste` and `wl-copy`.

The text clipboard watcher and image clipboard watcher are separated. Text polling is lightweight and slow by design. Image polling is slower and only stores supported image payloads when **Image History** is enabled.

COSMIC may start one applet process per panel/output on multi-monitor systems. This is supported. Encrypted history storage is process-safe: writes use a short storage lock plus atomic temp-file replacement, and the encryption key is a local `0600` file so multiple panel processes do not race through Secret Service/keyring.

For panel-sensitive systems, enable **Safe Core** in the applet settings. Safe Core keeps encryption enforced, disables image preview decoding, forces image size limiting, and reduces effective history limits.

## Safe Core mode

Safe Core is a low-resource stability mode for COSMIC panels.

When Safe Core is enabled:

- encrypted history remains enforced
- image history can still be stored if Image History is enabled
- image previews are not decoded/rendered in the popup
- image size limiting is forced on
- effective maximum history entries are clamped to 50
- effective maximum age is clamped to 7 days
- effective maximum text item size is clamped to 64 KiB
- effective maximum image item size is clamped to 5 MiB

Text history continues to work.

## Where COSMIC history is stored

COSMIC history is stored locally under:

```text
~/.local/share/tihulu-clipboard-manager/
```

Encrypted history is always enabled and is stored at:

```text
~/.local/share/tihulu-clipboard-manager/history.enc.json
```

The primary COSMIC encryption key is stored locally at:

```text
~/.local/share/tihulu-clipboard-manager/history.key
```

Expected permissions:

```text
700 ~/.local/share/tihulu-clipboard-manager
600 ~/.local/share/tihulu-clipboard-manager/history.key
600 ~/.local/share/tihulu-clipboard-manager/history.enc.json
```

Do **not** share `history.key`. If `history.key` is deleted, the existing `history.enc.json` file cannot be decrypted anymore. If you back up encrypted history, back up `history.key` and `history.enc.json` together.

The OS keyring is used only as a best-effort mirror/fallback for the local encryption key:

```text
service: io.github.tihulu.ClipboardManager
user: history-encryption-key-v1
```

The old plain history path is only kept as a legacy migration fallback and is removed after a successful encrypted save:

```text
~/.local/share/tihulu-clipboard-manager/history.json
```

Image entries are not stored in a separate cache directory. If image history is enabled, image payloads are stored as base64 inside the same encrypted history file and are pruned by the same maximum-entry and maximum-age rules.

## COSMIC encryption reset / repair

Use this only when you intentionally want to erase local clipboard history and create a fresh encrypted store:

```bash
pkill -f tihulu-clipboard-manager || true
rm -rf ~/.local/share/tihulu-clipboard-manager/history.lock
rm -f ~/.local/share/tihulu-clipboard-manager/history.enc.json ~/.local/share/tihulu-clipboard-manager/history.json ~/.local/share/tihulu-clipboard-manager/history.key
```

Then reinstall/update from the stable release and let the applet create a fresh encrypted store:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/v0.2.0/scripts/quick-install.sh | BRANCH=v0.2.0 bash
pkill -f tihulu-clipboard-manager || true
```

After COSMIC restarts the applet, the panel popup should show:

```text
Encryption: On
```

To verify permissions:

```bash
stat -c '%a %n' ~/.local/share/tihulu-clipboard-manager ~/.local/share/tihulu-clipboard-manager/history.key ~/.local/share/tihulu-clipboard-manager/history.enc.json
```

Expected output pattern:

```text
700 .../tihulu-clipboard-manager
600 .../history.key
600 .../history.enc.json
```

## FD/resource stability checks

For long-running COSMIC testing, watch the applet, panel, and compositor together:

```bash
bash scripts/fd-monitor.sh
```

Or manually:

```bash
for name in tihulu-clipboard-manager cosmic-panel cosmic-comp; do
  for pid in $(pgrep -x "$name" 2>/dev/null); do
    printf '%s pid=%s fd=%s\n' "$name" "$pid" "$(ls /proc/$pid/fd 2>/dev/null | wc -l)"
    grep -E 'VmRSS|VmSwap|Threads' /proc/$pid/status 2>/dev/null || true
  done
done
```

The FD count may move briefly during clipboard operations, but it should return close to baseline. A monotonic increase such as `80 -> 81 -> 82 -> 83` after repeated operations should be treated as a leak.

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
Alt + F2 -> r -> Enter
```

On GNOME Wayland, log out and log back in.

GNOME docs:

- [`gnome/README.md`](gnome/README.md)
- [`gnome/RELEASE_NOTES.md`](gnome/RELEASE_NOTES.md)
- [`gnome/CHECKS.md`](gnome/CHECKS.md)

## Features

- Text clipboard history
- Image clipboard history for PNG, JPEG, WebP, and GIF payloads
- Click-to-copy old entries
- Image click-to-copy while preserving MIME type
- Visible **Clear All / Erase All** controls
- Confirmation before destructive history deletion
- Encrypted history enforced at rest
- `ChaCha20Poly1305` authenticated encryption for history storage
- Local `0600` encryption key with best-effort OS keyring mirror
- Process-safe atomic encrypted history writes for multi-monitor COSMIC panels
- Private mode to stop storing new clipboard items
- Unique session mode
- Safe Core low-resource mode
- Maximum history size
- Maximum history age
- Sensitive-content filter for common secrets
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

By default, image history is limited to 25 MiB per entry. Safe Core reduces the effective image limit to 5 MiB and disables popup preview decoding.

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

Full local check:

```bash
bash scripts/check-all.sh
```

## Current status

The COSMIC applet has been runtime-tested on Pop!_OS 24.04 COSMIC Wayland with multiple panel applet processes on a multi-monitor setup. Encrypted history remains `Encryption: On` with local key permissions verified as `0600`.

Before release tagging, verify the target build with:

```bash
bash scripts/check-all.sh
```

## Security notes

Clipboard managers are sensitive software. Treat this applet like a password-adjacent tool. Encryption is enforced for COSMIC history storage, but clipboard contents are still visible to local processes that can access the active session clipboard.

Do not share `history.key`, screenshots containing secrets, or the contents of `history.enc.json` together with the matching key.
