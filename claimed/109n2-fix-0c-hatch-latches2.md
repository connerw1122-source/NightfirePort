## Your task: `0c-hatch-latches2`, 109n on today's master

`s/fix-0c-hatch-latches` (109n, NOT merged) fails on current master — its own tests and two older 0c suites. The main
agent's run (master 1288f0d6 + only 109n):

```
fix_0c_hatch_latches_live: a_real_shot_starts_the_winch_and_bond_rides_it FAILED
  "a real shot must start the winch sweep [M SS_Operate 0x1a9df8:35-39 …] — got t 0 → 0"
fix_0c_hatch_latches_live: the_two_bottom_hinge_latches_draw_from_the_walkway FAILED
  "hinge #140's sphere straddles its two overlap-band cels" left: true
fix_level_0c_live: burning_the_four_hinges_opens_the_display_vent FAILED
fix_level_0c_live: the_warehouse_vent_latch_opens_its_hatch FAILED
fix_0c_vent2_live: from_the_duct_top_the_hole_drops_bond_onto_the_balcony FAILED
fix_0c_vent2_live: the_pulley_winches_sweep_once_channel_0_rises FAILED
```

On master alone all of fix_level_0c_live and fix_0c_vent2_live pass. Merge `origin/s/fix-0c-hatch-latches` into
current master, find what master changed under it (the shot path, the SS operate/update arm, the cel lists), and make
109n's fixes hold on today's master without breaking the older 0c suites. Remove or `#[ignore]` the scratch probes
(`zz_latch_probe.rs`, `zz_ss_dump.rs`) — no unasserting tests in the suite.

Files: `crates/nf-game/src/ss.rs`, `crates/nf-game/src/drone.rs`, `crates/nf-game/src/world.rs`, the 0c tests.

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

**Branch:** `s/0c-hatch-latches2`.

---
claimed-by: sandbox1 at 2026-10-15T07:20Z
