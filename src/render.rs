//! SDL2 drawing for intersection and vehicles (A03/A07).

use std::path::Path;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::config::{
    INTERSECTION_CENTER_X, INTERSECTION_CENTER_Y, ROAD_WIDTH, VEHICLE_LENGTH, VEHICLE_WIDTH,
};
use crate::intersection::{Cardinal, IntersectionModel};

const ROAD_ASSET_DIR: &str = "assets/roads";

/// Loaded road tile textures (lives for the `TextureCreator` lifetime).
pub struct RoadAssets<'tex> {
    approach_ns: Texture<'tex>,
    approach_ew: Texture<'tex>,
    intersection_core: Texture<'tex>,
    road_width: u32,
    ns_arm_length: u32,
    ew_arm_length: u32,
}

impl<'tex> RoadAssets<'tex> {
    pub fn load(creator: &'tex TextureCreator<WindowContext>) -> Result<Self, String> {
        let approach_ns = load_texture(creator, "approach_ns.bmp")?;
        let approach_ew = load_texture(creator, "approach_ew.bmp")?;
        let intersection_core = load_texture(creator, "intersection_core.bmp")?;

        let road_width = approach_ns.query().width;
        let ns_arm_length = approach_ns.query().height;
        let ew_arm_length = approach_ew.query().width;

        Ok(Self {
            approach_ns,
            approach_ew,
            intersection_core,
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
    let half = (ROAD_WIDTH / 2.0) as i32;

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

    Ok(())
}

/// Draw one frame (intersection + vehicles). A03: intersection; A04: vehicle rects; A07: sprites.
pub fn draw_frame(
    canvas: &mut Canvas<Window>,
    intersection: &IntersectionModel,
    assets: &RoadAssets<'_>,
    vehicles: &[VehicleRenderSnapshot],
) -> Result<(), String> {
    draw_intersection(canvas, intersection, assets)?;
    for snapshot in vehicles {
        draw_vehicle(canvas, snapshot)?;
    }
    Ok(())
}

/// Vehicle render snapshot (A04: position + heading; A07: texture rotation).
#[derive(Debug, Clone, Copy)]
pub struct VehicleRenderSnapshot {
    pub position: crate::intersection::Vec2,
    pub heading_rad: f32,
    pub approach: Cardinal,
}

/// Draw a single vehicle as a colored oriented rectangle (A04; A07 adds sprite rotation).
pub fn draw_vehicle(
    canvas: &mut Canvas<Window>,
    snapshot: &VehicleRenderSnapshot,
) -> Result<(), String> {
    let (w, h) = vehicle_dimensions(snapshot.approach);
    let half_w = w / 2;
    let half_h = h / 2;
    let cx = snapshot.position.x.round() as i32;
    let cy = snapshot.position.y.round() as i32;

    canvas.set_draw_color(vehicle_color(snapshot.approach));
    let rect = Rect::new(cx - half_w, cy - half_h, w as u32, h as u32);
    canvas
        .fill_rect(rect)
        .map_err(|e| format!("draw vehicle: {e}"))?;
    Ok(())
}

fn vehicle_dimensions(approach: Cardinal) -> (i32, i32) {
    match approach {
        Cardinal::North | Cardinal::South => {
            (VEHICLE_WIDTH.round() as i32, VEHICLE_LENGTH.round() as i32)
        }
        Cardinal::East | Cardinal::West => {
            (VEHICLE_LENGTH.round() as i32, VEHICLE_WIDTH.round() as i32)
        }
    }
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
        let half = (ROAD_WIDTH / 2.0) as i32;
        let ns_arm = APPROACH_ARM_LENGTH as i32;
        let ew_arm = EW_ARM_LENGTH as i32;
        assert!(INTERSECTION_CENTER_X as i32 - half - ew_arm >= 0);
        assert!(INTERSECTION_CENTER_X as i32 + half + ew_arm <= WINDOW_WIDTH as i32);
        assert!(INTERSECTION_CENTER_Y as i32 - half - ns_arm >= 0);
        assert!(INTERSECTION_CENTER_Y as i32 + half + ns_arm <= WINDOW_HEIGHT as i32);
    }

    #[test]
    fn vehicle_dimensions_swap_for_ew_approaches() {
        let (ns_w, ns_h) = vehicle_dimensions(Cardinal::South);
        let (ew_w, ew_h) = vehicle_dimensions(Cardinal::West);
        assert_eq!(ns_w, ew_h);
        assert_eq!(ns_h, ew_w);
    }
}
