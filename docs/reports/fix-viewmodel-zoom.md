# fix-viewmodel-zoom — report

Branch `s/fix-viewmodel-zoom` (GAPS G-044 + the zoom overlays). All facts tagged [M]/[L]/[D]/[S].

## Done

### 1. G-044: the view-model tilt ignores the zoom (fixed)

`Player_WeaponRecoil` 0x1a3de8:52-57 [M] writes the weapon-anim object's tilt as
`rot+0x50 = BL+0x124·(−π/6) ÷ viewer+0x108` and
`rot+0x54 = viewer+0x114·(−BL+0x120·π/6 ÷ viewer+0x108) + BL+0x8E4`, with `rot+0x58 = 0`,
where `viewer = glb_viewer[BL+0x94e]` (0x2d88b0). `viewer+0x108` is the zoom: `Player_PositionCamera`
0x18f5d8:129 feeds it `BL+0x8D0` (the port's `Sim::zoom`) every frame, and `Camera_CalcViewAngles`
0x1baf70 resets it to 1.0. `viewer+0x114` is the aspect scale: **4/3** normally, **16/9** when the
widescreen flag `DAT_002a37c4` is set, halved/doubled for 2-viewer split [M 0x1baf70:25-45] —
[L ee_start_05: viewer 0x01fc6c10+0x108 = 1.0, +0x114 = 1.33333 (0x3faaaaab)].

The port (`scene.rs` `Sim::gun_matrix`) applied `4/3·cx·π/6` and `cy·π/6` with **no zoom divide**: a
zoomed view tilted the gun as if unzoomed. Fix: both terms divide by `self.zoom`; the 4/3 stays
(correct for the NTSC 4:3 the port renders; the port doesn't model the widescreen mode or split —
noted under "Left").

Test `the_gun_tilt_divides_by_the_zoom` (crates/nf-view/tests/fix_viewmodel_zoom_live.rs): loads
07000005 through `nf_view::load`, equips the sniper 0x24 [L weapon_data 0x2bf150: flags 0x4080D0
(scoped), zoom 10], aims with a fixed crosshair offset, holds the zoom button through
`Sim::step_bond` (`Player_Zoom` 0x1a1a70 raises the zoom to 10), and asserts the gun-tilt angle at
zoom z is the zoom-1 angle divided by z. **Failed before the fix** (the tilt stayed at the zoom-1
value at every zoom); passes after. Sister assertion: the zoomed tilt is strictly smaller.

### 2. The zoom overlay per weapon id (fixed)

The selection is `HUD_UpdateCrossHair` 0x19e6d0:106-147 [M]: in aim mode a scoped weapon (flags
+0x68 & 0x40 [L weapon_data]) hides the crosshair (the path ends at LAB_0019e9c8, layer 0xff) and
enables its zoom pane — the switch runs on `wid − 0x1a`, and the offsets decode as pane records
(created flag +0x1F gated, enabled byte +0x1C written 1, pane+0x1A zeroed — the inlined
`HUD_Enable`):

| weapon id | pane | overlay |
|---|---|---|
| 0x1a / 0x1b (OICW twins, zoom 6 [L]) | 17 | `OICWSprInfo` 0x2be008 |
| 0x32 / 0x33 (Ronin twins, zoom 3.5 [L]) | 19 | `LaserSprInfo` 0x2be288 |
| 0x54 / 0x55 (the watch, zoom 8 [L]; the game rewrites 0x55 to 0x54 mid-switch) | 11 | `CameraSprInfo` 0x2bd600 |
| default (the sniper 0x24, zoom 10, and any other scoped weapon) | 6 | `SightSprInfo` 0x2bd558 — the scope |

The port drew the **scope for every zoom** (`zoom_target > 1.0` → SightPane quads) — the user's
"The micro camera, when you zoom in, doesn't actually have the camera overlay, but the scope of a
sniper." (The zoomable gadget in the port is the watch 0x54/0x55 — the camera gadget 0x52 is not
scoped and cannot zoom yet; see "Left".) Fix (`crates/nf-view/src/hud.rs`):

