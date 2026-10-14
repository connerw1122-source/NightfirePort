# fix-weapon-effects (task 100c) — the report

Branch `s/fix-weapon-effects`, from origin/master 12a6b07, merged origin/master 810c109 before the
final push. 100a (`s/fix-weapon-assets`) was merged per the task instruction — it carried only its
branch-start WORKLOG commit, so there was no code to build on; nothing was duplicated. 101e/101f/
102c had not landed on master when this session closed.

## What's done — the impact explode, end to end

**The bug behind "the explosion is very small" (and worse): the port never exploded a projectile
on impact.** The game's `Bullet_CollisionHandler` destroys an exploding weapon's bullet at its
first world contact: after the impact effects, when `weapon_data+0x70` carries bit 2 (0x4) and the
contact is not a ricochet re-entry, `Explode_Create(radius = weapon_data+0x08, damage =
weapon_data+0x0C, script 0x6000052, attacker = BU+0x30)` fires at the hit point [M 0x12d068-0x12d0bc,
disassembled with capstone: `andi $v0, $v0, 4` / `lwc1 $f12, 8($v1)` / `lwc1 $f14, 0xc($v1)` /
`mov.s $f13, $f12` / `jal 0x17cbd0`; the bounce gate `bnel $s4` at 0x12d07c]. Which weapons carry
the bit [L ee_start_05.bin @ 0x2bf150, the full 115-record sweep of +0x70]:

- **AT-420 Sentinel 0x2c / 0x2d** — radius 8.0, damage 100.0, bit set;
- **Militek MGL 'Grenade' twin 0x2a** — radius 6.0, damage 75.0, bit set;
- the plain guns carry 0x70 = 0x12 (no bit), the frag grenade 0x34 = 0x280 (no bit — bounce-never-
  explodes holds).

The port's world-hit path played the impact effects and ricocheted the bullet like a rifle round —
the Sentinel's missile bounced off walls and died silently at its range end. And the MGL 'Grenade'
twin (which the port routed through the gravity/grenade path via its +0x6C bit 3) bounced too — the
user's "the MGL is always delayed" (already measured on PCSX2: 0x2A exploded on contact, 0x2B
bounced and fuse-exploded 3–4 s later, docs/live/2026-10-13-weapon-modes-pcsx2.md).

Two fixes in `crates/nf-game/src/drone.rs`:

1. **The world-hit impact explode**: after the effects, `+0x70 & 4 && bounces == 0` → the blast.
   The blast queues in the new `Drones::pending_blasts` instead of firing inline — the grenade
   section's per-tick `explosions.clear()` runs AFTER `update_player_bullets`, so an inline blast
   was wiped before the view ever saw it (found by probe: the world-hit printed, the explosion
   never appeared). The queue drains after that clear, so the blast plays the same tick: the
   `Explode_Propagate` damage, the 0x6000052 visual at scale radius·0.0667 [M 0x17cbd0:84-106], the
   camera shake [M 0x17cc28], one `explosions` entry.
2. **The grenade contact explode**: the `GrenadeEvent::Bounce` arm reads the weapon's `+0x70`; bit
   2 → the blast at the contact, the object destroyed — the frag (0x280) and the 'Delay' twin keep
   the bounce-never-explodes rule.

Plus the twin-records fix in `crates/nf-view/src/scene.rs`: the spawned grenade carried the
record's **+0x02 base** as its `weapon_type` — the MGL 'Delay' twin 0x2b bases to 0x2a, so every
death branch (the impact explode, the flash/smoke routing [M 0x12dd60]) read the WRONG twin's
flags and the 'Delay' round contact-detonated too. The grenade family bases to itself [L the +0x02
sweep: 0x34→0x34, 0x35→0x35, 0x36→0x36, 0x37→0x37, 0x3b→0x3b; only 0x2b→0x2a and 0x40→0x3b
differ], so carrying the fired id re-points exactly the twins.

**Full suite** (`tools/test/suite.py`) on the final merged tree (origin/master 810c109 + this
branch): **909 passed, 3 failed, 27 ignored (19 min)** — the only failures are the 3 allowed
`disc_golden` tests; no TIMEOUT, no CRASH.

## Verification (all through the real input path)

`crates/nf-view/tests/fix_weapon_effects_live.rs` — 6 tests, all failing pre-fix, all green
post-fix and on the merged tree (origin/master 810c109):

1. `the_sentinel_missile_explodes_on_impact` — the missile (given through the dev panel, the fire
   trigger through `step_bond`) explodes on the floor contact: exactly one blast, radius 8.0 /
   damage 100.0, the camera shake ≥ 7 (the tick-end decay 0.9·x−0.05 runs after the 8.0 set).
   Pre-fix: the missile never exploded (and the first cut of the fix exploded invisibly — the
   `explosions.clear()` wipe above).
2. `a_plain_gun_bullet_never_explodes` — the P2K fired into the floor for 120 ticks: no explosion
   (its +0x70 = 0x12).
3. `the_mgl_grenade_mode_detonates_on_contact` — the 0x2a round explodes on the contact with
   radius 6.0 / damage 75.0.
4. `the_mgl_delay_mode_bounces_and_waits_for_the_fuse` — 0x2b selected through the ○ mode button
   [M 0x1a24c8:246-260; the direct number-key select lands on the base — the live session switched
   with ○ too], fired at the floor: no explosion before tick 120 (fuse 210), the round alive past
   the contact (it bounced).
