# Tihulu Clipboard Manager — GNOME / Ubuntu

This folder contains the GNOME Shell version of Tihulu Clipboard Manager.

The GNOME version uses three parts:

- `gnome/extension-native/`: GNOME Shell panel UI
- `gnome/native-helper/`: Rust helper for encrypted storage and clipboard I/O
- `gnome/systemd/`: `systemd --user` service that keeps capture outside GNOME Shell

The main safety goal is that GNOME Shell does not synchronously read heavy clipboard contents. Clipboard capture runs outside the Shell process.

## Features

- GNOME top panel indicator
- Local copied text history
- Image clipboard history for PNG, JPEG, WebP, and GIF
- Copy old text or image entries back to the clipboard
- Search clipboard history
- Pin and unpin entries
- Delete individual entries
- Clear unpinned entries
- Erase all entries with confirmation
- Private mode toggle
- Unique session toggle
- Encrypted history storage by default
- Sensitive text filter
- Max entry count pruning
- Max age pruning
- Max text byte limit
- Image history toggle
- Image size limit toggle
- Systemd user service for background clipboard capture

## Data paths

```text
~/.local/share/tihulu-clipboard-manager-gnome/config.json
~/.local/share/tihulu-clipboard-manager-gnome/history.enc.json
~/.local/share/tihulu-clipboard-manager-gnome/history.json
```

When encrypted history is enabled, text and image payloads are stored in `history.enc.json`.

## Ubuntu / Pop!_OS dependencies

```bash
sudo apt update
sudo apt install -y git cargo gnome-shell-extensions xclip wl-clipboard coreutils build-essential pkg-config libssl-dev
```

## Quick install from GitHub

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/gnome/scripts/quick-install.sh | bash
```

The script builds and installs the helper to:

```text
~/.local/bin/tihulu-gnome-clipboard-helper
```

It installs the GNOME extension to:

```text
~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
```

It installs and starts the user service:

```text
~/.config/systemd/user/tihulu-gnome-clipboard-daemon.service
```

On Xorg, reload GNOME Shell after installing:

```text
Alt + F2 → r → Enter
```

On Wayland, log out and log back in.

## Clean install / reset history

This backs up the old data directory instead of deleting it:

```bash
RESET_HISTORY=1 bash gnome/scripts/quick-install.sh
```

## Service status

```bash
systemctl --user status tihulu-gnome-clipboard-daemon.service --no-pager
```

Logs:

```bash
journalctl --user -u tihulu-gnome-clipboard-daemon.service -f
```

## Helper commands

```bash
tihulu-gnome-clipboard-helper state
tihulu-gnome-clipboard-helper capture
tihulu-gnome-clipboard-helper copy <entry-id>
tihulu-gnome-clipboard-helper toggle-pin <entry-id>
tihulu-gnome-clipboard-helper delete <entry-id>
tihulu-gnome-clipboard-helper clear-unpinned
tihulu-gnome-clipboard-helper clear-all
tihulu-gnome-clipboard-helper set privateMode true
tihulu-gnome-clipboard-helper set encryptHistory true
tihulu-gnome-clipboard-helper set imageClipboard true
tihulu-gnome-clipboard-helper set limitImageSize true
```

## Verification

See [`CHECKS.md`](CHECKS.md).

## Release packaging

The workflow `.github/workflows/gnome-ubuntu-release.yml` creates a GNOME/Ubuntu zip package for tags matching:

```text
gnome-v*
```

## Uninstall

```bash
bash gnome/scripts/uninstall.sh
```

Remove history too:

```bash
REMOVE_HISTORY=1 bash gnome/scripts/uninstall.sh
```

## Known notes

The current service runs the existing helper `capture` command in a supervised user-service loop. The next helper revision can expose a dedicated native `daemon` command and runtime-only image preview URIs.
