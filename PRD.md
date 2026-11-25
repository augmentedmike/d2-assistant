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
2. Back up and import items from supported stashes into a searchable local bank.
3. Filter by item type, rarity, class/level requirements, sockets, and transfer status.
4. Keep multi-selection while refining filters and transfer selected items to a chosen character inventory or personal stash.
5. Preserve original character files; write only `_d2a` copies, back up a prior `_d2a`, and validate generated saves.
6. Browse offline runeword recipes with ordered runes, sockets, valid bases, effects, and availability labels.
7. Let users opt into or out of optional feature sets locally through the Features screen. Extended D2R/Reign recipe references are off by default; safety behavior cannot be disabled.

## Safety requirements

- Show the selected source path and explain that a full stash import creates a verified backup and empties the selected stash after import.
- Never write to an original character save. Export only to `_d2a` paths.
- Back up an existing app-generated file before replacing it.
- Validate generated saves before reporting success.
- Clearly show unknown item requirements instead of guessing.
- Never claim offline item ownership or uniqueness is verified. A user can restore or copy a previous save outside the app.

## Feature proposals and contributions

- Use GitHub Discussions for feature ideas and voting. Votes guide prioritization; the maintainer decides based on quality, safety, game compatibility, and maintenance cost.
- Welcome contributions from people and coding agents under the same quality bar. See `CONTRIBUTING.md`.
- Optional features are registered as local feature flags with a description, default state, and compatibility notes.

## Future work

- Improve item-property and requirement decoding.
- Complete configurable backups and restore.
- Add user-controlled experimental feature sets.
- Invite community contributors with current D2R installs to design and validate separate D2R offline-save support.
- Explore a local item fingerprint and duplicate-screening utility for item data users provide, with confidence and false-positive limits shown.
- Assess Android only after desktop save workflows are stable.
- Keep the peer-to-peer offline LoD marketplace deferred under `MARKETPLACE-PRD.md`.

## Next version — skill trees and character points

- Add a skill-tree view for each classic Diablo II / LoD class, showing learned skills, levels, prerequisites, and unspent points.
- Let a player allocate and edit skill points with prerequisite and point-total validation.
- Let a player reset skill allocations and reassign earned skill points.
- Let a player edit and reset Strength, Dexterity, Vitality, and Energy allocations while preserving the character's earned point total.
- Let a player edit carried character gold within the save format's legal maximum.
- Treat this as an experimental, opt-in feature set until validated against supported offline save versions.
- On every apply/reset operation, preserve the original, create a verified backup of an existing `_d2a` output before replacing it, write only a `_d2a` copy, and validate checksum and parsed values before reporting success.
- Show a review of all proposed point and gold changes before writing. Require explicit confirmation for a full reset.
- Keep original Diablo II + LoD as the supported baseline. Any D2R save support must be a separately versioned, opt-in capability with its own validation.

Community proposals may extend the supported game matrix. D2R work must use its own version-specific parser and fixtures. Battle.net duplicate analysis, if pursued, must remain a local utility unless a supported data source is established; it must not claim to authenticate ownership or prevent server-side duplication.
