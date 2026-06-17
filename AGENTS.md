# AGENTS.md — smart-road

> Coding agent instructions for **smart-road**.
> Read this file in full before writing any code.

---

## Project Overview

**smart-road** is a smart intersection autonomous-vehicle simulation (Rust/SDL2).

Primary scope in this repository:

- satisfy `docs/requirements.md`
- satisfy `docs/audit.md`
- deliver a runnable cross-intersection simulation passing audit

---

## Source of Truth

| Document | Path | Purpose |
|----------|------|---------|
| Requirements | `docs/requirements.md` | What must be built (external or stakeholder spec) |
| Audit Gate | `docs/audit.md` | Pass/fail acceptance criteria |
| PRD | `docs/PRD.md` | Detailed requirements and architecture |
| SDS | `docs/SDS.md` | Technical spec, API contracts, and examples |
| Ticket Tracker | `docs/ticket-tracker.md` | Work breakdown, status, and traceability |
| Agent Workflow | `docs/AGENT_WORKFLOW.md` | How agents navigate docs and close tickets |
| This file | `AGENTS.md` | Agent coding guidelines |

**Always verify your work against `docs/audit.md` before marking a ticket done.**

---

## Technology Constraints

| Rule | Detail |
|------|--------|
| **Language** | Rust (edition 2021) |
| **Runtime / Platform** | Native desktop binary (SDL2) |
| **Package manager** | cargo |
| **Frameworks** | **Allowed:** `sdl2`, `sdl2-image` (via crate features). **Forbidden:** full game engines (Bevy, macroquad, etc.) unless explicitly approved in a ticket |
| **Dependencies** | `sdl2` is required; avoid new runtime deps without noting in PR/ticket |
| **Styling** | N/A (native SDL2 render) |
| **Database** | N/A |

---

## Directory Structure

```
smart-road/
├── src/                    # Application source (main, app, modules)
│   ├── main.rs
│   ├── app.rs
│   ├── config.rs
│   ├── intersection.rs
│   ├── vehicle.rs
│   ├── smart.rs
│   ├── spawn.rs
│   ├── render.rs
│   ├── input.rs
│   ├── stats.rs
│   └── stats_window.rs
├── assets/                 # Sprites, tiles, fonts
│   ├── vehicles/
│   ├── roads/
│   └── fonts/
├── tests/                  # Integration and unit tests
├── docs/
│   ├── requirements.md     # Stakeholder requirements
│   ├── audit.md            # Acceptance checklist
│   ├── PRD.md              # Product requirements
│   ├── SDS.md              # Technical design spec
│   ├── ticket-tracker.md   # Sprint/ticket status
│   ├── AGENT_WORKFLOW.md   # Agent navigation guide
│   ├── raw/                # Read-only source briefs
│   └── pr-messages/        # Per-ticket PR handover artifacts
├── .agents/
│   ├── workflows/          # Repeatable agent playbooks
│   ├── skills/             # Project-specific agent skills
│   └── rules/              # Supplemental agent rules
├── .cursor/rules/          # Cursor IDE rules
├── Cargo.toml
├── AGENTS.md               # THIS FILE
└── README.md
```

---

## Coding Standards

### General

- Prefer `const` for fixed values; use `config` module for tunables (safe distance, speeds).
- Keep modules focused: intersection topology ≠ render ≠ smart algorithm.
- No `println!` in hot paths; use `log` or stderr only for startup/errors if needed.
- Follow existing patterns in the codebase before introducing new abstractions.

### Naming Conventions

| Entity | Convention | Example |
|--------|-----------|---------|
| Files | `snake_case.rs` | `stats_window.rs` |
| Functions | `snake_case` | `schedule_velocity`, `spawn_vehicle` |
| Structs / enums | `PascalCase` | `SmartController`, `VehicleState` |
| Constants | `SCREAMING_SNAKE_CASE` | `SAFE_DISTANCE`, `MAX_VELOCITIES` |
| Private fields | `snake_case` | `time_in_crossing` |

### Module Pattern

- Library-style modules under `src/` with `pub` only where needed for tests.
- `main.rs` wires `App`; domain logic stays out of `main`.
- Shared types (`LaneId`, `Route`, `Vec2`) live in the smallest owning module and re-export if cross-cutting.

### Domain-Specific Rules

- **Smart controller owns intersection velocity commands** — render and spawn must not override commanded speed inside the managed zone.
- **Route adherence** — vehicles stay on precomputed lane polylines; no mid-path lane changes.
- **Statistics** — crossing timer starts at smart-system detection (REQ-4), not at spawn.
- **Animation** — update vehicle `heading` from path tangent; use SDL rotation when blitting.

---

## Testing Guidelines

- **Runner**: `cargo test` (built-in Rust test harness)
- **Location**: `tests/` for integration; `#[cfg(test)]` in modules for unit tests
- **Coverage expectation**: physics, spawn cooldown, and smart scheduling have unit tests; visual AUD items verified manually per `docs/audit.md`
- **Run command**: `cargo test`

Before completing a ticket:

- [ ] New logic has tests where applicable.
- [ ] All existing tests pass.
- [ ] Manual audit items from `docs/audit.md` relevant to this ticket are verified.

---

## Development Workflow

```bash
cargo build              # Compile (requires SDL2 dev libraries on system)
cargo run                # Run simulation
cargo test               # Run test suite
cargo clippy -- -D warnings   # Lint
cargo fmt --check        # Format check
cargo build --release    # Release build (when needed)
```

> SDL2 system dependency must be installed per platform (document in README). Not yet scripted in repo.

---

## Commit & Branch Conventions

| Branch | Purpose |
|--------|---------|
| `main` | Stable, audit-ready code |
| `{username}/{ticket-id}` | Per-ticket work branches (e.g. `andy/B02`, `andy/A04-arrow-spawn`) — named by the developer |

Commit messages: `feat(A01): add SDL2 window and empty loop` (type, track ticket ID, short description)

Agents do not create or rename branches; developers choose their own `{username}/{ticket-id}` branch.

### Multi-developer rules

- **One ticket per branch** — each developer works on `{username}/{ticket-id}` (optional short suffix).
- **Stay in your track's modules** — see file ownership in `docs/SDS.md` §13.1; do not edit another track's owned files without updating the interface section first.
- **Cross-track deps** — if ticket lists a 🔗 dep (e.g., `B01` needs `A04`), that ticket must be ✅ before you start.
- **PR message path** — `docs/pr-messages/A01-short-desc-pr.md` (track prefix required).
- **Integration** — `app.rs` / `main.rs` changes go through Track A unless C07 session hook is explicitly scoped.

---

## PR Checklist (for each ticket)

- [ ] Code compiles and runs (`cargo run`).
- [ ] All existing tests pass (`cargo test`).
- [ ] New code has tests where applicable.
- [ ] Lint/format checks pass (`cargo clippy`, `cargo fmt --check`).
- [ ] No unapproved dependencies added.
- [ ] Audit checklist items covered by this ticket still pass (`docs/audit.md`).
- [ ] Documentation updated if public API or behavior changed.
- [ ] `docs/ticket-tracker.md` updated with ticket status.
- [ ] PR message saved to `docs/pr-messages/` using `docs/pr-messages/pr-template.md`.
