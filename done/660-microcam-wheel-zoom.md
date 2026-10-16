The user's playtest 2026-10-15 (results id 1015-1738) said: "It'd be cool if we could add some zoom to the micro
camera and have the scroll wheel do the zoom while you're right-clicking to aim in. And if you're not
right-clicking, of course, weapon switch." Implement for every zoomable weapon/gadget: while right-click (aim) is
held, the mouse wheel steps the zoom between the weapon's own min/max zoom (the game's per-weapon zoom range and
step [M the zoom input arm in Player_Weapon / weapon_data zoom fields]; for the Micro-Camera, its own range). When
right-click is not held, the wheel keeps switching weapons. Headless tests through the window's input path.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/microcam-wheel-zoom`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T04:25Z

---
## Report (done, 2026-10-16T05:12Z)


---
## Report from the worker (done, 2026-10-16T05:13Z)

# microcam-wheel-zoom — report

Branch `s/microcam-wheel-zoom` (from `origin/master` 6bab6409). Task 660.

## What's done

The 2026-10-15 playtest (results id 1015-1738): *"It'd be cool if we could add some zoom to the
micro camera and have the scroll wheel do the zoom while you're right-clicking to aim in. And if
you're not right-clicking, of course, weapon switch."* Implemented for **every zoomable
weapon/gadget**, through the window's input path:

- **While the right button (aim) is held**, one mouse-wheel notch steps the zoom target **×2 (wheel
  up) / ÷2 (wheel down)** within the held weapon's own range **[1.0, weapon_data+0x88]** — the max
  `Player_Weapon` 0x1a24c8:156-187 reads into BL+0x8D8 [M]. The ×2 step is the port's own choice
  [S]: the game's own zoom is continuous (`Player_Zoom` 0x1a1a70 multiplies the held target by
  1.025/tick), and ×2 notches give every weapon a clean ladder from its disc data — the sniper 0x24
  1→2→4→8→10, the Micro-Camera 0x54/0x55 1→2→4→8 (16 upgraded by `Upgrade_Weapon` 0x1a8f30 [M]),
  the Laser 0x32/0x33 1→2→3.5. The zoom itself still chases the target at the game's rates (×1.025
  in, ×1.15 out per tick [M Player_Zoom 0x1a1a70:22-39]).
- **Once the wheel has stepped, the hold-ramp to max yields** (`Sim::zoom_wheel` latch) so the notch
  stays while the button is held. Without wheel input the behaviour is exactly as before
  (right-click alone zooms to the weapon's max — the pad's zoom button keeps the same arm, it has no
  wheel). Releasing the aim resets the target to 1.0 [M the aim-bit arm] and clears the latch, so
  the next hold ramps to max again. A landed twin switch clears the latch with its zoom reset.
- **When right-click is not held — or the held weapon cannot zoom — the wheel keeps switching
  weapons.** Zoomable = the game's own scope flag `weapon_data+0x68 & 0x40` **and** a magnification
  above 1 [L weapon_data 0x2bf150 via the port's loader: 0x11 (4), 0x1e/0x1f (10), 0x20..0x23 (16),
  0x24/0x25 (10), 0x26..0x29 (16), 0x32/0x33 (3.5), 0x54/0x55 (8), 0x5d/0x5e (10)] — the same two
  conditions the sim's zoom block gates on, so the window never feeds a notch the sim would drop.

### Where the code went

- `nf_game::player::Input::zoom_cycle` (i8, +1 in / −1 out) — one wheel notch, consumed by the next
  tick like `weapon_cycle` [M file player.rs]. **Local view input: `pack_input` leaves it off the
  wire** — the remote path never runs the zoom pipeline (`Sim::step_net_remotes` ticks
  `Bond::tick_ctx` only), the 20-byte lockstep packet is byte-for-byte unchanged, and
  `unpack_input`/replay inputs always yield 0. `net_state_hash` covers poses/RNG/MP state only, so
  the lockstep cannot see the latch.
- The sim's zoom block (`nf-view/src/scene.rs`, the `Player_Zoom` arm) consumes `input.zoom_cycle`
  while `input.zoom && aiming` on a scoped weapon; new `Sim::zoom_wheel` latch field (own line,
  lesson 4).
- The window (`nf-view/src/lib.rs` `device_event`): routes a wheel notch to `zoom_cycle` when
  `wheel_steps_zoom(right_button_held, flags, zoom)` holds, else to `weapon_cycle` as before. The
  dev-menu / front-end wheel arms are untouched.
- `nf-view/src/replay.rs` fills `zoom_cycle: 0` (the replay format predates the feature).

## How it was verified

