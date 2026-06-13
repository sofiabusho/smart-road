#!/usr/bin/env python3
"""Generate simple road tile BMPs for smart-road (A03). Stdlib only — no PIL."""

from __future__ import annotations

import os
import struct

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "assets", "roads")

ASPHALT = (58, 58, 62)
MARKING = (235, 210, 70)


def write_bmp(path: str, width: int, height: int, pixels: list[list[tuple[int, int, int]]]) -> None:
    row_stride = ((width * 3 + 3) // 4) * 4
    pixel_data = bytearray()
    for row in reversed(pixels):
        row_bytes = bytearray()
        for r, g, b in row:
            row_bytes.extend((b, g, r))
        row_bytes.extend(b"\x00" * (row_stride - len(row_bytes)))
        pixel_data.extend(row_bytes)

    file_header_size = 14
    dib_header_size = 40
    offset = file_header_size + dib_header_size
    file_size = offset + len(pixel_data)

    file_header = struct.pack(
        "<2sIHHI",
        b"BM",
        file_size,
        0,
        0,
        offset,
    )
    dib_header = struct.pack(
        "<IiiHHIIiiII",
        dib_header_size,
        width,
        height,
        1,
        24,
        0,
        len(pixel_data),
        2835,
        2835,
        0,
        0,
    )

    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(file_header)
        f.write(dib_header)
        f.write(pixel_data)


def fill(w: int, h: int, color: tuple[int, int, int]) -> list[list[tuple[int, int, int]]]:
    return [[color for _ in range(w)] for _ in range(h)]


def dashed_vertical(pixels: list, x: int, dash: int = 12, gap: int = 10) -> None:
    h = len(pixels)
    on = True
    y = 0
    while y < h:
        if on:
            for yy in range(y, min(y + dash, h)):
                pixels[yy][x] = MARKING
            y += dash
        else:
            y += gap
        on = not on


def dashed_horizontal(pixels: list, y: int, dash: int = 12, gap: int = 10) -> None:
    w = len(pixels[0])
    on = True
    x = 0
    while x < w:
        if on:
            for xx in range(x, min(x + dash, w)):
                pixels[y][xx] = MARKING
            x += dash
        else:
            x += gap
        on = not on


def road_vertical(width: int, height: int, lane_width: int) -> list[list[tuple[int, int, int]]]:
    pixels = fill(width, height, ASPHALT)
    dashed_vertical(pixels, lane_width)
    dashed_vertical(pixels, lane_width * 2)
    return pixels


def road_horizontal(width: int, height: int, lane_width: int) -> list[list[tuple[int, int, int]]]:
    pixels = fill(width, height, ASPHALT)
    dashed_horizontal(pixels, lane_width)
    dashed_horizontal(pixels, lane_width * 2)
    return pixels


def intersection_core(size: int, lane_width: int) -> list[list[tuple[int, int, int]]]:
    pixels = fill(size, size, ASPHALT)
    mid = size // 2
    dashed_vertical(pixels, mid - lane_width // 2)
    dashed_vertical(pixels, mid + lane_width // 2)
    dashed_horizontal(pixels, mid - lane_width // 2)
    dashed_horizontal(pixels, mid + lane_width // 2)
    return pixels


def main() -> None:
    lane_width = 40
    road_width = lane_width * 3
    window_half = 768 // 2
    approach_margin = 48
    arm_length = window_half - road_width // 2 - approach_margin

    write_bmp(
        os.path.join(OUT_DIR, "approach_ns.bmp"),
        road_width,
        arm_length,
        road_vertical(road_width, arm_length, lane_width),
    )
    write_bmp(
        os.path.join(OUT_DIR, "approach_ew.bmp"),
        arm_length,
        road_width,
        road_horizontal(arm_length, road_width, lane_width),
    )
    write_bmp(
        os.path.join(OUT_DIR, "intersection_core.bmp"),
        road_width,
        road_width,
        intersection_core(road_width, lane_width),
    )
    print(f"Wrote road assets to {OUT_DIR}")


if __name__ == "__main__":
    main()
