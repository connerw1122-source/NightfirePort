# Pickup atlas (PICKUPINFO, obj class 0x2F)

One pickup object: a 0x40-byte `PICKUPINFO` hanging off a control-list object. Created by `Pickup_Create`
(0x18dda0) for level placements (type 0xF0), script entities (obj_type 0xF0), weapon drops and grenade drops;
updated by `Pickup_Update` (0x18e148, `control_funcs[0x2F].update`, read live in `ee_start_05.bin`); taken via
`Pickup_Handler` (0x18e650), which `Pickup_Update` calls when Bond touches it.

Tags: `[M]` decompile (`work/decomp/ACTION.ELF/`, fn@addr:line), `[L]` live snapshot (`work/ee_start_05.bin` =
07000005, `work/ee_start_07000001.bin` = 07000001; file offset = EE address), `[D]` disc, `[S]` suggested.
No disassembler in this sandbox: where the decompiler dropped arguments they are marked and cited to the
line that shows the field being written instead.

## Name, size, allocation site

| symbol | addr | size | meaning | tag |
|---|---|---|---|---|
| `PICKUPINFO` | heap (ptr at `obj+0xE0`) | 0x40 | the pickup state block | `[M]` `control_create_object(0x40,…)` — first arg is the info size: SP_CreateScriptPlayer passes 0x3C for its 0x3C-byte block `SP_CreateScriptPlayer@0x1db9a8:19`, `Pickup_Create@0x18dda0:108` |
| pickup obj | heap | — | control object, class byte `obj+0xFF = 0x2F` | `[M]` `Pickup_Create@0x18dda0:163` |
| `PickupLastDeletedIndex` / `PickupNextAddIndex` | 0x0030d778 / 0x0030d77c | 4 | MP pickup registry indices | `[M]` symtab (MP-only users) |

`obj+0xD8` = the celglist (model) pointer `[M] 0x18dda0:134`; `obj+0xF0` = glist draw flags — create writes
`(flags & ~0x2000) | 0x840`, type 3 adds `0x80000` and clears `0x800`, type 6/MP clears `0x800` `[M
0x18dda0:137-146]`; **`obj+0xF0 & 0x10` = hidden — set by `Pickup_Handler` when the pickup waits to respawn
`[M] 0x18e650:345`, cleared by `Pickup_Update`'s respawn timeout `[M] 0x18e148:187`. This retags U-036 from [S]
to [M].**

## PICKUPINFO field table

