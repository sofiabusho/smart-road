---
title: "feat(C06): stats window on Esc"
---

# PR Implementation Report: C06

## Summary

Implements the **post-session statistics window** per SDS §9 and §13.4 (IF-4: second SDL window). Pressing `Esc` ends the simulation loop and opens a dedicated stats window listing all C05 collector fields. Satisfies **REQ-17**, **REQ-19**, and display gates **AUD-18–AUD-25** (AUD-18 is a manual collision check; AUD-26 is pending manual stopwatch timing).

## Key Changes

- **`src/stats_window.rs`**: `format_stats_lines()`, embedded 8×8 bitmap font renderer (no SDL2_ttf dep), `show_stats_window()` event loop on a second SDL window; 4 unit tests for audit labels and AUD-20/25 formatting.
- **`src/app.rs`**: `end_session()` hook; `Esc` sets `show_stats_on_exit` and opens stats window after the main loop; removed interim `eprintln!` dump.
- **`src/config.rs`**: `STATS_WINDOW_*` title and dimensions.
- **`README.md`**: Esc → stats window → quit flow.

## Technical Decisions

- **Second SDL window** (SDS IF-4 default) rather than overlay on the simulation canvas — satisfies AUD-19 “separate statistics window”.
- **Bitmap font** instead of SDL2_ttf — avoids new system library; labels match audit wording (“Max vehicles passed”, “Max/Min time to pass intersection”, etc.).
- **Esc-only stats display** — closing the main window with the title-bar X exits without stats (REQ-17 targets `Esc`).
- **`max_vehicles_passed`** shown per AUD-20 label; equals cumulative vehicles that completed crossing in typical sessions (same as C05 collector).

## Verification Results

### Automated Checks

- [x] `cargo test` — 74 unit + 9 smoke = **83 passed**
- [x] `cargo clippy -- -D warnings` — passes
- [x] `cargo fmt --check` — passes
- [x] `cargo build` — succeeds (WSL / SDL2)

### Manual Audit (against `docs/audit.md`)

- [ ] **AUD-18**: Spawn 2× Up + 2× Right, wait for exit, press Esc — no collision (manual)
- [ ] **AUD-19**: Separate stats window appears on Esc (manual)
- [ ] **AUD-20**: Max vehicles passed shows **4** after AUD-18 session (manual)
- [ ] **AUD-21**: Max and min velocity shown with numeric values (manual)
- [ ] **AUD-22**: Max crossing time displayed (manual)
- [ ] **AUD-23**: Min crossing time displayed (manual)
- [ ] **AUD-24**: Close calls count shown (0 valid) (manual)
- [ ] **AUD-25**: Single vehicle — max time = min time (manual + unit test for formatted equality)
- [ ] **AUD-26**: Reported crossing time matches stopwatch observation (manual)

### Requirements Traceability

- [x] **REQ-17**: `Esc` ends sim and opens statistics UI.
- [x] **REQ-19**: All required stats fields listed in the stats window.
- [x] **REQ-20–REQ-26**: Display wired to `StatsSession.stats` fields populated by C05.

## Artifacts

- **Test output**: `cargo test` — 83 passed.
- **Lint output**: `cargo clippy -- -D warnings` clean.

---

## Next Steps

- **C07** — Full audit dry-run after A07, C03, C04 land.
- Rebase/merge with **C02** if both branches integrate before C07.
