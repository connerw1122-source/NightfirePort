# verify-ledger — the claim ledger over the bit-exact call-verification machinery

Branch `s/verify-ledger`, task 600. Sandbox 17:03–~19:00 UTC 2026-10-15.

## What's done

**The ledger** (`docs/verify/ledger.md`, new): one row per ported game function the sim depends on, with the
strongest claim the evidence supports. Structure:

- **Status grammar** (top of the file, shared with the guard test): `bit-exact on N recorded calls` /
  `within tolerance (eps)` / `unverified` / `unverified (prose)` / `known divergent (reason)`. Anything else
  fails the guard test.
- **Verified now** (3 rows): `PS2Sinf` 0x120700, `Quat_QuatToMat` 0x1e3e80, `Rand_Rand` 0x1e3738 — each
  `bit-exact on 40 recorded calls`, carried over from 045n (the machinery itself: `tools/trace/gdbcall.py`,
  format v1 in `docs/tools/call-traces.md`, `tests/calls/*`, the replay harness `crates/nf-game/tests/diff_calls.rs`).
- **Hot paths** (17 rows): the functions the task listed plus the rest of the Rand family and the two
  already-recorded player functions — `Vision_InView` 0x1e89e0, `Quat_Slerp_Acc` 0x1e4188, `Mat_Align2Dir`
  0x1df2a8, `Collide_SphereIntersect` 0x1ec940, `Intersect_RayGeom` 0x10a8c8, the `Rand_*` family
  (`FRand` 0x1e3780, `FRandHalf` 0x1e3810, `FRand_MVar2` 0x1e38b0, `Random` 0x1e36f0, `FRandHalf_Vec` 0x1dfef0,
  `FRand_MVar2_Vec` 0x1e00d0), `NDrone2_HitDamage` 0x145558, `DroneWeap_DoBulletAccuracy` 0x179740,
  `Explode_Propagate` 0x17c518, `AnimFrameSet` 0x1c75a0, `Check_Target` 0x1a7810, `Player_AutoAim` 0x1a7c98 —
  each with its port fn and an explicit **Adapter** column (yes / partial / no + why):
  a `diff_calls.rs` adapter can only replay what registers + captured memory determine; scene-coupled ports
  (Check_Target, Player_AutoAim, Explode_Propagate's targets, AnimFrameSet's pointer-chased channel blob) cannot,
  and the ledger says so instead of promising a bit-exact row that can never be enforced.
- **Recording requests**: the exact `gdbcall.py --spec` command, the level and the game situation for every
  missing recording, grouped into four sessions (math at the 07000005 start; the Rand family, one session; the
  collision pair, one shot into a wall; the combat four, aggroed guards). The specs came from reading each
  decompile (plus the raw disassembly for `Collide_SphereIntersect`'s eabi-style argument registers: radius in
  `f12`, 5th–8th pointer args in `t0..t3`, the 10th at entry-`sp+0x00` — the Ghidra mangling lies about the
  order, the same demangler bug `docs/fields/hitdata-spawner-ally.md` already documents).
- **Appendix** (454 rows): every other ported function from the port maps (`guard-ai`, `core-loop`, `levels`,
  `weapon-switching`, `audio`; the trigger-opcode tables and the per-task tracking tables are excluded), each
  seeded `unverified` with its port-map port fn and a subsystem tag. One row per (address, name).

**The guard test** (`crates/nf-game/tests/diff_calls.rs`): `ledger_claims_match_the_recordings` parses every
`| 0x…` row of the ledger and fails unless

- every row's status is in the grammar (typos fail loudly, they don't pass as prose);
- a `bit-exact on N recorded calls` row has the recording file under `tests/calls/`, its `function=` header
  equal to the row's name, its entry address equal to the row's address, **at least N** parsed calls, and an
  adapter registered for that name in `adapters()` — so a recording that shrinks, a deleted adapter, a renamed
  function or a forged row all fail;
- no duplicate (address, name) rows; the three verified claims are still present (`claims >= 3`).

