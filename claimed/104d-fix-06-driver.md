## Your task: `06-driver`, 07000006: the friendly truck driver

The user, playtest 1011-1535 (06): the friendly driver "does get scared, but because of where he's standing, he like
crouches on top of the boxes that he's next to... Also, you can still shoot him, and he like flies back like a guard
would with the same guard death sound." Check against the game: (1) the scared/cower state's position and anim for a
civilian ally standing by props (does the game move him, or crouch in place on the floor? the cower anim's root and
the floor snap); (2) whether the driver is damageable at all in the game (an invulnerable/ally flag on the drone, the
team check in the bullet hit), and if he is, his own death reaction and sound (not the guard's). Port the differences.
Tests (NF_ISO): the driver's scared pose keeps him on the floor at the game's position; the shot result matches the
game (no damage, or his own death set); sister assertions.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report; a test on the drawn set alone is not enough.

Files: `crates/nf-game/src/drone.rs` (small, local).

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

**Branch:** `s/06-driver`.

---
claimed-by: sandbox6 at 2026-10-14T16:40Z
