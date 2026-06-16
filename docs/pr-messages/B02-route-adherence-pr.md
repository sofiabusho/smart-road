---
title: "feat(B02): route adherence"
---

# PR Implementation Report: B02

## Summary

Implements **route adherence** for all 12 lanes per SDS §13.3. Each `LaneInfo` now carries a 4-waypoint polyline (`spawn → junction_entry → junction_exit → off_screen`); `advance_along_path()` moves the vehicle along that polyline each frame with remainder carry-over across segment boundaries and `heading_rad` tracking the current segment tangent. Satisfies **REQ-2** (three distinct routes per approach) and **REQ-6** (vehicles follow their designated route with no lane changes).

Two geometry bugs were found and fixed during review (see Per-Lane Geometry Table):

1. **path[0] mismatch** — all three routes per approach shared the left-lane hardcoded coordinate as `path[0]`; fixed by calling `spawn_point_for(approach, route)` for every lane.
2. **Approach-segment drift** — `path[1]` also used the left-lane coordinate, so right/straight vehicles drifted laterally on the approach arm; fixed by deriving `path[1]` from the same per-lane x/y offset and the junction-edge constant.

## Key Changes

- **`src/intersection.rs`** — Added `pub path: Vec<Vec2>` to `LaneInfo` (line 86); `pub type LanePathMap` (line 90); `pub fn attach_paths()` (line 163); `fn build_all_lane_paths()` (line 171) builds all 12 four-waypoint polylines with coordinates derived from `spawn_point_for()` and config constants; `IntersectionModel::new()` calls both at startup (lines 116–117). Two regression tests added: `all_lane_paths_start_at_spawn_point` and `approach_segment_is_axial_for_all_lanes`.
- **`src/vehicle.rs`** — `pub fn advance_along_path()` (line 84): moves vehicle along lane polyline each frame, carries remainder across segment boundaries, updates `heading_rad` from segment tangent.
- **`src/spawn.rs`** — `SpawnSystem::update` signature updated to `update(&mut self, model: &IntersectionModel, dt: f32)` (line 98); calls `advance_along_path` after `integrate_physics` each frame (line 104). Cross-track edit — see below.
- **`src/app.rs`** — Call site updated to `self.spawn.update(&self.intersection, FIXED_TIMESTEP_SECS)` (line 112). Cross-track edit — see below.

## Technical Decisions

- **`path[0]` via `spawn_point_for()`, not hardcoded**: Ensures spawn point and path start are bit-identical, guaranteed by sharing the same computation. Verified by `all_lane_paths_start_at_spawn_point` test.
- **`path[1]` preserves lane offset to junction edge**: The approach arm segment (`path[0]→path[1]`) keeps constant x (N/S lanes) or constant y (E/W lanes). `path[1]` is computed as `Vec2::new(lane_x, jy_n)` for North, etc., where `lane_x = spawn_point_for(approach, route).x`. This eliminates lateral drift on the approach. Verified by `approach_segment_is_axial_for_all_lanes` test.
- **Turn exit lanes derived from exit road's spawn coordinates**: `path[2]` for right/left turns uses the exit road's right-lane or left-lane y/x value, also derived from `spawn_point_for()` of the perpendicular approach. This keeps turn geometry consistent with Track A's lane layout.
- **Straight-through path[2]/path[3] use same lane offset**: Unlike turns, straight vehicles keep the same x or y through the full junction to the off-screen point.
- **`integrate_physics` + `advance_along_path` both called each frame**: B02 keeps both per spec. `integrate_physics` accumulates crossing metrics and velocity; `advance_along_path` overrides position and heading to enforce path adherence. B03/B04 will rebalance velocity authority.
- **Hardcoded 4-point polylines, not algorithmic**: Turn waypoints inside the junction involve diagonal steps that don't fall out cleanly from lane-center-offset arithmetic alone; explicit values (derived from config helpers, not magic numbers) are more auditable.

