## Your task: `0c-hatch2`, the SimpleScript timing fix breaks four AI traces

101g-fix-0c-hatch (`origin/s/fix-0c-hatch`, held) changed the SimpleScript step to the game's `(keys-1)/rate` per tick
[M SS_Create 0x1a9c58:51, SS_Update 0x1a9948:103-108]. On its own base it passed; merged with master 69c0104 the suite
fails four AI traces: `ai_traces_live::ai_01_doors`, `ai_01_raptor`, `ai_02_civilian`, `ai_05_truck` (master alone
passes all four). Every SimpleScript-driven object now moves at the game's speed (doors, lifts, the truck's props), so
the traces' timing shifts. Find out which: if the new timing is the game's (the traces were recorded in PCSX2 against
the real game, so a correct port should match them better, not worse), the old SS timing may have been compensated
somewhere else that must now go; if a trace really depended on the old wrong speed, re-check the trace against its
recording. Do not loosen the traces. Merge master first, fix, full suite 0 failures; keep the hatch tests. Also finish
the vent traversal the report left open (two legs) if time allows.

Files: `crates/nf-game/src/ss.rs` and what the bisect points at.

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

**Branch:** `s/0c-hatch2`.
