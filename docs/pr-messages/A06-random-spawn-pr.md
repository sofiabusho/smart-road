---
title: "feat(A06): R continuous random spawn"
---

# PR Implementation Report: A06

## Summary

Implements **continuous random vehicle spawn** while `R` is held. Each game-loop tick picks a random cardinal approach and random route (`r` / `s` / `l`), respecting the existing per-direction spawn cooldown from A05. Satisfies **REQ-16** and **AUD-7**.

## Key Changes

- **`src/spawn.rs`**: `SpawnRng` (xorshift32, no new dependency), `SpawnSystem::spawn_random`, unit tests for per-direction cooldown (same approach blocked, others allowed) and approach/route variety.
- **`src/input.rs`**: `random_stream_active` flag tracked across `R` key down/up; `random_stream_active()` query for the game loop.
- **`src/app.rs`**: Calls `spawn_random` each frame while `R` is held.
- **`tests/smoke.rs`**: Integration test for `R` → random spawn pipeline.
- **`docs/SDS.md` §13**: `spawn_random` API and `random_stream_active` input contract documented.

## Technical Decisions

- **Hold-to-spawn (PRD OQ-1)**: `R` key down enables continuous spawn each frame; key up stops. Matches SDS §7.2 and audit hold semantics.
- **Per-direction cooldown preserved**: Random picks may target the same approach twice in a row; A05 cooldown still throttles that direction — different approaches can spawn on consecutive frames.
- **Inline PRNG**: `SpawnRng` uses xorshift32 seeded from `Instant` to avoid adding a `rand` crate dependency.
- **Independent of arrow route rotation**: `spawn_random` does not advance `route_counters`; arrow keys keep their r→s→l rotation.

## Verification Results

### Automated Checks

- [x] `cargo test` passes (43 unit + 5 integration tests)
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] `cargo build` succeeds

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-7**: Pass — Hold `R` for several seconds: vehicles appear from multiple approaches with varied routes (r/s/l). Verified by unit tests `spawn_random_produces_varied_approaches_and_routes` (80 spawns cover all 4 approaches and 3 routes) and `spawn_random_respects_per_direction_cooldown` (per-direction throttle after random spawn), plus manual `cargo run` hold-`R` observation.
- [x] **AUD-31**: N/A — Velocity levels (≥3 distinct speeds) are **B03** scope; all random spawns use `DEFAULT_SPAWN_VELOCITY` until B03 lands.

### Requirements Traceability

- [x] **REQ-16**: `InputState` tracks `R` held state; `App::update` calls `SpawnSystem::spawn_random` each frame while active, picking random approach + route via the game loop.

## Artifacts

- **Test output**:
  ```text
  running 43 tests ... ok
  running 5 tests (smoke) ... ok
  ```
- **Lint output**: clippy clean with `-D warnings`
- **PR message**: `docs/pr-messages/A06-random-spawn-pr.md`

---

## Next Steps

- **A07** — turn animation (REQ-11); blocked on B02 ✅ (path tangents available)
- **Gate G1** — A01–A06 complete; AUD-1–AUD-7 and AUD-27 ready for sign-off
- **C04** — sustained random traffic (AUD-16/AUD-17) can proceed once C02 ✅ and A06 ✅
