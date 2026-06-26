//! Keyboard-driven vehicle spawning (A04+).

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::intersection::{Cardinal, IntersectionModel, LaneId, Route, Vec2};
use crate::vehicle::{spawn_vehicle, Vehicle, VehicleId, VehicleState};

/// Maximum vehicles allowed on a single lane before new spawns are blocked (AUD-17).
pub const LANE_CAPACITY: usize = 8;

/// Lightweight PRNG for spawn randomization (no extra crate dependency).
#[derive(Debug, Clone)]
struct SpawnRng {
    state: u32,
}

impl SpawnRng {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(1))
            .as_nanos() as u32;
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift32
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn pick_cardinal(&mut self) -> Cardinal {
        Cardinal::ALL[self.next_u32() as usize % Cardinal::ALL.len()]
    }

    fn pick_route(&mut self) -> Route {
        Route::ALL[self.next_u32() as usize % Route::ALL.len()]
    }
}

/// Per-direction spawn throttle (REQ-18 / AUD-27).
#[derive(Debug)]
pub struct SpawnCooldown {
    per_direction_ms: u64,
    last_spawn: HashMap<Cardinal, Instant>,
}

impl SpawnCooldown {
    pub fn new() -> Self {
        Self {
            per_direction_ms: crate::config::SPAWN_COOLDOWN_MS,
            last_spawn: HashMap::new(),
        }
    }

    /// Whether a spawn on this approach is allowed right now.
    pub fn allows(&self, approach: Cardinal) -> bool {
        self.allows_at(approach, Instant::now())
    }

    /// Record a successful spawn on this approach.
    pub fn record(&mut self, approach: Cardinal) {
        self.record_at(approach, Instant::now());
    }

    fn allows_at(&self, approach: Cardinal, now: Instant) -> bool {
        match self.last_spawn.get(&approach) {
            None => true,
            Some(&last) => now.duration_since(last) >= Duration::from_millis(self.per_direction_ms),
        }
    }

    fn record_at(&mut self, approach: Cardinal, now: Instant) {
        self.last_spawn.insert(approach, now);
    }
}

impl Default for SpawnCooldown {
    fn default() -> Self {
        Self::new()
    }
}

/// Request to create a vehicle on an approach (SDS §13.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnRequest {
    pub approach: Cardinal,
    pub route: Route,
    pub lane_id: LaneId,
}

impl SpawnRequest {
    /// Build a spawn request for an approach and route.
    pub fn new(approach: Cardinal, route: Route) -> Self {
        Self {
            approach,
            route,
            lane_id: crate::intersection::lane_id(approach, route),
        }
    }
}

/// Crossing data captured when a vehicle leaves the canvas (C05).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleExit {
    pub id: VehicleId,
    pub time_in_crossing: f32,
}

/// Spawn pipeline: keyboard requests → vehicles (A04).
#[derive(Debug)]
pub struct SpawnSystem {
    vehicles: Vec<Vehicle>,
    next_id: u64,
    route_counters: HashMap<Cardinal, u8>,
    cooldown: SpawnCooldown,
    rng: SpawnRng,
}

impl SpawnSystem {
    pub fn new() -> Self {
        Self {
            vehicles: Vec::new(),
            next_id: 1,
            route_counters: HashMap::new(),
            cooldown: SpawnCooldown::new(),
            rng: SpawnRng::new(),
        }
    }

    /// Active vehicles in the simulation.
    pub fn vehicles(&self) -> &[Vehicle] {
        &self.vehicles
    }

    /// Mutable access for smart-system updates (C01+).
    pub fn vehicles_mut(&mut self) -> &mut [Vehicle] {
        &mut self.vehicles
    }

    /// Attempt to spawn a vehicle from a request. Returns `None` if cooldown or congestion rejects.
    pub fn try_spawn(&mut self, req: SpawnRequest, model: &IntersectionModel) -> Option<VehicleId> {
        if !self.cooldown.allows(req.approach) {
            return None;
        }

        let queued = self
            .vehicles
            .iter()
            .filter(|v| v.lane_id == req.lane_id && v.state != VehicleState::Done)
            .count();
        if queued >= LANE_CAPACITY {
            return None;
        }

        let lane = model.lane(req.lane_id)?;
        let spawn_pos = spawn_position_on_lane(lane, &self.vehicles);
        if same_lane_spawn_too_close(spawn_pos, lane, &self.vehicles) {
            return None;
        }

        let id = VehicleId(self.next_id);
        self.next_id += 1;

        let mut vehicle = spawn_vehicle(id, lane, crate::config::DEFAULT_SPAWN_VELOCITY);
        vehicle.position = spawn_pos;
        self.vehicles.push(vehicle);
        self.cooldown.record(req.approach);
        Some(id)
    }

