//! Autonomous vehicle state and physics (B01+).

use crate::intersection::{LaneId, LaneInfo, Route, Vec2, VehicleRenderSnapshot};

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
pub fn spawn_vehicle(id: VehicleId, lane: &LaneInfo, velocity: f32) -> Vehicle {
    Vehicle {
        id,
        lane_id: lane.id,
        route: lane.route,
        approach: lane.approach,
        position: lane.spawn_point,
        heading_rad: lane.approach.travel_heading(),
        velocity,
        commanded_velocity: velocity,
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

    let dx = vehicle.velocity * dt * vehicle.heading_rad.cos();
    let dy = vehicle.velocity * dt * vehicle.heading_rad.sin();
    let distance_moved = (dx * dx + dy * dy).sqrt();

    vehicle.position.x += dx;
    vehicle.position.y += dy;

    if vehicle.state == VehicleState::Managed {
        vehicle.time_in_crossing += dt;
        vehicle.distance_in_crossing += distance_moved;
    }
}
