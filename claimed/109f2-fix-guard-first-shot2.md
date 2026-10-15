## Your task: `guard-first-shot2`, redo the first-shot delay without breaking the hit-rate matrix

`s/fix-guard-first-shot` (109f, NOT merged) holds a guard's first bullet until the sight reaction ends: in
`crates/nf-game/src/drone.rs` (the sight block, ~line 6872) it stops resetting `unseen_ticks` on the raw LOS tick and
counts it up during the reaction window, resetting it only once `seen` flips (`ProcessDroneSight` 0x176a28,
`DoFiring`'s `+0x274 < 0x10` gate 0x17a080:24-25). Start from that branch (`git merge origin/s/fix-guard-first-shot`)
and read its report `docs/reports/fix-guard-first-shot.md`.

The main agent's merge suite caught a regression, reproduced alone (passes on master d2ce47e5, fails with 109f):

```
cargo test --release -p nf-view --test fix_guard_weapons_live the_3x3
[gw] guard 3 (weapon 0x12): no level parking spot
[gw] guard 4 (weapon 0x12): no fire in 6000 ticks (shots 0, seen 0)
[gw] 07000001.bin diff 3 weapon 0x12 (guard 8, acc 5, mul 1): 152/200 hits (model 75.00) ... dist 12.63
panicked at crates\nf-view\tests\fix_guard_weapons_live.rs:217:13:
guard 8 ended inside the TooClose band or far off the engagement (dist 12.625309)
```

`unseen_ticks` feeds more than `DoFiring`'s gate (movement / engagement-range decisions read it too), so the new
counting changed where guard 8 settles. Find every reader of the port's `unseen_ticks` and check each against the game's
readers of +0x274 in the decompile: the fix must match the game for ALL of them, not only the first-shot gate. Then
both `fix_guard_first_shot_live` and `fix_guard_weapons_live` pass, and the full suite is green. If the game really
does put guard 8 inside the band, show it from the decompile and fix the test's expectation with the citation instead.

Files: `crates/nf-game/src/drone.rs`, `crates/nf-view/tests/fix_guard_first_shot_live.rs`,
`crates/nf-view/tests/fix_guard_weapons_live.rs` (only if the decompile says the expectation is wrong).


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

**Branch:** `s/guard-first-shot2`.

---
claimed-by: sandbox7 at 2026-10-15T03:58Z
