## Your task: `gate-guard-truck`, the 07000005 truck-ride exit's gate guard (channel 0x22)

GAPS G-037: the 07000005 level end by truck (route (a): the script-created triggers + MuxIn + ScriptPlayer + event 0x11
+ LOAD_LEVEL chain) is ported and NF_ISO-tested up to one missing link — the gate-guard drone must be at its post
(channel 0x72), and what sets channel 0x22 (the guard's activation) is unknown (U-048 in `docs/UNKNOWNS.md`). Find the
writer of channel 0x22 in the level's scripts/triggers/drone params and the decompile (the drone's activation path,
`Drone_InitComms`, the placement params, any script event that sets 0x22), port it, and add the full truck-ride
integration test: Bond rides the truck from the start to the LOAD_LEVEL of 07000006, through the real input path. Use
`NF_LEVELLOG=1` to trace. Non-visual: the test is the proof.

Files: `crates/nf-game/src/drone.rs`, `crates/nf-game/src/triggers.rs`, a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/fix-gate-guard-truck`.

---
claimed-by: sandbox1 at 2026-10-15T08:42Z
