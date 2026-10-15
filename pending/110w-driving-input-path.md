## Your task: `driving-input-path`, the driving levels' input path from DRIVING.ELF (G-004, U-003)

G-004 (docs/GAPS.md, U-003 in docs/UNKNOWNS.md): the driving levels' input path is unported. Only DRIVING.SYM name hints
exist (`PS2PadDevice`, `InputDevice::DeadZoneChop/Scale`). The on-foot path is done (docs/systems/input.md, fix-input).

Do (non-visual, decode and logic only):
1. In DRIVING.ELF (Ghidra project work/ghidra/NightfirePort.gpr; the decomp export under work/decomp/DRIVING.ELF), find
   the pad read, the dead-zone chop/scale, the steering/throttle/brake mapping and any ramps. Document them in
   docs/systems/driving-input.md with addresses, and compare with the on-foot ACTION.ELF path (input.md §2): the same
   threshold 40 and scale 127/87, or different?
2. Port it as a pure function module `crates/nf-game/src/input/driving.rs` (raw pad bytes in, the vehicle control values
   out) with bit-exact unit tests on the decompiled arithmetic (golden tables from the formulas, sister assertions).
3. Map the keyboard+mouse bindings the port uses onto it (mouse and keyboard only; no controller work beyond what the
   pure function needs). Note where a driving level would call it; don't wire the driving levels themselves.

Files: `crates/nf-game/src/input/driving.rs` (new), docs/systems/driving-input.md (new), docs/GAPS.md (G-004),
docs/UNKNOWNS.md (U-003).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).


**Branch:** `s/driving-input-path`.
