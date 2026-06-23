---
title: "Review: C03-C04 yield-on-conflict proof + congestion cap"
reviewer: Andy
date: 2026-06-23
branch: andy/C03-C04
commit: 9c0355c
---

# PR Review: C03 + C04 — Yield-on-conflict proof and congestion cap

**Verdict**: REQUEST-CHANGES
**Branch**: `andy/C03-C04`
**Pinned commit**: `9c0355c C03-C04 Tracks`
**Tickets**: C03 (AUD-15), C04 (AUD-16, AUD-17)
**PR message**: `docs/pr-messages/C03-C04-pr.md` — found

---

## STEP 0 — Branch state

```
git fetch origin                        (no output — up to date)
git checkout andy/C03-C04              (Already on 'andy/C03-C04')
git log origin/main..HEAD --oneline
  9c0355c C03-C04 Tracks

git diff --stat origin/main...HEAD
  docs/pr-messages/C03-C04-pr.md  |  71 ++++++++++++++++++++++
  docs/ticket-tracker.md          |   4 +-
  src/spawn.rs                    |  14 +++-
  src/vehicle.rs                  |   1 +
  tests/smoke.rs                  | 149 +++++++++++++++++++++++++++++++++++++++++
  5 files changed, 236 insertions(+), 3 deletions(-)
```

All findings below trace to lines in this diff only.

---

## STEP 1 — Context loaded

| Document | Status | Notes |
|----------|--------|-------|
| `docs/pr-messages/C03-C04-pr.md` | ✅ Found | Covers AUD-15, AUD-16, AUD-17, REQ-3, REQ-7, REQ-9 |
| `docs/ticket-tracker.md` C03 row | ✅ ⬜→✅ Done | Changed from ⬜ Not Started to ✅ Done |
| `docs/ticket-tracker.md` C04 row | ✅ ⬜→✅ Done | Changed from ⬜ Not Started to ✅ Done |
| `docs/audit.md` AUD-15 | ✅ Read | "Pass if at least one trial shows a vehicle slowing down to avoid collision" |
| `docs/audit.md` AUD-16 | ✅ Read | "R for ≥1 minute — no collisions" |
| `docs/audit.md` AUD-17 | ✅ Read | "fewer than 8 vehicles stuck in the same lane simultaneously" |
| `docs/SDS.md` §13.1 | ✅ Read | `spawn.rs` owned by **A**; `vehicle.rs` owned by **B** |

**AUD-15 exact requirement** (`docs/audit.md` line 138–144):
> Spawn vehicles on collision-course lanes; pass if at least one trial shows a vehicle
> **slowing down** to avoid collision (visible speed reduction, not teleport or overlap).
> Covers REQ-3, REQ-7, REQ-9.

**AUD-16 exact requirement** (`docs/audit.md` line 148–152):
> Press R and let random spawns run for **at least 60 seconds**. Pass if no collisions
> occur during the period. Covers REQ-3, REQ-16.

**AUD-17 exact requirement** (`docs/audit.md` line 155–159):
> During the same ≥1 minute R run (AUD-16), watch lane queues. Pass if congestion stays
> low — e.g., **fewer than 8 vehicles stuck in the same lane simultaneously**. Covers REQ-3.

**SDS §13.1 `spawn.rs` authorised C-track edits**: C01 (`vehicles_mut`), C02 (queue offset + proximity clamp), C05 (`VehicleExit`). C04 is **not** listed.
**SDS §13.1 `vehicle.rs` authorised C-track edits**: C02 (`clamp_velocity_for_proximity`). C03 is **not** listed.

---

## STEP 2 — Automated gates

| Command | Result | Output |
|---------|--------|--------|
| `cargo test` | ✅ PASS | 70 unit + 19 smoke = 89 tests, 0 failures |
| `cargo clippy -- -D warnings` | ✅ PASS | No warnings |
| `cargo fmt --check` | ✅ PASS | No formatting issues |
| `cargo build` | ✅ PASS | Clean build |

All four gates pass. No blocking failures from automated checks.

---

## STEP 3 — REQ/AUD traceability

### AUD-15 (`tests/smoke.rs:513`)

