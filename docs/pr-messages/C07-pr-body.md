## Summary
- Closes **Gate G2**: README audit runbook (controls, smoke-test map, known limitations).
- Fixes pre-audit issues: cross-lane close-call detection, multi-key spawn batching, spawn RNG seed, min velocity includes 0.
- Marks **C07** and all mandatory tickets complete in `docs/ticket-tracker.md`.

## Test plan
- [x] `cargo test --lib` (90 passed)
- [x] `cargo test --test smoke` (23 passed)
- [x] `cargo clippy -- -D warnings`
- [x] `cargo fmt --check`
- [x] **Manual `cargo run`** (Windows, SDL2 DLL on `PATH`) — main window opens with intersection + road/vehicle BMP assets visible (**AUD-1**, **AUD-2**)
- [x] **`cargo test --test manual_stats_window manual_audit19_stats_window_opens -- --ignored --nocapture`** — stats window opens and closes cleanly (**AUD-19** interactive)

### Manual verification notes (Windows)
- SDL2 runtime: add `C:\Users\ianna\SDL2\SDL2-2.30.10\lib\x64` to `PATH` (or copy `SDL2.dll` next to the binary).
- `manual_audit19_stats_window_opens` passed in ~15s (separate stats surface titled `smart-road — session statistics`).
- `cargo run --release`: `smart-road` main window detected; intersection rendered with 6-lane road assets and vehicle sprites.
- Full **Esc** flow from the live sim is best confirmed interactively (spawn → wait → **Esc**); AUD-19 behaviour is covered by the dedicated manual test above.
