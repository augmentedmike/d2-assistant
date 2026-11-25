# Security review

**Review date:** 2026-11-25
**Scope:** checked-out repository, 22-commit Git history, locked JavaScript and Rust dependencies, Tauri capabilities/CSP, desktop file operations, and GitHub Actions configuration.

This was a source and dependency review, not a penetration test or a review of a built installer. No application window was opened and no personal save file was accessed.

## Summary

- **No secrets detected** in the current non-ignored project files or the 22-commit Git history scanned with Gitleaks 8.30.1. The tracked project files were also checked for common credential formats. An unrestricted scan of the working directory reported six generic-key matches inside generated Rust `.rmeta` build artifacts; those are compiler metadata false positives, not tracked project files. The follow-up scan of tracked and non-ignored files reported none.
- **JavaScript dependency audit:** `bun audit --audit-level=low` reported no vulnerabilities.
- **Rust dependency audit:** `cargo audit --file src-rs/Cargo.lock` initially found a medium SQLite vulnerability. The dependency was upgraded and the current scan reports no vulnerabilities. Three upstream warnings remain; details follow.
- **Tauri hardening applied:** configured a restrictive production CSP, separate localhost-only development CSP, prototype freezing, and removed unused SQL/log plugins plus the SQL capability.
- **Build and static checks:** frontend production build/lint and Rust check, tests, formatting, and Clippy pass after these changes. The production webview policy was schema-validated by Tauri's Rust build, but was not runtime-tested in an opened app window.

## Findings and action taken

### Fixed: bundled SQLite memory disclosure — medium

The original lockfile contained `rusqlite 0.31.0` and `libsqlite3-sys 0.28.0`, which bundle SQLite before the fixed SQLite 3.54.0 release. RustSec advisory [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html) describes a local attack involving a database and crafted sibling `-wal` file that can expose process memory when queried. This matters because the app opens a local SQLite database and enables rusqlite's bundled SQLite feature.

Updated rusqlite to `0.41` (locked to `0.41.1`) and libsqlite3-sys to `0.39.1`, which bundles the fixed SQLite version. The upgrade also removed the vulnerability from the current Cargo audit report. The API change required storing SQLite `file_size` as a checked signed integer and validating the conversion back to the public unsigned field; values outside the representable range now return an error instead of wrapping.

### Fixed: unnecessary Tauri SQL capability and plugins

The frontend had no use of the Tauri SQL plugin; persistence is implemented with Rust's `rusqlite`. Nevertheless, the app initialized `tauri-plugin-sql` and granted `sql:default`, which enables frontend connection loading, closing, and select operations. Removed the plugin, permission, and dependency. Also removed `tauri-plugin-log`, which was initialized but unused, and the unused direct `log` dependency. The native dialog permission remains because the UI invokes the file/directory picker.

### Fixed: no configured production CSP

`tauri.conf.json` had `csp: null`. Tauri documents CSP as an important WebView defense and only applies its CSP protection when configured ([Tauri CSP documentation](https://v2.tauri.app/security/csp/)). Added a production policy limiting default resources to the bundled app, IPC connections to Tauri's IPC origins, images to bundled/Tauri asset/data sources, styles to bundled sources, base URLs to the app, and objects to none. Set `freezePrototype` to true. Added a separate development policy permitting only Vite's configured localhost origin and HMR WebSocket; its `unsafe-eval` allowance is development-only and is not present in the production policy.

The CSP has been accepted by Tauri's config/build checks. Runtime behavior in WebViews across macOS, Windows, and Linux still needs verification before release.

## Dependency warnings still present

The current Cargo audit reports **zero vulnerabilities** and three warnings:

| Advisory | Current status | Assessment |
| --- | --- | --- |
| RUSTSEC-2026-0543, `chrono` unmaintained | Present in the lockfile through an optional `serde_with` dependency; `cargo tree --target all -i chrono@0.4.45` reports no active dependency path | No fixed upstream release is listed. Keep under review; not presently in the compiled dependency graph. |
| RUSTSEC-2024-0370, `proc-macro-error` unmaintained | Active through Linux GTK `glib-macros` | Build-time macro dependency in the Linux desktop stack; no replacement upgrade is available from the advisory. Revisit when Tauri/GTK dependencies move. |
| RUSTSEC-2024-0429, `glib 0.18.5` unsound iterator implementation | Active in the Linux GTK/WebKit stack used by Tauri | The affected GTK stack is platform-specific and the app does not directly call the cited API. This is still upstream soundness debt; no local suppression was added. |

The SQLite advisory was the only vulnerability-level finding in the initial Rust scan. Informational/unmaintained/unsound warnings are reported separately and were not described as exploitable app vulnerabilities.

## Secret handling and data flow

- The application does not make HTTP requests, use `fetch`, open WebSockets, authenticate to Battle.net, or collect account credentials in the frontend.
- Character and stash paths are supplied by the local UI's file/directory selection and passed to registered Rust commands. Those commands read and parse local files, store preferences and bank metadata in the app-local SQLite database, and write saves/backups as required by the chosen operation.
- Save files and backup contents are not intentionally sent off-device. This is based on source inspection; the app has not undergone packet capture or runtime telemetry testing.
- Error messages can include user-local file paths, and paths are intentionally shown in the UI for the user's selected saves. Do not copy raw diagnostic output into public reports without checking for personal paths.
- Gitleaks scanned all Git history with no findings. The current non-ignored project files were scanned separately to include working-tree changes.

## Release and CI risks

These are follow-up hardening items, not secret leaks or confirmed exploits:

1. GitHub Actions are referenced by movable version tags (`@v4`, `@v2`, `@stable`, `@v1`) rather than immutable commit SHAs. Pin actions to full SHAs and enable Dependabot updates for GitHub Actions, Cargo, and Bun dependencies to reduce supply-chain drift.
2. The release build job receives `contents: write` so it can draft/upload release assets. Keep this token limited to the release workflow and avoid exposing it to untrusted build scripts or pull-request code.
3. Desktop installers are unsigned and macOS packages are not notarized, as already disclosed in release documentation. This prevents users from getting publisher/signature verification from the OS; establish signing/notarization before broad distribution.
4. Dependency audits and secret scanning are not yet enforced in CI. The current clean results are from this review only. Add recurring CI checks once tool installation and advisory-warning policy are decided.
5. Add sanitized save fixtures and failure-injection tests for backup, restore, stash import, and character transfer. The present tests do not prove those complete file mutation workflows against real-format files.
6. Exercise the configured CSP in the bundled app on all three desktop OSes, including dialog use, skill images, and production IPC, before the next release.

## Reproduction

```sh
gitleaks git . --log-opts='--all --full-history' --redact
gitleaks dir <clean-copy-of-tracked-and-non-ignored-files> --redact
bun audit --audit-level=low
cargo audit --file src-rs/Cargo.lock
bun run build
bun run lint
cargo check --manifest-path src-rs/Cargo.toml
cargo test --manifest-path src-rs/Cargo.toml
cargo clippy --manifest-path src-rs/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-rs/Cargo.toml -- --check
```

Gitleaks 8.30.1 scanned approximately 1.22 MB across 22 commits with no findings. The filtered current-tree scan covered approximately 2.53 MB with no findings. The unfiltered working-directory scan included generated Rust build metadata and produced the six false positives described above.
