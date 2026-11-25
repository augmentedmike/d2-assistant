# Classic skill tree data

`classic-skills-v96.json` is a compact, normalized catalog for classic Lord of Destruction. It contains the seven classic classes' 30 skill IDs, display labels, tree/row/column positions, level gates, hard-point caps, and prerequisite links; it does not include skill formulas, game assets, or full text tables.

The catalog was derived from the Diablo II 1.13 data tables `Skills.txt`, `SkillDesc.txt`, and `CharStats.txt` mirrored in [fabd/diablo2](https://github.com/fabd/diablo2/tree/master/code/d2_113_data). The class starting attributes used by the editor also come from `CharStats.txt`. Names with changed in-game labels (for example, Dopplezon → Decoy) are normalized in the catalog. The `icon` field maps each skill to its extracted classic game icon; see [SKILL-ICON-ASSETS.md](./SKILL-ICON-ASSETS.md) for the asset source and rights notice. Verify against supported game data before changing it.

The upstream repository is MIT licensed. Its notice is included here for attribution:

> The MIT License (MIT)
>
> Copyright (c) 2014 Fabrice Denis
>
> Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
