## Your task: `ai-traces2`, land fix-ai-traces' PlayScript work on master green

`s/fix-ai-traces` (35a65d1, report `docs/reports/fix-ai-traces.md`) built the trace-replay harness
(`crates/nf-view/tests/ai_traces_live.rs`) and ported `NDrone2_DSTATE_PlayScript` (state 3, `Drone::start_script` =
placement param 1, the `NDrone2_DSTATE_Global` 0x15b830 branch). Its report says its own suite was green, but merged
into master (f74deef) the full suite went **677 passed, 6 failed**, so the merge was backed out (not pushed):

- `fix_drone3_live::the_truck_driver_reaches_his_idle_state_and_never_twists` (fix_drone3_live.rs:50): the 07000006
  truck driver is now in state **3** where the test expects TruckDriverIdle **52** ("TruckDriverInit's script-less
  branch [M 0x160bd0 case 1 else]"). Either the driver really has a start script and that test's reading is wrong,
  or `start_script` reads a param word that isn't `Drone+0x554` for this drone type. Decide it from the decompile
  (`NDrone2_DefaultInit` 0x14b300 vs the TruckDriver init 0x160bd0) and the 05-truck trace, not by editing the test.
- `fix_npcs_live::an_idle_gate_guard_sees_and_attacks_bond_on_the_same_floor` (fix_npcs_live.rs:168): "d15 is an
  enabled idle guard" fails. d15 is presumably now in PlayScript.
- `fix_objects_live::a_death_draws_its_drop_and_nothing_else_moved` (fix_objects_live.rs:232),
  `fix_player3_live::death_drops_match_the_game_exactly` (fix_player3_live.rs:91) and
  `render2_live::dropped_guns_dim_to_the_cel_ambient` (render2_live.rs:247): all "the guard died" on 07000001.
  The guard these tests shoot is now held in the start script.
- `gpu_palette_matches_the_cpu_pose_on_07000005_06_07`: panics at `crates/nf-formats/src/anim.rs:70:34`,
  "index out of bounds: the len is 22 but the index is 22". `play_script` with a start-script id appears to run a
  sequence/key index one past the end. The port must never panic on it; find what the game does at that index.

Master has moved since your base (c8dae25): world-objects, walkreal-c, verify-bitexact (the exact sine everywhere)
and world-fx have landed. Some of the 6 may come from that combination. Start a fresh branch from `origin/master`,
bring over the harness and the PlayScript port, reproduce all 6 failures, and fix each from the decompile. Where an
older test's expectation is shown wrong by the trace or the decompile, correct it with the evidence cited in the
test. The UPDATE arm's missing `NDrone2_SeenAndAttacking` (your "unresolved") is likely why guards stay passive in
the shooting tests: port it if the evidence allows. No test may stay red. Run the full suite with `--no-fail-fast`
across all four crates on the merged tree and report the count.

Files: `crates/nf-game/src/drone.rs`, `crates/nf-view/tests/ai_traces_live.rs`, the 5 test files named above (only
where the evidence says their expectation is wrong), `crates/nf-formats/src/anim.rs` (the bounds fix only).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/ai-traces2`.

---
claimed-by: sandbox5 at 2026-10-13T11:32Z
