## Your task: `guards-0c-b` — the 0c drawn-gate fix, without stopping Drake's spawn on 0700001b

Your earlier branch `s/guards-0c` (task 698) was HELD at merge (2026-10-16). Start from `origin/s/guards-0c`
merged with the current `origin/master`.

With guards-0c alone on master, this test fails (it passes on master):

    crates/nf-view/tests/fix_finale_live.rs::the_0700001b_drakes_defeat_ends_the_space_station
    fix_finale_live.rs:161 — "the gate spawns Drake (spawner #187 [D])"

So moving `visible_to`'s body rays onto the bullet segment also changed something that spawner #187's release on
0700001b depends on: a spawner visibility or "player can see" gate, a zero-g level detail, or a pass-flagged surface
on the station. Find which reader of `visible_to` (or of the drawn flag) the spawner path uses, and what the game
gates the spawner release on [M DroneSpawner_Control 0x137e68 and its release test]. Then:
- If the spawner should not use the drawn gate at all, decouple it the game's way.
- If it should, find why the bullet-segment query differs on 1b, and fix that.

Keep both green: guards_0c_live and all of fix_finale_live. Run the FULL suite before you finish and list every
failure.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guards-0c-b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
