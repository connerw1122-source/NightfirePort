## Your task: `grenade-rattle` — a grenade resting on the ground stops making bounce sounds

Playtests 1013-1825 and 1013-2313: "once it kind of settles, it's rapidly ... contacting the ground", and "when a
grenade is on the ground it makes rapid bouncing sounds". Port the grenade's rest detection and the bounce-sound
gate:
- the speed/impulse threshold for the bounce sound;
- the settled state that stops collision responses [M Bullet_update's grenade arm, the bounce sound call and its
  conditions].

Test: drop a frag on flat ground on 07000005; count the bounce sounds over its life (the game's count, a few at
most), and none once it rests. Sister: a grenade thrown at a wall still sounds on impact.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/grenade-rattle`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
