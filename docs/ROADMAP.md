# Roadmap

## 0.1.0 scaffold

- [x] COSMIC applet package skeleton
- [x] Main popup layout
- [x] Visible Clear All / Erase All button
- [x] Confirmation state for destructive erase
- [x] Pin, unpin, delete, clear unpinned actions
- [x] Local JSON history model for early UI testing

## 0.2.0 real clipboard integration

- [ ] Connect Wayland data-control clipboard watcher
- [ ] Store text clipboard entries from real clipboard events
- [ ] Implement click-to-copy by setting clipboard content
- [ ] Deduplicate entries reliably
- [ ] Enforce max history size while preserving pinned entries

## 0.3.0 usability

- [ ] Search box
- [ ] Keyboard navigation
- [ ] Better row styling and timestamps
- [ ] Settings page for max entries and confirmation behavior

## 0.4.0 privacy

- [ ] Optional sensitive-content filters
- [ ] Auto-clear timer
- [ ] Clear system clipboard option
- [ ] App blacklist if COSMIC/Wayland exposes source app metadata

## Later

- [ ] Image clipboard support
- [ ] Import/export
- [ ] Packaging for Pop!_OS/COSMIC
