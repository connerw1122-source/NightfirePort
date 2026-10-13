## Your task: `verify-bitexact`, differential testing against the real game's own calls

Today the port is checked against our *reading* of the decompile. This task checks it against **the running game**:
record real calls of a game function in PCSX2 (its inputs and outputs), replay each one through the port's version,
and require the same result **bit for bit**. (Idea from morluto/rea's reconstruction showcases, which pass thousands
of cases recorded from the original binary; facts only, no code taken.)

**Who does what.** The sandbox has no PCSX2. The main agent records the calls (`tools/trace/gdbcall.py`) and commits
them to `master` under `tests/calls/`. You build the harness and the fixes, and say exactly what you need recorded.

1. **The recorder and the format already exist** (main agent, master 411b86c): `tools/trace/gdbcall.py` and
   `docs/tools/call-traces.md` (format v1). The first recordings are in `tests/calls/`: `Quat_QuatToMat.calls`,
   `Rand_Rand.calls` (with SEED_X/SEED_Y 0x30d0a0/0x30d0a4 before and after) and `PS2Sinf.calls` (40 calls each,
   07000005). **`PS2Sinf` is bit-identical to the f32 sine on only 13 of 40 calls** (up to 3.84e-5 off): the
   first target. Parse format v1 as documented; if you need a field it doesn't carry, say so in your report and
   the main agent extends the recorder.
2. **The harness** (`crates/nf-game/tests/diff_calls.rs`, NF_ISO not needed): it reads every `.calls` file, maps each
   function to a port adapter (`fn(&CallRecord) -> CallResult`), runs it and compares the results bit for bit. On a
   mismatch it prints the first differing call (inputs, game result, port result, the ULP distance for floats). A
   function with no adapter yet is reported as "not covered", not as a failure. A sister test proves a one-bit change
   in a recorded output fails.
3. **The first adapters and their fixes**, in this order (the previous `250-fix-bitexact` task folds in here):
   - `PS2Sinf3` / `PS2Sinf` / `PS2Cosf`: port the game's approximation exactly (coefficients, range reduction, the
     EE FPU's behaviour: no denormals, round toward zero, clamp on overflow) and use it everywhere the game does
     (`View_RotTransMatrix`, `control_create_object` 0x1344e0 and every other caller). GAPS G-017.
   - `Quat_QuatToMat` and `psiBuildMatrixPalette`'s VU0 composition: the VU0 operation order and rounding
     (truncation, no denormals). GAPS G-024 (world matrices up to 4.6e-5 off).
   - `Rand_Rand` / `Rand_FRand`: the shared RNG, bit for bit, including the state after each call.
   - A PS2 float helper (`crates/nf-game/src/ps2math.rs`: EE FPU and VU0 multiply, add, multiply-add, with unit tests).
   - Then the game logic the user sees: `Lights_CalcClosestLights` 0x1cb2e0, `Check_Target` 0x1a7810,
     `Player_AutoAim` 0x1a7c98, the collision ray (`Collide_RayIntersect`), `HUD_UpdateAmmoPane` 0x19a7f0. For each,
     the spec of what to record.
4. **The recording requests:** your report ends with a table: function, address, regions, the game situation (level
   and what the player does), and how many calls. The main agent records them and you (or the next task) close them.

Tests: the existing EE-snapshot checks (`nf-extract pose-check` against `work/ee_start_*.bin`) must pass bit-identical
for every object and bone matrix once the sine and the palette are exact, with a sister assertion that the host `sin`
path is not. All existing trace tests must still pass. If a trace's outcome changes, the new one must be closer to the
real game's trace, and you say so.

Files: `docs/tools/call-traces.md`, `crates/nf-game/tests/diff_calls.rs`, a new `crates/nf-game/src/ps2math.rs`, the
matrix code in `crates/nf-formats/src/placement.rs` and the animation module, `docs/formats/animation.md`
("Arithmetic"). Update G-017 and G-024.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day, and the main agent's
recordings land there.

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

**Branch:** `s/verify-bitexact`.

---
claimed-by: sandbox3 at 2026-10-13T09:06Z

---
## Report (done, 2026-10-13T10:25Z)

