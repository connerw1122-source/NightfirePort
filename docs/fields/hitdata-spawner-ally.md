# HITDATA_tag, the DroneSpawner block, and the ally / mission-fail drone fields

Task 4-e (2026-10-13). Proof levels per [README](README.md): [S] guess, [M1] one reader/writer
read by hand, [M] the function+address (all readers/writers found listed), [M*] every reader/writer and they agree,
[L-read] a value read in an EE snapshot (file offset == EE address). Decompile: work/decomp/ACTION.ELF (one .c per
function, filename = ADDRESS_Name). No game bytes below — offsets, addresses, values, names only.

---

## 1. `HITDATA_tag` — 0x60 bytes, pool-allocated hit-list node

**What it is.** One collision hit. The engine keeps a per-object doubly-linked list of them: the list head lives at
`obj+0xD0` (nodes added at the head; `+0` = next/newer, `+4` = prev/older), and `obj+0xD4` = the object's `HITTEST_tag`
(0xA0 bytes — `Collide_RayIntersect` 0x1ec690 memsets 0xa0 on the stack; the bullet embeds one at `BU+0x50`,
`Bullet_update` 0x12e7b0:419-465). Every "HITDATA*" in the decomp that walks `+4` is one of these list nodes.

**Size = 0x60** [M*]: nodes come from the LList pool 0x34b7b0 and every alloc site memsets 0x60
(`Coll_AddHitToList` 0x1ede68:13, `Collide_PickObj` 0x1eae80:112/172/238, `Collide_Update` 0x1ea398:176); and
`Player_DealWithObjHit` 0x194458:77 copies a whole hit into the level's store with `amemcpy(level+0xDC, hit, 0x60)`.
No stack-declared HITDATA was found — the bullet/damage dispatch uses the pool nodes; the one whole-struct copy is
the level's `level+0xDC` store (0x60 bytes). `CompHitData` 0x1d33c0 sorts nodes by `+0xC`.

