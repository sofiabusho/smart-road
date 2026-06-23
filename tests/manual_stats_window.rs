//! Manual AUD-19: requires a display and SDL2.
//!
//! Run: `cargo test manual_audit19_stats_window_opens -- --ignored --nocapture`

use smart_road::stats::Stats;
use smart_road::stats_window::{session_summary_from, show_stats_window};

#[test]
#[ignore = "manual AUD-19: requires display — run with --ignored"]
fn manual_audit19_stats_window_opens() {
    let sdl = sdl2::init().expect("SDL init");
    let summary = session_summary_from(Stats {
        vehicles_passed: 4,
        max_vehicles_passed: 4,
        max_velocity: 120.0,
        min_velocity: 80.0,
        max_crossing_time: 2.5,
        min_crossing_time: 1.0,
        close_calls: 0,
    });

    show_stats_window(&sdl, summary).expect("stats window should open and close cleanly");
}