    /// Spawn with a random approach and route (REQ-16 / AUD-7).
    pub fn spawn_random(&mut self, model: &IntersectionModel) -> Option<VehicleId> {
        let approach = self.rng.pick_cardinal();
        let route = self.rng.pick_route();
        self.try_spawn(SpawnRequest::new(approach, route), model)
    }

    /// Spawn on an approach, rotating through r/s/l lanes (PRD OQ-6).
    pub fn spawn_on_approach(
        &mut self,
        approach: Cardinal,
        model: &IntersectionModel,
    ) -> Option<VehicleId> {
        let route = self.route_for_approach(approach);
        let id = self.try_spawn(SpawnRequest::new(approach, route), model)?;
        self.advance_route_for_approach(approach);
        Some(id)
    }

    /// Advance movement along lane paths and remove vehicles that left the canvas.
    pub fn update(&mut self, model: &IntersectionModel, dt: f32) -> Vec<VehicleExit> {
        crate::vehicle::enforce_follow_distance(&mut self.vehicles, crate::config::SAFE_DISTANCE);
        let mut exited = Vec::new();

        for vehicle in &mut self.vehicles {
            if vehicle.state == VehicleState::Done {
                continue;
            }
            crate::vehicle::advance_along_path(vehicle, model, dt);
            if is_off_screen(vehicle.position) {
                if vehicle.state != VehicleState::Approaching {
                    exited.push(VehicleExit {
                        id: vehicle.id,
                        time_in_crossing: vehicle.time_in_crossing,
                    });
                }
                vehicle.state = VehicleState::Done;
            }
        }
        self.vehicles.retain(|v| v.state != VehicleState::Done);
        exited
    }

    /// Backdate all per-direction cooldowns so that the next `try_spawn` / `spawn_random`
    /// call on any approach is not gated by the wall-clock window.  Use this in tests
    /// whose frame loops run faster than the 400 ms cooldown period.
    pub fn force_cooldowns_expired(&mut self) {
        let past = std::time::Instant::now()
            - std::time::Duration::from_millis(crate::config::SPAWN_COOLDOWN_MS + 1);
        for approach in Cardinal::ALL {
            self.cooldown.record_at(approach, past);
        }
    }

    fn route_for_approach(&self, approach: Cardinal) -> Route {
        let count = self.route_counters.get(&approach).copied().unwrap_or(0);
        Route::ALL[count as usize % Route::ALL.len()]
    }

    fn advance_route_for_approach(&mut self, approach: Cardinal) {
        let count = self.route_counters.entry(approach).or_insert(0);
        *count = count.wrapping_add(1);
    }
}

impl Default for SpawnSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// True when `spawn_pos` is closer than safe distance to any same-lane vehicle.
fn same_lane_spawn_too_close(
    spawn_pos: Vec2,
    lane: &crate::intersection::LaneInfo,
    existing: &[Vehicle],
) -> bool {
    let heading = lane.approach.travel_heading();
    for vehicle in existing {
        if vehicle.lane_id != lane.id || vehicle.state == VehicleState::Done {
            continue;
        }
        let dx = vehicle.position.x - spawn_pos.x;
        let dy = vehicle.position.y - spawn_pos.y;
        let along = (dx * heading.cos() + dy * heading.sin()).abs();
        if along < crate::config::SAFE_DISTANCE {
            return true;
        }
    }
    false
}

/// Place a new vehicle behind the rearmost same-lane queue near the spawn point (AUD-8).
fn spawn_position_on_lane(lane: &crate::intersection::LaneInfo, existing: &[Vehicle]) -> Vec2 {
    let heading = lane.approach.travel_heading();
    let mut position = lane.spawn_point;
    let mut rearmost: Option<&Vehicle> = None;
    let mut smallest_along = f32::INFINITY;

    for vehicle in existing {
        if vehicle.lane_id != lane.id || vehicle.state == VehicleState::Done {
            continue;
        }
        let dx = vehicle.position.x - lane.spawn_point.x;
        let dy = vehicle.position.y - lane.spawn_point.y;
        let along = dx * heading.cos() + dy * heading.sin();
        if along < smallest_along {
            smallest_along = along;
            rearmost = Some(vehicle);
        }
    }

    if let Some(leader) = rearmost {
        if smallest_along < crate::config::SAFE_DISTANCE * 4.0 {
            position = Vec2::new(
                leader.position.x - crate::config::SAFE_DISTANCE * 2.0 * heading.cos(),
                leader.position.y - crate::config::SAFE_DISTANCE * 2.0 * heading.sin(),
            );
        }
    }

    position
}

