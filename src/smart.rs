//! Smart intersection controller (C01+).

use crate::intersection::{IntersectionModel, Vec2};
use crate::vehicle::{Vehicle, VehicleState};

/// Coordinates AV passage without traffic lights (C01/C02).
#[derive(Debug, Default)]
pub struct SmartController;

impl SmartController {
    pub fn new() -> Self {
        Self
    }

    /// Transition a vehicle into managed state when it enters the intersection zone (REQ-4).
    pub fn on_vehicle_enter_zone(&mut self, vehicle: &mut Vehicle) {
        if vehicle.state != VehicleState::Approaching {
            return;
        }
        vehicle.state = VehicleState::Managed;
        vehicle.time_in_crossing = 0.0;
        vehicle.distance_in_crossing = 0.0;
    }

    /// Detect zone entry/exit and update vehicle lifecycle (REQ-4, REQ-23).
    pub fn update(&mut self, vehicles: &mut [Vehicle], model: &IntersectionModel, _dt: f32) {
        for vehicle in vehicles {
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
    use crate::intersection::{Cardinal, LaneId, Route};
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
            state,
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
}
