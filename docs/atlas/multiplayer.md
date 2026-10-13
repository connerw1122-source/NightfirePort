# Atlas: multiplayer — the arenas, the modes, the bots, split screen

research-multiplayer (2026-10-13). Everything here is first-hand: [M] = decompile/disassembly read in
this task (ACTION.ELF SHA-1 `e7ffdcb3…`), [D] = disc content measured by `mp_arenas_live.rs` or the
level survey, [S] = suggested/inferred, needs a live check or more reading. Sub-reports (full detail,
scratch): the three research files this doc condenses were `mp-arenas.md`, `mp-bots.md`,
`mp-splitscreen.md` (worker sandbox scratch, not committed). Nothing in `nf-game` changed — this is a
research deliverable plus one opt-in test.

## 1. The arenas

**Level codes.** The multiplayer codes are 0x07000021–0x29, 0x0700004b, 0x0700004c — the
loading-image switch `ResetMap_LevelCode2Img` 0x1d5258 returns the shared MP image 0x300016e for all
of them [M 0x1d5258:369-386]. The on-foot ISO ships **8** of them: `07000023…07000029.bin` +
`0700004b.bin` [D work/levels_new.tsv; 0x21, 0x22, 0x4c absent]. No story-table entry, no
mission-layer record for any of them [M docs/levels/0700004b.md].

**Content, pinned by `crates/nf-game/tests/mp_arenas_live.rs`** (probe run measured, then pinned):

| arena | placements | objects | drones (0x0f) | pickups (0xf0) | MP spawns (0x25) | MP objects (0x26) |
|---|---|---|---|---|---|---|
| 0x07000023 | 181 | 78 | 0 | 20 | 32 | 27 |
| 0x07000024 | 266 | 155 | 0 | 28 | 32 | 25 |
| 0x07000025 | 343 | 236 | 0 | 24 | 32 | 18 |
| 0x07000026 | 124 | 41 | 0 | 18 | 30 | 22 |
| 0x07000027 | 188 | 89 | 0 | 19 | 31 | 28 |
| 0x07000028 | 138 | 59 | 0 | 17 | 31 | 26 |
| 0x07000029 | 264 | 86 | 0 | 20 | 32 | 24 |
| 0x0700004b | 225 | 113 | 0 | 28 | 32 | 28 |

Every arena: **no campaign drones, no type-0x24 player start, no mission record** [D]; 30–32 spawn
placements (the type-0x25 registration caps the table at 64) and 17–28 pickups [D]. There is no
per-map scenario table in code — the scenario and map menus are independent lists (`mp_scenario` /
`mp_level` menu data, entries {id +0x0c, unlock flag +0x10}; quick-play samples `mp_level % 7`) [M
arena report]; map 0x0700004b only hides one options entry [M P_MPOPTIONS_Handler 0x217678].

**Placement types** ([M] `parsemap_create_dynamic_objects` dispatch; registration functions):

- **0x25 → `MP_RegisterSpawnPoint(vec3 pos, vec3 facing, u16 team)` 0x184f20** — appends a 0x30-byte
  SpawnPoint to the 64-entry `SpawnPoints` table (0xc00 bytes @0x2a7360 area): +0x00 pos vec4
  (floor-snapped, +1.6 eye), +0x10 facing vec4, +0x20 used flag [M]. `SpawnPntTeamCount` (8 bytes)
  counts per team; team games split the table [0,0x20)/[0x20,0x40) [M]. `MP_GetSpawnPoint(team,obj)`
  0x185098 picks farthest-from-enemies / 2nd-farthest / random per `MPSettings+0x448` [M].
- **0x26 → `MP_RegisterMPObject(vec3, vec3, level_tag*, celglist_tag*)` 0x185990** — the arena
  objects; dispatch on the celglist's type field `+0x2c`: 0/1 CTF flag+base, 2 uplink terminal, 3
  demolition site, 4 blueprint base, 5 blueprint item, 6/7 golden-eye item sets, 8 protection site,
  9 KOTH hill [M]. Each lands in a global array (see §3) plus an `MPOBJECT` at `obj+0xe0` (+0x00
  type 0..9, +0x02 team, +0x04 auto-return, +0x08 carrier, +0x10 home matrix, +0x50 last damager,
  +0x52 last claimer; `obj[0xff]=0x35` marker) and a 0x90-byte `MP_OBJ_EXT` (backlink +0x84) in the
  per-mode array; "places" records are 0x60 = matrix + 0x1c config + script ptr [M].
