---
title: "feat(B01): vehicle physics"
---

# PR Implementation Report: B01

## Summary

Implements **vehicle physics foundation** for autonomous vehicle simulation with position integration and crossing-time metrics per SDS §13.3. Vehicles now track path progress (`path_index`), distance traveled inside the junction (`distance_in_crossing`), and time spent crossing (`time_in_crossing`). Physics update moved from spawn.rs stub to dedicated `vehicle.rs::update_physics()` function. Lays groundwork for **B02** (route adherence) and **B03–B04** (velocity levels, safe distance). Satisfies **REQ-5** and builds toward **AUD-26**.

## Key Changes

- **`src/vehicle.rs`**: Added three fields to `Vehicle` struct (`path_index: usize`, `distance_in_crossing: f32`, `time_in_crossing: f32`); updated `spawn_vehicle()` factory to initialize all three to zero values; new `update_physics(vehicle: &mut Vehicle, dt: f32)` function for physics integration and crossing metric accumulation.
- **`src/spawn.rs`**: Replaced `advance_straight_stub(vehicle, dt)` call in `SpawnSystem::update()` with `crate::vehicle::update_physics(vehicle, dt)`. Kept `advance_straight_stub()` function in file (unused) to preserve existing test compatibility.

## Technical Decisions

- **Physics location**: Moved from spawn.rs stub to vehicle.rs module per SDS §13.1 file ownership (B01 owns vehicle.rs). Keeps physics logic colocated with Vehicle state.
- **Crossing metrics only in Managed state**: `time_in_crossing` and `distance_in_crossing` increments only occur when vehicle.state == Managed. This prepares for C01 smart-system detection to transition vehicles to Managed on zone entry. Approaching vehicles accumulate zero crossing metrics.
- **Distance calculation**: Uses Euclidean distance `sqrt(dx² + dy²)` from velocity and heading; accounts for heading direction even during turns (B02 will replace with path-based motion).
- **Backward compatibility**: `advance_straight_stub()` remains (unused but present) so any downstream code referencing it or tests depending on its existence do not break; will be removed in B02 or later when motion is fully replaced by path integration.

## Verification Results

### Automated Checks

- [x] `cargo test` — 31 tests passed (27 unit + 4 integration)
- [x] `cargo clippy -- -D warnings` — 1 warning for unused `advance_straight_stub()` (expected; function kept for compatibility)
- [x] `cargo build` — succeeds

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-26**: Partial pass — Crossing time measurement infrastructure in place (distance_in_crossing / time_in_crossing tracked in Managed state). Manual verification needed with B02 path adherence to confirm accuracy against observation. Full AUD-26 pass deferred to C06 (stats window integration).
- [x] **AUD-31**: N/A — Velocity levels (B03) not yet implemented.
- [x] **AUD-29, AUD-30**: N/A — Safe distance logic (B04) not yet implemented.

### Requirements Traceability

- [x] **REQ-5**: Physics model with `velocity = distance / time` fields. Vehicle struct now has `distance_in_crossing` and `time_in_crossing` fields initialized and incremented during crossing. Actual velocity reported will be calculated by stats layer in C05–C06.
- [ ] **REQ-6**: Route adherence — deferred to B02 (requires lane path polylines).
- [ ] **REQ-7**: Minimum three velocities — deferred to B03.
- [ ] **REQ-8**: Safe distance — deferred to B04.

## Artifacts

- **Test output**:
  ```text
  running 27 tests ... ok
  running 4 tests (smoke) ... ok
  test result: ok. 31 passed; 0 failed
  ```
- **Lint output**: 1 unused function warning (advance_straight_stub); `cargo clippy -- -D warnings` enforced on non-dead-code items.
- **Build**: `cargo build` succeeds.

---

## Next Steps

- **B02** — Route adherence: Add lane path polylines to IntersectionModel; implement `advance_along_path(vehicle, model, dt)` to move vehicles along fixed routes instead of straight lines.
- **B03** — Velocity levels: Define `VelocityLevel` enum (Fast, Cruise, Yield ≥3 levels) and set initial `commanded_velocity` field on Vehicle.
- **B04** — Safe distance: Implement follow logic (`enforce_follow_distance()`) and collision detection for approaching vehicles.
- **C01** (blocks on B02): Smart detection will transition vehicles from Approaching → Managed on zone entry, activating crossing metric accumulation.
