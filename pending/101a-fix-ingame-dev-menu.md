## Your task: `ingame-dev-menu`, the dev menu inside the game window, no browser

The user (2026-10-13): "this dev menu sucks. I'm tired of opening a browser to use it." Today `--dev` starts an HTTP
panel (`crates/nf-view/src/panel.rs`, 127.0.0.1:7007) for the catalogue (every weapon/gadget/grenade by the game's own
name, ammo, NPC spawns, clear NPCs, arena, level switch, screenshots), and the in-window F2 menu (`dev.rs`, WORKLOG 0081)
only has a few toggles.

Move ALL of the panel's functions into the in-window F2 menu and stop starting the HTTP panel (keep `panel::apply` as the
shared command layer, so nothing is implemented twice; drop the server or put it behind `--panel`). The menu must be
usable without leaving the game: opens over the frame and pauses the sim (Esc/F2 closes), keyboard (arrows/Enter/
Backspace, PageUp/PageDown, type-to-filter) and mouse (click, wheel); categories as pages: Weapons, Gadgets, Grenades/
explosives, Ammo/health/armour, NPCs (spawn by skin and mode, clear), Teleport (named spots per level, from
`dist`-independent data: put a table in `dev.rs` with spots from the existing tests: the 05 Q-Claw point (16.5, 29.6,
58.6), the 0c display vent (5.8, 15.8, -108.5), the 08 safe (155.19, 43.12, -113.38), the 07 Kimiko trigger
(94.2, 44.6, -45.9), the 16 door (5.0, 0.9, -70.0), the 02 pond bank (-82.3, 5.3, 32.05), the 05 cable (-23.58, 34.0, 2.65)),
Level (jump to any level), Toggles (god, infinite ammo, noclip, AI overlay, portal culling on/off, interpolation on/off,
fps), Screenshot. Draw it with the game's HUD font and a dark translucent box like the rest of the HUD, readable at
1280x720 and at 4K. Spawning a weapon equips and draws it (the `--give` path in `lib.rs`).

The user once crashed the game on 07000015 by pausing to spawn grenades from the panel (results 1013-1758 `frag15`, no
panic text was written, so a hard crash): make sure spawning while paused or with the menu open is safe and covered by a
test.

Tests: headless menu tests through the real key path (open, navigate to Weapons, Enter on the frag grenade: the inventory
has 0x34 and it's drawn; the Teleport page moves Bond to the spot and he's grounded after 60 ticks; filter by typing
"laser" finds the Q-Laser); sister assertions. List `--shot` commands to render each page (add `--dev-page <name>`).

Files: `crates/nf-view/src/{dev,panel,hud,lib}.rs`, a new `crates/nf-view/tests/dev_menu_flow.rs`.

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

**Branch:** `s/ingame-dev-menu`.