| Claim | Verified? | Evidence |
|-------|-----------|----------|
| Test calls ONLY `SmartController::update()` (no advance, no clamp) | ✅ Yes | Phase 3 at `smoke.rs:590–597` calls only `smart.update()`; `advance_along_path` and `clamp_velocity_for_proximity` are never called |
| Positions are unchanged | ✅ Yes | `pos_before` recorded at `smoke.rs:582`; per-vehicle x/y assertions at `smoke.rs:598–609` |
| `managed_scheduler_yielded` returns true | ✅ Yes | Asserted at `smoke.rs:612–616` |
| `clamp_velocity_for_proximity` never called in this test | ✅ Yes | Diff confirms: no call to `spawn.update()` or `clamp_velocity_for_proximity` in the AUD-15 test |

Causality chain confirmed via `smart.rs:101–148`: `schedule_managed_velocities` resets all Managed vehicles to `nominal_velocity`, then applies gap-based yield. Gap = `SAFE_DISTANCE * 0.5` = 20.0 ≤ SAFE_DISTANCE (40.0) → target = 0.0 (complete stop). East vehicle `commanded_velocity` → 0.0. `managed_scheduler_yielded` at `smart.rs:73–79` checks `commanded_velocity < 1.0` → true. Proof is sound.

### AUD-16 (`tests/smoke.rs:602`)

| Claim | Verified? | Evidence |
|-------|-----------|----------|
| 3600-frame loop asserts gap ≥ `VEHICLE_LENGTH * 0.9` for ALL pairs every frame | ✅ Structurally yes | `smoke.rs:628–641`: nested i/j loop covers all pairs |
| Vehicles are actually spawning continuously | ❌ **No** — see Finding #3 | `spawn_random` uses wall-clock cooldown (400ms); test loop runs in ~100–500ms wall time; cooldown for all 4 seeded approaches never expires; almost no additional spawns occur |

### AUD-17 (`tests/smoke.rs:602`)

| Claim | Verified? | Evidence |
|-------|-----------|----------|
| Lane count ≤ 8 asserted every frame | ✅ Structurally yes | `smoke.rs:643–652` counts per `lane_id` each frame |
| `try_spawn` enforces cap with pre-spawn count check | ✅ Code yes | `spawn.rs:149–156`: `if queued >= LANE_CAPACITY { return None; }` |
| Test exercises the cap boundary | ❌ **No** — see Finding #4 | Only 1 vehicle per lane ever present; cap guard never fires in test |

### REQ-3 — SmartController responsible for yield

✅ Verified. `smart.rs:144–146` shows `vehicles[yielder].commanded_velocity` written by `schedule_managed_velocities`. No clamp involvement in AUD-15 test.

### REQ-7 — ≥3 velocity levels exercised

⚠️ Partially verified. AUD-15 test uses `VelocityLevel::Fast.speed()` as nominal and observes 0.0 commanded (gap ≤ SAFE_DISTANCE branch). That is 2 effective speed points, not 3. `VelocityLevel::Yield.speed()` is the intermediate level defined in `smart.rs:102` but is only used when gap is between SAFE_DISTANCE and SAFE_DISTANCE*1.5 — a condition not exercised in the AUD-15 test. REQ-7 is properly gated by AUD-31/B03 (`b03_spawned_vehicles_have_three_distinct_commanded_velocities`). See Finding #6.

### REQ-9 — Scheduler causality proven

✅ Verified for the 4-vehicle scenario. AUD-15 proves scheduler causality directly. AUD-16 collision check is structurally present but weakened by Finding #3.

---

## STEP 4 — Adversarial test quality

### AUD-15 test (`tests/smoke.rs:513`)

