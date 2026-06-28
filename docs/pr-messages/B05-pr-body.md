## Summary

- Implements **gradual acceleration/deceleration** per REQ-B3 (bonus)
- Vehicles ramp `velocity` toward `commanded_velocity` each frame — no instant speed jumps
- Three per-vehicle motion profiles (`VehicleId % 3`) for distinct accel/decel rates (AUD-B3)

## Test plan

### Automated

- [x] `cargo test` — **79 passed** (70 unit + 9 smoke)
- [x] `cargo clippy -- -D warnings`
- [x] `cargo fmt --check`
- [x] `velocity_decelerates_gradually_not_instantly`
- [x] `different_vehicles_use_different_deceleration_rates`
- [x] `motion_profile_exposes_three_distinct_deceleration_scales`
- [x] Existing B04 follow-distance tests still pass with ramped motion

### Manual audit (bonus)

- [x] **AUD-B3** — Vehicle slows gradually when follow-distance caps speed (no instant jump); per-vehicle decel rates differ (verified visually)

**Audit progress: 1 / 1 bonus task completed (AUD-B3)**

## Ticket

B05 — Acceleration / deceleration *(bonus)*

## PR artifact

`docs/pr-messages/B05-acceleration-pr.md`
