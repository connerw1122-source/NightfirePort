# models-world: every level's world geometry and placed static objects (task 041)

Lane: the room/cel geometry and every static placed object on the 27 on-foot levels
(`docs/systems/levels.md` §1) — walls, floors, terrain, props, decorations (0x47), breakables in
their intact state (0x20), SimpleScript props (0xE2), doors (0xDB), sky objects (0x2A), and the
shadow/decal geometry baked into the levels. Reader: `crates/nf-formats/src/{ps2gfx,level,map,texture}.rs`;
world build: `crates/nf-view/src/scene.rs`; draw state + order: `crates/nf-view/src/gpu.rs`.
Golden: `crates/nf-view/golden/models_world.tsv` (numbers only); tests: `crates/nf-view/tests/models_world_live.rs`.
Tags: [M] measured in code/disassembly, [D] measured on the disc, [L] EE snapshot, [S] suggested.

## 1. Status: complete

Every placement of every story level resolves to an object, and every object's glist, leaves,
batches and texture REFs decode (`story_levels_match_the_model_inventory_golden` asserts it against
the golden; `total_unresolved == 0` is asserted explicitly). Per level
(`models_survey`, NF_SURVEY=1):

| level | maps | placements | statics drawn | skipped by the cel pass | leaves | batches | vertices | triangles |
|---|---|---|---|---|---|---|---|---|
| 07000001 | 54 | 331 | 110 | 2 | 644 | 4329 | 171600 | 113714 |
| 07000002 | 52 | 502 | 217 | 7 | 965 | 6068 | 230867 | 146793 |
| 07000003 | 51 | 511 | 262 | 0 | 820 | 5185 | 199349 | 129445 |
| 07000004 | 50 | 70 | 40 | 1 | 461 | 2701 | 114981 | 76945 |
| 07000005 | 51 | 363 | 114 | 18 | 713 | 4362 | 160328 | 100424 |
| 07000006 | 49 | 813 | 349 | 10 | 807 | 5412 | 214832 | 136342 |
| 07000007 | 48 | 513 | 295 | 7 | 871 | 6270 | 243729 | 161103 |
| 07000008 | 52 | 308 | 97 | 13 | 859 | 5792 | 226416 | 148364 |
| 07000009 | 40 | 288 | 169 | 3 | 774 | 4795 | 177982 | 117672 |
| 0700000a | 38 | 800 | 470 | 18 | 1156 | 7316 | 244641 | 161621 |
| 0700000b | 51 | 380 | 167 | 20 | 1093 | 7203 | 264044 | 178384 |
| 0700000c | 46 | 575 | 266 | 11 | 1046 | 7981 | 248586 | 148316 |
| 0700000d | 50 | 568 | 260 | 8 | 979 | 7175 | 237668 | 145736 |
| 07000011 | 44 | 319 | 132 | 19 | 890 | 6218 | 227489 | 153781 |
| 07000012 | 39 | 792 | 424 | 26 | 1290 | 7976 | 268693 | 176659 |
| 07000013 | 42 | 173 | 48 | 2 | 691 | 4226 | 155024 | 102210 |
| 07000014 | 55 | 1034 | 274 | 7 | 1097 | 7432 | 279230 | 180882 |
| 07000015 | 55 | 638 | 371 | 0 | 921 | 7629 | 280887 | 182319 |
| 07000016 | 64 | 360 | 107 | 7 | 785 | 5072 | 196106 | 126510 |
| 0700001b | 27 | 304 | 20 | 1 | 731 | 4152 | 150739 | 98559 |
| 07000041 | 100 | 120 | 48 | 0 | 587 | 3895 | 185925 | 122589 |
| 07000043 | 78 | 2 | 1 | 0 | 245 | 3120 | 169961 | 120925 |
| 07000046 | 93 | 75 | 9 | 0 | 594 | 3918 | 168343 | 112511 |
| 07000048 | 5 | 1 | 0 | 0 | 64 | 86 | 2004 | 1314 |
| 07000049 | 71 | 2 | 1 | 0 | 238 | 3180 | 173686 | 123478 |
| 0700004a | 42 | 294 | 64 | 0 | 839 | 5538 | 240420 | 147716 |
| 0700004b | 110 | 225 | 56 | 40 | 764 | 5985 | 250205 | 165329 |

