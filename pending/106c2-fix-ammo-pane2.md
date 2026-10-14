## Your task: `fix-ammo-pane2`, ammo pane: the tests read a local snapshot

Base: `s/fix-ammo-pane` — merge origin/master FIRST. HELD: the pane itself renders right (the main agent's render at
0700000c `--bond 14.22,16.28,-139.54,0.986,-0.269` shows the icon, "5-30" and "Single" like the real game), and the full
suite passed everything except your two tests, which panic at ammo_pane_live.rs:17 reading `work/ee_snapshot.bin`
("The system cannot find the path specified") — a local PCSX2 snapshot that isn't in the repo and doesn't exist in the
merge checkouts. Tests may depend only on NF_ISO (CLAUDE.md: ISO-reading tests opt-in via NF_ISO; no game bytes in the
repo). Take the weapon records / pane data the tests need from the disc through the port's own loaders (the weapon
table the port parses from ACTION.ELF on the ISO), keep every assertion, and make anything that truly needs a live
snapshot an `#[ignore]`d probe with a reason. Run the full `tools/test/suite.py`; total in the report.

Files: `crates/nf-view/tests/ammo_pane_live.rs`.

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

**Branch:** `s/fix-ammo-pane2`.
