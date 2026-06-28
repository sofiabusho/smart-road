## Summary

- Implements post-session **statistics window** on `Esc` (REQ-17, REQ-19)
- Second SDL window lists all C05 collector fields with audit-aligned labels (AUD-18–AUD-26 display)
- Embedded 8×8 bitmap font renderer — no SDL2_ttf dependency
- `end_session()` hook in `app.rs`; audit mirror smoke tests + live window check
- **Fix:** corrected bitmap font bit order (text no longer mirrored horizontally)

## Test plan

### Automated

- [x] `cargo test` — **86 passed** (70 unit + 12 smoke + 1 manual ignored by default)
- [x] `cargo clippy -- -D warnings`
- [x] `cargo fmt --check`
- [x] `crate_smoke_audit18_four_vehicle_session_no_collision`
- [x] `crate_smoke_audit19_stats_window_is_separate_surface`
- [x] `crate_smoke_audit25_single_vehicle_equal_crossing_times`
- [x] `manual_audit19_stats_window_opens` (run with `--ignored`; Windows SDL2 on PATH)

### Manual audit (C06 / `docs/audit.md`)

- [x] **AUD-18** — Four vehicles (2× Up + 2× Right), all cross without collision
- [x] **AUD-19** — Separate statistics window appears on `Esc`; text renders correctly (font fix verified)
- [x] **AUD-20** — Max vehicles passed shows **4** after four-vehicle session
- [x] **AUD-21** — Max and min velocity displayed with numeric values
- [x] **AUD-22** — Max crossing time displayed
- [x] **AUD-23** — Min crossing time displayed
- [x] **AUD-24** — Close calls count shown (0 valid in test scenario)
- [x] **AUD-25** — Single vehicle: max crossing time = min crossing time
- [ ] **AUD-26** — Reported crossing time matches stopwatch observation (pending hands-on timing)

**Audit progress: 8 / 9 tasks completed (AUD-18–AUD-25)**

## Ticket

C06 — Stats window on Esc

## PR artifact

`docs/pr-messages/C06-stats-window-pr.md`
