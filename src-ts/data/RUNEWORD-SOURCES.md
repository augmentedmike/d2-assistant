# Runeword catalog sources

The app ships a local JSON catalog; it makes no network requests and does not integrate with external tools.

- The original LoD and D2R through patch 2.6 entries were adapted from the MIT-licensed `diablo-tools/d2-runewords` dataset. See `RUNEWORD-DATA-LICENSE.txt`.
- The five Reign of the Warlock recipes, Hustle’s Mania/Hysteria variants, and the patch 3.4 / Season 16 Eternity variant were checked against the current game-data based catalog at https://d2runewizard.com/runewords and Blizzard's expansion announcement at https://news.blizzard.com/en-gb/article/24243863/rain-annihilation-in-reign-of-the-warlock.
- Original LoD recipe constraints were cross-checked against The Arreat Summit: https://classic.battle.net/Diablo2exp/items/runewords.shtml.

The source dataset is versioned content, not a live service. Patch labels show recipe origin. LoD ladder-only recipes are labeled separately; D2R-only recipes are excluded from LoD compatibility; Reign of the Warlock recipes require that D2R expansion. Season-specific limits can change. The catalog records known ladder notes where confirmed; it is not a live ladder-status service. The Season 16 Eternity entry is called out as Ladder/Offline-only, and Hustle is shown as superseded by Mania/Hysteria in Reign of the Warlock.
