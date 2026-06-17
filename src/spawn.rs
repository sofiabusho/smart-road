//! Keyboard-driven vehicle spawning (A04+).

use std::collections::HashMap;

use crate::intersection::{Cardinal, IntersectionModel, LaneId, Route, Vec2};
use crate::vehicle::{spawn_vehicle, Vehicle, VehicleId, VehicleState};

/// Per-direction spawn throttle (A05 implements real cooldown).
#[derive(Debug, Default)]
pub struct SpawnCooldown {
    // A05: per_direction_ms, last_spawn timestamps
}

impl SpawnCooldown {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a spawn on this approach is allowed (always true until A05).
    pub fn allows(&mut self, _approach: Cardinal) -> bool {
        true
    }

    /// Record a successful spawn (no-op until A05).
    pub fn record(&mut self, _approach: Cardinal) {}
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

/// Spawn pipeline: keyboard requests → vehicles (A04).
#[derive(Debug)]
pub struct SpawnSystem {
    vehicles: Vec<Vehicle>,
    next_id: u64,
    route_counters: HashMap<Cardinal, u8>,
    cooldown: SpawnCooldown,
}

impl SpawnSystem {
    pub fn new() -> Self {
        Self {
            vehicles: Vec::new(),
            next_id: 1,
            route_counters: HashMap::new(),
            cooldown: SpawnCooldown::new(),
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

    /// Attempt to spawn a vehicle from a request. Returns `None` if cooldown rejects.
    pub fn try_spawn(&mut self, req: SpawnRequest, model: &IntersectionModel) -> Option<VehicleId> {
        if !self.cooldown.allows(req.approach) {
            return None;
        }

        let lane = model.lane(req.lane_id)?;
        let id = VehicleId(self.next_id);
        self.next_id += 1;

        let vehicle = spawn_vehicle(id, lane, crate::config::DEFAULT_SPAWN_VELOCITY);
        self.vehicles.push(vehicle);
        self.cooldown.record(req.approach);
        Some(id)
    }

    /// Spawn on an approach, rotating through r/s/l lanes (PRD OQ-6).
    pub fn spawn_on_approach(
        &mut self,
        approach: Cardinal,
        model: &IntersectionModel,
    ) -> Option<VehicleId> {
        let route = self.next_route_for_approach(approach);
        self.try_spawn(SpawnRequest::new(approach, route), model)
    }

    /// Advance movement along lane paths and remove vehicles that left the canvas.
    pub fn update(&mut self, model: &IntersectionModel, dt: f32) {
        for vehicle in &mut self.vehicles {
            if vehicle.state == VehicleState::Done {
                continue;
            }
            crate::vehicle::integrate_physics(vehicle, dt);
            crate::vehicle::advance_along_path(vehicle, model, dt);
            if is_off_screen(vehicle.position) {
                vehicle.state = VehicleState::Done;
            }
        }
        self.vehicles.retain(|v| v.state != VehicleState::Done);
    }

    fn next_route_for_approach(&mut self, approach: Cardinal) -> Route {
        let count = self.route_counters.entry(approach).or_insert(0);
        let route = Route::ALL[*count as usize % Route::ALL.len()];
        *count = count.wrapping_add(1);
        route
    }
}

impl Default for SpawnSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// True when the vehicle center is well outside the window bounds.
fn is_off_screen(position: Vec2) -> bool {
    const MARGIN: f32 = 64.0;
    position.x < -MARGIN
        || position.y < -MARGIN
        || position.x > crate::config::WINDOW_WIDTH as f32 + MARGIN
        || position.y > crate::config::WINDOW_HEIGHT as f32 + MARGIN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::lane_id;

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

        spawn.spawn_on_approach(Cardinal::West, &model);
        spawn.spawn_on_approach(Cardinal::West, &model);
        spawn.spawn_on_approach(Cardinal::West, &model);
        spawn.spawn_on_approach(Cardinal::West, &model);

        let routes: Vec<_> = spawn.vehicles().iter().map(|v| v.route).collect();
        assert_eq!(routes[0], Route::Right);
        assert_eq!(routes[1], Route::Straight);
        assert_eq!(routes[2], Route::Left);
        assert_eq!(routes[3], Route::Right);
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
