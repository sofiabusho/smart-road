//! Cross intersection topology (lane registry — A03; path polylines — B02).

use crate::config::{
    APPROACH_MARGIN, INTERSECTION_CENTER_X, INTERSECTION_CENTER_Y, INTERSECTION_HALF_SIZE,
    LANES_PER_APPROACH, LANE_WIDTH,
};

/// 2D world position (origin top-left, +x east, +y south).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Cardinal approach direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cardinal {
    North,
    South,
    East,
    West,
}

impl Cardinal {
    pub const ALL: [Self; 4] = [Self::North, Self::South, Self::East, Self::West];

    pub const fn index(self) -> u32 {
        match self {
            Self::North => 0,
            Self::South => 1,
            Self::East => 2,
            Self::West => 3,
        }
    }

    /// Unit heading (radians) for straight travel from this approach into the junction.
    pub const fn travel_heading(self) -> f32 {
        match self {
            Self::North => std::f32::consts::FRAC_PI_2,
            Self::South => -std::f32::consts::FRAC_PI_2,
            Self::East => std::f32::consts::PI,
            Self::West => 0.0,
        }
    }
}

/// Fixed lane route through the intersection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    Right,
    Straight,
    Left,
}

impl Route {
    pub const ALL: [Self; 3] = [Self::Right, Self::Straight, Self::Left];

    pub const fn index(self) -> u32 {
        match self {
            Self::Right => 0,
            Self::Straight => 1,
            Self::Left => 2,
        }
    }
}

/// Stable lane identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LaneId(pub u32);

/// Lane metadata (path geometry added in B02).
#[derive(Debug, Clone)]
pub struct LaneInfo {
    pub id: LaneId,
    pub approach: Cardinal,
    pub route: Route,
    pub spawn_point: Vec2,
}

/// Intersection layout and detection zone (paths in B02).
#[derive(Debug)]
pub struct IntersectionModel {
    pub lanes: Vec<LaneInfo>,
    /// Smart-system detection boundary (C01 reads).
    pub zone_polygon: Vec<Vec2>,
}

impl IntersectionModel {
    pub fn new() -> Self {
        let lanes = build_lane_registry();
        let zone_polygon = junction_zone_polygon();
        Self {
            lanes,
            zone_polygon,
        }
    }

    /// Lookup lane by stable id.
    pub fn lane(&self, id: LaneId) -> Option<&LaneInfo> {
        self.lanes.iter().find(|lane| lane.id == id)
    }

    /// All lane ids for one approach (right, straight, left order).
    pub fn lanes_for_approach(&self, approach: Cardinal) -> Vec<LaneId> {
        self.lanes
            .iter()
            .filter(|lane| lane.approach == approach)
            .map(|lane| lane.id)
            .collect()
    }
}

impl Default for IntersectionModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute stable lane id from approach and route.
pub fn lane_id(approach: Cardinal, route: Route) -> LaneId {
    LaneId(approach.index() * LANES_PER_APPROACH + route.index())
}

fn build_lane_registry() -> Vec<LaneInfo> {
    let mut lanes = Vec::with_capacity(Cardinal::ALL.len() * Route::ALL.len());
    for approach in Cardinal::ALL {
        for route in Route::ALL {
            lanes.push(LaneInfo {
                id: lane_id(approach, route),
                approach,
                route,
                spawn_point: spawn_point_for(approach, route),
            });
        }
    }
    lanes
}

fn spawn_point_for(approach: Cardinal, route: Route) -> Vec2 {
    let lane_center_offset = lane_center_offset(approach, route);
    match approach {
        Cardinal::North => Vec2::new(INTERSECTION_CENTER_X + lane_center_offset, APPROACH_MARGIN),
        Cardinal::South => Vec2::new(
            INTERSECTION_CENTER_X + lane_center_offset,
            crate::config::WINDOW_HEIGHT as f32 - APPROACH_MARGIN,
        ),
        Cardinal::East => Vec2::new(
            crate::config::WINDOW_WIDTH as f32 - APPROACH_MARGIN,
            INTERSECTION_CENTER_Y + lane_center_offset,
        ),
        Cardinal::West => Vec2::new(APPROACH_MARGIN, INTERSECTION_CENTER_Y + lane_center_offset),
    }
}

