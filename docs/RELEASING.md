# Desktop release process

The repository release command updates the application version, records the release notes, runs the local quality gates, and pushes a version tag. The tag starts the GitHub Actions workflow that builds macOS, Windows, and Linux installers. GitHub publishes the release only after every platform build and the release quality gate succeed.

## Prepare

1. Add concise user-facing bullets to `CHANGELOG.md` under `## Unreleased`.
2. Merge the intended changes into `main`, update local `main` from `origin`, and ensure the worktree is clean.
3. Preview the next stable semantic version:

   ```sh
   bun run release -- 0.2.0 --dry-run
   ```

4. When ready to publish, run:

   ```sh
   bun run release -- 0.2.0
   ```

The command refuses a non-main branch, dirty worktree, stale `main`, mismatched app versions, missing release notes, or a version tag that already exists. It updates `package.json`, the Tauri config, Cargo metadata, and the changelog; runs frontend build/tests, Rust formatting/Clippy/tests, and the duplication/complexity reports; commits the version bump; creates an annotated `vX.Y.Z` tag; then atomically pushes `main` and the tag to `origin`.

If a local quality check fails, the version and changelog edits are left in the worktree for review. If the push fails after the commit and tag are created, inspect the remote state before retrying; the command will stop if that tag already exists. Do not delete or move a published tag to retry a release.

## GitHub workflow

Pull requests and pushes to `main` use `.github/workflows/quality.yml`. The release workflow calls that same reusable workflow before building installers on native GitHub-hosted runners. All platform builds must succeed before the draft release is published.

To build Windows and Linux installers without publishing a release, run **Desktop build check** from the repository's Actions tab, or open a pull request touching app/build inputs. It packages Windows MSI/NSIS and Linux AppImage/deb files on native runners and uploads them as artifacts for seven days. This verifies compilation and packaging; it does not visually test the UI or replace launching and checking the installers on real Windows and Linux machines. Standard GitHub-hosted runners are free and unlimited for public repositories; artifact retention and GitHub's general Actions policies still apply.
