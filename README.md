<p align="center"><img src="branding/d2-assistant.png" alt="D2 Assistant gothic red-and-steel emblem on a transparent background" width="320"></p>

# D2 Assistant

**A safer, searchable item bank for offline Diablo II: Lord of Destruction.**

D2 Assistant helps organize classic Diablo II items and move selected gear back onto local character saves. It is a native desktop app built with Tauri; item data stays on your computer.

> **Compatibility:** Character and stash editing currently targets original Diablo II + Lord of Destruction save files. Diablo II: Resurrected and online/Battle.net characters are not supported. D2R and Reign of the Warlock recipes are optional reference data only; turn them on from **Features** if you want to browse them.

## What it does

- **Local shared bank:** import a supported version 96 `.d2x` stash with a byte-verified backup, then browse items as responsive cards or game-shaped grids with original classic item sprites. The source stash is emptied only after its items are banked.
- **Search and filters:** search item names, codes, and affixes from three-character prefixes; filter by affix, item type, rarity, class requirements, level range, socket count, compatibility, and transfer state. Filters combine, and selections remain while you refine results.
- **Bulk item movement:** select items from a character's inventory or private stash to deposit into the shared bank, or select bank items and place them into a chosen character's inventory or private stash. Every character write targets the chosen save in place, with a verified backup and output validation.
- **Duplicate review:** detect matching nonzero item fingerprints in the bank and avoid repeat transfers. Batch deposits skip individually identified repeats while allowing other selected items to move. Seed-zero items cannot be distinguished by that fingerprint method.
- **Detailed item inspection:** hover or keyboard-focus an item for its decoded properties, sockets, quality, requirements, and other available details. Properties are formatted by item type, and unknown values remain explicitly unknown.
- **Character pages:** inspect character class, level, stats, equipment, inventory, and private stash in separate Overview, Items, Skills & stats, and Backups tabs.
- **Skill and attribute editing:** view all seven classic LoD skill trees with original skill icons. Spend only earned unspent points, observe level/prerequisite rules, reallocate spent points, or respec. Writes are confirmed, backed up, validated, and limited to supported version 96 saves.
- **Gold editing:** edit carried (belt/inventory) and stash gold separately, with game limits applied and a Max out action for each balance.
- **Versioned recovery:** automatic local character backups are on by default. New versions are created only when save contents change; the history shows the newest five first and supports restore with a preserved current save.
- **Live-game write protection:** while Diablo II or D2R is running, character and stash files are read only. The game process does not reliably reveal which character is loaded, so the lock covers all save files until the game exits.
- **Persistent setup and local data:** remember the selected game-save folder in the app's local settings database. Item bank and character data stay on the computer; the app does not connect to Battle.net.
- **Offline runeword recipes:** browse ordered runes, bases, socket counts, and bonuses. Standard LoD recipes are available by default; ladder, D2R, and Reign of the Warlock recipes are clearly marked and can be filtered through the local Features setting.
- **Desktop app and releases:** native Tauri app with a responsive red, ember, and steel visual theme, transparent logo/app icons, and GitHub Actions workflows for macOS, Windows, and Linux installers.
- **Community contribution workflow:** human- and agent-authored patches are welcome. Feature proposals can be discussed and voted on in GitHub Discussions; local feature switches support optional capabilities.

## Current support boundary

Character and stash editing targets original Diablo II + Lord of Destruction version 96 saves. D2R save editing, online characters, Battle.net connectivity, and server-side dupe protection are not implemented. D2R/Reign recipe entries are reference data only. Desktop installer builds are automated, but installers are currently unsigned and should be validated on each target OS.

## Why I built this

> I’ve been running Diablo II on Mac hardware through Wine for years. I tried ATMA, D2 Character Editor, GoMule, and browser tabs, but the workflows were frustrating and I kept needing separate tools for offline character and item management. I wanted one place to organize characters, transfer items in bulk, search and filter well, plan builds, and look up runeword recipes. I asked my agent, **helloam.bot**, to help build it. A few hours later, I had something much better for my workflow. Now I use D2 Assistant as a companion while I play offline and plan characters. The project is still growing toward that all-in-one goal.

## Safety and known limits

