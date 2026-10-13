## Your task: `fix-npc-doors2`, finish the NPC door work so it merges green

`s/fix-npc-doors-escort` (754c7d9, report `docs/reports/fix-npc-doors-escort.md`) ported the game's NPC door path:
`DSTATE_OpenDoor` 0x1681f8 (state 0x81), the route door-node binding, and `ControlSTANDARD`'s +0xBA8 check. Plus the
07000002 SS grates. The main agent merged it into master (20fe32b) and the full suite went **583 passed, 2 failed**,
so the merge was held (not pushed). **Start from `s/fix-npc-doors-escort`, merge `origin/master`, and make both pass
against the game:**

1. **`fix_mission_live::the_07000001_exit_works_end_to_end` regressed** (it passes on master). Mayhew enters state
   0x81 at shoji door #1 (7.92, 5.34, 11.14) by tick 800 and never leaves it: 4,800 ticks oscillating between z 10.1
   and 12.1 (the test nudges him 2 units toward his target), log:
   `[mission] t 800 … state 0x81 … t 4800: Mayhew [7.08, 6.35, 12.09] state 0x81`. The test also places Bond 1.5 units
   behind Mayhew, inside the door's swing; a door whose sweep Bond blocks pauses (the "pause when blocked" seen by
   fix-lighting2 on this same test). So the door never reaches state 4 and `DSTATE_OpenDoor` never returns. Find the
   game's answer: what `Door_Update` does while blocked (pause, reverse, re-open), whether `DSTATE_OpenDoor`/
   `ControlSTANDARD` gives up (the 2.0 forget distance, timers), and what a walk-through drone does then. Fix the port
   to match; change the test only if the game itself would stall there, and say why.
2. **`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` fails** ("the contact must walk his
   route to the LoadPointActivator plate"; he settles at (−4.5, 6.3, 14.2), plate at (−33.1, 2.3, 15.4)). This is the
   `AllyLeadHide` (0x1f) stall your report lists as open: Mayhew hides while armed guards stand within 14
   [M 0x15ec08]. Either make him reach the plate the way the game does, or, if the game really hides there until the
   guards die, make the test clear those guards first and assert that.
3. **Remove `agent-ctx/fields/` from the branch** (scratch copies of the field docs from walkreal-a's 812a0fd; the real
   ones are `docs/fields/`). `agent-ctx/B-3a-b.md` is an old master file: leave it.

Run the **full** suite (all four crates, `--no-fail-fast`) on the merged tree before you push, and report the count.

Files: `crates/nf-game/src/{drone,door,ally}.rs`, `crates/nf-view/tests/{fix_mission_live,walkreal_07000001_live,fix_npc_doors_live}.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-npc-doors2`.
