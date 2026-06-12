# Changelog

## v0.1.0 - Initial preview release

This is the first public preview release of Tihulu Clipboard Manager, a security-first COSMIC panel clipboard manager applet.

### Added

- COSMIC panel applet scaffold.
- Local clipboard history for text entries.
- Image clipboard history for PNG, JPEG, WebP, and GIF.
- Larger image previews in the history popup.
- Local search across clipboard history.
- Click-to-copy for text and image entries.
- Pin, unpin, delete, Clear Unpinned, and Clear All actions.
- Confirmation before destructive Clear All.
- Encrypted local history by default.
- OS keyring-backed encryption key.
- Incognito mode to stop storing new copied content.
- Unique Session mode to clear persisted history on applet startup.
- Image size limit toggle.
- Sensitive content filter for common secret patterns.
- Quick install script for Pop!_OS / COSMIC systems.

### Security notes

- History is local-only; there is no cloud sync, telemetry, or external search.
- Search runs only on local in-memory history entries.
- Image previews are size-limited to reduce memory pressure.
- Image MIME types are allowlisted.
- Encrypted history nonce length is validated before decrypting.

### Install

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/quick-install.sh | bash
```

After installing, log out and log back in, then add **Tihulu Clipboard Manager** from COSMIC panel settings.
