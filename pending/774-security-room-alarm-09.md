## Your task: `security-room-alarm-09`, 09's security-room guards never raise the alarm on Bond

Live, 2026-10-17 (fresh-boot PCSX2, 07000009): Bond placed at the elevator panel in the security room
(game pos (−18.1, −367.4, 26.1), facing the panel) — within a few seconds a guard raised the alarm: ch 0x61 = 1 and the
"Mission Fail / Alarm Raised" banner, then the Mission Failed menu (objective "Fail: Don't let the security guards raise
the alarm"). The port, same spot (`nf-view 07000009 --bond -18.3,-368.4,27.0,3.1416,-0.25`, no --hold-npcs), runs 900
ticks with no alarm and no fail. Find the security room's guards in the census (placement, start state, sight cone /
the alarm route — likely an alarm-point run to a panel or a direct alarm [M]), compare their detection of a stationary
Bond at the panel with the game's (`NDrone2_*` vision + the alarm path), and fix the divergence. Test: Bond at the
panel → alarm + mission fail within the game's time window; sister: Bond outside their sight keeps ch 0x61 off.
09 is a stealth level — the whole level's fail-on-sight must work.

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

**Branch:** `s/security-room-alarm-09`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
