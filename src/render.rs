//! SDL2 drawing for intersection and vehicles.

use std::path::Path;

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::config::{
    APPROACH_MARGIN, INTERSECTION_CENTER_X, INTERSECTION_CENTER_Y, LANE_WIDTH, ROAD_ARM_WIDTH,
    VEHICLE_LENGTH, VEHICLE_WIDTH,
};
use crate::intersection::{Cardinal, IntersectionModel, Route, VehicleRenderSnapshot};

const ROAD_ASSET_DIR: &str = "assets/roads";
const VEHICLE_ASSET_DIR: &str = "assets/vehicles";
const LANE_LABEL_INSET: f32 = 80.0;
const GLYPH_W: u32 = 5;
const GLYPH_H: u32 = 7;
const GLYPH_SCALE: u32 = 2;
const GLYPH_GAP: u32 = 2;
const LABEL_PAD: u32 = 3;
const LANE_MARKING: Color = Color::RGB(235, 210, 70);
const LANE_DASH_LEN: i32 = 12;
const LANE_DASH_GAP: i32 = 10;

/// Loaded road tile textures (lives for the `TextureCreator` lifetime).
pub struct RoadAssets<'tex> {
    approach_ns: Texture<'tex>,
    approach_ew: Texture<'tex>,
    intersection_core: Texture<'tex>,
    vehicle_south: Texture<'tex>,
    vehicle_north: Texture<'tex>,
    vehicle_west: Texture<'tex>,
    vehicle_east: Texture<'tex>,
    lane_labels: Vec<(Texture<'tex>, u32, u32)>,
    road_width: u32,
    ns_arm_length: u32,
    ew_arm_length: u32,
}

impl<'tex> RoadAssets<'tex> {
    pub fn load(creator: &'tex TextureCreator<WindowContext>) -> Result<Self, String> {
        let approach_ns = load_texture(creator, ROAD_ASSET_DIR, "approach_ns.bmp")?;
        let approach_ew = load_texture(creator, ROAD_ASSET_DIR, "approach_ew.bmp")?;
        let intersection_core = load_texture(creator, ROAD_ASSET_DIR, "intersection_core.bmp")?;

        // Per-approach vehicle sprites (Time Fantasy style, eastbound authorship).
        let vehicle_south = load_vehicle_texture(creator, Cardinal::South)?;
        let vehicle_north = load_vehicle_texture(creator, Cardinal::North)?;
        let vehicle_west = load_vehicle_texture(creator, Cardinal::West)?;
        let vehicle_east = load_vehicle_texture(creator, Cardinal::East)?;

        let lane_labels = build_lane_label_textures(creator)?;

        let road_width = approach_ns.query().width;
        let ns_arm_length = approach_ns.query().height;
        let ew_arm_length = approach_ew.query().width;

        Ok(Self {
            approach_ns,
            approach_ew,
            intersection_core,
            vehicle_south,
            vehicle_north,
            vehicle_west,
            vehicle_east,
            lane_labels,
            road_width,
            ns_arm_length,
            ew_arm_length,
        })
    }
}

fn load_texture<'tex>(
    creator: &'tex TextureCreator<WindowContext>,
    dir: &str,
    file_name: &str,
) -> Result<Texture<'tex>, String> {
    let path = Path::new(dir).join(file_name);
    let surface = Surface::load_bmp(&path)
        .map_err(|e| format!("failed to load asset {}: {e}", path.display()))?;
    let texture = creator
        .create_texture_from_surface(&surface)
        .map_err(|e| format!("failed to create texture from {}: {e}", path.display()))?;
    Ok(texture)
}

fn vehicle_asset_name(approach: Cardinal) -> &'static str {
    match approach {
        Cardinal::South => "vehicle_south.bmp",
        Cardinal::North => "vehicle_north.bmp",
        Cardinal::West => "vehicle_west.bmp",
        Cardinal::East => "vehicle_east.bmp",
    }
}

fn load_vehicle_texture<'tex>(
    creator: &'tex TextureCreator<WindowContext>,
    approach: Cardinal,
) -> Result<Texture<'tex>, String> {
    let file_name = vehicle_asset_name(approach);
    let path = Path::new(VEHICLE_ASSET_DIR).join(file_name);
    let surface = Surface::load_bmp(&path).map_err(|e| {
        format!(
            "failed to load vehicle asset {}: {e} (run python3 scripts/generate_vehicle_sprites.py)",
            path.display()
        )
    })?;
    let mut texture = creator.create_texture_from_surface(&surface).map_err(|e| {
        format!(
            "failed to create vehicle texture from {}: {e}",
            path.display()
        )
    })?;
    texture.set_blend_mode(sdl2::render::BlendMode::Blend);
    Ok(texture)
}

