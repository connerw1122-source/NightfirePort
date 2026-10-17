The user (2026-10-17): "09 level elevator buttons dont work. make sure we're prioritizing level work so we can actually complete each level completely and get all models and functions per level working" and "I want EVERYTHING covered. ALL ON FOOT LEVELS". Level work now runs ahead of everything else in the queue.

**Reproduce first, through the REAL path.** Load the level with `nf_view::load` (or `load_weapons` when a
level's own weapon is needed), move Bond with `Sim::step_bond` inputs (walking, turning, the use key, the
watch gadgets, the weapons), let the level's own scripts and triggers run, and observe. No teleporting past
a problem unless the task says so; a teleport to reach a spot is fine when the test then operates the
thing through the real input. Cite the decompile [M], the disc [D] or a fresh-boot PCSX2 value [L] for every
game claim; tests pin only proven behaviour.

## Your task: `elevator-buttons-09`, the 07000009 elevator buttons and the lift

The user: "09 level elevator buttons dont work". On 07000009 (Night of the Agent) the lobby's main lift is
the exit (docs/levels/07000009.md "The lift"): objective 2 "Get to the lobby and activate the main
elevator system" (ch 0x46 = AND(ch 0x39); ch 0x39 = SCRIPTPLAYER_Elevators #70's out), the two 0xE2
SimpleScripts LIFTDOORS_LEFT/RIGHT_ENDLEVELDOORS waiting on ch 0x39, the TOUCH_ONCE #67 inside the cab
(ch 0x32) and LOAD_LEVEL #63 to 0700000a.

1. Find every button, panel, switch or use-point on 09 that has anything to do with the elevators (the
   call buttons at the lift, the elevator system activation, any keypad or panel in the security room or
   lobby) from the disc: placements, their types, params, channels, and what the game's update/operate code
   does with them [M]. Document them in docs/levels/07000009.md.
2. Drive Bond to each through the real path and press use (or whatever the game's operate path is). Make
   each do exactly what the game does: the activation chain to ch 0x39, the doors opening with their
   animation and sound, the cab ride script 0x0600059B, the touch into the cab, the load into 0700000a with
   objective 2 complete.
3. A live test that walks from the level start to the lift, operates the buttons, rides, and lands in
   0700000a with objective 2 done. Sister: without operating the buttons the doors stay shut.

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

**Branch:** `s/elevator-buttons-09`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-17T10:41Z