## Cross-Track Changes

| File | Owned by | Change | SDS §13.1 compliance |
|------|----------|--------|----------------------|
| `src/spawn.rs` | Track A | `update()` signature changed from `(&mut self, dt: f32)` to `(&mut self, model: &IntersectionModel, dt: f32)`; `advance_along_path` called after `integrate_physics` each frame | Necessary integration point per SDS §13.3; SDS §13.2 updated in this PR |
| `src/app.rs` | Track A | Call site updated to pass `&self.intersection` | Mechanical follow-on required by `spawn.rs` signature change |

`src/render.rs` and `src/config.rs` — **not touched**.

## Per-Lane Geometry Table

Coordinates verified by reasoning from config constants:
`INTERSECTION_CENTER_X=512, INTERSECTION_CENTER_Y=384, INTERSECTION_HALF_SIZE=60, LANE_WIDTH=40, APPROACH_MARGIN=48`.
Junction edges: west x=452, east x=572, north y=324, south y=444.

| Lane | Approach | Route | spawn / path[0] | path[1] (junction entry) | path[2] (junction exit) | path[3] (off-screen) | path[0]==spawn? | Axial approach? |
|------|----------|-------|-----------------|--------------------------|-------------------------|----------------------|-----------------|-----------------|
| 0 | North | Right | (552, 48) | (552, 324) | (452, 344) | (-64, 344) | ✅ | ✅ x=552 constant |
| 1 | North | Straight | (512, 48) | (512, 324) | (512, 444) | (512, 832) | ✅ | ✅ x=512 constant |
| 2 | North | Left | (472, 48) | (472, 324) | (572, 344) | (1088, 344) | ✅ | ✅ x=472 constant |
| 3 | South | Right | (472, 720) | (472, 444) | (572, 424) | (1088, 424) | ✅ | ✅ x=472 constant |
| 4 | South | Straight | (512, 720) | (512, 444) | (512, 324) | (512, -64) | ✅ | ✅ x=512 constant |
| 5 | South | Left | (552, 720) | (552, 444) | (452, 424) | (-64, 424) | ✅ | ✅ x=552 constant |
| 6 | East | Right | (976, 344) | (572, 344) | (552, 444) | (552, 832) | ✅ | ✅ y=344 constant |
| 7 | East | Straight | (976, 384) | (572, 384) | (452, 384) | (-64, 384) | ✅ | ✅ y=384 constant |
| 8 | East | Left | (976, 424) | (572, 424) | (552, 324) | (552, -64) | ✅ | ✅ y=424 constant |
| 9 | West | Right | (48, 424) | (452, 424) | (472, 324) | (472, -64) | ✅ | ✅ y=424 constant |
| 10 | West | Straight | (48, 384) | (452, 384) | (572, 384) | (1088, 384) | ✅ | ✅ y=384 constant |
| 11 | West | Left | (48, 344) | (452, 344) | (472, 444) | (472, 832) | ✅ | ✅ y=344 constant |

Axial approach = path[0] and path[1] share the same perpendicular-axis coordinate (no lateral drift on the approach arm).

## Verification Results

### Automated Checks (run 2026-06-16 on Linux, no SDL2 required for tests)

```
cargo test       — 37 tests: 33 unit + 4 smoke, 0 failed
cargo clippy -- -D warnings  — clean, no warnings
cargo fmt --check            — pass
cargo build                  — succeeds
```

Test names in `src/intersection.rs` (intersection module, 7 tests):
- `lane_registry_has_twelve_unique_lanes`
- `each_approach_has_three_routes`
- `lane_id_mapping_is_stable`
- `zone_polygon_is_axis_aligned_box`
- `spawn_points_sit_on_approach_edges`
- `approach_segment_is_axial_for_all_lanes` ← **new in this PR (regression)**
- `all_lane_paths_start_at_spawn_point` ← **new in this PR**

