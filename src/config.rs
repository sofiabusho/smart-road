//! Window, timing, and tunable simulation constants.

/// Application window title.
pub const WINDOW_TITLE: &str = "smart-road";

/// Default window width in pixels.
pub const WINDOW_WIDTH: u32 = 1024;

/// Default window height in pixels.
pub const WINDOW_HEIGHT: u32 = 768;

/// Target frames per second for the game loop.
pub const TARGET_FPS: u32 = 60;

/// Fixed simulation timestep in seconds (`1 / TARGET_FPS`).
pub const FIXED_TIMESTEP_SECS: f32 = 1.0 / TARGET_FPS as f32;

/// World coordinate system: origin top-left, +x east, +y south (SDL convention).
/// Lane width in world units (pixels at 1:1 scale).
pub const LANE_WIDTH: f32 = 40.0;

/// Number of lanes per approach (right, straight, left).
pub const LANES_PER_APPROACH: u32 = 3;

/// Total road width across all lanes on one approach.
pub const ROAD_WIDTH: f32 = LANE_WIDTH * LANES_PER_APPROACH as f32;

/// Half-width of the junction box (smart-system zone) in world units.
pub const INTERSECTION_HALF_SIZE: f32 = ROAD_WIDTH / 2.0;

/// Distance from window edge to the near end of each approach arm.
pub const APPROACH_MARGIN: f32 = 48.0;

/// Intersection center in world coordinates.
pub const INTERSECTION_CENTER_X: f32 = WINDOW_WIDTH as f32 / 2.0;
pub const INTERSECTION_CENTER_Y: f32 = WINDOW_HEIGHT as f32 / 2.0;

/// Length of each approach arm from junction edge to near window edge.
pub const APPROACH_ARM_LENGTH: f32 =
    INTERSECTION_CENTER_Y - INTERSECTION_HALF_SIZE - APPROACH_MARGIN;

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
        assert!((ROAD_WIDTH - LANE_WIDTH * LANES_PER_APPROACH as f32).abs() < f32::EPSILON);
    }
}
