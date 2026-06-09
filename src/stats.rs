//! Simulation statistics collection (C05+).

/// Session metrics displayed on exit (C05/C06).
#[derive(Debug, Default)]
pub struct Stats {
    pub vehicles_passed: u32,
    pub close_calls: u32,
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }
}
