## Your task: `level-chain`, levels with no exit trigger: how the game moves on

docs/reports/campaign-flow.md, blockers 2-4: 07000006, 07000011 and 0700004a have no 0xE8 exit trigger in any map, and
09/0a's LOAD_LEVEL triggers point at their own level; the runner had to force the mission complete, which lands on the
front end. The game obviously moves on (06 -> 07, 11 -> 12, 09 -> 0a -> 0b). Find the mechanism in the code: a
scripted channel-100 write, `Mission_Update` 0x182c60 case 6's next-level table, `ResetMap_LevelToLoad` 0x1d3800 /
`GetLevelToLoadFromMovieCode` 0x1d3560, a script command that requests the load, or a trigger kind the port's parser
skips. Port it so each of those levels chains to the right next level when its objectives are done. Tests (NF_ISO):
for 06, 09, 0a, 11, 4a the completed level requests the right next level (numbers: the hash); sister assertions.
List any part that needs a PCSX2 watch, with the exact addresses to watch, for the main agent.

Files: `crates/nf-game/src/{mission,triggers}.rs` and the script layer as needed.

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

**Branch:** `s/level-chain`.

---
claimed-by: sandbox3 at 2026-10-14T15:48Z
