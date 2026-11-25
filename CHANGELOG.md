# Changelog

## Unreleased

- Persisted the selected game-save folder in the app's local SQLite settings, with one-time migration from the old WebView setting.
- Added classic LoD skill trees and skill/attribute editing with class starting values, skill-level gates, prerequisites, point conservation, respec-all, pre-write backups, validated in-place saves for version 96 characters, and a local Features toggle enabled by default.
- Changed character gold controls to inline editable carried/stash values with automatic caps and per-value Max out buttons.
- Changed gold editing to update the selected save in place after creating a verified versioned backup; no extra character file is created.
- Changed bank transfers to update the selected character file in place after creating a verified pre-change backup.
- Added separate carried (belt/inventory) and personal stash gold editing for classic LoD version 96 saves, with level/stash caps and a review prompt.
- Added default-on, local versioned character-save backups with 30-second background change detection. The first scan records a baseline without creating a version; the first observation only records a baseline, and a version is created only after contents change. Backup history shows the latest five first, with an option to expand.
- Added confirmed backup restore; the current save is preserved and the restored save is validated.

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