/// Offset from road centerline to lane center for right-hand traffic.
///
/// Lanes run along X for N/S approaches and along Y for E/W approaches.
/// "Right" is the driver's right given the approach heading into the junction.
fn lane_center_offset(approach: Cardinal, route: Route) -> f32 {
    let right = match approach {
        Cardinal::North | Cardinal::West => LANE_WIDTH,
        Cardinal::South | Cardinal::East => -LANE_WIDTH,
    };
    match route {
        Route::Right => right,
        Route::Straight => 0.0,
        Route::Left => -right,
    }
}

fn junction_zone_polygon() -> Vec<Vec2> {
    let cx = INTERSECTION_CENTER_X;
    let cy = INTERSECTION_CENTER_Y;
    let h = INTERSECTION_HALF_SIZE;
    vec![
        Vec2::new(cx - h, cy - h),
        Vec2::new(cx + h, cy - h),
        Vec2::new(cx + h, cy + h),
        Vec2::new(cx - h, cy + h),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn lane_registry_has_twelve_unique_lanes() {
        let model = IntersectionModel::new();
        assert_eq!(model.lanes.len(), 12);
        let ids: HashSet<_> = model.lanes.iter().map(|lane| lane.id).collect();
        assert_eq!(ids.len(), 12);
    }

    #[test]
    fn each_approach_has_three_routes() {
        let model = IntersectionModel::new();
        for approach in Cardinal::ALL {
            let lanes: Vec<_> = model
                .lanes
                .iter()
                .filter(|lane| lane.approach == approach)
                .collect();
            assert_eq!(lanes.len(), 3);
            let routes: HashSet<_> = lanes.iter().map(|lane| lane.route).collect();
            assert_eq!(routes.len(), 3);
        }
    }

    #[test]
    fn lane_id_mapping_is_stable() {
        assert_eq!(lane_id(Cardinal::North, Route::Right), LaneId(0));
        assert_eq!(lane_id(Cardinal::South, Route::Left), LaneId(5));
        assert_eq!(lane_id(Cardinal::West, Route::Straight), LaneId(10));
    }

    #[test]
    fn zone_polygon_is_axis_aligned_box() {
        let model = IntersectionModel::new();
        assert_eq!(model.zone_polygon.len(), 4);
        let min_x = model
            .zone_polygon
            .iter()
            .map(|p| p.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = model
            .zone_polygon
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(max_x - min_x > 0.0);
    }

    #[test]
    fn spawn_points_sit_on_approach_edges() {
        let model = IntersectionModel::new();
        let south = model
            .lane(lane_id(Cardinal::South, Route::Straight))
            .unwrap();
        assert!(south.spawn_point.y > INTERSECTION_CENTER_Y);
        let north = model
            .lane(lane_id(Cardinal::North, Route::Straight))
            .unwrap();
        assert!(north.spawn_point.y < INTERSECTION_CENTER_Y);
    }

    #[test]
    fn spawn_offsets_match_right_hand_traffic() {
        let model = IntersectionModel::new();

        let north_right = model.lane(lane_id(Cardinal::North, Route::Right)).unwrap();
        let north_left = model.lane(lane_id(Cardinal::North, Route::Left)).unwrap();
        assert!(north_right.spawn_point.x > INTERSECTION_CENTER_X);
        assert!(north_left.spawn_point.x < INTERSECTION_CENTER_X);

        let south_right = model.lane(lane_id(Cardinal::South, Route::Right)).unwrap();
        let south_left = model.lane(lane_id(Cardinal::South, Route::Left)).unwrap();
        assert!(south_right.spawn_point.x < INTERSECTION_CENTER_X);
        assert!(south_left.spawn_point.x > INTERSECTION_CENTER_X);

        let east_right = model.lane(lane_id(Cardinal::East, Route::Right)).unwrap();
        let east_left = model.lane(lane_id(Cardinal::East, Route::Left)).unwrap();
        assert!(east_right.spawn_point.y < INTERSECTION_CENTER_Y);
        assert!(east_left.spawn_point.y > INTERSECTION_CENTER_Y);

        let west_right = model.lane(lane_id(Cardinal::West, Route::Right)).unwrap();
        let west_left = model.lane(lane_id(Cardinal::West, Route::Left)).unwrap();
        assert!(west_right.spawn_point.y > INTERSECTION_CENTER_Y);
        assert!(west_left.spawn_point.y < INTERSECTION_CENTER_Y);
    }
}
