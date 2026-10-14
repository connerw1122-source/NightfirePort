## Your task: `handcannon`, weapon 0x38: fire shows nothing

weapon-matrix3's F family: record 0x38 fires on 22 of 27 levels with sounds recorded but no shot tick and no observable
event (no bullet, no blast, no muzzle flash). Find what 0x38 is (its label through the language bank, its +0x6C/+0x70
flags, ammo, `Player_WeaponFiring` 0x1a5460's path for its fire type) and what the game does when it fires; port the
missing part. Tests (NF_ISO): firing 0x38 at 07000005's spawn produces the game's event (bullet / projectile / effect,
numbers); sister assertion.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. Note: the main agent's PCSX2 captures have a CRT filter on; use them for presence and placement,
not exact colour or brightness.

Files: `crates/nf-game/src/{gun,drone}.rs` or scene.rs's fire path (small, local).

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

**Branch:** `s/handcannon`.
