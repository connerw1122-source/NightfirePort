## Your task: `escort-01b` — Mayhew's escort on 07000001, without breaking the exit door

Your earlier branch `s/escort-01` (task 690) was HELD at merge (2026-10-16). Start from `origin/s/escort-01` merged
with the current `origin/master`.

The merge suite failed on its own: with escort-01 alone on master,
`crates/nf-view/tests/fix_mission_live.rs::the_07000001_exit_works_end_to_end` fails:

    the courtyard shoji reached OPEN (state 4) at least once [M NDrone2_DoorIsOpen 0x14aaa0] — an activation every
    tick kept it ping-ponging 0x8<->0x10

So the new escort follow re-activates the courtyard door every tick, and the door never reaches OPEN. In the batch
`npc_doors_live::the_escort_opens_his_door_without_bonds_help` also failed; check it on your tree too.

Find which part of the escort change touches the door: the follow, the waiting at doors, or the door use. Do what
the game does: a door being opened is not re-activated, per `NDrone2_DoorIsOpen` and the drone door handler's
state checks [M]. Keep all of these green: fix_mission_live (all 4), npc_doors_live, walkreal_07000001_live,
escort_follow_07000001_live. Run the FULL suite before you finish, and list every failure.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/escort-01b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
