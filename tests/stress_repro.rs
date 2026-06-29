//! Regression: heavy spawn spam must not permanently freeze lane queues.
use smart_road::config::FIXED_TIMESTEP_SECS;
use smart_road::intersection::{Cardinal, IntersectionModel};
use smart_road::smart::SmartController;
use smart_road::spawn::SpawnSystem;
use smart_road::vehicle::{resolve_proximity_overlaps, VehicleState};

fn tick(spawn: &mut SpawnSystem, smart: &mut SmartController, model: &IntersectionModel) {
    smart.update(spawn.vehicles_mut(), model, FIXED_TIMESTEP_SECS);
    spawn.update(model, FIXED_TIMESTEP_SECS);
    smart.enforce_zone_gate(spawn.vehicles_mut(), model);
    resolve_proximity_overlaps(spawn.vehicles_mut());
}

#[test]
fn stress_heavy_spawn_does_not_freeze_lanes() {
    let model = IntersectionModel::new();
    let mut spawn = SpawnSystem::new();
    let mut smart = SmartController::new();

    for frame in 0..6000 {
        if frame % 25 == 0 {
            spawn.force_cooldowns_expired();
            for approach in Cardinal::ALL {
                spawn.spawn_on_approach(approach, &model);
            }
            for _ in 0..4 {
                spawn.spawn_random(&model);
            }
        }
        tick(&mut spawn, &mut smart, &model);
    }

    // Let the intersection drain without new spawns — permanent freezes survive this phase.
    for _ in 0..12000 {
        tick(&mut spawn, &mut smart, &model);
    }

    let mut stuck_lanes = Vec::new();
    for lane in 0u32..12 {
        let lane_id = smart_road::intersection::LaneId(lane);
        let on_lane: Vec<_> = spawn
            .vehicles()
            .iter()
            .filter(|v| v.lane_id == lane_id && v.state != VehicleState::Done)
            .collect();
        if on_lane.len() < 2 {
            continue;
        }
        let all_stopped = on_lane
            .iter()
            .all(|v| v.velocity < 0.01 && v.commanded_velocity < 0.01);
        if all_stopped {
            stuck_lanes.push((lane, on_lane.len()));
        }
    }

    assert!(
        stuck_lanes.is_empty(),
        "lanes frozen after heavy spawn stress: {:?}",
        stuck_lanes
    );
}
