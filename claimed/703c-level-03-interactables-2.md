## Your task: `level-03-interactables-2`, re-land level-03-interactables on the newest master (HELD at merge)

`s/level-03-interactables` (task 703a) adds its own breakables system (`crate::breakable::Breakables`, the
`Drones::breakables` field), but master (via level-01/02-interactables) already has one in the same file:
`crate::breakable::Breaks` (`Drones::breaks`) plus `crate::monitor::Monitors` — both porting `Break_Create`
0x12c230 / `Break_Update` 0x12bfd8 / `Break_Kill` 0x12b970. The merge conflicts across the whole of
`crates/nf-game/src/breakable.rs` and the create pass in drone.rs. Merge `origin/s/level-03-interactables`
with the newest origin/master, keep ONE breakables system (master's `Breaks`, extended with anything your
version covers that it lacks — the bullet reject mask, the glass pierce, the per-kind sounds, the census),
move your level-03 tests and fixes onto it, and keep both branches' tests green. Note: master is behind the
local merge lane until pushes resume — fetch again before you start.

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

**Branch:** `s/level-03-interactables-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T16:44Z
