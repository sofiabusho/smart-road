# Audit Gate — smart-road

> Acceptance checklist for 01-edu Smart Road.  
> Derived from `docs/raw/audit-source.md`. Each `######` question maps to one AUD ID.

**How to use**: Build and run the native binary (`cargo run`). Walk sections in order. Mark ✅ / ❌ / ⬜.

---

## Status Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Pass |
| ❌ | Fail |
| ⬜ | Not yet verified |
| N/A | Not applicable |

---

## 1. Core Functionality

### Intersection visibility

#### AUD-1: Cross intersection visible

- **Covers**: REQ-1
- **Verify**:
  1. Run `cargo run` from the project root.
  2. Observe the main simulation window.
  3. **Pass** if a cross (four-way) intersection layout is clearly visible.

#### AUD-2: Intersection represented by assets

- **Covers**: REQ-1, REQ-10
- **Verify**:
  1. With the simulation running, inspect the intersection area.
  2. **Pass** if roads/intersection are rendered using images or tile assets (not bare solid-color placeholder with no road artwork).

### Spawn directions (arrow keys)

#### AUD-3: Arrow Up spawns from South

- **Covers**: REQ-12
- **Verify**:
  1. Start the simulation.
  2. Press `Arrow Up` once.
  3. **Pass** if a new vehicle appears on the **south approach** and travels northward.

#### AUD-4: Arrow Down spawns from North

- **Covers**: REQ-13
- **Verify**:
  1. Press `Arrow Down` once.
  2. **Pass** if a vehicle appears on the **north approach** and travels southward.

#### AUD-5: Arrow Right spawns from West

- **Covers**: REQ-14
- **Verify**:
  1. Press `Arrow Right` once.
  2. **Pass** if a vehicle appears on the **west approach** and travels eastward.

#### AUD-6: Arrow Left spawns from East

- **Covers**: REQ-15
- **Verify**:
  1. Press `Arrow Left` once.
  2. **Pass** if a vehicle appears on the **east approach** and travels westward.

#### AUD-7: `R` spawns random lane and route

- **Covers**: REQ-16
- **Verify**:
  1. Press and hold (or toggle) `R` for several seconds.
  2. **Pass** if vehicles appear from **different approaches** with **different routes** (`r` / `s` / `l`), not a single fixed spawn pattern.

### Collision-free crossing scenarios

#### AUD-8: Three vehicles same lane — no collision

- **Covers**: REQ-3, REQ-8, REQ-9
- **Verify**:
  1. Pick one approach (e.g., South via `Arrow Up`).
  2. Spawn **three vehicles in the same lane** (press the same arrow key three times with brief spacing).
  3. Repeat for **all four approaches**.
  4. **Pass** if every vehicle in each trial completes its route **without collision or overlap**.

#### AUD-9: Four vehicles — West + East entries

- **Covers**: REQ-3, REQ-9
- **Verify**:
  1. Simultaneously spawn **one** vehicle with `Arrow Right` and **three** with `Arrow Left`.
  2. Repeat until routes that can conflict are exercised.
  3. **Pass** if all vehicles exit without collision.

#### AUD-10: Four vehicles — South + East entries

- **Covers**: REQ-3, REQ-9
- **Verify**:
  1. Simultaneously spawn **one** with `Arrow Up` and **three** with `Arrow Left`.
  2. Repeat until conflicting paths occur.
  3. **Pass** if all vehicles exit without collision.

#### AUD-11: Four vehicles — South + West entries

- **Covers**: REQ-3, REQ-9
- **Verify**:
  1. Simultaneously spawn **one** with `Arrow Up` and **three** with `Arrow Right`.
  2. Repeat until conflicting paths occur.
  3. **Pass** if all vehicles exit without collision.

#### AUD-12: Four vehicles — North + East entries

- **Covers**: REQ-3, REQ-9
- **Verify**:
  1. Simultaneously spawn **one** with `Arrow Down` and **three** with `Arrow Left`.
  2. Repeat until conflicting paths occur.
  3. **Pass** if all vehicles exit without collision.

#### AUD-13: Four vehicles — North + West entries

- **Covers**: REQ-3, REQ-9
- **Verify**:
  1. Simultaneously spawn **one** with `Arrow Down` and **three** with `Arrow Right`.
  2. Repeat until conflicting paths occur.
  3. **Pass** if all vehicles exit without collision.

#### AUD-14: Seven vehicles — five South + two West

- **Covers**: REQ-3, REQ-9
- **Verify**:
  1. Spawn **five** vehicles with `Arrow Up` and **two** with `Arrow Right` in quick succession.
  2. **Pass** if all seven complete without collision.

#### AUD-15: Collision avoidance via velocity reduction

- **Covers**: REQ-3, REQ-7, REQ-9
- **Verify**:
  1. At least **three times**, spawn vehicles on **collision-course lanes** (paths that would intersect without coordination).
  2. Observe vehicles as they approach the conflict point.
  3. **Pass** if at least one trial shows a vehicle **slowing down** to avoid collision (visible speed reduction, not teleport or overlap).

### Sustained random traffic

#### AUD-16: `R` for ≥1 minute — no collisions

- **Covers**: REQ-3, REQ-16
- **Verify**:
  1. Press `R` and let random spawns run for **at least 60 seconds**.
  2. **Pass** if no collisions occur during the period.

#### AUD-17: `R` for ≥1 minute — low congestion

- **Covers**: REQ-3
- **Verify**:
  1. During the same ≥1 minute `R` run (AUD-16), watch lane queues.
  2. **Pass** if congestion stays low — e.g., **fewer than 8 vehicles stuck** in the same lane simultaneously (per auditor guidance in source).

