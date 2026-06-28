//! Smart intersection controller (C01/C02 + reservation gate).

use std::collections::{HashMap, HashSet};

use crate::config::{RESERVATION_TRIGGER_DISTANCE, SAFE_DISTANCE};
use crate::intersection::{Cardinal, IntersectionModel, LaneId, Vec2};
use crate::vehicle::{
    apply_reservation_braking, retract_vehicle_outside_zone, sprite_separation_gap, Vehicle,
    VehicleState, SCHEDULER_CLEAR_GAP, SCHEDULER_SPRITE_GAP_THRESHOLD,
};

/// Coordinates AV passage without traffic lights (C01 detection, C02 scheduling).
#[derive(Debug)]
pub struct SmartController {
    lane_conflicts: HashSet<(u32, u32)>,
    conflicts_ready: bool,
    next_entry_seq: u32,
    entry_sequence: HashMap<crate::vehicle::VehicleId, u32>,
    wait_order: HashMap<crate::vehicle::VehicleId, u32>,
    next_wait_seq: u32,
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
            wait_order: HashMap::new(),
            next_wait_seq: 0,
        }
    }

    /// Transition a vehicle into managed state when it enters the intersection zone (REQ-4).
    pub fn on_vehicle_enter_zone(&mut self, vehicle: &mut Vehicle) {
        if vehicle.state != VehicleState::Approaching || !vehicle.reservation_granted {
            return;
        }
        vehicle.state = VehicleState::Managed;
        vehicle.time_in_crossing = 0.0;
        vehicle.distance_in_crossing = 0.0;
        vehicle.commanded_velocity = vehicle.nominal_velocity;
        vehicle.scheduler_yield = false;
        vehicle.reservation_hold = false;
        self.entry_sequence.insert(vehicle.id, self.next_entry_seq);
        self.next_entry_seq += 1;
    }

    /// Detect zone entry/exit, grant reservations, schedule managed velocities (REQ-3, REQ-4).
    pub fn update(&mut self, vehicles: &mut [Vehicle], model: &IntersectionModel, _dt: f32) {
        self.ensure_lane_conflicts(model);

        for vehicle in vehicles.iter_mut() {
            if vehicle.state == VehicleState::Done {
                continue;
            }

            let in_zone = point_in_polygon(vehicle.position, &model.zone_polygon);

            if vehicle.state == VehicleState::Managed && !in_zone {
                vehicle.state = VehicleState::Exiting;
                vehicle.reservation_granted = false;
                vehicle.scheduler_yield = false;
                self.entry_sequence.remove(&vehicle.id);
                self.wait_order.remove(&vehicle.id);
            }
        }

        self.expand_reservations(vehicles, model);

        for idx in 0..vehicles.len() {
            if vehicles[idx].state != VehicleState::Approaching {
                if vehicles[idx].reservation_hold {
                    vehicles[idx].reservation_hold = false;
                }
                continue;
            }

            if vehicles[idx].reservation_granted {
                vehicles[idx].reservation_hold = false;
                continue;
            }

            let in_zone = point_in_polygon(vehicles[idx].position, &model.zone_polygon);
            let distance_to_zone = distance_to_zone_entry(model, &vehicles[idx]);
            let blocked = self.reservation_blocked(&vehicles[idx], vehicles, model);

            if in_zone {
                vehicles[idx].reservation_hold = true;
                vehicles[idx].commanded_velocity = 0.0;
                vehicles[idx].velocity = 0.0;
                continue;
            }

            if blocked {
                let stop_buffer =
                    vehicles[idx].velocity * crate::config::FIXED_TIMESTEP_SECS + 2.0;
                let near_stop = distance_to_zone
                    .map(|d| d > 0.0 && d <= SAFE_DISTANCE + stop_buffer)
                    .unwrap_or(false);
                if vehicles[idx].reservation_hold || near_stop {
                    vehicles[idx].reservation_hold = true;
                    vehicles[idx].commanded_velocity = 0.0;
                    vehicles[idx].velocity = 0.0;
                    continue;
                }
            } else {
                vehicles[idx].reservation_hold = false;
            }

            if let Some(dist) = distance_to_zone {
                if dist > 0.0 && dist <= RESERVATION_TRIGGER_DISTANCE {
                    self.track_waiter(vehicles[idx].id);
                    apply_reservation_braking(&mut vehicles[idx], dist);
                }
            }
        }

        self.expand_reservations(vehicles, model);

        for vehicle in vehicles.iter_mut() {
            if vehicle.state != VehicleState::Approaching || !vehicle.reservation_granted {
                continue;
            }
            if point_in_polygon(vehicle.position, &model.zone_polygon) {
                self.on_vehicle_enter_zone(vehicle);
            }
        }

        self.schedule_managed_velocities(vehicles);
    }

    /// True when the scheduler commanded a managed vehicle below its nominal speed (C02 / tests).
    pub fn managed_scheduler_yielded(vehicles: &[Vehicle]) -> bool {
        vehicles.iter().any(|vehicle| {
            vehicle.state == VehicleState::Managed
                && (vehicle.scheduler_yield
                    || vehicle.commanded_velocity + 0.01 < vehicle.nominal_velocity
                    || vehicle.commanded_velocity < 1.0)
        })
    }

    /// True when two managed vehicles are close enough for the scheduler to act (C02 / tests).
    pub fn managed_vehicles_in_scheduler_range(vehicles: &[Vehicle]) -> bool {
        let len = vehicles.len();
        for i in 0..len {
            if vehicles[i].state != VehicleState::Managed {
                continue;
            }
            for j in (i + 1)..len {
                if vehicles[j].state != VehicleState::Managed {
                    continue;
                }
                if sprite_separation_gap(&vehicles[i], &vehicles[j]) < SCHEDULER_SPRITE_GAP_THRESHOLD {
                    return true;
                }
            }
        }
        false
    }

    /// Command velocities for vehicles inside the managed zone (C02 / REQ-3, REQ-9).
    fn schedule_managed_velocities(&self, vehicles: &mut [Vehicle]) {
        for vehicle in vehicles.iter_mut() {
            if vehicle.state != VehicleState::Managed {
                if vehicle.scheduler_yield {
                    vehicle.scheduler_yield = false;
                }
                continue;
            }
            if vehicle.scheduler_yield {
                vehicle.commanded_velocity = 0.0;
                vehicle.velocity = 0.0;
            } else {
                vehicle.commanded_velocity = vehicle.nominal_velocity;
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

                let gap = sprite_separation_gap(&vehicles[i], &vehicles[j]);
                if gap >= SCHEDULER_SPRITE_GAP_THRESHOLD {
                    continue;
                }

                let seq_i = self.entry_sequence.get(&vehicles[i].id).copied();
                let seq_j = self.entry_sequence.get(&vehicles[j].id).copied();
                let yielder = match (seq_i, seq_j) {
                    (Some(a), Some(b)) if a > b => i,
                    (Some(a), Some(b)) if a < b => j,
                    _ => continue,
                };

                vehicles[yielder].scheduler_yield = true;
                vehicles[yielder].commanded_velocity = 0.0;
                vehicles[yielder].velocity = 0.0;
            }
        }

        for i in 0..len {
            if !vehicles[i].scheduler_yield || vehicles[i].state != VehicleState::Managed {
                continue;
            }
            let mut clear = true;
            for j in 0..len {
                if i == j {
                    continue;
                }
                if vehicles[j].state != VehicleState::Managed
                    && vehicles[j].state != VehicleState::Exiting
                {
                    continue;
                }
                let pair = lane_pair_key(vehicles[i].lane_id, vehicles[j].lane_id);
                let same_lane = vehicles[i].lane_id == vehicles[j].lane_id;
                if !same_lane && !self.lane_conflicts.contains(&pair) {
                    continue;
                }
                if sprite_separation_gap(&vehicles[i], &vehicles[j]) < SCHEDULER_CLEAR_GAP {
                    clear = false;
                    break;
                }
            }
            if clear {
                vehicles[i].scheduler_yield = false;
                vehicles[i].commanded_velocity = vehicles[i].nominal_velocity;
            }
        }
    }

    fn track_waiter(&mut self, id: crate::vehicle::VehicleId) {
        if !self.wait_order.contains_key(&id) {
            self.wait_order.insert(id, self.next_wait_seq);
            self.next_wait_seq += 1;
        }
    }

    fn expand_reservations(&mut self, vehicles: &mut [Vehicle], model: &IntersectionModel) {
        let mut candidates: Vec<(u32, usize)> = vehicles
            .iter()
            .enumerate()
            .filter_map(|(idx, vehicle)| {
                if vehicle.state != VehicleState::Approaching || vehicle.reservation_granted {
                    return None;
                }
                let in_zone = point_in_polygon(vehicle.position, &model.zone_polygon);
                let in_trigger = distance_to_zone_entry(model, vehicle)
                    .map(|d| d > 0.0 && d <= RESERVATION_TRIGGER_DISTANCE)
                    .unwrap_or(false);
                if in_zone || in_trigger {
                    let order = self.wait_order.get(&vehicle.id).copied().unwrap_or(u32::MAX);
                    Some((order, idx))
                } else {
                    None
                }
            })
            .collect();
        candidates.sort_by_key(|(order, _)| *order);

        for (_, idx) in candidates {
            if vehicles[idx].reservation_granted {
                continue;
            }
            if self.reservation_blocked(&vehicles[idx], vehicles, model) {
                continue;
            }
            vehicles[idx].reservation_granted = true;
        }
    }

    fn reservation_blocked(
        &self,
        candidate: &Vehicle,
        vehicles: &[Vehicle],
        model: &IntersectionModel,
    ) -> bool {
        for other in vehicles {
            if other.id == candidate.id || other.state == VehicleState::Done {
                continue;
            }
            if !self.lanes_conflict(candidate.lane_id, other.lane_id) {
                continue;
            }
            if point_in_polygon(other.position, &model.zone_polygon) {
                return true;
            }
            if other.state == VehicleState::Approaching && other.reservation_granted {
                return true;
            }
        }
        false
    }

    fn lanes_conflict(&self, a: LaneId, b: LaneId) -> bool {
        a == b || self.lane_conflicts.contains(&lane_pair_key(a, b))
    }

    fn ensure_lane_conflicts(&mut self, model: &IntersectionModel) {
        if self.conflicts_ready {
            return;
        }
        self.lane_conflicts = build_lane_conflicts(model);
        self.conflicts_ready = true;
    }

    /// Register FIFO entry order for a vehicle already in Managed state (integration tests).
    pub fn register_managed_entry(&mut self, vehicle_id: crate::vehicle::VehicleId, seq: u32) {
        self.entry_sequence.insert(vehicle_id, seq);
    }

    /// Push unreserved vehicles back outside the junction after physics overshoot.
    pub fn enforce_zone_gate(vehicles: &mut [Vehicle], model: &IntersectionModel) {
        for vehicle in vehicles.iter_mut() {
            if vehicle.state != VehicleState::Approaching || vehicle.reservation_granted {
                continue;
            }
            if !model.point_in_zone(vehicle.position) {
                continue;
            }
            vehicle.velocity = 0.0;
            vehicle.commanded_velocity = 0.0;
            retract_vehicle_outside_zone(vehicle, model);
        }
    }
}

