## Your task: `weapon-effects`, firing: muzzle flash, tracers, casings, impacts, projectiles per weapon

CONTEXT (the user, 2026-10-13): "need heavy work on getting weapons to work no matter what level, with sounds and
proper animations and particles." The dev tools (the dev window / F2 menu, `--give id,id`) hand Bond ANY weapon on ANY
level. Today `scene.rs` builds a first-person view model only when the weapon's first-person skin (`weapon_data+0xDC`),
its sleeve and its parts are in the CURRENT level's archive (`level.skins.get(&fp)`, scene.rs "The first-person gun
models"), and the same holds for its view-model animation scripts, its muzzle-flash mesh (`weapon+0x48`), its casing,
its projectile and its sounds (the level's own sound bank). So a weapon the level doesn't natively carry is invisible,
silent, unanimated or fires the wrong effects (the user saw the grapple fire a bullet impact, the laser and stunner fire
"bullets", no Q-Claw model). The real game only ever loads a level's own set; the port goes further on purpose, so that
every weapon behaves exactly as it does on a level that carries it.

The four 100x tasks split the work: 100a the assets and animations (the foundation), 100b the sounds, 100c the fire
behaviour and effects, 100d the all-weapons x all-levels check. They run in parallel: pull `master` often, and if
100a's branch (`s/weapon-assets`) isn't merged yet, build on it (`git merge origin/s/weapon-assets`) and say so in
the report. Keep the per-level behaviour bit-identical for weapons the level carries (the existing tests stay green).

YOUR PART: what each weapon DOES when fired, matched to the game per weapon class, on any level:
- guns: the muzzle flash (mesh, colour, light, duration), the tracer (which weapons have one, its speed and length),
  the casing (ejected only when firing: the user saw casings fly when walking backwards on 16), the bullet impact per
  surface material (decal, dust/sparks/water-splash particles, sound), the fire rate and spread from `weapon_data`;
- shotguns: the pellet count and spread;
- explosives: rockets (projectile model, smoke trail, flight sound, explosion size and damage: the user said "the
  explosion is very small"), grenades (thrown on the animation event, bounce, settle, fuse), mines, the crossbow bolt;
- gadgets: the laser watch (a beam, burns locks, no bullet), the stunner, the Q-Claw (the hook projectile and cable, no
  bullet), the micro-camera (no shot);
- the alt fire and twin variants (silencer, heat-seeker) and their effects.
Use the game's own code for each (`Player_WeaponFiring` 0x1a5460, `Player_MuzzleFlash`, the effect/particle system,
`Explosion_*`, the projectile classes); compare against PCSX2 where the docs have measured numbers. Tests per class on
07000005 through the real input path: one shot spawns exactly the game's set of effects (counts, ids, sizes and
lifetimes as numbers), nothing extra (no bullet impact from a gadget), nothing while walking; sister assertions. 101e
(laser), 101f (Q-Claw) and 102c (grenades) are queued too: merge their branches if they land first, don't duplicate.

Files: `crates/nf-game/src/{gun,bullet,casing,grenade,explode,effect,gadget}.rs`, `crates/nf-view/src/scene.rs` (draw
hooks), tests.

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

**Branch:** `s/weapon-effects`.
