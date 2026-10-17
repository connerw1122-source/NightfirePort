# The front end's data (fix-menus2)

The menus are data-driven: `MenuManager_Load` 0x21de88 stashes a kind-8 LevelBin entry and
`MenuManager_Create` 0x21b908 walks it. Confidence: high for the chunk layouts (derived from the
decompile's `BIN_Get*` order and validated against the real bytes to a clean END); the widget semantics
are measured at the level the port draws them.

## Where the data lives

- `FILES.BIN` → `07000048.bin` (the front-end level archive) → kind-8 entry **`08000002`** (133,536 B).
  `bootup_bootup` 0x1c7e78 sets GameState+0x20 = 0x80000002 (the file hash) and +0x28 = the page id;
  `ResetMap_Load` 0x1d6360 calls `MenuManager_Create(+0x20, +0x28, …)` on the front-end map.
- The archive's textures carry the art: title 0x03000074 (512×512), panel 0x030000b3 (512×256), ring
  0x030000bf, ring-masked previews 0x03000084/b6/c4, difficulty emblem 0x030000a5, logo 0x0300016d,
  stamp 0x0300014c, button/arrow atlas 0x03000042 (64×64: square face (0,0), circle (0,16), ▲▼◀▶ arrows,
  scroll bar, big ◀▶).
- Strings come from `USATxt.dat` through `Txt_BindLabel` 0x1cad60 (`nf-formats::hud::Lang`; the bank is
  CP-1252 — fix-menus2 decodes it at parse time).
- The campaign is `sp_level` 0x2df2e0: 12 × 0x18 records {u32, u32 name label, u32 description label,
  u32 level, u8 unlocked, …} [M C_SBNFMAP 0x20b8d8:21].

## The tag stream (0x21b908's switch)

| tag | payload | meaning |
|---|---|---|
| −0x10 PAGE | 17 B {u32 page id, u32 file hash, u32 aux movie, u8 kind, u32 size} | kind 0/3 + file match → `Manager_SendMessage(mgr, 0x43, page id, hash)`, page+0xc4 = aux; else skip `size` bytes |
| −0x07 CONTROL | 30 B {u32 name, u8 b, u8 type, u16 x, y, w, h, u16 f1, u32 d1, u16 f2, u32 d2, u32 d3} | finalize the previous control (msg 0x51); type 0/3 → `Manager_SendMessage(mgr, 0x42, …)` creates; full-screen fixup (type 5, x<1, y<1, w>0x27f, h>0x1df → w/h 0x200/0x1c0); control 0x10000224 forced to (340,198) [M 0x21b908:305-314] |
| −0x08 NAME | 32 B string | msg 0x23 (wide → narrow if it matches DAT_0030d610) |
| −0x09 SCRIPT | {u32 script id, u32 arg} | `Script_AddScript`; script+0x1c = arg |
| −0x0a MSG | {u8 msg, u32 a, u32 b} | `__Menu_SendMessage`; **0x18 = Txt_BindLabel, 0x24 = texture hash, 0x1b/0x1c = colours**; 0x1a-0x1c with a == 0x7d6d59ff rewritten to 0x645a49ff [M :238-243] |
| −0x0b KFRAME | 5×u16 + u32 | `FixupResolution` + `Script_AddKeyFrame` |
| −0x0c KMSG | {u32 time, u8 msg, u32, u32} | `Script_AddMessage` |
| −0x0d MGRMSG | {u8 msg, u32, u32} | manager message |
| −0x03 SET | u16 | `Component_AddSet` — a sprite set begins |
| −0x04 COMPONENT | 14 B {u16 count, 3×u16, u32 texture hash} | `Component_AddComponent` (stride 0x14); followed by `count` SPRITE records |
| −0x05 SPRITE | 32 B {u16 x, y, w, h, u1, v1, u2, v2, u32, u32, f32, f32} | `Component_InitInstance` (instance stride 0x38) |
| −0x02 SKIP | u32 | skipped dword |
| −0x0e END | — | finalize + return |

Parsed 2026-10-13: 56 pages, 1003 controls, clean END with 19 spare bytes. Coordinates are a **640 × 480
space** (each page's root control is (0,0,640,480)); `FixupResolution` scales per output resolution — the
port maps ×512/640, ×448/480 onto the game's 512 × 448 screen.

## The pages the port draws (all cited at the use site in `crates/nf-view/src/menu.rs`)

| page | identity [M] | key controls (640×480) |
|---|---|---|
| 0x40000034 | boot page (P_LANGUAGE [M 0x2029e0:302]; the title P_START is 0x40000009, :321) — its 7 textured controls are language FLAGS (0x030000c9-cf) | flags at (282,109..349,64×32); hint 0x3c3; logo 0x0300016d |
| 0x40000009 | the visible title | art 0x03000074 (0,0,640,480); prompt 0x100000fa (131,309) yellow 0x7f621fff; legal plate (50,332,540×108) |
| 0x40000002 | main menu | rows (167,262/294/326,306×28) LBL 0x28/0x149/0x14b; description (103,357,434×42); hints 0x54e; aux 0x07330048 |
| 0x40000025 | Select Codename | header (50,40) 0x1f2; panel 0x030000b3 (50,53,590×295); emblem 0x030000bf+0x030000b6 (114,118,150×150); wheel rows (323/333/304/333/323, 123..241); confirm plate (50,301,540×113); hints 0x216; aux 0x07350048 |
| 0x40000023 | Select Difficulty | header 0x1f8; emblem 0x030000a5+0x030000bf; wheel rows; description plate; hints 0x218 |
| 0x4000001c | Single Player (mission list) | header 0x14c; preview 0x03000084 + ring; wheel rows; description plate; hints 0x218 (the game swaps in the □ Dossier set 0x208 [L PCSX2]); LOADING 0x67e |
| 0x4000002b | Dossier | header 0x27f; emblem 0x030000c4+0x030000bf; rows; hints 0x218 |
| 0x40000047 | difficulty wheel (P_NFDFCTY) | full-screen 0x03000074; spinner bars 0x100001eb; frame corners 0x03000042 |
| 0x4000001c = C_SBNFDFCTY's confirm target | — | difficulty confirm opens the Single Player list [M 0x216ca8:10-17] |

The wheel rows: five visible slots, the selected row centred (y=182, x=304) at full colour, ±1 rows at
alpha 0x80, ±2 at 0x40 [M the rows' msg-0x1b colours].

## The flow [M the page handlers + L PCSX2 fresh boot 2026-10-16]

Boot: P_LANGUAGE 0x40000034 → P_PS2MEMCARDINIT (→ 0x40000032 [M 0x218538:18]) → **P_INTRO 0x40000032**
plays 0x07370048 (EA Games) then 0x07380048 (MGM Interactive) and opens the title 0x40000009 [M
P_INTRO_Handler 0x209e88]. Title (P_START 0x40000009; 45 s idle → attract 0x40000035 [M 0x20a408]) → START →
P_PARISENUM 0x4000004a (the "Checking memory card" box [M 0x20a648]) → **P_ESTHERO 0x40000043** plays
0x073B0048 (the rings/Earth intro; 3 s unskippable grace, `DAT_002e1c4c = 0xb4`) → Main 0x40000002 [M
P_ESTHERO_Handler 0x20a060]. **Corrected 2026-10-16:** this section used to say START → P_INTRO's logos →
Main (an observation from a save-state resume, not a fresh boot); the port and its tests followed it and
showed the logos after START with no real intro. **fe-audit 2026-10-16:** the port runs both intros as
menu background movies with GameFlow in its runner state (no state 6) and the run's own page up
(P_INTRO / P_PARISENUM / P_ESTHERO — Main opens only on the movie's end [M 0x20a060:28-45]); the boot
logos are unskippable (c49 = 0 [M 0x209e88:41,55; 0x215a50:6]); NightFire/Codenames change page after
0x13 ticks with input blocked [M 0x2169b8:29-37, 0x216b38:29-37]; the codename page opens on row 1 from
Main [M 0x20aec0:96-97], the difficulty wheel on index 0 [M 0x216bf8:12], the mission list on the highest
unlocked row [M 0x20b608:58-64]; scroll wheels wrap unless msg 0x69 [M 0x2214f0:102, 0x221810:136]; a
failed mission's menu opens over the paused mission with no load [M 0x182c60:77-80]. Main → NightFire →
Select Codename 0x40000025 → "Play without saving" → the confirm box (Txt 0x10000bb) → ✕ → Select
Difficulty 0x40000023 → ✕ → Single Player 0x4000001c → ✕ on an unlocked row →
`ResetMap_LevelToLoad(sp_level[i]+0xc, 0, 0)` + a fade [M C_SBNFMAP 0x20b8d8:18-41]. Locked rows:
`if (!menu_unlock_everything 0x30d2a7 && !sp_level[i*0x18+0x10]) return` [M]; completing row *i* unlocks
*i+1* [M Menu_SetLevelBonus 0x1ffa10:63-65]; □ opens the Dossier.

Port notes: `crates/nf-view/src/menu.rs` (the layouts as cited constants + the wheel/hint/emblem
drawers), `crates/nf-view/src/lib.rs` `front_end_menu` (`--shot --page …` renders each page),
`crates/nf-view/tests/menus_flow.rs` (the sequence, the label ids per page, the sprite hashes per page,
the unlock rule — with NF_ISO sisters against USATxt.dat and the EE snapshot).

## The 2026-10-13 render gaps, closed (fix-menus3)

The main agent's `nf-view --shot` renders next to the PCSX2 captures found eight differences; what each
was and what fixed it:

1. **Framing** — the game draws the 640×480 menu space as a centred 4:3 picture [L PCSX2 captures]; the
   port's `Anchor::Stretch` quads filled the whole window. The overlay now maps every quad and text
   through `Anchor::Centre`, which is the window's central 4:3 band (identical to Stretch at a 4:3
   window). `crates/nf-view/src/menu.rs` `quad640`/`plate`/all text calls.
2. **The legal block** — wraps inside its plate (50,332,540×108) and each line is centred at the plate's
   centre x=320; `wrap_px` measures lines with the real font instead of the old char-count wrap. The
   "Â©" came from `Font::layout` walking BYTES of the CP-1252-decoded UTF-8: "©" (C2 A9) drew glyphs
   0xC2 + 0xA9. `layout` now walks chars — one glyph per source byte, the game's own mapping
   (`__Font_DrawText` 0x1ee598 walks NUL-terminated byte strings).
3. **The main menu's backdrop** — page 0x40000002 has NO static art control; its PAGE record names aux
   movie 0x07330048 (every list page 0x07350048) [L the menu-def PAGE records]. The real background is
   the looping FMV. The trimmed ISO stubs every movie (0-byte PSS [L]), so the port keeps the dark
   plate; a still from the retail image is the follow-up. The rows themselves are LBL text at
   (167,262/294/326,306×28) — the port's text rows are the page data's own.
4. **Headers** — the page data says LBL text (0x1f2/0x1f8/0x14c at (50,40,376×17)), not images; the
   stylised look in the captures is the game's menu font, which the port has not parsed (reported).
5. **Hint icons** — the game draws the ~A/~B/~X/~Y/... tokens through the `specialchar` table
   (0x2dd8d8, 17 × 0x18 bytes: texture 0x03000075, u0, v0, w, h, advance) [M symtab + table bytes].
   The port's 64×64-atlas 0x03000042 tiles and the letter-X fallback for ✕ are gone; the table's own
   tiles draw (✕ = 235,1,19×19).
6. **The ring preview** — clipped to the ring's circle: the emblem controls draw the preview (f1
   0x1d/0x1e) UNDER the ring (f1 0x1c — lower layers draw on top [M hud.rs finish]); the port renders
   the preview as 24 chord strips of the inscribed circle (a HudFrame has no per-pixel mask).
7. **Mission previews** — `sp_level[i]+0x00` is the row's own preview texture hash [M
   `Menu_UpdateWheel` 0x1f8388 sends it as msg 0x24 to control 0x10000020; the 12 hashes read from the
   EE snapshot at 0x2df2e0 [L]]. The port's CAMPAIGN held only name/description/level (the old
   "+0x04 name" reading was wrong); it now carries all four lanes and the Single Player page draws
   CAMPAIGN[cursor].0. Locked rows dim to 0x60 and show +0x14's "Continue playing to unlock this
   mission." A fresh profile unlocks rows 0 AND 1 [D sp_level's boot state in ACTION.ELF .data,
   file offset 0x1d91e0].
