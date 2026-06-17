//! Autonomous vehicle state and physics (B01+).

use crate::intersection::{
    IntersectionModel, LaneId, LaneInfo, Route, Vec2, VehicleRenderSnapshot,
};

/// Unique vehicle identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VehicleId(pub u64);

/// Lifecycle inside the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleState {
    Approaching,
    Managed,
    Exiting,
    Done,
}

/// Discrete speed levels assigned at spawn (B03; SDS §13.3; REQ-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VelocityLevel {
    Fast,
    Cruise,
    Yield,
}

impl VelocityLevel {
    /// Concrete speed in world units per second, derived from the default cruise speed.
    pub fn speed(self) -> f32 {
        match self {
            VelocityLevel::Fast => crate::config::DEFAULT_SPAWN_VELOCITY * 1.4,
            VelocityLevel::Cruise => crate::config::DEFAULT_SPAWN_VELOCITY * 1.0,
            VelocityLevel::Yield => crate::config::DEFAULT_SPAWN_VELOCITY * 0.5,
        }
    }
}

/// Vehicle simulation state (physics integration in B01).
#[derive(Debug)]
pub struct Vehicle {
    pub id: VehicleId,
    pub lane_id: LaneId,
    pub route: Route,
    pub approach: crate::intersection::Cardinal,
    pub position: Vec2,
    pub heading_rad: f32,
    pub velocity: f32,
    pub commanded_velocity: f32,
    pub state: VehicleState,
    pub path_index: usize,
    pub distance_in_crossing: f32,
    pub time_in_crossing: f32,
}

/// Create a vehicle at a lane spawn point (IF-1: B allocates id; A04 factory stub).
pub fn spawn_vehicle(id: VehicleId, lane: &LaneInfo, _velocity: f32) -> Vehicle {
    let level = match id.0 % 3 {
        0 => VelocityLevel::Fast,
        1 => VelocityLevel::Cruise,
        _ => VelocityLevel::Yield,
    };
    Vehicle {
        id,
        lane_id: lane.id,
        route: lane.route,
        approach: lane.approach,
        position: lane.spawn_point,
        heading_rad: lane.approach.travel_heading(),
        velocity: level.speed(),
        commanded_velocity: level.speed(),
        state: VehicleState::Approaching,
        path_index: 0,
        distance_in_crossing: 0.0,
        time_in_crossing: 0.0,
    }
}

/// Render-facing snapshot (A04 stub; B01 expands).
pub fn snapshot_for_render(vehicle: &Vehicle) -> VehicleRenderSnapshot {
    VehicleRenderSnapshot {
        position: vehicle.position,
        heading_rad: vehicle.heading_rad,
        approach: vehicle.approach,
    }
}

/// Update vehicle physics: advance position and accumulate crossing metrics (B01).
pub fn integrate_physics(vehicle: &mut Vehicle, dt: f32) {
    if vehicle.state == VehicleState::Done {
        return;
    }
    // commanded_velocity is the authoritative speed source (B03); copy into velocity so
    // both integrate_physics and advance_along_path use the commanded level each frame.
    vehicle.velocity = vehicle.commanded_velocity;

    let dx = vehicle.velocity * dt * vehicle.heading_rad.cos();
    let dy = vehicle.velocity * dt * vehicle.heading_rad.sin();
    let distance_moved = (dx * dx + dy * dy).sqrt();

    vehicle.position.x += dx;
    vehicle.position.y += dy;

    if vehicle.state == VehicleState::Managed || vehicle.state == VehicleState::Exiting {
        vehicle.time_in_crossing += dt;
        vehicle.distance_in_crossing += distance_moved;
    }
}

