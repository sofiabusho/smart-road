//! SDL2 drawing for intersection and vehicles (A03/A07).

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::intersection::IntersectionModel;

/// Draw one frame (intersection + vehicles). A01: clear background only via caller.
pub fn draw_frame(
    _canvas: &mut Canvas<Window>,
    _intersection: &IntersectionModel,
) -> Result<(), String> {
    // A03: road assets; A07: rotated vehicle sprites.
    Ok(())
}
