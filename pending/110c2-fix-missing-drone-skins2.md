## Your task: `fix-missing-drone-skins2`, finish 110c (held at merge)

Branch `s/fix-missing-drone-skins` (110c) was HELD by the main agent on 2026-10-15 03:50 MST. Start from that branch,
merged with `origin/master`.

Why it was held:
1. Its own test `crates/nf-view/tests/drone_skins_all_levels_live.rs` FAILS: it panics on `0700000e.bin: not in
   FILES.BIN` (`nf_game::mission::LEVELS` includes levels that aren't on-foot archives in FILES.BIN). Make the census
   cover exactly the on-foot archives the disc has (skip or filter the others, with the reason in a comment).
2. On every level it reached (07000001..0700000d), the census printed `sleeve entries 0 (raw present 0, left after load
   0)`. So the new sleeve substitution changed nothing on those levels. Either find the levels where 0x10-flagged
   MAIN entries really occur (list them with skin hash and entry index), or show that the flag test reads the wrong
   bits (re-check `AnimProcessSkinData` 0x1bd500:159-166: which word, which byte, bit 0x10) and fix it. A port of a
   mechanism with no observable case needs that case found, or a statement that none exists on the disc.
3. The report `docs/reports/fix-missing-drone-skins.md` still has PENDING placeholders (census table, fail-first,
   suite totals). Fill every one with real numbers.
4. 0700000c shows `missing skin in archive 7 (before)`. Name those 7 placements, say where each skin comes from after
   load, and confirm in the report that they draw.

Keep the merge-resolution note: on master the loader block in `crates/nf-view/src/lib.rs` (~line 564) is
`{ let mm = level.main_map(); } let mm = level.main_map();`. Your branch's edit of that block conflicted, so keep
master's braces.

Files: the branch's files only (`crates/nf-formats/src/character.rs`, `crates/nf-view/src/lib.rs`, the census test, the
report, docs).

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


**Branch:** `s/fix-missing-drone-skins2`.
