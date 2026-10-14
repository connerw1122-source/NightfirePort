# fix-ingame-dev-menu — the in-window dev menu (no browser)

Task 101a: the user — "this dev menu sucks. I'm tired of opening a browser to use it." Move ALL of the
HTTP panel's functions into the in-window F2 menu and stop starting the HTTP server by default.

## What's done

**The F2 menu now carries the whole panel** (`crates/nf-view/src/dev.rs`, `panel.rs` doc): nine pages
over the frame — Weapons (every non-gadget record incl. grenades, the game's own name per row, plus
"level kit"), Gadgets, Grenades/explosives (0x34-0x37 ×5), Ammo/health/armour (full health / full
armour / +3 clips per owned weapon), NPCs (mode select normal/sniper/ambush, spawn by skin with the
free/total pool count, crosshair spawn, clear NPCs, arena), Teleport (save/restore spot + the seven
named spots), Level (the disc's 34 archives), Toggles (god, infinite ammo, no reload, noclip, AI
overlay, portal culling, interpolation, fps display, time scale), Screenshot (the F12 spot-shot path).

**Nothing is implemented twice**: every give/spawn/clear/health command routes through
`panel::apply` (panel.rs — the shared command layer), via the new `dev::apply_menu_action_to_scene`
— the same router `App::apply_menu_action` uses, so the menu and (with `--panel`) the HTTP page
cannot drift. `App::apply_menu_action` is now a thin shell applying only what needs the window:
`weapon_switch_to`, `pending_level`, the viewer-cel reset ([M] `Camera_Enable` 0x1b9920's placement
order, as RestoreSpot already did), noclip, and the screenshot.

**The HTTP panel is opt-in**: `--dev` no longer starts the server; `--panel` does (lib.rs). The
in-window menu is the default surface; `poll_panel` still serves `--panel` users through the same
`panel::apply`.