- **0xf0 → pickups** — the campaign pickup path (`Pickup_Create`), plus registration into
  `MPpickups` (64 × 0xa0 = 0x2800 @0x2a4b58) by `MP_RegisterPickup(obj)` 0x189d80: MP_PICKUP = obj
  +0x00, pos +0x08, AI emitter +0x20, per-bot visit floats +0x70 (f32[4] respawn deadlines) [M].
  `Pickup_MakeRandomWeaponSet` fills a row of the 10×10 `PickupMatrix` (u16 weapon ids) keyed by
  `MPSettings+0x436` (weapon set 0..10) [M].

## 2. MPSettings (the match config)

Global struct @**0x2a47a0**, saved/loaded by `LS_MakeMPSettings` 0x1ce508 / `LS_LoadMPSettings`
0x1ce5f0; only the first 0x1dc bytes are menu-persisted [M]. Key fields:

| off | meaning | notes |
|---|---|---|
| +0x000..0x17F | 8 per-player records × 0x30 | humans slots 0..3, bots 4..7; +0x00 name, +0x20 team (u32; = is-target in assassin), +0x24 skin, +0x28 saved-flag bit, +0x2c armor bonus (health = +100 at respawn) [M] |
| +0x188 | started flag | set by `MP_Start` 0x183a50 tail [M] |
| +0x1A4 | gametype bitmask | bot goal scoring (CTF/demo/prot/esp/GE/uplink/hill/assassin bits) [M] |
| +0x1AC | player count (u16) | = viewports; campaign sets 1 (`C_SBNFMAP_Handler` 0x20b8d8), MP 1–4, results screen forces 4 [M] |
| +0x384 | MP master enable | `MP_Start` returns if 0 [M] |
| +0x396 | team game | derived by `MP_Init` 0x183438 as `(scenario >> 29) & 1` [M] |
| +0x400 | bit30 flag | gates kill-scoring [M] |
| +0x412 | frag/score limit | 1..120 [M] |
| +0x416 | time limit | 1..60, −1 = none (units: minutes [S — verify]) |
| +0x420 | **scenario word** | the mode id, §3 [M] |
| +0x424 | level code | the selected arena [M] |
| +0x428 | human count | u32; `MP_Start` inits this many players [M] |
| +0x432 | bot count | clamped ≤ 4 [M] |
| +0x436 | weapon set | 0..10 → PickupMatrix row [M] |
| +0x444 | capture-weapon reward | [M] |
| +0x448 | spawn selection policy | 0/1/2 (farthest/2nd/random) [M] |
| +0x452 | radar on | [M] |
| +0x472 | registered pickup count | [M] |

**MPGame** @0x2a4980 (0x1d0) is the live match state: per-slot 0x30 (+0x18 f32 score, +0x1c obj ptr,
+0x22/+0x24 timers, +0x26 status bits, +0x28 last killer; +0x18 = score-limit copy when scenario
0x10), team scores +0x180/+0x184, state machine +0x392, round clock +0x400 / limit +0x404, elapsed
+0x412 [M].

## 3. The game modes (scenario word, MPSettings+0x420)

Dispatch: `MP_Update` 0x184bd0 (per-frame), `MP_CheckForEndCondition` 0x184268,
`MP_SortOutWhoWon` 0x1848d0, plus one `MP_*Update` per mode. 13 scenario values [M; the decompiler
prints some comparisons as pointer constants, e.g. `(undefined *)x == &DAT_20000040`]:

