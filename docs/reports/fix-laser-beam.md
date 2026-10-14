# fix-laser-beam — the Q-Laser and stunner fire beams, not bullets

Branch `s/fix-laser-beam`. Facts tagged [M] code (function + address), [D] disc, [L] parsed disc data, [S] suggested.

## Root cause: the gadget ids were shifted one pair down

The disc records settle the names [L weapon_data via `name_label` +0x38, bound in USATxt.dat]:

| ids | name | +0x6C | beam draw |
|---|---|---|---|
| 0x4E/0x4F | **Laser** (0x05000043) | 0x900 (0x800\|0x100) | `Draw_LaserBeam` 0x1a4560 via the +0x100 bit [M Player_MuzzleFlash 0x1a4298:131] |
| 0x4A/0x4C | **Stunner** (0x05000042) | 0x880 (0x800\|0x80) | `Draw_TaserBeam` 0x1a46a8 via the +0x80 bit [M 0x1a4298:134] |
| 0x50/0x51 | Grapple (0x05000044) | 0x4000 | none |
| 0x54/0x55 | Micro-Camera (0x05000045) | 0 | none |

The port's `is_laser_weapon`/`runs_beam` sets (0x50/0x51/0x54/0x55 + 0x4e/0x4f) and the gadget.rs/
scene.rs comments had the Grapple and the Micro-Camera where the lasers are. `gadget.rs`'s id sets are
fixed (the names kept — they're port helpers, not game constants), and `the 0x54 = Micro-Camera` also
explains the old `Sound_Play(0xec)` per 0x54 "shot": that's the camera's shutter [M
Player_WeaponInitBullet 0x1a4bf0:261-263], not a laser zap.

## The game's fire model, from the decompile + the disc scripts

Held fire, interval `weapon_data+0x40` = 1 tick for both gadgets [L records]: every tick is a shot.
`Player_WeaponFiring` 0x1a5460's fire branch runs `Player_WeaponInitBullet` 0x1a4bf0 (the +0x44 =
0 family spawns in the branch itself [M :681-690]), which
1. aims from `Player_GetHeadPos` through `Player_GetAimingPoint` — THE CROSSHAIR — and casts the shot
   ray [M :152-159];
2. for the Stunner (the 'J'/'L' = 0x4A/0x4C weapon ids) tests the ray: a class-2/3 hit within
   `weapon_data+0x1c` (5/10) sets BLData+0x95d; **no target → no `Bullet_init` at all** [M :187-200];
3. otherwise spawns the bullet (the Laser's is what cuts fusebox wires — category word +0x10 = 0x0002
   [L, kept by `Collide_FilterBullets`]) and sets BLData+0x95b + BLData+0x60 = the ray hit;
4. the next tick's `Player_CollisionHandler` 0x1949a8:189-191 sees +0x95b → `Player_MuzzleFlash`
   0x1a4298: the muzzle light and, for +0x6C & 0x180, the beam from the view-model muzzle to the
   re-cast hit of BLData+0x60's direction [M :107-137].

The world death of a +0x100 bullet is **effect kind 0x10 → `Effect_Laser` 0x1cef38** [M
`Bullet_CollisionHandler` 0x12ce80:186-192] — a burn puff, NO `Effect_Bullet` decal, and its sound arm
gates on effect flags 0x40 (set only by `weapon_data+0x70 & 2`, clear on the Laser's +0x70 = 0x800
[L]) — **no wall-impact sound**. The bullet's fire script (0x060004e9 laser / 0x0600048a stunner [L
07000008.bin kind-5]) plays the LOOP (0x12 / 0x3d) at its frame 1 on every restart — and `AnimScriptAdd`
0x1c6d40 **deletes and recreates** the script each refire tick [M], so the loop holds while the fire is
held. On release the view state 0xb (set per shot for the +0x68 & 0x400 family [M 0x1a5460:673-679])
waits for the fire script to stop, then queues **+0xB4** (0x060004e8 / 0x06000488) whose **op-4.3
frame 1 stops the loop** [L the scripts; M 0x1a2e38 case 0xb:499-507, at 1.5× M 0x1a5460:677-679].

## What's fixed (all cited above)

1. **No bullet impacts from the gadgets** (`drone.rs`, the shared hook): a player bullet whose record
   carries +0x6C & 0x100 skips `impact_sound` and the `Effect_Bullet` decal/ricochet path; it spawns
   the `Effect_Laser` burn puff (the port's gas pool, kind 5, 0x7f7f64ff — the per-surface mesh field
   EffectInfo+idx·0x88+0x10 is unparsed yet [S]).
2. **The beam drawn each held tick** (`scene.rs` + a renderer slot): `Sim::laser_beam` is now set from
   the CROSSHAIR ray (look + the cross offsets — the same construction the bullets take) and the
   renderer draws it as one stretched trail (a new tracer slot TRACER_SLOTS). [S] the mesh is the
   port's 0x02000123 trail stand-in; the game's 0x2000131/0x20006c1 beam sprites live in the global
   gfx table the port doesn't resolve — the main agent's render check owns the look.
