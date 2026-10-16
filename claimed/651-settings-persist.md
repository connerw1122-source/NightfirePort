## Your task: `settings-persist` — the PC port saves its options between runs

The front-end option pages have these row sets in `crates/nf-view/src/menu.rs`:
- Advanced: `ADV_ROWS`
- Multiplayer: `MP_ROWS`
- Audio/video: `AV_ROWS`

Today `default_opts()` resets them every launch. On PS2 they live in the memory-card save. This PC port should keep
them in a small config file in the user's config directory, `%APPDATA%\NightfirePort\settings.toml` or the
platform equivalent. Resolve the path with std only; don't add a `dirs` crate unless the workspace already has one.

- Load the file at start (missing or corrupt → defaults, with a warning on stderr) and save it when an options page
  is confirmed or backed out of.
- Store the values by row label key, not by index, so reordering rows doesn't scramble saved settings.
- Unknown keys are ignored and out-of-range values are clamped.
- Also persist the window settings the port already has: size/fullscreen, if they exist as state.
- Coordinate with `635-memcard-save-flow`: game progress stays in the save-game model; this file is settings only.
  Say so in docs/systems/.
- Tests: round trip, missing file, corrupt file, unknown key, reordered rows, and the sister assertion (a changed
  value really changes the file). Tests must write to a temp dir, never the real user dir (an env override such
  as `NF_CONFIG_DIR`).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/settings-persist`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T02:30Z