| id | mode | rules / scoring | source |
|---|---|---|---|
| 0x1 | Deathmatch | kill = +1 (`MP_PlayerKilled` 0x188f70 gates kill-scoring on id ∈ {1, 0x20000002, 0x40000800, 0x60001000}) [M] |
| 0x20000002 | Team Deathmatch | bit29 = team game (proven by the `MP_Init` shift); team score = Σ members [M] |
| 0x20000004 | Capture the Flag | `MP_FlagUpdate` 0x1861a8; flag obj state 0/1/2 at obj+0xf4; capture = +1 team; capture-weapon reward +0x444 [M] |
| 0x10 | (death-limit FFA — likely Last Agent / Espionage) | ends when all-but-one reach the death limit; winner = fewest deaths; −1 per death; no respawn at limit; 5 s pre-debrief delay [M; name [S]] |
| 0x20 | no-objective special | referenced only as an exclusion in kill-scoring [M; role [S]] |
| 0x20000040 | Demolition | round-based; site target HP obj+0xf4 init 2000 (`MP_DemolitionProtectionUpdate` 0x1870a0); attackers score on destroy, defenders on timeout; attackers get weapon 0x3b; demo team 1 / prot team 0 [M] |
| 0x20000080 | Protection | the same updater, mirrored sides [M] |
| 0x20000100 | BluePrint/Espionage | deliver blueprint to `EsponageBase[team]` (`MP_BluePrintUpdate` 0x1882d8, `MP_BluePrintReachedBase` 0x188970): +1 team +1 personal [M] |
| 0x20000200 | GoldenEye | two item types 6/7 (`MP_GoldenEyeUpdate` 0x187568); both carried by one team → the laser kills a random enemy, +1 to the opposite team [M] |
| 0x400 | Assassin | `Assasin` global = target obj, `DAT_0030d774` = assassin obj (`MP_assassinReset` 0x18aa70); +5 assassin kill, +3 target-kills-assassin; per-slot +0x20 repurposed as is-target; forces split spawn ranges [M] |
| 0x40000800 | King of the Hill (FFA) | `MP_KOHUpdate` 0x186ae0 + `MP_isPosOnHill` 0x18bc20; score += 0.2·60 Hz per frame held; 5 s regrab lockout [M] |
| 0x60001000 | King of the Hill (team) | same hill; team score = Σ members [M] |
| 0x60000008 | Uplink | ≤8 terminals (`MP_UplinkUpdate` 0x186d98); ownership 0/1/2 (neutral), hold-to-score; terminal RGB bytes obj+0x103..0x105 [M] |

Win conditions: score/time limit via `MP_CheckForEndCondition` (frag limit 1..120, time limit from
+0x416); 0x10 ends on the death limit [M]. `MP_SortOutWhoWon` picks the winner (highest score; 0x10
= fewest deaths) [M].

## 4. The bots

**Architecture** — an MP bot is an ordinary campaign Drone object: `BOT_init` 0x1249e8 calls
`Drone_Create` with a 0xB0 behaviour-prop block, then links the 0xD20 instance (`Drone+0xD1C`) to a
**0x780-byte `BOT_vars_t`** in the global array **0x26D660 (4 entries, slots 4..7, slot−4 = bot
index)** [M]. `BOT_initSys` 0x1262b0 and `NDrone2_initDTYPE_BotInit` 0x1291e8 are **empty** [M] —
bots reuse the whole shared drone state machine (`Drone_SM_SetState` on instance+0x108) plus a
dedicated `NDrone2_DSTATE_Bot*` family (~55 states, ids 0xC3..0xF9) dispatched by
`NDrone2_DSTATE_BotGlobal` 0x16e060 [M]. Bot detection = `Drone+0x4F8` bit 0x100 set / 0x600 clear
[M].

**The per-bot record** — 0x12 bytes at **0x2DEEAA** (`MP_Start` reads `(byte)[0x2deeb9 + i*0x12]`
for the team [M 0x183a50]): stats at +0x0..0xD (0 aggression, 2 aim, 4..5 u16 HP, 6 speed, 7/8
recovery, 0xA preferred weapon class, 0xB trait, 0xC flags — bit3 360° vision, bit4 regen), +0x0E
enabled, +0x0F team, +0x10 character, +0x11 dirty [M]. Defaults from `default_bot_stats` presets
(`BOT_getDefaultStats`); `BOT_setDroneStats` 0x126318 copies them to Drone+0xB4..0xBA [M]. There is
no global MP difficulty setting — difficulty is per-bot via this record [M].

