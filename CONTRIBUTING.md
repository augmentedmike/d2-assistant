# Contributing

People and coding agents are welcome to contribute. The same review bar applies to every patch: it should solve a clear user problem, be safe with game files, fit the supported-game scope, and be maintainable by the project.

## Before proposing a feature

1. Search existing [Discussions](https://github.com/augmentedmike/d2-assistant/discussions) and open feature requests.
2. Add a discussion if the idea needs design or trade-off discussion. State the game/version, user problem, proposed behavior, and any save-file or privacy impact.
3. Vote with the discussion's upvote reaction. Maintainers will periodically review votes and mark ideas `planned`, `needs-design`, `declined`, or `shipped`.
4. Small fixes can go directly to a pull request. Larger changes should have maintainer agreement on the discussion first.

Votes inform prioritization; they do not guarantee implementation. Security, data safety, accessibility, and maintenance cost take precedence over vote totals.

## Priority community work: D2R and item fingerprints

Contributors who play the current D2R release are especially valuable. We welcome proposals and patches for current-version offline save parsing, item/property tables, migration handling, and sanitized compatibility fixtures. Keep D2R support distinct from original LoD support; never infer compatibility from the game name alone.

We also welcome research on a local item fingerprint and duplicate-screening utility that could help Battle.net players compare item records they provide. Start with a design discussion describing the available data, fingerprint inputs, collision/false-positive risks, and how confidence will be shown. The current app does not connect to Battle.net or inspect online characters, and a local fingerprint cannot prove an item is genuine or prevent a server-side dupe. Do not request or submit account credentials, private account access, or private save files.

## Agent-authored contributions

Agent-authored patches are welcome when a human contributor takes responsibility for the submission and review. Agents should inspect the existing product requirements and code before editing, keep changes focused, explain assumptions, and provide build or validation results. Do not add AI attribution or co-author trailers to commits unless the project owner explicitly asks for them.

## Pull request quality bar

- Explain the user problem and the behavior change.
- Keep original `.d2s` files protected and preserve the `_d2a` output rule.
- Back up before any destructive stash operation and verify outputs.
- Do not claim support for D2R, Battle.net, or save formats that have not been implemented and validated.
- Treat D2R and fingerprinting support as separate, opt-in capabilities until their game-version support and validation are reviewed.
- For duplicate screening, distinguish exact record matches from heuristic similarities and explain false-positive/false-negative limits. Never claim server-side protection.
- Add or update documentation for new settings, compatibility, or data formats.
- Keep optional capabilities behind local feature flags. Flags must default off for experimental features and must never disable safety checks or corrupt existing user data when switched.
- Include screenshots or a short recording for visible UI changes when practical; never include real save files without permission.
- Keep commits focused and use lowercase imperative subjects.

## Local validation

```sh
bun install --frozen-lockfile
bun run build
bun run lint
bun run test:frontend
cargo fmt --manifest-path src-rs/Cargo.toml -- --check
bun run lint:rust
bun run test:rust
bun run metrics:duplication
```

Save-file integration checks are ignored unless you explicitly provide local fixtures with `D2A_TEST_CHARACTER` or `D2A_TEST_STASH` and pass `-- --ignored`. Never add personal fixtures to a pull request.

Private character and stash fixtures are not distributed in this repository. Do not commit personal save files, database files, credentials, or absolute local paths.
