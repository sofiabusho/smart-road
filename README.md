# smart-road

Smart intersection autonomous-vehicle simulation (Rust / SDL2) for the 01-edu Smart Road project.

## Prerequisites

### SDL2 (required)

**Ubuntu / Debian / WSL2 (Debian-based)**

```bash
sudo apt update
sudo apt install -y libsdl2-dev
```

**Fedora**

```bash
sudo dnf install SDL2-devel
```

**macOS (Homebrew)**

```bash
brew install sdl2
```

**Windows**

Install SDL2 development libraries via [vcpkg](https://vcpkg.io/) or the official SDL2 MSVC development package, then ensure `SDL2.dll` is on your `PATH` when running the binary.

### Rust

Install [Rust](https://www.rust-lang.org/tools/install) (edition 2021). The project uses Cargo:

```bash
cargo --version
```

### Road assets (A03+)

Road tiles live in `assets/roads/` (BMP). Regenerate with:

```bash
python3 scripts/generate_road_assets.py
```

Layout constants in `src/config.rs` and `scripts/generate_road_assets.py` must stay in sync; rerun the script after changing window size, margins, or lane dimensions.

### Vehicle sprites (A08+)

Vehicle BMPs live in `assets/vehicles/` (one per approach, eastbound authorship).
Regenerate Time Fantasy–style placeholders with:

```bash
python3 scripts/generate_vehicle_sprites.py
```

**Attribution:** pixel-art style by [Jason Perry (finalbossblues)](https://finalbossblues.itch.io/) — see [assets/vehicles/ATTRIBUTION.md](assets/vehicles/ATTRIBUTION.md). You may replace the shipped BMPs with frames from the free [Pixel Shooter and Towers Asset Pack](https://finalbossblues.itch.io/pixel-shooter-towers-asset-pack) (public domain).

Turn animation (A07) rotates these sprites via SDL `copy_ex` and `heading_rad` from lane path tangents.

### SDL2_image (optional)

To load PNG vehicle sprites directly, enable the `image` feature in `Cargo.toml` and install:

```bash
# Ubuntu / Debian
sudo apt install -y libsdl2-image-dev
```

Without SDL2_image, use 32-bit BMP with alpha (default).

## Controls

| Key | Action |
|-----|--------|
| **Arrow Up** | Spawn from **South** (vehicle travels north) |
| **Arrow Down** | Spawn from **North** (vehicle travels south) |
| **Arrow Right** | Spawn from **West** (vehicle travels east) |
| **Arrow Left** | Spawn from **East** (vehicle travels west) |
| **R** (hold) | Continuous random spawn (approach + route) |
| **Esc** | End session and open **statistics window** |

Spawn cooldown (~400 ms per approach) prevents overlapping spawns on the same direction. Close the stats window (or press **Esc** again) to quit after a session.

## Audit dry-run (Gate G2)

Full acceptance checklist: **[docs/audit.md](docs/audit.md)** (AUD-1–AUD-31).

**Quick verification workflow** (C07):

1. Install SDL2 and build (`cargo build`). Run `cargo test` — all tests should pass.
2. Start the sim: `cargo run`.
3. Walk **§1 Core Functionality** in `docs/audit.md` in order (spawn keys → collision scenarios → sustained `R` traffic → stats on **Esc**).
4. Run `cargo clippy -- -D warnings` and `cargo fmt --check` for NFR-5 evidence.

**Automated audit mirrors** (smoke tests in `tests/smoke.rs`):

| AUD IDs | Smoke test |
|---------|------------|
| AUD-8 | `crate_smoke_audit8_three_same_lane_all_approaches` |
| AUD-9–AUD-14 | `crate_smoke_audit9` … `audit14` |
| AUD-15 | `crate_smoke_aud15_scheduler_yields_without_proximity_clamp` |
| AUD-16–AUD-17 | `crate_smoke_aud16_aud17_sustained_no_overlap_no_lane_overflow` |
| AUD-18 | `crate_smoke_audit18_four_vehicle_session_no_collision` |
| AUD-19 | `crate_smoke_audit19_stats_window_is_separate_surface` |
| AUD-20–AUD-24 | `crate_smoke_session_stats_populated_before_esc_exit` |
| AUD-25 | `crate_smoke_audit25_single_vehicle_equal_crossing_times` |
| AUD-27–AUD-30 | unit tests in `spawn.rs`, `vehicle.rs` |

Manual spot-checks still required for **AUD-1–AUD-2** (visual assets), **AUD-7** (varied random spawns), **AUD-28** (route adherence / turn animation), **AUD-26** (stopwatch vs reported crossing time), and **AUD-31** (three visible speed levels).

**Known limitations** (documented in `docs/audit_plan.md`):

- Managed-zone velocity changes are instantaneous (B05 ramp applies on approach lanes, not inside the scheduler zone).
- Center-to-center safe distance is 40 px vs 36 px vehicle length — vehicles may look nearly bumper-to-bumper without overlapping.

## Build and run

```bash
cargo build
cargo run
```

A window titled **smart-road** opens showing a **four-way cross intersection** with road tile assets. Press **Esc** after a session to close the simulation and open the **session statistics** window. Close that window (or press **Esc** again) to quit. You can also close the main window directly without viewing stats.

## Development commands

```bash
cargo test                  # Unit + integration smoke tests
cargo clippy -- -D warnings # Lint (warnings denied)
cargo fmt                   # Format source
cargo fmt --check           # CI-style format check
cargo build --release       # Optimized binary
```

## Project layout

```text
src/           # Application modules (see docs/SDS.md)
assets/        # Sprites and tiles (vehicles/, roads/, fonts/)
tests/         # Integration tests
docs/          # Requirements, audit, PRD, SDS, ticket tracker
```

## Documentation

| Doc | Purpose |
|-----|---------|
| [AGENTS.md](AGENTS.md) | Agent coding guidelines |
| [docs/requirements.md](docs/requirements.md) | Stakeholder requirements |
| [docs/audit.md](docs/audit.md) | Acceptance checklist |
| [docs/audit_plan.md](docs/audit_plan.md) | Audit dry-run notes and known limitations |
| [docs/ticket-tracker.md](docs/ticket-tracker.md) | A/B/C track tickets |

## License

MIT — see [LICENSE](LICENSE).
