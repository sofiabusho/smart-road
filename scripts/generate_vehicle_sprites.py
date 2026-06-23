#!/usr/bin/env python3
"""Generate Time Fantasy-style vehicle BMPs for smart-road (A08).

Palette and proportions follow Jason Perry (finalbossblues) Time Fantasy /
Pixel Shooter aesthetic (16 px grid, SNES-style shading). Regenerate after
changing VEHICLE_LENGTH / VEHICLE_WIDTH in src/config.rs.
"""

from __future__ import annotations

import os
import struct

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "assets", "vehicles")

# Layout — keep in sync with src/config.rs
VEHICLE_LENGTH = 36
VEHICLE_WIDTH = 22

# Time Fantasy-inspired palette tuples (R, G, B, A)
TRANSPARENT = (0, 0, 0, 0)
SHADOW = (28, 28, 36, 255)
OUTLINE = (18, 18, 24, 255)
WINDOW = (140, 200, 220, 255)
HIGHLIGHT = (245, 245, 250, 255)

PALETTES = {
    "south": {
        "body": (192, 48, 48, 255),
        "body_dark": (120, 28, 28, 255),
        "accent": (230, 90, 70, 255),
    },
    "north": {
        "body": (56, 112, 200, 255),
        "body_dark": (32, 64, 128, 255),
        "accent": (90, 150, 230, 255),
    },
    "west": {
        "body": (210, 160, 32, 255),
        "body_dark": (130, 96, 18, 255),
        "accent": (240, 200, 80, 255),
    },
    "east": {
        "body": (48, 168, 72, 255),
        "body_dark": (28, 104, 44, 255),
        "accent": (90, 210, 110, 255),
    },
}


def write_bmp_rgba(path: str, width: int, height: int, pixels: list[list[tuple[int, int, int, int]]]) -> None:
    row_stride = ((width * 4 + 3) // 4) * 4
    pixel_data = bytearray()
    for row in reversed(pixels):
        row_bytes = bytearray()
        for r, g, b, a in row:
            row_bytes.extend((b, g, r, a))
        row_bytes.extend(b"\x00" * (row_stride - len(row_bytes)))
        pixel_data.extend(row_bytes)

    file_header_size = 14
    dib_header_size = 40
    offset = file_header_size + dib_header_size
    file_size = offset + len(pixel_data)

    file_header = struct.pack("<2sIHHI", b"BM", file_size, 0, 0, offset)
    dib_header = struct.pack(
        "<IiiHHIIiiII",
        dib_header_size,
        width,
        height,
        1,
        32,
        0,
        len(pixel_data),
        2835,
        2835,
        0,
        0,
    )

    with open(path, "wb") as f:
        f.write(file_header)
        f.write(dib_header)
        f.write(pixel_data)


def blank_canvas() -> list[list[tuple[int, int, int, int]]]:
    return [[TRANSPARENT for _ in range(VEHICLE_LENGTH)] for _ in range(VEHICLE_WIDTH)]


def set_px(pixels: list[list[tuple[int, int, int, int]]], x: int, y: int, color: tuple[int, int, int, int]) -> None:
    if 0 <= x < VEHICLE_LENGTH and 0 <= y < VEHICLE_WIDTH:
        pixels[y][x] = color


def fill_rect(
    pixels: list[list[tuple[int, int, int, int]]],
    x0: int,
    y0: int,
    w: int,
    h: int,
    color: tuple[int, int, int, int],
) -> None:
    for y in range(y0, y0 + h):
        for x in range(x0, x0 + w):
            set_px(pixels, x, y, color)


def draw_topdown_car(palette: dict[str, tuple[int, int, int, int]]) -> list[list[tuple[int, int, int, int]]]:
    """Eastbound (+x) top-down car; render.rs rotates via heading_rad."""
    px = blank_canvas()
    body = palette["body"]
    body_dark = palette["body_dark"]
    accent = palette["accent"]

    # Shadow underbody
    fill_rect(px, 4, 3, 28, 16, SHADOW)
    # Main hull
    fill_rect(px, 5, 4, 26, 14, body)
    fill_rect(px, 6, 5, 24, 12, accent)
    fill_rect(px, 7, 6, 22, 10, body)

    # Nose (east / +x)
    for dy, row in enumerate(
        [
            "......###.......",
            ".....#####......",
            "....#######.....",
            "...#########....",
            "..###########...",
            ".#############..",
        ]
    ):
        y = 8 + dy
        for x, ch in enumerate(row):
            if ch == "#":
                set_px(px, x, y, body_dark if dy >= 4 else body)

    # Cabin / windshield (west of nose)
    fill_rect(px, 10, 7, 10, 8, body_dark)
    fill_rect(px, 11, 8, 8, 6, WINDOW)
    fill_rect(px, 12, 9, 6, 4, HIGHLIGHT)

    # Rear deck
    fill_rect(px, 20, 7, 8, 8, body_dark)
    fill_rect(px, 21, 8, 6, 6, body)

    # Wheels (north/south of hull)
    for wx in (9, 10, 22, 23):
        fill_rect(px, wx, 2, 3, 4, OUTLINE)
        fill_rect(px, wx, 16, 3, 4, OUTLINE)
        fill_rect(px, wx + 1, 3, 1, 2, SHADOW)
        fill_rect(px, wx + 1, 17, 1, 2, SHADOW)

    # Outline accents
    for x in range(5, 31):
        set_px(px, x, 4, OUTLINE)
        set_px(px, x, 17, OUTLINE)
    for y in range(4, 18):
        set_px(px, 5, y, OUTLINE)
        set_px(px, 30, y, OUTLINE)

    return px


def main() -> None:
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, palette in PALETTES.items():
        path = os.path.join(OUT_DIR, f"vehicle_{name}.bmp")
        write_bmp_rgba(path, VEHICLE_LENGTH, VEHICLE_WIDTH, draw_topdown_car(palette))
        print(f"Wrote {path}")


if __name__ == "__main__":
    main()
