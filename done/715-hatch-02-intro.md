## Your task: `hatch-02-intro` — the hatch in 07000002's intro cut-scene never opens

Playtest 1014-1725 [intro02]: "the hatch still doesn't open".

Find the hatch object the 02 intro's script animates [D the level's script lib: the intro NIS streams, the
`Script_AnimStart` / object event that targets it]. Then find why the port's run of that stream doesn't move it:
the wrong object, a missing event type, or a placement that never resolves.

Test: run the intro through the real path; at the stream's open key the hatch object's transform has changed by the
animation's delta. Sister: before the key it is closed.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour, and say so in the report. Don't change working code to make a task look busy.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/hatch-02-intro`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T18:43Z

---
## Report (done, 2026-10-16T20:22Z)

