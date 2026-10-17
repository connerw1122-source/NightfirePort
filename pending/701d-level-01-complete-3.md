## Your task: `level-01-complete-3`, finish the 01 full walk (HELD at merge again)

`s/level-01-complete-2` (701c, a141125d — its 53-line `crates/nf-game/src/drone.rs` change) was held: merged onto origin/master
(d56aceb6) it breaks
`escort_follow_07000001_live::escort_at_walking_speed_keeps_the_games_distance_and_the_exit_opens` (line 289: "the AND plate
must fire with Bond AND the contact inside [M Trigger_Hit 0x1b1fc8]"), which passes on master (checked both ways). Your
walk test passes; the escort-at-walking-speed path now leaves the contact outside the AND plate. Merge the newest
origin/master, find which part of the drone.rs change moves the contact (follow distance / stop rule / the plate approach),
keep it to what the game does [M], and make both the escort test and your walk pass.

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

**Branch:** `s/level-01-complete-3`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
