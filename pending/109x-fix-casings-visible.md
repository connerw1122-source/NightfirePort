## Your task: `casings-visible`, shells and casings the player can actually see

The user (playtest 1015-0046): SG5 (`--give 0x16`, walk forward and fire): "they're big on my screen, but I also can't
really see them. It's kind of the whole point of showing the bullets flying out"; the Frinesi shotgun (`--give 0x1c`):
"I don't see any shotgun shells flying out". 109l moved the eject point to the gun bone. Render the port frame by frame
(`nf-view <iso> 07000005 --give 0x16 --draw 0x16 --pull 120 --frame 121..130 --shot`, same for 0x1c) and compare with
the real game (`docs/live/2026-10-14-weapon-measurements-pcsx2.md`, the SG5 casings section: small brass shells from the
right side of the receiver, up and to the left, gone off the top-left within ~2 frames). Check the casing mesh scale,
the eject velocity/direction through the gun bone, the delay (+0x78), and whether the shotgun ejects at all (its
pump script's eject frame). Fix every difference against the decompile (`Bullet_init_casing_ex` 0x12f678). Report the
before/after renders.

Files: `crates/nf-game/src/casing.rs`, `crates/nf-view/src/scene.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/casings-visible`.
