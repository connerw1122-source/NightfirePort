**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `ladder-underwater-02`, the underwater ladder on 07000002 softlocks the game

The user, 2026-10-16 evening, for the THIRD time: "ladder on 02 underwater is fucked and softlocks the game. I already
told you about this." Three merged tasks touched it (109i/109i2 fix-ladder-water: the re-latch climbing down into
the pond; 691 ladder-02: camera, weapon re-equip, infinite latch) — read their reports
(`git show origin/queue:done/691-ladder-02.md`, `done/109i2-fix-ladder-water2.md`, docs/reports/*) and find what they
did NOT cover. The ladder(s) that reach into or start under the water on 07000002: find them (ladder volumes,
`docs/levels/07000002.md`), and reproduce the softlock: approaching from underwater (swimming into the ladder),
climbing up out of the water, climbing down into it, letting go mid-water. Port the game's ladder/water interplay
from the decompile (Player_Ladder*, the swim state, the ladder grab/release tests) — cite every rule [M]. A softlock
means the player can no longer move/act: the test must assert Bond regains normal control and can reach the bank.

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

**Branch:** `s/ladder-underwater-02`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
