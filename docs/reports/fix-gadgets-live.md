# fix-gadgets-live — the laser's charge/beam/light, the stunner's economics, the AIMS-20's lights, the hat's contact kill

Branch `s/fix-gadgets-live` (from master a4164b9). The playtest (2026-10-13 night, after 101e/101f
merged) had four findings; all four are addressed. Every fix cites the decompile; the tests run the
real input path (`nf_view::load_weapons` + `Sim::step_bond`).

## 1. The Laser (0x4e): the beam, the wall light and the charge bar

**What the game does** [M decompile]: the whole visible shot rides the refire that spent a round —
`Player_RoundToFire` 0x1a18e0 spends `weapon_data+0x91` (= 1 [L record]) from the clip slot
`BLData+0x1B8 + w·0xC` (`Player_AmmoInGun` 0x1a9190), then `Player_WeaponInitBullet` 0x1a4bf0 runs on
the same tick; its +0x95b latch makes the next tick's `Player_CollisionHandler` 0x1949a8 call
`Player_MuzzleFlash` 0x1a4298, whose `Light_Create(radius +0x58 = 5.0, rgb +0x56/+0x55/+0x54 =
(64,10,10) — RED [L record])` is the wall glow and whose `Draw_LaserBeam` 0x1a4560 stretches the
`lazer` sprite (0x2000131 for the class-'N' 0x4E; `uv_lazer` 0x20006c1 for the 0x4F upgrade) from the
view model's bone 0 to the re-cast hit (`obj+0xE8` = the distance, the yaw/pitch in obj+0x50). The
HUD bar is `clip·100/clip_size` for ammo type 0x1A [M HUD_UpdateAmmoPane 0x19a7f0:177-183] — the
laser's clip is 500 [L +0x92], so the bar drains 1 % per 5 shots and the beam STOPS at 0 (the user's
31→0 capture ends there).

**What was broken**: the port's burn/beam block ran on the RAW trigger at the top of the tick —
not the shot tick — so it beamed through the draw/switch cycle, never stopped at clip 0 (the fire
gate is `RoundToFire`'s, which the block ignored), and the renderer stretched the
`GUN_BULLET_TRAIL` stand-in (the comment claimed the game's sprites were "unresolvable" [S] — wrong:
`lazer`/`uv_lazer`/`taser` resolve in every level archive [D probe]).

**The fix** (`scene.rs`): the target test + burn/beam compute in the `release` branch where the
spent refire is known — the clip gate falls out of `update_with`'s `RoundToFire`; at clip 0 nothing
fires and the beam stops (the probe: 500 held ticks → "beam stopped at +470, clip 0"). The muzzle
light now runs on every gadget shot tick (the stunner's no-target ticks included — the +0x95b latch
fires `MuzzleFlash` regardless of target [M 0x1a4bf0:193-199 sets it on the miss arm too]). The beam
draws the game's own sprites: three beam slots (`lazer`/`taser`/`uv_lazer`), picked by the active
weapon, anchored at the posed view model's root in first person (the game's bone-0 anchor
[M 0x1a4560:20]).

## 2. The Stunner (0x4a): "doesn't work"

