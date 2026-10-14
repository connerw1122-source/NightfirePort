## Your task: `0c-vent2`, 0700000c: get Bond through the display vent

101g-fix-0c-hatch fixed the hatch speed (SimpleScript `(keys-1)/rate`) and mapped the vent traversal, verifying two of
its four legs (docs/reports/fix-0c-hatch.md "The traversal"): Bond drops through the open hole onto the inner ledge.
The remaining two legs (from the inner ledge down to the display case floor ~6.3-7.3, and whatever the game requires to
fit through: crouch, a creep volume (none on 0c per the census), or plain geometry clearance) are open, and the user
couldn't get through in the playtest ("he can't crouch through"). Finish it through the real input path from
`--bond 5.8,15.8,-107.5,3.14,0` with the four latches burned: a test that reaches the display floor at (5.4, 7.3, -113);
sister assertions; and the exact keys the user should press, for the next playtest.

Files: `crates/nf-game/src/{player,world}.rs` as needed.

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

**Branch:** `s/0c-vent2`.

---
claimed-by: sandbox7 at 2026-10-14T10:30Z

---
## Report (done, 2026-10-14T11:18Z)

