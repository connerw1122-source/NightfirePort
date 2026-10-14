## Your task: `swim-entry`, swimming: the entry depth off the 02 bank

102h-fix-swim fixed the stroke, dive/surface, footsteps and the pond ladder. Its report leaves the ENTRY open: on the
07000002 bank slope (`--bond -82.3,5.3,32.05,1.57`) the port enters the swim only when Bond's feet are 0.62 under the
surface (pos 3.747 vs surface 3.7675), because `find_cel` picks the smallest containing cel and the bank's land cel wins
until he's deep; the user felt he "very, very slowly got into the water". Find the game's cel pick for the water entry
(`Player_InWater`, `build_FindCel` 0x1c8cb0's tie-break, the cel+0x90 surface test) and match it, so the swim starts at
the game's depth. Test: walking off the bank enters the swim at the game's depth and tick count; sister assertions.

Files: `crates/nf-game/src/{player,world}.rs`.

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

**Branch:** `s/swim-entry`.

---
claimed-by: sandbox6 at 2026-10-14T10:50Z
