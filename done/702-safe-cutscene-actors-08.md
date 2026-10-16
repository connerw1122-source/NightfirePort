## Your task: `safe-cutscene-actors-08` — 07000008's cut-scene shows its actors

Playtest 1013-1758 (07000008): "Kiko is the only one showing in this cutscene besides me. In the beginning, the voice
is played... But I can't see any other models. Any character models. Very concerning because I used to see some of
them." List the 08 cut-scene's actors from its script (the entity creates, skins, attach) and check each is created,
bound to its skin and posed (`script_anim` returns a pose) in the port. Find the regression (likely a merge since the
time the user saw them; bisect the cut-scene actor tests over master history if needed). Tests: every script actor
of the 08 scene has a live pose during its window.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/safe-cutscene-actors-08`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T16:05Z

---
## Report (done, 2026-10-16T17:39Z)

