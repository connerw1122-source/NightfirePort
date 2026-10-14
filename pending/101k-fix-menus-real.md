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
