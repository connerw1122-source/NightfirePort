## Your task: `menus-real2`, the front end: the main panel art, title font, button glyphs, the dossier rows

101k-fix-menus-real (merged after its suite) rebuilt the front end: the rings and their pictures, the list layout, the
centred wrapped descriptions, the hint rows and the title page now match PCSX2. The main agent's side-by-side
(`nf-view <iso> 07000048 --page <p> --cursor 1 --frame 60 --shot`) against the PCSX2 captures still shows:
1. **The main page's panel art:** the real main menu has the big orange curved lens panel (gold rim along its lower arc)
   with Bond in a tuxedo holding a pistol on the right and the 007 gun-barrel ring with the globe above the logo; the
   port's main page shows only the logo and the three items over the backdrop. Find the panel and the Bond art (page
   0x? controls, their textures) and draw them.
2. **The page-title font:** "Select Codename", "Select Difficulty", "Single Player", "Dossier" are in the game's
   bold ITALIC header font in the real game; the port uses the body font.
3. **The footer glyphs:** the real hints show the PS2 button icons (blue X, pink square, the D-pad, green triangle);
   the port's are tiny dots. Draw the game's button sprites at their size.
4. **The Dossier list:** the real page lists Records / Rewards / Gadgets (three rows); the port adds "Weapons".
   Check the page data (it may be locked/hidden by progress) and match.
5. **The backdrop:** the report says it's the aux FMV on the movie plane, drawn only with the `movies` feature and
   the retail PSS. Verify with `--features movies` and `--movie-frame 12` that every page shows the orange streak
   backdrop like PCSX2, and that the no-movies fallback isn't a near-black plate (use the movie's first frame or a
   still if the game has one).
Tests per item (element lists, font ids, sprite hashes, row lists vs the page data); sister assertions; `--shot`
commands per page.

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

**Branch:** `s/menus-real2`.
