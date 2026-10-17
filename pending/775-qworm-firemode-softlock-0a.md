## Your task: `qworm-firemode-softlock-0a`, the Q-Worm's fire-mode press soft-locks the weapons (playtest 1017-1139)

User, 0700000a (reached by riding the 09 lift): "I pressed the change fire rate on the Qworm gadget and now I'm soft
locked with my weapon and can't really switch weapons." Reproduce on the real path (the Q-Worm out, the fire-mode /
alt-fire input the port maps to the game's mode toggle — `Player_Weapon` 0x1a24c8's mode arm, the watch pair's 0x54/0x55
toggle is the precedent), find what the game does for a gadget with no second mode (the Q-Worm's weapon record [D]), and
fix the stuck switch state. Test: mode press on the Q-Worm, then switch weapons — the switch completes; sister: a weapon
with a real second mode still toggles.

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

**Branch:** `s/qworm-firemode-softlock-0a`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