**Input** (the task's list, all through the real key path — the window handler calls
`dev::menu_key` / `dev::menu_type` / `dev::menu_activate`): ↑/↓ move, Enter activates, PageUp/PageDown
switch pages, Backspace deletes a filter char (or returns to the index), typing filters
(case-insensitive substring on the rows' game names; `dev_filter_char` maps physical keys to ASCII),
Esc/F2 close and re-grab the mouse; mouse click hit-tests rows (draw and hit-test share one
`layout_rows`, so they cannot drift) and the wheel moves the selection.

**The menu pauses the sim** (the task's "opens over the frame and pauses the sim"): while open the
frame runs `Dev::paused_tick_count()` — 0 ticks, or exactly 1 when F11's one-shot step was pressed.
The old menu left the sim running under the open panel.

**Spawning while paused is safe and tested** — the user's 07000015 crash (pause to spawn grenades
from the panel; results 1013-1758, no panic text — a hard crash on the game side). In the port the
menu-paused give/spawn lands through `panel::apply` between frames and resuming runs the real tick
path over the new state; `spawn_with_the_menu_open_while_paused_is_safe` (07000015) gives frag ×5
through the Grenades page and spawns a pool NPC with the menu open, then resumes 120 ticks: no
panic, the inventory and NPC are right.

**Teleport spots** (`dev::TELEPORT_SPOTS`, the task's table verbatim): 05 Q-Claw point
(16.5, 29.6, 58.6), 05 cable (-23.58, 34.0, 2.65), 08 safe (155.19, 43.12, -113.38), 07 Kimiko
trigger (94.2, 44.6, -45.9), 16 door (5.0, 0.9, -70.0), 02 pond bank (-82.3, 5.3, 32.05), 0c display
vent (5.8, 15.8, -108.5). Teleport = `Bond::stand_at` (the game's own placement path), which
grounds Bond on the floor under the spot's stand height.

**Toggles plumbing**: portal culling off passes no world to `Renderer::cel_visibility` (the one
visibility walk, [M] 0x1e86e8's stand-in); interpolation off forces alpha 1.0 (the `NF_NOINTERP`
behaviour, now a toggle); fps display is a wall-clock readout drawn by `dev::hud` (never read by the
Sim — determinism untouched; no RNG draws added anywhere).

**`--dev-page <name>`** renders a page over the game HUD headless: menu/weapons/gadgets/grenades/
ammo/npcs/teleport/level/toggles/screenshot, combined with `--shot`.

**Readability**: the panel widened 150 → 190 HUD-px so the game's own names fit ("0x2c AT-420
Sentinel" = 20 glyphs); at a real 16:9 window the aspect correction puts that at ~28% of the screen.
Drawn with the HUD font + the dark translucent boxes the rest of the HUD uses (tex 0x0300_0003 at
0.7-0.9 alpha), HUD-space so 720p and 4K scale the same way.

## Evidence

- The panel's command set and ids: `panel::apply`'s arms [M crates/nf-view/src/panel.rs], the
  grenade ids 0x34 Fragmentation / 0x35 Stun / 0x36 Smoke / 0x37 Remote Mine [L USATxt.dat via
  `Txt_BindLabel(+0x38)`, docs/fields/weapon_data.md +0x38].
- The bank's own names re-verified this session [D]: 0x34 "Fragmentation Grenade",
  0x35 "Stun Grenade", 0x3a "Laser Tripbomb" — typing "laser" on 07000005 finds 0x3a; the old
  hardcoded table's "Q-Laser"/"Laser Watch" names were the memory-asserted wrongness the panel
  already replaced (0x50/0x54 are Grapple/Micro-Camera [D]). The filter finds the bank's own
  laser rows.
- Gadget page bounds: ids 0x4A+ (`Player_WeaponChange`'s gadget cycle) [M 0x1a6f80].
- Portal culling walk [M 0x1e86e8]; the viewer-cel clear on placement [M 0x1b9920].

## Tests (all through the real key path; NF_ISO-gated, `crates/nf-view/tests/dev_menu_flow.rs`)

- `menu_open_weapons_enter_on_frag_gives_0x34_and_draws_it` — open → Weapons (Enter) → type "frag" →
  Enter: `panel::apply` owns 0x34, the clip slot is filled, the switch request returns, and after
  the real `step_bond` path `sim.gun.weapon == 0x34` (it's drawn). Sister: 0x35 (stun) is NOT
  touched — the old panel's "Frag"-gives-stun mix-up cannot come back.
- `typing_laser_finds_the_q_laser` — "laser" narrows the page to the bank's laser row(s)
  (0x3a Laser Tripbomb on 07000005); sister: the filter narrows, and a dead filter shows no
  weapon rows.
- `menu_teleport_page_moves_bond_to_the_display_vent_and_he_grounds` — PageDown ×6 → the 0c spot →
  Bond at (5.8, ·, -108.5) exactly (x/z), standing on the vent floor, `grounded` after 60 real
  ticks; sister: Backspace returns to the index.
- `spawn_with_the_menu_open_while_paused_is_safe` — the 07000015 crash flow (above); asserts the
  paused tick count (0 open / 1 with F11), the give, the NPC spawn, and 120 resumed ticks.
- `every_panel_command_has_a_menu_row` — every `panel::apply` arm (weapon, ammo_for, ammo_all,
  levelkit, health, armour, npc, clear_npcs, arena) and every `poll_panel` special (god, ammo,
  shot) is reachable from a menu row; the index lists all nine sections; `--dev-page` resolves
  every name.

`menu_live.rs` / `dev_tools_live.rs` (pre-existing menu tests) updated for the new signatures
(`menu_items(dev, Some(&scene))`) and the panel width (the B-11 cap moved to 40% of the 512 HUD
width with the reason in the test — 190 px ≈ 28% of a real 16:9 window). All their assertions
(no row overlap, glyph-height-derived layout, every action runs without panicking) still hold.

## Verified

- 5/5 `dev_menu_flow` tests green on this branch (07000005 / 0700000c / 07000015 loads).
- Full suite, merged with origin/master c3b989b: **884 passed, 3 failed, 27 ignored** over all 211
  test binaries (the fast runner's exact binary set, run serially per binary with
  `--test-threads=1` in bounded foreground batches — see the WORKLOG note on why suite.py's nohup
  form cannot survive this sandbox). The 3 failures are exactly the allowed `disc_golden` set on
  the slim ISO (`disc_golden: 11 passed, 3 failed`); every other binary green.

## What's left / notes

- The crosshair "spawn at crosshair" row uses Bond's look direction instead of the free camera's
  (the router is headless-testable, so it has no camera); on foot they are the same ray.
- The browser page's per-row "+Ammo" button for unowned weapons became the Ammo page's per-OWNED
  rows (the panel only ever listed owned top-ups via `ammo_all`; per-weapon `ammo_for` works for
  any id through `panel::apply` if wanted later).
- The 07000015 archive has no 0x34 first-person skin (the give still draws the weapon per the
  game's own equip path — `Player_EquipWeapon` 0x1a8a58 gates on the weapon_data record, not the
  level archive).

## --shot commands for the main agent (each page)

```
nf-view <iso> 07000005 --shot /tmp/menu-index.png      --frame 1 --dev-page menu
nf-view <iso> 07000005 --shot /tmp/menu-weapons.png    --frame 1 --dev-page weapons
nf-view <iso> 07000005 --shot /tmp/menu-gadgets.png    --frame 1 --dev-page gadgets
nf-view <iso> 07000005 --shot /tmp/menu-grenades.png   --frame 1 --dev-page grenades
nf-view <iso> 07000005 --shot /tmp/menu-ammo.png       --frame 1 --dev-page ammo
nf-view <iso> 07000005 --shot /tmp/menu-npcs.png       --frame 1 --dev-page npcs
nf-view <iso> 07000005 --shot /tmp/menu-teleport.png   --frame 1 --dev-page teleport
nf-view <iso> 07000005 --shot /tmp/menu-level.png      --frame 1 --dev-page level
nf-view <iso> 07000005 --shot /tmp/menu-toggles.png    --frame 1 --dev-page toggles
nf-view <iso> 07000005 --shot /tmp/menu-screenshot.png --frame 1 --dev-page screenshot
```
Each should show the dark translucent panel top-left with the page header `[N/9] <page> pgup/dn`,
the rows in the HUD font (selected row highlighted), and the description line under the list.

## Live checks for the main agent

PCSX2 is not required for the menu itself (it is viewer tooling), but the two toggle hooks touch
render behaviour — please spot-check in nf-view:

1. `nf-view <iso> 07000005 --dev` (windowed): F2 → the menu opens over the frame and the sim pauses
   (the HUD ammo strip freezes; F11 advances exactly one tick). Esc/F2 closes and the mouse
   re-grabs. Expected: no tick advance while the menu is open, the time scale unchanged after.
2. Toggles → "portal culling: OFF": the whole level draws (07000005's backrooms visible through
   walls); back ON: the normal culling. Same for "interpolation: OFF" (steppy but correct motion).
3. Weapons → "0x2c AT-420 Sentinel" Enter: the rocket launcher equips and draws (the user's
   "no clue what the rocket launcher is" — the row says the game's own name).
4. `--bond` spots for the teleport page renders (0700000c):
   `--bond 5.8,15.8,-108.5` should look out of the display vent; (07000005)
   `--bond 16.5,29.6,58.6` at the Q-Claw point, `--bond -23.58,34.0,2.65` at the cable car.

## Files

- `crates/nf-view/src/dev.rs` — the paged menu, the shared router, teleport spots, pause, filter.
- `crates/nf-view/src/panel.rs` — doc header only (server behind `--panel`; `apply` unchanged).
- `crates/nf-view/src/lib.rs` (task file; additive hooks): the key interceptor, wheel scroll, the
  menu pause, the `--panel` flag, `--dev-page`, the portal/interp toggle hooks, `set_noclip`,
  `dev_filter_char`, fps tracking; `apply_menu_action` delegated to the shared router.
- `crates/nf-view/src/hud.rs` — untouched (the menu reuses the HUD font/quad path).
- `crates/nf-view/tests/dev_menu_flow.rs` — new; `tests/menu_live.rs`, `tests/dev_tools_live.rs` —
  signature/width updates only.
- No shared-file surprises: `drone.rs`/`scene.rs`/`player.rs` untouched; `lib.rs` is a task file.
