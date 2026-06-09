//! Post-session statistics window (C06).

use crate::stats::Stats;

/// Payload for the statistics UI on `Esc` (C06).
#[derive(Debug)]
pub struct SessionSummary {
    pub stats: Stats,
}

/// Show the statistics window (C06 — stub).
pub fn show_stats_window(_summary: SessionSummary) -> Result<(), String> {
    Ok(())
}
