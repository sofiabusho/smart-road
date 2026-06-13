//! SDL2 drawing for intersection and vehicles (A03/A07).

use std::path::Path;

use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::config::{INTERSECTION_CENTER_X, INTERSECTION_CENTER_Y, ROAD_WIDTH};
use crate::intersection::IntersectionModel;

const ROAD_ASSET_DIR: &str = "assets/roads";

/// Loaded road tile textures (lives for the `TextureCreator` lifetime).
pub struct RoadAssets<'tex> {
    approach_ns: Texture<'tex>,
    approach_ew: Texture<'tex>,
    intersection_core: Texture<'tex>,
    road_width: u32,
    arm_length: u32,
}

impl<'tex> RoadAssets<'tex> {
    pub fn load(creator: &'tex TextureCreator<WindowContext>) -> Result<Self, String> {
        let approach_ns = load_texture(creator, "approach_ns.bmp")?;
        let approach_ew = load_texture(creator, "approach_ew.bmp")?;
        let intersection_core = load_texture(creator, "intersection_core.bmp")?;

        let road_width = approach_ns.query().width;
        let arm_length = approach_ns.query().height;

        Ok(Self {
            approach_ns,
            approach_ew,
            intersection_core,
            road_width,
            arm_length,
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
    let al = assets.arm_length as i32;
    let cx = INTERSECTION_CENTER_X as i32;
    let cy = INTERSECTION_CENTER_Y as i32;
    let half = (ROAD_WIDTH / 2.0) as i32;

    let core_dst = Rect::new(cx - half, cy - half, rw, rw);
    canvas
        .copy(&assets.intersection_core, None, core_dst)
        .map_err(|e| format!("draw intersection core: {e}"))?;

    let north_dst = Rect::new(cx - half, cy - half - al, rw, al as u32);
    canvas
        .copy(&assets.approach_ns, None, north_dst)
        .map_err(|e| format!("draw north approach: {e}"))?;

    let south_dst = Rect::new(cx - half, cy + half, rw, al as u32);
    canvas
        .copy(&assets.approach_ns, None, south_dst)
        .map_err(|e| format!("draw south approach: {e}"))?;

    let west_dst = Rect::new(cx - half - al, cy - half, al as u32, rw);
    canvas
        .copy(&assets.approach_ew, None, west_dst)
        .map_err(|e| format!("draw west approach: {e}"))?;

    let east_dst = Rect::new(cx + half, cy - half, al as u32, rw);
    canvas
        .copy(&assets.approach_ew, None, east_dst)
        .map_err(|e| format!("draw east approach: {e}"))?;

    Ok(())
}

/// Draw one frame (intersection + vehicles). A03: intersection assets; A07: vehicles.
pub fn draw_frame(
    canvas: &mut Canvas<Window>,
    intersection: &IntersectionModel,
    assets: &RoadAssets<'_>,
) -> Result<(), String> {
    draw_intersection(canvas, intersection, assets)?;
    Ok(())
}

/// Vehicle render snapshot (A07).
#[derive(Debug, Clone, Copy)]
pub struct VehicleRenderSnapshot {
    pub position: crate::intersection::Vec2,
    pub heading_rad: f32,
    pub texture_id: u32,
}

/// Draw a single vehicle sprite (A07 stub).
pub fn draw_vehicle(
    _canvas: &mut Canvas<Window>,
    _snapshot: &VehicleRenderSnapshot,
) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{APPROACH_ARM_LENGTH, WINDOW_HEIGHT, WINDOW_WIDTH};

    #[test]
    fn road_asset_paths_are_under_assets_dir() {
        let path = Path::new(ROAD_ASSET_DIR).join("intersection_core.bmp");
        assert!(path.starts_with("assets/roads"));
    }

    #[test]
    fn layout_constants_fit_default_window() {
        let half = (ROAD_WIDTH / 2.0) as i32;
        let arm = APPROACH_ARM_LENGTH as i32;
        assert!(INTERSECTION_CENTER_X as i32 - half - arm >= 0);
        assert!(INTERSECTION_CENTER_X as i32 + half + arm <= WINDOW_WIDTH as i32);
        assert!(INTERSECTION_CENTER_Y as i32 - half - arm >= 0);
        assert!(INTERSECTION_CENTER_Y as i32 + half + arm <= WINDOW_HEIGHT as i32);
    }
}
