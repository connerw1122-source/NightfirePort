## Your task: `dev-menu-layout`, the F2 dev menu: room to read it

101a-fix-ingame-dev-menu moved the browser panel into the F2 menu (thank you: the user asked for exactly that). Rendered
with `nf-view <iso> 07000005 --dev --dev-page <menu|weapons|teleport|toggles> --skip-cutscenes --frame 30 --shot x.png`
at 1280x720, it is too cramped to use comfortably:
- the box is ~180 px wide and shows only 7 rows; long lists (71 weapons) need endless paging;
- names and descriptions are cut with "..": "05 cable (-23.58, ..", "give Bond any weapon..", "the game's own loado..";
- the header line ("[0/9] menu pgup/dn") is clipped by the top of the screen;
- the key-hint footer runs together ("arrows/enter pgup/dn type=bksp f2") and overlaps the list's page counter;
- the box barely darkens the frame behind it, so white text over snow is hard to read.
Make it a proper overlay: a dark panel of about 40% of the screen width and most of its height (scaled with the
window: readable at 1280x720 and at 4K), a header inside the screen (page name, breadcrumbs, the filter text), as many
rows as fit (20+ at 720p), no truncation (wrap the description into its own area under the list; names fit or the
panel widens), a selected-row highlight bar, a scrollbar or "n/m" indicator that doesn't overlap, and a footer with the
keys spelled out ("Enter select  Backspace back  PgUp/PgDn page  type to filter  F2 close"). Keep all behaviour and the
shared `panel::apply` layer. Tests: the layout puts every row and the header inside the screen at 1280x720 and
3840x2160; no drawn string is truncated; sister assertions. Render each page with `--dev-page` and include the commands.

Files: `crates/nf-view/src/dev.rs` (the draw and layout), `crates/nf-view/tests/dev_menu_flow.rs`.

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

**Branch:** `s/dev-menu-layout`.
