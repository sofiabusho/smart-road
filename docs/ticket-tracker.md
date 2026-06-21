# Ticket Tracker — smart-road

> Legend: 🔴 Blocked · 🟡 Ready · 🟢 In Progress · ✅ Done · ⬜ Not Started
>
> **ID Legend**:
> - **REQ-***: Functional Requirements (`docs/requirements.md`)
> - **AUD-***: Audit Acceptance Criteria (`docs/audit.md`)
> - **A/B/C**: Parallel developer tracks (see §2)
> - **🔗**: Cross-track dependency — external ticket must be ✅ first

Last refreshed: 2026-06-17 (A01–A06, B01–B04, C01, C05 ✅)

---

## Migration: T-series → A/B/C tracks

| Old ID | New ID | Track |
|--------|--------|-------|
| T01 | **A01** | A |
| T02 | **A02** | A |
| T03 | **A03** | A |
| T04 | **A04** | A |
| T05 | **A05** | A |
| T06 | **A06** | A |
| T15 | **A07** | A |
| T19 | **A08** | A (bonus) |
| T07 | **B01** | B |
| T08 | **B02** | B |
| T09 | **B03** | B |
| T10 | **B04** | B |
| T21 | **B05** | B (bonus) |
| T11 | **C01** | C |
| T12 | **C02** | C |
| T13 | **C03** | C |
| T14 | **C04** | C |
| T16 | **C05** | C |
| T17 | **C06** | C |
| T18 | **C07** | C |
| T20 | **C08** | C (bonus) |

---

## 1) Scope Contract

This tracker is **requirements-first** and **audit-first**, organized for **three parallel developers**.

Execution order:

1. `docs/requirements.md` — stakeholder requirements
2. `docs/audit.md` — acceptance gates
3. Feature delivery per track → `docs/SDS.md` §13 cross-track interfaces
4. Optional stretch goals (A08, B05, C08 — non-blocking)

---

## 2) Team Assignment

| Track | Developer focus | Primary `src/` modules | Ticket range |
|-------|-----------------|------------------------|--------------|
| **A** | Platform & presentation | `main.rs`, `app.rs`, `config.rs`, `intersection.rs` (topology/visual), `spawn.rs`, `render.rs`, `input.rs` | A01–A08 |
| **B** | Vehicle simulation | `vehicle.rs`, `intersection.rs` (path polylines only — coordinate with A) | B01–B05 |
| **C** | Smart control & delivery | `smart.rs`, `stats.rs`, `stats_window.rs`, `app.rs` (session lifecycle hooks) | C01–C08 |

**Integration owner**: whoever lands **C07** runs the full audit dry-run; all tracks participate in merge order per `docs/SDS.md` §13.

---

## 3) Requirement and Audit IDs (index)

### Requirements

- `REQ-1` … `REQ-26` — see `docs/requirements.md`
- `REQ-B1` … `REQ-B3` — bonus (optional)
- `NFR-1` … `NFR-5` — non-functional

### Audit

- `AUD-1` … `AUD-31` — mandatory
- `AUD-B1` … `AUD-B4` — bonus (non-blocking)

---

## Track A — Platform & presentation

> **Goal**: Runnable SDL2 shell, visible intersection, keyboard spawn, turn animation.

| ID | Status | Ticket | Size | Deps | REQ / AUD | Verification gate | Track |
|----|--------|--------|------|------|-----------|-------------------|-------|
| A01 | ✅ | **Project scaffolding**: `cargo init`, SDL2 window, empty game loop, `assets/` dirs | S | — | NFR-1, NFR-2, NFR-4 | `cargo run` opens blank window; `cargo build` succeeds | A |
| A02 | ✅ | **Quality gates**: `cargo test` harness, clippy/fmt notes in README | S | A01 | NFR-5 | `cargo test`; `cargo clippy`; `cargo fmt --check` | A |
| A03 | ✅ | **Intersection render**: cross layout, road assets, lane ID registry stub | M | A01 | REQ-1, REQ-2, REQ-10 · AUD-1, AUD-2 | AUD-1, AUD-2 | A |
| A04 | ✅ | **Arrow-key spawn**: four cardinal approaches, `SpawnRequest` API | M | A03 | REQ-12–REQ-15 · AUD-3–AUD-6 | AUD-3–AUD-6 | A |
| A05 | ✅ | **Spawn anti-spam**: per-direction cooldown | S | A04 | REQ-18 · AUD-27 | AUD-27 | A |
| A06 | ✅ | **`R` random spawn**: continuous random lane/route | S | A04 | REQ-16 · AUD-7 | AUD-7 | A |
| A07 | ⬜ | **Turn animation**: sprite rotation along path tangent | M | A04, B02 🔗 | REQ-11 | Visual check; path alignment with AUD-28 | A |
| A08 | ⬜ | **Custom assets** *(bonus)* | M | A03 | REQ-B1 · AUD-B2 | AUD-B2 | A |

