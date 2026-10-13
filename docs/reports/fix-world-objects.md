# fix-world-objects — report

Branch `s/fix-world-objects` (from origin/master 03ad7f5). Time box: this session only (sandbox
until ~11:18 UTC). Everything below cites the decompile; the port's behaviour is pinned by NF_ISO
tests.

## What's done

The four light gameplay families from `docs/systems/levels.md` §7's "unhandled placement types"
are ported in a new file, `crates/nf-game/src/worldobj.rs`, created in `Drones::attach_level`
(placement order) and ticked after `useobjs.tick` (the game's `control_movement_object_handler`
0x1336f8 pass-4 update order):

1. **InverterTrigger (0x41, class 0x53)** — `InverterTrigger_Create` 0x1b3040, `Update` 0x1b30a8:
   while the IN channel (p0) is set, the OUT channel (p1) is forced to 0. Instances: 07000006
   (0xb1→0x37), 07000012 (0x49→0x87), 07000014 (0xe→0xb8; 0x85→0x6f/0x22/0xd1).
2. **Shooter (0xd2, class 0x4e)** — `Shooter_Create` 0x17d030, `Shooter_Update` 0x17d0c0: on the
   arm channel (p0) it fires one `Bullet_init` 0x12c5a0 bullet with weapon p1 along the matrix
   forward — or straight down (0, −1, 0) when the weapon record's class halfword `+0x02 == 0x3b` —
   then sets obj+0xFE bit 0, which the delete pass (0x1336f8:86-87) honours the same tick: the
   shooter is a one-shot emitter. The 0x3b case is REAL: 0700004a's two guns carry weapon 0x3d
   (class 0x3b, damage 2000). 07000001 (weapon 0x36, arm 0xd) and 07000015 (0x34, arm 0xfb) fire
   along their placement rotation.
3. **DynamicObject (0x3b, class 0x52)** — `DynamicObject_Create` 0x18dbf8, `Update` 0x18d3b8: the
   gate channel p0 (off → inert), the action channel p1 with the mode byte p3 (mode 0: armed →
   obj+0xFE bit 0 → the prop is deleted that tick; mode 1: armed → obj+0xF0 |= 0x30, else the
   glist base word is restored), and the p2 distance fade writing the alpha byte obj+0x106
   (255 at the radius → 0 at half of it) from the world sphere to `glb_viewer+0xC0`.
   Instances: 07000002/03 (one each, chB 1, mode 0), 07000005 ×2 (chB 0x71, mode 1, glist 0x10),
   0700000c ×5 (two pure fades, p2 = 9), 07000014 ×45 (channel PAIRS — the chB deleter and the
   chA gated prop of one channel), 07000007/0d/16/1b/41 per the sweep.
4. **ThirdCam (0xe7, class 0xC)** — `ThirdCam_Create` 0x1bc510: registers
   `ThirdPersonCameras[cam_id]` with its 0x19 key-list spline (`m_paths[+0x44]`, count from
   +0x40); `ThirdCam_GetPos` 0x1bc590 rides the spline (`Spline_Interp`, neighbour indices
   clamped into [0, count] per 0x1e1858). The port stores the four fields and exposes
   `get_pos(t)`; no Sim camera consumer exists yet (render-side, live check listed).

The sweep census (`levels_sweep_live.rs` HANDLED) now covers 0x3b/0x41/0xd2/0xe7, and
`docs/systems/levels.md` §7.1 carries the additive update note.

## How it was verified

- **Fail-first**: every assertion reads `sim.drones.worldobjs`, which did not exist before this
  branch (the kinds were skipped by every creator — no compile pre-fix), and each asserted
  behaviour (a shooter bullet, a channel forced low, a prop deleted on its channel, the fade) had
  no ported writer at all. The probe that surfaced the per-level instance data ran on the ported
  tree only.
- `crates/nf-view/tests/worldobj_live.rs` (5 NF_ISO tests, real `nf_view::load` → `Sim::step_bond`
  path), all green:
  `shooter_0700004a_fires_straight_down_on_the_arm_edge` (sister: no arm → no bullet; the bullet
  on the gun's column, below the muzzle, damage 2000; one shot, no repeats),
  `inverter_07000006_forces_the_out_channel_low` (sister: IN off → OUT survives),
  `dynobj_mode0_deletes_on_its_action_channel` (07000002 + 07000014's pairs; sisters: channel off
  → present), `dynobj_0700000c_distance_fade` (far → 0xFF untouched, nearer → falling),
  `thirdcam_07000005_registers_the_camera_splines` (4 cams, ids 1/2/3/5, key counts match, the
  spline translation moves).
- **Full suite before push**: `NF_ISO=… cargo test --offline --release -j2 -p nf-game -p
  nf-formats -p nf-audio -p nf-view -- --test-threads=1 --no-fail-fast` → **630 passed / 3 failed
  — the 3 allowed `disc_golden`** (slim ISO), 0 ignored beyond the pre-existing set.
- Determinism: none of the four creators draws RNG (each create's decompile checked); the only
  stream draws are the shooter's in-`Bullet_init` spread draws, which the game makes too, at the
  same game-ordered point (the update pass).

## Shared-file deltas (small, additive — per the task's list)

- `crates/nf-game/src/drone.rs`: one struct field (`worldobjs`, own line), its `Default::default()`
  in `Drones::new`, the create call in `attach_level`, one tick call after `useobjs.tick`. No
  existing line re-written (the merge-lesson-4 form).
- `crates/nf-game/src/lib.rs`: the `pub mod worldobj;` line.
- `crates/nf-view/tests/levels_sweep_live.rs`: the HANDLED census list extended in place.

## What's left (for the next message on this branch)

- **0xd8 RB (rigid bodies)** — the decompile reading is already in `docs/atlas/world-objects.md`
  ("What remains"): creator 0x1d6bf0 (class 0x160, RB_Init type p2, glist-bounds extents with the
  p0 width override, RB_Build box/sphere), step = `RB_Update` 0x1d6e38 + `RB_EulerEvolve` 0x1d7810
  + `RB_ApplyImpulses` 0x1d6ff8 + `RB_RestingContact` 0x1d7168/0x1d73a0 + `RB_CalcTorque`
  0x1d7658 + `RB_ApplyWind` 0x1d7c78. Needs its own session (the integration must be the game's,
  not a generic engine).
- **0xda Car** (`Car_Create` 0x1305b0, 07000046/4b): driving-adjacent (NumTanks/Car_InitBits/the
  MP gate); skipped this session.
- **Renderer consumption**: obj+0xF0 bit 0x10 hiding, the obj+0x106 alpha byte, and wiring a
  camera to `ThirdPersonCameras[cam_id]` — visible work needing the main agent's renders.
- The shooter's placement-rotation forward is the port's RotZYX convention (`object_matrix`
  column 2); if the live render shows a mis-aimed 07000001/15 gun, the matrix-source question
  (quaternion vs Euler in control_create_object) is the first thing to check.

## Live checks for the main agent

1. `--bond` near the 0700004a gun at [3.04, −48.40, −1.67] (facing +z): poke switch channel 0x22
   to 1 — expect ONE 2000-damage straight-down beam and no repeats (the gun self-deletes).
2. `--bond` at 0700000c walking toward a p2 = 9 DynamicObject: expect the object to fade out as
   the camera closes inside ~4.5 units (the obj+0x106 alpha consumption).
3. 07000005's cable crossing: the ride's camera should follow one of cams 1/2/3/5's splines
   (`ThirdPersonCameras[id]`), not the shoulder cam.

## ntfy

Sent after this report (see the WORKLOG tail for the result line if it failed).
