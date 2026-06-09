//! Keyboard-driven vehicle spawning (A04+).

use crate::intersection::Cardinal;

/// Spawn pipeline and cooldown (A04/A05).
#[derive(Debug, Default)]
pub struct SpawnSystem;

impl SpawnSystem {
    pub fn new() -> Self {
        Self
    }
}

/// Request to create a vehicle on an approach (A04 stub per SDS §13.2).
#[derive(Debug, Clone, Copy)]
pub struct SpawnRequest {
    pub approach: Cardinal,
}
