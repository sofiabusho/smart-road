//! Window, timing, and tunable simulation constants.

/// Application window title.
pub const WINDOW_TITLE: &str = "smart-road";

/// Default window width in pixels.
pub const WINDOW_WIDTH: u32 = 1024;

/// Default window height in pixels.
pub const WINDOW_HEIGHT: u32 = 768;

/// Post-session statistics window title (C06 / REQ-19).
pub const STATS_WINDOW_TITLE: &str = "smart-road — session statistics";

/// Target frames per second for the game loop.
pub const TARGET_FPS: u32 = 60;

/// Fixed simulation timestep in seconds (`1 / TARGET_FPS`).
pub const FIXED_TIMESTEP_SECS: f32 = 1.0 / TARGET_FPS as f32;

// World coordinate system: origin top-left, +x east, +y south (SDL convention).
// Layout constants below are mirrored in scripts/generate_road_assets.py — regenerate
// assets/roads/*.bmp after changing window size, margins, or lane dimensions.

/// Lane width in world units (pixels at 1:1 scale).
pub const LANE_WIDTH: f32 = 40.0;

/// Number of inbound route lanes per approach (right, straight, left).
pub const LANES_PER_APPROACH: u32 = 3;

/// Lanes per road arm: three inbound + three outbound (one direction each).
pub const LANES_PER_ARM: u32 = LANES_PER_APPROACH * 2;

/// Width of the three inbound (or outbound) lanes on one side of the arm.
pub const INBOUND_ROAD_WIDTH: f32 = LANE_WIDTH * LANES_PER_APPROACH as f32;

/// Full road-arm width — six one-direction lanes (3 per side of the center divider).
pub const ROAD_ARM_WIDTH: f32 = LANE_WIDTH * LANES_PER_ARM as f32;

/// Half-width of the junction box (smart-system zone) in world units.
pub const INTERSECTION_HALF_SIZE: f32 = ROAD_ARM_WIDTH / 2.0;

/// Distance from window edge to the near end of each approach arm (0 = asphalt flush to edge).
pub const APPROACH_MARGIN: f32 = 0.0;

/// Intersection center in world coordinates.
pub const INTERSECTION_CENTER_X: f32 = WINDOW_WIDTH as f32 / 2.0;
pub const INTERSECTION_CENTER_Y: f32 = WINDOW_HEIGHT as f32 / 2.0;

/// North/south arm length from junction edge to top/bottom approach margin.
pub const APPROACH_ARM_LENGTH: f32 =
    INTERSECTION_CENTER_Y - INTERSECTION_HALF_SIZE - APPROACH_MARGIN;

/// East/west arm length from junction edge to left/right approach margin.
pub const EW_ARM_LENGTH: f32 = INTERSECTION_CENTER_X - INTERSECTION_HALF_SIZE - APPROACH_MARGIN;

/// Default cruise speed for newly spawned vehicles (world units per second).
/// Three spawn levels (B03): Fast ×1.4, Cruise ×1.0, Yield ×0.5 → 224 / 160 / 80 px/s at this base.
pub const DEFAULT_SPAWN_VELOCITY: f32 = 160.0;

/// Minimum milliseconds between spawns on the same approach (REQ-18).
pub const SPAWN_COOLDOWN_MS: u64 = 400;

/// Vehicle sprite width in world units (pixels at 1:1 scale).
pub const VEHICLE_WIDTH: f32 = 22.0;

/// Vehicle sprite length in world units (pixels at 1:1 scale).
pub const VEHICLE_LENGTH: f32 = 36.0;

/// Visible buffer beyond sprite length for same-lane following (audit LIM-2).
pub const FOLLOW_MARGIN: f32 = 16.0;

/// Minimum center-to-center gap between vehicles on the same lane (REQ-8 / AUD-29).
/// PRD OQ-2: strictly positive and at least one vehicle length.
pub const SAFE_DISTANCE: f32 = VEHICLE_LENGTH + FOLLOW_MARGIN;

/// Maximum acceleration toward commanded speed (world units/s²). B05 scales per vehicle.
pub const BASE_ACCELERATION: f32 = 90.0;

/// Maximum deceleration toward commanded speed (world units/s²). B05 scales per vehicle.
pub const BASE_DECELERATION: f32 = 120.0;

/// Distance before the junction stop line where reservation gating begins (world units).
pub const RESERVATION_TRIGGER_DISTANCE: f32 = 200.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_dimensions_are_positive() {
        assert!(WINDOW_WIDTH > 0);
        assert!(WINDOW_HEIGHT > 0);
    }

    #[test]
    fn fixed_timestep_matches_target_fps() {
        let expected = 1.0 / TARGET_FPS as f32;
        assert!((FIXED_TIMESTEP_SECS - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn road_width_matches_lane_count() {
        assert!((INBOUND_ROAD_WIDTH - LANE_WIDTH * LANES_PER_APPROACH as f32).abs() < f32::EPSILON);
        assert!((ROAD_ARM_WIDTH - INBOUND_ROAD_WIDTH * 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn arm_lengths_reach_window_margins() {
        let half = INTERSECTION_HALF_SIZE;
        assert!(
            (APPROACH_ARM_LENGTH - (INTERSECTION_CENTER_Y - half - APPROACH_MARGIN)).abs()
                < f32::EPSILON
        );
        assert!(
            (EW_ARM_LENGTH - (INTERSECTION_CENTER_X - half - APPROACH_MARGIN)).abs() < f32::EPSILON
        );
    }

    #[test]
    fn safe_distance_is_positive_and_vehicle_scaled() {
        assert!(SAFE_DISTANCE > 0.0);
        assert!(
            SAFE_DISTANCE >= VEHICLE_LENGTH,
            "safe distance should be at least one vehicle length"
        );
    }

    #[test]
    fn motion_limits_are_positive() {
        assert!(BASE_ACCELERATION > 0.0);
        assert!(BASE_DECELERATION > 0.0);
    }
}
