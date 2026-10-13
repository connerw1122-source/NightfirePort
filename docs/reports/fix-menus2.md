# fix-menus2 — the front end matches the real game

Branch `s/fix-menus2`. Facts tagged [M] code (function + address), [D] disc, [L] EE snapshot / PCSX2, [S] suggested.

## What's done

The port's screens are now built from the game's own data, in the game's own order:

- **The disc's page data is decoded and used.** The kind-8 LevelBin entry `08000002` inside `07000048.bin`
  (FILES.BIN) is `MenuManager_Load` 0x21de88's buffer; `MenuManager_Create` 0x21b908 walks it as a tag
  stream. I derived the exact chunk layouts from the decompile (BIN_GetDWord/Word/Byte order): PAGE −0x10
  = 17 bytes {u32 page id, u32 file hash (0x80000002 — `bootup` sets GameState+0x20 to it [M 0x1c7e78]),
  u32 aux background-movie code, u8 kind, u32 size}; CONTROL −0x07 = 30 bytes {name, u8, u8 type, u16
  x, y, w, h, u16, u32, u16, u32, u32}; NAME −0x08 32-byte string; SCRIPT −0x09; MSG/MGRMSG −0x0a/−0x0d
  {u8 msg, u32, u32}; KMSG −0x0c; KFRAME −0x0b; SET −0x03; COMPONENT −0x04 {u16 count, 3×u16, u32
  texture hash}; SPRITE −0x05 32 bytes; SKIP −0x02; END −0x0e. Parsed: **56 pages, 1003 controls, clean
  END with 19 spare bytes**. MSG 0x18 = Txt_BindLabel, 0x24 = texture (0x03xxxxxx), 0x1b/0x1c = colours
  (0x7d6d59ff → 0x645a49ff rewrite [M 0x21b908:238-243]). The menu coordinates are a **640 × 480 space**
  (every page's root control is (0,0,640,480)); the overlay maps it onto the 512 × 448 screen
  (×512/640, ×448/480). The full dump drives every layout constant in `crates/nf-view/src/menu.rs` —
  each page's draw block cites its controls; `docs/systems/frontend.md` documents the format.
- **Page identity, settled by the data** (the old report's guesses were wrong): the real main menu is
  menu-def page **0x4000002e→0x40000002** (rows NightFire/Multiplayer/Codenames at (167,262/294/326));
  **0x40000025 = Select Codename** (header Txt 0x1f2, five wheel rows, ring emblem 0x030000bf +
  preview 0x030000b6, hints 0x216 "~A Select ~Y Create ~V Scroll ~B Back", confirm plate (50,301,540×113));
  **0x40000023 = Select Difficulty** (header 0x1f8, emblem 0x030000a5, wheel with the selected row's full
  alpha and the neighbours' 0x80/0x40 [M the rows' colours]); **0x4000001c = Single Player** (header
  0x14c, preview 0x03000084 over the ring, confirm plate, LOADING 0x67e); **0x4000002b = Dossier**
  (header 0x27f, emblem 0x030000c4). 0x40000009 = the visible title (full-screen art control 0x10000196
  TEX 0x03000074, prompt control 0x100000fa at (131,309) colour 0x7f621fff = yellow, legal plate
  (50,332,540×108)); the boot page 0x40000034's seven controls are language FLAGS (0x030000c9-cf = the
  Union Jack etc. [D tex-export]). P_MAIN 0x4000002e's old identity was actually a multiplayer-options
  page (its labels bind "Radar"/"Auto Aim").
- **The order is the driver's observed one [L PCSX2]**: Title → START → the intro movie (the game's
  state 6 [M P_INTRO 0x209e88 plays background FMVs 0x07370048/0x07380048 then opens the next page; the
  port models the state with a skippable countdown because the trimmed ISO's /MOVIES/30_FPS PSS files
  are zeroed]) → Main 0x40000002 → NightFire → Select Codename 0x40000025 → "Play without saving" pops
  the game's confirm box (Txt 0x10000bb — the driver's exact transcript string) → ✕ continues → Select
  Difficulty 0x40000023 → confirm → Single Player 0x4000001c → confirm starts the mission. Backs walk up
  the chain; □/Tab/E opens the Dossier from the list.
