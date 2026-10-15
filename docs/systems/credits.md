# The credits roll — data and timing (P_CREDITS_Handler 0x2152b0)

Sources: ACTION.ELF decompile `work/decomp/ACTION.ELF/002152b0_P_CREDITS_Handler__FUcP9M_CONTROLUiUiii.c`,
`001f9a88_Menu_SetupCredits__FPUi.c`, `00215100_P_WINGAME_Handler__FUcP9M_CONTROLUiUiii.c`,
`002029e0_Handler_HandleMessage__FUcP9M_CONTROLUiii.c`, `001cad60_Txt_BindLabel__FUiSc.c`,
`00218890_Label_SendMessage__FP7M_LABELUiii.c`; ELF .data dumps. Port side:
`crates/nf-view/src/credits.rs` (model), `crates/nf-view/src/credits_data.rs` (the 578-line table),
`crates/nf-view/src/menu.rs` (Page::Credits + the AV row), tests `crates/nf-view/tests/credits_data.rs`.

## Where the credits page lives and who opens it

- `P_CREDITS_Handler` 0x2152b0 is the handler of menu page **0x40000030** [M Handler_HandleMessage
  0x2029e0:411-413]. `P_WINGAME_Handler` 0x215100 is the handler of page **0x40000053** [M
  0x2029e0:569-572] — the end-of-game page: on enter it sets the win flags (DAT_002e1c49/4a/4c),
  removes the menu-loop sounds and starts the background movie **0x073f0048** [M 0x215100:12-21];
  on each 0x50 tick it waits for `psiMovieFinished` (0x001082e8), restarts the menu-loop sounds
  once, and sends msg **0x44 with 0x40000030** — the page change to the credits [M 0x215100:23-49].
- AV Options (page 0x40000031) opens the same page from its Credits action row (port side:
  AV_ROWS[8], control 0x10000138, label Txt 0x198 — the ISO binds 0x198 as "Credits" [L
  USATxt.dat; test `the_av_credits_row_label_binds_the_credits_string`]). The menu def wires the
  row to the page change; the handler receives the opener as its `page_from` param_6.

## Where the credit lines come from

