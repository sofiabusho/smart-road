//! Autonomous vehicle state and physics (B01+).

use crate::intersection::{LaneId, Route};

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
    pub velocity: f32,
    pub state: VehicleState,
}
