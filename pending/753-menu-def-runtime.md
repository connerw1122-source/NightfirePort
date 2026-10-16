## Your task: `menu-def-runtime` — drive the front end from the game's own menu-def data

The user: "the menus aren't consistent and look awful". Today the port's menus are hand-transcribed constants (layouts, colours, rows) in crates/nf-view/src/menu.rs. The game drives them from data:
- the kind-8 menu-def entry (one per level, 08000002 in 07000048.bin; the format is documented at the top of menu.rs);
- the uncited `Script_*` keyframe runtime: KFRAME/KMSG animation of controls, fades, slides [M — the audit's gap 8, `docs/systems/uncited-audit.md` §8].

Port the parser for the whole menu-def (pages, controls, components, sprites, keyframes, messages) and the keyframe runtime as a pure data/sim layer: given a page id and a tick, produce the list of what each control draws (position, size, colour/alpha, texture, text label) exactly as the game would at that tick. Then add a comparison test: for every page the port transcribed, the runtime's tick-0 and settled draw lists equal (or list every difference from) the transcribed constants. Do NOT change the drawing code; the main agent wires the runtime into the renderer and compares with PCSX2. Report every page where the transcription disagrees with the data.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/menu-def-runtime`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
