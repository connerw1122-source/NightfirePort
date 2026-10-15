## Your task: `0c-hatch-latches`, 0c display hatch: two latches missing, culling on the drop, still can't get through

User (0700000c): "It's missing two latches on that hatch. Don't know why... when I dropped down, I got some culling
issues. I took some screenshots of that. And I couldn't get through still." The hatch needs its four latches drawn
and burnable (two don't draw - find which placements and why: the misc gate, a glist flag, the striped-box gate's
0x10 rule, LOD), the culling after the drop through the hole (the eye cel / portal on the inner ledge - the user's
F12 spots: `0700000c --bond 5.34,12.49,-111.81,-3.654,-0.234` (frame 2326) and `--bond 5.33,12.89,-110.94,-3.654,-0.234` (frame 2454)), and the traversal still
blocked (fix-0c-vent2's report: the way down needs the winch platforms #351/#364/#377 - port their motion).
From the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`). Drive the real input
path in tests; fail first on master; render before/after for anything visible; put exact reproduction steps in the
report for the main agent's PCSX2 compare.

Files: `crates/nf-game/src/{world,props}.rs`, `crates/nf-view/src/scene.rs`.

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

**Branch:** `s/0c-hatch-latches`.

---
claimed-by: sandbox2 at 2026-10-15T04:39Z
