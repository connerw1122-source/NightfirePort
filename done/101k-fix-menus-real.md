## Your task: `menus-real`, the front end: make every page match the real game

The user on the front end (results 1013-1825): "This menu is so far from finished. None of this looks right." The main
agent drove the real game in PCSX2 through every page (2026-10-13) and rendered the port's pages with
`nf-view <iso> 07000048 --page <start|main|codename|difficulty|missions|dossier> --cursor 1 --frame 60 --shot x.png`.
The real pages, described (no images may be committed), and what the port gets wrong:

ALL PAGES (4:3, black side bars on wide windows):
- The BACKGROUND is missing everywhere. Real: an orange/red animated backdrop (streaks of light, sparks) behind every
  page; on the main page a big orange curved panel (a lens shape, lower edge an arc with a gold rim) with Bond in a tuxedo
  holding a pistol on the right and the 007 gun-barrel ring with a globe above the "007 nightfire" logo on the left;
  the other pages show the streaky backdrop with Bond's silhouette faint behind. The port draws a near-black page.
- The PAGE TITLE ("Select Codename", "Select Difficulty", "Single Player", "Dossier") is in the game's bold ITALIC
  header font, top left, with a thin gold rule under the header band; the port uses the plain body font.
- The FOOTER button hints use the PS2 glyphs (blue X, pink square, multi-arrow D-pad, green triangle) centred at the
  bottom; the port draws tiny dots, left aligned.
- The DESCRIPTION text is CENTRED in the lower band and wrapped inside it; the port left-aligns it and lets it run off.
- The list: the selected row is white on a dark bar that runs from the ring to the right edge; the others are dim and
  smaller above/below, as a rolling list (two above, two below).
TITLE ("Press the START button"): yellow prompt and the legal text CENTRED and wrapped under the logo; the port's
  text is left-aligned from the middle and runs off the right edge; the port's title art sits left with black around.
MAIN: "NightFire / Multiplayer / Codenames" in the large bold italic font, centred under the logo, the selected one
  white, the others dark orange; the description below ("Start your 007 adventure." / "Test your skills against other
  agents." / "Access Options and create your saved game profile."); the port draws small plain text with a yellow
  highlight and no art.
CODENAME: the list is "New Codename / Play without saving / BOND" (saved profiles), the ring shows the blue "007" logo;
  the port lists only "Play without saving" and shows a level picture in the ring. Choosing "Play without saving" opens
  a translucent dark message box over the page: "You won't be able to save your progress using this default profile. /
  This controller style is a traditional one-thumb setup... / Press X to continue or triangle to cancel."
DIFFICULTY: Operative / Agent / 00 Agent; the ring shows a green radar screen with Bond's face; the descriptions
  ("For trainee Agents. Enemies are slower and weaker, and deal less damage. In-game hints are provided." etc.).
MISSIONS ("Single Player"): the ring shows the mission's picture; locked missions are dim and show "Continue playing to
  unlock this mission."; footer adds "Dossier" (square).
DOSSIER: a menu "Records / Rewards / Gadgets" with a green briefcase in the ring ("View your highest scores and the
  best medals you've earned."); Records opens a table page ("< Paris Prelude >", High Score / Bond Moves / Upgrades
  Unlocked, footer "Change / Back"). The port's Dossier shows a mission briefing with a watch instead.
LOADING (after choosing a mission): left Bond-with-hand-on-chin art over a chandelier, three framed pictures in a
  column, a gold arc and the logo with a map of Europe on the right; "LOADING" with six dots under it; at the bottom
  "OBJECTIVE <first objective>" and "HINT <a hint>" in yellow labels.
PAUSE (in-game, Start; 184-fix-pause-menu owns it but check it against this): a wide orange rounded panel centred over
  the frame with four tabs across the top "MISSION / OBJECTIVES / CONTROLS / SCORE" (D-pad left/right "Next"); MISSION:
  CONTINUE / RESTART / QUIT with a yellow arrow cursor and an orange silhouette of a woman with a gun on the right; QUIT
  asks "QUIT: ARE YOU SURE?" YES / NO; OBJECTIVES: the list with "X Hint"; CONTROLS: the button layout "Classic Bond",
  "Y-Axis Normal"; SCORE: a Category / Current / Target table (Bond Moves, Opponents Dispatched/Subdued/Surrendered,
  Accuracy Rating, Health Remaining, Time, 007 Bonus).

Find each element in the page data (`MenuManager_Create` 0x21b908's tag stream, entry 08000002 in 07000048.bin, the
sprite/texture and font tables the pages reference, the backdrop's animated layer) and draw it. Work page by page,
title first. Tests: per page, the element list the port draws (sprite hashes, font ids, text rects, alignment flags)
equals the page data's (numbers only); text rects lie inside the screen and are centred where the data says centred;
sister assertions. List the `--shot` commands per page.

Files: `crates/nf-view/src/{menu,hud}.rs`, `crates/nf-view/tests/menus_flow.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/menus-real`.

---
claimed-by: sandbox1 at 2026-10-14T03:35Z

---
## Report (done, 2026-10-14T05:06Z)

Closed by the driver: the report is on `s/fix-menus-real` (docs/reports/fix-menus-real.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T05:24Z)


---
## Report from the worker (done, 2026-10-14T05:24Z)


---
## Report from the worker (done, 2026-10-14T05:25Z)

# fix-menus-real — the front end rebuilt against the re-parsed menu-def (2026-10-14)

Branch `s/fix-menus-real`. Files: `crates/nf-view/src/menu.rs`,
`crates/nf-view/tests/menus_flow.rs`, this report, `docs/WORKLOG.md`. **Shared-file hooks (1)**:
`crates/nf-view/src/lib.rs` — one additive line in the `--page` name table
(`"records" => menu::Page::Records,`) so the new page renders headless; nothing else in that file
changed.

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
- Final tree (`tools/test/suite.py`, `NF_ISO` = the onfoot ISO): **913 passed, 4 failed, 27
  ignored (17 min)** — the 3 allowed `disc_golden` + one stale navigation in
  `movie_playback_flow::a_menu_started_mission_rides_state6…` (it confirmed on the codename page's
  old row 0; row 0 is now "New Codename" [M 0x1f7b78:58-66]). Fixed after the run with a `Down`
  first — the binary re-run is **9/9**; the rest of the suite was untouched by that one-line test
  change.

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
