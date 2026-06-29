//! Post-session statistics window (C06).

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;
use std::time::Duration;

use crate::config::STATS_WINDOW_TITLE;
use crate::stats::Stats;

/// Payload for the statistics UI on `Esc` (C06).
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub stats: Stats,
}

/// Build the session summary from collected stats (SDS §13.4).
pub fn session_summary_from(stats: Stats) -> SessionSummary {
    SessionSummary { stats }
}

/// Human-readable lines for the stats window (unit-testable without SDL).
pub fn format_stats_lines(stats: &Stats) -> Vec<String> {
    vec![
        "Session statistics".to_string(),
        String::new(),
        format!(
            "Max number of vehicles that passed the intersection: {}",
            stats.vehicles_passed
        ),
        format!(
            "Max velocity of all vehicles: {}",
            format_velocity(stats.max_velocity, true)
        ),
        format!(
            "Min velocity of all vehicles: {}",
            format_velocity(stats.min_velocity, false)
        ),
        format!(
            "Max time that took the vehicle to pass the intersection (s): {}",
            format_crossing_time(stats.max_crossing_time, true)
        ),
        format!(
            "Min time that took the vehicle to pass the intersection (s): {}",
            format_crossing_time(stats.min_crossing_time, false)
        ),
        format!("Close calls: {}", stats.close_calls),
        String::new(),
        "--- Additional statistics (bonus) ---".to_string(),
        format!(
            "Session duration (s): {}",
            format_crossing_time(stats.session_duration_secs, true)
        ),
        format!(
            "Avg crossing time (s): {}",
            format_crossing_time(stats.avg_crossing_time_secs, true)
        ),
        format!("Peak concurrent in zone: {}", stats.peak_concurrent_in_zone),
        format!("Vehicles entered zone: {}", stats.vehicles_entered_zone),
    ]
}

fn format_velocity(value: f32, is_max: bool) -> String {
    if is_max && value <= 0.0 {
        return "0".to_string();
    }
    if !is_max && value == f32::MAX {
        return "N/A".to_string();
    }
    format!("{value:.1}")
}

fn format_crossing_time(value: f32, is_max: bool) -> String {
    if is_max && value <= 0.0 {
        return "0.00".to_string();
    }
    if !is_max && value == f32::MAX {
        return "N/A".to_string();
    }
    format!("{value:.2}")
}

const MARGIN_X: i32 = 20;
const MARGIN_Y: i32 = 20;
const BODY_TEXT_SCALE: u32 = 2;
const FOOTER_TEXT_SCALE: u32 = 1;
const LINE_STEP: i32 = 24;
const EMPTY_LINE_STEP: i32 = 8;
const FOOTER_TOP_GAP: i32 = 12;
const BOTTOM_MARGIN: i32 = 20;
const FOOTER_TEXT: &str = "Press Esc or close window to exit";
const GLYPH_W: i32 = 8;
const GLYPH_H: i32 = 8;

/// Pixel width of `text` at the given bitmap scale (matches [`draw_text`]).
pub fn text_width_px(text: &str, scale: u32) -> i32 {
    let scale = scale.max(1) as i32;
    if text.is_empty() {
        return 0;
    }
    let advance = GLYPH_W * scale + scale;
    text.chars().count() as i32 * advance
}

fn text_height_px(scale: u32) -> i32 {
    GLYPH_H * scale.max(1) as i32
}

/// Window size in pixels that fits all stat lines and the footer hint.
pub fn stats_window_dimensions(lines: &[String]) -> (u32, u32) {
    let mut max_width = text_width_px("Session statistics", BODY_TEXT_SCALE);
    for line in lines {
        if !line.is_empty() {
            max_width = max_width.max(text_width_px(line, BODY_TEXT_SCALE));
        }
    }
    max_width = max_width.max(text_width_px(FOOTER_TEXT, FOOTER_TEXT_SCALE));

    let mut y = MARGIN_Y;
    for (index, line) in lines.iter().enumerate() {
        if index == 0 {
            y += LINE_STEP;
            continue;
        }
        if line.is_empty() {
            y += EMPTY_LINE_STEP;
        } else {
            y += LINE_STEP;
        }
    }
    y += FOOTER_TOP_GAP + text_height_px(FOOTER_TEXT_SCALE) + BOTTOM_MARGIN;

    ((max_width + MARGIN_X * 2).max(1) as u32, y.max(1) as u32)
}