Disc-wide over the 27 levels: 3,571,061 static vertices, 2,278,382 triangles, 47,051 leaves,
69,445 batches decoded from the PS2_GFX packets; every leaf's box min/max equals the decoded vertex
bounds bit for bit (0 mismatches, re-checked per level); no batch reads a VU1 qword that its packet
did not unpack (the decoder's generation check — the VIF-unpack-total cross-check — runs on every
batch). LOD blocks (map block 0x1f, `parsemap_block_LOD` 0x1d1f10 — the switch-channel glist swap in
`control_movement_object_handler` 0x1336f8) never occur on the disc (docs/formats/map.md's measured
block table; probe: 0 blocks in all 2,230 maps) — no hidden swap-variant models exist.

## 2. Placement types of this lane — ported and verified

| Type | Class | Draw | Evidence |
|---|---|---|---|
| 0x8000 statics (room cels 0xc023/0xc047, decorations 0x47, blockers) | World list 0 | list 0, placement-parse order | `parsemap_block_map_data_static` 0x1d1998: one runtime cel per placement; `View_AddCels` 0x1e6ba0 pushes every cel's primary glist into **list 0** with the synthetic key −90000+10k in cel-chain order, before any distance key [M decompile; the port's `Material::chain`] |
| 0x47 decorations | same | same | they are the creates_cel statics; `levels.md` §2's "drawn as PS2_GFX" |
| 0x20 breakables (intact) | World | list 0 by distance (dynamic objects) | `Break_Create`; drawn via `scene_instances(cels_only)` + the static loop [M] |
| 0xE2 SimpleScript props (key count 0) | World | list 0 by distance | `SS_Create` 0x1a9c58; animated ones (key count ≠ 0) ride the door slot space (map-tower2) |
| 0xDB doors | Character (posed per tick) | pass 2 | `Door_Create` 0x136100, `Door_Interp`; door.rs (WORKLOG 0074) |
| 0x2A sky | SkyCamera / SkyFixed | sky passes | `View_AddSkyObj` 0x1e79a0; WORKLOG 0014 |
| baked shadow decals / alpha-shaped static textures | World list 0 | **in parse order** — after the wall they lie on | the R2 hole fix (§4 below); `gpu::alpha_shaped` remains the pinned classifier |
| env-map overlays (ALPHA 0x58) / FIX=0 mod batches (0x64), 07000041's `EnvModTest_*` | World | blend 4 (Cs·Ad+Cd) / blend 3 (keep-destination) | §3 below; `EnvModTest_ModEnv`/`EnvModTest_BaseModEnv`/`polySurface2` [D probe] |

The cel-pass skip (glist flags bit 0x10, `MapObject::skipped_by_cel_pass`) is the game's own:
`View_AddCels` 0x1e6ba0 never pushes a record whose `+0x44` flags have bit 0x10 — the blocking
polygons and skies ("18 skipped by the cel pass" on 07000005, per level in the golden's
`cel_skipped` column) [M decompile re-read this session].

## 3. G-018 closed: the per-object GS state is exactly three registers

Measured over every `DIRECT` A+D write of all 27 on-foot levels (ALPHA_1 0x42: 179,862 + 28,925 +
312 + 159 + 159 writes; plus TEST_1 (0x47) and ZBUF_1 (0x4e)): **no CLAMP (0x08), no TEX1 (0x14),
no TEX0 (0x06) write ever occurs** in a PS2_GFX packet. CLAMP/filtering are global texture-upload
state (render.md: TEX1 0x60 bilinear, REGION_REPEAT in-page), not per-object — the port's fixed
repeat/bilinear sampler needs no per-batch state. The six A/B/C/D shapes that do occur (ps2tek GS
register tables, *sourced*; values *measured*):

