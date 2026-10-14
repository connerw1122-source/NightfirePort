## Your task: `satchel-tripbomb`, satchels and the laser tripbomb

The satchel family 0x3b-0x40 and the Laser Tripbomb 0x3a stick on contact (`+0x70 & 0x100` [L], like the hat and the
mine). Port their full behaviour from the game: the satchel's stick and its timer/remote (the family fuse
((id-0x3b)*5+5)*60 [M `Bullet_init` 0x12c5a0]), the tripbomb's beam (its laser line, the trigger when crossed by a guard
or Bond, the arm delay), their blast scripts, sounds and models while stuck. Tests (NF_ISO): a satchel thrown at a
wall sticks and blows at the game's tick; a tripbomb on a wall arms, draws its beam, and a guard walking through sets
it off; sister assertions.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. PCSX2 captures carry a CRT filter: use them for presence/placement, not exact colour.

Files: `crates/nf-game/src/{grenade,drone}.rs`, scene.rs's draw for the beam (small).

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

**Branch:** `s/satchel-tripbomb`.

---
claimed-by: sandbox7 at 2026-10-14T21:29Z

---
## Report (done, 2026-10-14T22:37Z)

Closed by the driver: the report is on `s/fix-satchel-tripbomb` (docs/reports/fix-satchel-tripbomb.md); the worker didn't run finish.sh.
