---
title: "feat(A07): turn animation"
---

# PR Implementation Report: A07

## Summary

Implements **turn animation** by rendering each vehicle as a rotated sprite whose orientation follows the **path tangent** provided by B02’s lane polylines. This satisfies **REQ-11** and contributes to **NFR-3** by ensuring vehicles visually rotate as they traverse right, straight, and left routes through the junction, aligned with the lane paths validated in **AUD-28**.

## Key Changes

- **`src/render.rs`**: Extends `RoadAssets` with per-approach vehicle textures and updates `draw_frame` to render vehicles via a new internal `draw_vehicle_sprite` helper using `canvas.copy_ex` with `heading_rad`-derived rotation.
- **`src/render.rs`**: Removes the superseded `draw_vehicle` colored-rectangle helper; live frame rendering uses sprite-based path-tangent rotation only.

## Technical Decisions

- **Procedural vehicle sprites**: Instead of introducing new on-disk BMP assets (which would add repo and setup surface area for A07), simple per-approach colored rectangles are generated at startup as SDL `Texture`s and then rotated; this preserves A04’s color semantics while meeting REQ-11’s animation requirement.
- **Rotation source of truth**: Vehicle orientation uses `heading_rad` updated in `vehicle::advance_along_path`, ensuring that rendered rotation stays in lockstep with the B02 lane polylines (no independent angle math in `render.rs`).

## Verification Results

### Automated Checks

- [x] `cargo test` — all unit + integration tests pass (including intersection/path and render layout tests).
- [x] `cargo fmt --check` — clean (no diff).
- [x] `cargo clippy -- -D warnings` — clean (no warnings emitted).

### Manual Audit (against `docs/audit.md`)

- [ ] **AUD-28**: Pending manual reviewer check — run the sim, spawn vehicles from all approaches and routes, and confirm that as vehicles traverse each lane polyline they both (a) stay on their route and (b) visually rotate to follow the path curvature (no “sliding” through turns).

### Requirements Traceability

- [x] **REQ-11**: Vehicles are now rendered as rotated sprites whose orientation is driven by the path tangent (`heading_rad`) each frame, so turns visibly rotate rather than sliding with a fixed orientation.
- [x] **NFR-3**: Animation is clearly visible during turning, reinforcing that the project delivers an animated, not static, SDL2 simulation.

## Artifacts

- **Test output**: `cargo test` from project root — 51 unit tests + 6 integration tests passing.
- **Lint/format output**: `cargo fmt --check` and `cargo clippy -- -D warnings` both pass with no warnings or formatting diffs.

---

## Next Steps

A07 unblocks **C07**’s audit dry-run work for the “animation required” portions of Gate G2, once C03/C04 finish collision and sustained-traffic scenarios. Manual reviewers should record AUD-28 visual notes alongside B02’s route-adherence checks to confirm that sprite rotation and path-following both look correct on all 4×3 lane combinations.
