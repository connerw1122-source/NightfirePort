# fix-fire-modes2 — the fire modes landed on master's explosion path (no double blasts)

Branch `s/fix-fire-modes2` (taking over `s/fix-weapon-fire-modes`, task 100f), merged
`origin/master` twice (`a4164b9`, then `66651ba` — WORKLOG-union conflicts only; `drone.rs` and
`scene.rs` auto-merged the second time). Everything 100f built stays: the guided-missile camera
(`guided_cam`, `steer_missile`, the substate-10 camera ride, `Sim.missile_cam`), the HUD overlay,
the mode table (`docs/systems/weapons.md`), the B-key mode mapping and its tests.

## What was duplicated, and what the merge keeps

100f forked before 100c-fix-weapon-effects landed, so the branch had its OWN impact explode next
to master's:

1. **The bullet world-hit explode.** Master's path: `update_player_bullets` queues
   `(hit, radius, damage, usize::MAX, b.weapon)` into `Drones::pending_blasts` when the weapon's
   `+0x70` carries bit 2 (0x4) and `b.bounces == 0` — `Bullet_CollisionHandler` 0x12d068-0x12d0bc
   [M, disassembled], radius/damage from `weapon_data+0x08/+0x0C`; the queue drains AFTER the
   grenade section's per-tick `explosions.clear()` through ONE `spawn_blast`
   (`Explode_Create` 0x17cbd0: propagate, the 0x6000052 script at radius·0.0667, the shake [M]).
   The branch's inline `blast_at` (drone.rs) — deleted, together with its call site. The naive
   merge (both active) is demonstrated below: two damage applications per rocket hit, the inline
   explosion entry wiped by `explosions.clear()` before the view ever saw it.
2. **The grenade contact explode.** Master's path: the `GrenadeEvent::Bounce` arm in
   `Drones::tick` reads the record's `+0x70 & 4` and queues the blast at the contact, setting the
   grenade's state to 2 (the 'Grenade' twin 0x2A carries it, radius 6 / damage 75; the 'Delay'
   twin 0x2B, 0x280, bounces to its 210-tick fuse — the PCSX2 pass measured exactly that pair,
   docs/live/2026-10-13-weapon-modes-pcsx2.md). The branch's `Grenade::explode_on_impact` field
   and its contact-`Explode` hunk in `grenade.rs::update` — deleted; the field had a second copy
   of the same blast (the MGL twin would have detonated twice).

## Game details folded from the branch into master's path (with citations)

- **The silent range-out.** [M `Bullet_update` 0x12e7b0:158-162] the travelled-past-`+0x1C` arm is
  a SILENT state-4 death — no `Explode_Create`; the fuse explosion lives in `Bullet_DoTrails`
  0x12d890:262-281, which only ticks for `+0x70 & 0x200` records. Folded as an early-out in the
  Sim's `GrenadeEvent::Explode` arm: `flags70 & 0x200 == 0` → the grenade is removed quietly.
  [L] sweep of the record table (`work/ee_start_05.bin` @ 0x2bf150, stride 0x10C): every
  fuse-death record carries 0x200 (0x2B/0x34 = 0x280, 0x35 = 0x2280, 0x36 = 0x20280, 0x37 =
  0x18300, the satchel family = 0x300), and no impact record does (0x1B/0x2A/0x2C-0x2F = 0x4).
  Master's previous behaviour blasted a ranged-out 0x2A round at its 210-tick default fuse —
  wrong on both counts.
- **The no-fuse fuse model in the view's spawn** (`crates/nf-view/src/scene.rs`, kept from 100f):
  a record without `+0x70 & 0x200` gets `range = (+0x1C / speed).max(1.0)` ticks — the
  tick-equivalent of the game's `+0x1C` unit range-out [M 0x12e7b0:158-162] — instead of the
  210-tick default.
- **The record note** folded into master's world-hit comment: the Scorpion 0x2E/0x2F carry the
  0x4 bit too [L ee_start_05.bin @ 0x2bf150].

## Tests (`crates/nf-view/tests/fix_weapon_fire_modes_live.rs`, 5, all through the real path)

`nf_view::load_with_carry("07000005.bin")` + `Sim::step_bond`, real trigger/mode-press/look input.

