# World objects: InverterTrigger (0x41), Shooter (0xd2), DynamicObject (0x3b), ThirdCam (0xe7)

The placement kinds `docs/systems/levels.md` §7 listed as unhandled that give gameplay through the
`control_funcs` update pass (`control_movement_object_handler` 0x1336f8 pass 4). Ported in
`crates/nf-game/src/worldobj.rs` (created in `Drones::attach_level`, ticked after `useobjs.tick`).
All four creators draw nothing from the shared RNG (each create's decompile is free of `Rand_`
calls), so creating them does not move the stream. The placement-parameter mapping follows
docs/fields/level-tag.md (param k = level_tag+0x2C+4k; header +0x02 key count = placement+0x40,
+0x08 key-list pointer = `m_paths[placement+0x44]`).

## InverterTrigger — placement 0x41, class 0x53

| Item | Value | Proof |
|---|---|---|
| Creator | `InverterTrigger_Create` 0x1b3040: `control_create_object(4, …)` (no position — the placement transform is ignored), `+0xFF = 0x53`, `+0xF4 = p0` (u16), `+0xF6 = p1` (u16) | [M 0x1b3040] |
| Update | `InverterTrigger_Update` 0x1b30a8: `if (switch_channels[+0xF4] && +0xF6) switch_channels[+0xF6] = 0;` — while the IN channel is set, the OUT channel is held low | [M 0x1b30a8] |
| Params | p0 = in channel, p1 = out channel | [M level-tag.md row 0x41] |
| Instances | 07000006 (0xb1→0x37), 07000012 (0x49→0x87), 07000014 (0xe→0xb8, 0x85→0x6f, 0x85→0x22, 0x85→0xd1) — read off the port's parsed placements | [D placement scan] |
| Port | `worldobj::Inverter.tick` — `channels.set(out, 0)` through the port's `Channels` (the 0 write does not stamp `time`, matching the game's raw store) | port |

## Shooter — placement 0xd2, class 0x4e

| Item | Value | Proof |
|---|---|---|
| Creator | `Shooter_Create` 0x17d030: `control_create_object(0, pos, quat)`, `+0xD8 = glist`, `+0xFF = 0x4e`, `+0xF4 = p0` (u16 arm channel), `+0xF6 = p1` (u16 weapon id), `build_LinkToRoom` | [M 0x17d030] |
| Update | `Shooter_Update` 0x17d0c0: while `switch_channels[+0xF4]`: `Mat_GetDir(&dir, obj+0x90)`; if the weapon record's class halfword `weapon_data[w·0x10c+2] == 0x3b` the direction becomes (0, −1, 0); `Bullet_init(0, 0, obj, weapon_data+w, obj+0x30, &dir, 0xffff)`; then `obj+0xFE |= 1` | [M 0x17d0c0] |
| One-shot | obj+0xFE bit 0 is the delete-me flag — the movement handler's delete pass removes the object the same tick (`Effect_RemoveSObjs` + `control_delete_object` 0x1336f8:86-87): the shooter fires exactly one bullet on the arm edge and is gone. (Same pattern as `Emitter_Update`/`Pickup_Update`'s bit-0 sets.) | [M 0x1336f8:84-87] |
| Bullet | `Bullet_init` 0x12c5a0 (the drone bullet factory the port already models in `crate::bullet`): the record's spread sphere (+0x24, flags +0x68), damage +0x0C, range +0x1C, speed +0x20 | [M; port bullet.rs] |
| Instances | 07000001 (pos [13.36, 6.31, 2.35], dir +z, weapon 0x36 class 0x36, arm 0xd); 07000015 ([35.52, 43.57, 58.87], weapon 0x34 dmg 100, arm 0xfb); 0700004a ×2 (weapon 0x3d, class **0x3b** → the straight-down case, dmg 2000, arms 0x22/0x4f) | [D placement scan] |
| Port | `worldobj::Shooter.tick` — one bullet (spread via `bullet::spread`, the shared RNG draws the game makes inside `Bullet_init`), then `fired = true` stands in for the deletion | port |

## DynamicObject — placement 0x3b, class 0x52

