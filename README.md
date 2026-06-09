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

### SDL2_image (later tickets)

When loading road/vehicle assets (ticket **A03**), install the image development library:

```bash
# Ubuntu / Debian
sudo apt install -y libsdl2-image-dev
```

The `sdl2` crate `image` feature will be enabled in that ticket.

## Build and run

```bash
cargo build
cargo run
```

A window titled **smart-road** opens with an empty dark-green loop. Close the window or press **Esc** to quit.

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
