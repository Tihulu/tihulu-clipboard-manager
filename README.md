# Tihulu Clipboard Manager

A COSMIC panel clipboard manager applet for Pop!_OS / COSMIC.

The first design goal is simple: **Erase All must be visible in the main popup** so clipboard history does not accumulate silently.

## Planned features

- Visible **Clear All / Erase All** button in the main popup
- Confirmation before destructive history deletion
- Clear unpinned items while keeping pinned entries
- Delete individual entries
- Pin / unpin entries
- Search history
- Text clipboard history first
- Image clipboard support later
- Privacy filters for passwords, tokens, and ignored apps

## Current status

This is the initial applet scaffold. The UI and local history model are started, including the Clear All flow.

Still needed:

- Connect the real Wayland data-control clipboard watcher
- Implement click-to-copy by setting the Wayland clipboard
- Test against the current COSMIC/libcosmic API on Pop!_OS

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

## Create the GitHub repository

The expected repository is:

```text
https://github.com/Tihulu/tihulu-clipboard-manager
```

Create it with GitHub CLI:

```bash
gh repo create Tihulu/tihulu-clipboard-manager \
  --public \
  --description "COSMIC panel clipboard manager with visible erase-all controls" \
  --source . \
  --remote origin \
  --push
```

## App identity

- Applet name: **Tihulu Clipboard Manager**
- Binary name: `tihulu-clipboard-manager`
- App ID: `io.github.tihulu.ClipboardManager`
- License: GPL-3.0-or-later
