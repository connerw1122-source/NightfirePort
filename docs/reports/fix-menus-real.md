# fix-menus-real — the front end rebuilt against the re-parsed menu-def (2026-10-14)

Branch `s/fix-menus-real`. Files: `crates/nf-view/src/menu.rs`,
`crates/nf-view/tests/menus_flow.rs`, this report, `docs/WORKLOG.md`. **No shared files touched**
(`drone.rs`/`scene.rs`/`player.rs`/`lib.rs`/`hud.rs` are byte-identical to master).

The evidence base is a fresh walk of the menu-def tag stream (kind-8 entry `08000002` in
`07000048.bin`, walked exactly like `MenuManager_Create` 0x21b908) over the pages the driver named,
plus two decompile passes (the label/text pipeline and the page handlers). All page-data citations
below are from that dump (2026-10-14); [M] = decompile function + address, [L] = the parsed page
data / ELF .data.

## Done

1. **The centring bug behind "left-aligned / runs off"** [M]. The overlay's text was left-aligned
   from its anchor: `text_scaled`'s pen starts at x, and `quad()` maps both corners (the anchor is
   only an origin shift — glyphs draw left-edged at their pen x), so every "centred" string ran
   right from its x. The game centres menu labels by DEFAULT: `Label_Init` 0x218638:53 stamps
   `DAT_003096c0` = `ff 02 fe 03` (SerpLight + centre) on every menu label, and `__Font_DrawText`
   0x1eee48:69-71 applies flag 2 as `x -= width * 0.5`. The port now centres each row by its
   measured ink (`[p0.x, plast.x + gwlast]`); the footer hint bar lays the whole row out as ink
   items (text segments + the specialchar icons) and centres at the bar's `ctrl.x + ctrl.w/2` =
   320 [M `Label_Update` 0x218cb8:177-182]. This is the driver's "DESCRIPTION … CENTRED … the port
   left-aligns it" and "FOOTER … centred … the port left aligns" in one mechanism.
2. **Headers** [L the pages' header controls (50,40,376×17) on 0x40000025/0x40000023/0x4000001c/
   0x4000002b/0x4000003a]: the label centres in its own band at (238, 48.5) [M Label_Update
   :177-182/:202-206] in the control's own colour (msg-0x1b 0x7d6d59ff → 0x645a49ff, the rewrite at
   0x21b908:238-243). The old draw centred AT x=50 — half the header hung off-screen. The stylised
   header look in the captures is the game's own SerpLight menu font (the port parses the real
   FONTSerpLight from ACTION.ELF — same data the game uses [M FontTable 0x2dd8b0]); the header is
   LBL text, not art [L no msg-0x24 on the header controls].