- **Every string comes from the language bank** (`Lang`, `Txt_LoadLanguage` 0x1caad0 / `Txt_BindLabel`
  0x1cad60 in nf-formats/hud.rs): main items 0x28/0x149/0x14b, descriptions 0x1000000-2, codename header
  0x1f2 + rows 0x29b/0x10002ff, difficulty 0x1f8 + 0x14d/0x14e/0x14f ("Operative"/"Agent"/"00 Agent" —
  the old port's names/order were wrong) + descriptions 0x1000006/7/8, list header 0x14c, hints
  0x3c3/0x54e/0x216/0x218/0x208, legal 0x375, prompt 0x10001eb, Dossier 0x27f. All pinned in
  `the_menu_strings_come_from_the_language_bank`.
- **The campaign is `sp_level` 0x2df2e0** [M 12 × 0x18 records; C_SBNFMAP 0x20b8d8:21 reads +0xc/+0x10]:
  the port's 12 rows carry each record's own name label, description label and level code (bank-3 Txt:
  "Paris Prelude", "The Exchange", "Alpine Escape", **"Enemies Vanquished"** — the old hard-coded list
  said "Enemies Vanish" — "Double Cross", "Night Shift", "Chain Reaction", "Phoenix Fire", "Deep
  Descent", "Island Infiltration", "Deep Water", "Countdown"), cross-checked record-by-record against
  the EE snapshot (`the_campaign_is_sp_level_on_the_disc` [L]). Unlock rule: fresh profile = row 0 only
  [S — no writer of the other flags was found; the snapshot's save has all 12], completing row *i*
  unlocks *i+1* [M Menu_SetLevelBonus 0x1ffa10:63-65], locked rows draw dimmed and can't be scrolled to
  or picked [M C_SBNFMAP 0x20b8d8:18-20], `--unlock-all` = the game's `menu_unlock_everything` 0x30d2a7.
- **The sprites are the game's own.** `MENU_TEXTURES` (menu.rs) lists the ten hashes the pages draw —
  title art 0x03000074 (512×512, the Bond/red-orange art with the baked-in logo [D tex-export]),
  panel 0x030000b3, ring 0x030000bf, previews 0x03000084/b6/c4, difficulty emblem 0x030000a5, logo
  0x0300016d, stamp 0x0300014c, button atlas 0x03000042 — and `Scene::build` now loads them into
  `hud_textures` beside the HUD set. The hint bars parse the game's `~A/~B/~X/~Y/~V/~H/~S` tokens and
  draw the atlas's own tiles (□ (0,0), ○ (0,16), ▲ (16,0), ▼ (32,0), ◀/▶ (48,0)/(48,16)) — the □ Dossier
  hint renders the game's square tile, the ↕ Scroll the game's up/down arrows.
- **`--shot` draws the menu overlay** (lib.rs `front_end_menu`): a front-end boot shot now renders the
  title page by default; `--page <start|main|codename|difficulty|missions|dossier>` jumps to a page,
  `--cursor N` picks the row, `--unlock-all` unlocks the list. The F10 screenshot path draws the menu
  when the front end is up too.
- **Two real bugs fixed on the way** (both cited in the tests):
  1. `Lang::parse` handed the bank's raw bytes to `str::from_utf8`, and every string containing a
     CP-1252 byte (©, ™, é…) came back **empty** — this was fix-menus' unresolved legal line 0x375
     (docs/reports/fix-menus.md "What's left" 7). The bank is now CP-1252-decoded at parse time with the
     offsets remapped, and 0x375 resolves ("NIGHTFIRE Interactive Game … © 2002 Electronic Arts Inc. …").
  2. The loader's movie-wrap check `level & 0xff0000 != 0` mis-fired on the campaign's driving codes
     (0x09xxxxxx are real `ResetMap_LevelToLoad` codes — sp_level's Paris Prelude/Alpine Escape/Enemies
     Vanquished rows carry them [L ee_snapshot]); it now tests the 0x71/0x73 prefix at bits 20-27
     [M 0x1d6360 "the level still carries the 0x71/0x73 prefix"].
- `GameFlow` gained the state-6 movie countdown (`play_movie`, popped by `main_tick`, the skip action's
  behaviour per GameFlow_Main case 6 [M 0x1c98c8]).

## How it was verified

- 12 headless tests in `crates/nf-view/tests/menus_flow.rs` (all green, no ISO needed except two):
  boot → title through the real load path; **the page sequence with the game's page ids** (title → movie
  → main → codename + confirm box → difficulty → missions → launch through the 0x71 wrap, the movie
  state and the back-map); the unlock rule (locked rows unscrollable/unpickable, completion unlocks the
  next, `--unlock-all`); pause resume/restart/two-step quit; results restart/continue/quit; cursor wrap
  + backs + the Dossier side-action; the attract timer; **the overlay drawing every page with the game's
  sprites — the per-page texture hashes are pinned (numbers only)**; the label ids per page bound from
  USATxt.dat on the disc (sisters: `the_menu_strings_come_from_the_language_bank`,
  `the_pause_labels_the_handler_uses_resolve_from_the_disc`, `the_campaign_is_sp_level_on_the_disc`).
- Full suite: `cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view --
  --test-threads=1` — see the count at the bottom (the only failures allowed: the 3 `disc_golden`
  tests on the slim ISO).
- Texture/asset evidence: `nf-extract tex-list` + `tex-export` over 07000048.bin (291 textures; the ten
  menu hashes' sizes/contents eyeballed from the exported PNGs); the menu-def parsed with a throwaway
  reader against `docs/formats/level-bin.md`'s container layout (the walk matches the archive's exact
  byte total).

## Render commands for the main agent (side-by-side with PCSX2)

```
nf-view Nightfire-onfoot.iso --shot /tmp/menu-title.png --page start
nf-view Nightfire-onfoot.iso --shot /tmp/menu-main.png --page main
nf-view Nightfire-onfoot.iso --shot /tmp/menu-codename.png --page codename
nf-view Nightfire-onfoot.iso --shot /tmp/menu-difficulty.png --page difficulty
nf-view Nightfire-onfoot.iso --shot /tmp/menu-missions.png --page missions --unlock-all
nf-view Nightfire-onfoot.iso --shot /tmp/menu-missions-locked.png --page missions
nf-view Nightfire-onfoot.iso --shot /tmp/menu-dossier.png --page dossier --unlock-all --cursor 1
```
Expected: title = the red/orange Bond art full-screen + yellow "Press the START button" + the small
legal lines; main = the three rows NightFire/Multiplayer/Codenames (selected orange-red) + the
description line + "↕ Scroll ✕ Select" at the bottom; codename/difficulty/missions = the header at
(50,40), the orange panel with the ring emblem at (114,118), the five-slot wheel (selected row centred,
neighbours dimmed), the description/confirm plate, the hint bar; missions = "Single Player" + the
campaign names + locked rows dimmed. `nf-view <iso>` (windowed) still boots to the title; mouse +
keyboard unchanged (arrows/Enter/Esc, Tab/E = the □ side action).

## What's left

1. **The menu background FMVs** (each page's aux code: 0x07330048 main, 0x07350048 the list pages) are
   zeroed in the trimmed ISO — the port draws the dark plate; with a full ISO the PSS files exist and
   the pages would need the movie player behind the widgets.
2. **The ~A (✕) glyph**: the game's button-icon sprites for the cross and the triangle faces are not in
   the 0x03000042 atlas (it holds the square/circle faces and the arrows). The port draws the font's X
   for ~A and the atlas's ▲ for ~B. Live check: breakpoint the text renderer's ~ handling (msg 0x18
   text pass) with the pause hint up, log the sprite/texture it draws per token.
3. **Per-mission preview images**: the list draws 0x03000084 (the Alpine-Escape ring preview [D]) for
   every row; the game swaps a preview per mission — the 12 ring-masked preview textures need mapping
   (likely by sp_level record order among the archive's ring textures).
4. The wheel rows' runtime labels are rendered from `sp_level`'s own labels (the game fills them via
   messages); the codename page's stub shows only "Play without saving" (per the task) — the save-file
   integration is 210-fix-progress's.
5. The mission launch uses sp_level's level codes verbatim [M C_SBNFMAP] — the five 0x09xxxxxx rows are
   the DRIVING maps, which this onfoot port can't play; the load path will try to open e.g.
   09000001.bin (not in the trimmed FILES.BIN) and show a load error instead of silently loading the
   wrong map. A foot-level fallback per row is a product decision, not a decompile fact.
6. Attract demo, options page, multiplayer — unchanged stubs (fix-menus' report still applies).

## Live checks for the main agent

1. **Hint glyphs**: on the mission list, the hint bar should read "✕ Select □ Dossier ↕ Scroll △ Back"
   — compare the rendered icons' shapes with PCSX2's and tell me what the ✕ and △ faces look like
   (sprite sheet + uv if you can grab them with fieldprobe).
2. **The stamp control 0x10000224**: `MenuManager_Create` forces it to (340,198) size 255×255 [M
   0x21b908:305-314] — on PCSX2's codename/difficulty/list pages there should be a large soft watermark
   around centre-right; confirm the texture (0x0300014c) is what shows.
3. **Wheel row alpha**: the selected row full-bright, ±1 rows ~50%, ±2 rows ~25% [M the rows' msg-0x1b
   colours 0x7d6d59ff/0x80/0x40] — eyeball against PCSX2.
4. **The title's legal line**: four dim cream lines starting at (50,332) — confirm the line count and
   that the port's CP-1252 © renders like the game's.

## ntfy

(sent after this report is pushed; see the queue finish log if it failed)

## Suite count

nf-formats: 61 passed + the 3 allowed `disc_golden` failures (bigf × 2 + directory tree, the slim ISO).
nf-view: **379 passed, 0 failed** (12 of them the new/rewritten menus_flow tests). nf-game/nf-audio are
untouched by this branch (the only shared file changed is nf-formats' `Lang::parse`, whose CP-1252
decode only turns previously-empty binds into strings; nf-game's ASCII weapon names are unaffected).
