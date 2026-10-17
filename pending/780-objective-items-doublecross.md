## Your task: `objective-items-doublecross`, objective items can't be picked up in Double-Cross (playtest 1017)

User: "cant grab objective items either. look at f12." Two F12 spots:
- 07000003 `--bond 5.90,5.97,-42.41,1.710,-0.458` frame 1696: an item (a card/disc) on the floor of a glass display case.
- 07000001 `--bond -0.34,6.47,-6.22,6.260,-0.520` frame 2816: a crossbow-like object standing on the floor.
For each: what placement it is (kind/params [D]), how the game lets Bond take it (Pickup / objective item path, any glass or
switch that must open first [M]), and why the port refuses. Fix so the real-path walk takes each item and the objective
updates; tests per item with sisters. Also sweep 01-04 for every objective item and pin that each can be taken.

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

**Branch:** `s/objective-items-doublecross`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
