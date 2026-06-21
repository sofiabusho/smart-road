# Software Design Specification (SDS) — smart-road

> **Project:** smart-road  
> **Date:** 2026-06-10  
> **Purpose:** Technical implementation guide for Rust/SDL2 cross-intersection AV simulation.  
> **Note:** Reference alongside `AGENTS.md`, `docs/PRD.md`, `docs/requirements.md`, and `docs/audit.md`.

---

## 1. Architecture Overview

```text
main.rs
  └── App
        ├── Window / Renderer (SDL2)
        ├── EventHandler (keyboard)
        ├── GameLoop (fixed timestep)
        ├── IntersectionModel
        ├── SmartController
        ├── VehicleSim[]
        ├── StatsCollector
        └── StatsWindow (on Esc)

src/
  main.rs           # entry, App wiring
  app.rs            # loop orchestration
  config.rs         # safe distance, speeds, spawn cooldown
  intersection.rs   # topology, lanes, routes, conflict graph
  vehicle.rs        # state, physics integration
  smart.rs          # detection, scheduling, velocity commands
  spawn.rs          # arrow/R spawn, anti-spam
  render.rs         # assets, blit, rotation
  input.rs          # key map, repeat filtering
  stats.rs          # counters, min/max, close calls
  stats_window.rs   # post-Esc presentation
```

### 1.1 Design Principles

- **Simulation vs presentation**: vehicle logic updates in world units; render layer maps world → screen and handles rotation only.
- **Single writer for velocity commands**: `SmartController` is the only module that commands intersection-zone speed changes (REQ-3, REQ-7).
- **Deterministic test hooks**: physics and conflict resolution exposed to unit tests without SDL2 where possible.
- **Fail loud on missing assets/SDL**: startup errors print actionable messages (NFR-5).

---

## 2. Crate Layout

```toml
# Cargo.toml (planned)
[package]
name = "smart-road"
version = "0.1.0"
edition = "2021"

[dependencies]
sdl2 = { version = "0.37", features = ["image"] }  # exact version pinned at scaffold

[[bin]]
name = "smart-road"
path = "src/main.rs"
```

| Path | Role |
|------|------|
| `src/` | Application modules |
| `assets/vehicles/` | Vehicle sprites (per orientation or rotatable base) |
| `assets/roads/` | Intersection tiles / background |
| `assets/fonts/` | Stats window typography (if needed) |
| `tests/` | Integration tests (physics, smart scheduling) |

---

## 3. Game Loop

```text
each frame:
  1. poll SDL events → InputState
  2. handle Esc → end session, open StatsWindow
  3. handle arrows / R → SpawnSystem (respect cooldown)
  4. SmartController.update(vehicles, intersection, dt)
  5. VehicleSim.integrate_physics(dt)  // velocity = distance / time
  6. detect close calls + completed crossings → StatsCollector
  7. remove exited vehicles
  8. render intersection + vehicles (with rotation)
```

- **Timestep**: fixed `dt` (e.g., 1/60 s) for stable physics; optional accumulator pattern.
- **Coordinate system**: origin top-left, +x east, +y south (matches SDL); document in `config.rs`.

---

## 4. Intersection Model

### 4.1 Topology

Cross intersection with **four approaches** (N, S, E, W). Each approach has **three lanes** with fixed routes:

| Lane index (per approach) | Route |
|---------------------------|-------|
| 0 | `r` |
| 1 | `s` |
| 2 | `l` |

Each lane has:

- `spawn_point: Vec2`
- `path: Vec<Waypoint>` through intersection (polyline)
- `exit_point: Vec2`

### 4.2 Conflict graph

Precompute **conflicting lane pairs** whose paths intersect in the junction box. `SmartController` uses this graph to schedule passage (time slots, velocity caps, or reservation tokens — implementation choice documented in `smart.rs`).

```rust
// intersection.rs (outline)
pub enum Route { Right, Straight, Left }

pub struct Lane {
    pub id: LaneId,
    pub approach: Cardinal,  // N, S, E, W
    pub route: Route,
    pub spawn_point: Vec2,
    pub path: Vec<Vec2>,
}

pub fn conflict_pairs(lanes: &[Lane]) -> Vec<(LaneId, LaneId)>;
```

