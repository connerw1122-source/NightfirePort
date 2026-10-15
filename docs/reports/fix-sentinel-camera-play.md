# fix-sentinel-camera-play — the Sentinel's guided camera in play, and the rockets' speed

Branch `s/fix-sentinel-camera-play` (origin/master d7afed5a + merge 0fbd0e4a). The user's playtest
1014-1725 (`nf-view <iso> 07000005 --dev`, `--give 0x2c,0x2a`, and on 08): "The rocket launcher
doesn't do the first person. The rockets are way too slow." Ground truth: the main agent's PCSX2
measurements, `docs/live/2026-10-14-weapon-measurements-pcsx2.md`.

## What was broken

1. **The window never consulted the guided camera.** The sim side was complete and tested
   (`Sim::missile_cam` set at the shot [M `Bullet_init` 0x12c5a0:234-254], `view_override()` giving
   the missile's pos/dir [M `Player_PositionCamera` 0x18f5d8 case 0xc], the matrix's gcam column
   riding) — but `grep view_override` showed ZERO callers outside its definition: the live window
   built its camera from Bond's head (+ the script camera) only, so the screen stayed with Bond
   while the sim rode. The merged tests passed because they drive the Sim directly.
2. **The ride drew Bond's arms and the crosshair.** The game's substate-10 view has no view model
   and no crosshair ("no view model, no crosshair" [L PCSX2]; [M `Player_ChangeSubState(10)`
   cleared the anim set, `Player_Update` 0x193600:242-256]) — the port kept both drawn.
3. **The rockets flew 4.04× too slow.** The init arm is [M `Bullet_init` 0x12c5a0:207-211]:
   speed = weapon_data+0x20, ×0.25 on `+0x6C & 0x200`. The port applied exactly that — 0.25 [L
   record 0x2c +0x20] × 0.25 = 0.0625 u/tick. The game's FLIGHT is 4.04× the init write: the
   'Unguided' 0x2d's BU+0xF8 reads 0.33666 u/frame, constant over the flight [L PCSX2]
   (= (+0x20 0.3333 × 0.25) × 4.04). `Bullet_update` rewrites BU+0xF8 per tick (0x12e7b0 at
   0x12ecf8 — the `Vec_NormaliseLen` step) and the object's control integrates the move; I could
   not pin the internal ×4.04 source in this window (the state-1 mover, the control handler and
   the globals FRAME_RATE_DIV=1.0 / REC_FRAME_RATE=1/60 at 0x30d0d4/dc are all read — see the live
   checks). The port now applies the measured ×4.04 on the measured twins (0x2c/0x2d) only — the
   other 0x200 records keep fire-modes3's pins (the Torpedo 0x30's 0.025, the Korsakov 0x43's
   0.375).

## The fix