- Work on backups. Before importing a supported `.d2x`, the app creates and verifies a backup, moves the item records into its local bank, then empties the selected stash. Review the source path before confirming this operation.
- Gold edits and item transfers update the selected character save in place after a verified backup. Confirm the selected character path before applying changes.
- The current stash transfer path supports version 96 `.d2x` files. Other stash formats are rejected.
- Item requirements added by some magical, set, unique, crafted, or runeword properties may be unknown. The app labels unknown requirements rather than guessing.
- The bank uses item fingerprints to avoid accidental repeat exports inside the app. Offline saves have no authoritative server, so the app cannot prove ownership or prevent someone restoring an older backup.
- This is an early release. Keep independent backups and report any save compatibility issue before doing bulk transfers.

## Download

Download the package for your operating system from [GitHub Releases](https://github.com/augmentedmike/d2-assistant/releases):

- **macOS:** open the `.dmg`, then move D2 Assistant to Applications. The initial build is unsigned and may require Control-click → Open on first launch.
- **Windows:** run the `.msi` installer or the `.exe` installer when provided. Windows may show a SmartScreen warning because the release is not code-signed.
- **Linux:** use the `.AppImage` or `.deb` package. Package support depends on the distribution's WebKitGTK runtime.

If an installer is missing for your platform, that platform's release build has not completed yet. Download only from this repository's Releases page.

## First run

1. Launch D2 Assistant.
2. On **Characters**, choose your Diablo II save folder with **Browse…**. The selection is saved in the app's local settings database on this computer and restored on later launches.
3. Open **Bank** and choose a `.d2x` file only when you intend to back it up, move all its items into the bank, and empty that stash file.
4. Search and filter the bank, then select items and choose a character and destination.
5. Check the updated character in game. The app keeps versioned character backups and a verified stash backup.

## Contributions

Contributions from people and coding agents are welcome. **We especially need contributors who play the current D2R release** to help implement and validate its save formats, item tables, and version changes. We also welcome research and contributions for a local item fingerprint/duplicate-analysis utility that could help Battle.net players identify exact matches or suspiciously similar items from data they provide.

The current release does not read online characters, connect to Battle.net, or guarantee server-side dupe protection. Fingerprints can report evidence and confidence; they cannot prove an online item's ownership or authenticity. Do not submit account credentials or private save files. See [CONTRIBUTING.md](CONTRIBUTING.md) for the quality bar, feature voting, and pull-request process. Vote on ideas in [GitHub Discussions](https://github.com/augmentedmike/d2-assistant/discussions). Optional features will use local feature flags so users can opt in or out; safety behavior will never be hidden behind a flag.

For an overview of the AI-assisted development approach, engineering guardrails, delivery history, and an assessment of the project's engineering decisions, see the [engineering case study](docs/ENGINEERING-CASE-STUDY.md).

## Building from source

Prerequisites: Bun, Rust, and the native Tauri dependencies for your operating system. See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
bun install --frozen-lockfile
bun run build
bun run tauri:dev
```

Build the local native app with:

```sh
bun run tauri:build
```

This project currently builds for macOS on the development machine. Windows and Linux installers are built by the release workflow on native GitHub-hosted runners.

Maintainers can run **Desktop build check** from the GitHub Actions tab to build Windows and Linux installers without publishing a release. The workflow uploads temporary installer artifacts for review; see [RELEASING.md](docs/RELEASING.md). Standard GitHub-hosted runners are free for public repositories.

## Preparing a release

See the [desktop release process](docs/RELEASING.md) for the preflight command and publishing steps.

## Roadmap

- **Next version:** add saved build plans that compare a character's target skills and stats with gear already in the local bank. The development build now includes classic skill trees, point-conserving skill/attribute respecs, separate carried and stash gold editing, default-on changed-save backups, and five-at-a-time backup history.
- Invite D2R contributors to bring current-version save-format knowledge and sanitized fixtures; explore local fingerprint analysis for duplicate screening without connecting to Battle.net.
- Improve item-property and requirement decoding.
- Add optional, user-controlled experimental features.
- Investigate Android as a future target after the desktop save workflow is stable. Android is not supported in this release.
- A peer-to-peer marketplace for offline LoD items is deferred; it is not part of this release.

See [CHANGELOG.md](CHANGELOG.md), [TODO.md](TODO.md), and the [deferred marketplace concept](MARKETPLACE-PRD.md).

## License

D2 Assistant is released under the MIT License. Bundled runeword and classic skill data include source details and notices in [`src-ts/data`](src-ts/data). The original Diablo II skill artwork is not covered by the project MIT license; see [`SKILL-ICON-ASSETS.md`](src-ts/data/SKILL-ICON-ASSETS.md).
