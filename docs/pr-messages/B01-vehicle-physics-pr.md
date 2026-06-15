---
title: "feat(B01): vehicle physics"
---

# PR Implementation Report: B01

## Summary

Implements **vehicle physics foundation** for autonomous vehicle simulation with position integration and crossing-time metrics per SDS §13.3. Vehicles now track path progress (`path_index`), distance traveled inside the junction (`distance_in_crossing`), and time spent crossing (`time_in_crossing`). Physics update function renamed to `integrate_physics()` per spec; new `commanded_velocity` field added to Vehicle struct (initialized to spawn velocity; Track C writes this in Managed state for smart coordination). Lays groundwork for **B02** (route adherence) and **B03–B04** (velocity levels, safe distance). Satisfies **REQ-5** and builds toward **AUD-26**.

## Key Changes

- **`src/vehicle.rs`**: Added four fields to `Vehicle` struct (`path_index: usize`, `distance_in_crossing: f32`, `time_in_crossing: f32`, `commanded_velocity: f32`); updated `spawn_vehicle()` factory to initialize all fields (commanded_velocity initialized to spawn velocity); new `integrate_physics(vehicle: &mut Vehicle, dt: f32)` function for physics integration and crossing metric accumulation in Managed and Exiting states. Added three unit tests for crossing metric behavior (lines 86–174).
- **`src/spawn.rs`**: Updated `SpawnSystem::update()` to call `crate::vehicle::integrate_physics(vehicle, dt)`. Removed unused `advance_straight_stub()` function (dead code).
- **SDS §13.3 alignment**: Renamed `update_physics` to `integrate_physics` to match spec. Added `commanded_velocity: f32` field per IF-2 (B01 field; Track C writes in Managed state).

## Technical Decisions

- **Physics location & naming**: Moved from spawn.rs stub to vehicle.rs module per SDS §13.1 file ownership (B01 owns vehicle.rs). Function renamed to `integrate_physics()` per SDS §13.3 spec. Keeps physics logic colocated with Vehicle state.
- **Commanded velocity field**: Added `commanded_velocity: f32` per SDS §13.3 IF-2 (open interface decision). Initialized to spawn velocity in `spawn_vehicle()`; Track C writes to this field when vehicle enters Managed state for smart coordination.
- **Crossing metrics in Managed and Exiting states**: `time_in_crossing` and `distance_in_crossing` increments occur when vehicle.state is Managed or Exiting (line 75). This allows crossing metrics to be captured through the full crossing transition, preparing for C01 smart-system detection and C05–C06 stats collection.
- **Distance calculation**: Uses Euclidean distance `sqrt(dx² + dy²)` from velocity and heading; accounts for heading direction even during turns (B02 will replace with path-based motion).

## Verification Results

### Automated Checks

- [x] `cargo test` — 34 tests passed (30 unit + 4 smoke)
- [x] `cargo clippy -- -D warnings` — passes with no warnings
- [x] `cargo fmt --check` — passes; all code formatted per rustfmt
- [x] `cargo build` — succeeds

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-26**: Partial pass — Crossing time measurement infrastructure in place (distance_in_crossing / time_in_crossing tracked in Managed and Exiting states). Manual verification needed with B02 path adherence to confirm accuracy against observation. Full AUD-26 pass deferred to C06 (stats window integration).
- [x] **AUD-31**: N/A — Velocity levels (B03) not yet implemented.
- [x] **AUD-29, AUD-30**: N/A — Safe distance logic (B04) not yet implemented.

### Requirements Traceability

- [x] **REQ-5** (partial / foundation): Physics model with `velocity = distance / time` fields. Vehicle struct now has `distance_in_crossing` and `time_in_crossing` fields initialized and incremented during crossing and exiting. Actual velocity reported will be calculated by stats layer in C05–C06.
- [ ] **REQ-6**: Route adherence — deferred to B02 (requires lane path polylines).
- [ ] **REQ-7**: Minimum three velocities — deferred to B03.
- [ ] **REQ-8**: Safe distance — deferred to B04.

## Review Response

All reviewer feedback has been addressed:

- **Blocker: advance_straight_stub dead code** → Removed entirely from `src/spawn.rs` (previously lines 125-134)
- **Blocker: cargo fmt --check failing** → Test assertions reformatted per rustfmt (lines 104-115, 137-144, 166-173 in `src/vehicle.rs`); now passes
- **Blocker: cargo clippy warnings** → Confirmed passing with `cargo clippy -- -D warnings` exit code 0
- **Major: unit tests missing** → Added 3 new tests in `src/vehicle.rs` (lines 86-174):
  - `integrate_physics_does_not_accumulate_crossing_metrics_when_approaching()` (lines 86-116)
  - `integrate_physics_accumulates_crossing_metrics_when_managed()` (lines 119-145)
  - `integrate_physics_accumulates_crossing_metrics_when_exiting()` (lines 148-174)
- **Major: Exiting state fix** → `integrate_physics()` condition updated to `if vehicle.state == VehicleState::Managed || vehicle.state == VehicleState::Exiting` (line 75)
- **Major: ticket-tracker.md status** → B01 row updated from `⬜` to `✅` (line 108 in ticket-tracker.md)
- **Minor: test count accuracy** → Corrected from 31 to 34 tests (added 3 vehicle tests: 27→30 unit)

## Artifacts

- **Test output**:
  ```text
  running 30 tests (unit) ... ok
  running 4 tests (smoke) ... ok
  test result: ok. 34 passed; 0 failed
  ```
- **Lint output**: `cargo clippy -- -D warnings` passes with no warnings.
- **Format output**: `cargo fmt --check` passes.
- **Build**: `cargo build` succeeds.

---

## Next Steps

- **B02** — Route adherence: Add lane path polylines to IntersectionModel; implement `advance_along_path(vehicle, model, dt)` to move vehicles along fixed routes instead of straight lines.
- **B03** — Velocity levels: Define `VelocityLevel` enum (Fast, Cruise, Yield ≥3 levels) and set initial `commanded_velocity` field on Vehicle.
- **B04** — Safe distance: Implement follow logic (`enforce_follow_distance()`) and collision detection for approaching vehicles.
- **C01** (blocks on B02): Smart detection will transition vehicles from Approaching → Managed on zone entry, activating crossing metric accumulation.
