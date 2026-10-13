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

# fix-anim-blend2 — the cover lean-out at speed 1.5 lasted 90 ticks instead of the game's 37

Branch `s/fix-anim-blend2` (the task header names it twice; the tail's `s/anim-blend2` does not
exist on the remote — `s/fix-anim-blend2` is used everywhere). Cut from `origin/s/fix-anim-blend`
(a14976d) with `origin/master` (0cda458) merged in; the WORKLOG conflict kept both sides.

## What was actually wrong

The failing test was `ai_live::guard_takes_cover_like_the_game` ("UnderCoverAim lasted 90 (live
37)"). An env-gated per-tick anim trace showed the port's UnderCoverAim playing **danim 178** (90
frames, `Drone_AnimInfo +0x14` = 1.0 → exactly 90 ticks) where the game plays **lean-out 176** (51
frames at 1.5 ≈ live 37) [L work/chase_trace.tsv]. Danim 178 is the transition that `CCModes`
(0x270768) resolves for the *step-out* cover type (state 0x1c); 176 is the *stand lean* (state
0x18). So the cover-type `Rand(2)` at `UnderCoverInit` drew 1 (step) where the game drew 0 (lean)
— an RNG-stream misalignment, not a fractional-time end-detection bug. The task's three suspects
(ftime/wrap handling, the "anim done" test, the AnimSeqTick relation applied twice) were all
checked: the done test (`time >= len`) is unchanged from the pre-blend port and fires correctly at
51/1.5.

Two earlier claims fell during the investigation, both worth recording:

- fix-anim-blend's report said this failure was "pre-existing on master (verified by stash)". A
  master-side bisect disproves it: the guard test is green at a8884f3, 37fdc7f, 18fd3a7, 2dbd10f
  and 13abef4, and red on a14976d — the regression came in with fix-anim-blend's code commit
  5c7b457 itself. (The stash check presumably failed to revert cleanly.)
- The task text says the blend work was "merged onto master (0cda458)": at this session's fetch,
  `origin/master` 0cda458 does not contain 5c7b457 at all (`git merge-base --is-ancestor` fails;
  its anim.rs has no `root_translation_at`). The regression lived on the `s/fix-anim-blend`
  branch, which is what this task inherited.

## The mechanism [M]

`Drone::update_anim`'s end branch samples the wrap step from the script's last two frames. The
blend session added `self.anim.ftime = self.anim.time as f32` around that sampling so the new
fractional `root_value` would see the sampled time. But `AnimSeqTick` (0x1c28d0) only **reads**
the script's float clock (+0x90: `seq.frame = time − start + 1.0`) — nothing in the game writes
it. In the port, the clobber landed *before* `set_danim` → `begin_blend`, which starts the
outgoing fade at `1.0 + self.anim.ftime` (the replaced script's clock): every blended script
switch started the fade one frame early, and its first step (`at(t) − at(t − speed)`) replayed the
replaced script's last step on the switch tick. The pre-blend port moved exactly 0 there (its
integer sampler clamps to the sequence's last frame).

Proof [L]: a per-tick position dump of the cover scenario on both trees. The gate guard's walk
328→392 switch at tick 74 — positions identical through tick 73, then the broken tree steps
+0.0095x/−0.0063z on the switch tick while the base tree holds. ~0.01 per switch across a patrol
shifts another guard's burst-fire tick; the RNG stream at the cover pick diverges; the pick flips
LEAN→STEP; UnderCoverAim plays 178 and lasts 90.

## The fix

`crates/nf-game/src/drone.rs` — the wrap-end sample goes through a new `root_value_at(d, time)`
that computes the sequence window and frame (`1.0 + time − start + 1.0`, then
`anim::root_translation_at`, the AnimFrameSet 0x1c75a0 f/f+1 blend) for an *explicit* script time
without touching the live clock. The fractional root-motion feature (the point of fix-anim-blend)
is untouched: at frac = 0 the blend gate returns the exact integer sample, so speed-1.0 behaviour
is bit-identical to the pre-blend port.

## Verification (failing first)

- `ai_live::blended_switch_tick_applies_no_root_step` (new, NF_ISO): runs the cover scenario's
  first 76 ticks, finds the walk's hand-over to 392, asserts the switch tick applies **no** root
  step (< 1e-4). RED before the fix — "switch tick moved 0.01141" — GREEN with it. Sister
  assertion: the walk takes its ~0.011 step on the following tick, so a stalled sim cannot pass.
- `ai_live::guard_takes_cover_like_the_game` is green again (the state sequence, UnderCoverInit
  28–34, UnderCoverAim 32–40 vs live 37, corner node 1, the fire position, and the
  cover-depends-on-situation sister all hold).
- fix-anim-blend's own unit tests stay green (`anim::tests`: slerp, distance table) — anim.rs is
  not touched by this fix.
- Full four-crate suite (the driver's command, run in foreground chunks on the slim ISO):
  **nf-game 231 passed / 0 failed**; **nf-formats + nf-audio 89 / 4** — the 3 permitted
  `disc_golden` failures plus `pss_movies_live` (the slim ISO carries no PSS movies; the skip
  guard exists only on the unmerged fix-swim-vent4, both pre-existing per fix-anim-blend's
  report); **nf-view 396 / 0** — the walkreal net and the cut-scene pose tests included. Total
  ≈ 716 passed, 4 failed, all four permitted/pre-existing. Chunk caveat: the nf-view chunk hits
  my 10-minute foreground timeout on its very last probe binary (0 failures anywhere in its log);
  everything up to it completed green.

## What's left

- The driver may want the same fix surfaced on master once 0cda458 actually receives 5c7b457 —
  the merge will be conflict-free from this branch (drone.rs is the only shared file touched, in
  the region the blend work owns).
- The loop-wrap's `ftime = ftime_in − len` (max 0) loses the fraction on every wrap at fractional
  speeds (it is ≤ 0 at the first wrap by construction). Speed-1.0 behaviour and the live-pinned
  walks are unaffected; a fractional-speed looping anim (e.g. the player walk sets once
  AnimSetUpdate 0x1c0938 lands) should re-examine it against a live trace.

## Live checks for the main agent

- PCSX2, 07000005, the gate-guard cover scenario (chase_trace): confirm UnderCoverAim shows the
  *lean* lean-out (no step-out with a turn) and fires from (58.83, 99.32) — the port now matches
  the trace; a render spot at (58.9, 99.3) facing the corner should show the guard leaning around
  node 1 with his weapon up.
- nf-view `--bond` spot on 07000005 at the gate guard's patrol (61.34, 27.06, 99.49): walking
  should show no hitch on walk-cycle hand-overs (the extra switch-tick step is gone).
