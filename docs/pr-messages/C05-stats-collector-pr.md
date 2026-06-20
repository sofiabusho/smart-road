---
title: "feat(C05): stats collector for session metrics"
---

# PR Implementation Report: C05

## Summary

Implements **session statistics collection** per SDS §9 and §13.4. `StatsSession` observes vehicles each frame for velocity samples and managed-zone entry; `SpawnSystem::update` returns `VehicleExit` events when vehicles leave the canvas after crossing. Wired into `app.rs` after smart detection. Satisfies collector side of **REQ-20–REQ-26** (display deferred to **C06**).

## Key Changes

- **`src/stats.rs`**: `Stats` fields, `StatsEvent`, `apply_event()`, `StatsSession` with `observe_vehicles` / `record_exit` / `record_close_call`; 4 unit tests.
- **`src/spawn.rs`**: `VehicleExit` struct; `update()` returns exits for vehicles that left after entering managed zone.
- **`src/app.rs`**: `StatsSession` + `session_time`; records velocity samples and crossing exits each tick.
- **`docs/SDS.md`**: §13.1 cross-track notes for C05 `app.rs` / `spawn.rs` edits.
- **`tests/smoke.rs`**: `crate_smoke_stats_collector_pipeline` end-to-end smoke test.

## Cross-track edits (announced per SDS §13.1)

| File | Owner | C05 change |
|------|-------|------------|
| `src/app.rs` | A | Wire `StatsSession::observe_vehicles` and `record_exit` in tick loop |
| `src/spawn.rs` | A | `SpawnSystem::update` returns `Vec<VehicleExit>` for stats on vehicle removal |

## Technical Decisions

- **Exit only after managed zone**: Vehicles that leave before smart detection (`Approaching`) are not counted as passed (REQ-23 crossing timer).
- **Peak velocity per vehicle**: Tracked in `StatsSession` and applied on exit alongside live velocity samples.
- **Close calls**: `record_close_call` API ready; auto-detection from B04 `detect_close_call` wired when B04 merges (C05 collector only).
- **C05 scope only**: No Esc stats window (C06); stats accumulate in memory during session.

## Verification Results

### Automated Checks

- [x] `cargo test` — 55 unit + 7 smoke = 62 passed
- [x] `cargo clippy -- -D warnings` — passes
- [x] `cargo fmt --check` — passes
- [x] `cargo build` — succeeds (SDL2 configured)

### Manual Audit (against `docs/audit.md`)

- [ ] **AUD-20–AUD-24**: Deferred to **C06** (stats window display) — collector fields populated in `StatsSession.stats`.
- [x] **REQ-23** (collector): `time_in_crossing` from vehicle at exit after C01 detection path.

### Requirements Traceability

- [x] **REQ-20**: `vehicles_passed` / `max_vehicles_passed` updated on `VehicleExited` events.
- [x] **REQ-21 / REQ-22**: `max_velocity` / `min_velocity` from per-frame samples and exit peaks.
- [x] **REQ-23**: Crossing time taken from `vehicle.time_in_crossing` at canvas exit (post-detection).
- [x] **REQ-24 / REQ-25**: `max_crossing_time` / `min_crossing_time` bounds on exit events.
- [x] **REQ-26**: `close_calls` counter + `record_close_call` API (auto-detect when B04 available).

## Artifacts

- **Test output**: `cargo test` — all unit + smoke tests pass.
- **Lint output**: `cargo clippy -- -D warnings` clean.

---

## Next Steps

- **C06** — Stats window on Esc: display all `Stats` fields (AUD-18–AUD-25).
- **B04 integration** — Wire `detect_close_call` → `StatsSession::record_close_call` in app loop.
