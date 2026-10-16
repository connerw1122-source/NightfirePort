## Your task: `drive-hud-data`, what the driving HUD shows

From DRIVING.ELF (docs/atlas/driving-elf.md): the driving missions' HUD — speed/gear, the vehicle's health/damage,
weapon + ammo, the minimap/radar, objective markers, timers. Port the data model the HUD reads as a per-frame snapshot
(values + label/texture ids + positions as the game sets them), wired to 506's physics/107's state where they exist.
The main agent draws it. Tests: the snapshot follows the sim (numbers only).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-hud-data`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T01:18Z

---
## Report (done, 2026-10-16T02:51Z)

