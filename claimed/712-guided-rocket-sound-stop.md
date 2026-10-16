## Your task: `guided-rocket-sound-stop` — the guided rocket's flight sound outlives the rocket

Playtest 1015-1738 (07000005, the Sentinel's guided rocket): "when the rocket finishes, I can still hear a sound being
played from where I was standing".

Find every sound the guided launch starts: the launch sound, the flight loop and the camera-ride sounds. For each,
find the game's stop and ownership: does the loop follow the rocket object, and is it stopped by the detonation /
`Bullet` death / the camera hand-back? [M the guided rocket's bullet update + the sound calls with their handle
storage]. Port the stops and the positions.

Test: a guided shot on 05 through the real fire path; after the detonation and the hand-back, no voice from the
launch is still playing, and while it flies the loop's position tracks the rocket, not Bond.

First check whether current master already behaves like the game. If it does, pin it with a test that would fail on the old behaviour, and say so in the report. Don't change working code to make a task look busy.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guided-rocket-sound-stop`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T17:43Z