**Q1: Does the test pass because of the scheduler (not a safety net)?**
✅ Yes. The test deliberately bypasses `SpawnSystem::update()` (which calls `clamp_velocity_for_proximity`). Only `SmartController::update()` is called in Phase 3. Positions never advance → proximity clamp cannot fire even if called (which it isn't). The yield must come from the scheduler.

**Q2: Can you confirm causality?**
✅ Yes. `smart.rs:106–108` resets `commanded_velocity` to `nominal_velocity` at the start of each `schedule_managed_velocities` call. The reduction to 0.0 happens only inside the conflict/gap loop (`smart.rs:139–146`). Positions are verified unchanged before and after. Causality is airtight.

**Q3: Are positions advanced before the assertion?**
✅ No. Phase 3 calls only `smart.update()`. Positions are snapshotted before and asserted unchanged after. `smart.update()` writes `commanded_velocity` and `velocity` only.

### AUD-16 test (`tests/smoke.rs:602`)

**Q: Are vehicles actually spawning continuously?**
❌ **No.** `SpawnRng::new()` at `spawn.rs:19–21` uses `Instant::now().elapsed().as_nanos() as u32`. In test context this is near 0 → seed = 1 (deterministic). The cooldown (`SpawnCooldown`) uses real wall-clock `Instant::now()` with a 400ms window (`config::SPAWN_COOLDOWN_MS`). The initial 4 `try_spawn` calls at `smoke.rs:604–609` each record a cooldown on their respective approach. The 3600-frame simulation loop runs in approximately 100–400ms of wall-clock time. Since 400ms < cooldown window, cooldowns never expire during the loop. `spawn_random` returns `None` on virtually every frame after the initial seeding.

Consequence: after the 4 initial vehicles traverse and exit the canvas (~720 frames at 120 units/s over ~1440 units of path), `spawn.vehicles()` is empty. The gap check (`smoke.rs:628–641`) iterates `0..0` × `1..0` = zero iterations. The lane-count loop (`smoke.rs:643–652`) iterates over an empty `HashMap`. Both assertions are vacuously true for the remaining ~2880 frames.

**This is a HIGH finding.** The test does not demonstrate 60 seconds of sustained random traffic. It demonstrates 4 vehicles on non-conflicting lanes traversing the intersection, followed by ~2880 frames of empty assertions.

### AUD-17 test (`tests/smoke.rs:602`)

**Q: Does the cap check use the correct threshold?**
The test uses hardcoded `8` (`smoke.rs:650`); `LANE_CAPACITY` const is `8` (`spawn.rs:10`). Values match, but the const is non-pub and inaccessible from `tests/smoke.rs`. See Finding #5.

**Q: Could the test pass even if `try_spawn` never enforces the cap?**
✅ Yes. Only 1 vehicle per lane is ever present (4 initial seeds on 4 different lanes; random spawning never occurs due to cooldown). The `count <= 8` assertion passes trivially. Remove the cap from `try_spawn` and the test still passes. **MEDIUM finding** — see Finding #4.

---

## STEP 5 — Cross-track ownership

Per `docs/SDS.md` §13.1:

| File | Owner | Diff change | Authorized? |
|------|-------|-------------|-------------|
| `src/spawn.rs` | **Track A** (sbusho) | `LANE_CAPACITY` const + cap guard in `try_spawn` | ❌ C04 not listed in §13.1 |
| `src/vehicle.rs` | **Track B** (iannakopylova) | `vehicle.nominal_velocity = speed;` in test helper | ❌ C03 not listed in §13.1 |
| `tests/smoke.rs` | **Track C** | New AUD-15, AUD-16/17 tests | ✅ C owns `smart.rs`, `stats*.rs` |
| `docs/pr-messages/C03-C04-pr.md` | **Track C** | New file | ✅ |
| `docs/ticket-tracker.md` | Shared | C03/C04 status update | ✅ |
| `docs/SDS.md` | Not in diff | — | ⚠️ Table NOT updated for either cross-track edit |

SDS §13.1 Rule: *"Do not edit another track's owned file without updating this table and announcing in PR."*

The PR announces both edits in the PR message. The SDS §13.1 table is **not updated** to record either C03→vehicle.rs or C04→spawn.rs. Both conditions of the rule must be met; only one is.

---

## STEP 6 — Findings

| # | Severity | File:Line | Finding | Action |
|---|----------|-----------|---------|--------|
| 1 | **HIGH** | `src/spawn.rs:10,149–156` | **Cross-track violation — Track A file.** C04 adds `LANE_CAPACITY` const and cap guard to `spawn.rs`, which is owned by Track A (sbusho). SDS §13.1 lists C01, C02, C05 as authorised C-track editors of `spawn.rs`; C04 is not listed. The PR announces the rationale but the §13.1 table was not updated. SDS §13.1 rule requires both. | BOUNCE |
| 2 | **HIGH** | `src/vehicle.rs:543` | **Cross-track violation — Track B file.** C03 adds `vehicle.nominal_velocity = speed;` to the `south_straight_lane_vehicle` helper in `vehicle.rs`, owned by Track B (iannakopylova). SDS §13.1 lists C02 as the only authorised C-track editor of `vehicle.rs`; C03 is not listed. Table not updated. | BOUNCE |
| 3 | **HIGH** | `tests/smoke.rs:602–655` | **AUD-16 sustained-traffic claim is not satisfied.** `spawn_random` depends on wall-clock `Instant::now()` for the 400ms cooldown. The 3600-frame test loop runs in ~100–500ms of wall time; cooldowns for all 4 seeded approaches never expire. Virtually no additional vehicles spawn after the initial 4. After those 4 vehicles exit (~720 frames), `spawn.vehicles()` is empty and all assertions in the remaining ~2880 frames iterate over empty collections — vacuously true. The PR claim "3 600-frame simulation asserts gap ≥ `VEHICLE_LENGTH * 0.9` for all vehicle pairs" is not realised beyond the initial 4-vehicle run. | BOUNCE |
| 4 | **MEDIUM** | `tests/smoke.rs:643–652` | **AUD-17 cap enforcement is not exercised.** The test never accumulates more than 1 vehicle per lane (4 seeds on 4 different lanes; random spawns don't occur). The `count <= 8` assertion passes trivially. Remove the `LANE_CAPACITY` guard from `try_spawn` entirely and the test still passes. A dedicated unit test in `spawn.rs` that fills a lane to 8 vehicles and asserts the 9th spawn returns `None` is required to prove the cap fires. | BOUNCE |
| 5 | **MEDIUM** | `tests/smoke.rs:650` | **AUD-17 assertion uses hardcoded literal `8` instead of `LANE_CAPACITY`.** `LANE_CAPACITY` is non-pub in `spawn.rs` and inaccessible from the smoke test. If the constant changes, the assertion diverges silently. Fix: either mark `LANE_CAPACITY` as `pub` (or re-export it) so tests can reference it, or add a comment `// must match spawn::LANE_CAPACITY` to the literal. | SELF-FIX |
| 6 | **NIT** | `docs/pr-messages/C03-C04-pr.md:57` | **REQ-7 claim in PR message is misleading.** The AUD-15 test uses `VelocityLevel::Fast.speed()` as nominal and observes a stop (0.0) — 2 effective speed levels. The PR says this "exercises the ≥3 velocity-level requirement." The intermediate `VelocityLevel::Yield.speed()` path in `schedule_managed_velocities` (`smart.rs:141`) is not reachable in this test (gap ≤ SAFE_DISTANCE takes the stop branch). REQ-7 is properly gated by AUD-31/B03. No code change needed; adjust prose. | NOTE |

---

## STEP 7 — Verdict

### REQUEST-CHANGES

Three HIGH findings and one MEDIUM BOUNCE prevent merge:

1. **Findings #1 and #2** are cross-track violations requiring the SDS §13.1 table to be updated (and, for vehicle.rs, Track B owner awareness). The PR correctly justifies both edits, making these fixable: update the SDS table to add C03→vehicle.rs and C04→spawn.rs, obtain acknowledgement from affected track owners if required per team process.

2. **Finding #3** is the most consequential. The AUD-16 smoke test does not prove sustained random traffic has no collisions — it proves 4 vehicles on non-conflicting initial lanes complete without collision, which is a weaker claim. The test must be redesigned to either: (a) bypass the wall-clock cooldown in test mode (e.g., expire cooldowns between frames using the existing `SpawnCooldown::record_at` test helper), or (b) seed sufficient concurrent traffic explicitly rather than relying on `spawn_random`.

3. **Finding #4** means AUD-17's congestion cap enforcement is unproven by any test. A unit test in `spawn.rs::tests` should spawn 8 vehicles on the same lane and assert that a 9th `try_spawn` returns `None`.

Findings #5 (SELF-FIX) and #6 (NOTE) may be addressed in the same pass.

---

## Checklist

- [x] Ticket scope respected — changes cover only C03 and C04 scope
- [ ] Cross-track ownership respected — `spawn.rs` (Track A) and `vehicle.rs` (Track B) edited without SDS §13.1 table update (**Findings #1, #2**)
- [x] PR message artifact present and filled in — `docs/pr-messages/C03-C04-pr.md`
- [x] Tracker updated — C03, C04 marked ✅ Done
- [ ] Tests adequate — AUD-16 sustained traffic not exercised; AUD-17 cap not exercised (**Findings #3, #4**)
- [ ] Audit items fully verified — AUD-16 claim not satisfied by the test (**Finding #3**)
