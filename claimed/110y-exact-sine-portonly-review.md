## Your task: `exact-sine-portonly-review`, review the port-only trig stand-ins (G-046's last list)

G-046 (docs/GAPS.md) lists four PORT_ONLY trig sites: places where the port uses trig and the game has none, so they
are port inventions rather than exact-sine swaps: `ladder.rs` mesh columns, `player.rs` crawl advance, `casing.rs` eject
matrix (`Casings::eject`, the guard path's yaw/pitch basis), `drone.rs` object-space head-track.

For each: find what the game really does at that point (decompile, with addresses). Replace the port's construction with
the game's mechanism: a matrix it already has, a table, a datum, whatever the game reads. If the stand-in is in fact
equivalent, prove it with a test and say so. Each change needs a fail-first NF_ISO test, and the full suite must stay
green.

Note: the PLAYER casing path now ejects from the view model's datum at the end of its delay (`Casings::reserve`/`arm`,
merged 2026-10-15). Only the guards' `eject` (the yaw/pitch basis at the muzzle) is in scope for casing.rs. The game
reads the drone's datum through `AnimGetBoneWorldTrans` (0x12f678:100-113, the drone arm).

Files: `ladder.rs`, `player.rs` (crawl), `casing.rs` (`eject`), `drone.rs` (head-track), the new tests, docs/GAPS.md (G-046).

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


**Branch:** `s/exact-sine-portonly-review`.

---
claimed-by: sandbox5 at 2026-10-15T14:20Z
