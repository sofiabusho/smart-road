# Product Requirements Document — smart-road

> Version 0.1 · Date 2026-06-10 · Status DRAFT

## 1. Executive Summary

**smart-road** is a Rust/SDL2 desktop simulation of autonomous vehicles crossing a four-way intersection without traffic lights. A smart intersection algorithm coordinates passage by adjusting vehicle velocity, timing, and spacing. Auditors and testers drive the simulation entirely via keyboard, then validate behavior against `docs/audit.md`.

The project must deliver:

1. A runnable cross-intersection simulation with animated vehicles and road assets.
2. Keyboard-controlled vehicle spawning and session exit with a statistics summary window.
3. Collision-free, low-congestion crossing under scripted and sustained random traffic scenarios.

## 2. Goals and Non-Goals

### 2.1 Goals

| ID | Goal |
|----|------|
| G1 | Zero collisions under all mandatory audit collision scenarios |
| G2 | Visible smart coordination (velocity changes, safe spacing) without traffic lights |
| G3 | Auditor-ready keyboard flows and exit statistics matching REQ-19–REQ-26 |
| G4 | Animated turns — vehicle orientation follows route through intersection |

### 2.2 Non-Goals

- Emergency vehicles, pedestrians, or mixed human/AV traffic.
- Traffic lights, stop signs, or traditional signal phases.
- Lane changing, overtaking outside assigned route, or dynamic rerouting.
- Multi-intersection networks, real maps, or production AV stack integration.
- Automated CI reproduction of all manual audit steps (manual audit remains authoritative for AUD-3–AUD-26).

## 3. User Flows

### 3.1 Auditor / tester — basic inspection

```text
cargo run
  → See cross intersection (AUD-1, AUD-2)
  → Press each arrow key once
  → Confirm spawn from correct cardinal direction (AUD-3–AUD-6)
```

### 3.2 Auditor — collision scenarios

```text
Spawn scripted groups (same lane, opposing entries, mixed batches)
  → Watch vehicles traverse intersection
  → Confirm no overlap/collision (AUD-8–AUD-14)
  → Confirm slowdown when paths conflict (AUD-15)
```

### 3.3 Auditor — sustained traffic

```text
Hold/toggle R ≥ 60 seconds
  → No collisions (AUD-16)
  → No lane stuck with ≥8 vehicles (AUD-17)
```

### 3.4 Auditor — statistics session

```text
Spawn 2× Up + 2× Right → wait for all to exit
  → Press Esc
  → Stats window shows all required fields; max vehicles = 4 (AUD-18–AUD-24)

Spawn 1 vehicle → time crossing manually → Esc
  → max time = min time; value matches observation (AUD-25, AUD-26)
```

### 3.5 Developer — quality check

```text
cargo test
cargo clippy
cargo fmt --check
cargo run
```

## 4. Architecture Overview

```text
┌─────────────────────────────────────────────────────────┐
│                    SDL2 Event Loop                       │
│  keyboard → spawn / R-random / Esc-exit                  │
└────────────┬───────────────────────────────┬────────────┘
             │                               │
             ▼                               ▼
┌────────────────────────┐      ┌───────────────────────────┐
│   Render + Animation   │      │  Smart Intersection Ctrl  │
│  roads, sprites, rotate│◄────►│  detect, schedule, yield  │
└────────────────────────┘      └─────────────┬─────────────┘
                                              │
                                              ▼
                                ┌───────────────────────────┐
                                │   Vehicle Physics Engine   │
                                │ v=d/t, safe dist, ≥3 speeds│
                                └─────────────┬─────────────┘
                                              │
                                              ▼
                                ┌───────────────────────────┐
                                │   Statistics Collector     │
                                │  counts, min/max, close call│
                                └─────────────┬─────────────┘
                                              │ Esc
                                              ▼
                                ┌───────────────────────────┐
                                │   Statistics Window        │
                                └───────────────────────────┘
```

## 5. Detailed Requirements (by priority)

### 5.1 P0 — Must ship for audit

