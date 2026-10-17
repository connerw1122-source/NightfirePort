## Your task: `floating-rifle-02`, a small rifle shape floats over the 02 bathhouse wall (user F12 1017)

`nf-view 07000002 --skip-cutscenes --bond -43.11,6.46,10.37,2.305,0 --hold-npcs --frame 300`: a small dark rifle-shaped
sprite/mesh (with a red highlight) is drawn over the wall right of the bathhouse doorway, around screen (1020, 480) of 1280x720;
also in the user's F12 005-07000002 (frame 4206). PCSX2 at the same spot shows nothing there. The main agent ruled out: Bond's
view model (the only attachments in view), weapon placements near the spot, and every main-pass material (NF_SKIP_MAT over
0..4000 never removes it) — so it comes from a separate draw: impacts/decals, a drop, a projectile/dart, an effect, or a
pickup pass. Identify the object (which list/pool, which entry, its source — e.g. a stuck dart or a decal spawned at load),
compare with the game's rule [M], and remove the divergence; test that the object is not created/drawn at that spot.

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

**Branch:** `s/floating-rifle-02`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
