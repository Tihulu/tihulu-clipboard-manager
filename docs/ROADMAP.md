# Roadmap

## 0.1.0 security scaffold

- [x] COSMIC applet package skeleton
- [x] Main popup layout
- [x] Visible Clear All / Erase All button
- [x] Confirmation state for destructive erase
- [x] Pin, unpin, delete, clear unpinned actions
- [x] Local encrypted history model for early UI testing
- [x] Unix history directory/file permission hardening
- [x] Private mode config
- [x] Unique session config
- [x] Max entries pruning
- [x] Max age pruning
- [x] Sensitive-content filter
- [x] Encrypted history using OS keyring-backed random key

## 0.2.0 real clipboard integration

- [ ] Connect Wayland data-control clipboard watcher
- [ ] Store text clipboard entries from real clipboard events
- [ ] Implement click-to-copy by setting clipboard content
- [ ] Deduplicate entries reliably
- [ ] Add clear-system-clipboard action
- [ ] Re-run security review on real clipboard integration

## 0.3.0 usability

- [ ] Search box
- [ ] Keyboard navigation
- [ ] Better row styling and timestamps
- [ ] Settings page for private mode, encryption, max entries, max age, and confirmation behavior

## 0.4.0 privacy

- [ ] Per-application ignore rules if COSMIC/Wayland exposes source app metadata
- [ ] Auto-clear timer
- [ ] Optional passphrase-derived encryption key mode
- [ ] Migration path from plaintext history to encrypted history

## Later

- [ ] Image clipboard support with separate size limits
- [ ] Import/export encrypted backups
- [ ] Packaging for Pop!_OS/COSMIC
