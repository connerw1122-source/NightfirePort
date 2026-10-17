## Your task: `audit-fix-door-guard`, door_guard_live contradicts the decompile (daily audit 2026-10-17)

`crates/nf-game/tests/door_guard_live.rs` (and the port code it pins):
1. :131-132 calls the check a "yaw gate" — 0x141360:90 compares +0x34 (position Y) against 2.0: a HEIGHT gate.
2. :136-187 builds the plane normal as cross(v0, (0,0,1)) — 0x141360:125-133 uses (0,1,0), so the game's normal is
   horizontal; the "degenerate zero normal" for a pair along z never happens in the game.
3. :195-288 treats s0 as height and the gate as aim yaw — 0x1416d8:22 gates on the opponent's height difference, so the
   zone spots the test uses are not the game's geometry.
Re-read 0x141360 and 0x1416d8, port them exactly, rewrite the tests to the game's rule with sisters that can fail.

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

**Branch:** `s/audit-fix-door-guard`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
