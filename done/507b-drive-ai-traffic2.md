## Your task: `drive-ai-traffic2`, land 507 on top of 506

507 (`s/drive-ai-traffic`, 1fb6f04 — the driving AI cars) was HELD on 2026-10-15: it does not build on master. 506
(`s/drive-physics`, merged) made `nf_game::driving` a single file `crates/nf-game/src/driving.rs`; 507 created
`crates/nf-game/src/driving/mod.rs` — rustc E0761 "file for module `driving` found at both". Start from
`origin/s/drive-ai-traffic`, merge `origin/master`, and restructure into ONE module layout (e.g. move 506's
`driving.rs` into `driving/physics.rs` + `driving/mod.rs` re-exporting, keeping every public path 506's tests and
callers use), wire the AI onto 506's physics model where the task asked for it, and get both crates' tests green.
Report the layout chosen.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-ai-traffic2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-15T22:10Z

---
## Report (done, 2026-10-15T23:34Z)

