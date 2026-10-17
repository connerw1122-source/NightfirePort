**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `create-save`, the player cannot create a save (codename)

The user, 2026-10-16: "you wont let me create a save". In the game, Select Codename → New Codename opens the name
entry (P_CNNAME 0x40000020, the letter grid C_KEYBOARD 0x213a98), the ✓ confirms, the codename is created and
saved to the memory card (the port's stand-in: the settings/save files under %APPDATA%/NightfirePort), and
progress then saves after each mission. Walk it through the REAL window input path (the keyboard keys the window
maps: arrows/Enter/Esc/Tab and typed letters — read lib.rs's key handling for the name page) on a fresh profile and
find where it fails: the page not opening, typing not reaching the grid, the ✓ refused, the create/save step
missing or erroring, or the new codename not listed / not selectable afterwards. Fix it to the game's flow with [M]
citations (codename-entry, save-progress docs) and pin it: create "BOND" from a fresh profile, see it saved to disk
and offered on the next boot's Select Codename; finishing a mission under it saves progress.

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

**Branch:** `s/create-save`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