Note: `Coll_AddHitToList__FPP11HITDATA_tagfP7cel_tagP7obj_tag`'s mangled first param type is wrong in the
demangler — the call in `Explode_Propagate` 0x17c518:129-130/215-216 passes a float (0x461c3c00 ≈ 9989.0) as p1,
`obj+0xD0` (the list head) as p2, 0 as cel, the owner as obj; the returned node's `+8` is then written with the
damage. Real signature: `(f32 dist, HITDATA** list, cel_tag* cel, obj_tag* obj) -> HITDATA*` [M].

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | list next (toward the newer node; head `obj+0xD0` points at the newest) | [M*] `Coll_AddHitToList` 0x1ede68:17-24 (link build), `Collide_Update` 0x1ea398:167-186 (same inline), walked via `+4` by `Collide_FilterBullets` 0x1ece30:23, `Collide_GetDamageNObjects` 0x1ee220:25, `Bullet_CollisionHandler` 0x12ce80:546, `Drone_CollisionHandler` 0x140968:91 |
| +0x04 | u32 | list prev (toward the older node) | [M*] same link-build sites |
| +0x08 | f32 | **damage amount** — copied from `HITTEST+0x90` = the weapon's damage (`weapon_definition+0xC`, `Bullet_update` 0x12e7b0:391) | [M*] writers: `Collide_Jointy` 0x1ec180:127, `Collide_Intersect` 0x1ebaa8:322, `Collide_Update` 0x1ea398:191 (the mirrored node), `Explode_Propagate` 0x17c518:164/217, `Drone_ModPlayerHitDamage` 0x13a940:18 (rewritten for player-vs-drone), `DoHitEffects` 0x1458e0:54 (MP perk scaling), `Player_DealWithObjHit` 0x194458:137/163; readers: `Drone_BulletHit` 0x13a9f8:20 (>0 gate), `Drone_ModBulletDamage` 0x139f50:13, `Collide_GetDamageNObjects` 0x1ee220:20-44 (sums the list), `NDrone2_DoHitEffects` 0x1458e0:45-105, `Player_DealWithObjHit` 0x194458:97/185/196, `Break_ApplyDamage` 0x12c128:8, `RB_RestingContact` 0x1d7168:113 |
| +0x0C | f32 | **hit distance / penetration** — the sort key; 0x4cbebc20 (1e8) = "no hit / filtered out" | [M*] writers: `Coll_AddHitToList` (p1), `Collide_PickObj` 0x1eae80:114/175/240 (squared dist at pick time), `Collide_Jointy` 0x1ec180:92/132/150/271, `Collide_Intersect` 0x1ebaa8:179/182/197/236/271/269-271, `Intersect_PointGeom` 0x1e92d8:165/311/337, `Intersect_CylGeom` 0x109da0:562/587, `Collide_Update`/`FilterBullets`/`Collide_RayIntersect` set 1e8 to drop; readers: `CompHitData` 0x1d33c0:9-10 (the QuickSort comparator), the sort loops in `Collide_Update` 0x1ea398:122-141 and `Bullet_CollisionHandler` |
| +0x10 | VECTOR16 | **hit normal** (world; unit). Written in the hit geometry's local space, transformed to world by `Collide_Intersect` | [M*] writers: `Collide_Jointy` 0x1ec180:153, `Collide_Intersect` 0x1ebaa8:384-388, `Intersect_PointGeom` 0x1e92d8:326-329 (the tri's packed normal ×6.103888e-05), `Intersect_CylGeom` 0x109da0:583, `Collide_Update` 0x1ea398:201-207 (mirror copy), `Effect_Create` 0x1ce688:69-71 (negated for material 0x10), `Break_Kill` —; readers: `Effect_Bullet` 0x1ce8a8:77-84/151-160 (decal alignment), `Effect_Laser` 0x1cef38:60-70, `RB_CalcTorque` 0x1d7658:38-41, `Bullet_CollisionHandler` 0x12ce80:384-391/452-469 (bounce), `Debris_Create` 0x1aaa00:40-48 (scatter velocity), `Player_DealWithObjHit` 0x194458:197-226 (the BLData+0x967 damage-direction nibble) |
| +0x20 | VECTOR16 | **surface tangent** (u+v of the orthonormal basis of `+0x10`); scratch the effect/debris code shares | [M] writer: `Effect_Create` 0x1ce688:73-78 (`Vec_MakeOrthonormalBasis` sum); readers: `Debris_Create` 0x1aaa00:40-48 (velocity base), `Effect_Laser` 0x1cef38:112; zeroed by `Break_Kill` 0x12b970:140 before `Debris_Create_Loop` |
| +0x30 | VECTOR16 | **direction of the hit** (unit; for point tests = the normal) — the bullet's travel direction at the impact | [M*] writers: `Collide_Jointy` 0x1ec180:190-193 (copy of `+0x10`), `Intersect_CylGeom` 0x109da0:581 (tri normal), `Collide_Update` 0x1ea398:208-214 (mirror copy); readers: `Drone_ModBulletDamage` 0x139f50:42-45 (ATAN2(x,z) vs the victim's yaw → the back-shot multiplier), `NDrone2_DoHitEffects` 0x1458e0:71-72 (the MP-assassin backstab angle), `NDrone2_DealWithObjHit` 0x13fff0:258 (knockback along it), `Player_DealWithObjHit` 0x194458:157-159 (assassin angle), 0x194458:199-201 (the hit-direction indicator), `RB_RestingContact` 0x1d7168:113, `ShatterPolyRecurse` 0x12b270:147/183 (via the obj path), `Explode_Propagate` — |
| +0x40 | VECTOR16 | **hit point** (world) | [M*] writers: `Collide_Jointy` 0x1ec180:133-150/272-279, `Intersect_PointGeom` 0x1e92d8:320/330-333, `Intersect_CylGeom` 0x109da0:526-529, `Collide_Intersect` 0x1ebaa8:321-338 (matrix transform), `Collide_Update` 0x1ea398:194-200 (mirror), `Debris_Create_Loop` 0x1ed548:126-131 (rewritten per triangle); readers: `Env_WaterRipple` 0x17c118:20 (Gas_Init), `NDrone2_DoHitEffects` 0x1458e0:108/111 (hit sounds 0x601/0x413), `NDrone2_DealWithObjHit` 0x13fff0:115-123 (drone aims at it, msg 0x22) / :246-255 (sparks), `Effect_Bullet`/`Effect_Laser`/`Effect_Player`/`Effect_Body` (all effect origins), `Drone_CollisionHandler` 0x140968:102-118 (ground alignment), `Bullet_CollisionHandler` 0x12ce80:312-322/373-391 (ricochet), `RB_CalcTorque` 0x1d7658:43-44, `Debris_Create` 0x1aaa00:25 (spawn point), `Break_Kill` — |
| +0x50 | u8 | **material / surface id** (masked 0x3F): indexes the per-material `EffectInfo` table (0x88 bytes each: sound +0x14/+0x18, effect +4, debris HEINFO +0x84…). 0x0D/0x0E = water (ripples), 0x10 = flesh/none | [M*] writers: `Intersect_PointGeom` 0x1e92d8:321, `Intersect_CylGeom` 0x109da0:568 (from the coll-data material array), `Collide_Update` 0x1ea398:192 (mirror copy), `Break_Kill` 0x12b970:137-138 (=0x10 when 0); readers: `NDrone2_DealWithObjHit` 0x13fff0:72-99 (`&0x3F` < 0xF water gate → `Env_WaterRipple`; 0x1D-mode velocity flip), `Player_DealWithObjHit` 0x194458:65-85, `Env_WaterRipple` 0x17c118:9 (`&0x3F` − 0xD < 2), `Effect_*` (every EffectInfo index), `Bullet_CollisionHandler` 0x12ce80:137/149-157/470-517/500-513 (flesh pass-through 0x10, 0x40/0x80 bits, ricochet), `Collide_Update` 0x1ea398:216-225 |
| +0x51 | u8 | **pass-through flags**: bit 1 = don't stop the bullet / no ground contact; bit 0 (on the mirrored node) = "I was hit by this bullet" | [M*] writers: `Intersect_PointGeom` 0x1e92d8:264-269 (`\|= 2` from material bits 0x40/0x80), `Intersect_CylGeom` 0x109da0:421/536 (`Collide_Filter` result), `Collide_Intersect` 0x1ebaa8:97/144/305-310 (`\|= 2` when `obj+0xF0 & 0x40` or material bits), `Collide_Update` 0x1ea398:190 (`\|= 1` on the mirrored node); readers: `Drone_CollisionHandler` 0x140968:86-94 (ground node = last with bit 1 clear), `Bullet_CollisionHandler` 0x12ce80:112-118/496, `Collide_Update` 0x1ea398:216, `Intersect_CylGeom` 0x109da0:421 (`& 6` skips the push-out) |
| +0x52 | u16 | **hit part / bone id** (the anim coll-box row that won; 5 = head, 4 = torso, 0x31–0x38 legs, 0x14/0x15/0x17/0x20/0x23/0x27 arms) | [M*] writers: `Collide_Jointy` 0x1ec180:126 (from the box's +0x50), `Collide_Update` 0x1ea398:193 (mirror); readers: `NDrone2_DoHitEffects` 0x1458e0:68/105 (→ `NDrone2_HitDamage`'s part), `Player_DealWithObjHit` 0x194458:143-181/196 (→ `Player_HandlePain`), 0x194458:230-234 (head sound 0x600 vs 0x412) |
| +0x54 | cel_tag* | the static-geometry cel the hit belongs to (0 = a dynamic obj hit; then effects attach to `+0x58`'s obj) | [M*] writers: `Coll_AddHitToList` 0x1ede68:26 (p3), `Collide_Update` 0x1ea398:188 (0 for mirrors), `Collide_PickObj` 0x1eae80:125/186/251 (0); readers: `Env_WaterRipple` 0x17c118:10-18 (gas origin, else `obj+0x20`), `Effect_Create` 0x1ce688:64-67, `Effect_Bullet` 0x1ce8a8:166-186 (decal parent choice), `Effect_Player` 0x1cec48:124-125, `Bullet_CollisionHandler` 0x12ce80:106-115 |
| +0x58 | obj_tag* | **the hit object** (0 = static world). On the victim's mirrored node this is the BULLET obj (so `DealWithObjHit` case 5 can read the shooter off its BU) | [M*] writers: `Coll_AddHitToList` 0x1ede68:27 (p4), `Collide_PickObj` 0x1eae80:124/185/250, `Collide_Update` 0x1ea398:189 (= the bullet obj on mirrors); readers: `Collide_Jointy` 0x1ec180:101/206-225 (anim coll data, capsule-vs-capsule), `Collide_Intersect` 0x1ebaa8:84-107/262-270, `Collide_GetDamageNObjects` 0x1ee220:35 (collects hit objs), `Collide_FilterBullets` 0x1ece30:20-30, `Explode_Propagate` 0x17c518:71-216 (the whole explosion dispatch), `NDrone2_DealWithObjHit` 0x13fff0 (case 5 = the bullet's BU → weapon id + shooter), `Drone_BrokenObject` 0x138588:42-44 (the shooter is the player), `DroneFunc_RecoverTime` 0x147188:13-32, `NDrone2_DoHitEffects` 0x1458e0:28-31 (the shooter's weapon for MP perks), `Player_DealWithObjHit` 0x194458:69-190, `Effect_Bullet` 0x1ce8a8:163-186 |
| +0x5C | u32 | — (no reader or writer found; memset 0 at allocation) | [M*] only the 0x60 memsets reach it |

Drone-side state the hit flow writes (context for the trace): `Drone+0x470` = the HITDATA being processed
[writer `NDrone2_DealWithObjHit` 0x13fff0:103], `Drone+0x1D8` (s16) = hit count [writers `Drone_BulletHit`
0x13a9f8:22, `NDrone2_DealWithObjHit` 0x13fff0:303], `Drone+0x2B4` (obj*) = last hit by [writers `Drone_BulletHit`
0x13a9f8:23 (`BU+0x30` = the shooter), `NDrone2_DealWithObjHit` 0x13fff0:304 (the shooting drone's obj,
`Drone_ExplosiveHit` 0x13aab8:20; cleared by `Drone_Delete` 0x136ce8:69].

### One full path: a player bullet hits a guard

1. `Player_WeaponInitBullet` → `Bullet_init` 0x12c5a0 creates the bullet obj (`obj+0xFF` = 5, BU at `obj+0xE0`;
   `BU+0x24` = weapon_definition*, `BU+0x30` = owner obj, `BU+0x44` → the weapon record).
2. `Bullet_update` 0x12e7b0:419-465 fills the embedded HITTEST at `BU+0x50`: start `obj+0x30` → `+0x20`,
   end → `+0x30`, dir → `+0x40`, radius → `+0x8C` (`obj+0x8C`, floor 0.25; 0 for shotguns 0x2C-0x31/0x62/0x6D),
   damage = `weapondef+0xC` → `+0x90`, type 0x101 (point, step < radius) / 0x201 (ray) → `+0x94`,
   `+0x9A` = 0x20 unless `weapondef+0x6C & 0x800`.
3. Each frame `Collide_Update` 0x1ea398 resets the HITTEST (`+0x70` = the bullet obj, `+0x9C` = 0), then
   `Collide_Pick` 0x1eb320 → `Collide_PickObj` 0x1eae80 allocates a node per candidate (skips the shooter and
   `obj+0xF0 & 0x20` corpses) into `bullet->obj+0xD0` with `+0x0C` = squared distance, `+0x58` = the candidate,
   `+0x54` = 0; then `Collide_Intersect(HITTEST, node, obj+0xFA-flags)` 0x1ebaa8 per node → type 0x201 with anim
   coll boxes → `Collide_Jointy` 0x1ec180 (first box in list order wins) writes `+0x52` part, `+0x08` damage,
   `+0x40` point, `+0x10` normal, `+0x0C` = radius·t; the survivors are sorted by `+0x0C` (`CompHitData` 0x1d33c0)
   and the 1e8 ones dropped.
4. `Collide_Update` 0x1ea398:160-234 then MIRRORS each surviving node into the victim's own `obj+0xD0` list:
   `+0x08` = the bullet's HITTEST+0x90 damage, `+0x40`/`+0x10`/`+0x30` copied, `+0x50`/`+0x52` copied, `+0x51` |= 1,
   `+0x58` = the bullet obj.
5. `Bullet_CollisionHandler` 0x12ce80 (the bullet's own list) does the fx: material `+0x50` → `Effect_Create`
   0x1ce688 (sound/gas/decal at `+0x40` along `+0x10`), pass-through bits decide ricochet vs death.
6. `Drone_CollisionHandler` 0x140968:84-95 walks the guard's list → `NDrone2_DealWithObjHit` 0x13fff0 case 5:
   weapon id = `**(u16**)(BU+0x44)` → 1/0x4B/0x4D → msg 6 (punch), 0x4A/0x4C → msg 7 (taser), 0x43/0x44 → msg 0x19,
   else `Drone_BulletHit` 0x13a9f8 (damage > 0 gate at `+0x08`; `Drone+0x1D8`++, `Drone+0x2B4` = `BU+0x30`) → msg 8
   with the HITDATA as payload.
7. `DroneFunc_HandleImpact` 0x1469b8 case 8 → `NDrone2_BulletImpact` 0x1461c8 (sets `Drone+0x50C` = 1.0, alert 2)
   → `NDrone2_DoHitEffects` 0x1458e0 (damage `+0x08` × `Drone+0x14C`, part `+0x52`, shooter `+0x58`; hit sounds at
   `+0x40`) → `NDrone2_HitDamage` (docs/systems/drone-damage.md). The state's own msg-8 arm
   (`NDrone2_DSTATE_BulletImpact` 0x173098) replays it as `BulletImpact(hitdata, 0, 0)`.

---

## 2. The spawner object (obj type 0x3C, data block at `obj+0xE0`)

`DroneSpawner_Create` 0x1378d0: `control_create_object(0x3C)`; `obj+0x30/+0x40` = param VECTOR (pos),
`obj+0x50/+0x60` = second VECTOR, `obj+0xFF` = 0x30, `obj+0x20` = `build_FindCel`. The placement param block
(p0..p8) maps word-for-word into the spawner block — **verified live** on the 07000005 snapshot (below). The
created obj is registered in the per-group table `DAT_00298e9c[group] = obj` [M 0x1378d0:56; L-read below].
Globals: `DAT_00298e20+4·group` = the group's template-drone list head, `DAT_00298f18+2·group` = its count
[readers `DroneSpawner_Init` 0x137aa0:16/29].

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | **p0 mode** (> 1 required): 2 = spawn while alive < max, 3 = only when none alive (on 0x700000C/0D waves>0 plays the spawn SFX 0xD6/0xD7); other values never spawn | [M 0x1378d0:24, 0x137e68:44/174-204] [L-read: both 07000005 spawners mode 2] |
| +0x04 | u32 | **p1 group** (the templates are the group's 0x0F placements; also the `DAT_00298e9c` index) | [M 0x1378d0:25, 0x137aa0:16-29, 0x13a338:26] [L-read: groups 1/2 live] |
| +0x08 | u16 | the group's template count (copied from `DAT_00298f18` at Init; 0 → the spawner disables itself) | [M 0x137aa0:16-31] [L-read: count 2 = 2 templates each] |
| +0x0A | u16 | **p2 max alive** (clamped to the count at Init; the live-array length and the per-wave budget) | [M 0x1378d0:26, 0x137aa0:22-23/46, 0x137e68:89-145] [L-read: 1 and 2] |
| +0x0C | u16 | **p3 waves** (0 → 32000) | [M 0x1378d0:27/37-40] [L-read: 1] |
| +0x0E | u16 | **p4 trigger channel** — consumed (`= 0`) when it fires and `+0x1E` set to 1 | [M 0x1378d0:28, 0x137e68:55-62] [L-read: group 2 = 0x22] |
| +0x10 | u16 | **p5 stop channel** — on: un-triggers (`+0x1E = 0`); pauses an armed spawner (returns); deletes an un-armed one (`obj+0xFE \|= 1`) | [M 0x1378d0:29, 0x137e68:72-85] [L-read: both 0 here] |
| +0x12 | u16 | **p6 done channel** — raised (+ `switch_channels_time` stamp) when all waves spawned and (when `+0x14` is set) no spawned drone is alive; then the spawner disables (`obj+0xFE \|= 1`, `+0x1D = 0`) | [M 0x1378d0:30, 0x137e68:126-143] |
| +0x14 | u16 | **p7** — `Control` latches `ch[p7]` → the byte at +0x20 = the **KILL** channel **[L-verified, see "Live result" below]** | [M 0x1378d0:31, 0x137e68:50-52] [L-read: group 2 = 0x43 (the 07000005 wall-hug plate #301)] |
| +0x18 | f32 | **p8 min spot distance** (0 → 10.0) — `SpawnDrone` accepts a template spot only when the player is at least this far (unless +0x1F) | [M 0x1378d0:32-36, 0x137cb0:30-34] [L-read: 10.0 and 4.0] |
| +0x1C | u8 | the per-frame "spawn now" decision (`Control` internal; cleared at wave rollover) | [M 0x137e68:151/177/185/203] |
| +0x1D | u8 | initialised (Init sets 1; `Control` clears it when done; 0 → `Control` returns) | [M 0x137aa0:69, 0x137e68:47/142/287] [L-read: 1] |
| +0x1E | u8 | triggered (Init: 1 when p4 = 0; `Control` sets it consuming p4; the stop channel clears it) | [M 0x137aa0:72-74, 0x137e68:62/78/141/287] [L-read: group 1 = 1 (no trigger), group 2 = 0] |
| +0x1F | u8 | "no stealth checks" — Init sets it when (p4 = 0 or on) and (p5 = 0 or off); `SpawnDrone` skips the distance/visibility checks while set; cleared at wave rollover | [M 0x137aa0:33-45, 0x137cb0:30, 0x137e68:148] [L-read: 0 on both (their waves already ran / not started)] |
| +0x20 | u8 | the p7 latch — **[DISPUTED]**: fix-actors calls it `armed` ("an armed spawner needs no trigger"); the decompile's branch at 0x137e68:86 sends it to the cleanup block (0x137e68:235-288), which on levels outside {0x11..0x13, 0x16, 0x4A} disables every spawned drone and the spawner, and on those levels only drones with `obj+0xFC` (f32) == 0 and waits for the rest — i.e. triggers.md's original "kill channel" reading | [M 0x137e68:50-52 (writer), :54/73/76/79 (readers), :235-288 (the armed branch)] [L-read: 0 on both] |
| +0x28 | u16 | spawns this wave (SpawnDrone increments; the wave rolls over when max_alive ≤ it: reset, `+0x2C`++, `+0x1F` = 0, `+0x1C` = 0, every template's record count reset) | [M 0x137cb0:83, 0x137e68:145-172] [L-read: 0] |
| +0x2A | s16 | currently-alive count (the scan of `+0x34`; an entry dies when its obj is gone/disabled/dead — `Drone+0x4F8 & 0x600`, health ≤ 0 — or not an NDrone2 obj, `obj+0xFF` ≠ 2) | [M 0x137e68:88-125] [L-read: 1 and 0] |
| +0x2C | u16 | waves spawned so far (done check: waves ≤ this) | [M 0x137e68:126/147] [L-read: 1 and 0] |
| +0x2E | u16 | the count copy — the bound of `SpawnDrone`'s `Rand_Rand` template pick | [M 0x137aa0:31, 0x137cb0:21] [L-read: 2] |
| +0x34 | ptr | the live-drone `obj*` array `[max_alive]` (malloc'd at Init; freed by `DroneSpawner_Delete` 0x13a338; slots cleared by `DroneSpawner_DroneDelete` 0x13a3b8 when a drone dies) | [M 0x137aa0:46-51, 0x137e68:90-123/218-220] [L-read: group 1 slot 0 = a live spawned obj] |
| +0x38 | ptr | the template record array `[count]` × 0xD0: +0x00 u32 spawned-this-epoch, +0x04 the template's cel (obj+0x20), +0x10..0xCF the template drone's DIVars image (`Drone+0xC00`, 0xC0 bytes — the placement params the spawn recreates: `NDrone2_CreateObj(DIVars)` then `DCVars+0xC10/+0xC20` = pos/facing, `+0xC34..` the rest, `+0x144/+0x148` = DIVars+0x54/0x60) | [M 0x137aa0:52-66, 0x137cb0:39-78] [L-read: allocated for both] |

`DroneSpawner_Init` 0x137aa0 harvests the group's placed drones into the records (each template is disabled and
deleted; `DAT_00298f18[group] = 0xFFFF` consumed; a count of 0 → the spawner disables itself and clears its table
slot). `DroneSpawner_SpawnDrone` 0x137cb0 picks a random unused record (skipping used ones), requires the spot
≥ `+0x18` from the player and not visible to him unless `+0x1F`, creates the drone, stamps `obj+0xEC = Rand(10000)`
and links it as `DCVars+0xC00`.

**The p7 disagreement (do not resolve silently).** fix-actors (docs/WORKLOG.md "## fix-actors",
docs/systems/actors.md §5, crates/nf-game/src/spawners.rs:26-30) says p7 = the **ARM** channel: "Control latches
`ch[p7] → armed`; an armed spawner needs no trigger", and the pre-fix port (kill reading) made 07000011's stairwell
spawner die instead of arriving — pinned by `fix_actors_live::the_spawner_arrives_on_its_channels` (the group-1
templates are held back before the touch, arrive after ch 0x1D). My read of the same function: the latch at
0x137e68:50-52 (`ch[p7] → SP+0x20 = 1`) skips the whole spawn machinery (`if (cVar4 == '\0')` at :86 wraps the
alive-scan + done-channel + wave-rollover + spawn loop) and falls into the 0x138014 block which disables spawned
drones (unconditionally on most levels, `obj+0xFC == 0.0`-gated on 0x11/0x12/0x13/0x16/0x4A) and then the spawner —
the old docs/systems/triggers.md:51-54 "p7 kill channel" wording. Both readings agree on the field offsets and the
latch; they disagree on what the latch does. [L] shape note: 07000005's group-2 spawner carries p4 = 0x22 AND
p7 = 0x43 (two independent plates), and its group-1 sibling (all channels 0) had already spawned one of its two
snipers at snapshot time — under the kill reading a no-trigger spawner spawns at load (`+0x1E = 1` from Init),
which matches that [L] state.

---

## 3. The ally states (0x1C–0x28) and the drone fields they use

Handler shape: `(DCVars* p1, Drone* p2, obj* p3, MsgObject* p4)`; `DCVars` = the 16-byte header {+0 obj*, +4
Drone*, +8 cel, +0xC SM*} (built by `Drone_DCVfromOBJ` / on the stack in `Drone_BulletHit` 0x13a9f8:26-31);
`Drone` = `obj+0xE0` (the big drone struct; its `+0x108` = the StateMachineInfo the docs call "obj+…" in places).
Every state also answers msgs 6/7/8/0x17/0x18/0x19 via `DroneFunc_HandleImpact` and 0x21 via
`DroneFunc_ConsiderExplosive`; only the drone fields are listed here.

| Offset (struct) | Type | Meaning | Proof |
|---|---|---|---|
| Drone+0x854 | ptr | **the current mission-route node pointer** (an AIRoute node; 0 = none). Every Lead-family ENTER re-arms it to `Drone+0x970` | [M*] writers: AllyLead 0x15e698:20, AllyLeadPlayerInWay 0x15ea28:21, AllyLeadHide 0x15ec08:20, AllyLeadWait 0x15eda8:21, AllyLeadMissionWait 0x15efc8:22, AllyLeadBondCombat 0x15f120:20 (all `= Drone+0x970`), `NDrone2_MoveToGoalPosition` 0x1517d8:24, `NDrone2_FollowRoute` 0x155be0:42 (= the AIRoute), `NDrone2_InvalidateAttackRoute` 0x155020:6 (= 0), `NDrone2_ReFindMissionPath` 0x153a30:202 (= 0), `NDrone2_FindCover` 0x152548:182 (= 0); readers: `RunForCover` 0x169150:64/69 (node+0x88), `OpenDoor` 0x1681f8:54, `DroneFunc_CombatState` 0x148338:100/156, `NDrone2_DoorIsOpen` 0x14aaa0:8, `BOTSTATE_isPathWithinObjectRange` 0x128f80:74 |
| Drone+0x970 | — | the ally's embedded route storage (`+0x854` points into it; `AllyLeadMissionWait` ENTER walks `NDrone2_MoveTo_RouteNode(1, Drone+0x970)` 0x15efc8:25; `AllyGoToGoalPosition`'s port comment calls +0x970 the goal) | [M 0x15e698:20, 0x15efc8:22/25] |
| Drone+0x9F2 | u16 | **route-set flag** (0 = no mission route): gates LeadInit's idle-vs-lead and the wait transitions | [M*] readers: AllyLeadInit 0x15e4f0:25/43, `ReFindMissionPath` 0x153a30:101/256/268, `DroneMove_ObjectMissionPathDistance` 0x153f50:86/137/144, InitPatrol 0x1724f0:33, PartyGirlInit 0x1728c0:20, Idle 0x15bf18:37/87, AstronautLaunch 0x16dd28:22, PartyGirl 0x160140:26, CivilianPatrol 0x15d610:21, CivilianInit 0x172730:31/39, Alert 0x15c248:25/62, TruckDriverInit 0x160bd0:47; filled by `NDrone2_AssignAIPath` 0x154190 (the port's note: InitAIPath 0x153720 fills it as the route count) |
| Drone+0x9F8 | f32 | a route-distance the AllyLead update thresholds (≥ 25 → stay leading, ≤ 15 → stay; between → BondCombat unless `SM+0x20 ≥ 0` and `DAT_00298f80` = 0). No writer found in this pass | [M1 reader 0x15e698:80/83] |
| SM+0x20 | f32 | scratch: the mission-path distance to Bond, written every Lead/Wait/BondCombat tick via `DroneMove_ObjectMissionPathDistance` 0x153f50 (5.0 = 0x40A00000); 0 = no route → straight-line checks; ≤ −7 (Wait) / < 0 (BondCombat) = stay | [M*] writers 0x15e698:60, 0x15eda8:35, 0x15f120:31; readers same functions :61/79/33 |
| Drone+0x12C | u32 | **AllyLeadMissionWait's wait channel id** — read once at ENTER and stored to Drone+0xCD8; 0 → 0x99 | [M 0x15efc8:17-21] — note: `Drone`+0x12C, not `obj`+0x12C as the port comments say |
| Drone+0xCD8 | u32 | **the active wait channel** while in MissionWait (UPDATE: `switch_channels[+0xCD8]` → Lead 0x1D). Elsewhere this slot is the special-death marker (drone-damage.md) — shared storage | [M 0x15efc8:21/31] |
| Drone+0x5A2 | u16 | **the return state** — written to 0x1F (AllyLeadHide) before `AllyGoToGoalPosition` 0x28 in every hide branch; `AllyGoToGoalPosition` returns to `Drone+0x5A2` when the goal is reached (MoveTo result 3) or on msg 0x1B | [M*] writers 0x15e698:96, 0x15ea28:56, 0x15eda8:60, 0x15f120:44; reader 0x15f938:28/58 — the port's ally.rs:104 says "+0x5A6" (wrong slot) |
| Drone+0x6F0 | ptr | the goal `AIPoint*` (`NDrone2_DistanceToAIPoint`) | [M1 0x15f938:22] |
| Drone+0x704 / +0x710 | ptr / VECTOR16 | the goal `CelPos*` and goal position for `NDrone2_MoveToGoalPosition` | [M1 0x15f938:23-25] |
| Drone+0x170 | obj* | the follow/lead's angle target = the current opponent (`NDrone2_SetOpponent` 0x1422d0:20; cleared in `Drone_Delete` 0x136ce8:54) — AllyFollow/FollowWait turn toward it | [M*] readers 0x15f4e0:28, 0x15f6a0:28 |
| Drone+0x228 | u32 | vision flags; **bit 2 (4) = Bond visible** — AllyFollowInit's UPDATE gate to AllyFollow 0x25 | [M 0x15f3c8:23; same bit read by CivilianScared 0x15d3a0-family and drone.rs:2567] |
| obj+0x4F8 | u32 | drone flag word; **\|= 0x10000 on every ally-state ENTER** (all 13 states); the armed-peer scan requires `& 0x600 == 0`, `& 0x100 != 0`, `& 0x4200 == 0` on the peer | [M*] 0x15e698:21, 0x15e4f0:24, 0x15ea28:20, 0x15ec08:18, 0x15eda8:20, 0x15efc8:24, 0x15f120:19, 0x15f3c8:16, 0x15f4e0:21, 0x15f6a0:21, 0x15f938:20; peer test `NDrone2_NearArmedDrone` 0x177420:77-89 |
| Drone+0x2B8 | obj* | **the armed-peer snapshot**: `NDrone2_NearArmedDrone` 0x177420 clears it at entry and stores the found peer (a TEAM-1 drone only — the candidate gate `Drone+0x44 == 1` at :110 — alive, with a HITTEST whose `+0x62` weapon-class byte is a gun class, within the radius in 3D and seen from the head). Callers: AllyLead/PlayerInWay/Wait/BondCombat radius 10.0 (0x41200000); `NotNearArmedDrone` 0x177640 (AllyLeadHide) radius 14.0 (0x41600000) | [M 0x177420:55/126; every ally-state UPDATE] |
| obj+0xCC0 / +0xCC4 / +0xCC8 | u8 / u32 / u32 | the generic state-timer registers: AllyLeadHide arms one-shot `+0xCC0 = 1`, deadline `+0xCC4 = GameState._52_4_ (the global tick) + 2·FRAME_RATE_INT`, `+0xCC8 = 0`; the expiry drives msg 0xC → `NDrone2_ReFindMissionPath` → AllyLeadWait 0x20 | [M 0x15ec08:35-38/56-62] |
| obj+0xCD8 (as latch) | u32 | AllyLeadHide's one-shot latch: ENTER = 0; UPDATE: if 0 and `NotNearArmedDrone(14)` → set 1 and arm the timer; non-zero → stay hiding | [M 0x15ec08:22/27/35] |
| Drone+0x4D8 | ptr | the behaviour **property block** (8×u32 bitfield words); props are looked up by id through `behaviour_util_getProperty` 0x1c7d98 (`bitDescs[id*2]` = word|shift<<3, `bitMasks[bitDescs[id*2+1]]`) — AllyLeadInit's prop 0x1D (AssignAIPath(8)) reads here | [M 0x15e4f0:20/27] |

State transitions the port must model (all [M], from the named decomps): AllyLeadInit 0x15e4f0 (ENTER: prop 0x1D
→ `AssignAIPath(8)`; no route `+0x9F2` → stand idle + clear prop 0x1D; UPDATE: Bond < 3.0 → 0x20 AllyLeadWait when
routed), AllyLead 0x15e698 (UPDATE: armed peer ≤ 10 → FindRunToPoint ? 0x28 : 0x1F; `UpdateMissionRoute` 0x154a80:
3 → 0x23 Done, 9/0xC → anim 0x21; Bond in the 0.5265-rad cone and < 3.0 → 0x1E; the `SM+0x20`/`+0x9F8` bands →
0x22/0x20/stay), PlayerInWay 0x15ea28 (UPDATE: no armed peer and not (Bond in the 1.06-rad cone and < 3.5) → 0x1D),
Hide 0x15ec08 (above), Wait 0x15eda8 (mission-path ≤ −7 stay; = 0 → Bond < 5 stay; else 0x1D), MissionWait
0x15efc8 (channel on → 0x1D), BondCombat 0x15f120 (path < 0 stay; `DAT_00298f80` (a global alert?) ≠ 0 stay; else
0x1D; armed peer → FindRunToPoint/0x1F), Done 0x15f2e8 (terminal stand), FollowInit 0x15f3c8 (alert 1; `+0x228 & 4`
→ 0x25), Follow 0x15f4e0 (Bond < 4.0 → 0x26; else `MoveToObject(2.0)` non-0/1/2/10 → 0x26), FollowWait 0x15f6a0
(Bond < 8.0 stay; MoveTo 0/1/2 → 0x25), FollowDone 0x15f858 (terminal), GoToGoalPosition 0x15f938 (above).

---

## 4. The mission-fail path

`DroneFunc_SetMissionFailReason` 0x13e560 (takes the drone only for the 0x10 arm):

| Input | Action | Proof |
|---|---|---|
| reason 1/3/6/9 | `Mission_SetFailLabel(0x4000034 "Killed Civilians")`; `switch_channels[0x60] = 1`; `DAT_00270310 = GameState._52_4_` (ch 0x60's time stamp) | [M 0x13e560:11-17] |
| reason 2/4/5/7/8/10/11/12 and default (e.g. 0xF) | `Mission_SetFailLabel(0x4000033 "Cover Lost")` (group only); `switch_channels[99] = 1`; `DAT_0027031c = GameState._52_4_` | [M 0x13e560:19-31] |
| reason 0x10 | `Switch_GetNearest(Drone+0xC → obj+0x30)` → `Switch_GetSwitchChannel`; if the channel == 0x61 → `Mission_SetFailLabel(0x4000032 "Alarm Raised")`; writes `switch_channels[ch] = 1` + `switch_channels_time[ch] = tick` | [M 0x13e560:32-42] |

The monitor `Mission_MonitorObjectives` 0x1827d8:98-107 turns ch 0x61 → 0x4000032 + ch 99 and ch 0x60 → 0x4000034 +
ch 99 (`DAT_0027031c` stamped) — the standing hooks fix-actors wired the port to.

`DroneFunc_OnInitDeath` 0x145bd0 — reads (all [M], every reader listed where single):

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| Drone+0x2B4 | obj* | **the killer** (last hit by). `bVar7` = killer exists and `killer->obj+0xFF == 3` (a drone-class killer — NOT the player, whose obj type is 2/0x11) | [M*] writers `Drone_BulletHit` 0x13a9f8:23, `NDrone2_DealWithObjHit` 0x13fff0:304, `Drone_ExplosiveHit` 0x13aab8:20, `Drone_Delete` 0x136ce8:69 (clear); readers 0x145bd0:18-19/50/66, `Drone_Delete` 0x136ce8:68 |
| Drone+0x44 | u8 | **the team byte** (1 = team 1 guards; else the team 2/3 protected classes) | [M*] readers 0x145bd0:21/58, `NDrone2_DealWithObjHit` 0x13fff0:188/196 (MP), `NDrone2_PunchImpact` 0x145f28:60, `NDrone2_NearArmedDrone` 0x177420:110; writer: the DefaultInit/DMODE family (see actors.md §1) |
| Drone+0x4F8 | u32 | flag word; **bit 0x1000000 = the KO flag** — OnInitDeath skips the whole fail/stat/talk block when set (the grenade-drop tail still runs) | [M*] writer `NDrone2_DSTATE_KnockedOut_Anim` (drone-damage.md); readers 0x145bd0:22/57, the armed-peer scan 0x177420:79, the spawner's alive scan 0x137e68:101 |
| Drone+0x4D8 | ptr | the property block — **this is where props 0x53/0x54 live**: `behaviour_util_getProperty(0x54/0x53, Drone+0x4D8)`, NOT a fixed drone offset. Writers found: `NDrone2_DoModeSettingsOLD` 0x14cfa0:97-98 (both = 1), `init_DMODE_Civilian` 0x14dd28:21 (0x53), `init_DMODE_CivilianScared` 0x14de48:17 (0x53), `GetDTYPENEW` 0x14c938:39 (0x54 = 1, the 07000007 party types); other reader: `HostageDead` 0x172628:17 (0x53) | [M 0x145bd0:26/30/83/87] |
| Drone+0x3A | u8 | the silent-kill flag (head-damage ×100 sets it) — suppresses `NDrone2_DeathTalk` | [M*] reader 0x145bd0:63/74-79/106; writer `NDrone2_HitDamage` (drone-damage.md) |
| Drone+0x19 | u8 | the grenade-drop-on-death flag: != 0 → `DroneFunc_DropGrenade` (0x35 with arg 2 on levels 0x7000001-8; 0x34 on 0x9/0xA/0xB/0x11-0x16/0x4A) | [M 0x145bd0:76-79/114-145] |
| glb_players / DAT_00298fb0 / DAT_00298fb4 | obj* | the player obj (MP: DAT_00298fb4 when DAT_00298fb0) — killer == it → `PlrStat_LogEnemyDispatched(0)` when team 1 and not KO (MP: killer == `Drone+0x170` opponent instead) | [M 0x145bd0:45-62] |

The fail verdict [M 0x145bd0:26-105]: **team 1** — prop 0x54 set: a drone killer → reason 2; a player kill
re-checks prop 0x53 (set → reason 4, unset → **no fail**). Prop 0x53 only (0x54 clear): drone killer → 2, player
killer → 4. Neither → no fail. **Team 2/3** — prop 0x54 set: drone killer → 1; player kill re-checks 0x53 (set → 3,
unset → no fail). Prop 0x53 only: drone killer → 1, player killer → 3. (Because the DMODE writers set 0x53+0x54
together on the protected classes, the observable rule stays "team 1 → 2/4, team 2/3 → 1/3".)

---

## 5. Port cross-check (report only — nothing fixed)

- `crates/nf-game/src/ally.rs:52-69` `on_init_death`: team 1 maps prop54 → 2 **regardless of killer** and prop53 → 4;
  the decompile's prop54+player-kill arm re-checks prop53 (no fail when unset) and prop53+drone-killer → 2. Team 2/3:
  the port's prop54 → drone?1:3 and prop53 → 3; the decompile's prop53+drone-killer → 1 and prop54+player-kill →
  (prop53 ? 3 : nothing). Corner-case-only mismatches while the props co-occur; ally.rs:56-57/63-66.
- `crates/nf-game/src/ally.rs:104` says AllyGoToGoalPosition returns to "+0x5A6"; the decompile reads `Drone+0x5A2`
  (0x15f938:28/58). ally.rs:85 says the MissionWait channel is "obj+0x12C"; the decompile reads `Drone+0x12C`
  (0x15efc8:17).
- `crates/nf-game/src/drone.rs:3236` models the MissionWait channel from `self.second_channel` (the port's param-6
  slot = game `Drone+0x135` per drone-damage.md); the game reads `Drone+0x12C`. Reachable only via a forced state
  today, so low impact.
- `crates/nf-game/src/drone.rs:3219/3226` reproduce the PlayerInWay/Hide gates (10.0/14.0, dist 3.0); the decompile's
  PlayerInWay also requires Bond outside the 1.06-rad cone (0x3f860a92) before returning to Lead — the port drops the
  angle term (drone.rs:3219).
- `crates/nf-game/src/spawners.rs`: field map matches (mode/group/max/waves/start/stop/done/arm/min_dist, the
  template + live arrays, the trigger consumption, the stop-channel pause/delete, the done-channel wait). The
  **armed branch is not modelled**: the port keeps a `killing` latch fed by the caller (spawners.rs:63-66,80-83)
  instead of the decompile's armed-path cleanup (0x137e68:235-288) — consistent with the arm reading, inconsistent
  with my decompile reading; see the disagreement above.
- `crates/nf-game/src/damage.rs:83` `hit_damage` takes `dmg` = HITDATA+8 · `Drone+0x14C` and `part` = HITDATA+0x52 —
  matches `NDrone2_DoHitEffects` 0x1458e0:104-105. No HITDATA struct exists in the port (the bullet path computes
  the part/point directly) — the fields above are what a future port of the effect/debris layer would need.

## Disagreements

1. **Spawner p7 (+0x14 → +0x20 latch): arm vs kill.** fix-actors/actors.md/spawners.rs: arm — an armed spawner
   spawns without the trigger (playtest C2 evidence, `the_spawner_arrives_on_its_channels`). My decompile read of
   `DroneSpawner_Control` 0x137e68: the latch routes to the 0x138014 cleanup block (disable spawned drones + the
   spawner; `obj+0xFC`-gated on 0x11/0x12/0x13/0x16/0x4A) and skips the spawn machinery — the old
   triggers.md:51 "kill channel". Both agree on the offsets. [L] 07000005: the group-2 spawner holds p4 = 0x22 and
   p7 = 0x43 (independent plates); the all-zero-channels group-1 spawner had already spawned 1 of 2 snipers at
   snapshot time (kill reading predicts no-trigger = spawn at load).
2. **HITDATA list direction**: `Coll_AddHitToList` builds head-at-newest with `+0` = next-newer / `+4` = next-older,
   and every consumer walks `+4` (newest → oldest). The `CompHitData` sort order (ascending/descending `+0xC`)
   depends on QuickSort's comparator convention — not re-derived here; the game's "first hit wins" behaviour is
   pinned by drone-damage.md's box-order finding instead.
3. **OnInitDeath's prop corners** (port vs decompile) — see §5 bullet 1.

## Live checks for the main agent

Group 1 (HITDATA): 07000005, shoot a guard in the torso then the head with the P2K. With a PINE watch on a HITDATA
node (find it via a bullet obj: walk `obj+0xD0` of the victim right after the shot): expect `+0x08` = 3.5 × multipliers,
`+0x52` = 4 (torso) then 5 (head), `+0x40` = the impact point on the body, `+0x10` pointing back along the shot,
`+0x50` = 0x10 (flesh), `+0x58` = the bullet obj whose `BU+0x30` = the player. Then shoot a window/prop and check
`+0x54` != 0 (static cel) vs a drone hit (`+0x58` != 0, `+0x54` = 0). A grenade near two guards: each victim gets a
mirrored node (`+0x51` bit 0 set, `+0x58` = the explosion owner) — `Explode_Propagate` 0x17c518:215-217 wrote it.

Group 2 (spawner): 07000005 — (a) confirm the field map [L, done here]: group 1 = snipers (all channels 0,
maxalive 1, already 1 alive), group 2 = p4 0x22 / p7 0x43 / maxalive 2 / mindist 4.0; (b) **settle the p7
disagreement**: walk the gate area so ch 0x22 fires and let both gate guards spawn, then step on the wall-hug plate
#301 (x 23.9-25.9, z 70.8-72.2) → ch 0x43. Kill reading: both guards vanish within a frame or two (and the spawner
disables). Arm reading: nothing changes. Also 07000011: watch whether the group-1 stairwell templates are absent at
load (both readings need a non-zero p4 to explain that) and dump that spawner's SP+0x0E/+0x14 from a live dump.

Group 3 (ally/mission-fail): 07000001 (Mayhew) — let a guard shoot Mayhew dead: `Drone+0x2B4` = the guard obj
(type 3), team = 2, props 0x53/0x54 set → expect ch 0x60 + FailLabel 0x4000034 + MissionState 2 (the port's test
pins this); then kill Mayhew yourself (type-2 killer) → reason 3, same channels. 07000007 — scare-don't-kill a
guest (the sister test): no channel writes. Watch `Drone+0x2B8` on an escort during a firefight: it should hold the
nearest armed TEAM-1 guard's obj while `NearArmedDrone(10)` is true, 0 otherwise — and never an ally's or Bond's.
AllyLeadHide: after the last armed guard leaves the 14-unit band, `obj+0xCC0` = 1 and `obj+0xCC4` = tick + 120, then
msg 0xC two frames later and the escort resumes the lead.

## Unresolved

- Drone+0x9F8 (f32, AllyLead's 15/25-band input): no writer found in this pass; likely written by
  `NDrone2_UpdateMissionRoute` 0x154a80 or the route follower — needs its own pass.
- `DAT_00298f80` (AllyLeadBondCombat's stay gate) and `DAT_00270310/0x1c/0x20` (the ch 0x60/99/100 time stamps):
  globals confirmed by use, owners not traced.
- HITTEST_tag's own field map (+0x62 weapon-class byte, +0x8C radius, +0x90 damage, +0x94 shape type, +0x98/0x9A
  request flags, +0x9C result flags) is only sketched here from the readers HITDATA needs.

## Live result: p7 is the KILL channel (main agent, 2026-10-12)

GDB-server PCSX2, 07000005, nobody playing. Channels written directly in `switch_channels` 0x26FC90 (PINE); RAM
from slot-9 save states (`drones_snapshot.py`); gate spawner object 0x016da670, data block 0x016da780.

| Step | Spawner block | Drones |
|---|---|---|
| start | +0x0E = 0x22, +0x14 = 0x43, +0x1D = 1, +0x1E = 0, +0x20 = 0 | 13 |
| ch 0x22 = 1 | +0x0E → 0 (consumed), +0x1E → 1 | **15**: two gate guards spawn at (28.3, 37.2, 44.9) and (32.1, 37.0, 44.9) |
| ch 0x43 = 1 | +0x20 stays 0 at first: ch 0x43 plays a script whose event calls `Drone_EnableAll(0)` (`Script_EventHandler` 0x1d94c0), so `NPCGlobals` byte 0 = 0 and `DroneSpawner_Control` returns at its first line. A read watchpoint on ch 0x43 saw only `amemcpy` (the per-frame channel copy) | 15 |
| script ends (`Script_Free` re-enables NPCs) | **+0x20 → 1**, +0x1E → 0, both slots zeroed, the spawner's `obj+0xFE` → 1 (it deletes itself) | **13: both gate guards removed** |

So the decompile's cleanup branch (0x137e68:235-288) is what runs: **p7 = the kill channel**, triggers.md's original
reading. fix-actors' ARM reading is refuted. Its 07000011 playtest symptom fits the cleanup's level split: on
0x11..0x13, 0x16 and 0x4A only drones with `obj+0xFC` == 0 are removed. The port fix is queued as `046b-fix-spawner-p7`.