| ALPHA_1 | fields A,B,C,D | equation | port blend | writes |
|---|---|---|---|---|
| 0x44 | 0,1,0,1 | (Cs−Cd)·As+Cd | 0 normal | 179,862 |
| 0x7F00000048 | 0,2,0,1 | (Cs−0)·As+Cd | 1 additive | 28,925 |
| 0x7F00000042 | 2,0,0,1 | (0−Cs)·As+Cd | 2 subtractive | 312 |
| 0x58 | 0,2,1,1 | (Cs−0)·**Ad**+Cd | 4 additive by destination alpha | 159 |
| 0x64 | 0,1,2(FIX=0),1 | (Cs−Cd)·0+Cd | 3 keep-destination | 159 |

The 0x58/0x64 batches exist only in 07000041 map 0x63 (`EnvModTest_ModEnv`, `EnvModTest_BaseModEnv`,
`polySurface2` 0x02000631) [D probe]. Nothing in the load/draw path patches their FIX
(`psiCreateEntityGfx` 0x1171b0 rewrites only texture REF addresses; `DrawThisBox` 0x1230d8 only
kicks the packet; `CacheAndFillNewGlistTextures` only uploads textures) — the disc values are
final: the 0x64 mod batches paint nothing (while still depth-testing/writing), and the 0x58 env
batches add only where the base batch wrote framebuffer alpha. The port mapped both to the normal
blend and drew them opaque — fixed (`Draw::from_gs` + gpu.rs pipelines 3/4).

TEST_1 fix: 0x5000b (GEQUAL 0 — everything passes, 159 writes, 6 drawn batches) and ATST ALWAYS now
map to "no alpha test"; the old mapping rejected a = 0 texels and skipped their depth write, which
the game only does for AFAIL ≠ 0.

## 4. Draw order: the list-0 cel primaries draw in placement-parse order

`View_DrawObjList` (0x1e69a0) sorts ONE mixed array of cel entries and object entries by key and
draws each: a cel entry via `View_DrawCelGlist` (0x1e6538), an object via
`View_SetupRenderModes_Tweak` + `View_DrawObjectGlist`. `View_AddCels` (0x1e6ba0) pushes each
visible cel's primary glist into **list 0 with the synthetic key −90000 + 10k** (k = the GfxList
walk order — the VisCelList QuickSort orders the chain by cel pointer ≈ allocation ≈ parse order),
and only `View_AddObjects`' objects (and the cel +0x18 chain records, which never exist on this
disc — no LOD blocks) carry distance keys. So the game's list 0 is: **all visible statics in
placement-parse order, then the dynamic objects front-to-back**.

The port sorted everything front-to-back by object centre — which could draw a baked shadow decal
(nearer centre) before the wall it lies on, its depth write rejecting the wall (the user's navy
blocks, R2). fix-render4's `alpha_shaped` reorder was a texture-shape stand-in. The port now
implements the game's key exactly (`Material::chain`, gpu.rs sort): statics first in parse order,
dynamics by dist². The alpha_shaped hack is retired (the classifier stays, pinned by
`decal_order_live`). Also: a glist-flag-0x1 **static** no longer moves to the alpha list — the
primary push has no list choice; only dynamic objects pick list 1 by flags (`View_AddObjects`).

## 5. Per-level eyeball spots (`nf-view --bond x,y,z,yaw,pitch`)

One spot per model class per level (from the real placements; "script pose" = the object rides the
script player at run time — any view shows the class elsewhere in the level). yaw/pitch in radians.

