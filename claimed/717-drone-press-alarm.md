## Your task: `drone-press-alarm` — port NDrone2's PressAlarm state

The decompile census lists `NDrone2_PressAlarm` (and its helpers) as not cited anywhere in the port. Port it from
ACTION.ELF:
- how a guard picks an alarm panel;
- walking to it, the press animation;
- the channel it raises and when the attempt is abandoned (death, stun, being shot);
- which levels' guards use it [M + D: the placements whose drone setup can reach the state].

Tests: on a level that has alarm panels, an alerted guard runs to the nearest panel, presses it and the alarm
channel goes up at the game's tick. Sister: a guard shot on the way never raises it.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour, and say so in the report. Don't change working code to make a task look busy.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drone-press-alarm`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T19:00Z
