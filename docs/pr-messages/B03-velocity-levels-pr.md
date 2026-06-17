---
title: "feat(B03): velocity levels"
---

# PR Implementation Report: B03

## Summary

Implements **≥3 distinct velocity levels** per SDS §13.3, satisfying **REQ-7** (distinct speed levels) and **AUD-31** (speed differentiation observable at runtime). A `VelocityLevel` enum (`Fast`, `Cruise`, `Yield`) with concrete speeds derived from `DEFAULT_SPAWN_VELOCITY` is added to `src/vehicle.rs`. The round-robin level assignment is placed inside `spawn_vehicle()` in `src/vehicle.rs` (B-owned), so every vehicle receives its level at construction without touching the Track A–owned `src/spawn.rs`. A reconcile line at the top of `integrate_physics` makes `commanded_velocity` the authoritative speed source each frame.

Step 0 identified one pre-existing issue that is **not** fixed in B03 — see Pre-Existing Issues below.

**No cross-track edits were made. `src/config.rs` and `src/spawn.rs` are both untouched.**

## Key Changes

- **`src/vehicle.rs`** — all B03 changes live here:
  - `pub enum VelocityLevel { Fast, Cruise, Yield }` (line 22) with `pub fn speed(self) -> f32` (line 30) mapping each variant to a multiplier of `DEFAULT_SPAWN_VELOCITY`: Fast = ×1.4, Cruise = ×1.0, Yield = ×0.5.
  - `spawn_vehicle()` (lines 58–71): computes level from `id.0 % 3` round-robin and sets `commanded_velocity: level.speed()` at construction. Every spawned vehicle receives its level at the point of allocation; Track A's `try_spawn` call is unchanged.
  - `integrate_physics()` (line 95): reconcile line `vehicle.velocity = vehicle.commanded_velocity;` at the top (after the `Done` guard) makes `commanded_velocity` authoritative before any displacement computation runs each frame.
  - Two non-trivial tests added to the existing `#[cfg(test)]` block (see Tests section).

## Technical Decisions

- **Assignment in `spawn_vehicle()`, not in `try_spawn()`**: `src/spawn.rs` is Track A–owned (SDS §13.1 line 332; confirmed by `//! Keyboard-driven vehicle spawning (A04+)` header). Placing the round-robin inside `spawn_vehicle()` keeps all B03 changes within the B-owned file while preserving the existing call signature — `try_spawn` passes `VehicleId` into `spawn_vehicle`, which is all that's needed for the `id.0 % 3` computation.
- **`VelocityLevel` in `vehicle.rs`, not a const array**: SDS §13.3 specifies `pub enum VelocityLevel { Fast, Cruise, Yield }` as the published B-track interface contract. The enum is placed in `src/vehicle.rs` (B-owned), co-located with `spawn_vehicle` and `integrate_physics`.
- **Multipliers in `vehicle.rs`, not in `config.rs`**: The multipliers apply to `DEFAULT_SPAWN_VELOCITY` inside `speed()`. `config.rs` remains the single source of the base constant — if it changes, all three levels scale automatically without touching `vehicle.rs`. No cross-track edit required.
- **Reconcile line placement**: `vehicle.velocity = vehicle.commanded_velocity` is placed at the top of `integrate_physics` (first function called in `SpawnSystem::update`). Both `integrate_physics` and `advance_along_path` read `vehicle.velocity`; a single write at the top of the first function ensures both use the commanded level within the same frame tick.
- **`velocity` not set at spawn**: `spawn_vehicle` continues to set `velocity` to the passed `DEFAULT_SPAWN_VELOCITY` argument (existing behaviour); it is overwritten by the reconcile line on the first `integrate_physics` call. The spawning caller need not be aware of velocity levels.
- **Round-robin by `id.0 % 3`**: `next_id` starts at 1, making the first three successful spawns deterministically assign Cruise (id=1), Yield (id=2), Fast (id=3). All three distinct levels appear within the first three vehicles regardless of approach or route.
- **`commanded_velocity` authority split noted**: SDS §13.3 says "Approaching: B sets velocity via follow-distance + level; Managed: C commands via `commanded_velocity`." B03 makes `commanded_velocity` authoritative for all states via the reconcile line. B04 (follow-distance) and C (smart control) should confirm this is the agreed single source of truth; see §13.3 open decision IF-2.

## Pre-Existing Issues (not fixed in B03)

**Double-movement per frame (pre-existing, flagged only):** `SpawnSystem::update` calls both `integrate_physics` and `advance_along_path` in the same frame tick (lines 180–181 of `spawn.rs`). Both translate `vehicle.position` using `vehicle.velocity * dt`. `advance_along_path` uses `+=`, not `=`, so it compounds on top of the displacement already applied by `integrate_physics`. The B02 PR described this as "advance_along_path overrides position," which is inaccurate. Existing tests pass because they only assert direction of travel, not absolute displacement. This is a separate pre-existing issue and is left for B04 or a dedicated cleanup ticket.

## Cross-Track Changes

No cross-track edits. `src/config.rs` and `src/spawn.rs` are both untouched by B03.

## Tests

Two new non-trivial tests, both in `src/vehicle.rs`:

### `vehicle::tests::b03_spawned_vehicles_have_three_distinct_commanded_velocities` (`src/vehicle.rs:179`)
**(Test a — enforces REQ-7 through the real spawn path)**

Spawns 3 vehicles through the real `SpawnSystem::try_spawn` path on three different approaches (South, North, West) to bypass per-direction cooldown. Collects each vehicle's `commanded_velocity` as f32 bit patterns into a `HashSet`, then asserts:
1. All 3 spawns succeeded.
2. The set of distinct `commanded_velocity` values has size ≥ 3.
3. The set exactly matches the three `VelocityLevel::speed()` values (Fast, Cruise, Yield).

