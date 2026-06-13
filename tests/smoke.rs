//! Integration smoke tests (A02) — no SDL2 required.

use smart_road::config::{
    FIXED_TIMESTEP_SECS, TARGET_FPS, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH,
};
use smart_road::intersection::{lane_id, Cardinal, IntersectionModel, Route};
use smart_road::smart::SmartController;
use smart_road::spawn::SpawnSystem;
use smart_road::stats::Stats;

#[test]
fn crate_smoke_config_constants() {
    assert_eq!(WINDOW_TITLE, "smart-road");
    assert!(WINDOW_WIDTH >= 640);
    assert!(WINDOW_HEIGHT >= 480);
    assert!(TARGET_FPS > 0);
    assert!(FIXED_TIMESTEP_SECS > 0.0);
}

#[test]
fn crate_smoke_module_defaults_construct() {
    let _ = IntersectionModel::new();
    let _ = SpawnSystem::new();
    let _ = SmartController::new();
    let _ = Stats::new();
}

#[test]
fn crate_smoke_intersection_lane_registry() {
    let model = IntersectionModel::new();
    assert_eq!(model.lanes.len(), 12);
    assert_eq!(model.zone_polygon.len(), 4);
    let south_straight = model.lane(lane_id(Cardinal::South, Route::Straight));
    assert!(south_straight.is_some());
    assert_eq!(model.lanes_for_approach(Cardinal::North).len(), 3);
}
