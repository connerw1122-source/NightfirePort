## Your task: `drone-investigate` — port NDrone2's Investigate state

The decompile census lists `NDrone2_Investigate` as not cited in the port. Port the state:
- what triggers it (a sound, a corpse, a thrown object);
- where the guard walks, how long he searches;
- how it hands back to his normal state or escalates to alert.

[M the state handler and its entry sites; the alert levels it reads.]

Tests on 07000005: a sound below the alert threshold sends a guard to investigate its origin, and he returns to his
post at the game's tick. Sister: one that hears nothing stays put.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour, and say so in the report. Don't change working code to make a task look busy.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drone-investigate`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T19:10Z
