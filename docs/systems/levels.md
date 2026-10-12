# Levels survey

The on-foot levels of **007: Nightfire (PS2, NTSC-U, SLUS-20579)**, as listed by ACTION.ELF's `FileList` (118 entries
in FILES.BIN; docs/formats/files-bin.md). Survey run with `nf-extract level-survey <iso>` (WORKLOG 0077): every level
archive is loaded through the same `Level::parse` path nf-view uses, and the per-level counts below are read back from
the parsed `Level`. Tags: **[M]** measured in code, **[D]** measured on the disc, **[L]** snapshot, **[S]** suggested.

The driving levels (DRIVING.ELF, EA EAGL formats in `DRIVING/*.VIV`) are **out of scope** for this area (PLAN §5,
phase 4) and are listed in §3 only.

## 1. Story order and status

The story order is the case order of `ResetMap_LevelCode2Img` (0x1d5258) — the level codes ascend in mission order.
Level names are the `UpdateMFX_*` function names (ACTION.ELF symbol table) and the `Txt_BindLabel` ids (*sourced*,
wiki.nightfire.dev, CC BY 4.0 — facts with attribution).

| Code | Name (MFX updater / mission) | Maps | Objects | Placements | Drones | Collision | Parse | Game logic¹ | Render² |
|---|---|---|---|---|---|---|---|---|---|
| 07000001 | Paris Prelude (Castle A) `UpdateMFX_01CastleA` | 54 | 215 | 331 | 20 | 190 | ok | ok | 07000005's systems³ |
| 07000002 | The Exchange (Castle B) `UpdateMFX_01CastleB` | 52 | 353 | 502 | 25 | 321 | ok | ok | ³ |
| 07000003 | Alpine Escape (Castle C) `UpdateMFX_01CastleC` | 51 | 347 | 511 | 20 | 335 | ok | ok | ³ |
| 07000004 | Enemies Vanish (Castle D) `UpdateMFX_01CastleD` | 50 | 54 | 70 | 1 | 42 | ok | ok | ³ |
| 07000005 | Paris Prelude (Mayhew A) `UpdateMFX_02MayhewA` | 51 | 178 | 363 | 16 | 120 | ok | ok | **the test level** |
| 07000006 | The Exchange (Mayhew B) `UpdateMFX_02MayhewB` | 49 | 223 | 813 | 32 | 174 | ok | ok | ³ |
| 07000007 | Alpine Escape (Mayhew C) `UpdateMFX_02MayhewC` | 48 | 291 | 513 | 58 | 256 | ok | ok | ³ |
| 07000008 | Enemies Vanish (Mayhew D) `UpdateMFX_02MayhewD` | 52 | 208 | 308 | 26 | 170 | ok | ok | ³ |
| 07000009 | Night of the Agent (Tower A) `UpdateMFX_03TowerA` | 40 | 151 | 288 | 6 | 120 | ok | ok | ³ |
| 0700000a | (Tower B) `UpdateMFX_03TowerB` | 38 | 601 | 800 | 20 | 543 | ok | ok | ³ |
| 0700000b | (Tower C) `UpdateMFX_03TowerC` | 51 | 242 | 380 | 10 | 177 | ok | ok | ³ |
| 0700000c | `UpdateMFX_PowerStationA1` | 46 | 274 | 575 | 40 | 231 | ok | ok | ³ |
| 0700000d | `UpdateMFX_PowerStationA2` | 50 | 218 | 568 | 41 | 189 | ok | ok | ³ |
| 07000011 | `UpdateMFX_HT_Level_Tower2A` | 44 | 195 | 319 | 37 | 157 | ok | ok | ³ |
| 07000012 | `UpdateMFX_HT_Level_Tower2B` | 39 | 612 | 792 | 32 | 569 | ok | ok | ³ |
| 07000013 | `UpdateMFX_HT_Level_Tower2C` | 42 | 77 | 173 | 12 | 69 | ok | ok | ³ |
| 07000014 | `UpdateMFX_HT_Level_Tower2Elevator` | 55 | 340 | 1034 | 61 | 273 | ok | ok | ³ |
| 07000015 | `UpdateMFX_HT_Level_EvilBase` | 55 | 181 | 638 | 20 | 173 | ok | ok | ³ |
| 07000016 | `UpdateMFX_HT_Level_EvilSilo` | 64 | 189 | 360 | 20 | 170 | ok | ok | ³ |
| 0700001b | `UpdateMFX_HT_Level_SpaceStationD` | 27 | 181 | 304 | 20 | 89 | ok | ok | ³ |
| 07000041 | (a story level; `0x41` code) | 100 | 94 | 120 | 1 | 81 | ok | ok | ³ |
| 07000043 | (cut-scene / wrapper) | 78 | 2 | 2 | 1 | 2 | ok | ok | ³ |
| 07000046 | (cut-scene / wrapper) | 93 | 26 | 75 | 1 | 25 | ok | ok | ³ |
| 07000048 | (cut-scene / wrapper) | 5 | 1 | 1 | 0 | 1 | ok | ok | ³ |
| 07000049 | (cut-scene / wrapper) | 71 | 2 | 2 | 1 | 2 | ok | ok | ³ |
| 0700004a | (a story level; `0x4a` code) | 42 | 72 | 294 | 38 | 66 | ok | ok | ³ |
| 0700004b | (a story level; `0x4b` code) | 110 | 113 | 225 | 1 | 108 | ok | ok | ³ |

