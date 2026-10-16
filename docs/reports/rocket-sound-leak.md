# rocket-sound-leak — the guided rocket's launch whoosh never stopped

Playtest 1015-1738: after a guided (Sentinel) rocket finished, "I can still hear a sound being
played from where I was standing when I shot the rocket, and it doesn't stop, and it can stack up
the more times I shoot rockets."

## What was wrong

The launch whoosh. `Bullet_init` 0x12c5a0:277-282 [M] plays sound 0x10e for every weapon whose
ammo-type byte (`weapon_data+0x90`) is 0x0e and stores the play's handle at `BU+0xF0`
(`pfVar3[0x3c]` where `pfVar3 = *(obj+0xe0)`); 0x10e is a LOOPED id ([L] ACTION.ELF
`SFXOutputData` @0x2c94a0, entry 0x10e: looped 1, inner 5.0 / outer 30.0 / weight 40.0), so the EE
`DYNAMICSOUNDS` pool frees it only on `Sound_Stop`. The port's `fire_player` played it
fire-and-forget (`self.audio.play3d(0x10e, from, None)`), nothing ever stopped it, and each shot
allocated a fresh pool node — one looping whoosh stuck at the muzzle per rocket, stacking.

The game's full lifecycle [M]:

- `Bullet_init` 0x12c5a0:269-290 — the LAST launch play's handle lands at `BU+0xF0` (the
  caller-passed fire-sound id first — every `Bullet_init` caller passes 0xffff/-1, so none —
  then the ammo-0x0e whoosh 0x10e overwrites, then the `weapon_data+2 == 0x45` arm's 0x5ef).
- `Bullet_update` 0x12e7b0:98-108 — the head of every tick, before the state machine: while the
  handle is non-zero, `Sound_IsFinnished(handle)` clears it when the driver finished the sound,
  else `Sound_SetPosition(handle, obj)` moves the voice with the rocket (the whoosh FOLLOWS the
  rocket; that is why the game never leaves it behind).
- `Bullet_update` case 3 (0x12ed68) — `Sound_Stop(handle); BU+0xF0 = 0` when the bullet enters
  the stuck/lingering state.
