---
title: "feat(C01): smart detection and timer start"
---

# PR Implementation Report: C01

## Summary

Implements **smart-system zone detection** per SDS §6.1 and §13.4. `SmartController::update()` checks each vehicle's position against `IntersectionModel::zone_polygon` each frame: **Approaching → Managed** on zone entry (REQ-4), **Managed → Exiting** on zone exit. Crossing metrics (`time_in_crossing`, `distance_in_crossing`) reset at detection so the timer starts at the smart-system entry point (REQ-23). Wired into the main loop after spawn physics/path updates.

## Key Changes

- **`src/smart.rs`**: `on_vehicle_enter_zone()`, `update()`, ray-casting `point_in_polygon()`; 6 unit tests for detection transitions and timer start.
- **`src/app.rs`**: Calls `smart.update()` after `spawn.update()` each tick.
- **`src/spawn.rs`**: Adds `vehicles_mut()` for smart-system access to the vehicle list.
- **`docs/SDS.md`**: §13.1 cross-track ownership updated for C01 `app.rs` / `spawn.rs` edits; §13.2 `vehicles_mut()`; §13.4 `on_vehicle_enter_zone` signature aligned with implementation.
- **`tests/smoke.rs`**: Headless smoke test for spawn → physics → smart detection pipeline.

## Cross-track edits (announced per SDS §13.1)

| File | Owner | C01 change |
|------|-------|------------|
| `src/app.rs` | A | Wire `SmartController::update()` in tick loop after `spawn.update()` |
| `src/spawn.rs` | A | Add `SpawnSystem::vehicles_mut()` for smart-system vehicle access |

## Technical Decisions

- **Detection after movement**: Smart update runs after `spawn.update()` so zone checks use the current frame position.
- **Managed → Exiting on zone exit**: Metrics continue in `Exiting` until the vehicle leaves the canvas (REQ-23: detection → removal).
- **Counters reset on entry**: `time_in_crossing` and `distance_in_crossing` zeroed in `on_vehicle_enter_zone()` so the crossing clock starts exactly at detection.
- **C01 scope only**: No velocity scheduling (C02), stats collection (C05), or Esc window (C06).

## Verification Results

### Automated Checks

- [x] `cargo test` — 40 unit + 5 smoke tests passed
- [x] `cargo clippy -- -D warnings` — passes
- [x] `cargo fmt --check` — passes
- [x] `cargo build` — succeeds (with SDL2 configured)

### Manual Audit (against `docs/audit.md`)

- [ ] **AUD-25**: Deferred to C06 (stats window) — detection infrastructure in place.
- [ ] **AUD-26**: Partial — timer starts at Managed transition; full pass with stats display in C06.

### Requirements Traceability

- [x] **REQ-4**: Vehicle center entering `zone_polygon` transitions to `Managed`.
- [x] **REQ-23**: Crossing timer resets at detection; accumulates through `Managed` and `Exiting` until off-screen.
- [x] **REQ-3**: Partial — smart controller active; velocity coordination deferred to C02.

## Artifacts

- **Test output**:
  ```text
  running 40 tests (unit) ... ok
  running 5 tests (smoke) ... ok
  test result: ok. 45 passed; 0 failed
  ```
- **Lint output**: `cargo clippy -- -D warnings` clean.
- **Format output**: `cargo fmt --check` clean.
- **Build**: `cargo build` succeeds.

---

## Next Steps

- **C02** — Smart scheduler: velocity/time coordination (needs B03, B04).
- **C05** — Stats collector: record crossing times from `time_in_crossing` on vehicle exit.
- **C06** — Stats window on Esc: display AUD-25/AUD-26 fields.
