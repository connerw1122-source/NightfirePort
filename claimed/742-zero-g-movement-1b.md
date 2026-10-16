## Your task: `zero-g-movement-1b` — Bond's zero-G movement on 0700001b

Playtest 1016-1355 (the user, on master 2026-10-16): "we're in space ... there's a different movement system that you're missing". Port Bond's zero-G substate: `Player_ZeroG` 0x192ab0 (dispatch cases 8/9), the thruster controls, inertia, the auto-stabilisers ('Press Action to activate spacesuit auto-stabilizers') and the camera rules [M]. The level already starts in zero-g gravity (p3 == 2). Tests: movement integrates the game's way (drift with inertia; the stabiliser damps it); sister: on-foot levels are unchanged.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/zero-g-movement-1b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T23:42Z
