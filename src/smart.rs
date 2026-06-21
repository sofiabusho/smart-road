//! Smart intersection controller (C01/C02).

use std::collections::{HashMap, HashSet};

use crate::config::SAFE_DISTANCE;
use crate::intersection::{IntersectionModel, LaneId, Vec2};
use crate::vehicle::{Vehicle, VehicleState, VelocityLevel};

/// Coordinates AV passage without traffic lights (C01 detection, C02 scheduling).
#[derive(Debug)]
pub struct SmartController {
    lane_conflicts: HashSet<(u32, u32)>,
    conflicts_ready: bool,
    next_entry_seq: u32,
    entry_sequence: HashMap<crate::vehicle::VehicleId, u32>,
}

impl Default for SmartController {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartController {
    pub fn new() -> Self {
        Self {
            lane_conflicts: HashSet::new(),
            conflicts_ready: false,
            next_entry_seq: 0,
            entry_sequence: HashMap::new(),
        }
    }

    /// Transition a vehicle into managed state when it enters the intersection zone (REQ-4).
    pub fn on_vehicle_enter_zone(&mut self, vehicle: &mut Vehicle) {
        if vehicle.state != VehicleState::Approaching {
            return;
        }
        vehicle.state = VehicleState::Managed;
        vehicle.time_in_crossing = 0.0;
        vehicle.distance_in_crossing = 0.0;
        vehicle.commanded_velocity = vehicle.nominal_velocity;
        vehicle.velocity = vehicle.nominal_velocity;
        self.entry_sequence.insert(vehicle.id, self.next_entry_seq);
        self.next_entry_seq += 1;
    }

    /// Detect zone entry/exit, schedule managed velocities, update lifecycle (REQ-3, REQ-4).
    pub fn update(&mut self, vehicles: &mut [Vehicle], model: &IntersectionModel, _dt: f32) {
        self.ensure_lane_conflicts(model);

        for vehicle in vehicles.iter_mut() {
            if vehicle.state == VehicleState::Done {
                continue;
            }

            let in_zone = point_in_polygon(vehicle.position, &model.zone_polygon);

            match vehicle.state {
                VehicleState::Approaching if in_zone => self.on_vehicle_enter_zone(vehicle),
                VehicleState::Managed if !in_zone => vehicle.state = VehicleState::Exiting,
                _ => {}
            }
        }

        self.schedule_managed_velocities(vehicles);
    }

    /// Command velocities for vehicles inside the managed zone (C02 / REQ-3, REQ-9).
    fn schedule_managed_velocities(&self, vehicles: &mut [Vehicle]) {
        let yield_speed = VelocityLevel::Yield.speed();

        for vehicle in vehicles.iter_mut() {
            if vehicle.state == VehicleState::Managed {
                vehicle.commanded_velocity = vehicle.nominal_velocity;
                vehicle.velocity = vehicle.nominal_velocity;
            }
        }

        let len = vehicles.len();
        for i in 0..len {
            if vehicles[i].state != VehicleState::Managed {
                continue;
            }
            for j in (i + 1)..len {
                if vehicles[j].state != VehicleState::Managed {
                    continue;
                }

                let pair = lane_pair_key(vehicles[i].lane_id, vehicles[j].lane_id);
                let same_lane = vehicles[i].lane_id == vehicles[j].lane_id;
                if !same_lane && !self.lane_conflicts.contains(&pair) {
                    continue;
                }

                let gap = center_distance(&vehicles[i], &vehicles[j]);
                if gap >= SAFE_DISTANCE * 1.5 {
                    continue;
                }

                let seq_i = self.entry_sequence.get(&vehicles[i].id).copied();
                let seq_j = self.entry_sequence.get(&vehicles[j].id).copied();
                let (yielder, leader_seq, follower_seq) = match (seq_i, seq_j) {
                    (Some(a), Some(b)) if a > b => (i, b, a),
                    (Some(a), Some(b)) => (j, a, b),
                    _ => continue,
                };

                let _ = (leader_seq, follower_seq);
                let target = if gap <= SAFE_DISTANCE {
                    0.0
                } else {
                    yield_speed
                };
                vehicles[yielder].commanded_velocity =
                    vehicles[yielder].commanded_velocity.min(target);
                vehicles[yielder].velocity = vehicles[yielder].commanded_velocity;
            }
        }
    }

