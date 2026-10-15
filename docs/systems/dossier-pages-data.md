# The Dossier's Rewards / Gadgets / Weapons pages (dossier-pages-data)

The Dossier wheel (Records / Rewards / Gadgets / Weapons) had only Records ported. This doc is the
evidence base for the other three pages' DATA, STATE, FLOW and GETTERS — no drawing (the main agent
draws each page and compares against PCSX2). Everything here is delivered in
`crates/nf-view/src/menu.rs` (`FrontEnd` getters + `Page` variants + `action()`) and
`crates/nf-game/src/progress.rs` (`Progress::object_upgrade_level`).

Evidence tags: [M] code (function + address, ACTION.ELF under `work/decomp/ACTION.ELF/`), [D] disc
bytes, [L] EE snapshot / .data (address + value), [S] suggested. The .data file-offset mapping used
below: EE address − 0x106100 = ACTION.ELF file offset (sp_level 0x2df2e0 ↔ 0x1d91e0, checked).

## The pages and their handlers

| page | id | PAGE handler | list/wheel handler |
|---|---|---|---|
| Records (ported) | 0x4000003a | P_DSRECORDS 0x217248 | C_RBDSRECORDS 0x20bf00 |
| Rewards | 0x4000003b | P_DSREWARDS 0x2172c8 | C_RBDSREWARDS 0x20c248 |
| Gadgets | 0x4000003c | P_DSGADGETS 0x217190 | C_SBDSGTSCROLL 0x20bcc0 |
| Weapons | 0x4000003d | P_DSWEAPONS 0x217038 | C_SBDSWPSCROLL 0x20bab0 |

The Dossier wheel's rows open them: C_SBDOSSIER 0x216eb0:12-31 — index 0 → 0x4000003a, 1 →
0x4000003b, 2 → 0x4000003c, 3 → 0x4000003d [M]. Page ids cross-checked against the menu-def's own
PAGE records ([D] 08000002 in 07000048.bin: pages 0x4000003a/3b/3c/3d all carry file hash
0x80000002, aux movie 0x07350048 — the same list-page backdrop as Records and the Single Player
list).

### The page-open / init messages [M]

Handler message ids (the same convention as every P_/C_ handler pair):

- **0x4c — page created**: P_DSREWARDS:11-15 `__Menu_Send(0x1000017a, 0x39, 0, 0)` (msg 0x39 on a
  scroll returns the scroll component [M Scroll_SendMessage 0x221810:110-111]) then
  `Menu_AddItemsToControl(scroll, sp_level 0x2df2e0, 0xc, 0, 1)` — the Rewards list IS the 12
  campaign rows — and `__Menu_SendMessage(scroll, 0x1e, GameState+0x72, 0)` (the list's
  select-by-value; GameState+0x72 is the level the front end had highlighted [S — read offset only]).
  P_DSRECORDS 0x217248 is the identical shape (list 0x10000174). P_DSGADGETS:9-16 resets the wheel
  (`__Menu_Send(0x1000016f, 0x2e, 0, 0)` — msg 0x2e sets the scroll's index [M Scroll_SendMessage
  :98-106]) and plays the open iris (`Menu_StartIris(0, …, 0x1000016d)`). P_DSWEAPONS:11-35 picks
  the overview row first (see below) then resets 0x10000173 the same way.
- **0x50 — page shown**: P_DSGADGETS:18-21 / P_DSWEAPONS:37-39 `Menu_PlayIris(1, …)`. The LIST
  handlers rebind on it: C_RBDSREWARDS:21-31 runs the whole bonus bind (below).
- **0x49 — the scroll moved** (NOT "up"): Scroll_Update 0x2221b20:237-241 sends msg 0x49 to the
  scroll's owner after EVERY index change, either direction. The list handlers rebind on it.
