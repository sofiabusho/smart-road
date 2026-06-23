//! Cross intersection topology (lane registry — A03; path polylines — B02).

use std::collections::HashMap;

use crate::config::{
    APPROACH_MARGIN, INBOUND_ROAD_WIDTH, INTERSECTION_CENTER_X, INTERSECTION_CENTER_Y,
    INTERSECTION_HALF_SIZE, LANES_PER_APPROACH, LANE_WIDTH, ROAD_ARM_WIDTH, WINDOW_HEIGHT,
    WINDOW_WIDTH,
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

    /// Single-letter approach prefix for debug lane labels (N/S/E/W).
    pub const fn label_prefix(self) -> char {
        match self {
            Self::North => 'N',
            Self::South => 'S',
            Self::East => 'E',
            Self::West => 'W',
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
    pub path: Vec<Vec2>,
}

/// Map from lane id to its path polyline.
pub type LanePathMap = HashMap<LaneId, Vec<Vec2>>;

/// Render-facing vehicle snapshot (shared by B `snapshot_for_render` and A `draw_frame`).
#[derive(Debug, Clone, Copy)]
pub struct VehicleRenderSnapshot {
    pub position: Vec2,
    pub heading_rad: f32,
    pub approach: Cardinal,
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
        let mut model = Self {
            lanes,
            zone_polygon,
        };
        let paths = build_all_lane_paths();
        attach_paths(&mut model, paths);
        model
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

/// Debug label for an inbound lane (e.g. `S1` = South approach, right-turn lane).
pub fn lane_display_label(approach: Cardinal, route: Route) -> &'static str {
    match (approach, route) {
        (Cardinal::North, Route::Right) => "N1",
        (Cardinal::North, Route::Straight) => "N2",
        (Cardinal::North, Route::Left) => "N3",
        (Cardinal::South, Route::Right) => "S1",
        (Cardinal::South, Route::Straight) => "S2",
        (Cardinal::South, Route::Left) => "S3",
        (Cardinal::East, Route::Right) => "E1",
        (Cardinal::East, Route::Straight) => "E2",
        (Cardinal::East, Route::Left) => "E3",
        (Cardinal::West, Route::Right) => "W1",
        (Cardinal::West, Route::Straight) => "W2",
        (Cardinal::West, Route::Left) => "W3",
    }
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
                path: Vec::new(),
            });
        }
    }
    lanes
}

pub fn attach_paths(model: &mut IntersectionModel, paths: LanePathMap) {
    for lane in &mut model.lanes {
        if let Some(path) = paths.get(&lane.id) {
            lane.path = path.clone();
        }
    }
}

const OFF_SCREEN_MARGIN: f32 = 64.0;

fn off_screen_west(y: f32) -> Vec2 {
    Vec2::new(-OFF_SCREEN_MARGIN, y)
}

fn off_screen_east(y: f32) -> Vec2 {
    Vec2::new(WINDOW_WIDTH as f32 + OFF_SCREEN_MARGIN, y)
}

fn off_screen_north(x: f32) -> Vec2 {
    Vec2::new(x, -OFF_SCREEN_MARGIN)
}

fn off_screen_south(x: f32) -> Vec2 {
    Vec2::new(x, WINDOW_HEIGHT as f32 + OFF_SCREEN_MARGIN)
}