| Area | REQ IDs | Priority |
|------|---------|----------|
| Cross intersection + assets | REQ-1, REQ-2, REQ-10 | P0 |
| Smart algorithm, no lights | REQ-3, REQ-4, REQ-9 | P0 |
| Physics + safe distance + ≥3 speeds | REQ-5–REQ-8 | P0 |
| Keyboard spawn + anti-spam + R + Esc | REQ-12–REQ-18 | P0 |
| Turn animation | REQ-11 | P0 |
| Exit statistics | REQ-19–REQ-26 | P0 |
| Rust + SDL2 | NFR-1, NFR-2, NFR-3 | P0 |

### 5.2 P1 — Polish

| Area | REQ IDs | Priority |
|------|---------|----------|
| Congestion quality under sustained `R` | REQ-3 | P1 |
| README build/run clarity | NFR-5 | P1 |

### 5.3 P2 — Bonus (optional)

| Area | REQ IDs | Priority |
|------|---------|----------|
| Custom assets | REQ-B1 | P2 |
| Extra stats | REQ-B2 | P2 |
| Acceleration/deceleration | REQ-B3 | P2 |

## 6. Technology Constraints

| Concern | Decision |
|---------|----------|
| Language | Rust (edition per `Cargo.toml`, likely 2021) |
| Runtime | Native desktop binary via SDL2 |
| Graphics/input | `sdl2` crate; no alternative game engines without approval |
| Dependencies | Minimize deps; `sdl2` required; new runtime deps need justification in ticket/PR |
| Tooling | `cargo`, `cargo test`, `cargo clippy`, `cargo fmt` |

## 7. Repository Structure

```text
smart-road/
├── src/                 # lib + bin (main loop, modules)
├── assets/              # sprites, tiles, fonts
├── tests/               # integration/unit tests
├── docs/                # requirements, audit, PRD, SDS, tracker
├── .agents/workflows/   # implement-ticket playbook
├── AGENTS.md
├── Cargo.toml
└── README.md
```

## 8. Acceptance and Audit Mapping

Primary gate: `docs/audit.md`

| Category | AUD range | Blocking |
|----------|-----------|----------|
| Core functionality | AUD-1–AUD-26 | Yes |
| General | AUD-27–AUD-31 | Yes |
| Bonus | AUD-B1–AUD-B4 | No |

## 9. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| SDL2 system library missing on auditor machine | HIGH | Document distro-specific install in README; fail fast with clear error |
| Smart algorithm deadlock under heavy `R` traffic | HIGH | Reservation/time-slot design; cap in-queue vehicles; test AUD-16/17 early |
| Crossing-time stat drift vs visual observation | MEDIUM | Tie timer to algorithm detection + despawn events; unit-test clock |
| Turn animation looks like static slide | MEDIUM | Explicit rotation/interpolation per route segment (REQ-11) |
| Spawn spam if key-repeat unhandled | MEDIUM | Debounce/cooldown per direction (REQ-18) |

## 10. Open Questions

| # | Question | Assumption until resolved |
|---|----------|---------------------------|
| OQ-1 | Exact key-repeat semantics for `R` — hold vs toggle? | Hold `R` enables continuous random spawn each N frames; document in README |
| OQ-2 | World scale and safe-distance value? | Choose constant ≥ 1 vehicle length; expose in `config` module |
| OQ-3 | How many lanes per approach (brief diagram implies 3: r/s/l)? | Three lanes per arm, each with fixed route per REQ-2 |
| OQ-4 | Statistics window — second SDL window vs overlay panel? | Separate SDL window on `Esc`; simplest for auditors |
| OQ-5 | Units for velocity/time in stats display? | World units per second and seconds; label clearly in UI |
| OQ-6 | Lane assignment on arrow spawn — random among r/s/l or fixed per key? | Random route among the three lanes for that approach unless brief implies otherwise; arrow only fixes **approach direction** |

## 11. Success Metrics

1. All mandatory AUD items (AUD-1–AUD-31) pass on auditor walkthrough.
2. `cargo test`, `cargo clippy`, and `cargo fmt --check` pass in CI/local dev.
3. Sustained `R` traffic ≥60s completes with no collisions and no lane queue ≥8 vehicles.

## Cross-References

| Document | Relationship |
|----------|--------------|
| `docs/requirements.md` | Authoritative stakeholder requirements |
| `docs/SDS.md` | Technical implementation of this PRD |
| `docs/audit.md` | Acceptance gate |
| `docs/ticket-tracker.md` | Implementation traceability |
