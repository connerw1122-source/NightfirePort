## Your task: `doublecross-transitions`, every Double-Cross level transition is missing (playtest 1017)

User, playing Double-Cross (07000001 onward): "all of the doublecross level transitions are missing." For each level exit in
the Double-Cross chain (find the chain from the mission table / each level's LOAD_LEVEL exits and the next level's start
[D]): what the game plays between levels — the in-engine exit cutscene (script/NIS), the movie, the loading screen, the
next level's intro — and what the port does instead. Port what is missing on the real path (exit trigger → sequence →
next level start), with a test per transition that the sequence runs (sister: a level that has none keeps its straight
load). Report each transition's evidence ([M] the exit path, [D] the scripts/movies).

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

**Branch:** `s/doublecross-transitions`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
