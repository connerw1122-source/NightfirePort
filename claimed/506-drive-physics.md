## Your task: `drive-physics`, the driving missions' car physics

The driving missions run DRIVING.ELF (map: docs/atlas/driving-elf.md from 501; input path: 110w's
crates/nf-game/src/input/driving.rs; formats: 500 refpack, 503 CRP; tuning files `data	uning\Physics\…` in the
VIV archives). Port the player car's physics step: the tuning parameters (parse the .tun files), the integration
(engine/gear/brake/handbrake, tyre grip, steering, suspension if any), collision response against the track's
road/collision data, at the game's tick rate. A headless `nf_game::driving` sim with tests that drive a scripted
input on a flat test surface and assert the game's speed/turn numbers (derive them from the decompile; record any
measured PCSX2 numbers the main agent should take, precisely, as requests). No rendering.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-physics`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-15T20:08Z
