//! SDL2 application shell and empty game loop.

use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;

use crate::config::{FIXED_TIMESTEP_SECS, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH};
use crate::input::{InputEvent, InputState};
use crate::intersection::IntersectionModel;
use crate::render::{self, RoadAssets};
use crate::smart::SmartController;
use crate::spawn::SpawnSystem;
use crate::stats::StatsSession;
use crate::vehicle::snapshot_for_render;

type WindowCanvas = Canvas<Window>;

/// Top-level application state (expanded in later tickets).
pub struct App {
    _sdl: Sdl,
    running: bool,
    intersection: IntersectionModel,
    spawn: SpawnSystem,
    smart: SmartController,
    stats: StatsSession,
    input: InputState,
    session_time: f32,
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

        let mut canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| format!("SDL canvas failed: {e}"))?;

        // Textures borrow the canvas `TextureCreator`; keep both in this scope for
        // the whole loop so lifetimes stay sound without unsafe erasure.
        let texture_creator = canvas.texture_creator();
        let road_assets = RoadAssets::load(&texture_creator)?;

        let mut app = App {
            _sdl: sdl,
            running: true,
            intersection: IntersectionModel::new(),
            spawn: SpawnSystem::new(),
            smart: SmartController::new(),
            stats: StatsSession::new(),
            input: InputState::new(),
            session_time: 0.0,
        };

        while app.running {
            app.poll_events()?;
            app.update();
            app.draw(&mut canvas, &road_assets)?;
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
        for event in self.input.drain_events().collect::<Vec<_>>() {
            match event {
                InputEvent::SpawnCardinal(approach) => {
                    self.spawn.spawn_on_approach(approach, &self.intersection);
                }
                InputEvent::RandomStream(_) => {}
                InputEvent::Exit => {
                    // C06 replaces this with end_session + stats window.
                    self.running = false;
                }
            }
        }

        if self.input.random_stream_active() {
            self.spawn.spawn_random(&self.intersection);
        }

        self.session_time += FIXED_TIMESTEP_SECS;

        let exited = self.spawn.update(&self.intersection, FIXED_TIMESTEP_SECS);
        self.smart.update(
            self.spawn.vehicles_mut(),
            &self.intersection,
            FIXED_TIMESTEP_SECS,
        );
        self.stats
            .observe_vehicles(self.spawn.vehicles(), self.session_time);
        for exit in exited {
            self.stats.record_exit(exit.id, exit.time_in_crossing);
        }
    }

    fn draw(&self, canvas: &mut WindowCanvas, road_assets: &RoadAssets<'_>) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(42, 90, 42));
        canvas.clear();

        let snapshots: Vec<_> = self
            .spawn
            .vehicles()
            .iter()
            .map(snapshot_for_render)
            .collect();

        render::draw_frame(canvas, &self.intersection, road_assets, &snapshots)?;

        canvas.present();
        Ok(())
    }
}
