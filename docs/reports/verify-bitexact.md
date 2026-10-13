# verify-bitexact — differential testing against the real game's own calls

Branch `s/verify-bitexact`, task 045n. Sandbox 09:09–11:08 UTC 2026-10-13.

## What's done

**The harness** (`crates/nf-game/tests/diff_calls.rs`, no NF_ISO needed): parses gdbcall format v1
(docs/tools/call-traces.md) from every `tests/calls/*.calls` file, maps each recorded function to a port adapter
(`fn(&CallRecord) -> PortResult`), and requires the same bits — `v0`/`v1`/`f0` where the adapter declares them,
and out-region bytes under a per-byte mask (so `Quat_QuatToMat`'s 9 written matrix floats are compared while the
caller-owned 7 are not). On a mismatch it prints the first differing call with inputs, game result, port result
and the ULP distance for floats. A function with no adapter is reported `not covered`, not failed.

**Results at the final commit** (level 07000005, 40–80 calls each):

| Recording | Adapter / port | Result |
|---|---|---|
| `PS2Sinf.calls` (0x120700) | `trig::sinf` → `anim::sinf` (the game's own polynomial) | **40/40 bit-exact** |
| `Quat_QuatToMat.calls` (0x1e3e80) | new EE-FPU port in the decompile's op order (`ps2math` ops) | **40/40 bit-exact** |
| `Rand_Rand.calls` (0x1e3738) | `drone::Rng::rand`, incl. SEED_X/SEED_Y after each call | **40/40 bit-exact** |
| `Check_Target.calls` (0x1a7810) | no adapter yet | not covered |
| `Player_AutoAim.calls` (0x1a7c98) | no adapter yet | not covered |

**Sisters** (each a test that would fail if the property broke):
- `the_host_sin_path_is_not_bit_exact`: the platform f32 sin differs from the game's PS2Sinf on **24/40**
  recorded calls — the old "close enough" sine is measurably not the game's.
- `the_vu0_guard_bit_sum_is_load_bearing`: the same polynomial with EE-FPU no-guard sums differs on **11/40**
  calls; the VU0 guard-bit accumulator model is what matches.
- `a_one_bit_change_in_a_recorded_output_fails`: flipping one bit of a recorded output (f0, a written matrix
  float, v0) fails the comparison; flipping a byte the function does not write does not.

**`crates/nf-game/src/ps2math.rs`** (new): the EE FPU (chop multiply, no-guard add/sub with the d ≥ 25
vanishing rule) and VU0 (guard-bit add, `madd`) operations with unit tests whose constants were verified
computationally (a genuine chop case `0x3F001907²`, the `1 − 2⁻²⁴` tie, the trim tie at `2^24 + 3.9999998`).

**`anim::sinf`** is now the canonical bit-exact `PS2Sinf` (0x120700: range reduction to [−π/2, π/2] on the FPU,
then the VU0 polynomial; `PS2Sinf2` 0x11f968 / `PS2Sinf3` 0x11fc20 are the same polynomial on 2/3 lanes).
`trig::sinf` delegates — identical outputs by construction, and trig's interpreter-validated tests still pass.

## How it was verified

- The harness itself, against the recordings (above), plus the sister tests.
- `nf-extract pose-check` on all three EE snapshots (`work/ee_start_05.bin`, `ee_start_07000001.bin`,
  `ee_start_0700000a.bin`), measured **before and after** the exact-math switch: local pose bit-exact (238/238
  on the posed 07000005 drone), world 0.0 everywhere except the documented `sAnimObject+0x58 = 0x40f`
  head-track/aim drone (runtime look-at state, 2.2e-1 — same as before; not a port error). The snapshots do not
  discriminate the palette models — both are world-0.0 — so the palette change was gated on the playtests
  instead (below). Note: pose-check's level argument takes the FILES.BIN names (`07000005.bin`).
- The full playtest suite exposed a real regression from the exact-math swap: with
  `placement::object_matrix` on the exact sine **or** the palette functions (`anim::quat_trans_to_mat` /
  `anim::mul`) on the EE models, `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`
  fails — the escort stalls short of the LoadPoint plate; the first visible divergence is guard 5 already dead
  at the round-0 clearing (a damage flip somewhere in the guard-vs-drone fight; both exact-math variants flip
  it, separately). With master's host math the same test passes (4.5 s, deterministic). Bisected to those two
  independent swaps; everything else in the sim path is byte-identical to master.
- Final state: escort walk **green** on the branch; diff_calls and all unit tests green; the full-suite re-run
  after the revert timed out with the sandbox (600 s tool cap; the earlier full run with the exact math was
  633 passed / 3 allowed `disc_golden` failures / 16 ignored / 1 escort failure, and the revert restores
  master's sim behaviour, verified by the escort test directly).

## What's left (owned by the next task, with the repro)

1. **Root-cause the escort damage flip, then swap the call sites** (GAPS G-017/G-024 carry the evidence):
   with `git checkout <this branch>` the flip reproduces by reverting the three host-math functions to the
   exact models (all written and snapshot-clean): `placement::object_matrix` (use `anim::sinf` + `ps2math`,
   the version is in commit e246658), `anim::quat_trans_to_mat`, `anim::mul` (VU0 products + guard-bit sums,
   also in e246658). The flip is a damage outcome in the 07000001 contact fight — prime suspects: bone/datum
   positions through `anim::mul`/`quat_trans_to_mat` (`drone.rs` 1957/2001/3986, `damage.rs` 39) shifting a
   hit/LOS decision by an ulp. A recording of the deciding comparison (e.g. `Check_Target` with a hit, or the
   damage roll's inputs) would pin it — see the request table.
2. **Adapters for `Check_Target` and `Player_AutoAim`**: both need the drone/weapon structs marshalled from
   the io regions into the port's types (`Drones::auto_aim`); the no-hit `Check_Target` path is the easier
   first case.
3. The remaining game-logic recordings (table below) once the harness grows adapters for them.

## Shared files touched (hooks)

- `crates/nf-game/src/lib.rs`: `pub mod ps2math;` (one line).
- `crates/nf-extract/src/main.rs`: one-line compile fix — `Player` gained `capsule` on master and nf-extract
  no longer built (standing capsule ends, same as scene.rs's construction); plus nothing else.
- `docs/WORKLOG.md`, `docs/GAPS.md`, `docs/tools/call-traces.md`, `docs/formats/animation.md`: appends/row
  updates only.

## Live checks for the main agent

None required for this task. For the follow-up: a `Check_Target` recording **with a hit** (the current 40 are
all no-hit: +0x132 = 0, +0x133 = 2) — same spec, played until the crosshair actually marks a drone.

## Recording requests

| Function | Address | Regions | Game situation | Calls |
|---|---|---|---|---|
| `PS2Sinf3` | 0x11fc20 | registers only (`f12`/`f14`/`f16` in, `f0` = lane 0 out) | any level at start (object creation calls it) | 200 |
| `RotMatrix` | 0x1e2368 | `in:a0:12 out:a1:64` | level load / object creation, varied placements | 200 |
| `Check_Target` (hit case) | 0x1a7810 | `io:@(a0+0xe0)+0x130:4` | aim mode with a drone actually under the crosshair | 80 |
| `Vec_Dist3D` | 0x1e2b28 [S: confirm addr in the symtab] | registers only | combat, guards engaging | 200 |
| `Lights_CalcClosestLights` | 0x1cb2e0 | spec TBD from the decompile (needs a read of its arg struct first) | 07000005 outdoors, Bond walking | 40 |
| `Collide_RayIntersect` | (address from the port map) | `in:a0:<ray struct> out:a1:<hit>` | shooting walls/floors at varied angles | 200 |
| `HUD_UpdateAmmoPane` | 0x19a7f0 | `in:a0:<pane struct>` | firing until the clip count changes | 80 |

## ntfy failed

(not reached — notification sent via curl before this section was written; if it had failed, the exact error
output would be here.)
