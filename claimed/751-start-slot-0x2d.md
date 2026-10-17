## Your task: `start-slot-0x2d` — the second start-position set (0x2D)

51 placements, one per level including every MP map: `Player_AddNewStartPos(..., 0)` 0x195c00. The port registers only the 0x24 starts (nf-view lib.rs `level_starts`), so on levels that ship both (07000007 has 5 of type 0x2D) Bond may spawn at the wrong start. Port the registration and `Player_Start`'s pick across both sets [M 0x190fd0]. Test: each level's chosen start matches the game's rule; check 07000007 against a PCSX2 start position if one is measured in docs/live.

The script census (task 709, merged) lists this gap with occurrence counts and the game handler in `docs/systems/script-census.md` ("Top 10 ready-to-queue tasks"). Port from the decompile [M], test on real levels through the real path with a sister assertion, and update `crates/nf-formats/tests/script_census.rs`'s handled sets.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/start-slot-0x2d`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-17T04:39Z
