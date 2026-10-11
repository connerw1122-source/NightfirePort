# Atlas: the Drone (`NDrone2`)

Plan v2 §2 struct 2. Build: ACTION.ELF `e7ffdcb3…`. Sources: decomp + disassembly [M];
snapshot `work/ee_start_05.bin` [L]. Port: `crates/nf-game/src/drone.rs`. Companion file:
`drone-port-check.md` (every `+0xNNN` the port mentions, audited).

## Name, size, allocation site

- The drone is **not** a standalone allocation: it is the 0xD20-byte **instance** that
  `control_create_object(0xD20, …)` appends to the 0x110-byte object header [M
  `NDrone2_CreateObj` 0x14d7c8; object header: `docs/atlas/object.md`]. Total block 0xE30
  bytes from `Mem_Malloc(size+0x110, 0x404, 0)` [M `control_create_object` 0x1344e0].
  In these docs **`Drone+X` means instance+X = obj+0x110+X** (the port's convention).
- **Allocation chain**: placement type 0xF → `Drone_Create` 0x136be0 (gated:
  `Drone_bDisableSystem` 0x30ca9e clear and `placement+0x34 <= GameState[0x40]`) →
  `NDrone2_CreateObj` 0x14d7c8 → `control_create_object` [M]. Script path: `Control_CreateObjEx`
  0x1343c8 with the same 0xD20 size; initial class 0x4D instead of 2 [M disasm].
- Class byte: **2** (placement) / 0x4D (script-created, before any re-class) [M 0x14d7c8].
- `NDrone2_CreateObj` also sets `Drone+0xC = obj` and appends the instance to the drone LList
  at **0x298e14** [M] (`LList_Add` 0x1cca28 appends at the tail: list info {first, last, u16
  count, u16 size}; node links: prev +0x00, next +0x04 inside the instance) — see "present"
  below.
- Post-create by `Drone_Create` 0x136be0 [M, all disasm-verified]: `obj+0xEC = Rand_Rand(10000)`;
  placement translation → `Drone+0xC10` (vec4), rotation → `Drone+0xC20`; the 33 placement
  param words (`placement+0x2C`, 0x84 bytes) → **`Drone+0xC34`**; `Drone+0xC00 = obj`;
  `Drone+0x144 = DIVars+0x44`, `Drone+0x148 = DIVars+0x50` (two words the caller provides —
  for the placement path both are parts of the copied param block: see the param map).
- `NDrone2_PostLoad_Init(obj)` 0x14d890 calls **`NDrone2_DefaultInit(obj+0xC00)`** [M] —
  i.e. the creation parameter block ("DIVars") lives **inside the instance at +0xC00**.

## The creation parameter block (`DIVars`) = `Drone+0xC00..`

| Off (DIVars) | Off (Drone) | Type | Meaning | Tag |
|---|---|---|---|---|
| +0x00 | +0xC00 | ptr | the object (back-reference) | [M] |
| +0x10 | +0xC10 | vec3 | placement translation | [M] |
| +0x20 | +0xC20 | vec3 | placement rotation | [M] |
| +0x30 | +0xC30 | ptr | glist (0 on the placement path; set on the script path — selects class 0x4D) | [M] |
| +0x34 | +0xC34 | u32×33 | **params[0..32]** — the placement's `+0x2C+4k` words | [M] |

**Param k = `Drone+0xC34+4k` = `DIVars+0x34+4k` = `piVar19[13+k]`** in the decompile. Any
`piVar19[k]` with k ≥ 13 is param k−13. This offset is the port-check's headline finding
(grenades are param **11**, the port reads param 24 — see drone-port-check.md).

## Field table (selected; writers are `NDrone2_DefaultInit` 0x14b300 unless noted)

| Off | Type | Name | Meaning | Writers | Readers | Tag |
|---|---|---|---|---|---|---|
| +0x00/+0x04 | ptr | LList prev/next | drone-LList links (0x298e14) | `LList_Add`/`LList_Remove` | list walkers | [M] |
| +0x0C | ptr | obj | back-reference | `NDrone2_CreateObj` 0x14d7c8 | everywhere | [M] |
| +0x10 | ptr | BodyGlow object | `BodyGlow_Create(obj)` result | DefaultInit (tail) | renderer | [M] |
| +0x14 | u8 | damageable | 1 default; 0 for models 0x0500003B/3C and 0x05000053 | DefaultInit | hit paths | [M] |
| +0x15 | u8 | female voice | 1 for female voice sets (default pain/death lines) | DefaultInit | speech | [M] |
| +0x17 | u8 | | bunched flag: another drone within 3 units (port's reading; writer in `Drone_InitComms`/`Check_Target` area) [S] | | combat moves | [S] |
| +0x18 | u8 | | cleared at init; 1 when voice set 2 model override | DefaultInit | | [M] |
| +0x19 | u8 | captain | set by difficulty via properties 9 (diff 1) / 5 (diff 2) / 8 (diff 3) | DefaultInit | stat multipliers | [M] |
| +0x1A | u8 | free root motion | special-death free-root flag | DefaultInit path | special death | [S] |
| +0x1B | u8 | silent kill | 1 for models 0x05000053 and voice set 7 (0x7000016) | DefaultInit | death | [M] |
| +0x1F | u8 | | 1 default (SP), 0 for dtypenew 0xD/0x1B/0x1C/0x1D | DefaultInit | | [M] |
| +0x23 | u8 | special-move flag | 1 for models {0x5000078, 0x5000094, 0x50000BA}; also raises route flag 0x80 and speeds 0x8F0/0xA00 to 4.0 | DefaultInit | routes | [M] |
| +0x24 | u8 | | 1 at init end | DefaultInit | | [M] |
| +0x27 | u8 | | 0 at init | DefaultInit | | [M] |
| +0x2A | u32 | aimed-at until | Bond's crosshair on this drone, until this frame (`Check_Target` every 6 ticks, held 30) [S — matches `DroneWeap_OpponentIsAimingAtMe` 0x17a500 touching +0x2A] | | `NDrone2_ChooseCombatMove` | [S] |
| +0x32 | u8 | | nonzero → init calls `NDrone2_DoTypeSettingsOLD` and zeroes `obj+0xF4` | DefaultInit | | [M] |
| +0x33 | u8 | investigating | (port claim; hearing state — writers in `DroneFunc_HandleSoundAlerts` 0x148a30) [S] | | hearing | [S] |
| +0x34 | u8 | turned | (port claim, hearing "turned") [S] | | | [S] |
| +0x38 | u8 | | 0 at init (with +0x14=1, +0x18=0, +0x1B=0, +0x27=0 — the init zero group) | DefaultInit | | [M] |
| +0x3A | u8 | head shot | last hit was a head shot (port claim; readers in hit paths) [S] | | flinch | [S] |
| +0x3B | u8 | firing | firing flag (gun drops while clear — port claim, matches firing-state readers) [S] | | firing | [S] |
| +0x3C | u8 | burst done | (port claim) [S] | | | [S] |
| +0x3E | u8 | | from the AnimObject's +0x62 byte: <28 → 0, 28..29 → 1, ≥30 → 0 | DefaultInit | | [M] |
| +0x41 | u8 | sight lost | port claim ("sight lost since the last shot") [S] | | firing | [S] |
| +0x43 | u8 | noise level | loudest noise level, never reset (port claim) [S] | | hearing | [S] |
| +0x44 | u8 | team | 1 = enemies; types 10/11/13/14 → 3, 12 → 2 (DoModeSettingsNEW per docs/systems/drone-ai.md); 1 also triggers `PlrStat_LogEnemySpawned` | DefaultInit (via DoModeSettings), DefaultInit tail | spawn stat | [M] |
| +0x45 | char | | params[10] byte | DefaultInit | | [M] |
| +0xA0 | f32 | hip height | 1.0 default; per-model overrides visible live ([L] 1.14806 / 1.15059 on two 07000005 drones — set by the per-model path, exact fn not chased) | DefaultInit 1.0 | position (feet→hip) | [M,L] |
| +0xA4 | f32 | | 1.0 at init | DefaultInit | | [M] |
| +0xAC | f32 | health | 10.0 default; per-type/per-model overrides (voice set 0x10 ×3 on 0x7000014; dtypenew 0xD → 100.0; `DroneInit_TakeXHits` for 0x18; `DroneCaptain_Mod_Health` for captains) | DefaultInit | damage | [M,L] |
| +0xB0 | f32 | max health | copied from +0xAC at the end of init | DefaultInit | damage | [M] |
| +0xB4/+0xB5 | u8 | accuracy / aggression | stats; captain accuracy × 1/`DroneCaptain_Mod_BulletAccuracy` (DefaultInit); 07000005 drones [L]: acc 5, agr 0 | DefaultInit, `drone_stats` | firing/AI | [M,L] |
| +0xB8 | u8 | reaction | stat byte (reaction time math, docs/systems/drone-ai.md); [L] 0 | stat table | AI | [M,L] |
| +0xBB | u8 | | 0 at init | DefaultInit | | [M] |
| +0xC0 | f32 | | 1.0 at init (SP) | DefaultInit | | [M] |
| +0xC4 | u8 | alternate dtypenew | 0x1D for model 0x50000BA (0x18 case) | DefaultInit | modes | [M] |
| +0xC5 | u8 | dtypenew | mode/type after `GetDTYPENEW` (sniper = 2; [L] drone 0 = 2 ✓) | DefaultInit/`DoModeSettingsNEW` | mode switches | [M,L] |
| +0xC6 | u8 | alternate mode | 0 unless set (0x18 case) | DefaultInit | modes | [M] |
| +0xC7 | u8 | DTYPE | placement param 15 (behaviour_util_get per docs/systems/drone-ai.md); init reads +0xC7 == 0xC to patch properties | DoModeSettingsNEW | property patches | [M] |
| +0xC8/+0xC9 | u8 | normal / alert block mode | params 16 / 21 | `DoModeSettingsNEW` | AI | [M,L] |
| +0xD0 | u32 | | 0xF at init (per-model overrides: 0x3C for voice set 0x13 on 0x7000014) | DefaultInit | | [M] |
| +0xD4 | u32 | | 0 at init (obj+0xD4 points at +0x3D0 before this) | DefaultInit | | [M] |
| +0xD8 | u16 | voice set | from the model id switch (models 0x0500002E/2F/30 → 2 [L voice=2 on all 13]; 0x5000035 → 0xF; …) | DefaultInit | speech/property patches | [M,L] |
| +0xDA | u16 | voice variant | from the same switch (12 default; [L] 5 on drones 1-12, 0x13 on the two patrol drones — later patched by `DoModeSettings*`, writer not chased) | DefaultInit, DoModeSettings | speech | [M,L] |
| +0xE0 | u32 | model id | params[0], the 0x0500xxxx skin; captain overrides rewrite it (voice-set switch: 1→0x5000038, 2→0x500002F, 3→0x5000006, 6→0x5000093, 7→0x50000B2) | DefaultInit | anims, hit reactions | [M,L] |
| +0xE4 | f32 | half fov | π/2 at init (0x3FC90FDB); 0 for sight class 5 | DefaultInit | vision | [M] |
| +0xE8 | f32 | sight range | by sight class (params[14]): default 24, 1→12, 2→16, 3→40, 4→1000, 5→0 | DefaultInit (Rand_FRand draws per docs/systems/drone-ai.md) | vision | [M] |
| +0xEC | f32 | | 4.0 at init; 2.0 for dtypenew 0xD | DefaultInit | anim/move | [M] |
| +0xF0 | f32 | engage range | f0: class default 12+FRand(4), 1→12+FRand(3), 2→12+FRand(3), 3→25+FRand(6), 4→75+FRand(8), 5→12+FRand(4); 07000005 special (params[9] == 0x16): 10.0 | DefaultInit | AI | [M] |
| +0xF4 | f32 | | +0xEC + Rand_FRand(0) | DefaultInit | | [M] |
| +0xF8 | f32 | | 2.0 at init | DefaultInit | | [M] |
| +0xFC | f32 | combat sight range | fc: default f0+1+FRand(4), 1→f0+1+FRand(3), 2→same, 3→100+FRand(6), 4→1000, 5→f0+1+FRand(4) | DefaultInit | AI | [M] |
| +0x100 | f32 | damage multiplier | 1.0 default; 2.0 for voice sets {2,6,10,0xB,0xE,0x10,0x3B..0x40}; `DroneCaptain_Mod_BulletDamage` for captains | DefaultInit | `NDrone2_HitDamage` 0x145558 | [M] |
| +0x108 | u32 | state-machine id | creation counter [L: 1..13, newest = 1] | state machine init | msgs | [M,L] |
| +0x10C | u32 | current state | state id; mirrored to `obj+0xF4` [L: 0x29↔0x29, 4↔4] | `Drone_SM_SetState` | dispatch | [M,L] |
| +0x110/+0x114 | u32 | prev/next state | state-machine info (+8/+0xC per docs/systems/drone-ai.md) | SM | SM | [M] |
| +0x118 | u32 | state after flinch | port claim (docs/systems/drone-damage.md) [S] | | damage | [S] |
| +0x134 | u8 | switch channel | params[3] (0 = none); +0x136 = `switch_channels[ch]` snapshot at init | DefaultInit | WaitSwitch | [M,L] |
| +0x135 | u8 | second switch | params[6]; +0x137 = its channel snapshot | DefaultInit | second tag | [M,L] |
| +0x136/+0x137 | u8 | channel state snapshots | init-time `switch_channels[]` reads | DefaultInit | Disabled state | [M] |
| +0x138 | i16 | | params[5] (behaviour mode also drives OLD/NEW branch: <0x24 OLD else NEW) | DefaultInit | | [M] |
| +0x13A | u16 | second tag | params[8] (dropped when block modes equal — `NDrone2_GetDroneTypeAttackTypeFriend` per docs) | DefaultInit | second switch | [M,L] |
| +0x13C | u8 | death channel | params[12] (0x63 = mission-fail reasons, not modelled) | DefaultInit | death | [M,L] |
| +0x144 | u32 | spawner group | params[4] (0 = none; [L] drone 0 has 1, others 0) | DefaultInit | spawners | [M,L] |
| +0x148 | u32 | | params[7] | DefaultInit | | [M] |
| +0x14C | f32 | damage-taken multiplier | 1.0 at init | DefaultInit | `NDrone2_HitDamage` 0x145558 | [M] |
| +0x150 | f32 | last damage | port claim (docs/systems/drone-damage.md) [S] | | | [S] |
| +0x170..0x2AF | — | zeroed block | memset 0, 0x140 bytes | DefaultInit | | [M] |
| +0x228 | u32 | vision bits | bit 2 line-of-sight, 4 seen this tick, 8 ever seen (docs/systems/drone-ai.md; writers `DroneVision_*` 0x176a28/0x176f18) [S — bit semantics documented from the same decomp, not re-read here] | | AI | [S] |
| +0x22C | f32 | light factor | product of AI-volume kind-0 factors (`Drone_ProcessOpponents` 0x1390c0) [S] | | vision | [S] |
| +0x230 | f32 | visibility | `DroneVision_OpponentVisibility` output | vision | AI | [S] |
| +0x270/+0x274 | u32 | seen / unseen ticks | round-robin sight counters [S] | | reaction | [S] |
| +0x330 | vec3 | explosive pos | msg 0x21 `DroneFunc_ConsiderExplosive` (port claim) [S] | | flee | [S] |
| +0x350 | f32 | explosive run dist | 5.0 stun, 8.0 mine, 12.0 frag (port claim) [S] | | flee | [S] |
| +0x3D0 | — | scratch | obj+0xD4 points here at init | DefaultInit | | [M] |
| +0x464 | u16 | | 0x800 at init | DefaultInit | | [M] |
| +0x480/+0x490 | vec3 | | zeroed at init | DefaultInit | | [M] |
| +0x4A0 | f32 | | 0.1 / FRAME_RATE_DIV | DefaultInit | | [M] |
| +0x4CE | u16 | | 0 at init | DefaultInit | | [M] |
| +0x4D0 | u32 | speech line | 0 at init; current line id while playing (docs/systems/speech.md) [S] | DefaultInit | speech | [M,S] |
| +0x4D8 | u32×3 | normal behaviour block | params[16..18] via `DoModeSettings*` (behaviour words; property get/set fns 0x… read here) | DoModeSettingsNEW/OLD | behaviour | [M] |
| +0x4E8 | u32×3 | alert behaviour block | params[21..23] via `DoModeSettingsNEW` | DoModeSettingsNEW | behaviour | [M] |
| +0x4F4 | u32 | | 0 at init | DefaultInit | | [M] |
| +0x4F8 | u32 | flags word | see the bit table below | many | many | [M,L] |
| +0x4FC | u32 | alert bits | bit 0x10 = attacking (visible to other drones); observers see the source's bits << 16 (docs/systems/drone-alerts.md) [S]; init: 0x10 when the start state is Attack | DefaultInit, `DroneFunc_SetAsAttacking` | `Drone_InitComms` | [M,L,S] |
| +0x510/+0x514 | f32 | loudness / share | hearing per docs/systems/drone-alerts.md [S] | | | [S] |
| +0x524 | u32 | attack frame | (port claim) [S] | | | [S] |
| +0x530 | ptr | AnimObject | anim handle; `NDrone2_Enable` writes `+0x98` on it (0 disabled / 1.0 enabled) | anim creation | Enable | [M] |
| +0x540 | u32 | last flinch | port claim [S] | | | [S] |
| +0x54C | f32 | turn remainder | port claim (`DroneAnim_OnEndAnim` 0x13e2c0) [S] | | turning | [S] |
| +0x554 | u32 | initial script | params[1] (0x6000xxxx script ids); 0x6000892/0x600089C are consumed into dtypenew 0x1C/0x1B instead | DefaultInit | PlayScript | [M] |
| +0x56C | u16 | | 0xFFFF at init | DefaultInit | | [M] |
| +0x568/+0x56A | u8 | danim class / state | anim-call bookkeeping (docs/systems/drone-animation.md) [S] | | anims | [S] |
| +0x578..0x57B | u8 | anim-call cluster | blend wait / event-7 clear / one-shot / request-started flags (docs/systems/drone-animation.md) [S] | | anims | [S] |
| +0x59C | u32 | stand-idle hold | (port claim) [S] | | idle | [S] |
| +0x5A0 | u32 | hearing return state | (port claim) [S] | | | [S] |
| +0x5A2 | u16 | start state | the state entered once enabled [L: 0x29 sniper / 0x56 attack / 6 idle / 0x85]; 0xBD for model 0x50000BA | DefaultInit, DoModeSettings | enable path | [M,L] |
| +0x5A4 | u16 | alternate state | [L: 0x85 on the two channel-paired drones, 0 on the sniper] | DefaultInit | FirstAttack | [M,L] |
| +0x5B0 | u32 | hearing home | (port claim) [S] | | | [S] |
| +0x710 | vec3 | goal point | (port claim) [S] | | cover | [S] |
| +0x830 | vec3 | cover point | (port claim) [S] | | cover | [S] |
| +0x854 | u32 | | 0 at init | DefaultInit | | [M] |
| +0x860 | AIRoute | combat route | `AINetwork_ResetRoute(+0x860, obj, 0)`; +0x860 |= 0x80 when +0x23 | DefaultInit | combat movement | [M] |
| +0x8EC/+0x8F0 | f32 | | 2.0 / 2.0 (4.0 when +0x23) | DefaultInit | movement | [M] |
| +0x950/+0xA60 | ptr | path buffers | malloc'd `DAT_002c7958 << 1` bytes each (0x1508/0x4D08 tags), memset 0 | DefaultInit | navigation | [M] |
| +0x954 | u32 | nav path | 0 at init (docs: `+0x954` path id) | DefaultInit | navigation | [M] |
| +0x970 | AIRoute | patrol route | `AINetwork_ResetRoute(+0x970, obj, 0)`; +0x970 |= 0x80 when +0x23 | DefaultInit | patrol | [M] |
| +0x9FC/+0xA00 | f32 | | 0.5 / 2.0 (4.0 when +0x23) | DefaultInit | patrol | [M] |
| +0xA80..0xAD4 | — | zeroed group | 0 (0xA80, 0xA90/94/98, 0xAB0/B4/B8, 0xAC0, 0xAD0/2, 0xAD4, 0xA84) | DefaultInit | | [M] |
| +0xBA0 | u16 | | 0x80; |= 0x200 if prop 0x34 && !0x36; |= 0x100 if prop 0x3B or 0x3A | DefaultInit | | [M] |
| +0xBA4 | u32 | | 0x1C00 at init | DefaultInit | | [M] |
| +0xBAE | u16 | last wait node | (port claim) [S] | | patrol | [S] |
| +0xBB4 | u32 | cover node | (port claim) [S] | | cover | [S] |
| +0xBB8 | u16 | cover side/leave | (port claim) [S] | | cover | [S] |
| +0xBBB | u8 | cover type | (port claim) [S] | | cover | [S] |
| +0xBBC/+0xBBE | u16 | | 0/0 default; per-voice-set values (0xC, 0x40, 5, 9, 8, 10000 cases) | DefaultInit | | [M] |
| +0xBCC | u16 | grenade weapon | from **params[11]** (see below) | DefaultInit | grenade throw | [M] |
| +0xBCE | u16 | grenade count | params[11] & 0xF (before the switch overrides) | DefaultInit | grenade throw | [M] |
| +0xBD0 | f32 | | 0.5 at init | DefaultInit | | [M] |
| +0xBD4 | u32 | special-death script | params[13] (0x6000xxxx); when set: `+0xBE0` = obj pos vec4 copy, `+0xBF0` = obj rot copy; when != 0x600021F also sets 0x4F8 & 0x10 (stationary) | DefaultInit | special death | [M] |
| +0xBE0..0xBEC | vec4 | special-death pos | copy of `obj+0x30` | DefaultInit | special death | [M] |
| +0xBF0..0xBFC | vec4 | special-death rot | copy of `obj+0x50` | DefaultInit | special death | [M] |
| +0xC00 | ptr | obj (DIVars base) | `NDrone2_PostLoad_Init` reads the DIVars from here | `Drone_Create`/`DroneSpawner_SpawnDrone` | DefaultInit | [M] |
| +0xC10/+0xC20 | vec4 | placement trans / rot | copies of the placement's +0x0C / +0x18 | `Drone_Create` | — | [M] |
| +0xC34..+0xCB7 | u32×33 | params | verbatim `placement+0x2C` copy | `Drone_Create` | DefaultInit | [M] |

## Placement param → field mapping (from `NDrone2_DefaultInit` 0x14b300)

`piVar19[k]` in the decompile is `Drone+0xC00+4k`; params start at `piVar19[13]`. Param k =
`placement+0x2C+4k`.

| Param | DIVars word | Lands in | Meaning |
|---|---|---|---|
| 0 | piVar19[0xD] | `+0xE0` | model/skin id (0x0500xxxx); level 0x7000014 swaps 0x50000B1→0x500008C |
| 1 | piVar19[0xE] | `+0x554` | initial script (0x6000xxxx); 0x6000892/0x600089C → dtypenew 0x1C/0x1B, +0xDA 0xF |
| 3 | piVar19[0x10] | `+0x134` | switch channel (u8; +0x136 = its init state) |
| 4 | piVar19[0x11] | `+0x144` | spawner group |
| 5 | piVar19[0x12] | `+0x138` + mode select | behaviour mode: <0x24 → `DoModeSettingsOLD`, else `DoModeSettingsNEW` (07000005 uses 0x64 = NEW) |
| 6 | piVar19[0x13] | `+0x135` | second switch channel (+0x137 = init state) |
| 7 | piVar19[0x14] | `+0x148` | (raw word copied) |
| 8 | piVar19[0x15] | `+0x13A` | second tag |
| 9 | piVar19[0x16] | weapon/clip path | weapon id (`nf_formats::drone::weapon_id`); 0x16 on levels 0x7000005/6 sets props 0,1,6,0x41,0x42, clears 0x13, engage range 10.0 |
| 10 | piVar19[0x17] | `+0x45` | (byte) |
| **11** | **piVar19[0x18]** | **`+0xBCC`/`+0xBCE`** | **grenades**: `+0xBCC = raw >> 4`, `+0xBCE = raw & 0xF`; switch on `(raw >> 4) & 0xFFFF`: 1 → 0x36 (stun), 2 → 0x35 (frag), 4 → 0x34, 5 → weapon `count_raw + 0x3A` count 1, 6 → 0x6C count 1, default → 0/0; on 0x7000014 a weapon in (0x34, 0x37) is clamped to 0x34 |
| 12 | piVar19[0x19] | `+0x13C` | death channel (0x63 = mission fail) |
| 13 | piVar19[0x1A] | `+0xBD4` | special-death script (+pos/rot copies; stationary bit) |
| 14 | piVar19[0x1B] | `+0xE8/+0xF0/+0xFC/+0xE4` | sight class (consumes `Rand_FRand` draws in the documented order) |
| 15.. | piVar19+0x1C | behaviour blocks | passed to `DoModeSettingsOLD/NEW(instance, mode, params+15, DIVars)`: 15 = DTYPE +0xC7, 16/17 = normal mode/count, 18-20 words → +0x4D8, 21/22 = alert mode/count, 23-25 words → +0x4E8 |

## The flags word `Drone+0x4F8` [M except where noted]

`NDrone2_DefaultInit`: `+0x4F8 = (old & 0xFFEA05E2) | 0x000801E2` — i.e. clears 0x0015FA1D
and sets 0x80|0x100|0x40|0x20|0x2 plus 0x80000. Later in init: `|= 0x1A0` (0x20|0x80|0x100 —
gravity + collision); `|= 0x10` (stationary) when a special-death script other than 0x600021F
is present; `|= 0x44000000` when the start state is Attack (0x56/0x12). `NDrone2_Enable`
0x149dd0: enable sets `0x801A0` (0x80000|0x100|0x80|0x20), disable clears `0x1801A0`
(0x8000000|0x1000000|0x100|0x80|0x20).

| Bit | Meaning | Evidence |
|---|---|---|
| 0x2 | set at init | DefaultInit mask [M] |
| 0x10 | stationary (special-death; snipers via `initDTYPE_Sniper`) | DefaultInit [M]; [L] drone 0 (the sniper) has it, drone 9 does not |
| 0x20 | gravity | DefaultInit `|= 0x1A0` [M]; docs/systems/drone-collision.md |
| 0x40 | | set at init [M] |
| 0x80 | collision spheres | DefaultInit `|= 0x1A0` [M] |
| 0x100 | set at init; cleared when the drone is inside nothing (init tail: `obj+0xFE |= 1` delete if no cel) [M] | |
| 0x800 | cover claimed this run | port claim [S] |
| 0x1000 | has cover | port claim [S] |
| 0x2000 | may notice | port claim [S] |
| 0x8000 | | seen on [L] 0x19A1F2/0x19A1E2 drones (patrol pair) — meaning not chased |
| 0x10000/0x20000 | 0x20000 alerted by another drone | port claim [S] |
| 0x18000/0x100000 | seen on the patrol pair [L] — meaning not chased | [L] |
| 0x80000 | enabled (set by `NDrone2_Enable`, cleared by Disable) | [M 0x149dd0]; [L] set exactly on the four drones with `obj+0xF0 & 0x30 == 0` |
| 0x4000000 | a sighting happened | port claim [S]; init sets 0x44000000 for Attack-start drones [M] |
| 0x8000000 | own sighting / first sight | port claim [S] |
| 0x8000000+0x1000000 | cleared by `NDrone2_Enable` disable path [M] | |
| 0x80000000 | AI-boundary push-out | port claim [S]; present on all live drones [L] |

[L] raw values, 07000005 start: disabled channel-waiters `0x80000042` / `0xC6000042`
(Attack-start ones carry 0x44000000), enabled patrols `0x8019A1E2`, enabled sniper
`0x0019A1F2` (no 0x80000000 — the sniper never gets pushed).

## present / started / enabled / in-llist — the game-side fields

- **in llist / present**: membership in the drone LList **0x298e14** (links at instance+0x00/
  +0x04). Every drone with an object is in it (appended by `NDrone2_CreateObj` [M]). A
  **spawner template** is not: drones whose placement param 4 names a spawner group are held as
  pre-built DIVars inside the `DroneSpawner` object and only become objects (and list members)
  when `DroneSpawner_SpawnDrone` 0x137cb0 runs `NDrone2_CreateObj(template+0x10)` [M]. So the
  port's `present == false` = "no object yet" = "not in DynamicObjList, not in the LList".
  [L] the 07000005 spawner objects (3, class 0x30) coexist with 13 live drones.
- **enabled**: game-side = **`obj+0xF0` bits 0x10 (no draw) + 0x20** cleared, plus
  `Drone+0x4F8` bit 0x80000 set [M `NDrone2_Enable` 0x149dd0: disable sets 0x10+0x20 and clears
  0x4F8's 0x1801A0; enable does the reverse]. Disabled drones also get `AnimObject+0x98 = 0`.
  [L] 07000005: drones 1-8 (channel waiters, `obj+0xF0 & 0x30 == 0x30`, `+0x4F8` lacks
  0x80000) vs drones 9-12 (enabled, `& 0x30 == 0`, 0x80000 set) — the port's reading matches.
  The port's "object flags 0x30" is exactly these two bits.
- **started** (the state machine has had its first enter): there is no dedicated game byte;
  the state machine info at `Drone+0x108` (+0x10C current state, mirrored to `obj+0xF4`) is the
  evidence. [L] disabled drones sit in state 1 with `obj+0xF4 == 1`; the port's `started`
  (false until the first enter) is a faithful stand-in [S]. The Disabled state itself is
  `NDrone2_DSTATE_Disabled` 0x172440, which re-enables on its channel (reads +0x134/+0x136)
  or when channel 0 [M].

## Snapshot check [L `work/ee_start_05.bin`, 13 drones]

Three drones, raw values (instance = `u32(obj+0xE0)`):

1. **obj `0x01382bc0`** (the sniper): inst `0x01382cd0`; `inst+0x00C` = `0x01382bc0` and
   `inst+0xC00` = `0x01382bc0` (both back-refs equal the object ✓); health `+0xAC` =
   10.0, max `+0xB0` = 10.0 (default ✓); accuracy `+0xB4` = 5; dtypenew `+0xC5` = 2 (sniper
   mode ✓ matches docs/systems/drone-ai.md's 07000005 sniper); voice set `+0xD8` = 2;
   model `+0xE0` = `0x0500002F` (= params[0] at `+0xC34` ✓); switch channel `+0x134` = 0 =
   params[3] ✓; group `+0x144` = 1 = params[4] ✓; `+0x4F8` = `0x0019A1F2` (0x10 stationary
   ✓); `+0x5A2` = 0x0029 (SniperIdle ✓) mirrored to `obj+0xF4` = 0x0029 ✓; grenade fields
   `+0xBCC/+0xBCE` = 0/0 with params[11] (`+0xC60`) = 0 ✓; `obj+0xEC` = `0x0001A677`.
2. **obj `0x016d4a90`** (channel waiter): `+0x134` = 0x43 (= params[3] = 0x43 ✓);
   `+0x4F8` = `0x80000042` (no 0x80000 → disabled ✓, no stationary bit); `obj+0xF0` =
   0x000008B0 (& 0x30 = 0x30 → no draw, no vision ✓); `obj+0xFC` = 0 (not drawn); sphere
   radius `+0x8C` = 0.00 (AnimObject-driven sphere collapsed — never visible ✓);
   `+0x5A2` = 6 (Idle start), current state `+0x10C` = 1 mirrored to `obj+0xF4` = 1 ✓.
3. **obj `0x016ea040`** (patrol): `+0x4F8` = `0x8019A1E2` (0x80000 enabled ✓, no 0x10);
   `obj+0xF0` = 0x00000880 (& 0x30 = 0 → drawn/vision capable); radius `+0x8C` = 1.06;
   hip `+0xA0` = 1.15059 (per-model override of the 1.0 default); params[24] (`+0xC94`) =
   `0xECF377E5` — the alert-block word, **not** a grenade value; params[11] (`+0xC60`) = 0.

Also: `Drone+0xC00` = obj for all 13; the LList at 0x298e14 holds all 13 instances in creation
order (first = sm_id 1, last = sm_id 13, count 13).
