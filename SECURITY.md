# Security Policy

## Current security status

This applet is still under development and should be tested carefully before daily use. The security storage layer is now designed to be stronger than a plaintext clipboard history, and the first `wl-clipboard`-based text/image watcher plus click-to-copy path is implemented. The code still needs compile, runtime, and security review on the target COSMIC system.

## Clipboard data sensitivity

Clipboard managers are sensitive software. Clipboard history can contain passwords, recovery codes, API tokens, personal messages, addresses, commands, screenshots, photos, and other private data.

Current mitigations:

- History encryption is enabled by default.
- Encrypted history uses `ChaCha20Poly1305` authenticated encryption.
- The encryption key is generated randomly and stored in the OS keyring.
- The plaintext history file is removed when encryption mode is enabled or toggled.
- The history file is stored under the user's XDG data directory.
- On Unix systems, the history directory is forced to `0700`.
- On Unix systems, the history file is forced to `0600`.
- `Clear All / Erase All` is visible in the main popup.
- Clear All requires confirmation by default.
- Clear All removes both plaintext and encrypted persisted history files before re-saving an empty encrypted store.
- History length is bounded by configuration.
- History age is bounded by configuration.
- Private mode prevents newly captured clipboard items from being stored.
- Unique session mode clears persisted history at applet startup.
- Sensitive-content filtering is enabled by default for common passwords, API keys, private keys, tokens, OTPs, and recovery phrase patterns.
- Oversized text entries are skipped by default.
- Image clipboard storage is limited to PNG, JPEG, WebP, and GIF MIME types.
- Image clipboard payloads are limited to 25 MiB by default.
- Image clipboard entries are encrypted at rest with the same history encryption layer.

Current gaps:

- Native Wayland data-control clipboard capture is not implemented yet; the current backend uses `wl-paste` / `wl-copy`.
- Per-application ignore rules are not implemented yet.
- The sensitive-content filter is heuristic and can have both false positives and false negatives.
- Image payloads can contain sensitive visual data such as screenshots, QR codes, documents, and photos.
- The OS keyring must be available; if it is unavailable, encrypted history load/save will fail rather than silently falling back to plaintext.

## Safe testing guidance

Until real COSMIC runtime testing is complete, test with non-sensitive text and non-sensitive images only.

Do not copy the following while testing development builds:

- Passwords
- One-time codes
- API keys
- SSH keys
- Recovery phrases
- Personal documents
- Private screenshots
- QR codes containing secrets

## Reporting vulnerabilities

Open a private security advisory on GitHub if possible, or contact the maintainer directly before publishing details.
