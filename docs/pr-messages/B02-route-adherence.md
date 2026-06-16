# PR Review: B02 — Route adherence

**Verdict**: Request changes  
**Branch**: `andy/B02`  
**PR**: [sofiabusho/smart-road#4](https://github.com/sofiabusho/smart-road/pull/4)  
**Ticket**: B02  
**PR message**: `docs/pr-messages/B02-route-adherence-pr.md` (found)

## Automated checks

| Command | Result |
|---------|--------|
| `cargo test` | **not run locally** — linker error `LNK1181: cannot open input file 'SDL2.lib'` on reviewer Windows host; author reports 35 passed |
| `cargo clippy -- -D warnings` | **not run locally** — same SDL2 link dependency |
| `cargo fmt --check` | **pass** |
| `cargo build` | **not run locally** — same SDL2 link dependency |

> Re-run `cargo test`, `cargo clippy -- -D warnings`, and `cargo build` in CI or an SDL2-equipped environment before merge. Do not treat author-reported green checks as verified until reproduced.

## Traceability

| ID | Claimed | Verified | Notes |
|----|---------|----------|-------|
| REQ-2 | yes (tracker) | **partial** | 12 polylines exist with distinct r/s/l exit geometry, but **9 of 12** `path[0]` values do not match `LaneInfo.spawn_point` (see Blocker below). Only the **left** lane per approach aligns. |
| REQ-6 | yes | **no** | `advance_along_path()` is implemented and wired each frame, but vehicles spawned on right/straight lanes start **40 px off-lane** and move along another lane's polyline — violates "follow that route" / no lane changes. |
| AUD-28 | yes (ticket gate) | **no** | Ticket verification gate is AUD-28; PR manual section marks REQ-6/AUD-26 but **does not document AUD-28** visual verification. One horizontal unit test does not satisfy "spawn on each approach; observe paths." |
| AUD-26 | partial (out of scope) | n/a | Correctly deferred; not a B02 gate. |
| REQ-11 | prep (A07) | **partial** | `heading_rad` updated from segment tangent in `advance_along_path`; adequate groundwork, not ticket scope. |

## Findings

| Severity | Location | Finding |
|----------|----------|---------|
| **Blocker** | `src/intersection.rs` (`build_all_lane_paths`) | **Path start points mismatch spawn points for 9/12 lanes.** Each approach uses a single hardcoded `path[0]` (always the **left** lane coordinate) for all three routes. Example: North Right spawns at `(552, 48)` but path starts at `(472, 48)`. Vehicles on r/s lanes never start on their polyline and drift along the wrong lane center (segment movement uses segment axis, not snap-to-path). Verified by comparing `spawn_point_for()` math to `build_all_lane_paths()` constants. |
| **Blocker** | `docs/audit.md` AUD-28 | **AUD-28 not evidenced.** Gate requires observing vehicles on **each approach** following fixed r/s/l routes with no lane changes. No manual verification notes; no per-lane or per-approach tests. |
| **Major** | `docs/ticket-tracker.md` | **Tracker not updated.** B02 row still ⬜; should be 🟢 in-progress or ✅ on completion per project workflow. |
| **Major** | `docs/SDS.md` §13.2 | **API contract drift.** `SpawnSystem::update` signature changed to `update(&mut self, model: &IntersectionModel, dt: f32)` in `src/spawn.rs` but SDS §13.2 still documents `update(&mut self, dt: f32)`. Cross-track edit to Track A-owned `spawn.rs` requires SDS update per §13.1 rule. |
| **Major** | `src/spawn.rs`, `src/app.rs` | **Cross-track file edits without SDS announcement.** Track B changed Track A-owned `spawn.rs` (signature + path call) and `app.rs` (integration line). Functionally necessary for B02, but PR does not call out SDS §13.1 compliance / ownership exception. |
| **Major** | `src/intersection.rs` | **No tests that built-in paths align with spawn points or cover all 12 lanes.** `advance_along_path` test uses a synthetic 2-point path; regression on hardcoded geometry is unguarded. |
| **Minor** | Branch name | Branch is `andy/B02`; convention in `AGENTS.md` is `feat/B02-<short-desc>`. |
| **Minor** | `docs/pr-messages/B02-route-adherence-pr.md` | Template expects a **Key Changes** section; report uses **Implementation Traceability** instead (content is good, structure diverges). |
| **Minor** | `docs/SDS.md` §13.3 | `attach_paths` is documented under Track B vehicle exports but implemented in `intersection.rs`. Acceptable given B owns `path` on `LaneInfo`, but consider re-export or SDS file note for clarity. |
| **Nit** | `src/vehicle.rs` | `integrate_physics` then `advance_along_path` each frame: first step moves along spawn `heading_rad`, second overrides position. Acknowledged in PR as interim; may cause one-frame lateral jitter until spawn/path alignment is fixed. |

## Checklist summary

- [ ] Ticket scope respected — core B02 work present; geometry bug breaks REQ-6
- [ ] Cross-track ownership respected — necessary edits made; **SDS not updated**
- [x] PR message artifact present and mostly accurate — automated claims unverified locally
- [ ] Tracker updated
- [ ] Tests adequate — one path unit test; no lane/spawn alignment coverage
- [ ] Audit items verified — **AUD-28 not demonstrated**

## Merge recommendation

**Do not merge yet.** The path-following machinery (`advance_along_path`, `LaneInfo.path`, spawn integration) is structurally sound and matches SDS §13.3 intent, but **hardcoded polylines use the wrong lane coordinates for right and straight routes on every approach** — only left-lane `path[0]` matches `spawn_point`. That undermines REQ-6 and AUD-28 for 75% of lanes. Fix `build_all_lane_paths()` so each route's waypoints use `spawn_point_for(approach, route)` (or equivalent lane-center math) for `path[0]` and lane-consistent junction segments; add tests asserting `lane.path[0] == lane.spawn_point` for all 12 lanes; document AUD-28 manual pass; update `docs/ticket-tracker.md` and SDS §13.2 `SpawnSystem::update` signature. Re-run full `cargo test` / `clippy` / `build` before re-review.

---

*Reviewed against `.agents/workflows/review-pr.md` on branch `origin/andy/B02` (commits `c613bb3`–`a2bd61a`). Reviewer could not link SDL2 locally; `cargo fmt --check` passed.*
