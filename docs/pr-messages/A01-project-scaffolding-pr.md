---
title: "feat(A01): project scaffolding"
---

# PR Implementation Report: A01

## Summary

Initial Rust/SDL2 project scaffold for **smart-road**: Cargo crate with lib+binary layout, SDL2 window and empty game loop, module skeleton per `docs/SDS.md`, `assets/` directory tree, and `.gitignore`. Satisfies **NFR-1** (Rust), **NFR-2** (SDL2), **NFR-4** (native binary).

## Key Changes

- **`Cargo.toml`**: `smart-road` package, `sdl2` 0.37, lib + bin targets.
- **`src/`**: `lib.rs`, `main.rs`, `app.rs` (SDL2 loop), skeleton modules (`config`, `intersection`, `vehicle`, `smart`, `spawn`, `render`, `input`, `stats`, `stats_window`).
- **`assets/`**: `vehicles/`, `roads/`, `fonts/` with `.gitkeep`.
- **`.gitignore`**: Rust/Cargo, IDE, OS artifacts.

## Technical Decisions

- **Lib + bin split**: `smart_road` library enables integration tests without launching SDL2 (A02).
- **`sdl2` without `image` feature**: defers `libsdl2-image-dev` requirement to **A03** when assets load.
- **Empty loop**: poll events (Quit/Esc), no-op `update`, clear dark-green frame via `render::draw_frame` stub.
- **Canvas draw APIs**: SDL2 0.37 `set_draw_color` / `clear` / `present` return `()` — no `Result` wrapping.

## Verification Results

### Automated Checks

- [x] `cargo build` succeeds
- [x] `cargo run` opens blank window (manual — closes on Esc or window X)
- [x] `cargo test` passes (config unit tests only at A01)

### Manual Audit (against `docs/audit.md`)

- N/A — no AUD IDs on A01 (NFR-only ticket).

### Requirements Traceability

- [x] **NFR-1**: Rust edition 2021 crate
- [x] **NFR-2**: SDL2 window and event loop
- [x] **NFR-4**: Native desktop binary via `cargo run`

## Artifacts

- **Build output**: `Finished dev profile` — clean compile
- **PR message**: `docs/pr-messages/A01-project-scaffolding-pr.md`

---

## Next Steps

- **A02** — smoke tests, clippy/fmt, README (✅ landed same session)
- **A03** — intersection render + lane registry stub