---

## 5. Vehicle State

```rust
// vehicle.rs (outline)
pub struct Vehicle {
    pub id: u64,
    pub lane_id: LaneId,
    pub route: Route,
    pub position: Vec2,
    pub heading: f32,           // radians, for render rotation
    pub path_index: usize,
    pub velocity: f32,          // world units / second
    pub distance_in_crossing: f32,
    pub time_in_crossing: f32,
    pub state: VehicleState,
    pub commanded_velocity: f32,
}

pub enum VehicleState {
    Approaching,
    Managed,    // detected by smart system (REQ-4) — timer starts
    Exiting,
    Done,
}
```

### 5.1 Physics (REQ-5, REQ-7, REQ-8)

- Maintain `distance_in_crossing` and `time_in_crossing` while `state >= Managed`.
- Reported velocity: `velocity = distance_in_crossing / time_in_crossing` (guard divide-by-zero).
- **Velocity levels**: at least three discrete setpoints, e.g. `FAST`, `CRUISE`, `YIELD` (configurable floats).
- **Safe distance**: `SAFE_DISTANCE: f32` in `config.rs` (strictly positive). Follower must keep `distance_to_leader >= SAFE_DISTANCE`.
- **Close call**: record when two vehicles pass with separation `< SAFE_DISTANCE` but `> collision_threshold`.

### 5.2 Route adherence (REQ-6)

- Vehicle locked to `lane_id` at spawn; path pulled from `Lane::path`.
- No lateral lane shifts; position snapped/ projected onto path polyline.

---

## 6. Smart Intersection Algorithm (outline)

**Goal**: no traffic lights; coordinate via velocity/time/distance (REQ-3).

### 6.1 Detection (REQ-4)

When vehicle center enters `intersection_zone` polygon → transition to `Managed`, start crossing timer.

### 6.2 Coordination strategy (recommended v1)

**Reservation-time / velocity scheduling**:

1. For each conflicting lane pair, maintain active reservation interval on the conflict point.
2. When vehicle `V` enters `Managed`, `SmartController` assigns earliest feasible entry time `t_enter` based on reservations.
3. Compute required velocity to hit `t_enter` (or yield with `YIELD` speed until slot opens).
4. On conflict risk, command lower `commanded_velocity` (AUD-15).

```rust
// smart.rs (outline)
pub struct SmartController {
    reservations: HashMap<ConflictPointId, TimeInterval>,
    safe_distance: f32,
}

impl SmartController {
    pub fn update(&mut self, vehicles: &mut [Vehicle], lanes: &[Lane], dt: f32);
    fn schedule_velocity(&self, vehicle: &Vehicle, conflicts: &[LaneId]) -> f32;
}
```

Alternative acceptable approaches: platoon gaps, distance-based slowing, or time-slot tokens — must satisfy same REQ/AUD behavior.

### 6.3 Congestion target

Avoid ≥8 vehicles queued in one lane (AUD-17): throttle spawns or increase yield aggressiveness when queue depth high.

---

## 7. Input Map

| Key | Action | REQ |
|-----|--------|-----|
| `Up` | Spawn on south approach | REQ-12 |
| `Down` | Spawn on north approach | REQ-13 |
| `Right` | Spawn on west approach | REQ-14 |
| `Left` | Spawn on east approach | REQ-15 |
| `R` | Random spawn (loop) | REQ-16 |
| `Esc` | End sim + stats window | REQ-17 |

### 7.1 Anti-spam (REQ-18)

```rust
// spawn.rs
pub struct SpawnCooldown {
    per_direction_ms: u64,  // e.g., 300–500 ms
    last_spawn: HashMap<Cardinal, Instant>,
}
```

Ignore or defer spawns inside cooldown window. SDL key-repeat should not bypass cooldown.

### 7.2 Random spawn (`R`)

Each tick while active: pick random `Cardinal` + random lane/route on that approach; apply same cooldown globally or per-direction.

---

## 8. Rendering and Animation

### 8.1 Assets (REQ-10)

- Load road background and vehicle sprites via `sdl2::image` or textures from `assets/`.
- Define **world units per pixel** for consistent speed display.

### 8.2 Rotation while turning (REQ-11)

