## Your task: `frag-throw-timing2`, finish frag-throw-timing (HELD at merge)

`s/frag-throw-timing` (task 741) was held at the 2026-10-16 batch merge: on top of the batch it fails
`nf-view` `fix_grenade_types_live::the_stun_grenade_whites_bond_out_at_the_games_time_and_alpha` (alpha 0.673 vs the
formula's 0.758 / 0.379 on the observed geometry — tolerance 0.08) [M 0x12dfb0]. The test reconstructs the blast
point from the last grenade position it saw and Bond's look; with the new release timing the grenade is still
moving at the pop, so the reconstruction drifts. Do NOT loosen the tolerance. Either expose what the game used
(the blast position, the eye point and the aim yaw the flash took — e.g. the sim records them where it calls
`flash_amount`) and make the test assert against exactly those, or show the timing change is wrong. Start from
`origin/s/frag-throw-timing` merged with the newest origin/master; keep its own frag_throw_timing_live green.

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

**Branch:** `s/frag-throw-timing2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-17T03:10Z

---
## Report (done, 2026-10-17T03:34Z)

Closed by the driver: the report is on `s/frag-throw-timing2` (docs/reports/frag-throw-timing2.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-17T04:23Z)

# frag-throw-timing2 (750k) — finishing the held frag-throw-timing (741)

Branch `s/frag-throw-timing2` = `origin/s/frag-throw-timing` (7f41e160) re-applied on top of
`origin/master` (31dd6161). Task 741's merge was HELD at the 2026-10-16 batch: on top of the batch,
`nf-view` `fix_grenade_types_live::the_stun_grenade_whites_bond_out_at_the_games_time_and_alpha`
failed (alpha 0.673 vs the formula's 0.758 / 0.379 on the observed geometry, tolerance 0.08
[M 0x12dfb0]). This task resolves the hold without loosening the tolerance.

## What was wrong

The timing change itself is sound (the case-0xc release gate [M 0x1a2e38:505-519]; the full chain
in docs/reports/frag-throw-timing.md) — what failed was the TEST's geometry reconstruction. The
test rebuilt the blast point from the last grenade position it sampled and Bond's post-pop look.
Two drifts: the grenade record is deleted the same tick as the pop (the flash branch sets state 2,
the sim retains `state < 2` [M 0x12e118's tail → `Bullet_Delete`]), so the sample lags one tick;
and with the case-0xc release the pop catches the grenade still in flight (measured drift
0.615 u at the pop — see the run log below), where the old press+17 release happened to pop a
rested grenade. The reconstruction then missed the 0.08 tolerance — not the code's fault.

## The fix — expose what the flash actually used

`crates/nf-game/src/drone.rs`:

- `FlashProbe { blast, head, px, pz, yaw, los, f }` — the exact inputs of one stun pop, recorded
  where the sim calls `flash_amount` (the flash arm of `Bullet_DoTrails` 0x12d890's
  0x12dfb0-0x12e0b8 [M]): the blast position, the eye the dist/LOS legs ran from, the player xz
  the facing leg ran from, the facing yaw (the cel+0x54 stand-in [*suggested*]), the LOS branch,
  and the factor f the formula returned.
- `Drones::flash_probe: Option<FlashProbe>` — one pop deep, overwritten per pop; port-side
  instrumentation with NO game counterpart (the game computes and forgets). Own line in the
  constructor (merge lesson 4).

`crates/nf-view/tests/fix_grenade_types_live.rs` — both stun tests now assert against the
recorded geometry instead of a reconstruction:

- `the_stun_grenade_whites_bond_out_at_the_games_time_and_alpha`:
  - sister: the recorded blast is the grenade's own pop position — the last sampled flight
    position sits within 3.0 u of it (one tick of flight; measured 0.615);
  - `flash_alpha == flash_amount(probe)` exactly (1e-6 — the alpha IS that call's output
    [M 0x12dfb0 → 0x196040]);
  - `flash_total == 600·f` exactly [M 0x12e0b8; D input.md:126]; the remaining count within one
    countdown tick of 600·f (the pop step's own 1.0/tick decay [M 0x193600] runs before the read;
    the window is one-sided — no slop);
  - the plateau/fade shape assertions unchanged [M 0x193600:328-352].
- `the_stun_flash_scales_with_distance`: the retreat pin `d > 20` now reads the flash's own dist
  (probe.head → probe.blast — the geometry the formula consumed), plus the same blast-is-the-
  grenade sister and the exact `alpha == flash_amount(probe)`; the old `alpha < 0.7` distance
  scale pin stays.

The old both-LOS-branches OR (the `0.758 or 0.379` reconstruction) is gone — with the recorded
geometry there is exactly one answer, and the tolerance question dissolves.

## Verification

- The held failure reproduced first on the restored tree (alpha 0.6732224 vs 0.7584983/0.37924916
  — the task's numbers exactly), then the reworked test passes on the same tree.
- Run log (the pop): `probe FlashProbe { blast [102.664635, 23.79279, 109.656784], head
  [104.954, 25.677416, 108.82527], yaw -2.1445389, los: true, f: 0.6732224 }, last_seen
  [103.01836, 24.145588, 109.29816], drift 0.6149791` — the grenade was 0.615 u past its last
  sampled position when it popped (still in flight); the pop sat ~3.1 u from Bond's eye (the
  distance factor 1.0) and ~53° off his facing yaw (the facing factor 0.674) → f = 0.673, which
  the formula reproduces on the recorded geometry, exactly.
- Binaries run before pushing (all green): `fix_grenade_types_live` 3 passed + 1 ignored (the
  107a guard-stun test, pre-existing), `frag_throw_timing_live` 1, `fix_grenade_release_live` 4,
  `fix_grenade_throw_live` 4, `remote_mine_detonate` 3.
- Full suite on the final tree (all four crates, `--test-threads=1`, per-binary 600 s limit —
  the tools/test/suite.py shape run through `scripts/suite_chunked.py`, the same invocation
  chunked to fit the sandbox's 10-minute tool cap after it reaped every background job):

  **2008 passed, 6 failed, 73 ignored** (479 test binaries).

  The 6 failures:
  - 3 × `disc_golden` (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
    `directory_tree_matches_the_golden_manifest`) — the expected slim-ISO failures.
  - 3 × the pylon render trio (`probe_render_pylon_only`,
    `fix_0c_pylon_live::the_0c_pylon_fills_the_view_from_the_closeup_spot`,
    `fix_0c_pylon_live::the_0c_pylon_is_on_screen_at_the_user_spot`) — render-verification tests
    in this GPU-less sandbox; this task's change is sim-side data recording (a new `Option`
    field on `Drones` + the flash call site) plus two grenade test files, and the 0c pylon path
    touches none of it — no stun can pop in those tests. They are most plausibly the visual
    batch's GPU-dependent tests (the tree's merge-log commit is literally the GPU facial-morphs
    work) failing without a GPU; the main agent should re-run the trio on the merge tree with a
    renderer. The sandbox reset before their failure output could be archived — treat as
    environment-unverified, not pinned either way.

## What's left

- The 741 report's live checks (the release tick, the playhead, the state machine) still stand —
  they confirm the timing on hardware, not this fix.
- The 0x35 fuse-vs-flight observation above is port-side (the pop at 210 ticks caught the
  grenade 0.6 u into a bounce); no game assertion depends on the flight state.
- The ignored `the_stun_stuns_the_guard_for_3600_and_drops_his_weapon` (107a's open anim-set
  chase) is unchanged by this task.

## Live checks for the main agent

None new. (Optional, only if convenient: fresh-boot 07000005, throw the stun at Bond's feet,
gdbwatch `Player_SetFlashBang` 0x196040 entry — a1 should read ~600·f ticks and a2 the f·255
byte; the port's f for the same throw geometry is 0.673. This would pin the formula's output on
hardware, but the decompile chain is already complete.)

## Usage

not visible to me
