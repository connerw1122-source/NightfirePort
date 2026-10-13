# fix-explosions — the explosion entity, the per-type blast, the flashbang stun

Branch `s/fix-explosions`. The user's grenade retest after fix-player3: the thrown grenade kills a
guard on impact, bounces back and damages Bond, with a bounce sound but no explosion sound or
effect; the flashbang kills enemies it wasn't thrown at. All three symptoms traced to three port
bugs, each fixed against the decompile and the disc data.

## What was wrong, and the game's own answer

1. **The blast damage was hardcoded 100 for every grenade type.** The old reading took the
   `addiu $t2, $zero, 0x64` (100) at the fuse call site for the damage. The disassembly says
   otherwise: `Bullet_handle_object_destruction` 0x12cd98 loads `f12 = *(float*)(weapon_data+0x08)`
   and `f14 = *(float*)(weapon_data+0x0C)` (0x12ce1c/0x12ce20), `f13 = f12` in the delay slot, and
   `Explode_Create` 0x17cbd0's prologue maps f13 → the radius (`EXPLODEINFO+0x2C`, the shake amount,
   `obj+0xE8 = radius·0.06666667`) and f14 → the damage (`EXPLODEINFO+0x28`) that
   `Explode_CollisionHandler` 0x17cde0 feeds `Explode_Propagate` 0x17c518. The 100 lands in an u8
   argument slot `Explode_Create` never reads (its prologue touches only a0-a3, f12-f14, t3 and the
   stack u16). The disc records agree: frag 0x35 = radius 10 / **damage 0**, flashbang 0x36 = 0/0,
   mine 0x37 and the rocket 0x3A = 8/100, the missiles 0x3B+ = 8/2000, the launcher round 0x2B =
   6/75 [L weapon_data_live.bin]. So the game's own frag blast damages NOBODY through Propagate —
   the user's "kills on impact, bounces back and damages Bond" was the hardcode.

2. **The blast sound id was wrong.** The port played 0x4A7, which in `Bullet_CollisionHandler`
   0x12ce80:315-327 is the *rest-impact* sound of weapons 0x3B/0x45, not a blast. The blast sound is
   INSIDE the explosion script: 0x6000052 carries exactly one `Sound(0x3E)` command (stream 36)
   [D nf-extract dump of 07000001/07000005]. The script player already produces `Request::Sound` and
   the Sim drains it into `audio.play3d`, so once the wrong 0x4A7 push was removed the blast rides
   the script like the game's. (fix-hud's report predicted exactly this; confirmed and landed.)

3. **The flashbang ran the lethal damage path.** Weapon 0x36's record is radius 0 / damage 0, and
   `Explode_Propagate` early-returns on radius < 0.0002 [M 0x17c550] — the flashbang hurts nobody
   through the explosion. Its real work is `Bullet_DoTrails` 0x12d890's death branch gated on
   `weapon_data+0x70 & 0x2000` (set on 0x36 [L]): `Gas_Init(0.5, 1.68, …, fx 0x200001f, 0xB)`,
   `Camera_Shake(2.0, pos)`, `Sound_Play3D(0x16)`, `Light_Create(10.0, 5.0, white, 0, 10, 1, 0)`
   [M 0x12df58-0x12df8c], then a 30-unit sphere from the blast: Bond's head position feeds the
   dist (5→90 u) × facing × LOS falloff into `Player_SetFlashBang` [M 0x12e014-0x12e0b8], and every
   live drone within 20 u gets msg 0x18 → `DroneFunc_HandleImpact` case 0x18 →
   `Drone_SM_SetState(0x4D)` [M 0x12e0c0, 0x1469b8]. **No `HitDamage` anywhere on that path.**

## What landed

