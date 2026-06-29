//! Autonomous vehicle state and physics (B01+).

use crate::intersection::{
    Cardinal, IntersectionModel, LaneId, LaneInfo, Route, Vec2, VehicleRenderSnapshot,
};

/// Unique vehicle identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VehicleId(pub u64);

/// Lifecycle inside the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleState {
    Approaching,
    Managed,
    Exiting,
    Done,
}

/// Discrete speed levels assigned at spawn (B03; SDS §13.3; REQ-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VelocityLevel {
    Fast,
    Cruise,
    Yield,
}

impl VelocityLevel {
    /// Concrete speed in world units per second, derived from the default cruise speed.
    pub fn speed(self) -> f32 {
        match self {
            VelocityLevel::Fast => crate::config::DEFAULT_SPAWN_VELOCITY * 1.4,
            VelocityLevel::Cruise => crate::config::DEFAULT_SPAWN_VELOCITY * 1.0,
            VelocityLevel::Yield => crate::config::DEFAULT_SPAWN_VELOCITY * 0.5,
        }
    }
}

/// Vehicle simulation state (physics integration in B01).
#[derive(Debug)]
pub struct Vehicle {
    pub id: VehicleId,
    pub lane_id: LaneId,
    pub route: Route,
    pub approach: crate::intersection::Cardinal,
    pub position: Vec2,
    pub heading_rad: f32,
    pub velocity: f32,
    pub commanded_velocity: f32,
    /// Spawn-assigned speed level (B03); restored when follow-distance gap clears (B04).
    pub nominal_velocity: f32,
    pub state: VehicleState,
    pub path_index: usize,
    pub distance_in_crossing: f32,
    pub time_in_crossing: f32,
    /// True when the smart controller granted intersection entry (reservation gate).
    pub reservation_granted: bool,
    /// Latched by the junction scheduler — vehicle must stay at zero until conflicts clear.
    pub scheduler_yield: bool,
    /// Latched when reservation is blocked near the junction — no creep or zone-gate oscillation.
    pub reservation_hold: bool,
}

/// True when an external controller owns this vehicle's speed (scheduler or reservation gate).
pub fn is_motion_frozen(vehicle: &Vehicle) -> bool {
    vehicle.scheduler_yield || vehicle.reservation_hold
}

/// True when a stationary vehicle may begin rolling — avoids move-then-brake creep.
///
/// Vehicles already in motion keep normal follow/proximity braking; this gate only
/// blocks the first acceleration from rest until the path ahead is clear.
pub fn coast_clear_for_departure(
    vehicle: &Vehicle,
    vehicles: &[Vehicle],
    model: &IntersectionModel,
    safe_distance: f32,
) -> bool {
    if is_motion_frozen(vehicle) || vehicle.commanded_velocity < 0.01 {
        return false;
    }
    if vehicle.velocity >= 0.01 {
        return true;
    }

    if vehicle.state == VehicleState::Approaching && !vehicle.reservation_granted {
        if let Some(dist) = distance_to_zone_entry(model, vehicle) {
            if dist > 0.0 && dist <= crate::config::RESERVATION_TRIGGER_DISTANCE {
                return false;
            }
        }
    }

    let mut gap_ahead = f32::INFINITY;
    let mut leader_velocity = 0.0_f32;
    for other in vehicles {
        if other.id == vehicle.id || other.state == VehicleState::Done {
            continue;
        }
        if other.lane_id != vehicle.lane_id {
            continue;
        }
        if let Some(gap) = longitudinal_gap(vehicle, other) {
            if gap < gap_ahead {
                gap_ahead = gap;
                leader_velocity = other.velocity;
            }
        }
    }

    if leader_velocity < 0.01 && gap_ahead < f32::INFINITY {
        let start_buffer = vehicle.nominal_velocity * crate::config::FIXED_TIMESTEP_SECS * 2.0;
        if gap_ahead <= safe_distance + start_buffer {
            return false;
        }
    }

    for other in vehicles {
        if other.id == vehicle.id || other.state == VehicleState::Done {
            continue;
        }
        if sprite_separation_gap(vehicle, other) >= PROXIMITY_BOX_CLEARANCE {
            continue;
        }
        if proximity_yielder_is_a(vehicle, other) {
            return false;
        }
    }

    true
}