    fn ensure_lane_conflicts(&mut self, model: &IntersectionModel) {
        if self.conflicts_ready {
            return;
        }
        self.lane_conflicts = build_lane_conflicts(model);
        self.conflicts_ready = true;
    }
}

fn lane_pair_key(a: LaneId, b: LaneId) -> (u32, u32) {
    (a.0.min(b.0), a.0.max(b.0))
}

fn center_distance(a: &Vehicle, b: &Vehicle) -> f32 {
    let dx = a.position.x - b.position.x;
    let dy = a.position.y - b.position.y;
    (dx * dx + dy * dy).sqrt()
}

fn build_lane_conflicts(model: &IntersectionModel) -> HashSet<(u32, u32)> {
    let mut conflicts = HashSet::new();
    let lanes = &model.lanes;
    let zone = &model.zone_polygon;

    for i in 0..lanes.len() {
        for j in (i + 1)..lanes.len() {
            if paths_conflict(&lanes[i].path, &lanes[j].path, zone) {
                conflicts.insert(lane_pair_key(lanes[i].id, lanes[j].id));
            }
        }
    }

    conflicts
}

fn paths_conflict(a: &[Vec2], b: &[Vec2], zone: &[Vec2]) -> bool {
    if a.len() < 2 || b.len() < 2 {
        return false;
    }

    for ai in 0..a.len() - 1 {
        for bj in 0..b.len() - 1 {
            if let Some(point) = segment_intersection(a[ai], a[ai + 1], b[bj], b[bj + 1]) {
                if point_in_polygon(point, zone) {
                    return true;
                }
            }
        }
    }

    false
}

fn segment_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = p4.x - p3.x;
    let d2y = p4.y - p3.y;
    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < f32::EPSILON {
        return None;
    }

    let t = ((p3.x - p1.x) * d2y - (p3.y - p1.y) * d2x) / denom;
    let u = ((p3.x - p1.x) * d1y - (p3.y - p1.y) * d1x) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(Vec2::new(p1.x + t * d1x, p1.y + t * d1y))
    } else {
        None
    }
}

