## Your task: `guard-reaction-delay` — guards don't fire the instant they see you

The user, many times: "guards shoot way too soon upon spotting you, at least give it half a second" and "his gun's
firing before it lowers at me" (07000006 wine cellar, the searchlights, 07000005). Port the game's
sight-to-fire pipeline exactly:
- the noticed/alert latch timers;
- the aim-raise animation gate (the fire arm must wait for the aim pose);
- the first-shot delay / reaction time, per difficulty if the game scales it;
- the searchlight-spot path [M the NDrone sight/attack states and their timers; 110f's sight-pause finding, 61
  frames measured in PCSX2, is in the WORKLOG].
Measure the frames from first sight to first shot for 3 guard types and table them against the game's numbers.
Keep `walkreal_07000001` and the `ai_live` tests green (the 110f hold broke them: read why first).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guard-reaction-delay`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T05:17Z

---
## Report (done, 2026-10-16T08:25Z)

Closed by the driver: the report is on `s/guard-reaction-delay` (docs/reports/guard-reaction-delay.md); the worker didn't run finish.sh.