3. **The loop sound stops** (`scene.rs`, `sound.rs`): (a) the fire-script restart gate now applies
   only to the release-frame family (+0x44 ≠ 0) — the Laser/Stunner restart per tick as the game does;
   (b) `Sounds::play_loop` keeps ONE voice per looped id (the re-played op-1 used to allocate a node
   per tick); (c) the release runs the game's state-0xb → +0xB4 transition, whose op-4.3 stops the
   loop; (d) a weapon switch stops the current weapon's looped op-1 ids (the game's `MenuManager_Create`
   pauses streams with `SFXPause` [M 0x21b908:120-123]; the port's pause path can call
   `gadget_loop_ids` + `stop_all_with_id` — hook left for the pause menu task, noted below).
4. **The stunner** (`scene.rs`): the per-tick ray test (`Sim::stunner_target`) — no live guard within
   +0x1c → NO bullet (the release block is gated) and NO beam. The aim ray is the crosshair ray.
5. **The accuracy** (the 0c display vent): the burn/beam ray is the crosshair ray;
   `fix_level_0c_live`'s hinge burn now runs on the Laser 0x4E through it and passes.
6. `--give 0x4e,0x4a` verified: both equip (`equip_weapon` + the first-person model gate).
7. The old 0x54 "watch laser" tests are rewritten to the Laser 0x4E (0x54 is the Micro-Camera — no
   beam, no lock burn on the disc).

## How it was verified

- `gadget_laser_live.rs` **7/7**: the safe-bolt burn through the Laser (switch_to 0x4e), the hold
  test (60 held ticks → **0 bullet decals**, **exactly one 0x12 loop voice**, no voice churn, the beam
  parked; then 24 released ticks → **the loop gone** — the game's own ring-out, see below), the beam's
  far end **on the crosshair ray** (dot > 0.999) landing on the aimed-at face, the targetless stunner
  (no decals, no beam, `stunner_target == Some(false)`).
- `fix_level_0c_live.rs` **4/4** (the four hinges burn through the crosshair ray → ch 18 → the vent
  opens — the user's accuracy symptom), `gadget_grapple_live` 1/1, `fix_fusebox_cut_live` 4/4 (the
  laser still cuts the 07000008 wire).
- Suite: nf-game lib 139/139; nf-game + nf-formats + nf-audio 349 passed with exactly the 3 allowed
  `disc_golden` failures; nf-view non-walk **479 passed, 0 failed**; the walk samples
  (01/05/08/0c + walkreal 01/0c) green. All on the tree merged with origin/master (12a6b07).

## What's left

1. **The pause-menu stop hook**: the game silences streams under the menu (`SFXPause` [M
   0x21b908:120-123]); the port's Esc pause lives in lib.rs (App) — the helper
   (`Scene::gadget_loop_ids` + `Sounds::stop_all_with_id`) is ready; the call site is one line for the
   pause-menu task (this task's lib.rs budget went to nothing — no lib.rs change was needed).
2. **The taser's drone effect**: the game's msg 7 (`NDrone2_DealWithObjHit` case 5) stuns the guard;
   the port still routes the stunner's drone hit to msg 8 (a bullet hit). The stun state machine is
   unported (drone.rs's comment says so since fix-actors).
3. **The `Effect_Laser` look**: the burn puff uses the pool's default mesh; the per-surface mesh at
   EffectInfo+idx·0x88+0x10 needs the parse.
4. **The grapple ids** (found on the way, NOT fixed here): the claw is 0x50/0x51 — its bolt latches
   on a class-0x51 point's sphere at `Bullet_Delete` 0x12e660:36-60, not the press-ray pick; the
   port's grapple gate + tests still use 0x4A/0x4C (harmless on levels without 0x3A placements — the
   user's 08/0c play never hits it; gadget_grapple_live still passes on its own model).

## Live checks for the main agent

1. **The beam look**: in PCSX2, hold fire with the Laser (0x4E) on 07000008 aimed at a wall — what
   does the beam look like (a thin bright line muzzle → hit? the flicker 0x2000131/0x20006c1 swap?),
   and in nf-view `nf-view Nightfire-onfoot.iso 07000008 --bond 160.3,44.6,-115.4,0.85,0.12 --give 0x4e` holding
   fire (F or the panel's fire) — the port draws the trail-mesh stand-in from the eye to the hit.
2. **The stunner's beam**: aim at a guard within ~5 units — the taser arc draws to the guard [M
   +0x95d]; aim at a wall — nothing. Spot: 07000001's entrance guard.
3. **The loop sound**: hold the Laser fire ≥ 3 s, release — the hum should ring out ~0.3 s (the fire
   script's 16 frames) and stop; switch weapons mid-hold — the loop stops with the weapon. Compare
   the port's cadence (the task's "1 tick after release" is the game's own state-0xb ring-out, ~16
   ticks — I pinned ≤ 24; if PCSX2 shows a harder cut, tell me and I'll stop the loop on the release
   tick directly).
4. **The 0c display vent**: burn the four hinges (the aim now goes through the crosshair) — the
   vent opens at the same stands as the test (`fix_level_0c_live.rs`'s spots).
5. **nf-view spots**: `--bond 6.0,13.8,-110.5,0.9,0.05` (0c, the lower walkway before the display
   vent) with `--give 0x4e` holding fire at each hinge; `--bond 160.3,44.6,-115.4,0.85,0.12` (08, the
   safe) holding fire at the bolts.

## ntfy

(sent after this report is pushed; see the queue finish log if it failed)