**Goals** — two 0x50-byte goal slots at BOT_vars+0/+0x50 (slot 1 = mode objectives): CelPos, weights
(+0x2C/+0x30/+0x34 = pickup classes 3/1/0, +0x38 objective), 5 s timeout, type +0x45 (1 pickup, 2
objective, 3 hunt player), subtype +0x4A (1 flag … 8 demo/protect, 9 hunt) [M]. `pickGoal` 0x126b88
scores mode objectives by the `MPSettings+0x1A4` gametype bitmask and up to 64 pickups (per-bot
visit floats at MP_PICKUP+0x70, 45 s cooldown via `BOTSTATE_setPickupVisitTime` 0x129958 /
`MP_resetPickupBotVisitTimes` 0x18be50) [M]. `gotoGoal` 0x1278a8 drives the shared
`AINetwork_SetupGoalPosition(ToObj)`; `processGoals` 0x127c78 validates/completes and switches to
melee (state 0xF7) at dist² < 20.25 [M].

**Navigation** — `setNearestNavNode` 0x129420 just invalidates (0xFFFF at BOT_vars+0x760);
`validateRoute` 0x129880 dispatches 12 thunks via table **0x2F3DE0** (jump table unresolved in the
decomp — dump it from the ELF [M, open item]); `isPathWithinObjectRange` 0x128f80 walks the route at
`Drone+0x854` [M]. Bots use the same AI nav graph as campaign drones [M].

**Combat** — states 0xEB BotAttack, 0xD4/0xD5 strafe, 0xD7 backoff, 0xD9–0xDC rolls/steps, 0xDD/0xDE
weapon anims, 0xDF aim, 0xF9 goto [M]. Move chance = `base + (aggression+2) − speedLevel`
(`BOT_getMovePossibility` 0x1265d0) [M]; aim = sin sway ×1.2/×1.7 with a 0.1→0.001 error burst
scaled by aggression (`BOT_opponentTargetting`) [M]. Opponents chosen by trait
(`BOTSTATE_getPreferredTraitOpponentObjIndex` 0x128b50: nearest / top score / weakest / grudge /
lowest HP) + a 16-slot history ring (BOT_vars+0x6DC, current target ×5) [M]. Weapon choice:
`combatWeaponChangeChoice` 0x128708 sorts held/loaded, prefers explosives vs the demo/protection
objective, min-range from `weapon_data[w*0x10C+8]`; announces via `MP_sendBotMessage(0x42)` 0x18bb60
[M]. Messages: 0x3B objective, 0x42 weapon, 0x43 kill, event 0xC5 (`MP_sendTeamBotMessage`
0x18bbc8) [M].

**Difficulty stats** — aggression mult 0.3–1.0 (×3 point-blank), speed mult 0.7/1.0/1.3, regen +5 HP
flag, stun = stats[8]×2/3 frames, armour pool BOT_vars+0x769, per-bot 360° vision [M].

**Bot vs campaign drone** — shared: perception (`setOtherPlayerInfo` inside
`NDrone2_ControlSTANDARD`), movement, cover, impacts. Bot-only: the goal layer, traits, opponent
history, the BOTWEAP weapon-choice layer, messages, team objective locking
(`BOTSTATE_isObjAlreadyAnotherTeamObjective` 0x1297b8), Vengeful ×2 score in `MP_PlayerKilled` [M].
The trait↔name mapping (8 stub states) is [S].

## 5. Split screen

