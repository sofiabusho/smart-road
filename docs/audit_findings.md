# Session Audit Findings — smart_road

## Branch / Commit
Branch: `iana/collision-fix` — top commit `469c3bd` (edit roads)

---

## Code Changes Made

| File | Constant | Old value | New value | Reason |
|------|----------|-----------|-----------|--------|
| [src/config.rs](src/config.rs) | `DEFAULT_SPAWN_VELOCITY` | `2.0` | `3.5` | Vehicles were not clearly visible at 2.0 px/s during manual audit; 3.5 gives a better balance of visibility and realism |

No other source files were modified. `BASE_ACCELERATION` (3.0) and `BASE_DECELERATION` (4.0) were set earlier in the same session and were left at those values.

---

## Audit Results Summary

### AUD-16 — Sustained random traffic, no collisions
**Verdict: UNCERTAIN**

The R key routes through the same `try_spawn` path as manual arrow keys, so cooldown and lane-capacity checks apply equally. Collision avoidance is multi-layered: same-lane follow-distance capping, a FIFO reservation gate that serialises conflicting zone entries, a zone backstop that physically retracts any vehicle that overshoots without a reservation, and an OBB proximity clamp that zeroes the yielder's velocity and corrects any residual sprite overlap. The reservation gate is the hard safety net and has no identified failure mode. The concern is the position-push correction loop (`vehicle.rs:582–629`): it has no iteration cap and pushes both vehicles in a pair apart regardless of whether the leader is scheduler-frozen, which can displace a frozen vehicle off its path segment and corrupt its `path_index`. Under dense sustained R traffic this loop must resolve many overlapping pairs simultaneously, making convergence slower and the path-corruption risk higher. No actual collision was reproduced in static analysis, but the scenario cannot be ruled out without running the simulation. **Watch for:** two vehicles from conflicting lanes (e.g. South-Straight and East-Straight) appearing inside the zone at the same moment — that would confirm a reservation gate failure. Also watch for any vehicle that flashes backward on the approach road, which signals path-index corruption from the push loop.

### AUD-17 — Random traffic, low congestion
**Verdict: PASS**

`spawn_random` calls `try_spawn`, which enforces a 400 ms per-direction cooldown and rejects any spawn on a lane that already holds 8 or more non-Done vehicles (`LANE_CAPACITY = 8`, `spawn.rs:10`). The R key receives no special bypass. The FIFO reservation system (`expand_reservations`, `smart.rs:259–290`) processes waiting vehicles in arrival order, so no vehicle can be starved indefinitely. No circular-wait deadlock scenario was identified. **Watch for:** a specific lane that appears to stop accepting new vehicles during a sustained R run — that is the capacity cap working correctly, not a bug. Confirm that no single lane visually accumulates 8 or more stationary vehicles simultaneously.

### AUD-31 — At least 3 distinct velocities
**Verdict: PASS**

Three named velocity levels are defined in `VelocityLevel` (`vehicle.rs:21–37`) and assigned at spawn by `id.0 % 3`: Fast (3.5 × 1.4 = 4.9 px/s), Cruise (3.5 × 1.0 = 3.5 px/s), Yield (3.5 × 0.5 = 1.75 px/s). All three levels appear after the third vehicle spawns. The assignment is stored in `nominal_velocity` and actively restored by `enforce_follow_distance` whenever a vehicle's following gap clears. Two unit tests confirm the mechanism end-to-end without running the simulation. **Watch for:** spawn at least three vehicles and observe that they travel at visibly different speeds in free-flowing conditions — the Fast vehicle should noticeably outpace the Yield vehicle. Do not confuse a VelocityLevel::Yield vehicle (slow but moving) with a vehicle that has been stopped by the scheduler (velocity = 0); these are different states.

### AUD-B3 — Acceleration and deceleration component
**Verdict: PASS (gradual)** — with one bounded exception inside the junction zone

When a vehicle needs to change speed, `step_velocity_toward_command` (`vehicle.rs:99–121`) is called every frame by `advance_along_path`. It computes the maximum allowed speed change for this frame as `BASE_ACCELERATION * accel_scale * dt` (speeding up) or `BASE_DECELERATION * decel_scale * dt` (slowing down), clamps the delta to that ceiling, and writes the result back. `BASE_ACCELERATION = 3.0` and `BASE_DECELERATION = 4.0` are live constants actually used in this calculation. Three motion personalities (id mod 3) give different vehicles different multipliers (1.4/1.2, 1.0/1.0, 0.7/0.6), so vehicles visibly brake and accelerate at different rates. The one exception is the scheduler emergency stop inside the junction zone: `schedule_managed_velocities` (`smart.rs:178–180`) writes `velocity = 0.0` directly when a vehicle must yield to a crossing car. This is intentional and only occurs inside the zone boundary. **Watch for:** on the approach road, a follower behind a stopped leader should decelerate over roughly 60–70 frames (~1 second at current constants); if it stops in a single frame on the approach, the ramp is being bypassed. Inside the zone, an instant stop is expected and correct.

---

## Outstanding Risks

- **AUD-16 position-push loop** (`vehicle.rs:582–629`): no iteration cap, can displace scheduler-frozen leaders off their path. Dense sustained R-key traffic is the highest-risk trigger. Needs a 60+ second live run to confirm no collision or path-corruption event occurs.
- **AUD-16 path-index corruption**: after a proximity push, a vehicle's `path_index` may be stale relative to its new position. Watch for vehicles that snap backward or stutter on the approach road during high-density traffic.
- **LIM-1 (from audit_plan.md)**: inside the managed zone, velocity changes are instant (direct write to `vehicle.velocity`), not ramped. AUD-B3 passes for approach-road behaviour; the live auditor should be clear that the gradual ramp only applies on the approach and exit arms, not mid-junction.
- **LIM-2 (from audit_plan.md)**: `SAFE_DISTANCE = 40.0` is only 4 px larger than `VEHICLE_LENGTH = 36.0`. Following vehicles will appear nearly bumper-to-bumper. Confirm no actual centre-overlap before marking as a collision.
- **Frame rate**: the game loop has no vsync or frame-rate cap. On a fast machine the simulation runs at many multiples of 60 fps because it uses a fixed `dt = 1/60` regardless of actual elapsed time. If vehicles still appear too fast or too slow at `DEFAULT_SPAWN_VELOCITY = 3.5`, this constant is the only tunable.

---

## Checks Not Yet Covered

The following AUD IDs from the full audit list were **not** audited this session and still need human review during the live run:

AUD-1, AUD-2, AUD-3, AUD-4, AUD-5, AUD-6, AUD-7, AUD-8, AUD-9, AUD-10, AUD-11, AUD-12, AUD-13, AUD-14, AUD-15, AUD-18, AUD-19, AUD-20, AUD-21, AUD-22, AUD-23, AUD-24, AUD-25, AUD-26, AUD-27, AUD-28, AUD-29, AUD-30, AUD-B1, AUD-B2, AUD-B4

Covered this session: **AUD-16, AUD-17, AUD-31, AUD-B3** (4 of 35 total checks).
