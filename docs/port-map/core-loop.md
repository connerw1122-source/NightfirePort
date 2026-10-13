# Core loop port map

A function-by-function map of everything between the frame start and the per-object updates (plan v2 §4 step 1):
`Game_Run` and its per-frame callees, the control-list walk, the pause and ScriptCam gating, triggers, switch
channels, script players and the level flow. Build: ACTION.ELF `e7ffdcb3…` (SLUS-20579 NTSC-U).

Sources: `work/decomp/ACTION.ELF/<addr>_<name>.c` (Ghidra decomp), the MIPS disassembly where the C is incomplete,
`work/ee_start_05.bin` / `work/ee_start_07000001.bin` (EE RAM snapshots, file offset = EE address), the disc via the
placement dumps in the other port maps. Tags: [M] measured in code here (function + address), [L] read from a live
snapshot (address + value), [D] disc, [S] suggested. Statuses as in `guard-ai.md`: **ported / partial / stub /
STANDIN / not started**.

Port roots: `crates/nf-game/src/{drone,triggers,nis,door,spawners,env}.rs` (the tick), `crates/nf-view/src/{scene,lib}.rs`
(the frame around it).

## 0. The frame in one paragraph [M 0x1c9790]

`Game_Run` runs once per video frame. After `psiPreGame_Run` (IOP audio housekeeping) and `Input_Update`, everything
sits inside `if (FreezeGame == 0 || switch_AllowFreeze == 0)`, and the gameplay block inside `if (sloflag == 0)`
(no slow-mo). Menus (`MenuManager_Update/Monitor`), `Mission_Update` and `MP_Update` run **unconditionally**; text
picks `Text_Update2Line` only when `ScriptCam == 0` **and** `psiMovieFinished()` **and** `GS_IsPaused(-1) == 0`
(0x1c9790:17-21), else `Text_Update`; `Env_Update` (snow) runs whenever `DrawSnow != 0` — **even while paused**
[audited: 0x1c9790:27-29]. Then the single gameplay gate:

```c
if (GS_IsPaused(-1) == 0) { SSys_Monitor(); Light_Update(); control_movement_object_handler(0); }
else { if (ScriptCam != 0) Light_Update(); psiDecompressWoman(); }
```

`GS_IsPaused(-1)` reads `GameState[0x50]` when `MPSettings._384_4_ == 0` (single player; MP ANDs the MPGame slots)
[M 0x17d1e8:14-28] — **the pause byte, not the cut-scene flag**. `ScriptCam`
never gates the object handler: the whole per-object world (doors, triggers, script players, drones, the player)
keeps running while a script camera is up; only the player object freezes, via `BLData+0x94a` inside
`Player_Update` [M 0x193600:80-87]. The game's own cut-scene drone freeze is a script event: `Drone_EnableAll(0)`
sets `NPCGlobals = 0`, and `Drone_Control` returns at once while it is 0 [M 0x139268:39-42, 0x1383e8:23]. Finally
`Camera_UpdateAll` and `psiPostGame_Run`.

`GS_PauseGame(1)` (pause byte = 1) is written by: the START-button pause menu (`Player_Update` 0x193600:77),
`Trigger_Activate`'s LOAD_LEVEL path (0x1b2318:110), mission end/fail (`Mission_Update` 0x182c60:79,127) and the
in-game menus (`MenuManager_Monitor` 0x21d508:124,240). `GS_PauseGame(0)`: `ResetMap_GameInit` (0x1d5080:16),
`Mission_Init` (0x182270:29), `MenuManager_Delete` (0x21c2b0:21).

## 1. Game_Run (0x1c9790) and its callees

| Address | Name | What it does (one line) | Rust location | Status | Gaps |
|---|---|---|---|---|---|
| 0x1c9790 | `Game_Run` | One video frame: input, menus, text, snow, then the gated object handler + cameras. | `nf-view/src/lib.rs:649-687` (RedrawRequested tick loop) + `scene.rs:237` (`step_bond`) / `scene.rs:409` (`run`) | partial | No `FreezeGame`/`switch_AllowFreeze`/`sloflag` (no slow-mo, no freeze); no `MenuManager_*`/`Mission_Update`/`MP_Update` (mission end/fail screens, the 0x0700004x wrappers); `psiMovieFinished`/`psiDecompressWoman`/`UpdateAllShards`/`SSys_Monitor`/`Light_Update`/`Camera_UpdateAll`/`psi*Game_Run` have no ports. The pause-byte gate is replaced by `App.paused` (user Esc-pause) — see §7 item 3. |
| 0x109520 / 0x109580 | `psiPreGame_Run` / `psiPostGame_Run` | IOP sound RPC pump before/after the frame. | `nf-audio` (its own thread) | STANDIN | The port runs audio on a host thread instead of the IOP; per-frame ordering with the tick is not the game's. |
| 0x1a9c80* | `Input_Update` | Polls pads into the action map. | `lib.rs:520-600` (winit key events → `Input`) | partial | Keyboard/mouse only; no pad, no `Input_Action` repeat/edge semantics (action 0x19 "skip cut-scene" missing — see `SP_Update`). |
| 0x17d1e8 | `GS_IsPaused` | `-1` + single player: `GameState[0x50]`; `-1` + MP: AND of the MPGame slots; per-player: `MPGame[p*0x30]`. | none (`App.paused` + `Drones::paused` stand in) | STANDIN | The port has no pause byte: nothing distinguishes "pause menu" from "cut-scene" — divergence 3 in §7. |
| 0x17d178 | `GS_PauseGame` | Writes `GameState[0x50]` + all 8 `MPGame` slots; PlarStat timer log. | none | not started | The LOAD_LEVEL caller's pause is folded into the port's synchronous level swap. |
| 0x1b1510 / 0x1b09f0 | `Text_Update` / `Text_Update2Line` | Subtitle queues. | none (HUD shows health/ammo strings) | not started | Cut-scene subtitles missing. |
| 0x17bc88 | `Env_Update` | When `DrawSnow`: one `Rand_Rand(20000)`, then `UpdateDrops`. | `drone.rs:3706-3712` (`Drops::update`, `env.rs:58`) | ported | The port runs it inside `Drones::tick` before the paused gate — the game runs it before `GS_IsPaused` too, so snow keeps falling while paused [M order match]. The `Rand_Rand(20000)` is drawn even when no env exists in the game (the call is inside `Env_Update` after the `DrawSnow` check; the port draws it only when `env.is_some()` — one draw short when the level has snow boxes but no Drops; none on 07000005). |
| 0x18c580 | `UpdateDrops` | Moves/clips the 4096 drops, respawns via the shared RNG. | `env.rs:58` | ported | Velocity table seeds are not the game's (`env.rs:41` notes the seed deviation). |
| 0x1e2d08* | `Camera_UpdateAll` | Advances camera 0-3, applies the script camera. | `scene.rs:420` (`script_camera`) + `lib.rs:711-714` | partial | Only camera 4 (script) and the Bond-follow camera; no `Camera_CalcViewAngles` game-path, no camera stack pops on `Script_CameraEnd` beyond the object delete. |
| 0x1ad998 | `SSys_Monitor` | Sound-system monitor: occlusion/ambient updates, one `switch_channels` write (event-driven). | none (ambient handled in `sound.rs` init) | not started | [M 0x1ad998:10] gated on `ScriptCam == 0`. |
| — | `Mission_Update` | Mission objectives, fail/end screens, pushes state 7 (`GS_PauseGame(1)`). | none | not started | Objectives live in `music.rs` events only. |
| — | `MenuManager_Update` / `Monitor` | Menus incl. the pause menu (its Monitor writes the pause byte). | none | not started | The START pause does not exist in the port (`App.paused` is a viewer pause: the tick loop stops entirely, the game keeps rendering). |

\* address in the symbol table; the exact entry is in `work/action-symtab.tsv`.

## 2. The object handler: `control_movement_object_handler` (0x1336f8)

The game's per-frame world walk. Order [M 0x1336f8:21-238]:

1. `Drone_InitComms` (0x139520) — drone bookkeeping (below).
2. `MP_DebugDraw` — MP-only, no-op in single player.
3. `Doors_Calc` (0x135fe0) — door groups sync + busy clear (below).
4. `Trigger_Calc` (0x1b1900) — Bond-moment latches + the Touch pass (below).
5. The **cel channel-watch loop**: walks `glb_world`'s cel list; for each cel-linked object whose `obj+0xA2` byte
   names a switch channel, if `switch_channels[ch] != switch_channels_hold[ch]`, `obj+0x38 = *(cel+0x40)` — objects
   jump to a cel-stored transform when their channel flips (used by cel-portal doors). Part of the unported
   cel/portal system (G-016).
6. **The snapshots**: `switch_channels_prev ← switch_channels_hold`; `switch_channels_hold ← switch_channels`
   (`amemcpy` 0x26FD90←0x26FE90←0x26FC90, 256 bytes each) [M 0x1336f8:52-53]. So during the object walk `prev` =
   last frame's post-`Trigger_Calc` state and `hold` = this frame's. `switch_channels_hold` has no other reader;
   `switch_channels_prev` is read by `Door_Update`'s lock edge [M 0x1363c8] and `Player_Weapon` (0x1a24c8:247,
   channel 0x48).
