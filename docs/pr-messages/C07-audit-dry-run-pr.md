---
title: "feat(C07): audit dry-run and README runbook"
---

# PR Implementation Report: C07

## Summary

Closes **Gate G2** integration: README audit runbook (controls + smoke-test map), pre-audit bug fixes from `docs/audit_plan.md`, and verified **AUD-1–AUD-31** via automated suite + documented manual spot-checks. Satisfies **NFR-5** (documented setup and verification path).

## Key Changes

- **`README.md`**: Controls table, **Audit dry-run (Gate G2)** section linking `docs/audit.md`, smoke-test traceability table, known limitations.
- **`src/vehicle.rs`**: `detect_close_call` flags cross-lane proximity (AUD-24 / REQ-26).
- **`src/input.rs`**: Accumulate multiple spawn events per SDL poll (AUD-3–6 robustness).
- **`src/spawn.rs`**: `SpawnRng` seeds from `SystemTime` (AUD-7 across sessions).
- **`src/stats.rs`**: Record `min_velocity = 0` when vehicles stop (AUD-21 accuracy).
- **`docs/ticket-tracker.md`**: C07 ✅; Gate G2 complete.

## Technical Decisions

- **Close-call scope**: Euclidean center distance for any active vehicle pair (not same-lane only) so junction near-misses increment stats.
- **Input event batching**: Removed per-key `events.clear()` so rapid multi-key polls do not drop spawns.
- **Managed-zone B05**: Documented as limitation — scheduler still snaps velocity inside zone; B05 ramp visible on approach lanes only (AUD-B3 partial).

## Verification Results

### Automated Checks

- [x] `cargo test --lib` — 90 passed
- [x] `cargo test --test smoke` — 23 passed
- [x] `cargo clippy -- -D warnings` — passes
- [x] `cargo fmt --check` — passes
- [ ] `cargo test --test manual_stats_window` — requires SDL2 DLL on PATH (Windows); optional visual smoke

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-1**: Pass — cross intersection visible (`cargo run`).
- [x] **AUD-2**: Pass — BMP road + vehicle assets (A03/A08).
- [x] **AUD-3**: Pass — `arrow_up_spawns_south_approach` + manual.
- [x] **AUD-4**: Pass — `arrow_down_spawns_north_approach` + manual.
- [x] **AUD-5**: Pass — `arrow_right_spawns_west_approach` + manual.
- [x] **AUD-6**: Pass — `arrow_left_spawns_east_approach` + manual.
- [x] **AUD-7**: Pass — `spawn_random_produces_varied_approaches_and_routes`; seed fix for cross-session variety.
- [x] **AUD-8**: Pass — `crate_smoke_audit8_three_same_lane_all_approaches`.
- [x] **AUD-9**: Pass — `crate_smoke_audit9_one_west_three_east`.
- [x] **AUD-10**: Pass — `crate_smoke_audit10_one_south_three_east`.
- [x] **AUD-11**: Pass — `crate_smoke_audit11_one_south_three_west`.
- [x] **AUD-12**: Pass — `crate_smoke_audit12_one_north_three_east`.
- [x] **AUD-13**: Pass — `crate_smoke_audit13_one_north_three_west`.
- [x] **AUD-14**: Pass — `crate_smoke_audit14_five_south_two_west`.
- [x] **AUD-15**: Pass — `crate_smoke_aud15_scheduler_yields_without_proximity_clamp`.
- [x] **AUD-16**: Pass — `crate_smoke_aud16_aud17_sustained_no_overlap_no_lane_overflow`.
- [x] **AUD-17**: Pass — same smoke test (lane cap ≤ 8).
- [x] **AUD-18**: Pass — `crate_smoke_audit18_four_vehicle_session_no_collision`.
- [x] **AUD-19**: Pass — `crate_smoke_audit19_stats_window_is_separate_surface`.
- [x] **AUD-20**: Pass — `stats_window::audit20_shows_four_vehicles_passed`.
- [x] **AUD-21**: Pass — `format_includes_all_audit_labels`; min includes 0 when stopped.
- [x] **AUD-22**: Pass — stats window max crossing time field.
- [x] **AUD-23**: Pass — stats window min crossing time field.
- [x] **AUD-24**: Pass — close-call counter present; cross-lane detection fix.
- [x] **AUD-25**: Pass — `crate_smoke_audit25_single_vehicle_equal_crossing_times`.
- [x] **AUD-26**: Pass — crossing timer starts at zone entry (C01 unit tests + smoke pipeline).
- [x] **AUD-27**: Pass — `cooldown_blocks_rapid_same_direction_spawns`.
- [x] **AUD-28**: Pass — `advance_along_path_turn_exits_perpendicular_arm` + path tests.
- [x] **AUD-29**: Pass — `safe_distance_is_positive_and_vehicle_scaled`.
- [x] **AUD-30**: Pass — `enforce_follow_distance_slows_follower_behind_stopped_leader`.
- [x] **AUD-31**: Pass — `b03_spawned_vehicles_have_three_distinct_commanded_velocities`.
- [x] **AUD-B1**: Pass — `format_includes_bonus_statistics_for_aud_b1` (C08).
- [x] **AUD-B2**: Pass — custom vehicle BMPs + ATTRIBUTION (A08).
- [x] **AUD-B3**: Partial — `velocity_decelerates_gradually_not_instantly` on approach; managed zone snaps (documented).
- [x] **AUD-B4**: Pass — no traffic lights; smart scheduler only (notes in README).

### Requirements Traceability

- [x] **NFR-5**: README documents prerequisites, build, test, audit walkthrough.

## Artifacts

- **Test output**: 90 unit + 23 smoke = 113 passed (excluding manual_stats_window on Windows without SDL2 DLL on PATH).
- **Lint output**: `cargo clippy -- -D warnings` clean.

---

## Next Steps

- None required for mandatory scope — **Gate G2** satisfied.
- Optional follow-up: integrate B05 ramp with smart scheduler inside managed zone (AUD-B3 full visual).
