---
title: "feat(A03): intersection render"
---

# PR Implementation Report: A03

## Summary

Implements the **cross intersection render** and **lane ID registry stub** for Track A. The simulation window now draws a four-way junction from BMP road tile assets, and `IntersectionModel` exposes 12 lanes (4 approaches × 3 routes), spawn points, and a `zone_polygon` for downstream smart-system detection. Satisfies **REQ-1**, **REQ-2** (registry stub), **REQ-10**, **AUD-1**, and **AUD-2**.

## Key Changes

- **`src/intersection.rs`**: `Vec2`, `LaneInfo` with `spawn_point`, `lane_id()` registry, `zone_polygon`, lookup helpers, unit tests.
- **`src/render.rs`**: `RoadAssets` BMP loader, `draw_intersection` / `draw_frame`, layout math, `VehicleRenderSnapshot` stub.
- **`src/config.rs`**: layout constants (`LANE_WIDTH`, `ROAD_WIDTH`, `INTERSECTION_CENTER_*`, `APPROACH_ARM_LENGTH`).
- **`src/app.rs`**: grass-green backdrop + intersection draw each frame.
- **`assets/roads/`**: `approach_ns.bmp`, `approach_ew.bmp`, `intersection_core.bmp`.
- **`scripts/generate_road_assets.py`**: stdlib BMP generator for road tiles.
- **`tests/smoke.rs`**: lane registry integration smoke test.
- **`README.md`**: run description and asset regeneration notes.

## Technical Decisions

- **BMP via core SDL2**: Road tiles load with `Surface::load_bmp` (no `sdl2` `image` feature / `libsdl2-image-dev` required for A03). Satisfies AUD-2 asset requirement; PNG/SDL2_image deferred to vehicle sprites (A07+).
- **Per-frame texture load**: Textures are created each draw call to avoid self-referential `Canvas`/`Texture` lifetimes. Acceptable for three small tiles at A03 scope.
- **Lane ID scheme**: `LaneId(approach_index * 3 + route_index)` — stable for B02 path attachment and A04 spawn.
- **Zone polygon**: Axis-aligned square around junction center (`INTERSECTION_HALF_SIZE`) for C01 detection contract.

## Verification Results

### Automated Checks

- [x] `cargo test` — 13 tests passed (10 unit + 3 integration)
- [x] `cargo clippy -- -D warnings` — clean
- [x] `cargo fmt --check` — clean
- [x] `cargo build` — succeeds

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-1**: Pass — `cargo run` shows four-way cross layout (N/S/E/W arms + center junction).
- [x] **AUD-2**: Pass — intersection rendered from `assets/roads/*.bmp` tile assets (not solid-color placeholder).

### Requirements Traceability

- [x] **REQ-1**: Cross intersection visible with four approaches meeting at central junction.
- [x] **REQ-2**: Lane registry stub — 12 lanes with fixed `r`/`s`/`l` routes per approach (paths in B02).
- [x] **REQ-10**: SDL2 rendering with road image assets and defined world coordinate system in `config.rs`.

## Artifacts

- **Test output**:
  ```text
  running 10 tests ... ok
  running 3 tests (smoke) ... ok
  ```
- **Lint output**: clippy clean with `-D warnings`
- **PR message**: `docs/pr-messages/A03-intersection-render-pr.md`

---

## Next Steps

- **A04** — arrow-key spawn (`SpawnRequest` API, four cardinal approaches)
- **B02** — attach `path: Vec<Vec2>` polylines to `LaneInfo` (after B01 + A03 ✅)
