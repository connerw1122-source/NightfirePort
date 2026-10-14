## Your task: `casings-16`, casings when walking backwards; 16's stray mission-complete

The user's playtest (results 1013-1825, 07000016): "I see bullet casings flying when I walk backwards" (casings spawn
without firing: probably on a footstep or animation event that the port reads as the eject event, or the eject offset
moving with the walk) and "I saw a mission complete thing pop up for some reason in this mission, but I still couldn't
continue". Reproduce both headlessly (walk backwards 3 s with no fire: 0 casings; play 16 from its start to that point:
which objective/mission message fires and why), fix from the game's code, test, sister assertions.

Files: `crates/nf-game/src/{casing,gun,mission}.rs` as needed.

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

**Branch:** `s/casings-16`.

---
claimed-by: sandbox7 at 2026-10-14T09:15Z