/// True when the vehicle center is at or beyond the window margin.
fn is_off_screen(position: Vec2) -> bool {
    const MARGIN: f32 = 64.0;
    position.x <= -MARGIN
        || position.y <= -MARGIN
        || position.x >= crate::config::WINDOW_WIDTH as f32 + MARGIN
        || position.y >= crate::config::WINDOW_HEIGHT as f32 + MARGIN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::lane_id;

    #[test]
    fn update_does_not_emit_exit_for_approaching_vehicle_off_screen() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        spawn.try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model);

        spawn.vehicles_mut()[0].position = Vec2::new(-200.0, -200.0);
        assert_eq!(spawn.vehicles()[0].state, VehicleState::Approaching);

        let exited = spawn.update(&model, 0.0);

        assert!(
            exited.is_empty(),
            "vehicles that never enter the managed zone must not emit VehicleExit"
        );
        assert!(
            spawn.vehicles().is_empty(),
            "off-screen vehicle is still removed"
        );
    }

    #[test]
    fn update_emits_exit_for_managed_vehicle_off_screen() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        let id = spawn
            .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
            .expect("spawn succeeds");

        spawn.vehicles_mut()[0].state = VehicleState::Managed;
        spawn.vehicles_mut()[0].time_in_crossing = 1.5;
        spawn.vehicles_mut()[0].position = Vec2::new(-200.0, -200.0);

        let exited = spawn.update(&model, 0.0);

        assert_eq!(exited.len(), 1);
        assert_eq!(exited[0].id, id);
        assert_eq!(exited[0].time_in_crossing, 1.5);
    }

    #[test]
    fn spawn_request_carries_lane_id() {
        let req = SpawnRequest::new(Cardinal::South, Route::Straight);
        assert_eq!(req.approach, Cardinal::South);
        assert_eq!(req.route, Route::Straight);
        assert_eq!(req.lane_id, lane_id(Cardinal::South, Route::Straight));
    }

    #[test]
    fn try_spawn_places_vehicle_on_lane_spawn_point() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        let req = SpawnRequest::new(Cardinal::South, Route::Straight);
        let lane = model.lane(req.lane_id).unwrap().clone();

        let id = spawn.try_spawn(req, &model).expect("spawn should succeed");
        assert_eq!(spawn.vehicles().len(), 1);
        assert_eq!(spawn.vehicles()[0].id, id);
        assert_eq!(spawn.vehicles()[0].position, lane.spawn_point);
    }

    #[test]
    fn spawn_on_approach_rotates_routes() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        let expected = [Route::Right, Route::Straight, Route::Left, Route::Right];

        for &route in &expected {
            expire_cooldown(&mut spawn, Cardinal::West);
            spawn
                .spawn_on_approach(Cardinal::West, &model)
                .expect("spawn should succeed after cooldown");
            assert_eq!(spawn.vehicles().last().unwrap().route, route);
        }
    }

    fn expire_cooldown(spawn: &mut SpawnSystem, approach: Cardinal) {
        let expired = Instant::now() - Duration::from_millis(crate::config::SPAWN_COOLDOWN_MS);
        spawn.cooldown.record_at(approach, expired);
    }

    #[test]
    fn south_vehicle_moves_northward() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        spawn.try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model);
        let y0 = spawn.vehicles()[0].position.y;
        spawn.update(&IntersectionModel::new(), 1.0);
        assert!(spawn.vehicles()[0].position.y < y0);
    }

    #[test]
    fn north_vehicle_moves_southward() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        spawn.try_spawn(SpawnRequest::new(Cardinal::North, Route::Straight), &model);
        let y0 = spawn.vehicles()[0].position.y;
        spawn.update(&IntersectionModel::new(), 1.0);
        assert!(spawn.vehicles()[0].position.y > y0);
    }

    #[test]
    fn west_vehicle_moves_eastward() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        spawn.try_spawn(SpawnRequest::new(Cardinal::West, Route::Straight), &model);
        let x0 = spawn.vehicles()[0].position.x;
        spawn.update(&IntersectionModel::new(), 1.0);
        assert!(spawn.vehicles()[0].position.x > x0);
    }

    #[test]
    fn east_vehicle_moves_westward() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        spawn.try_spawn(SpawnRequest::new(Cardinal::East, Route::Straight), &model);
        let x0 = spawn.vehicles()[0].position.x;
        spawn.update(&IntersectionModel::new(), 1.0);
        assert!(spawn.vehicles()[0].position.x < x0);
    }

    #[test]
    fn cooldown_blocks_rapid_same_direction_spawns() {
        let mut cooldown = SpawnCooldown::new();
        let t0 = Instant::now();
        assert!(cooldown.allows_at(Cardinal::South, t0));
        cooldown.record_at(Cardinal::South, t0);
        assert!(!cooldown.allows_at(Cardinal::South, t0));
        let t1 = t0 + Duration::from_millis(crate::config::SPAWN_COOLDOWN_MS);
        assert!(cooldown.allows_at(Cardinal::South, t1));
    }

    #[test]
    fn cooldown_is_per_direction() {
        let mut cooldown = SpawnCooldown::new();
        let t0 = Instant::now();
        cooldown.record_at(Cardinal::South, t0);
        assert!(!cooldown.allows_at(Cardinal::South, t0));
        assert!(cooldown.allows_at(Cardinal::North, t0));
        assert!(cooldown.allows_at(Cardinal::West, t0));
        assert!(cooldown.allows_at(Cardinal::East, t0));
    }

    #[test]
    fn spawn_on_approach_does_not_advance_route_on_cooldown_reject() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();

        let id1 = spawn
            .spawn_on_approach(Cardinal::West, &model)
            .expect("first spawn should succeed");
        assert_eq!(
            spawn.vehicles().iter().find(|v| v.id == id1).unwrap().route,
            Route::Right
        );

        assert!(spawn.spawn_on_approach(Cardinal::West, &model).is_none());

        expire_cooldown(&mut spawn, Cardinal::West);
        let id2 = spawn
            .spawn_on_approach(Cardinal::West, &model)
            .expect("second spawn should succeed after cooldown");
        assert_eq!(
            spawn.vehicles().iter().find(|v| v.id == id2).unwrap().route,
            Route::Straight
        );
    }

    #[test]
    fn try_spawn_rejects_rapid_duplicate_on_same_approach() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        let req = SpawnRequest::new(Cardinal::South, Route::Straight);
        assert!(spawn.try_spawn(req, &model).is_some());
        assert!(spawn.try_spawn(req, &model).is_none());
        assert_eq!(spawn.vehicles().len(), 1);
    }

    #[test]
    fn try_spawn_allows_different_approaches_without_cooldown_gap() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        assert!(spawn
            .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
            .is_some());
        assert!(spawn
            .try_spawn(SpawnRequest::new(Cardinal::North, Route::Straight), &model)
            .is_some());
        assert_eq!(spawn.vehicles().len(), 2);
    }

    #[test]
    fn spawn_random_respects_per_direction_cooldown() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();

        let id1 = spawn.spawn_random(&model).expect("first random spawn");
        let approach = spawn
            .vehicles()
            .iter()
            .find(|v| v.id == id1)
            .expect("spawned vehicle")
            .approach;

        // Cooldown applies to the spawned approach only (not a global block).
        assert!(spawn
            .try_spawn(SpawnRequest::new(approach, Route::Straight), &model)
            .is_none());

        let other = Cardinal::ALL
            .iter()
            .copied()
            .find(|c| *c != approach)
            .expect("another approach exists");
        assert!(spawn
            .try_spawn(SpawnRequest::new(other, Route::Straight), &model)
            .is_some());
        assert_eq!(spawn.vehicles().len(), 2);

        expire_cooldown(&mut spawn, approach);
        assert!(spawn
            .try_spawn(SpawnRequest::new(approach, Route::Left), &model)
            .is_some());
        assert_eq!(spawn.vehicles().len(), 3);
    }

    #[test]
    fn spawn_random_produces_varied_approaches_and_routes() {
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();
        let mut approaches = std::collections::HashSet::new();
        let mut routes = std::collections::HashSet::new();

        for _ in 0..80 {
            if let Some(id) = spawn.spawn_random(&model) {
                let vehicle = spawn.vehicles().iter().find(|v| v.id == id).unwrap();
                approaches.insert(vehicle.approach);
                routes.insert(vehicle.route);
            }
            expire_all_cooldowns(&mut spawn);
        }

        assert_eq!(
            approaches.len(),
            Cardinal::ALL.len(),
            "all approaches appear"
        );
        assert_eq!(routes.len(), Route::ALL.len(), "all routes appear");
    }

    fn expire_all_cooldowns(spawn: &mut SpawnSystem) {
        for approach in Cardinal::ALL {
            expire_cooldown(spawn, approach);
        }
    }

    #[test]
    fn travel_heading_for_each_approach() {
        assert!(
            (Cardinal::South.travel_heading() + std::f32::consts::FRAC_PI_2).abs() < f32::EPSILON
        );
        assert!(
            (Cardinal::North.travel_heading() - std::f32::consts::FRAC_PI_2).abs() < f32::EPSILON
        );
        assert!(Cardinal::West.travel_heading().abs() < f32::EPSILON);
        assert!((Cardinal::East.travel_heading() - std::f32::consts::PI).abs() < f32::EPSILON);
    }
}
