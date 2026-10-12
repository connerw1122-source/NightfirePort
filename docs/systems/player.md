# Bond on foot: movement, collision, camera, aim, crosshair

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp export (work/decomp/ACTION.ELF), [L] level-start snapshot
`work/ee_start_05.bin`, [S] suggested. Source: two research passes (subagents, 2026-10-09); the items marked
"checked" were re-read here. 60 Hz: `FRAME_RATE_MUL` 1, `REC_FRAME_RATE` 1/60 [L].

## Objects

- Player object `obj` (`glb_players` 0x2d88e0): `+0x30` position, `+0x54` yaw (0 faces +Z), matrix `+0x90` rows: row0
  = left (cos y, 0, −sin y), row1 = up, row2 = forward (sin y, 0, cos y) [M, L]. `+0xDC` AnimObj: `+0x60` flags
  (1 aim mode, 8 grounded, 0x10 touched, 0x20 head bump), `+0xCC` root height above the feet cc = 1.0347 [L].
- BLData (`obj+0xE0`): `+0x10` local velocity this tick (x strafe = left, z forward), `+0x20` last tick's, `+0x30`
  rotation delta, `+0x50` world velocity (m/s), `+0x110` floor normal y, `+0x8A8` pitch (units of π/2, ±1),
  `+0x908` eye 0.7 [L, checked], `+0x90C` crouch drop, `+0x928` fall timer, `+0x940` grounded history, `+0x94C`
  jump state, `+0x960` crouch lockout. Start on 07000005 [L, checked]: pos (104.954, 24.9633, 108.8253), yaw −2.14675.

## Per tick [M]

`Player_Update` (0x193600): zero `BL+0x10` / `+0x30`, save the position; the substate handler (walk
`Player_SSWalk` 0x192310 / crouch `Player_SSCrouch` 0x1924c0: `Player_Move`, `Player_HandleJump`, rotate the matrix);
aim and pitch (`Player_Aiming`, `Player_ViewClamping`); position += row0 · v.x + row1 · v.y + row2 · v.z; then
`Player_Collision` (0x194df0: gravity, position += velocity / 60, the capsule). `Collide_Update` pushes the capsule
out of geometry; `Player_CollisionHandler` (0x1949a8): the floor ray first, then position += the summed push; grounded
and not jumping → vertical velocity 0. The camera (`Player_PositionCamera`) last.

## Moving — `Player_Move` (0x191dd0) [M, checked]

- Targets from the actions (−1…1): forward a2 > 0 → 0.1 a2² + 0.005; a2 < 0 → −0.075 a2² − 0.005; strafe 0.075 a1.
- Each axis: v = last + (target − last) · 0.2, no overshoot, 0 when |target| ≤ 0.0002 and |v| < 0.01, clamped to
  ±0.1, × k (walk 1.0, crouch 0.75). Top speeds: forward 0.1 / tick (6 m/s), back 0.08, strafe 0.075; ~14 ticks
  to full speed, ~11 to stop.
- Aim mode (AnimObj flag 1) zeroes turning and forward: only strafing.
- Turning (pad): `AccelFunc0(a0, 0.04, 2, 120, 0.05, 0.95)` rad / tick (input.md §7). Mouse: D-002 (PLAN.md).

## Collision — capsule [M]

- Radius 0.55. Standing: top A = pos + 0.275 up, bottom B = pos + (0.55 − cc) up. Crouched: A = pos + (0.65 − cc),
  B = pos + (0.45 − cc) (`Player_Collision`, checked: 0.55, `0.1 − (cc − 0.55)`, `0.45 − cc`).