/// Show the statistics window until the user closes it or presses `Esc`.
pub fn show_stats_window(sdl: &Sdl, summary: SessionSummary) -> Result<(), String> {
    let video = sdl.video().map_err(|e| format!("SDL video failed: {e}"))?;
    let lines = format_stats_lines(&summary.stats);
    let (width, height) = stats_window_dimensions(&lines);
    let window = video
        .window(STATS_WINDOW_TITLE, width, height)
        .position_centered()
        .build()
        .map_err(|e| format!("stats window failed: {e}"))?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .build()
        .map_err(|e| format!("stats canvas failed: {e}"))?;

    let mut event_pump = sdl
        .event_pump()
        .map_err(|e| format!("SDL event pump failed: {e}"))?;

    let stats_window_id = canvas.window().id();
    let mut running = true;
    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::Window {
                    window_id,
                    win_event: WindowEvent::Close,
                    ..
                } if window_id == stats_window_id => running = false,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => running = false,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(28, 32, 40));
        canvas.clear();

        let mut y = MARGIN_Y;
        for (index, line) in lines.iter().enumerate() {
            let color = if index == 0 {
                Color::RGB(240, 240, 245)
            } else if line.is_empty() {
                y += EMPTY_LINE_STEP;
                continue;
            } else {
                Color::RGB(210, 215, 225)
            };
            draw_text(&mut canvas, line, MARGIN_X, y, BODY_TEXT_SCALE, color)?;
            y += LINE_STEP;
        }

        draw_text(
            &mut canvas,
            FOOTER_TEXT,
            MARGIN_X,
            y + FOOTER_TOP_GAP,
            FOOTER_TEXT_SCALE,
            Color::RGB(130, 135, 150),
        )?;

        canvas.present();
        std::thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}

fn draw_text(
    canvas: &mut Canvas<Window>,
    text: &str,
    x: i32,
    y: i32,
    scale: u32,
    color: Color,
) -> Result<(), String> {
    canvas.set_draw_color(color);
    let mut cursor_x = x;
    let scale = scale.max(1);

    for ch in text.chars() {
        if let Some(glyph) = glyph_rows(ch) {
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8_i32 {
                    // font8x8: LSB is the left-most pixel in each row
                    if bits & (1 << col) != 0 {
                        let rect = Rect::new(
                            cursor_x + col * scale as i32,
                            y + row as i32 * scale as i32,
                            scale,
                            scale,
                        );
                        canvas.fill_rect(rect)?;
                    }
                }
            }
        }
        cursor_x += 8 * scale as i32 + scale as i32;
    }

    Ok(())
}

fn glyph_rows(ch: char) -> Option<&'static [u8; 8]> {
    let index = ch as u32;
    if !(32..=126).contains(&index) {
        return None;
    }
    Some(&FONT8X8_BASIC[(index - 32) as usize])
}

