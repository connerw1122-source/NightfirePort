# difficulty-effects2 — report

Branch `s/difficulty-effects2` (from `origin/master` 4108d7b2). Task: land HELD 629
(`s/difficulty-effects`, tip 1d56131d) — its branch tip doesn't compile (E0308), its final merge
was never built, and on master the front-end pick never reaches the create gate (00 Agent roster
20 vs 37) with 9 guard tests failing.

## What was done

1. **Re-applied 629** by reverting master's revert `dc841e30` (master had merged 629 as
   `1e96af8a` and then held it by reverting). The re-application (`5641c552`) restores the main
   agent's own 053 net-match-host resolution verbatim:
   `load_impl(.., mp: bool, mp_settings: Option<MpSettings>, difficulty: u32)`, every wrapper
   passing both, `load_mp_carry` passing `Some(settings), 1`, and the async front-end load
   choosing `load_mp_carry` → dev → `load_with_carry_difficulty(selected_difficulty)`.
2. **The E0308 is already resolved in that content** — `menu.rs:3866` reads
   `self.requested_difficulty = Some(self.difficulty as u32 + 1)` (the wheel pick is 1-based
   [M C_SBNFDFCTY 0x216ca8:15], the page's cursor is 0-based), and `nf-view` builds clean.
3. **The pick now reaches the creates** (`crates/nf-view/src/scene.rs`, `Scene::build`): the
   create-time `drone::Data` carried a hard-coded `difficulty: 1`, so `Drone_Create`'s
   min-difficulty gate read 1 on every load and `drones.set_difficulty(difficulty)` only ran
   after the creates. The game holds the pick in `GameState+0x28` (0x2a3790) BEFORE the load's
   creates run [M C_SBNFDFCTY 0x216ca8:15 writes it at confirm; `Drone_Create` 0x136be0:49 reads
   it], so the `Data` now carries the load's difficulty. `difficulty_wiring_live` passes end to
   end again (00 Agent roster 37, Operative 20 — the 629 numbers).
4. **The direct-load difficulty stays 1** — the recorded-session replay parity the pre-629 code
   documents ("The direct-level load replays the user's live sessions: they ran at difficulty 1",
   scene.rs before 629; the boot default is 2 [M 0x1c7e78:25] and the front-end boot keeps it
   until the pick). With the gate active, difficulty 1 is now the game's real Operative behavior
   instead of a chimera that created every placement.

## The 10 failures, reconciled (each with the game's evidence)

The gate [M `Drone_Create` 0x136be0:49: a 0x0F placement spawns iff `level_tag+0x34` (param 2)
`<= GameState+0x28`] makes the pre-gate calibration roster (which the failing tests index) the
**00-Agent** roster: every param2=2/3 placement is created only at difficulty 3. That is the
evidence used throughout — the same move 629 itself made for `enemies_07000012`
("the walkthrough now loads at 00 Agent where the full roster exists").

| Failure | Root cause | Resolution |
|---|---|---|
| `difficulty_wiring_live::front_end_pick_drives_the_sim_load` (00 Agent roster 20 vs 37) | the hard-coded `difficulty: 1` in the create-time `Data` | fixed in `Scene::build` (above); green unchanged |
| `fix_actors_live::the_spawner_arrives_on_its_channels` (07000011) | the group-1 stairwell templates are param2≥2 placements — an Operative load never creates them, so `tpls` is empty | loads at 00 Agent via `load_with_carry_difficulty(.., 3)` |
| `spawner_kill_live::tower2_stairwell_the_kill_channel_keeps_the_drawn_guard` (07000011) | same templates | same move |
| `walkreal_07000012_live::walk_spawn_to_the_shaft_exit` (tower2.rs) | NOT the roster: the Q-Decryptor keypad burn never fires because 07000012's keypad locks (placements 656/694) carry **param 6 = 1** — `Lock_Create` auto-unlocks them at difficulty 1 [M 0x1ae1f8 tail: `level_tag+0x44 != 0 && GameState+0x28 == 1` → `Lock_Unlock` 0x1b0650]. At Operative the game opens the keypad door without the burn; the route's burn step is the Agent+ behavior | loads at 00 Agent |
| `walkreal_probe13::probe_13_entrance_warm` | same lock auto-unlock on the shared `walk_12` leg | all three legs (12/4a/13) load at 00 Agent |
| `fix_searchlights_guards_live::a_guard_shot_mid_approach_holds_his_first_shot_for_the_impact_danim` (07000006) | the guard (placement 445) exists at d1, but the staged danim's remaining script length at the stage tick is draw-stream dependent: the gate skips six param2=2 creates (442/447/635/648/680/704), each of which drew in `Drone_Create` [M 0x136be0], shifting the stream; the staged hit-reaction ended at +29 and the anim pipeline fired before timer C. The deferral itself is game-correct [M 0x166328:48-52; AimCrouchFire 0x167060 really fires immediately — the crouch path has no deferral] | loads at 00 Agent, where no create is gated and the draw stream matches the pre-gate calibration bit-for-bit |
| `fix_npcs_live::an_idle_gate_guard_sees_and_attacks_bond_on_the_same_floor` (07000006) | index shift: the idle gate guard (placement 487, the pre-gate `list[15]`) sits at `list[13]` once the six param2=2 creates are held back | stays at the direct load's difficulty 1; index updated 15→13 with the placement noted |
| `progress_remainder_live::a_guard_spotting_bond_counts_one_detection_per_guard` | same guard, same shift | same index update (placement 487) |
| `progress_remainder_failfirst::fail1_a_guard_spotting_bond_increments_detected` | same | same |
| `enemies_07000012_live::every_woken_and_spawned_guard_takes_damage` | 629's d3 load was starved by the same hard-coded `Data` 1 | fixed by (3); green unchanged |

No assertion was weakened: every moved test keeps its body; only the load difficulty (with the
param2 evidence) or the list index (with the placement evidence) changed.

## What was verified

- The 10 previously-failing tests, then their full binaries (29 tests): all green —
  fix_npcs_live 4, progress_remainder_live 4, progress_remainder_failfirst 3, spawner_kill_live 5
  (+1 ignored), fix_actors_live 4, enemies_07000012_live 3, fix_searchlights_guards_live 2,
  walkreal_07000012_live 1, walkreal_probe13 1, difficulty_wiring_live 2.
- 629's own tests: `nf-game difficulty_gates` 3/3, `difficulty_spawn_gate_live` 1/1.
- The full suite on the final tree (merged with `origin/master` 6bab6409): **this sandbox reaps
  background jobs between tool calls** — both `nohup` and `setsid`-detached `tools/test/suite.py`
  runs died during their build phase (the same limitation the 629 and drive-mission-scripts
  reports hit), so the suite ran as prioritized **foreground chunks** with the runner's exact
  per-binary env (`NF_SILENT=1`, `--test-threads=1`, 100–300 s caps):
  **76 binaries, 722 passed, 0 failed, 1 ignored** — nf_game 234, nf_formats 79, nf_audio 9,
  nf_view's mp/net/menu/flow/load/level test binaries, every difficulty/guard/progress/spawner/
  searchlight binary around the changed code paths, the master's new movie_pacing_live /
  sfx_rejected_start_live / mp_net_lobby, and a broad sample of weapon/replay/scenario/music.
  The `nf_view` unit binary (17 tests) cannot run here — it opens a winit window in `main` and
  the sandbox has no display stack (panics under Xvfb too: xkbcommon-dl missing x11 libs) — a
  pre-existing limitation, unchanged by this task; its menu/flow coverage ran green via the
  headless binaries above. The ~280 binaries not reached are dominated by the per-level
  `walk_*`/`walkreal_*` walkthroughs this task does not touch.

## What's left

- Nothing known. The two pre-existing open items from 629 stand: the difficulty→DRIVING.ELF path
  (PTPDATA+0x984) still needs a DRIVING decomp pass, and `Player_InitWeapon`'s write-only
  `MAX_AUTO_AIM_DIST_*` globals stay unthreaded.
- The suite's `disc_golden` trio remains the documented slim-ISO allowance.

## Live checks for the main agent

1. Roster spot-check with the gate live: on 07000011 at each difficulty (poke `[0x2a3790]` = 1/2/3
   via fieldprobe before a restart), the stairwell reinforcements should appear only at Agent/00
   Agent (roster 20/33/37 created, the group-1 spawner wave included from Agent).
2. Keypad auto-unlock on Operative: on 07000012 at `[0x2a3790] = 1`, the door[19] keypad door
   (locks at placements 656/694) should open without the Q-Decryptor burn — `Lock_Unlock`'s
   state +0xf4=1 [M 0x1b0650:28-31] — worth a `--bond` look at the door.
