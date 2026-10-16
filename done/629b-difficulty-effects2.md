## Your task: `difficulty-effects2` — land 629 (HELD: the branch doesn't compile, 10 failures on master)

`s/difficulty-effects` (629) was HELD on 2026-10-15 by the main agent.
- **The branch tip does not compile:** `crates/nf-view/src/menu.rs` has
  `self.requested_difficulty = Some(self.difficulty + 1);` but `FrontEnd::difficulty` is `usize` and
  `requested_difficulty` is `Option<u32>` (E0308). Your final merge of origin/master was never built, so the
  report's suite tallies can't be from that tree. **Build and run the suite on the exact tree you push.**
- **It conflicts with master's 053 net-match-host:** both add a parameter to `nf_view::load_impl`. The main agent's
  resolution (keep it) is `load_impl(.., mp: bool, mp_settings: Option<MpSettings>, difficulty: u32)`, with every
  wrapper passing both, `load_mp_carry` passing `Some(settings), 1`, and the async front-end load choosing
  `load_mp_carry` → dev → `load_with_carry_difficulty(selected_difficulty)`.
- **With that resolution on master 32647326, 10 failures:**
  - `difficulty_wiring_live::front_end_pick_drives_the_sim_load`: "00 Agent roster" left 20, right 37. The
    create-time gate you added (`if param(2) > d.difficulty { return None; }` in drone.rs) reads
    `Data::difficulty`, but `Scene::build`'s `level_drones` call builds its `Data` with a hard-coded
    `difficulty: 1` (scene.rs, about line 5795). `drones.set_difficulty(difficulty)` runs AFTER the creates,
    so the pick never reaches the gate.
  - `fix_npcs_live::an_idle_gate_guard_sees_and_attacks_bond_on_the_same_floor` (line 169)
  - `progress_remainder_live::a_guard_spotting_bond_counts_one_detection_per_guard`
  - `progress_remainder_failfirst::fail1_a_guard_spotting_bond_increments_detected`
  - `spawner_kill_live::tower2_stairwell_the_kill_channel_keeps_the_drawn_guard`
  - `fix_actors_live::the_spawner_arrives_on_its_channels`
  - `enemies_07000012_live::every_woken_and_spawned_guard_takes_damage`
  - `fix_searchlights_guards_live::a_guard_shot_mid_approach_holds_his_first_shot_for_the_impact_danim`
  - `walkreal_07000012_live::walk_spawn_to_the_shaft_exit` and `walkreal_probe13::probe_13_entrance_warm`
    (tests/common/tower2.rs:47)

  These mostly look like the guard roster or the gates shifting at the default difficulty. Decide from the game
  which difficulty the direct-load tests represent (the boot default is 2 [M 0x1c7e78:25]; your wrappers keep 1
  for replay parity). Each test must then either stay green unchanged or move with stated evidence.

Start from origin/master, merge `origin/s/difficulty-effects`, apply the resolution above, and fix the roster gate so
the pick reaches the creates. Get every one of the tests above green with the game's evidence (don't weaken them),
then run the FULL suite on the final tree before pushing.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/difficulty-effects2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T03:53Z

---
## Report (done, 2026-10-16T05:21Z)

