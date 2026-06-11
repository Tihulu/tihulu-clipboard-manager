# Security Policy

## Current security status

This applet is **not ready for daily use yet**. It is an early scaffold and does not yet include the real Wayland clipboard watcher or click-to-copy implementation.

## Clipboard data sensitivity

Clipboard managers are sensitive software. Clipboard history can contain passwords, recovery codes, API tokens, personal messages, addresses, commands, and other private data.

Current mitigations:

- The history file is stored under the user's XDG data directory.
- On Unix systems, the history directory is forced to `0700`.
- On Unix systems, the history file is forced to `0600`.
- `Clear All / Erase All` is visible in the main popup.
- Clear All requires confirmation by default.
- History length is bounded by configuration.

Current gaps:

- History is stored as plaintext JSON.
- Sensitive-content detection is not implemented yet.
- Per-application ignore rules are not implemented yet.
- Clipboard capture over Wayland data-control is not implemented yet.
- Click-to-copy back to the clipboard is not implemented yet.

## Safe testing guidance

Until privacy filters and real clipboard integration are reviewed, test with non-sensitive text only.

Do not copy the following while testing development builds:

- Passwords
- One-time codes
- API keys
- SSH keys
- Recovery phrases
- Personal documents

## Reporting vulnerabilities

Open a private security advisory on GitHub if possible, or contact the maintainer directly before publishing details.