**Intra-track chain**: A01 → A03 → A04 → A05 / A06; A07 after B02 delivers path tangents.

---

## Track B — Vehicle simulation

> **Goal**: Physics, fixed routes, velocity levels, safe-distance follow logic.

| ID | Status | Ticket | Size | Deps | REQ / AUD | Verification gate | Track |
|----|--------|--------|------|------|-----------|-------------------|-------|
| B01 | ✅ | **Vehicle physics**: position along path, v=d/t fields | M | A04 🔗 | REQ-5 | Unit tests; AUD-26 (with C06) | B |
| B02 | ✅ | **Route adherence**: lane-locked polylines on `IntersectionModel` | M | B01, A03 🔗 | REQ-2, REQ-6 · AUD-28 | AUD-28 | B |
| B03 | ✅ | **Velocity levels**: ≥3 distinct speeds | S | B01 | REQ-7 · AUD-31 | AUD-31 | B |
| B04 | ✅ | **Safe distance**: follow logic, positive constant | M | B01 | REQ-8 · AUD-29, AUD-30 | AUD-29, AUD-30 | B |
| B05 | ⬜ | **Acceleration / deceleration** *(bonus)* | M | B01 | REQ-B3 · AUD-B3 | AUD-B3 | B |

**Intra-track chain**: B01 → B02 → B03 → B04 (B03/B04 can parallel after B01 if stubs stable).

---

## Track C — Smart intersection & stats

> **Goal**: Smart algorithm, sustained traffic, statistics, audit-ready delivery.

| ID | Status | Ticket | Size | Deps | REQ / AUD | Verification gate | Track |
|----|--------|--------|------|------|-----------|-------------------|-------|
| C01 | ✅ | **Smart detection + timer start**: managed zone entry | M | B02 🔗 | REQ-3, REQ-4, REQ-23 | AUD-25, AUD-26 (with C06) | C |
| C02 | ⬜ | **Smart scheduler**: velocity/time coordination | L | C01, B03 🔗, B04 🔗 | REQ-3, REQ-9 · AUD-8–AUD-14 | AUD-8–AUD-14 | C |
| C03 | ⬜ | **Yield on conflict**: slowdown avoidance | M | C02 | REQ-9 · AUD-15 | AUD-15 | C |
| C04 | ⬜ | **Sustained traffic**: R ≥1 min, congestion cap | M | C02, A06 🔗 | REQ-3 · AUD-16, AUD-17 | AUD-16, AUD-17 | C |
| C05 | ✅ | **Stats collector**: min/max velocity, crossing times, close calls | M | C01 | REQ-20–REQ-26 | AUD-20–AUD-24 | C |
| C06 | ⬜ | **Stats window on Esc**: display all fields | M | C05 | REQ-17, REQ-19 · AUD-18, AUD-19 | AUD-18–AUD-24, AUD-25 | C |
| C07 | ⬜ | **Audit dry-run**: full AUD-1–AUD-31 pass, README runbook | S | C06, A07 🔗, C03 🔗, C04 🔗 | NFR-5 · AUD-1–AUD-31 | Gate G2 | C |
| C08 | ⬜ | **Extra statistics** *(bonus)* | S | C05 | REQ-B2 · AUD-B1 | AUD-B1 | C |

**Intra-track chain**: C01 → C02 → C03; C01 → C05 → C06 → C07.

**C07** intentionally waits for animation (A07) and collision scenarios (C03, C04) before audit sign-off.

---

## Cross-track dependency map

```text
A01 ──→ A03 ──→ A04 ──┬──→ A05
                      ├──→ A06 ──────────────┐
                      │                      │
                      └──→ B01 ──→ B02 ──→ C01 ──→ C02 ──┬──→ C03
                              │         ↑    │              ├──→ C04 ←─ A06
                              │    B03 ─┘    │              └──→ C05 ──→ C06 ──→ C07
                              │    B04 ──────┘                      ↑
                              └──→ A07 (render rotation)            └── needs A07, C03, C04
```

| Ticket | Cross-track deps (🔗) |
|--------|------------------------|
| B01 | A04 |
| B02 | A03 |
| A07 | B02 |
| C01 | B02 |
| C02 | B03, B04 |
| C04 | A06 |
| C07 | A07, C03, C04 |

No circular dependencies.

---

## Parallel work queue (per developer)

### Dev A (Platform & presentation)

