## Your task: `fix-door-react-path2`, land 110p (held: the 07000001 escort walk times out)

110p (`s/fix-door-react-path`, Door_ReactToHit through the game's collide path) was HELD on 2026-10-15. On master + 110p
the main agent's full suite gave **1335 passed, 1 failed**: `nf-view/tests/walkreal_07000001_live.rs` TIMEOUT after
600 s (`walk_from_the_spawn_to_the_exit_with_the_contact`). Master alone passes it (1333/0 on the same base).
07000001's escort route goes through portal-linked doors (docs/atlas/door.md: 5 doors, all param 4 = 1).

Do: reproduce with the walk test alone. Find where the walk stalls: a door that now reverses on the contact or on Bond
and never finishes opening? A door stuck closing on the escort? Compare the new collide-path anti-crush with the game's
rule (who counts as "in the way", the reversal condition, the timer). Fix it the game's way, and pin the door's
behaviour in that spot with a focused test (the door at the stall point opens and lets the contact through). The
escort walk must pass 2/2 and the FULL suite be green on the final tree. Report the total line.

Files: the branch's files plus whatever the root cause needs.

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


**Branch:** `s/fix-door-react-path2`.