8. **Descriptions** — wrapped inside their 540px plates with `wrap_px` (the difficulty and mission
   descriptions ran off the right edge; the mission one was even re-joined into a single line).

## G-048: the CloseIris delay, the per-page sound flags, the results order, the card check (fe-gaps-g048, 2026-10-17)

**`Menu_ChangePageCloseIris(page, mgr, ctl)` 0x202190** [M, disasm 0x202190-0x20220c]: the page
change is THREE sends — `Menu_StartIris(1, …)` (the close) at once, `__Menu_SendDelayedMessage(1,
…, 0x68, 1, 0)` (the input block) at tick 1, and `__Menu_SendDelayedMessage(10, …, 0x44, page,
DAT_002e1adc)` at tick 10; the 0x44's own processing lifts the block (`Manager_SendMessage(0x68, 0,
0)` on the lparam bit-1-clear arm [M 0x21c858:314-315]). `DAT_002e1adc` reads 0 in .data [L
ACTION.ELF file offset 0x1dbadc] and P_PARISENUM's 0x4c zeroes it [M 0x20a648:21] — lparam 0, so
the arrival pushes the opener onto the manager's page stack [M 0x21c858:262-263]. Every caller
[M the decompile's xrefs]: C_SBNFDFCTY 0x216ca8:16 (the difficulty confirm → the SP list),
P_NFSELECT 0x20aec0:103/:108, C_SBNFCN 0x20b390:47 (→ the name entry), C_SBNFMAP 0x20b8d8:48 (□ →
the Dossier), C_SBMPSCEN 0x20c600:45/:142, C_SBMPOPTIONS 0x20e818:116, C_SBBOTS 0x210a08:22,
P_CNSELECT 0x212ad0:105/:112, C_SBCNSELECT 0x212f58:41, P_CNMENU 0x213288:49, C_SBCNOPTIONS
0x213528:58, C_SBDOSSIER 0x216eb0:20/:26, C_SBMPMAP 0x217538:17. The port routes the walks it
models through `FrontEnd::close_iris_page` (`CLOSE_IRIS_PAGE_TICKS = 10`, the block from the next
tick, lifted at the arrival); the draw code reads `close_iris_in_flight()` for the iris window.
The Back arms (the 0x5f pop) are NOT CloseIris and stay immediate.

**`Menu_EnableSounds(page)` 0x1f31f0** [M]: `Manager_SendMessage` case 0x44 calls it with the new
page's id when the page is found [M 0x21c858:267-268] — the flags are PER PAGE OPEN, not a boot
constant. The switch: 0x40000002/09/15 → 1; 0x40000005/1c/25/4b → 0xb; 0x40000022 → 0xd;
0x40000033 → 3; 0x40000036/38 → 7; 0x40000037/3a-3e → 8; default → 9. `Menu_PlaySound` 0x1f3278
gates MENU_SOUND 0/1/2/8 on bits 0/1/2/3 — so on the title and Main (flags 1) only the ✕ select
blips and □/○/△ are silent [M]. The port's `menu_enable_sounds(page)` runs in `enter_page`; the
boot value is P_LANGUAGE's default-arm 9 (the manager-create 0x43 sends a direct 0x44 for the
first page [M 0x21c858:252]). The ee_start snapshots' 0x0b [L ee_start 0x30d184] is the value a
MISSION inherits from the SP list (0x4000001c → 0xb [M]), not a boot value.

**The results order** [M]: `Mission_Update` 0x182c60 case 4's complete arm only sets
GameState+0x1c = 0x40000036, runs `ResetMap_LevelToLoad(LevelToEndTo, 0, 0)` and pushes the
60-frame fade (:72-75) — NO page opens; the results page comes up when the front-end map's load
tail runs `MenuManager_Create(GameState+0x20, +0x28)` [M 0x1d6360:160-163]. The fail arm is the
opposite: `MenuManager_Create(0x80000004, 0x40000042, …)` runs at once over the paused mission
(:77-80). The MP match end mirrors the complete arm: `MP_Update` 0x184bd0 case 4 sets GameState+0x1c
= 0x40000033 (P_MPDEBRIEFING [M the handler table 0x2029e0:434-437]), loads 0x7000048 and pushes
the fade (:62-78) — the debriefing opens at the load's create, not over the arena. (The
`MenuManager_Monitor` 0x21d508 case-3 create of 0x4000004b is the controller hot-swap monitor's
page, a different path.) The port models the no-page window with `page_open`/`up_page()`; the
load tail's `reopen_queued_page` lands the queued page.

**The boot card check** [M unless noted]: P_PARISENUM's 0x4c sets the message-box mode 4 and runs
`Menu_UpdateMessageBox(page, 0, 0)` [M 0x20a648:12-24] whose case 4 calls `Menu_EnumSaves(page, 1)`
+ `PS2_DoCardFlow` [M 0x1f7de8:53-58]; each 0x50 with `DAT_002e1b0a == 0` re-runs it [M
0x20a648:36-38]. `Menu_EnumSaves` state 0 arms `psiStartSaveEnum` [M 0x110fe0 — push card state
0xe, `GettingSaveGames = -1`] and state 1 polls `psiGetNextSave` [M 0x111038] until the card
machine lands the dir read. `HandlePS2CardState` 0x110b98 is the stack machine `PS2PollMemoryCard`
pumps every menu tick on the front-end map [M 0x21c3d8:45-47]: 0xe's sub 1 pushes GET_CARD_STATUS
(10) [M 0x110188:27-29], 10's sub 0 starts `sceMcGetInfo` and pushes the sync waiter [M
0x10e798:10-16], the waiter pops on completion [M 0x110b98 case 1], 0xe's sub 2 skips
CHECK_FOR_CARD2 in one-codename mode [M 0x110188:31-33], sub 3 sets the 0xb message state and
starts `sceMcGetDir` [M :37-43], sub 4 pops with the sync result [M :46-49]. The enum then
completes to state 7 [M 0x1f7b78:27] and `PS2_DoCardFlow`'s state>3 arm sets
`*(MENU_LS+0x3a) = DAT_002e1b0a = 1` [M 0x217bd8:68]; P_PARISENUM's next 0x50 sends the 0x44 to
P_ESTHERO lparam 1 [M 0x20a648:39-47]. The port's `ParisCardCheck` walks the same states with one
bounded loop per tick; the only unpinned number is the sceMc completion latency (`mc_ticks = 1`
[S unverified] — see the report's live checks).
