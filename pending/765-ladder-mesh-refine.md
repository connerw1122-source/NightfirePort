## Your task: `ladder-mesh-refine`, port the ladder pick's glist triangle refine

`nf-game/src/ladder.rs` stands in for the game's per-object pick refine (`Collide_Update` 0x1ea398:84-93 ->
`Collide_Intersect` 0x1ebaa8 over the object's glist / 0x2E collision triangles, plus the straddled-cel
scoping in `Collide_Pick` 0x1eb320:125-134/318-328) with a capsule-vs-bounds-box test (`in_mesh_box`). The
box is measured, but a box is not the rails-and-rungs mesh.

Port the real refine for class 0x2D objects: the HITTEST capsule (the substate arm's segment and radius, see
`capsule_hits*`) against the ladder's triangles, and the straddled-cel scoping. Document the functions in
`docs/atlas/ladder.md` first. Every existing ladder test must stay green, including
`ladder_underwater_02_live::hut_pocket_swim_out_passes_the_ladder_like_the_ps2` (PCSX2-pinned). Add tests
with the triangle distances at the spots those tests use.

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

**Branch:** `s/ladder-mesh-refine`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
