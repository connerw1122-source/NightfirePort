## Your task: `zoe-gondola-08b`, finish zoe-gondola-08 (HELD at merge)

`s/zoe-gondola-08` (task 750d) was held at the 2026-10-16 batch merge: on top of the batch it breaks three tests
that pass without it (each fails alone, bisected to this merge):
- `nf-view` `cover_nodes_live::a_06_guard_under_fire_takes_a_game_cover_node` — "no guard entered the cover
  states under fire";
- `nf-view` `npc_doors_live::the_escort_opens_his_door_without_bonds_help` — "the escort never entered
  DSTATE_OpenDoor at the courtyard shoji";
- `nf-view` `rocket_origin_live::the_rocket_leaves_the_launcher_not_the_eye` — "aimed at the aiming point,
  missed by 4.28".
Its fix must be scoped to what the game does for Zoe (her team/target rule) without changing the other drones'
targeting, cover or door behaviour, nor Bond's aim. Start from `origin/s/zoe-gondola-08` merged with the newest
origin/master; make those three pass alongside your own test; never loosen a decompile-backed test.

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

**Branch:** `s/zoe-gondola-08b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T04:33Z
