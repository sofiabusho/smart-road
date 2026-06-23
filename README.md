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

### SDL2_image (optional, later tickets)

PNG vehicle sprites (ticket **A07+**) may use the `sdl2` `image` feature. Install when needed:

```bash
# Ubuntu / Debian
sudo apt install -y libsdl2-image-dev
```

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
| [docs/ticket-tracker.md](docs/ticket-tracker.md) | A/B/C track tickets |

## License

MIT — see [LICENSE](LICENSE).