¹ "Game logic ok" = `nf-extract ai-sim <iso> <level>` runs 5+ ticks without panic: the collision world builds, drones
spawn and step their AI state machines, and the bullet/player-damage path runs (WORKLOG 0077). Verified for all
story codes 01–1b.
² "Render" requires a Vulkan device (wgpu). This environment has no lavapipe/ICD, so headless `--shot` could not be
captured here; the render path is the same `Level::parse` + `Scene::build` the test level uses, so levels marked ³
share 07000005's renderer systems (sky, fog, PS2_GFX, textures, collision, drones, doors, searchlights, coronas,
scripts, audio, music) unless a placement type they introduce is unported (see §2).
³ Shares 07000005's systems — the placement types in use are the same set the test level exercises (see §2), so the
renderer, collision, guards and scripts load through the same code paths.

## 2. Placement types (the low byte of `Placement.flags`)

The histogram per level is produced by `level-survey`. The types the port currently **creates gameplay/render for**
(matching 07000005's exercise):

| Type | Name | Ported? | Notes |
|---|---|---|---|
| 0x0f | drone (guard) placement | **yes** | `Drone_Create` (drone.rs); all levels with guards use it |
| 0x19 | path / spline data | **yes** | `Level::path_blocks`; doors, the wine truck, rigid bodies |
| 0x20 | door | **yes** | `door.rs` (WORKLOG 0074) |
| 0x21 | portal / cel link | partial | G-016 (the cel/portal system isn't built; the port uses one cel + collision LOS) |
| 0x24 | sky slot | **yes** | nf-view sky pass (WORKLOG 0014) |
| 0x2a | sky slot (alt) | **yes** | skipped from the world pass (scene.rs:710) |
| 0xd9 | script player (cut-scene / scripted object) | **yes** | `nis.rs` / script-player (WORKLOG 0058–0062) |
| 0xdb | searchlight beam part | **yes** | `searchlight.rs` (WORKLOG 0072) |
| 0xde | searchlight spot part | **yes** | `searchlight.rs` |
| 0xe8 | level-exit trigger | **yes** (0077) | `trig.rs`; routes to `ResetMap_LevelToLoad` |
| 0xfc | corona / glow | **yes** | `corona.rs` (WORKLOG 0073) |
| 0x2c/0x2d/0x2e/0x2f/0x30/0x31/0x32/0x33 | pickups / breakables / rigid bodies | partial | G-029 (pickups), G-026 (breakables); the collision stands in |
| 0x22/0x3a/0x3e/0xf0 | ladder / zip wire / grapple / pickup | partial | G-029; **0x22 ladders: yes (fix-ladder)** — `docs/atlas/ladder.md` |
| 0xe5/0xe6 | sensor / hurt | partial | G-026 |
| 0x47 | static decoration | **yes** | drawn as PS2_GFX (no gameplay) |
| 0x19 (0x19:103 etc.) | ambient sound trigger | **yes** | `MapSounds` (sound.rs) |
| 0x71 | spawner | partial | G-025 (`DroneSpawner_Control` timing) |

Types **not yet ported** that appear in story levels: none new — every placement type present in the story codes
(01–1b) is already handled (or partially handled per the GAPS above) by 07000005's systems. The wrapper/cut-scene
levels (0x41–0x4b) use only the subset {0x19, 0x24, 0x31, 0x47, 0xd9} — all ported.

## 3. Driving levels (out of scope)

DRIVING.ELF (EA EAGL-based, stripped; `DRIVING/*.VIV|MUS|SPE` BIGF archives of SSH textures, CRP models, ASF audio,
MPC video). Listed here for completeness; **not ported** (PLAN §5 phase 4, "don't touch the driving levels"):

- `07000023`, `07000024`, `07000025`, `07000026`, `07000027`, `07000028`, `07000029` — the driving mission archives
  (110 maps each; the high map count is the EA road-network sharding).
- `0700004b` — listed in both; the survey shows 110 maps, consistent with a driving level.

## 4. Level flow (story transitions)

`Trigger_Activate` (0x1b2318): the 0xE8 end-of-level trigger calls `ResetMap_LevelToLoad(next_level, 1, 0)` — the next
level's hashcode is the trigger placement's parameter 0 [M]. `GameFlow_PushState(0x43960000, 7, 0xff)` then runs the
load screen (`ResetMap_GenLoadScreen` 0x1d5de0) and `ResetMap_Load` (0x1d6360) loads the next archive.

Port (0077): `nf-game::trig` builds the exit triggers; nf-view's level argument (`<level>` on the command line, or the
`NF_LEVEL` env var) selects the level. The end-of-level transition is wired through `Music_Event(3, 1)` (the end sting)
→ `SFXFadeDown` (0075) → the next level's hashcode, mirroring the game's `ResetMap_LevelToLoad` path.

### 07000005 — three exit routes (A-11)

The wine truck level has three ways to progress to 07000006, all of which must end the level:

| Route | Mechanism | Position | Condition |
|---|---|---|---|
| (a) Truck ride | Truck script creates TOUCH_ONCE ch 0x70 (parked pos) + TOUCH ch 0x71 (truck bed, follows truck) → gate-guard drone at (28, 37, 44) touches ch 0x72 → MuxIn ch 0x65 = AND(0x70, 0x71, 0x72) → ScriptPlayer #320 (out 0x65) plays EndLevel script 0x06000A23 → event 0x11 at tick 300 → ch 0x1D = 1 → placement 118 (LOAD_LEVEL, gate 0x1D) fires | Truck bed + gate guard at (28, 37, 44) | Bond on truck + guard at post |
| (b) On-foot gate door | Press E on placement 118 (0xE8 LOAD_LEVEL) → `Player_Activate` → `Trigger_Activate` → level loads | Castle door ~(30, 38.8, 38.2) | Bond at door, presses E |
| (c) Zip wire / around left | Press E on placement 112 (0xE8 LOAD_LEVEL, gate 0x00 = E-key only) → same `Player_Activate` path | (37.25, 50.1, -68.4) | Bond at zip-wire exit, presses E |

All three converge on `ResetMap_LevelToLoad(0x07000006, 1, 0)`. Routes (b) and (c) use `Player_Activate` (the E key),
which calls `Trigger_Activate` for class-0x1D objects [M 0x1a5170:134-137]. Route (a) uses the gate-channel path
(`Trigger_Update` case 10, set by the EndLevel script's event 0x11).

`NF_LEVELLOG=1` traces: script events, switch-channel writes (rising edge), and each LOAD_LEVEL trigger's gate check
(once per second). Lets the user see why the level doesn't end — e.g. ch 0x1D is off because the EndLevel script
never ran (ch 0x65 never fired because the gate-guard drone wasn't at post).

## 5. Level-start and intro cut-scenes (fix-cutscenes)

Every cut-scene ScriptPlayer on the 20 story levels, from `nf-extract placements <iso> <level.bin> d9` plus the
levels' type-0xC loadable records [D, measured with the survey behind `crates/nf-view/tests/fix_cutscenes_live.rs`].
"Mode" is placement param 2 (script A's mode): **1** = plays the moment the level loads (state 7 → auto-play),
**64** = preloaded at create (primed state 1; plays when its trigger fires), **0** = waits (loads+plays on its
trigger), **8** = loops. A script lives either in the level archive (kind-7 entry) or in a **loadable pack**:
the level's type-0xC entries hold (script hash, flag) pairs (`LoaderProcess` 0x1d4880 case 0xC), and
`SP_LoadScript` (0x1dbc08) → `isLoadable` (0x1d69a0) → `LoadableLoad` (0x1d69e8) streams the pack named with the
**level id's (flag+7) nibble**: flag 1 → `078000NN.bin`, flag 2 → `079000NN.bin`, flag 3 → `07A000NN.bin`
[measured: every record's pack exists in FILES.BIN and contains exactly the recorded script]. The packs also
carry the cut-scene animation sequences (kind-4).

The port (fix-cutscenes) reads the type-0xC records in `nf_view::load_with_carry` and merges every pack's
kind-7 scripts into the script lib and kind-4 sequences into the animation set — previously the pack scripts
were missing, the ScriptPlayer deleted itself on its first trigger (`Player::load`'s missing-script arm), and
the cut-scene never played.

### 5.1 Loadable-pack cut-scenes (broken before fix-cutscenes, working after)

| Level | SP (placement) | Script | Pack | Trigger → start | Plays now |
|---|---|---|---|---|---|
| 07000001 | #277 "Script Player Wedge" (mode 1) | 0x06000087 | 07800001 | level start | **yes (was deleted at load)** |
| 07000002 | #400 "Script Player" (mode 1) | 0x06000084 | 07800002 | level start | **yes (was deleted at load)** |
| 07000002 | #399 (mode 0, mask 1, out 53) | 0x0600007f | 07900002 | ch 0x35 | **yes** |
| 07000007 | #419 "NIS_Trigger" (mask 1, out 21) | 0x06000072 | 07900007 | TouchOnce #356 (box 92.9–98.0, 49.0–53.8, −79.8–−78.9) raises ch 0x15 | **yes — the party fly-through where Bond first sees Drake (3000 ticks, camera + 25 guest anims)** |
| 07000007 | #420 "Script_Trigger" (mask 1, out 72) | 0x06000073 | 07800007 | TouchOnce #412 (box 93.3–95.2, 43.2–44.1, −46.4–−45.4, lock 0x63) raises ch 0x48 | **yes — the control-taking cut-scene (3 × event 4/5/6/7 cycles)** |
| 07000008 | #294 "ScriptPlayer_Trigger_1stNIS" (out 73) | 0x06000077 | 07800008 | TouchOnce #237 (box 137.1–141.2, 48.5–50.5, −82.4–−80.2) raises ch 0x49; Bond spawns at (139.5, 50.5, −76.4), ~3.5 m away — "starts almost immediately" | **yes — the gondola intro (3880 ticks)** |
| 07000008 | #296 "ScriptPlayer_Trigger_1stNIS" (out 83) | 0x06000078 | 07900008 | ch 0x53 | **yes** |
| 0700000b | #177 (out 45) / #246 (out 51) | 0x060004fa / 0x060004f9 | 0790000b / 0780000b | ch 0x2D / ch 0x33 | **yes** |
| 0700000d | #377 (mode 1) | 0x0600068f | 0780000d | level start | **yes** |
| 07000013 | #65 "Script Player" (mode 1) | 0x060006d7 | 07900013 | level start | **yes** |
| 07000014 | #276 (out 126) / #382 (out 8) / #407 (out 9) | 0x06000833 / 0x0600062f / 0x06000848 | 07800014 / 07900014 / 07A00014 | ch 0x7E / ch 8 / ch 9 | **yes** |
| 07000016 | #262 (out 129) / #263 (out 130) | 0x06000631 / 0x06000632 | 07800016 / 07900016 | ch 0x81 / ch 0x82 | **yes** |
| 0700001b | #147 "ScriptPlayer_Trigger" (mode 1, mask 1) | 0x0600085c | 0780001b | level start | **yes** |
| 0700004a | #273 + #289 (out 131/64) / #276 (out 132) | 0x060006cf / 0x060006d3 | 0780004a / 0790004a | ch 0x83 / ch 0x40 / ch 0x84 | **yes** |

Records whose script no SP references directly (loaded into the hashtable for sub-scripts/other users, all now
in the lib): 07000009's 0x060004f8 → 07800009, 0700000b's 0x060004fb → 07A0000b, 0700000d's 0x0600073a →
0790000d, 07000011's 0x06000655 → 07800011, 07000013's 0x06000646 → 07800013, 07000016's 0x06000633 → 07A00016,
0700001b's 0x0600085f → 0790001b.

One dead reference: 07000005 #71 waits on ch 0x7D for 0x06000074, but 07000005 has **no** loadable record for it
and no `07800005/07900005.bin` exists on the disc — the real game cannot stream it either (`isLoadable` returns
0, `Script_Load` misses the hashtable), so it never plays there either.

### 5.2 Level-start cut-scenes with scripts in the level archive (played before and after the fix)

| Level | SP | Script | Note |
|---|---|---|---|
| 07000004 | #3 | 0x06000083 | mode 1 |
| 07000005 | #37 "Scriptplayer_SC10" | 0x06000A22 | the wine truck (5895 ticks, keep-objects) |
| 0700000a | #779 "Q-WORM SCRIPTPLAYER" (mode 1, out 72) | 0x060009f0 | with #780 looping partner |
| 0700000b | #347 / #355 "Script Player Wedge" | 0x060009ee / 0x060009fb | mode 1/1 and 1/0 |
| 07000016 | #272 "ScriptPlayer_ShuttleLaunch" (out 185) / #347 "ScriptPlayer_ShuttleBayLift" (out 80) | 0x0600090a / 0x060009e9 | |
| 0700001b | #41–56 "ScriptPlayer_SolarPanel" ×16 | 0x06000832 | ambient |
| 0700001b | #212 "ScriptPlayerShuttleFlyBy1"; #200 "ScripPlayer_Missile4" (gate 191); #148/#201 "Speech" (gate 191/19); #213–220 "MissileDissarm" ×8 (gates 31–38) | … | gated ones wait for their gate channel |
| 0700004a | #42 "SCRIPTPLAYER_Lift" (out 34) | 0x06000571 | |

### 5.3 Mode-64 preload ScriptPlayers (primed at level start, play on a trigger)

Cut-scene-worthy ones (the rest are object animations — vents, doors, indicators, hatches):

| Level | SP | Script | Waits on |
|---|---|---|---|
| 07000007 | #312 "OBJECT_UrnBreak" (mask 2) | 0x0600007A (archive) | a bullet hit — `SP_Hit` bit 2, not modelled in the port yet (the priming itself is: state 1 at the first tick, tested) |
| 07000008 | #295 "ScriptPlayer_CableCar" (out 1) | 0x0600004C (archive) | mux #235: AND(ch 0x4A, ch 4) — the cable-car boarding |
| 07000005 | #320 "ScriptPlayer_EndLevel" (out 101) | 0x06000A23 (archive) | ch 0x65 (the level-exit chain, already works) |
| 07000006 | #809 "Script-IceTrigger" (out 40) | 0x06000A28 (archive) | ch 0x28 |
| 0700000d | #492 "ScriptPlayer_CraneRotate" (out 10) | 0x06000A08 (archive) | ch 0xA |

### 5.4 The level intros (core-loop item 8: movie + subtitle scripts)

`ResetMap_Load` (0x1d6360) phase 0 loads `07000500.bin` (the global pack — it holds 14 tiny kind-7 scripts,
64–320 bytes each) and, **only when the subtitles option is on** (`DAT_002a37c8`, the flag `Sound_DoSubtitle`
0x1dde58 and `Text_AddMsg` 0x1b0c30 read), creates one ScriptPlayer per level from the movie-code table
[0x1d6360:40-99]: 0x7100001→0x0600095A, 0x7100005→0x06000961, 0x7100009→0x06000963, 0x710000c→0x06000965,
0x7100011→0x06000967, 0x7100014→0x06000969, 0x7100018→0x0600096B (level 0700001b; movie codes 0x710000xx map
1:1 from 0x070000xx per `ResetMap_LevelToLoad` 0x1d3800, except 0x0700001b→0x7100018 and 0x0700004a→0x710004a,
which is not in the table → no intro), plus the driving codes 0x7300004/8/b/d/13/17/1b → 0x0600095D/62/64/66/68/7A/6C.
Measured content [D]: every one of the 14 scripts is a single stream of op-0x1E `Script_TextStart` commands —
subtitle timings for the intro MOVIE (the `GameFlow_PushState(0,6)` background video), not an in-engine
cut-scene. The port plays neither movies nor subtitles (no PSS player), so there is nothing to port here beyond
what exists: the port's `load` skips the intro SP creation and the playtest symptom "no intro" is the movie
gap (GAPS, `ResetMap_Load` row), not a ScriptPlayer bug.

## 6. Open items

- **G-016** (cel/portal system): until ported, every level uses one cel + collision LOS instead of the room/cel
  visibility + reverb zones. This affects rendering (no per-room culling) and audio (`SFXSetEnvironment` defaults to
  no reverb, 0075). Independent of which level is loaded.
- **NF_ISO live tests**: `crates/nf-game/tests/level_live.rs` (0077) loads each story level and checks its drone /
  object / placement counts against the disc values in §1. Opt-in via `NF_ISO`.
- **Rendering verification**: headless `--shot` needs a Vulkan ICD (lavapipe); this environment lacks one. The render
  path is the same `Scene::build` 07000005 uses, so levels load through it; visual diffing against PCSX2 is pending a
  GPU environment (U-0xx to add).
- **`SP_Hit` trigger bits** (nis.rs): mask bits 2 (bullet) / 8 / 0x10 (bullet and neighbour proxies on the SP's
  stream objects) are not modelled — 07000007's urn ScriptPlayer #312 (mask 2, script 0x0600007A, primed) is the
  one cut-scene that still waits on them.

## Ladders and water, per level (fix-ladder)

From `crates/nf-view/tests/fix_ladder_live.rs::ladder_and_water_counts_per_level` [D disc]: ladder
placements (type 0x22, `Ladder_Create` 0x1966c0) and water cels (flag word 0xC023, whose bbox top is the
swimmable surface — `docs/atlas/ladder.md`).

| Level | Ladders | Water cels | Notes |
|---|---|---|---|
| 07000001 | 0 | 0 | |
| **07000002** | **4** | **5** | The bunker exit ≈(−26.9, 2.3, 42.2) yaw −π (world-sphere top 5.96); water surfaces 3.76–4.34 (the bunker channels and the garden moat) |
| 07000003 | 1 | 0 | (34.2, 7.6, 18.2) |
| 07000004 | 0 | 0 | |
| 07000005 | 1 | 0 | (60.9, 29.9, 89.2) — the test level's ladder |
| 07000006 | 0 | 0 | |
| 07000007 | 0 | 0 | |
| 07000008 | 4 | 0 | two degenerate placements at the origin |
| 07000009 | 0 | 0 | |
| 0700000a | 1 | 0 | (−13.2, −1.3, −9.5) |
| 0700000b | 0 | 0 | |
| 0700000c | 3 | 0 | |
| 0700000d | 3 | 0 | |
| 07000011–14 | 0 | 0 | |
| 07000015 | 3 | 0 | |
| 07000016 | 1 | 1 | (−0.4, −6.2, −86.2) + one water cel |
| 0700001b | 0 | 0 | |


## Grenade-carrying guards (WORKLOG B-1; corrected by fix-drone)
`NDrone2_DefaultInit` 0x14b300 takes the grenade word from **placement param 11**
(`piVar19[0x18]`, params start at `piVar19[13]` — drone-port-check.md #1). The original B-1 scan
read **param 24** (the alert block's middle word), so its table documented the wrong word; the
corrected scan (`NF_ISO=<iso> cargo test -p nf-game --test drone_portcheck_live grenade_fields`,
plus the full `placements … f` sweep behind it) finds 75 carriers on 9 levels:

Having the grenade word does not mean the guard ever throws (fix-drone2, WORKLOG): 07000001's
two carriers are structurally blocked by the game itself — the 0x13 carrier at (−30.1, 10.3, 13.4)
is dtypenew 2 (sniper; `GetDTYPENEW` 0x14c938 case 2) and the sniper states never call
`DroneWeap_CanThrowGrenade` (its only caller is `DroneFunc_CombatState` 0x148338, states 0x68/0x70),
and the 0x11 carrier at (7.0, 6.4, 2.2) waits on switch channel 5, which nothing on the disc ever
raises (no trigger, no NIS event, no pickup, no death channel).

| Level | Guards with grenades | Param 11 (raw) | Weapon | Count |
|---|---|---|---|---|
| 07000001 | 2 | 0x13 | 0x36 (stun) | 3 |
| 07000003 | 1 | 0x13 | 0x36 (stun) | 3 |
| 07000004 | 1 | 0x26 | 0x35 (frag) | 6 |
| 07000011 | 6 | 0x21/0x36/0x41 | 0x35 / 0x34 | 1/6/1 |
| 07000012 | 11 | 0x11/0x21/0x43/0x51… | 0x36 / 0x35 / 0x34 / 0x3B | … |
| 07000013 | 2 | 0x21/0x23 | 0x35 | 1/3 |
| 07000014 | 24 | 0x11/0x22/0x23… | clamped to 0x34 (0x14b300 lines 374-376) | … |
| 07000015 | 20 | 0x11/0x23/0x43… | 0x36 / 0x34 | … |
| 07000016 | 8 | 0x11/0x43… | 0x36 / 0x34 | … |

07000002, 07000005, 07000009 and 0700000a carry **none** — the B-1 table's "carriers" there were
param-24 artifacts. On 0x7000014 a decoded weapon in (0x34, 0x37) clamps to 0x34.

**07000005 (the test level) has no grenade-carrying guards.** The grenade scenarios and tests must run on a level where
guards actually have grenades, or set the grenade fields the way `NDrone2_DefaultInit` decodes param 11 (grenade_live
uses raw 0x13 → 0x36 × 3). The port's `Drone::new` reads **param 11** to set `+0xBCC`/`+0xBCE` (fix-drone; it
previously read param 24, which decodes to no grenades on every level).

## 7. Objectives per level (fix-mission)

`MissionData` (ACTION.ELF 0x2a4350, docs/formats/mission-data.md) gives every story level its
objective list; the port embeds it in `crates/nf-game/src/mission/`. Texts resolved from
USATxt.dat [L ee_start_05.bin; the port's `Lang::bind`]. "ch" = the condition channel that
completes the objective (FAIL = the channel failing it instead), "en" = the announce-gate channel
(0 = announce at level start). The mission's objectives AND together into success channel 100, so
it fires at the mission's last level only [M 0x1827d8].

| Level | Objective (text) | ch | en | |
|---|---|---|---|---|
| 0x07000001 | "Escort Mayhew to bunker" | 0x47 | 0x00 |  |
| 0x07000002 | "Destroy Mayhew's computer" | 0x48 | 0x47 |  |
|  | "Rescue geisha girls" | 0x4b | 0x47 |  |
|  | "Find door to servants quarters" | 0x4d | 0x4c |  |
| 0x07000003 | "Retrieve Dragon Safe contents" | 0x4e | 0x4d |  |
|  | "Rescue Mayhew's servants" | 0x51 | 0x4d |  |
|  | "Find door leading to Mayhew" | 0x53 | 0x52 |  |
| 0x07000004 | "Defeat the assassin" | 0x54 | 0x53 |  |
| 0x07000005 | "Breach the castle walls" | 0x46 | 0x45 |  |
| 0x07000006 | "Find a way inside to the party" | 0x02 | 0x46 |  |
| 0x07000007 | "Rendezvous with undercover agents and maintain cover as party guest" | 0x48 | 0x47 |  |
|  | "Spy on secret meeting" | 0x49 | 0x48 |  |
| 0x07000008 | "Retrieve guidance chip from "Safe Room"" | 0x4a | 0x49 |  |
|  | "Escape with Zoe in gondola" | 0x4b | 0x4a |  |
| 0x07000009 | "Don't let the security guards raise the alarm" | 0x61 | 0x00 | FAIL |
|  | "Get to the lobby and activate the main elevator system" | 0x46 | 0x00 |  |
|  | "Take main elevator to office floor" | 0x47 | 0x46 |  |
| 0x0700000a | "Install Q-Worm on office computer systems" | 0x48 | 0x47 |  |
|  | "Find the security center and unlock exterior door" | 0x49 | 0x47 |  |
|  | "Find door to outer balcony" | 0x4b | 0x4a |  |
| 0x0700000b | "Install Q-Worm on Mayhew's computer system" | 0x4c | 0x4b |  |
|  | "Locate the Secure Terminal Room" | 0x4d | 0x4b |  |
|  | "Retrieve the NightFire files from the Secure Terminal" | 0x4e | 0x4d |  |
|  | "Escape to the roof" | 0x59 | 0x4f |  |
|  | "Acquire parachute from helicopter then jump off tower roof" | 0x5a | 0x59 |  |
| 0x0700000c | "Investigate building ST- 1 " | 0x1b | 0x00 |  |
|  | "Photograph the Jetpack Prototype " | 0x1c | 0x1b |  |
|  | "Advance through Warehouse TR - 2" | 0x23 | 0x1c |  |
| 0x0700000d | "Investigate building ST- 3 " | 0x1d | 0x23 |  |
|  | "Photograph the Laser Prototype" | 0x1e | 0x1d |  |
|  | "Destroy door C5 with mounted laser on catwalk" | 0x1f | 0x1d |  |
| 0x0700000e | "Disable all security measures before using the lift" | 0x0f | 0x00 |  |
|  | "Disable system 1 of 3" | 0x0a | 0x07 |  |
|  | "Disable system 2 of 3" | 0x0b | 0x08 |  |
|  | "Disable system 3 of 3" | 0x0c | 0x09 |  |
| 0x0700000f | "Mission Objective 5" | 0x00 | 0x00 |  |
| 0x07000010 | "Mission Objective 1" | 0x00 | 0x00 |  |
|  | "Mission Objective 2" | 0x00 | 0x00 |  |
|  | "Mission Objective 3" | 0x00 | 0x00 |  |
| 0x07000011 | "Escape from the tower" | 0x64 | 0x00 |  |
|  | "Gain access to lower office floor" | 0x48 | 0x00 |  |
| 0x07000012 | "Activate elevator override control in security center" | 0x49 | 0x48 |  |
|  | "Escape through the elevator shaft" | 0x4d | 0x49 |  |
| 0x07000013 | "Raise security gate and escape through front entrance" | 0x50 | 0x4a |  |
| 0x07000014 | "Infiltrate Drake's facility undetected" | 0x47 | 0x00 |  |
|  | "Follow Kiko to the Security Control Center" | 0x48 | 0x47 |  |
|  | "Sabotage base security and escape from room" | 0x4a | 0x48 |  |
|  | "Prevent Rook from cutting the power to Delta Sector" | 0x54 | 0x4a |  |
|  | "Take the exit to Delta Sector" | 0xce | 0x54 |  |
| 0x07000015 | "Find the exit to Omega Sector" | 0x8e | 0x4b |  |
| 0x07000016 | "Reach Drake's launch silo" | 0x8f | 0x8e |  |
|  | "Prevent Kiko from launching into space" | 0x90 | 0x8f |  |
| 0x0700001b | "Disable the missiles before they are launched" | 0x09 | 0x00 |  |
|  | "Defeat Drake before the base goes critical" | 0x0b | 0x09 |  |
| 0x0700004a | "Gain access to lobby" | 0x4a | 0x4d |  |

