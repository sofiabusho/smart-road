//! Manual AUD-19: requires a display and SDL2.
//!
//! Run: `cargo test manual_audit19_stats_window_opens -- --ignored --nocapture`

use smart_road::stats::Stats;
use smart_road::stats_window::{session_summary_from, show_stats_window};

#[test]
#[ignore = "manual AUD-19: requires display — run with --ignored"]
fn manual_audit19_stats_window_opens() {
    let sdl = sdl2::init().expect("SDL init");
    let mut stats = Stats::new();
    for (crossing_time, peak_velocity) in [(2.5, 120.0), (1.0, 80.0), (2.0, 100.0), (1.5, 90.0)] {
        smart_road::stats::apply_event(
            &mut stats,
            smart_road::stats::StatsEvent::VehicleExited {
                id: smart_road::vehicle::VehicleId(1),
                crossing_time,
                peak_velocity,
            },
        );
    }
    stats.vehicles_entered_zone = 4;
    stats.peak_concurrent_in_zone = 2;
    stats.finalize_session(12.0);
    let summary = session_summary_from(stats);

    show_stats_window(&sdl, summary).expect("stats window should open and close cleanly");
}
