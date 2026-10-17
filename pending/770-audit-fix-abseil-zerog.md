## Your task: `audit-fix-abseil-zerog`, three tests that contradict the decompile (daily audit 2026-10-17)

The daily citation audit re-read the decompile behind these assertions; each is WRONG as written. Fix the port where it
follows the wrong value, fix the test, and fix the docs, all citing the addresses:
1. `abseil_elevator_live.rs:45` asserts `descend_speed == 0.05` — `NDrone2_DefaultInit` 0x14b300:772-774 stores
   0x3d99999a = **0.075**.
2. `abseil_elevator_live.rs:106` asserts 6.5 — `AbseilSlide` 0x169730:22 stores 0x40dccccd = **6.9**.
3. `zerog_0700001b_live.rs:63` asserts "walk stays walk" on 0700001b — `Player_Init` 0x1910f8 sends a p3 == 2 start
   straight to substate 8 (zero-G), and the test's own header says 1b is a p3 == 2 level.
Re-read each address yourself before changing anything; if the decompile says something else, report it.

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

**Branch:** `s/audit-fix-abseil-zerog`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
