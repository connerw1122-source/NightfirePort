## Your task: `anim-events`, animation script events with effects

GAPS G-030: the animation scripts' events with effects are partly unported: grenades thrown by an anim event, close
combat, weapon changes (`DroneAnim_EventFunc` datum 0x28/100), messages, footprints; `PlayFiringAnim`'s use of the
+0x20/+0x21 flags. Census the event opcodes every anim script on the disc uses (nf-extract's script dump), list which the
port handles, then port the rest from `AnimProcessScriptCmds` 0x1c1b38 and `DroneAnim_EventFunc`. A test per event kind on
a real script that uses it.

Files: `crates/nf-game/src/{anim,drone}.rs`, `docs/systems/anim-engine.md`.

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

**Branch:** `s/anim-events`.
