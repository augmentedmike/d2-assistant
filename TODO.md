# D2 Assistant — Roadmap

This is the public project task list. Completed work is recorded in `CHANGELOG.md`.

## Release follow-up

- [ ] Pin GitHub Actions to immutable commit SHAs and enable Dependabot for Bun, Cargo, and GitHub Actions.
- [ ] Add recurring secret and dependency vulnerability scans to CI, with an explicit policy for upstream unmaintained/unsound warnings.
- [ ] Verify production CSP and IPC/dialog/image behavior in macOS, Windows, and Linux bundles.
- [x] Start backend refactor by extracting SQLite/settings, bank transfer commands, and inventory-grid placement into focused Rust modules.
- [x] Add Rust unit coverage for item bit parsing, grid placement bounds, and repeatable database initialization; run tests and Clippy in CI.
- [x] Extract bank item display/filter rules into a pure module with focused unit tests.
- [ ] Add sanitized save fixtures and behavior tests for character edits, backup/restore, and bank import/export; add frontend component tests.
- [x] Add a repeatable code-clone metric with an initial 0% duplicate-line baseline (8-line/60-token minimum, 5% CI ceiling).
- [x] Record the engineering standards and measured quality baseline, including complexity hotspots and test limitations.
- [x] Save and restore the selected game-save folder in the app's local settings database, migrating the previous WebView preference.
- [ ] Improve decoding for magical, rare, set, unique, crafted, and runeword item requirements.
- [ ] Add starter build knowledge and build-aware item matching.
- [ ] Add a broader, sanitized fixture set for supported classic save formats.
- [ ] Invite current-release D2R players to help design/validate a separate D2R offline-save adapter and item tables.
- [ ] Design a local item fingerprint/duplicate-screening utility; specify inputs, collision limits, and confidence labels before implementation.
- [ ] Validate installers on real Windows and Linux machines.
- [ ] Capture approved product screenshots and add a short usage demo.
- [ ] Review accessibility, keyboard use, and small-window layouts.
- [ ] Add code signing and notarization if credentials become available.

## Next version — character editing and backup recovery

- [x] Add classic LoD skill trees with current allocations, prerequisites, and unspent points.
- [x] Add locally bundled original classic skill icons to the Character Lab skill trees.
- [x] Allow skill allocation only from unspent earned skill points; permit respec/reallocation of already spent points without increasing the earned total.
- [x] Allow attribute allocation only from unspent earned stat points; permit respec/reallocation of already spent points without increasing the earned total.
- [x] Add separate carried (belt/inventory) and stash gold fields with save-format limits and a review before writing.
- [x] Add a local feature flag for classic skill and attribute editing (on by default, switchable off); show a review and require confirmation before any write.
- [x] Back up and validate the selected save before gold edits and bank transfers; update the chosen character in place to avoid duplicate character files.
- [x] Add automatic, versioned character-save backups behind a local setting that defaults on.
- [x] Detect changed saves by SHA-256 on app start, folder refresh, and background checks while the app is open; establish an initial baseline without creating a version and save one verified snapshot only after file contents change.
- [x] Add character backup history with timestamps and verification status; show the newest five by default with an option to expand.
- [x] Add confirmed restore with a verified backup of the current save, atomic write, and post-restore validation.
- [ ] Add local retention controls and disclose backup storage location.
- [ ] Add saved build plans and compare their gear requirements with owned bank items.

## Future platform / product ideas

- [ ] Evaluate Android after the desktop file workflow is stable.
- [ ] Revisit invite-only peer trading for classic LoD offline saves after release; see `MARKETPLACE-PRD.md`.

## Scope guardrails

- D2R recipe reference entries do not imply D2R save support.
- No online character, Battle.net, or in-game automation support is planned.
- Never create an extra character file during a character edit or item transfer. Keep a verified recoverable backup before changing the selected save.
- Never claim offline item transfers are dupe-proof.