fn distance_to_zone_entry(model: &IntersectionModel, vehicle: &Vehicle) -> Option<f32> {
    let zone = &model.zone_polygon;
    if zone.len() < 4 {
        return None;
    }
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

/// Maximum speed that still allows a full stop within `distance` at deceleration `decel`.
pub fn max_speed_to_stop(distance: f32, decel: f32) -> f32 {
    if distance <= 0.0 {
        0.0
    } else {
        (2.0 * decel * distance).max(0.0).sqrt()
    }
}

/// Cap commanded speed so an unreserved vehicle can stop before the junction zone.
pub fn apply_reservation_braking(vehicle: &mut Vehicle, distance_to_zone: f32) {
    let (_, decel_scale) = motion_profile(vehicle.id);
    let decel = crate::config::BASE_DECELERATION * decel_scale;
    let cap = max_speed_to_stop(distance_to_zone, decel);
    vehicle.commanded_velocity = vehicle.commanded_velocity.min(cap);
    if !vehicle.reservation_granted && distance_to_zone <= crate::config::VEHICLE_LENGTH {
        vehicle.commanded_velocity = 0.0;
    }
}

/// Per-vehicle acceleration and deceleration scale (REQ-B3 / AUD-B3).
pub fn motion_profile(id: VehicleId) -> (f32, f32) {
    match id.0 % 3 {
        0 => (1.4, 1.2),
        1 => (1.0, 1.0),
        _ => (0.7, 0.6),
    }
}

/// Ramp `velocity` toward `commanded_velocity` with bounded accel/decel (B05).
pub fn step_velocity_toward_command(vehicle: &mut Vehicle, dt: f32) {
    if is_motion_frozen(vehicle) {
        vehicle.velocity = 0.0;
        vehicle.commanded_velocity = 0.0;
        return;
    }

    let target = vehicle.commanded_velocity;
    let current = vehicle.velocity;
    if (target - current).abs() < 0.01 {
        vehicle.velocity = target;
        return;
    }

    let (accel_scale, decel_scale) = motion_profile(vehicle.id);
    let max_change = if target > current {
        crate::config::BASE_ACCELERATION * accel_scale * dt
    } else {
        crate::config::BASE_DECELERATION * decel_scale * dt
    };
    let delta = (target - current).clamp(-max_change, max_change);
    vehicle.velocity = current + delta;
}

/// Create a vehicle at a lane spawn point (IF-1: B allocates id; A04 factory stub).
pub fn spawn_vehicle(id: VehicleId, lane: &LaneInfo, _velocity: f32) -> Vehicle {
    let level = match id.0 % 3 {
        0 => VelocityLevel::Fast,
        1 => VelocityLevel::Cruise,
        _ => VelocityLevel::Yield,
    };
    let speed = level.speed();
    Vehicle {
        id,
        lane_id: lane.id,
        route: lane.route,
        approach: lane.approach,
        position: lane.spawn_point,
        heading_rad: lane.approach.travel_heading(),
        velocity: speed,
        commanded_velocity: speed,
        nominal_velocity: speed,
        state: VehicleState::Approaching,
        path_index: 0,
        distance_in_crossing: 0.0,
        time_in_crossing: 0.0,
        reservation_granted: false,
        scheduler_yield: false,
        reservation_hold: false,
    }
}

/// Render-facing snapshot (A04 stub; B01 expands).
pub fn snapshot_for_render(vehicle: &Vehicle) -> VehicleRenderSnapshot {
    VehicleRenderSnapshot {
        position: vehicle.position,
        heading_rad: vehicle.heading_rad,
        approach: vehicle.approach,
    }
}

/// Update vehicle physics: advance position and accumulate crossing metrics (B01).
///
/// Used in unit tests and when no lane path drives movement. The live simulation
/// uses [`advance_along_path`] as the single position authority (B04 / DEF-01).
pub fn integrate_physics(vehicle: &mut Vehicle, dt: f32) {
    if vehicle.state == VehicleState::Done {
        return;
    }
    // B05: ramp actual speed toward commanded speed before integrating motion.
    step_velocity_toward_command(vehicle, dt);

    let dx = vehicle.velocity * dt * vehicle.heading_rad.cos();
    let dy = vehicle.velocity * dt * vehicle.heading_rad.sin();
    let distance_moved = (dx * dx + dy * dy).sqrt();

    vehicle.position.x += dx;
    vehicle.position.y += dy;

    if vehicle.state == VehicleState::Managed || vehicle.state == VehicleState::Exiting {
        vehicle.time_in_crossing += dt;
        vehicle.distance_in_crossing += distance_moved;
    }
}

/// Longitudinal distance from `follower` to `leader` when `leader` is ahead on the same lane.
fn longitudinal_gap(follower: &Vehicle, leader: &Vehicle) -> Option<f32> {
    if follower.lane_id != leader.lane_id {
        return None;
    }

    let dx = leader.position.x - follower.position.x;
    let dy = leader.position.y - follower.position.y;
    let along = dx * follower.heading_rad.cos() + dy * follower.heading_rad.sin();

    if along > 0.0 {
        Some(along)
    } else {
        None
    }
}

/// Whether `vehicle` participates in B-side follow-distance logic (SDS §13.3).
fn uses_follow_distance(vehicle: &Vehicle) -> bool {
    matches!(
        vehicle.state,
        VehicleState::Approaching | VehicleState::Exiting
    )
}

/// Cap speeds so same-lane followers keep separation (REQ-8 / REQ-9 / AUD-30).
///
/// Skips vehicles in `Managed` state — the smart controller owns the junction zone (C02+).
/// Uses both center-to-center longitudinal gap and oriented sprite box gap on the same lane.
pub fn enforce_follow_distance(vehicles: &mut [Vehicle], safe_distance: f32) {
    let len = vehicles.len();
    let min_box_gap = safe_distance - crate::config::VEHICLE_LENGTH;
    let dt = crate::config::FIXED_TIMESTEP_SECS;

    for i in 0..len {
        if !uses_follow_distance(&vehicles[i]) || is_motion_frozen(&vehicles[i]) {
            continue;
        }

        let nominal = vehicles[i].nominal_velocity;
        let mut gap_ahead = f32::INFINITY;
        let mut leader_velocity = 0.0_f32;
        let mut leader_idx: Option<usize> = None;

        for j in 0..len {
            if i == j || vehicles[j].state == VehicleState::Done {
                continue;
            }
            if vehicles[i].lane_id != vehicles[j].lane_id {
                continue;
            }

            if let Some(gap) = longitudinal_gap(&vehicles[i], &vehicles[j]) {
                if gap < gap_ahead {
                    gap_ahead = gap;
                    leader_velocity = vehicles[j].velocity;
                    leader_idx = Some(j);
                }
            }
        }

        if gap_ahead > safe_distance {
            vehicles[i].commanded_velocity = nominal;
            continue;
        }

        let mut box_gap = f32::INFINITY;
        if let Some(j) = leader_idx {
            box_gap = sprite_separation_gap(&vehicles[i], &vehicles[j]);
        }

        let center_scale = (gap_ahead / safe_distance).clamp(0.0, 1.0);
        let box_scale = if box_gap.is_finite() && box_gap >= 0.0 {
            (box_gap / min_box_gap).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let scale = if box_gap < 0.0 {
            center_scale
        } else {
            center_scale.min(box_scale)
        };

        let target = if gap_ahead <= safe_distance * 0.1 {
            0.0
        } else if box_gap >= 0.0 && box_gap <= min_box_gap * 0.1 {
            0.0
        } else {
            leader_velocity.min(nominal * scale)
        };

        let capped = if leader_velocity < 0.01 {
            let usable_center = (gap_ahead - safe_distance).max(0.0);
            let usable_box = if box_gap >= 0.0 {
                (box_gap - min_box_gap).max(0.0)
            } else {
                f32::INFINITY
            };
            let max_speed_for_gap = usable_center.min(usable_box) / dt;
            target.min(max_speed_for_gap)
        } else {
            target.min(leader_velocity)
        };

        vehicles[i].commanded_velocity = vehicles[i].commanded_velocity.min(capped);
        vehicles[i].velocity = vehicles[i].velocity.min(capped);
    }
}

/// Apply proximity speed limits before movement (yielder held at zero when too close).
pub fn apply_proximity_speed_limits(vehicles: &mut [Vehicle]) {
    let min_gap = PROXIMITY_BOX_CLEARANCE;
    let len = vehicles.len();

    for i in 0..len {
        if vehicles[i].state == VehicleState::Done {
            continue;
        }
        for j in (i + 1)..len {
            if vehicles[j].state == VehicleState::Done {
                continue;
            }

            let sep = sprite_separation_gap(&vehicles[i], &vehicles[j]);
            if sep >= min_gap {
                continue;
            }

            let a_yields = proximity_yielder_is_a(&vehicles[i], &vehicles[j]);
            let yielder = if a_yields { i } else { j };
            let leader = if a_yields { j } else { i };

            if is_motion_frozen(&vehicles[yielder]) {
                continue;
            }

            vehicles[yielder].commanded_velocity = 0.0;
            vehicles[yielder].velocity = 0.0;

            if sep < 0.0 && vehicles[leader].state == VehicleState::Managed {
                let cap = VelocityLevel::Yield.speed();
                vehicles[leader].commanded_velocity =
                    vehicles[leader].commanded_velocity.min(cap);
                vehicles[leader].velocity = vehicles[leader].velocity.min(cap);
            }
        }
    }
}

/// Single pre-move pass: follow distance, proximity limits, and departure gate.
pub fn apply_pre_move_safety(
    vehicles: &mut [Vehicle],
    model: &IntersectionModel,
    safe_distance: f32,
) {
    enforce_follow_distance(vehicles, safe_distance);
    apply_proximity_speed_limits(vehicles);

    let len = vehicles.len();
    for i in 0..len {
        if vehicles[i].state == VehicleState::Done {
            continue;
        }
        if is_motion_frozen(&vehicles[i]) {
            vehicles[i].velocity = 0.0;
            vehicles[i].commanded_velocity = 0.0;
            continue;
        }
        if vehicles[i].velocity < 0.01 && vehicles[i].commanded_velocity > 0.01
            && !coast_clear_for_departure(&vehicles[i], vehicles, model, safe_distance)
        {
            vehicles[i].velocity = 0.0;
            vehicles[i].commanded_velocity = 0.0;
            continue;
        }
        vehicles[i].velocity = vehicles[i].velocity.min(vehicles[i].commanded_velocity);
    }
}

/// True when two active vehicles are within safe distance (REQ-26 / AUD-24).
///
/// Uses center-to-center Euclidean distance for same-lane and cross-traffic pairs.
pub fn detect_close_call(a: &Vehicle, b: &Vehicle, safe_distance: f32) -> bool {
    if a.id == b.id {
        return false;
    }
    if a.state == VehicleState::Done || b.state == VehicleState::Done {
        return false;
    }

    let dx = b.position.x - a.position.x;
    let dy = b.position.y - a.position.y;
    let dist_sq = dx * dx + dy * dy;
    dist_sq > 0.0 && dist_sq < safe_distance * safe_distance
}

/// Minimum clearance between oriented vehicle boxes (world units).
pub const PROXIMITY_BOX_CLEARANCE: f32 = 2.0;

/// Scheduler reacts when sprite gap falls below this threshold (world units).
pub const SCHEDULER_SPRITE_GAP_THRESHOLD: f32 = PROXIMITY_BOX_CLEARANCE + 4.0;

/// Hysteresis: yield clears only after gaps widen past this (prevents boundary jitter).
pub const SCHEDULER_CLEAR_GAP: f32 = SCHEDULER_SPRITE_GAP_THRESHOLD + 8.0;

/// Projected half-extent of the vehicle sprite onto a unit axis.
fn projected_half_extent(heading_rad: f32, axis_x: f32, axis_y: f32) -> f32 {
    let half_length = crate::config::VEHICLE_LENGTH * 0.5;
    let half_width = crate::config::VEHICLE_WIDTH * 0.5;
    let cos = heading_rad.cos().abs();
    let sin = heading_rad.sin().abs();
    half_length * (cos * axis_x.abs() + sin * axis_y.abs())
        + half_width * (sin * axis_x.abs() + cos * axis_y.abs())
}

/// Gap between oriented vehicle boxes along the center-to-center axis (negative = overlap).
pub fn sprite_separation_gap(a: &Vehicle, b: &Vehicle) -> f32 {
    let dx = b.position.x - a.position.x;
    let dy = b.position.y - a.position.y;
    let center_dist = (dx * dx + dy * dy).sqrt();
    if center_dist <= f32::EPSILON {
        return -1.0;
    }
    let axis_x = dx / center_dist;
    let axis_y = dy / center_dist;
    center_dist
        - projected_half_extent(a.heading_rad, axis_x, axis_y)
        - projected_half_extent(b.heading_rad, -axis_x, -axis_y)
}

fn project_point_on_segment(point: Vec2, from: Vec2, to: Vec2) -> (Vec2, f32) {
    let seg_dx = to.x - from.x;
    let seg_dy = to.y - from.y;
    let len_sq = seg_dx * seg_dx + seg_dy * seg_dy;
    if len_sq <= f32::EPSILON {
        return (from, 0.0);
    }
    let t = ((point.x - from.x) * seg_dx + (point.y - from.y) * seg_dy) / len_sq;
    let t_clamped = t.clamp(0.0, 1.0);
    (
        Vec2::new(
            from.x + t_clamped * seg_dx,
            from.y + t_clamped * seg_dy,
        ),
        t_clamped,
    )
}

/// Re-project `position` onto the lane polyline and sync `path_index` / `heading_rad`.
///
/// Searches near the current `path_index` first so backward corrections are not
/// snapped forward onto an earlier segment.
pub fn snap_to_lane_path(vehicle: &mut Vehicle, model: &IntersectionModel) {
    let path = match model.lane(vehicle.lane_id) {
        Some(lane) if lane.path.len() >= 2 => &lane.path,
        _ => return,
    };

    let max_seg = path.len() - 2;
    let anchor = vehicle.path_index.min(max_seg);
    let window_start = anchor.saturating_sub(2);
    let window_end = (anchor + 3).min(max_seg);

    let mut best_dist_sq = f32::MAX;
    let mut best_index = anchor;
    let mut best_point = vehicle.position;

    for seg_idx in window_start..=window_end {
        let (proj, _) = project_point_on_segment(vehicle.position, path[seg_idx], path[seg_idx + 1]);
        let dx = vehicle.position.x - proj.x;
        let dy = vehicle.position.y - proj.y;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_index = seg_idx;
            best_point = proj;
        }
    }

    if best_dist_sq > 8.0 * 8.0 {
        for seg_idx in 0..path.len() - 1 {
            if (window_start..=window_end).contains(&seg_idx) {
                continue;
            }
            let (proj, _) =
                project_point_on_segment(vehicle.position, path[seg_idx], path[seg_idx + 1]);
            let dx = vehicle.position.x - proj.x;
            let dy = vehicle.position.y - proj.y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_index = seg_idx;
                best_point = proj;
            }
        }
    }

    vehicle.path_index = best_index;
    vehicle.position = best_point;

    let from = path[vehicle.path_index];
    let to = path[vehicle.path_index + 1];
    let seg_dx = to.x - from.x;
    let seg_dy = to.y - from.y;
    if seg_dx * seg_dx + seg_dy * seg_dy > f32::EPSILON {
        vehicle.heading_rad = seg_dy.atan2(seg_dx);
    }
}