`Menu_SetupCredits(0x30d54c)` 0x1f9a88 (called from the 0x4c enter case [M 0x2152b0:73]) builds
**578 (0x242)** 12-byte entries on the stack, heap-copies 0x1b18 bytes (`Menu_Malloc(0x1b18, 0xc)`
+ `amemcpy`) once per session (guarded by `ptr_credits` 0x… — `Menu_InitCredits` 0x202420 clears it
on the 0x51 re-show so the next enter rebuilds [M 0x202420/0x2152b0:100]), and stores the count
**0x242** through the caller's pointer **0x30d54c** [M 0x1f9a88:3444-3450]. So 0x30d54c is the
count cell (the handler's `count_521`), not a data table — the data is code-built.

Entry layout (read by the handler at `credits_516 + i*0xc` [M 0x2152b0:121-161]):

| off | type | meaning |
|-----|------|---------|
| +0  | ptr  | left string — a `Txt_BindLabel(id, 0)` result, an ELF literal, the gap `" "` at 0x30d240, or NULL |
| +4  | ptr  | right string (same kinds) |
| +8  | u8   | left font kind |
| +9  | u8   | right font kind |
| +10 | u8   | wide flag (0 = two columns, else the left string spans the page) |

The strings come from three places:

1. **USATxt.dat labels** through `Txt_BindLabel(id, 0)` 0x1cad60: `id = bank<<24 | index`,
   `index = FixupTable[bank] + (id & 0xffffff)` [M 0x1cad60:13]. The table references 209 label
   slots: bank-0 ids 0x2a4-0x2b3 and bank-1 ids in the ranges 0x100021c-0x1000240,
   0x1000247-0x1000259, 0x1000259-0x1000281, 0x1000283-0x10002e0+, 0x1000312-0x100031a [M the
   parsed table]. The opening headings are the sequential bank-1 run 0x10002a5, 0x10002a6,
   0x10002a7 … [M 0x1f9a88:1457-1469].
2. **ELF literals** — 492 name strings in ACTION.ELF's .rodata/.data ("Mike Kaczmarek",
   "Francois \"KoiHZ\" Lafleur", …) [M 0x1f9a88's inline stores].
3. **.data strings** the decompile references by address [D ELF bytes]: 0x30d270 "EUROCOM",
   0x30d278 "Ryan Ho", 0x30d280 "Gia Luc", 0x30d288 "Olo", 0x307460 "Marcelo Vásquez",
   0x307698 "Maëllenn Lumineau", 0x3076c0 "Frédéric Aubrun", 0x3076d0 "David Gély",
   0x307820 "Luis Pinés", 0x307870 "Enrique Sánchez-Real", 0x307970 "Mattias Göthe",
   0x308220 "Miché Bond, Tammi Cave, Eva LeTourneau," (CP-1252).

The gap `" "` is the 1-byte string at **0x30d240** [D ELF byte 0x20]; a NULL string sends msg 0x18
with 0, which binds the blank string at 0x30d5a8 [M Label_SendMessage 0x218890:44-47].

Extraction: the port's table was parsed from the function's own memset/store sequence (entry k at
frame offset 0x1bd0 − 12k). All 1156 slots (578×2) are accounted for: 209 label + 492 literal +
12 .data + 241 gap + 202 NULL [test `every_string_slot_is_accounted_for`].

## Line kinds and their fonts

The kind byte picks the format string the handler sends to the item with msg 0x23 [M 0x2152b0:122-147;
`Label_SendMessage` case 0x23 sprintf's it into the label's format buffer 0x218890:100-104]:

| kind | pointer | bytes [D ELF] | reading |
|------|---------|---------------|---------|
| 0 (names) | 0x30d570 | `ff 02 fe 03` + sprintf tail 0x30d5b8 `fc 64 fb 64` | = the default label format DAT_003096c0 [D — byte-identical] |
| 1 | 0x30d560 | `ff 01 fe 03` | face 1, centred |
| 2 (headings) | 0x30d568 | `ff 03 fe 03` | face 3, centred |

The escapes are the menu text renderer's own tags (`ff 0N` = the face, `fe 03` = centred — the same
tags menu.rs documents for DAT_003096c0). Kind distribution in the data: left 0×475 / 2×89 / 1×14,
right 0×516 / 2×59 / 1×3; wide rows 270, two-column rows 308 [M the parsed table].

Kinds as line kinds: kind 0 = name lines (the big count), kind 2 = section headings (also the
kind-1/wide EUROCOM-and-friends lines at the roll's opening: entry 53 "EUROCOM" is kl=1/kr=1
two-column [M entry 53]), kind 1 = the three highlighted roles. The kind-0/1/2 → face mapping is
mechanical from the pointers; which face id maps to which font file is draw-side (the main agent).

## Positions (the 512×448 menu space)

Per entering line [M 0x2152b0:108-163]: both list controls 0x10000213 (left column) and
0x10000214 (right column) get the line in list slot `index_519` (cycling `% 0x1a` = 26 slots,
:162); the item's y (+0x72) = **0x200 (512)** (:117-119). Two-column lines (`w == 0`): left item x
(+0x70) = 0x10, width (+0x74) = 0x100; right item x = 0xf0, width = 0x100, text = the right string
(:148-156). Wide lines: left item x = 0x10, width = **0x1e0** (so it spans 16..496, centred on
256); the right item hides (msg 0x2b/1) (:157-161).

## Scroll speed and timing (fixed 60 Hz)

[M 0x2152b0:103-195] — the 0x50 tick in order:

1. `frame_518 += 1`.
2. Every **14th frame** (`frame % 0xe == 0`) while `name_520 < 578`: the next line enters (the
   item setup above), then `index_519 = (index_519 + 1) % 26`, `name_520 += 1`.
3. Music fade once `578 − 50 < frame/14`: `SFXMusicSetVolume((count×14 − frame) × music_vol/700)`
   clamped ≥ 0 (:165-168) — a linear fade over the last 50 lines (700 frames), reaching 0 the
   frame the last line enters. `music_vol_524 = SFXMusicGetVolume()` was captured on enter (:89).
