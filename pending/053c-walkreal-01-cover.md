## Your task: `walkreal-01-cover`, get the 07000001 contact walk green again

`crates/nf-view/tests/walkreal_07000001_live.rs::walk_from_the_spawn_to_the_exit_with_the_contact` was parked
(`#[ignore]`) at the fix-ai-traces2 merge (7e69b84). The full suite on the merged tree:

- The walk runs; the contact (Mayhew) goes 0x1d → PlayerInWay (the step-back recovery works) → **LeadHide 0x1f** at
  (−0.5, 6.3, 13.6), 32 units short of the LoadPointActivator plate, for 12 rounds.
- LeadHide holds while a hostile is alive. The last one is **guard 16**: crouched in cover (state 0x5c), health 10,
  at (−5.2, 6.3, 10.5), 3.2 from Bond. The walker's torso segment hits the WORLD first (`torso seg hit Some(0.82)`),
  so its shots never land. Rounds 2–11 cleared the other guards ([5, 6, 7, 11], [10, 14, 15]).
- Cause of the change: today's merges added draws to the shared RNG stream (fix-impacts' casings/debris, the
  PlayScript scripts), so the 01 fight plays out differently. Same mechanism as the ai_live seed re-picks.

Make the walker handle a guard in cover the way a player does, not by seeding the RNG to dodge the fight. Re-aim at
any exposed part (the part list it already logs: head 5, arms 53–56), then step sideways or forward until a part's
segment is clear, then fire. Keep it in `tests/common/walker.rs` so every walk benefits. Un-ignore the test and run
it on `origin/master`. Also run the other walkreal tests that use the walker to show nothing regressed.

If the walk still can't finish, find out whether guard 16's cover is the game's behaviour (cover node choice,
`UnderCover*` states) or a port bug, with evidence. Report which.

Files: `crates/nf-view/tests/common/walker.rs`, `crates/nf-view/tests/walkreal_07000001_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/walkreal-01-cover`.
