# smart-road

## What This Project Is

smart-road is a Rust/SDL2 simulation of autonomous vehicles navigating a four-way cross intersection without traffic lights. Vehicles spawn from four directions, each placed in one of three dedicated lanes (right turn, straight, or left turn), and a smart intersection controller coordinates their passage by granting entry reservations and commanding speeds — preventing collisions and minimising congestion. The project is an 01-edu school assignment that replaces signal-controlled junctions with a reservation-based algorithmic strategy.

## Demo

| Key | Action |
|-----|--------|
| Arrow Up | Spawn a vehicle from the south approach (travels north) |
| Arrow Down | Spawn a vehicle from the north approach (travels south) |
| Arrow Right | Spawn a vehicle from the west approach (travels east) |
| Arrow Left | Spawn a vehicle from the east approach (travels west) |
| R (hold) | Continuously spawn vehicles at random approaches and routes |
| Esc | End the session and open the statistics window |

Each arrow key spawns from the **opposite** edge: pressing Up sends a northbound vehicle that originates at the south side of the window. A 400 ms per-direction cooldown prevents stacking vehicles on the same approach. Close the stats window (or press Esc again) to quit.

## Requirements

### System requirements

- **Rust (2021 edition)** — any stable toolchain ≥ 1.56. Install via [rustup](https://rustup.rs).
- **SDL2** — Simple DirectMedia Layer 2, a cross-platform library that provides the window, hardware-accelerated 2D renderer, and keyboard input this simulation relies on.
- No other runtime dependencies. The sole Cargo dependency is `sdl2 = "0.37"`.

### Installing SDL2

**Ubuntu / Debian**

```bash
sudo apt update
sudo apt install -y libsdl2-dev
```

**macOS (Homebrew)**

```bash
brew install sdl2
```

If Cargo cannot find the library at link time, add this to `~/.cargo/config.toml`:

```toml
[env]
LIBRARY_PATH = "/opt/homebrew/lib"   # adjust if brew installed elsewhere
```

**Windows**

Install SDL2 development libraries via [vcpkg](https://vcpkg.io/) or download the official SDL2 MSVC development package from the [SDL2 releases page](https://github.com/libsdl-org/SDL/releases), then ensure `SDL2.dll` is on your `PATH` when running the binary. See the [sdl2 crate README](https://github.com/Rust-SDL2/rust-sdl2#windows-msvc) for detailed steps.

## How to Run

```bash
git clone <repo-url>
cd smart-road
cargo run --release
```

Assets are loaded from `assets/` relative to the working directory, so run from the project root. A window titled **smart-road** opens at 1024 × 768 pixels. Press **Esc** after a session to open the statistics window; close it or press **Esc** again to quit.

**Regenerate road tile assets** (only needed after changing layout constants in `src/config.rs`):

```bash
python3 scripts/generate_road_assets.py
```

**Regenerate vehicle sprites** (only needed if replacing the shipped BMPs):

```bash
python3 scripts/generate_vehicle_sprites.py
```

## Testing

```bash
cargo test --lib           # unit tests (~90)
cargo test --test smoke    # integration smoke tests (~23)
cargo clippy -- -D warnings
cargo fmt --check
```

## Architecture

```text
src/
  main.rs         — entry point; calls App::run()
  app.rs          — SDL2 init, fixed-timestep loop, Esc → stats window
  config.rs       — window size, lane geometry, spawn cooldown, physics constants
  intersection.rs — lane registry, route polylines, junction zone polygon
  vehicle.rs      — vehicle state machine, path following, proximity clamping
  smart.rs        — reservation gate, FIFO scheduler, managed-zone velocity commands
  spawn.rs        — keyboard/random spawn pipeline, per-direction cooldown
  render.rs       — BMP asset loading, road tiles, vehicle sprites, lane labels
  input.rs        — SDL key → InputEvent mapping
  stats.rs        — per-frame metric collection (velocity, crossing time, close calls)
  stats_window.rs — post-Esc statistics display window

assets/
  roads/          — intersection_core.bmp, approach_ns.bmp, approach_ew.bmp
  vehicles/       — vehicle_{north,south,east,west}.bmp
```

**How the smart controller works**

1. A vehicle spawns at the edge of the window and travels toward the junction.
2. When it comes within 200 world-units of the zone boundary, the `SmartController` registers it as a waiter and applies reservation braking so it can stop before entering.
3. Reservations are granted in FIFO arrival order. A vehicle is blocked while any other vehicle with a geometrically conflicting path is already inside the zone or holds an earlier reservation.
4. Once granted, the vehicle crosses the junction (`Managed` state) at its nominal speed.
5. On exit from the zone it moves to `Exiting`, the reservation releases, and the next waiter can enter.
6. Vehicles on non-conflicting paths receive reservations simultaneously.

**Vehicle lifecycle states**

| State | Meaning |
|-------|---------|
| `Approaching` | Travelling toward the intersection; subject to follow-distance and reservation braking |
| `Managed` | Inside the junction zone; speed controlled by the smart controller |
| `Exiting` | Has left the zone; still on screen, subject to follow-distance |
| `Done` | Off-screen; removed from the simulation |

**Velocity levels** — assigned at spawn, cycling Fast → Cruise → Yield by vehicle ID:

| Level | Speed (world units/s) |
|-------|----------------------|
| Fast | ≈ 4.9 |
| Cruise | ≈ 3.5 |
| Yield | ≈ 1.75 |

Each vehicle also has a distinct acceleration/deceleration profile, so braking ramps gradually rather than snapping.

## Statistics Window

Pressing Esc ends the session and opens a separate window showing:

- Max vehicles passed through the intersection
- Max / min observed velocity
- Max / min time to cross the intersection
- Close-call count (vehicle pairs that came within safe distance)
- Session duration, average crossing time, peak concurrent vehicles in zone, total vehicles that entered the zone

## Audit Dry-Run (Gate G2)

All tickets A01–A08, B01–B05, C01–C08 are merged. Run before an audit:

```bash
cargo test --lib && cargo test --test smoke
```

Then launch and manually verify:

- Vehicles animate visibly through turns (rotation follows path tangent)
- Approaching vehicles decelerate gradually behind stopped leaders
- No sprite overlap during normal operation
- Statistics window appears after Esc with all required fields populated

**Automated audit coverage** (`tests/smoke.rs`):

| AUD IDs | Smoke test |
|---------|------------|
| AUD-8 | `crate_smoke_audit8_three_same_lane_all_approaches` |
| AUD-9–14 | `crate_smoke_audit9` … `audit14` |
| AUD-15 | `crate_smoke_aud15_scheduler_yields_without_proximity_clamp` |
| AUD-16–17 | `crate_smoke_aud16_aud17_sustained_no_overlap_no_lane_overflow` |
| AUD-18 | `crate_smoke_audit18_four_vehicle_session_no_collision` |
| AUD-19 | `crate_smoke_audit19_stats_window_is_separate_surface` |
| AUD-20–24 | `crate_smoke_session_stats_populated_before_esc_exit` |
| AUD-25 | `crate_smoke_audit25_single_vehicle_equal_crossing_times` |

Manual spot-checks required for AUD-1–2 (visual assets), AUD-7 (varied random spawns), AUD-26 (stopwatch vs reported crossing time), AUD-28 (turn animation), and AUD-31 (three visible speed levels).

**Known limitations** (see [`docs/audit_plan.md`](docs/audit_plan.md)):

- Managed-zone speed changes are instantaneous; gradual ramp (B05) applies only on approach lanes.
- Center-to-center safe distance (40 px) is only marginally larger than vehicle length (36 px) — vehicles may look nearly bumper-to-bumper without visually overlapping.

## Documentation

| Doc | Purpose |
|-----|---------|
| [docs/requirements.md](docs/requirements.md) | Stakeholder requirements |
| [docs/SDS.md](docs/SDS.md) | Software design specification |
| [docs/audit.md](docs/audit.md) | Acceptance checklist (AUD-1–AUD-31) |
| [docs/audit_plan.md](docs/audit_plan.md) | Audit dry-run notes and known limitations |
| [docs/ticket-tracker.md](docs/ticket-tracker.md) | A/B/C track ticket history |

## License

MIT
