## Your task: `pause-music-1b` — music keeps playing in the pause menu on 0700001b

Playtest 1016-1355 (the user, on master 2026-10-16): "Music still plays during the pause menu" (0700001b). Earlier fixes stopped it on other levels. Find what is different on 1b (the level's music source or stream, a script-driven track, the space ambience) and pause or stop it the way the game's pause does [M the pause entry's music calls]. Test: pausing on 1b silences the level music; unpausing resumes it. Sister: 05 still behaves.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/pause-music-1b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
