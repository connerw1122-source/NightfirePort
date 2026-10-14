# fix-zoom-overlays (102a) — the zoom overlays draw as the game draws them

Branch `s/fix-zoom-overlays` (task `102a-fix-zoom-overlays`). Files: `crates/nf-view/src/hud.rs`
(the fix), `crates/nf-view/tests/fix_zoom_overlays_live.rs` (new), `crates/nf-view/tests/fix_viewmodel_zoom_live.rs`
(selection test updated), `crates/nf-view/src/lib.rs` (one small additive shared-file hook, below).
`drone.rs` / `scene.rs` / `player.rs` untouched. No game bytes, tokens or URLs committed.

## The symptom (user playtest 1013-1825)

Zoomed with the Micro-Camera (07000005) and the AIMS-20 (07000016), the port drew the overlay as a
2×2 GRID: four copies of the frame, each with its own rounded corners, so a black cross ran through
the middle of the view. PCSX2 shows ONE frame (the Micro-Camera's full-screen viewfinder; the AIMS-20's
one centred circle).

## Root causes (with the evidence)

1. **The mirror-tiling sprite flag 0x400 was unported.** `View_DrawSprites` 0x1e5290:52-162 [M
   ACTION.ELF/001e5290] splits a sprite whose flags carry 0x400 into
   `tiles_x = (du+1)/texW × tiles_y = (dv+1)/texH` tiles (integer division against the texture's
   pixel size), each tile `w/tilesX × h/tilesY` screen px sampling `(u, v, du>>tilesX−1, dv>>tilesY−1)`,
   the U flip alternating per X tile (start parity = the sprite's 0x40 bit) and the V flip per Y tile
   (0x80) — a mirror repeat. The zoom overlay designs are QUARTER textures [D: the viewfinder
   0x03000055 is 128×64 in 07000005.bin, the OICW circle 0x03000141 is 64×64 and the scope 0x03000034
   256×256 in 07000016.bin], so the game's mirroring completes them into ONE frame. The port drew each
   sprite ONCE with the full UV extents, which its renderer wraps — an unmirrored 2×2 repeat: the grid.
2. **Two weapons mapped to the wrong pane.** fix-viewmodel-zoom gave 0x52/0x53 the Camera pane on the
   belief they were the micro camera. The game's overlay choice is `HUD_UpdateCrossHair` 0x19e6d0:106-147
   [M]: a switch on `wid − 0x1a` with exactly the cases 0/1 (→ pane 17, the OICW/AIMS-20), 0x18/0x19
   (→ pane 19, the laser), 0x3a/0x3b (→ pane 11, the camera) and the default (pane 6, the scope).
   There is **no case 0x38/0x39**, so 0x52/0x53 fall to the default scope. The name table settles the
   ids [L ee_start_05, through the game's own resolver `Txt_BindLabel` 0x1cad60 (FixupTable 0x1f9b200,
   Bank 0x1f9b230) over `weapon_data` 0x2bf150 + wid·0x10c + 0x38]: 0x11 'Delta Repeater',
   **0x1a/0x1b 'AIMS-20'** (the user's "AMES 20" — the Burst rifle, 07000016), 0x1e 'Winter Tactical
   Sniper', 0x24 'Winter Covert Sniper', 0x32/0x33 'Phoenix Samurai', **0x52/0x53 'Phoenix Ronin'**,
   **0x54/0x55 'Micro-Camera'**. Each twin pair shares one label, as it should.

## The fix

- `hud.rs::tiled_quad` — the game's tiler ported exactly ([M 0x1e5290:52-162], per-tile UV extents
  `du >> tiles−1`, the flip parity from 0x40/0x80 alternating per tile, blend bits 0x100/0x200).
- The four zoom panes route their sprites through it: the scope (SightSprInfo 0x2bd558 sprite 0:
  (32,0) 448×448, uv (0,0,511,511), flags 0x0600 — 2×2 mirror tiles of 0x03000034), the Micro-Camera
  viewfinder (CameraSprInfo 0x2bd600 sprite 1: (0,0) 512×448, uv (0,0,255,127), flags 0x0600 — 2×2
  of 0x03000055), the AIMS-20 circle (OICWSprInfo 0x2be008 sprite 1: (192,160) 128×128, uv (1,1,127,127),
  flags 0x0500 — 2×2 of 0x03000141). The laser frame (LaserSprInfo 0x2be288, flags 0x0100 — no 0x400)
  stays six whole strips. Sprite values dumped from ee_start_05 at the pane addresses above; the four
  panes' other sprites (the side strips 0/479 33×448, the mid brackets 0x03000056/57 at the game's
  rects, the photo-flash sprite 0 — managed by `HUD_UpdateCameraPane` 0x1a12d8, hidden at rest — and
  the 3D-positioned sprite 8) unchanged.
- `zoom_overlay`: 0x52/0x53 fall to the default Sight pane; 0x54/0x55 alone map to the Camera pane.
- The overlay keeps the game's 4:3 pixel shapes centred on wider windows (`Anchor::Centre`; identical
  mapping at 4:3), with black side bars on windows wider than 4:3 — the port's model of the game's
  512×448 screen [S: the game ran 4:3 or a stretched 16:9 of the same space; nothing outside it].

## Reproduce-before-fix and tests

