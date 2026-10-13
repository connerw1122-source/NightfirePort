## Your task: `fix-ai-traces`, the guards against the real game's recordings

The user recorded real PCSX2 play with `tools/trace/trace_record.py` (god mode, per-frame Bond and drone state):
`tests/traces/05-truck.trace`, `01-doors.trace`, `01-raptor.trace`, `02-civilian.trace` (the format is in the file
header and `docs/atlas/*` "trace check" sections). Earlier tasks used them for the truck, the doors and the pickups.
Use them for the **AI**:

1. **Replay Bond's recorded path** through the port's sim (`nf_view::load` + `step_bond` with Bond placed per frame
   from the trace, as fix-truck did), and compare every drone's state id, position and facing with the recording,
   frame by frame. Write the first divergence per drone into `docs/atlas/drone-port-check.md`.
2. **Fix the divergences** from the decompile, in order of how early they happen: alert timing, sight and hearing,
   pathing, cover use, firing cadence (`DroneWeap_NextBulletTime`), reactions to being hit. Each fix cites the address,
   and its test replays the trace window and asserts the drone now matches.
3. **Note what needs a new recording** (a situation no trace covers) and write the exact steps for the user to record
   it with `dist/record.ps1`, in your report.

Keep the RNG in game order: a divergence caused by an RNG draw out of order is a fix, not noise. Tests: per trace, the
number of frames matching per drone before the first divergence, pinned so it can only go up.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in
your report. Put new behaviour in new files. Pull `master` before you start and before every push: other workers
merge into it all day.

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

**Branch:** `s/fix-ai-traces`.

---
claimed-by: sandbox2 at 2026-10-13T09:33Z
