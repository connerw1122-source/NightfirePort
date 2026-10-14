## Your task: `samurai-fire`, the Phoenix Samurai fires nothing at an empty spot

weapon-matrix4 (docs/reports/weapon-matrix4.md, red #1): record 0x32 — the Phoenix Samurai, a LASER RIFLE with the
Overcharge 0x32 / Beam 0x33 modes (docs/live/2026-10-13-weapon-modes-pcsx2.md; not a sword) — draws, plays its fire
script and sounds, but nothing observable spawns on 16 of 18 levels (clean only where a target was in line). Find the
game's fire path for 0x32/0x33 (`Player_WeaponFiring` 0x1a5460, `Player_WeaponInitBullet` 0x1a4bf0, the charge for
Overcharge, the beam for Beam, any target gate like the stunner's no-target arm) and port what is missing: the
charged bolt must fly and hit walls (fix-fire-modes3 ported its impact blast), the beam must draw. Tests (NF_ISO): at
07000005's spawn, Overcharge fired at a wall spawns the bolt and its blast; Beam draws to the wall; sister assertions.

VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. The main agent's PCSX2 captures have a CRT filter on: use them for presence/placement/layout,
not exact colour or brightness.

Files: `crates/nf-game/src/{gun,drone}.rs`, the fire path in `crates/nf-view/src/scene.rs` (small, local).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/samurai-fire`.
