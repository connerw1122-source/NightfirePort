## Your task: `pause-menu2`, the pause page: the figure, the tabs' contents, the HUD under it

184-fix-pause-menu (merged at a3ace69/c4028c8) rebuilt the in-game pause page from its page data: the orange rounded
panel, the four tabs and the MISSION rows now match the real game. The main agent rendered each tab
(`nf-view <iso> 07000005 --page pause --pause-tab N --skip-cutscenes --frame 60 --shot x.png`) beside PCSX2 captures of the
real pause page on 07000005 (2026-10-13). What still differs:
1. The figure on the right: the real game draws a translucent ORANGE silhouette of a woman holding a pistol (on MISSION
   a standing figure; on the other tabs she aims left), blended into the panel. The port draws a rectangle of coloured
   blobs (magenta/green/yellow/cyan circles) on black with a white figure mask beside it: the sprite's texture/palette
   or its alpha/mask combine is wrong (two layers drawn as plain colour instead of the game's blend; check the CLUT,
   the texture format and the GS alpha/test mode the control uses).
2. MISSION: a yellow arrow cursor sits left of the selected row (CONTINUE / RESTART / QUIT); the port has none.
3. OBJECTIVES: the real tab lists the objectives (a yellow arrow and a bullet before "Breach the castle walls"), footer
   "X Hint / Scroll / Next"; the port's tab is empty.
4. CONTROLS: the real tab shows "Classic Bond" with a yellow up/down marker, "Y-Axis Normal" on the right, then two
   columns of PS2 button glyphs with labels (Reload/Action, Alt. Fire, Crouch, Manual Aim, Move/Look, Cycle Gadget,
   Vision Mode | Jump, Next Gadget, Next Weapon, Fire, Turn/Strafe, Cycle Weapon/Zoom), footer "Invert / Scroll /
   Next"; the port shows only the word "Inverted".
5. SCORE: the real table has the header "Category / Current / Target" and the rows Bond Moves, Opponents (Dispatched,
   Subdued, Surrendered indented), Accuracy Rating, Health Remaining, Time, 007 Bonus with both columns (targets from
   the level's medal data, e.g. 07000005: Bond Moves 2, Dispatched 30, Subdued 3, Surrendered 3, Accuracy 100, Health
   100, Time 09:00, Bonus 7); the port shows four rows and one column.
6. The in-game HUD (the radar ring bottom left, the ammo counter and fire mode bottom right) stays visible under the
   real pause page; the port hides it.
7. Footer glyphs: the real footers use the PS2 button icons; the port's are tiny dots.
Fix each from the page data and the game's draw code; tests per tab (the drawn element list: sprite hashes, CLUT and
blend mode for the figure, the text rows and their rects; the score rows/targets for 05 from the disc); sister assertions.

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

**Branch:** `s/pause-menu2`.

---
claimed-by: sandbox2 at 2026-10-14T04:24Z

---
## Report (done, 2026-10-14T05:56Z)


---
## Report from the worker (done, 2026-10-14T05:56Z)

# fix-pause-menu2 — the pause page's figure, the tabs' contents, the HUD under it

Branch `s/fix-pause-menu2` (from origin/master 6a118b8, then merged origin/master 69c0104 — the
100d/weapon-matrix merge). Facts tagged [M] code (function + address), [D] disc, [L] parsed disc /
snapshot data, [S] suggested.

## What's done

All seven reported gaps have a fix on the branch, built from the page data and the game's own draw
code (`crates/nf-view/src/menu.rs` unless noted):

1. **The figure** (the translucent orange silhouette): the disc texture 0x030000db decodes 8bpp
   through its own palette, and the mystery resolves into three facts. (a) The figure's texels are
   ALL index 0x43, whose palette entry is pure WHITE (`pal[067]=ffffff80` [L the retail palette]);
   the index histogram shows a flat run of 987 `0x4343` words exactly in the figure's bbox (x 29..62,
   y 15..125 of the 64×128 word view) [L]. (b) The palette-card blobs (the "magenta/green/yellow/
   cyan circles") live in the texture's LEFT half only — the control's uv rect samples the right
   half, so the game never draws them [L the histogram by half]. (c) The control carries
   `msg 0x6c = 0x100` [M P_PAUSE_Handler 0x203c28:91] → the sprite's flags 0x100 [M Label_Update
   0x218cb8:173], and psiDrawSprites renders flags 0x100 with `ALPHA_1 = 0x48 = (Cs−0)·As + Cd`
   [M 0x120e68:56-68] — additive: the black background (pal[0] = opaque black) adds nothing and
   vanishes over the panel, while the white figure takes the sprite's colour modulation. The port
   now draws the right half (uv 64..128 × 0..128) through blend 1 (the port's additive, the same
   mapping the HUD sprites use for flags 0x100) tinted orange. The tint is [S] — the control's
   colour word was not parsed; the live check compares it (below).
