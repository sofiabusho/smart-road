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


def solid_vertical(pixels: list, x: int) -> None:
    h = len(pixels)
    for y in range(h):
        pixels[y][x] = MARKING


def solid_horizontal(pixels: list, y: int) -> None:
    w = len(pixels[0])
    for x in range(w):
        pixels[y][x] = MARKING


def road_vertical(width: int, height: int, lane_width: int) -> list[list[tuple[int, int, int]]]:
    """Six-lane vertical arm: inbound (west) + outbound (east) with center divider."""
    pixels = fill(width, height, ASPHALT)
    inbound_lanes = 3
    # Inbound lane dividers (west half).
    dashed_vertical(pixels, lane_width)
    dashed_vertical(pixels, lane_width * 2)
    # Center divider (double solid).
    mid = inbound_lanes * lane_width
    solid_vertical(pixels, mid - 1)
    solid_vertical(pixels, mid)
    # Outbound lane dividers (east half).
    dashed_vertical(pixels, mid + lane_width)
    dashed_vertical(pixels, mid + lane_width * 2)
    return pixels


def road_horizontal(width: int, height: int, lane_width: int) -> list[list[tuple[int, int, int]]]:
    """Six-lane horizontal arm: inbound (north) + outbound (south) with center divider."""
    pixels = fill(width, height, ASPHALT)
    inbound_lanes = 3
    dashed_horizontal(pixels, lane_width)
    dashed_horizontal(pixels, lane_width * 2)
    mid = inbound_lanes * lane_width
    solid_horizontal(pixels, mid - 1)
    solid_horizontal(pixels, mid)
    dashed_horizontal(pixels, mid + lane_width)
    dashed_horizontal(pixels, mid + lane_width * 2)
    return pixels


def intersection_core(size: int, lane_width: int) -> list[list[tuple[int, int, int]]]:
    """Six-lane junction core with center dividers on both axes."""
    pixels = fill(size, size, ASPHALT)
    inbound_lanes = 3
    mid = inbound_lanes * lane_width

    # Inbound lane markings.
    dashed_vertical(pixels, lane_width)
    dashed_vertical(pixels, lane_width * 2)
    dashed_horizontal(pixels, lane_width)
    dashed_horizontal(pixels, lane_width * 2)

    # Center dividers.
    solid_vertical(pixels, mid - 1)
    solid_vertical(pixels, mid)
    solid_horizontal(pixels, mid - 1)
    solid_horizontal(pixels, mid)

    # Outbound lane markings.
    dashed_vertical(pixels, mid + lane_width)
    dashed_vertical(pixels, mid + lane_width * 2)
    dashed_horizontal(pixels, mid + lane_width)
    dashed_horizontal(pixels, mid + lane_width * 2)

    return pixels


def main() -> None:
    # Layout constants — must match src/config.rs (regenerate assets after changes).
    window_width = 1024
    window_height = 768
    lane_width = 40
    lanes_per_approach = 3
    lanes_per_arm = lanes_per_approach * 2
    approach_margin = 48

    road_arm_width = lane_width * lanes_per_arm
    ns_arm_length = window_height // 2 - road_arm_width // 2 - approach_margin
    ew_arm_length = window_width // 2 - road_arm_width // 2 - approach_margin

    write_bmp(
        os.path.join(OUT_DIR, "approach_ns.bmp"),
        road_arm_width,
        ns_arm_length,
        road_vertical(road_arm_width, ns_arm_length, lane_width),
    )
    write_bmp(
        os.path.join(OUT_DIR, "approach_ew.bmp"),
        ew_arm_length,
        road_arm_width,
        road_horizontal(ew_arm_length, road_arm_width, lane_width),
    )
    write_bmp(
        os.path.join(OUT_DIR, "intersection_core.bmp"),
        road_arm_width,
        road_arm_width,
        intersection_core(road_arm_width, lane_width),
    )
    print(f"Wrote road assets to {OUT_DIR}")


if __name__ == "__main__":
    main()