| When | Pick up | Blocked by |
|------|---------|------------|
| ~~**Start**~~ | ~~**A01**~~ ✅ · ~~**A02**~~ ✅ · ~~**A03**~~ ✅ · ~~**A04**~~ ✅ · ~~**A05**~~ ✅ · ~~**A06**~~ ✅ | — |
| **Now** | **A07** (turn animation) | — |
| Anytime after A03 ✅ | **A08** *(bonus)* | — |

### Dev B (Vehicle simulation)

| When | Pick up | Blocked by |
|------|---------|------------|
| While waiting | Read `docs/SDS.md` §13 stubs; draft `vehicle.rs` types offline | — |
| **Now** | **B05** *(bonus)* | — |
| After B01 ✅ | ~~**B03** ∥ **B04**~~ ✅ | — |

### Dev C (Smart control & stats)

| When | Pick up | Blocked by |
|------|---------|------------|
| While waiting | Read `docs/SDS.md` §13; sketch `smart.rs` / `stats.rs` interfaces | B02 |
| After B02 ✅ | **C01** | B02 🔗 |
| After C01 + B03 + B04 ✅ | **C02** | — |
| After C02 ✅ | **C03** ∥ **C04** (C04 also needs A06 ✅) | A06 🔗 for C04 |
| **Now** | **C06** (stats window) | — |
| After C01 ✅ | ~~**C05**~~ ✅ (can overlap with C02) | — |
| After C06 + A07 + C03 + C04 ✅ | **C07** | A07 🔗, C03, C04 |
| After C05 ✅ | **C08** *(bonus)* | — |

---

## Verification Gates

### Gate G1 — Playable intersection (Required)

**Pass criteria**:

- **Track A**: A01, A02, A03, A04, A05, A06 ✅
- AUD-1–AUD-7, AUD-27 pass

**Evidence**: PR messages `A01`–`A06`; manual audit notes.

### Gate G2 — Audit-ready delivery (Required)

**Pass criteria**:

- **Track A**: A01–A07 ✅ (A08 optional)
- **Track B**: B01–B04 ✅ (B05 optional)
- **Track C**: C01–C07 ✅ (C08 optional)
- All AUD-1–AUD-31 pass
- Coverage matrices below have no gaps

**Evidence**: C07 audit walkthrough; `cargo test` / `cargo clippy` / `cargo fmt --check` logs.

---

## Requirements ↔ Audit Coverage Matrix

| Requirement | Summary | Audit IDs |
|-------------|---------|-----------|
| REQ-1 | Cross intersection | AUD-1, AUD-2 |
| REQ-2 | Lane routes r/s/l | AUD-28 |
| REQ-3 | Smart algorithm, no lights | AUD-8–AUD-18, AUD-B4 |
| REQ-4 | Smart-system detection | AUD-25, AUD-26 |
| REQ-5 | Physics v=d/t | AUD-26 |
| REQ-6 | Route adherence | AUD-28 |
| REQ-7 | ≥3 velocities | AUD-15, AUD-31 |
| REQ-8 | Safe distance | AUD-8, AUD-29, AUD-30 |
| REQ-9 | Collision avoidance | AUD-8–AUD-16, AUD-30 |
| REQ-10 | SDL2 assets | AUD-2 |
| REQ-11 | Turn animation | (visual; AUD-28 path check) |
| REQ-12 | Arrow Up / South | AUD-3 |
| REQ-13 | Arrow Down / North | AUD-4 |
| REQ-14 | Arrow Right / West | AUD-5 |
| REQ-15 | Arrow Left / East | AUD-6 |
| REQ-16 | R random spawn | AUD-7, AUD-16 |
| REQ-17 | Esc exit | AUD-18, AUD-19 |
| REQ-18 | Anti-spam spawn | AUD-27 |
| REQ-19 | Stats window | AUD-19 |
| REQ-20 | Max vehicles passed | AUD-20 |
| REQ-21 | Max velocity | AUD-21 |
| REQ-22 | Min velocity | AUD-21 |
| REQ-23 | Crossing time definition | AUD-26 |
| REQ-24 | Max crossing time | AUD-22, AUD-25 |
| REQ-25 | Min crossing time | AUD-23, AUD-25 |
| REQ-26 | Close calls | AUD-24 |
| REQ-B1 | Custom assets | AUD-B2 |
| REQ-B2 | Extra stats | AUD-B1 |
| REQ-B3 | Acceleration | AUD-B3 |
| NFR-1 | Rust | A01 |
| NFR-2 | SDL2 | A01 |
| NFR-3 | Animation required | A07 |
| NFR-4 | Native binary | A01 |
| NFR-5 | Documented setup | A02, C07 |

