# D2 Assistant — Product Requirements

## Product

D2 Assistant is a local desktop item bank and transfer helper for offline characters in original Diablo II and Lord of Destruction. It prioritizes searchable bulk item management and safe save-file handling.

## Supported scope

- Original Diablo II / Lord of Destruction offline character saves.
- Version 96 `.d2x` stash item records for import.
- Local bank storage; no account, cloud sync, or game-server connection.
- D2R and Reign of the Warlock runeword data is optional reference content only. D2R save parsing and transfer are out of scope.
- No online characters, Battle.net integration, botting, or in-game automation.
- Android is a future feasibility item, not a supported platform.

## Core release features

1. Scan a user-selected local save folder and show character details.
2. Persist the selected save-folder path in app-local settings on disk and restore it at startup; a scan failure must not clear the saved path.
3. Back up and import items from supported stashes into a searchable local bank.
4. Filter by item type, rarity, class/level requirements, sockets, and transfer status.
5. Keep multi-selection while refining filters and transfer selected items to a chosen character inventory or personal stash.
6. Show each character's 10×4 carried inventory and 6×8 private stash as item grids; let users select individual item boxes and move only those items into the shared bank.
7. Back up the selected character save before any edit or item transfer, update that save in place, and validate the result to avoid generating duplicate character files.
8. Browse offline runeword recipes with ordered runes, sockets, valid bases, effects, and availability labels.
9. Let users opt into or out of optional feature sets locally through the Features screen. Extended D2R/Reign recipe references are off by default; safety behavior cannot be disabled.

## Safety requirements

- Show the selected source path and explain that a full stash import creates a verified backup and empties the selected stash after import.
- Character-to-bank deposits are selective: show inventory/stash grids, move only explicitly selected items, create a verified versioned backup, validate the updated save, and update the character in place.
- Gold edits and bank item transfers update the selected character save in place only after a verified backup. Restoring a backup requires confirmation and creates a verified pre-restore backup.
- Back up an existing app-generated file before replacing it.
- Validate generated saves before reporting success.
- Clearly show unknown item requirements instead of guessing.
- Never claim offline item ownership or uniqueness is verified. A user can restore or copy a previous save outside the app.

## Feature proposals and contributions

- Use GitHub Discussions for feature ideas and voting. Votes guide prioritization; the maintainer decides based on quality, safety, game compatibility, and maintenance cost.
- Welcome contributions from people and coding agents under the same quality bar. See `CONTRIBUTING.md`.
- Optional features are registered as local feature flags with a description, default state, and compatibility notes.

## Character backup history and restore

- Automatic versioned character-save backups default on.
- On app start, character-folder refresh, and every 30 seconds while the app is open, track selected `.d2s` saves by SHA-256. Establish a baseline without creating a version; create one timestamped, checksum-verified version only after file contents change. Repeated scans of unchanged saves create no versions.
- Create a verified pre-change backup before every app operation that writes or restores a character save.
- Show each character's backup history with timestamp, file size, and verification status; initially display the five newest versions and let users expand the list.
- Let users preview and restore a selected version with a clear confirmation. Before restoring over a save, preserve the current version, write atomically, and validate the restored save.
- Let users disable automatic versioned backups in local settings while keeping pre-restore safety backups mandatory.
- Keep snapshots local, provide a clear backup folder location, and document storage use and retention controls.

## Future work

- Improve item-property and requirement decoding.
- Add user-controlled experimental feature sets.
- Invite community contributors with current D2R installs to design and validate separate D2R offline-save support.
- Explore a local item fingerprint and duplicate-screening utility for item data users provide, with confidence and false-positive limits shown.
- Assess Android only after desktop save workflows are stable.
- Keep the peer-to-peer offline LoD marketplace deferred under `MARKETPLACE-PRD.md`.

## Current editor — character gold

- Provide separate edit fields for carried gold (belt/inventory) and personal stash gold.
- Enforce the classic LoD limits: carried gold is capped by character level and stash gold at 2,500,000.
- Review the requested amounts and confirm before writing the selected save in place; create a verified versioned backup first.

## Current editor — skills and attributes

- Show the seven classic LoD skill trees with current hard-point levels, skill requirements, and prerequisites.
- Display the corresponding original classic skill icon for each skill, bundled locally with source and rights attribution.
- Let players edit and respec skill levels using only unspent plus already allocated skill points. Never increase the character's earned point total.
- Let players edit and respec strength, dexterity, vitality, and energy using only unspent plus already allocated stat points, while preserving class starting values and the earned total.
- Before a write, show the proposed allocations and unspent totals, require confirmation, create a verified versioned backup, update the selected save in place, and validate the result.
- Restrict writes to validated classic LoD version 96 saves until other formats have fixtures and separate validation.
- Register character editing as a local feature flag, enabled by default for this release and switchable off from Features.

## Next version — character editing

- Add saved build plans that compare skill/stat targets with the character's current allocations and the items/runes in the local bank.
- Keep original Diablo II + LoD as the supported baseline. Any D2R save support must be a separately versioned, opt-in capability with its own validation.

Community proposals may extend the supported game matrix. D2R work must use its own version-specific parser and fixtures. Battle.net duplicate analysis, if pursued, must remain a local utility unless a supported data source is established; it must not claim to authenticate ownership or prevent server-side duplication.