**Viewers** — `glb_viewer` @0x2d88b0 is an array of **11 `viewer_tag*`** (each 0x240 bytes,
`build_alloc_viewer` 0x1c9378): 0–3 = gameplay viewports, 4 = NIS overlay, 5 = full-screen 2-D HUD
overlay (empty world), 7 = menu, 8 disabled, 10 created but never drawn [M]. Created per level load
by `Camera_CreateCameras` 0x1bb2b8 ← `ResetMap_GameInit` 0x1d5080; `Camera_Create(idx, world, enable,
x, y, w, h)` 0x1bb0f8 stores the rect at viewer+0xf0(x)/0xf4(x+w)/0xf8(y)/0xfc(y+h)/0x10c(w)/0x110(h)
[M]. Frame loop = `Game_Draw` 0x1e6f50 ← `GameFlow_Main` 0x1c98c8 [M]. `psiUseCamera` 0x123700 writes
the GS SCISSOR (reg 0x40) straight from the viewer rect [M].

**Viewport rects** [M]: 1P = full 512×448; 2P: `DrawInfo`@0x2a37c0==1 → side-by-side 2×(256×448),
else stacked 2×(512×224); 3–4P = quadrants 256×224 (TL/TR/BL/BR = viewers 0..3). FOV refit via
`Camera_CalcViewAngles` 0x1baf70 (60° default; aspect ×0.5 side-by-side / ×2 stacked). Object
visibility = u16 view mask at `obj_tag+0xF8` (`View_SetDrawInOtherViewsOnly` 0x1e7ba0 =
`~(1<<i)&0x1f`; `Player_Update` 0x193600 reapplies it per frame for first-person body hiding) [M].

**Per-player HUD** — `HUD_Init` 0x199e88 hangs a 0x2C8 `HUDINFO` off `BLData+0x7e0` with 22 pane
slots from `MPPaneList` @0x2be9e0 (MP) vs `PaneList` (campaign) [M]. Every pane SpriteInfo's viewer
byte (SpriteInfo+0x28) is **0x05** — all pane HUD lives in the viewer-5 full-screen overlay pass,
one sprite-set per player, positioned into the owner's quadrant at creation via `HUD_ValidateXY`
0x19a250 (flags 0x80/0x100 add the player's viewer origin) and `HUD_FixMP` 0x19b110 (16 px insets on
outer screen edges per player count/split orientation; sprites halved when view < 512×448) [M]. Only
the crosshair is per-view (re-linked to `glb_viewer[BLData->0x94e]`, view-local coords) [M].
**`BLData+0x94E` (player index) is the viewer index** — the universal `glb_viewer[BLData->0x94e]`
idiom [M]. Radar (MP-only): `MP_GetRadarObjects` 0x189840 fills `RObj` @0x318e70 (0x20 stride: pos,
ARGB, type); `HUD_RadarUpdate` 0x19f6e8 unprojects via inverse viewer+0x120 matrix and
`View_3DPoint2Screen` 0x1e7708 [M]. The MP scoreboard/timer sprites (TimeSpr/StatusSpr from
templates 0x2a4710/0x2a4740, viewer 5) are repositioned per player count in `MP_Start` [M].

**Input routing** — `scePadPortOpen` ×8 slots (0–3 = port 0 + multitap, 4–7 = port 1) → `tSlot`
@0x245680 (0x180 stride, state==99 = connected) → `psiInput_PollDevices` 0x10ca78 →
**`psiInput_MapInputs(PlayerSetting@0x2a38c8, MPSettings._428_2_)`** 0x10cd80 → per-action
floats/edges → `Input_Action(playerIdx, id, mask)` 0x17f208 consumed per player [M].
**`MapPlayerToSlot` 0x10cc48** is the pad→player table: P0→slot0; P1→slot1 (multitap) else slot 4;
P2→slot2, P3→slot3 (multitap only, gated on prior pads) [M]. The 0x158-byte per-player block is
simultaneously PlayerSetting and PlayerInput (style u16 @+0x0E, invert @+0x00, 40 action floats
@+0x14, counters/edges/repeat) [M]; M_CONTROL (obj+0xDC) holds only runtime control state (+0x60 aim
flags, +0x62 weapon slot) [M]. Join screen: `P_MPJOIN` 0x20ca00 + `mpjoin` @0x2dee68 +
`Menu_UpdateMPControllers` 0x1f8d90 (connected flag +0x155) [M].

