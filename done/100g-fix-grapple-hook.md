## Your task: `grapple-hook`, the grapple point's hook isn't drawn

PCSX2 vs the port at `--bond 16.5,29.6,58.6,-2.01,0.24` on 07000005 (main agent, 2026-10-13): the real game shows a
PULLEY WITH A HOOK hanging from the T-shaped bracket on the castle wall: that is the grapple point the Q-Claw latches
onto (#282, (13.75, 32.29, 57.3), class 'Q' 0x51 per fix-qclaw). The port draws the bracket only: the hook/pulley object
is missing, so the user sees "no grapple hook point". Find the grapple point's mesh (the 0x51 object's model or its
attached part, which archive/hash; is it skipped by the cel pass, the class filter, a script-object hold-back, or never
built?) and draw it on every level that has grapple points (05, 0c, the others: list them). Also its state change when
latched, if the game animates it. Test: the grapple point's mesh is in the drawn set at that spot on 05 (and one
grapple point per level that has them); sister assertions. Render the spot with `--shot` before/after.

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

**Branch:** `s/grapple-hook`.

---
claimed-by: sandbox4 at 2026-10-14T05:36Z

---
## Report (done, 2026-10-14T06:24Z)

Closed by the driver: the report is on `s/fix-grapple-hook` (docs/reports/fix-grapple-hook.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T06:51Z)

