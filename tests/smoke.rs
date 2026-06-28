//! Integration smoke tests (A02) — no SDL2 required.
use std::thread;
use std::time::Duration;

use sdl2::keyboard::Keycode;
use smart_road::config::{
    APPROACH_ARM_LENGTH, DEFAULT_SPAWN_VELOCITY, FIXED_TIMESTEP_SECS, SPAWN_COOLDOWN_MS,
    STATS_WINDOW_TITLE, TARGET_FPS, VEHICLE_LENGTH, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH,
};
use smart_road::input::{approach_for_arrow, InputEvent, InputState};
use smart_road::intersection::{lane_id, Cardinal, IntersectionModel, Route, Vec2};
use smart_road::smart::SmartController;
use smart_road::spawn::{SpawnRequest, SpawnSystem, VehicleExit};
use smart_road::stats::StatsSession;
use smart_road::stats_window::format_stats_lines;
use smart_road::vehicle::{clamp_velocity_for_proximity, sprite_separation_gap, VehicleState};

/// One simulation tick: smart (schedule) → physics → zone gate → proximity clamp.
fn simulation_tick(
    spawn: &mut SpawnSystem,
    smart: &mut SmartController,
    model: &IntersectionModel,
) -> Vec<VehicleExit> {
    smart.update(spawn.vehicles_mut(), model, FIXED_TIMESTEP_SECS);
    let exited = spawn.update(model, FIXED_TIMESTEP_SECS);
    SmartController::enforce_zone_gate(spawn.vehicles_mut(), model);
    clamp_velocity_for_proximity(spawn.vehicles_mut(), model);
    exited
}
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
    let frames_to_zone = ((APPROACH_ARM_LENGTH / DEFAULT_SPAWN_VELOCITY) / FIXED_TIMESTEP_SECS)
        .ceil() as u32
        + 60;
    for _ in 0..frames_to_zone {
        simulation_tick(&mut spawn, &mut smart, &model);
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
        let exited = simulation_tick(&mut spawn, &mut smart, &model);
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
        let exited = simulation_tick(&mut spawn, &mut smart, &model);
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
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();

    spawn
        .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
        .expect("south spawn");
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::East, Route::Straight), &model)
        .expect("east spawn");

    let mut saw_scheduler_yield = false;
    let mut saw_scheduler_range = false;

    for _ in 0..1200 {
        simulation_tick(&mut spawn, &mut smart, &model);
        let vehicles = spawn.vehicles();
        if SmartController::managed_vehicles_in_scheduler_range(vehicles) {
            saw_scheduler_range = true;
        }
        if SmartController::managed_scheduler_yielded(vehicles) {
            saw_scheduler_yield = true;
        }

        let vehicles = spawn.vehicles();
        for i in 0..vehicles.len() {
            for j in (i + 1)..vehicles.len() {
                let sep = sprite_separation_gap(&vehicles[i], &vehicles[j]);
                assert!(
                    sep >= 0.0,
                    "vehicle sprites overlapped (sep={sep})"
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
    use smart_road::config::SPAWN_COOLDOWN_MS;
    use std::thread;
    use std::time::Duration;

    /// Run smart-road tick loop; panic on overlap; return when all vehicles exit.
    pub fn run_until_all_exited(
        spawn: &mut SpawnSystem,
        model: &IntersectionModel,
        max_frames: u32,
    ) {
        let mut smart = SmartController::new();
        let mut saw_scheduler_yield = false;
        let mut saw_scheduler_range = false;

        for _ in 0..max_frames {
            simulation_tick(spawn, &mut smart, model);
            let vehicles = spawn.vehicles();
            if SmartController::managed_vehicles_in_scheduler_range(vehicles) {
                saw_scheduler_range = true;
            }
            if SmartController::managed_scheduler_yielded(vehicles) {
                saw_scheduler_yield = true;
            }

            let vehicles = spawn.vehicles();
            for i in 0..vehicles.len() {
                for j in (i + 1)..vehicles.len() {
                    let sep = sprite_separation_gap(&vehicles[i], &vehicles[j]);
                    assert!(
                        sep >= 0.0,
                        "vehicles overlapped (sep={sep}, ids {:?} vs {:?})",
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

/// AUD-15: scheduler commands a yield on a conflict lane pair without clamp_velocity_for_proximity.
///
/// Bypasses SpawnSystem.update() (which calls the proximity clamp) to prove the smart
/// scheduler — not the safety net — is responsible for reducing commanded_velocity.
#[test]
fn crate_smoke_aud15_scheduler_yields_without_proximity_clamp() {
    use smart_road::config::{
        FIXED_TIMESTEP_SECS, INTERSECTION_CENTER_X, INTERSECTION_CENTER_Y, SAFE_DISTANCE,
    };
    use smart_road::vehicle::{Vehicle, VehicleId, VelocityLevel};

    let model = IntersectionModel::new();
    let mut smart = SmartController::new();
    let nominal = VelocityLevel::Fast.speed();
    let center = Vec2::new(INTERSECTION_CENTER_X, INTERSECTION_CENTER_Y);

    // Phase 1: register South-Straight vehicle as Managed first (entry_sequence = 0).
    let mut vehicles = vec![Vehicle {
        id: VehicleId(1),
        lane_id: lane_id(Cardinal::South, Route::Straight),
        route: Route::Straight,
        approach: Cardinal::South,
        position: center,
        heading_rad: Cardinal::South.travel_heading(),
        velocity: nominal,
        commanded_velocity: nominal,
        nominal_velocity: nominal,
        state: VehicleState::Approaching,
        path_index: 0,
        distance_in_crossing: 0.0,
        time_in_crossing: 0.0,
        reservation_granted: false,
        scheduler_yield: false,
        reservation_hold: false,
    }];
    smart.update(&mut vehicles, &model, FIXED_TIMESTEP_SECS);
    assert_eq!(
        vehicles[0].state,
        VehicleState::Managed,
        "south vehicle must enter Managed zone"
    );

    // Phase 2: add East-Straight vehicle already Managed (reservation gate would block Approaching).
    // South-Straight and East-Straight are confirmed conflicting lanes.
    let east = Vehicle {
        id: VehicleId(2),
        lane_id: lane_id(Cardinal::East, Route::Straight),
        route: Route::Straight,
        approach: Cardinal::East,
        position: Vec2::new(center.x + SAFE_DISTANCE * 0.5, center.y),
        heading_rad: Cardinal::East.travel_heading(),
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
    };
    smart.register_managed_entry(east.id, 1);
    vehicles.push(east);
    smart.update(&mut vehicles, &model, FIXED_TIMESTEP_SECS);
    assert_eq!(
        vehicles[1].state,
        VehicleState::Managed,
        "east vehicle must remain Managed for in-zone scheduler test"
    );

    // Record positions before the scheduler-only update.
    let pos_before: Vec<Vec2> = vehicles.iter().map(|v| v.position).collect();

    // Phase 3: run the smart controller ONLY — no advance_along_path, no clamp_velocity_for_proximity.
    smart.update(&mut vehicles, &model, FIXED_TIMESTEP_SECS);

    // Positions must not change: smart.update() never writes position.
    for (v, &pos) in vehicles.iter().zip(pos_before.iter()) {
        assert_eq!(
            v.position.x, pos.x,
            "vehicle {:?} position.x must not change during scheduler-only update",
            v.id
        );
        assert_eq!(
            v.position.y, pos.y,
            "vehicle {:?} position.y must not change during scheduler-only update",
            v.id
        );
    }

    // The scheduler must have reduced commanded_velocity for the later-entry (East) vehicle.
    assert!(
        SmartController::managed_scheduler_yielded(&vehicles),
        "scheduler must command a yield when conflicting managed vehicles are in range; \
         east commanded_velocity={:.1}, nominal={nominal:.1}",
        vehicles[1].commanded_velocity
    );
}

/// AUD-16 + AUD-17: 60-second sustained simulation — no overlap, no lane overflow.
#[test]
fn crate_smoke_aud16_aud17_sustained_no_overlap_no_lane_overflow() {
    use std::collections::HashMap;

    use smart_road::spawn::LANE_CAPACITY;

    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();

    // Seed traffic on all four approaches (no cooldown conflict between different approaches).
    spawn.try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model);
    spawn.try_spawn(SpawnRequest::new(Cardinal::North, Route::Straight), &model);
    spawn.try_spawn(SpawnRequest::new(Cardinal::East, Route::Straight), &model);
    spawn.try_spawn(SpawnRequest::new(Cardinal::West, Route::Straight), &model);

    let total_frames = (60.0 / FIXED_TIMESTEP_SECS) as u32;

    // Tracks that at least one frame had vehicles present (proves assertions are non-vacuous).
    let mut saw_vehicles = false;

    for frame in 0..total_frames {
        // Backdate all per-direction cooldowns so spawn_random fires this frame.
        // Wall-clock time is near-zero in tests, so without this the cooldown would
        // always block and no random spawns would occur (the original vacuous bug).
        spawn.force_cooldowns_expired();
        spawn.spawn_random(&model);

        simulation_tick(&mut spawn, &mut smart, &model);

        let vehicles = spawn.vehicles();

        if !vehicles.is_empty() {
            saw_vehicles = true;
        }

        // AUD-16: no two vehicles may overlap.
        for i in 0..vehicles.len() {
            for j in (i + 1)..vehicles.len() {
                let sep = sprite_separation_gap(&vehicles[i], &vehicles[j]);
                assert!(
                    sep >= 0.0,
                    "frame {frame}: vehicles {:?} and {:?} overlapped (sep={sep:.2})",
                    vehicles[i].id,
                    vehicles[j].id
                );
            }
        }

        // AUD-17: no single lane may exceed the congestion cap.
        let mut lane_counts: HashMap<_, u32> = HashMap::new();
        for v in vehicles {
            *lane_counts.entry(v.lane_id).or_default() += 1;
        }
        for (&lid, &count) in &lane_counts {
            assert!(
                count <= LANE_CAPACITY as u32,
                "frame {frame}: lane {lid:?} has {count} vehicles, exceeds cap of {LANE_CAPACITY}"
            );
        }
    }

    assert!(
        saw_vehicles,
        "vehicles must be present during at least one assertion frame"
    );
}

/// AUD-17 unit: the `LANE_CAPACITY` guard blocks exactly the ninth spawn on a full lane.
#[test]
fn crate_unit_lane_cap_blocks_ninth_spawn() {
    use smart_road::spawn::LANE_CAPACITY;

    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let req = SpawnRequest::new(Cardinal::South, Route::Straight);

    // Fill the lane to capacity; expire cooldowns between each spawn so the wall-clock
    // gate never interferes with the cap check.
    for i in 0..LANE_CAPACITY {
        spawn.force_cooldowns_expired();
        assert!(
            spawn.try_spawn(req, &model).is_some(),
            "spawn {i} must succeed while lane holds fewer than LANE_CAPACITY vehicles"
        );
    }
    assert_eq!(
        spawn
            .vehicles()
            .iter()
            .filter(|v| v.lane_id == req.lane_id)
            .count(),
        LANE_CAPACITY,
        "lane must hold exactly LANE_CAPACITY vehicles after filling"
    );

    // The ninth attempt must be rejected by the cap, not the cooldown.
    spawn.force_cooldowns_expired();
    assert!(
        spawn.try_spawn(req, &model).is_none(),
        "try_spawn must return None when lane is already at LANE_CAPACITY"
    );
}

fn simulate_session_tick(
    spawn: &mut SpawnSystem,
    smart: &mut SmartController,
    stats: &mut StatsSession,
    model: &IntersectionModel,
    session_time: &mut f32,
) {
    *session_time += FIXED_TIMESTEP_SECS;
    let exited = simulation_tick(spawn, smart, model);
    stats.observe_vehicles(spawn.vehicles(), *session_time);
    for exit in exited {
        stats.record_exit(exit.id, exit.time_in_crossing);
    }
}

fn spawn_cardinal_with_cooldown(
    spawn: &mut SpawnSystem,
    approach: Cardinal,
    model: &IntersectionModel,
    count: u32,
) {
    for _ in 0..count {
        spawn.spawn_on_approach(approach, model);
        thread::sleep(Duration::from_millis(SPAWN_COOLDOWN_MS + 10));
    }
}

fn vehicles_overlap(a: &smart_road::vehicle::Vehicle, b: &smart_road::vehicle::Vehicle) -> bool {
    let dx = a.position.x - b.position.x;
    let dy = a.position.y - b.position.y;
    let min_gap = VEHICLE_LENGTH * 0.85;
    dx * dx + dy * dy < min_gap * min_gap
}

fn assert_no_collisions(vehicles: &[smart_road::vehicle::Vehicle]) {
    for i in 0..vehicles.len() {
        if vehicles[i].state == VehicleState::Done {
            continue;
        }
        for j in (i + 1)..vehicles.len() {
            if vehicles[j].state == VehicleState::Done {
                continue;
            }
            assert!(
                !vehicles_overlap(&vehicles[i], &vehicles[j]),
                "AUD-18: vehicles {:?} and {:?} overlapped at ({:.1}, {:.1}) vs ({:.1}, {:.1})",
                vehicles[i].id,
                vehicles[j].id,
                vehicles[i].position.x,
                vehicles[i].position.y,
                vehicles[j].position.x,
                vehicles[j].position.y
            );
        }
    }
}

fn assert_stats_window_fields(stats: &smart_road::stats::Stats, session_time: f32) {
    let mut display_stats = stats.clone();
    display_stats.finalize_session(session_time);
    let lines = format_stats_lines(&display_stats);
    let joined = lines.join("\n");

    assert!(
        joined.contains("Max vehicles passed:"),
        "AUD-20 label missing"
    );
    assert!(joined.contains("Max velocity:"), "AUD-21 max label missing");
    assert!(joined.contains("Min velocity:"), "AUD-21 min label missing");
    assert!(
        joined.contains("Max time to pass intersection (s):"),
        "AUD-22 label missing"
    );
    assert!(
        joined.contains("Min time to pass intersection (s):"),
        "AUD-23 label missing"
    );
    assert!(joined.contains("Close calls:"), "AUD-24 label missing");

    assert!(
        !joined.contains("Min velocity: N/A") || stats.min_velocity == f32::MAX,
        "min velocity should be numeric after vehicles moved"
    );
    assert!(
        !joined.contains("Max velocity: 0"),
        "max velocity should be positive after vehicles moved"
    );
    assert!(
        joined.contains("Additional statistics (bonus)"),
        "AUD-B1 bonus section missing"
    );
    assert!(
        joined.contains("Session duration (s):"),
        "AUD-B1 session duration label missing"
    );
    assert!(
        display_stats.session_duration_secs > 0.0,
        "AUD-B1 session duration should be positive after simulation"
    );
    assert!(
        joined.contains("Avg crossing time (s):"),
        "AUD-B1 avg crossing time label missing"
    );
    assert!(
        display_stats.avg_crossing_time_secs > 0.0,
        "AUD-B1 avg crossing time should be positive after vehicles crossed"
    );
    assert!(
        joined.contains("Peak concurrent in zone:"),
        "AUD-B1 peak concurrent label missing"
    );
    assert!(
        display_stats.peak_concurrent_in_zone > 0,
        "AUD-B1 peak concurrent should be positive when vehicles crossed zone"
    );
    assert!(
        joined.contains("Vehicles entered zone:"),
        "AUD-B1 vehicles entered zone label missing"
    );
    assert!(
        display_stats.vehicles_entered_zone > 0,
        "AUD-B1 vehicles entered zone should count managed entries"
    );
}

/// AUD-18/20–24 mirror: two Arrow Up + two Arrow Right, all cross without overlap.
#[test]
fn crate_smoke_audit18_four_vehicle_session_no_collision() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();
    let mut stats = StatsSession::new();
    let mut session_time = 0.0_f32;

    spawn_cardinal_with_cooldown(&mut spawn, Cardinal::South, &model, 2);
    spawn_cardinal_with_cooldown(&mut spawn, Cardinal::West, &model, 2);

    let deadline = 120.0_f32;
    while stats.stats.vehicles_passed < 4 && session_time < deadline {
        simulate_session_tick(
            &mut spawn,
            &mut smart,
            &mut stats,
            &model,
            &mut session_time,
        );
        assert_no_collisions(spawn.vehicles());
    }

    assert_eq!(
        stats.stats.vehicles_passed, 4,
        "AUD-18/20: expected four completed crossings"
    );
    assert_eq!(stats.stats.max_vehicles_passed, 4);

    assert_stats_window_fields(&stats.stats, session_time);
    let mut display_stats = stats.stats.clone();
    display_stats.finalize_session(session_time);
    let lines = format_stats_lines(&display_stats);
    assert!(lines.iter().any(|l| l.contains("Max vehicles passed: 4")));
    assert!(lines.iter().any(|l| l.contains("Close calls: 0")));
}

/// AUD-25 mirror: one vehicle — max crossing time equals min crossing time.
#[test]
fn crate_smoke_audit25_single_vehicle_equal_crossing_times() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();
    let mut stats = StatsSession::new();
    let mut session_time = 0.0_f32;

    spawn
        .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
        .expect("spawn succeeds");

    while stats.stats.vehicles_passed < 1 && session_time < 60.0 {
        simulate_session_tick(
            &mut spawn,
            &mut smart,
            &mut stats,
            &model,
            &mut session_time,
        );
    }

    assert_eq!(stats.stats.vehicles_passed, 1);
    assert!(
        (stats.stats.max_crossing_time - stats.stats.min_crossing_time).abs() < f32::EPSILON,
        "AUD-25: max and min crossing time should match for one vehicle"
    );

    assert_stats_window_fields(&stats.stats, session_time);
    let mut display_stats = stats.stats.clone();
    display_stats.finalize_session(session_time);
    let lines = format_stats_lines(&display_stats);
    let max_line = lines
        .iter()
        .find(|l| l.starts_with("Max time"))
        .expect("max line");
    let min_line = lines
        .iter()
        .find(|l| l.starts_with("Min time"))
        .expect("min line");
    assert_eq!(
        max_line.split(": ").nth(1),
        min_line.split(": ").nth(1),
        "AUD-25: formatted max/min crossing times must match"
    );
}

/// AUD-19 structural check: stats UI uses a distinct window title from the sim window.
#[test]
fn crate_smoke_audit19_stats_window_is_separate_surface() {
    use smart_road::config::WINDOW_TITLE;

    assert_ne!(
        STATS_WINDOW_TITLE, WINDOW_TITLE,
        "AUD-19: stats window must be a separate surface from the simulation window"
    );
    assert!(STATS_WINDOW_TITLE.contains("statistics"));
}

/// Cross-traffic waiter must not creep while blocked at the junction (↑ then →).
#[test]
fn crate_smoke_cross_traffic_waiter_stays_still() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();

    spawn
        .try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model)
        .unwrap();
    spawn
        .try_spawn(SpawnRequest::new(Cardinal::West, Route::Straight), &model)
        .unwrap();

    let mut max_wait_delta = 0.0f32;

    for _ in 0..900 {
        let before: Vec<_> = spawn
            .vehicles()
            .iter()
            .map(|v| {
                (
                    v.id,
                    v.position,
                    v.reservation_granted,
                    v.reservation_hold,
                    v.scheduler_yield,
                )
            })
            .collect();

        simulation_tick(&mut spawn, &mut smart, &model);

        let after = spawn.vehicles();
        for (id, pos, _granted, hold, sched_yield) in before {
            let Some(v) = after.iter().find(|x| x.id == id) else {
                continue;
            };
            let dx = v.position.x - pos.x;
            let dy = v.position.y - pos.y;
            let delta = (dx * dx + dy * dy).sqrt();

            let waiting =
                hold || v.reservation_hold || sched_yield || v.scheduler_yield;

            if waiting {
                max_wait_delta = max_wait_delta.max(delta);
            }
        }

        if spawn.vehicles().is_empty() {
            break;
        }
    }

    assert!(
        max_wait_delta < 0.01,
        "blocked cross-traffic waiter moved {max_wait_delta:.4} px in one frame"
    );
}
