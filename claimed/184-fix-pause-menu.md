## Your task: `pause-menu`, the in-game pause menu from the game's own page data

The front end was rebuilt from the disc's menu page data (fix-menus2/3: `MenuManager_Create` 0x21b908's tag stream in
07000048.bin's entry 08000002, the language bank, the game's sprites and hint icons). The IN-GAME pause menu (page
0x10000028, handled by `C_GCPAUSE_Handler` 0x204408) still uses fix-menus' plain text overlay, and the user says it
"doesn't look right". Rebuild it from its page data the same way: its controls, layout, strings, sprites and hint icons,
drawn 4:3 centred over the paused frame; the same rows and actions (resume / restart = BaseMap cold / options / quit with
the two-step confirm). Also the fail/death menu 0x40000042 (`P_ENDMISSION_Handler` 0x216618) and the results page, if
they're still plain. And make F12 screenshots capture the menu overlay (today they render only the 3D scene, so the user
couldn't show us the menu). Tests: the label ids and sprite hashes per page (numbers only), the row actions unchanged.
List `--shot` commands to render each page (add a `--page pause` if needed).

Files: `crates/nf-view/src/{menu,lib}.rs`, `crates/nf-view/tests/menus_flow.rs`.

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

**Branch:** `s/pause-menu`.

---
claimed-by: sandbox2 at 2026-10-13T23:55Z
