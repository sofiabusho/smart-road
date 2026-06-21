---
title: "feat(B05): acceleration and deceleration physics (bonus)"
---

# PR Implementation Report: B05

## Summary

Implements **gradual acceleration and deceleration** per REQ-B3. Vehicles ramp `velocity` toward `commanded_velocity` each frame instead of snapping instantly. Per-vehicle motion profiles (from `VehicleId`) give three distinct accel/decel rates for AUD-B3.

## Key Changes

- **`src/config.rs`**: `BASE_ACCELERATION`, `BASE_DECELERATION` constants.
- **`src/vehicle.rs`**: `motion_profile()`, `step_velocity_toward_command()`; wired into `integrate_physics` and `advance_along_path`; `enforce_follow_distance` sets command only (B05 ramps actual speed).
- **Tests**: gradual decel, distinct per-vehicle rates, three profile scales; updated B04 restore test for ramped acceleration.

## Technical Decisions

- **Command vs actual speed**: Follow distance and smart scheduler (C02) continue to set `commanded_velocity`; B05 owns the ramp to that target.
- **Per-vehicle profiles**: `id % 3` → fast/normal/sluggish accel-decel scales (matches B03 velocity-level assignment pattern).
- **Single integration point**: `step_velocity_toward_command` runs inside both `advance_along_path` (live sim) and `integrate_physics` (unit tests).

## Verification Results

### Automated Checks

- [x] `cargo test` — all unit + smoke tests pass
- [x] `cargo clippy -- -D warnings`
- [x] `cargo fmt --check`

### Manual Audit (against `docs/audit.md`)

- [ ] **AUD-B3**: Observe vehicle slowing — speed changes gradually, not instantly (manual visual)

### Requirements Traceability

- [x] **REQ-B3**: Acceleration/deceleration physics with per-vehicle rate variation.

## Artifacts

- **Test output**: `cargo test` clean.

---

## Next Steps

- Optional manual `cargo run` visual check for AUD-B3 before audit dry-run (C07).