- **cc is dynamic** (fix-truck): the root height lives at **AnimObject+0xCC** (obj+0xDC+0xCC — standing
  1.0347183 [L]; the feet-point math reads it there [M `Player_Collision` 0x194df0:86, 222-236]) and the
  anim/stance writes it. The every-tick copy `Player_Update` 0x193600:449-453 / `Player_CollisionHandler`
  0x1949a8:179-183 performs is `obj+0xC0..0xCC = obj+0x30..0x3C` — the MATRIX ROW 3 (translation)
  mirroring the position, gated on flags2 bit 4 — **not** a root-height write (fields-player correction;
  the earlier `obj+0xCC = obj+0x3C` reading mistook it). `obj+0x3C` is the placement vec4's W riding in
  the position (−0.9875 on 07000005 [L]), written only by the whole-position copies (`Player_StandAtNewPosition`
  0x190d88 takes it from the placement vec4's W; `Player_Enable` 0x195498 copies the free script's matrix
  translation row — including its W — onto obj+0x30..0x3C when a cut-scene hands Bond back). The crouched
  ride parks at floor + 0.607 in the 05-truck trace, i.e. cc ≈ 0.556 after the 0.051 rest offset — the
  port models the two stances as `ROOT_HEIGHT` / `CROUCH_ROOT_HEIGHT` = 0.55 [L trace-derived, ≈ the
  capsule radius]. The port's capsule/floor-ray code reads `Bond::cc`; the crouch toggle swaps it.
- `Intersect_CylGeom` (0x109da0): triangle by triangle, the segment point meeting the plane (clamped), the triangle's
  closest point to it (re-taken on edges), closer than r → pushed out by r − d; each push moves the capsule before the
  next triangle; a walkable face (normal y > 0.7) touched below B marks "standing". No explicit step-up: low edges
  push up as the capsule rides over them.
- Floor ray (`Player_FeetOnPoint` 0x194188): from pos down cc + 0.6; grounded when distance − 0.1 ≤ cc or the
  capsule stood on a walkable face. No snap: Bond rests where he stops in the 0.1 band (live 1.0891 above the floor).

## Gravity, jump, fall damage [M]

- Not grounded, or floor normal y < 0.5: velocity += (0, −9.8, 0) / 60; jumping keeps it; else 0. |vy| ≤ 45;
  position += velocity / 60.
- Jump (action 7, grounded within the last 4 ticks, no head bump, clear line from pos to the previous pos + 0.95 up):
  velocity = (0, 3.92, 0), state 1, landing delay 4 ticks; head bump → 0, state 2; landing after the delay when
  grounded and vy ≤ 0. Sound 0xA3.
- Fall timer +1 per tick while not grounded for 4 ticks and falling; landing with timer > 60 → damage timer − 60 if
  ≥ 10 (`Player_HandlePain` type 6).

## Crouch [M]

Action 8 (toggle by default, `PlayerSetting+4` = 1 [L]); lockout 60 ticks; standing up needs a clear line from pos to
the previous pos + 0.95 up; speed × 0.75; the eye drops 0.45 at 0.0225 / tick (20 ticks).

## Camera [M, L checked]

Eye = pos + (0, 0.7 − crouch drop, 0) (`Player_GetHeadPos` 0x195aa0); orientation = object × RotX(−pitch · π/2);
pitch clamped ±90°; no head bob. Projection [L]: 512 × 448 screen, fx 332.62, fy 388.06, vertical FOV 60° (1.0472),
aspect 4/3, near 0.1.

## Aim and bullets [M]

- Bullet origin = the eye; aim point = eye + 256 · (−tx, −ty, 1) through the view (tx = 0.76966 cx, ty = 0.57724 cy,
  cx / cy the smoothed crosshair offset `BL+0x120 / +0x124`); the first world hit replaces it (same ray).
- Aim mode (action 0x13 held): cursor `BL+0x118` += stick · 0.1275 · 0.125 (x), 0.136 · 0.125 (y), box ±0.296 /
  ±0.4; edge-turn ramps. Smoothed cursor += (raw − smoothed) · 0.5. The P2K has no zoom (`weapon+0x88` = 1).
- Auto-aim (`PlayerSetting+1` = 1 [L, checked — on]; D-001: off by default with a mouse): the crosshair snaps onto
  the target's bone 2, offsets halve each tick.
- No view recoil for the P2K (`Player_WeaponRecoil` never changes the view); the fire script moves the gun model.

## Crosshair and HUD [M, L]

