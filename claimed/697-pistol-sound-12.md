## Your task: `pistol-sound-12` — the starting pistol on 07000012 sounds right

Playtest 1012-1723 (07000012): "the pistol you start with does not sound right". Find 12's start loadout weapon (the
level kit [M Player_InitWeapon]) and its fire/reload sound ids [L weapon_data], and which sound bank 12 loads. Check
the port's driver resolves each id from the bank the GAME would use (fix-weapon-sounds' own-bank-first rule, the
spare banks), and that the effect record/sample is the same one. Test: fire the start pistol on 12 and assert the
played sample hash equals the owning bank's decode for each id. Sister: a different weapon's id doesn't match.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/pistol-sound-12`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T14:13Z
