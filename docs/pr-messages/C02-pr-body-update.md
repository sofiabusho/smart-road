## Summary

- Implements smart intersection velocity scheduling in `SmartController` for `Managed` vehicles (REQ-3, REQ-9 partial)
- Builds lane conflict graph from path geometry; FIFO entry priority commands later entrants to yield
- Smart tick runs before movement; spawn queue offset + proximity clamp prevent overlap
- **7 / 7 collision audit tasks completed** (AUD-8–AUD-14) via audit-mirror smoke tests

## Test plan

- [x] `cargo test` — 87 passed (70 unit + 17 smoke)
- [x] `cargo clippy -- -D warnings`
- [x] `cargo fmt --check`
- [x] `cargo build` / `cargo run` (SDL2 configured; WSL spot-check)
- [x] **AUD-8** — `crate_smoke_audit8_three_same_lane_all_approaches`
- [x] **AUD-9** — `crate_smoke_audit9_one_west_three_east`
- [x] **AUD-10** — `crate_smoke_audit10_one_south_three_east`
- [x] **AUD-11** — `crate_smoke_audit11_one_south_three_west`
- [x] **AUD-12** — `crate_smoke_audit12_one_north_three_east`
- [x] **AUD-13** — `crate_smoke_audit13_one_north_three_west`
- [x] **AUD-14** — `crate_smoke_audit14_five_south_two_west`
- [ ] **AUD-15** — deferred to C03

**Audit progress: 7 tasks completed (AUD-8–AUD-14)**

## Ticket

C02 — Smart scheduler

## PR artifact

`docs/pr-messages/C02-smart-scheduler-pr.md`
