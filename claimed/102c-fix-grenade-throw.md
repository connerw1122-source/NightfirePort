## Your task: `grenade-throw`, grenades: the throw timing, the explosion size, the settle sound, smoke

The user's playtest (results 1013-1825, 07000015 and 07000005): "The grenade gets thrown out before the pin animation
finishes. It takes forever to throw another grenade, even after the explosion it still takes a few seconds. The explosion
is very small and not accurate to the game. You can hear the bounce, but once it settles it makes a rapid sound as if it's
rapidly contacting the ground." And: "Smoke grenade didn't work." Find in the game: the throw's release frame in the
grenade view-model script (the projectile spawns on an animation event, not at the press; fix-anim-events ported the
event machinery), the refire/re-draw time after a throw, the explosion effect's size (the effect record/scale used by
`Explosion_*` for 0x34) and its damage radius, the bounce sound's rule (one sound per impact above a speed threshold, none
while resting), and the smoke grenade 0x36's effect (smoke emitter, no damage). Fix and test each: the projectile spawns
on the event frame; the next throw is possible at the game's interval; the effect scale equals the record's; a resting
grenade plays 0 bounce sounds; a smoke grenade spawns its smoke and damages no one; sister assertions.

Files: `crates/nf-game/src/{grenade,explode,effect}.rs`, `crates/nf-game/src/gun.rs` (the throw timing).

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

**Branch:** `s/grenade-throw`.

---
claimed-by: sandbox1 at 2026-10-14T05:06Z
