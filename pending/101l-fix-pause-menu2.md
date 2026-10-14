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
