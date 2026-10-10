# Port map: levels

Every ACTION.ELF function the level-loading, level-transition and level-fidelity subsystems involve: the address,
the name, and the port status. Tags: **ported**, **partial**, **stub**, **standin**, **not-started**.

Sources: `work/decomp/ACTION.ELF/`, the Rust source in `crates/nf-view/src/{main,scene,gpu}.rs`,
`crates/nf-game/src/{triggers,door,corona,searchlight,drone,world}.rs`, `crates/nf-formats/src/{level,levelbin,map,placement,filelist}.rs`.

## 1. Level archive loading (ACTION.ELF + nf-formats)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x10bd08 | `psiFileOpen` | ported | `disc::files_bin_entries` (the FileList lookup by name). |
| 0x10ba38 | `psiFileLoad` | ported | `IsoImage::read_bytes` (the offset + size read). |
| 0x1d1b68 | `parsemap_block_map_data_dynamic` | partial | `Level::parse` handles placements (0x1A), but **not** the door record (key count/rate at +0x40/+0x42, the anim at +0x44) — that's in `door.rs`'s own parse. The runtime DOORINFO struct is **standin**. |
| 0x1d04f0 | `parsemap_create_dynamic_objects` | partial | the object-creation dispatch (type → Door_Create, Drone_Create, etc.); the port builds the instances in `Scene::build` but the per-type creation order and the cel linking are **standin** (G-016). |
| 0x1d24d0 | `parsemap_block_path_data` | ported | `Level::path_blocks` (the 0x19 key-list payloads). |
| — | `LevelBin::parse` | ported | `nf_formats::levelbin::LevelBin` (the archive entries). |
| — | `MapFile::parse` | ported | `nf_formats::map::MapFile` (the block list). |

## 2. Level flow / transition (ACTION.ELF)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x1d3800 | `ResetMap_LevelToLoad` | ported | `Trigger`'s `LOAD_LEVEL` arm sets `Drones::level_exit` to the next level's hashcode; `nf-view/main.rs` reloads. |
| 0x1d5258 | `ResetMap_LevelCode2Img` | ported | the level-code → archive-name switch (read from the decompile; the 36 on-foot levels). |
| 0x1d6360 | `ResetMap_Load` | ported | `nf_view::load` (the archive read + `Level::parse`). |
| 0x1d5080 | `ResetMap_GameInit` | partial | the level-start init (bank load, music start, drone creation); ported in `Scene::build` + `build_sound`, but the `GameState` stack machine is **standin**. |
| 0x1d5de0 | `ResetMap_GenLoadScreen` | **not-started** | the load screen (the port loads synchronously). |
| 0x1c98c8 | `GameFlow_Main` | **standin** | the GameState stack machine is not ported; `nf-view` drives the single-level loop. `STANDIN` in `main.rs`. |
| 0x1c9f38 | `GameFlow_PushState` | **not-started** | the state stack (load screen, etc.). |
| 0x1c9eb0 | `GameFlow_PopState` | **not-started** | same. |
| 0x1b2318 | `Trigger_Activate` | ported | `Trigger::update` (the `LOAD_LEVEL` arm, the `Music_Event(3,1)`, the `GS_PauseGame`). |

