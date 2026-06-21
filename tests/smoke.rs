//! Integration smoke tests (A02) — no SDL2 required.
use sdl2::keyboard::Keycode;
use smart_road::config::{
    FIXED_TIMESTEP_SECS, TARGET_FPS, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH,
};
use smart_road::input::{approach_for_arrow, InputEvent, InputState};
use smart_road::intersection::{lane_id, Cardinal, IntersectionModel, Route, Vec2};
use smart_road::smart::SmartController;
use smart_road::spawn::{SpawnRequest, SpawnSystem};
use smart_road::stats::StatsSession;
use smart_road::vehicle::VehicleState;
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
    let _ = StatsSession::new();
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
#[test]
fn crate_smoke_spawn_smart_detection_pipeline() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
        .expect("spawn succeeds");
    assert_eq!(spawn.vehicles()[0].state, VehicleState::Approaching);
    for _ in 0..200 {
        spawn.update(&model, FIXED_TIMESTEP_SECS);
        smart.update(spawn.vehicles_mut(), &model, FIXED_TIMESTEP_SECS);
        if spawn.vehicles()[0].state == VehicleState::Managed {
            break;
        }
    }
    assert_eq!(
        spawn.vehicles()[0].state,
        VehicleState::Managed,
        "vehicle should enter Managed after spawn physics + smart detection"
    );
    assert_eq!(spawn.vehicles()[0].time_in_crossing, 0.0);
    assert_eq!(spawn.vehicles()[0].distance_in_crossing, 0.0);
}

#[test]
fn crate_smoke_same_approach_follower_slows_behind_stopped_leader() {
    use std::thread;
    use std::time::Duration;

    use smart_road::config::{SAFE_DISTANCE, SPAWN_COOLDOWN_MS};
    use smart_road::vehicle::VelocityLevel;

    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let req = SpawnRequest::new(Cardinal::South, Route::Straight);
    let fast_speed = VelocityLevel::Fast.speed();

    spawn.try_spawn(req, &model).expect("leader spawn succeeds");
    thread::sleep(Duration::from_millis(SPAWN_COOLDOWN_MS + 10));
    spawn
        .try_spawn(req, &model)
        .expect("follower spawn succeeds after cooldown");

    assert_eq!(spawn.vehicles().len(), 2);
    assert_eq!(spawn.vehicles()[0].lane_id, spawn.vehicles()[1].lane_id);
    assert_eq!(spawn.vehicles()[0].approach, Cardinal::South);

    let lane_x = spawn.vehicles()[0].position.x;
    spawn.vehicles_mut()[0].position = Vec2::new(lane_x, 500.0);
    spawn.vehicles_mut()[0].commanded_velocity = 0.0;
    spawn.vehicles_mut()[0].velocity = 0.0;
    spawn.vehicles_mut()[1].position = Vec2::new(lane_x, 500.0 + SAFE_DISTANCE * 2.0);
    spawn.vehicles_mut()[1].commanded_velocity = fast_speed;
    spawn.vehicles_mut()[1].velocity = fast_speed;

    let mut saw_slowdown = false;
    for _ in 0..300 {
        let _ = spawn.update(&model, FIXED_TIMESTEP_SECS);

        assert_eq!(
            spawn.vehicles().len(),
            2,
            "both vehicles must stay on canvas during follow-distance scenario"
        );

        let leader = &spawn.vehicles()[0];
        let follower = &spawn.vehicles()[1];
        let gap = follower.position.y - leader.position.y;

        assert!(
            gap >= SAFE_DISTANCE * 0.9,
            "follower must stay behind stopped leader (gap={gap})"
        );

        if follower.velocity < fast_speed {
            saw_slowdown = true;
            break;
        }
    }

    assert!(
        saw_slowdown,
        "follower should slow behind stopped leader on same lane"
    );
}