4. End once `578 + 18 < frame/14` (:169): `fading_517 = 1`; a **delayed 0x13 (19 frames)**
   message to the manager — **msg 0x44 with 0x40000002** (the main menu) when `page_from` was the
   win page 0x40000053, else **msg 0x5f** (pop back to the opener, e.g. AV Options 0x40000031);
   plus a delayed msg 0x68, and an exit process on control 0x10000215 with params
   {0xe1, 0xff, 0} over 0xf frames [M the Process_Create call :179-183; its visual effect is a
   page fade [S]].
5. Every frame (including entry frames): ALL 26 slots of both columns scroll **y −= 2** (:186-194)
   — so a freshly entered line shows y 510 on its enter frame and travels 512→0 in 256 frames;
   with entries every 14 frames a line is 18 slots behind the newest, inside the 26-slot window.

Derived frames (all in the port as consts [M-derived]): line 0 enters at frame 14; the last (578th)
at 578×14 = 8092; the music fade starts at (578−49)×14 = 7406; the fade trigger at (578+19)×14 =
8358; the delayed page change at +19 = **8377** (~2 min 20 s after enter).

## Sounds and movies

- Enter (0x4c): `SFXRemove(0, 0x1d7)`, `SFXRemove(0, 0x470)` (the menu-loop sounds), `menu_movie = 0`,
  **`SFXStartMusic(0x27, 0)`** — the credits music track [M 0x2152b0:89-94].
- Leave (0x4d): `Menu_KillCredits` (empty, 0x202428), `SFXStopMusic()`,
  `SFXMusicSetVolume(music_vol_524)` (restore), then `SFXStart(0, 0x1d7)`, `SFXStart(0, 0x470)`,
  `menu_movie = 1` [M 0x2152b0:54-64].
- The win page before the credits plays movie **0x073f0048** and pauses/unpauses the sound system
  around it (`SFXUnPause`/`SFXUnPauseAllStreams` when the movie finishes) [M 0x215100:39-47].
- No input cases exist in the handler — the roll cannot be skipped [M 0x2152b0's switch].

## Background alpha

On enter the background control 0x1000023c gets msg 0x1a with **0xff** when `GameState[0x52] == 0`,
else **0x404040ff** [M 0x2152b0:83-88]. `GameState[0x52]` is the level-bonus/medal flag
`Menu_SetLevelBonus` 0x1ffa10:68 sets, persisted in the PTP save block [M 0x1c8198:57/0x1c8560:62]
(the save's bit pushed by LS_MakeMission 0x1cd720:64).

## Port shape

- `crates/nf-view/src/credits_data.rs` — `CREDITS: [CreditLine; 578]` (`Str::Label(u32)` /
  `Str::Lit(&'static str)` / `Str::Space` / `Str::Null` + `kl/kr/w`) and `KIND_FORMAT`.
- `crates/nf-view/src/credits.rs` — the timing model `CreditsRoll` (the handler's statics as
  fields; `tick()` returns the exit), getters `visible()` / `slot_y()` / `music_volume()` /
  `lines_entered()`; consts for every number above. State + getters only — no drawing.
- `crates/nf-view/src/menu.rs` — `Page::Credits` (id 0x40000030), `FrontEnd::credits` +
  `credits_from`, `open_credits(from_page, music_vol, bonus_flag)`; the tick integrates the roll
  and routes the delayed exit (ToMain → Page::Main, PopBack → the opener); the AV Options Credits
  row (0x10000138) confirms into `open_credits`. Drawing the roll is left to the main agent
  (`fe.credits.visible()`).

## Live checks for the main agent

- [ ] PCSX2, AV Options → Credits: confirm the roll's first visible rows are the label-0x10002a5/6/7
      headings (wide, centred) and that entry 53's "EUROCOM" row is two-column kind-1 — pins the
      table order against the real scroll.
- [ ] The y spacing: two adjacent entries 14 frames apart should sit 28 units apart at any moment
      (−2/frame). A screenshot with the debugger's frame counter would pin the entry period.
- [ ] GameState[0x52] = 1 (a saved game with a medal) → the credits background's alpha word reads
      0x404040ff on the 0x1000023c control (address 0x30d54c region watch: the msg 0x1a send at
      0x2152b0:87).
