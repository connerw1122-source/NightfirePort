# MissionData (ACTION.ELF 0x2a4350): on-foot mission and level order

*Measured* from the ELF's `.data` (symbol `MissionData`, 960 bytes = 24 records × 40). First pointed out by
Raikaru/nightfire-engine (*sourced*, facts only); layout re-derived here. Field meanings beyond the first three are
not yet established (U-024).

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

Table order (assumed to be play order — plausible, unverified):

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
