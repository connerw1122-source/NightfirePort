## Your task: `options-persist-roundtrip` — the options survive a restart (651 end to end)

651 (settings-persist) added the config file. Prove it by the real path:
- change every Advanced / Multiplayer / AV option row through the pages' Left/Right and Confirm;
- drop the FrontEnd;
- a fresh FrontEnd with the same config dir shows the same values;
- the AV volume rows reach the sound layer's volumes at boot;
- Back on a page throws the edits away (they're not saved).

Unknown keys and out-of-range values in the file are tolerated (clamped). Use `NF_CONFIG_DIR` / a temp dir, never
the user's.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/options-persist-roundtrip`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T17:37Z
