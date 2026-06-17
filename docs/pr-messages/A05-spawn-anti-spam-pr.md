---
title: "feat(A05): per-direction spawn cooldown"
---

# PR Implementation Report: A05

## Summary

Implements **per-direction spawn anti-spam** via `SpawnCooldown` in `spawn.rs`. Rapid key-repeat or hammering the same arrow key no longer stacks vehicles at the same spawn point; each cardinal approach has an independent 400 ms throttle. Satisfies **REQ-18** and **AUD-27**.

## Key Changes

- **`src/config.rs`**: `SPAWN_COOLDOWN_MS` (400 ms) tunable constant per SDS §7.1.
- **`src/spawn.rs`**: `SpawnCooldown` with `per_direction_ms` + `last_spawn` map; `try_spawn` rejects spawns inside the window; unit tests for same-direction block, per-direction independence, and `SpawnSystem` integration.
- **`docs/SDS.md` §13**: `SpawnCooldown` stub updated to reflect implemented fields.

## Technical Decisions

- **Per-direction, not global**: Cooldown is keyed by `Cardinal` so pressing different arrow keys can still spawn immediately (REQ-18 targets same-key spam / SDL key-repeat).
- **400 ms window**: Mid-range of SDS §7.1 example (300–500 ms); long enough to prevent overlap at spawn, short enough for deliberate multi-spawn play.
- **Testable time logic**: `allows_at` / `record_at` helpers use `Instant` so unit tests verify cooldown without sleeping; route-rotation test expires cooldown between spawns.

## Verification Results

### Automated Checks

- [x] `cargo test` passes (42 tests: 38 unit + 4 integration)
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] `cargo build` succeeds

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-27**: Pass — Rapid same-arrow presses create at most one vehicle per 400 ms per approach; vehicles appear staggered, not stacked. Verified by unit tests (`try_spawn_rejects_rapid_duplicate_on_same_approach`, `cooldown_blocks_rapid_same_direction_spawns`) and manual `cargo run` key-repeat on one arrow.
- [x] **AUD-31**: N/A — Velocity levels (≥3 distinct speeds) are **B03** scope, not this ticket.

### Requirements Traceability

- [x] **REQ-18**: `SpawnSystem::try_spawn` consults `SpawnCooldown::allows` before creating a vehicle; rejected spawns return `None` so vehicles do not overlap at creation.

## Artifacts

- **Test output**:
  ```text
  running 38 tests ... ok
  running 4 tests (smoke) ... ok
  ```
- **Lint output**: clippy clean with `-D warnings`
- **PR message**: `docs/pr-messages/A05-spawn-anti-spam-pr.md`

---

## Next Steps

- **A06** — `R` continuous random spawn (REQ-16 / AUD-7); must respect same per-direction cooldown
- **Gate G1** — A06 remains for full G1 pass (AUD-1–AUD-7, AUD-27)
