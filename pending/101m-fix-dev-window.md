## Your task: `dev-window`, the dev menu in its own window (not in the game, not a browser)

The user (2026-10-13): "the dev menu should manifest outside of the game, but not in a browser." So: a SEPARATE native
window, opened by `--dev` beside the game window, that holds the whole dev menu. Not an overlay inside the game frame
(101a-fix-ingame-dev-menu built the pages as an F2 overlay; keep its page model, `dev::PAGES`, and the shared
`panel::apply` command layer), and not the old HTTP panel (drop the server, or leave it behind `--panel`).

Build it with what nf-view already uses: a second `winit` 0.30 window from the same event loop
(`ActiveEventLoop::create_window`, events routed by `WindowId`) with its own `wgpu` surface on the same device, drawing
with the existing text/quad renderer (the HUD font). NO new crates: downloads need the user's approval (if you think a
GUI crate like egui is necessary, say so in the report instead of adding it). The window:
- opens beside the game window (to its right if the screen has room), its own title "Nightfire dev", resizable,
  remembers nothing between runs; closing it doesn't close the game; F2 in the game window shows/raises it;
- clicking or typing in it never grabs the game's mouse or steals the game's keys; the game keeps running (no pause)
  while you use it, so you can spawn a grenade and watch it in the game window;
- shows every page as a full list: page tabs or a left column (Weapons, Gadgets, Grenades/explosives,
  Ammo/health/armour, NPCs, Teleport, Level, Toggles, Screenshot), the rows with the game's own names, no truncation,
  a filter box (type to filter), a scrollbar/mouse wheel, double-click or Enter to apply, a status line with the last
  action's result ("equipped 0x34 Fragmentation Grenade");
- readable at 100% and 200% Windows scaling.
Apply each action on the game's next tick (commands queue from the dev window to the sim, as the panel did). Make
actions safe while the game is paused or in a menu (the user crashed 07000015 spawning grenades while paused, with no
panic text: cover that). Tests: the command queue applies a weapon/teleport/toggle on the next tick; the layout keeps
every row inside the window at small and large sizes; spawning while paused is safe; sister assertions. Note the second
window can't be opened in headless tests: test the layout and the command path, and give the user the exact steps to try
it.

Files: `crates/nf-view/src/{dev,lib,panel}.rs`, a new `crates/nf-view/src/devwin.rs`, `crates/nf-view/tests/dev_menu_flow.rs`.

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

**Branch:** `s/dev-window`.
