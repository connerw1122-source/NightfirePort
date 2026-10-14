## Your task: `grenade-types`, grenades: stun, smoke, flash, gas behave like the game

The user reported "smoke grenade didn't work" (playtest 1013-1825). Check every thrown grenade record against the game:
the frag 0x34, stun 0x35, smoke 0x36 and any flash/gas/other thrown records (0x3a-0x40 aside — satchels/tripbombs are
107b). For each: the fuse, the blast/cloud (`Occlude_Create` 0x17ea80/0x17eba0 for smoke, the stun's effect on guards
and on Bond — the white-out/blur and the guards' stunned state), the cloud's size and lifetime, guards' reaction
(coughing/blind states), sounds and particles. Port what differs. Tests (NF_ISO): per grenade type, the effect spawns
with the game's numbers and a guard inside reacts as the game does; sister assertions.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. PCSX2 captures carry a CRT filter: use them for presence/placement, not exact colour.

Files: `crates/nf-game/src/{grenade,drone,explode}.rs`, the screen effect in `crates/nf-view` (small).

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

**Branch:** `s/grenade-types`.

---
claimed-by: sandbox4 at 2026-10-14T21:24Z

---
## Report (done, 2026-10-14T23:11Z)