- `rocket_hit_applies_the_blast_exactly_once` — the headline: one unguided Sentinel rocket 0x2D
  (`+0x70 = 0x4`, r8/d100 [L]) aimed at the floor by the real look input (the rocket's
  `+0x6C & 0x20000` homing otherwise curves it to a guard 37.8 units away — measured by probe),
  exactly ONE explosion entry, exactly ONE `player_hits` entry (ONE damage application), the
  entry `(8.0, 100.0, usize::MAX, 0x2d)`, and zero further blasts in the following 60 ticks.
- `mgl_grenade_0x2a_detonates_on_impact_and_delay_0x2b_runs_its_fuse` — the real fire path +
  the real B-press mode switch: 0x2A detonates on contact (ONE entry, ONE damage application,
  `(6.0, 75.0, usize::MAX, 0x2a)`, within 2 s), 0x2B never detonates on contact and dies on its
  ~210-tick fuse (ONE entry).
- `the_sentinel_guided_mode_rides_the_missile_and_the_unguided_twin_does_not` — the camera ride,
  the steering, the 15-tick static restore, and now also: exactly ONE blast entry for the whole
  flight, `(8.0, 100.0, usize::MAX, 0x2c)`.
- `grenade_events_match_the_records` — the unit sister (no ISO): the grenade event contract the
  Sim's arms consume (0x2A: one Bounce at the contact and one silent range-out Explode; 0x2B:
  bounce + the 210-tick fuse Explode).
- `the_unguided_twin_never_rides_the_camera` — the 0x2D sister.

**Failing first (the reproduction).** The double-blast state the naive merge produces was
reconstructed verbatim (the branch's `blast_at` body re-inserted next to master's queued blast,
uncommitted): `rocket_hit_applies_the_blast_exactly_once` FAILED with
`exactly ONE damage application … left: 2, right: 1` while the explosion entry count stayed 1 —
the inline entry wiped by the per-tick `explosions.clear()`, the damage doubled. Reverted; 5/5
green on the merged tree (`cargo test --offline --release -p nf-view --test
fix_weapon_fire_modes_live -- --test-threads=1`).

## Shared files (small, additive — the list the task asks for)

- `crates/nf-game/src/drone.rs` (shared): the `Explode` arm's `+0x200` early-out (a new break
  arm before the flash/smoke branches, cited above); the Scorpion record note added to master's
  world-hit comment. Nothing renamed, no match arm repurposed; master's
  `pending_blasts`/`spawn_blast`/Bounce-arm/drain are untouched.
- `crates/nf-view/src/scene.rs` (shared): the fuse-model comment updated (the field it named is
  gone); `missile_cam: None` in the direct-load `Sim` constructor (100f's field, unioned with
  master's new `cel_graph`/`icon_cels`).
- `player.rs`, `lib.rs`: untouched. `grenade.rs`, `gun.rs`: `gun.rs` untouched (master's
  `pending_blasts` lives in `drone.rs`; the branch's `gun.rs` had no changes to keep).

## Suite

Run on the final tree (both master merges in): see the tail of this report's "Suite" line —
filled in below after `tools/test/suite.py` completes.

## Live checks for the main agent

PCSX2 (NTSC-U, level 07000005 or anywhere the weapons are held):
- spot: hold the MGL, mode 0x2A 'Grenade'; action: one wall shot at close range. expected:
  exactly ONE explosion (one flash, one `Explode_Create` — watch 0x17cbd0 call count = 1) and
  Bond damaged when point-blank; no second flash a frame later.
- spot: fire 0x2A into open sky (never touches anything). expected: the round silently vanishes
  at its `+0x1C` range (state 4, `Bullet_update` 0x12e7b0:158) — NO explosion at any fuse.
- spot: the Sentinel 0x2C 'Guided'. expected: the camera ride, one blast at impact, the red
  static, restore — as already measured in docs/live/2026-10-13-weapon-modes-pcsx2.md.

nf-view (render check, the merge's visual parity):
- `--bond 104.95,25.0,108.83,-2.15,0.9` on 07000005 (the start room, pitched down): firing the
  MGL 0x2A should show ONE blast puff at the floor a couple of units ahead; the Sentinel rocket
  the single blast + the HUD overlay ring/brackets per docs/live/2026-10-13-weapon-modes-pcsx2.md.

## What's left

Nothing on the task's list. The mode/guided coverage, the modes table and the HUD overlay carry
over from 100f; the explosion path is master's alone.