**Player init** — `Player_Init(short idx, VECTOR* pos, …, level_tag*)` 0x1910f8: sets
`BLData+0x94E=idx`, `Camera_Enable(idx,…)`, binds `glb_viewer[idx]` (+0x224 = player obj),
`Camera_Set(viewer, pos, yaw)`, registers the per-frame `Camera_SetUpdator(idx, obj,
Player_PositionCamera@0x18f5d8)`, then `HUD_Init` [M]. MP health = `MPSettings[i*0x30+0x2c]+100`
[M]; respawn `MP_ReSpawn` 0x185368 re-binds the camera the same way [M].

## 6. What the port reuses, what's new, and the phased plan

**Reusable now (ported code):** the arena maps parse with the current `Level` parser (`mp_arenas_live.rs`
is green on all 8 [D]); the drone object + state machine (bots are drones [M §4]); the AI nav graph
(`AINetwork_*` shared [M]); pickups incl. the placement path and PICKUPINFO (0xf0) [M]; the viewer
cels/renderer for a single view; the weapon system and weapon_data.

**Reusable data, no code:** the arenas' placements (0x25/0x26/0xf0 all parse [D]); the bot records at
0x2DEEAA and the MP pane lists (MPPaneList @0x2be9e0) are data tables to lift [M].

**New for MP (nothing ported yet):** the MP match layer (MP_Init/Start/Update, §2–3 globals ≈ 26 KB
of BSS state), the per-mode rules, the bot goal/BOT_vars layer (0x780/bot), the split-screen viewer
rects + scissor, the MP HUD panes + radar, and the pad→player map.

**Phased plan** (sizes = decompiled function counts touched, measured with `ls | grep -c` on
work/decomp/ACTION.ELF [D]):

1. **MP core harness** — MP_Init/MP_Start/MP_Update + MPSettings/MPGame + spawn-point registration
   and selection; load an arena, spawn 1 human + N static bots at spawn points, single viewport.
   ~15 functions (mp_* core), size M. Reuses the existing sim + player.
2. **Deathmatch end-to-end** — kill scoring (MP_HitBy/PlayerKilled/BulletHit), MPGame scores, HUD
   score/time panes (viewer 5 overlay), respawn (MP_ReSpawn). ~25 functions, size M.
3. **Split screen** — viewer rects for 2/4 players + scissor + per-view crosshair + MapPlayerToSlot
   input. ~15 functions (camera/view/input), size M. Needs the main agent's render check (lesson 6).
4. **The bots** — BOT_init + BOT_vars + goal layer (pickup/objective scoring) + the DSTATE_Bot combat
   states already partially shared with the drone port. ~40 functions, size L. Depends on the drone
   port's state-machine coverage.
5. **The other modes** — CTF/KOTH/Uplink/Demolition/Protection/GoldenEye/BluePrint/Assassin, each a
   small rules module on the shared MPObject/MP_OBJ_EXT layer (~10 functions each, size S–M each).
6. **Radar + MP panes + weapon sets** — MPpickups respawn, PickupMatrix random sets, radar
   unprojection. ~15 functions, size S–M.

**Live checks for the main agent** (none runnable here):
1. Boot an MP deathmatch in PCSX2; watch MPSettings+0x420 (w) — expect the menu's scenario id;
   MPSettings+0x428 (w) — expect the human count; MPGame+0x400 (r per frame) — the round clock.
2. `fieldprobe` MPSettings+0x424 after selecting map "Arena 1" — expect 0x07000023.
3. Watch `SpawnPoints` @0x2a7360 first entry +0x00 (w at level load) — expect the arena's first
   spawn placement translation (matches the 0x07000023 placement list).
4. Watch `glb_viewer` @0x2d88b0 entries 0..3 +0xf0..0xfc in a 2P match — expect 256×448 halves or
   512×224 stacks per DrawInfo.
5. Watch BOT_vars array @0x26D660 entry 0 +0x45 (w) during play — expect goal type 1/2/3 values.
