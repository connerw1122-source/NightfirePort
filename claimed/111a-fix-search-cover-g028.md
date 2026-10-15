## Your task: `fix-search-cover-g028`, guard cover use (G-028, part of G-026)

G-026/G-028 (docs/GAPS.md; docs/systems/drone-ai.md): guards don't use cover nodes the way the game does. The port runs
the cover pass after vision (`Drone_InitComms` order note, WORKLOG 0034) and has no real cover-node selection. The
level's AI network carries the cover nodes (`crates/nf-game/src/cover.rs`, `nav.rs`).

Do:
1. From the decompile: the cover-node selection (`NDrone2_*Cover*` / `Drone_FindCover*`), the scoring (distance, LOS to
   the threat, crouch/stand flags 0x20 seen in drone.rs), when a guard enters and leaves cover, and the order inside the
   tick relative to vision. Document it in docs/systems/drone-ai.md (cover section) with addresses.
2. Port the selection, the order, and the enter/leave transitions. RNG calls in the game's order.
3. Test (NF_ISO, a level with cover nodes near guards; 07000005's courtyard or 07000012): under fire, a guard
   picks the node the game's scoring picks and crouches/stands per the node flags. Where the live traces
   (work/ai_trace_*.tsv) show cover behaviour, match them. The full suite must stay green; AI fight tests are sensitive,
   so explain any change.

Files: `crates/nf-game/src/cover.rs`, `drone.rs` (the hook), docs/systems/drone-ai.md, the new test.

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


**Branch:** `s/fix-search-cover-g028`.

---
claimed-by: sandbox4 at 2026-10-15T14:31Z
