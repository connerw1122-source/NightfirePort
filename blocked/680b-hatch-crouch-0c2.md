## Your task: `hatch-crouch-0c2` — land 680's crouch fix without breaking movement everywhere else

`s/hatch-crouch-0c` (680) was HELD on 2026-10-16. Its new grounded-rule "pick-contact arm" in `player.rs` breaks 6
movement tests on master:
- `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`;
- `fix_ladder_water_live::dismount_into_the_pond_lets_go_of_the_ladder`;
- `walk_0700004a_live::the_descent_plates_fire_and_the_objective_reveals`;
- `fix_pickups_trace_live::with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace`;
- `fix_input_kbd_trace::keyboard_trace_unchanged`;
- its own `hatch_crouch_live::from_the_hatch_top_the_open_hole_drops_the_crouch_onto_the_balcony`, which failed on
  the merged tree.

Several of these are PCSX2-measured traces, so the grounded rule must match the game everywhere, not just under the
0c hatch. Re-derive the pick-contact arm from the decompile (Player_Collide / the grounded test) [M], find what the
traces show that your arm gets wrong, and land a rule that passes all of them AND the 0c crouch. Also, don't commit
scratch probes (`hatch_crouch_probe.rs` was removed in the merge).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/hatch-crouch-0c2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-16T11:34Z

---
## Report (blocked, 2026-10-16T15:12Z)

