## Your task: `guard-reaction-delay2` — land 664 without breaking the measured hit matrix

`s/guard-reaction-delay` (664) was HELD on 2026-10-16. On master plus 663/667/668 plus 664,
`fix_guard_weapons_live::the_3x3_hit_rate_and_damage_matrix_matches_the_game` FAILS 4 of 4 standalone runs. Without
664 it passes 2 of 2 (and every earlier suite). Your report called it a "tolerance-edge flake", but it's
deterministic on the merged tree: 664's sight-to-fire changes move the guards' fire streams out of the
PCSX2-measured hit-rate tolerance.

The matrix is measured on the real game, so both must hold: the reaction delay the user wants ("give it half a
second") AND the measured per-cell hit rates. Find which part of 664 shifts the streams (the first-shot delay, the
aim-raise gate, or a changed RNG draw order) and port it so the measured matrix stays green. Prove it on the exact
merged tree (origin/master + your branch). Also note 688 (cross-process determinism) is queued for the
process-to-process variance you saw; don't widen tolerances to hide either.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guard-reaction-delay2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T09:27Z

---
## Report (done, 2026-10-16T13:21Z)

