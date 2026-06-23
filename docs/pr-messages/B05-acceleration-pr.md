---
title: "feat(B05): acceleration and deceleration physics (bonus)"
---

# PR Implementation Report: B05

## Summary

Implements **gradual acceleration and deceleration** per REQ-B3. Vehicles ramp `velocity` toward `commanded_velocity` each frame instead of snapping instantly. Per-vehicle motion profiles (from `VehicleId`) give three distinct accel/decel rates for AUD-B3.

## Key Changes

- **`src/config.rs`**: `BASE_ACCELERATION`, `BASE_DECELERATION` constants.
- **`src/vehicle.rs`**: `motion_profile()`, `step_velocity_toward_command()`; wired into `integrate_physics` and `advance_along_path`; `enforce_follow_distance` sets command only (B05 ramps actual speed).
- **`docs/SDS.md`**: §5.1 and §13.3 updated for B05 acceleration API and velocity-ramp semantics.
- **Tests**: gradual decel/accel, distinct per-vehicle rates, three profile scales; updated B04 restore test for ramped acceleration.

## Cross-track edits (announced per SDS §13.1)

| File | Owner | B05 change |
|------|-------|------------|
| `src/config.rs` | A | Add `BASE_ACCELERATION` (180) and `BASE_DECELERATION` (240) tunables (B proposes via ticket PR) |

## Technical Decisions

- **Command vs actual speed**: Follow distance and smart scheduler (C02) continue to set `commanded_velocity`; B05 owns the ramp to that target.
- **Per-vehicle profiles**: `id % 3` → fast/normal/sluggish accel-decel scales (matches B03 velocity-level assignment pattern).
- **Single integration point**: `step_velocity_toward_command` runs inside both `advance_along_path` (live sim) and `integrate_physics` (unit tests).

## Verification Results

### Automated Checks

- [x] `cargo test` — all unit + smoke tests pass
- [x] `cargo clippy -- -D warnings`
- [x] `cargo fmt --check`
- [x] `cargo build` — succeeds

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-B3**: Pass — `velocity_decelerates_gradually_not_instantly` proves one frame does not snap to zero when commanded to stop; `different_vehicles_use_different_deceleration_rates` and `different_vehicles_use_different_acceleration_rates` prove per-vehicle rate variation; `enforce_follow_distance_restores_nominal_when_gap_safe` shows gradual speed-up after gap clears. `cargo run` launches; follower behind stopped leader (AUD-30 smoke scenario) visibly eases to a stop over multiple frames rather than halting instantly.

### Requirements Traceability

- [x] **REQ-B3**: Acceleration/deceleration physics with per-vehicle rate variation.

## Artifacts

- **Test output**: `cargo test` clean.

---

## Next Steps

- C07 audit dry-run can re-confirm AUD-B3 visually alongside full AUD-1–AUD-31 sweep.