| offset | type | name | meaning | writers (fn@addr) | readers (fn@addr) | tag |
|---:|:---:|---|---|---|---|:---:|
| +0x00 | f32[3] | `velocity` | fall velocity per tick; zeroed at create | `Vec_Zero` `Pickup_Create@0x18dda0:133`; `+= WldGravity/REC_FRAME_RATE²` `Pickup_Update@0x18e148:89-100`; zeroed on landing `:170` | fall move `:101-105` | `[M]` |
| +0x10 | f32[3] | `sphere_offset` | matrix translation − world-sphere centre, fixed after the first `Control_BuildWorldSph` | `Pickup_Create@0x18dda0:190-195` | rest position `Pickup_Update@0x18e148:163-166` | `[M]` |
| +0x20 | u16 | `state` | 0 falling, 1 resting, 2 taken/respawn-wait | `Pickup_Create@0x18dda0:185,188` (rest arg 0/≠0), `Pickup_Update@0x18e148:173,188`, `Pickup_Handler@0x18e650:346` | `Pickup_Update@0x18e148:88,182` | `[M]` `[L]` |
| +0x22 | u16 | `kind` | pickup type (see cases) | `Pickup_Create@0x18dda0:122` (arg 5) | `Pickup_Handler@0x18e650:33,61,166` | `[M]` `[L]` |
| +0x24 | u16 | `id` | weapon id (−1 = refuse at create); key id for kind 2 | `Pickup_Create@0x18dda0:119` (arg 6, −1 checked `:89`) | `Pickup_Handler@0x18e650:38-39,141,218` | `[M]` `[L]` |
| +0x26 | u16 | `rounds` | rounds for kinds 0/1; armour amount for the drone path (unused in the player path) | `Pickup_Create@0x18dda0:120` (arg 7) | `Pickup_Handler@0x18e650:65,101,110,128,192,207,211` | `[M]` `[L]` |
| +0x28 | u16 | `switch_channel` | switch channel set to 1 when taken | `Pickup_Create@0x18dda0:121` (arg 8) | `Pickup_Handler@0x18e650:330-333` | `[M]` `[L]` |
| +0x2A | u16 | `sound` | take sound (0xFFFF = none); overwritten per kind at take time (0xF5 weapon/ammo, 0x141 armour, 0x14C bonus) | `Pickup_Create@0x18dda0:123-128` (arg 9; 0 → 0xFFFF), `Pickup_Handler@0x18e650:290,300,319,321` | `Pickup_Handler@0x18e650:336-337` (`Sound_Play3D`) | `[M]` `[L]` |
| +0x2C | u16 | `respawn` | 0 = delete when taken; 0xFFFF = stays takeable forever; else hide for `FRAME_RATE_INT·respawn·10` ticks | `Pickup_Create@0x18dda0:129` (arg 11) | `Pickup_Handler@0x18e650:338-348`, `Pickup_Update@0x18e148:182-189` | `[M]` `[L]` |
| +0x2E | u16 | `lifetime` | ticks left; 0 = forever; deleted (obj+0xFE \|= 1) at 0 | `Pickup_Create@0x18dda0:131` (arg 12) | `Pickup_Update@0x18e148:254-258` | `[M]` `[L]` |
| +0x30 | u16 | `mp_slot` | 0xFFFF at create (registry slot for MP) | `Pickup_Create@0x18dda0:130,198` | MP code (`MP_getMPpickupFromPickupinfo`) | `[M]` `[L]` |
| +0x34 | u32 | `label` | `Txt_BindLabel` hashcode for the kind-4 message | `Pickup_Create@0x18dda0:132` (arg 13) | `Pickup_Handler@0x18e650:304-305` | `[M]` `[L]` |
| +0x38 | u16 | `spin` | 1 = rotate about +Y by `REC_FRAME_RATE` every tick (type 6 or any MP pickup) | `Pickup_Create@0x18dda0:138-144` | `Pickup_Update@0x18e148:179,193-201` | `[M]` `[L]` |
| +0x3A.. | — | pad | (0x40-byte block) | — | — | `[S]` |

All fields confirmed against live instances: `ee_start_05.bin` pickups at obj 0x16d2760 (kind 1 id 6 rounds 30
snd 0x13F, state 1, r 0.20), 0x16d2a20 (kind 3 id 6 rounds 50 snd 0xFFFF, r 0.48, flags 0x80060) `[L]`;
`ee_start_07000001.bin`: kind 3 id 0x47 rounds 50 **sw 13** snd 0x141, kind 1 id 6 rounds 20 snd 0xF5 `[L]`.

## `Pickup_Create` (0x18dda0) arguments → fields

`Pickup_Create(pos, rot, M, glist, kind, id, rounds, switch, sound, rest, respawn, lifetime, label)`
`(VECTOR*, VECTOR, MATRIX*, celglist_tag*, ushort, short, ushort, ushort, int, char, ushort, ushort, u32)`

| arg | field | note |
|---|---|---|
| pos/rot | `control_create_object` transform | pos NULL → M's translation row `[M 0x18dda0:96-103]` |
| M | `obj+0x90` matrix `[M :113]` | |
| glist | `obj+0xD8`, model bounds | `hashtable_hashcode_to_celglist(weapon_data[id]+0x80)` from `Pickup_CreateSimple@0x18f348:45` |
| kind (5) | +0x22 `[M :122]` | |
| id (6) | +0x24 `[M :119]` | −1 → refuse `[M :89]`; id 0x2C on 07000008 → forced 4 `[M :116-118]` |
| rounds (7) | +0x26 `[M :120]` | |
| switch (8) | +0x28 `[M :121]` | |
| sound (9) | +0x2A `[M :123-128]` | 0 stored as 0xFFFF |
| rest (10) | state 0 / 1 `[M :164-189]` | 0 = dropped: `Mat_Align2Up` upright, `Collide_SphereIntersect` (mask 0x50D) push-out, translation.y −= radius − \|glist centre y\| `[M :165-183]` |
| respawn (11) | +0x2C `[M :129]` | |
| lifetime (12) | +0x2E `[M :131]` | |
| label (13) | +0x34 `[M :132]` | |

