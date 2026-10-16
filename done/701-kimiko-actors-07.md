## Your task: `kimiko-actors-07` — the right actors in the 07000007 Kimiko cut-scene

Playtest 1012-2337: "models in that cutscene arent accurate". Earlier reports from the same scene: duplicates in the
Drake scene, and models present before they should be. List every actor the Kimiko scene's script creates or uses:
its skin hash, its spawn/despawn ticks, whether it's a script actor or a level drone reused [M the script's entity
create ops, Script_AnimStart 0x1d8f40, the actor binding]. Compare against the script data and fix the port's binding
(the wrong skin, a level drone drawn instead of the script actor, a missing despawn). Tests: the actor set and skins
per scene phase match the script, with no duplicates.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/kimiko-actors-07`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T15:37Z

---
## Report (done, 2026-10-16T16:47Z)

Closed by the driver: the report is on `s/kimiko-actors-07` (docs/reports/kimiko-actors-07.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T17:33Z)