`crates/nf-view/tests/fix_zoom_overlays_live.rs` (6 tests, all through the real paths — `nf_view::load`
+ `Sim::step_bond` / `Player_Zoom` 0x1a1a70 for the Micro-Camera): 4 failed before the fix (the
Micro-Camera and AIMS-20 drew 1 wrapped quad instead of 4 mirrored tiles; the wide-window bars were
missing), the sisters (the scope geometry, the laser strips, the pane selection) stayed green. Now 6/6
pass. The assertions pin, per overlay: the four tiles partition the game's rect, each tile samples the
texture once, the flips mirror (TL U+V+, TR U−, BL V−, BR U+V−), and the screen centre has NO seam
(mirrored neighbours sample the same texel at the shared edge) — the exact symptom.
`fix_viewmodel_zoom_live.rs`'s selection test was updated to the name-table evidence (3/3 pass).

## Suite (the merged tree: origin/master 66651ba + this branch, 8b5f071)

`tools/test/suite.py` semantics (227 binaries of the four crates, each binary `--test-threads=1` under
a wall-clock cap), run per-binary in the FOREGROUND because this sandbox kills background jobs between
tool calls (the same constraint the fix-portal-eyecel worker recorded; `nohup … suite.py` died twice):

**923 passed, 3 failed, 31 ignored.** The 3 failures are exactly the allowed slim-ISO `disc_golden`
set (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
`directory_tree_matches_the_golden_manifest`). `pss_movies_live` passed (3/0).

One runner caveat, reported straight: `weapon_matrix_live`'s single-binary wall clock (~710 s here:
its two `NF_MATRIX` halves each took 355 s) exceeds the runner's 600 s per-binary cap, so a strict
`suite.py` run flags it TIMEOUT on this sandbox. Its 28 tests ALL PASS when run through its own
`NF_MATRIX` selector in two halves (13 story levels + 14 wrapper levels + the dev sister, zero panics —
the test only fails on panics). The binary sat at the 600 s boundary in the grapple worker's run this
morning on this same machine; the slowdown came with the sandbox resets and the six merged tasks' scene
work, not this branch (the overlay draw adds 9 quads on zoomed cells; the matrix's zoomed cells hit
`tiled_quad` once per cell). **Live check for the main agent**: on the GPU machine, time
`NF_MATRIX=07000005 python3 tools/test/suite.py --only weapon_matrix_live` — if it fits comfortably
under 600 s there, the cap is fine as is; if it times out there too, give the binary a higher limit.

## Shared-file hook (additive, listed per the rules)

`crates/nf-view/src/lib.rs` (the headless `--shot` block): `--zoom-hold` — holds aim + the zoom key
every simulated tick (`Input { aiming: true, zoom: true, ..Default::default() }` into both
`step_bond` calls of the shot path), so the main agent can render the zoomed overlays headlessly —
until now there was no way to hold aim+zoom through the real input path in a `--shot`. No behaviour
change without the flag (default input stays `Default::default()`).

## Live checks for the main agent

PCSX2 (07000005 first, then 07000016):
1. Give the Micro-Camera (0x54), aim + zoom: ONE full-screen viewfinder — rounded corners only at the
   four SCREEN corners, the scale marks down the left and right, the two brackets at mid-height, the
   small centre circle, "Micro-Camera" bottom right. No black cross through the middle.
2. Give the AIMS-20 (0x1a, 'AIMS-20' in the name table), aim + zoom: the cyan-tinted screen with ONE
   centred circle (thin rim, thick posts), not four.
3. The sniper (0x1e) on 0700000c and the Delta Repeater (0x11): the scope oval unchanged (regression).
4. The Phoenix Ronin (0x52, if giveable): zooms into the plain scope (no camera frame) — the switch
   has no case 0x38/0x39.
5. `fieldprobe` `weapon_data + wid*0x10c + 0x38` for 0x52 and 0x54 → the labels 'Phoenix Ronin' /
   'Micro-Camera' on the HUD when selected (the name-table walk in this report, via the pause menu's
   weapon name line is enough).

nf-view `--shot` renders (headless, the new `--zoom-hold`; before/after = master vs this branch):
- `nf-view Nightfire-onfoot.iso 07000005 --give 0x54 --zoom-hold --frame 60 --shot zoom_microcam.png`
  → the viewfinder frame, whole, centred at 4:3.
- `nf-view Nightfire-onfoot.iso 07000016 --give 0x1a --zoom-hold --frame 60 --shot zoom_aims20.png`
  → the one circle inside the cyan tint.
- `nf-view Nightfire-onfoot.iso 0700000c --give 0x1e --zoom-hold --frame 60 --shot zoom_scope.png`
  → the scope oval (regression).
- `nf-view Nightfire-onfoot.iso 07000016 --give 0x32 --zoom-hold --frame 60 --shot zoom_laser.png`
  → the six thin laser strips, no circle, no tiles.
- A wide window (e.g. 1920×800): the same spots keep the 4:3 frame centred with black side bars.

## What's left

Nothing for the symptom. Notes for later tasks:
- OICWPane sprites 2-6 are text sprites (the zoom readout) — the port has no reader for their text
  yet (unchanged from fix-viewmodel-zoom; the live check above covers the readout's presence).
- The Micro-Camera's photo-flash (CameraSprInfo sprite 0, `HUD_UpdateCameraPane` 0x1a12d8's post-shot
  countdown) and the 3D-positioned sprite 8 are still not drawn (they were not drawn before; the
  countdown flash is a separate feature).