- **`crates/nf-game/src/explode.rs` (new)** — the explosion entity: `Explosion::create` per
  `Explode_Create` (lifetime = script? `SCRIPTINFO+0xAB0` : 15 [M 0x17cd38], radius, damage,
  attacker, debris kind, propagate latch), `Explosion::update` per `Explode_Update` 0x17c350 (the
  script's death ends the object; the debris gate is documented, the port has no debris system), and
  `explode::propagate` — the **full 12-case `Explode_Propagate` dispatch** (0 copter, 2 drone, 3
  player, 5 destructible models {0x2B, 0x34-0x37, 0x3A}, 0x20 break, 0x21 destroy, 0x28 vehicle,
  0x35 MP, 0x36 GT, 0x37 sensor, 0x38 monitor, 0x3D sub, 0x4B hit-list) over a `Targets` struct the
  Sim fills with the lists it has (drones, Bond; break/destroy lists wired for when the port grows
  them). The falloff carries the game's quirks: `dist − obj_radius` (the sphere at `obj+0x8C`) and
  the **integer truncation** of that distance before the division [M 0x17c61c-0x17c628], the 0.0002
  cutoff, and case 0x21's ≥ 1.0 smash gate [M 0x17c6a8]. Unit tests: the 15-tick lifetime, the
  truncation (5.7 − 1 → (int)4 → 60, not 57), inside-the-sphere full damage, radius-0 no-op, the
  smash gate.
- **`drone.rs`** — the explode arm: per-type damage/radius from the grenade's weapon record; the
  flashbang (0x36) branches to the DoTrails flash (sound 0x16, the whiteout through the new
  `PlayerHealth.flashbang`, `Light_Create(10, 5, white)`, `Gas_Init`, shake 2.0, the ≤ 20 u stun
  broadcast) instead of the damage path; the other types run `explode::propagate` (drones get the
  msg-9-equivalent damage application as before, Bond through `Player_Hurt` semantics); the wrong
  0x4A7 removed; `Drones::shake` set from the radius (≥ 0.5 gate [M 0x17cc28]) and decayed 10%/tick
  − 0.05 [*suggested* curve — the game's `Camera_Shake` ramp is view-side, not modelled].
- **The stun family** — states `STUN_GRENADE_IMPACT 0x4D / STUN_GRENADE_LOOP 0x4E /
  STUN_GRENADE_RECOVER 0x4F` (state-table indices [L NDrone2_StateFuncs 0x29b300+0x134]), ported
  from 0x172bb0/0x1631c0/0x163350: ENTER stuns (recover = `DroneFunc_RecoverTime` 0x147188 =
  36·(200 − +0xB9) = **7200 ticks** with the stats-default +0xB9 = 0 [M 0x14c600:90 + the port's
  07000005 stats-zero snapshot note], alert 0.66, vision off, anim 0x2D/0x2E by `Rand_Rand(2)` with
  follow-on Loop), Loop arms the timer and at msg 0xC goes to Recover when nobody is within 8.0
  else re-arms frame+300, Recover plays the get-up 0x76 with follow-on Attack. A bullet on a stunned
  guard (msg 8/9) → `Death_Anim` 0x44 [M all three case 8/9]. The guard's weapon drop
  (`DroneWeap_DropWeapon` + `ChangeWeapon(1)` when kind ≠ 4) is NOT modelled — the port's guards
  carry one fixed weapon; noted as remaining.
- **`scene.rs`** — the player's throw fills the grenade with the weapon record's own radius/damage
  (the same fix as the guards' `throw_grenade`), with the disassembly citation replacing the wrong
  "hardcoded 100" comment.
- **`fix_player3_live.rs`** — its "guard near the blast takes damage" assertion locked the bug; it
  now locks the game: the frag blast damages nobody, and the sister (a 0x37 mine pushed at the
  guard, 8/100 from the record) still hurts him through the same Propagate path.
- **`crates/nf-view/tests/explode_live.rs` (new, NF_ISO)** — through the real path: the frag thrown
  with a 12-tick fuse explodes, every guard keeps his health, Bond keeps 100, the 0x6000052 one-shot
  is playing (its Sound(0x3E) request is the blast sound); the mine blast hurts; the flashbang stuns
  the guard into 0x4D/0x4E with health untouched, sets Bond's whiteout and the 2.0 shake, and a
  direct msg-8 on the stunned guard drops him into 0x44.

## Verification

- `cargo test --offline --release -p nf-game --lib`: 106 passed (the new explode unit tests included).
- NF_ISO live: `explode_live` 2/2, `fix_player3_live` (updated grenade test) ✓, `models_misc_live`
  (the blast entities) ✓, `grenade_live`, `stun_grenade_live`, `fix_weapons_all_live`,
  `fix_drone3_live`, `fix_drone4_live`, `simplescript_live` ✓.
- Full four-crate suite (`-p nf-game -p nf-formats -p nf-audio -p nf-view`, `--test-threads=1`):
  nf-formats/nf-audio/nf-game green; nf-view 84 test binaries green; only the 3 `disc_golden`
  bigf tests fail (the slim-ISO allowance).

## Live checks for the main agent

1. **Frag blast, sound + visual**: `--bond 100,26,108,0,0` on 07000005, throw the frag (wheel to the
   grenade, fire-release). Expect: the bounce tick (0x51C), then at the fuse the blast **sound 0x3E**
   and the fireball script at the landing point, **no damage** to a guard standing next to it and no
   self-damage. (Old build: silent blast, 100-damage kill.)
2. **Flashbang stun**: `--bond 100,26,108,0,0`, force-stun scenario or a picked-up 0x36 thrown at a
   guard within 20 u. Expect: white flash on screen, sound 0x16, a white light at the burst, the
   guard crumples (anim 0x2D/0x2E), stays down ~2 min (recover 7200 ticks — the game's own number
   with stats-default +0xB9 = 0), no health loss; a shot on the downed guard kills into the normal
   death. Old build: the guard simply died from the "explosion".
3. **Camera shake**: stand ~10 u from a mine detonation — the view should wobble (Sim provides
   `drones.shake`; **the view does not consume it yet** — wiring the offset into the render camera is
   the remaining view-side half, listed below).
4. **Mine/rocket damage**: a mine (0x37) next to a guard kills with the linear falloff (100 at the
   centre, integer-truncated steps of 12.5 per unit at radius 8).

## What's left

- **Camera shake in the renderer**: `Drones::shake` is set and decayed Sim-side; the view never
  reads it. One consumption point in the nf-view camera (offset by a deterministic wobble × shake).
- **The whiteout render**: `health.flashbang` holds the strength; the HUD/white overlay draw is
  view-side (the game's `Camera_SetFade`).
- **The guards' weapon drop on stun** (`DroneWeap_DropWeapon` + `ChangeWeapon(1)`, kind ≠ 4): needs
  the gun-pickup spawn at the guard's position.
- **Explosive objects (barrels)**: the placement-driven barrels/breakables are not in the Sim's
  object lists yet, so Propagate's break/destroy cases (0x20/0x21/5) have no targets on the ported
  levels. The dispatch and gates are in `explode.rs`, tested at unit level; the placement types need
  the dynamic-object pass (parsemap_create_dynamic_objects 0x1d04f0) — the same lane as the
  SimpleScript 0xE2 work.
- **Grenade impacts vs actors**: the game's `Bullet_CollisionHandler` bounces grenades off drones
  (0x12ce80 `+0x70 & 0x100` arm) and zeroes the direct-hit damage for the grenade types 0x2B-0x40
  (`hitdata damage = 0`, Bullet_update tail 0x12efd8). The port's grenades collide with the world
  only; the guard-side bounce is not modelled (it affects trajectories, not damage).
- **SimpleScript 0xE2 activation/keyframes and the 0700000d C5-door laser sequence**: not started —
  out of this session's time. The `SS_Create/SS_Activate/SS_Operate/SS_Update` decompiles are read;
  the placement study stands in `simplescript_live.rs`.
- `DroneFunc_RecoverTime`'s per-weapon bases (0x35 → 1800, 0x43 → 3600, 0x44 → 7200, 0x4A/0x4C →
  600 frames) matter for the dart/taser weapons, not the flashbang; the +0xB9 stat read is folded
  into the 7200 constant until the port carries the behaviour stats blob.

## Commit map

- `explode.rs` + `lib.rs` — the entity and the 12-case Propagate.
- `drone.rs` — the explode arm, the flash branch, the stun family, per-type throw damage, shake.
- `bullet.rs` — `PlayerHealth.flashbang`.
- `scene.rs` — the player throw's per-type damage.
- `explode_live.rs` (new), `fix_player3_live.rs` (the grenade test re-locked to the game).
