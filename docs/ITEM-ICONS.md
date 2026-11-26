# Classic item icons

The bank cards and character storage grids use the classic Diablo II inventory sprites. `public/item-icons/` contains transparent PNG conversions of the first frame of the game's `inv*.dc6` sprites. `src-ts/data/classic-item-icons-v96.json` maps version 96 item codes to those sprites using the bundled classic item tables.

The PNGs are extracted from Diablo II game art and remain Blizzard Entertainment property. Their inclusion does not change the MIT license for original D2 Assistant code, imply Blizzard endorsement, or grant rights to redistribute the game. The app bundles the converted PNGs and mapping; it does not read from or depend on another utility or a Diablo II installation at runtime.

The source inventory art is low-resolution pixel art (commonly 28×28 or 56×56 pixels). The UI uses nearest-neighbor scaling (`image-rendering: pixelated`) to preserve sharp sprite edges when enlarging it.

For maintainers who already have locally extracted original inventory sprites and the matching 256-color palette, regenerate the PNGs with:

```sh
python3 scripts/extract_item_icons.py /path/to/inventory-graphics /path/to/palette.dat public/item-icons
```

Python Pillow is required for the conversion script. The sprite inputs and palette are local build inputs and are not included in this repository.
