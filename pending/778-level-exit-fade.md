## Your task: `level-exit-fade`, the level end cuts instead of fading (playtest 1017-1139)

User, riding the 09 lift out: "level end doesn't match how the game does it. Music fades to black. This one just cuts
with a little bit of lag." Port the game's level-exit sequence: what `LOAD_LEVEL` (Trigger_LoadLevelCreate 0x1b2950 /
the ResetMap path) does between the trigger and the next level — the screen fade (duration, colour, curve) and the music
fade-out [M], and any hold before the load. Deliver the state + timings + a getter the renderer can draw (fade alpha per
tick) and the audio fade in the port's music layer; the main agent draws the overlay. Test: the exit runs the fade over the
game's tick count with the music volume ramp; sister: a death/mission-fail exit keeps its own path.

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

**Branch:** `s/level-exit-fade`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
