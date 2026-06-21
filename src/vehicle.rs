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
    /// Spawn-assigned speed level (B03); restored when follow-distance gap clears (B04).
    pub nominal_velocity: f32,
    pub state: VehicleState,
    pub path_index: usize,
    pub distance_in_crossing: f32,
    pub time_in_crossing: f32,
}

/// Per-vehicle acceleration and deceleration scale (REQ-B3 / AUD-B3).
pub fn motion_profile(id: VehicleId) -> (f32, f32) {
    match id.0 % 3 {
        0 => (1.4, 1.2),
        1 => (1.0, 1.0),
        _ => (0.7, 0.6),
    }
}

/// Ramp `velocity` toward `commanded_velocity` with bounded accel/decel (B05).
pub fn step_velocity_toward_command(vehicle: &mut Vehicle, dt: f32) {
    let target = vehicle.commanded_velocity;
    let current = vehicle.velocity;
    if (target - current).abs() < 0.01 {
        vehicle.velocity = target;
        return;
    }

    let (accel_scale, decel_scale) = motion_profile(vehicle.id);
    let max_change = if target > current {
        crate::config::BASE_ACCELERATION * accel_scale * dt
    } else {
        crate::config::BASE_DECELERATION * decel_scale * dt
    };
    let delta = (target - current).clamp(-max_change, max_change);
    vehicle.velocity = current + delta;
}

