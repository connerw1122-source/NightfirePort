# models-misc — vehicles, effects, broken pieces, front-end, cut-scene props

Lane 044: every remaining model class — the vehicles and big movers, the effect meshes, the broken
states, the front-end models and the script-only props. Companion to `models/characters.md` (042) and
the world/weapon lanes. Every claim is tagged [M] (code, function + address), [D] (disc bytes),
[L] (EE snapshot), [S] (the port's suggestion where the decompile is incomplete).

The draw that every class here shares: a dynamic object's visible mesh is `obj+0xD8` (the glist) drawn
at `obj+0x90` (the matrix) by `View_DrawObjList` 0x1e69a0 → `View_DrawObjectGlist` 0x1e6780 [M]. A
creator attaches the mesh with `Control_SetGList(obj, glist)` 0x134960 (which also copies glist+0x44
into obj+0xF0), a direct obj+0xD8 store, or `hashtable_set_object_to_entity_gfx(obj, hash)` 0x1ca6d8.
`control_create_object` 0x1344e0:20-29 builds obj+0x90 from the placement pos/rot, so a creator whose
update never moves the matrix draws exactly a static instance at the placement transform — which is
what the port's `misc_static_sweep` (scene.rs) adds. Pass selection: `View_AddObjects` 0x1e62d8 skips
objects with obj+0xF0 bit 0x10/0x8000, puts bit-0x1 objects in the alpha list and bit-0x2000 objects in
the post list; `psiDrawObjectMatrix` no-ops on an empty glist, so collision-only objects draw nothing.

## The kind census (what the disc carries)

`nf-extract kind-census` (this branch) counts, per level archive: kind → placements / resolved / with
gfx / with collision. Models-lane highlights of the 35 archives (27 on-foot story + MP + front end):

| kind | creator [M] | placements | with mesh | drawn by the game? |
|---|---|---|---|---|
| 0x2E lock | `Lock_Create` 0x1ae1f8:55 | 24 | 24 | yes (direct store; the unlock spline starts here) |
| 0x30 monitor | `Monitor_Create` 0x1b08b0:26 | 17 | 17 | yes (Control_SetGList) |
| 0x34 gun imp | `GunImp_Create` 0x131f20:68 | 15 | 15 | yes (parent + a TurretGuns child) |
| 0x35 PCQ worm | `PCQWorm_Create` 0x1b07a8:12 | 7 | 7 | yes |
| 0x3A grapple | `Grapple_Create` 0x198298:17 | 57 | 57 | yes |
| 0x3B dynamic object | `DynamicObject_Create` 0x18dbf8:19 | 75 | 74 | yes (param 0 ≠ 0 hides until its channel) |
| 0x3F creep wall | `CreepWall_Create` 0x196fc8:23 | 5 | 2 | yes |
| 0x71 rotor | `rotor_init` 0x18f4b8:24 | 15 | 15 | yes (spins obj+0x50, not the matrix) |
| 0xD5 one-sided | `OneSided_Create` 0x1e7ce8:14 | 1 | 1 | yes (per-frame backface self-cull) |
| 0xD8 rigid body | `RB_Create` 0x1d6bf0:52 | 35 | 35 | yes (RB_EulerEvolve once disturbed) |
| 0xE1 destroyable | `Destroy_Create` 0x12c318:23 | 189 | 187 | yes; `Destroy_Smash` 0x12bb60 swaps to the block-0x27 variant when one exists |
| 0xE3 flicker | `Flicker_Create` 0x1acb00:49 | 2 | 1 | yes |
| 0xDA car | `Car_Create` 0x1305b0:20 + `Car_InitBits` 0x12fec8 | 29 | 29 | body + the fixed parts 0x2000502/0x2000505 (MP levels) |
| 0xFA third icon | `ThirdIcon_Create` 0x197070:33 | 44 | 10 | yes when the glist has meshes (the rest are collision-only and draw nothing in the game either) |
| 0xFD cam subject | `CamSubject_Create` 0x1bc5d8:17 | 5 | 1 | yes when the glist has meshes |
| 0xFE mine | `Mine_Create` 0x17ce28:50 | 10 | 10 | yes |
| 0x4A apocalypse | `Apocalypse_Create` 0x18dcc0:20 | 26 | 8 | yes where the glist has meshes (07000014/15/16/41's resolve to collision-only glists — the game draws nothing, only the rumble/shake [M 0x18d4f0]) |
| 0xD4 cloud | `Cloud_Create` 0x18d620:26 | 2 | 2 | sky pipeline (`View_SetDrawInNoViews` + `View_AddSkyObj`, slot = param 4 [M 0x18d620:27]) |

Kinds whose creators attach NO mesh — the port must not draw them (the sweep asserts this):

- **0x29 switch** (`Switch_Create` 0x1adcc0:35: `*(obj+0xD8) = 0`) — the switch mesh is baked level
  geometry; the anim rides its SP sub-object.
- **0x31 fusebox** (`FuseBox_Create` 0x1b0448:25: obj+0xD8 = 0) — 402 placements, 397 with gfx on their
  objects, none of it ever attached. The visible box is static geometry; the lid animates via the box's
  SP script (0x06000A3B on 07000014).
- **0x27 gas trap** (`GT_Create` 0x17ffa0:78: obj+0xF0 |= 0x10 — hidden at create until deployed).
- **0x32 rainbox** (`RainBox_Create` 0x18d840:19 attaches the glist, but `RainBox_Clip` 0x18d0d0 only
  ever reads its AABB and the object is never room-linked) — the 39 mesh-bearing rainboxes never draw.
- **0x38/0x39 space laser/missile** (glist arg 0 [M 0x1aa8b8:13, 0x1a9ef0:14]; visuals via scripts).
- **0xF2 emitter** (`Emitter_Create` 0x17ae00:100: no glist ever; drawn by `Emitter_Draw` 0x17b708
  particles).
- **0x36/0x37 subs** (`Sub_Create` 0x1ad6c0, `MiniSub_Create` 0x182130 — AnimObject 0x5000068/0x5000074
  + anim scripts): **no placements on any on-foot main map** [D kind-census], so nothing to draw.

## Port status per lane item

**Static dynamic entities — ported this lane** (`misc_static_sweep` in scene.rs, `Scene::misc_drawn`
per kind; the census test recomputes every level's expectation from the placements independently):
0x2E, 0x30, 0x34 (parent), 0x35, 0x3A, 0x3B (visible ones), 0x3F, 0x71, 0xD5, 0xD8, 0xE1, 0xE3, 0xDA
(+ the two fixed parts), 0xFA/0xFD/0x4A (mesh-ful only). Drawn as World-layer instances at the placement
transform with the 0x800 ambient dim and the mirror-only (glist 0x400) skip [M 0x1e62d8:38-47].

**Explosion visual — ported this lane.** `Explode_Create` 0x17cbd0 Script_Loads **0x6000052** on every
disc caller (0x12cd98, 0x12ce80, 0x17ffa0, 0x180498, the copter's 0x17dbb8 case 6), aligns the script
matrix to the blast direction (0x2d6210/0x2d6220) and plays it at scale = radius·0.06666667 [M
0x17cbd0:84-106]. The script's ~40 entities are keyframed glist meshes: `cloud_1` 0x0200021a (4v/2t),
`burst` 0x0200021e, `smoke` 0x0200021d, the 16 `Blast_frame_N` 0x020003d7…0x020003e8 spawned one per
tick, `Particle_frame` 0x0200042d sparks and the 120-tri `Blast_radius_ball` 0x020006a7 [D the script
decode, identical across levels]. Port: the sim spawns a one-shot script player per blast
(`spawn_once_scaled`, scale radius·0.06666667) and the scene's script-entity attachments pose them at
their live matrices. *Still wrong*: the game's `Mat_Align2Dir` tilts the blast toward the explosion
direction — the port's blasts play axis-aligned [S]; overlapping blasts share the newest live entity
set (`script_entity` finds the newest object).

**Smoke and gas — ported this lane.** One shared mesh: `Gas_Init` 0x1aacc8 attaches 0x0200039
`smoke_1_dust` (4v/2t [D gfx-list]) to every puff [M 0x1aacc8:81]. Scale starts at param 1 and grows by
(param 2 − param 1)/life; the rgba starts at the packed param 9 (the muzzle's 0x48403080) and fades to
the callers' ~0; the spin is `Rand_FRand(π/2·REC)` sign-flipped by a coin; velocity by type (4 given,
7 wind + jitter, default upward drift). The update `gases_smoke` 0x1ab2d0 damps/floats the puff, fades
the rgba bytes at obj+0x103..0x106 and kills at life ≤ 0. Ported: `nf-game/src/gas.rs` (types 0/4/7),
the muzzle puff (`Player_MuzzleFlash` 0x1a4298:91-103, skipped by weapon flag 0x2000), the
broken-searchlight puff (0x1cb900:373-385, every 7th frame), drawn as billboarded quads with the live
rgba. *Deviation*: the game's 5 shared-RNG draws per puff are not consumed (G-033's discipline) — the
jitter comes from a counter hash; the decomp drops both call sites' life argument (8/24 ticks [S]).

**Muzzle flash — resolved** (previous lanes; scene.rs Flash layer, roll/scale/alpha randomisation per
`Draw_MuzzleFlash` 0x1a3fe0). *Remaining*: the muzzle dynamic light's radius/colour wiring, the
+0x51/+0x52 per-weapon alpha range, and the 0x2000039 smoke puff IS this lane's gas (done).

**Coronas (0xFC) — resolved** (corona.rs + scene.rs; the 13/13 golden). The searchlight family's
corona alpha byte/scale are the known G-033 remainder (the "white blobs").

**Sky clouds (0xD4) — ported this lane**: `sky_instances` also carries them (slot = param 4 [M
0x18d620:27], the variant flag only applies to 0x2A). 07000041 and 07000006 each carry one.

**Front end — measured, no 3D models to port.** 07000048's main map (0100009a) is 291 textures, ONE
placement (a 0x31 fusebox — a meshless-creator kind, correctly undrawn) and no 3D geometry placements;
the four sub-maps' objects are sprite/menu resources [D block dump + kind census]. The game's front-end
visuals are the kind-8 menu-def archive entry (`LoaderProcess` 0x1d4880 case 8 → `MenuManager_Load`)
drawn as sprite pages by `MenuManager_Create` 0x21b908 — a sprite pipeline the port does not have
(fix-frontend is its own task; the entry exists, asserted in models_misc_live::frontend_level).

**Cut-scene props (`Script_CreateEntity` 0x1d8968) — resolved** by the earlier lanes' machinery
(nis.rs records `Obj::Entity`; scene.rs poses the non-trigger kinds via `level.by_hash` at the live
matrix — the OBJECT_Carriage path). *Remaining (other lanes' scope)*: kind-specific sim objects for
script-spawned breakables/destroyables/pickups (G-031).

### Still wrong / remaining, with the mechanism

1. **Lightning** — entirely unported. The mechanism [M]: placement 0x2C → `Lightning_Create` 0x18cde8
   makes a controller + 8 hidden beam objects whose endpoints chain the placements that share the
   16-bit id (placement param 1 — 07000001 carries an id-1 chain of 6 and an id-2 chain of 5, 07000041
   two pairs [D placements dump]); `Lightning_Update` 0x18cf68, while the sky flash flag is set,
   re-jiggles the intermediate nodes with `FractalThing` 0x1a94d8 (midpoint displacement, 3
   `Rand_FRand(mag)` per node, mag = ½·dist — 21 draws per reflicker) and stretches mesh 0x0200032c
   `Lightning` (50v/32t [D gfx-list]) over each of the 7 segments via `PositionBeam` 0x1a9400 (scale =
   segment length, the billboard path). The flash driver is the 0x2A sky slots with param 1 ≠ 0
   (`View_AddSkyObj` fills the slot's variant pointers + flash length/gap; `View_DrawSky` 0x1e5ad0 runs
   the countdowns, plays sounds 0x5D/0x5E and rolls the 20 % reflicker). `level.rs` still skips the
   variant slots (G-019). Everything needed is measured; the RNG discipline question (FractalThing's 21
   draws) is the same one gas.rs answered with the counter hash.
2. **Breakables' broken states** — the intact 0x20 meshes draw (world lane's sweep); the death is
   unported. [M] `Break_Kill` 0x12b970 spawns debris per COLLIDATA triangle (`Debris_Create_Loop`
   0x1ed548), each piece a real object whose mesh = the material table HEINFO@0x3037c0 +0x74 hashcode
   (`Debris_Create` 0x1aaa00:57); glass (param 0 == 2) shatters into procedural 2-tri VU packets carved
   from the model's own triangles (`psiCreateBreakableData` 0x117418, `psiCreateShard` 0x1178e0, the
   global shard pool `UpdateAllShards` 0x12b808 / `DrawAllShards` 0x12bef8). Needs the HEINFO table
   (ELF data, the G-041 companion) and the class-0x20 damage plumbing.
3. **Destroy swap path** — `Destroy_Smash` 0x12bb60 swaps to `celglist+0x40` = the block-0x27 LOD entry
   (arg proven by disasm 0x12bbb8 `lw a1,0(s1)`); the parser landed this lane (`Map::lod_target`), and
   the C5 doors on 0700000d have NO variant (their map's block is all −1 but 010000cf's) so they stay
   drawn intact after the smash — game-accurate. The swap needs wiring into the draw when a variant
   exists (only 7-16 objects per level carry LOD entries [D]).
4. **GunImp child** — the turret child comes from the ELF's `TurretGuns` table (hash at
   `TurretGuns+gun·0x1c+0xC`, yaw/pitch animated in `GunImp_Update` 0x1320d8:162-172). Parent drawn;
   the child needs the table read (the HudTables precedent).
5. **Rotor/Apocalypse/RB motion** — drawn at rest. The rotor spins obj+0x50 by axis·rate (params 0/1
   [M 0x18f4b8]); Apocalypse's update only counts down + shake [M 0x18d4f0]; RB physics
   (`RB_EulerEvolve`) is a whole rigid-body sim — none of it moves the drawn matrix at rest, so the
   static instance is exact until disturbed.
6. **Car MP variant** — the SP default (body + 0x2000502 at the body matrix + 0x2000505 at body
   translation) is drawn; the MPSettings lift +1.5 and the 0x2000194 re-skin (data+0x112) are not.
7. **Tree (0xF3)** — anim-object path (`AnimObjectNew` + `AnimScriptAdd(0x600008c/0x600008d)` [M
   0x17c200:17-25]) — 8 placements on 07000002/04; needs the skins-path extension for hash-resolved
   rigid anim objects.

## --bond render spots (for the main agent)

One spot per level where a lane model appears; the class and what it should show:

| level | --bond x,y,z,yaw,pitch | shows |
|---|---|---|
| 07000001 | `11.1,10.5,-0.19,0,0` | the two stack rotors (0x71, kind census 2) turning over the start hall; 4 rigid bodies (0xD8) at (2.0,7.9,2.0)/(-2.9,7.9,9.0)/(-11.2,7.9,9.0) |
| 07000001 | `-17.0,26.0,-2.97,0,0` | the lightning bolt chains (id 1 and 2, param-1 chains) — currently NOT drawn (remaining item 1); the sky flash should light them |
| 07000002 | `0,10,0,0,0` near (47.5,2.2,36.8)-class streets | the 21 rigid bodies (0xD8) and 6 destroyables (0xE1); 6 trees (0xF3, unported anim-objects) |
| 07000005 | `-35.3,38.5,-31.4,0,0` | the two third icons (0xFA) at (-35.6,37.2,-37.3)/(-35.0,37.1,-25.5); fire the P2K here: the muzzle smoke puff (gas) drifts up |
| 07000005 | any searchlight head | shoot a light: the break smoke puffs every 7th tick (gas) |
| 07000006 | `-10.9,43.5,-47.9,0,0` | the destroyables (0xE1) at (-10.94,42.25,-47.47)/(-10.99,42.25,-48.29); the sky cloud (0xD4) drift |
| 0700000c | `7.1,16.5,-112.37,0,0` | the 5 locks (0x2E) stacked at (7.09,15.70/16.87,-112.37) and (4.50,15.70,-112.35); 5 grapple points (0x3A) at (61.1,12.3,-143.5)/(77.5,12.1,-143.9) |
| 0700000d | `152.7,23.0,-256.65,0,0` | the creep wall (0x3F) at (152.7,22.1,-256.7) |
| 0700000d | `137.3,10.3,-324.66,0,0` | the gun imp (0x34) parent mesh at (137.3,9.3,-324.7) (its turret child is remaining item 4) |
| 07000014 | `47.5,3.2,36.8,0,0` | the 12 monitors (0x30) at (47.5,2.2,36.8)/(32.5,1.5,21.8)/(32.5,1.5,3.9) and 22 visible dynamic objects (0x3B) |
| 07000015 | `35.6,45.2,17.5,0,0` | the 9 mines (0xFE) at (35.6,44.2,17.5)/(37.3,41.5,18.0)/(34.0,41.5,18.0) |
| 0700001b | `-55.4,-12.2,0,0,0` | the 8 Apocalypses (0x4A) in a line at y −13.19 (the only level whose 0x4A glists have meshes) |
| 07000024 (MP) | `8.4,2.8,86.0,0,0` | a car (0xDA): body + turret part 0x2000502 + wheel part 0x2000505 at (8.4,1.8,86.0); two more at (-24.5,20.6,-68.9)/(20.2,20.6,-68.9) |
| 07000001 | fire a grenade (or any blast) | the explosion: the fireball script entities scale by the blast radius, plus the smoke puffs |
| 07000048 | dev-menu level jump | the front-end archive loads; nothing 3D draws (game-accurate — see above) |

## The per-level drawn counts (the golden)

`models_misc_live::misc_census_golden` pins the kind→count map per archive as a SHA-1 over the lines
(`sha1 dc647240c3f1770d4018ce4410aef13af0008f07`); `misc_static_entities_match_the_census_on_every_level`
recomputes every count from the placements and compares. The census itself (numbers only, no bytes):

- 07000001: 0x30:1 0x71:2 0xd8:4 0xe1:11 · 07000002: 0x30:1 0x3b:1 0xd8:21 0xe1:6 · 07000003: 0x3b:1 0xd8:3 0xe1:5
- 07000004: 0xd8:2 · 07000005: 0x3a:2 0x3b:2 0xfa:2 · 07000006: 0xd8:1 0xe1:6 · 07000007: 0x3b:7
- 07000008: 0x2e:2 0xe1:4 · 07000009: 0x71:1 · 0700000a: 0x2e:4 0x30:1 0x35:3 0x3a:1 0x71:1
- 0700000b: 0x2e:1 0x35:1 · 0700000c: 0x2e:5 0x3a:5 0x3b:5 0x71:1 0xe1:12 · 0700000d: 0x34:1 0x3a:3 0x3b:4 0x71:1 0xd8:3 0xe1:9
- 07000011: 0x2e:1 0x35:1 0x3a:1 0x71:3 0xfa:2 · 07000012: 0x2e:2 0x30:1 0x35:1 0x3a:1 0x71:1 0xe1:1 0xfa:1
- 07000013: 0x71:1 · 07000014: 0x2e:3 0x30:12 0x3a:3 0x3b:22 0xe1:48 0xfa:3 · 07000015: 0x2e:3 0xe1:11 0xfe:9
- 07000016: 0x3b:2 0xe1:7 · 0700001b: 0x3b:1 0x4a:8 0xe1:6 · 07000024: 0xda:12 (4 cars × 3) · MP 23-29/4b: cars, grapples, gun imps
