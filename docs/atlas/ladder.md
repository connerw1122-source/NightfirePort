# The ladder (placement type 0x22) and the water/swim substate

Plan v2 §2, struct-11 territory (the wire/grapple atlas, `wire-grapple.md`, documents the shared substate
machinery). Everything here is fix-ladder. Tags: **[M]** decompile/disassembly, **[L]** EE snapshot, **[D]**
disc, **[S]** suggested.

## The ladder object — `Ladder_Create` 0x1966c0 [M disasm 0x1966c0-0x19675c]

Created by `parsemap_create_dynamic_objects` 0x1d04f0 **case 0x22** (`Ladder_Create(translation, rotation,
level_tag, celglist)`), guarded by the dynamic-object flag like the wire. `control_create_object(4, …)` —
**instance size 4 bytes**: `inst+0x00 = 0` (`sh zero,0(v0)` @0x19670c), `inst+0x02 = *(u16*)(level_tag+0x30)`
= **placement param 1** (`lhu v1,48(s1)` @0x196710, `sh v1,2(v0)` @0x196714). Header: `obj+0xFF = 0x2D`
(the class byte — `addiu a2,zero,45` @0x1966fc, `sb` @0x196718), `obj+0xD8 = celglist` (@0x19671c),
`build_LinkToRoom(obj,0,glb_world)`, `Controls_StraddleTest(obj)`. **No parameter reader exists** — no ladder
code reads `inst` back; `channel` is kept in the port for the layout's sake. The rotation quaternion is
passed through **unmodified**: the wire-grapple atlas's "rot.z is negated at create … as for ladders" side
note is wrong for ladders (corrected there).

The world sphere (`obj+0x80..0x8C`, centre + radius) comes from the glist model bounds through the object
matrix — `Control_BuildWorldSph` 0x133070's pattern, the same computation the port's `Bond::sphere_centre`
documents. The model bounds are the glist's centre/radius words (`MapObject.centre/radius`,
docs/formats/level.rs). The ladder's climb range is expressed against this sphere: **top = `cy + r + 0.85`,
bottom = `cy − r + 1.0`** [M `Player_CollLadder` 0x196118 / `Player_Climb` 0x196310 recompute both per tick].

Port: `crates/nf-game/src/ladder.rs` (`Ladder::create` = `Ladder_Create`; `top()`/`bottom()`; `capsule_hits`
= the grab zone). Wiring: `Scene::build` collects `flags & 0xff == 0x22` placements into `Sim::ladders`.

## The grab — `Player_CollLadder` 0x196118 (504 bytes) [M]

Called from `Player_DealWithObjHit` 0x194458 (hit-class dispatch: 0x2C → CollWire, **0x2D → CollLadder**,
0x27 → Hurt) with the HITDATA records `Collide_Update` 0x1ea398 produces from per-object HITTEST shapes
against the player's capsule; `Player_CollisionHandler` 0x1949a8 walks them. Guards: substate `obj+0xF6`
∉ {1, 2}, regrab lockout `BLData+0x951 == 0`, health `BLData+0x894 > 0`. Conditions:

- `|Vec_AngleDifference(player.yaw, ladder.yaw)| > π/2` — the player faces the ladder's climbable face
  (`Vec_AngleDifference` 0x1e2b78 = wrap(second − first) into (−π, π] [M]);
- the y window: `bottom = cy − r + 1.0 < pos.y < top = cy + r + 0.85`, after `pos.y` is raised to
  `bottom + 0.02` (and `obj+0x44` set to the clamped y).

On grab [M]: snap to `centre + fwd(ladder.yaw)·0.5` (x = `cx − sin(yaw+π)·0.5`, z = `cz − sin(yaw+3π/2)·0.5`
= the forward offset); `obj+0x50/+0x58 = 0` (the horizontal velocity); `BLData+0x904 = yaw`;
`Player_ClearInertia`; `Player_ChangeSubState(obj, 1)` (**the climb substate is 1**); mode `+0x94D = 1`;
`+0x130 = 2`; `Player_WeaponNone` (fists); `BLData+0x884 = ladder` (the current-ladder pointer).

**Not input-driven**: walking into the ladder attaches (the use key is not involved) [M].

