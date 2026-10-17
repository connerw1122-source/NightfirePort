## Your task: `garage-elevator-buttons-09`, the car-park elevator buttons still don't work (playtest 1017, repeat)

User: "elevator buttons in the garage still dont work" (07000009's underground car park — not the lobby panel, which the
elevator-buttons-09 task fixed). docs/levels/07000009.md lists liftswitch_off/on and liftsymbols FuseBox art (0x31, no
params) in the car park. First establish on the REAL path what the game does there [M/D, plus the main agent's PCSX2 check]:
are those buttons interactive (which placement/class takes the use, what channel/script they drive — a lift call, a door,
a prompt) or set dressing? Then port exactly that, real-path test (walk to the button, E) with a sister. If the game's
buttons do nothing, write that down with the evidence and say so — don't invent behaviour. Rebase on origin/master's
channel-persistence fix (699) first.

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

**Branch:** `s/garage-elevator-buttons-09`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