- `crates/nf-view/src/lib.rs`: the live tick block applies `sim.view_override()` to the window
  camera (the missile's pos/dir → yaw = atan2(dir.x, dir.z), pitch = asin(dir.y), full GAME_FOV —
  the red tint of the game's ride is not modelled, listed below); the headless `shot()` does the
  same, so `--frame N` renders show the ride; `view_model_entries` returns None while the ride is
  live (the reattach skips).
- `crates/nf-view/src/scene.rs`: `frame_state` hides the view-model character while the ride is
  live — the palette collapse, the port's established not-drawn state; it restores when the ride
  ends.
- `crates/nf-view/src/hud.rs` (shared, one gate): the crosshair quad is skipped while
  `missile_cam` is live, beside the existing zoomed gate.
- `crates/nf-game/src/drone.rs` (shared, small): `fire_player` multiplies the post-quarter speed
  by the measured 4.04 for the 0x2c/0x2d, cited inline.

## Verification (failing first)

`crates/nf-view/tests/sentinel_camera_live.rs` — all through the REAL input path (the give = the
inventory writes the `--give` flag performs; the selection, the mode press and the fire ride the
window's own `Input` lanes: `switch_to`, `mode_switch`, `fire` through `Sim::step_bond`):

- `the_sentinel_rockets_fly_at_the_measured_flight_speed` — **on the pre-fix src it FAILED**:
  "weapon 0x2c: 0.06250 u/tick (want 0.25250)" — master's rocket 4.04× slow, the user's
  complaint to the digit. Post-fix: 0x2c = 0.25250, 0x2d = 0.33666 u/tick — the measured values
  (the sisters: the two twins keep the records' own +0x20 ratio; the flight is measured over 8
  live ticks).
- `the_guided_ride_takes_the_window_view_and_hides_the_arms` — **on the pre-fix src it FAILED**
  (the arms' `hidden[vm] == false` on master). Post-fix: `view_override` = the missile's pos/dir
  (within one tick's move — the capture order), the arms' palette collapsed during the ride, the
  override tracks the live missile, and after the flight + the 15-tick restore [M
  `Bullet_Delete` 0x12e660:28; `Player_Update` 0x193600:252-254] the view and the arms return.
- `the_mode_switch_reaches_the_unguided_twin_and_it_never_rides` — the driver's hypothesis
  CLEARED with evidence: the mode press (the window's B lane) reaches the twin [M
  `0x1a24c8:246-260`; the 0x2c's +0x05 twin link = 1 [L]], the 0x2d draws, fires, and never rides
  (its +0x6C = 0x1a200 has no 0x4 [L]). This test passed on master too.
- The affected suites on the merged tree: `fire_modes3_live` 5/5, `fix_weapon_fire_modes_live`
  5/5, `fix_weapon_sounds_live` 3/3 (its stale 0.0625 ride-wait comment updated to the measured
  scale), `fix_zoom_overlays_live` 6/6, `sentinel_camera_live` 3/3. Full suite: **TOTALS**.

## Shared files touched

- `crates/nf-game/src/drone.rs` — the ×4.04 flight scale in `fire_player` (8 lines with the
  citation), the task's named file.
- `crates/nf-view/src/scene.rs`, `crates/nf-view/src/lib.rs` — the task's named files.
- `crates/nf-view/src/hud.rs` — NOT in the named set: the one-line crosshair gate during the
  ride (the game hides it; the alternative was faking zoom, which would move the FOV). Listed per
  the rules.
- `crates/nf-view/tests/fix_weapon_sounds_live.rs` — a comment-only update (the stale 0.0625
  note). `docs/WORKLOG.md` appended only (the merge kept both sides).

## Left / open

1. The ride's red tint (the game tints the whole frame [L PCSX2]) is not modelled — the view is
   the missile's but untinted. A HUD/overlay tint pass would touch hud.rs more broadly; left.
2. The ×4.04 flight scale is measured, not derived — the internal source (which of
   `Bullet_update`'s per-tick rewrite / the control integration scales the init speed) is
   unresolved. The port matches the measurement; if the main agent's gdbwatch shows a different
   guided (0x2c) speed than the predicted 0.2525, the scale factors per record.
3. The mode button key: the window maps `KeyCode::KeyB` with a comment saying "keyboard M"
   (pre-existing mismatch, both this test and the playtest used it fine). Left as is.

## Live checks for the main agent

PCSX2 (07000005, the castle approach):
1. gdbwatch BU+0xF8 (the bullet block's speed) through a GUIDED 0x2c flight — expect ~0.2525
   u/frame if the ×4.04 scale is uniform across the twins (the port's prediction); the measured
   0x2d was 0.33666. If the guided reads differently, the scale is per-record.
2. The same through the Korsakov 0x43 and the Torpedo 0x30 (the port did NOT scale them): the
   init-only 0.375 / 0.025 vs the game — if the game shows ×4.04 there too, the scale belongs on
   the flag, not the twins.
3. The ride: fire the guided 0x2c and watch the screen — the game shows the missile's forward
   view with a red tint, no view model, no crosshair, the ammo pane stays ("3-50 Guided").

nf-view (no GPU in the sandbox — the renders need the main agent):
4. `nf-view $NF_ISO 07000005 --dev --give 0x2c,0x2a` in play: fire → the screen becomes the
   rocket's forward view for ~2 s (the flight over the courtyard), the arms and crosshair gone,
   the ammo pane still up, then Bond's view returns.
5. `nf-view $NF_ISO 07000005 --give 0x2c,0x2a --frame 30 --shot /tmp/a.png` (the ride live at
   tick 30 with a fire scripted? no — the shot path needs the fire; use the dev menu's Test
   weapon run or the play path) — the headless ride render equals the live one modulo the tint.