/// Create a vehicle at a lane spawn point (IF-1: B allocates id; A04 factory stub).
pub fn spawn_vehicle(id: VehicleId, lane: &LaneInfo, _velocity: f32) -> Vehicle {
    let level = match id.0 % 3 {
        0 => VelocityLevel::Fast,
        1 => VelocityLevel::Cruise,
        _ => VelocityLevel::Yield,
    };
    let speed = level.speed();
    Vehicle {
        id,
        lane_id: lane.id,
        route: lane.route,
        approach: lane.approach,
        position: lane.spawn_point,
        heading_rad: lane.approach.travel_heading(),
        velocity: speed,
        commanded_velocity: speed,
        nominal_velocity: speed,
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
///
/// Used in unit tests and when no lane path drives movement. The live simulation
/// uses [`advance_along_path`] as the single position authority (B04 / DEF-01).
pub fn integrate_physics(vehicle: &mut Vehicle, dt: f32) {
    if vehicle.state == VehicleState::Done {
        return;
    }
    // B05: ramp actual speed toward commanded speed before integrating motion.
    step_velocity_toward_command(vehicle, dt);

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

/// Longitudinal distance from `follower` to `leader` when `leader` is ahead on the same lane.
fn longitudinal_gap(follower: &Vehicle, leader: &Vehicle) -> Option<f32> {
    if follower.lane_id != leader.lane_id {
        return None;
    }

    let dx = leader.position.x - follower.position.x;
    let dy = leader.position.y - follower.position.y;
    let along = dx * follower.heading_rad.cos() + dy * follower.heading_rad.sin();

    if along > 0.0 {
        Some(along)
    } else {
        None
    }
}

/// Whether `vehicle` participates in B-side follow-distance logic (SDS §13.3).
fn uses_follow_distance(vehicle: &Vehicle) -> bool {
    matches!(
        vehicle.state,
        VehicleState::Approaching | VehicleState::Exiting
    )
}

/// Cap speeds so same-lane followers keep separation (REQ-8 / REQ-9 / AUD-30).
///
/// Skips vehicles in `Managed` state — the smart controller owns the junction zone (C02+).
pub fn enforce_follow_distance(vehicles: &mut [Vehicle], safe_distance: f32) {
    let len = vehicles.len();

    for i in 0..len {
        if !uses_follow_distance(&vehicles[i]) {
            continue;
        }

        let nominal = vehicles[i].nominal_velocity;
        let mut gap_ahead = f32::INFINITY;
        let mut leader_velocity = 0.0_f32;

        for j in 0..len {
            if i == j || vehicles[j].state == VehicleState::Done {
                continue;
            }
            if vehicles[i].lane_id != vehicles[j].lane_id {
                continue;
            }

            if let Some(gap) = longitudinal_gap(&vehicles[i], &vehicles[j]) {
                if gap < gap_ahead {
                    gap_ahead = gap;
                    leader_velocity = vehicles[j].velocity;
                }
            }
        }

        if gap_ahead >= safe_distance {
            vehicles[i].commanded_velocity = nominal;
            continue;
        }

        let scale = (gap_ahead / safe_distance).clamp(0.0, 1.0);
        let target = if gap_ahead <= safe_distance * 0.1 {
            0.0
        } else {
            leader_velocity.min(nominal * scale)
        };

        vehicles[i].commanded_velocity = target;
    }
}

/// True when two same-lane vehicles are within safe distance (REQ-26 scaffolding for C05).
///
/// Uses center-to-center Euclidean distance. [`enforce_follow_distance`] uses
/// longitudinal gap along heading — C05 should prefer longitudinal checks for close calls.
pub fn detect_close_call(a: &Vehicle, b: &Vehicle, safe_distance: f32) -> bool {
    if a.lane_id != b.lane_id || a.id == b.id {
        return false;
    }
    if a.state == VehicleState::Done || b.state == VehicleState::Done {
        return false;
    }

    let dx = b.position.x - a.position.x;
    let dy = b.position.y - a.position.y;
    let dist_sq = dx * dx + dy * dy;
    dist_sq > 0.0 && dist_sq < safe_distance * safe_distance
}

/// Move vehicle along its lane path polyline for this frame.
pub fn advance_along_path(vehicle: &mut Vehicle, model: &IntersectionModel, dt: f32) {
    step_velocity_toward_command(vehicle, dt);
    let start = vehicle.position;
    let track_crossing =
        vehicle.state == VehicleState::Managed || vehicle.state == VehicleState::Exiting;

    let path = match model.lane(vehicle.lane_id) {
        Some(lane) if !lane.path.is_empty() => &lane.path,
        _ => {
            if track_crossing {
                vehicle.time_in_crossing += dt;
            }
            return;
        }
    };

    if vehicle.path_index >= path.len() - 1 {
        if track_crossing {
            vehicle.time_in_crossing += dt;
        }
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

    let moved_dx = vehicle.position.x - start.x;
    let moved_dy = vehicle.position.y - start.y;
    let distance_moved = (moved_dx * moved_dx + moved_dy * moved_dy).sqrt();

    if vehicle.state == VehicleState::Managed || vehicle.state == VehicleState::Exiting {
        vehicle.time_in_crossing += dt;
        vehicle.distance_in_crossing += distance_moved;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::{attach_paths, lane_id, Cardinal, IntersectionModel, LaneId, Route};
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
            nominal_velocity: commanded_velocity,
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
            nominal_velocity: 50.0,
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
            nominal_velocity: 50.0,
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
            nominal_velocity: 50.0,
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
            nominal_velocity: 50.0,
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

    fn south_straight_lane_vehicle(id: u64, y: f32, speed: f32) -> Vehicle {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();
        let mut vehicle = spawn_vehicle(VehicleId(id), lane, speed);
        vehicle.commanded_velocity = speed;
        vehicle.velocity = speed;
        vehicle.position.y = y;
        vehicle.position.x = lane.spawn_point.x;
        vehicle
    }

    #[test]
    fn detect_close_call_flags_same_lane_violation() {
        let a = south_straight_lane_vehicle(1, 500.0, 120.0);
        let mut b = south_straight_lane_vehicle(2, 520.0, 120.0);
        b.position.y = a.position.y + crate::config::SAFE_DISTANCE * 0.5;

        assert!(detect_close_call(&a, &b, crate::config::SAFE_DISTANCE));
    }

    #[test]
    fn detect_close_call_ignores_different_lanes() {
        let model = IntersectionModel::new();
        let south = model
            .lane(lane_id(Cardinal::South, Route::Straight))
            .unwrap();
        let north = model
            .lane(lane_id(Cardinal::North, Route::Straight))
            .unwrap();
        let a = spawn_vehicle(VehicleId(1), south, 120.0);
        let b = spawn_vehicle(VehicleId(2), north, 120.0);

        assert!(!detect_close_call(&a, &b, crate::config::SAFE_DISTANCE));
    }

    #[test]
    fn enforce_follow_distance_slows_follower_behind_stopped_leader() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();

        let mut leader = spawn_vehicle(VehicleId(1), lane, VelocityLevel::Fast.speed());
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);
        leader.commanded_velocity = 0.0;
        leader.velocity = 0.0;

        let mut follower = spawn_vehicle(VehicleId(2), lane, VelocityLevel::Fast.speed());
        follower.position = Vec2::new(
            lane.spawn_point.x,
            leader.position.y + crate::config::SAFE_DISTANCE * 2.0,
        );
        let fast_speed = VelocityLevel::Fast.speed();
        let mut saw_slowdown = false;

        let mut vehicles = vec![leader, follower];
        for _ in 0..300 {
            enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);
            for vehicle in &mut vehicles {
                advance_along_path(vehicle, &model, crate::config::FIXED_TIMESTEP_SECS);
            }

            let gap = vehicles[1].position.y - vehicles[0].position.y;
            assert!(
                gap >= crate::config::SAFE_DISTANCE * 0.9,
                "follower must not close inside safe distance (gap={gap})"
            );
            if vehicles[1].velocity < fast_speed {
                saw_slowdown = true;
            }
        }

        assert!(
            saw_slowdown,
            "follower should slow while approaching a stopped leader"
        );
    }

    #[test]
    fn enforce_follow_distance_does_not_adjust_managed_follower() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();

        let mut leader = spawn_vehicle(VehicleId(1), lane, VelocityLevel::Cruise.speed());
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);
        leader.commanded_velocity = 0.0;
        leader.velocity = 0.0;

        let mut managed = spawn_vehicle(VehicleId(2), lane, VelocityLevel::Fast.speed());
        managed.state = VehicleState::Managed;
        managed.position = Vec2::new(
            lane.spawn_point.x,
            leader.position.y + crate::config::SAFE_DISTANCE * 0.5,
        );
        let mut vehicles = vec![leader, managed];
        let initial_speed = vehicles[1].commanded_velocity;

        enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);

        assert_eq!(
            vehicles[1].commanded_velocity, initial_speed,
            "Managed vehicles defer to smart controller, not B04 follow logic"
        );
    }

    #[test]
    fn enforce_follow_distance_steady_follow_behind_moving_leader_no_ratcheting() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();
        let nominal = VelocityLevel::Fast.speed();
        let leader_speed = 60.0_f32;
        let gap = crate::config::SAFE_DISTANCE * 0.5;

        let mut leader = spawn_vehicle(VehicleId(1), lane, leader_speed);
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);
        leader.velocity = leader_speed;
        leader.commanded_velocity = leader_speed;

        let mut follower = spawn_vehicle(VehicleId(2), lane, nominal);
        follower.position = Vec2::new(lane.spawn_point.x, leader.position.y + gap);
        follower.velocity = nominal;
        follower.commanded_velocity = nominal;

        let mut vehicles = vec![leader, follower];
        enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);
        let steady = vehicles[1].commanded_velocity;

        assert!(
            steady > 0.0 && steady < nominal,
            "follower should slow but not stop behind moving leader (steady={steady})"
        );

        for _ in 0..100 {
            enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);
            assert_eq!(
                vehicles[1].commanded_velocity, steady,
                "velocity must not ratchet frame-over-frame (steady={steady}, now={})",
                vehicles[1].commanded_velocity
            );
        }
    }

    #[test]
    fn enforce_follow_distance_restores_nominal_when_gap_safe() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();
        let nominal = VelocityLevel::Fast.speed();

        let mut leader = spawn_vehicle(VehicleId(1), lane, nominal);
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);

        let mut follower = spawn_vehicle(VehicleId(2), lane, nominal);
        follower.nominal_velocity = nominal;
        follower.position = Vec2::new(
            lane.spawn_point.x,
            leader.position.y + crate::config::SAFE_DISTANCE * 2.0,
        );
        follower.commanded_velocity = 10.0;
        follower.velocity = 10.0;

        let mut vehicles = vec![leader, follower];
        enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);

        assert_eq!(
            vehicles[1].commanded_velocity, nominal,
            "follower should restore nominal speed when gap is safe"
        );
        let before = vehicles[1].velocity;
        for _ in 0..120 {
            step_velocity_toward_command(&mut vehicles[1], crate::config::FIXED_TIMESTEP_SECS);
        }
        assert!(
            (vehicles[1].velocity - nominal).abs() < 1.0,
            "velocity should ramp up to nominal after gap clears (got {})",
            vehicles[1].velocity
        );
        assert!(
            vehicles[1].velocity > before,
            "velocity should increase gradually"
        );
    }

    #[test]
    fn velocity_decelerates_gradually_not_instantly() {
        let mut vehicle = make_vehicle(1, VelocityLevel::Fast.speed());
        vehicle.velocity = vehicle.commanded_velocity;
        vehicle.commanded_velocity = 0.0;

        let before = vehicle.velocity;
        step_velocity_toward_command(&mut vehicle, crate::config::FIXED_TIMESTEP_SECS);

        assert!(vehicle.velocity > 0.0, "one frame should not snap to zero");
        assert!(
            vehicle.velocity < before,
            "velocity should decrease toward command"
        );
    }

    #[test]
    fn different_vehicles_use_different_deceleration_rates() {
        let dt = crate::config::FIXED_TIMESTEP_SECS;
        let start = VelocityLevel::Fast.speed();

        let mut quicker = make_vehicle(1, start);
        quicker.velocity = start;
        quicker.commanded_velocity = 0.0;

        let mut slower = make_vehicle(2, start);
        slower.velocity = start;
        slower.commanded_velocity = 0.0;

        step_velocity_toward_command(&mut quicker, dt);
        step_velocity_toward_command(&mut slower, dt);

        assert!(
            quicker.velocity < slower.velocity,
            "VehicleId(1) should decelerate faster than VehicleId(2) (AUD-B3)"
        );
    }

    #[test]
    fn motion_profile_exposes_three_distinct_deceleration_scales() {
        let scales: std::collections::HashSet<u32> = (0..12)
            .map(|id| {
                let (_, decel) = motion_profile(VehicleId(id));
                (decel * 1000.0) as u32
            })
            .collect();
        assert_eq!(scales.len(), 3, "expected three distinct decel profiles");
    }

    #[test]
    fn advance_along_path_accumulates_time_at_path_terminal() {
        let mut model = IntersectionModel::new();
        let lane_id_val = model.lanes[0].id;
        let paths = HashMap::from([(
            lane_id_val,
            vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)],
        )]);
        attach_paths(&mut model, paths);

        let mut vehicle = Vehicle {
            id: VehicleId(1),
            lane_id: lane_id_val,
            route: model.lanes[0].route,
            approach: model.lanes[0].approach,
            position: Vec2::new(100.0, 0.0),
            heading_rad: 0.0,
            velocity: 0.0,
            commanded_velocity: 0.0,
            nominal_velocity: 50.0,
            state: VehicleState::Managed,
            path_index: 1,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
        };

        advance_along_path(&mut vehicle, &model, 0.25);

        assert_eq!(
            vehicle.time_in_crossing, 0.25,
            "time should accumulate at path terminal for Managed vehicles"
        );
        assert_eq!(
            vehicle.distance_in_crossing, 0.0,
            "no distance moved when already at terminal waypoint"
        );
    }
}