The decompiler dropped the stack stores for args 9-13 at the two call sites below; each field's writing line in
`Pickup_Create` is the evidence for the mapping. Types 7–11 are intercepted first → `Pickup_CreateFromSet`
`[M :104-105]`.

**Gold-medal gate**: kind 6 (Bond bonus) without `PlrStats_HasGoldMedal()` → nothing is created `[M :93-95]`.

## Kind cases in `Pickup_Create` / `Pickup_CreateSimple` (0x18f348)

| kind | create | tag |
|---|---|---|
| 0 weapon | normal; the model is `weapon_data[id]+0x80`'s glist | `[M]` |
| 1 ammo | normal | `[M]` |
| 2 key | normal; the id is the key index (`Player_EquipKey`) | `[M]` |
| 3 armour | glist flags get `0x80000`, −`0x800` `[M :145-146]` | `[M]` `[L]` |
| 6 Bond bonus | spins (`+0x38 = 1`), no gold medal → refused | `[M]` |
| 7–11 | `Pickup_CreateFromSet` (MP): weapon id = `PickupMatrix[(kind−7)·2 + MPSettings._436_2_·10]`, rounds = clip·2, always handed over as kind 0; weapon 0x1A also drops weapon 0x1B as kind 1 at pos+0.1 with `obj+0xE8 = 0.5` | `[M 0x18f1b0:56-73]` |
| other | normal (no special casing seen) | `[M]` |

`Pickup_CreateSimple(M, id, rounds, kind)`: rounds 0 → refuse; refuse ids 1, 0x45, 0x47, 0x50, 0x51, 0x53;
0x6A/0x6E → 0x33; `id − 1 > 0x70` → refuse; glist from `weapon_data[id]+0x80`; passes
`(NULL, NULL, M, glist, kind, weapon_data[id]+0x00, rounds, 0, …)` — the trailing stack args (sound 0xF5, rest 0,
respawn 0, lifetime `FRAME_RATE_INT·120` SP / `·30` MP, label 0) were read in the disassembly on the build machine
(`docs/systems/pickups.md`); the C visible here stops at the 8th argument `[M 0x18f348:42-48 + [S] for the
dropped five]`.

## The drop paths

**`DroneWeap_DropWeapon` (0x17a2a0)** — needs the drone's anim object (`+0xDC`) and weapon byte `anim+0x62 ≥ 1`,
and `weapon_data[w]+0x80 ≠ 0` `[M :57-65]`:

1. gun matrix = `AnimGetBoneWorldTrans(obj, max(0, AnimDatumGetWeaponInfo(obj,0,0,0)), 0, …)` (the held gun's
   world transform) `[M :66-71]`;
2. `Collide_RayIntersect(drone obj+0x30 → matrix translation, mask 0x125)`: if anything is in the way the spawn
   position becomes the drone's position `[M :72-87]`;
3. rounds = `clip_size(+0x92)` · f, `f = Rand_FRand(0.5) + 0.5` if clip < 10 else `Rand_FRand(0.5) + 0.25` —
   **one `Rand_FRand(0.5)` draw per drop** `[M :88-95]`;
4. `Pickup_CreateSimple(M, weapon_data[w]+0x02 (the BASE id), rounds, kind 0)` `[M :96-98]`;
5. base id 0x1A also drops 0x1B: rounds = `clip_size[0x1B]` · (`Rand_FRand(0.5)` + 0.25), kind 1, and the new
   pickup gets `obj+0xF0 |= 0x10` (hidden) — a **second RNG draw** `[M :99-111]`;
6. `anim+0x62 = 0` (the drone draws no gun afterwards) `[M :115]`.

**`DroneFunc_DropGrenade` (0x14b158)**: `Pickup_CreateSimple(drone obj+0x90 (its matrix), id, rounds, 0)` —
id/rounds come from the caller (`DroneFunc_OnInitDeath` captains: weapon 0x35 ×2 on 07000001–08, 0x34 ×1
elsewhere, per `docs/systems/pickups.md`) `[M]`.