Port: `Bond::ladder_grab` (crates/nf-game/src/player.rs). The grab zone is the substate arm's capsule
against the ladder mesh's bounds box in the ladder's frame (`Ladder::in_mesh_box`; the shipped ladders
are ±0.354 wide × ±0.060 deep [M glist bounds + 0x2E triangles, 07000002]) instead of the unported
glist triangle refine [S stand-in; the yaw window and y window above are the real gates and are [M]].
PCSX2 pin (2026-10-16): from the 02 hut pocket (−76.27, 3.17, 22.14) a swimmer at yaw 0.4/0.7 passes
the ladder and ≥ 1.3 grabs (`hut_pocket_swim_out_passes_the_ladder_like_the_ps2`).

## The climb — `Player_Climb` 0x196310 (936 bytes, substate 1) [M]

1. Yaw: `diff = AngleDiff(yaw, ladder.yaw + π)` (face the ladder); hard-clamp to ±π/2, then ease
   `yaw += diff·0.2` per tick while AnimObj flag bit 0 is clear (the port eases always [S]).
2. `Player_Move(0, …)` — the movement multiplier 0: no input walk; the local axes decay.
3. `Vec_Zero(BLData+0x50)` — the world velocity zeroes every climb tick.
4. Carry: `pos += ladder(+0xC0) − ladder(+0x40)` (the object-matrix row 3 / previous-position pair the
   riding objects use) — zero for static ladders (no moving-ladder code found [S]).
5. Input/height [M]:
   - action (7) pressed with `forward < 0.05` → dismount with `+0x18 −= 0.05` (a slight backward push);
   - recompute `top`/`bottom`; `pos.y > top` → `+0x18 += 0.6` (a forward lunge onto the platform), dismount;
   - `pos.y ≥ bottom` → climb: `fwd = forward`; a hit above (AnimObj `+0x60 & 0x20`) zeroes an up-push;
     `!(hit below (& 8) && fwd < 0)` → `+0x14 += fwd·0.02` (0.02 units/tick = 1.2 u/s at full push), stay;
     else dismount;
   - `pos.y < bottom` → dismount.
6. Every dismount: `regrab = (u8)FRAME_RATE` (60 — [L 0x30d0d0]), `+0x942 = substate`,
   `AnimObj+0xDC = 0`, then `ChangeSubState(0)` + `AnimObjectUpdate` + the class-'G' (0x47) anim carry
   (`AnimObj[99] = AnimObj[100]`, the same fixup the wire drop uses).

Constants [L ee_start_05.bin 0x30d0c8..0x30d0dc]: `FRAME_RATE = 60.0`, **`FRAME_RATE_DIV = 1.0`**,
**`FRAME_RATE_MUL = 1.0`**, `REC_FRAME_RATE = 1/60`.

The climb-time 0x20/8 flags come from `Player_CollisionHandler`'s substate-1 branch [M 0x1949a8:24-58]: it
scans the hit list, skips ladder-own hits (a hit whose object == `BLData+0x884` is discarded) and flags
`0x20` (hit point above `BLData+0x760`, the capsule-A y) / `8` (below). The port reads them from its own
world collision: `head_bump` for 0x20 and a floor within 1.2 under the feet for 8 [S].

Port: `Bond::climb_tick` + `physics_tick_climb`. The BLData+0x10..0x1F wish vector is **zeroed every tick**
by `Player_Update` (the `obj+0xF4 == 1` branch, [M disasm 0x193828-0x19383c: `Vec_Zero(s2+16)`,
`Vec_Zero(s2+48)`]) — so the climb velocity `+0x14 += fwd·0.02` is a one-tick displacement, and the
±0x18 pushes are one-tick local-forward pushes consumed by the next tick's move (the port's `push_fwd`).

## The water — the cel's surface, the swim substate, the air

- **`cel+0x90` is the water surface.** `build_alloc_cel` 0x1c93f8 (cel size 0xB0) defaults it to
  **−500.0**; `parseentity_fixup_entity` 0x1d00c0 sets `cel+0x90 = cel+0x54` for the flag word **0xC023**
  and to −500.0 for **0xC047**. `cel+0x54` is the **glist bbox max y**: `parsemap_block_map_data_static`
  0x1d1998 `amemcpy(cel+0x40, glist+0x20, 0x20)` copies the bbox (min at +0x40, max at +0x50, so +0x54 =
  max y). A water volume is therefore a cel entity (0xC023) whose box top is the surface [M]. The port:
  `Cel::water` in nf-view's `parse_room_cels`.
