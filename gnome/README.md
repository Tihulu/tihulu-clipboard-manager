# Tihulu Clipboard Manager for GNOME

This folder contains a separate GNOME Shell extension implementation for Tihulu Clipboard Manager.

It is independent from the COSMIC applet in the repository root. The GNOME version is written as a GNOME Shell JavaScript extension and is designed to match the COSMIC applet workflow as closely as possible on GNOME Shell.

## Implemented features

- GNOME top panel indicator
- Local copied text history
- Click `Copy` to copy an old entry back to the clipboard
- Search clipboard history
- Pin and unpin entries
- Delete individual entries
- Clear unpinned entries
- Erase all entries with confirmation
- Private mode toggle that temporarily stops saving new clipboard text
- Unique session toggle that clears history when enabled
- Sensitive text filter for common secrets such as private keys, API keys, tokens, and password-like assignments
- Max entry count pruning
- Max age pruning
- Max text byte limit
- Local config file under `~/.local/share/tihulu-clipboard-manager-gnome/config.json`
- Local history file under `~/.local/share/tihulu-clipboard-manager-gnome/history.json`

## Not yet equal to the COSMIC version

The COSMIC version has native Rust storage and can support stronger security features directly. GNOME Shell extensions run inside GNOME Shell JavaScript, so full parity needs a native helper process.

Still missing from the GNOME version:

- Encrypted history storage
- Image clipboard history
- Image size limit controls
- GNOME preferences window
- Native keyring-backed encryption key management

The current GNOME extension is now feature-aligned for text history and privacy workflow, but not yet fully security-equivalent to the COSMIC Rust applet.

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

## Safety notes

The GNOME version currently stores copied text history in a local JSON file. It has a sensitive-text filter and private mode, but it does not yet encrypt the history file. Do not use it with real passwords, access tokens, API keys, private keys, or private secrets until the native helper with encryption is added.

## Next parity step

To reach full COSMIC parity, add a native helper under `gnome/native-helper/` that handles encrypted storage, image clipboard reading/writing, keyring integration, and security tests. The GNOME Shell extension should then become only the panel UI and call the helper for storage and clipboard operations.
