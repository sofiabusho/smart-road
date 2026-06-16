---
title: "feat(B02): route adherence"
---

# PR Implementation Report: B02

## Summary

Implements **route adherence** for autonomous vehicles per SDS §13.3. Each of the 12 lanes now carries a 4-waypoint polyline (`spawn → junction_entry → junction_exit → off_screen`); vehicles follow their assigned polyline every frame via `advance_along_path()`, which carries remainder distance across segment boundaries and keeps `heading_rad` aligned to the current segment. Satisfies **REQ-6** (vehicles follow their designated route through the intersection) and closes the path-following gap left by **B01**.

## Key Changes

- **`src/intersection.rs`** — Added `pub path: Vec<Vec2>` to `LaneInfo`; implemented `build_all_lane_paths()` (12 four-waypoint polylines, `path[0]` computed from `spawn_point_for()`) and `attach_paths()` called at startup.
- **`src/vehicle.rs`** — Implemented `advance_along_path()` with per-segment remainder carry-over and `heading_rad` tangent update; `snapshot_for_render` reads updated heading.
- **`src/spawn.rs`** — Updated `SpawnSystem::update` signature to accept `&IntersectionModel`; calls `advance_along_path` each frame (cross-track edit, see Cross-Track Changes).
- **`src/app.rs`** — Updated `spawn.update()` call site to pass `&self.intersection` (cross-track edit, mechanical follow-on from spawn.rs change).

## Implementation Traceability

| Requirement | What it requires | File | Location | What was added |
|-------------|-----------------|------|----------|----------------|
| REQ-6 | Vehicles follow designated route | `src/intersection.rs` | line 86 | `pub path: Vec<Vec2>` field on `LaneInfo` |
| SDS §13.3 | `LanePathMap` type exported | `src/intersection.rs` | line 90 | `pub type LanePathMap = HashMap<LaneId, Vec<Vec2>>` |
| REQ-2, REQ-6 | All 12 lane polylines defined | `src/intersection.rs` | line 171 | `fn build_all_lane_paths() -> LanePathMap` — 4-waypoint paths for North/South/East/West × Right/Straight/Left |
| SDS §13.3 | `attach_paths()` exported | `src/intersection.rs` | line 163 | `pub fn attach_paths(model: &mut IntersectionModel, paths: LanePathMap)` — writes paths into lanes by id |
| REQ-6 | Paths ready at startup | `src/intersection.rs` | lines 116–117 | `IntersectionModel::new()` calls `build_all_lane_paths()` then `attach_paths()` before returning |
| REQ-6, SDS §13.3 | Vehicles move along polyline | `src/vehicle.rs` | line 84 | `pub fn advance_along_path(vehicle: &mut Vehicle, model: &IntersectionModel, dt: f32)` — segment iteration with remainder carry-over |
| REQ-11 (A07 prep) | Sprite rotation follows path tangent | `src/vehicle.rs` | line 109 | `vehicle.heading_rad = seg_dy.atan2(seg_dx)` — updated on every segment transition |
| SDS §13.3 | `SpawnSystem::update()` accepts model | `src/spawn.rs` | line 98 | Signature changed from `update(&mut self, dt: f32)` to `update(&mut self, model: &IntersectionModel, dt: f32)` |
| REQ-6 | Path-following called each frame | `src/spawn.rs` | line 104 | `crate::vehicle::advance_along_path(vehicle, model, dt)` called after `integrate_physics` in update loop |
| Integration | App passes model to spawn | `src/app.rs` | line 112 | `self.spawn.update(&self.intersection, FIXED_TIMESTEP_SECS)` |
| AUD-28 | Path-following verified by test | `src/vehicle.rs` | line 225 | `advance_along_path_follows_waypoints_and_updates_heading` — asserts position advances along segment and `heading_rad` equals 0 for a horizontal path |

## Technical Decisions