#[test]
fn crate_smoke_stats_collector_pipeline() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();
    let mut stats = StatsSession::new();
    let mut session_time = 0.0_f32;

    spawn
        .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
        .expect("spawn succeeds");

    let mut recorded_exit = false;
    for _ in 0..800 {
        session_time += FIXED_TIMESTEP_SECS;
        let exited = spawn.update(&model, FIXED_TIMESTEP_SECS);
        smart.update(spawn.vehicles_mut(), &model, FIXED_TIMESTEP_SECS);
        stats.observe_vehicles(spawn.vehicles(), session_time);
        for exit in exited {
            stats.record_exit(exit.id, exit.time_in_crossing);
            recorded_exit = true;
        }
        if recorded_exit {
            break;
        }
    }

    assert!(recorded_exit, "vehicle should exit after crossing");
    assert_eq!(stats.stats.vehicles_passed, 1);
    assert!(stats.stats.max_velocity > 0.0);
    assert!(stats.stats.max_crossing_time > 0.0);
}

#[test]
fn crate_smoke_session_stats_populated_before_esc_exit() {
    use std::thread;
    use std::time::Duration;

    use smart_road::config::SPAWN_COOLDOWN_MS;
    use smart_road::input::{InputEvent, InputState};

    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();
    let mut stats = StatsSession::new();
    let mut input = InputState::new();
    let mut session_time = 0.0_f32;
    let mut running = true;

    input.on_key_down(Some(Keycode::Up));
    for event in input.drain_events() {
        if let InputEvent::SpawnCardinal(approach) = event {
            spawn.spawn_on_approach(approach, &model);
        }
    }

    thread::sleep(Duration::from_millis(SPAWN_COOLDOWN_MS + 10));
    input.on_key_down(Some(Keycode::Up));
    for event in input.drain_events() {
        if let InputEvent::SpawnCardinal(approach) = event {
            spawn.spawn_on_approach(approach, &model);
        }
    }

    while running {
        for event in input.drain_events() {
            match event {
                InputEvent::SpawnCardinal(approach) => {
                    spawn.spawn_on_approach(approach, &model);
                }
                InputEvent::Exit => running = false,
                _ => {}
            }
        }

        session_time += FIXED_TIMESTEP_SECS;
        let exited = spawn.update(&model, FIXED_TIMESTEP_SECS);
        smart.update(spawn.vehicles_mut(), &model, FIXED_TIMESTEP_SECS);
        stats.observe_vehicles(spawn.vehicles(), session_time);
        for exit in exited {
            stats.record_exit(exit.id, exit.time_in_crossing);
        }

        if stats.stats.vehicles_passed >= 1 {
            input.on_key_down(Some(Keycode::Escape));
        }

        if session_time > 30.0 {
            break;
        }
    }

    assert!(
        stats.stats.vehicles_passed >= 1,
        "at least one vehicle should complete crossing before Esc"
    );
    assert!(stats.stats.max_velocity > 0.0);
    assert!(stats.stats.min_velocity < f32::MAX);
    assert!(stats.stats.max_crossing_time > 0.0);
    assert!(stats.stats.min_crossing_time < f32::MAX);
    assert_eq!(stats.stats.max_vehicles_passed, stats.stats.vehicles_passed);
}

#[test]
fn crate_smoke_cross_traffic_scheduler_avoids_overlap() {
    use smart_road::config::VEHICLE_LENGTH;

    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();

    spawn
        .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
        .expect("south spawn");
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::East, Route::Straight), &model)
        .expect("east spawn");

    let collision_threshold = VEHICLE_LENGTH * 0.9;

    for _ in 0..1200 {
        let _ = spawn.update(&model, FIXED_TIMESTEP_SECS);
        smart.update(spawn.vehicles_mut(), &model, FIXED_TIMESTEP_SECS);

        let vehicles = spawn.vehicles();
        for i in 0..vehicles.len() {
            for j in (i + 1)..vehicles.len() {
                let dx = vehicles[i].position.x - vehicles[j].position.x;
                let dy = vehicles[i].position.y - vehicles[j].position.y;
                let gap = (dx * dx + dy * dy).sqrt();
                assert!(
                    gap >= collision_threshold,
                    "vehicles overlapped (gap={gap})"
                );
            }
        }

        if spawn.vehicles().is_empty() {
            break;
        }
    }
}
