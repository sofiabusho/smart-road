//! Cross intersection topology (lane registry — A03; path polylines — B02).

/// Cardinal approach direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cardinal {
    North,
    South,
    East,
    West,
}

/// Fixed lane route through the intersection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Right,
    Straight,
    Left,
}

/// Stable lane identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LaneId(pub u32);

/// Lane metadata (path geometry added in B02).
#[derive(Debug, Clone)]
pub struct LaneInfo {
    pub id: LaneId,
    pub approach: Cardinal,
    pub route: Route,
}

/// Intersection layout and detection zone (render + paths in later tickets).
#[derive(Debug, Default)]
pub struct IntersectionModel {
    pub lanes: Vec<LaneInfo>,
}

impl IntersectionModel {
    pub fn new() -> Self {
        Self::default()
    }
}