Sister test `a_bit_exact_claim_without_a_recording_fails_the_ledger_guard`: a forged `Quat_Slerp_Acc` row
(a function with neither recording nor adapter) is parsed with the real parser and shown to fail the guard, and
the common typos (`bit-exact on 40 calls`, `verified`, …) are shown to fail the grammar. If a later task records
`Quat_Slerp_Acc` and registers an adapter, the sister re-aims itself at the "no full backing" property (it
documents the exit condition in its assert message).

**`docs/tools/call-traces.md`**: the harness-status section now points at the ledger and the guard.

## How it was verified

- `cargo test --offline --release -p nf-game --test diff_calls` on the final tree: **6 passed / 0 failed**
  (the 3 pre-existing replay/sister tests still green — no regression in the harness — plus the two new guard
  tests).
- The ledger row count is asserted (`>= 400` rows) so the appendix can't silently vanish.
- Full suite on the final tree (merged with origin/master ee1619d1): see the total line below (started in the
  background right after the merge; the three slim-ISO `disc_golden` failures are the allowed ones).

## Live checks for the main agent

The recording session is the follow-up work; every command is in `docs/verify/ledger.md` § "Recording requests"
(they need PCSX2, which the sandbox doesn't have). Beyond running them, three first-trace sanity checks:

1. `Collide_SphereIntersect` (0x1ec940): the `in:sp:4` region must read **4** on every call (both traced callers
   store 4 there — if it doesn't, the eabi stack-arg offset is wrong and the spec needs fixing before the
   recording is trusted).
2. `Rand_FRandHalf_Vec` (0x1dfef0) / `Rand_FRand_MVar2_Vec` (0x1e00d0): the `a0` region bytes must CHANGE per
   call (the Vec variants write/accumulate 12 bytes; if they don't change, the mangled-name argument order
   conclusion is wrong and the spec needs `in`/`out` swapped).
3. `Check_Target` (0x1a7810) hit-case: re-record with a drone actually under the crosshair — the committed 40
   calls are all no-hit (+0x132 = 0, +0x133 = 2); confirm some calls have +0x132 != 0.

Expected first results when the replays are wired (from the decompile analysis, to be confirmed by the
recordings): `Quat_Slerp_Acc`/`Mat_Align2Dir`/`Vision_InView` and the four `Rand_*` scalars should replay
bit-exact immediately (`anim::slerp` composes the already-proven `trig::sinf`; `Rng::frand` is proven);
`NDrone2_HitDamage` will likely expose the port's `armour: 0, mode: 0` stand-in at `drone.rs:2142` (the
recording measures the real `+0xBB`/`+0xC5`); `DroneWeap_DoBulletAccuracy` will surface the four known deltas
listed in the ledger (missing first-stopped branch, yaw-only wobble, hardcoded 120 gate, per-level constants) —
those are the port-map's "partial" gaps, made precise by the replay.

## What's left (owned by the next task / the main agent)

1. **Record** the requested functions (the four sessions in the ledger) and commit the `.calls` files.
2. **Wire the replays**: adapters for `Quat_Slerp_Acc`, `Mat_Align2Dir`, `Vision_InView`, the Rand family and
   `NDrone2_HitDamage` (each marked "yes" in the ledger's Adapter column); flip their rows to
   `bit-exact on N recorded calls` — the guard enforces the claims from then on.
3. `Collide_SphereIntersect` / `Intersect_RayGeom` partial adapters (bool + centre push; dist/normal/point/
   surface with the packed-normal re-encode; `v0/v1` vertex indices are don't-care).
4. The `within tolerance` rows nobody has measured yet (e.g. the `+0x58` head-track drone in the pose checks) —
   the grammar has room for them once someone measures.

## Files touched

- `docs/verify/ledger.md` (new, 641→474 rows after the one-row-per-function dedup).
- `crates/nf-game/tests/diff_calls.rs` (the guard + sister tests appended; no adapter or harness change).
- `docs/tools/call-traces.md` (harness-status section: the ledger pointer).
- `docs/WORKLOG.md` (the task entry).

No gameplay code changes (the task's rule); no shared-file edits beyond the WORKLOG append.

## Full-suite result on the final tree

(to be filled from suite.out before the finish steps)

## ntfy failed

(not reached yet)
