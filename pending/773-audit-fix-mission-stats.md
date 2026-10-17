## Your task: `audit-fix-mission-stats`, Mission Stats rows contradict the decompile (daily audit 2026-10-17)

`crates/nf-game/tests/mission_stats_rows.rs`:
1. :38 asserts "01:00:00" — `Timer_Seconds2String` 0x1deaa8:27-31, mode 0 with hours != 0, prints the two-field
   `"%.2d:%.2d"`.
2. :162, :163, :165 assert the current values are zeroed at a non-6 status — `GetScore` 0x1984b0:321-332 zeroes only the
   +0x10 scores and the totals; the currents the handler shows (0x209938:70/84/101) survive, so the game shows 1, "01:19"
   and the ratio.
Fix progress.rs / the stats rows to the game and the tests with sisters. Also re-cite `interact_07000004_live.rs:220-221`:
it cites Env_Create at 0x1e5d20 (inside View_DrawSky) — the real one is 0x17b968; and make `interact_07000001_live.rs:341`
(`on(0) || true`) and the three interact_07000001 fallback branches (shoji, vase, monitor) actually assert.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/audit-fix-mission-stats`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
