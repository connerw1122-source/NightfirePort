## Your task: `walkreal-b2`, land walkreal-b's work on master green

`s/walkreal-b` (005defa, report `docs/reports/walkreal-b.md`) walked 8 of 9 levels and found a real game-code fix (the
door lock edge: the two-stage `prev ← hold ← channels` latch, `door.rs` `Doors::end_tick` [M 0x1336f8]). The main
agent merged it into master and the full suite went **591 passed, 7 failed**, so it was backed out (not pushed):

- `fix_mission_live::the_07000001_exit_works_end_to_end` regressed (it passes on master). walkreal-b is built on
  walkreal-a (812a0fd), whose `drone.rs` carries the old **nearest-door-within-2.0 stand-in** for `NDrone2_OpenDoor`.
  That stand-in breaks Mayhew's walk. The real NPC door path is `s/fix-npc-doors-escort` → task `045g-fix-npc-doors2`.
- Its own walk tests fail on today's master: `walkreal_07000001_live`, `walkreal_0700000a_live`
  (`walk_office_floor_qworm_keyboard_hatch`, walker.rs:448), `walkreal_07000012_live` and `walkreal_07000013_live`
  (both at `common/tower2.rs:47`), and two more in the same run.
- It also carries `agent-ctx/fields/` (scratch copies of the field docs). Don't bring that.

**Do:** start a fresh branch from `origin/master`. Bring over the walker harness (`tests/common/walker.rs`,
`tests/common/tower2.rs`), the door lock-edge fix with its test, and the per-level walk tests, but **not** walkreal-a's
`drone.rs` door stand-in and not `agent-ctx/`. Then make every walk test pass on master, or, where a test is blocked
by a known open gap (the C5 door on 0d, 14's vent scene, the 12 doorway crossing), mark it `#[ignore = "<gap>"]` with
the gap named and the task that owns it. No test may stay red. If 045g-fix-npc-doors2 has landed by then, rebase on
it. Run the full suite (all four crates, `--no-fail-fast`) and report the count.

Files: `crates/nf-game/src/door.rs`, `crates/nf-view/tests/{common/*,walkreal_*}.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/walkreal-b2`.
