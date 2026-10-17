## Your task: `level-01-complete-2`, finish the 01 full walk (HELD at merge)

`s/level-01-complete` (701b, 9e842b6 — the task ended "blocked" with an empty report, but the branch has the
meet-plate chain, the hall event and a report) was held: merged onto origin/master it breaks
`walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` (walkreal_07000001_live.rs:308 "the
contact must walk his route to the LoadPointActivator plate"), which passes without it (bisected). Merge the newest
origin/master, find which of your changes stalls the contact's route (the report mentions an escort stall and a door
wedge), fix it with evidence, and make that walk pass alongside your own tests.

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

**Branch:** `s/level-01-complete-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-17T17:39Z
