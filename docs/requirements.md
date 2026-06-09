# Requirements — smart-road

> Derived from `docs/raw/requirements-source.md` (01-edu Smart Road brief).  
> Treat as READ-ONLY once approved. Changes require stakeholder review.

## 1. Context

**smart-road** is an 01-edu school project that simulates autonomous vehicles (AVs) navigating a **cross intersection** using a **smart intersection management strategy** — without traffic lights. The simulation must prevent collisions, minimize congestion, and provide keyboard-driven testing plus exit statistics for auditors.

Reference brief: [`road_intersection`](https://public.01-edu.org/subjects/road_intersection/) (prior Piscine raid); this project replaces traffic-light control with algorithmic velocity/time/distance coordination.

## 2. Objectives

- Implement a **smart intersection algorithm** so AVs pass a cross intersection **without collisions** and with **minimal traffic congestion**.
- Simulate **autonomous vehicles** with realistic physics (`velocity = distance / time`) and controllable speeds.
- Deliver a **visual SDL2 animation** (not static sprites) that auditors can exercise via keyboard commands and verify via a statistics window on exit.

## 3. Functional Requirements

### Intersection layout

#### REQ-1: Cross intersection

The simulation must render a **cross intersection** — four approaches meeting at a central junction. Each approach has lanes with fixed routes.

#### REQ-2: Lane routes (`r` / `s` / `l`)

Each lane has exactly **one outgoing direction**, represented by route:

| Route | Meaning |
|-------|---------|
| `r` | Turn right |
| `s` | Go straight |
| `l` | Turn left |

A vehicle's path through the intersection area is fully determined by its lane route. Routes are fixed per lane; vehicles cannot choose a different exit.

### Smart intersection algorithm

#### REQ-3: No traffic lights; algorithmic control

Intersection management must **not** use traffic lights. A **smart intersection system** coordinates AV passage by controlling **velocity**, **time**, and/or **distance** (implementation choice) so vehicles cross safely with low congestion.

#### REQ-4: Vehicle detected by smart system

The smart intersection algorithm must **detect** each vehicle when it enters the managed intersection zone. Crossing-time statistics (REQ-23) start at this detection point.

### Vehicle physics and behavior

#### REQ-5: Physics model

Each AV must track:

- `time` — duration to leave the intersection
- `distance` — distance traveled to leave the intersection
- `velocity` — current speed, computed as **`velocity = distance / time`**

#### REQ-6: Route adherence

AVs on a lane with a given route (`r`, `s`, or `l`) must **follow that route**. Vehicles **must not change lanes or routes**.

#### REQ-7: Minimum three velocities

AVs must support **at least three distinct velocity levels**. The smart intersection system uses velocity control as its primary coordination mechanism.

#### REQ-8: Safe distance

Each AV must maintain a **strictly positive safety distance** from other AVs. When a faster vehicle approaches another, it must detect it and keep safe separation — **no collisions**. The exact distance value is implementer-defined but must be positive and reasonable.

#### REQ-9: Collision avoidance

When conflict is imminent, vehicles must avoid collision (e.g., by **reducing velocity**) rather than overlapping or crashing.

### Animation

#### REQ-10: SDL2 animation with assets

Animation is **required**. The simulation must use visual assets for vehicles and roads (e.g., from suggested sources in the brief: limezu, finalbossblue, mobilegamegraphics, spriters-resource). Rendering must use a defined world coordinate system — not a single static image blit.

#### REQ-11: Movement animation while turning

Vehicles must be **animated while moving**. When a vehicle turns (e.g., route `r`), its rendered orientation must **change to match the turn** — not remain facing the approach direction. Sprite rotation or equivalent manipulation is required.

### Keyboard commands

#### REQ-12: Arrow Up — spawn from South

`Arrow Up` spawns a vehicle traveling **from South toward North** (entering from the south approach).

#### REQ-13: Arrow Down — spawn from North

`Arrow Down` spawns a vehicle traveling **from North toward South**.

#### REQ-14: Arrow Right — spawn from West

`Arrow Right` spawns a vehicle traveling **from West toward East**.

#### REQ-15: Arrow Left — spawn from East

`Arrow Left` spawns a vehicle traveling **from East toward West**.

#### REQ-16: `R` — continuous random spawn

`R` continuously generates **random vehicles** (random lane and route) via the game loop while held or toggled per implementation.

#### REQ-17: `Esc` — exit and statistics

`Esc` ends the simulation and opens a **statistics window** (REQ-18–REQ-24).

#### REQ-18: Anti-spam spawn prevention

Rapidly pressing the same spawn key must **not** create vehicles on top of each other. Spawns must be rate-limited or staggered so vehicles do not overlap at creation.

### Statistics (on exit)

#### REQ-19: Statistics window

After `Esc`, display a window listing all required statistics below.

#### REQ-20: Max vehicles passed

Show the **maximum number of vehicles that passed the intersection** during the session.

#### REQ-21: Max velocity

Show the **fastest speed achieved** by any vehicle.

#### REQ-22: Min velocity

Show the **slowest speed reached** by any vehicle.

#### REQ-23: Crossing time measurement

For each vehicle, measure time from **smart-intersection detection** until the vehicle is **removed from the canvas** (fully exited the intersection).

#### REQ-24: Max crossing time

Display the **longest** intersection crossing time among all vehicles.

#### REQ-25: Min crossing time

Display the **shortest** intersection crossing time among all vehicles.

#### REQ-26: Close calls

Display **close calls** — events where two vehicles passed each other with a **violation of the safe distance** (REQ-8) but without collision.

### Bonus (optional, non-blocking)

#### REQ-B1: Custom assets

Create **original** vehicle/road assets instead of (or in addition to) third-party packs.

#### REQ-B2: Additional statistics

Expose **more statistics** beyond REQ-20–REQ-26.

#### REQ-B3: Acceleration and deceleration

Vehicles change speed via **acceleration/deceleration** rather than instantaneous velocity jumps; different vehicles may take different times to change speed.

## 4. Non-Functional Requirements

| ID | Requirement |
|----|-------------|
| NFR-1 | Implementation language: **Rust** |
| NFR-2 | Graphics/input: **SDL2** (`sdl2` crate) |
| NFR-3 | Animation is mandatory — static non-animated rendering is insufficient |
| NFR-4 | Native desktop binary; no web-only or headless-only delivery for audit |
| NFR-5 | Auditor can run the binary and complete manual checks in `docs/audit.md` without undocumented setup |

## 5. Constraints

- **Rust** + **SDL2** — required stack per brief.
- **Animation required** — movement and turn orientation changes.
- **No emergency vehicles** — only autonomous passenger vehicles.
- **No traffic lights** — coordination via smart algorithm only.
- Safe distance must be a **strictly positive** value.

## 6. Out of Scope

- Emergency vehicles and other non-AV traffic.
- Traffic lights or traditional signal timing.
- Human-driven vehicle behavior models.
- Lane or route changes mid-journey.
- Real-world deployment, map import, or GPS integration.
- Networked/multiplayer simulation.

## 7. Documentation Requirements

- README must explain how to install SDL2 dependencies, build (`cargo build`), and run the simulation.
- `docs/audit.md` must be executable by an auditor without guessing controls or expected outcomes.

## Cross-References

| Document | Relationship |
|----------|--------------|
| `docs/PRD.md` | Expands these requirements into detailed product spec |
| `docs/audit.md` | Maps acceptance checks back to REQ IDs |
| `docs/ticket-tracker.md` | Tracks implementation coverage per REQ ID |
| `docs/raw/requirements-source.md` | Authoritative stakeholder source text |
| `AGENTS.md` | Agent coding guidelines aligned with these requirements |