### Statistics window and accuracy

#### AUD-18: Four vehicles then exit — no collision

- **Covers**: REQ-3, REQ-17
- **Verify**:
  1. Spawn **two** vehicles with `Arrow Up` and **two** with `Arrow Right`.
  2. Wait until all four have fully exited the intersection.
  3. Press `Esc`.
  4. **Pass** if all four crossed without collision before exit.

#### AUD-19: Statistics window appears on `Esc`

- **Covers**: REQ-17, REQ-19
- **Verify**:
  1. After AUD-18 (or any session), press `Esc`.
  2. **Pass** if a **separate statistics window** (or clearly distinct stats overlay) appears.

#### AUD-20: Max vehicles passed = 4

- **Covers**: REQ-20
- **Verify**:
  1. After the AUD-18 session (exactly four vehicles crossed), read the stats window.
  2. **Pass** if **"Max number of vehicles that passed the intersection"** (or equivalent label) shows **4**.

#### AUD-21: Max and min velocity displayed

- **Covers**: REQ-21, REQ-22
- **Verify**:
  1. In the stats window, locate max and min velocity fields.
  2. **Pass** if both **Max velocity** and **Min velocity** are shown with numeric values.

#### AUD-22: Max crossing time displayed

- **Covers**: REQ-24
- **Verify**:
  1. In the stats window, locate the maximum crossing-time field.
  2. **Pass** if **Max time to pass the intersection** (or equivalent) is shown.

#### AUD-23: Min crossing time displayed

- **Covers**: REQ-25
- **Verify**:
  1. In the stats window, locate the minimum crossing-time field.
  2. **Pass** if **Min time to pass the intersection** (or equivalent) is shown.

#### AUD-24: Close calls displayed when applicable

- **Covers**: REQ-26
- **Verify**:
  1. If any close calls occurred (safe-distance violation while passing), check the stats window.
  2. **Pass** if a **Close calls** count/section is present (0 is valid when none occurred).

#### AUD-25: Single vehicle — max time equals min time

- **Covers**: REQ-24, REQ-25
- **Verify**:
  1. Spawn **one** vehicle; wait until it exits.
  2. Press `Esc` and read max/min crossing times.
  3. **Pass** if **max time = min time** (only one sample).

#### AUD-26: Crossing time matches observed duration

- **Covers**: REQ-23, REQ-5
- **Verify**:
  1. For the single-vehicle trial (AUD-25), manually time from intersection detection to canvas removal (stopwatch or frame count).
  2. Compare to the reported crossing time in stats.
  3. **Pass** if reported time is **consistent** with manual observation (reasonable tolerance for frame rounding).

---

## 2. General

#### AUD-27: Spawn spam prevention

- **Covers**: REQ-18
- **Verify**:
  1. Rapidly press the same arrow key many times in quick succession.
  2. **Pass** if vehicles are **not** created overlapping at the same spawn point (staggered/rate-limited spawns).

#### AUD-28: Route adherence per lane

- **Covers**: REQ-2, REQ-6
- **Verify**:
  1. Spawn vehicles on each approach; observe paths through the intersection.
  2. **Pass** if each vehicle follows its lane's fixed route (`r`, `s`, or `l`) with **no lane changes**.

#### AUD-29: Safe distance is configured

- **Covers**: REQ-8
- **Verify**:
  1. Inspect code or in-game behavior documentation for the safety-distance constant/logic.
  2. **Pass** if distance is **strictly positive** and plausibly scaled to vehicle size.

#### AUD-30: Safe distance maintained under slowdown

- **Covers**: REQ-8, REQ-9
- **Verify**:
  1. Create a scenario where a leading vehicle slows or stops in-lane; spawn a follower behind it.
  2. **Pass** if the follower maintains separation and **does not collide**.

#### AUD-31: At least three distinct velocities

- **Covers**: REQ-7
- **Verify**:
  1. Run mixed traffic (`R` or manual spawns) and observe speed changes.
  2. **Pass** if vehicles visibly operate at **≥3 different speed levels** (e.g., cruise, slow, yield).

---

## 3. Bonus (non-blocking)

#### AUD-B1: Additional statistics beyond required set

- **Covers**: REQ-B2
- **Verify**:
  1. Press `Esc` after a session.
  2. **Pass** if stats include **extra metrics** not listed in REQ-20–REQ-26.

#### AUD-B2: Custom-created assets

- **Covers**: REQ-B1
- **Verify**:
  1. Compare assets to stock packs cited in the brief.
  2. **Pass** if student-authored vehicle and/or road artwork is used.

#### AUD-B3: Acceleration and deceleration physics

- **Covers**: REQ-B3
- **Verify**:
  1. Observe a vehicle commanded to slow down.
  2. **Pass** if speed changes **gradually** over time (not instant jumps); different vehicles may decelerate at different rates.

#### AUD-B4: Real-life algorithm viability (auditor opinion)

- **Covers**: REQ-3
- **Verify**:
  1. Review the smart-intersection strategy during sustained `R` traffic.
  2. **Pass** (subjective) if the auditor judges the approach **plausibly implementable** in real AV intersection control (document rationale in audit notes).

---

## Cross-References

| Document | Relationship |
|----------|--------------|
| `docs/requirements.md` | Source REQ IDs audited here |
| `docs/PRD.md` | Detailed spec behind each check |
| `docs/ticket-tracker.md` | Maps tickets → AUD IDs |
| `docs/raw/audit-source.md` | Authoritative auditor question source |
| `docs/pr-messages/` | Evidence of audit verification per ticket |