/// Correct lateral drift on the current path segment without jumping to another segment.
pub fn align_to_path_segment(vehicle: &mut Vehicle, model: &IntersectionModel) {
    let path = match model.lane(vehicle.lane_id) {
        Some(lane) if lane.path.len() >= 2 => &lane.path,
        _ => return,
    };

    let idx = vehicle.path_index.min(path.len() - 2);
    vehicle.path_index = idx;
    let from = path[idx];
    let to = path[idx + 1];
    let seg_dx = to.x - from.x;
    let seg_dy = to.y - from.y;
    let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();
    if seg_len <= f32::EPSILON {
        return;
    }

    let ux = seg_dx / seg_len;
    let uy = seg_dy / seg_len;
    let rel_x = vehicle.position.x - from.x;
    let rel_y = vehicle.position.y - from.y;
    let along = (rel_x * ux + rel_y * uy).clamp(0.0, seg_len);
    vehicle.position.x = from.x + ux * along;
    vehicle.position.y = from.y + uy * along;
    vehicle.heading_rad = seg_dy.atan2(seg_dx);

    if along >= seg_len - 0.5 && idx < path.len() - 2 {
        vehicle.path_index = idx + 1;
    }
}

/// Step backward along the lane polyline by `distance` world units.
fn step_backward_on_path(vehicle: &mut Vehicle, model: &IntersectionModel, distance: f32) -> bool {
    let path = match model.lane(vehicle.lane_id) {
        Some(lane) if lane.path.len() >= 2 => &lane.path,
        _ => return false,
    };

    snap_to_lane_path(vehicle, model);
    let mut remaining = distance;

    while remaining > 0.0 && vehicle.path_index < path.len() {
        let seg_idx = vehicle.path_index;
        let from = path[seg_idx];
        let to = path[seg_idx + 1];
        let seg_dx = to.x - from.x;
        let seg_dy = to.y - from.y;
        let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();
        if seg_len <= f32::EPSILON {
            if seg_idx == 0 {
                return false;
            }
            vehicle.path_index -= 1;
            continue;
        }

        let rel_x = vehicle.position.x - from.x;
        let rel_y = vehicle.position.y - from.y;
        let along = rel_x * (seg_dx / seg_len) + rel_y * (seg_dy / seg_len);

        if along >= remaining {
            vehicle.position.x -= (seg_dx / seg_len) * remaining;
            vehicle.position.y -= (seg_dy / seg_len) * remaining;
            vehicle.heading_rad = seg_dy.atan2(seg_dx);
            return true;
        }

        remaining -= along;
        if seg_idx == 0 {
            vehicle.position = from;
            vehicle.heading_rad = seg_dy.atan2(seg_dx);
            return false;
        }

        vehicle.path_index -= 1;
        vehicle.position = path[vehicle.path_index + 1];
    }

    snap_to_lane_path(vehicle, model);
    true
}