- Update `heading` from path tangent each frame.
- `render.rs` uses `canvas.copy_ex(texture, None, dst_rect, heading_degrees, None, false, false)`.
- Do not reuse a single down-facing sprite without rotation through turns.

---

## 9. Statistics Collection

```rust
// stats.rs (outline)
pub struct Stats {
    pub vehicles_passed: u32,
    pub max_vehicles_passed: u32,   // peak concurrent or cumulative per brief
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_crossing_time: f32,
    pub min_crossing_time: f32,
    pub close_calls: u32,
}

impl Stats {
    pub fn on_vehicle_managed(&mut self, v: &Vehicle);
    pub fn on_vehicle_exited(&mut self, v: &Vehicle);
    pub fn on_close_call(&mut self);
}
```

- **Max vehicles passed**: track peak count of vehicles that completed intersection crossing in session (align with AUD-20 — four in test scenario).
- **Crossing time**: `time_in_crossing` at exit from `Managed` → `Done`.
- **Stats window** (`stats_window.rs`): new SDL window or modal surface listing all fields (REQ-19–REQ-26).

---

## 10. Module Specifications (summary)

| Module | Responsibility | Key AUD |
|--------|----------------|---------|
| `intersection` | Lanes, routes, conflicts | AUD-1, AUD-28 |
| `spawn` | Keyboard spawn, cooldown | AUD-3–7, AUD-27 |
| `vehicle` | Physics, safe distance | AUD-29–31 |
| `smart` | Algorithm, yield | AUD-8–17, AUD-15 |
| `render` | Assets, rotation | AUD-2, AUD-11 |
| `stats` / `stats_window` | Metrics on Esc | AUD-19–26 |
| `input` | Event dispatch | AUD-3–7, AUD-17 |

---

## 11. Testing Notes

| Layer | Approach |
|-------|----------|
| Unit | `velocity = distance/time`, safe-distance braking, conflict scheduler picks yield |
| Integration | Spawn cooldown rejects rapid duplicate spawns |
| Manual | Full `docs/audit.md` walkthrough — authoritative for visual/animation checks |

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo run
```

---

## 12. Error Handling

- Missing asset file → panic at load with path in message (dev); consider graceful exit in release.
- SDL init failure → stderr message with platform SDL2 install hint.
- Zero vehicles at Esc → stats window still shows zeros / N/A for min velocity.

---

## 13. Cross-track interfaces (parallel development)

Three developers work tracks **A**, **B**, and **C** per `docs/ticket-tracker.md`. This section defines **stub contracts** so tracks can proceed in parallel once predecessors are ✅.

### 13.1 File ownership

| Module / file | Owner track | Notes |
|---------------|-------------|-------|
| `main.rs`, `app.rs` | **A** (A01) | **C01** may wire `SmartController::update()` in the tick loop; **C07** adds `App::end_session()` — cross-track edits require SDS update + PR note |
| `config.rs` | **A** (A01) | B/C propose constants via ticket PR; A merges tunables |
| `intersection.rs` | **A** (A03) + **B** (B02) | A owns topology + render-facing layout; B owns `path: Vec<Vec2>` per lane — separate impl blocks or `lane_paths` submodule |
| `spawn.rs`, `input.rs` | **A** | **C01** may add `SpawnSystem::vehicles_mut()`; **B02**/**B04** call `advance_along_path` / `enforce_follow_distance` from `SpawnSystem::update` |
| `render.rs` | **A** (A03, A07) | Reads `VehicleRenderSnapshot` only — no mutation |
| `vehicle.rs` | **B** (B01) | **A04 stub** (IF-1): `spawn_vehicle`, `approach` / `position` / `heading_rad` fields; B01 owns physics and `VehicleId` allocation |
| `smart.rs` | **C** | |
| `stats.rs`, `stats_window.rs` | **C** | |

**Rule**: Do not edit another track's owned file without updating this table and announcing in PR.

### 13.2 Track A exports (platform & presentation)

Delivered by **A03** / **A04**; stable once those tickets merge.

```rust
// intersection.rs (A03) — topology stub
pub enum Cardinal { North, South, East, West }
pub enum Route { Right, Straight, Left }
pub struct LaneId(pub u32);

