//! Keyboard input mapping (A04+).

use sdl2::keyboard::Keycode;

/// High-level input events (expanded in A04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    SpawnCardinal(crate::intersection::Cardinal),
    RandomStream(bool),
    Exit,
}

/// Aggregated keyboard state for the current frame.
#[derive(Debug, Default)]
pub struct InputState {
    events: Vec<InputEvent>,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_key_down(&mut self, _keycode: Option<Keycode>) {
        self.events.clear();
        // A04: map arrow keys, R, Esc to InputEvent.
    }

    pub fn on_key_up(&mut self, _keycode: Option<Keycode>) {
        // A04: R key release handling.
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = InputEvent> + '_ {
        self.events.drain(..)
    }
}
