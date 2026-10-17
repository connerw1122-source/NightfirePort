## Your task: `winch-sweep-0c`, a shot winch moves one tick and stops (main-agent render check)

level-0c-complete (712b) made the 0700000c bullet-driven pulley winches (mask-0x2 SSs #351/#364/#377) respond to a shot
[M SS_Operate 0x1a9df8:12-45]. Render check (the test's own fire path, winch ss[4] / placement 351 at (5.92, 14.50, −115.90)):
after the hit `t` = 0.04 and it stays 0.04 for 120 ticks — the state stays 0x40 and nothing visibly moves; the test only
asserts `t > 0`. Establish what the game does after a hit [M SS_Operate / SS_Update 0x1a9948: does one contact start the
full sweep (state change, play to the end key, the channel/sound), or advance only while hit?] and port it; strengthen the
test to the game's full outcome (end key reached / the drop's effect) with the sister (no shot → rests). The main agent will
compare a render with PCSX2.

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

**Branch:** `s/winch-sweep-0c`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
