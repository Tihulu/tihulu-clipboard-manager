## v0.2.1 - Keyring-only encryption hardening

- Store the COSMIC applet ChaCha20Poly1305 history key only in the OS keyring.
- Migrate legacy COSMIC `history.key` material into the keyring and remove the local key only after a verified keyring round trip.
- Fail closed when Secret Service/keyring access is unavailable or locked.
- Never generate a replacement key when encrypted history already exists but the key is missing.
- Lock persistence for the current applet session after encrypted-history load failure to prevent accidental overwrite after transient keyring recovery.
- Refuse to delete plaintext history until the encrypted history can be read successfully.
- Serialize key initialization and migration across concurrent COSMIC panel processes.

# Changelog

## v0.2.0 - Stable COSMIC encryption release

This release stabilizes the COSMIC/Wayland applet for Pop!_OS 24.04 COSMIC, especially on multi-monitor setups where COSMIC may start one applet process per panel/output.

### Fixed

- Prevented encrypted history corruption from concurrent panel applet processes.
- Replaced direct encrypted-history truncation with atomic temp-file writes.
- Added a short storage lock around encrypted history save/delete/reset operations.
- Fixed encrypted history reset so it creates a fresh key and a fresh encrypted store.
- Avoided relying on Secret Service/keyring as the only source of the encryption key.

### Changed

- COSMIC encrypted history now uses a local `0600` key file at:

```text
~/.local/share/tihulu-clipboard-manager/history.key
```

- The OS keyring is now only a best-effort mirror/fallback for the local key.
- Safe Core keeps encrypted history enforced and disables popup image preview decoding while still allowing image history storage when Image History is enabled.
- Local full checks now run formatting before check/test/clippy/audit to avoid local format-only failures.

### Verified runtime behavior

- `Encryption: On` verified after encrypted reset.
- Multi-process COSMIC panel state verified with three `tihulu-clipboard-manager` processes.
- Verified permissions:

```text
700 ~/.local/share/tihulu-clipboard-manager
600 ~/.local/share/tihulu-clipboard-manager/history.key
600 ~/.local/share/tihulu-clipboard-manager/history.enc.json
```

### Install

```bash
curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/quick-install.sh | bash
```

After installing, log out and log back in, then add **Tihulu Clipboard Manager** from COSMIC panel settings if it is not already present.

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