| level | class | bond |
|---|---|---|
| 07000001 | door | `--bond 4.89,6.95,2.87,1.571,-0.2` (SlidingDoor) |
| 07000001 | sky | `--bond -29.02,34.16,-71.71,-0.540,-0.2` (moon) |
| 07000001 | SS prop | `--bond 5.70,9.63,-0.13,0.000,-0.2` (corridor_scroll2) |
| 07000001 | breakable | `--bond -10.58,8.41,8.99,0.000,-0.2` (EntryHall_plant-right2) |
| 07000001 | decal | script pose — EntryHall_hanging_plants (alpha-shaped cutout) |
| 07000002 | breakable | script pose — pavillion_table |
| 07000002 | SS prop | `--bond -13.62,6.95,-32.65,0.000,-0.2` (corry_dining_akarailamp) |
| 07000002 | door | `--bond -46.07,8.36,-26.21,0.000,-0.2` (garden_door) |
| 07000002 | sky | script pose — SkyBox1-terra |
| 07000002 | decal | `--bond -19.21,7.03,-13.94,0.000,-0.2` (northerngarden1_stone2) |
| 07000003 | sky | script pose — moon_old |
| 07000003 | door | `--bond 14.68,8.25,-38.40,0.000,-0.2` (door_secret_bedroom) |
| 07000003 | breakable | `--bond -11.10,9.07,-36.54,0.000,-0.2` (closet_creme) |
| 07000003 | SS prop | `--bond -3.45,9.11,-46.51,0.000,-0.2` (masterarea_picture) |
| 07000003 | decal | `--bond -10.91,8.55,-41.09,0.000,-0.2` (closet_jacket) |
| 07000004 | sky | script pose — mountains |
| 07000004 | decal | `--bond -13.15,6.37,33.67,0.000,-0.2` (carpark_plants2) |
| 07000005 | sky | script pose — Sky_Wall |
| 07000005 | door | `--bond 153.68,17.36,83.49,0.000,-0.2` (Door_Locked_3) |
| 07000005 | decal | `--bond 33.61,32.12,59.22,1.117,-0.2` (OBJECT_PineTree — the alpha-cutout foliage class) |
| 07000006 | door | `--bond 34.25,50.60,-67.75,1.571,-0.2` (Locked_Door_Top_I) |
| 07000006 | SS prop | `--bond -25.98,50.00,-44.24,0.000,-0.2` (Corona_3) |
| 07000006 | breakable | script pose — Window_Securityroom |
| 07000006 | sky | script pose — Sky_Tower10 |
| 07000006 | decal | script pose — Object_Banister_23 |
| 07000007 | breakable | script pose — OBJECT_Vase1 |
| 07000007 | door | `--bond 132.00,44.60,-58.38,0.000,-0.2` (Door_Wooden_2) |
| 07000007 | sky | `--bond 2.96,-176.76,-3.00,0.000,-0.2` (skybox) |
| 07000007 | decal | script pose — OBJECT_ShieldRedWhite2 |
| 07000008 | sky | `--bond 79.73,105.91,-175.11,0.000,-0.2` (skybox) |
| 07000008 | door | `--bond 115.89,23.60,-126.04,0.000,-0.2` (Door_unLocked_2) |
| 07000008 | breakable | script pose — Glass_Controlroom1 |
| 07000008 | SS prop | `--bond 165.25,44.93,-117.13,-1.571,-0.2` (Object_SafeDoor) |
| 07000008 | decal | script pose — object_oilpainting |
| 07000009 | SS prop | `--bond -3.40,-366.59,16.23,-3.142,-0.2` (LIFTDOORS_RIGHT_ENDLEVELDOORS) |
| 07000009 | breakable | script pose — securityroom_glass |
| 07000009 | door | `--bond 18.82,-366.75,35.95,0.000,-0.2` (DOOR_lobby-stairwell) |
| 07000009 | sky | `--bond 146.19,-136.34,329.64,0.000,-0.2` (Skyline_Buildings1) |
| 07000009 | decal | `--bond -3.94,-366.59,8.48,0.000,-0.2` (LIFT SHADOWMAPS) |
| 0700000a | door | `--bond -41.95,2.70,30.03,2.950,-0.2` (office_a_door1) |
| 0700000a | breakable | script pose — Office-A_glass0 |
| 0700000a | SS prop | `--bond -2.97,2.90,10.03,0.000,-0.2` (room-m_LIFTDOOR2) |
| 0700000a | sky | `--bond 25.78,-125.79,286.48,-0.296,-0.2` (Skyline_Buildings) |
| 0700000a | decal | script pose — Office-Z_shadowmaps |
| 0700000b | door | `--bond 13.82,48.38,10.09,-3.142,-0.2` (door_waitingroom_corridor_right) |
| 0700000b | SS prop | `--bond 5.57,51.33,19.60,1.571,-0.2` (vent_grill1, interior) |
| 0700000b | sky | `--bond 25.78,-125.79,286.48,-0.296,-0.2` (Skyline_Buildings) |
| 0700000b | decal | script pose — secretary shadows |
| 0700000c | SS prop | `--bond 5.78,18.00,-109.32,-3.142,-0.2` (ventmesh-front) |
| 0700000c | sky | script pose — SkyBox1-terra |
| 0700000c | door | `--bond 39.99,14.17,-207.45,3.142,-0.2` (door2turbine) |
| 0700000c | decal | script pose — obj_turb5_floorshadow |
| 0700000d | door | `--bond 119.36,10.87,-277.80,2.705,-0.2` (obj_doortop) |
| 0700000d | SS prop | `--bond 186.25,12.03,-291.33,0.000,-0.2` (shipping_handle) |
| 0700000d | sky | script pose — Mountains_new |
| 0700000d | decal | script pose — room_ShippingWarehouse0 |
| 07000011 | door | `--bond 13.82,48.38,10.09,-3.142,-0.2` (door_waitingroom_corridor_right) |
| 07000011 | SS prop | `--bond 5.57,51.33,19.60,1.571,-0.2` (vent_grill1, interior) |
| 07000011 | sky | `--bond 25.78,-125.79,286.48,-0.296,-0.2` (Skyline_Buildings_2) |
| 07000011 | decal | script pose — secretary shadows |
| 07000012 | breakable | `--bond -9.93,-31.99,-34.96,0.000,-0.2` (skybridge_glass1) |
| 07000012 | door | `--bond -32.61,2.70,17.91,3.142,-0.2` (office-G_door1) |
| 07000012 | SS prop | `--bond -16.19,-0.34,28.09,0.785,-0.2` (sh_shuttersclosed_LEFT) |
| 07000012 | sky | `--bond 25.78,-125.79,286.48,-0.296,-0.2` (Skyline_Buildings_old2) |
| 07000012 | decal | script pose — Office-Z_shadowmaps |
| 07000013 | door | `--bond 21.82,-366.75,38.95,-1.571,-0.2` (DOOR_lobby-stairwell) |
| 07000013 | breakable | script pose — securityroom_glass |
| 07000013 | SS prop | script pose — Shutters2 |
| 07000013 | sky | `--bond 25.78,-125.79,286.48,-0.296,-0.2` (Skyline_Buildings_old) |
| 07000013 | decal | `--bond 2.19,-366.59,8.49,0.000,-0.2` (LIFT SHADOWMAPS) |
| 07000014 | sky | `--bond 120.39,4.20,50.56,-0.087,-0.2` (Light_rays_Underwater_Silo_section_1) |
| 07000014 | breakable | script pose — ServerRoomWindow |
| 07000014 | door | `--bond 27.79,2.80,-28.98,0.000,-0.2` (Door_Locked_3B) |
| 07000014 | decal | script pose — CargoArea2Grill |
| 07000015 | door | `--bond 84.71,43.16,-1.04,-1.571,-0.2` (door_exitdoor) |
| 07000015 | decal | `--bond 23.01,36.36,35.62,1.571,-0.2` (lamp_red) |
| 07000016 | door | `--bond 1.84,1.50,-117.23,0.000,-0.2` (Door_1) |
| 07000016 | sky | script pose — Underwater_Silo_section_1 |
| 07000016 | decal | script pose — shadows_blastroom |
| 0700001b | sky | script pose — SKY_StarField |
| 07000041 | breakable | `--bond 23.92,1.61,-71.31,0.000,-0.2` (Glass) |
| 07000041 | sky | script pose — SkyBox |
| 07000041 | decal | `--bond -14.21,1.60,-76.26,1.204,-0.2` (Object_Foliage) |
| 07000041 | envmap | `--bond 34.65,3.35,-38.61,0.000,-0.2` (EnvModTest_Env — the 0x58/0x64 batches) |
| 07000046 | SS prop | `--bond -92.03,-8.28,48.06,0.000,-0.2` (Pool-Ice) |
| 07000046 | door | `--bond -44.36,-8.27,55.64,1.308,-0.2` (Door_Tunnel_G9_2) |
| 0700004a | door | script pose — OBJECT_DoorLeft |
| 0700004a | decal | script pose — Floor70 |
| 0700004b | SS prop | `--bond 108.96,31.30,-126.71,1.571,-0.2` (OBJECT_CableCarbody) |
| 0700004b | sky | script pose — skybox |
| 0700004b | decal | script pose — object_carpetshadow |

