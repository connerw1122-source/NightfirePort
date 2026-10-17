## Your task: `glass-breach-guards-2`, finish the 08 glass-breach guards (HELD at merge)

`s/glass-breach-guards` (task 750p) was held at the 2026-10-17 batch: merged onto master (a union-resolved
conflict in `crates/nf-game/src/drone.rs`) it breaks eight tests that pass without it:
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`,
`fix_objects_live::a_death_draws_its_drop_and_nothing_else_moved`,
`render2_live::dropped_guns_dim_to_the_cel_ambient`,
`escort_follow_07000001_live::escort_at_walking_speed_keeps_the_games_distance_and_the_exit_opens`,
`fix_pickups_trace_live::with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace`,
`fix_guard_weapons_live::the_3x3_hit_rate_and_damage_matrix_matches_the_game`,
`fix_player3_live::death_drops_match_the_game_exactly`,
`npc_doors_live::the_escort_opens_his_door_without_bonds_help`.
Most are guard deaths/drops and the 01 escort — so the change reaches drones beyond the 08 glass guards.
Start from `origin/s/glass-breach-guards` merged with the newest origin/master (resolve drone.rs by hand),
scope the glass-breach behaviour to what the game does for those guards, and make all eight pass alongside
your own tests. Never loosen a measured test.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/glass-breach-guards-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-17T16:55Z
