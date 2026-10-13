## Your task: `final-blockers2`, the 4a landing enable and the rest of the 16 walk

Read `docs/reports/fix-final-blockers.md` and its WORKLOG entry first (merged into master at 4f729f7).

1. **0700004a, the lift landing (first priority).** At the −52 landing the lift script starts cut-scene #289
   (0x060006cf), which disables Bond (stream 9 kind 0x04). When it ends (~t2163, played out, `skip_cutscenes =
   false`), `Script_Free`'s enable (flag 0x100, [M 0x1da880], `nis.rs` `free()`) places Bond at y −107.66, 36.8 below
   the platform's live pose (want ≈ −70.89), and he falls. The event-6 `player_at` (`nis.rs` `event()` case 6, the
   previous stream's entity matrix captured at event time) is the suspect. Dump the cut-scene's stream list over its
   whole run and decide from `Script_EventHandler` 0x1d94c0 case 6 and `Script_Free` 0x1da880 whether the game
   re-reads the entity's CURRENT matrix at free time, or whether the prev-stream resolution picks the wrong entity.
   Fix it, un-ignore `walkreal_0700004a_live`, then continue its walk out of the lobby to the level's next blocker
   or its end.
2. **07000016, door[0] onward.** The walk parks at door[0]'s approach (Bond wedged at (5.16, 1.75, −113.63) after
   the stuck handler's hop onto furniture). door[0] is locked by ch 0x24 [D]; find its mask-0x04 panel writer.
   Re-lane west of x 3.1 if the doorway has the same fixed half as door[7]. Then doors [1]/[3]/[2], the silo
   plate, the launch chain and the chaser, as far as you get. Each new blocker: either fix it (a port bug, from the
   decompile) or park the test at it with the evidence in its header.

Files: `crates/nf-game/src/nis.rs` (the enable matrix), `crates/nf-view/tests/walkreal_{0700004a,07000016}_live.rs`,
`crates/nf-view/tests/common/walker.rs` (lane/stuck handling only), probes as new files.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`, `door.rs`): small, additive hooks only, listed in
your report. Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. The previous session ran out of time before a full pass;
  start the full suite early (in the background) so it finishes before your session does.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/final-blockers2`.