New `crates/nf-view/tests/microcam_wheel_zoom_live.rs` (NF_ISO-gated, everything through the real
input path `nf_view::load`/`load_with_carry_weapons` + `Sim::step_bond`):

1. **the_wheel_steps_the_zoom_between_the_weapon_min_and_max** — the sniper 0x24 drawn through the
   real switch (fire_modes3's arm+draw recipe), held right-click zooms to 10 (unchanged), wheel
   down: 10→5 and *stays* (the hold-ramp yields), zoom chases down; 5→2.5→5→10→10 (the clamp);
   release → target 1.0, latch clear; re-hold ramps to 10 again.
2. **the_micro_camera_steps_through_its_own_range** — the Micro-Camera equipped through the real
   number-key switch (the brackets test's recipe), hold zooms to ITS max 8, wheel down 8→4→2→1 with
   `camera_scan_roots` (CameraPane 11's bracket gate: target > 1 [M HUD_UpdateCameraPane
   0x1a12d8]) up at 4/2, hidden at exactly 1, back at 2.
3. **the_wheel_keeps_switching_weapons_when_not_aiming** — the wheel without the aim button sends a
   `weapon_cycle` switch that lands; no zoom. A notch on a non-scoped weapon leaves the zoom at 1.0.
4. **the_window_routes_the_wheel_by_the_weapons_zoom** — `wheel_steps_zoom` against the disc's own
   values (sniper/micro camera true when held; not held false; zoom-1-scoped false; rec 0x18
   zoom-3-without-the-flag false).

**Test runs on the final tree (all green unless noted):**

- `nf-game` lib 234/234; all `nf-game` integration tests: net_session 35/35 (the wire round-trip
  pins the unchanged 20-byte packet), ai_live 4, drive_hud 6, drive_physics 13, driving_ai_physics 2,
  sound_coverage 2, + the small suites — **except** `driving_ai_follow::real_mis01_track_rngp_walks`
  (8/9): *pre-existing and unrelated* — it parses the leftover untracked `DRIVING/` data folder in
  the sandbox root (another task's scratch); my diff touches player.rs/net.rs/scene.rs/lib.rs/
  replay.rs only, nothing in the RoadNet/refpack path.
- `nf-formats` 11/14 — the 3 failures are exactly the allowed `disc_golden` trio (slim ISO).
- `nf-view` lib 15/15; live: fix_viewmodel_zoom 5, microcam_brackets 5, fix_zoom_overlays 3,
  fix_aim_raise 3, fix_hud_zoom_tilt 6, fire_modes3 10, fix_auto_switch 2,
  fix_switch_after_cold_load 2, replay_live 3, net_match_live 3 (1 000-tick lockstep, hashes equal),
  weapon_matrix 6, mp_net_lobby 6, fix_weapon_modes 5, fix_weapon_fire_modes 6, fix_weapon_effects 9,
  weapon_anims 7, fix_weapon_sounds 3, weapon_assets 3, weapon_fx_levels 5, fix_guard_weapons 2,
  fix_weapons_all 4, microcam_wheel_zoom 4 (new).
- `nf-audio` untouched (no code change; not run).

**Fast-runner suite (`tools/test/suite.py`) was NOT run**: in this sandbox background jobs are
reaped between tool calls (it dies within seconds of `nohup … &`; verified twice), and a foreground
run exceeds the 10-minute tool-call window. Same limitation the previous two WORKLOG entries hit.
The touched crates' tests above stand in; the only failures outside my diff are the allowed
disc_golden trio and the pre-existing DRIVING-scratch test.

## Visual check for the main agent (rule 6)

No renderer/draw-output code changed — the zoom numbers feed the existing FOV/overlay paths. Still,
two renders would confirm the feel:

- `07000005` with the Micro-Camera held right-click (zoom 8) facing guards: the viewfinder pane +
  green brackets as before; after one wheel-down notch (target 4) the same overlay at twice the FOV;
  wheel down to the floor (target 1.0) hides the pane.
- The sniper held right-click, then wheel steps: the sight overlay persists and the FOV halves per
  notch down (10→5→2.5).

## What's left

Nothing known. Possible follow-ups if the playtest wants them: a per-weapon step count in
weapon_data (there is no step field on the disc [M docs/fields/weapon_data.md — +0x88 is the only
zoom entry]); the pad's zoom button could get the same stepping via its own input lane; MP: the
wheel zoom is deliberately local (view state), so remote players never see it — matching the
first-person-only view pipeline.

## Live checks for the main agent

None needed — the feature is port-only ([S] on top of [M] zoom facts); no EE addresses beyond the
cited Player_Zoom/Player_Weapon arms were consulted.
