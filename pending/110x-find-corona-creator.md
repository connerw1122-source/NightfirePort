## Your task: `find-corona-creator`, who creates the runtime class-0x45 coronas (U-042)

U-042 (docs/UNKNOWNS.md; G-034 in docs/GAPS.md): `Corona_Update` (the class-0x45 update with a line-of-sight fade) has no
known creator, so the port doesn't run it. The static placement coronas (type 0xFC) are ported.

Do:
1. Find every write of class 0x45 into a new object (`control_create_object` callers passing 0x45, or a
   `control_funcs` slot assignment) across ACTION.ELF. Check script entity creation (`Script_CreateEntity` 0x1d8968's
   type switch), the searchlight/light builders, and the vehicle/explosion code. List each with its address.
2. If a creator exists: port it and `Corona_Update`'s LOS fade (the fade rate, the occlusion ray, the alpha byte) in
   `crates/nf-game/src/corona.rs`, with an NF_ISO test on a level that creates one. If none exists on the disc's paths,
   prove it (the full caller scan) and close U-042 as dead code.

Files: `crates/nf-game/src/corona.rs`, docs/systems/corona.md (or the existing corona section), docs/UNKNOWNS.md (U-042),
the new test.

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


**Branch:** `s/find-corona-creator`.
