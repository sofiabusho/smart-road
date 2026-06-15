---
title: "feat(B02): route adherence"
---

# PR Implementation Report: B02

## Summary

Implements **route adherence** for autonomous vehicles per SDS §13.3. Each of the 12 lanes now carries a 4-waypoint polyline (`spawn → junction_entry → junction_exit → off_screen`); vehicles follow their assigned polyline every frame via `advance_along_path()`, which carries remainder distance across segment boundaries and keeps `heading_rad` aligned to the current segment. Satisfies **REQ-6** (vehicles follow their designated route through the intersection) and closes the path-following gap left by **B01**.

## Key Changes

- **`src/intersection.rs`**: Added `path: Vec<Vec2>` field to `LaneInfo` (line 86); `LanePathMap` type alias (line 90); `attach_paths(model, paths)` public function (line 163) that writes path vecs into each lane by id; private `build_all_lane_paths()` (line 171) with hardcoded 4-point polylines for all 12 lanes (North/South/East/West × Right/Straight/Left); `IntersectionModel::new()` updated to call both so paths are ready at startup.
- **`src/vehicle.rs`**: `advance_along_path(vehicle, model, dt)` (line 84) — looks up the lane path, iterates segments moving `velocity × dt` units, carries remainder across waypoints, sets `heading_rad = seg_dy.atan2(seg_dx)` on every segment update, stops at final waypoint. New test `advance_along_path_follows_waypoints_and_updates_heading` (line 225).
- **`src/spawn.rs`**: `SpawnSystem::update()` signature extended to `update(&mut self, model: &IntersectionModel, dt: f32)` (line 98); now calls `integrate_physics` then `advance_along_path` each frame. Four existing directional tests updated to pass `&IntersectionModel::new()` (lines 179–219).
- **`src/app.rs`**: `self.spawn.update(FIXED_TIMESTEP_SECS)` updated to `self.spawn.update(&self.intersection, FIXED_TIMESTEP_SECS)`.

## Technical Decisions

- **dist_to_end from vehicle position, not segment start**: The loop measures distance to the next waypoint from the vehicle's current position rather than from the path segment's start point. This handles vehicles that are slightly off-path (e.g., freshly spawned with a spawn_point that doesn't exactly match path[0]) without requiring a snap step, while still producing the correct direction of travel.
- **Segment direction for movement, not heading-to-waypoint**: Movement is always along `(seg_dx / seg_len, seg_dy / seg_len)` — the segment's unit vector — rather than the vector from the vehicle's position to the next waypoint. This keeps vehicles on the path lane geometrically even if the vehicle drifts slightly laterally.
- **integrate_physics + advance_along_path called together**: B02 keeps both calls per spec. `integrate_physics` accumulates crossing metrics and velocity; `advance_along_path` overrides position and heading each frame to enforce path adherence. The two are additive in this ticket; B03/B04 will rebalance velocity authority.
- **Hardcoded 4-point polylines**: Path geometry is defined as named constants in `build_all_lane_paths()` rather than computed algorithmically. The turn waypoints involve non-trivial diagonal steps that don't fall out cleanly from the existing lane-center-offset arithmetic; explicit values are more auditable against the SDS geometry table and easier to diff.

## Verification Results

### Automated Checks

- [x] `cargo test` — 35 tests passed (31 unit + 4 smoke)
- [x] `cargo clippy -- -D warnings` — passes with no warnings
- [x] `cargo fmt --check` — passes; all code formatted per rustfmt
- [x] `cargo build` — succeeds

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
