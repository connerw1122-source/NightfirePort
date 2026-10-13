## Your task: `auto-switch`, turn on the pickup auto-switch without breaking the 01 escort walk

Read `docs/reports/fix-pickups-remainder.md` section 3 first. `Player_IsBetterWeapon` 0x1a6eb0 is ported and tested, but
the switch drain is GATED OFF (`if false` in scene.rs) because applying it wedges
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`: after the first Raptor take, the draw machine
swaps to the Raptor and the contact follow wedges at d=3.89, Bond standing 1.13 above the floor.
Find the interaction: the switch's view-model/anim side effects (a holster/draw anim moving Bond's capsule or blocking
movement?) or the walker reacting to the new weapon. Decide from the decompile what the game does while a switch draws
(`Player_Weapon` 0x1a24c8, the view states), make the port match, open the gate, and keep the walk green. If the walker is
at fault (not the game logic), fix the walker.

Files: `crates/nf-view/src/scene.rs`, `crates/nf-game/src/{player,pickup}.rs`, `crates/nf-view/tests/common/walker.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/auto-switch`.

---
claimed-by: sandbox1 at 2026-10-13T22:30Z