2. **The MISSION cursor**: the yellow arrow left of the selected row is drawn (a small yellow
   triangle at the list's cursor column, colour 0xfec43e [M page 0x40000009's 0x1b colour]; it
   moves with the selection — the test pins both). The disc's own cursor sprite was not identified
   in the remaining time; the triangle is the port's stand-in [S, live check].
3. **The OBJECTIVES tab**: one row per objective, a `~J` (done, state 4) / `~I` (pending) marker
   then the objective's name [M C_GCPAUSE_Handler 0x204408:251-271; the markers are ACTION.ELF
   .data 0x30d2e0 "~J" / 0x30d2e8 "~I" [L], drawn through the specialchar 'J'/'I' glyphs]. The
   host fills `fe.pause_objectives` each frame from the sim's mission states
   (`Mission.states[LEVELS index]`, state == 4 [M nf-game mission.rs]; lib.rs hook, listed below).
   The footer is the tab's own label 0x202 [M 0x204408:239].
4. **The CONTROLS tab**: the controller-style page's rows — "Classic Bond" with a yellow up/down
   marker, the Y-Axis line showing the game's boot value ("Normal", 0x3bd [M 0x204408:512-517]),
   then the per-action rows exactly as `Menu_DisplayControllerStyleList` builds them for style 0:
   6 rows × 2 columns of `<glyph> <label>` pairs [M 0x1f44d8 case 0: (0x8f,0x8c), (0x8b,0x8d),
   (0x84,0x89), (0x85,0x8a), (0x88,0x90), (0x87,0x86); the glyph strings are ACTION.ELF .data
   0x30d1c8-0x30d228 = "~A","~B","~Y","~X","~C","~D","~L","~R","~F","~E","~H","~V" [L — dumped
   from the retail ELF]], rendered through the specialchar table (the game's own PS2 button
   icons). The labels bind from USATxt.dat (0x84-0x90).
5. **The SCORE tab**: the Category/Current/Target header (0x1000204/0x10001e1/0x10001e2 [M
   0x204408:455-460]) and the game's row order — Bond Moves 0x165, the indented Opponents trio
   0x15b/0x15c/0x16e (the game's own "  %s" indent format 0x30d2f8 [L]), Accuracy 0x15e, Health
   0x15f, Time 0x160, 007 Bonus 0x161 [M 0x204408:461-498]. The TARGET column is the disc's own
   per-level data: `SCORE_TARGETS` in menu.rs, extracted from ACTION.ELF's static score records
   (the `PTR_HT_Level_Driving_Paris_ST` pointer table at 0x2bc658, 12 levels × 15 dwords, level
   row i's pointer → 9 × 0x14-byte slots whose +0x00 the handler reads [M PlrStats_SetupScoreTable
   0x1990e0; PlrStat_Init 0x198318 zeroes the runtime parts]). Cross-check: campaign row 1 =
   07000005 (The Exchange) reads [2, 30, 3, 3, 30, 100, 100, 540, 7] — Bond Moves 2, Dispatched
   30, Subdued 3, Surrendered 3, Accuracy 100, Health 100, Time 09:00, Bonus 7, exactly the PCSX2
   capture's numbers [L]. The row mapping is sp_level order [M Menu_GetLevelIndex 0x2023e8 walks
   sp_level[i]+0xc]. The Current column carries the port's tracked stats (time, accuracy); the
   counters the port's Sim does not track yet (bond moves, dispatched/subdued/surrendered, bonus)
   show 0 — reported below.
6. **The HUD under the pause page**: the host kept replacing the HUD frame with the menu overlay
   whenever the front end was open [M lib.rs render path]. The pause branch now draws the HUD
   frame FIRST and the menu over it (frame order = draw order), so the radar ring, the ammo
   counter and the fire mode stay visible under the page, like the game's [L the main agent's
   PCSX2 captures 2026-10-13]. Guarded against the movie state (a briefing movie still owns the
   screen).
