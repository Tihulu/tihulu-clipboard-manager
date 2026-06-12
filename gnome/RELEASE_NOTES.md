# Release notes — Tihulu GNOME / Ubuntu v2.3.0

## Highlights

- New daemon-style GNOME architecture using a `systemd --user` service.
- GNOME Shell extension no longer reads clipboard synchronously.
- Text history works automatically.
- Image history works automatically in the background service.
- Encrypted persistent history for text and image payloads.
- Persistent plaintext preview cache is removed by the installer.
- X11 screenshot freeze risk is reduced by moving clipboard work out of GNOME Shell.
- Clipboard reads are service-isolated and timeout-bound by the service wrapper.

## Known limitations

- This release uses the existing native helper command in a user service loop; the next native helper revision can expose a dedicated `daemon` command.
- Large image previews require helper support for `imagePreviewUri`; until that helper lands, image entries may appear without large thumbnails.
- Very large images above the configured image byte limit are skipped after being read by the current helper.
- On Wayland, some apps may expose clipboard images differently depending on portal/backend behavior.
- GNOME Shell must be reloaded/logged out after extension source changes.

## Upgrade notes

Run:

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/gnome/scripts/quick-install.sh | bash
```

For a clean history test:

```bash
RESET_HISTORY=1 bash gnome/scripts/quick-install.sh
```

Old persistent plaintext preview cache under:

```text
~/.local/share/tihulu-clipboard-manager-gnome/previews
```

is removed automatically.