/// Ray-casting point-in-polygon test for the axis-aligned junction zone.
fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = polygon.len() - 1;

    for i in 0..polygon.len() {
        let pi = polygon[i];
        let pj = polygon[j];

        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }

    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::{lane_id, Cardinal, Route};
    use crate::vehicle::VehicleId;

    fn test_vehicle_at(position: Vec2, state: VehicleState) -> Vehicle {
        Vehicle {
            id: VehicleId(1),
            lane_id: LaneId(0),
            route: Route::Straight,
            approach: Cardinal::South,
            position,
            heading_rad: 0.0,
            velocity: 120.0,
            commanded_velocity: 120.0,
            nominal_velocity: 120.0,
            state,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
        }
    }

    fn managed_vehicle(
        id: u64,
        lane: LaneId,
        approach: Cardinal,
        route: Route,
        position: Vec2,
        nominal: f32,
    ) -> Vehicle {
        Vehicle {
            id: VehicleId(id),
            lane_id: lane,
            route,
            approach,
            position,
            heading_rad: approach.travel_heading(),
            velocity: nominal,
            commanded_velocity: nominal,
            nominal_velocity: nominal,
            state: VehicleState::Managed,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
        }
    }

    #[test]
    fn point_in_polygon_detects_junction_center() {
        let model = IntersectionModel::new();
        let center = Vec2::new(
            crate::config::INTERSECTION_CENTER_X,
            crate::config::INTERSECTION_CENTER_Y,
        );
        assert!(point_in_polygon(center, &model.zone_polygon));
    }

    #[test]
    fn point_in_polygon_rejects_far_off_screen_point() {
        let model = IntersectionModel::new();
        assert!(!point_in_polygon(
            Vec2::new(-100.0, -100.0),
            &model.zone_polygon
        ));
    }

    #[test]
    fn build_lane_conflicts_marks_crossing_approaches() {
        let model = IntersectionModel::new();
        let conflicts = build_lane_conflicts(&model);
        let south_straight = lane_id(Cardinal::South, Route::Straight);
        let east_straight = lane_id(Cardinal::East, Route::Straight);
        assert!(conflicts.contains(&lane_pair_key(south_straight, east_straight)));
    }

    #[test]
    fn build_lane_conflicts_covers_cross_intersection() {
        let model = IntersectionModel::new();
        let conflicts = build_lane_conflicts(&model);
        assert!(
            conflicts.len() > 10,
            "cross intersection should have many conflicting lane pairs"
        );
    }

    #[test]
    fn update_transitions_approaching_to_managed_on_zone_entry() {
        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        let center = Vec2::new(
            crate::config::INTERSECTION_CENTER_X,
            crate::config::INTERSECTION_CENTER_Y,
        );
        let mut vehicles = vec![test_vehicle_at(center, VehicleState::Approaching)];

        smart.update(&mut vehicles, &model, 0.0);

        assert_eq!(vehicles[0].state, VehicleState::Managed);
        assert_eq!(vehicles[0].time_in_crossing, 0.0);
        assert_eq!(vehicles[0].distance_in_crossing, 0.0);
    }

    #[test]
    fn update_transitions_managed_to_exiting_on_zone_exit() {
        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        let outside = Vec2::new(100.0, 100.0);
        let mut vehicles = vec![test_vehicle_at(outside, VehicleState::Managed)];

        smart.update(&mut vehicles, &model, 0.0);

        assert_eq!(vehicles[0].state, VehicleState::Exiting);
    }

    #[test]
    fn approaching_outside_zone_stays_approaching() {
        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        let outside = Vec2::new(100.0, 100.0);
        let mut vehicles = vec![test_vehicle_at(outside, VehicleState::Approaching)];

        smart.update(&mut vehicles, &model, 0.0);

        assert_eq!(vehicles[0].state, VehicleState::Approaching);
    }

    #[test]
    fn crossing_metrics_start_only_after_zone_detection() {
        use crate::vehicle::integrate_physics;

        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        let outside = Vec2::new(100.0, 100.0);
        let mut vehicles = vec![test_vehicle_at(outside, VehicleState::Approaching)];

        integrate_physics(&mut vehicles[0], 0.1);
        assert_eq!(vehicles[0].time_in_crossing, 0.0);
        assert_eq!(vehicles[0].distance_in_crossing, 0.0);

        let center = Vec2::new(
            crate::config::INTERSECTION_CENTER_X,
            crate::config::INTERSECTION_CENTER_Y,
        );
        vehicles[0].position = center;
        smart.update(&mut vehicles, &model, 0.0);
        assert_eq!(vehicles[0].state, VehicleState::Managed);

        integrate_physics(&mut vehicles[0], 0.1);
        assert!(vehicles[0].time_in_crossing > 0.0);
        assert!(vehicles[0].distance_in_crossing > 0.0);
    }

    #[test]
    fn schedule_managed_velocities_yields_later_entry_on_conflict() {
        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        smart.ensure_lane_conflicts(&model);

        let center = Vec2::new(
            crate::config::INTERSECTION_CENTER_X,
            crate::config::INTERSECTION_CENTER_Y,
        );
        let nominal = VelocityLevel::Fast.speed();
        let leader = managed_vehicle(
            1,
            lane_id(Cardinal::South, Route::Straight),
            Cardinal::South,
            Route::Straight,
            center,
            nominal,
        );
        let follower = managed_vehicle(
            2,
            lane_id(Cardinal::East, Route::Straight),
            Cardinal::East,
            Route::Straight,
            Vec2::new(center.x + SAFE_DISTANCE * 0.25, center.y),
            nominal,
        );

        smart.entry_sequence.insert(leader.id, 0);
        smart.entry_sequence.insert(follower.id, 1);

        let mut vehicles = vec![leader, follower];
        smart.schedule_managed_velocities(&mut vehicles);

        assert_eq!(vehicles[0].commanded_velocity, nominal);
        assert!(vehicles[1].commanded_velocity < nominal);
    }

    #[test]
    fn schedule_managed_velocities_same_lane_follower_yields() {
        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        smart.ensure_lane_conflicts(&model);

        let lane = lane_id(Cardinal::South, Route::Straight);
        let nominal = VelocityLevel::Fast.speed();
        let leader = managed_vehicle(
            1,
            lane,
            Cardinal::South,
            Route::Straight,
            Vec2::new(512.0, 400.0),
            nominal,
        );
        let follower = managed_vehicle(
            2,
            lane,
            Cardinal::South,
            Route::Straight,
            Vec2::new(512.0, 400.0 + SAFE_DISTANCE * 0.5),
            nominal,
        );

        smart.entry_sequence.insert(leader.id, 0);
        smart.entry_sequence.insert(follower.id, 1);

        let mut vehicles = vec![leader, follower];
        smart.schedule_managed_velocities(&mut vehicles);

        assert_eq!(vehicles[0].commanded_velocity, nominal);
        assert!(vehicles[1].commanded_velocity < nominal);
    }
}