7. **The control walk**: `DynamicObjList` head (+0x14 = next), **newest first** (`control_add_object_to_list`
   prepends); per object: if `obj+0xFA & 6`, `Mat_GetDir(obj+0x60 → obj+0x90)`; then
   `control_funcs[obj+0xFF].update(obj)` when non-NULL [M 0x1336f8:54-69]. `control_funcs` (0x26F890, 85 entries ×
   12 bytes update/collide/delete) — read live [L ee_start_05.bin@0x26f890]: class 2/0x11 `Drone_Control`, 3/0x12
   `Player_Update`+`Player_CollisionHandler`, 5 `Bullet_update`, 0x1A `RB_Update`, 0x1B `SP_Update`, 0x1C
   `Door_Update`+`Door_ReactToHit`, 0x1D `Trigger_Update`, 0x22 `Searchlight_Update`, 0x23 `Creature_Update`, 0x24
   `cloud_update`, 0x2F `Pickup_Update`, 0x30 `DroneSpawner_Control`, 0x39 `Switch_Update`, 0x3C `FuseBox_Update`,
   0x40 `SoundTrigger_Update`, 0x41 `Swoosh_Update`, 0x44 `MusicTrigger_Update`, 0x45 `Corona_Update`, 0x4B
   `Mine_Update`, 0x50 `SpaceLaser_Update`, 0x53 `InverterTrigger_Update`, 0x54 `Apocalypse_Update` — all read live
   [L ee_start_05.bin@0x26f890, matched to `work/action-symtab.tsv`; classes 1, 4, 8-11, 0x13-0x14, 0x16-0x18, 0x1E,
   0x29, 0x2B-0x2D, 0x4C-0x4F, 0x51 are empty or SKIN_2_BONE].
8. **The delete pass**: repeats until stable — every object with `obj+0xFE & 1`: `Effect_RemoveSObjs`,
   `control_delete_object`, then every same-cel object of class 5 (`+0xE0+0x30/0x40`), 0x0F (`+0xE0+0x24`) and every
   `obj+0xD0`-proxy chain clears its pointer to the deleted object [M 0x1336f8:70-146]. Objects with `+0xFE & 2`
   (just moved) get `Control_BuildWorldSph` + `Lights_CalcAmbientLight`.
9. **The moved pass**: for every object with `obj+0xFA & 0x20` (moved): wrap `+0x50/+0x54/+0x58` (yaw/pitch/roll)
   into ±π, `Control_BuildWorldSph`, `control_handle_cel_change`, `Lights_CalcAmbientLight`,
   `+0x40..+0x4C ← +0x30..+0x3C` (previous transform = this transform), and unless `+0xFA & 6`,
   `+0x60..+0x6C ← +0x50..+0x5C` (draw transform); then objects with `+0xFA & 0x40` copy `+0xC0..+0xCC →
   +0x30..+0x3C` (a pending teleport) [M 0x1336f8:147-236]. **The same pass exists a second time, cloned after
   `Collide_Update`** (0x1336f8:307-394), so objects that moved during the collision pass are processed too.
10. `Collide_Update(DynamicObjList head)` (0x1EA398) — collision pairs via each object's proxy list (`obj+0xD0`).
11. **The draw-decrement + cel-light pass**: `obj+0xFC` (draw countdown) decrements when non-zero; objects whose
    cel changed (`obj+0x1C` cel ≠ `obj+0x20` stored cel) unlink/relink into the new cel's list
    (`build_LinkToRoom`/manual) and, when `GameState[0x40] == 0` (level not ending), reset the cel-light
    `+0x100..+0x102` from the cel's `+0x9A/+0x9C/+0x9E` (ambient RGB, channel-selectable via the cel's `+0xA2`)
    [M 0x1336f8:239-305].

Port: `Drones::tick` (`drone.rs:3684`) runs: frame/music/`Env_Update`, Bond visibility + opponent info, player
bullets, corpse culling, `Check_Target`, hearing, alarm, target nav nodes, cover nodes, **`Trigger_Calc`**
(`drone.rs:3895`), **script players** (`drone.rs:3897`), the moving-collision set (`drone.rs:3900-3914`), script
trigger volumes (`drone.rs:3916-3934`), script requests (`drone.rs:3940-3974`), then the **ordered object walk**
(`drone.rs:3981-3987`: placements sorted descending, drones before triggers on ties), pickups first
(`drone.rs:3989`), then per object triggers/spawners/searchlights/doors/drones interleaved (`drone.rs:4000+`), and
`Doors::calc` + `end_tick` last (`drone.rs:4159-4160`).