/// Four-point lane polyline: spawn → junction entry → exit lane → off-screen.
///
/// Turn geometry matches `iana/B05`: axial approach, corner connector into the
/// perpendicular outbound lane, then an axial exit leg. Exit coordinates use
/// `outbound_lane_line` so paths stay aligned with the six-lane arm layout.
fn build_all_lane_paths() -> LanePathMap {
    let mut map = HashMap::with_capacity(12);

    let jx_w = INTERSECTION_CENTER_X - INTERSECTION_HALF_SIZE;
    let jx_e = INTERSECTION_CENTER_X + INTERSECTION_HALF_SIZE;
    let jy_n = INTERSECTION_CENTER_Y - INTERSECTION_HALF_SIZE;
    let jy_s = INTERSECTION_CENTER_Y + INTERSECTION_HALF_SIZE;

    let n_r_x = spawn_point_for(Cardinal::North, Route::Right).x;
    let n_s_x = spawn_point_for(Cardinal::North, Route::Straight).x;
    let n_l_x = spawn_point_for(Cardinal::North, Route::Left).x;
    let s_r_x = spawn_point_for(Cardinal::South, Route::Right).x;
    let s_s_x = spawn_point_for(Cardinal::South, Route::Straight).x;
    let s_l_x = spawn_point_for(Cardinal::South, Route::Left).x;
    let e_r_y = spawn_point_for(Cardinal::East, Route::Right).y;
    let e_s_y = spawn_point_for(Cardinal::East, Route::Straight).y;
    let e_l_y = spawn_point_for(Cardinal::East, Route::Left).y;
    let w_r_y = spawn_point_for(Cardinal::West, Route::Right).y;
    let w_s_y = spawn_point_for(Cardinal::West, Route::Straight).y;
    let w_l_y = spawn_point_for(Cardinal::West, Route::Left).y;

    let west_out_r = outbound_lane_line(Cardinal::West, Route::Right);
    let west_out_l = outbound_lane_line(Cardinal::West, Route::Left);
    let east_out_r = outbound_lane_line(Cardinal::East, Route::Right);
    let east_out_l = outbound_lane_line(Cardinal::East, Route::Left);
    let north_out_r = outbound_lane_line(Cardinal::North, Route::Right);
    let north_out_l = outbound_lane_line(Cardinal::North, Route::Left);
    let south_out_r = outbound_lane_line(Cardinal::South, Route::Right);
    let south_out_l = outbound_lane_line(Cardinal::South, Route::Left);

    // North approach (southbound inbound)
    map.insert(
        lane_id(Cardinal::North, Route::Right),
        vec![
            spawn_point_for(Cardinal::North, Route::Right),
            Vec2::new(n_r_x, jy_n),
            Vec2::new(jx_w, west_out_r),
            off_screen_west(west_out_r),
        ],
    );
    map.insert(
        lane_id(Cardinal::North, Route::Straight),
        vec![
            spawn_point_for(Cardinal::North, Route::Straight),
            Vec2::new(n_s_x, jy_n),
            Vec2::new(n_s_x, jy_s),
            off_screen_south(n_s_x),
        ],
    );
    map.insert(
        lane_id(Cardinal::North, Route::Left),
        vec![
            spawn_point_for(Cardinal::North, Route::Left),
            Vec2::new(n_l_x, jy_n),
            Vec2::new(jx_e, east_out_l),
            off_screen_east(east_out_l),
        ],
    );

    // South approach (northbound inbound)
    map.insert(
        lane_id(Cardinal::South, Route::Right),
        vec![
            spawn_point_for(Cardinal::South, Route::Right),
            Vec2::new(s_r_x, jy_s),
            Vec2::new(jx_e, east_out_r),
            off_screen_east(east_out_r),
        ],
    );
    map.insert(
        lane_id(Cardinal::South, Route::Straight),
        vec![
            spawn_point_for(Cardinal::South, Route::Straight),
            Vec2::new(s_s_x, jy_s),
            Vec2::new(s_s_x, jy_n),
            off_screen_north(s_s_x),
        ],
    );
    map.insert(
        lane_id(Cardinal::South, Route::Left),
        vec![
            spawn_point_for(Cardinal::South, Route::Left),
            Vec2::new(s_l_x, jy_s),
            Vec2::new(jx_w, west_out_l),
            off_screen_west(west_out_l),
        ],
    );

    // East approach (westbound inbound)
    map.insert(
        lane_id(Cardinal::East, Route::Right),
        vec![
            spawn_point_for(Cardinal::East, Route::Right),
            Vec2::new(jx_e, e_r_y),
            Vec2::new(north_out_r, jy_n),
            off_screen_north(north_out_r),
        ],
    );
    map.insert(
        lane_id(Cardinal::East, Route::Straight),
        vec![
            spawn_point_for(Cardinal::East, Route::Straight),
            Vec2::new(jx_e, e_s_y),
            Vec2::new(jx_w, e_s_y),
            off_screen_west(e_s_y),
        ],
    );
    map.insert(
        lane_id(Cardinal::East, Route::Left),
        vec![
            spawn_point_for(Cardinal::East, Route::Left),
            Vec2::new(jx_e, e_l_y),
            Vec2::new(south_out_l, jy_s),
            off_screen_south(south_out_l),
        ],
    );

    // West approach (eastbound inbound)
    map.insert(
        lane_id(Cardinal::West, Route::Right),
        vec![
            spawn_point_for(Cardinal::West, Route::Right),
            Vec2::new(jx_w, w_r_y),
            Vec2::new(south_out_r, jy_s),
            off_screen_south(south_out_r),
        ],
    );
    map.insert(
        lane_id(Cardinal::West, Route::Straight),
        vec![
            spawn_point_for(Cardinal::West, Route::Straight),
            Vec2::new(jx_w, w_s_y),
            Vec2::new(jx_e, w_s_y),
            off_screen_east(w_s_y),
        ],
    );
    map.insert(
        lane_id(Cardinal::West, Route::Left),
        vec![
            spawn_point_for(Cardinal::West, Route::Left),
            Vec2::new(jx_w, w_l_y),
            Vec2::new(north_out_l, jy_n),
            off_screen_north(north_out_l),
        ],
    );

    map
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

/// Offset from road centerline to inbound lane center for right-hand traffic.
///
/// Each arm is six lanes wide (three inbound + three outbound). Inbound lanes sit on
/// the west half for North, east half for South, north half for East, south half for West.
fn lane_center_offset(approach: Cardinal, route: Route) -> f32 {
    let inbound_half_center = INBOUND_ROAD_WIDTH / 2.0;
    let half_arm = ROAD_ARM_WIDTH / 2.0;
    let toward_center = match approach {
        Cardinal::North | Cardinal::East => -half_arm + inbound_half_center,
        Cardinal::South | Cardinal::West => half_arm - inbound_half_center,
    };
    let right = match approach {
        // Northbound-from-north traffic: driver's right is west, so right-turn offset is negative.
        Cardinal::North => -LANE_WIDTH,
        Cardinal::South | Cardinal::West => LANE_WIDTH,
        Cardinal::East => -LANE_WIDTH,
    };
    let route_offset = match route {
        Route::Right => right,
        Route::Straight => 0.0,
        Route::Left => -right,
    };
    toward_center + route_offset
}

/// Offset from road centerline to **outbound** lane center (mirror of inbound across arm center).
fn outbound_lane_center(approach: Cardinal, route: Route) -> f32 {
    let inbound_half_center = INBOUND_ROAD_WIDTH / 2.0;
    let half_arm = ROAD_ARM_WIDTH / 2.0;
    let outbound_half_center = match approach {
        Cardinal::North | Cardinal::East => half_arm - inbound_half_center,
        Cardinal::South | Cardinal::West => -half_arm + inbound_half_center,
    };
    let right = match approach {
        Cardinal::North => LANE_WIDTH,
        Cardinal::South => -LANE_WIDTH,
        Cardinal::East => LANE_WIDTH,
        Cardinal::West => -LANE_WIDTH,
    };
    let route_offset = match route {
        Route::Right => right,
        Route::Straight => 0.0,
        Route::Left => -right,
    };
    outbound_half_center + route_offset
}

/// World x (N/S arms) or y (E/W arms) for an outbound lane centerline.
pub fn outbound_lane_line(approach: Cardinal, route: Route) -> f32 {
    let offset = outbound_lane_center(approach, route);
    match approach {
        Cardinal::North | Cardinal::South => INTERSECTION_CENTER_X + offset,
        Cardinal::East | Cardinal::West => INTERSECTION_CENTER_Y + offset,
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
    fn lane_display_labels_are_unique_per_lane() {
        let mut labels = HashSet::new();
        for approach in Cardinal::ALL {
            for route in Route::ALL {
                assert!(labels.insert(lane_display_label(approach, route)));
            }
        }
        assert_eq!(labels.len(), 12);
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
    fn approach_segment_is_axial_for_all_lanes() {
        // Verifies that the approach arm (path[0]→path[1]) has no perpendicular drift.
        // N/S lanes travel vertically: x must be constant. E/W lanes travel horizontally: y must be constant.
        // This is the regression test that catches the original bug (path[1] used the left-lane
        // coordinate for all three routes) and its incomplete fix (only path[0] was corrected).
        let model = IntersectionModel::new();
        for lane in &model.lanes {
            let p0 = lane.path[0];
            let p1 = lane.path[1];
            match lane.approach {
                Cardinal::North | Cardinal::South => assert_eq!(
                    p0.x, p1.x,
                    "N/S approach segment has x-drift for {:?} {:?}: path[0].x={} path[1].x={}",
                    lane.approach, lane.route, p0.x, p1.x
                ),
                Cardinal::East | Cardinal::West => assert_eq!(
                    p0.y, p1.y,
                    "E/W approach segment has y-drift for {:?} {:?}: path[0].y={} path[1].y={}",
                    lane.approach, lane.route, p0.y, p1.y
                ),
            }
        }
    }

    #[test]
    fn all_lane_paths_start_at_spawn_point() {
        let model = IntersectionModel::new();
        for lane in &model.lanes {
            assert_eq!(
                lane.path[0], lane.spawn_point,
                "path[0] mismatch for {:?} {:?}",
                lane.approach, lane.route
            );
        }
    }

    #[test]
    fn lane_exit_direction_matches_route() {
        // Classic RHT: right/left turns exit the perpendicular arm.
        //   N1→WEST  N3→EAST   S1→EAST   S3→WEST
        //   E1→NORTH E3→SOUTH  W1→SOUTH  W3→NORTH
        let model = IntersectionModel::new();
        let expected_exit = |approach: Cardinal, route: Route| -> &'static str {
            match (approach, route) {
                (Cardinal::North, Route::Right) => "WEST",
                (Cardinal::North, Route::Straight) => "SOUTH",
                (Cardinal::North, Route::Left) => "EAST",
                (Cardinal::South, Route::Right) => "EAST",
                (Cardinal::South, Route::Straight) => "NORTH",
                (Cardinal::South, Route::Left) => "WEST",
                (Cardinal::East, Route::Right) => "NORTH",
                (Cardinal::East, Route::Straight) => "WEST",
                (Cardinal::East, Route::Left) => "SOUTH",
                (Cardinal::West, Route::Right) => "SOUTH",
                (Cardinal::West, Route::Straight) => "EAST",
                (Cardinal::West, Route::Left) => "NORTH",
            }
        };
        let w = crate::config::WINDOW_WIDTH as f32;
        let h = crate::config::WINDOW_HEIGHT as f32;
        for lane in &model.lanes {
            let exit = lane.path[3];
            let actual = if exit.x < 0.0 {
                "WEST"
            } else if exit.x > w {
                "EAST"
            } else if exit.y < 0.0 {
                "NORTH"
            } else if exit.y > h {
                "SOUTH"
            } else {
                "NONE (not off-screen)"
            };
            let expected = expected_exit(lane.approach, lane.route);
            assert_eq!(
                actual, expected,
                "{:?} {:?}: expected exit {} but last waypoint = ({}, {})",
                lane.approach, lane.route, expected, exit.x, exit.y
            );
        }
    }

    #[test]
    fn turning_lane_paths_use_four_waypoints() {
        let model = IntersectionModel::new();
        for lane in &model.lanes {
            if lane.route == Route::Straight {
                continue;
            }
            assert_eq!(
                lane.path.len(),
                4,
                "{:?} {:?} turn path should have four waypoints (B05 style)",
                lane.approach,
                lane.route
            );
        }
    }

    #[test]
    fn approach_never_reverses_before_turn() {
        let model = IntersectionModel::new();
        for lane in &model.lanes {
            if lane.route == Route::Straight {
                continue;
            }
            for w in lane.path.windows(2).take(2) {
                let seg_dx = w[1].x - w[0].x;
                let seg_dy = w[1].y - w[0].y;
                let along = match lane.approach {
                    Cardinal::North => seg_dy,
                    Cardinal::South => -seg_dy,
                    Cardinal::East => -seg_dx,
                    Cardinal::West => seg_dx,
                };
                assert!(
                    along.abs() < 0.01 || along > -0.01,
                    "{:?} {:?} reverses on inbound axis: ({:.1},{:.1})",
                    lane.approach,
                    lane.route,
                    seg_dx,
                    seg_dy
                );
            }
        }
    }

    #[test]
    fn turn_exits_use_outbound_lane_centers() {
        let model = IntersectionModel::new();

        let before_exit = |lane: &LaneInfo| lane.path[lane.path.len() - 2];

        let n1 = model.lane(lane_id(Cardinal::North, Route::Right)).unwrap();
        assert!(
            (before_exit(n1).y - outbound_lane_line(Cardinal::West, Route::Right)).abs() < 1.0,
            "N1 should exit on west outbound right"
        );

        let n3 = model.lane(lane_id(Cardinal::North, Route::Left)).unwrap();
        assert!(
            (before_exit(n3).y - outbound_lane_line(Cardinal::East, Route::Left)).abs() < 1.0,
            "N3 should exit on east outbound left"
        );

        let s1 = model.lane(lane_id(Cardinal::South, Route::Right)).unwrap();
        assert!(
            (before_exit(s1).y - outbound_lane_line(Cardinal::East, Route::Right)).abs() < 1.0,
            "S1 should exit on east outbound right (W1 stripe)"
        );

        let s3 = model.lane(lane_id(Cardinal::South, Route::Left)).unwrap();
        assert!(
            (before_exit(s3).y - outbound_lane_line(Cardinal::West, Route::Left)).abs() < 1.0,
            "S3 should exit on west outbound left"
        );

        let e1 = model.lane(lane_id(Cardinal::East, Route::Right)).unwrap();
        assert!(
            (before_exit(e1).x - outbound_lane_line(Cardinal::North, Route::Right)).abs() < 1.0,
            "E1 should exit on north outbound right"
        );

        let e3 = model.lane(lane_id(Cardinal::East, Route::Left)).unwrap();
        assert!(
            (before_exit(e3).x - outbound_lane_line(Cardinal::South, Route::Left)).abs() < 1.0,
            "E3 should exit on south outbound left"
        );

        let w1 = model.lane(lane_id(Cardinal::West, Route::Right)).unwrap();
        assert!(
            (before_exit(w1).x - outbound_lane_line(Cardinal::South, Route::Right)).abs() < 1.0,
            "W1 should exit on south outbound right"
        );

        let w3 = model.lane(lane_id(Cardinal::West, Route::Left)).unwrap();
        assert!(
            (before_exit(w3).x - outbound_lane_line(Cardinal::North, Route::Left)).abs() < 1.0,
            "W3 should exit on north outbound left"
        );
    }

    #[test]
    fn outbound_lanes_sit_on_opposite_half_from_inbound() {
        for approach in Cardinal::ALL {
            for route in Route::ALL {
                let inbound = match approach {
                    Cardinal::North | Cardinal::South => {
                        spawn_point_for(approach, route).x - INTERSECTION_CENTER_X
                    }
                    Cardinal::East | Cardinal::West => {
                        spawn_point_for(approach, route).y - INTERSECTION_CENTER_Y
                    }
                };
                let outbound = outbound_lane_center(approach, route);
                assert!(
                    inbound.signum() != outbound.signum() || inbound.abs() < 0.01,
                    "{approach:?} {route:?}: inbound and outbound should be on opposite halves"
                );
            }
        }
    }

    #[test]
    fn spawn_offsets_match_right_hand_traffic() {
        let model = IntersectionModel::new();

        let north_right = model.lane(lane_id(Cardinal::North, Route::Right)).unwrap();
        let north_left = model.lane(lane_id(Cardinal::North, Route::Left)).unwrap();
        assert!(north_right.spawn_point.x < north_left.spawn_point.x);
        assert!(north_right.spawn_point.x < INTERSECTION_CENTER_X);
        assert!(north_left.spawn_point.x < INTERSECTION_CENTER_X);

        let south_right = model.lane(lane_id(Cardinal::South, Route::Right)).unwrap();
        let south_left = model.lane(lane_id(Cardinal::South, Route::Left)).unwrap();
        assert!(south_right.spawn_point.x > south_left.spawn_point.x);
        assert!(south_right.spawn_point.x > INTERSECTION_CENTER_X);
        assert!(south_left.spawn_point.x > INTERSECTION_CENTER_X);

        let east_right = model.lane(lane_id(Cardinal::East, Route::Right)).unwrap();
        let east_left = model.lane(lane_id(Cardinal::East, Route::Left)).unwrap();
        assert!(east_right.spawn_point.y < east_left.spawn_point.y);
        assert!(east_right.spawn_point.y < INTERSECTION_CENTER_Y);
        assert!(east_left.spawn_point.y < INTERSECTION_CENTER_Y);

        let west_right = model.lane(lane_id(Cardinal::West, Route::Right)).unwrap();
        let west_left = model.lane(lane_id(Cardinal::West, Route::Left)).unwrap();
        assert!(west_right.spawn_point.y > west_left.spawn_point.y);
        assert!(west_right.spawn_point.y > INTERSECTION_CENTER_Y);
        assert!(west_left.spawn_point.y > INTERSECTION_CENTER_Y);
    }
}
