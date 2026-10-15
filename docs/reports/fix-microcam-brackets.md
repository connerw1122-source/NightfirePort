# fix-microcam-brackets — the Micro-Camera's green target brackets

Task 109u: the real game's Micro-Camera (0x54/0x55) draws a green bracket box around every enemy on
screen, visible through walls; the port drew none (playtest 1015-0046). Branch `s/fix-microcam-brackets`.

## The game's mechanism (all cited)

- `HUD_UpdateCameraPane` 0x1a12d8 [M]: while the pane is enabled (the zoomed Micro-Camera view), it
  walks the game's drone list (`DAT_00298e14` = the 0x298E14 llist) and for every drone that is
  drawn (`obj+0xFC` != 0), alive (`Drone+0xac` > 0) and not in WaitSwitch (`obj+0xF4` != 1) creates
  a sprite object at the drone's root (`obj+0x30`) with gfx **0x02000832** and `obj+0xF0 |= 0x2000`,
  owned by the view object (+0x1C). No LOS test anywhere in the chain — the brackets draw through
  walls (the user's report).
- The gfx is "DroneScan" [D 07000005.bin map 15]: a 2×2 world-unit flat quad (min (−0.5, −1, −0.01),
  max (0.5, 1, 0)), texture **0x03000145** (64×64, one frame) — the green bracket box.
- The object is class 0x10 (`control_funcs[0x10]` = `Debris_Update` 0x1aac00 [L ee_snapshot
  @0x26f890+0x10·12]); the create zeroes its glist life (+0x30) and fade rate (+0x32), so
  `Debris_Update` deletes it after one update — a ONE-FRAME bracket the pane re-creates every tick.
- `obj+0xF0 & 0x2000` routes it to `View_AddObjects` 0x1e62d8's sort-list 2 (and stamps
  `obj+0xFC = 2` — the drawn byte the pane's own gate reads). The draw is the world picture's alpha
  pass — no depth occlusion for the bracket mesh, hence through-walls.

## The port's implementation (`crates/nf-view/src/hud.rs`)

- `TEXTURES` += 0x03000145 (the level-archive texture the build already resolves by hashcode).
- `camera_brackets(s, aspect)` [pub, the bracket draw list]: the game's candidate rule (present +
  enabled — the port's stand-in for the drawn byte's hide gate; health > 0; state != WaitSwitch),
  each candidate's root projected with `View_3DPoint2Screen` 0x1e7708's math on the gameplay camera
  (the render's own per-tick build: the eye, yaw, pitch·π/2, GAME_FOV/zoom — lib.rs:2328) and the
  box sized from the quad's 2×2 world extent through `Camera::view_proj`'s projection, converted
  into the hud's 512×448 space by `Hud::ndc`'s mapping. Behind-camera and wholly-off-screen candidates
  cull (the world draw's frustum). NOTE: the port's `drawn` field carries a LOS ray the game's
  `obj+0xFC` never had — the bracket candidates use present+enabled instead, or the through-wall
  draw could never happen.
- The `ZoomOverlay::Camera` branch draws one centred bracket quad per rect (layer 38, under the
  viewfinder frame at 39), texture 0x03000145 whole.

## Verification (NF_ISO, the real input path)

`crates/nf-view/tests/microcam_brackets_live.rs` — 10 tests, all green on the final tree
(merged with origin/master 56d6dc24). Shown failing on the pre-fix code first: all three behavioural
tests failed at the bracket assertion (the texture not loaded, no quads).

- `the_micro_camera_brackets_every_candidate_guard`: Bond equips the Micro-Camera through the real
  number-key switch, raises and zooms through the held aim/zoom buttons (`zoom` > 1.5 asserted),
  faces a living guard — the overlay carries the guard's bracket at the screen centre
  (1 quad drawn; the level's other candidates culled off-view like the game's cel draw).
- `a_corpse_gets_no_bracket`: the faced guard is killed through the real P2K fire path, the camera
  re-equipped — the dead guard's bracket is gone from the screen centre (the `Drone+0xac` > 0 gate).
- `brackets_draw_through_a_wall`: a ring walk finds a spot with a wall on the eye→guard ray (the
  world segment asserts it), the zoom held at the game's own endpoint (+0x8D4 = the per-weapon cap
  8 [M 0x1a24c8:156-187]) — `camera_brackets` returns exactly the walled-off guard's bracket and
  the overlay draws it. The wet ring spots refuse the aim raise itself (the port's water gate), so
  the sister holds the zoom state instead of the raise; the draw is what it verifies.
- Sisters: `probe_hud_textures` (the texture loads), plus the zoom-raise diagnostics pinned during
  the work (the swap must settle before the aim edge — a raise queued mid-swap is refused and a
  held button never re-edges).

Suite: **1228 passed, 6 failed, 66 ignored (50 min)** on the branch tree — the 6 are the 3 allowed
`disc_golden` tests plus 3 pylon-render probes (another session's 0700000c scope, pre-existing on
master). The final tree (merged with origin/master) runs this task's 10 tests green plus the
targeted `damage`/`spawner` suites.

## The render (for the main agent)

No GPU in the sandbox — `--shot` needs an adapter. Render spots (`nf-view <iso> 07000005 --dev
--give 0x54 --draw 0x54 --zoom-hold --bond x,y,z,yaw,pitch --frame 260 --shot out.png`):

- `--bond 15.0,1.1,16.4,1.57,0` — the office pair by the spawn: each living guard on screen wears
  the green DroneScan box (2×2 world units — the box overshoots a 1.3-unit guard slightly), the
  viewfinder frame + mid brackets around them, all through the office walls.
- `--bond 61.3,27.1,109.5,0,0` — the pool balcony: the guard at (61.34, 27.06, 99.5) brackets
  through the wall between.
- The box rides the guard (re-created per tick), shrinks with distance, and vanishes the tick the
  guard dies (the health gate) or its room's wake plate hasn't fired (WaitSwitch hides it — the
  game hides waiting guards entirely [M `NDrone2_Enable` 0x149dd0]).

## Live checks for the main agent (PCSX2)

1. Zoom the Micro-Camera at a guard on 07000005 and fire a round past him: the bracket stays while
   he lives, vanishes the tick he dies (gdbwatch `Drone+0xac` of that drone: > 0 while bracketed).
2. Aim at a guard through a thin wall: the bracket shows (no LOS gate). gdbwatch the bracket
   object's creation site 0x1a12d8:56 (`control_create_object` call) ticking per frame while the
   pane is up.
3. A waiting (unwoken) guard's room: no bracket and no guard (the hide gate); after the wake plate
   fires, both appear.
4. Whether the game brackets non-enemies too (allies/civilians — the decompile has NO team filter;
   07000005 has none to check — on a level with allies, e.g. 07000006's driver, confirm whether the
   bracket draws; the port currently follows the decompile: it does).

## What's left

- The pane's own sprite 0 (the photo flash, the +0x95b countdown) stays unported — hidden at rest,
  cosmetic, and the port has no photo mode.
- The brackets draw as HUD quads; the game draws world alpha sprites — visually identical at the
  bracket's size, but a main-agent render should confirm the box scale reads right at long range.
