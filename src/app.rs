//! SDL2 application shell and empty game loop.

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;

use crate::config::{WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH};
use crate::input::InputState;
use crate::intersection::IntersectionModel;
use crate::render;
use crate::smart::SmartController;
use crate::spawn::SpawnSystem;
use crate::stats::Stats;

type WindowCanvas = Canvas<Window>;

/// Top-level application state (expanded in later tickets).
pub struct App {
    _sdl: Sdl,
    canvas: WindowCanvas,
    running: bool,
    intersection: IntersectionModel,
    #[allow(dead_code)]
    spawn: SpawnSystem,
    #[allow(dead_code)]
    smart: SmartController,
    #[allow(dead_code)]
    stats: Stats,
    input: InputState,
}

impl App {
    /// Initialize SDL2, open the window, and run the empty loop until quit.
    pub fn run() -> Result<(), String> {
        let sdl = sdl2::init().map_err(|e| format!("SDL init failed: {e}"))?;
        let video = sdl.video().map_err(|e| format!("SDL video failed: {e}"))?;

        let window = video
            .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
            .build()
            .map_err(|e| format!("SDL window failed: {e}"))?;

        let canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| format!("SDL canvas failed: {e}"))?;

        let mut app = App {
            _sdl: sdl,
            canvas,
            running: true,
            intersection: IntersectionModel::new(),
            spawn: SpawnSystem::new(),
            smart: SmartController::new(),
            stats: Stats::new(),
            input: InputState::new(),
        };

        app.game_loop()
    }

    fn game_loop(&mut self) -> Result<(), String> {
        while self.running {
            self.poll_events()?;
            self.update();
            self.draw()?;
        }
        Ok(())
    }

    fn poll_events(&mut self) -> Result<(), String> {
        let mut pump = self
            ._sdl
            .event_pump()
            .map_err(|e| format!("SDL event pump failed: {e}"))?;

        for event in pump.poll_iter() {
            match event {
                Event::Quit { .. } => self.running = false,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => self.running = false,
                Event::KeyDown { keycode, .. } => {
                    self.input.on_key_down(keycode);
                }
                Event::KeyUp { keycode, .. } => {
                    self.input.on_key_up(keycode);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn update(&mut self) {
        // Simulation update hooks land in B/C tracks (A01: no-op).
        let _ = (
            &mut self.intersection,
            &mut self.spawn,
            &mut self.smart,
            &mut self.stats,
            &self.input,
        );
    }

    fn draw(&mut self) -> Result<(), String> {
        self.canvas.set_draw_color(Color::RGB(32, 48, 32));
        self.canvas.clear();
        render::draw_frame(&mut self.canvas, &self.intersection)?;
        self.canvas.present();
        Ok(())
    }
}