- `pub fn zoom_overlay(weapon: u8) -> ZoomOverlay` — the selection above, with 0x52/0x53 (the micro
  camera / briefcase, the mission layer's zoom) mapped to the Camera pane for when their fire-side
  is ported.
- The four pane draws, from the SprInfo tables [L ee_start_05]:
  - **Camera pane 11**: the viewfinder texture 0x03000055 over the whole screen (uv 0..255 × 0..127,
    subtractive, layer 39) + four corner brackets 0x03000056/57 (sprite 3 h-flipped by flag 0x40).
    Sprite 0 is the photo flash (shown only during `HUD_UpdateCameraPane` 0x1a12d8's post-shot
    countdown — hidden at rest) and sprite 8 (flags 0x900) is 3D-positioned; neither is drawn.
  - **Laser pane 19**: six solid-texture strips (top/bottom bars + four centre ticks), blend 1,
    colour 0x7f7f7f alpha 0x78, layers 40/34.
  - **OICW pane 17**: the subtractive cyan full-screen tint (col 0x00ffffff) + the dim scope circle
    0x03000141 (alpha 0x40). Sprites 2–6 are text sprites (the zoom readout) — not drawn.
  - **Sight pane 6**: the scope, unchanged (4 mirrored tiles + the two subtractive strips).
- The crosshair hides while an overlay draws [M 0x19e6d0 aim+scoped path]. Gated on the port's
  established `zoom_target > 1.0` (the button state) — the game gates on aim mode, which arrives a
  few ticks earlier; see "Left".
- `hud::TEXTURES` grew by 0x03000055/56/57/90 and 0x03000141. Texture availability checked against
  the disc: 0x03000055/56/57/90 ship on nearly every level archive, 0x03000141 on 07000014-16 + MP
  (`nf-extract tex-list`).

Tests (`fix_viewmodel_zoom_live.rs`):
- `each_zoomable_weapon_draws_its_own_zoom_overlay` (NF_ISO, 07000014 — it ships the scope,
  viewfinder and OICW textures): the sniper draws the scope; the watch draws the viewfinder and NOT
  the scope (failed before: the scope quads were there and no viewfinder); the OICW draws its circle
  and not the scope; the Ronin draws its strips and neither; unzoomed, nothing draws.
- `the_zoom_overlay_selection_follows_the_weapon_id` (pure): all ten ids of the table above.

## Shared files touched

- `crates/nf-view/src/scene.rs` — the task-named gun-axes hook: the two-line zoom divide in
  `Sim::gun_matrix` + its comment. Nothing else.
- `crates/nf-view/src/hud.rs` — the task-named HUD overlay code. Nothing else.
- `crates/nf-game/*`, `drone.rs`, `player.rs`, `lib.rs` — untouched.

## Verification

- The three new tests failed before the fix (selection: `Sight != Oicw`; draw: "the watch draws the
  camera viewfinder" missing; tilt: the zoom stayed 1 until the setup's pending loadout switch was
  cleared, then the tilt assert failed) and pass after.
- Full suite: `NF_ISO=$NF_ISO cargo test --offline --release -j2 --no-fail-fast -p nf-game -p
  nf-formats -p nf-audio -p nf-view -- --test-threads=1` — result: **see the count below** (on the
  slim ISO only the 3 `disc_golden` tests may fail).
- No `Rand_*` draws added; no seed-pinned test can move.

## Left (for the next session / live checks)

1. The camera gadget 0x52's fire-side (the mission layer's photo/zoom handler, level 0700000c) is
   unported — its zoom cannot be exercised; 0x52/0x53 map to the Camera pane ready for it.
2. The OICW pane's text sprites (the zoom readout, OICWSprInfo[2..6]) need a text reader.
3. The crosshair hide uses the port's `zoom_target > 1.0`, not the game's aim-mode gate — during the
   few aim-before-zoom ticks the port shows the crosshair where the game shows none.
4. The widescreen 16/9 (`viewer+0x114`, `Camera_CalcViewAngles` 0x1baf70 `DAT_002a37c4`) and the
   2-viewer split halving are not modelled (the port renders a fixed 4:3 pixel space).

## Live checks for the main agent

PCSX2 (07000005; grant the laser watch 0x54 via the dev panel / cheat engine):
1. Equip the watch, hold aim+zoom: **expected** the camera viewfinder (screen frame + 4 corner
   brackets), NOT the circular sniper scope; release → the overlay disappears instantly and the
   crosshair returns.
2. The sniper 0x24, aim+zoom: the circular scope as before (regression), crosshair hidden while
   zoomed, back when released.
3. `fieldprobe` the weapon-anim object's tilt: `BL+0x7E8` → obj, watch `obj+0x50` (pitch tilt) with
   the same crosshair at zoom 1 vs zoom 8 — the zoomed value should be ~1/8 of the unzoomed one
   (G-044's divide, now ported).

nf-view (`--bond` spots; 07000005 `--bond 104.9,25.1,108.8,0.0`):
4. With the watch equipped and the zoom key held: the viewfinder frame + corner brackets, no scope
   circle, no crosshair.
5. With the sniper and the zoom key held: the scope circle as before (the tiles + the two black
   side strips).
6. On 07000014 with the OICW (0x1a): the cyan-tinted zoom circle. With the Ronin (0x32): the thin
   laser frame, no circle.

## Suite count

`NF_ISO=$NF_ISO cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p
nf-audio -p nf-view -- --test-threads=1` (run per crate after the sandbox killed background jobs):
**nf-game 32/32 binaries green; nf-view 154/154 binaries green (the 3 new fix-viewmodel-zoom tests
included); nf-formats + nf-audio green except the allowed 3 `disc_golden` failures**
(`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
`directory_tree_matches_the_golden_manifest` — the slim ISO's known misses). Zero unexpected
failures, zero panics. No `Rand_*` draws added; no seed-pinned test moved.

## ntfy failed

All three channels, in order, same cause (the daily message quota):
1. `curl`: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh/docs/publish/#limitations","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. `python3 urllib`: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget`: exit 8 (server error, 429), no body printed with `-q`.

The result line that should have been delivered: "fix-viewmodel-zoom: done — G-044 gun tilt
divides by the zoom (0x1a3de8) + per-weapon zoom overlays (0x19e6d0): the watch/micro-camera
viewfinder, the OICW and Ronin panes, the scope; 3 failing-first tests green; suite 32/32 nf-game
+ 154/154 nf-view green, 3 allowed disc_golden; finish.sh OK 283-fix-viewmodel-zoom."
