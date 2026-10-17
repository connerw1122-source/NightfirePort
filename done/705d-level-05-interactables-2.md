## Your task: `level-05-interactables-2`, finish the 05 interactables (HELD at merge)

`s/level-05-interactables` (task 705a, 41f5ae1) was held at the 2026-10-17 batch: merged onto the main
agent's master it breaks three tests that pass without it:
`searchlight_live::channels_and_tracking_behave` (the re-lit dark head's spot never leaves −10000: "the
spot moved to its ray hit: [-10057.8, -9963.4, -10059.7]" — the new own-housing skip in
`Searchlights::update` pushes the ray start past its end; note `dir_unit` assumes a 100-unit ray),
`fix_finale_live::the_07000016_chasers_defeat_completes_the_final_mission` and
`spurious_complete_16_live::the_real_end_condition_posts_the_mission_complete_banner`.

The two 16 failures come from the hurt-volume param map, which is right [M Hurt_Create 0x17e870: 0 type,
1 damage, 2 gate, 3 sound, 4 mask]: 07000016's #309 "HurtPoly-ShuttleLaunch" (params (0,2),(1,1000),(2,243),
(3,65535),(4,1); box x −30.7..35.3, y −24.1..−21.3, z −197.3..−133.8 [D]) now deals 1000 and kills Bond on the
silo floor (3.57, −23.18, −179.55) during the chaser fight, so the mission never reaches state 6. Find what
raises ch 243 in the port and when (no placement writes it [D], so a script event — the shuttle launch),
compare with the game (when is 243 up relative to ch 0x90 / ch 100 / Mission state 6 [M 0x182c60]), and fix
the real divergence. Do not zero the damage.

**Read first: `docs/systems/hurt-volumes.md`** (from `s/level-06-interactables`, task 706a — on origin/master). It traces `Hurt_Update` 0x17e5d0's start law [M]: param 4 is a WHO-mask over the
contact classes {8, 0x10, 2}; who 1 with no gate never starts (05's #328–330/#334–340 are configured-inert,
only #331–333 with who 0 are live); a gated volume starts when its channel rises and the channel VALUE is
the mask. Port that state machine (and the contact gate it implies) rather than a plain "inside → damage"
test: it decides whether 16's #309 (gate 0xf3) can hurt during the chaser fight at all.

**Two hurt systems.** origin/master already has `crates/nf-game/src/hurt.rs` on `Sim.hurts`
(scene.rs, with hurt_02_live, and the who-mask start law). The held merge kept that ONE
system with your param map and your capsule-vs-mesh contact, and dropped the copy on `Drones`. Build on that module;
do not add a second one.

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

**Branch:** `s/level-05-interactables-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-17T14:51Z

---
## Report (done, 2026-10-17T16:54Z)

Closed by the driver: the report is on `s/level-05-interactables-2` (docs/reports/level-05-interactables-2.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-17T16:54Z)

