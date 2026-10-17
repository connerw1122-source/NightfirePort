## Your task: `level-05-complete-2`, finish the 05 completion (HELD at merge)

`s/level-05-complete` (task 705b, 32f6a18) was held at the 2026-10-17 batch: its player trigger reach
change (the toucher at `pos − 0.6` instead of `pos − 0.4` in drone.rs) breaks two tests that pass without it:
`fix_mission_live::objective_announces_on_its_enable_channel_and_completes_on_its_condition` ("no objective
notice before the enable channel 0x45 fires" — the 05 reveal plate #36 now fires on frame 1, so the test's
premise moved; decide from [L ee_start_05.bin] which is right and fix the TEST if the game's boot snapshot
already has ch 0x45 = 1, citing it) and
`npc_doors_live::the_escort_opens_his_door_without_bonds_help` ("the escort never entered DSTATE_OpenDoor at
the courtyard shoji" — a reach change on every level; find which plate Bond now touches on that walk and
whether the game fires it). Keep the reach evidence-backed on every level, not just 05; make both pass with
your own tests.

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

**Branch:** `s/level-05-complete-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-17T15:41Z

---
## Report (done, 2026-10-17T16:42Z)

