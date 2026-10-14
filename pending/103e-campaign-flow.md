## Your task: `campaign-flow`, play every mission end to end headless: the blockers list

Make a headless runner that plays each single-player mission's levels in order from the front end's mission start,
driving Bond through each level's objectives with the existing walkreal walkers / scripted routes where they exist (and
the dev teleport + objective triggers where they don't), through the level transitions and the end-of-mission flow
(outro, results, the next mission unlocked). Record per level: reached the exit / which objective blocked / crashes /
stuck script / missing trigger. Write `docs/reports/campaign-flow.md` with the table, and fix the first blockers you can
pin with the game's code (each with a test). This is the map of what still stops a full playthrough; list the rest as
precise follow-up tasks in the report.

Files: a new `crates/nf-view/tests/campaign_flow_live.rs`, `docs/reports/campaign-flow.md`, fixes as found.

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

**Branch:** `s/campaign-flow`.