**Script pickups**: `Script_CreateEntity` obj_type 0xF0 → `Pickup_Create(…, glist, p0, p1, p2, p3)` with the
type/id/rounds/switch from the script-entity param buffer at 0x349974 (`Script_EntityStart@0x1d8d78:80-87`
writes them, `Script_CreateEntity@0x1d8968:156-160` reads them) `[M]`.

**Level placements**: `parsemap_create_dynamic_objects` type 0xF0 → `Pickup_Create(pos, rot, NULL,
placement_glist, level_tag+0x2C (kind), +0x30 (id), +0x34 (rounds), +0x38 (switch), …)` — parameter k lives at
`level_tag+0x2C+4k`, scattered by param index (`parseentity_fixup_entity`/placement docs), not packed positionally
`[M 0x1d04f0:457-462]`. The trailing args (sound, rest, respawn, lifetime, label) are decompiler-dropped here;
live values show placed pickups at rest with per-pickup sounds (0x13F/0xF5/0x141/0xFFFF) and no lifetime `[L]`.

## `Pickup_Update` (0x18e148) — fall, respawn, spin, touch

- **State 0 (falling)**: velocity += `WldGravity` / `REC_FRAME_RATE²` per tick; translation += velocity
  `[M :89-105]`. Ray from `obj+0x40` (the create-time position) to translation − (0, \|glist centre y\|, 0),
  mask 0x50D `[M :110-114]`. When the translation passes the hit: position = hit + `HIT+0x30` · (−\|centre
  y\|) + `+0x10` offset, into `obj+0x30` and `obj+0xC0` `[M :150-168]`; hits on non-static classes (anything but
  2/5/0x11) hand the pickup to `Control_InheritVelocity(3.0)` at the object's position and keep it falling
  `[M :119-149]`; velocity zeroed on the landing tick `[M :169-171]`; state → 1 only when nothing dynamic was
  hit `[M :172-175]`.
- **State 2 (respawn wait)**: returns until `GameState+0x52 − obj+0xEC ≤ FRAME_RATE_INT · respawn · 10`, then
  clears `obj+0xF0 & 0x10` and state → 1 `[M :182-189]`. (`obj+0xEC` = the take timestamp, written by
  `Pickup_Handler` `[M 0x18e650:347]`.)
- **Spin** (`+0x38`): `RotMatrix((0, REC_FRAME_RATE, 0))` concatenated into the matrix every tick `[M :193-201]`.
- **Touch test** (runs for state 0 AND 1 — a falling pickup can be taken mid-air; state 2 returned earlier):
  for each of the 4 `glb_players` slots: player exists, `player+0xF4 == 1` (alive), `player+0xFF != 0x12`,
  `Vec_SqDist3D(pickup obj+0x80, player obj+0x30) < (pickup radius obj+0x8C + 1.1)²`, and
  `Collide_LineOfSight(player obj+0x80 → pickup obj+0x80, mask 0x721)` — then `Pickup_Handler(obj, info, player)`
  and return `[M :202-220]`. **The distance test uses the player's position (+0x30); the LOS test uses the
  player's world-sphere centre (+0x80).** MP bots: a second loop over `MPGame` with the LOS from `+0x30`
  `[M :221-253]`.
- **Lifetime**: `+0x2E` decremented every tick it survives; 0 ends it (`obj+0xFE |= 1`, MP unregister) `[M :254-258]`.

There is no cooldown, latch or settle timer: a taken pickup is deleted or hidden in the same call, and a refused
one (`Pickup_Handler` bailing early) stays exactly as it was, retested next tick.

## `Pickup_Handler` (0x18e650) — what each kind gives

Entry: `PICKUPINFO+0x22` kind; the taker (`param_3`) class 2 = drone/bot path (MP: `BOTWEAP_EquipWeapon/
EquipAmmo`, armour on `Drone+0xAC` and `+0x769` capped at 50) `[M :34-137]`; otherwise the human path
`[M :139-236]`:

