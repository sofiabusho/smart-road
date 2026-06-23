---
title: "feat(C02): smart scheduler velocity coordination"
---

# PR Implementation Report: C02

## Summary

Implements **smart intersection velocity scheduling** per SDS §6.2. `SmartController` builds a lane conflict graph from path geometry, tracks managed-zone entry order, and commands `commanded_velocity` for `Managed` vehicles so later entrants yield on conflicting or same-lane close approaches. Tick order updated to **smart before movement** (SDS §3). Satisfies **REQ-3** and **REQ-9** (partial); **AUD-8–AUD-14** verified via audit-mirror smoke harness.

## Key Changes

- **`src/smart.rs`**: Conflict graph from path segment intersections in junction zone; FIFO entry priority; `schedule_managed_velocities` for cross-traffic and same-lane managed spacing; `managed_scheduler_yielded` / `managed_vehicles_in_scheduler_range` test hooks; `entry_sequence` cleanup on Managed→Exiting; 4 unit tests.
- **`src/app.rs`**: Smart `update()` runs before `spawn.update()` so scheduled velocities apply before movement.
- **`src/spawn.rs`**: Same-lane spawn queue offset (`spawn_position_on_lane`); proximity clamp after movement.
- **`src/vehicle.rs`**: `clamp_velocity_for_proximity()` — stop and nudge apart when centers within one vehicle length.
- **`tests/smoke.rs`**: AUD-8–AUD-14 mirror tests assert scheduler yield when managed vehicles are in range (not only proximity-clamp overlap guard); follow-distance smoke fixes explicit `nominal_velocity` on stopped-leader scenarios.

## Review response (PR #13 — software-sappho)

| Finding | Action |
|---------|--------|
| **Major 1** — SDS §13.1 missing C02 cross-track footprint | Updated `docs/SDS.md` §13.1 for `app.rs`, `spawn.rs`, `vehicle.rs`, `smart.rs` |
| **Major 2** — AUD smoke tests pass via clamp backstop only | `audit_sim::run_until_all_exited` tracks `managed_vehicles_in_scheduler_range` + `managed_scheduler_yielded` after `smart.update()` |
| **Major 3** — Follow-distance tests vacuous (`VehicleId(2)` → Yield nominal) | Set explicit `nominal_velocity` on follower; leader `nominal_velocity = 0` so enforce does not restore cruise speed |
| **Minor 4** — `entry_sequence` leak on exit | `entry_sequence.remove` on Managed→Exiting |
| **Minor 5** — `detect_close_call` same-lane only | Deferred to C03/C04 (unchanged) |
| **Minor 6** — `VelocityLevel` ownership vs B03 | Documented in SDS §13.1 `vehicle.rs` row |
| **Nit 7** — Dead bindings in `schedule_managed_velocities` | Simplified FIFO `yielder` match arms |

## Cross-track edits (announced per SDS §13.1)

| File | Owner | C02 change |
|------|-------|------------|
| `src/app.rs` | A | Reorder tick: smart schedule before spawn physics |
| `src/spawn.rs` | A | Spawn queue offset + post-move proximity clamp |
| `src/vehicle.rs` | B | `clamp_velocity_for_proximity` collision guard |

## Technical Decisions

- **Path-derived conflict graph**: Lane pairs conflict when polylines intersect inside `zone_polygon` (SDS §4.2). Computed once per controller lifetime.
- **FIFO priority**: Earlier managed entry keeps nominal speed; later entrant yields with `VelocityLevel::Yield` or stop when gap ≤ `SAFE_DISTANCE`.
- **Managed-only authority**: Scheduling applies to `VehicleState::Managed` only; approach follow-distance remains B04; yield-on-conflict choreography (AUD-15) deferred to **C03**.
- **Proximity clamp**: Last-resort separation when centers closer than `0.95 × VEHICLE_LENGTH` after movement.

## Verification Results

### Automated Checks

- [x] `cargo test` — 70 unit + 17 smoke = 87 passed
- [x] `cargo clippy -- -D warnings` — passes
- [x] `cargo fmt --check` — passes
- [x] `cargo build` / `cargo run` — succeeds (SDL2 configured; WSL manual spot-check)

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-8**: Pass — `crate_smoke_audit8_three_same_lane_all_approaches` (3 same-lane spawns × 4 approaches, no overlap).
- [x] **AUD-9**: Pass — `crate_smoke_audit9_one_west_three_east` (1 West + 3 East rotating routes).
- [x] **AUD-10**: Pass — `crate_smoke_audit10_one_south_three_east`.
- [x] **AUD-11**: Pass — `crate_smoke_audit11_one_south_three_west`.
- [x] **AUD-12**: Pass — `crate_smoke_audit12_one_north_three_east`.
- [x] **AUD-13**: Pass — `crate_smoke_audit13_one_north_three_west`.
- [x] **AUD-14**: Pass — `crate_smoke_audit14_five_south_two_west`.
- [ ] **AUD-15**: Deferred to **C03** (velocity reduction on conflict — partial via scheduler + clamp).

### Requirements Traceability

- [x] **REQ-3**: Smart controller commands velocity in managed zone via conflict-aware scheduling (no traffic lights).
- [x] **REQ-9**: Partial — imminent conflict reduces velocity and proximity clamp prevents overlap; full yield choreography in **C03**.

## Artifacts

- **Test output**: `cargo test` — 87 passed; 0 failed.
- **Lint output**: `cargo clippy -- -D warnings` clean.

---

## Next Steps

- **C03** — Yield on conflict (AUD-15).
- **C04** — Sustained traffic / congestion cap (needs C02 ✅).