/// Move an unreserved vehicle backward along its lane until it is outside the junction zone.
pub fn retract_vehicle_outside_zone(vehicle: &mut Vehicle, model: &IntersectionModel) {
    const STEP: f32 = 6.0;
    snap_to_lane_path(vehicle, model);

    for _ in 0..120 {
        if !model.point_in_zone(vehicle.position) {
            snap_to_lane_path(vehicle, model);
            return;
        }
        if !step_backward_on_path(vehicle, model, STEP) {
            vehicle.position.x -= vehicle.heading_rad.cos() * STEP;
            vehicle.position.y -= vehicle.heading_rad.sin() * STEP;
            snap_to_lane_path(vehicle, model);
        }
    }
}

/// True when `a` is ahead of `b` on the same lane (along `b`'s heading).
fn same_lane_a_is_ahead_of_b(a: &Vehicle, b: &Vehicle) -> Option<bool> {
    if a.lane_id != b.lane_id {
        return None;
    }
    let dx = a.position.x - b.position.x;
    let dy = a.position.y - b.position.y;
    let along = dx * b.heading_rad.cos() + dy * b.heading_rad.sin();
    if along.abs() < 0.5 {
        None
    } else {
        Some(along > 0.0)
    }
}

/// Pick which vehicle should yield in a proximity conflict (`true` => index `a` yields).
fn proximity_yielder_is_a(a: &Vehicle, b: &Vehicle) -> bool {
    if let Some(a_ahead) = same_lane_a_is_ahead_of_b(a, b) {
        return !a_ahead;
    }
    if let Some(b_ahead) = same_lane_a_is_ahead_of_b(b, a) {
        return b_ahead;
    }

    let a_managed = a.state == VehicleState::Managed;
    let b_managed = b.state == VehicleState::Managed;
    if a_managed && b_managed {
        if a.commanded_velocity + 0.01 < a.nominal_velocity
            && b.commanded_velocity + 0.01 >= b.nominal_velocity
        {
            return true;
        }
        if b.commanded_velocity + 0.01 < b.nominal_velocity
            && a.commanded_velocity + 0.01 >= a.nominal_velocity
        {
            return false;
        }
    }

    a.id.0 > b.id.0
}

/// Emergency separation when oriented boxes overlap after movement.
pub fn resolve_proximity_overlaps(vehicles: &mut [Vehicle]) {
    let min_gap = PROXIMITY_BOX_CLEARANCE;
    let len = vehicles.len();

    loop {
        let mut any_pushed = false;

        for i in 0..len {
            if vehicles[i].state == VehicleState::Done {
                continue;
            }
            for j in (i + 1)..len {
                if vehicles[j].state == VehicleState::Done {
                    continue;
                }

                let sep = sprite_separation_gap(&vehicles[i], &vehicles[j]);
                if sep >= 0.0 {
                    continue;
                }

                any_pushed = true;
                let a_yields = proximity_yielder_is_a(&vehicles[i], &vehicles[j]);
                let yielder = if a_yields { i } else { j };
                if vehicles[yielder].scheduler_yield {
                    continue;
                }
                let push = (min_gap - sep) * 0.5 + 0.5;
                let dx = vehicles[i].position.x - vehicles[j].position.x;
                let dy = vehicles[i].position.y - vehicles[j].position.y;
                let center_dist = (dx * dx + dy * dy).sqrt();
                if center_dist > f32::EPSILON {
                    let nx = dx / center_dist;
                    let ny = dy / center_dist;
                    vehicles[i].position.x += nx * push;
                    vehicles[i].position.y += ny * push;
                    vehicles[j].position.x -= nx * push;
                    vehicles[j].position.y -= ny * push;
                } else {
                    let heading = vehicles[yielder].heading_rad;
                    vehicles[j].position.x -=
                        (min_gap + crate::config::VEHICLE_LENGTH) * heading.cos();
                    vehicles[j].position.y -=
                        (min_gap + crate::config::VEHICLE_LENGTH) * heading.sin();
                }
            }
        }

        if !any_pushed {
            break;
        }
    }
}

