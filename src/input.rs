//! Keyboard input mapping (A04+).

use sdl2::keyboard::Keycode;

use crate::intersection::Cardinal;

/// High-level input events (A04: arrow spawn; A06: R stream; C06: Esc exit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    SpawnCardinal(Cardinal),
    RandomStream(bool),
    Exit,
}

/// Map arrow keys to the approach they spawn on (REQ-12–REQ-15).
pub fn approach_for_arrow(key: Keycode) -> Option<Cardinal> {
    match key {
        Keycode::Up => Some(Cardinal::South),
        Keycode::Down => Some(Cardinal::North),
        Keycode::Right => Some(Cardinal::West),
        Keycode::Left => Some(Cardinal::East),
        _ => None,
    }
}

/// Aggregated keyboard state for the current frame.
#[derive(Debug, Default)]
pub struct InputState {
    events: Vec<InputEvent>,
    random_stream_active: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_key_down(&mut self, keycode: Option<Keycode>) {
        let Some(key) = keycode else {
            return;
        };

        if let Some(approach) = approach_for_arrow(key) {
            self.events.push(InputEvent::SpawnCardinal(approach));
            return;
        }

        match key {
            Keycode::R => {
                self.random_stream_active = true;
                self.events.push(InputEvent::RandomStream(true));
            }
            Keycode::Escape => self.events.push(InputEvent::Exit),
            _ => {}
        }
    }

    pub fn on_key_up(&mut self, keycode: Option<Keycode>) {
        if keycode == Some(Keycode::R) {
            self.random_stream_active = false;
            self.events.push(InputEvent::RandomStream(false));
        }
    }

    /// Whether `R` is currently held (continuous random spawn while true).
    pub fn random_stream_active(&self) -> bool {
        self.random_stream_active
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = InputEvent> + '_ {
        self.events.drain(..)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_up_spawns_south_approach() {
        assert_eq!(approach_for_arrow(Keycode::Up), Some(Cardinal::South));
    }

    #[test]
    fn arrow_down_spawns_north_approach() {
        assert_eq!(approach_for_arrow(Keycode::Down), Some(Cardinal::North));
    }

    #[test]
    fn arrow_right_spawns_west_approach() {
        assert_eq!(approach_for_arrow(Keycode::Right), Some(Cardinal::West));
    }

    #[test]
    fn arrow_left_spawns_east_approach() {
        assert_eq!(approach_for_arrow(Keycode::Left), Some(Cardinal::East));
    }

    #[test]
    fn key_down_emits_random_stream_on_r() {
        let mut input = InputState::new();
        input.on_key_down(Some(Keycode::R));
        let events: Vec<_> = input.drain_events().collect();
        assert_eq!(events, vec![InputEvent::RandomStream(true)]);
        assert!(input.random_stream_active());
    }

    #[test]
    fn key_up_clears_random_stream() {
        let mut input = InputState::new();
        input.on_key_down(Some(Keycode::R));
        let _: Vec<_> = input.drain_events().collect();
        input.on_key_up(Some(Keycode::R));
        let events: Vec<_> = input.drain_events().collect();
        assert_eq!(events, vec![InputEvent::RandomStream(false)]);
        assert!(!input.random_stream_active());
    }

    #[test]
    fn key_down_emits_spawn_event() {
        let mut input = InputState::new();
        input.on_key_down(Some(Keycode::Up));
        let events: Vec<_> = input.drain_events().collect();
        assert_eq!(events, vec![InputEvent::SpawnCardinal(Cardinal::South)]);
    }

    #[test]
    fn key_down_accumulates_spawn_events_in_one_poll() {
        let mut input = InputState::new();
        input.on_key_down(Some(Keycode::Up));
        input.on_key_down(Some(Keycode::Right));
        let events: Vec<_> = input.drain_events().collect();
        assert_eq!(
            events,
            vec![
                InputEvent::SpawnCardinal(Cardinal::South),
                InputEvent::SpawnCardinal(Cardinal::West),
            ]
        );
    }

    #[test]
    fn key_down_ignores_unmapped_keys() {
        let mut input = InputState::new();
        input.on_key_down(Some(Keycode::A));
        assert_eq!(input.drain_events().count(), 0);
    }
}
