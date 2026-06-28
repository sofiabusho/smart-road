//! SDL2 application shell and empty game loop.

use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;
use std::time::Instant;

use crate::config::{
    FIXED_TIMESTEP_SECS, SAFE_DISTANCE, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH,
};
use crate::input::{InputEvent, InputState};
use crate::intersection::IntersectionModel;
use crate::render::{self, RoadAssets};
use crate::smart::SmartController;
use crate::spawn::SpawnSystem;
use crate::stats::StatsSession;
use crate::stats_window::{session_summary_from, show_stats_window, SessionSummary};
use crate::vehicle::snapshot_for_render;
use crate::vehicle::{detect_close_call, clamp_velocity_for_proximity, VehicleState};

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
    session_started: Instant,
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
            session_started: Instant::now(),
        };

        let mut show_stats_on_exit = false;
        while app.running {
            let frame_start = Instant::now();
            app.poll_events()?;
            if app.update() {
                show_stats_on_exit = true;
            }
            app.draw(&mut canvas, &road_assets)?;

            // Cap the loop to TARGET_FPS so each update() tick matches one real frame.
            // Without this, fast machines run many sim ticks per wall-clock second and
            // `time_in_crossing` reads ~2× (or more) what a stopwatch measures (AUD-26).
            let frame_budget =
                std::time::Duration::from_secs_f32(FIXED_TIMESTEP_SECS);
            let elapsed = frame_start.elapsed();
            if elapsed < frame_budget {
                std::thread::sleep(frame_budget - elapsed);
            }
        }

        if show_stats_on_exit {
            show_stats_window(&app._sdl, end_session(&app))?;
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

    /// Returns `true` when the user pressed `Esc` to end the session (C06).
    fn update(&mut self) -> bool {
        let mut exit_requested = false;
        for event in self.input.drain_events().collect::<Vec<_>>() {
            match event {
                InputEvent::SpawnCardinal(approach) => {
                    self.spawn.spawn_on_approach(approach, &self.intersection);
                }
                InputEvent::RandomStream(_) => {}
                InputEvent::Exit => {
                    exit_requested = true;
                    self.running = false;
                }
            }
        }

        if self.input.random_stream_active() {
            self.spawn.spawn_random(&self.intersection);
        }

        self.session_time += FIXED_TIMESTEP_SECS;

        self.smart.update(
            self.spawn.vehicles_mut(),
            &self.intersection,
            FIXED_TIMESTEP_SECS,
        );
        let exited = self.spawn.update(&self.intersection, FIXED_TIMESTEP_SECS);
        SmartController::enforce_zone_gate(self.spawn.vehicles_mut(), &self.intersection);
        clamp_velocity_for_proximity(self.spawn.vehicles_mut(), &self.intersection);
        self.stats
            .observe_vehicles(self.spawn.vehicles(), self.session_time);
        self.record_close_calls();
        for exit in exited {
            self.stats.record_exit(exit.id, exit.time_in_crossing);
        }

        exit_requested
    }

    fn record_close_calls(&mut self) {
        let vehicles = self.spawn.vehicles();
        for i in 0..vehicles.len() {
            if vehicles[i].state == VehicleState::Done {
                continue;
            }
            for j in (i + 1)..vehicles.len() {
                if vehicles[j].state == VehicleState::Done {
                    continue;
                }
                if detect_close_call(&vehicles[i], &vehicles[j], SAFE_DISTANCE) {
                    self.stats.record_close_call(vehicles[i].id, vehicles[j].id);
                }
            }
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

/// Capture session metrics for the post-`Esc` statistics window (SDS §13.4).
pub fn end_session(app: &App) -> SessionSummary {
    let mut stats = app.stats.stats.clone();
    stats.finalize_session(app.session_started.elapsed().as_secs_f32());
    session_summary_from(stats)
}
