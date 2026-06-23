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
        smart.update(spawn.vehicles_mut(), &model, FIXED_TIMESTEP_SECS);
        let _ = spawn.update(&model, FIXED_TIMESTEP_SECS);
        if spawn.vehicles()[0].state == VehicleState::Managed {
            break;
        }
    }
    assert_eq!(
        spawn.vehicles()[0].state,
        VehicleState::Managed,
        "vehicle should enter Managed after spawn physics + smart detection"
    );
    assert!(
        spawn.vehicles()[0].time_in_crossing <= FIXED_TIMESTEP_SECS,
        "crossing timer starts at detection; at most one tick may elapse in the break frame"
    );
    assert!(
        spawn.vehicles()[0].distance_in_crossing
            <= spawn.vehicles()[0].velocity * FIXED_TIMESTEP_SECS + 0.01,
        "at most one movement step after managed detection in the break frame"
    );
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
    spawn.vehicles_mut()[0].nominal_velocity = 0.0;
    spawn.vehicles_mut()[0].commanded_velocity = 0.0;
    spawn.vehicles_mut()[0].velocity = 0.0;
    spawn.vehicles_mut()[1].position = Vec2::new(lane_x, 500.0 + SAFE_DISTANCE * 2.0);
    spawn.vehicles_mut()[1].nominal_velocity = fast_speed;
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
        smart.update(spawn.vehicles_mut(), &model, FIXED_TIMESTEP_SECS);
        let exited = spawn.update(&model, FIXED_TIMESTEP_SECS);
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
        smart.update(spawn.vehicles_mut(), &model, FIXED_TIMESTEP_SECS);
        let exited = spawn.update(&model, FIXED_TIMESTEP_SECS);
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
    let mut saw_scheduler_yield = false;
    let mut saw_scheduler_range = false;

    for _ in 0..1200 {
        smart.update(spawn.vehicles_mut(), &model, FIXED_TIMESTEP_SECS);
        let vehicles = spawn.vehicles();
        if SmartController::managed_vehicles_in_scheduler_range(vehicles) {
            saw_scheduler_range = true;
        }
        if SmartController::managed_scheduler_yielded(vehicles) {
            saw_scheduler_yield = true;
        }
        let _ = spawn.update(&model, FIXED_TIMESTEP_SECS);

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

    assert!(
        !saw_scheduler_range || saw_scheduler_yield,
        "cross-traffic scheduler must command yield when managed vehicles are in range"
    );
}

mod audit_sim {
    use super::*;
    use smart_road::config::{SPAWN_COOLDOWN_MS, VEHICLE_LENGTH};
    use std::thread;
    use std::time::Duration;

    /// Run smart-road tick loop; panic on overlap; return when all vehicles exit.
    pub fn run_until_all_exited(
        spawn: &mut SpawnSystem,
        model: &IntersectionModel,
        max_frames: u32,
    ) {
        let mut smart = SmartController::new();
        let collision_threshold = VEHICLE_LENGTH * 0.9;
        let mut saw_scheduler_yield = false;
        let mut saw_scheduler_range = false;

        for _ in 0..max_frames {
            smart.update(spawn.vehicles_mut(), model, FIXED_TIMESTEP_SECS);
            let vehicles = spawn.vehicles();
            if SmartController::managed_vehicles_in_scheduler_range(vehicles) {
                saw_scheduler_range = true;
            }
            if SmartController::managed_scheduler_yielded(vehicles) {
                saw_scheduler_yield = true;
            }
            let _ = spawn.update(model, FIXED_TIMESTEP_SECS);

            let vehicles = spawn.vehicles();
            for i in 0..vehicles.len() {
                for j in (i + 1)..vehicles.len() {
                    let dx = vehicles[i].position.x - vehicles[j].position.x;
                    let dy = vehicles[i].position.y - vehicles[j].position.y;
                    let gap = (dx * dx + dy * dy).sqrt();
                    assert!(
                        gap >= collision_threshold,
                        "vehicles overlapped (gap={gap}, ids {:?} vs {:?})",
                        vehicles[i].id,
                        vehicles[j].id
                    );
                }
            }

            if spawn.vehicles().is_empty() {
                assert!(
                    !saw_scheduler_range || saw_scheduler_yield,
                    "C02 scheduler must command yield when managed vehicles are in range"
                );
                return;
            }
        }

        panic!(
            "vehicles still on canvas after {max_frames} frames (count={})",
            spawn.vehicles().len()
        );
    }