/// Draw the cross intersection using road tile assets.
pub fn draw_intersection(
    canvas: &mut Canvas<Window>,
    _model: &IntersectionModel,
    assets: &RoadAssets<'_>,
) -> Result<(), String> {
    let rw = assets.road_width;
    let ns_al = assets.ns_arm_length as i32;
    let ew_al = assets.ew_arm_length as i32;
    let cx = INTERSECTION_CENTER_X as i32;
    let cy = INTERSECTION_CENTER_Y as i32;
    let half = (ROAD_ARM_WIDTH / 2.0) as i32;

    let core_dst = Rect::new(cx - half, cy - half, rw, rw);
    canvas
        .copy(&assets.intersection_core, None, core_dst)
        .map_err(|e| format!("draw intersection core: {e}"))?;

    let north_dst = Rect::new(cx - half, cy - half - ns_al, rw, ns_al as u32);
    canvas
        .copy(&assets.approach_ns, None, north_dst)
        .map_err(|e| format!("draw north approach: {e}"))?;

    let south_dst = Rect::new(cx - half, cy + half, rw, ns_al as u32);
    canvas
        .copy(&assets.approach_ns, None, south_dst)
        .map_err(|e| format!("draw south approach: {e}"))?;

    let west_dst = Rect::new(cx - half - ew_al, cy - half, ew_al as u32, rw);
    canvas
        .copy(&assets.approach_ew, None, west_dst)
        .map_err(|e| format!("draw west approach: {e}"))?;

    let east_dst = Rect::new(cx + half, cy - half, ew_al as u32, rw);
    canvas
        .copy(&assets.approach_ew, None, east_dst)
        .map_err(|e| format!("draw east approach: {e}"))?;

    draw_approach_lane_markings(canvas, cx, cy, half, ns_al, ew_al)?;

    Ok(())
}

fn lane_width_i() -> i32 {
    LANE_WIDTH as i32
}

fn draw_approach_lane_markings(
    canvas: &mut Canvas<Window>,
    cx: i32,
    cy: i32,
    half: i32,
    ns_al: i32,
    ew_al: i32,
) -> Result<(), String> {
    let lw = lane_width_i();
    let road_left = cx - half;
    let road_top = cy - half;

    let dash_x = [lw, lw * 2, half + lw, half + lw * 2];
    let center_x = [half - 1, half];

    for &dx in &dash_x {
        draw_dashed_vertical(canvas, road_left + dx, cy - half - ns_al, cy - half)?;
        draw_dashed_vertical(canvas, road_left + dx, cy + half, cy + half + ns_al)?;
    }
    for &dx in &center_x {
        draw_solid_vertical(canvas, road_left + dx, cy - half - ns_al, cy - half)?;
        draw_solid_vertical(canvas, road_left + dx, cy + half, cy + half + ns_al)?;
    }

    let dash_y = [lw, lw * 2, half + lw, half + lw * 2];
    let center_y = [half - 1, half];

    for &dy in &dash_y {
        draw_dashed_horizontal(canvas, road_top + dy, cx + half, cx + half + ew_al)?;
        draw_dashed_horizontal(canvas, road_top + dy, cx - half - ew_al, cx - half)?;
    }
    for &dy in &center_y {
        draw_solid_horizontal(canvas, road_top + dy, cx + half, cx + half + ew_al)?;
        draw_solid_horizontal(canvas, road_top + dy, cx - half - ew_al, cx - half)?;
    }

    Ok(())
}

fn draw_dashed_vertical(
    canvas: &mut Canvas<Window>,
    x: i32,
    y0: i32,
    y1: i32,
) -> Result<(), String> {
    canvas.set_draw_color(LANE_MARKING);
    let (start, end) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    let mut y = start;
    let mut on = true;
    while y < end {
        if on {
            let dash_end = (y + LANE_DASH_LEN).min(end);
            canvas
                .fill_rect(Rect::new(x, y, 1, (dash_end - y) as u32))
                .map_err(|e| format!("draw dashed vertical: {e}"))?;
            y = dash_end;
        } else {
            y = (y + LANE_DASH_GAP).min(end);
        }
        on = !on;
    }
    Ok(())
}

fn draw_dashed_horizontal(
    canvas: &mut Canvas<Window>,
    y: i32,
    x0: i32,
    x1: i32,
) -> Result<(), String> {
    canvas.set_draw_color(LANE_MARKING);
    let (start, end) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    let mut x = start;
    let mut on = true;
    while x < end {
        if on {
            let dash_end = (x + LANE_DASH_LEN).min(end);
            canvas
                .fill_rect(Rect::new(x, y, (dash_end - x) as u32, 1))
                .map_err(|e| format!("draw dashed horizontal: {e}"))?;
            x = dash_end;
        } else {
            x = (x + LANE_DASH_GAP).min(end);
        }
        on = !on;
    }
    Ok(())
}

