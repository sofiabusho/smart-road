# PR Review: A03 — Intersection Render

**Branch**: `sbusho/A03`
**Ticket**: A03 — Intersection render: cross layout, road assets, lane ID registry stub
**Reviewer**: Claude Code (automated review)
**Date**: 2026-06-13
**Verdict**: ✅ PASS WITH FINDINGS

---

## 1. Quality Gates

| Check | Result | Notes |
|-------|--------|-------|
| `cargo test` | ✅ Pass | 10 unit + 3 integration = 13 tests, 0 failures |
| `cargo clippy -- -D warnings` | ✅ Pass | Clean, no warnings |
| `cargo fmt --check` | ✅ Pass | No formatting drift |
| `cargo build` | ✅ Pass | Compiles without errors |

---

## 2. REQ / AUD Traceability

| ID | Claimed | Verified | Evidence |
|----|---------|----------|----------|
| REQ-1 | ✅ | ✅ | `IntersectionModel::new()` builds 4-approach cross topology; `draw_intersection` renders all 5 tiles (core + 4 arms) |
| REQ-2 | ✅ (stub) | ✅ | 12 lanes at 4 approaches × 3 routes (Right/Straight/Left), stable `LaneId` registry; path polylines correctly deferred to B02 |
| REQ-10 | ✅ | ✅ | BMP tiles loaded via `Surface::load_bmp` (no sdl2-image dependency); `assets/roads/approach_ns.bmp`, `approach_ew.bmp`, `intersection_core.bmp` all present |
| AUD-1 | ✅ | ✅ (code) | Topology + draw path covers all 4 arms and junction core; manual visual verification required at `cargo run` |
| AUD-2 | ✅ | ✅ | Assets loaded from `assets/roads/*.bmp`, not solid-colour placeholder |

No REQ/AUD IDs are claimed without corresponding implementation in the diff.

---

## 3. Cross-Track File Ownership (SDS §13.1)

| File changed | Owner (SDS §13.1) | Status |
|---|---|---|
| `src/intersection.rs` | **A** (topology) + **B** (paths) | ✅ A only touches topology/registry; `path: Vec<Vec2>` deferred to B02 |
| `src/render.rs` | **A** (A03/A07) | ✅ |
| `src/config.rs` | **A** | ✅ |
| `src/app.rs` | **A** | ✅ |
| `assets/roads/` | **A** (render assets) | ✅ |
| `scripts/generate_road_assets.py` | Not in SDS | ✅ No cross-track concern |
| `tests/smoke.rs` | Shared | ✅ Only adds A03-scope lane registry assertions |
| `docs/pr-messages/`, `docs/ticket-tracker.md` | Documentation | ✅ |

**No cross-track violations.** B and C owned files (`vehicle.rs`, `smart.rs`, `stats.rs`, `stats_window.rs`) are untouched.

---

## 4. Findings

### F1 — Per-frame texture load (Medium)

**Location**: `src/app.rs` `App::draw()`, `src/render.rs` `RoadAssets::load`

```rust
// app.rs — called every frame
let texture_creator = self.canvas.texture_creator();
let road_assets = render::RoadAssets::load(&texture_creator)?;
```

`RoadAssets::load` opens and reads 3 BMP files from disk and creates 3 SDL2 textures on every draw call (~60× per second). The PR correctly identifies this as a workaround for SDL2 `Canvas`/`Texture` self-referential lifetimes and calls it acceptable at A03 scope.

**Risk**: This pattern does not scale. A07 will add per-vehicle sprite loading. If the same pattern is applied there, it becomes 60 × (3 road + N vehicle) disk reads per second, causing visible frame drops.

**Recommendation**: Resolve before A07. Store `RoadAssets` in `App` struct or use a texture atlas / `unsafe` raw pointer approach that SDL2 crate supports via `TextureCreator`.

---

### F2 — EW arm length asymmetric with window aspect ratio (Medium)

