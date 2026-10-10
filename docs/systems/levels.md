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

`Trigger_Activate` (0x1b2318): the 0xE8 end-of-level trigger calls `ResetMap_LevelToLoad(param_0x20, 1, 0)` — the next
level's hashcode is the trigger placement's parameter 0x20 [M]. `GameFlow_PushState(0x43960000, 7, 0xff)` then runs the
load screen (`ResetMap_GenLoadScreen` 0x1d5de0) and `ResetMap_Load` (0x1d6360) loads the next archive.

Port (0077): `nf-game::trig` builds the exit triggers; nf-view's level argument (`<level>` on the command line, or the
`NF_LEVEL` env var) selects the level. The end-of-level transition is wired through `Music_Event(3, 1)` (the end sting)
→ `SFXFadeDown` (0075) → the next level's hashcode, mirroring the game's `ResetMap_LevelToLoad` path.

## 5. Open items

- **G-016** (cel/portal system): until ported, every level uses one cel + collision LOS instead of the room/cel
  visibility + reverb zones. This affects rendering (no per-room culling) and audio (`SFXSetEnvironment` defaults to
  no reverb, 0075). Independent of which level is loaded.
- **NF_ISO live tests**: `crates/nf-game/tests/level_live.rs` (0077) loads each story level and checks its drone /
  object / placement counts against the disc values in §1. Opt-in via `NF_ISO`.
- **Rendering verification**: headless `--shot` needs a Vulkan ICD (lavapipe); this environment lacks one. The render
  path is the same `Scene::build` 07000005 uses, so levels load through it; visual diffing against PCSX2 is pending a
  GPU environment (U-0xx to add).

## Grenade-carrying guards (WORKLOG B-1)

`NF_ISO=<iso> cargo test -p nf-game --test grenade_scan` — scans every level's drone placements for param 24 (the
`NDrone2_DefaultInit` grenade field `piVar19[0x18]`). The high 16 bits (after `>> 4`) select the grenade type (1=stun
0x36, 2=frag 0x35, 4=0x34, 5=custom, 6=0x6C); the low nibble is the count.

| Level | Guards with grenades | Weapon | Count |
|---|---|---|---|
| 07000002 | 2 (placements 464, 466) | 0x36 (stun) | 3 each |
| 07000003 | 2 (placements 388, 389) | 0x36 (stun) | 1 each |
| 07000009 | 6 (placements 62, 64, 65, 66, 250, 261) | 0x36 (stun) | 3 each |
| 0700000a | 24+ (many placements) | 0x36 (stun) | 3 each |

**07000005 (the test level) has no grenade-carrying guards.** The grenade scenarios and tests must run on 07000002 or
another level where guards actually have grenades. The port's `Drone::new` now reads param 24 to set `+0xBCC`/`+0xBCE`
(previously hardcoded to 0x35/0).