- id remap: SP `Upgrade_Weapon(id)` (0x1a8f30: 6→`UpgradedHandguns[Upgrades]`, 0x1E/0x24/0x43/0x4A/0x4E via their
  tables, 10→0xC when `GameState[0x52]`; default unchanged) `[M :142-144]`; MP remaps 10↔0xD per settings `[M :146-163]`.
- **kind 0 weapon**: `BLData + id·0xC + 0x1BA == 0` (not owned) → `Player_EquipWeapon(BLData, id, +0x26)`;
  refused (0) → the pickup stays. Owned → `Player_EquipAmmo(BLData, id, +0x26)`; 0 rounds taken → stays `[M :167-209]`.
  MP-only: an owned-weapon drop can evict the weakest owned gun (the `PickupMatrix`/0x1BA loop, `+0x1B8/+0x1BA`
  cleared, the held byte `sAnimObject+0x62` swapped) `[M :171-203]`.
- **kind 1 ammo**: `Player_EquipAmmo`; 0 → stays `[M :210-215]`.
- **kind 2 key**: `Player_EquipKey(BLData, id)` → `BLData[+0x70C + id] = 1` (`Player_EquipKey@0x1a97a0:7`);
  never refuses `[M :217-219]`.
- **kind 3 armour**: refused while `BLData+0x8B0 ≥ 50.0`; else `+0x8B0 = 50.0`, `+0x8BC = 1.0` — the amount is
  hardcoded 50, `+0x26` is not read on this path `[M :220-225]`.
- **kind 5**: `BLData+0x888 = 0` when the taker is class 3 `[M :229-233]`.
- **kind 6 Bond bonus**: `PlrStat_LogBondBonus()` only — no weapon, no ammo `[M :234-236]`.
- Messages (`Text_AddMsg`, 180 s SP / 90 s MP): 0 = weapon name from `weapon_data+0x38` (SP) / `+0x3C` (MP);
  1 = "%d %s" rounds + `ammo_data[weapon_data[id]+0x90]+8`; 2 = label 0x2000010; 3 = 0x200000F; 4 = the +0x34
  label; 5 = 0x2000013; 6 = 0x200004A `[M :238-329]`.
- `if (+0x28) switch_channels[+0x28] = 1; switch_channels_time[+0x28] = GameState+0x52` — taking a pickup flips
  its switch channel `[M :330-333]`.
- `Sound_Play3D(1.0, −1.0, −1.0, +0x2A, obj+0xC0, …)` `[M :334-337]`.
- Consumption: `+0x2C == 0` → `obj+0xFE |= 1` (deleted); `+0x2C == 0xFFFF` → left as-is; else hidden
  (`obj+0xF0 |= 0x10`), state 2, `obj+0xEC = now` `[M :338-348]`.

## Snapshot check `[L]`

`work/ee_start_05.bin` (07000005 start), objects found by walking `DynamicObjList` (head u32 @0x2705A0+0x14,
next at obj+0x14 — 523 objects; classes match `control_funcs`):

| obj | EE addr | fields read | value | meaning |
|---|---|---|---|---|
| 0x16d2760 | info 0x16d2870 | +0x22 / +0x24 / +0x26 / +0x2A / +0x20 | 1 / 6 / 30 / 0x13F / 1 | ammo, handgun clips ×30, sound 0x13F, resting (an identical copy at obj 0x16d28c0) |
| 0x16d2760 | obj | +0x80 / +0x8C / +0x30 | (48.07, 49.06, −59.26) / 0.20 / (48.06, 49.00, −59.25) | world sphere vs position |
| 0x16d2a20 | info 0x16d2b30 | +0x22 / +0x24 / +0x26 / +0x2A / +0x38 | 3 / 6 / 50 / 0xFFFF / 0 | armour, 50, no sound, no spin (identical copy at obj 0x16d82e0) |
| 0x16d2a20 | obj | +0xF0 / +0xFF | 0x80060 / 0x2F | armour flag bits, class |
| 07000001 0x18fc120 | info 0x18fc230 | +0x22 / +0x24 / +0x28 / +0x2A | 3 / 0x47 / 13 / 0x141 | armour on a switch channel (13) |