- **Enter — `Player_InWater` 0x195e50** [M], called as the FIRST statement of `Player_SSWalk` 0x192310
  (and `Player_SSCrouch`; `Player_HandleDeath` too): `obj+0x20 == 0` (no cel) → false; else
  `pos.y − 0.3 < cel+0x90` → in water: AnimObj `+0x60 |= 0x100`; if not MP and `obj+0xF4 != 2`:
  `ChangeSubState(3)` (**the swim substate is 3**, the same id the atlas lists as "riding" — the case-3
  body in `Player_Update` is the swim, [M 0x193958 case 3]), `+0x94C = 0`, `+0x130 = 2`, `+0x14 = 0`,
  `+0x951 = (u8)(FRAME_RATE_DIV·5)` = 5, `obj+0x50/+0x58 = 0`, the per-tick deltas zeroed,
  `AnimObj+0xDC = 0`. Returns `substate == 3`.
- **Swim** [M case 3]: `Player_Move(1.0, …)` (the walk physics, full speed), then with a cel the wish
  vector is transformed by the object matrix, world-up clamped — `pos.y < surface − 0.3` →
  `vel.y = min(vel.y, surface − 0.3 − pos.y)`, else upward velocity zeroed — and inverse-transformed back.
  Exit: `(AnimObj+0x60 & 8)` (grounded) **and** `surface ≤ pos.y − 0.3` → `+0x130 = 2`,
  `ChangeSubState(0)`.
- **No gravity in water** [M]: `Player_Collision` 0x194df0's default case adds `WldGravity·REC_FRAME_RATE`
  only when `cel+0x90 ≤ obj+0x34` (above the surface); below it, control jumps to the substate capsule
  switch and no gravity accumulates. (`WldGravity` matches the port's −9.8 u/s².)
- **Air — `Player_MonitorAir` 0x191a70** [M], run after every substate: the head bone (bone −5; the port
  uses the eye [S]) against `cel+0x90`: below → air `+0x8C0` drains 5/s (from 100) and the underwater fade
  `+0x8C4` rises; above → refills 20/s; `+0x8C8` bit 0x10 = underwater. At `air < 1.0`, every
  `GameState % (FRAME_RATE·0.5)` = 30 ticks: `Sound_Play(100, 0x60b)` + `Player_HandlePain(0x40000000 = 2.0, type 7)`
  (the drown damage). The HUD air pane (5) and the breath sounds (0x60c / the `+0x89C` loop) are not
  ported this pass.

Port: `Bond::enter_water` / `swim_tick` / `monitor_air` (crates/nf-game/src/player.rs); the Sim applies
`drown_damage` to `drones.health`. Vertical swim input (rise/sink) is the port's jump/crouch [S — the
game's vertical swim control is not resolved; enter/leave/surface/drowning are [M]].

## Disc data [D] (`crates/nf-view/tests/fix_ladder_live.rs ladder_and_water_counts_per_level`)

Ladder placements (type 0x22) and 0xC023 water cels per story level: 07000002 has **4 ladders**
(incl. the bunker exit at ≈(−26.9, 2.3, 42.2), yaw −π, world top 5.96 — the playtest's garden) and
**5 water cels** (surfaces 3.76–4.34); 07000003: 1 ladder; 07000005: 1 (60.9, 29.9, 89.2); 07000008: 4;
0700000a: 1; 0700000c/0d: 3 each; 07000015: 3; 07000016: 1 ladder + 1 water cel; the rest: 0. The EE
snapshot check (07000005's ladder at (60.9, 29.9, 89.2)) was not run — ee_start_05.bin covers the level
start area only [S].

## Snapshot check

Not run against ee_start_05.bin: the ladder instances live where the ladder placements are (07000005's is
at (60.9, 29.9, 89.2) — far from the start), so no live instance is in the snapshot [D probe]. The
`FRAME_RATE` family is verified [L 0x30d0c8].

## Not verified this pass

- The exact per-object HITTEST shape ladders get (`Collide_Pick`/`Collide_Intersect` 0x1ea398 path) — the
  port uses the axis column [S].
- The game's vertical swim input (dive/rise controls) and the AnimObj flag bit 0 (the climb-yaw ease gate).
- `Player_Climb`'s `AnimObj+0xDC = 0` carry and `BLData+0x904` (the saved grab yaw) consumers.
- The ladder/wire instance `+0xC0/+0x40` carry pair for moving ladders (none found in the SP game [S]).
- Climb/wire/grapple view-model anims (the port holsters to fists on grab; the climb anims are not ported).