5. `walking_backwards_spawns_nothing` — 240 ticks of backwards walking with the P2K out: no
   bullet, no casing, no blast, no muzzle light. The casings rule: `Bullet_init_casing` 0x12fde0
   runs from `Player_WeaponInitBullet` 0x1a4bf0:306, on the shot's release tick only. (The user's
   "casings fly when walking backwards on 16" did not reproduce headlessly on 05 — with this test
   in place any regression is caught; if the console-side symptom persists, the live check below
   covers it.)

6. `one_p2k_shot_spawns_exactly_the_game_effect_set` — one pull: the muzzle light with the
   record's radius 7.0 / rgb (255,138,0) [+0x58/+0x56/55/54, L; M Player_MuzzleFlash 0x1a4298], the
   smoke puff inside its ~8-tick life (the +0x68 = 0x1004 record clears the 0x2000 no-smoke bit),
   the shot's casing ejected, exactly one bullet resolved, no blast (+0x70 = 0x12).

Regression on the merged tree: `fix_weapons_all_live` 4/4 (the frag fuse / stun flash tests ride
the reworked grenade Explode path unchanged), `explode_live` 2/2, `fix_weapon_modes_live` 8/8,
`fix_death_restart_live` 6/6, `fix_weapon_effects_live` 6/6.

## The laser / stunner question (the task's explicit investigation)

The user "hears rapid bullet-impact sounds" holding the watch laser and the stunner. What the
evidence says:

- The watch laser (0x4e/0x4f) **does** fire real bullet objects in the game: the lock burn is
  driven by `Lock_Update` 0x1ae3d8 case 0 walking class-5 collision contacts whose `BU+0x44` reads
  0x4e/0x4f [M, the port's lock.rs cites 0x1ae3d8:57-103] — a class-5 contact IS a bullet object.
  Its record [L]: range 4.0, speed 33.33, damage 0.0 — a short-range fast bolt.
- The stunner (0x4a) is a taser bolt: `NDrone2_DealWithObjHit` case 5 routes weapons 0x4a/0x4c to
  msg 7 (the taser/grapple message), not msg 8 [M 0x13fff0:269-306]; the record: range 5.0,
  speed 33.33, damage 0.0 [L].
- The impacts themselves run the game's standard per-material path (`Effect_Create` 0x1ce688 + the
  EffectInfo sounds), so "rapid bullet-impact sounds" = the bolts really hitting the wall each
  pull/refire — not a fake sound. Whether the console's beam is SILENT (or only the lock-contact
  sparks sound) is not decidable from the decompile alone.

**Live checks for the main agent (PCSX2)**

1. Watch laser on any lock-less wall (e.g. 07000005 spawn, fire at the rock): does each beam tick
   play a material impact sound / leave decals? `gdbwatch` the EffectInfo sound call site
   (`Effect_Create` 0x1ce688) for reads while the beam holds — expected if the game is silent: no
   `Effect_Create` per tick, only the lock-contact path.
2. The stunner at a wall: same question (sounds/decals per shot?).
3. The casings-when-walking symptom on 07000016: walk backwards with the SG5 out and watch
   `Bullet_init_casing` 0x12fde0's callers — expected: only `Player_WeaponInitBullet` 0x1a4bf0 /
   `DroneWeap_FireWeapon` 0x178f68. If a third caller exists, it's the bug.
4. The Sentinel impact blast: fire at a wall, confirm the blast's visual scale (radius 8 → the
   script scale 0.533) and the damage feel matches the port.

**nf-view render commands**

- `nf-view <iso> 07000005 --bond 104.9,26.1,108.8,1.57,0 --frame 40 --shot a.png` — the Sentinel
  fired down at the floor ~2 units ahead: the 0x6000052 blast at the contact point.
- `nf-view <iso> 07000005 --bond 104.9,26.1,108.8,1.57,-1.2 --frame 8 --shot b.png` — the missile
  in flight (its tracer/mesh) before the impact.

## Files

- `crates/nf-game/src/drone.rs` — the impact explode (world-hit + grenade Bounce), the
  `pending_blasts` queue, the `spawn_blast` extraction, `update_player_bullets`' player-pos arg.
- `crates/nf-view/src/scene.rs` — one line: the grenade's `weapon_type` carries the fired id.
- `crates/nf-view/tests/fix_weapon_effects_live.rs` — the 5 tests (new).
No other files touched; drone.rs's changes are additive (one struct field on its own line, one
constructor line, one private fn, the queue block).

## What's left (the next 100c session)

1. **The one-shot effects set for the OTHER classes** (the P2K's is test 6): the shotgun's
   8-pellet loop (+0x18 = 8 [L 0x1c/0x1d] — the port's `fire_player` loop reads a u32 at +0x18
   where the game reads a u8 [M 0x1a4ce8 `lbu`], equal on every shipped record but fragile), the
   sniper's zoom-shot path, the twins' per-mode effects.
2. **The laser/stunner fire rate**: +0x40 reads 0.0 for the whole gadget family [L] — the actual
   refire interval must come from the class defaults in `Player_WeaponFiring` 0x1a5460; whether the
   port's held-trigger cadence matches the console is unverified (needs the live checks above).
3. **The Sentinel's guided camera** (the view rides the missile, 0x60004b3 was observed switching
   in the probe) — the port switches the view script but the steering/camera mode is 100f's scope.
4. **The per-material impact table pass** (docs/atlas/effect.md's EffectInfo records vs the port's
   effect.rs) — the counts/lifetimes per material as a sweep test.
5. Merge 101e/101f/102c when they land; re-run the pairs against their fixes.

## ntfy

(sent successfully — see the session log; no failure text needed)
