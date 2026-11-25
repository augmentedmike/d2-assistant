# D2 Assistant — Roadmap

This is the public project task list. Completed work is recorded in `CHANGELOG.md`.

## Release follow-up

- [ ] Improve decoding for magical, rare, set, unique, crafted, and runeword item requirements.
- [ ] Complete configurable character/stash backup and restore.
- [ ] Add starter build knowledge and build-aware item matching.
- [ ] Add a broader, sanitized fixture set for supported classic save formats.
- [ ] Invite current-release D2R players to help design/validate a separate D2R offline-save adapter and item tables.
- [ ] Design a local item fingerprint/duplicate-screening utility; specify inputs, collision limits, and confidence labels before implementation.
- [ ] Validate installers on real Windows and Linux machines.
- [ ] Capture approved product screenshots and add a short usage demo.
- [ ] Review accessibility, keyboard use, and small-window layouts.
- [ ] Add code signing and notarization if credentials become available.

## Next version — skill trees and character editing

- [ ] Add class skill trees with current allocations, prerequisites, and unspent points.
- [ ] Allow skill allocation/editing and reset/reallocation with point-total validation.
- [ ] Allow Strength, Dexterity, Vitality, and Energy allocation/reset while preserving earned totals.
- [ ] Allow editing carried gold within the save format's legal limit.
- [ ] Gate the editor behind a local experimental feature flag; show a review and require confirmation before any write.
- [ ] Write only validated `_d2a` copies and back up prior outputs; never modify originals.

## Future platform / product ideas

- [ ] Evaluate Android after the desktop file workflow is stable.
- [ ] Revisit invite-only peer trading for classic LoD offline saves after release; see `MARKETPLACE-PRD.md`.

## Scope guardrails

- D2R recipe reference entries do not imply D2R save support.
- No online character, Battle.net, or in-game automation support is planned.
- Keep original saves protected and preserve the `_d2a` export rule.
- Never claim offline item transfers are dupe-proof.