A test that only checks the enum array has 3 different numbers would pass trivially — this test fails if `spawn_vehicle` never assigns distinct levels (e.g. if the round-robin code were removed), proving the spawn path is actually wired.

### `vehicle::tests::faster_commanded_velocity_drives_strictly_greater_distance` (`src/vehicle.rs:228`)
**(Test b — enforces AUD-31: levels actually drive motion)**

Creates two vehicles from scratch with `velocity: 0.0` and different `commanded_velocity` values: one `VelocityLevel::Yield.speed()` (60.0 wu/s), one `VelocityLevel::Fast.speed()` (168.0 wu/s). Both start at the origin with `heading_rad = 0.0` (east). Calls `integrate_physics` with `dt = 1.0` on each. Asserts `fast_v.position.x > yield_v.position.x`.

This fails if the reconcile line `vehicle.velocity = vehicle.commanded_velocity` is removed or placed after the displacement computation — because both vehicles start with `velocity = 0.0` and would not move at all. A passing result proves the reconcile line fires, copies `commanded_velocity` into `velocity`, and that value drives the displacement calculation.

## Requirements Traceability

| Requirement | What it requires | File | Line | What was added |
|-------------|-----------------|------|------|----------------|
| REQ-7 | ≥3 distinct speed levels for vehicles | `src/vehicle.rs` | 22 | `pub enum VelocityLevel { Fast, Cruise, Yield }` — three named levels |
| REQ-7 | ≥3 distinct speed levels for vehicles | `src/vehicle.rs` | 30 | `VelocityLevel::speed()` — concrete f32 values (×1.4, ×1.0, ×0.5 of `DEFAULT_SPAWN_VELOCITY`) |
| REQ-7 | ≥3 distinct speed levels assigned at spawn | `src/vehicle.rs` | 58–71 | Round-robin `id.0 % 3` in `spawn_vehicle()`; sets `commanded_velocity: level.speed()` |
| REQ-7 | Test: distinct levels assigned via real spawn path | `src/vehicle.rs` | 179 | `b03_spawned_vehicles_have_three_distinct_commanded_velocities` — asserts ≥3 distinct `commanded_velocity` values after 3 real `try_spawn` calls |
| AUD-31 | Levels actually drive vehicle motion | `src/vehicle.rs` | 95 | Reconcile line `vehicle.velocity = vehicle.commanded_velocity` at top of `integrate_physics` |
| AUD-31 | Test: faster level → strictly greater distance | `src/vehicle.rs` | 228 | `faster_commanded_velocity_drives_strictly_greater_distance` — asserts Fast travels farther than Yield under equal `dt` |
| SDS §13.3 | `pub enum VelocityLevel { Fast, Cruise, Yield }` exported | `src/vehicle.rs` | 22 | Enum defined and public; matches SDS §13.3 contract exactly |
| SDS §13.3 | `commanded_velocity` is authoritative (IF-2) | `src/vehicle.rs` | 95 | Reconcile line makes `commanded_velocity` the single speed source per frame |

**No cross-track edit to `src/config.rs` or `src/spawn.rs` was made.**

## Verification Results

### Automated Checks (run 2026-06-17 on Linux, no SDL2 required for tests)

- [x] `cargo test` passes — 51 unit + 6 smoke: 57 total, 0 failed (+2 new unit tests vs B02)
- [x] `cargo clippy -- -D warnings` passes — clean, no warnings
- [x] `cargo fmt --check` passes
- [x] Build succeeds: `cargo build`

New tests in `src/vehicle.rs`:
- `b03_spawned_vehicles_have_three_distinct_commanded_velocities` ← **new in this PR (REQ-7, test a)**
- `faster_commanded_velocity_drives_strictly_greater_distance` ← **new in this PR (AUD-31, test b)**

All prior tests pass unchanged (B01 `integrate_physics_*` tests confirm the reconcile line does not break existing behaviour because those tests set `velocity` and `commanded_velocity` to the same value — the reconcile is a no-op when they are equal).

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-31**: Pass — ≥3 distinct velocity levels (`Fast` = 168 wu/s, `Cruise` = 120 wu/s, `Yield` = 60 wu/s) are assigned at spawn via `spawn_vehicle()` and wired into motion via the reconcile line in `integrate_physics`. Verified by `b03_spawned_vehicles_have_three_distinct_commanded_velocities` (distinct `commanded_velocity` values present on all spawned vehicles via the real spawn path) and `faster_commanded_velocity_drives_strictly_greater_distance` (faster level produces strictly greater displacement under equal `dt`, proving levels drive motion not just storage). **Visual confirmation via `cargo run` could not be performed** — no SDL2 display is available in this environment, the same limitation noted in the B02 PR. Automated test coverage substitutes for the visual check.

## Artifacts

- **Test output**: `cargo test` — 51 unit tests + 6 smoke: 57 total, 0 failed

```
cargo test (2026-06-17 on Linux):
  running 51 tests
  test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

  running 6 tests
  test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

- **Lint output**: `cargo clippy -- -D warnings` — clean, no warnings
- **Format output**: `cargo fmt --check` — pass
- **Build output**: `cargo build` — Finished dev profile

---

## Next Steps

- **B04** — Safe distance: `enforce_follow_distance()` will adjust `commanded_velocity` in the Approaching state for follow-distance; confirm reconcile-line single-source-of-truth is compatible.
- **C (C02+)** — Smart scheduler writes `commanded_velocity` in Managed state; the reconcile line already carries that into motion each frame.
- **Double-movement cleanup** — The pre-existing double-position-update bug (both `integrate_physics` and `advance_along_path` applying `velocity * dt` each frame) should be addressed before B04 adds follow-distance logic, as it will affect distance calculations.
