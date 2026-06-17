---
title: "feat(B04): safe distance follow logic"
---

# PR Implementation Report: B04

## Summary

Implements **same-lane safe-distance follow logic** per SDS §13.3 and REQ-8/REQ-9. Adds `SAFE_DISTANCE` in `config.rs`, `enforce_follow_distance()` and `detect_close_call()` in `vehicle.rs`, and wires follow checks into `SpawnSystem::update` before path movement. Also fixes **DEF-01** (double movement per frame) by using `advance_along_path` as the sole live-sim position authority.

## Key Changes

- **`src/config.rs`**: `SAFE_DISTANCE` (40 world units, ≥ `VEHICLE_LENGTH`).
- **`src/vehicle.rs`**: `enforce_follow_distance()`, `detect_close_call()`, longitudinal gap helper; crossing metrics now accumulate inside `advance_along_path`.
- **`src/spawn.rs`**: Call `enforce_follow_distance` each tick; remove duplicate `integrate_physics` call (DEF-01 fix).
- **`tests/smoke.rs`**: `crate_smoke_same_approach_follower_slows_behind_stopped_leader` — full spawn pipeline manual scenario.

## Cross-track edits (announced per SDS §13.1)

| File | Owner | B04 change |
|------|-------|------------|
| `src/spawn.rs` | A | Invoke `enforce_follow_distance` before `advance_along_path` in `SpawnSystem::update` (same pattern as existing B02 path call) |

## Technical Decisions

- **40 unit safe distance**: PRD OQ-2 — strictly positive and ≥ one vehicle length (36).
- **Approach / exiting only**: `enforce_follow_distance` skips `Managed` followers (smart zone defers to C02); leaders on any non-`Done` state still affect followers.
- **Proportional slowdown**: When gap &lt; `SAFE_DISTANCE`, cap `commanded_velocity` by leader speed and gap ratio; hard stop when gap ≤ 10% of safe distance.
- **DEF-01 fix**: `integrate_physics` remains for unit tests; live loop uses `advance_along_path` only so vehicles move once per frame.

## Verification Results

### Automated Checks

- [x] `cargo test` — 56 unit + 7 smoke = 63 passed
- [x] `cargo clippy -- -D warnings` — passes
- [x] `cargo fmt --check` — passes
- [x] `cargo build` / `cargo run` — succeeds (SDL2 configured; window launches)
- [x] **Manual (AUD-30 scenario)**: Two vehicles on south straight lane — follower slows behind stopped leader; verified by `crate_smoke_same_approach_follower_slows_behind_stopped_leader` (spawn → cooldown → same lane → `SpawnSystem::update` loop)

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-29**: Pass — `SAFE_DISTANCE` in `config.rs` is strictly positive (40) and ≥ vehicle length; unit test `safe_distance_is_positive_and_vehicle_scaled`.
- [x] **AUD-30**: Pass — `enforce_follow_distance_slows_follower_behind_stopped_leader` simulates follower approaching stopped leader; gap stays ≥ 90% of safe distance and follower slows.

### Requirements Traceability

- [x] **REQ-8**: Configured positive safe distance; same-lane followers reduce speed to maintain separation.
- [x] **REQ-9**: Partial — in-lane collision avoidance via velocity reduction on approach; full intersection conflict avoidance deferred to C02/C03.

## Artifacts

- **Test output**:
  ```text
  running 56 tests (unit) ... ok
  running 7 tests (smoke) ... ok
  test result: ok. 63 passed; 0 failed
  ```
- **Lint output**: `cargo clippy -- -D warnings` clean.

---

## Next Steps

- **C02** — Smart scheduler (B04 ✅ unblocks cross-track dep)
- **C05** — Stats collector can wire `detect_close_call` for close-call events
- **B05** *(bonus)* — acceleration / deceleration smoothing
