---
title: "feat(A04): arrow-key spawn"
---

# PR Implementation Report: A04

## Summary

Implements **arrow-key vehicle spawning** from all four cardinal approaches with the **`SpawnRequest` API** per SDS §13.2. Pressing arrow keys creates vehicles on the correct approach; each vehicle appears at its lane spawn point and travels straight toward the junction. Satisfies **REQ-12–REQ-15** and **AUD-3–AUD-6**.

## Key Changes

- **`src/spawn.rs`**: `SpawnRequest` (approach, route, lane_id), `SpawnCooldown` stub, `SpawnSystem::try_spawn` / `spawn_on_approach`, route rotation (r→s→l), stub straight-line integrator, unit tests for all four travel directions.
- **`src/input.rs`**: `approach_for_arrow` mapping (Up→South, Down→North, Right→West, Left→East), `InputEvent::SpawnCardinal`, key-down event dispatch.
- **`src/vehicle.rs`**: `spawn_vehicle` factory (IF-1 stub), `position`/`heading_rad`/`approach` fields, `snapshot_for_render`.
- **`src/intersection.rs`**: `Cardinal::travel_heading()` for approach-aligned motion.
- **`src/render.rs`**: `draw_vehicle` (colored oriented rects per approach), `draw_frame` accepts vehicle snapshots.
- **`src/app.rs`**: spawn on input events, `SpawnSystem::update` each frame, render active vehicles.
- **`src/config.rs`**: `DEFAULT_SPAWN_VELOCITY`, `VEHICLE_WIDTH`, `VEHICLE_LENGTH`.
- **`tests/smoke.rs`**: integration test for arrow→spawn pipeline.

## Technical Decisions

- **Arrow fixes approach only (PRD OQ-6)**: Each key press rotates through Right → Straight → Left on that approach via a per-approach counter; arrow keys never pick the approach incorrectly.
- **SpawnCooldown stub**: `allows()` always returns `true` until **A05** adds per-direction throttle (REQ-18 / AUD-27).
- **Straight-line stub motion**: Vehicles move along `Cardinal::travel_heading()` at `DEFAULT_SPAWN_VELOCITY` until off-screen. **B01** replaces integrator; **B02** adds lane polylines.
- **Colored rects vs sprites**: A04 draws distinct per-approach rectangles (no vehicle BMP yet). **A07** adds sprite rotation.
- **Minimal `vehicle.rs` touch**: Only the IF-1 spawn factory stub — no B01 physics, safe distance, or path adherence.

## Verification Results

### Automated Checks

- [x] `cargo test` — 31 tests passed (27 unit + 4 integration)
- [x] `cargo clippy -- -D warnings` — clean
- [x] `cargo fmt --check` — clean
- [x] `cargo build` — succeeds

### Manual Audit (against `docs/audit.md`)

- [x] **AUD-3**: Pass — `Arrow Up` → `Cardinal::South` spawn at south edge; `travel_heading` is northward (−y). Verified by unit test `south_vehicle_moves_northward` + manual `cargo run` key press.
- [x] **AUD-4**: Pass — `Arrow Down` → North approach, moves southward (+y). Test: `north_vehicle_moves_southward`.
- [x] **AUD-5**: Pass — `Arrow Right` → West approach, moves eastward (+x). Test: `west_vehicle_moves_eastward`.
- [x] **AUD-6**: Pass — `Arrow Left` → East approach, moves westward (−x). Test: `east_vehicle_moves_westward`.

### Requirements Traceability

- [x] **REQ-12**: `Keycode::Up` → `SpawnCardinal(South)` → vehicle on south approach traveling north.
- [x] **REQ-13**: `Keycode::Down` → `SpawnCardinal(North)` → vehicle on north approach traveling south.
- [x] **REQ-14**: `Keycode::Right` → `SpawnCardinal(West)` → vehicle on west approach traveling east.
- [x] **REQ-15**: `Keycode::Left` → `SpawnCardinal(East)` → vehicle on east approach traveling west.

## Artifacts

- **Test output**:
  ```text
  running 27 tests ... ok
  running 4 tests (smoke) ... ok
  ```
- **Lint output**: clippy clean with `-D warnings`
- **PR message**: `docs/pr-messages/A04-arrow-spawn-pr.md`

---

## Next Steps

- **A05** — per-direction spawn cooldown (REQ-18 / AUD-27)
- **A06** — `R` continuous random spawn (REQ-16 / AUD-7)
- **B01** — replace stub integrator with path-based physics (after A04 ✅)
