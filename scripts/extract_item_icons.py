#!/usr/bin/env python3
"""Convert classic Diablo II inventory DC6 sprites to transparent PNG files.

Usage: python3 scripts/extract_item_icons.py GFX_DIR PALETTE_DAT OUTPUT_DIR
The source assets and palette are local build inputs and are not runtime dependencies.
"""

from __future__ import annotations

import argparse
import struct
from pathlib import Path

from PIL import Image


def decode_frame(path: Path, palette: bytes) -> Image.Image:
    data = path.read_bytes()
    directions, frames_per_direction = struct.unpack_from("<II", data, 0)
    if directions < 1 or frames_per_direction < 1:
        raise ValueError(f"{path.name}: empty DC6 file")
    frame_offset = struct.unpack_from("<I", data, 24)[0]
    flip, width, height, _, _, _, _, next_block = struct.unpack_from("<8I", data, frame_offset)
    if width < 1 or height < 1 or width > 1024 or height > 1024:
        raise ValueError(f"{path.name}: invalid frame dimensions {width}x{height}")
    pixels = bytearray(width * height)
    cursor = frame_offset + 32
    x = y = 0
    while cursor < min(next_block, len(data)) and y < height:
        command = data[cursor]
        cursor += 1
        if command == 0x80:
            x = 0
            y += 1
            continue
        if command & 0x80:
            x = min(width, x + (command & 0x7F))
            continue
        end = min(x + command, width)
        run_length = end - x
        pixels[y * width + x : y * width + end] = data[cursor : cursor + run_length]
        cursor += command
        x = end

    rgba = bytearray(width * height * 4)
    for index, color_index in enumerate(pixels):
        offset = index * 4
        palette_offset = color_index * 3
        rgba[offset : offset + 3] = palette[palette_offset : palette_offset + 3]
        rgba[offset + 3] = 0 if color_index == 0 else 255
    image = Image.frombytes("RGBA", (width, height), bytes(rgba))
    if flip:
        image = image.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    return image


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("gfx", type=Path)
    parser.add_argument("palette", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    palette = args.palette.read_bytes()
    if len(palette) != 256 * 3:
        raise SystemExit("palette must contain 256 RGB entries (768 bytes)")
    args.output.mkdir(parents=True, exist_ok=True)
    files = sorted(args.gfx.glob("inv*.dc6"))
    if not files:
        raise SystemExit(f"no inventory DC6 files found in {args.gfx}")
    for source in files:
        decode_frame(source, palette).save(args.output / f"{source.stem[3:].lower()}.png", optimize=True)
    print(f"Extracted {len(files)} item icons to {args.output}")


if __name__ == "__main__":
    main()
