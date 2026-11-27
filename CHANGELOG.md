# Changelog

## Unreleased

- Locked character and stash file writes while Diablo II or D2R is running, with a visible read-only state and backend process checks; the game does not reliably expose which character is loaded, so the guard covers all saves.
- Expanded the README into a complete shipped-feature inventory and added an employer-facing engineering case study grounded in the repository's safeguards, quality harness, and Git history.
- Added a manual Windows/Linux installer validation workflow that uploads short-lived artifacts without publishing a release.
- Added a release command that updates version metadata, runs project quality gates, and commits, tags, and pushes a release; GitHub installer builds now wait for the same reusable quality workflow.
- Fixed bank searches to match terms at word boundaries, preventing `Mage` from matching `Damage`.
- Added a searchable affix filter to the bank and made affix text searchable in the main query, including short prefixes.
- Added a timed explainer video script with voiceover, shot directions, and footage accuracy safeguards.
- Fixed concatenated item stat labels across bank and character item details, including older stored records; skill bonuses resolve numeric layers to skill names.
- Fixed character-to-bank source keys for identical seed-zero items; batch deposits now skip only repeated nonzero fingerprints and move the rest, and the bank has a review utility for removing selected duplicate records.
- Corrected the vertical orientation of all classic item sprites and their DC6 conversion path; character-to-bank deposits now refresh the save data while preserving the current character subtab.
- Added hover and keyboard-focus detail cards to character inventory and private-stash items, including parsed quality, status, item level, socket contents, and decoded item, set, and runeword stats.
- Ordered character tabs as Overview, Skills & stats, Items, and Backups.
- Replaced one-point attribute plus/minus clicking with direct target-value fields, capped by the character's earned stat points.
- Changed bank search to update on each keystroke and match short terms at item-name/code word boundaries; added regression coverage for `tir`, `tal`, `eth`, item codes, and the “Crystal Sword” false match.
- Added dedicated character tabs for Overview, Items, Skills & stats, and Backups; the character view now uses more of the available width and keeps skill editing and backup history out of the overview.
- Reworked the global header into separate responsive navigation and page-action rows, and kept skill and attribute summaries stacked until there is enough room for two columns.
- Added 470 transparent classic inventory sprite PNGs with a version 96 item-code mapping. Bank cards, character inventory/stash boxes, and equipped-item rows now show the original item art. Documented game-art ownership and conversion inputs in `docs/ITEM-ICONS.md`.
- Kept storage grids close to the source sprites' native 28px cell size instead of stretching them across the full pane; removed forced pixelated enlargement.
- Added character inventory and private stash grids with per-item selection and selective moves into the shared bank. Deposits create a verified versioned backup, remove only selected records from the original save in place, validate the result, and check nonzero item seeds against the bank to avoid duplicate transfers.
- Made available bank items the default view and marked transferred history records as unavailable.
- Hardened the Tauri WebView with a restrictive production CSP, a separate localhost-only Vite development CSP, and prototype freezing. Removed unused Tauri SQL/log plugins and the unneeded SQL frontend permission. Upgraded rusqlite to 0.41.1 to resolve the bundled SQLite memory-disclosure advisory; documented the full security review and remaining upstream/release hardening items in `docs/SECURITY-REVIEW.md`.
- Refactored Rust persistence and bank transfer commands out of the Tauri entry module, extracted grid placement into a dedicated module, and removed unused parser fields/helpers.
- Extracted bank item metadata, attributes, compatibility, filtering, and gold validation from the main React component into tested pure functions; metadata is parsed once per bank refresh and reused by filters and cards.
- Added frontend tests for bank item presentation, compatibility, filtering, and gold caps plus Rust unit coverage for item bit parsing, labels/classification, grid placement boundaries, setting validation, and idempotent DB setup. CI now runs frontend/Rust tests, formatting, Clippy with warnings denied, clone detection, and reports function complexity.
- Added `docs/ENGINEERING-STANDARDS.md` to record the quality gates, baseline metrics, coverage limits, and remaining high-risk test gaps.
- Added all 210 classic skill tree icons, mapped from game data, to the Character Lab and documented the Blizzard artwork rights separately from the project license.
- Fixed the icon palette and asset output path so the bundled skill icons render with their original colors instead of corrupted colors.
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
