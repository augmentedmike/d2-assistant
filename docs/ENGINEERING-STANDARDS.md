# Engineering standards, architecture, and evidence

**Status:** baseline after the November 25, 2026 refactor pass
**Scope:** the current TypeScript frontend and Rust/Tauri backend, with emphasis on item banking, character-save safety, and the rules extracted in this pass.

This document is the engineering record for the codebase. It describes the design constraints, checks, measured results, and known limits. It does not assign work by author. The intent is to make the project easier to change without accumulating repeated policy, accidental file mutations, or tests that create confidence beyond what they actually prove.

## Engineering objectives

The quality target is **fast, correct, and demonstrably safe for the supported workflow**. Those goals translate into specific constraints:

1. **Keep one implementation of each business rule.** Filtering, compatibility, save bounds, and grid placement should not be reimplemented in multiple UI callbacks or command handlers.
2. **Keep side effects at clear boundaries.** Pure calculations should be testable without Tauri, SQLite, or a user save. File writes should validate inputs and output, preserve a recoverable prior state, and target the selected save path.
3. **Reject ambiguous or malformed data safely.** Unknown requirements should stay unknown; invalid dimensions and malformed records should not be treated as usable data.
4. **Test behavior at the smallest useful boundary.** Pure functions get table-driven boundary tests. File-format workflows require sanitized fixtures and round-trip assertions before they can be called proven.
5. **Use metrics to find work, not to optimize a vanity score.** Clone counts, complexity, line counts, and coverage expose specific risks. None measures correctness by itself.

## Architecture and boundaries

### Frontend

`src-ts/App.tsx` remains the application coordinator: it owns tabs, selected character, loading/error/notice state, Tauri calls, and rendering. The refactor moved reusable item rules to `src-ts/bankItemPresentation.ts`:

- `bankItemDetails` parses stored metadata into type, rarity, properties, and type-appropriate attributes. It does not show socket counts for gems or unrelated item categories.
- `itemCompatibility` produces `usable`, `unusable`, or `unknown`; missing character or item requirements never silently become a positive match.
- `filterBankItems` applies the search, type, class, rarity, transfer state, selected-character compatibility, level interval, and socket filters in one place.
- `validateGoldDrafts` checks integer values, nonnegative bounds, the level-based carried-gold cap, and the stash cap before a write is requested.

The component memoizes item details once per bank item refresh and reuses that map for the filter options and cards. This removes repeated JSON parsing from the per-item/per-render filter path. It is an algorithmic improvement (parse each item once per refresh); there is no timing benchmark yet, so no wall-clock speedup is claimed.

`src-ts/CharacterBuildEditor.tsx` owns the skill and attribute editing UI. It is still a large component and remains a complexity hotspot. Further extraction should follow testable behavior seams (allocation rules, reset rules, presentation) rather than splitting JSX solely to lower a number.

### Rust/Tauri backend

`src-rs/src/lib.rs` remains the Tauri command registration and character workflow entry module. Responsibility was moved into modules with narrower jobs:

- `database.rs`: app database path, schema initialization/migration, and settings access. Schema initialization is tested for repeatability; setting keys and values are bounded.
- `bank_commands.rs`: bank CRUD, stash import, character transfer, item identity checks, and transfer state updates. The Tauri invoke payload is kept stable.
- `grid.rs`: inventory/stash occupancy and first-fit row-major rectangle placement. It rejects zero-sized or oversized items and malformed buffers; marking is clipped to grid bounds.
- `items.rs`, `d2s.rs`, and `d2x.rs`: item bit parsing and save/stash format handling. Parser tests use synthetic byte sequences; private local save fixtures are deliberately not committed.

The bank transfer path checks that selected bank rows exist and have retained encoded data, refuses rows already marked transferred, detects repeated item seeds within the target save and current batch, finds legal destination grid space, serializes and reparses the result, verifies item count, captures a backup, writes through a temporary path, verifies written bytes, then updates transfer state. This is a sequence of safety checks, not a proof of compatibility with every save variant. The current seed comparison is not a universal Battle.net or cross-character duplicate detector.

The stash import path copies and byte-verifies the source backup before database insertion, imports item records in a database transaction, then prepares and validates an empty stash before replacing the selected stash file. Tests still need to cover this entire operation over sanitized `.d2x` fixtures, including failures between those stages.

