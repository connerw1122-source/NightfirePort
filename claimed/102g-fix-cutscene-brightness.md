## Your task: `cutscene-brightness`, cut-scene characters lit far too bright

The user's playtest (results 1013-1825, the 07000002 intro): "the models in the game during the cutscene are extremely
bright." PCSX2's 02 intro (main agent, 2026-10-13) shows Bond and the man at the computer lit darkly to match the room;
the port's are near-white (the user's screenshot at `--bond -82.30,5.30,32.05` frame 170). Find what the game lights
cut-scene actors with (the character room light / the scene's own light set; 290-fix-character-room-light is pending for
in-game characters: check whether cut-scene actors take a different path in the port, e.g. no cel light or a default
full-bright ambient). Fix and test: an actor in the 02 intro gets the lighting the game gives it (numbers), not full
bright; sister assertions.

Files: `crates/nf-view/src/scene.rs`, `crates/nf-game/src/nis.rs`.

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

**Branch:** `s/cutscene-brightness`.

---
claimed-by: sandbox6 at 2026-10-14T07:48Z