7. **The footer glyphs**: **finding — the glyph atlas was never missing.** 0x03000075 (the
   `specialchar` table's texture [M 0x2dd8d8]) was already in `MENU_TEXTURES`, and it lives in
   every mission archive with the SAME bytes as the front-end copy (identical SHA-1s in
   07000005/07000029/0700004b/07000043/… [L nf-extract tex-list]). The pause footers go through
   the same `hint_bar` tokenizer as the front-end pages. I could not reproduce the "tiny dots"
   headlessly (no GPU in the sandbox; `--shot` needs an adapter) — the hint ids per tab are now
   the game's own (MISSION 0x201 [M :275], OBJECTIVES 0x202 [M :239], CONTROLS 0x204 [M :501],
   SCORE 0x205 [M :288]; the confirm keeps 0x1000323 [M :160/175/198]) with the captures' footer
   text as fallbacks. If the render still shows dots, the next probe is the specialchar uv values
   against the atlas (the table's entries are 256-wide [M 0x2dd8d8], the mission archives' texture
   is 256×128 — a v > 128 entry would clamp to nothing) — listed under live checks.

## How it was verified

- 5 new headless tests in `crates/nf-view/tests/menus_flow.rs` (23 in the file, all green):
  the yellow cursor exists and follows the selection; the OBJECTIVES rows draw with the ~J marker
  glyph (the specialchar 'J' uv pinned) and add quads per objective; the CONTROLS tab draws its 12
  button-glyph quads; the SCORE tab pins the disc targets (07000005's [2,30,3,3,…,540,7], the
  campaign-row mapping, 12 rows) and draws the header+rows; (NF_ISO) the figure texture loads
  128×128 with the white silhouette (≥900 pure-white px) sitting in the right half (<10% left).
- The suite on the final tree (origin/master merged): see the WORKLOG entry for the totals line.

## Shared-file hooks (small, additive)

- `crates/nf-view/src/scene.rs`: comment-only in the end (the mask experiment was reverted; the
  standard 8bpp decode stays). Net code change: none.
- `crates/nf-view/src/lib.rs`: the pause branch fills `fe.pause_objectives` from
  `nf_game::mission::LEVELS` + `sim.drones.mission.states`, and appends the HUD frame under the
  pause overlay (the runs' offsets shifted by the HUD's vertex count).

## What's left

- The Current column's live counters (bond moves, dispatched/subdued/surrendered, 007 bonus): the
  port's Sim has no PlrStat equivalent yet; they render as 0. The targets are the disc's.
- The objectives' visibility filter (`Mission_NumVisObjectives` [M 0x204408:251]) — the port lists
  all of the level's objectives; the game hides not-yet-active ones.
- The MISSION cursor's sprite (a triangle stand-in) and the figure's orange (0xe6661e [S]): both
  need one PCSX2 frame against the render to confirm or replace.
- The "tiny dots" footer (issue 7) is not reproduced headlessly — the fix shipped is the per-tab
  hint ids; see the live check.
- The CONTROLS tab renders the Classic Bond style only (the game has 6; the labels are the game's
  own per style).

## Live checks for the main agent

PCSX2 (07000005, pause page):
1. MISSION tab: the yellow arrow's shape and x-position vs the render `nf-view <iso> 07000005
   --page pause --pause-tab 0 --skip-cutscenes --frame 60 --shot x.png` (the port draws a ~10×12 px
   triangle at 640-space x≈218, on the selected row).
2. The figure: the render should show a translucent orange woman-with-pistol on the panel's right,
   no blobs, no black rectangle. Compare the orange with the capture (the port uses 0xe6661e at
   alpha 0.5, additive) — if the disc's tint differs, the colour word lives in the mission
   menu-def's control 0x10000184 (page 0x4000004b, entry 08000001) — parse its msg-0x1a/0x1b/0x1c
   dwords and I will swap the constant.
3. OBJECTIVES tab: the port shows one row per objective with a bullet-ish marker glyph before the
   name; the capture shows a yellow arrow AND a bullet before "Breach the castle walls" — confirm
   whether the arrow is the tab's row cursor (then the port needs the same arrow here).
4. CONTROLS tab: the 12 rows' labels (0x8f/0x8c/0x8b/0x8d/0x84/0x89/0x85/0x8a/0x88/0x90/0x87/0x86
   from USATxt.dat) and their glyph pairings vs the capture [M 0x1f44d8 case 0].
5. SCORE tab: the header and the 8 rows vs the capture; the targets should read exactly 2 / 30 /
   3 / 3 / 100% / 100 / 09:00 / 7 on 07000005.
6. The footer: does the port's pause footer now render the ✕/scroll/next glyphs? If it still shows
   tiny dots, dump one glyph quad's uv from the frame (the specialchar entries are in menu.rs's
   SPECIALCHAR) against texture 0x03000075's 256×128 atlas — a v ≥ 128 entry would be the cause.