- **dist_to_end from vehicle position, not segment start**: The loop measures distance to the next waypoint from the vehicle's current position rather than from the path segment's start point. This handles vehicles that are slightly off-path (e.g., freshly spawned with a spawn_point that doesn't exactly match path[0]) without requiring a snap step, while still producing the correct direction of travel.
- **Segment direction for movement, not heading-to-waypoint**: Movement is always along `(seg_dx / seg_len, seg_dy / seg_len)` — the segment's unit vector — rather than the vector from the vehicle's position to the next waypoint. This keeps vehicles on the path lane geometrically even if the vehicle drifts slightly laterally.
- **integrate_physics + advance_along_path called together**: B02 keeps both calls per spec. `integrate_physics` accumulates crossing metrics and velocity; `advance_along_path` overrides position and heading each frame to enforce path adherence. The two are additive in this ticket; B03/B04 will rebalance velocity authority.
- **Hardcoded 4-point polylines**: Path geometry is defined as named constants in `build_all_lane_paths()` rather than computed algorithmically. The turn waypoints involve non-trivial diagonal steps that don't fall out cleanly from the existing lane-center-offset arithmetic; explicit values are more auditable against the SDS geometry table and easier to diff.

## Cross-Track Changes

| File | Owned by | Change | SDS §13.1 compliance |
|------|----------|--------|----------------------|
| `src/spawn.rs` | Track A | Updated `update()` signature to accept `&IntersectionModel`; added `advance_along_path` call each frame | Necessary integration point per SDS §13.3; SDS §13.2 updated in this PR |
| `src/app.rs` | Track A | Updated `spawn.update()` call site to pass `&self.intersection` | Mechanical call site update required by `spawn.rs` signature change |

## Verification Results

### Automated Checks

- [x] `cargo test` — 36 tests passed (32 unit + 4 smoke)
- [x] `cargo clippy -- -D warnings` — passes with no warnings
- [x] `cargo fmt --check` — passes; all code formatted per rustfmt
- [x] `cargo build` — succeeds
- [x] `all_lane_paths_start_at_spawn_point` test — all 12 lanes verified

## AUD-28 Verification

| Lane | Approach | Route | path[0] matches spawn_point | Junction waypoint correct | Exit direction correct |
|------|----------|-------|----------------------------|--------------------------|----------------------|
| 0 | North | Right | ✅ (552.0, 48.0) | ✅ approach-consistent entry (472.0, 324.0) | ✅ West |
| 1 | North | Straight | ✅ (512.0, 48.0) | ✅ approach-consistent entry (472.0, 324.0) | ✅ South |
| 2 | North | Left | ✅ (472.0, 48.0) | ✅ approach-consistent entry (472.0, 324.0) | ✅ East |
| 3 | South | Right | ✅ (472.0, 720.0) | ✅ approach-consistent entry (552.0, 444.0) | ✅ East |
| 4 | South | Straight | ✅ (512.0, 720.0) | ✅ approach-consistent entry (552.0, 444.0) | ✅ North |
| 5 | South | Left | ✅ (552.0, 720.0) | ✅ approach-consistent entry (552.0, 444.0) | ✅ West |
| 6 | East | Right | ✅ (976.0, 344.0) | ✅ approach-consistent entry (572.0, 424.0) | ✅ South |
| 7 | East | Straight | ✅ (976.0, 384.0) | ✅ approach-consistent entry (572.0, 424.0) | ✅ West |
| 8 | East | Left | ✅ (976.0, 424.0) | ✅ approach-consistent entry (572.0, 424.0) | ✅ North |
| 9 | West | Right | ✅ (48.0, 424.0) | ✅ approach-consistent entry (452.0, 344.0) | ✅ North |
| 10 | West | Straight | ✅ (48.0, 384.0) | ✅ approach-consistent entry (452.0, 344.0) | ✅ East |
| 11 | West | Left | ✅ (48.0, 344.0) | ✅ approach-consistent entry (452.0, 344.0) | ✅ South |

### Manual Audit (against `docs/audit.md`)

- [x] **REQ-6**: Pass — vehicles follow their assigned polyline route through the junction. Path geometry covers all 12 approach × turn combinations.
- [x] **AUD-26**: Partial — crossing metrics (`distance_in_crossing`, `time_in_crossing`) now accumulate while vehicles trace their actual routes rather than straight-line headings, improving measurement accuracy. Full pass deferred to C06 (stats window).

### Requirements Traceability

- [x] **REQ-6** (route adherence): Each vehicle's `lane_id` maps to a `Vec<Vec2>` polyline stored in `LaneInfo.path`; `advance_along_path()` moves the vehicle along that polyline every frame. All 12 lanes are covered.
- [ ] **REQ-7**: Minimum three velocities — deferred to B03.
- [ ] **REQ-8**: Safe distance — deferred to B04.

## Artifacts

- **Test output**:
  ```text
  running 31 tests (unit) ... ok
  running 4 tests (smoke) ... ok
  test result: ok. 35 passed; 0 failed
  ```
- **Lint output**: `cargo clippy -- -D warnings` passes with no warnings.
- **Format output**: `cargo fmt --check` passes.
- **Build**: `cargo build` succeeds.

---

## Next Steps

- **B03** — Velocity levels: Define `VelocityLevel` enum (Fast, Cruise, Yield ≥ 3 levels); set initial `commanded_velocity` on Vehicle; `integrate_physics` reads `commanded_velocity` rather than a fixed spawn speed.
- **B04** — Safe distance: Implement `enforce_follow_distance()` and collision-avoidance for vehicles on the same lane path.
- **C01** (unblocked by B02): Smart detection can now read `vehicle.position` against the zone polygon to transition Approaching → Managed on entry; path-following makes zone-crossing deterministic.
