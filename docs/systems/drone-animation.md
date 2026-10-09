# Drone animation: which script a drone plays

Drones (placement object type 0x0F, `Drone_Create` 0x136be0) choose animations through three static tables in
ACTION.ELF and the drone's type and anim set. Code: `crates/nf-formats/src/drone.rs`; the tables are read from the
user's ACTION.ELF through its symbol table (never committed). Everything here is *measured* from the decomp unless
tagged; none of it is confirmed live yet (U-028).

## Parameters

`parsemap_block_map_data_dynamic` (0x1d1b68) writes placement parameter k to `level_tag + 0x2C + 4k`;
`Drone_Create` copies 0x84 bytes from there to `DIVars + 0x34` (`Drone + 0xC34`). So in `NDrone2_DefaultInit`
(0x14b300): `DIVars+0x34` = parameter 0 (skin hashcode), `+0x38` = parameter 1 (start script), `+0x58` = parameter 9
(weapon), `+0x5C` = parameter 10, `+0x60` = parameter 11.

## Tables (ACTION.ELF symbols)

| Symbol | Address | Size | Layout |
|---|---|---|---|
| `DroneAnimStates` | 0x272c68 | 1,428 = 0x77 × 12 | per anim state: u16 own danim, u8, u8, …, u32 → transition list `{u8 key (requested state), u8 count, s16 danim, u8, u8}` until key 0xFF |
| `Drone_AnimInfo` | 0x273200 | 10,464 = 436 × 24 | per danim: u32 flags (→ `Drone+0x570`), u32, u16 anim state it enters (+8), u16 follow-on danim (+10), …, f32 (+0x14) |
| `Drone_AnimTables` | 0x275ae0 | 47,088 = 436 × 0x6C | per danim, 27 anim sets × {u16 script low half, u16}; script = `0x06000000 +` low half |

`DroneAnim_GetDAnimForAnimState(requested, current)` (0x13de40): the first transition of the current state's list
whose key is the requested state or 0. `DroneAnim_SetDAnimInternal` (0x13aee8): script = table[danim][anim set], or
set 0's entry when the set's is 0; 0x06000000 (both 0) means no animation (`DroneAnim_SetEndAnim`).

## Type and anim set (`NDrone2_DefaultInit`)

1. Skin switch (parameter 0) → `Drone+0xD8` (type) and `+0xDA` (anim set); unknown skins: type 0, set 0xC. Level
   0x07000014 replaces skin 0x050000B1 by 0x0500008C. Full table in `drone::drone_setup`.
2. Weapon switch (parameter 9; 1 when 0, but skin 0x0500001E forces 0): weapons 2, 6, 10, 0xB, 0xE, 0x10, 0x3B–0x40 →
   set 4 (8 for type 0xC); 0x12/0x14/0x15 → 6 (not type 0xB); 0x16, 0x18–0x1B → 5; 0x1C, 0x1D, 0x2A–0x2D → 9;
   0x1E, 0x1F, 0x24, 0x25 → 0x13; 0x30, 0x32, 0x33, 0x6A, 0x6E → 6 (not type 0x18).
3. Parameter 1: 0x06000892 / 0x0600089C → mode (`Drone+0xC5`) 0x1C / 0x1B, previous set kept in `+0xDC`, set 0xF
   (`NDrone2_DSTATE_AbseilSlide` restores it: *suggested* these are abseiling drones). Any other script is kept in
   `Drone+0x554` and played by `NDrone2_DSTATE_PlayScript` (0x15ba68).
4. Start danim (`DroneAnim_SetAnimD`): type 0xB → 0x1A5, type 0x18 → 0x82; mode 2 / 0x19 with set 5 → set 0x12 and
   danim 0x7D / 0x32; otherwise **0x11A = 282**, `DroneAnimStates[0x24]`'s own danim: anim state 0x24, stand idle.
   `DroneAnim_SetStandIdleAnim` (0x13ca68) later picks states 0x25–0x27 at random for some modes.

## Weapon

`NDrone2_DefaultInit` stores the weapon (parameter 9; 1 when 0, 0 for skin 0x0500001E) in `anim+0x62`; what is drawn
from it: docs/formats/weapon-data.md (`drone::weapon_id`).

## Live check (PCSX2, 07000005, WORKLOG 0017)

13 drones in the list at 0x298e14 (16 placements; the two at the gate z 44.86 and one at (50.85, 28.27, 93.63) were
gone — all have parameter 2 = 1 ≤ `GameState+0x28` = 1, so not the creation filter; U-029). Types 2, sets 5 / 0x13,
weapons 0x16 / 0x24 as predicted; the 9 drones not yet updated were on danim 0x11A, script `0600025c`; one at
placement y 37.085 sat at 37.003 with `Drone+0xA0` = 1.0, i.e. 1.0 above a floor at 36.0005 (feet sphere on the
floor). Updated drones had `Drone+0xA0` = hip − 0.02 exactly.

## Spawners (which drones exist at level start)

*Measured* from the decomp, confirmed live in three levels (WORKLOG 0018):

- `DroneSpawner_Create` (placement object type 0xF1; needs parameter 0 > 1) registers a spawner for group =
  parameter 1: max alive = parameter 2 (`+0xA`; 0 → the group size), max total = parameter 3 (`+0xC`, 0 → 32000),
  start channel = parameter 4 (`+0xE`), stop channel = parameter 5 (`+0x10`), spawn distance = parameter 8 (`+0x18`,
  0 → 10).
- Drones join group parameter 4 (`NDrone2_AddToGroup`, `Drone+0x140/+0x144`). `NDrone2_SetupGroups` →
  `DroneSpawner_Init`: every drone of a group with a spawner becomes a template (its `DIVars`, 0xC0 bytes, copied) and
  is deleted. With the start channel 0 (or already on) the spawner is active at once and ignores visibility.
- `DroneSpawner_SpawnDrone` recreates a template at its own placement position — chosen with `Rand_Rand`, and only
  when the player is beyond the spawn distance and cannot see the spot (unless the spawner ignores visibility).

`drone::spawn_roles` models the level-start result: immediate spawners bring back up to "max alive" templates (we
take placement order where the game draws at random), the rest are reinforcements. Live counts at start:
07000005 13 of 16 (the two gate guards wait for channel 0x22; of group 1 one of two is spawned — the one at
(62.43, 34.58, 104.08) in both sessions), 07000001 15 of 20, 0700000A 0 of 20 (six spawners). All 21 levels:
`drones.tsv` columns `at_start`, `spawners` (284 of 555 at start, 79 spawners). nf-view hides reinforcements; `R`
shows them at their spawn positions.

## On the disc (`nf-extract drones`, golden `drones.tsv`)

555 drones in 21 levels; 550 get a script that exists in their level and whose sequence is in the 07F archive; the 5
without are the 0x06000892 drones (07000011 ×2, 0700004A ×3; set 0xF has no stand idle). 15 use a parameter-1 script.
07000005: all 16 start with danim 0x11A; weapons 0x16 / 0x24 give sets 5 / 0x13, both → script `0600025c`
(150 frames, not looping) → sequence `040001d8`.

## Deviations in nf-view

- Drone mode (`Drone+0xC5`) comes from `NDrone2_DoModeSettings*` and is not modelled (0) except for parameter 1.
- A finished non-looping start script restarts at once (the AI re-calls the same stand idle); the game's AI decides
  when and may choose a variant.
- No AI, collision or gravity: drones stay at their placement x/z; height as docs/formats/animation.md.
