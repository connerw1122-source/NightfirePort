## Your task: `anim-blend2`, the lean-out at speed 1.5 now lasts 90 ticks instead of the game's 37

Read `docs/reports/fix-anim-blend.md` on `s/fix-anim-blend` (ddc2259). The exact `Quat_Slerp_Acc` and the distance-table
machinery are wanted. The main agent merged it onto master (0cda458) and the full suite went **790 passed, 1 failed**:

- `ai_live::guard_takes_cover_like_the_game` (ai_live.rs:249): **"UnderCoverAim lasted 90 (live 37)"**. The 37 is the
  real game's measured duration (`work/chase_trace.tsv`, the lean-out anim 176 played at `Drone_AnimInfo +0x14` = 1.5,
  your report's own example). A seed scan (`NF_TEST_SEED` 0..15) gives **exactly 90 on 11 of 16 seeds**; the few that
  pass take a different cover type. So it isn't an RNG shift: a 1.5×-speed anim's end/duration now comes out wrong.
  Likely suspects: the fractional `ftime` / wrap handling you changed in `Drone::script_frame` and the root sampler, the
  "anim done" test (whole-frame `time` vs the float clock +0x90), or the `AnimSeqTick` 0x1c28d0 seq.frame = time −
  start + 1.0 relation applied twice.

Reproduce with the single test, find where the lean-out's end detection diverges from `AnimScriptTick` 0x1c2630 /
`AnimSeqTick`, fix it, and keep your new unit tests green. Do NOT re-pick the seed: the live 37 is the check. Then run
the full four-crate suite (in the background, first) and report the count.

Files: `crates/nf-game/src/{anim,drone}.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/anim-blend2` (start from `s/fix-anim-blend`, then merge `origin/master` into it).

---
claimed-by: sandbox6 at 2026-10-13T20:20Z

---
## Report (done, 2026-10-13T21:34Z)