Character editing is expected to act on the selected character file in place, create a verified versioned backup before mutation, and validate the resulting save. Never infer safety from a successful UI refresh alone. Tests against real-format sanitized fixtures are the missing evidence for this boundary.

## Change and review standards

For changes to business rules or save handling:

- Put pure transformations and validation in functions that accept explicit inputs and return explicit results. Avoid hidden reads, writes, or UI state in these functions.
- Test both normal cases and boundary/failure cases: empty, malformed, unknown, minimum, maximum, over-limit, and conflicting inputs as applicable.
- Keep unknown/unparsed metadata distinguishable from a known value. Do not make a search/filter result look authoritative if the underlying requirement is unknown.
- Preserve the external Tauri command contract unless a coordinated API change is intended; add compatibility tests when changing serialized data or DB schema.
- For any file mutation, test that the source path is the intended path, that failures do not produce a second character file, and that backup/restore behavior is correct. Do not use a developer's personal save files in CI.
- Extract code when it establishes a single authoritative rule or isolates a side effect. Do not extract merely to make a file shorter or satisfy a numeric threshold.
- Review clone reports for meaningful repeated policy; generated assets, static game data, and build output are excluded from the scan.

## Metrics and how to reproduce them

Run at the repository root with the project lockfile installed:

```sh
bun run build
bun run lint
bun run test:frontend
cargo fmt --manifest-path src-rs/Cargo.toml -- --check
bun run lint:rust
bun run test:rust
bun run metrics:duplication
bun run metrics:complexity
bun test --coverage tests-ts
```

Pull request CI runs build, frontend tests, Rust formatting, Clippy, clone detection, complexity reporting, and Rust tests. TypeScript type checking is part of `build`. Clippy warnings fail CI (`-D warnings`). The complexity job is deliberately report-only while large components are being reduced; a warning is visible but does not block the change. `bun test --coverage` currently provides a local coverage report, not a project-wide enforced minimum.

### Metric definitions

| Metric | Tool and current rule | What it tells us | What it cannot tell us |
| --- | --- | --- | --- |
| Repeated code | jscpd 5.4.3; 8-line and 60-token minimum; CI fails above 5%; scans TS/TSX/Rust and excludes dependencies, build output, public assets, static game data, and Rust target output | Large copy/paste blocks likely to drift | Short clones, semantic duplication, duplicated SQL/business meaning, and duplicated behavior expressed differently |
| Branch complexity | ESLint `complexity` rule; diagnostic limit 10 for TS | Number of independent control-flow paths in a function; useful for identifying code that is harder to reason about and test | Function size, data coupling, readability, correctness, or whether splitting the function would improve design |
| Frontend coverage | Bun coverage report | Which statements/functions in the tested frontend module are exercised | Assertion quality, integration coverage, or coverage of the React app unless those tests are added |
| Rust tests/lint | `cargo test`; Clippy on all targets with warnings denied; rustfmt check | Compiled/tested backend behavior, common suspicious patterns, and formatting consistency | Save-format compatibility outside the actual test inputs or runtime behavior on each desktop OS |
| Source lines | `wc -l` on named files | Whether a known coordination module has been materially reduced | Complexity, coupling, maintainability, or whether code was moved without reducing conceptual load |
| Runtime performance | No benchmark suite currently | Nothing quantitative yet | No latency, throughput, startup, memory, or UI responsiveness claim is supported by this pass |

## Measured results

Commands above were run successfully on November 25, 2026. The baseline is the working tree after the refactor changes, not a released build.

