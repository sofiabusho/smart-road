## Summary

- Adds **bonus session statistics** beyond REQ-20–REQ-26 (REQ-B2 / AUD-B1)
- Stats window shows an **Additional statistics (bonus)** section after mandatory fields
- Tracks session duration, average crossing time, peak concurrent vehicles in zone, and zone entries
- Includes C06 font fix: bitmap text renders correctly (not mirrored)

## Bonus metrics (REQ-B2)

| Field | Description |
|-------|-------------|
| Session duration (s) | Wall-clock session time at `Esc` |
| Avg crossing time (s) | Mean crossing time for vehicles that completed |
| Peak concurrent in zone | Max simultaneous managed/exiting vehicles |
| Vehicles entered zone | Distinct vehicles detected by smart system |

## Test plan

### Automated

- [x] `cargo test` — **85 passed** (73 unit + 12 smoke)
- [x] `cargo clippy -- -D warnings`
- [x] `cargo fmt --check`
- [x] `format_includes_bonus_statistics_for_aud_b1`
- [x] `bonus_stats_track_zone_entries_and_finalize_average`
- [x] `fix(C06): correct bitmap font bit order in stats window`

### Manual audit

- [x] **AUD-B1** — `Esc` → stats window shows bonus section beyond REQ-20–REQ-26 (verified visually)
- [x] **Stats window readability** — bonus + mandatory fields render correctly after font fix

**Audit progress: 1 / 1 bonus task completed (AUD-B1)**

## Dependencies

- **C06** (stats window) — this branch is stacked on `iana/C06`

## Ticket

C08 — Extra statistics *(bonus)*

## PR artifact

`docs/pr-messages/C08-extra-stats-pr.md`
