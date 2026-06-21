---
title: "feat(C02): smart scheduler velocity coordination"
---

# PR Implementation Report: C02

## Summary

Implements **smart intersection velocity scheduling** per SDS §6.2. `SmartController` builds a lane conflict graph from path geometry, tracks managed-zone entry order, and commands `commanded_velocity` for `Managed` vehicles so later entrants yield on conflicting or same-lane close approaches. Satisfies **REQ-3** and the collector side of **REQ-9**; **AUD-8–AUD-14** require manual spawn scenarios.

## Key Changes

- **`src/smart.rs`**: Conflict graph from path segment intersections in junction zone; FIFO entry priority; `schedule_managed_velocities` for cross-traffic and same-lane managed spacing; 4 new unit tests.
- **`tests/smoke.rs`**: `crate_smoke_cross_traffic_scheduler_avoids_overlap` — South + East straight spawn pipeline with overlap guard.

## Technical Decisions

- **Path-derived conflict graph**: Lane pairs conflict when polylines intersect inside `zone_polygon` (SDS §4.2). Computed once per controller lifetime.
- **FIFO priority**: Earlier managed entry keeps nominal speed; later entrant yields with `VelocityLevel::Yield` or stop when gap ≤ `SAFE_DISTANCE`.
- **Managed-only authority**: Scheduling applies to `VehicleState::Managed` only; approach follow-distance remains B04; yield-on-conflict slowdown (AUD-15) deferred to **C03**.

## Verification Results

### Automated Checks

- [x] `cargo test` — 70 unit + 10 smoke = 80 passed
- [x] `cargo clippy -- -D warnings` — passes
- [x] `cargo fmt --check` — passes
- [x] `cargo build` — succeeds (SDL2 configured)

### Manual Audit (against `docs/audit.md`)

- [ ] **AUD-8–AUD-14**: Manual — run spawn scenarios (3 same-lane, 4 cross-traffic, 7 mixed); scheduler + B04 should prevent overlap; verify visually per audit steps.

### Requirements Traceability

- [x] **REQ-3**: Smart controller commands velocity in managed zone via conflict-aware scheduling (no traffic lights).
- [x] **REQ-9**: Partial — imminent conflict reduces velocity; full yield choreography in **C03**.

## Artifacts

- **Test output**: `cargo test` — 80 passed; 0 failed.
- **Lint output**: `cargo clippy -- -D warnings` clean.

---

## Next Steps

- **C03** — Yield on conflict (AUD-15).
- **C04** — Sustained traffic / congestion cap (needs C02 ✅).
