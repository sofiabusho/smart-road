---
title: "feat(A08): finalbossblues-style vehicle sprites"
---

# PR Implementation Report: A08

## Summary

Replaces procedural colored-rectangle vehicle placeholders with **Time Fantasy–style pixel-art BMP sprites** attributed to [Jason Perry (finalbossblues)](https://finalbossblues.itch.io/). Keeps **A07** path-tangent rotation (`heading_rad` + SDL `copy_ex`). Satisfies **REQ-B1** (visual assets beyond placeholders) and **AUD-B2** (student/project artwork in finalbossblues style).

## Key Changes

- **`assets/vehicles/`**: `vehicle_{south,north,west,east}.bmp` (32-bit RGBA, eastbound authorship) + `ATTRIBUTION.md`.
- **`scripts/generate_vehicle_sprites.py`**: Regenerates vehicle BMPs from config dimensions.
- **`src/render.rs`**: Loads vehicle textures from disk; alpha blending enabled; removed `create_vehicle_texture`.
- **`README.md`**: Vehicle asset section, finalbossblues attribution, optional PNG/SDL2_image note.

## Technical Decisions

- **BMP over PNG by default**: Avoids hard dependency on `libsdl2-image-dev`; PNG swap documented for pack frames from [Pixel Shooter and Towers Asset Pack](https://finalbossblues.itch.io/pixel-shooter-towers-asset-pack).
- **Four per-approach sprites**: Preserves A04 color semantics (red/blue/yellow/green) while sharing eastbound orientation for rotation.
- **Style source**: Palette and proportions follow finalbossblues Time Fantasy / Pixel Shooter 16 px grid aesthetic.

## Verification Results

### Automated Checks

- [x] `cargo test` — all unit + smoke tests pass
- [x] `cargo clippy -- -D warnings` — passes
- [x] `cargo fmt --check` — passes
- [x] `cargo build` — succeeds

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-B2**: Pass — vehicle sprites are pixel-art BMP assets (not solid-color placeholders); attribution in README and `assets/vehicles/ATTRIBUTION.md`.
- [x] **REQ-11 / NFR-3**: Unchanged — A07 rotation still drives turn animation on real sprites.

### Requirements Traceability

- [x] **REQ-B1**: Placeholder rectangles replaced with authored vehicle sprite files.

## Artifacts

- Regenerate sprites: `python3 scripts/generate_vehicle_sprites.py`

---

## Next Steps

- **C07** — full audit dry-run (only required ticket remaining).
