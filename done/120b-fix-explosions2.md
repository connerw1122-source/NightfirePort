## Your task: `explosions2`, find the frag grenade's real damage path, then land fix-explosions

Read `docs/reports/fix-explosions.md` on `s/fix-explosions` (88189da) first. Its flashbang stun, the blast sound
(0x3E from the 0x6000052 script) and the 12-case `Explode_Propagate` port are good work. The main agent did NOT
merge it for one reason:

**It makes the frag grenade harmless.** It reads weapon 0x35's record as radius 10 / damage 0, so the blast hurts
nobody, and it re-locked `fix_player3_live`'s grenade test to assert "the frag blast damages nobody". In the real
game a frag grenade thrown next to a guard kills him. That's ordinary play, and the user's original complaint was
the kill happening *on impact* and the grenade *bouncing back* onto Bond, not that frags kill at all. So the
frag's lethal damage comes from a path this reading misses. Candidates to check in the decompile, not guess:
- the explosion script itself (0x6000052 or the frag's own script): does it carry a hurt/damage command, or start
  an object whose collision handler damages?
- whether 0x35 is really the frag (the weapon id → record mapping; the frag's weapon id as Bond holds it vs the
  projectile's), and whether +0x08/+0x0C are read from the thrower's weapon_data or the projectile's;
- a second damage call in `Bullet_handle_object_destruction` 0x12cd98, `Bullet_DoTrails` 0x12d890's death
  branch, or `Explode_CollisionHandler` 0x17cde0 for the frag's flag bits (`weapon_data+0x70`);
- the per-level/difficulty damage scaling applied after Propagate.

Show the answer with evidence ([M] the call chain, [L] a live record or trace if available) and make the port's
frag kill a guard next to the blast the way the game does, with the real falloff. Keep the fix-explosions fixes
that are right (no on-impact kill, the blast sound, the flashbang stun with no damage, the mine/rocket 8/100). If
you can't find the path, say so in the report and keep master's current frag damage (don't zero it). Then bring
the branch up to `origin/master` and run the full suite.

Write a live-check request for the main agent: frag next to a guard (kill or not, and at what distance), and Bond's
own damage from his own frag at 2/5/8 units.

Files: as fix-explosions (`crates/nf-game/src/{explode,drone,bullet}.rs`, `crates/nf-view/src/scene.rs`,
`crates/nf-view/tests/{explode_live,fix_player3_live}.rs`, `docs/port-map/guard-ai.md`).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/explosions2` (start from `s/fix-explosions`, then merge `origin/master` into it).

---
claimed-by: sandbox1 at 2026-10-13T12:41Z

---
## Report (done, 2026-10-13T14:16Z)