    pub fn spawn_with_cooldown(
        spawn: &mut SpawnSystem,
        model: &IntersectionModel,
        approach: Cardinal,
        route: Route,
        count: usize,
    ) {
        for i in 0..count {
            if i > 0 {
                thread::sleep(Duration::from_millis(SPAWN_COOLDOWN_MS + 10));
            }
            spawn
                .try_spawn(SpawnRequest::new(approach, route), model)
                .expect("spawn");
        }
    }

    pub fn spawn_on_approach_with_cooldown(
        spawn: &mut SpawnSystem,
        model: &IntersectionModel,
        approach: Cardinal,
        count: usize,
    ) {
        for i in 0..count {
            if i > 0 {
                thread::sleep(Duration::from_millis(SPAWN_COOLDOWN_MS + 10));
            }
            spawn
                .spawn_on_approach(approach, model)
                .expect("spawn on approach");
        }
    }
}

/// AUD-8: three vehicles same lane per approach — no collision.
#[test]
fn crate_smoke_audit8_three_same_lane_all_approaches() {
    let model = IntersectionModel::new();

    for approach in Cardinal::ALL {
        let mut spawn = SpawnSystem::new();
        audit_sim::spawn_with_cooldown(&mut spawn, &model, approach, Route::Straight, 3);
        assert_eq!(spawn.vehicles().len(), 3);
        audit_sim::run_until_all_exited(&mut spawn, &model, 3000);
    }
}

/// AUD-9: one West + three East entries (rotating east routes per arrow-left).
#[test]
fn crate_smoke_audit9_one_west_three_east() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::West, Route::Straight), &model)
        .expect("west");
    audit_sim::spawn_on_approach_with_cooldown(&mut spawn, &model, Cardinal::East, 3);
    audit_sim::run_until_all_exited(&mut spawn, &model, 5000);
}

/// AUD-10: one South + three East entries.
#[test]
fn crate_smoke_audit10_one_south_three_east() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
        .expect("south");
    audit_sim::spawn_with_cooldown(&mut spawn, &model, Cardinal::East, Route::Straight, 3);
    audit_sim::run_until_all_exited(&mut spawn, &model, 5000);
}

/// AUD-11: one South + three West entries.
#[test]
fn crate_smoke_audit11_one_south_three_west() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
        .expect("south");
    audit_sim::spawn_with_cooldown(&mut spawn, &model, Cardinal::West, Route::Straight, 3);
    audit_sim::run_until_all_exited(&mut spawn, &model, 5000);
}

/// AUD-12: one North + three East entries.
#[test]
fn crate_smoke_audit12_one_north_three_east() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::North, Route::Straight), &model)
        .expect("north");
    audit_sim::spawn_with_cooldown(&mut spawn, &model, Cardinal::East, Route::Straight, 3);
    audit_sim::run_until_all_exited(&mut spawn, &model, 5000);
}

/// AUD-13: one North + three West entries.
#[test]
fn crate_smoke_audit13_one_north_three_west() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::North, Route::Straight), &model)
        .expect("north");
    audit_sim::spawn_with_cooldown(&mut spawn, &model, Cardinal::West, Route::Straight, 3);
    audit_sim::run_until_all_exited(&mut spawn, &model, 5000);
}

/// AUD-14: five South + two West entries.
#[test]
fn crate_smoke_audit14_five_south_two_west() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    audit_sim::spawn_with_cooldown(&mut spawn, &model, Cardinal::South, Route::Straight, 5);
    audit_sim::spawn_with_cooldown(&mut spawn, &model, Cardinal::West, Route::Straight, 2);
    assert_eq!(spawn.vehicles().len(), 7);
    audit_sim::run_until_all_exited(&mut spawn, &model, 5000);
}
