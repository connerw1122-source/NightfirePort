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
| 0x22/0x3a/0x3e/0xf0 | ladder / zip wire / grapple / pickup | partial | G-029 |
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

## 5. Open items

- **G-016** (cel/portal system): until ported, every level uses one cel + collision LOS instead of the room/cel
  visibility + reverb zones. This affects rendering (no per-room culling) and audio (`SFXSetEnvironment` defaults to
  no reverb, 0075). Independent of which level is loaded.
- **NF_ISO live tests**: `crates/nf-game/tests/level_live.rs` (0077) loads each story level and checks its drone /
  object / placement counts against the disc values in §1. Opt-in via `NF_ISO`.
- **Rendering verification**: headless `--shot` needs a Vulkan ICD (lavapipe); this environment lacks one. The render
  path is the same `Scene::build` 07000005 uses, so levels load through it; visual diffing against PCSX2 is pending a
  GPU environment (U-0xx to add).

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
