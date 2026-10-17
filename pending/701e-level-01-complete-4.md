## Your task: `level-01-complete-4`, port the masked sight ray so the 01 walk AND the escort pass

Start from `origin/s/level-01-complete-3` (b9321d2) — the previous session (701d, blocked) bisected the hold and found the
fix path; read its report (queue `blocked/701d-level-01-complete-3.md`) first. Summary:
- The armed-peer scan hardening's head-to-head LOS gate is what moves the escort's contact off the AND plate #180; the
  port's `world.line_of_sight` is a raw wall ray, but the game's sight test (`DroneVision_CanSeeObjectFrom` 0x176f18:56-104)
  uses the bone-5 head (`DroneFunc_GetBonePos` + `ClampPosToCollisionCylinder`), the same-AI-cel / `AINetwork_TestRayCels`
  pre-test, then `Collide_LineOfSight` with **mask 0xa27** — the port's unmasked segment hits stairwell geometry the game
  ignores.
- Port the masked ray (a `segment_masked` honouring 0xa27's cel/material exclusions [M Collide_LineOfSight + Collide_Pick
  flag tests]) and use it in the peer scan; keep health>0 + ≤20 m; re-derive or drop the weapon gate per the disasm
  (NearArmedDrone 0x17752c-0x177578 accepts a NULL record or byte+0x62 == 0x47 only; NotNearArmedDrone 0x177640 keeps the
  remembered +0x2b8 peer without the sight test).
- Both must pass: `escort_follow_07000001_live::escort_at_walking_speed_keeps_the_games_distance_and_the_exit_opens` and
  `walkreal_07000001_live` (3/3). Run the FULL suite to a total line before pushing (the last session's window ended first).
The main agent will run the report's live checks (the 0xa27 call's result for the Mayhew→guard-2 pair, the AND plate
latches) and append them here if they land before you start.

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

**Branch:** `s/level-01-complete-4`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
