## Your task: `fix-gas-rng3`, land the gas RNG work (111d held again: the same 4 failures)

111d (`s/fix-gas-rng2`, 9bcf2ee — the collide-pass draw order + the puff's 128-tick lifetime on top of 110o) was
HELD on 2026-10-15. On master a1cb5b36 + 111d the main agent's full suite gave **1355 passed, 4 failed** (master
alone is green):
- `nf-view/tests/fix_gas_rng_live.rs:84` (its own test) — "the muzzle smoke puff exists (pre-fix: life 8.0,
  counter-hash fields)": no puff found on the merged tree.
- `nf-view/tests/walkreal_07000001_live.rs:218` — "the contact must walk his route to the LoadPointActivator plate".
- `nf-view/tests/fix_guard_weapons_live.rs:156` — "no candidate guard with weapon 0x12 completed the collection".
- `nf-game/tests/ai_live.rs:249` — "UnderCoverAim lasted 90 (live 37)".
These are the same four 110o broke: every extra or reordered `Rand` draw shifts the shared stream the AI/walk tests
were tuned against. Either the new draw order is right and those tests' expectations (or the code they exercise)
must be re-derived from the game — show with the decompile/trace which — or the order is still wrong.

Start from `origin/s/fix-gas-rng2`, merge `origin/master` first, reproduce the four failures, and fix them for real
(no loosened tolerances without the game's evidence). Report each test's before/after and why.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/fix-gas-rng3`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-15T17:48Z