**Location**: `src/config.rs:39-40`, `scripts/generate_road_assets.py`

```rust
// config.rs
pub const APPROACH_ARM_LENGTH: f32 =
    INTERSECTION_CENTER_Y - INTERSECTION_HALF_SIZE - APPROACH_MARGIN;
// = 384 - 60 - 48 = 276 px
```

The constant is derived from the **Y axis** (window height 768). For a 1024×768 window, the EW arm should extend `INTERSECTION_CENTER_X - INTERSECTION_HALF_SIZE - APPROACH_MARGIN = 512 - 60 - 48 = 404 px` to reach the approach margins. With 276px arms, EW sides have a 128px gap of exposed grass background on each side. NS arms are flush to `APPROACH_MARGIN = 48px`; EW arms reach only to pixel 176 from the window edge.

The Python script hardcodes `window_half = 768 // 2` which encodes the same assumption.

**Consequences**: AUD-1 ("cross layout clearly visible") still passes because four arms are present and the junction is recognisable, but the visual symmetry is broken in a 1024×768 window.

**Recommendation**: Add a second constant `EW_ARM_LENGTH = INTERSECTION_CENTER_X - INTERSECTION_HALF_SIZE - APPROACH_MARGIN` and resize `approach_ew.bmp` accordingly, or switch to a square window (768×768).

---

### F3 — Orphaned doc comment in config.rs (Low)

**Location**: `src/config.rs:18-19`

```rust
/// World coordinate system: origin top-left, +x east, +y south (SDL convention).
/// Lane width in world units (pixels at 1:1 scale).
pub const LANE_WIDTH: f32 = 40.0;
```

The first line was intended as a standalone section comment but is attached to `LANE_WIDTH` as its `rustdoc` comment. `rustdoc` will render "World coordinate system… Lane width…" together on `LANE_WIDTH`. The coordinate-system note belongs as a `//` comment above the block, not as `///` on a constant.

---

### F4 — Python script constants not tied to Rust config (Low)

**Location**: `scripts/generate_road_assets.py`

`generate_road_assets.py` hardcodes `lane_width = 40`, `window_half = 768 // 2`, `approach_margin = 48` independently of `config.rs`. If any layout constant changes, the script must be updated manually or assets will be stale.

**Recommendation**: Add a cross-reference comment in both files, or note in README that asset regeneration is required after layout constant changes.

---

## 5. Positive Observations

- **Lane ID scheme** (`approach_index * 3 + route_index`) is stable and documented; B02 can attach paths without changing IDs.
- **`zone_polygon`** is correctly plumbed as an axis-aligned box around the junction, giving C01 a well-defined managed zone boundary without needing coordinate recalculation.
- **`VehicleRenderSnapshot` stub** and `draw_vehicle` no-op are correctly scoped to A07, preventing over-engineering at A03.
- **Smoke tests** cover lane registry, zone polygon, and approach-count invariants — good integration coverage for downstream tracks.
- **Test assertions** (`spawn_points_sit_on_approach_edges`, `zone_polygon_is_axis_aligned_box`) will catch regressions if config constants change.
- **`Default` implemented** on `IntersectionModel` via explicit `impl Default` (not `#[derive(Default)]`) — correct given non-trivial constructor logic.

---

## 6. Verdict Summary

| Category | Status |
|----------|--------|
| Automated quality gates | ✅ All pass |
| REQ-1, REQ-2, REQ-10 traceability | ✅ Covered |
| AUD-1, AUD-2 traceability | ✅ Code-verified; AUD-1 needs manual `cargo run` confirmation |
| Cross-track ownership (SDS §13.1) | ✅ No violations |
| Blocking findings | None |
| Non-blocking findings | F1 (medium, address before A07), F2 (medium, visual defect), F3–F4 (low) |

**PASS** — A03 satisfies its acceptance criteria. F1 and F2 are recommended to address before A07 to prevent compounding tech debt. No blockers.
