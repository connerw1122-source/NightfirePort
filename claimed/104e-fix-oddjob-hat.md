## Your task: `oddjob-hat`, Oddjob's hat: a thrown one-shot kill, not a grenade

The user, playtest 1013-2313: "Oddjob's hat should be a one-shot kill but it acts like a grenade". Find the hat's
weapon record (the id, its +0x6C/+0x70 flags, damage, speed, the return-to-thrower behaviour) and the code path the
game runs for it (`Bullet_init`, `Bullet_CollisionHandler` 0x12ce80, any hat-specific update), and port it: thrown,
flies, kills what it hits in one hit, no blast. Tests (NF_ISO): the hat thrown at a guard kills him in one hit with no
explosion spawned; at a wall it does what the game does (bounce/return/drop); sister assertions.

Files: `crates/nf-game/src/{drone,gun}.rs` or wherever the projectile path lives (small, local).

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

**Branch:** `s/oddjob-hat`.

---
claimed-by: sandbox4 at 2026-10-14T16:47Z
