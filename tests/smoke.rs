//! Integration smoke tests (A02) — no SDL2 required.

use sdl2::keyboard::Keycode;
use smart_road::config::{
    FIXED_TIMESTEP_SECS, TARGET_FPS, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH,
};
use smart_road::input::{approach_for_arrow, InputEvent, InputState};
use smart_road::intersection::{lane_id, Cardinal, IntersectionModel, Route};
use smart_road::smart::SmartController;
use smart_road::spawn::{SpawnRequest, SpawnSystem};
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

#[test]
fn crate_smoke_arrow_spawn_pipeline() {
    let model = IntersectionModel::new();
    let cases = [
        (Keycode::Up, Cardinal::South),
        (Keycode::Down, Cardinal::North),
        (Keycode::Right, Cardinal::West),
        (Keycode::Left, Cardinal::East),
    ];

    for (key, expected_approach) in cases {
        let approach = approach_for_arrow(key).expect("arrow key maps to approach");
        assert_eq!(approach, expected_approach);

        let mut spawn = SpawnSystem::new();
        let id = spawn
            .try_spawn(SpawnRequest::new(approach, Route::Straight), &model)
            .expect("spawn succeeds");
        assert_eq!(spawn.vehicles().len(), 1);
        assert_eq!(spawn.vehicles()[0].id, id);
        assert_eq!(spawn.vehicles()[0].approach, expected_approach);

        let mut input = InputState::new();
        input.on_key_down(Some(key));
        let events: Vec<_> = input.drain_events().collect();
        assert_eq!(events, vec![InputEvent::SpawnCardinal(expected_approach)]);
    }
}

#[test]
fn crate_smoke_random_spawn_pipeline() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut input = InputState::new();

    input.on_key_down(Some(Keycode::R));
    let events: Vec<_> = input.drain_events().collect();
    assert_eq!(events, vec![InputEvent::RandomStream(true)]);
    assert!(input.random_stream_active());

    if input.random_stream_active() {
        assert!(spawn.spawn_random(&model).is_some());
    }
    assert_eq!(spawn.vehicles().len(), 1);

    input.on_key_up(Some(Keycode::R));
    let events: Vec<_> = input.drain_events().collect();
    assert_eq!(events, vec![InputEvent::RandomStream(false)]);
    assert!(!input.random_stream_active());
}
