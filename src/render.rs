//! SDL2 drawing for intersection and vehicles.

use std::path::Path;

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::config::{
    APPROACH_MARGIN, INTERSECTION_CENTER_X, INTERSECTION_CENTER_Y, ROAD_ARM_WIDTH, VEHICLE_LENGTH,
    VEHICLE_WIDTH,
};
use crate::intersection::{Cardinal, IntersectionModel, Route, VehicleRenderSnapshot};

const ROAD_ASSET_DIR: &str = "assets/roads";
const LANE_LABEL_INSET: f32 = 80.0;
const GLYPH_W: u32 = 5;
const GLYPH_H: u32 = 7;
const GLYPH_SCALE: u32 = 2;
const GLYPH_GAP: u32 = 2;
const LABEL_PAD: u32 = 3;

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
        let approach_ns = load_texture(creator, "approach_ns.bmp")?;
        let approach_ew = load_texture(creator, "approach_ew.bmp")?;
        let intersection_core = load_texture(creator, "intersection_core.bmp")?;

        // Per-approach vehicle sprites (colored rectangles) rotated in `draw_frame` via SDL.
        let vehicle_south = create_vehicle_texture(creator, vehicle_color(Cardinal::South))?;
        let vehicle_north = create_vehicle_texture(creator, vehicle_color(Cardinal::North))?;
        let vehicle_west = create_vehicle_texture(creator, vehicle_color(Cardinal::West))?;
        let vehicle_east = create_vehicle_texture(creator, vehicle_color(Cardinal::East))?;

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
    file_name: &str,
) -> Result<Texture<'tex>, String> {
    let path = Path::new(ROAD_ASSET_DIR).join(file_name);
    let surface = Surface::load_bmp(&path)
        .map_err(|e| format!("failed to load road asset {}: {e}", path.display()))?;
    let texture = creator
        .create_texture_from_surface(&surface)
        .map_err(|e| format!("failed to create texture from {}: {e}", path.display()))?;
    Ok(texture)
}

/// Create a solid-colored vehicle sprite texture that we can rotate.
fn create_vehicle_texture<'tex>(
    creator: &'tex TextureCreator<WindowContext>,
    color: Color,
) -> Result<Texture<'tex>, String> {
    let width = VEHICLE_LENGTH.round() as u32;
    let height = VEHICLE_WIDTH.round() as u32;
    let mut surface = Surface::new(width, height, PixelFormatEnum::RGBA8888)
        .map_err(|e| format!("failed to create vehicle surface: {e}"))?;
    surface
        .fill_rect(None, color)
        .map_err(|e| format!("failed to fill vehicle surface: {e}"))?;
    let texture = creator
        .create_texture_from_surface(&surface)
        .map_err(|e| format!("failed to create vehicle texture: {e}"))?;
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

    // South/West arms reuse the same BMP as North/East but inbound lanes sit on the
    // opposite half (RHT mirror), so flip tiles to align markings with lane paths.
    let south_dst = Rect::new(cx - half, cy + half, rw, ns_al as u32);
    canvas
        .copy_ex(&assets.approach_ns, None, south_dst, 0.0, None, true, false)
        .map_err(|e| format!("draw south approach: {e}"))?;

    let west_dst = Rect::new(cx - half - ew_al, cy - half, ew_al as u32, rw);
    canvas
        .copy_ex(&assets.approach_ew, None, west_dst, 0.0, None, false, true)
        .map_err(|e| format!("draw west approach: {e}"))?;

    let east_dst = Rect::new(cx + half, cy - half, ew_al as u32, rw);
    canvas
        .copy(&assets.approach_ew, None, east_dst)
        .map_err(|e| format!("draw east approach: {e}"))?;

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

fn vehicle_color(approach: Cardinal) -> Color {
    match approach {
        Cardinal::South => Color::RGB(230, 70, 70),
        Cardinal::North => Color::RGB(70, 130, 230),
        Cardinal::West => Color::RGB(240, 180, 40),
        Cardinal::East => Color::RGB(60, 190, 90),
    }
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
    fn vehicle_sprite_texture_is_authored_eastbound() {
        let w = VEHICLE_LENGTH.round() as i32;
        let h = VEHICLE_WIDTH.round() as i32;
        assert!(w > h, "default sprite is longer along +x (eastbound)");
    }
}