/// Move vehicle along its lane path polyline for this frame.
pub fn advance_along_path(vehicle: &mut Vehicle, model: &IntersectionModel, dt: f32) {
    let path = match model.lane(vehicle.lane_id) {
        Some(lane) if !lane.path.is_empty() => &lane.path,
        _ => return,
    };

    if vehicle.path_index >= path.len() - 1 {
        return;
    }

    let mut remaining = vehicle.velocity * dt;

    while remaining > 0.0 && vehicle.path_index < path.len() - 1 {
        let from = path[vehicle.path_index];
        let to = path[vehicle.path_index + 1];

        let seg_dx = to.x - from.x;
        let seg_dy = to.y - from.y;
        let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();

        if seg_len == 0.0 {
            vehicle.path_index += 1;
            continue;
        }

        vehicle.heading_rad = seg_dy.atan2(seg_dx);

        let to_end_dx = to.x - vehicle.position.x;
        let to_end_dy = to.y - vehicle.position.y;
        let dist_to_end = (to_end_dx * to_end_dx + to_end_dy * to_end_dy).sqrt();

        if remaining <= dist_to_end {
            vehicle.position.x += (seg_dx / seg_len) * remaining;
            vehicle.position.y += (seg_dy / seg_len) * remaining;
            remaining = 0.0;
        } else {
            remaining -= dist_to_end;
            vehicle.position = to;
            vehicle.path_index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::{attach_paths, Cardinal, IntersectionModel, LaneId, Route};
    use crate::spawn::{SpawnRequest, SpawnSystem};
    use std::collections::HashMap;

    fn make_vehicle(id_val: u64, commanded_velocity: f32) -> Vehicle {
        Vehicle {
            id: VehicleId(id_val),
            lane_id: LaneId(0),
            route: Route::Straight,
            approach: crate::intersection::Cardinal::South,
            position: Vec2 { x: 0.0, y: 0.0 },
            heading_rad: 0.0,
            velocity: 0.0,
            commanded_velocity,
            state: VehicleState::Approaching,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
        }
    }

    #[test]
    fn b03_spawned_vehicles_have_three_distinct_commanded_velocities() {
        // Test (a) — B03: spawn 3 vehicles through the real SpawnSystem path on different
        // approaches (no cooldown conflict), then assert the set of distinct commanded_velocity
        // values has size >= 3.  A test that only checks the constants array is distinct would
        // pass trivially without exercising the spawn path at all.
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();

        spawn.try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model);
        spawn.try_spawn(SpawnRequest::new(Cardinal::North, Route::Straight), &model);
        spawn.try_spawn(SpawnRequest::new(Cardinal::West, Route::Straight), &model);

        assert_eq!(spawn.vehicles().len(), 3, "all 3 spawns must succeed");

        // Collect distinct commanded_velocity values (f32 bits for HashSet membership).
        let distinct: std::collections::HashSet<u32> = spawn
            .vehicles()
            .iter()
            .map(|v| v.commanded_velocity.to_bits())
            .collect();

        assert!(
            distinct.len() >= 3,
            "expected >= 3 distinct commanded_velocity values but got {}: {:?}",
            distinct.len(),
            spawn
                .vehicles()
                .iter()
                .map(|v| v.commanded_velocity)
                .collect::<Vec<_>>(),
        );

        // Also confirm the values match the published VelocityLevel speeds.
        let expected: std::collections::HashSet<u32> = [
            VelocityLevel::Fast.speed(),
            VelocityLevel::Cruise.speed(),
            VelocityLevel::Yield.speed(),
        ]
        .iter()
        .map(|f| f.to_bits())
        .collect();

        assert_eq!(
            distinct, expected,
            "commanded_velocity values must match VelocityLevel::speed() for all three levels",
        );
    }

    #[test]
    fn faster_commanded_velocity_drives_strictly_greater_distance() {
        // Test (b) — B03: proves commanded_velocity is wired into motion, not just stored.
        // The reconcile line `velocity = commanded_velocity` in integrate_physics must fire,
        // causing the Fast vehicle to cover more ground than the Yield vehicle in equal time.
        let mut yield_v = make_vehicle(1, VelocityLevel::Yield.speed());
        let mut fast_v = make_vehicle(2, VelocityLevel::Fast.speed());

        let dt = 1.0_f32;
        integrate_physics(&mut yield_v, dt);
        integrate_physics(&mut fast_v, dt);

        // heading_rad = 0.0 → motion is purely along +x
        assert!(
            fast_v.position.x > yield_v.position.x,
            "Fast vehicle (x={}) must travel farther than Yield vehicle (x={}) for equal dt; \
             commanded_velocity is not wired into motion if this fails",
            fast_v.position.x,
            yield_v.position.x,
        );
    }

    #[test]
    fn integrate_physics_does_not_accumulate_crossing_metrics_when_approaching() {
        let mut vehicle = Vehicle {
            id: VehicleId(1),
            lane_id: crate::intersection::LaneId(0),
            route: crate::intersection::Route::Straight,
            approach: crate::intersection::Cardinal::South,
            position: Vec2 { x: 100.0, y: 100.0 },
            heading_rad: 0.0,
            velocity: 50.0,
            commanded_velocity: 50.0,
            state: VehicleState::Approaching,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
        };

        integrate_physics(&mut vehicle, 0.1);

        assert_eq!(
            vehicle.distance_in_crossing, 0.0,
            "distance should not accumulate when Approaching"
        );
        assert_eq!(
            vehicle.time_in_crossing, 0.0,
            "time should not accumulate when Approaching"
        );
        assert!(
            vehicle.position.x > 100.0,
            "position should change despite no metric accumulation"
        );
    }

    #[test]
    fn integrate_physics_accumulates_crossing_metrics_when_managed() {
        let mut vehicle = Vehicle {
            id: VehicleId(2),
            lane_id: crate::intersection::LaneId(0),
            route: crate::intersection::Route::Straight,
            approach: crate::intersection::Cardinal::South,
            position: Vec2 { x: 200.0, y: 200.0 },
            heading_rad: 0.0,
            velocity: 50.0,
            commanded_velocity: 50.0,
            state: VehicleState::Managed,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
        };

        integrate_physics(&mut vehicle, 0.1);

        assert!(
            vehicle.distance_in_crossing > 0.0,
            "distance should accumulate when Managed"
        );
        assert_eq!(
            vehicle.time_in_crossing, 0.1,
            "time should accumulate by dt when Managed"
        );
    }

    #[test]
    fn integrate_physics_accumulates_crossing_metrics_when_exiting() {
        let mut vehicle = Vehicle {
            id: VehicleId(3),
            lane_id: crate::intersection::LaneId(0),
            route: crate::intersection::Route::Straight,
            approach: crate::intersection::Cardinal::South,
            position: Vec2 { x: 300.0, y: 300.0 },
            heading_rad: 0.0,
            velocity: 50.0,
            commanded_velocity: 50.0,
            state: VehicleState::Exiting,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
        };

        integrate_physics(&mut vehicle, 0.1);

        assert!(
            vehicle.distance_in_crossing > 0.0,
            "distance should accumulate when Exiting"
        );
        assert_eq!(
            vehicle.time_in_crossing, 0.1,
            "time should accumulate by dt when Exiting"
        );
    }

    #[test]
    fn advance_along_path_follows_waypoints_and_updates_heading() {
        let mut model = IntersectionModel::new();
        let lane_id = model.lanes[0].id;

        let paths = HashMap::from([(lane_id, vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)])]);
        attach_paths(&mut model, paths);

        let mut vehicle = Vehicle {
            id: VehicleId(1),
            lane_id,
            route: model.lanes[0].route,
            approach: model.lanes[0].approach,
            position: Vec2::new(0.0, 0.0),
            heading_rad: 0.0,
            velocity: 50.0,
            commanded_velocity: 50.0,
            state: VehicleState::Approaching,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
        };

        advance_along_path(&mut vehicle, &model, 1.0);

        assert!(vehicle.position.x > 0.0);
        assert!(vehicle.position.x <= 100.0);
        assert_eq!(vehicle.position.y, 0.0);
        assert_eq!(vehicle.heading_rad, 0.0);
    }
}
