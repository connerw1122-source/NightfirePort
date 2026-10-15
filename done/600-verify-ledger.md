## Your task: `verify-ledger`, prove more ported functions bit-exact against the real game's recorded calls

Idea taken from REA (morluto/rea, MIT): `compare_functions` / `verify_reconstruction` judge a reconstruction against a
finite, explicit specification, and "a pass covers only the declared claims". We already have that machinery for three
functions (045n-verify-bitexact: `tools/trace/gdbcall.py` records real calls in PCSX2, `docs/tools/call-traces.md` is the
format, `tests/calls/*` the recordings, `crates/nf-game/tests/diff_calls.rs` replays them; PS2Sinf, Quat_QuatToMat and
Rand_Rand are 40/40 bit-exact). Extend it into a **ledger**:

1. `docs/verify/ledger.md`: one row per ported game function the sim depends on (address, name, the port's function,
   status: bit-exact on N recorded calls / within tolerance / unverified / known divergent with the reason). Seed it
   from the port map and the atlas, starting with the hot paths: the drone vision/LOS math, `Vision_InView` 0x1e89e0,
   `Quat_Slerp_Acc` 0x1e4188, `Mat_Align2Dir` 0x1df2a8, `Collide_SphereIntersect` 0x1ec940, `Intersect_RayGeom`
   0x10a8c8, the `Rand_*` family, `NDrone2_HitDamage` 0x145558, `Explode_Propagate` 0x17c518, `AnimFrameSet` 0x1c75a0,
   `DroneWeap_DoBulletAccuracy` 0x179740.
2. For each function you can replay WITHOUT the emulator: add a replay test in `diff_calls.rs` style. Where recordings
   don't exist yet, write the exact `gdbcall.py --spec` command (registers + memory regions in/out) the main agent
   should run in PCSX2 to record 40 calls, and the level/spot to run it at. The main agent records them and you (or the
   next session) wire the replays.
3. A test that fails if a ledger row claims "bit-exact" without a recording + replay test behind it.

You can't run PCSX2 in the sandbox. Write the recording requests precisely, and the main agent will run them.

Files: `docs/verify/ledger.md` (new), `crates/nf-game/tests/diff_calls.rs`, `docs/tools/call-traces.md`.

**Run the crates your tests touch before pushing**, and report what you ran. No gameplay code changes in this task.

**Branch:** `s/verify-ledger`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-15T16:57Z

---
## Report (done, 2026-10-15T17:57Z)

Closed by the driver: the report is on `s/verify-ledger` (docs/reports/verify-ledger.md); the worker didn't run finish.sh.
