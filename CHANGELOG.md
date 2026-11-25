# Changelog

## Unreleased

- Added default-on, local versioned character-save backups with 30-second background change detection, history, and confirmed restore. Restores preserve and validate the current save first.

## 0.1.0 — Initial desktop release

- Added an offline character library and character overview for original Diablo II / Lord of Destruction saves.
- Added a local item bank with search, combined filters, persistent selection, and bulk transfers.
- Added supported version 96 `.d2x` stash import with byte-verified backup creation.
- Added `_d2a` character-save exports, prior-output backups, and save validation; original character files remain untouched.
- Added known class/level compatibility checks and explicit unknown states.
- Added offline runeword recipe browsing with ordered runes, base types, sockets, effects, and game availability labels.
- Added a local feature-set preference that hides D2R/Reign recipe references by default; this does not enable D2R save support.
- Added contribution guidance welcoming high-quality human- and agent-authored patches, plus GitHub Discussions voting for feature proposals.
- Added native macOS, Windows, and Linux build/release automation, the red-and-steel logo/icon set, and a Diablo-inspired ember palette. Installers are unsigned.
- Added an opt-in Features screen for extended recipe rulesets.

## Development history

See the repository history for earlier implementation work. Private save files, machine paths, and internal handoff notes are intentionally excluded from the public repository.
