# Audit Plan — smart_road

## Overview

The codebase implements a Rust/SDL2 smart-intersection simulation with:
- A four-way cross intersection rendered with BMP tile assets.
- Twelve inbound lanes (four approaches × three routes: Right/Straight/Left).
- Polyline path-following for vehicles, with per-vehicle velocity levels (Fast/Cruise/Yield).
- A FIFO-entry smart controller (`smart.rs`) that yields later-arriving vehicles on conflicting paths.
- B05 gradual acceleration/deceleration physics (`vehicle.rs::step_velocity_toward_command`).
- Spawn anti-spam cooldown (400 ms per approach direction).
- Post-Esc statistics window with seven required fields plus four bonus fields.

**Ticket status (from `docs/ticket-tracker.md`, refreshed 2026-06-25):**
All tickets **A01–A08**, **B01–B05**, **C01–C08** ✅ — **Gate G2** complete.
**C07** (audit dry-run) merged — README runbook in [README.md](../README.md#audit-dry-run-gate-g2), handover in [docs/pr-messages/C07-audit-dry-run-pr.md](pr-messages/C07-audit-dry-run-pr.md).

**Automated coverage (2026-06-25):** `cargo test --lib` (90) + `cargo test --test smoke` (23). See README audit table for AUD ↔ smoke mapping. Manual spot-checks still recommended for AUD-1/2 visuals, AUD-26 stopwatch, AUD-28 turn animation, AUD-31 visible speeds, AUD-B3/B4.

**Referenced documents that exist:** `docs/requirements.md` ✅, `docs/ticket-tracker.md` ✅,
`docs/PRD.md` ✅, `docs/SDS.md` ✅, `docs/audit.md` ✅.

---

## Known limitations (post-C07)

### FIXED in C07 (no longer apply)

| ID | Was | Fixed in |
|----|-----|----------|
| BUG-1 | Close-call detection same-lane only | `vehicle.rs::detect_close_call` — Euclidean distance for any vehicle pair |
| BUG-3 | `SpawnRng` always seeded at 1 | `spawn.rs` — `SystemTime` nanosecond seed |
| BUG-4 | `on_key_down` cleared event queue | `input.rs` — events accumulate per SDL poll |
| BUG-5 | `min_velocity` skipped 0 | `stats.rs` — records `velocity >= 0` |

### Still watch during manual audit

### LIM-1 — Smart controller bypasses B05 physics for managed vehicles (affects AUD-B3)

**File:** `src/smart.rs` (`schedule_managed_velocities`, `on_vehicle_enter_zone`)

```rust
// schedule_managed_velocities — runs every frame
vehicle.commanded_velocity = vehicle.nominal_velocity;
vehicle.velocity = vehicle.nominal_velocity;  // ← instant snap, not a ramp
```

And on zone entry:
```rust
vehicle.velocity = vehicle.nominal_velocity;  // ← also snaps on entry
```

`step_velocity_toward_command` (the B05 ramp) is called by `advance_along_path` AFTER the smart controller has already matched `velocity` to `commanded_velocity`, so the ramp is a no-op for managed vehicles. Vehicles inside the junction teleport to full speed each frame after the reset, then may be immediately yanked to yield speed — all instantaneously.

**Spot it during testing:** AUD-B3 asks you to watch a vehicle slow down gradually. This works for **approaching** vehicles (B04 follow-distance sets `commanded_velocity`; B05 ramp applies in `advance_along_path`). Inside the managed zone, speed changes are **instant** — AUD-15 still passes (visible reduction), AUD-B3 passes only when observing approach-lane deceleration.

---

### LIM-2 — `SAFE_DISTANCE` is only marginally larger than `VEHICLE_LENGTH` (affects AUD-29, AUD-30)

**File:** [src/config.rs:77](src/config.rs#L77) and [src/config.rs:73](src/config.rs#L73)

```
SAFE_DISTANCE = 40.0
VEHICLE_LENGTH = 36.0   // gap is only 4 px
```

The B04 follow logic maintains a 40 px center-to-center gap, but the vehicle sprite is 36 px long. Visually, vehicles may appear nearly bumper-to-bumper. Do not mark collision unless centers actually overlap.

---

## Person 1 — AUD-1 to AUD-10

### Assigned Checks

#### AUD-1: Cross intersection visible — REQ-1
Verify: Run `cargo run`. Observe main window. Pass if a four-way cross intersection is clearly visible.

#### AUD-2: Intersection represented by assets — REQ-1, REQ-10
Verify: Inspect intersection area. Pass if roads/intersection use image/tile assets (BMP files), not bare solid-color rectangles.

#### AUD-3: Arrow Up spawns from South — REQ-12
Verify: Press Arrow Up once. Pass if a vehicle appears on the south approach (bottom of screen) and travels northward (upward).

#### AUD-4: Arrow Down spawns from North — REQ-13
Verify: Press Arrow Down once. Pass if a vehicle appears on the north approach (top of screen) and travels southward.

#### AUD-5: Arrow Right spawns from West — REQ-14
Verify: Press Arrow Right once. Pass if a vehicle appears on the west approach (left of screen) and travels eastward.

#### AUD-6: Arrow Left spawns from East — REQ-15
Verify: Press Arrow Left once. Pass if a vehicle appears on the east approach (right of screen) and travels westward.

#### AUD-7: `R` spawns random lane and route — REQ-16
Verify: Hold R for several seconds. Pass if vehicles appear from different approaches with different routes (not a fixed pattern).

#### AUD-8: Three vehicles same lane — no collision — REQ-3, REQ-8, REQ-9
Verify: Spawn three vehicles in succession on the same approach (same arrow key ×3, brief spacing). Repeat for all four approaches. Pass if all vehicles complete their routes without collision or overlap.

#### AUD-9: Four vehicles — West + East entries — REQ-3, REQ-9
Verify: Spawn one with Arrow Right, three with Arrow Left. Repeat until conflicting paths occur. Pass if all exit without collision.

#### AUD-10: Four vehicles — South + East entries — REQ-3, REQ-9
Verify: Spawn one with Arrow Up, three with Arrow Left. Repeat until conflicting paths occur. Pass if all exit without collision.

---

### Code Areas to Inspect

| Check | Primary files |
|-------|---------------|
| AUD-1, AUD-2 | [src/render.rs](src/render.rs), `assets/roads/` dir |
| AUD-3–AUD-6 | [src/input.rs](src/input.rs) (`approach_for_arrow`), [src/spawn.rs](src/spawn.rs) (`spawn_on_approach`) |
| AUD-7 | [src/input.rs](src/input.rs) (`random_stream_active`), [src/spawn.rs](src/spawn.rs) (`spawn_random`, `SpawnRng`) |
| AUD-8–AUD-10 | [src/smart.rs](src/smart.rs) (`schedule_managed_velocities`), [src/vehicle.rs](src/vehicle.rs) (`enforce_follow_distance`, `clamp_velocity_for_proximity`) |

---

### Potential Issues

**AUD-3–AUD-6 — Key mapping is direction-of-travel, not source**

`input.rs:16-23`: Arrow Right → `Cardinal::West` (vehicle comes FROM west, travels east). Arrow Left → `Cardinal::East`. This matches REQ-14/REQ-15 ("Arrow Right spawns from West") but is counterintuitive. Confirm by watching the vehicle enter from the expected side.

**AUD-8 — Same-lane following may still look tight (LIM-2)**

The 4 px visual gap between VEHICLE_LENGTH (36) and SAFE_DISTANCE (40) means bumper-to-bumper vehicles look nearly touching. Don't mark as collision unless actual position overlap occurs.

**AUD-8 — clamp_velocity_for_proximity called twice per frame**

`spawn.rs:191-209`: `clamp_velocity_for_proximity` runs before movement AND after movement each tick. This is intentional for cascade handling, but if a vehicle is pushed backward by the second call, its path position may become inconsistent with its `path_index`, causing a visible backward glitch. Watch for vehicles that snap backward briefly when three vehicles queue up.

**AUD-9, AUD-10 — Smart controller stability at conflict boundary**

`smart.rs:128`: Yield logic only activates when center-to-center distance `< SAFE_DISTANCE * 1.5` (60 px). The reset step (lines 103-108) snaps velocity to nominal each frame BEFORE applying yield. This means a yielding vehicle that briefly drifts outside the 60 px range will snap to full speed and immediately re-enter the yield zone, potentially oscillating. At Fast speed (168 px/s ≈ 2.8 px/frame), this is unlikely but watch for jitter when two vehicles are exactly at the 60 px boundary.

---

## Person 2 — AUD-11 to AUD-20

### Assigned Checks

#### AUD-11: Four vehicles — South + West entries — REQ-3, REQ-9
Verify: Spawn one with Arrow Up, three with Arrow Right. Repeat until conflicting paths occur. Pass if all exit without collision.

#### AUD-12: Four vehicles — North + East entries — REQ-3, REQ-9
Verify: Spawn one with Arrow Down, three with Arrow Left. Repeat until conflicting paths occur. Pass if all exit without collision.

#### AUD-13: Four vehicles — North + West entries — REQ-3, REQ-9
Verify: Spawn one with Arrow Down, three with Arrow Right. Repeat until conflicting paths occur. Pass if all exit without collision.

#### AUD-14: Seven vehicles — five South + two West — REQ-3, REQ-9
Verify: Spawn five with Arrow Up and two with Arrow Right in quick succession. Pass if all seven complete without collision.

#### AUD-15: Collision avoidance via velocity reduction — REQ-3, REQ-7, REQ-9
Verify: At least three times, spawn vehicles on collision-course lanes. Observe as they approach the conflict point. Pass if at least one trial shows a vehicle visibly slowing to avoid collision (not teleport/overlap).

#### AUD-16: `R` for ≥1 minute — no collisions — REQ-3, REQ-16
Verify: Hold R for at least 60 seconds. Pass if no collisions occur.

#### AUD-17: `R` for ≥1 minute — low congestion — REQ-3
Verify: During the same ≥1 min R run (AUD-16), watch lane queues. Pass if fewer than 8 vehicles stuck in the same lane simultaneously.

#### AUD-18: Four vehicles then exit — no collision — REQ-3, REQ-17
Verify: Spawn two with Arrow Up, two with Arrow Right; wait until all four have fully exited; press Esc. Pass if all four crossed without collision before exit.

#### AUD-19: Statistics window appears on `Esc` — REQ-17, REQ-19
Verify: After AUD-18 (or any session), press Esc. Pass if a separate statistics window (or clearly distinct overlay) appears.

#### AUD-20: Max vehicles passed = 4 — REQ-20
Verify: After the AUD-18 session (exactly four vehicles crossed), read the stats window. Pass if "Max vehicles passed: 4" is shown.

---

### Code Areas to Inspect

| Check | Primary files |
|-------|---------------|
| AUD-11–AUD-14 | [src/smart.rs](src/smart.rs) (`schedule_managed_velocities`, `build_lane_conflicts`) |
| AUD-15 | [src/smart.rs](src/smart.rs) (yield logic), [src/vehicle.rs](src/vehicle.rs) (`step_velocity_toward_command`) |
| AUD-16–AUD-17 | [src/spawn.rs](src/spawn.rs) (`LANE_CAPACITY`, `spawn_random`), [src/smart.rs](src/smart.rs) |
| AUD-18 | [src/app.rs](src/app.rs) (game loop exit logic), [src/spawn.rs](src/spawn.rs) (`update`) |
| AUD-19 | [src/stats_window.rs](src/stats_window.rs) (`show_stats_window`) |
| AUD-20 | [src/stats.rs](src/stats.rs) (`max_vehicles_passed`) |

---

### Potential Issues

**AUD-15 — Velocity reduction may be instant inside the junction (LIM-1)**

`smart.rs:108` directly assigns `vehicle.velocity = vehicle.nominal_velocity` before the yield check, and `smart.rs:146` assigns `vehicles[yielder].velocity = vehicles[yielder].commanded_velocity` for the yield step. Neither uses the B05 ramp. For vehicles already inside the junction, deceleration will look instantaneous. The gradual ramp only works for **approaching** vehicles (B04 follow-distance logic, which sets `commanded_velocity` without touching `velocity`). AUD-15 asks for "visible speed reduction, not teleport or overlap" — a speed snap inside the junction might still be visible as a speed change, just not gradual.

**AUD-16–AUD-17 — Congestion cap via LANE_CAPACITY**

`spawn.rs:10,149-153`: `LANE_CAPACITY = 8`. When 8 vehicles are queued on a single lane, no more spawn on that lane. This is counted per `lane_id`, not per approach — each approach has three lanes, so an approach can hold up to 24 queued vehicles total. AUD-17's threshold is "fewer than 8 vehicles stuck in the same lane." Verify that no single lane accumulates 8+ stopped vehicles.

**AUD-16–AUD-17 — Automated smoke coverage**

`crate_smoke_aud16_aud17_sustained_no_overlap_no_lane_overflow` exercises sustained random traffic with lane cap. Manual 60 s `R` run still recommended for visual confirmation.

**AUD-16 — RNG variety (fixed in C07)**

`spawn.rs`: PRNG seeds from `SystemTime` nanoseconds — sequences differ across restarts. Within a session, xorshift still produces varied lane/route picks.

**AUD-19 — Stats window only appears on Esc, not on window close**

`app.rs:70-82`: `show_stats_on_exit` is only set to `true` when `update()` returns `true` (i.e., Esc was pressed). Closing the window with the X button triggers `Event::Quit`, sets `running = false`, but `show_stats_on_exit` stays false and the stats window is never shown. Make sure to press Esc, not close the window.

**AUD-20 — `max_vehicles_passed` equals total passed count**

`stats.rs:96-97`:
```rust
stats.vehicles_passed += 1;
stats.max_vehicles_passed = stats.max_vehicles_passed.max(stats.vehicles_passed);
```
`max_vehicles_passed` always equals `vehicles_passed` (it tracks cumulative total, not peak concurrent). After exactly four vehicles exit, this will show 4. This satisfies AUD-20 but note the field name is misleading — it's a running total, not a "max simultaneous" value.

---

## Person 3 — AUD-21 to AUD-31 + Bonus

### Assigned Checks

#### AUD-21: Max and min velocity displayed — REQ-21, REQ-22
Verify: In the stats window, locate max and min velocity fields. Pass if both show numeric values.

#### AUD-22: Max crossing time displayed — REQ-24
Verify: In the stats window, locate the maximum crossing-time field. Pass if "Max time to pass intersection" is shown.

#### AUD-23: Min crossing time displayed — REQ-25
Verify: In the stats window, locate the minimum crossing-time field. Pass if "Min time to pass intersection" is shown.

#### AUD-24: Close calls displayed when applicable — REQ-26
Verify: If any close calls occurred, check the stats window. Pass if a "Close calls" count is present (0 is valid when none occurred).

#### AUD-25: Single vehicle — max time equals min time — REQ-24, REQ-25
Verify: Spawn one vehicle; wait until it exits; press Esc; read max/min crossing times. Pass if max time = min time (only one sample).

#### AUD-26: Crossing time matches observed duration — REQ-23, REQ-5
Verify: For the single-vehicle trial (AUD-25), manually time from intersection entry to canvas removal; compare to reported crossing time. Pass if consistent (reasonable frame tolerance).

#### AUD-27: Spawn spam prevention — REQ-18
Verify: Rapidly press the same arrow key many times. Pass if vehicles are NOT created overlapping at the same spawn point.

#### AUD-28: Route adherence per lane — REQ-2, REQ-6
Verify: Spawn vehicles on each approach; observe paths through the intersection. Pass if each vehicle follows its lane's fixed route with no lane changes.

#### AUD-29: Safe distance is configured — REQ-8
Verify: Inspect code or in-game behavior for a positive safety-distance constant scaled to vehicle size. Pass if distance is strictly positive and plausibly scaled.

#### AUD-30: Safe distance maintained under slowdown — REQ-8, REQ-9
Verify: Create a scenario where a leading vehicle slows or stops; spawn a follower behind it. Pass if the follower maintains separation and does not collide.

#### AUD-31: At least three distinct velocities — REQ-7
Verify: Run mixed traffic (R or manual spawns) and observe speed changes. Pass if vehicles visibly operate at ≥3 different speed levels.

#### AUD-B1: Additional statistics beyond required set — REQ-B2
Verify: Press Esc after a session. Pass if stats include extra metrics not listed in REQ-20–REQ-26.

#### AUD-B2: Custom-created assets — REQ-B1
Verify: Compare assets to stock packs. Pass if student-authored vehicle and/or road artwork is used.

#### AUD-B3: Acceleration and deceleration physics — REQ-B3
Verify: Observe a vehicle commanded to slow down. Pass if speed changes gradually over time (not instant jumps); different vehicles may decelerate at different rates.

#### AUD-B4: Real-life algorithm viability (auditor opinion) — REQ-3
Verify: Review the smart-intersection strategy during sustained R traffic. Pass (subjective) if the approach is plausibly implementable in real AV intersection control.

---

### Code Areas to Inspect

| Check | Primary files |
|-------|---------------|
| AUD-21–AUD-24 | [src/stats.rs](src/stats.rs) (`update_velocity_bounds`, `update_crossing_time_bounds`), [src/stats_window.rs](src/stats_window.rs) (`format_stats_lines`) |
| AUD-25, AUD-26 | [src/vehicle.rs](src/vehicle.rs) (`advance_along_path` — time_in_crossing), [src/smart.rs](src/smart.rs) (`on_vehicle_enter_zone`) |
| AUD-27 | [src/spawn.rs](src/spawn.rs) (`SpawnCooldown`, `SPAWN_COOLDOWN_MS = 400`) |
| AUD-28 | [src/intersection.rs](src/intersection.rs) (`build_all_lane_paths`), [src/vehicle.rs](src/vehicle.rs) (`advance_along_path`) |
| AUD-29 | [src/config.rs](src/config.rs) (`SAFE_DISTANCE = 40.0`, `VEHICLE_LENGTH = 36.0`) |
| AUD-30 | [src/vehicle.rs](src/vehicle.rs) (`enforce_follow_distance`, `clamp_velocity_for_proximity`) |
| AUD-31 | [src/vehicle.rs](src/vehicle.rs) (`VelocityLevel`, `spawn_vehicle`) |
| AUD-B1 | [src/stats_window.rs](src/stats_window.rs) (bonus stats lines) |
| AUD-B2 | `assets/vehicles/` dir |
| AUD-B3 | [src/vehicle.rs](src/vehicle.rs) (`step_velocity_toward_command`, `motion_profile`) |

---

### Potential Issues

**AUD-21 — Min velocity includes 0 when stopped (fixed in C07)**

`stats.rs::update_velocity_bounds` records samples where `velocity >= 0`, so a stopped vehicle can display `Min velocity: 0.0`.

**AUD-24 — Close calls count cross-traffic pairs (fixed in C07)**

`vehicle.rs::detect_close_call` uses center-to-center distance for any active vehicle pair within `SAFE_DISTANCE`. After sustained traffic with near-misses, Esc may show a non-zero close-call count.

**AUD-25, AUD-26 — Crossing time measurement window**

`smart.rs:35-46` (`on_vehicle_enter_zone`): When a vehicle enters the junction zone, `time_in_crossing` is reset to 0.0. `vehicle.rs:370` accumulates `time_in_crossing += dt` only when state is `Managed` or `Exiting`. The exit is detected in `spawn.rs:199-203` when the vehicle position is ≥64 px off the window edge. The time from zone entry to last frame before removal is what's reported. When timing manually (AUD-26), start your stopwatch when the vehicle visually enters the intersection box and stop when it disappears off-screen. Expect the reported time to be within 1–2 frame lengths (≈0.02–0.03 s at 60 fps) of your observation.

**AUD-27 — Cooldown is per approach, not per lane**

`spawn.rs:59-77` (`SpawnCooldown::allows`): The 400 ms cooldown applies per `Cardinal` direction. Pressing Arrow Up three times rapidly within 400 ms will spawn only the first vehicle on the South approach; subsequent presses are blocked. However, `spawn_on_approach` rotates through R/S/L routes — the first press spawns on Route::Right, the second (after cooldown) on Route::Straight. Vehicles do NOT overlap at spawn because `spawn_position_on_lane` pushes new vehicles behind the queue (lines 243-273). Both mechanisms work — verify the cooldown, then verify the stagger.

**AUD-28 — Lane rotation per approach (visual)**

`spawn.rs:177-186` (`spawn_on_approach`): Each press of the same arrow key cycles through Route::Right → Route::Straight → Route::Left → Route::Right. Auditors watching route adherence should note that consecutive presses of the same key spawn vehicles on different routes — a vehicle on route::Right turns right; the next on Straight goes straight through. Verify the turn animation aligns with the actual path the vehicle follows.

**AUD-29 — SAFE_DISTANCE is strictly positive but barely exceeds VEHICLE_LENGTH (LIM-2)**

`config.rs:77`: `SAFE_DISTANCE = 40.0`. `config.rs:73`: `VEHICLE_LENGTH = 36.0`. The constant passes the code test (`safe_distance_is_positive_and_vehicle_scaled` in config.rs:119-126). The visual gap between following vehicles is only 4 px, which at 1:1 pixel scale looks like bumper-to-bumper contact. Confirm no actual center-overlap — if vehicles appear to pass through each other, that is a collision regardless of the constant.

**AUD-30 — Proximity clamp after movement could cause backward glitch**

`spawn.rs:191-209`: `clamp_velocity_for_proximity` runs before AND after `advance_along_path`. The post-movement call can push vehicles apart AFTER they have advanced along the path polyline. This push is in Euclidean space and ignores the path direction, potentially moving a vehicle perpendicular to its lane. Watch for vehicles that flash sideways briefly when three or more queue up on a short approach segment.

**AUD-31 — Three velocities visible; confirm they are distinct speeds**

`vehicle.rs:88-93` (`spawn_vehicle`): `id.0 % 3` assigns the velocity level. VehicleId is sequential, so vehicles 1, 4, 7… get Yield (60 px/s); 2, 5, 8… get Cruise (120 px/s); 3, 6, 9… get Fast (168 px/s). In any session with ≥3 vehicles spawned, all three levels appear. Watch for visible speed differences — the fast vehicle noticeably outruns the yield vehicle.

**AUD-B3 — Gradual deceleration works only for approaching vehicles (LIM-1)**

`vehicle.rs:68-84` (`step_velocity_toward_command`): The ramp uses `BASE_DECELERATION * decel_scale * dt` per frame. For approaching vehicles, the B04 follow logic writes only to `commanded_velocity`, letting the B05 ramp take effect. But for managed vehicles, `smart.rs:108` writes directly to `velocity`, neutralizing the ramp. To see AUD-B3 pass: observe a vehicle **before** it enters the junction zone slow down behind a stopped leader — that deceleration is gradual. Inside the junction, speed changes are instant.

**AUD-B1 — Bonus stats are present in the window**

`stats_window.rs:50-62` (`format_stats_lines`): Four bonus fields are rendered: Session duration (s), Avg crossing time (s), Peak concurrent in zone, Vehicles entered zone. These are clearly labeled "Additional statistics (bonus)." AUD-B1 passes if any of these are visible and non-trivially meaningful.

---

---

# Audit Findings

## Branch / Commit
Branch: `iana/collision-fix` — top commit `469c3bd` (edit roads)

## AUD-16: Sustained random traffic — no collisions
**Verdict**: UNCERTAIN

**Reasoning**:
The R-key path is: `input.rs:49–51` (key-down sets `random_stream_active = true`) → `app.rs:126–128` (one `spawn_random` call per frame while held) → `spawn.rs:178–182` (`spawn_random` picks a random cardinal+route and delegates to `try_spawn`). `try_spawn` gates on a 400 ms per-direction cooldown and a lane capacity of 8, then calls `same_lane_spawn_too_close` (spawn.rs:249–267) to reject spawns that would land within `SAFE_DISTANCE` of an existing vehicle.

Collision avoidance is layered:
1. **Same-lane following**: `enforce_follow_distance` (vehicle.rs:212–265) caps follower `commanded_velocity` proportionally when gap falls below `SAFE_DISTANCE` (40 px), and zeros it below 10 % of the gap.
2. **Reservation gate**: `smart.update` (smart.rs:57–138) denies zone entry to any vehicle whose lane conflicts with a currently-granted or in-zone reservation. `expand_reservations` (smart.rs:259–290) processes candidates in FIFO wait-order, so only one vehicle per conflict group proceeds at a time.
3. **Zone backstop**: `enforce_zone_gate` (smart.rs:333–345) physically retracts any unreserved vehicle that overshoots the zone boundary, zeroing velocity and calling `retract_vehicle_outside_zone`.
4. **Post-move proximity clamp**: `clamp_velocity_for_proximity` (vehicle.rs:544–630) zeros the yielder's velocity whenever OBB separation (`sprite_separation_gap`) drops below `PROXIMITY_BOX_CLEARANCE` (2.0 px), then a second loop corrects any residual sprite overlap with a position push.

The mechanism is architecturally sound. Two specific edge-case risks remain:

- **Position-push ignores leader's `scheduler_yield`** (vehicle.rs:597–621): The push guard at line 602 skips the pair if the *yielder* has `scheduler_yield` set, but does not protect the *leader* from being nudged. A scheduler-frozen leader can be physically displaced by the push, making its `path_index` stale when `advance_along_path` runs next frame. The path-follow code re-clamps lateral drift, but if the longitudinal component is negative (`along < 0.0`, line 695) the vehicle teleports back to the segment start. This is a visible glitch, not a true collision, but it could move a managed vehicle outside the zone boundary briefly, triggering a spurious state transition.

- **Push-loop convergence with many simultaneous overlaps** (vehicle.rs:582–629): The `loop { ... if !any_pushed { break; } }` has no iteration cap. With many vehicles in close proximity, pairs can push each other in conflicting directions and the loop may cycle for many iterations before converging. No infinite-loop has been observed in tests, but dense sustained R traffic (approaching the 8-vehicle lane cap on multiple lanes simultaneously) is the highest-risk scenario.

The two random-specific concerns (simultaneous cross-lane spawning, opposed approaches entering trigger distance together) are handled by the FIFO reservation system and are equivalent to the manually-triggered scenarios already tested in AUD-8–AUD-14.

**Key files**:
- [src/input.rs:49–51](src/input.rs#L49) — R-key toggles `random_stream_active`
- [src/app.rs:126–128](src/app.rs#L126) — one `spawn_random` per frame while R held
- [src/spawn.rs:147–175](src/spawn.rs#L147) — `try_spawn` cooldown + lane-capacity + proximity guard
- [src/vehicle.rs:212–265](src/vehicle.rs#L212) — `enforce_follow_distance` (same-lane)
- [src/smart.rs:57–138](src/smart.rs#L57) — reservation gate + zone management
- [src/smart.rs:333–345](src/smart.rs#L333) — `enforce_zone_gate` backstop
- [src/vehicle.rs:544–630](src/vehicle.rs#L544) — `clamp_velocity_for_proximity` + push loop

**Watch out for**:
- A Managed vehicle briefly snapping backward to a path waypoint when the post-move push loop displaces it (visible backward flash).
- Under extreme R-key density (4 approaches all near lane-cap simultaneously), check for the push loop taking many frames to settle — this would appear as vehicles jittering in place for one or two frames.
- Any scenario where two vehicles from genuinely conflicting lanes (e.g. South-Straight and East-Straight) both appear inside the zone at the same moment; this would mean the reservation gate failed.

---

## AUD-17: Random traffic — low congestion
**Verdict**: PASS

**Reasoning**:
Three mechanisms bound lane queue size under sustained R-key traffic:

1. **Per-direction cooldown** (`SPAWN_COOLDOWN_MS = 400 ms`, config.rs:61; enforced in spawn.rs:62–75): Each approach direction allows at most one new vehicle per 400 ms (~2.5 vehicles/s). R key goes through `spawn_random` → `try_spawn` (spawn.rs:178–182), which calls `self.cooldown.allows(req.approach)` at line 148 — the same check as manual arrow keys. There is no bypass for random spawns.

2. **Lane capacity cap** (`LANE_CAPACITY = 8`, spawn.rs:10): `try_spawn` counts all non-Done vehicles on the target `lane_id` (lines 152–158) and rejects if `>= 8`. The count includes Managed and Exiting vehicles, not just Approaching, so a lane actively clearing the intersection still counts toward the cap. This means at most 8 simultaneous vehicles per lane.

3. **FIFO reservation progress**: Vehicles queued in Approaching state will eventually receive a reservation because `expand_reservations` (smart.rs:259–290) processes candidates in wait-order. No permanent starvation scenario exists: the head-of-queue vehicle gets a reservation as soon as the zone clears, crosses, and transitions to Done, freeing a slot. No circular-wait deadlock is possible because reservation grant is strictly one-at-a-time per conflict group.

Regarding the "fewer than 8 stuck in same lane" criterion: with the cap at 8, exactly 8 vehicles can exist on a lane simultaneously, but the Managed and Exiting vehicles among them are actively moving through the intersection, not "stuck." The number of truly stopped/waiting (Approaching, reservation_hold=true) vehicles on any one lane is in practice well below 8 because vehicles begin crossing as soon as the zone clears.

**Key files**:
- [src/spawn.rs:10](src/spawn.rs#L10) — `LANE_CAPACITY = 8`
- [src/spawn.rs:46–87](src/spawn.rs#L46) — `SpawnCooldown` with 400 ms per direction
- [src/spawn.rs:147–175](src/spawn.rs#L147) — `try_spawn` applies both cooldown and capacity checks
- [src/spawn.rs:178–182](src/spawn.rs#L178) — `spawn_random` uses same `try_spawn` path (no bypass)
- [src/smart.rs:259–290](src/smart.rs#L259) — `expand_reservations` FIFO ordering prevents starvation
- [src/config.rs:61](src/config.rs#L61) — `SPAWN_COOLDOWN_MS = 400`

**Watch out for**:
- At the 8-vehicle lane cap, new random spawns on that approach are silently rejected (no visual feedback). During a sustained R run, watch for a specific lane that appears to stop growing — confirm this is the cap working, not a bug.
- The cap is per `lane_id` (approach × route), not per approach. Three lanes per approach means one approach can hold up to 24 vehicles total across its three route lanes. Check whether the same *approach* (not lane) looks congested; this would still pass AUD-17 as long as each individual lane stays below 8.
- If two opposing straight-lane queues (e.g. South-Straight and North-Straight) both hit the cap simultaneously and their reservations alternate slowly, watch for the queue backing up visually to the spawn margin.

---

## AUD-31: At least 3 distinct velocities
**Verdict**: PASS

**Reasoning**:
Three named velocity levels are defined, assigned at spawn time, stored as the vehicle's `nominal_velocity`, and actively restored during operation:

- `VelocityLevel::Fast` → `DEFAULT_SPAWN_VELOCITY * 1.4` = **168.0 px/s** (vehicle.rs:32)
- `VelocityLevel::Cruise` → `DEFAULT_SPAWN_VELOCITY * 1.0` = **120.0 px/s** (vehicle.rs:33)
- `VelocityLevel::Yield` → `DEFAULT_SPAWN_VELOCITY * 0.5` = **60.0 px/s** (vehicle.rs:34)

Assignment in `spawn_vehicle` (vehicle.rs:125–130) uses `id.0 % 3`: remainder 0 → Fast, 1 → Cruise, 2 → Yield. Since IDs are sequential starting at 1 (spawn.rs:129), the first three vehicles receive Cruise, Yield, and Fast respectively — all three levels appear after the third spawn.

The speed is wired into motion: `spawn_vehicle` sets `velocity`, `commanded_velocity`, and `nominal_velocity` all to `level.speed()` (vehicle.rs:131–140). In free-flowing conditions, `enforce_follow_distance` restores `commanded_velocity = nominal` when the gap exceeds `SAFE_DISTANCE` (vehicle.rs:241). The B05 ramp (`step_velocity_toward_command`, vehicle.rs:99–121) drives `velocity` toward `commanded_velocity` at bounded accel/decel rates. Speed changes are triggered by real conditions (approaching another vehicle reduces commanded_velocity proportionally; entering the reservation trigger zone activates braking; scheduler yield freezes to zero).

Two unit tests confirm the mechanism end-to-end: `b03_spawned_vehicles_have_three_distinct_commanded_velocities` (vehicle.rs:752) verifies the actual commanded_velocity values match the three VelocityLevel speeds after going through the spawn path; `faster_commanded_velocity_drives_strictly_greater_distance` (vehicle.rs:801) proves the speed difference produces measurably different distances in the physics integrator.

**Key files**:
- [src/vehicle.rs:21–37](src/vehicle.rs#L21) — `VelocityLevel` enum and `speed()` values
- [src/vehicle.rs:125–149](src/vehicle.rs#L125) — `spawn_vehicle` assigns level by `id.0 % 3`
- [src/vehicle.rs:240–241](src/vehicle.rs#L240) — nominal speed restored when gap clears
- [src/vehicle.rs:99–121](src/vehicle.rs#L99) — `step_velocity_toward_command` ramp
- [src/config.rs:58](src/config.rs#L58) — `DEFAULT_SPAWN_VELOCITY = 120.0`
- [src/vehicle.rs:752](src/vehicle.rs#L752) — unit test confirming all 3 levels appear

**Watch out for**:
- `VelocityLevel::Yield` (60 px/s) is a *spawn-assigned nominal speed* for id%3==2 vehicles, not the same thing as the scheduler stopping a vehicle or the reservation gate braking a vehicle. A human auditor watching traffic may see vehicles slowed to zero by the scheduler and interpret that as a fourth speed level, or may conflate the nominal Yield speed with a "stopped" state. The distinct visible speeds to look for are: ~168 px/s (visibly fast, outruns others), ~120 px/s (medium), and ~60 px/s (noticeably slower, half the Cruise speed).
- Inside the managed zone, `smart.rs` can snap velocity to zero (scheduler yield) or to nominal instantly (LIM-1 — bypass of B05 ramp). This makes the *number* of observable velocity states larger than three during intersection crossing, but the three *nominal* levels are definitively present.

---

## Summary
The codebase has a well-layered collision-avoidance architecture: same-lane follow-distance capping, a FIFO reservation gate that admits exactly one vehicle per conflict group, a physical zone backstop that ejects reservation overshots, and an OBB-based post-move proximity correction. AUD-31 is the most straightforwardly safe check — three velocity levels are structurally guaranteed after the third vehicle spawns and are verifiable in unit tests without running the simulation. AUD-17 is similarly robust: R-key spawning is not privileged and is bounded by the same 400 ms cooldown and 8-vehicle lane cap as manual spawning, with FIFO reservation preventing starvation. The single biggest risk heading into the live audit is the position-push loop in `clamp_velocity_for_proximity` (vehicle.rs:582–629): it can displace a scheduler-frozen leader off its path segment and runs without an iteration bound, making it the most likely source of unexpected visual behaviour (vehicle snap/jitter) or, in rare dense traffic, a convergence delay. A human auditor running AUD-16 should specifically watch for vehicles that flash backward or stutter when three or more occupy a short approach segment simultaneously.

---

## Shared Setup Steps

All three auditors must complete these steps before starting any checks:

1. **Install SDL2 development libraries:**
   ```
   # Ubuntu/Debian:
   sudo apt-get install libsdl2-dev
   # macOS (Homebrew):
   brew install sdl2
   ```

2. **Build the project:**
   ```
   cargo build
   ```
   Expected: zero compilation errors. If asset files are missing, `cargo run` will fail immediately with a descriptive error (e.g., `failed to load asset assets/roads/approach_ns.bmp`).

3. **Verify assets are present** — the following directories must exist and contain BMP files:
   - `assets/roads/` — `approach_ns.bmp`, `approach_ew.bmp`, `intersection_core.bmp`
   - `assets/vehicles/` — `vehicle_south.bmp`, `vehicle_north.bmp`, `vehicle_west.bmp`, `vehicle_east.bmp`

4. **Run the simulation:**
   ```
   cargo run
   ```
   A 1024×768 window titled "smart-road" should appear with the intersection rendered.

5. **Run the test suite** (recommended before starting):
   ```
   cargo test --lib
   cargo test --test smoke
   ```
   Expect 90 unit + 23 smoke tests passing. Optional manual SDL test:
   `cargo test --test manual_stats_window manual_audit19_stats_window_opens -- --ignored --nocapture`
   (requires SDL2 DLL on `PATH` for Windows).

6. **Arrow key / approach reminder** (counterintuitive mapping):
   - Arrow Up → South approach (vehicle enters from bottom, travels upward)
   - Arrow Down → North approach (vehicle enters from top, travels downward)
   - Arrow Right → West approach (vehicle enters from left, travels rightward)
   - Arrow Left → East approach (vehicle enters from right, travels leftward)

7. **Press Esc to end session and open the statistics window.** Closing the main window with the X button does NOT show the statistics window.