| Item | Value | Proof |
|---|---|---|
| Creator | `DynamicObject_Create` 0x18dbf8: `control_create_object(0xC, …)`, `+0xD8 = glist`, `+0xFF = 0x52`, `obj+0xF0 = *(u32*)(glist+0x44)` (the draw-flags word, docs/fields/glist.md), `| 0x30` when p0 ≠ 0; instance (obj+0xE0 shorts): [0] = p0, [2] = p1, [4..6] (u32) = the obj+0xF0 word, [8] = p2 (u16), [10] = p3 (u8) | [M 0x18dbf8] |
| Update | `DynamicObject_Update` 0x18d3b8: (1) if p0 ≠ 0: `switch_channels[p0]` off → inert return; on → `obj+0xF0 = instance[4..6]` (restore the glist base word). (2) mode byte p3 == 0: p1 armed → `obj+0xFE |= 1` (deleted by the same tick's delete pass); p1 off or 0 → fall through. mode ≠ 0: p1 armed → `obj+0xF0 |= 0x30`, else restore the base word. (3) distance fade: if p2 ≠ 0 and `Vec_Dist3D(obj+0x80, glb_viewer+0xC0) < (int)p2`, `obj+0x106 = (int)(clamp(d/p2 − 0.5, 0, …) · 2 · 255)` — the alpha byte falls from 255 at the radius to 0 at half of it | [M 0x18d3b8] |
| obj+0x80 | The world sphere centre the fade reads: the movement pass rebuilds it from the object matrix and the glist sphere (`Control_BuildWorldSph`); the port uses the placement translation + the rotated glist centre | [M 0x1336f8 pass 6; port stand-in documented] |
| Instances | 07000002/03 (one each: chB 1, mode 0 — a prop deleted when ch 1 fires); 07000005 ×2 (chB 0x71, mode 1, glist base 0x10 — flags gain 0x30 when ch 0x71 arms); 0700000c ×5 (two with p2 = 9, no channels — pure distance fades); 07000014 ×45 (channel PAIRS: chB 0x51 + chA 0x51, chB 0x5c + chA 0x5c, chB 0x78 + chA 0x78 … the deleter and the gated prop of one channel) | [D placement scan] |
| Port | `worldobj::DynObject.tick` — the gate, the mode-0 delete (retained out of the list), the mode-1 flags, the fade into `alpha` (renderer consumption is a later task's concern; the state is kept) | port |

## ThirdCam — placement 0xe7, class 0xC

| Item | Value | Proof |
|---|---|---|
| Creator | `ThirdCam_Create` 0x1bc510: `control_create_object(0xC, …)`, instance: [0] = p0 (u16 camera id), [4] (u32) = the level_tag +0x08 key-list pointer, [8] = the level_tag +0x02 key count, [10] = p1 (u8); `ThirdPersonCameras[cam_id] = obj` | [M 0x1bc510] |
| Update | none — the object registers; `ThirdCam_GetPos` 0x1bc590 is called by the camera side: `Spline_Interp(keys, count·t, 0, count, m, 0, 2)` (the key-list spline, all neighbour indices clamped into [0, count] per Spline_Interp 0x1e1858:93-142) | [M 0x1bc590, 0x1e1858] |
| Keys | the 0x19 framelist (`m_paths[placement+0x44]`), the same 28-byte KEYED_POS records doors parse | [M level-tag.md header +0x08] |
| Instances | 07000002 (cam 3, 3 keys), 07000005 ×4 (cams 1/2/3/5 — the wire crossing's cameras), 07000008 (cam 13), 0700000c ×4 (cams 1/2/3/10), 0700000d, 07000041; p1 = 1 everywhere scanned | [D placement scan] |
| Port | `worldobj::ThirdCam { cam_id, keys, key_count, flag }` + `get_pos(t)` riding `door::interp` (Catmull-Rom + slerp, `last` = the final key index). The Sim has no camera consumer — the registration and the spline ride are the port's deliverable; wiring a camera to `cam_id` is render-side, left with a live check below | port |

## What remains (not ported here)

- **0xd8 RB / `RB_Create` 0x1d6bf0** (crate-and-barrel rigid bodies; 01/03/04/06/0d/41): read for the
  next session — `control_create_object(0x160, …)`, `+0xFF = 0x1A`, `RB_Init(rbody, p2)` (the type
  halfword at rbody+0xA8: 0 box / 2-4 sphere radii from the glist bounds / 3 none), extents from the
  glist bounds (`max−min` at +0x30−+0x20, +0x34−+0x24, +0x38−+0x28) with the X width from p0
  (default 3.0), `RB_CuboidMomentsOfInertia` + `RB_Build` (0x2d8890 box / 0x2d6200 sphere verts),
  `RB_SetState`, p3 (u8) → rbody+0x154, p4 → rbody+0x158 (0 → 0xffff). The step is `RB_Update`
  0x1d6e38 + `RB_EulerEvolve` 0x1d7810 + `RB_ApplyImpulses` 0x1d6ff8 + `RB_RestingContact`
  0x1d7168/0x1d73a0 + `RB_CalcTorque` 0x1d7658 + `RB_ApplyWind` 0x1d7c78.
- **0xda Car / `Car_Create` 0x1305b0** (07000046/4b): the tank/car path (NumTanks, `Car_InitBits`,
  the MP gate `MPSettings+0x460`, the 0x0700046b refusal) — driving-adjacent, out of this session.
- The **renderer side**: obj+0xF0 bit 0x10 hiding, obj+0x106 alpha, the ThirdCam camera wiring.

## Live checks for the main agent

1. `--bond` at the 0700004a space-laser gun ([3.04, −48.40, −1.67], facing +z): poke switch channel
   0x22 to 1 — a 2000-damage straight-down beam bullet should spawn and the gun should fire once
   (not repeat). Access: channel write; expected: one hit.
2. `--bond` at 0700000c near a p2 = 9 DynamicObject: approach it — the object should fade out
   (obj+0x106 falling to 0 within 4.5 units). Render check for the alpha byte consumption.
3. 07000005's wire crossing: the camera that rides the crossing should be one of cams 1/2/3/5
   (`ThirdPersonCameras[id]`); the render should follow the spline, not the shoulder cam.
