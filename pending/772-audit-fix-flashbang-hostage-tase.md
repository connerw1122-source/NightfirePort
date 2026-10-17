## Your task: `audit-fix-flashbang-hostage-tase`, guard-reaction tests that contradict the decompile (daily audit 2026-10-17)

1. `guard_drops_flashbang_live.rs` 247/260, 250/261, 267-271, 397-399, 404-405 assert the yard trio is stunned, disarmed
   and drops guns. The flash reaches drones only as msg 0x18 (0x12d890:362); WaitSwitch accepts only 0xe (0x15b960:40-42)
   and the Global fallback only 0x1d (0x15b830:59-61). The trio sits in WaitSwitch as loaded and these tests do not prime
   them — in the game the flash does nothing to them. Also :364-368 runs 120 ticks against a 210-tick fuse (the frag never
   explodes — a vacuous check).
2. `hostage_save_07000002_live.rs:93` accepts HOSTAGE_IDLE (0x0d) — `HostageSaved` 0x15ccf0:37-43 goes to state 100 or
   0xE when +0x135 != 0; 0x0d is never reached there, and the direct 0xE arm is wrongly rejected.
3. `tase_scream_live.rs:114` asserts the held beam always re-fells — `Stunned` 0x162eb0 case 7 re-fells only when the
   +0xCD8 anim is not 0x32, a `Rand_Rand(2)` pick (0x162b98:44-49); the test passes on the port seed's roll. Pin both rolls.
4. Comment-level but likely a real bug: blast_smoke:8-9 — Idle and Patrol pass allow = 1 for msg 9 (0x15bf18:100-110);
   check the port's `impact_args` for msg 9 (surviving guards may skip the 0x54 stagger).
Fix the port where it follows the wrong rule; re-read every address yourself first.

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

**Branch:** `s/audit-fix-flashbang-hostage-tase`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
