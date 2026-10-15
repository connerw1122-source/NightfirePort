## Your task: `missing-drone-skins`, the 33 drone placements whose skin isn't in their archive

GAPS G-021: 33 drone placements across the story levels reference skins that their level archive doesn't carry; the
game loads them from the common animation file (`CommonAnimFile`, `LoaderLoad` type 4) and/or substitutes sleeves
(`AnimSleeveGetEntity`). Today those drones load with no skin (count them per level first: the loader's "N drones with a
skin from another archive / unresolved" line). Port the game's resolution order from the decompile (`LoaderLoad`,
the common file's name and contents on the disc, the sleeve substitution rule), so every placement gets the skin the
game would draw. NF_ISO tests: every story level's drone placements resolve a skin, with a per-level count golden and
a sister assertion; list the before/after counts in your report. Non-visual: the main agent renders the result.

Files: `crates/nf-view/src/lib.rs` (the loader), `crates/nf-formats/src/character.rs`, a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/fix-missing-drone-skins`.

---
claimed-by: sandbox7 at 2026-10-15T08:51Z

---
## Report (done, 2026-10-15T09:38Z)

Closed by the driver: the report is on `s/fix-missing-drone-skins` (docs/reports/fix-missing-drone-skins.md); the worker didn't run finish.sh.