Generate again with `NF_SURVEY=1 NF_ISO=… cargo test -p nf-view --release --test models_world_live
bond_spots -- --ignored --nocapture`.

## 6. Model classes found in OTHER lanes (not fixed here)

Meshed dynamic placements the port does not draw yet — the per-level counts are in
`models_survey`'s `undrawn` column. The MP-gated creators (0x24/0x25/0x26) don't create their
objects in single-player, so their markers are correctly absent.

- **misc (044)**: 0x28 sensor, 0x29 switch, 0x2e lock, 0x2f copter, 0x30 monitor, 0x31 fuse box
  (up to 129 on 0700001b), 0x32 rain box, 0x33 hint, 0x35 PCQ worm, 0x38/0x39 space laser/missile,
  0x3b DynamicObject (45 on 07000014), 0x3f creep wall, 0x4a apocalypse, 0x4e light, 0x71 rotor,
  0xd4 cloud, 0xd5 one-sided, 0xd8 rigid body (21 on 07000002), 0xda car, 0xe1 destroyable
  (48 on 07000014), 0xe3 flicker, 0xe4 hurt, 0xfe mine, 0x2d leaf generator, 0xdf snow generator,
  0xea/0xeb touch triggers (up to 28), 0xf2 emitter (35 on 07000014), 0xf3 env tree, 0xf4 movie
  trigger, 0xf9 sound trigger, 0xfa third icon, 0xfd cam subject, 0x27 GT.
