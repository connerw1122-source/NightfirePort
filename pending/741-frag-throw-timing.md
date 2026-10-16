## Your task: `frag-throw-timing` — frag grenades leave the hand too early (a regression)

Playtest 1016-1355 (the user, on master 2026-10-16): "frag grenades are flying out way too soon. They don't let the animation finish. We're going in the opposite direction with that one" (07000014). Find the game's release frame for the frag throw (the fire script's spawn frame / the release-frame family [M Player_WeaponFiring 0x1a5460, +0x44]) and when the port's release changed: bisect the recent merges (grenade-rattle2, the throw-cadence work). Fix it so the grenade spawns on the game's frame. Test: a frag throw spawns the grenade exactly on the game's release tick, measured from the press.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/frag-throw-timing`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