fn draw_solid_vertical(
    canvas: &mut Canvas<Window>,
    x: i32,
    y0: i32,
    y1: i32,
) -> Result<(), String> {
    canvas.set_draw_color(LANE_MARKING);
    let (start, end) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    canvas
        .fill_rect(Rect::new(x, start, 1, (end - start) as u32))
        .map_err(|e| format!("draw solid vertical: {e}"))?;
    Ok(())
}

fn draw_solid_horizontal(
    canvas: &mut Canvas<Window>,
    y: i32,
    x0: i32,
    x1: i32,
) -> Result<(), String> {
    canvas.set_draw_color(LANE_MARKING);
    let (start, end) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    canvas
        .fill_rect(Rect::new(start, y, (end - start) as u32, 1))
        .map_err(|e| format!("draw solid horizontal: {e}"))?;
    Ok(())
}

/// Draw one frame: intersection tiles plus rotated vehicle sprites.
pub fn draw_frame(
    canvas: &mut Canvas<Window>,
    intersection: &IntersectionModel,
    assets: &RoadAssets<'_>,
    vehicles: &[VehicleRenderSnapshot],
) -> Result<(), String> {
    draw_intersection(canvas, intersection, assets)?;
    draw_lane_labels(canvas, intersection, assets)?;
    for snapshot in vehicles {
        draw_vehicle_sprite(canvas, assets, snapshot)?;
    }
    Ok(())
}

fn build_lane_label_textures<'tex>(
    creator: &'tex TextureCreator<WindowContext>,
) -> Result<Vec<(Texture<'tex>, u32, u32)>, String> {
    let mut labels = Vec::with_capacity(12);
    for approach in Cardinal::ALL {
        for route in Route::ALL {
            let text = crate::intersection::lane_display_label(approach, route);
            let (texture, w, h) = create_label_texture(creator, text)?;
            labels.push((texture, w, h));
        }
    }
    Ok(labels)
}

fn create_label_texture<'tex>(
    creator: &'tex TextureCreator<WindowContext>,
    text: &str,
) -> Result<(Texture<'tex>, u32, u32), String> {
    let char_w = GLYPH_W * GLYPH_SCALE;
    let char_h = GLYPH_H * GLYPH_SCALE;
    let gap = GLYPH_GAP;
    let text_w = text.len() as u32 * char_w + text.len().saturating_sub(1) as u32 * gap;
    let width = text_w + LABEL_PAD * 2;
    let height = char_h + LABEL_PAD * 2;

    let mut surface = Surface::new(width, height, PixelFormatEnum::RGBA8888)
        .map_err(|e| format!("failed to create label surface: {e}"))?;
    surface
        .fill_rect(None, Color::RGBA(20, 20, 24, 200))
        .map_err(|e| format!("failed to fill label surface: {e}"))?;

    for (idx, ch) in text.chars().enumerate() {
        let x0 = LABEL_PAD + idx as u32 * (char_w + gap);
        let y0 = LABEL_PAD;
        draw_glyph(&mut surface, ch, x0, y0, Color::RGB(250, 250, 250))?;
    }

    let texture = creator
        .create_texture_from_surface(&surface)
        .map_err(|e| format!("failed to create label texture: {e}"))?;
    Ok((texture, width, height))
}

fn draw_glyph(
    surface: &mut Surface,
    ch: char,
    x0: u32,
    y0: u32,
    color: Color,
) -> Result<(), String> {
    let rows = glyph_rows(ch).ok_or_else(|| format!("unsupported label character: {ch}"))?;
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..GLYPH_W {
            if bits & (1 << (GLYPH_W - 1 - col)) != 0 {
                let px = x0 + col * GLYPH_SCALE;
                let py = y0 + row as u32 * GLYPH_SCALE;
                let rect = Rect::new(px as i32, py as i32, GLYPH_SCALE, GLYPH_SCALE);
                surface
                    .fill_rect(rect, color)
                    .map_err(|e| format!("failed to draw glyph pixel: {e}"))?;
            }
        }
    }
    Ok(())
}

