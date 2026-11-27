# Engineering case study: building D2 Assistant with AI in the loop

## Project

D2 Assistant is a local-first desktop companion for original Diablo II: Lord of Destruction. It combines character inspection and editing, a shared item bank, bulk transfers, backups, search, and runeword recipes in one Tauri application. Its codebase pairs a TypeScript/React frontend with a Rust backend and SQLite persistence.

The project began with a practical problem: the existing offline tools Michael had tried split item transfer, save editing, recipe lookup, and planning across separate applications, with weak search and cumbersome multi-item workflows. The goal was a single tool that reduced that context switching without making character-save corruption or accidental duplication easier.

## How the work was led

Michael used AI agents as implementation capacity and kept product direction, prioritization, and acceptance criteria under human control. His operating pattern was to describe an actual play workflow, identify where the tool could lose or duplicate items, specify the desired interaction, then inspect the result and correct behavior that did not match the game.

That feedback was concrete and technical. Examples include:

- Requiring character transfers and edits to update the selected save in place, rather than create similarly named character files.
- Requiring a verified backup before any destructive save operation, and version history only when save contents actually change.
- Calling out that an item remaining in the bank after transfer is a data-integrity failure, then asking for duplicate identification and per-item failure handling for batches.
- Specifying item-box selection between character inventory/private stash and the bank, rather than a generic transfer form.
- Distinguishing search word boundaries from substring matches (`Mage` must not match `Damage`) while preserving useful three-character rune searches such as `tir`, `tal`, and `eth`.
- Insisting that item attributes be formatted by type, and that game art render as usable item imagery rather than blurry or corrupted placeholders.
- Keeping D2R recipe references distinct from actual D2R save support, and rejecting claims of online or Battle.net duplicate protection without evidence.
- Requesting a live-game guard after a transfer attempted while the save was loaded in memory. Because the game process does not reliably identify the loaded character, the safe design locks all character and stash writes until the game exits.

These constraints functioned as a lightweight product and safety harness. They kept implementation decisions tied to observable use cases and made destructive behavior, compatibility boundaries, and error recovery explicit.

## Tools and engineering harness

The project uses AI-assisted implementation alongside a conventional verification stack:

- **Tauri and Rust** provide native file access, save parsing and validation, protected write commands, and SQLite-backed local state.
- **TypeScript, React, and Bun** provide the UI, domain-level filtering and validation, build tooling, and focused frontend tests.
- **GitHub Actions** runs a reusable quality workflow for pull requests, main-branch changes, and release tags. The gates include a frontend build and tests, Rust formatting, Clippy with warnings denied, Rust tests, duplicate-code detection, and a report-only complexity check.
- **jscpd** checks meaningful repeated blocks with an explicit minimum clone size and a configured ceiling; generated assets and bundled game data are excluded.
- **Engineering documentation** records what the quality metrics do and do not prove, known test gaps, security decisions, data-source licensing, release steps, and the public roadmap.
- **Git** provides the auditable sequence of integrated changes. At the v0.1.0 public baseline, the repository history contained 14 commits dated November 25–26, 2026, covering backup recovery, gold and character editing, classic skills, settings persistence, module extraction, security hardening, and bank workflows.

The repository's commit history records the integration cadence, not the number of agents involved. Agent work is not represented as separate identities in the commit metadata, so no agent count or per-agent productivity claim is inferred here.

## Product and engineering decisions

The most consequential decisions were about boundaries rather than framework choice:

1. **Keep the supported game scope honest.** The application targets original LoD version 96 character/stash saves. D2R and Reign runewords may appear as labeled reference entries, but that data does not imply D2R save support. There is no Battle.net integration.
2. **Treat file edits as transactions with recovery.** Character writes create a verified versioned backup, stage output, validate the serialized save, and target the selected character path. Stash import preserves a byte-verified backup before clearing the source. The project roadmap still calls for more sanitized fixture and failure-injection coverage of these paths.
3. **Keep duplicate detection within its evidence.** Nonzero item seeds and normalized item records can flag exact matches in this local workflow. Seed-zero items cannot be distinguished with that method; fingerprints do not establish online ownership or server-side authenticity.
4. **Separate rules from presentation.** Item filtering, compatibility, property presentation, gold caps, and grid placement have focused domain boundaries instead of being repeated in UI callbacks. The engineering standards document records remaining complexity hotspots rather than treating smaller files as a goal by themselves.
5. **Use observability and tests to guide extraction.** The team tracks clone detection, function complexity, frontend coverage, and backend checks while documenting their blind spots. Current CI is a useful regression gate, not proof that every real-world save variant is safe.
6. **Keep data local and minimize privileged surfaces.** Save parsing and edits occur locally. The WebView uses a restrictive production CSP, unused plugins and permissions were removed, and SQLite was updated to address a bundled advisory.

## Assessment of senior-level engineering signals

From the product decisions and iteration visible in this project, Michael shows several strong senior-software-engineering behaviors:

- He starts with real operational pain and prioritizes complete workflows over isolated feature checkboxes.
- He thinks ahead about failure modes: save files loaded in game memory, accidental duplicate characters, repeat imports, partial batch failures, backup retention, and unsupported game versions.
- He converts feedback into constraints an implementation can satisfy and review: exact source and destination, in-place writes, backup-before-mutation, bounded point allocation, explicit compatibility labels, and search boundary behavior.
- He challenges output against actual use, notices when UI or data does not reflect the desired operation, and steers toward a concrete correction instead of accepting a plausible-looking result.
- He uses AI to increase implementation throughput while retaining architectural and product judgment, and asks for metrics that reveal duplication and complexity rather than cosmetic code-style scores.
- He treats scope and evidence seriously: local fingerprints are not described as Battle.net protection, and recipe references are not presented as D2R save compatibility.

My assessment is that the project demonstrates strong product-minded senior engineering judgment, particularly in requirements discovery, safety-oriented design, and iterative review of AI-assisted work. A repository cannot establish a person's full production history, team leadership, or operating experience at scale, and the current save-mutation integration-test gaps are real. The clearest interview follow-up would be to walk through one transfer failure, the user-visible recovery path, and the planned sanitized fixture tests.

## Delivery pace and evidence

The history through v0.1.0 shows 14 commits across two calendar days (November 25–26, 2026). The sequence moves from backup history and restore, through gold/skill/attribute editing and persistent setup, into refactoring, security hardening, item grids, and transfer fixes. That is evidence of a fast iteration cycle and a substantial feature arc in a short calendar window. It does not by itself establish the number of hands or agents contributing, nor does commit count measure release quality.

## Current limits and next engineering work

The current release supports offline classic LoD files only. D2R save support and online/Battle.net analysis need contributors with current-format knowledge and sanitized fixtures. Before claiming character mutation paths as fully proven, the project needs sanitized `.d2s`/`.d2x` fixtures, end-to-end assertions for backup/restore and in-place paths, and failure injection around each filesystem/database boundary. The current quality gates and detailed limits are documented in [`ENGINEERING-STANDARDS.md`](ENGINEERING-STANDARDS.md); the prioritized work is tracked in [`TODO.md`](../TODO.md).