A03/A04 tests confirmed passing (render, spawn, input modules):
`road_asset_paths_are_under_assets_dir`, `layout_constants_fit_default_window`, `vehicle_dimensions_swap_for_ew_approaches`, `spawn_request_carries_lane_id`, `try_spawn_places_vehicle_on_lane_spawn_point`, `spawn_on_approach_rotates_routes`, `south/north/west/east_vehicle_moves_*`, `travel_heading_for_each_approach`, `arrow_*_spawns_*_approach`, `key_down_*`.

### Manual / Visual Audit (AUD-28)

AUD-28 requires observing vehicles on each approach following fixed r/s/l routes with no lane changes. **This has not been visually confirmed in this environment — no local SDL2 display is available.** The geometry correctness is verified by:

- `approach_segment_is_axial_for_all_lanes`: asserts `path[0].x == path[1].x` (N/S) and `path[0].y == path[1].y` (E/W) for all 12 lanes — guarantees zero lateral drift on approach arms.
- `all_lane_paths_start_at_spawn_point`: asserts `lane.path[0] == lane.spawn_point` for all 12 lanes — guarantees vehicle starts exactly on its polyline.
- Coordinate derivation from `spawn_point_for()` and config constants (see table above) — geometric consistency with Track A's lane layout.

A full visual AUD-28 pass requires running the binary with an SDL2-capable display. The test suite provides mechanical correctness coverage; visual confirmation is deferred to the reviewer or CI environment with SDL2.

## Requirements Traceability

| Requirement | What it requires | File | Line | What was added |
|-------------|-----------------|------|------|----------------|
| REQ-2 | Three distinct routes (r/s/l) per approach | `src/intersection.rs` | 171 | `build_all_lane_paths()` — 12 polylines, one per approach×route |
| REQ-6 | Vehicles follow designated route, no lane changes | `src/vehicle.rs` | 84 | `advance_along_path()` — locks vehicle to its lane polyline each frame |
| REQ-5 (B01) | `velocity = distance/time` fields | `src/vehicle.rs` | 29–33 | `velocity`, `distance_in_crossing`, `time_in_crossing` fields; `integrate_physics` accumulates at lines 77–80 |
| SDS §13.3 | `LanePathMap` type exported | `src/intersection.rs` | 90 | `pub type LanePathMap = HashMap<LaneId, Vec<Vec2>>` |
| SDS §13.3 | `attach_paths()` exported | `src/intersection.rs` | 163 | `pub fn attach_paths(model: &mut IntersectionModel, paths: LanePathMap)` |
| SDS §13.2 | `SpawnSystem::update` signature | `src/spawn.rs` | 98 | Updated to `(&mut self, model: &IntersectionModel, dt: f32)` |
| REQ-11 prep (A07) | Heading from path tangent | `src/vehicle.rs` | 109 | `vehicle.heading_rad = seg_dy.atan2(seg_dx)` on each segment |

## Artifacts

```
cargo test output (2026-06-16):
  running 33 tests (unit) ... ok
  running 4 tests (smoke) ... ok
  test result: ok. 37 passed; 0 failed

cargo clippy -- -D warnings: Finished with no warnings
cargo fmt --check: pass
cargo build: Finished dev profile
```

## Next Steps

- **B03** — Velocity levels: `VelocityLevel` enum (Fast/Cruise/Yield ≥ 3 levels); `integrate_physics` reads `commanded_velocity`.
- **B04** — Safe distance: `enforce_follow_distance()`; collision-avoidance for vehicles on the same lane.
- **A07** (unblocked by B02): Turn animation using `heading_rad` tangent now updated by `advance_along_path`.
- **C01** (unblocked by B02): Smart detection reads `vehicle.position` against `zone_polygon`; path-following makes zone entry deterministic.
- **Visual AUD-28**: Reviewer should run binary on SDL2-capable host and observe all 4 approaches × 3 routes for no lane crossing.