Player (touch-test side): `glb_players[0]` obj 0x16cefe0: `+0xF4 = 1` (alive gate), class 3, sphere centre
(104.83, 24.84, 108.58) r 1.06 — **the LOS origin (+0x80) sits 0.12 below the position (+0x30)** and 0.12/0.25
off in x/z `[L]`.

## Untouchable-pickup conditions (game side)

A resting pickup cannot be taken this tick when any of these hold, each measured:

1. kind 3 and the taker's armour is already ≥ 50 (`Pickup_Handler@0x18e650:221-223`);
2. kind 0/1 and `Player_EquipWeapon`/`Player_EquipAmmo` returns 0 (owned + full/zero rounds — the cap lives in
   `ammo_data`) `[M :192-194, 207-215]`;
3. the squared distance ≥ (radius + 1.1)² `[M 0x18e148:203-211]`;
4. `Collide_LineOfSight(player sphere → pickup sphere, mask 0x721)` fails `[M :212-214]`;
5. state 2 (respawn wait) `[M :182-189]`;
6. no alive player (`+0xF4 != 1`, class 0x12) `[M :209]`;
7. lifetime expired (deleted outright) `[M :254-258]`.

Two overlapping drops are tested independently every tick — nothing in the game pairs a pickup to a collider;
the take test is a plain per-frame scan of every pickup against every player.

## The "second dropped Raptor" report

Mechanisms the game has that a port must reproduce for two near-simultaneous Raptor drops: independent per-frame
tests (above), the LOS from the **player sphere centre** (not the feet), radius = glist radius + 1.1, takable
while falling, refusal leaves the pickup untouched, and the single `Rand_FRand` per drop deciding rounds
(clip 30 → 30·(0.25..0.75) = 7..22 rounds — a low roll is a legal refuse when the ammo type is capped).
`docs/atlas/pickup-port-check.md` §1 pins the port-side divergence that matches the symptom.

## Port status (fix-objects): the drawn model and the kind come from one source now

Three port divergences matched the user's reports (master 0a2fcc4, screenshots on 07000001: a gold
"007" mesh at an armour spot, a wrong mesh at a guard drop, extra visible objects at corpses):

1. **The draw-slot desync.** `Drones::update_pickups` ended with `self.pickups.retain(|p| !p.gone)`,
   but the renderer draws placed pickup slot `k` with build-time `placed_pickups[k]`'s mesh and poses
   it at `Sim::drones.pickups[k].m` (`View_DrawObjectGlist` 0x1e6780 draws each pickup object's OWN
   glist at its OWN matrix [M] — the game never reorders a live pickup under a drawn instance). The
   first take shifted every later slot by one: meshes were drawn at other pickups' and at drops'
   matrices. Fixed: taken/expired pickups stay as `gone` tombstones; every consumer filters them.
2. **The hashcode re-lookup.** For a placed pickup the game draws the celglist handed to
   `Pickup_Create` (param_4 from `parsemap_create_dynamic_objects` 0x1d04f0:459-462 [M]) — a pointer,
   not a hashcode. The port re-looked the mesh up by the resolved object's own hashcode, and an
   object with hashcode 0xFFFFFFFF (07000005's `oOBJECT_AmmoPickUp_9mm`, the kind-1/9mm piles at
   (48.06, 49.00, −59.25) seen live in `ee_start_05.bin` [L]) has no by-hash entry, so those pickups
   were created, takeable, and never drawn. Fixed: `Pickup.obj` carries (map, object) and the draw
   uses it; the bounds are the object's own glist bounds, not the first hashcode match's.
3. **Taken drops stayed drawn.** The dropped-gun attachment search (`p.from == Some(di)`) did not
   exclude `gone`, so a tombstoned drop kept the gun visible after taking. Fixed with `!p.gone`.

Tests: `crates/nf-view/tests/fix_objects_live.rs` — slot identity after a take (fails on master:
index out of bounds / len shift), kind↔model for every placed pickup on 07000001 + 07000005 (fails
on master: `kind-1 pickup draws <unresolved 0xffffffff>`), the four 07000001 piles against their
disc kinds/ids/rounds/meshes, the death-dump (one visible drop; placed slots unmoved), and the
afc60ed guard (dynamic-vertex tail pairs 1:1 with `char_verts`; every placed pickup, door and
searchlight slot has vertices).

