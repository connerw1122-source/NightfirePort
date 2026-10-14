# fix-pause-menu — the in-game pause menu from the disc's page data

Branch `s/fix-pause-menu`. Facts tagged [M] code (function + address), [D] disc, [L] EE snapshot / parsed disc data, [S] suggested.

## What's done

- **The pause page's real identity, settled by the decompile and the disc.** The port's old
  `PAGE_PAUSE = 0x4000000c` is the NIS page (the handler table maps 0x4000000c → `P_NIS_Handler` [M
  `Handler_HandleMessage` 0x2029e0]). The in-game pause is page **0x4000004b** [M `Player_Update`
  0x193600:73 — Start in a mission runs `MenuManager_Create(0x80000002, 0x4000004b, …, 0x1e, 1, 3)` +
  `GS_PauseGame(1)`], and it does NOT live in the front-end menu def: the front-end stream
  (08000002 in 07000048.bin, 56 pages, all fh 0x80000002) has no 0x4000004b/0x40000042. The pause
  page, the fail menu 0x40000042 (fh 0x80000004), the briefing and the key-codes page (fh 0x80000003)
  are in the **mission archives' own kind-8 menu def** — entry `08000001` of each level archive
  (e.g. 07000001.bin, 27,424 B) [L parsed 2026-10-14]. Walk notes (they correct
  docs/systems/frontend.md): the pre-loop `BIN_GetDWord` eats a header dword [M 0x21b908 pre-loop];
  COMPONENT is 14 B (6 words + the texture dword) [M 0x21b908 case -0x04]; the CONTROL chunk's first
  byte after the name is the TYPE (the second is the create flag, 0/3 [M :275-330]) and
  `__Menu_SendEx(mgr, name, d1, msg, a, b)` matches `M_CONTROL+0x18` (name) and `+0x20` (the page
  data's d1 field) — so page 0x4000004b's controls named 0x10000028 are distinguished by d1
  (6 = the MISSION rows, 8 = the SCORE list, 0xa = the style list, 0xc = the confirm text, 0xd = the
  YES/NO list), which is exactly what C_GCPAUSE_Handler 0x204408 switches on.
- **The pause page rebuilt from its page data** (`crates/nf-view/src/menu.rs` `pause_page`):
  the backdrop plate (45,107,550×266) TEX 0x03000070 and the Bond image (385,121,202×218) TEX
  0x030000db [L the page's controls 0x10000001/0x10000184]; the four tabs **MISSION / OBJECTIVES /
  CONTROLS / SCORE** (LBL 0x167/0x176/0x168/0x169 at (84/199/339/465,120), the selected tab's 0x1c
  colour 0x201707ff); the MISSION rows **CONTINUE / RESTART / QUIT** (0x16a/0x16b/0x16c — the game's
  letter-spaced strings from USATxt.dat, the d1=6 list (153,183,334×120)); the YES/NO confirm box
  (0x5df "Y E S" / 0x5e0 "N O") with **0x1ff "RESTART: ARE YOU SURE?" / 0x200 "QUIT: ARE YOU SURE?"**
  and the selection opening on **NO** [L m0x1d:1; M 0x204408:181/204]; the hint bar 0x201
  "~A Select ~V Scroll ~H Next" (the confirm swaps it to 0x1000323 [M 0x204408:160/175/198]).
  Drawn **over the paused frame**: the full-screen dim plate is skipped for the in-game pages (the
  game dims nothing; the page's own backdrop carries them).
- **The actions are the game's, including a second discovery:** RESTART confirms too — the
  msg-0x4b d1=6 cases arm the same YES/NO list for rows 1 and 2 (`state_quit_166` picks the arm) [M
  0x204408]. Row 0 CONTINUE closes the menu (`MenuManager_Delete`), RESTART-YES runs
  `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)` [M :138-141], QUIT-YES sets +0x28 = 0x4000001c
  and loads 0x7000048 [M :143-148]. The old 4-row guess (resume/restart/options/quit) is gone —
  options are the game's **CONTROLS tab** (Y-Axis 0x492 + Inverted 0x206/Normal 0x3bd [M the
  msg-0x5e toggle arm; the port renders the labels, the setting stays the host's]), and `~H` (the
  port's Side action) cycles the tabs.
- **The fail/death menu 0x40000042 and the results page 0x40000036** are rebuilt per outcome
  (`results_page`): Failed = the 0x40000042 layout (header 0x2fd "Mission Failed" at (105,120),
  rows REPLAY 0x5bc / RESTART 0x16b / QUIT 0x16c in the 0x100001a4 list, hint 0x5c8 "~A Select
  ~V Scroll", the same backdrop + Bond art) — the row actions are still `P_ENDMISSION_Handler`
  0x216618:19-27's arms (warm replay / cold BaseMap restart / quit). Complete = the 0x40000036
  layout (header 0x2c5 "Results", the mission photo 0x03000140 (135,80,370×186), "Total Score:" 0x2cb
  / "Gold Medal Target:" 0x2cd, the stamp 0x0300014c + logo 0x0300016d, hint 0x2d0).
- **F12 can capture the menu now.** While a menu page was up, the menu key branch returned early for
  every key (`key_action(F12)` is None → early return), so the screenshot arm never ran and the user
  couldn't show us the pause menu. F12 is handled before the menu branch [fix-pause-menu, lib.rs].
- **`--page pause|results|complete`** (+ `--pause-tab N`, `--confirm`, `--cursor N`) render the
  in-game pages headless; `--page pause` gives the flow a mission level so the page opens the way it
  does over a running mission.
- `MENU_TEXTURES` grew to 25: +0x03000070 (512×256 plate), 0x030000db (128×128 Bond image) — in
  every mission archive [L nf-extract tex-list 07000029/0700004b/07000043/…] — and 0x03000140
  (256×128 results photo, front-end 07000048.bin [L]). Missing textures fall back to the dim plate.

## How it was verified

- `crates/nf-view/tests/menus_flow.rs` **18/18** (`NF_ISO=$NF_ISO cargo test --offline --release
  -p nf-view --test menus_flow`): the pause flow rewritten to the disc's three rows + BOTH confirms
  (restart arms 0x1ff, quit arms 0x200, the selection starts on NO, YES executes the same
  BaseMap-cold / front-end-quit loads as before — the row actions are unchanged); the page-data
  label pins against USATxt.dat (0x167 MISSION, 0x176 OBJECTIVES, 0x168 CONTROLS, 0x169 SCORE,
  0x16a/0x16b/0x16c, 0x1ff, 0x5df, 0x5e0, 0x2fd, 0x5bc, 0x5c8, 0x2c5, 0x2cb, 0x2cd); the sprite-hash
  pins (0x03000070/0x030000db/0x03000140 in MENU_TEXTURES); the confirm box's extra quads; all four
  tabs draw; both results outcomes draw.
- Suite: `NF_ISO=$NF_ISO cargo test --offline --release -j2 --no-fail-fast … -- --test-threads=1` —
  **476 passed, 0 failed** across the nf-view / nf-audio / nf-formats test binaries, with exactly
  the 3 allowed `disc_golden` failures on the slim ISO. The `walk*` live tests did not get a slot in
  the window; they are untouched by this branch (they exercise the sim, never `menu_overlay`).
- The sandbox has no GPU (lesson 6), so the pixel renders are the main agent's — commands below.

## Render commands for the main agent (side-by-side with PCSX2)

```
nf-view Nightfire-onfoot.iso --shot /tmp/pause-mission.png  --page pause
nf-view Nightfire-onfoot.iso --shot /tmp/pause-restart.png  --page pause --cursor 1 --confirm
nf-view Nightfire-onfoot.iso --shot /tmp/pause-quit.png     --page pause --cursor 2 --confirm
nf-view Nightfire-onfoot.iso --shot /tmp/pause-objectives.png --page pause --pause-tab 1
nf-view Nightfire-onfoot.iso --shot /tmp/pause-controls.png --page pause --pause-tab 2
nf-view Nightfire-onfoot.iso --shot /tmp/pause-score.png    --page pause --pause-tab 3
nf-view Nightfire-onfoot.iso --shot /tmp/failed.png         --page results
nf-view Nightfire-onfoot.iso --shot /tmp/complete.png       --page complete
```

Expected: the pause page = the warm plate centred over the (empty headless) frame, four tab labels
across the top (MISSION highlighted in the dark red-brown 0x201707), three letter-spaced rows
(C O N T I N U E / R E S T A R T / Q U I T, the selected full-bright), the Bond art on the right,
the hint bar with the game's ✕/scroll/R glyphs at the bottom; with `--confirm` the rows are covered
by "RESTART: ARE YOU SURE?" (or QUIT) and the YES/NO box with NO selected. `--page pause` on the
slim ISO draws the dim-plate fallback for the art (0x03000070/0x030000db are mission-archive
textures) — a mission-level shot shows the real art.

## What's left

1. **The OBJECTIVES tab's rows**: the game fills the list at open (one row per objective, a ~J done /
   ~I pending marker [L ACTION.ELF .data 0x30d2e0/0x30d2e8] + the objective's name) [M C_GCPAUSE
   0x204408's msg-0x4b d1=1 arm]. The port's FrontEnd carries no per-objective state yet, so the tab
   draws the box + hint only. Needs the objective-state plumbing (the mission layer's Objective
   events are in nf-game/mission.rs already).
2. **The SCORE tab's full rows**: the game fills the PlrStat table (Opponents/Dispatched/Subdued/
   Surrendered/Accuracy/Health/Time/007 Bonus [M 0x204408 case 3]); the port's Stats has only
   shots/hits/accuracy/time, so the tab shows those under the game's own labels. The score model
   (Total Score / Gold Medal Target on the complete page) is likewise unfilled ("—").
3. The complete results' rows: the disc page's own action is the hint's ~A Continue (0x2d0); the
   port keeps its three restart rows as its flow's affordances (drawn small in the left column).
   A product decision on continuing to the next mission belongs to the progress task.
4. The atlas strip control (88,156,465×27, TEX 0x03000042, msg 0x2d 0x260013/0x10001) — skipped;
   msg 0x2d's semantics (blend mode / chroma key?) are un-pinned.
5. The CONTROLS tab's controller-style list (the game's `Menu_DisplayControllerStyleList` fills the
   d1=0xa box from the d1=9 list) and the Y-Axis toggle are labels only; the port has no pad-settings
   model. The Y-axis invert itself is a host setting (fix-input's invert flags).

## Live checks for the main agent

1. **Pause page layout**: open the pause menu in PCSX2 (Start in 07000001) and screenshot. Compare
   with `--page pause`: the tab row, the three letter-spaced rows, the Bond art position, the hint
   bar. Tell me if the game's MISSION tab shows a selection bar/arrow sprite the page data doesn't
   name (the port dims the unselected rows only).
2. **The RESTART confirm**: choose RESTART in PCSX2 — expect the rows replaced by "RESTART: ARE YOU
   SURE?" + YES/NO with NO selected, the hint switching to "✕ Select ○ Back ↕ Scroll" (0x1000323).
   The port's draw is `--page pause --cursor 1 --confirm`.
3. **The OBJECTIVES tab**: how does the game render the ~J/~I state markers (green tick / hollow
   box?), and are failed objectives shown differently? (Port gap 1.)
4. **The fail menu**: die in PCSX2 and compare with `--page results` (0x40000042): header, three
   rows, art. In particular whether the middle row is visible (RESTART — the page data binds only
   rows 0 and 2; my row-1 label 0x16b is [S]).
5. **In nf-view**: run the windowed `nf-view <iso> 07000001`, press Esc mid-mission, then F12 — the
   saved nf-shots PNG must show the pause menu over the paused frame (the F12 fix), and ~H (E/Tab)
   must cycle the four tabs.

## ntfy

(sent after this report is pushed; see the queue finish log if it failed)
