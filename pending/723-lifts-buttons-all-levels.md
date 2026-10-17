## Your task: `lifts-buttons-all-levels`, every lift, elevator, call button and floor panel on every on-foot level

Sweep all on-foot levels (07000001-0d, 11-16, 1b, 41, 4a) for lifts, elevators, platforms, cab doors and
every button/panel/keypad that operates them [D placements + M their create/update/operate functions].
For each level list them (docs/systems/lifts.md, one table per level) and make each work through the use
key and the game's own channel chain: the doors' SimpleScripts, the cab ride scripts, the sounds, the
floor-to-floor moves, Bond riding the cab (standing on a moving platform). One live test per distinct
mechanism. The 09 lobby lift (task 700), the 0a cab doors (walkreal_0700000a's ignore) and the 12/14/4a
elevators are known broken or unverified.

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

**Branch:** `s/lifts-buttons-all-levels`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