/// Slow the yielder and separate sprites when oriented boxes overlap.
///
/// Prefer [`apply_pre_move_safety`] before movement and [`resolve_proximity_overlaps`]
/// after; this runs both for integration tests that invoke it directly.
pub fn clamp_velocity_for_proximity(vehicles: &mut [Vehicle], _model: &IntersectionModel) {
    apply_proximity_speed_limits(vehicles);
    resolve_proximity_overlaps(vehicles);
}

/// Move vehicle along its lane path polyline for this frame.
///
/// Path following owns position and heading during route adherence (B02). Crossing metrics
/// accumulate here so callers do not also run `integrate_physics` in the same tick.
pub fn advance_along_path(vehicle: &mut Vehicle, model: &IntersectionModel, dt: f32) {
    if vehicle.state == VehicleState::Done {
        return;
    }

    if is_motion_frozen(vehicle) {
        vehicle.velocity = 0.0;
        vehicle.commanded_velocity = 0.0;
        return;
    }

    step_velocity_toward_command(vehicle, dt);
    let start = vehicle.position;
    let track_crossing =
        vehicle.state == VehicleState::Managed || vehicle.state == VehicleState::Exiting;

    let path = match model.lane(vehicle.lane_id) {
        Some(lane) if !lane.path.is_empty() => &lane.path,
        _ => {
            if track_crossing {
                vehicle.time_in_crossing += dt;
            }
            return;
        }
    };

    if vehicle.path_index >= path.len() - 1 {
        if vehicle.path_index == path.len() - 1 {
            vehicle.state = VehicleState::Done;
        }
        if track_crossing {
            vehicle.time_in_crossing += dt;
        }
        return;
    }

    let mut remaining = vehicle.velocity * dt;

    while remaining > 0.0 && vehicle.path_index < path.len() - 1 {
        let from = path[vehicle.path_index];
        let to = path[vehicle.path_index + 1];

        let seg_dx = to.x - from.x;
        let seg_dy = to.y - from.y;
        let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();

        if seg_len == 0.0 {
            vehicle.path_index += 1;
            continue;
        }

        vehicle.heading_rad = seg_dy.atan2(seg_dx);

        let seg_unit_x = seg_dx / seg_len;
        let seg_unit_y = seg_dy / seg_len;
        let rel_x = vehicle.position.x - from.x;
        let rel_y = vehicle.position.y - from.y;
        let along = rel_x * seg_unit_x + rel_y * seg_unit_y;

        if along < 0.0 {
            vehicle.position = from;
        }

        let along_clamped = along.clamp(0.0, seg_len);
        let dist_to_end = seg_len - along_clamped;

        if remaining <= dist_to_end {
            vehicle.position.x += (seg_dx / seg_len) * remaining;
            vehicle.position.y += (seg_dy / seg_len) * remaining;
            remaining = 0.0;
        } else {
            remaining -= dist_to_end;
            vehicle.position = to;
            vehicle.path_index += 1;
        }
    }

    let moved_dx = vehicle.position.x - start.x;
    let moved_dy = vehicle.position.y - start.y;
    let distance_moved = (moved_dx * moved_dx + moved_dy * moved_dy).sqrt();

    if track_crossing {
        vehicle.time_in_crossing += dt;
        vehicle.distance_in_crossing += distance_moved;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::{attach_paths, lane_id, Cardinal, IntersectionModel, LaneId, Route};
    use crate::spawn::{SpawnRequest, SpawnSystem};
    use std::collections::HashMap;

    fn make_vehicle(id_val: u64, commanded_velocity: f32) -> Vehicle {
        Vehicle {
            id: VehicleId(id_val),
            lane_id: LaneId(0),
            route: Route::Straight,
            approach: crate::intersection::Cardinal::South,
            position: Vec2 { x: 0.0, y: 0.0 },
            heading_rad: 0.0,
            velocity: 0.0,
            commanded_velocity,
            nominal_velocity: commanded_velocity,
            state: VehicleState::Approaching,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
            reservation_granted: false,
            scheduler_yield: false,
            reservation_hold: false,
        }
    }

    #[test]
    fn b03_spawned_vehicles_have_three_distinct_commanded_velocities() {
        // Test (a) — B03: spawn 3 vehicles through the real SpawnSystem path on different
        // approaches (no cooldown conflict), then assert the set of distinct commanded_velocity
        // values has size >= 3.  A test that only checks the constants array is distinct would
        // pass trivially without exercising the spawn path at all.
        let model = IntersectionModel::new();
        let mut spawn = SpawnSystem::new();

        spawn.try_spawn(SpawnRequest::new(Cardinal::South, Route::Straight), &model);
        spawn.try_spawn(SpawnRequest::new(Cardinal::North, Route::Straight), &model);
        spawn.try_spawn(SpawnRequest::new(Cardinal::West, Route::Straight), &model);

        assert_eq!(spawn.vehicles().len(), 3, "all 3 spawns must succeed");

        // Collect distinct commanded_velocity values (f32 bits for HashSet membership).
        let distinct: std::collections::HashSet<u32> = spawn
            .vehicles()
            .iter()
            .map(|v| v.commanded_velocity.to_bits())
            .collect();

        assert!(
            distinct.len() >= 3,
            "expected >= 3 distinct commanded_velocity values but got {}: {:?}",
            distinct.len(),
            spawn
                .vehicles()
                .iter()
                .map(|v| v.commanded_velocity)
                .collect::<Vec<_>>(),
        );

        // Also confirm the values match the published VelocityLevel speeds.
        let expected: std::collections::HashSet<u32> = [
            VelocityLevel::Fast.speed(),
            VelocityLevel::Cruise.speed(),
            VelocityLevel::Yield.speed(),
        ]
        .iter()
        .map(|f| f.to_bits())
        .collect();

        assert_eq!(
            distinct, expected,
            "commanded_velocity values must match VelocityLevel::speed() for all three levels",
        );
    }

    #[test]
    fn faster_commanded_velocity_drives_strictly_greater_distance() {
        // B03: at equal current speed, Fast travels farther than Yield in one tick.
        let mut yield_v = make_vehicle(2, VelocityLevel::Yield.speed());
        let mut fast_v = make_vehicle(0, VelocityLevel::Fast.speed());
        yield_v.velocity = yield_v.commanded_velocity;
        fast_v.velocity = fast_v.commanded_velocity;

        let dt = crate::config::FIXED_TIMESTEP_SECS;
        integrate_physics(&mut yield_v, dt);
        integrate_physics(&mut fast_v, dt);

        // heading_rad = 0.0 → motion is purely along +x
        assert!(
            fast_v.position.x > yield_v.position.x,
            "Fast vehicle (x={}) must travel farther than Yield vehicle (x={}) for equal dt; \
             commanded_velocity is not wired into motion if this fails",
            fast_v.position.x,
            yield_v.position.x,
        );
    }

    #[test]
    fn integrate_physics_does_not_accumulate_crossing_metrics_when_approaching() {
        let mut vehicle = Vehicle {
            id: VehicleId(1),
            lane_id: crate::intersection::LaneId(0),
            route: crate::intersection::Route::Straight,
            approach: crate::intersection::Cardinal::South,
            position: Vec2 { x: 100.0, y: 100.0 },
            heading_rad: 0.0,
            velocity: 50.0,
            commanded_velocity: 50.0,
            nominal_velocity: 50.0,
            state: VehicleState::Approaching,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
            reservation_granted: false,
            scheduler_yield: false,
            reservation_hold: false,
        };

        integrate_physics(&mut vehicle, 0.1);

        assert_eq!(
            vehicle.distance_in_crossing, 0.0,
            "distance should not accumulate when Approaching"
        );
        assert_eq!(
            vehicle.time_in_crossing, 0.0,
            "time should not accumulate when Approaching"
        );
        assert!(
            vehicle.position.x > 100.0,
            "position should change despite no metric accumulation"
        );
    }

    #[test]
    fn integrate_physics_accumulates_crossing_metrics_when_managed() {
        let mut vehicle = Vehicle {
            id: VehicleId(2),
            lane_id: crate::intersection::LaneId(0),
            route: crate::intersection::Route::Straight,
            approach: crate::intersection::Cardinal::South,
            position: Vec2 { x: 200.0, y: 200.0 },
            heading_rad: 0.0,
            velocity: 50.0,
            commanded_velocity: 50.0,
            nominal_velocity: 50.0,
            state: VehicleState::Managed,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
            reservation_granted: false,
            scheduler_yield: false,
            reservation_hold: false,
        };

        integrate_physics(&mut vehicle, 0.1);

        assert!(
            vehicle.distance_in_crossing > 0.0,
            "distance should accumulate when Managed"
        );
        assert_eq!(
            vehicle.time_in_crossing, 0.1,
            "time should accumulate by dt when Managed"
        );
    }

    #[test]
    fn integrate_physics_accumulates_crossing_metrics_when_exiting() {
        let mut vehicle = Vehicle {
            id: VehicleId(3),
            lane_id: crate::intersection::LaneId(0),
            route: crate::intersection::Route::Straight,
            approach: crate::intersection::Cardinal::South,
            position: Vec2 { x: 300.0, y: 300.0 },
            heading_rad: 0.0,
            velocity: 50.0,
            commanded_velocity: 50.0,
            nominal_velocity: 50.0,
            state: VehicleState::Exiting,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
            reservation_granted: false,
            scheduler_yield: false,
            reservation_hold: false,
        };

        integrate_physics(&mut vehicle, 0.1);

        assert!(
            vehicle.distance_in_crossing > 0.0,
            "distance should accumulate when Exiting"
        );
        assert_eq!(
            vehicle.time_in_crossing, 0.1,
            "time should accumulate by dt when Exiting"
        );
    }

    #[test]
    fn advance_along_path_follows_waypoints_and_updates_heading() {
        let mut model = IntersectionModel::new();
        let lane_id = model.lanes[0].id;

        let paths = HashMap::from([(lane_id, vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)])]);
        attach_paths(&mut model, paths);

        let mut vehicle = Vehicle {
            id: VehicleId(1),
            lane_id,
            route: model.lanes[0].route,
            approach: model.lanes[0].approach,
            position: Vec2::new(0.0, 0.0),
            heading_rad: 0.0,
            velocity: 50.0,
            commanded_velocity: 50.0,
            nominal_velocity: 50.0,
            state: VehicleState::Approaching,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
            reservation_granted: false,
            scheduler_yield: false,
            reservation_hold: false,
        };

        advance_along_path(&mut vehicle, &model, 1.0);

        assert!(vehicle.position.x > 0.0);
        assert!(vehicle.position.x <= 100.0);
        assert_eq!(vehicle.position.y, 0.0);
        assert_eq!(vehicle.heading_rad, 0.0);
    }

    fn south_straight_lane_vehicle(id: u64, y: f32, speed: f32) -> Vehicle {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();
        let mut vehicle = spawn_vehicle(VehicleId(id), lane, speed);
        vehicle.nominal_velocity = speed;
        vehicle.commanded_velocity = speed;
        vehicle.velocity = speed;
        vehicle.position.y = y;
        vehicle.position.x = lane.spawn_point.x;
        vehicle
    }

    #[test]
    fn sprite_separation_gap_detects_oriented_overlap() {
        let mut a = make_vehicle(1, 120.0);
        let mut b = make_vehicle(2, 120.0);
        a.heading_rad = 0.0;
        b.heading_rad = 0.0;
        a.position = Vec2::new(0.0, 0.0);
        b.position = Vec2::new(crate::config::VEHICLE_LENGTH * 0.5, 0.0);
        assert!(sprite_separation_gap(&a, &b) < 0.0);

        b.position = Vec2::new(crate::config::VEHICLE_LENGTH + 4.0, 0.0);
        assert!(sprite_separation_gap(&a, &b) >= 0.0);
    }

    #[test]
    fn clamp_yielder_only_stops_follower_not_leader() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();

        let mut leader = spawn_vehicle(VehicleId(1), lane, VelocityLevel::Fast.speed());
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);
        leader.velocity = VelocityLevel::Fast.speed();
        leader.commanded_velocity = leader.velocity;

        let mut follower = spawn_vehicle(VehicleId(2), lane, VelocityLevel::Fast.speed());
        follower.position = Vec2::new(
            lane.spawn_point.x,
            leader.position.y + crate::config::VEHICLE_LENGTH * 0.5,
        );
        follower.velocity = VelocityLevel::Fast.speed();
        follower.commanded_velocity = follower.velocity;

        let mut vehicles = vec![leader, follower];
        clamp_velocity_for_proximity(&mut vehicles, &model);

        assert_eq!(vehicles[0].velocity, VelocityLevel::Fast.speed());
        assert_eq!(vehicles[1].velocity, 0.0);
        assert!(sprite_separation_gap(&vehicles[0], &vehicles[1]) >= 0.0);
    }

    #[test]
    fn detect_close_call_flags_same_lane_violation() {
        let a = south_straight_lane_vehicle(1, 500.0, 120.0);
        let mut b = south_straight_lane_vehicle(2, 520.0, 120.0);
        b.position.y = a.position.y + crate::config::SAFE_DISTANCE * 0.5;

        assert!(detect_close_call(&a, &b, crate::config::SAFE_DISTANCE));
    }

    #[test]
    fn detect_close_call_flags_cross_lane_proximity() {
        let model = IntersectionModel::new();
        let south = model
            .lane(lane_id(Cardinal::South, Route::Straight))
            .unwrap();
        let north = model
            .lane(lane_id(Cardinal::North, Route::Straight))
            .unwrap();
        let mut a = spawn_vehicle(VehicleId(1), south, 120.0);
        let mut b = spawn_vehicle(VehicleId(2), north, 120.0);
        a.position = Vec2::new(100.0, 100.0);
        b.position = Vec2::new(110.0, 105.0);

        assert!(detect_close_call(&a, &b, crate::config::SAFE_DISTANCE));
    }

    #[test]
    fn coast_clear_blocks_departure_behind_stopped_leader_until_gap_opens() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();
        let safe = crate::config::SAFE_DISTANCE;
        let fast = VelocityLevel::Fast.speed();

        let mut leader = spawn_vehicle(VehicleId(1), lane, fast);
        leader.position = Vec2::new(lane.spawn_point.x, 700.0);
        leader.velocity = 0.0;
        leader.commanded_velocity = 0.0;

        let mut follower = spawn_vehicle(VehicleId(2), lane, fast);
        follower.velocity = 0.0;
        follower.commanded_velocity = fast;
        follower.position = Vec2::new(lane.spawn_point.x, leader.position.y + safe + 1.0);

        let vehicles = vec![leader, follower];
        assert!(
            !coast_clear_for_departure(&vehicles[1], &vehicles, &model, safe),
            "must not roll when only barely past safe distance behind a stopped leader"
        );

        let mut follower_clear = spawn_vehicle(VehicleId(3), lane, fast);
        follower_clear.velocity = 0.0;
        follower_clear.commanded_velocity = fast;
        let start_buffer = fast * crate::config::FIXED_TIMESTEP_SECS * 2.0;
        let mut leader2 = spawn_vehicle(VehicleId(4), lane, fast);
        leader2.position = Vec2::new(lane.spawn_point.x, 700.0);
        leader2.velocity = 0.0;
        leader2.commanded_velocity = 0.0;
        follower_clear.position =
            Vec2::new(lane.spawn_point.x, leader2.position.y + safe + start_buffer + 4.0);
        let vehicles_clear = vec![leader2, follower_clear];
        assert!(
            coast_clear_for_departure(&vehicles_clear[1], &vehicles_clear, &model, safe),
            "may roll once a comfortable gap opens"
        );
    }

    #[test]
    fn enforce_follow_distance_slows_follower_behind_stopped_leader() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();

        let mut leader = spawn_vehicle(VehicleId(1), lane, VelocityLevel::Fast.speed());
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);
        leader.nominal_velocity = 0.0;
        leader.commanded_velocity = 0.0;
        leader.velocity = 0.0;

        let fast_speed = VelocityLevel::Fast.speed();
        let mut follower = spawn_vehicle(VehicleId(2), lane, fast_speed);
        follower.nominal_velocity = fast_speed;
        follower.commanded_velocity = fast_speed;
        follower.velocity = fast_speed;
        follower.position = Vec2::new(
            lane.spawn_point.x,
            leader.position.y + crate::config::SAFE_DISTANCE * 2.0,
        );
        let mut saw_slowdown = false;

        let mut vehicles = vec![leader, follower];
        for _ in 0..300 {
            enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);
            for vehicle in &mut vehicles {
                advance_along_path(vehicle, &model, crate::config::FIXED_TIMESTEP_SECS);
            }

            let gap = vehicles[1].position.y - vehicles[0].position.y;
            assert!(
                gap >= crate::config::SAFE_DISTANCE * 0.9,
                "follower must not close inside safe distance (gap={gap})"
            );
            if vehicles[1].velocity < fast_speed {
                saw_slowdown = true;
            }
        }

        assert!(
            saw_slowdown,
            "follower should slow while approaching a stopped leader"
        );
    }

    #[test]
    fn enforce_follow_distance_does_not_adjust_managed_follower() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();

        let mut leader = spawn_vehicle(VehicleId(1), lane, VelocityLevel::Cruise.speed());
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);
        leader.commanded_velocity = 0.0;
        leader.velocity = 0.0;

        let mut managed = spawn_vehicle(VehicleId(2), lane, VelocityLevel::Fast.speed());
        managed.state = VehicleState::Managed;
        managed.position = Vec2::new(
            lane.spawn_point.x,
            leader.position.y + crate::config::SAFE_DISTANCE * 0.5,
        );
        let mut vehicles = vec![leader, managed];
        let initial_speed = vehicles[1].commanded_velocity;

        enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);

        assert_eq!(
            vehicles[1].commanded_velocity, initial_speed,
            "Managed vehicles defer to smart controller, not B04 follow logic"
        );
    }

    #[test]
    fn enforce_follow_distance_steady_follow_behind_moving_leader_no_ratcheting() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();
        let nominal = VelocityLevel::Fast.speed();
        let leader_speed = 60.0_f32;
        let gap = crate::config::SAFE_DISTANCE * 0.5;

        let mut leader = spawn_vehicle(VehicleId(1), lane, leader_speed);
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);
        leader.velocity = leader_speed;
        leader.commanded_velocity = leader_speed;

        let mut follower = spawn_vehicle(VehicleId(2), lane, nominal);
        follower.position = Vec2::new(lane.spawn_point.x, leader.position.y + gap);
        follower.velocity = nominal;
        follower.commanded_velocity = nominal;

        let mut vehicles = vec![leader, follower];
        enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);
        let steady = vehicles[1].commanded_velocity;

        assert!(
            steady > 0.0 && steady < nominal,
            "follower should slow but not stop behind moving leader (steady={steady})"
        );

        for _ in 0..100 {
            enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);
            assert_eq!(
                vehicles[1].commanded_velocity, steady,
                "velocity must not ratchet frame-over-frame (steady={steady}, now={})",
                vehicles[1].commanded_velocity
            );
        }
    }

    #[test]
    fn enforce_follow_distance_restores_nominal_when_gap_safe() {
        let model = IntersectionModel::new();
        let lid = lane_id(Cardinal::South, Route::Straight);
        let lane = model.lane(lid).unwrap();
        let nominal = VelocityLevel::Fast.speed();

        let mut leader = spawn_vehicle(VehicleId(1), lane, nominal);
        leader.position = Vec2::new(lane.spawn_point.x, 500.0);

        let mut follower = spawn_vehicle(VehicleId(2), lane, nominal);
        follower.nominal_velocity = nominal;
        follower.position = Vec2::new(
            lane.spawn_point.x,
            leader.position.y + crate::config::SAFE_DISTANCE * 2.0,
        );
        follower.commanded_velocity = 10.0;
        follower.velocity = 10.0;

        let mut vehicles = vec![leader, follower];
        enforce_follow_distance(&mut vehicles, crate::config::SAFE_DISTANCE);

        assert_eq!(
            vehicles[1].commanded_velocity, nominal,
            "follower should restore nominal speed when gap is safe"
        );
        let before = vehicles[1].velocity;
        for _ in 0..250 {
            step_velocity_toward_command(&mut vehicles[1], crate::config::FIXED_TIMESTEP_SECS);
        }
        assert!(
            (vehicles[1].velocity - nominal).abs() < 1.0,
            "velocity should ramp up to nominal after gap clears (got {})",
            vehicles[1].velocity
        );
        assert!(
            vehicles[1].velocity > before,
            "velocity should increase gradually"
        );
    }

    #[test]
    fn velocity_decelerates_gradually_not_instantly() {
        let mut vehicle = make_vehicle(1, VelocityLevel::Fast.speed());
        vehicle.velocity = vehicle.commanded_velocity;
        vehicle.commanded_velocity = 0.0;

        let before = vehicle.velocity;
        step_velocity_toward_command(&mut vehicle, crate::config::FIXED_TIMESTEP_SECS);

        assert!(vehicle.velocity > 0.0, "one frame should not snap to zero");
        assert!(
            vehicle.velocity < before,
            "velocity should decrease toward command"
        );
    }

    #[test]
    fn different_vehicles_use_different_deceleration_rates() {
        let dt = crate::config::FIXED_TIMESTEP_SECS;
        let start = VelocityLevel::Fast.speed();

        let mut quicker = make_vehicle(1, start);
        quicker.velocity = start;
        quicker.commanded_velocity = 0.0;

        let mut slower = make_vehicle(2, start);
        slower.velocity = start;
        slower.commanded_velocity = 0.0;

        step_velocity_toward_command(&mut quicker, dt);
        step_velocity_toward_command(&mut slower, dt);

        assert!(
            quicker.velocity < slower.velocity,
            "VehicleId(1) should decelerate faster than VehicleId(2) (AUD-B3)"
        );
    }

    #[test]
    fn different_vehicles_use_different_acceleration_rates() {
        let dt = crate::config::FIXED_TIMESTEP_SECS;
        let target = VelocityLevel::Fast.speed();

        let mut quicker = make_vehicle(0, target);
        quicker.velocity = 0.0;
        quicker.commanded_velocity = target;

        let mut slower = make_vehicle(2, target);
        slower.velocity = 0.0;
        slower.commanded_velocity = target;

        step_velocity_toward_command(&mut quicker, dt);
        step_velocity_toward_command(&mut slower, dt);

        assert!(
            quicker.velocity > slower.velocity,
            "VehicleId(0) should accelerate faster than VehicleId(2) (REQ-B3)"
        );
        assert!(quicker.velocity > 0.0 && slower.velocity > 0.0);
        assert!(
            quicker.velocity < target && slower.velocity < target,
            "one frame should not snap to target speed"
        );
    }

    #[test]
    fn motion_profile_exposes_three_distinct_rate_scales() {
        let accel_scales: std::collections::HashSet<u32> = (0..12)
            .map(|id| {
                let (accel, _) = motion_profile(VehicleId(id));
                (accel * 1000.0) as u32
            })
            .collect();
        let decel_scales: std::collections::HashSet<u32> = (0..12)
            .map(|id| {
                let (_, decel) = motion_profile(VehicleId(id));
                (decel * 1000.0) as u32
            })
            .collect();
        assert_eq!(
            accel_scales.len(),
            3,
            "expected three distinct accel profiles"
        );
        assert_eq!(
            decel_scales.len(),
            3,
            "expected three distinct decel profiles"
        );
    }

    #[test]
    fn advance_along_path_accumulates_time_at_path_terminal() {
        let mut model = IntersectionModel::new();
        let lane_id_val = model.lanes[0].id;
        let paths = HashMap::from([(
            lane_id_val,
            vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)],
        )]);
        attach_paths(&mut model, paths);

        let mut vehicle = Vehicle {
            id: VehicleId(1),
            lane_id: lane_id_val,
            route: model.lanes[0].route,
            approach: model.lanes[0].approach,
            position: Vec2::new(100.0, 0.0),
            heading_rad: 0.0,
            velocity: 0.0,
            commanded_velocity: 0.0,
            nominal_velocity: 50.0,
            state: VehicleState::Managed,
            path_index: 1,
            distance_in_crossing: 0.0,
            time_in_crossing: 0.0,
            reservation_granted: false,
            scheduler_yield: false,
            reservation_hold: false,
        };

        advance_along_path(&mut vehicle, &model, 0.25);

        assert_eq!(
            vehicle.time_in_crossing, 0.25,
            "time should accumulate at path terminal for Managed vehicles"
        );
        assert_eq!(
            vehicle.distance_in_crossing, 0.0,
            "no distance moved when already at terminal waypoint"
        );
    }

    #[test]
    fn advance_along_path_turn_exits_perpendicular_arm() {
        let model = IntersectionModel::new();
        let dt = crate::config::FIXED_TIMESTEP_SECS;
        let w = crate::config::WINDOW_WIDTH as f32;
        let h = crate::config::WINDOW_HEIGHT as f32;

        for route in [Route::Right, Route::Left] {
            for approach in Cardinal::ALL {
                let lane = model
                    .lane(crate::intersection::lane_id(approach, route))
                    .expect("turn lane");
                let mut vehicle =
                    spawn_vehicle(VehicleId(1), lane, crate::config::DEFAULT_SPAWN_VELOCITY);

                for _ in 0..1500 {
                    advance_along_path(&mut vehicle, &model, dt);
                }

                let exited = match crate::intersection::exit_cardinal_for_turn(approach, route) {
                    Cardinal::West => vehicle.position.x < 0.0,
                    Cardinal::East => vehicle.position.x > w,
                    Cardinal::North => vehicle.position.y < 0.0,
                    Cardinal::South => vehicle.position.y > h,
                };
                assert!(
                    exited,
                    "{approach:?} {route:?} failed to exit off-screen, got ({}, {})",
                    vehicle.position.x, vehicle.position.y
                );
            }
        }
    }
}
