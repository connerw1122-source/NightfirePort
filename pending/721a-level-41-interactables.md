The user (2026-10-17): "09 level elevator buttons dont work. make sure we're prioritizing level work so we can actually complete each level completely and get all models and functions per level working" and "I want EVERYTHING covered. ALL ON FOOT LEVELS". Level work now runs ahead of everything else in the queue.

**Reproduce first, through the REAL path.** Load the level with `nf_view::load` (or `load_weapons` when a
level's own weapon is needed), move Bond with `Sim::step_bond` inputs (walking, turning, the use key, the
watch gadgets, the weapons), let the level's own scripts and triggers run, and observe. No teleporting past
a problem unless the task says so; a teleport to reach a spot is fine when the test then operates the
thing through the real input. Cite the decompile [M], the disc [D] or a fresh-boot PCSX2 value [L] for every
game claim; tests pin only proven behaviour.

## Your task: `level-41-interactables`, every interactive thing and every model on 07000041 (the space-station approach (scripted interlude))

1. **Census.** From the disc (an nf-formats parse of 07000041.bin), list every placement the player can
   touch, use, shoot or ride, by type, with its parameters and channel wiring: buttons and switches,
   lifts/elevators and their call buttons and doors, doors (locked, keyed, scripted, sliding, shoji),
   SimpleScripts (0xE2), ScriptPlayers, touch/use/load-level triggers, pickups and weapon racks, safes,
   keypads, monitors, terminals, computers, lasers and laser switches, breakable glass and destroyables,
   searchlights, cameras, turrets, fuseboxes, vents, hatches, ladders, wires, grapple points (watch gadget
   targets), vehicles. For each: what the game does with it [M], and the port's status. Put the table in
   docs/levels/07000041.md.
2. **Make each work.** Through the real input path, drive Bond to one instance of every KIND on this
   level and operate it the way a player does (use key, the right gadget, shooting, walking in). It must do
   what the game does: the channel writes, the animation and sound, the script, the item, the door. Fix every
   kind that does not. One live test per kind (not per placement), each asserting the game's effect, with a
   sister that fails without the operation.
3. **Every model draws.** Every placement's mesh/skin resolves on this level (no missing hash, no white
   fallback texture — `NF_TEX_TRACE=1` prints the fallbacks), every NPC has its skin, every pickup and
   script entity its model. List the misses in the report and fix them.

Known open items on this level (start here): it has no front-end entry; it must play through into 0700001b as the game does.

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

**Branch:** `s/level-41-interactables`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