## 3. Level fidelity — rendering (ACTION.ELF + nf-view)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x11e2e8 | `psiPreGameRun` | ported | `scene::fog_for` (the per-level fog colour + `Ps2Far1`; confirmed against the decompile). |
| 0x11b140 | `CreateSqueezeFrame` | **standin** | `GAIN` constant (`SqueezeColourIn` 0x91/0x80); the writer is unknown. `STANDIN` in `gpu.rs`. |
| 0x121578 | `psiFog` | ported | (empty in the game — no GS fog; the port's fog is a screen pass). |
| 0x10b960 | `psiDrawFogPass` | ported | the fog screen pass in the shader (`gpu.rs`); the `floor(zb/256)/128` factor is now clamped to [0,1] (0078). |
| 0x119228 | `CreateFogSprite` | ported | (the full-screen fog sprite; in the shader). |
| 0x118718 | `CreateMoveRG2BA` | ported | (the Z-bits-to-alpha move; in the shader). |
| 0x10b2b0 | `PS2WorldViewVU1Kick2` | **standin** | the depth `zb = N1 + (F1−N1)·n·(f/d−1)/(f−n)` is computed in the shader; the VU1 microcode is not ported. `STANDIN` in `gpu.rs`. |
| 0x1e6ba0 | `View_AddCels` | partial | the draw-list assignment (glist flags → list 0/1/2); the cel visibility system (G-016) is **standin**. |
| 0x1e62d8 | `View_AddObjects` | partial | the object draw dispatch; the port's `Scene::build` + `add_instance` mirrors it but the cel linking is **standin**. |
| — | per-leaf GS state (ALPHA_1, TEST_1, ZBUF_1) | ported | `scene::Draw::from_gs` (the `DIRECT` A+D packet parsing). |
| — | texture REF → material key | ported | `Batch::texture` from the DMA REF tag. |
| — | the untextured fallback (white 1×1) | ported | `tex_bgs[0]` (the white fallback). |

## 4. Level fidelity — doors (ACTION.ELF)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x136100 | `Door_Create` | ported | `door::Door::new` (the runtime record: key count/rate, the anim, channels). |
| 0x135be0 | `Door_Activate` | ported | `door::Door::activate` (the swing/slide state machine). |
| 0x135738 | `Door_Interp` | ported | `door::interp` (Catmull-Rom / slerp over the 28-byte keys). |
| 0x135440 | `Door_SetupSwing` | ported | the swing-speed `0.5/(int)(0.5/2s)`. |
| 0x1363c8 | `Door_Update` | ported | `door::Doors::update`. |
| 0x135fe0 | `Doors_Calc` | ported | `door::Doors::calc` (group sync). |
| 0x135980 | `Door_ReactToHit` | partial | the anti-crush (`door::Door::react_to_hit`); the proxy-list path is **standin** (U-043). |
| 0x135d40 | `Door_Link2Portal` | **not-started** | the door→portal link (G-016, block 0x21). |
| — | the door's drawn pose | ported | `scene::animate` reads `door::Door::matrix`. |

## 5. Level fidelity — other dynamic objects (ACTION.ELF)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x18da00 | `Corona_Create` | ported | `corona::placements` (the 13 static glow quads). |
| — | `Corona_Update` (class 0x45) | **not-started** | the runtime corona LOS fade (U-042). |
| 0x17d340 | `Copter_Create` | **not-started** | the helicopter. |
| 0x17b968 | `Env_Create` | ported | `env::Drops` (snow). |
| 0x18d840 | `RainBox_Create` | ported | `env::RainBox`. |
| — | `Searchlight_Create` | ported | `searchlight::part_placements` + `Searchlight`. |
| — | `Searchlight_Update` | ported | `searchlight::Searchlights::update` (sweep, tracking, breaking). |

## 6. Level fidelity — collision (ACTION.ELF + nf-game)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x1eae80 | `Collide_PickObj` | **standin** | the cel-linked collision pick; the port uses a flat triangle list (`CollisionWorld`). `STANDIN` in `world.rs`. |
| — | `Collision::parse` | ported | `nf_formats::collision::Collision` (block 0x2E). |
| — | `CollisionWorld::build` | ported | the static + dynamic (doors, script objects) collision world. |
| — | the "invisible wall" diagnostic | ported | `NF_COLL=1` + the `C` key (0078). |

## 7. Level fidelity — allegiance (ACTION.ELF)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x14cbd8 | `NDrone2_GetDroneTypeAttackTypeFriend` | ported | `drone.rs:609` (the `team` field: dtype 10/0xb/0xd/0xe → 3, 0xc → 2, else 1). |
| 0x177ed0 | `DroneVision_EnemyLookForOpponent` | ported | `drone.rs::look_for_opponent` (the `team == 3` gate, 0078). |
| 0x145150 | `NDrone2_SeenAndAttacking` | ported | `drone.rs` attack-gate caller (the `team == 1` filter, 0078). |
| 0x1770f0 | `NDrone2_ReactToOpponentSighted` | partial | the `team == 3` gate is ported in `look_for_opponent`; the rest of the react path is in the state machine. |
| 0x1424e0 | `NDrone2_FindOpponent` | ported | the opponent pointer (Bond) is set for all drones in SP. |

## Summary

| Subsystem | Ported | Partial | Stub | Standin | Not-started |
|---|---|---|---|---|---|
| Level archive loading | 5 | 2 | 0 | 0 | 0 |
| Level flow / transition | 4 | 1 | 0 | 1 | 3 |
| Level fidelity — rendering | 8 | 2 | 0 | 2 | 0 |
| Level fidelity — doors | 7 | 1 | 0 | 0 | 1 |
| Level fidelity — other objects | 5 | 0 | 0 | 0 | 2 |
| Level fidelity — collision | 2 | 0 | 0 | 1 | 0 |
| Level fidelity — allegiance | 4 | 1 | 0 | 0 | 0 |
| **Total** | **35** | **7** | **0** | **4** | **6** |
