# Tihulu Clipboard Manager for GNOME

This folder contains the GNOME Shell version of Tihulu Clipboard Manager.

The GNOME version uses two parts:

- `gnome/extension-native/`: GNOME Shell panel UI and preferences window
- `gnome/native-helper/`: Rust helper for storage, encryption, image clipboard, keyring, and clipboard I/O

This keeps the GNOME Shell extension small while the native helper handles the parts that need stronger local storage and clipboard support.

## Implemented features

- GNOME top panel indicator
- GNOME preferences window
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
- Native keyring-backed encryption key management
- Sensitive text filter
- Max entry count pruning
- Max age pruning
- Max text byte limit
- Image history toggle
- Image size limit toggle
- Max image byte limit
- Local config under `~/.local/share/tihulu-clipboard-manager-gnome/config.json`
- Encrypted history under `~/.local/share/tihulu-clipboard-manager-gnome/history.enc.json`

## Required packages

Ubuntu / Debian:

```bash
sudo apt update
sudo apt install -y git curl cargo gnome-shell-extensions wl-clipboard pkg-config libssl-dev build-essential
```

Fedora:

```bash
sudo dnf install -y git curl cargo gnome-extensions-app wl-clipboard openssl-devel pkgconf-pkg-config gcc
```

Arch:

```bash
sudo pacman -S --needed git curl rust gnome-shell-extensions wl-clipboard pkgconf openssl base-devel
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

Enable it if needed:

```bash
gnome-extensions enable tihulu-clipboard-manager@tihulu.dev
```

If it does not appear immediately, log out and log back in. On Xorg, you can also press `Alt+F2`, type `r`, and press Enter.

## Manual install from GitHub clone

```bash
git clone https://github.com/Tihulu/tihulu-clipboard-manager.git
cd tihulu-clipboard-manager
cargo build --release --manifest-path gnome/native-helper/Cargo.toml
mkdir -p ~/.local/bin
cp gnome/native-helper/target/release/tihulu-gnome-clipboard-helper ~/.local/bin/
chmod 0755 ~/.local/bin/tihulu-gnome-clipboard-helper
mkdir -p ~/.local/share/gnome-shell/extensions
rm -rf ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
cp -R gnome/extension-native ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
gnome-extensions enable tihulu-clipboard-manager@tihulu.dev
```

Then log out and log back in if GNOME Shell does not load the extension immediately.

## Development install

```bash
git clone https://github.com/Tihulu/tihulu-clipboard-manager.git
cd tihulu-clipboard-manager
cargo build --release --manifest-path gnome/native-helper/Cargo.toml
mkdir -p ~/.local/bin ~/.local/share/gnome-shell/extensions
ln -sf "$PWD/gnome/native-helper/target/release/tihulu-gnome-clipboard-helper" ~/.local/bin/tihulu-gnome-clipboard-helper
rm -rf ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
ln -s "$PWD/gnome/extension-native" ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
gnome-extensions enable tihulu-clipboard-manager@tihulu.dev
```

After editing `extension.js` or `prefs.js`, reload GNOME Shell or log out and back in.

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

## Uninstall

```bash
gnome-extensions disable tihulu-clipboard-manager@tihulu.dev || true
rm -rf ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
rm -f ~/.local/bin/tihulu-gnome-clipboard-helper
rm -rf ~/.local/share/tihulu-clipboard-manager-gnome
```

## Debug logs

```bash
journalctl --user -f /usr/bin/gnome-shell
```

```bash
gnome-extensions list | grep tihulu
```