fn lane_pair_key(a: LaneId, b: LaneId) -> (u32, u32) {
    (a.0.min(b.0), a.0.max(b.0))
}

fn zone_bounds(zone: &[Vec2]) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for point in zone {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    (min_x, max_x, min_y, max_y)
}

/// Distance along the approach to the nearest junction-zone edge (0 when already inside).
fn distance_to_zone_entry(model: &IntersectionModel, vehicle: &Vehicle) -> Option<f32> {
    let (min_x, max_x, min_y, max_y) = zone_bounds(&model.zone_polygon);
    let pos = vehicle.position;

    match vehicle.approach {
        Cardinal::South => {
            if pos.y <= max_y {
                Some(0.0)
            } else {
                Some(pos.y - max_y)
            }
        }
        Cardinal::North => {
            if pos.y >= min_y {
                Some(0.0)
            } else {
                Some(min_y - pos.y)
            }
        }
        Cardinal::East => {
            if pos.x <= max_x {
                Some(0.0)
            } else {
                Some(pos.x - max_x)
            }
        }
        Cardinal::West => {
            if pos.x >= min_x {
                Some(0.0)
            } else {
                Some(min_x - pos.x)
            }
        }
    }
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
    use crate::config::SAFE_DISTANCE;
    use crate::intersection::{lane_id, Cardinal, Route};
    use crate::vehicle::VelocityLevel;
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
            reservation_granted: false,
            scheduler_yield: false,
            reservation_hold: false,
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
            reservation_granted: true,
            scheduler_yield: false,
            reservation_hold: false,
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
    fn reservation_blocks_while_conflicting_vehicle_exits_zone() {
        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        let center = Vec2::new(
            crate::config::INTERSECTION_CENTER_X,
            crate::config::INTERSECTION_CENTER_Y,
        );
        let south_lane = lane_id(Cardinal::South, Route::Straight);
        let (_, _, _, max_y) = zone_bounds(&model.zone_polygon);
        let mut exiting = managed_vehicle(
            1,
            south_lane,
            Cardinal::South,
            Route::Straight,
            center,
            120.0,
        );
        exiting.state = VehicleState::Exiting;
        exiting.reservation_granted = false;

        let mut follower = test_vehicle_at(
            Vec2::new(
                center.x,
                max_y + crate::config::SAFE_DISTANCE,
            ),
            VehicleState::Approaching,
        );
        follower.id = VehicleId(2);
        follower.lane_id = south_lane;

        let mut vehicles = vec![exiting, follower];
        smart.update(&mut vehicles, &model, 0.0);

        assert!(!vehicles[1].reservation_granted);
    }

    #[test]
    fn reservation_blocks_conflicting_approach_until_leader_exits() {
        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        let center = Vec2::new(
            crate::config::INTERSECTION_CENTER_X,
            crate::config::INTERSECTION_CENTER_Y,
        );
        let south = managed_vehicle(
            1,
            lane_id(Cardinal::South, Route::Straight),
            Cardinal::South,
            Route::Straight,
            center,
            120.0,
        );
        let mut east = test_vehicle_at(
            Vec2::new(center.x + SAFE_DISTANCE * 0.5, center.y),
            VehicleState::Approaching,
        );
        east.id = VehicleId(2);
        east.lane_id = lane_id(Cardinal::East, Route::Straight);
        east.approach = Cardinal::East;

        let mut vehicles = vec![south, east];
        smart.entry_sequence.insert(VehicleId(1), 0);
        smart.update(&mut vehicles, &model, 0.0);

        assert_eq!(vehicles[1].state, VehicleState::Approaching);
        assert!(!vehicles[1].reservation_granted);
        assert_eq!(vehicles[1].commanded_velocity, 0.0);
    }

    #[test]
    fn reservation_brakes_in_trigger_zone_without_grant() {
        let model = IntersectionModel::new();
        let mut smart = SmartController::new();
        let center = Vec2::new(
            crate::config::INTERSECTION_CENTER_X,
            crate::config::INTERSECTION_CENTER_Y,
        );
        let (_, _, _, max_y) = zone_bounds(&model.zone_polygon);
        let south_x = crate::config::INTERSECTION_CENTER_X;
        let lane = lane_id(Cardinal::South, Route::Straight);
        let leader = managed_vehicle(1, lane, Cardinal::South, Route::Straight, center, 120.0);
        let mut follower = test_vehicle_at(
            Vec2::new(south_x, max_y + crate::config::VEHICLE_LENGTH * 0.5),
            VehicleState::Approaching,
        );
        follower.id = VehicleId(2);
        follower.lane_id = lane;
        smart.entry_sequence.insert(VehicleId(1), 0);

        let mut vehicles = vec![leader, follower];
        smart.update(&mut vehicles, &model, 0.0);

        assert_eq!(vehicles[1].state, VehicleState::Approaching);
        assert!(!vehicles[1].reservation_granted);
        assert_eq!(vehicles[1].commanded_velocity, 0.0);
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