impl Cardinal {
    pub const fn travel_heading(self) -> f32;  // A04: approach-aligned unit heading (radians)
}

pub struct LaneInfo {
    pub id: LaneId,
    pub approach: Cardinal,
    pub route: Route,
    pub spawn_point: Vec2,
    // path: added by B02 in same struct or parallel map
}

pub struct IntersectionModel {
    pub lanes: Vec<LaneInfo>,
    pub zone_polygon: Vec<Vec2>,  // smart-system detection boundary (C01 reads)
}

// Shared render DTO (A04) — used by B snapshot_for_render and A draw_vehicle
pub struct VehicleRenderSnapshot {
    pub position: Vec2,
    pub heading_rad: f32,
    pub approach: Cardinal,  // A04: per-approach color/dims; A07 adds texture rotation from heading
}

// spawn.rs (A04)
pub struct SpawnRequest {
    pub approach: Cardinal,
    pub route: Route,       // rotates r→s→l per approach (PRD OQ-6)
    pub lane_id: LaneId,
}

pub struct SpawnCooldown { per_direction_ms: u64, last_spawn: HashMap<Cardinal, Instant> }

pub struct SpawnSystem { /* vehicles, route_counters, cooldown */ }

impl SpawnSystem {
    pub fn try_spawn(&mut self, req: SpawnRequest, model: &IntersectionModel) -> Option<VehicleId>;
    pub fn spawn_on_approach(&mut self, approach: Cardinal, model: &IntersectionModel) -> Option<VehicleId>;
    pub fn spawn_random(&mut self, model: &IntersectionModel) -> Option<VehicleId>;  // REQ-16 / A06
    pub fn update(&mut self, model: &IntersectionModel, dt: f32);  // B02: signature updated to accept model for advance_along_path
    pub fn vehicles(&self) -> &[Vehicle];
    pub fn vehicles_mut(&mut self) -> &mut [Vehicle];  // C01: smart-system zone detection
}

// input.rs (A04+)
pub fn approach_for_arrow(key: Keycode) -> Option<Cardinal>;  // REQ-12–REQ-15

pub enum InputEvent {
    SpawnCardinal(Cardinal),
    RandomStream(bool),   // R key down/up (A06)
    Exit,                 // Esc (C06)
}

// InputState tracks `random_stream_active` while R is held (A06).

// render.rs (A03, A07)
pub struct RoadAssets<'tex> { /* BMP textures + layout dims */ }