The target test itself worked (101e's ray-sphere stand-in finds a guard at 3 units); what was wrong:
the spend/regen economics and the shot-tick gating. The game's model [M 0x1a4bf0:169-200]: a shot
tick with a class-2/3 target within `+0x1c` (5/10) spends 1 round and spawns the taser bullet; a miss
tick REGENS +0x91 and returns before `Bullet_init` — with `RoundToFire`'s spend the aimed-off-target
hold is FREE (that is the stunner's "regen"). The port spent on every fired tick with no refund and
kept the beam on raw trigger. Fixed: the fresh target test gates the spawn, the miss arm refunds
+1 (capped at the clip size 100 [L]), and the beam parks only on target. The blue arc light
(rgb (10,10,64) [L +0x54..0x56]) now flashes on the miss ticks too, as the game's light does.

**Left (documented, not ported)**: the guard's msg-7 STUN state machine (`NDrone2_DealWithObjHit`
case 5's taser arm — the port still routes the taser hit to the ordinary impact flinch; the
stun-grenade machinery (states 0x4D-0x4F) exists in `drone.rs` and is the model for it).

## 3. The AIMS-20 (0x1a): the side lights

The game attaches mesh **0x2000132** to the view model's bones 8..0xB every tick
(`AnimDatumSetEntity` [M `Player_WeaponFiring` 0x1a5460:256-360]) — the row of red lights the user
saw lit in PCSX2. The port never attached it. Fixed in `scene.rs`: for the records whose class
halfword is 0x1a/0x1b, four Effect-layer attachments (bones 8..0xB, mesh 0x2000132 — resolves in
every archive [D probe]) bind to the AIMS view model; the game's blink/off arms run in `frame_state`
off a new `Drones::aims_idle` (the port's stand-in for `BLData+0x93a`, which `Player_SetWeaponAnimObj`
0x1a2e38:41-57 increments while the view is idle and resets on any anim): idle > 15 → bone 8 blinks
on the frame counter's 20/40 duty [M 0x1a5460:264; the port's frame stands in for GameState+0x76
[S]], idle > 45 → all four off [M :348]. The bone-9/10/11 gates (+0x93e/+0x2fc/+0x2f0 with the
FlashMod variants) are unmeasured — live checks below; the port keeps those three lit. The scope is
102a's, untouched.

## 4. Oddjob's Hat (0x45): one-shot kill, not a grenade

The hat's record [L weapon_data[0x45]]: damage 100.0, gravity (`+0x6C & 0x8`), the 300-tick fuse
(`+0x70 & 0x10000`), throw interval 26. The game's hit path: `Bullet_CollisionHandler` 0x12ce80's
0x45 arm sets the post-hit lifetime (BU+0xc = 1500.0) + the 0x4a7 clang [M :311-316] and the hit
flows through the standard bullet path (`Drone_BulletHit` → msg 8) with the record's 100.0 — a
guard dies on the touch. The fuse death only sets the delete flag [M `Bullet_update` 0x12e7b0:368-373]
and `Bullet_Delete`'s effect kind (+0x70 & 7 = 0) spawns nothing — **the hat never explodes**. The
port threw it into the grenade path, whose only damage was the fuse blast — "it acts like a grenade".

Fixed in `drone.rs`: a per-tick contact pass for weapon 0x45 — a live guard inside the 1.05 contact
sphere takes the record's damage through the standard bullet-hit shape (`last_damage`,
`hit_by_player`, health, DEATH_ANIM / msg::BULLET), the hat keeps flying (the game's post-hit
lifetime), and the fuse death no longer queues a blast. The hat's speed 0.7 [L] keeps the per-tick
displacement under the contact sphere, so the point test doesn't tunnel. **Left**: the hat's
return-to-thrower (if the disc animates one — unmeasured) and the 0x4a7 contact clang.

## Tests (`crates/nf-view/tests/gadgets_live.rs`, 4/4 green; failing-first via the deleted scratch
probes: the laser's clip never drained and no beam parked on the raw-trigger model)

- `the_lasers_charge_drains_the_light_flashes_and_the_beam_stops_at_zero` — clip 500 → 1/tick; the
  red (64,10,10) light flashes [L]; the beam stops exactly at charge 0; nothing fires during the
  switch.
- `the_stunner_spends_only_on_a_target_and_holds_its_charge_aimed_off` — the aimed-at-guard ticks
  spend and latch the target; the aimed-off ticks are free (the regen refund) with no beam.
- `the_aims20s_side_lights_attach_to_its_view_model` — four light attachments on bones 8..0xB of the
  0x1a view model; the idle timer climbs while the gun sits idle.
- `oddjobs_hat_kills_on_contact_and_never_explodes` — the thrown hat kills; `explosions` stays empty.

