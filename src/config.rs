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
}
