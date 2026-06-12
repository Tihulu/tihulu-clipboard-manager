# Tihulu Clipboard Manager for GNOME

This folder contains a separate GNOME Shell extension prototype for Tihulu Clipboard Manager.

It is independent from the COSMIC applet in the repository root. The GNOME version is written as a GNOME Shell JavaScript extension and currently focuses on copied text history.

## Features

- GNOME top panel indicator
- Local copied text history
- Click an entry to copy it back to the clipboard
- Search clipboard history
- Clear history
- Private mode toggle that temporarily stops saving new clipboard text
- Local JSON history file under `~/.local/share/tihulu-clipboard-manager-gnome/history.json`

## Current limitations

- Text clipboard history only
- No image clipboard history yet
- No encryption yet
- No GNOME preferences window yet
- Tested as an initial GNOME Shell extension skeleton; GNOME Shell API compatibility should be verified on your target GNOME version

## Quick install from GitHub

Run this on the GNOME machine:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/gnome/scripts/quick-install.sh | bash
```

Then enable it if it is not enabled automatically:

```bash
gnome-extensions enable tihulu-clipboard-manager@tihulu.dev
```

If it does not appear immediately, log out and log back in. On Xorg, you can also press `Alt+F2`, type `r`, and press Enter.

## Manual install from GitHub clone

```bash
git clone https://github.com/Tihulu/tihulu-clipboard-manager.git
cd tihulu-clipboard-manager
mkdir -p ~/.local/share/gnome-shell/extensions
rm -rf ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
cp -R gnome/extension ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
gnome-extensions enable tihulu-clipboard-manager@tihulu.dev
```

Then log out and log back in if GNOME Shell does not load the extension immediately.

## Development install

For development, symlink the extension folder instead of copying it:

```bash
git clone https://github.com/Tihulu/tihulu-clipboard-manager.git
cd tihulu-clipboard-manager
mkdir -p ~/.local/share/gnome-shell/extensions
rm -rf ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
ln -s "$PWD/gnome/extension" ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
gnome-extensions enable tihulu-clipboard-manager@tihulu.dev
```

After editing `extension.js`, reload GNOME Shell or log out and back in.

## Uninstall

```bash
gnome-extensions disable tihulu-clipboard-manager@tihulu.dev || true
rm -rf ~/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev
rm -rf ~/.local/share/tihulu-clipboard-manager-gnome
```

## Debug logs

```bash
journalctl --user -f /usr/bin/gnome-shell
```

You can also inspect the installed extension list:

```bash
gnome-extensions list | grep tihulu
```

## Security notes

The current GNOME prototype stores copied text history in a local JSON file. Do not use it with real passwords, access tokens, API keys, or private secrets until encryption and sensitive-content filtering are added.

The COSMIC version in the repository root has a richer privacy/security model; the GNOME version starts as a separate clean-room implementation and should be hardened step by step.
