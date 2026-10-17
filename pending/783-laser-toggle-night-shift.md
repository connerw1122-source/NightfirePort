## Your task: `laser-toggle-night-shift`, switching the lasers off doesn't work (playtest 1017, repeat)

User: "toggling lasers off on the nightfire level doesnt work" (Night Shift — 07000009's lobby grid: Switch #71
"SCRIPTPLAYER_Lasers" (0x29, ch 0x37, animation 0x06000627) and the SCRIPT_Lasers_ON/OFF/AMBER art; 0700000a's
keyboard lasers: Switch #643 "SCRIPTPLAYER_KeyboardLasers" (ch 0x3B, gate 0x3C) [docs/levels]). docs/levels/07000009.md
notes the switch's position is scripted, so the port's use ray cannot target it. This is a repeat report: reproduce on the
REAL path (walk + E, not a channel poke), find how the game's player reaches the switch (the use pick vs the scripted
object position [M Player_Activate 0x1a5170 / Switch_Activate 0x1b02a8]), fix it, and test that the real-path press turns
the beams off (and their collision/damage), sister: the gate channel blocks it. Note origin/master just gained the
channel-persistence fix (699) — rebase on it first.

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

**Branch:** `s/laser-toggle-night-shift`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