Neighbours green after the changes: `gadget_laser_live` 7, `fix_level_0c_live` 4,
`fix_fusebox_cut_live`, `qclaw_live`, `gadget_grapple_live`, `explode_live`, `fix_weapon_effects_live`,
`fix_weapon_sounds_live`.

## Verification

- Full suite (`tools/test/suite.py`) on the tree with everything except the last renderer-only
  commit (the uv_lazer slot): the total line is in the WORKLOG tail. The final tree differs from the
  suite's tree by that one commit (`frame_state`'s beam-slot selector); the affected binaries
  (`gadgets_live`, `gadget_laser_live`, `fix_level_0c_live`) re-ran green on the final tree.
- No GPU in this sandbox (wgpu: no adapter) — the looks are the main agent's renders (below).

## Shared-file hooks (the driver's list: drone.rs, scene.rs, player.rs, lib.rs)

- `scene.rs`: the gadget beam block moved into the `release` branch (replacing the top-of-tick
  block); the light/smoke gate widened to every gadget shot tick; the tracer pool loop `0..=`
  → `0..` + three beam slots; `frame_state`'s beam selector + the `aims_light` visibility branch;
  the four AIMS light attachments in the view-model build; the `aims_idle` update after
  `update_with`.
- `drone.rs` (shared, additive): the `aims_idle` field (own line + the constructor init); the hat's
  contact-kill pass and the `weapon_type != 0x45` gate on the fuse blast.
- `grenade.rs` / `player.rs` / `lib.rs` / `gun.rs` / `gadget.rs` / `bullet.rs`: untouched (the hat
  rides the existing grenade struct; no field added there).

## What's left

1. The taser's msg-7 stun state machine (the visible part works: target, beam, light, charge).
2. The laser's battery-station recharge (0x1a5460:575-598 scans for a 0x20002f8 object within
   `+0x1c` — the laser fires free near a charger; unported).
3. The AIMS bone-9/10/11 gates (+0x93e/+0x2fc/+0x2f0 — live-check below).
4. The hat's return-to-thrower and its 0x4a7 contact clang (unmeasured on the disc).
5. The pause-menu stop hook (fix-laser-beam's ready helper — still one line for the pause task).

## Live checks for the main agent

PCSX2:

1. 07000008, `--give 0x4e`, the safe spot: hold fire — the short red beam from the watch, the red
   wall glow, the HUD bar draining ~1 %/5 ticks (500 clip), the beam stopping at 0; then the
   auto-reload from the reserve if it has rounds [M 0x1a5460:449-460]. The port must match each.
2. Same spot, the stunner (`--give 0x4a`): hold fire at a wall (no target) — the blue arc light
   flashes but NO beam and NO charge drain (the free hold); at a guard within 5 — the beam, the
   spend, and the guard's reaction (the port shows the flinch; the stun is the open item).
3. `--give 0x1a`: the four red side lights on the gun; idle > 45 ticks — do they go OFF in the real
   game? (the port turns them off). Between 15 and 45 idle ticks — does the top light blink? Watch
   BLData+0x93a (gdbwatch): does it climb while idle, and what do +0x93e/+0x2f0/+0x2fc hold?
4. The hat (`--give 0x45`): throw at a guard — the touch kills, no blast, no damage to a guard
   beside him; does the hat fly back to Bond? Does the 0x4a7 clang play on the touch?
5. The laser upgrade 0x4f if available: the beam sprite is `uv_lazer` (0x20006c1), not `lazer`.

nf-view renders (the main agent's --shot checks, on this branch):

- `--bond 155.19,43.12,-113.38,1.57,0 --give 0x4e --frame 60` held fire: the red beam from the watch
  to the crosshair hit + the red glow on the wall + the HUD bar/percent.
- The same spot with `--give 0x4a` at a spawned guard: the blue taser beam.
- `--bond` anywhere on 05 with `--give 0x1a`: the four red lights on the gun's side.
