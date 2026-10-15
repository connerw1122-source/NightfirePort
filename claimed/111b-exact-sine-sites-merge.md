## Your task: `exact-sine-sites-merge`, land 110k + 110m (held: the 07000001 escort walk breaks)

110k (`s/exact-sine-sites-1`) and 110m (`s/exact-sine-sites-3`) were HELD on 2026-10-15. On master + both (with G-046
in docs/GAPS.md resolved to one entry), the main agent's full suite gave **1329 passed, 2 failed**:
- `nf-view/tests/walkreal_07000001_live.rs::walk_from_the_spawn_to_the_exit_with_the_contact` — `the contact must walk
  his route to the LoadPointActivator plate` (line 218); 882 s alone, 600 s timeout in the suite. Master passes it.
- `nf-view/tests/fix_08_walls_live.rs::the_wire_icons_straddle_set_is_its_own_west_room_cel` — an exact equality on
  the wire icon's bounds centre: left [133.42567, 48.326435, -101.2065] vs right [133.42569, 48.32644, -101.2065]. The
  exact world sphere moves it by 2e-5. Decide from the game's math whether the test's "placement origin" expectation is
  the game's value or the old host value, and update the test with that justification (never just loosen it).

Do:
1. Bisect: master + 110k only, master + 110m only. Which one breaks the escort walk?
2. Find why with a side-by-side replay (exact vs host), the way fix-exact-math-swap did (docs/reports/fix-exact-math-
   swap*.md): the first divergent tick, the first AI-visible divergence, and the decision it flips (a guard's sight
   margin? a trigger touch? the contact's path?).
3. Fix it the game's way: if the port's exact math is right and some other ulp-wide stand-in breaks downstream, fix that
   stand-in. Don't revert the exact sine. Show the escort walk passing 2/2 and the full suite green.

Start from master, merge both branches, resolve G-046 into one entry. Files: whatever the root cause needs, listed in
the report.

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


**Branch:** `s/exact-sine-sites-merge`.

---
claimed-by: sandbox3 at 2026-10-15T14:47Z