- **0x51 — control initialized** (the manager's CONTROL-finalize message, the same id the menu-def
  walker sends [M 0x21b908]): C_SBDSGTSCROLL:64-67 sets the wheel's range and cursor —
  `__Menu_SendMessage(scroll, 0x27, 0, 0xd)` (msg 0x27 = set range [a, b] [M Scroll_SendMessage
  :69-83]) then `0x2e (0, 0)` — range [0, 13], cursor 0. C_SBDSWPSCROLL:64-68 the same with 0x1a —
  27 weapons, cursor 0.
- **0x54 — rebind now** (the set-index notification; Scroll_SendMessage 0x2e forwards it to the
  owner [M :102-103]) and **0x4b** — no handler arm; these pages have no select action (Confirm does
  nothing [M the handlers' remaining arms return 1 untouched]).

### Scrolling [M Scroll_Update 0x2221b20:152-242]

Up/Down step the index ±1 and CLAMP at both ends; the wrap to the other end happens only when the
scroll's +0x144 flag is set, and that flag is set only by msg 0x69 (Scroll_SendMessage :136-139),
which no menu-def control carries (the parsed stream has zero 0x69 — the Records task's dump). Every
move plays a menu sound and sends 0x49 to the owner, which rebinds the page's texts. The wheel's
five visible slots come from `Menu_UpdateWheel` 0x1f8388 (below).

## The Rewards page

`C_RBDSREWARDS_Handler` 0x20c248 (the list 0x1000017a's handler, same spinner as Records'
0x10000174 — both b9, f2 0x3 [D the page data]):

- msg 0x35 on the list returns the selected row's value = the sp_level entry's level hash [S —
  `Menu_AddItemsToControl` fills the list from sp_level, whose +0x0c is the level; the Records
  handler uses the same 0x35 return as PlrStats_GetLevelTotals' key]. `Menu_GetLevelBonuses(level)`
  0x1ffe98 builds a 52-byte struct at 0x2de510 (memset 0x34 [M :11-14, the disassembly's `addiu
  a2,zero,52`]):
  - `+0x04 k·0x0c`: `Menu_HasMedal(level, k, 0)` for k = 1..4 — the bronze..platinum flags;
  - `+0x08 + k·0x0c`: the row's LABEL from the RewardsTable entry (byte offset +0x1c + k·4);
  - `+0x0c + k·0x0c`: the row's TEXTURE from the entry (+0x30 + k·4).
  Verified from the disassembly (tools/crosscheck/mipsdis.py on 0x1ffe98): the four label loads are
  `lw 28/32/36/40(v1)` = entry +0x1c/+0x20/+0x24/+0x28 and the four value loads `lw 48/52/56/60` =
  entry +0x30/+0x34/+0x38/+0x3c. (The decompile's `b_265._44_4_/_48_4_` prints are a Ghidra framing
  artifact; the real stores are `sw 44(s0)`/`sw 48(s0)` = struct +0x2c/+0x30.)
- The bind [M :21-74]: msg 0x2b param 1 (hide) to all eight controls first, then per row k —
  value control 0x100001d5+k gets 0x2b/2 (show) + msg 0x24 (texture) with the table texture, label
  control 0x100001d1+k gets 0x2b/2 + msg 0x18 `Txt_BindLabel(table label)`. The gates are
  CUMULATIVE: row 2 shows only if flag1 && flag2, row 3 adds flag3, row 4 needs all four
  [M :38-74 — the cVar1 chain]. Row 1 needs only its own flag. A fresh profile (no medals) shows
  no rows.
- `RewardsTable` 0x2bc920, 12 × 64 B [M symtab; L ee_start_05.bin — file offset = address]: +0x00
  hash, +0x04 +0x08 +0x0c +0x10 +0x14 the five 4-byte descriptors {u16 bit, u16 kind}
  (`PlrStarts_ProcessRewardCounter` 0x199210:30-33 reads desc[k] at `0x2bc924 + k·4 + entry·0x40` —
  counter k ∈ 1..4 → +0x08..+0x14; slot +0x04 is never read), +0x18 spare 0, **+0x1c..+0x28 the four
  bonus-row labels**, +0x2c spare 0, **+0x30..+0x3c the four bonus-row textures**.
- [L] the labels/textures per campaign row (SP_LEVEL_HASHES order, `work/ee_start_05.bin`):

  | row | hash | labels (bronze..platinum) | textures |
  |---|---|---|---|
  | 0 | 0x09000001 | 0x2d6, 0x2ec, 0x010001f9, 0x2f2 | 0x0300012c, 0x03000111, 0x03000102, 0x03000118 |
  | 1 | 0x07000005 | 0x2d2, 0x2e9, 0x3a5, 0x2f1 | 0x03000134, 0x03000116, 0x030000f3, 0x03000108 |
  | 2 | 0x09000005 | 0x2d3, 0x2de, 0x2df, 0x10001fc | 0x03000131, 0x030000a1, 0x030000e2, 0x03000196 |
  | 3 | 0x09000006 | 0x2d4, 0x2ea, 0x3a6, 0x2ee | 0x03000133, 0x0300011a, 0x030000df, 0x03000113 |
  | 4 | 0x07000001 | 0x2d5, 0x2e0, 0x3a7, 0x2ed | 0x0300012f, 0x030000a4, 0x030000fa, 0x0300011d |
  | 5 | 0x07000009 | 0x2db, 0x2e1, 0x3a5, 0x2f0 | 0x0300012e, 0x030000a3, 0x030000f3, 0x0300010b |
  | 6 | 0x0700000c | 0x2d7, 0x3a3, 0x3a8, 0x2e8 | 0x03000130, 0x0300011c, 0x030000e0, 0x03000110 |
  | 7 | 0x07000011 | 0x2d8, 0x2e3, 0x3a5, 0x3a2 | 0x0300012a, 0x0300009d, 0x030000f3, 0x03000148 |
  | 8 | 0x09000002 | 0x2d9, 0x3a4, 0x3a9, 0x2ef | 0x03000198, 0x03000114, 0x030000e1, 0x0300010f |
  | 9 | 0x09000003 | 0x2da, 0x2e5, 0x3aa, 0x3a1 | 0x03000132, 0x0300009e, 0x030000e6, 0x0300010a |
  | 10 | 0x07000014 | 0x2dc, 0x2e6, 0x3ab, 0x2eb | 0x0300012d, 0x030000c4, 0x030000c4, 0x03000117 |
  | 11 | 0x0700001b | 0x10001ff, 0x2e7, 0x3a7, 0x10001fb | 0x0300012b, 0x030000a0, 0x030000fa, 0x03000194 |

  (The 0x0100xxxx labels are the composed-string bank — bound as-is, same as the Dossier wheel's
  0x100000a-d descriptions.)

- Page data [D 08000002, parsed 2026-10-15]: header LBL 0x281 (50,40,376×17); hint LBL 0x49e
  (50,419) — Records' own hint; the spinner 0x1000017a (132,93,376×16); four stamp controls
  0x100001d6..d9 (76,130/202/274/346, 51×51) with authoring TEX 0x0300012c/0x0300008c/0x03000113/
  0x03000118 and four caption controls 0x100001d2..d5 (137,147/219/291/363, 453×16) with authoring
  LBL 0x2d2/0x2dd/0x2e8/0x2e8 — both sets are overwritten by the handler per level; the authoring
  values only prove the control/row mapping. Four frames (68,84,505×34) and
  (68,121/193/265/337, 505×69); logo (434,38,156×43) TEX 0x0300016d.

## The Gadgets page

- Table `ds_gadgets` 0x2e0150, 14 × 24 B [M symtab; L ee_start_05.bin]. Row layout (read by
  `Menu_UpdateWheel` 0x1f8388 and `C_SBDSGTSCROLL` 0x20bcc0): **+0x00 preview texture hash,
  +0x04 wheel-row label, +0x08 description label, +0x0c upgrade object id, +0x10 unlocked byte,
  +0x14 locked-description label** (UpdateWheel binds +0x08 when +0x10 ≠ 0, else +0x14, and dims
  the preview to 0x60606060 when +0x10 == 0 [M 0x1f8388:96-112, 137-146]; the wheel's five visible
  rows bind +0x04 [M :174-203]).
- [L] the 14 rows (ee_start_05.bin 0x2e0150):
  `0x030000e6/0x323/0x331/0x12`, `0x030000c4/0x321/0x32f/0x15`, `0x030000e2/0x328/0x336/0x03`,
  `0x030000df/0x329/0x337/0x06`, `0x030000e1/0x326/0x334/0x0f`, `0x030000e4/0x327/0x335/0x3f`,
  `0x030000e5/0x32a/0x338/0x3f`, `0x030000e7/0x324/0x332/0x3f`, `0x030000e0/0x322/0x330/0x0c`,
  `0x030000e3/0x325/0x333/0x3f`, `0x03000104/0x32b/0x339/0x3f`, `0x030000fc/0x32c/0x33a/0x3f`,
  `0x03000190/0x32d/0x33b/0x3f`, `0x030000fb/0x32e/0x33c/0x3f` (preview/label/desc/object; every
  +0x10 = 1, +0x14 = 0 — all unlocked in the boot state [L]; no runtime writer of +0x10 was found
  [S]).
- P_DSGADGETS:11-15 overwrites `ds_gadgets+0x144` (the LAST row's +0x0c object field) with
  0x030000e5 at 60 Hz / 0x0300018f at 50 Hz (`VIDEO_FRAME_RATE == 0x32`) [M] — a texture hash in an
  object slot; `Menu_GetObjectUpgradeLevel`'s `param_1 < 0x3f` guard then always returns 0 for that
  row, i.e. no upgrade suffix either way. The port keeps the static 0x3f and cites the store.
- The upgrade suffix [M C_SBDSGTSCROLL:75-111]: after the wheel move, msg 0x40 returns the cursor
  (Scroll_SendMessage :119-125 — the clamped cursor), `Menu_GetObjectUpgradeLevel(object, 0)` gives
  the level, and when the level ≠ 0 the description becomes `sprintf("%s\n%s", desc,
  Txt_BindLabel(suffix))` — the format string at 0x30d358 is `"%s\n%s"` [D ACTION.ELF .data
  0x30d358]. Suffix labels by object: 3 → 0x3b3, 6 → 0x3b4, 0xc → 0x3b5, 0xf → 0x3b6, 0x12 →
  0x3b7, 0x15 → 0x3b8; any other object (0x3f) gets none [M the switch at :82-104]. The composed
  string is bound to control 0x1000016b with msg 0x18.
- Page data [D]: header LBL 0x282; hint 0x49f; panel TEX 0x030000b3 (50,53,590×295); wheel rows
  0x1000016a ×5 ((304,182,336×17) d1 2 centred, (323,123)/(333,152)/(333,211)/(323,241) d1 0/1/3/4);
  description plate 0x1000016b (50,301,540×113) f2 2; preview 0x1000016c (114,118,150×150) f1 0x1d
  (authoring TEX 0x03000085) UNDER the ring 0x1000016d f1 0x1c TEX 0x030000bf; the scroll control
  0x1000016f (312,48,16×32) b10 f2 2 with the page's own MSG 0x2e (0,0) — the init-to-row-0.

## The Weapons page

- Table `ds_weapons` 0x2dfec8, 27 × 24 B, the same row layout [M symtab + the handlers; L
  ee_start_05.bin]: rows 1..26 are the guns — e.g. [1] `0x030000ed/0x33f/0x35b/0x3f`, [9]
  `0x030000fe/0x347/0x363/0x09` (the watch — object 9), [26] `0x03000192/0x358/0x374/0x3f`. Full
  table in `menu.rs` `WEAPONS_TABLE` ([L] values).
- **Row 0 is the overview the page handler rewrites** [M P_DSWEAPONS 0x217038:11-33]:
  `Menu_GetObjectUpgradeLevel(0, 0)` — the watch upgrade level (object 0's kind-4 gold group has
  three members → level 0..3) — then writes `ds_weapons[0]` +0x00/+0x04/+0x08:
  | level | preview | name | desc |
  |---|---|---|---|
  | 0 | 0x030000f3 | 0x33d | 0x359 |
  | 1 | 0x0300014f | 0x3c5 | 0x3c6 |
  | 2 | 0x030000f2 | 0x33e | 0x35a |
  | 3 | 0x03000150 | 0x3c7 | 0x3c8 |
  (the boot .data holds exactly the level-0 values [L]). The getter returns these for row 0.
- The upgrade suffix [M C_SBDSWPSCROLL:76-89]: object 9 (the watch, rows with +0x0c == 9) at level
  1 appends label 0x3b1, at level 2 label 0x3b2 (`"%s\n%s"` again); level 3 adds NO suffix and
  other objects none.
- Page data [D]: header LBL 0x19c; hint 0x49f; wheel rows 0x10000171 ×5 (same geometry as the
  gadgets'); description plate 0x10000172 (50,301,540×113); ring 0x1000016e f1 0x1c TEX
  0x030000bf, preview 0x10000170 f1 0x1d (authoring TEX 0x030000b9) UNDER it; scroll 0x10000173
  (312,49,16×32) with MSG 0x2e (0,0).

## The upgrade levels (`Progress::object_upgrade_level`)

`Menu_GetObjectUpgradeLevel` 0x2025f8: guard `object < 0x3f` [M :57]; walk the campaign's 12 levels
in order (`sp_level+0xc`, 12 × 0x18 [M :58-84]), counters 1..4, `PlrStarts_ProcessRewardCounter(
&bonus[slot], level, counter, 0, &info)`; on the FIRST info with the kind-4 descriptor whose bit
equals `object` and whose (widened) bit is set, return the group's set-bit count. The object ids in
the two tables are kind-4 descriptor BITS: gadgets {0x12, 0x15, 0x03, 0x06, 0x0f, 0x0c}, weapons {9
(the watch), 0x3f = none}; object 0 = the three-member watch group (entries 1/5/7 → bits 0..2).

**Found while working — the gold-group fix**: `Progress::has_medal`/`process_reward_counter`
grouped kind-4 descriptors by `REWARDS_TABLE[i].desc[0]`'s packed word — but desc[0] is the never-
read slot 0 whose word is 0 in every entry, so every level collapsed into ONE 12-member group with
rank = table index (bits 0..11 by level). The game scans descriptors **1..4** of every entry for
the target descriptor's own packed word [M 0x199210:39-62], so the real groups are {entries 1,5,7}
(word 0x40000 → bits 0,1,2 — the WORKLOG test comment "the gold group of CastleExterior/TowerA/
Tower2A spreads bits 0..2" already described this) and {entries 4,11} (word 0x40009 → bits 9,10),
all other kind-4 descs singletons at their own bit. Fixed `gold_group` to take the counter and scan
desc[1..=4]; the non-gold medal bits are unchanged, and nf-game's 188 lib tests pass on the fixed
tree.

## What the port delivers (no drawing)

- `Page::Rewards` / `Page::Gadgets` / `Page::Weapons` (+ ids 0x4000003b/3c/3d), the Dossier wheel's
  Confirm rows 1..3 open them (cursor reset to 0 — every page's init msg 0x2e (0,0) [M]), Back/□
  return to the Dossier (the same default the Records page maps; no handler arm overrides it [S]),
  Up/Down clamp without wrap [M 0x2221b20].
- Getters: `FrontEnd::reward_rows(row)` (the four (shown, label, texture) rows with the cumulative
  gates), `FrontEnd::gadget_row(row)` / `weapon_row(row)` (preview, name, desc, suffix, unlocked),
  `FrontEnd::weapon_overview_row()` (the watch), `Progress::object_upgrade_level(object, slot)`.
- Tables as cited statics: `WEAPONS_TABLE`, `GADGETS_TABLE`, `REWARD_BONUS` (labels + textures).
- `--page rewards|gadgets|weapons` render hooks in `crates/nf-view/src/lib.rs` (additive; minimal
  draw arms in `menu_overlay` — the main agent's draw pass replaces them against PCSX2).
