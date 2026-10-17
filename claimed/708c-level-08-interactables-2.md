## Your task: `level-08-interactables-2`, finish the 08 interactables (HELD at merge)

`s/level-08-interactables` (708a, 857eb60) was held: its `crates/nf-game/src/hurt.rs` — the fuller port
(the `Hurt_Update` 0x17e5d0 state machine, gate value as the who-mask, param-1 damage, drone victims, on
`Drones::hurts`) — collides with the `hurt.rs` already on origin/master (on `Sim.hurts`, scene.rs, with the
who-mask start law). Folded in as the one system (master's `Sim.hurts` retired, hurt_02_live moved to
`Drones::hurts`) it breaks four tests that pass on master:
`fix_finale_live::the_07000016_chasers_defeat_completes_the_final_mission`,
`spurious_complete_16_live::the_real_end_condition_posts_the_mission_complete_banner` (16's #309
"HurtPoly-ShuttleLaunch", gate 0xf3, 1000 damage, kills Bond on the silo floor during the fight),
`hurt_02_live::the_electrocute_volume_hurts_once_the_cable_light_smash_feeds_the_chain` (line 83) and
`interact_07000006_live::the_well_shaft_is_open_and_the_hurt_volume_is_game_inert` (06's #373, who 1 — the
spec's own inert law). Start from origin/master's hurt.rs and `docs/systems/hurt-volumes.md`; build ONE
system (no second module) that passes all four plus your 08 tests (hurt_07000008_live). Coordinate with 705d
(the same ShuttleLaunch question): settle when ch 0xf3 is up relative to Mission state 6 [M 0x182c60].

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

**Branch:** `s/level-08-interactables-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T17:48Z