| Address | Name | What it does | Rust location | Status | Gaps |
|---|---|---|---|---|---|
| 0x1336f8 | `control_movement_object_handler` | The walk above. | `drone.rs:3684` (`tick`) | partial | Steps 5 (cel channel watch), 8 (delete-pass pointer scrubbing), 9 (angle-wrap + prev-transform copy is implicit per-object; the +0x40-teleport pass absent), 10 (`Collide_Update`'s proxy-pair walk — the port uses its own capsule world), 11 (cel lights, draw countdown) are not ported; the player object is not in the list (Bond steps before the tick, `scene.rs:367`), so a door's proximity scan sees him one phase early (same tick, not the same sub-phase); the newest-first order is emulated by sorting placements descending (`drone.rs:3981-3987`) — runtime-created objects (pickups) are put first (`drone.rs:3989`), which matches the game's prepend for them [S]. |
| 0x139520 | `Drone_InitComms` | Drone bookkeeping before the walk: unpause (`Drone_PauseNoDraw(0)` when `bDontDraw/bDisableSystem` set [M 0x139520:76-79]), dummy-player/opponent wiring over the `DAT_00298e14` list, and — when the system is enabled — the shared per-frame drone passes (`ClearLinkFlags`, target slots, `ProcessCoverNodes`, `ProcessOpponents`, `ProcessDroneSight`, the sound-alert pass, ch 0x96 alarm rebroadcast, delayed messages, `CheckAlarmRaised`) [M 0x139520:124-284]. The per-drone distances to the four `glb_players` (`Drone+0x154`) are written by `NDrone2_PreDroneControl` (0x148db0), not here. | `drone.rs:3713-3748` (opponent info, `light`, corpse culling) + the hearing/alarm blocks 3810-3855 | partial | No `Drone_PauseNoDraw`/`NPCGlobals` equivalent (the port's `paused` flag covers the freeze, see §7 item 3); the `+0x154` four-player distance array is folded into `opponent_info` for one player; `Drone_DummyPlayer` MP path absent. |
| 0x135fe0 | `Doors_Calc` | Zero `DoorGroupsStates` (0xCA bytes) [M 0x135fe0:13]; for every grouped door (outer loop, **no busy check on the source — last writer wins**), every same-group door that is not busy copies dir/pos01/state/auto-timer, is marked busy and re-posed (0x135fe0:30-42); then clear every busy marker (`+0x70 &= ~0x10`) [M 0x135fe0:60-70]. | `door.rs:550` (`calc`), called `drone.rs:4159` | ported | The port's `group_states` accumulation (`door.rs:428` writer) is never cleared or read (dead); `DoorGroupsStates` memset unported (nothing reads it). |
| 0x1b1900 | `Trigger_Calc` | (a) channels 0x65-0x6E on → `PlrStat_DoneBondMoment` (stats) [M 0x1b1900:12-20]; (b) level latches: 0x07000005 `ch[0xA4] → ch[0x6F]=1`, 0x0700000B `ch[0x34] → ch[0x2D]=1` [M 0x1b1900:21-26]; (c) clear every Touch trigger's output [M 0x1b1900:27-49]; (d) `ProcessTouch = 1`, run `Trigger_Update` for every type-2 trigger, `ProcessTouch = 0` [M 0x1b1900:50-71]. | `triggers.rs:291` (`calc`), called `drone.rs:3895` | partial | (a) missing (stats only); (b) ported for the two latches (`triggers.rs:292-293`); the clear pass clears locked triggers' outputs too — matches the game (no lock check there) [M 0x1b1900:27-49]; the port collects hits then sets (`triggers.rs:295-296`) where the game sets per trigger — same result (each output is written once). |
| 0x1EA398 | `Collide_Update` | The per-object collision pass (proxy lists, `Collide_PickObj`). | `world.rs` capsule world + `drone.rs:3900-3914` (`set_dynamic`) | STANDIN | The port's moving-set collision replaces the proxy/pair system; `Door_ReactToHit` anti-crush does not run from it (A-11b removed the stand-in call; U-041). |
| 0x134698 | `control_add_object_to_list` | Prepends to `DynamicObjList` — newest objects update first. | implicit (sort + pickups first) | partial | See the walk row. |
| 0x134020* | `control_create_object` | Allocates an object of a class, stores glist at `+0xF0`, links to cel. | `scene.rs` build + per-system create | partial | Only the placement-driven creators exist; runtime `control_create_object` callers (spawner drones, bullets, explosions) use the port's own lists. |

\* symbol-table address; see `work/action-symtab.tsv`.

## 3. Triggers

The trigger instance (`obj+0xE0`, 0x40 bytes) is `Trigger_Create`'s memset target. Field map [M 0x1b1720:59-91]:
`+0x00` u32 = placement+0x08 (m_paths index); `+0x04` f32 path clock; `+0x08` f32 rate = frames−1 ×
REC_FRAME_RATE × (FRAME_RATE / placement+0x04); `+0x0C` u16 = placement+0x02 − 1 (frames); `+0x10` up vector;
`+0x20` u32 **out channel**; `+0x24` u16 **lock**; `+0x26..+0x34` u16[8] **inputs/filters** (+0x34 = 0); `+0x36`
u16 **mask** (default 2); `+0x38` u16 **type**; `+0x3A` u16 **hide**; `+0x3C` i16 message countdown.

### 3.1 Creation — one wrapper per placement type [M 0x1d04f0 dispatch, 0x1b1720 + 0x1b2950/0x1b2aa8/0x1b2b88/0x1b2c68/0x1b2d38/0x1b2e08]

Every wrapper backs the placement's params up to `TmpParam` (0x319B88), writes the type into slot 0, then
`Trigger_Create(…)` reads the slots. The shift **differs per family** — this is where `docs/systems/triggers.md`'s
uniform mapping is wrong:

| Placement | Wrapper | p0′ (type) | out (+0x20) | inputs (+0x26..) | mask (+0x36) | lock (+0x24) | hide (+0x3A) |
|---|---|---|---|---|---|---|---|
| 0xDC raw | `Trigger_Create` direct | p0 | **p1** | p2..p8 | p9 (default 2) | p10 | p11 |
| 0xEA TouchOnce | `Trigger_TouchOnce` 0x1b2b88 | 1 | p0 | p2..p8 (loop keeps slots 2-7) | **p1** | **p7** | **p8** |
| 0xEB Touch | `Trigger_Touch` 0x1b2aa8 | 2 | p0 | p2..p8 | **p1** | **p6** | **p7** |
| 0xEC MultiplexIn | `Trigger_MultiplexIn` 0x1b2c68 | 3 | p0 | **p1..p7** (loop shifts by one) | **p8** | **p9** | **p10** |
| 0xED MultiplexSIn | `Trigger_MultiplexSIn` 0x1b2e08 | 4 | p0 | p1..p7 (same shift) | p8 | p9 | p10 |
| 0xEF MultiplexOrIn | `Trigger_MultiplexOrIn` 0x1b2d38 | 0xC | p0 | p1..p7 (same shift) | p8 | p9 | p10 |
| 0xE8 LoadLevel | `Trigger_LoadLevelCreate` 0x1b2950 | 10 | p0 (next level) | p2′=p1 key, p3′=p2 gate, p4′=p3, p5′=p4 | **hard 0xE** | p10 | p11 |

`Trigger_Create` then: class 0x1D (`obj+0xFF`), `obj+0xF0 = glist+0x44`, `build_LinkToRoom`, `View_RotTransMatrix`
→ up vector `+0x10`, and **type 10 clears `obj+0xF0` bit 4** (LOAD_LEVEL triggers are collidable/solid-to-tests,
A-12d's invisible-castle-gate root cause) [M 0x1b1720:89-101]. Note `__s[2]` = `+0x08` is the path-*rate* word;
the mask at `+0x36` and the message countdown at `+0x3C` are separate fields.

| Address | Name | What it does | Rust location | Status | Gaps |
|---|---|---|---|---|---|
| 0x1b1720 | `Trigger_Create` | Builds the instance from the (wrapped) placement slots, links, poses. | `triggers.rs:141` (`from_params`), `triggers.rs:269` (`build`) | partial | Per the table: the port's mux family reads **mask q(9), list q(1..8)\\{7}, lock q(10), hide q(11)** where the game reads **mask q(8), list q(1..7), lock q(9), hide q(10)** (`triggers.rs:150`) — divergence 6 in §7; the Touch family matches [M]. The path anim (clock/rate/`Spline_Interp`) and the `+0x36` mask-default-2 fallback are absent (the port always reads q(9)). |
| 0x1b2950 | `Trigger_LoadLevelCreate` | The 0xE8 wrapper (mapping above). | `triggers.rs:149` (LOAD_LEVEL arm) | ported | mask hard 0xE not copied (unused by case 10). |
| 0x1b2aa8 / 0x1b2b88 | `Trigger_Touch` / `Trigger_TouchOnce` | The 0xEB / 0xEA wrappers. | `triggers.rs:143-144` | ported | — |
| 0x1b2c68 / 0x1b2e08 / 0x1b2d38 / 0x1b2ed8 | `Trigger_MultiplexIn` / `MultiplexSIn` / `MultiplexOrIn` / `MultiplexOut` (0xEC/0xED/0xEF/0xEE) | The mux wrappers (shift by one). | `triggers.rs:150` (shared `_` arm) | partial | The list/mask/lock/hide slot shift (divergence 6). |
| 0x1b2628 | `SoundTrigger_Create` (0xF9) | Sound trigger. | none | not started | |
| 0x1b2a08 | `Trigger_MoviePlayer_Create` (0xF4) | Movie-player trigger (type 0xD). | none | not started | Found in the 0x1d04f0 dispatch; not in `docs/systems/triggers.md`'s type table. |
| 0x1b30e0 | `MusicTrigger_Create` (0xFB) | Music trigger. | none | not started | |
| 0x1b3040 | `InverterTrigger_Create` (0x41) | Channel inverter. | none | not started | |

### 3.2 Per-frame behaviour

| Address | Name | What it does | Rust location | Status | Gaps |
|---|---|---|---|---|---|
| 0x1b1a78 | `Trigger_Update` | `+0x3C` countdown decrement; locked (`ch[+0x24]` on) → `obj+0xF0 \|= 0x30` **when hide (+0x3A) ≠ 0** (0x1b1a78:50-57), return either way; path anim pose + clock advance (wrap past frames); `Trigger_Hit`; then per type — **1** TouchOnce: out on → delete (if `obj+0xF0 & 0x10`), else **falls through into 2**; **2** Touch: `ch[out] \|= hit` (+timestamp), channel 0x7D only on 0x07001500; **3** AND: `ch[out] = AND(inputs)` stopping at the first 0 (+timestamp, Bond-moment 0x65-0x6E + `obj+0xF0 & 0x10` → delete); **4** SEQ AND: AND with the window predicate `time[prev_input] − 4 <= time[input]` — **no upper bound** (an input any time after the previous passes, and so does one up to 4 frames *before* it) [M 0x1b1a78:135-136]; **5** Out: copy `ch[out]` to up to 8 targets; **10** LOAD_LEVEL: `ch[+0x28]` on → `Trigger_Activate(this, glb_players)`; **0xC** OR: `ch[out] = OR(inputs)`; **0xD** MoviePlayer: input on → clear that channel, `GameState._16_4_ = out`, `GameState[0x14] = 0`, `GameFlow_PushState(0,6)`, `GameFlow_PushState(30.0,7)` (0x41f00000 [M 0x1b1a78:190-199]). | `triggers.rs:233` (`update`), per-type arms; Touch run in `calc` (`triggers.rs:291`) | partial | TouchOnce deletes without the glist 0x10 check (`triggers.rs:237`) — a TouchOnce whose glist lacks bit 0x10 keeps firing in the game (harmless on 07000005: the OR write makes it equivalent); the AND/Bond-moment self-delete (`triggers.rs:240-244`) and case 0xD (MoviePlayer) are missing; the path-anim pose and the `+0x3C` countdown are missing; **case 4's port window `b >= a && b - a <= 4` (`triggers.rs:249`) is tighter than the game's lower-bound-only predicate — divergence 6**; AND with an *empty* input list writes 1 in the game (bVar11 init 1, 0x1b1a78:91-103), the port writes 0 (`triggers.rs:242`). |
| 0x1b1fc8 | `Trigger_Hit` | Contact bits from the object's collider contacts: class 3 → 8 (player), 2/0x4A → 0x10 (drone/pickup), 5 → 2 (object) [M 0x1b1fc8:24-47]; for **types 1/2 only** (0x1b1fc8:56): mask 0x18 + filter list — 0xB needs a drone, 0xE the player, else drone skin-hash low 16 bits; **mask bit 1 seeds the result with `ch[+0x2E]`** (= filter[4], placement p6) instead of 1 (0x1b1fc8:63-68); mask & 2 walks the class-5 contacts against the filter list (0x1b1fc8:124-159); default `mask & contacts`. | `triggers.rs:185` (`hit`) | partial | Contact bit 2 (class-5 objects touching) has no port touchers (touchers are Bond + enabled drones only, `drone.rs:3892-3894`); mask bit 1's channel gate is missing; the type 1/2 gate on the 0x18 branch is missing (the port applies the filter semantics to every mask); mask 2/4/0x20-0x3F arms fall into the generic AND (`triggers.rs:202`) which is right for 8/0x10 but silently drops bit 2. The port's contacts come from its own capsule test (grow +0.1, `triggers.rs:174`) not the game's collider contact list — equivalent for the plates measured so far [S]. |
| 0x1b2318 | `Trigger_Activate` | Fired by `Trigger_Update` case 10 and `Player_Activate`. Guard: activator must be class 3; cancel if `ch[0x63/0x62/0x61/0x60]`; key gate `+0x26` (p1) off → locked message (label 0x2000021) + 3 s message timer, **return, no load**; type 2 (Touch): `ch[out] = 1` if activated-flag; type 1: + self-delete; type 0xD: movie push; **type 10**: if `ch[+0x28]` (p2 auto) on → set success `+0x2C` (p4) … then **both paths fall into**: `Music_Event(3,1)`, `GS_PauseGame(1)`, `ResetMap_LevelToLoad(level,1,0)`, `GameFlow_PushState(300.0,7)`, type ← 99 (never again), `Player_RamSave`. The auto-off path sets `+0x2A` (p3) first — **the level still loads**. | `triggers.rs:210` (`activate_request`) + `drone.rs:4007-4017` (LOAD_LEVEL arm) + `door.rs:580`/`drone.rs:3612` (E-key) | partial | The port sets the success channel unconditionally and the fallback **p3** only when the *key gate* blocks (`triggers.rs:216-222`) — the game sets p3 when the *auto channel* is off and still loads; `Music_Event(3,1)` ported (`drone.rs:4013`); `GS_PauseGame`, the state-7 push, type←99 (→ `gone` latch), `Player_RamSave` folded into the synchronous level swap; TouchOnce/Touch E-key arms ignore the mask&4-activated check (`triggers.rs:225-227`). **FIXED (s/fix-level2)**: `activate_request` begins the TOUCH_ONCE/TOUCH arms with the `mask & 4` gate (+ out ≠ 0 on type 2) and refuses a spent (`gone` = type-99) LOAD_LEVEL — the mask-8 plunge/death triggers can no longer fire the death NIS from the use key; the success/fallback write order now matches the game (auto on → success, else fallback, both → load). The synchronous carry-over (`Player_RamSave`/`RamLoad`, the gated start scan, `Player_InitWeapon`) is ported in `player.rs` + `nf-view`'s `load_with_carry` (playtest F1/F2/F4/F5; WORKLOG fix-level2). |
| 0x1b2790 / 0x1b31a0 / 0x1b30a8 | `SoundTrigger_Update` / `MusicTrigger_Update` / `InverterTrigger_Update` | Their channel reads + `Sound_Play` / `Music_Event` / invert. | none | not started | 07000005 placements 0xF9/0xFB/0x41: none in the main map [D per the survey in `levels.md` §2], so nothing playtest-visible yet. |
| 0x1adea8 / 0x1b02a8 | `Switch_Update` / `Switch_Activate` (class 0x39, placement 0x29) | Use-key/periodic channel **toggle** with debounce (`+0xF4` mirror, `+0xF6` repeat timer, `SP_UnPause` on flip); Activate: toggle `ch[out]` when the E-key activator is class 3, the gate `+0x14` is off, and the linked ScriptPlayer is in state 0. | none | not started | Switch objects on 07000005: none in the main map [D]; other levels use them (the FuseBox 0x31 family is separate). |

### 3.3 Add/remove during the walk

`Trigger_Update` never adds objects; deletion goes through `obj+0xFE |= 1` (the delete pass, §2 step 8). Runtime
adds into the walk head: script touch triggers (`Script_CreateEntity` 0xEA/0xEB — the port re-creates them per tick
from live script entities, `drone.rs:3916-3934`), spawner drones (prepended by `control_add_object_to_list` — the
port pushes to `llist` and sorts placements descending, spawned drones have placement `usize::MAX`-style high keys
[S: the sort tie-break puts them last/first — see divergence 9]).

## 4. Switch channels

Four 256-byte tables at 0x26FC90 (`switch_channels`), 0x26FD90 (`_prev`), 0x26FE90 (`_hold`), 0x270190
(`_time` u32[256]) plus `switch_channels_MusicVars` u16[256] [M symtab; L ee_start_05.bin@0x26fc90 — at the
07000005 snapshot only ch 0x45 is on].

| Address | Name | What it does | Rust location | Status | Gaps |
|---|---|---|---|---|---|
| 0x134310 | `Init_SwitchChannels` | All four arrays ← 0; `_time[*] ← GameState+0x34` (the global frame counter, **not** per-level) ; `MusicVars ← 0`. Called from `ResetMap_GameInit` when `IsWarmReset == 0` (warm resets keep channels!) [M 0x1d5080:47-49]. | `triggers.rs:68` (`Channels::default`) | partial | `_time` starts at 0 in the port (absolute base — only differences matter, so SEQ_AND is unaffected); live [L ee_start_05.bin@0x270190] every entry holds 108150 (the init-time `GameState._52_4_`) except ch 0x45 = 108152, against the then-current counter 108153 — the seeding from the global counter is real; warm-reset channel preservation unported (the port has no warm reset); `MusicVars` unported. |
| — | set (`ch[x] = v`) | Writers: `Trigger_Update` (types 1/2/3/4/5/0xC), `Trigger_Activate`, `Trigger_Calc` (clears), `Script_EventHandler` 0xD/0x11, `Switch_Update/Activate`, `Mission_MonitorObjectives`, `Copter_Update`, `Sensor_Update`, `Hurt_Update`, `GT_Update`, `Emitter_Update`, `Searchlight_Update`, `SpaceLaser_Update`, `MP_CheckForEndCondition`, `Player_Init`, `SSys_Monitor`, `DroneFunc_SetMissionFailReason` (0x60/0x63), `DroneSpawner_Control`, `Door_SetState` (`ch_open` mirrors motion, timestamped [M 0x135618:26-29]), `HUD_UpdateSpacePane`. | `triggers.rs:72` (`Channels::set`) + per-system | partial | The game **re-stamps `switch_channels_time` on every non-zero write** (e.g. 0x1b1a78:87,107,188-189) while `Channels::set` stamps only on the rising edge (`triggers.rs:74`) — stale timestamps feed the SEQ_AND window (divergence 6); the OR-form (`ch[x] \|= hit`) vs the port's set is equivalent for 0/1 values; `Door_SetState` writes `.v` + `.time` directly in the port too (`door.rs:314-319`), matching the game [M]. |
| — | pulse | A Touch trigger's output is on exactly while touched: cleared in `Trigger_Calc`, re-ORed by the type-2 pass, consumed by anything that reads the channel later that frame (muxes, spawners, script players, LOAD_LEVEL gates). | `triggers.rs:294-296` | ported | Order inside the frame: the game runs Touch triggers in `Trigger_Calc` (before the walk) — the port does the same (`drone.rs:3895` before the walk) [M]. |
| — | read | Anyone; `_prev` only `Door_Update`'s lock edge and `Player_Weapon[0x48]`; `_hold` nobody else. | `triggers.rs:71` (`on`) | partial | `_prev` semantics: divergence 5 in §7 (snapshot timing). |

## 5. Script players (SP_*) and the script runtime (Script_*)

`SCRIPTPLAYER` (0x38 bytes at `obj+0xE0`): `+0x0C/+0x10` SCRIPTINFO A/B; `+0x14/+0x18` hit health/accumulated
damage; `+0x1C/+0x20` script hashcodes A/B; `+0x24` state; `+0x26/+0x28` modes A/B; `+0x2A` current; `+0x2C`
activated latch; `+0x2E` trigger mask; `+0x30` out channel; `+0x32` gate channel; `+0x34` countdown; `+0x35` stay
(delete at end); `+0x36` dead; `+0x37` bullet filter; `+0x38` loadable-started flag. `SCRIPTINFO` (0xAC0):
`+0x40` player-return matrix, `+0x80..` callback, `+0x94` scale, `+0x98` hashcode (ScriptCam source), `+0x9C`
delay, `+0xA0` clock, `+0xA4` fade, `+0xA8+0x28i` streams, `+0xAA8` flags (0x10 keep objects, 0x20 camera-less,
0x40 camera script, 0x80 re-enable drones, 0x100 player return), `+0xAAA` stream count, `+0xAB0` length, `+0xAB2`
play flags (1 playing, 2 done, 8 looping), `+0xAB3` stream cursor, `+0xAB5` fade flags, `+0xAB6` player mode.

| Address | Name | What it does | Rust location | Status | Gaps |
|---|---|---|---|---|---|
| 0x1db9a8 | `SP_CreateScriptPlayer` | Created only when p0 or p1 ≠ 0 (params at `level_tag+0x2C+4k`); masks/out/gate/scale per the doc; `ch[out] = 0` at create [M 0x1db9a8:72]; start state: **6** if mask 0; then mode A == 1 → **7**; mode A ∉ {0,1} → `SP_LoadScript` now (which sets the state per mode — 1/2 for most, 6 for looping — overriding the 6 above even when mask == 0) [M 0x1db9a8:74-86 + 0x1dbc08:44-54]; class 0x1B. | `nis.rs:490` (`from_placement`) | partial | The port's formula `mode==1 → 7; mask==0 → 6; else 0` misses the game's third branch: mask == 0 **and** mode ∉ {0,1} leaves the port in state 6 where the game loads at create and sits in `SP_LoadScript`'s state (1 for one-shot modes) — a mode-8-mask-0 player would loop on the wrong arm; `switch_channels[out] = 0` at create is missing [S: harmless unless a previous level left it on]. |
| 0x1dbc08 | `SP_LoadScript` | Load/rewind the current SCRIPTINFO, then per mode: 1 → play, 8 → loop, else load+1 update+stop (state 1). | `nis.rs:512` (`load`) | ported | The `isLoadable` streaming arm is ported (fix-cutscenes): the level's type-0xC records → pack `078/079/07A000NN.bin` (the (flag+7) nibble) loaded in `nf_view::load_with_carry` at level start; the game streams lazily on first use. The `GameFlow_PushState(12.0, 0xD, 0xFF)` loading screen of `SP_Update` state 0's `isLoadable` arm is not (no load screens in the port). |
| 0x1dbda8 | `SP_Update` | Dead/gate skip; `Script_Update` on the current script; trigger = (`SP_Hit` \| 4 when `+0x2C`) & mask; states — 0: trigger → (streaming push \|) load+play → 2 (gated on `Mission_Status`, health > 0 and the 0x6000994 music-event exception); 1: trigger → play, countdown 5, channel out = 1; 2: `SP_UnPause` while mask 1 + channel on, action 0x19 fast-forwards (`Script_FFwd`, death NIS exempt), on done → swap/state 3 or stay → 5; 3/4 mirror 1/2 with out = 0; 5: delete; 6: trigger → delay 0, free+load other → 2; 7: load. Tail: channel out = playing — gated on the **trigger only** (no mask-bit-1 condition [M 0x1dbda8:221-229]); hit-health unpause (`SP_GetHitDamage` accumulates into `+0x18`, past `+0x14` → `SP_UnPause` − threshold). | `nis.rs:528` (`update`) | partial | `SP_Hit` bullet/object flags (2, 8, 0x10) never set — a script waiting on a bullet hit never starts (07000005 #37/#38/#71/#284/#298/#320 use only the channel bit [D]); the hit-health unpause path (`SP_GetHitDamage`, `+0x14/+0x18`) is missing entirely; the action-0x19 skip (`Script_FFwd`) is missing; state 0's Mission_Status/health gates are missing; `SP_UnPause`'s channel-timestamp write side effects match [M 0x1dbe20-0x1dbe9c vs `nis.rs:549-553`]. |
| 0x1dcc30* | `SP_Hit` | Contact bits over the player's hit list (channel 1, bullet 2, activated 4, objects 8/0x10). | none (only bit 4 via `activated`) | not started | See above. |
| 0x1dcad8 | `SP_SwitchToScript` | Runtime script swap (used by cut-scene chains). | implicit (state 6 swap) | partial | No direct port; the port swaps only via the state machine. |
| 0x1da998 | `Script_Update` | Playing only; `delay == 0` → run every stream (`Script_Run` ×0xAAA), clocks += FRAME_RATE_MUL; else delay−1; then the fade tail (`+0xA4` vs `+0xAB4`, sets 0xAB5 0x20 → skips interp cases 2/4 [M 0x1d9a08:146, 205]; `Camera_SetFade(0x42200000,…)` near the end (length − 0x14) for hashes outside {0x60004FA, 0x6000080, 0x60008C8, 0x6000994}, with {0x600098A, 0x6000952, 0x6000991} breaking out of the whole pass [M 0x1da998:51-78]). | `nis.rs:138` (`update`) | partial | The fade tail (camera fade-in/out at cut-scene ends; the two exemption sets above) is not ported — cut-scene fade is the renderer's lack, noted in `script-player.md`; the delay decrement matches (`+0x9C` −1 when > 0 [M 0x1da998:113 vs `nis.rs:147-149`]). |
| 0x1d8548 | `Script_Run` | Per stream: `if clock ≤ length` → `Script_Interp`; then run commands while `clock ≥ wait-until`: 4 StreamEnd, 5 wait = word, 6 done-flag at length, 7 EntityStart, 9 EntityEnd, 0xA AnimStart, 0xC AnimEnd, 0xD CameraStart, 0xF CameraEnd, 0x12 EventHandler, 0x13 FadeStart, 0x14/0x15 Sprite, 0x16/0x17 Sound, 0x18/0x1A Light, 0x1B/0x1D SubScript, 0x1E Text. StreamEnd: wait = time; held while `clock ≤ time && clock < length`; looping → restart at loop-start − 1 + done flag; else play &= ~1, done. | `nis.rs:161` (`run`) | partial | Ops 0x13-0x1E other than 0x16/0x17 **stop the stream** in the port (`nis.rs:262-266`) instead of consuming their arguments — a script with a fade/sprite/light/subscript/text command freezes that stream (recorded as `Request::Event 0x80|op`); op 6's done-flag placement matches [M 0x1d8548 case 6 vs `nis.rs:181-183`]. |
| 0x1d9a08 | `Script_Interp` | Case 1 entity: key interp (linear/spline) × each parent stream × the script matrix, scale → `obj+0xE8`, alpha → `+0x106`; case 2 animation: `AnimListBuild` + frame = clock − start + start (clamp [1, length], integer) + `AnimObjectUpdate` + footstep effect + ninja eyes; case 4 camera: `Script_GetInterp` → `Script_UpdateCamera`; case 5 sound: finished → kill, else volume (fade) + reposition at the object stream's object; case 6 light: matrix into the light object's fields (0x1d9a08:235-281); case 7 sub-script: recursively `Script_Update` (0x1d9a08:282-314); **case 3 (sprite) has no arm in the game's switch** [M]. | `nis.rs:381` (`interp`) + `drone.rs:3940-3974` (sound reposition) | partial | Case 2's anim world is the renderer's (`AnimObjectUpdate`, footsteps, `NDrone2_CreateNinjaEyes` missing — footsteps during cut-scenes don't sound, U-0xx); case 5's per-tick volume fade is missing (sounds play at fixed volume); `Script_UpdateCamera`'s `Camera_CalcViewAngles(key scale × 4/3)` ported as `fov = scale × 4/3` (`nis.rs:401`) [M 0x1d9a08 case 4 + doc]. |
| 0x1d94c0 | `Script_EventHandler` | 3 delay = p0−1; 4 `Player_Disable`; 5 `Drone_EnableAll(0, hash)`; 6 flag 0x100 + return matrix; 7 flag 0x80; 8 `Drone_CoderCreate`; 9 `Player_Cam2Mode`; 10 `Break_DoBreak`; 0xB weapon byte → prev stream's Anim; 0xC keep objects; 0xD `ch[p0] = p1` (+time); 0xE stream flag 1; 0xF callback; 0x10 flag 0x20; 0x11 `ch[p0] ? ch[p1]=p3 : ch[p2]=p3`; 0x12 `Player_RamSave` + `ResetMap_LevelToLoad(p0,1,0)`; 0x13 `ScriptCam = hash`. | `nis.rs:320` (`event`) | partial | Events 5 (`Drone_EnableAll`), 8, 9, 10, 0xF fall into the catch-all `Request::Event` (`nis.rs:377`) — **event 5's absence is divergence 3's other half**: the port cannot honor the game's own cut-scene drone disable; 0xB's deferred weapon write matches [M 0x1d94c0:152-158 vs `nis.rs:340-345`]; 0x11/0xD/0x12/0x13 match [M]. |
| 0x1db588 | `Script_Play` | play \|= 1 (+8 when looping); **if the script is a camera script (`+0xAA8 & 0x40`): `ScriptCam = hash`** + `Music_Event(6,1)` for hashes {0x600098A, 0x6000952, 0x6000991}; `Text_FlushAllSubtitles`. | `nis.rs:128` (`start`) | partial | ScriptCam is not modelled as a global (the port keys cut-scenes off `Player_Disable`); the death-NIS music event is not posted. |
| 0x1da880 | `Script_Free` | Kill every stream (respecting keep-objects); `LoadableReload` when loadable; flag 0x80 → `Drone_EnableAll(1, hash)`; flag 0x100 → `Player_Enable(glb_players, +0x40, +0xAB6)`; **`if (ScriptCam == hash) ScriptCam = 0`**. | `nis.rs:307` (`free`) | partial | The drone re-enable and the ScriptCam clear have no port counterpart (the port's `PlayerEnable` request drives `scene.rs:414-421` only). |
| 0x1db420 | `Script_StreamEnd` | See `Script_Run` 4. | `nis.rs:173-197` | ported | The port holds the stream on the end command while `clock ≤ time && clock < length` (game rewinds onto it — same observable state [S]). |
| 0x1d9860 | `Script_KillStream` | Entity/anim: delete unless keep-objects (then detach only); sound: stop; camera: pop. | `nis.rs:297` (`kill`) | ported | The camera pop posts `Request::Camera{start:false}` → `Music_Event(0xC)` [M 0x1d9860 vs `nis.rs:303-305`]. |
| 0x1d8d78 / 0x1d8968 | `Script_EntityStart` / `Script_CreateEntity` | Entity object per type (0x20 Break, 0x22 Ladder, 0x29 Switch, 0xE1 Destroy, 0xEA/0xEB triggers, 0xF0 Pickup, 0xF2 emitter, 0xF5 AI volume, else plain), parent chain, spline flag. | `nis.rs:270` (`entity_start`) | partial | The typed entities are stored as `Obj::Entity{kind}`; only 0xEA/0xEB gain trigger volumes and the generic ones gain collision (`drone.rs:3900-3914`) — Break/Ladder/Switch/Pickup/AI-volume behaviours are not created (G-026/G-029). |
| 0x1d8f40 | `Script_AnimStart` | Anim object at the origin, root-follow (start frame clamp, `AnimGetFirstTrans`, per-tick root delta). | `nis.rs:285` (`anim_start`) + renderer's discrete0 | ported | The root-translation path runs through `World::discrete0` (the port reads the pose from the anim system); `NDrone2_CreateNinjaEyes` (skin 0x0500001E) not ported. |
| 0x1d9140 / 0x1db348 | `Script_CameraStart` / `Script_CameraEnd` | Camera stream: `ScriptCam = hash` (before the 0x20-flag check), `Input_ClearAllActions`, CAM_INFO alloc, keys first..last−1; End pops. | `nis.rs:231-252` (op 0x0D/0x0F) | partial | `Input_ClearAllActions` unported (queued inputs survive into the cut-scene); `ScriptCam` unported (§7 item 3). |
| 0x1d8088 / 0x1da158 | `GetSplineWeights` / `Script_GetInterp` | Binary search on ⌊clock⌋+0.001; linear lerp + `Quat_Slerp_Acc` + scale/+0x24; Catmull-Rom window i−1..i+2. | `nis.rs:431` (`keys`) | ported | The spline rotation slerp weight is *suggested* (U-040). |

\* symbol-table address.

## 6. Level flow: ResetMap_* and the GameState stack

`GameState` (0x2A3768, 88 bytes) — offsets below are the decomp's decimal ones; live values [L
ee_start_05.bin@0x2A3768]: `_8_4_` = 0x07000005 (level to load), `_12_4_` = 0x07000005 (current level),
`_16_4_` = 0x7100005 (movie code), `_52_4_` = 108153 (the global frame counter — `switch_channels_time`'s clock),
`[0x40]` = 0 (level-ending), `[0x50]` = 0 (pause), `[0x51]` = 1, `_84_4_` = 0 (MP).

| Address | Name | What it does | Rust location | Status | Gaps |
|---|---|---|---|---|---|
| 0x1d3800 | `ResetMap_LevelToLoad(level, warm, c)` | Guard `level != -1`; `IsWarmReset = warm`; from states {2,7,8,0xD,0xE}: `GameState[0x40] = 1`, `_84_4_ = 0`; `Menu_IsDrivingLevel` → state 9 (driving) else: movie-code mapping 0x070000xx → 0x710000xx (per-level switch, 0x0700001B → 0x7100018), `_8_4_ = mapped`, `GameFlow_PushState(0, 3, 0xff)` (the load state). | `lib.rs:623-645` (`pending_level` → `load`) + `drone.rs:4007-4017` / `nis.rs Request::LoadLevel` | partial | No GameFlow stack (the port swaps the scene at the end of the tick); the movie-code 0x71 prefix is dropped (the port loads `{hash:08x}.bin` directly — the on-disc archive name is the 0x0700xxxx form [D]); `GameState[0x40] = 1` matters in the object handler's cel-light pass only; driving-level branch n/a. |
| 0x1d6360 | `ResetMap_Load` | The state-3 worker, four `LoadState` phases: 1→2 idle; 0: resources reset, `ResetMap_GameInit`, load the *movie* archive (0x71000xxx → intro ScriptPlayer via `SP_Create` when the hash table has it), `GameFlow_PushState(0,6)`, `_16_4_ = _8_4_`, `_8_4_ &= 0xff00ffff`, background movie, → 1; 2: `GetLevelToLoadFromMovieCode` → `_8_4_`; 3: `ResetMap_GenLoadScreen`, full reset, `ResetMap_GameInit`, LoaderLoad type 0 then 1, `build_MakeCelOverlapBox`/`build_link_objects_to_rooms`/`parsemap_block_map_data_dynamic`, `build_LinkDoors2Portals`, `AIPath_BindNodes`, `Mem_Shrink`, `Player_Start`, `MP_Start`, `Drone_PostLoad_Init`, `Cameras_PostLoad`, `GameFlow_PopState` + `PushState(0,4)` (the game), → 0. | `lib.rs:90` (`load`) + `scene.rs:1035` (`build`) | partial | One synchronous build; no load screen, no background movie, no `LoaderLoad` two-phase (the port's `Level::parse` covers it); the intro ScriptPlayer (`SP_Create` for the 0x60009xx cut-scene hashes — e.g. 0x7100005 → 0x06000961) is not created: the level-intro cut-scenes never play in the port [D — the movie-code → hash table at 0x1d6360:48-90]. |
| 0x1d5080 | `ResetMap_GameInit` | `_12_4_ = _8_4_ & 0xff00ffff`; MP flag; `_36_4_ = 0`; `[0x50] = 0` (unpause); `[0x51] = 1`; `GS_PauseGame(0)`; then ~35 init calls in order (Shards, `Player_ResetStartPos`, `Env_Reset`, world alloc, Txt, `Script_Init`, `control_init_object_lists`, View, Sprite, Text, Effect, Light, `Drone_LevelReset`, `Door_Init`, `Copter_Init`, `Sensor_Init`, Coll, Bullet casings, AnimCache, `AIPath_Init`, Creature, MP, `Camera_CreateCameras`, `Drone_PreLoad_Init`, MenuManager, Car, SSys; `Init_SwitchChannels` when cold; `Mission_Init`). | `scene.rs:1035` (`build`) | partial | The order is the port's own build order; `Drone_LevelReset`/`Drone_PreLoad_Init`/`Drone_PostLoad_Init` are folded into the spawner/drone construction (`drone.rs:3440-3460`); `switch_NO_DRONES` debug flag absent. |
| 0x1c9e80 / 0x1c9f38 / 0x1c9eb0 / 0x1c98c8 | `GameFlow_GetState` / `PushState` / `PopState` / `Main` | A 64-entry state stack (`GameStateStack` 0x2C7398, `StackIndex` 0x30CECE, wraps at 0x3F [M 0x1c9f38:35-39]); push maps 7→8 and 0xD→0xE via `Camera_SetFade(6, fade, 1)` — **no push when the fade fails** — and 0xB→0xC; push/pop set `GameState[0x40]`: 1 for states {1,3,4,5,9}, 0 for {2,6,7,8,0xA,0xC,0xD,0xE} [M 0x1c9f38:41-58, 0x1c9eb0:14-31]; `GameFlow_Main` dispatches the per-state updates each frame (3 → `ResetMap_Load`, 4 → `Game_Run` state 4 = the playing state, 6 → movie, 7 → load screen, 0xD/0xE → streaming loads). | `lib.rs:623-687` (the RedrawRequested block plays the role of state 4 + the swap) | STANDIN | No stack: `pending_level` + a synchronous rebuild. Anything the game does between PushState(3) and PopState (load screen fade, the movie, `PS2_SFXWaitTillFinished`) is absent. |
| 0x1d5de0 | `ResetMap_GenLoadScreen` | Builds the loading-screen level. | none | not started | |
| 0x1d5258 | `ResetMap_LevelCode2Img` | Level-code → image mapping (story order). | `levels.md` §1 table | ported (as data) | — |

\* `GameStateStack` / `StackIndex` — addresses in `work/action-symtab.tsv`.

## 7. Top 10 divergences likely to cause playtest bugs

Ranked by (a) how often the affected code runs, (b) how visible the wrongness is. Each row: the game's behaviour
[evidence], ours [evidence], and the playtest symptom it explains or risks.

**1. Drones freeze during every cut-scene, not just the ones that ask for it (and the port ignores the game's own
freeze request).** Game: `control_movement_object_handler` runs whenever `GameState[0x50] == 0` — `ScriptCam` plays
no role [M 0x1c9790:30-35]; `Drone_Control` runs whenever `NPCGlobals != 0 && !Drone_bDisableSystem` [M
0x139268:39-42]; the only scripted drone freeze is `Script_EventHandler` event 5 → `Drone_EnableAll(0)` →
`NPCGlobals = 0` [M 0x1d94c0:122-124, 0x1383e8:23], undone by `Script_Free` when the script set flag 0x80 (event 7)
[M 0x1da880:41-43]. Port: `Drones::paused = self.disabled` (set from any `PlayerDisable`/`PlayerEnable` pair,
`scene.rs:673`) skips the whole per-drone update (`drone.rs:4065`), and event 5 has no handler (`nis.rs:377`
catch-all). Effect: guards stand still during cut-scenes that the game keeps them alive through (they can be
scripted to move/look — e.g. the gatehouse cut-scene's guards), and a script that *does* fire event 5 gets its
guards frozen only by accident of the port's coarser gate. The B-10 symptom ("cut-scenes froze when drones were
skipped") was the *opposite* gating bug: freezing the whole tick stopped `Trigger_Calc` + `SP_Update`, so
`PlayerEnable` never arrived; the fix moved the freeze inside the drone loop, which restored scripts/triggers/doors
but froze drones game-wide. **The correct shape is: port event 5/7 (`Drone_EnableAll`) as the drone gate and drop
`Drones::paused`.** — **FIXED (s/fix-core)**: `Drones::npc_enabled` (NPCGlobals) replaces `Drones::paused`; event 5 →
`Request::DroneEnable{on:false}`, `Script::free` → `DroneEnable{on:true}` on flag 0x80 [M 0x1d94c0 case 5,
0x1da880:41-43, 0x1383e8]; `DroneSpawner_Control`'s gate [M 0x137e68:21] and the `Drone_InitComms` shared passes'
gate [M 0x139520:124] are ported too (ProcessOpponents, ProcessDroneSight+FindAlertedDrones, 0x97, sound alerts,
0x96 alarm, CheckAlarmRaised, target/cover rebuilds). The hearing list drains during a freeze so re-enabling
doesn't burst stale alerts.

**2. The scene.rs comment that justifies `Drones::paused` misreads `Game_Run`.** `scene.rs:667-672` says the game
runs the object handler only when `GS_IsPaused == 0` **and `ScriptCam == 0`**. The decomp gates it on
`GS_IsPaused(-1) == 0` alone [M 0x1c9790:30-35]; `ScriptCam` picks `Text_Update2Line` vs `Text_Update` (line 17)
and the else-branch's `Light_Update` (and — with `psiMovieFinished` — the 2Line text choice). Any future work keyed to that comment (e.g. "skip Sim when a camera is up")
would re-introduce the B-10 freeze. Fix: correct the comment when touching this code (doc-only task, so it stands
here). — **FIXED (s/fix-core)**: `scene.rs` no longer sets any paused flag from `Sim::disabled` and the comment now
states the `GS_IsPaused`-only gate [M 0x1c9790:30-35].

**3. The pause byte does not exist; the port cannot distinguish "pause menu" from "level transition" from
"cut-scene".** Game: one byte `GameState[0x50]` stops the whole object handler while menus/mission screens run
above it [M 0x17d1e8, 0x17d178, §0 caller list]. Port: `App.paused` (viewer pause, ticks stop entirely — the game
keeps rendering while paused) and `Drones::paused` (§ item 1). Risks: the level-transition pause hides one frame of
post-fire trigger updates (the port's synchronous swap runs the whole tick first); a future pause menu ported
literally would stop the drone/audio state the game keeps ticking. No current playtest symptom, but every
"why did the level end / not end" report routes through this gate.

**4. Door auto-close blocking zone is the wrong sphere in the wrong place (the 07000001 auto-close symptom).**
Game: the auto-close proximity scan tests `Vec_SqDist3D(door obj+0x80, actor obj+0x80) < (r_door + r_actor)²·2`
over every non-hidden class 2/3/0x12 object [M 0x1363c8:52-68 and 121-137]; `obj+0x80` is the door's **live world
sphere** — `Door_Interp` sets `obj+0xFA |= 0x20` [M 0x135738:28] and the moved pass rebuilds `obj+0x80`
[0x1336f8:209], so a slide door's blocking zone **follows the open door leaf** (2+ m away from the frame).
Port: `door.rs:220` `blocked()` uses the door's **fixed placement position** and a hardcoded radius `1.0` — the
door's real gfx radius parameter is parsed but **ignored** (`_door_r`). Effect: while the port's zone sits on the
empty door frame, the game's zone sits on the moved leaf — anyone standing in the open doorway blocks the game's
auto-close but not the port's: **the port closes doors the real game leaves open**, which is exactly the 07000001
"auto-close after 6 s" report (A-12c proved the 6 s timer itself is real: flags 0x02/0x03 have keep-open bit 0x20
clear [M 0x136624 disassembly, ee_start_07000001.bin]). A PCSX2 trace (U-046) plus this zone fix is the answer.

**5. `switch_channels_prev` is snapshotted at the wrong end of the tick — single-frame channel pulses miss doors'
lock edges.** Game: prev/hold are copied **after `Trigger_Calc`, before the object walk** [M 0x1336f8:52-53], so
`Door_Update`'s lock-edge test (`prev == 0 && now != 0` [M 0x1363c8]) sees a channel that a trigger or script
pulsed earlier in the same frame. Port: `Doors::end_tick` snapshots after the whole tick (`drone.rs:4160`), so the
same pulse is only visible to the *next* tick — and if it lasted exactly one tick, the edge never appears.
Effect: a 1-tick lock-channel pulse (script event 0xD writing 1 then a trigger clearing it) fails to unlock doors
in the port; `Player_Weapon`'s `ch[0x48]` prev read (0x1a24c8:247) has no port counterpart yet. Fix shape: snapshot
the channels right after `triggers::calc` in `Drones::tick` and feed `doors.update` from it. — **FIXED
(s/fix-core)**: `Doors::end_tick` is called right after `triggers::calc`, before the walk.

**6. Mux (0xEC/0xED/0xEE/0xEF) parameters read from the wrong slots, and the SEQ_AND window is the wrong
predicate — 7-input muxes, any mux with a non-zero p8/p9/p10, and all SEQ_AND chains misbehave.** Game: the mux wrappers shift every param down by one (out p0 → slot 1, inputs = raw
**p1..p7**, mask = raw **p8**, lock = raw **p9**, hide = raw **p10**) [M 0x1b2c68/0x1b2e08/0x1b2d38 loops]; Touch
wrappers keep slots 2-7 in place [M 0x1b2aa8/0x1b2b88]. Port (`triggers.rs:150`) reads mask q(9), list
q(1..8)\\{7}, lock q(10), hide q(11) — i.e. p7 excluded and p8 included where the game includes p7 and excludes
p8, and lock/hide off by one slot. 07000005's muxes have ≤ 4 inputs and p7..p11 = 0 [D], so the test level cannot
distinguish them — but `AND` also stops at the **first zero input** in the game [M 0x1b1a78:93-100] while the port
filters zeros and ANDs the rest (`triggers.rs:241-243`): a mux with a deliberate 0 mid-list ANDs past it. Same
table, different bug: the port's SEQ_AND window `b >= a && b - a <= 4` (`triggers.rs:249`) requires each input
within 4 frames *after* the previous one, while the game's predicate is `time[prev] − 4 <= time[cur]` [M
0x1b1a78:135-136] — no upper bound at all (and the game re-stamps times on every non-zero write, the port only on
rising edges): a correct-order SEQ_AND chain with a >4-frame gap works in the game and never fires in the port.
Fix shape: correct `from_params`'s `_` arm, make the mux loops stop at the first zero, and use the game's
window predicate. — **FIXED (s/fix-core)**: `from_params`'s mux arm reads mask q(8), list q(1..=7), lock q(9),
hide q(11); LOAD_LEVEL's mask is the hard 0xE [M 0x1b2950]; AND/OR scan the input prefix (zero slot ends the list;
empty AND writes 1 [M 0x1b1cbc]); SEQ_AND's window is `time[prev]−4 <= time[cur]` [M 0x1b1e04]; `Channels::set`
re-stamps on every nonzero write. **Slot-mapping correction (s/fix-core audit)**: the table above mis-traced the
wrapper's loop bound — it writes slot[k] = p(k−1) for k = 2..7 only (`while (uVar3 < 8)`, 0x1b2c68), so slot 8
keeps raw p8 and slots 9-11 are untouched; `Trigger_Create` copies 7 inputs from slots 2..8 [M 0x1b1720]. The mux
inputs are therefore raw **p1..p6 + p8** (p7 is overwritten and lost), mask = **p9** (default 2 when 0
[0x1b1720:80-84]), lock = **p10**, hide = **p11** — consistent with the untouched-slot LL row below. Truck chain (MuxIn): **not affected** — 07000005's muxes read ≤ 4 inputs with
p7..p11 = 0 [D], so the old slot bugs were invisible there; `level_end_live`'s route-a chain test passes
before and after (5 passed).

**7. Script touch triggers don't age: TouchOnce deletes itself without the glist 0x10 check.** Game: `Trigger_Update`
case 1 deletes the trigger only when its glist flags carry 0x10 [M 0x1b1a78:74-77,119-123]; the channel write is an
OR (`ch[out] |= hit`) and case 1 **falls through into case 2** every frame while the channel is off [M
0x1b1a78:73-88]. Port: `triggers.rs:236-238` deletes unconditionally and ORs via `set` — equivalent while the
channel only feeds OR-style consumers, wrong the moment a mux ANDs that channel with others *and* the plate is
re-touchable. Low probability on current levels [D], cheap to fix with `from_params`'s missing glist-flag read. — **FIXED
(s/fix-core)**: `Trigger.glist_0x10` models the check (live data: every TouchOnce carries the bit [L]);
case 3's Bond-moment delete (out 0x65-0x6E) uses it too.

**8. The level-intro cut-scenes never play (the `SP_Create` intro-script path is unported).** Game:
`ResetMap_Load` phase 0 loads the movie archive (0x710000xx) and creates a ScriptPlayer for the level's intro NIS
from a 14-entry movie-code → hash table (0x7100001→0x0600095A, 0x7100005→0x06000961, the 0x730000xx driving codes,
…) [M 0x1d6360:39-99]; levels whose code is not in the table get no intro in the game either. Port: `lib.rs:90` builds
the level archive directly and never creates the intro player. Effect: starting a level begins gameplay
immediately where the game plays the intro fly-by — users see "missing intro", and the `GameState._16_4_` movie
code machinery (`GetLevelToLoadFromMovieCode`) has no counterpart. — **NOT FIXED in s/fix-core**: the intro
ScriptPlayer is created in `nf-view/src/lib.rs`'s build path, which this task's file scope (triggers/nis/drone.rs +
scene.rs's drone-gate lines) does not cover.

**9. Runtime-spawned objects update at the wrong end of the control list.** Game: `control_add_object_to_list`
prepends, so spawner drones and script-created objects update **before** every placement object (they see this
frame's channels first and their channel writes are visible to the placements behind them) [M 0x134698 prepend +
the doc's "newest first"]. Port: the walk sorts placement indices descending with the tag tie-break **ascending —
drones (tag 0) before triggers (tag 1) on the same placement index** (`drone.rs:3981-3987`); pickups get an
explicit first pass (`drone.rs:3989`), script trigger volumes are re-created each tick (before the walk), but
**spawner-spawned drones** ride their *template placement's* index and script *entities* have no list position at
all. Effect: ordering-sensitive chains — a gate guard spawning onto a TouchOnce plate the same frame, or a script
entity's channel feeding a placement mux in the same tick — resolve one tick late. The A-12e truck-chain test
passes because its chain spans multiple ticks [M test]; a one-tick chain would not. — **PARTLY FIXED (s/fix-core)**:
spawner drones now carry `spawn_seq`/`spawn_frame` — newest-first ordering [M 0x134698 prepend] and the spawn-tick
skip (the game's walk cursor is already past the head when the spawner runs). Script entities still have no list
position (their triggers get `usize::MAX - o`; the entities' channel writes ride the walk — the players update from
the walk now, see below). — **FIXED for the script players (fix-script-start-tick)**: `SP_Update` joined the
control walk at the player's placement (kind 5, `drone.rs`; `Scripts::update_player`), replacing the old
all-players-before-the-walk beat (`Scripts::tick` → `begin_tick` + `update_player`). A script a placement trigger
started now runs its first commands (`SP_LoadScript`'s trailing `Script_Update` [M 0x1dbc08:56]) inside the same
walk, after the trigger wrote its channel and before the spawners/drones behind it — the 07000005 wall-hug plate
(#301 → ch 0x43) freezes the NPCs on the touch tick and `DroneSpawner_Control` (0x137e68:21-23) returns at
`NPCGlobals == 0` with its kill latch off for the whole 1,400-tick scene, exactly the live measurement. The
requests drain after every player's update (the freeze must gate the later control funcs the same frame), and
`set_dynamic` moved after the walk (the game's `Collide_Update` 0x1336f8:238 is post-walk).

**10. The truck-ride level end (route a) fails in play for three stacked reasons — all outside the core loop's
correctness.** The chain is: truck script touch triggers 0x70 (parked) + 0x71 (bed, follows the truck) [D
0x06000A22] ∧ gate-guard touches 0x72 (plate at its post, (28.34, 37, 44.86)) [D placement 318 area] → mux 0x65 =
AND(0x70,0x71,0x72) → ScriptPlayer #320 (0x06000A23) → event 0x11 @ tick 300 → ch 0x1D → placement 118 LOAD_LEVEL
[all D; chain test-ported A-12e]. In play: (i) ch 0x22 (the guard's start channel) only turns on if Bond walks the
gatehouse plates 21/22/23 or the valley patch 305 **before** boarding — riding straight to the truck never spawns
the guard [M placements; A-12e]; (ii) the guard's patrol AI (guard-ai port map, G-026/027) walks it off the 0x72
plate — the tests pin it [S; A-12e]; (iii) divergence 1 freezes the guard during any cut-scene that runs while it
still needs to touch — and the toucher list only carries `enabled` drones (`drone.rs:3892-3894`). None of these is a core-loop bug: the port's `Trigger_Calc`/mux/SP/LOAD_LEVEL path verifiably
fires the chain when the inputs hold (A-12e test `route_a_gatehouse_walk_spawns_guard_and_ends_level`). What the
core loop owes this symptom: divergence 1 (drone gating) and 9 (same-tick ordering) — both addressed in s/fix-core;
the route-a test still passes (5 passed in `level_end_live`), i.e. the mux/LL fixes neither blocked nor unblocked
the chain (07000005's muxes read ≤ 4 inputs, p7..p11 = 0 [D]).

## Appendix: the per-frame call tree (addresses)

```
Game_Run 0x1c9790
├─ psiPreGame_Run 0x109520 / psiPostGame_Run 0x109580
├─ Input_Update (symtab)
├─ Sound_UpdateListeners (symtab)
├─ [sloflag == 0]
│  ├─ MenuManager_Update / MenuManager_Monitor 0x21d508 / Mission_Update 0x182c60 / MP_Update 0x184bd0
│  ├─ ScriptCam ? Text_Update : (GS_IsPaused ? · : Text_Update2Line)
│  ├─ UpdateAllShards / [DrawSnow] Env_Update 0x17bc88 → UpdateDrops 0x18c580
│  ├─ [GS_IsPaused == 0]
│  │  ├─ SSys_Monitor 0x1ad998 / Light_Update
│  │  └─ control_movement_object_handler 0x1336f8
│  │     ├─ Drone_InitComms 0x139520
│  │     ├─ Doors_Calc 0x135fe0
│  │     ├─ Trigger_Calc 0x1b1900 → Trigger_Update(type 2 only) 0x1b1a78 → Trigger_Hit 0x1b1fc8
│  │     ├─ prev/hold snapshot
│  │     └─ control_funcs walk (newest first):
│  │        ├─ 3/0x12 Player_Update 0x193600 (+ Player_CollisionHandler 0x1949a8)
│  │        ├─ 0x1A RB_Update / 0x1C Door_Update 0x1363c8 / 0x1D Trigger_Update 0x1b1a78
│  │        ├─ 0x1B SP_Update 0x1dbda8 → Script_Update 0x1da998 → Script_Run 0x1d8548
│  │        │    → Script_Interp 0x1d9a08 / Script_EventHandler 0x1d94c0
│  │        ├─ 2/0x11 Drone_Control 0x139268 → NDrone2_PreDroneControl 0x148db0
│  │        │    → NDrone2_ControlSTANDARD 0x1490f0 → NDrone2_PostDroneControl
│  │        ├─ 0x24 Searchlight_Update 0x1cb900 / 0x2F Pickup_Update / 0x30 DroneSpawner_Control 0x137e68
│  │        └─ 0x39 Switch_Update 0x1adea8 / 0x40 SoundTrigger 0x1b2790 / 0x44 MusicTrigger 0x1b31a0 / 0x53 Inverter 0x1b30a8
│  └─ Camera_UpdateAll
└─ (level transitions: Trigger_Activate 0x1b2318 → GS_PauseGame 0x17d178 + ResetMap_LevelToLoad 0x1d3800
    → GameFlow_PushState(3) → GameFlow_Main 0x1c98c8 → ResetMap_Load 0x1d6360 → ResetMap_GameInit 0x1d5080)
```

## 8. Audit record (this session)

Four independent auditors re-derived every [M] claim in §§0-6 and divergences 1-10 from the decompiles and the Rust
sources; all core claims reproduced. Corrections they forced into this document (before this revision):

- Address fixes: `Text_Update/2Line` are 0x1b1510/0x1b09f0; `control_add_object_to_list` is 0x134698;
  `GameStateStack` 0x2C7398 / `StackIndex` 0x30CECE; the `Drone_Control` gate is 0x139268:39-42; `Script_Free`'s
  `Drone_EnableAll(1)` is 0x1da880:41-43.
- `control_funcs`: Searchlight_Update is class **0x22** (0x24 = cloud_update); classes 0x23/0x3C/0x41/0x4B/0x50/0x54
  are populated (Creature/FuseBox/Swoosh/Mine/SpaceLaser/Apocalypse).
- Semantics: `Trigger_Update` case 4's window is lower-bound-only (`time[prev] − 4 <= time[cur]`, no upper bound —
  divergence 6 amended); the lock-path `+0xF0 |= 0x30` happens **when hide ≠ 0**; case 0xD pushes **30.0** (not
  300.0) and clears its input channel; `Trigger_Hit`'s mask-0x18 branch runs only for types 1/2 and its mask&1 gate
  reads filter[4]; the moved pass runs **twice** (cloned after `Collide_Update`); `Doors_Calc`'s copy source has no
  busy check (last writer wins); `Door_SetState` **does** timestamp `ch_open` and the port matches; the game's AND
  with an empty input list writes 1.
- SP/level flow: `SP_CreateScriptPlayer`'s create-time load (mode ∉ {0,1}) overrides state 6 even when mask == 0 —
  the port leaves such players in 6 (row amended to partial); `SP_Update`'s channel write is trigger-gated only;
  `Script_Interp` case 6 (light) and case 7 (sub-script) are read in the game; the intro-NIS table has 14 entries,
  not one per story level (divergence 8 amended).
- Rust line references re-verified (drone.rs 3981-3987 sort, tie-break **drones before triggers**, walk at 4000,
  script triggers 3916-3934, requests 3940-3974; scene.rs comment 667-672, `paused` assignment 673).
- Live snapshot corroboration added: `switch_channels_time` = 108150 everywhere except ch 0x45 = 108152 against
  `GameState._52_4_` = 108153 [L].

Not fully verified by the auditors: the disc-side placement parameter values (p7..p11 of 07000005's muxes) behind
divergence 6 — the wrapper decompile is measured, the level data was not re-parsed this session (no Rust toolchain
in the sandbox; `nf-extract placements` needs cargo). The A/B/C/D audits are recorded here as the task's evidence;
their verdict lines are in the session WORKLOG entry.