fn glyph_rows(ch: char) -> Option<[u8; 7]> {
    Some(match ch {
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'S' => [
            0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        _ => return None,
    })
}

fn draw_lane_labels(
    canvas: &mut Canvas<Window>,
    model: &IntersectionModel,
    assets: &RoadAssets<'_>,
) -> Result<(), String> {
    let mut label_idx = 0usize;
    for approach in Cardinal::ALL {
        for route in Route::ALL {
            let lane = model
                .lane(crate::intersection::lane_id(approach, route))
                .ok_or_else(|| format!("missing lane for {approach:?} {route:?}"))?;
            let (texture, w, h) = &assets.lane_labels[label_idx];
            label_idx += 1;

            let (x, y) = lane_label_screen_pos(lane.spawn_point, approach, *w, *h);
            let dst = Rect::new(x, y, *w, *h);
            canvas
                .copy(texture, None, dst)
                .map_err(|e| format!("draw lane label: {e}"))?;
        }
    }
    Ok(())
}

fn lane_label_screen_pos(
    spawn: crate::intersection::Vec2,
    approach: Cardinal,
    label_w: u32,
    label_h: u32,
) -> (i32, i32) {
    let half_w = (label_w / 2) as i32;
    let half_h = (label_h / 2) as i32;

    match approach {
        Cardinal::North => {
            let y = (spawn.y + LANE_LABEL_INSET).round() as i32 - half_h;
            let x = spawn.x.round() as i32 - half_w;
            (x, y.max(APPROACH_MARGIN as i32 / 2))
        }
        Cardinal::South => {
            let y = (spawn.y - LANE_LABEL_INSET).round() as i32 - half_h;
            let x = spawn.x.round() as i32 - half_w;
            (x, y)
        }
        Cardinal::East => {
            let x = (spawn.x - LANE_LABEL_INSET).round() as i32 - half_w;
            let y = spawn.y.round() as i32 - half_h;
            (x, y)
        }
        Cardinal::West => {
            let x = (spawn.x + LANE_LABEL_INSET).round() as i32 - half_w;
            let y = spawn.y.round() as i32 - half_h;
            (x, y)
        }
    }
}

/// Draw a single vehicle sprite rotated to match its path tangent.
fn draw_vehicle_sprite(
    canvas: &mut Canvas<Window>,
    assets: &RoadAssets<'_>,
    snapshot: &VehicleRenderSnapshot,
) -> Result<(), String> {
    let w = VEHICLE_LENGTH.round() as i32;
    let h = VEHICLE_WIDTH.round() as i32;
    let cx = snapshot.position.x.round() as i32;
    let cy = snapshot.position.y.round() as i32;

    // Per-approach sprite color matches spawn direction.
    let texture = match snapshot.approach {
        Cardinal::South => &assets.vehicle_south,
        Cardinal::North => &assets.vehicle_north,
        Cardinal::West => &assets.vehicle_west,
        Cardinal::East => &assets.vehicle_east,
    };

    // Texture is authored eastbound (+x); rotate by path tangent only (no axis swap).
    let dst = Rect::new(cx - w / 2, cy - h / 2, w as u32, h as u32);
    let angle_degrees = snapshot.heading_rad.to_degrees() as f64;

    canvas
        .copy_ex(texture, None, dst, angle_degrees, None, false, false)
        .map_err(|e| format!("draw vehicle sprite: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{APPROACH_ARM_LENGTH, EW_ARM_LENGTH, WINDOW_HEIGHT, WINDOW_WIDTH};

    #[test]
    fn road_asset_paths_are_under_assets_dir() {
        let path = Path::new(ROAD_ASSET_DIR).join("intersection_core.bmp");
        assert!(path.starts_with("assets/roads"));
    }

    #[test]
    fn layout_constants_fit_default_window() {
        let half = (ROAD_ARM_WIDTH / 2.0) as i32;
        let ns_arm = APPROACH_ARM_LENGTH as i32;
        let ew_arm = EW_ARM_LENGTH as i32;
        assert!(INTERSECTION_CENTER_X as i32 - half - ew_arm >= 0);
        assert!(INTERSECTION_CENTER_X as i32 + half + ew_arm <= WINDOW_WIDTH as i32);
        assert!(INTERSECTION_CENTER_Y as i32 - half - ns_arm >= 0);
        assert!(INTERSECTION_CENTER_Y as i32 + half + ns_arm <= WINDOW_HEIGHT as i32);
    }

    #[test]
    fn vehicle_asset_paths_are_under_assets_dir() {
        let path = Path::new(VEHICLE_ASSET_DIR).join("vehicle_east.bmp");
        assert!(path.starts_with("assets/vehicles"));
    }

    #[test]
    fn vehicle_sprite_texture_is_authored_eastbound() {
        let w = VEHICLE_LENGTH.round() as i32;
        let h = VEHICLE_WIDTH.round() as i32;
        assert!(w > h, "default sprite is longer along +x (eastbound)");
    }
}