`HUD_UpdateCrossHair` (0x19e6d0): sprite from sheet 0x03000026 (in every level archive), cell from `HUDCrossCoords`
(0x2bec48: 1 = aim 29 × 29 at (1, 2), 2 = normal 11 × 11 at (10, 11), 3 = friendly 14 × 13 at (31, 1)), centred at
(256 (1 + cx), 224 (1 − cy)), red 0xFF0000 at alpha 0x7F; hidden for scoped weapons and the third-person camera.

## First-person gun [M, L]

Skin `weapon+0xDC` (P2K 0x05000088); scripts idle 0x06000825, reload 0x06000827, fire 0x06000823, fire-last
0x06000824, draw 0x0600082B, holster 0x0600082A; offset `weapon+0xE0` (0.0283, 0.0755, −0.9026) + bob +
(0, 0.2, −0.5), pivoting about (0, −0.2, 0.5) in view space toward the crosshair; muzzle flash mesh `weapon+0x48`
(0x020001F6) for 3 ticks, light radius 7, colour (255, 138, 0).

## Weapon switching [M Player_WeaponChange 0x1a6f80, Player_SetWeaponAnimObj 0x1a2e38]

- `Player_WeaponChange(obj, dir, mode)`: iterates weapons 1..0x72 from `weapon + dir`, wrapping. A weapon is
  selectable when `weapon_data[w]+0x04 == 1`, `BLData+0x1BA + w·0xC` (owned) is non-zero, and
  `Player_WeaponHasAmmo(BLData, w)` is non-zero. Sets `anim+0x63` (the switch request); `anim+0x64` saves the
  previous weapon for toggle-back.
- `Player_SetWeaponAnimObj` state machine (`anim+0xF4`): state 0 (idle) sees `anim+0x63 ≠ anim+0x62` → starts the
  holster (`weapon+0xC4`, sound 0x280 for gadgets / 0x283 for weapons). State 1 (holstering): when the
  script stops, swaps `anim+0x62 = anim+0x63`, writes the old clip/spare back, reads the new, starts the draw
  (`weapon+0xC0`, sound 0x281 / 0x282). State 3 (drawing): when stopped, returns to idle (`weapon+0xA0`).
  Firing and reload are blocked while switching.
- Sounds: 0x280 holster (gadget, id 0x4A..0x5E), 0x281 draw (gadget), 0x282 draw (weapon), 0x283 holster (weapon),
  0x1DB the change click. The gadget/weapon test is `id - 0x4A < 0x15` (i.e. id ∈ [0x4A, 0x5E]) [M disassembly
  0x1a3704: `addiu v0,v0,-74; sltiu v0,v0,21`]. Per-weapon op-1 fire-sound id remap (docs/systems/sound.md §7):
  the script's id is replaced by the weapon's (1→2 for w7/9, 0x403→2 for w3/5, 0xFA→0x1FB for w0x14,
  0x106→0x284 for w0x1E..0x23, 0x1E1→0x602 for w0x10, 4→0x604 for w0x16/0x17).

## Port

`nf_game::player` (WORKLOG 0042): movement, capsule, floor ray, gravity, jump, crouch, eye; nf-view drives it with
WASD / mouse (no mouse aim mode, D-002), Space jump, C / Ctrl crouch, T auto-aim, N the free camera; draws the game's crosshair and HUD (docs/systems/hud.md). Open: U-037.

`nf_game::gun` (WORKLOG 0078): the trigger + weapon switching — `Gun::next_weapon` iterates owned, selectable
weapons with ammo (±1, wrapping 1..0x72). nf-view's `Sim::run` runs the holster/draw state machine
(`Player_SetWeaponAnimObj`): state 0→1 (holster, sound 0x280/0x283) → swap weapon byte → 3 (draw, sound
0x281/0x282) → 0 (idle). The trigger and reload are blocked while switching. The view model: one character per
weapon with a first-person skin (`Scene::view_models`); `animate` selects the active one from `sim.gun.weapon`.
The op-1 sound id remap is ported (`remap_op1_sound`). Controls: mouse wheel = next/prev, 1–7 = direct select,
Q = prev. Open: the auto-switch on pickup (`Player_IsBetterWeapon`), the toggle-back key, the weapon name HUD
label.
