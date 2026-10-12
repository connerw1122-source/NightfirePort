# MissionData (ACTION.ELF 0x2a4350): on-foot mission and level order

*Measured* from the ELF's `.data` (symbol `MissionData`, 960 bytes = 24 records × 40). First pointed out by
Raikaru/nightfire-engine (*sourced*, facts only); layout re-derived here. Field meanings re-derived in full by fix-mission (docs/systems/mission.md): `+0x0C` points at the
level's **objective table** (the named `.data` symbols `Castle_Exterior` 0x2a3e28 … `Space_Station_D`
0x2a4320) and `+0x10` is its record count (matches every table's symbol size ÷ 0x18). Objective record
(0x18 bytes): +0x00 u32 text label (bank 4), +0x04 u32 fail label (−1 = none), +0x08 u32 hint label,
+0x0C u8 condition switch channel, +0x0D u8 its init value, +0x0F u8 enable channel (0 = none),
+0x10 u8 flags (bit 1 = the channel ON means FAIL), +0x14 u32 runtime state. `+0x20` (0x5604) and
`+0x24` remain unexplained but unread [M 0x1827d8/0x182270 read only +0x00..+0x14].

```
+0x00 u32 level hashcode (the 070000NN.bin archive)
+0x04 u32 mission: hashcode of the mission's first level
+0x08 u32 part index within the mission (0, 1, …)
+0x0C u32 pointer into .data (per-level list; w4 looks like its entry count — objectives?)
+0x10 u32 count (1–5)
+0x14..+0x1C 0
+0x20 u32 0x5604 in every record
+0x24 u32 0–4
```

Table order: the mission-part order of `Mission_Init`'s scan (BaseMap = +0x04, ThisOrderNum = +0x08
[M 0x182270:34-52]); the archive numbers are NOT the story order — the story order is
`ResetMap_LevelCode2Img`'s case order (docs/systems/levels.md §1). Rows:

| # | Mission (first level) | Levels |
|---|---|---|
| 1 | 07000005 | 07000005, 07000006, 07000007, 07000008 |
| 2 | 07000001 | 07000001, 07000002, 07000003, 07000004 |
| 3 | 07000009 | 07000009, 0700000a, 0700000b |
| 4 | 0700000c | 0700000c, 0700000d |
| 5 | 0700000e | 0700000e, 0700000f |
| 6 | 07000011 | 07000011, 07000012, 0700004a, 07000013 |
| 7 | 07000010 | 07000010 |
| 8 | 07000014 | 07000014, 07000015, 07000016 |
| 9 | 0700001b | 0700001b |

Driving missions are not in this table (they run in DRIVING.ELF). The other 07 archives (e.g. 07000017–29, 48, 4b) are
not campaign levels in this table (multiplayer maps, test/menu levels — to identify, U-024).

Archive numbers are therefore not play order: I first called 07000001 "the first level" by its number (wrong, WORKLOG
0011).