| Area | Before → after / result | Evidence and qualification |
| --- | --- | --- |
| Rust command/persistence entry module | `src-rs/src/lib.rs`: 1,480 → 772 lines (−708, about 47.8%) | Database/schema, bank command bodies, and grid placement moved into focused modules. This is responsibility relocation, not a 47.8% reduction in total Rust code. |
| Frontend coordinator | `src-ts/App.tsx`: 1,126 → 1,056 lines (−70, about 6.2%) | Item detail/compatibility/filtering/gold validation moved to a 142-line pure module. App remains broad and is not considered decomposed. |
| Duplicate blocks | 0 clones; 0.00% duplicated lines | 15 source files / 4,688 lines were scanned. This is a clean result under configured minimum clone size, not evidence of zero repeated logic. |
| Complexity warnings | 9 functions above 10 | `App()` 49; `CharacterBuildEditor()` 29; seven other functions range from 11 to 15. This is recorded debt. The extracted `filterBankItems` has complexity 15, showing that moving policy out of JSX did not make its own control flow simple; its conditions are independently tested and could be decomposed further if that improves maintenance. |
| Frontend rule tests | 8 tests pass, 21 assertions | Tests cover metadata display, malformed metadata fallback, compatibility states, combined bank filters, unknown levels under ranges, and gold caps/rejections. |
| Extracted frontend module coverage | 100% functions; 98.92% lines | `src-ts/bankItemPresentation.ts` only; no whole-app coverage claim is made. |
| Rust tests | 11 pass; 2 ignored | Two ignored tests require local character/stash saves. Synthetic tests exercise LSB-first bit reading and bounds, simple item parsing/alignment, item labels/classification, grid placement, settings bounds, schema repeatability, and bank serialization. |
| Rust lint/format and frontend build/lint | Pass | `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `bun run build`, and `bun run lint` all passed. |

## Test evidence by behavior

| Behavior | Current test | Not yet demonstrated |
| --- | --- | --- |
| Item attribute presentation | sockets shown only for supported equipment categories; metadata labels omitted from properties; malformed JSON handled | Correct decoding of every item class/affix from actual stash records |
| Character compatibility | known class/level matches and mismatches; unknown requirement and missing character remain unknown | Accurate requirement decoding across all rare/set/unique/crafted/runeword cases |
| Bank filtering | search plus type, transfer, compatibility, and level conditions; unknown level excluded from a bounded-level result | UI interaction state, selection persistence under changing filters, and large-bank performance |
| Gold validation | integer-only values; carried and stash maximums accepted; over-cap/fraction rejected | End-to-end bytes changed exactly as expected in a sanitized `.d2s`; backup restore after interrupted write |
| Item bit/grid parser | bit order, simple record fields/alignment, truncation rejection, grid capacity and boundary behavior | Fuzz/property testing, complex/socketed item parsing in fixture corpus, and full stash round trips |
| SQLite setup/settings | repeatable schema initialization and key/value bounds | Migration tests from each previously shipped database schema and failure recovery |
| Bank import/export and character backup lifecycle | command code has backup, validation, temporary-write, and transaction checks | Automated integration tests for the complete workflow and each injected failure point |

## Remaining engineering work

The current evidence supports the extracted pure rules and the tested parser/grid/DB cases. It does **not** yet support calling all character editing or import/export paths proven. Priorities:

1. Create a small sanitized fixture corpus for each supported classic `.d2s` and `.d2x` shape, with expected parsed fields and byte-preserving/round-trip expectations.
2. Extract character mutation and backup lifecycle steps behind testable interfaces, then test in-place path behavior, exact changed fields, backup verification, restore, and failure injection. Include a regression that proves no `_d2a` or duplicate character file is created.
3. Add database integration tests for source identity, re-import idempotence, transfer status, and migration from old schemas.
4. Add component tests around confirmation, selected character identity, visible error state, refresh after writes, and bulk selection/filter behavior.
5. Reduce `App()` and `CharacterBuildEditor()` complexity by extracting cohesive state transitions or workflow hooks with tests, and split `filterBankItems` into named predicate helpers only if that makes the filter contract clearer. Keep complexity warnings visible and record the trend.
6. Add runtime benchmarks only for a demonstrated hot path (large bank filtering, metadata parsing, or save parsing). Record fixture size, hardware/runtime, warmup, iterations, and before/after values. Until then, performance statements are limited to structural facts such as parsing each bank item's metadata once per refresh.
7. Add Rust coverage reporting and fuzz/property testing for untrusted save bytes once fixture and parser boundaries are stable.

## Release interpretation

Passing the current CI means the frontend builds, lint passes, the listed unit tests pass, Rust formatting and Clippy pass, and no clone above the configured threshold was found. It does not certify every user save, every D2 version, every transfer failure mode, or OS-specific installer behavior. Release claims about those areas should wait for the fixture and integration evidence above.
