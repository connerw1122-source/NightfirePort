## Your task: `hurt-volumes-16b`, re-land the hurt volumes without killing Bond in the 16 finale

`s/hurt-volumes` (task 748) was dropped at the 1016c merge: see `docs/UNKNOWNS.md` U-051. Hurt volume 0 on
07000016 (placement 0xE4, type 2, damage 1000, gated on ch 243, box x -30.7..35.3, y -24.1..-21.3,
z -197.3..-133.8 [D]) kills Bond at the finale: with the launch chain driven (ch 0x81), ch 243 rises while
the port leaves Bond on the silo floor at (3.57, -23.18, -179.55) after the pack cinema, and the gate arm
[M Hurt_Update 0x17e5d0] kills him before the chaser's defeat (`fix_finale_live`,
`spurious_complete_16_live`).

From the decompile: where does the game put Bond when ch 243 rises (the pack cinema's end position / the
jetpack state), and which script raises ch 243 and when? Fix the port's side of that (not the hurt volume),
then re-land `origin/s/hurt-volumes` on the newest master with both 16 tests green. Answer U-051 in the
docs with [M] citations.

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

**Branch:** `s/hurt-volumes-16b`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