- **items (043)**: 0xfa third icons (07000005/07000011/07000012/07000014/07000041), 0x2e locks.

## 7. Known data quirks (no port defect)

- 0700000d map 0x31 object 215 `WallHug_Blocker1_Long`: one batch's texture REF is index 185 in a
  185-texture map. The object's glist flags are 0x10 (the cel-pass skip — a blocker), so the game
  never draws the batch and the REF never resolves [D probe]. The port skips it the same way.
- 104 non-finite UVs in `Room-L_atrium_bottom` (0700000a, 07000012): U-022 stands; the port draws
  those vertices with UV (0,0).
- The boxes' +0x30 field exceeds the decoded triangle count on 690 leaves across the levels (the
  worst: 0700000d with 274 and 0700004a with 243, counted per leaf) — U-020 stands (a game-side
  `Ps2NumVerts` statistic; the geometry itself is validated by the fresh-qword invariant, the
  bit-exact box bounds and the nfre cross-check in ps2gfx.md).

## 8. What's left (this lane)

- The VU1 microcode reading (G-014): the decoder's invariants (fresh qwords, w ∈ {0,−1}, GIF tag
  shape) hold on every batch of every level [D], and the nfre cross-check agrees on 6,227 objects —
  the microcode itself remains unread (no VU disassembler in this sandbox; the ELF is at
  `work/elf/ACTION.ELF`, `.vutext` at 0x241c80, file offset 0x13bb80, 0x2880 bytes).
- The viewer's sim-side swaps (breakable shatter, SS key animation, door swing) are the other
  systems' work (world-objects.md); this lane pinned the intact-state draws and their draw states.
- The main agent's render pass should eyeball §5's spots — especially 07000041's env-map object
  (the new blend 4) and any level where a glist-flag-0x1 static moved from the alpha list to list 0
  (§4; e.g. 07000005's pine trees).