impl<'tex> RoadAssets<'tex> {
    pub fn load(creator: &'tex TextureCreator<WindowContext>) -> Result<Self, String>;
}

pub fn draw_intersection(
    canvas: &mut Canvas,
    model: &IntersectionModel,
    assets: &RoadAssets<'_>,
);
pub fn draw_frame(
    canvas: &mut Canvas,
    intersection: &IntersectionModel,
    assets: &RoadAssets<'_>,
    vehicles: &[VehicleRenderSnapshot],  // A04
);
pub fn draw_vehicle(canvas: &mut Canvas, snapshot: &VehicleRenderSnapshot);
```

### 13.3 Track B exports (vehicle simulation)

Delivered by **B01**–**B04**; **B01** can start when A04's `SpawnRequest` stub exists.

```rust
// vehicle.rs — A04 IF-1 stub fields; B01 expands physics
pub struct VehicleId(pub u64);

pub struct Vehicle {
    pub id: VehicleId,
    pub lane_id: LaneId,
    pub route: Route,
    pub approach: Cardinal,       // A04 stub
    pub position: Vec2,           // A04 stub; B01 integrates
    pub heading_rad: f32,         // A04 stub
    pub velocity: f32,
    pub state: VehicleState,
    // B01+: path_index, distance_in_crossing, time_in_crossing
    // IF-2: commanded_velocity (B01 field; C writes in Managed state)
}

pub enum VehicleState { Approaching, Managed, Exiting, Done }

pub enum VelocityLevel { Fast, Cruise, Yield }  // ≥3 levels (B03)

pub fn spawn_vehicle(id: VehicleId, lane: &LaneInfo, velocity: f32) -> Vehicle;  // A04 IF-1 stub
pub fn integrate_physics(vehicle: &mut Vehicle, dt: f32);  // B01
pub fn snapshot_for_render(vehicle: &Vehicle) -> VehicleRenderSnapshot;  // A04 stub; uses intersection::VehicleRenderSnapshot

// B02 — route adherence
pub fn attach_paths(model: &mut IntersectionModel, paths: LanePathMap);
pub fn advance_along_path(vehicle: &mut Vehicle, model: &IntersectionModel, dt: f32);

// B04 — safe distance (approach zone; smart zone defers to C)
pub fn enforce_follow_distance(vehicles: &mut [Vehicle], safe_distance: f32);
pub fn detect_close_call(a: &Vehicle, b: &Vehicle, safe_distance: f32) -> bool;
```

**Velocity authority split** (see §13.6 open decision):

- **Approaching**: B sets velocity via follow-distance + level.
- **Managed** (inside zone): C commands via `commanded_velocity`; B integrates position.

### 13.4 Track C exports (smart control & stats)

Delivered by **C01**–**C07**; **C01** needs B02 paths + A03 `zone_polygon`.

```rust
// smart.rs
pub struct SmartController { /* reservations, config */ }

impl SmartController {
    pub fn on_vehicle_enter_zone(&mut self, v: &mut Vehicle);
    pub fn update(&mut self, vehicles: &mut [Vehicle], model: &IntersectionModel, dt: f32);
    // sets vehicle.commanded_velocity in Managed state only
}

// stats.rs
pub struct Stats { /* REQ-20..26 fields */ }

pub enum StatsEvent {
    VehicleManaged { id: VehicleId, t: f32 },
    VehicleExited { id: VehicleId, crossing_time: f32, peak_velocity: f32 },
    CloseCall { ids: (VehicleId, VehicleId) },
    VelocitySample { id: VehicleId, v: f32 },
}

pub fn apply_event(stats: &mut Stats, event: StatsEvent);

// stats_window.rs (C06)
pub struct SessionSummary { pub stats: Stats, pub vehicles_passed: u32 }

pub fn show_stats_window(summary: SessionSummary) -> Result<(), String>;

// app.rs hook (C06, coordinated with A)
pub fn end_session(app: &mut App) -> SessionSummary;
```

### 13.5 Integration order (minimal merge sequence)

```text
1. A01  → empty SDL2 loop
2. A03  → IntersectionModel + render (parallel A02)
3. A04  → SpawnRequest + arrow spawn  ──┐
4. B01  → Vehicle + physics (stub spawn)│ parallel after step 3
5. B02  → path polylines on lanes       │
6. A05, A06 → anti-spam, R spawn       │ Track A continues
7. C01  → managed-zone detection        │
8. B03, B04 → velocities, safe distance │ parallel
9. C02  → scheduler                     │
10. C03, C04 → yield + sustained traffic (C04 needs A06)
11. C05, C06 → stats + Esc window
12. A07  → turn animation (needs B02 tangents)
13. C07  → audit dry-run (all mandatory tracks)
```

### 13.6 Open interface decisions (human)

| ID | Question | Proposed default |
|----|----------|------------------|
| IF-1 | Who allocates `VehicleId`? | **B** (`vehicle.rs`) on spawn; **A04 interim**: `SpawnSystem::try_spawn` allocates id and passes to `spawn_vehicle`; B01 moves allocation to B |
| IF-2 | `commanded_velocity` field on `Vehicle`? | Add in **B01**; write access **C only** in `Managed` state |
| IF-3 | `intersection.rs` split pattern? | Single file, `mod topology` (A) + `mod paths` (B) to reduce merge pain |
| IF-4 | Stats window: second SDL window vs overlay? | **C** owns; default second window per PRD OQ-4 |
| IF-5 | Close-call detection owner? | **B** detects proximity; **C** records via `StatsEvent` in game loop |

---

## Cross-References

| Document | Relationship |
|----------|--------------|
| `docs/PRD.md` | Product-level requirements this spec implements |
| `docs/audit.md` | Behavioral checks derived from this SDS |
| `docs/requirements.md` | REQ IDs mapped to modules above |
| `docs/ticket-tracker.md` | A/B/C tracks and cross-deps |
| `AGENTS.md` | Coding standards for implementation |