## PCSX2 trace check (tests/traces/01-raptor.trace, 2026-10-11) [L]

User play on 07000001 (god + infinite ammo; inventory logged as change events):

- t = 6.0 s: weapon **0x0E** becomes owned with clip 9 — the first Raptor drop gives the weapon. (The port's pickups call the Raptor weapon 6; 0x0E is what the game equips — check `Upgrade_Weapon` 0x1a8f30 / the base-id mapping.)
- Every later Raptor drop gave ammo type 3 (spare BLData+0x170+3·2): +6 (19.8 s), +7 (71.0 s), +7 (74.3 s), +5 (76.3 s). Every drop the user walked over was taken.
- Other takes in the run: weapon 0x12 (clip 32) at +17.6 s, 0x1C (clip 8, shotgun) at +50.6 s, 0x35 (count 1, frag) at +52.9 s; ammo types 1 (+20, +9), 9 (+6, +5), 23 (+1), 2 (+16, +11).

**Resolved (fix-pickups-trace, s/fix-pickups-trace).** The chain, all measured:

- The "Raptor" is **weapon 0x0E**, not 6: `weapon_data[0x0E]` = base 0x0E, selectable, flags68 0x1000,
  **ammo type 3** (+0x90, ".357 Magnum rounds", cap 36), **clip 9** (+0x92), name label +0x38 =
  0x0500002D = **"Raptor Magnum"** [D USATxt.dat]. Weapon 6 is the **Wolfram P2K** (type 2, clip 16,
  label 0x05000036) — the old WORKLOG "Raptor (0x100C)" naming was wrong. The guards' weapon byte
  comes from the placement's param 9 = **0xE** [D 07000001.bin; 5 guards, skins 0x36/39/3a] via
  `NDrone2_DefaultInit` → `anim+0x62`.
- The drop id is `weapon_data[w]+0x02` = 0x0E (self-base) [M DroneWeap_DropWeapon 0x17a2a0:96-98];
  `Upgrade_Weapon(0x0E)` = 0x0E [M 0x1a8f30 — id not in any remap table], so the take equips 0x0E:
  not owned → `Player_EquipWeapon` gives clip = 9 [D +0x92] and carries only the rounds beyond it
  (a drop's rounds ≤ 8 → **no spare type-3 event**, exactly the trace's first take).
- The later drops are ammo type 3 with rounds = `clip_size(0xE) = 9 · (Rand_FRand(0.5) + 0.5)`
  truncated = **4..8** — the clip < 10 factor branch [M 0x17a2a0:88-98], one `Rand_FRand` per drop
  drawn before `Pickup_CreateSimple` (the 0x1A twin's draw comes after, :103). All four trace deltas
  (+6/+7/+7/+5) sit in that window.
- The "infinite ammo" of that session was the **recorder's cheat** (tools/trace/trace_record.py
  --ammo / cheats.py): every 30 frames each **owned** weapon's clip and each spare type topped back
  to the **most seen this run** (reset on BLData change). Spares therefore never exceed a take, so
  no ammo cap refuses (type 3: 0→6→13→20→25 against cap 36), and an unowned gun's clip stays 0.
  The port's dev-mode refill wrote `carried.max(200)` (above every cap → `Player_EquipAmmo` refused
  every later drop — "some later drops can't be picked up") and topped unowned clips (the first
  take then leaked the drop's rounds into the spare). Ported to the recorder's shape in
  `nf-view/src/dev.rs` (`Dev::after_tick`); god's armour write corrected to 50.0 (cheats.py F50;
  the trace's armour column).
- Tests: `crates/nf-view/tests/fix_pickups_trace_live.rs` — the first Raptor drop = weapon 0x0E,
  clip 9, no spare leak, HUD "Raptor Magnum" (no cheats); under god + infinite ammo every later
  drop is taken, each rounds ∈ 4..8, the spare = the sum of the takes only (fails on the pre-fix
  port at the first refused drop, shown by stashing the dev.rs fix).
