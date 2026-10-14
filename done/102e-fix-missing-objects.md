## Your task: `missing-objects`, furniture, the fountain and the 0c pylon aren't drawn

PCSX2 at the user's spots (main agent, 2026-10-13) shows whole objects the port doesn't draw:
- 07000006 `--bond 21.12,39.04,-0.48,8.683`: the courtyard FOUNTAIN (basin, tiers, two fish statues): the port shows only
  the water patch;
- 07000006 `33.50,41.13,-17.90,9.158` (the guard room): the bed, desk, chair, wall map and clock are missing;
- 07000006 `31.94,41.13,-17.36,10.756`: the lockers and the fusebox panel are missing;
- 0700000c `14.22,16.28,-139.54,0.986`: the steel power PYLON is missing (its baked shadow is drawn on the ground);
  the user also says wires are missing.
The user: "We're missing beds, desks. Wires or things missing." Find why: list every placement on 06 and 0c whose
mesh the port doesn't draw (class, mesh hash, the skip reason: the cel pass's flag 0x10 skip, an unresolved mesh, a
spawner hold-back, a world-object class with no draw), compare with the game's create path for each class, and draw what
the game draws. Run the same census on every on-foot level and report the counts. Tests: these named objects are in the
drawn set at those spots; a census with no unexplained skips; sister assertions.

Files: `crates/nf-view/src/scene.rs`, `crates/nf-game/src/{worldobj,props}.rs` as needed.

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

**Branch:** `s/missing-objects`.

---
claimed-by: sandbox5 at 2026-10-14T07:31Z

---
## Report (done, 2026-10-14T09:00Z)