| Audit ID | Covers REQ / NFR | Tickets |
|----------|------------------|---------|
| AUD-1 | REQ-1 | A03 |
| AUD-2 | REQ-1, REQ-10 | A03 |
| AUD-3 | REQ-12 | A04 |
| AUD-4 | REQ-13 | A04 |
| AUD-5 | REQ-14 | A04 |
| AUD-6 | REQ-15 | A04 |
| AUD-7 | REQ-16 | A06 |
| AUD-8 | REQ-3, REQ-8, REQ-9 | C02 |
| AUD-9 | REQ-3, REQ-9 | C02 |
| AUD-10 | REQ-3, REQ-9 | C02 |
| AUD-11 | REQ-3, REQ-9 | C02 |
| AUD-12 | REQ-3, REQ-9 | C02 |
| AUD-13 | REQ-3, REQ-9 | C02 |
| AUD-14 | REQ-3, REQ-9 | C02 |
| AUD-15 | REQ-3, REQ-7, REQ-9 | C03 |
| AUD-16 | REQ-3, REQ-16 | C04 |
| AUD-17 | REQ-3 | C04 |
| AUD-18 | REQ-3, REQ-17 | C06 |
| AUD-19 | REQ-17, REQ-19 | C06 |
| AUD-20 | REQ-20 | C06 |
| AUD-21 | REQ-21, REQ-22 | C06 |
| AUD-22 | REQ-24 | C06 |
| AUD-23 | REQ-25 | C06 |
| AUD-24 | REQ-26 | C06 |
| AUD-25 | REQ-24, REQ-25 | C06 |
| AUD-26 | REQ-23, REQ-5 | B01, C06 |
| AUD-27 | REQ-18 | A05 |
| AUD-28 | REQ-2, REQ-6 | B02 |
| AUD-29 | REQ-8 | B04 |
| AUD-30 | REQ-8, REQ-9 | B04, C03 |
| AUD-31 | REQ-7 | B03 |
| AUD-B1 | REQ-B2 | C08 |
| AUD-B2 | REQ-B1 | A08 |
| AUD-B3 | REQ-B3 | B05 |
| AUD-B4 | REQ-3 | C07 (notes) |

---

## Requirements Coverage Matrix (by ticket)

| Requirement ID | Tickets | Gate |
|----------------|---------|------|
| REQ-1 | A03 | G1 |
| REQ-2 | A03, B02 | G1 |
| REQ-3 | C01, C02, C03, C04 | G2 |
| REQ-4 | C01 | G2 |
| REQ-5 | B01 | G2 |
| REQ-6 | B02 | G2 |
| REQ-7 | B03, C03 | G2 |
| REQ-8 | B04 | G2 |
| REQ-9 | C02, C03 | G2 |
| REQ-10 | A03 | G1 |
| REQ-11 | A07 | G2 |
| REQ-12–REQ-15 | A04 | G1 |
| REQ-16 | A06, C04 | G1/G2 |
| REQ-17 | C06 | G2 |
| REQ-18 | A05 | G1 |
| REQ-19–REQ-26 | C05, C06 | G2 |
| NFR-1, NFR-2, NFR-4 | A01 | G1 |
| NFR-3 | A07 | G2 |
| NFR-5 | A02, C07 | G2 |

---

## Immediate Next Work Queue

### Dev A

1. **A07** — turn animation (B02 ✅ unblocks)
2. **A03** ✅ · **A02** ✅ · **A01** ✅ · **A04** ✅ · **A05** ✅ · **A06** ✅ — scaffolding through random spawn complete

### Dev B

1. **B05** *(bonus)* — acceleration / deceleration
2. **B01**–**B04** ✅ — physics through safe distance complete

### Dev C

1. **C06** — stats window on Esc (C05 ✅)
2. **C02** — smart scheduler (B03 ✅, B04 ✅, C01 ✅)

---

## Known Defects

> These are pre-existing bugs discovered during implementation, not yet fixed. Each must be resolved before **Gate G2**.

| ID | Status | Description | Size | Introduced | Fixed by | Audit risk |
|----|--------|-------------|------|------------|----------|------------|
| DEF-01 | ✅ | **Double-movement per frame**: `SpawnSystem::update` called both `integrate_physics` and `advance_along_path` each tick, doubling travel speed. Fixed in **B04** — live sim uses `advance_along_path` only; crossing metrics accumulate inside path movement. | S | B02 | B04 | AUD-28, AUD-29, AUD-30 |

---

## Cross-References

| Document | Relationship |
|----------|--------------|
| `docs/requirements.md` | Source of REQ IDs |
| `docs/audit.md` | Source of AUD IDs |
| `docs/SDS.md` §13 | Cross-track module contracts |
| `docs/pr-messages/` | Per-ticket handover artifacts (`A01-…-pr.md`, etc.) |
| `.agents/workflows/implement-ticket.md` | Playbook for executing tickets |
