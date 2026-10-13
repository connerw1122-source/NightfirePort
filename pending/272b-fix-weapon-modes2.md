## Your task: `weapon-modes2`, land fix-weapon-modes on current master

Read `docs/reports/fix-weapon-modes.md` on `s/fix-weapon-modes` (4ed1519) first. Its findings are evidence-based and
wanted (the ○/M mode switch, the Scorpion's heat-seeker `Bullet_homing` 0x12f358, the op-1 sound table pinned, the
0x07000009 grants and the Q-Worm, the gadget id corrections). The main agent merged it on top of master (which had
gained fix-movies, fix-menus2, fix-sky and fix-restart-target since your base 6519d6f), and the suite went
**764 passed, 4 failed**, so it was backed out:

1. `fix_weapon_modes_live::p2k_mode_button_toggles_the_silenced_twin` (line 49): the switch reaches weapon 0x03 but
   `switch_state` is still **8** after 200 ticks ("and settles idle" expects 0). It passed on your branch.
2. `fix_weapon_modes_live::the_heat_seeker_homes_onto_a_guard_and_the_unguided_twin_does_not` (line 141): weapon 0x2e's
   guard-hit verdict is `false` (the homing round misses).
3. `portal_live::cold_loadout_arms_bond_on_every_story_level` (line 210): on 07000009 Bond draws weapon 67 after 30
   ticks, not the P2K. **The game agrees with your grants**: `Player_InitWeapon` 0x1a7ef8 case 0x7000009 equips the set
   and sets the switch request to `uVar6` (the dart) [M :192-202]. So that test's expectation for 07000009 is the
   one to update, with the citation. Check the other levels the same way.
4. (Also red in that run, but already fixed on master: `walk_0700000a_live`'s Restart expectation.)

Start from your branch, merge `origin/master`, reproduce 1 and 2, and find which of the newly merged changes interacts
with the switch cycle and the homing (candidates: fix-progress's runtime difficulty threading into `Data`, fix-menus2's
input/front-end handling in `lib.rs`, fix-restart-target's new `Drones` fields, fix-sky's tick additions). Fix it from
evidence. Don't weaken the tests. Run the full suite with `--no-fail-fast` and report the count.

Files: as fix-weapon-modes, plus `crates/nf-view/tests/portal_live.rs` (the 07000009 expectation only).

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/weapon-modes2` (start from `s/fix-weapon-modes`, then merge `origin/master` into it).