- `Bullet_Delete` 0x12e660:13-16 — the object's delete pass: `if (BU+0xF0 != 0)
  { Sound_Stop(BU+0xF0); BU+0xF0 = 0; }` — disassembly checked (the decompiler drops the
  argument): 0x12e680 `lw $a0, 0xf0($s2)` → 0x12e68c `jal Sound_Stop` (0x1de0a8) → 0x12e694
  `sw $zero, 0xf0($s2)`. The guided camera's restore rides the same function (:22-31 —
  `BL+0x878 = 0`, `BL+0x944 = 0xf`), so "the rocket dies" and "the camera ends" are one event.

Which weapons carry the whoosh [L ee_start_05.bin, weapon_data @0x2bf150, the +0x90 sweep]: the
ammo-0x0e records 0x2c/0x2d (AT-420 Sentinel), 0x2e/0x2f (AT-600 Scorpion), 0x30/0x31 (Torpedo
Launcher) and 0x6d. All single-pellet, so one `Sound_Play3D` per shot. The MGL 0x42 is ammo 0x13 —
no whoosh; the grenades 0x34–0x40 are other ammo types — no whoosh.

## The fix (branch s/rocket-sound-leak)

`crates/nf-game/src/drone.rs`:

- `PlayerBullet.snd: crate::sound::Handle` (BU+0xF0; own line at the end of the struct, merge
  lesson 4). `fire_player` keeps the 0x10e play's handle (`let whoosh = if w[0x90] == 0x0e
  { self.audio.play3d(0x10e, from, None) } else { 0 }`) and stamps it on every spawned bullet.
- `update_player_bullets`'s per-bullet head (before the range-out check, the game's order): the
  finished-check + `Sounds::set_position(b.snd, b.pos)` — the whoosh follows the rocket.
- The stop moved to the REAL delete pass: since origin/master's fix-gas-rng4 merge the dead-bullet
  retain lives in `flush_world_impacts` (a deferred ricochet/pierce can REVIVE a bullet flagged
  dead, so stopping at the flag would kill a whoosh whose rocket lived on — the game's
  `Bullet_Delete` never ran for a bullet that survived). The stop runs for the bullets still dead
  at the flush's retain: `for b in … .filter(|b| b.dead) { if b.snd != 0 {
  self.audio.stop(b.snd); } }`.
- The drone grenade throw no longer plays the looped 0x5ef fire-and-forget (a per-throw stuck
  loop — the same leak family). The game plays NO throw sound: `DroneWeap_ThrowGrenade` 0x179558
  has no `Sound_Play3D` call at all, and `DroneWeap_FireWeapon` 0x178f68:275 explicitly skips the
  0x5ef id; `Bullet_init`'s own 0x5ef arm belongs to the Oddjob hat record (+2 == 0x45), whose
  handle `Bullet_Delete` stops.

Support hooks: `Sounds::live_ids()` (nf-game/src/sound.rs — the ids of all live pool nodes,
duplicates counted) and `Driver::item_pos(id)` (nf-audio/src/sfx.rs — the item's tracked 3-D
position). Both test-only readers.

## Verification

- `crates/nf-view/tests/fix_rocket_sound_leak_live.rs::three_guided_rockets_leave_no_looping_
  whoosh_behind` — the headless repro through the real path (`load_dev("07000005.bin")` → the
  Sentinel 0x2c given → draw via `step_bond` → fire → `nf_view::sound_tick` → the nf-audio
  driver), 3 rockets one per guided ride:
  - during each flight exactly ONE live 0x10e item in the driver and one node in the EE pool
    (pre-fix this count stacked k after k rockets);
  - the item's tracked position moves every tick and stays on the bullet
    (`Driver::item_pos` vs `sim.drones.player_bullets[0].pos`) [M 0x12e7b0:104];
  - after each rocket's death (ride over + the 15-tick restore) and after all three: ZERO live
    0x10e items in the driver and ZERO nodes in the EE pool — the playtest symptom, asserted
    numerically.
  - RED→GREEN: with the fix stashed the same test fails immediately at "the whoosh never moved …
    from where I was standing" (the playtest's first line), then at the post-death leftovers.
- The new test is green on the final merged tree; `cargo check --release` clean for nf-game and
  nf-audio; all nf-view test targets compile (the two pre-existing `PlayerBullet` literals got
  `snd: 0`).
- Full suite: `tools/test/suite.py` on the final tree merged with origin/master — the total line
  is in the driver's queue message; on this sandbox's slim ISO only the 3 `disc_golden` tests may
  fail.

## The audit — every other projectile/object loop

Per the task: grenades rolling, the MGL, thrown mines. Method: sweep every sound id the
projectile/object paths play against `SFXOutputData`'s looped flag [L], and every handle the game
stops against the port's coverage.

| Path | Sounds | Loop? [L] | Port status |
|---|---|---|---|
| Rocket family 0x2c–0x31, 0x6d (ammo 0x0e) | launch whoosh 0x10e | **looped** | FIXED (above) |
| Grenade bounce (0x34–0x40, MGL 0x2a/0x2b rounds) | 0x51c (even bounces < 0xF), 0x13/0x15 (smoke/flash first bounce) | no | one-shots, reap themselves — no leak |
| Grenade deaths | 0x16 (flash), 0x14 (smoke pop), the 0x6000052 blast's own script sounds | no | no leak |
| MGL 0x42 | fire 0x41b / 0x5c9 (ammo 0x13 — no whoosh [L]) | no | no leak |
| Thrown mine 0x37 | the stick arm plays NOTHING [M 0x12ce80:296-330] | — | no leak |
| Tripbomb 0x3a | stick 0x446, laser arm 0x27c | no | no leak |
| Satchel 0x3b | stick 0x4a7, countdown beeps 0x447 | no | no leak |
| Oddjob hat 0x45 | flight loop 0x5ef (looped [L]; `Bullet_init`'s +2==0x45 arm, stopped by `Bullet_Delete`) | **looped** | the port never STARTS it — a coverage gap, not a leak |
| Turret GunImp | charge loop 0x5e5 (looped [L]; handle at `+0x98`, stopped by `GunImp_Deactivate` 0x132c18:62 [M]) | **looped** | the port models neither the start nor the stop — coverage gap, not a leak |
| Car | the `CAR_INFO+0x108` handle stopped by `Car_Deactivate` 0x131d88:32 [M] | (game-side loop) | the port's driving module starts no car loop — no leak |
| SpaceMissile (0x1a9ef0/0x1aa118) | no `Sound_Play3D` at all [M] | — | no leak |
| Copter FireMissile (0x17d9f0) | via `Bullet_init` (param_7 = -1) | — | not ported yet (`copter.rs` is a 120-line stub); `Bullet_Delete`'s rule will cover it when it lands |
| Player_ChangeSubState 0x190630:83-85 | `Sound_StopAllWithId(0x12/0x3d/0xf0)` on the substate exits [M] | — | the port's Laser/Stunner loop stops ride the script ops that start them — unchanged |

## What's left

- Nothing for the leak itself. The two coverage gaps (the hat's 0x5ef flight loop, the turret's
  0x5e5 charge loop) are sound-coverage work for a future task — both need the game's start-site
  ported before a stop has anything to free.
- The `Player_ChangeSubState` substate-exit `Sound_StopAllWithId` triple (0x12/0x3d/0xf0) is not
  modelled as such in the port; the loops it guards are stopped by their own script ops today.
  Only worth revisiting if a loop starts outside a script.

## Live checks for the main agent

1. **The stop fires at the delete** (the fix's core claim): with `tools/trace/gdbwatch.py` on
   `Sound_Stop` (ACTION.ELF 0x1de0a8) during one guided Sentinel shot on 07000005: expect exactly
   one call shortly AFTER the rocket's death tick (from `Bullet_Delete` 0x12e68c), with `$a0` =
   the handle `Bullet_init`'s 0x10e play returned; a second call only if the shot's bullet takes a
   state-3 path. Then confirm audibly: the whoosh must die with the rocket, not keep looping at
   the launcher's position, and three quick rockets (ride → fire again ×3) must not stack.
2. **The whoosh follows the rocket**: gdbwatch on `Sound_SetPosition` (0x116118) — one call per
   frame while the rocket flies, its `a0` the same handle, position advancing with the rocket
   (predicted: the port's per-tick stand-in now matches; pre-fix the voice sat still).

Note: the port's audible-equivalent assertions are the numeric ones in
`fix_rocket_sound_leak_live.rs` (item counts and tracked positions); the render/audible check is
the main agent's.