3. **The wheel rows** [L the parsed row controls: slot 0 (323,123,317×17), slot 1 (333,152,307×17),
   slot 2 (304,182,336×17), slot 3 (333,211,307×17), slot 4 (323,241,317×17)]: each row's label
   centres at `ctrl.x + ctrl.w/2` (the old draw used x as the centre — ~170 px left of the game's),
   vertically centred [M Label_Update :202-206]; the static alphas 0x40/0x80/0xff come from the
   rows' own msg-0x1b/0x1c payloads [L]; locked rows go grey 0x484848xx [M 0x1f8388:210-234].
4. **Main menu rows** [L page 0x40000002's rows (167,262/294/326,306×28)]: the selected row draws
   the msg-0x1b bright cream (0x7d6d59ff → 0x645a49ff) and the others the msg-0x1c dark orange
   0x73330f80 — the port had them inverted (orange selected, cream others). The rows centre at
   167+306/2 = 320 [L]; the description binds the runtime labels and centres in its plate.
5. **Codename page** [M `Menu_EnumSaves` 0x1f7b78:58-66, `Menu_PutCodenamesIntoControl`
   0x1f79c0:15-23]: two rows — "New Codename" (Txt 0x29b) and "Play without saving" (Txt 0x10002ff);
   the ring preview follows the cursor (0x300013b create / 0x3000135 default [M
   `Menu_UpdateCodenameWheel` 0x1f89f0:80-99]) and the row's description binds (0x1000004/0x1000003).
   The confirm box stays [M `C_SBNFCN` 0x20b390:19-21 — index 1 pops `Menu_CreateOptionBox(0x10000bb,…)`].
6. **Dossier** [L the wheel table 0x2dfcd0; M `C_SBDOSSIER` 0x216eb0:48-49]: four rows — Records /
   Rewards / Gadgets / Weapons (0x280/0x281/0x282/0x19c) with per-row previews
   0x300019b/0x300019c/0x30000c4/0x30000ee and descriptions 0x100000a-d. The mission-briefing body
   the port used to draw is gone.
7. **Records** (new page 0x4000003a) [L the PAGE record parsed; M `P_DSRECORDS` 0x217248:11-13,
   `C_RBDSRECORDS` 0x20bf00:79-92]: the disc's own table — header 0x280, hint 0x49e, the mission
   list (the sp_level names, `Menu_AddItemsToControl`), the three score rows (0x302/0x165/0x305 at
   (83,169/202/235) with their value controls at (375,…)), the medal image (435,282,128×64) TEX
   0x0300013d and the medal label 0x307. Headless values draw as 0 (no save data).
8. **Title prompt** [L page 0x40000009 control 0x100000fa]: the page's own gold 0x7f621f (the old
   constant 0xfec43e contradicted its own citation), centred at the control's centre (320.5, 317).
9. **Flow**: the codename page gained its real row 0 (Confirm on it is the game's name entry, not
   ported — reported); Dossier Confirm row 0 opens Records; Records walks the campaign and Back
   returns.

Tests: `menus_flow` **26/26** — the footer/header/wheel centring (ink rows measured per-quad from
the drawn frame, the full-screen plates excluded), the main-row colours, the codename rows +
per-cursor previews, the dossier wheel + the Records page, the prompt colour, and all pre-existing
flows (the codename navigation gained a Down). Every new test cites its page-data/decompile
evidence; sisters pin the negative sides (e.g. the create preview is gone at cursor 1, every text
row inside the 512×448 screen).

## Suite

- Baseline before the work (origin/master tree): **906 passed, 3 failed, 27 ignored (18 min)** —
  the 3 allowed `disc_golden` only, no TIMEOUT/CRASH.
- Final tree: run in progress at report time — see the WORKLOG tail / the commit message for the
  recorded totals. (`tools/test/suite.py` with `NF_ISO` = the onfoot ISO.)

## What's left

1. **The backdrop** — unchanged from fix-menus3's finding: the pages' background IS the aux FMV
   (main 0x07330048, list pages 0x07350048 [L the PAGE records]); the port plays it on the ONE
   movie plane. The main agent's near-black renders are the plate standing in — verify with the
   retail PSS present and `--movie-frame` past the first GOP (frame 0 is the fade-in).
2. **The selected-row dark bar** ("white on a dark bar from the ring to the right edge") — the
   wheel rows are plain LBL controls in the page data (no bar sprite); the bar must be runtime art
   (a sprite `Menu_UpdateWheel` repositions, or baked into the panel texture 0x030000b3). Needs a
   live check (below).
3. **The gold rule under the header band** — no code draws it and no control carries it on the
   parsed pages; likely part of the panel/backdrop art. Live check.
4. **The msg-0x2d controls** — every page carries a fullscreen (0,0,640×480) control with TEX
   0x03000042 and msg-0x2d params like (0x00080017, 0x00020002) at f1 20 (the earliest layer), the
   missions one coloured 0x070202ff — a near-black wash (plausibly the fade). The port ignores
   them; decoding msg-0x2d (label-send case 0x2d) would pin their role.
5. **The per-slot arrow controls** ((312,48,16×32) msg-0x2e next to the header) — undrawn.
6. **Loading screen** — `ResetMap_GenLoadScreen` 0x1d5de0 (labels 0x3c1 "LOADING", 0x2000054
   "OBJECTIVE", 0x2000055 "HINT", art via `ResetMap_LevelCode2Img` 0x1d5258, cached as VRAM
   "loading blobs") — NOT a menu-def page; needs its own draw path keyed off the load state.
7. **Codename "New Codename"** opens the game's name-entry page — not ported (Confirm is a no-op).
8. **Pause/Score tabs** — owned by 184-fix-pause-menu; untouched here.

## Render commands for the main agent (all pages)

```
./target/release/nf-view $NF_ISO 07000048 --page title      --frame 60 --movie-frame 12 --shot t.png
./target/release/nf-view $NF_ISO 07000048 --page main       --frame 60 --movie-frame 12 --shot m.png
./target/release/nf-view $NF_ISO 07000048 --page codename   --frame 60 --movie-frame 12 --shot c.png
./target/release/nf-view $NF_ISO 07000048 --page codename   --frame 60 --movie-frame 12 --cursor 1 --shot c1.png
./target/release/nf-view $NF_ISO 07000048 --page difficulty --frame 60 --movie-frame 12 --shot d.png
./target/release/nf-view $NF_ISO 07000048 --page missions   --frame 60 --movie-frame 12 --shot s.png
./target/release/nf-view $NF_ISO 07000048 --page missions   --frame 60 --movie-frame 12 --unlock-all --shot s2.png
./target/release/nf-view $NF_ISO 07000048 --page dossier    --frame 60 --movie-frame 12 --shot do.png
./target/release/nf-view $NF_ISO 07000048 --page records    --frame 60 --movie-frame 12 --shot r.png
```

What each should show vs PCSX2: the footer hints CENTRED at the bottom with the PS2 glyphs; the
header inside its (50,40,376×17) band (not half off-screen); the wheel rows on the RIGHT of the
ring with the selected row centred-widest at x≈472 (640-space) and the neighbours dimmed; the main
rows bright-cream selected / dark-orange others; codename listing New Codename / Play without
saving with the per-row ring art; the dossier wheel Records/Rewards/Gadgets/Weapons; Records as
the score table with the medal image; the title prompt in gold 0x7f621f centred.

## Live checks for the main agent

PCSX2 (spot / action / expected result):
1. Main menu: pause the frame — the backdrop is the 0x07330048 FMV (its streaks + the orange panel
   + Bond tuxedo + 007 ring are IN the movie), the rows are LBL text on top.
2. Select Codename: is there a dark horizontal bar behind the selected row? If yes, note its rect
   (640-space) and whether it animates on cursor move (a runtime sprite vs baked panel art).
3. Select Codename / Difficulty / Single Player: is the thin gold rule under the header part of
   the backdrop movie or a page sprite? (freeze the FMV if possible).
4. Missions: the footer hint — does it read "… □ Dossier …" from LBL 0x218, or does the handler
   rewrite it (the parsed LBL is 0x218; the port previously used 0x208)?
5. Records: with a save, the three value controls (375,169/202/235) show numbers, the medal image
   0x0300013d swaps per medal (0x300013d-0x3000140 [M 0x20bf00:134-144]).

nf-view (`--page` + `--movie-frame 12`):
- The nine shots above; check the footer centre (±8 px of 256 HUD), the header inside its band,
  the wheel row centres at the parsed x+w/2, the main-row colours, the codename rows, the dossier
  wheel, the Records table.