// Public-domain 8x8 bitmap font (font8x8_basic, ASCII 32..126).
const FONT8X8_BASIC: [[u8; 8]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // space
    [0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00], // !
    [0x36, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // "
    [0x36, 0x36, 0x7F, 0x36, 0x7F, 0x36, 0x36, 0x00], // #
    [0x0C, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x0C, 0x00], // $
    [0x00, 0x63, 0x33, 0x18, 0x0C, 0x66, 0x63, 0x00], // %
    [0x1C, 0x36, 0x1C, 0x6E, 0x3B, 0x33, 0x6E, 0x00], // &
    [0x06, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00], // '
    [0x18, 0x0C, 0x06, 0x06, 0x06, 0x0C, 0x18, 0x00], // (
    [0x06, 0x0C, 0x18, 0x18, 0x18, 0x0C, 0x06, 0x00], // )
    [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00], // *
    [0x00, 0x0C, 0x0C, 0x3F, 0x0C, 0x0C, 0x00, 0x00], // +
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x06, 0x00], // ,
    [0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x00], // -
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00], // .
    [0x60, 0x30, 0x18, 0x0C, 0x06, 0x03, 0x01, 0x00], // /
    [0x3E, 0x63, 0x73, 0x7B, 0x6F, 0x67, 0x3E, 0x00], // 0
    [0x0C, 0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x3F, 0x00], // 1
    [0x1E, 0x33, 0x30, 0x1C, 0x06, 0x33, 0x3F, 0x00], // 2
    [0x1E, 0x33, 0x30, 0x1C, 0x30, 0x33, 0x1E, 0x00], // 3
    [0x38, 0x3C, 0x36, 0x33, 0x7F, 0x30, 0x78, 0x00], // 4
    [0x3F, 0x03, 0x1F, 0x30, 0x30, 0x33, 0x1E, 0x00], // 5
    [0x1C, 0x06, 0x03, 0x1F, 0x33, 0x33, 0x1E, 0x00], // 6
    [0x3F, 0x33, 0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x00], // 7
    [0x1E, 0x33, 0x33, 0x1E, 0x33, 0x33, 0x1E, 0x00], // 8
    [0x1E, 0x33, 0x33, 0x3E, 0x30, 0x18, 0x0E, 0x00], // 9
    [0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x00], // :
    [0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x06, 0x00], // ;
    [0x18, 0x0C, 0x06, 0x03, 0x06, 0x0C, 0x18, 0x00], // <
    [0x00, 0x00, 0x3F, 0x00, 0x00, 0x3F, 0x00, 0x00], // =
    [0x06, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x06, 0x00], // >
    [0x1E, 0x33, 0x30, 0x18, 0x0C, 0x00, 0x0C, 0x00], // ?
    [0x3E, 0x63, 0x7B, 0x7B, 0x7B, 0x03, 0x1E, 0x00], // @
    [0x0C, 0x1E, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x00], // A
    [0x3F, 0x66, 0x66, 0x3E, 0x66, 0x66, 0x3F, 0x00], // B
    [0x3C, 0x66, 0x03, 0x03, 0x03, 0x66, 0x3C, 0x00], // C
    [0x1F, 0x36, 0x66, 0x66, 0x66, 0x36, 0x1F, 0x00], // D
    [0x7F, 0x46, 0x16, 0x1E, 0x16, 0x46, 0x7F, 0x00], // E
    [0x7F, 0x46, 0x16, 0x1E, 0x16, 0x06, 0x0F, 0x00], // F
    [0x3C, 0x66, 0x03, 0x03, 0x73, 0x66, 0x7C, 0x00], // G
    [0x33, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x33, 0x00], // H
    [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00], // I
    [0x78, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E, 0x00], // J
    [0x67, 0x66, 0x36, 0x1E, 0x36, 0x66, 0x67, 0x00], // K
    [0x0F, 0x06, 0x06, 0x06, 0x46, 0x66, 0x7F, 0x00], // L
    [0x63, 0x77, 0x7F, 0x7F, 0x6B, 0x63, 0x63, 0x00], // M
    [0x63, 0x67, 0x6F, 0x7B, 0x73, 0x63, 0x63, 0x00], // N
    [0x1C, 0x36, 0x63, 0x63, 0x63, 0x36, 0x1C, 0x00], // O
    [0x3F, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x0F, 0x00], // P
    [0x1E, 0x33, 0x33, 0x33, 0x3B, 0x1E, 0x38, 0x00], // Q
    [0x3F, 0x66, 0x66, 0x3E, 0x36, 0x66, 0x67, 0x00], // R
    [0x1E, 0x33, 0x07, 0x0E, 0x38, 0x33, 0x1E, 0x00], // S
    [0x3F, 0x2D, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00], // T
    [0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x3F, 0x00], // U
    [0x33, 0x33, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00], // V
    [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00], // W
    [0x63, 0x63, 0x36, 0x1C, 0x1C, 0x36, 0x63, 0x00], // X
    [0x33, 0x33, 0x33, 0x1E, 0x0C, 0x0C, 0x1E, 0x00], // Y
    [0x7F, 0x63, 0x31, 0x18, 0x4C, 0x66, 0x7F, 0x00], // Z
    [0x1E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x1E, 0x00], // [
    [0x03, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00], // backslash
    [0x1E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x1E, 0x00], // ]
    [0x08, 0x1C, 0x36, 0x63, 0x00, 0x00, 0x00, 0x00], // ^
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF], // _
    [0x0C, 0x0C, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00], // `
    [0x00, 0x00, 0x1E, 0x30, 0x3E, 0x33, 0x6E, 0x00], // a
    [0x07, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x3B, 0x00], // b
    [0x00, 0x00, 0x1E, 0x33, 0x03, 0x33, 0x1E, 0x00], // c
    [0x38, 0x30, 0x30, 0x3e, 0x33, 0x33, 0x6E, 0x00], // d
    [0x00, 0x00, 0x1E, 0x33, 0x3f, 0x03, 0x1E, 0x00], // e
    [0x1C, 0x36, 0x06, 0x0f, 0x06, 0x06, 0x0F, 0x00], // f
    [0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x1F], // g
    [0x07, 0x06, 0x36, 0x6E, 0x66, 0x66, 0x67, 0x00], // h
    [0x0C, 0x00, 0x0E, 0x0C, 0x0C, 0x0C, 0x1E, 0x00], // i
    [0x30, 0x00, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E], // j
    [0x07, 0x06, 0x66, 0x36, 0x1E, 0x36, 0x67, 0x00], // k
    [0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00], // l
    [0x00, 0x00, 0x33, 0x7F, 0x7F, 0x6B, 0x63, 0x00], // m
    [0x00, 0x00, 0x1F, 0x33, 0x33, 0x33, 0x33, 0x00], // n
    [0x00, 0x00, 0x1E, 0x33, 0x33, 0x33, 0x1E, 0x00], // o
    [0x00, 0x00, 0x3B, 0x66, 0x66, 0x3E, 0x06, 0x0F], // p
    [0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x78], // q
    [0x00, 0x00, 0x3B, 0x6E, 0x66, 0x06, 0x0F, 0x00], // r
    [0x00, 0x00, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x00], // s
    [0x08, 0x0C, 0x3E, 0x0C, 0x0C, 0x2C, 0x18, 0x00], // t
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x33, 0x6E, 0x00], // u
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00], // v
    [0x00, 0x00, 0x63, 0x6B, 0x7F, 0x7F, 0x36, 0x00], // w
    [0x00, 0x00, 0x63, 0x36, 0x1C, 0x36, 0x63, 0x00], // x
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x3E, 0x30, 0x1F], // y
    [0x00, 0x00, 0x3F, 0x19, 0x0C, 0x26, 0x3F, 0x00], // z
    [0x38, 0x0C, 0x0C, 0x07, 0x0C, 0x0C, 0x38, 0x00], // {
    [0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00], // |
    [0x07, 0x0C, 0x0C, 0x38, 0x0C, 0x0C, 0x07, 0x00], // }
    [0x6E, 0x3B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // ~
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{apply_event, StatsEvent};
    use crate::vehicle::VehicleId;

    fn sample_stats() -> Stats {
        let mut stats = Stats::new();
        for (crossing_time, peak_velocity) in [(2.5, 120.0), (1.0, 80.0), (2.0, 100.0), (1.5, 90.0)]
        {
            apply_event(
                &mut stats,
                StatsEvent::VehicleExited {
                    id: VehicleId(1),
                    crossing_time,
                    peak_velocity,
                },
            );
        }
        stats.vehicles_entered_zone = 4;
        stats.peak_concurrent_in_zone = 2;
        stats.finalize_session(12.0);
        stats
    }

    #[test]
    fn stats_window_dimensions_fit_all_content() {
        let lines = format_stats_lines(&sample_stats());
        let (width, height) = stats_window_dimensions(&lines);

        for line in &lines {
            if line.is_empty() {
                continue;
            }
            assert!(
                text_width_px(line, BODY_TEXT_SCALE) + MARGIN_X * 2 <= width as i32,
                "line {:?} wider than window ({width}px)",
                line
            );
        }
        assert!(
            text_width_px(FOOTER_TEXT, FOOTER_TEXT_SCALE) + MARGIN_X * 2 <= width as i32,
            "footer wider than window"
        );

        let mut y = MARGIN_Y;
        for (index, line) in lines.iter().enumerate() {
            if index == 0 {
                y += LINE_STEP;
                continue;
            }
            if line.is_empty() {
                y += EMPTY_LINE_STEP;
            } else {
                y += LINE_STEP;
            }
        }
        y += FOOTER_TOP_GAP + text_height_px(FOOTER_TEXT_SCALE);
        assert!(
            y <= height as i32,
            "content height {y} exceeds window height {height}"
        );
    }

    #[test]
    fn format_includes_bonus_statistics_for_aud_b1() {
        let lines = format_stats_lines(&sample_stats());
        let joined = lines.join("\n");
        assert!(joined.contains("Additional statistics (bonus)"));
        assert!(joined.contains("Session duration (s):"));
        assert!(joined.contains("Avg crossing time (s):"));
        assert!(joined.contains("Peak concurrent in zone:"));
        assert!(joined.contains("Vehicles entered zone:"));
    }

    #[test]
    fn format_includes_all_audit_labels() {
        let lines = format_stats_lines(&sample_stats());
        let joined = lines.join("\n");
        assert!(joined.contains("Max number of vehicles that passed the intersection"));
        assert!(joined.contains("Max velocity of all vehicles"));
        assert!(joined.contains("Min velocity of all vehicles"));
        assert!(joined.contains("Max time that took the vehicle to pass the intersection"));
        assert!(joined.contains("Min time that took the vehicle to pass the intersection"));
        assert!(joined.contains("Close calls"));
    }

    #[test]
    fn audit20_shows_four_vehicles_passed() {
        let lines = format_stats_lines(&sample_stats());
        assert!(lines.iter().any(|line| {
            line.contains("Max number of vehicles that passed the intersection: 4")
        }));
    }

    #[test]
    fn single_vehicle_max_equals_min_crossing_time() {
        let mut stats = Stats::new();
        apply_event(
            &mut stats,
            StatsEvent::VehicleExited {
                id: VehicleId(1),
                crossing_time: 1.75,
                peak_velocity: 100.0,
            },
        );
        let lines = format_stats_lines(&stats);
        let max_line = lines
            .iter()
            .find(|line| line.starts_with("Max time that took"))
            .expect("max crossing line");
        let min_line = lines
            .iter()
            .find(|line| line.starts_with("Min time that took"))
            .expect("min crossing line");
        assert_eq!(
            max_line.split(": ").nth(1),
            min_line.split(": ").nth(1),
            "AUD-25: max and min crossing time should match for one vehicle"
        );
    }

    #[test]
    fn empty_session_uses_na_for_unset_mins() {
        let lines = format_stats_lines(&Stats::new());
        assert!(lines.iter().any(|line| line.contains("Min velocity of all vehicles: N/A")));
        assert!(lines
            .iter()
            .any(|line| line.contains("Min time that took the vehicle to pass the intersection (s): N/A")));
    }
}
