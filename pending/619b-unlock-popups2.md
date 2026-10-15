## Your task: `unlock-popups2`, land 619 (held: 1 suite failure)

619 (`s/unlock-popups`, b143289 — the post-results bonus / cheat-medal / win pages) was HELD on 2026-10-15. On master
+ 619 (the menu.rs merge kept codename-entry/secret-codes/screen-adjust fields beside 619's — check it) the main
agent's full suite gave **1540 passed, 1 failed**:
- `nf-view/tests/campaign_flow_live.rs:751` `front_end_chain_01_to_05` — expected `Page::MissionSelect` after the
  results page's Confirm, got `Page::NfBonus`.
Decide from the game which is right for that chain (does P_NFRESULTS → P_NFBONUS appear after mission 01's
completion with that test's stats? which medal/bonus condition gates it?) and fix the code or the test with that
evidence (the test may need to Confirm through the bonus page when the game shows it). Start from
`origin/s/unlock-popups`, merge `origin/master` first, reproduce, fix, report before/after.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/unlock-popups2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
