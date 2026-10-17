# level-01-interactables — report

Branch `s/level-01-interactables` (from origin/master d628f01f). Task: every interactive thing and
every model on 07000001 (Paris Prelude, Castle A), driven through the real input path.

## What's done

**1. The census** (docs/levels/07000001.md, appended section): all 35 placement types on the level,
from the disc (`nf-extract placements 07000001.bin` per kind [D]) with the creator dispatch of
`parsemap_create_dynamic_objects` 0x1d04f0 [M], each with its placements, the game's semantics
(function + address) and the port status. Headline corrections the census produced:

- **0x2D is not a Lock.** The one 0x2D placement (#104 "PlayerStart") is a second, MP-gated start
  (`Player_AddNewStartPos(…, 0)` behind `MPSettings != 0` [M 0x1d04f0 case 0x2d]). `Lock_Create` is
  case **0x2E** [M 0x1ae1f8] and 07000001 has zero 0x2E placements — no lock test on this level.
- **0xE5/0xE6 are cover nodes** (corner/low, `Drone_CoverCornerNode` 0x136ff0 / `CoverLowNode`
  0x137340), already ported; ThirdCam is 0xE7 and 07000001 has none.
- **The 12 fuseboxes (0x31) never create** — every one carries the 0x2000 template flag [D] and is
  filtered before the dispatch (the port mirrors this; levels_sweep_live pins it).
- **The TouchOnce outs are 15/50/12/38/38/39 with mask 8 = player** (#317 mask 0x10, lock 153)
  [D params] — an earlier note in this repo said "out 0x8"; that was the mask, fixed here.
- The shoji doors are SimpleScripts 0xE2 #16/#17, and **all 16 of this level's SS carry p2 = 2
  (BULLETS only), p3 = 0 (no channel), p4 = 0 (no sound)** [D placements; M SS_Operate 0x1a9df8] —
  E-use and walk-in are masked off; a bullet is the only opener.

**2. Three kinds were draw-only and now simulate:**

- **Break_Create (0x20, 55 placements)** — new `crates/nf-game/src/breakable.rs` [M 0x12c230
  create, 0x12bfd8 update, 0x12c128 apply, 0x12b970 kill]: bullet contacts subtract their damage
  from the param-1 health; health < 0 → `Break_Kill` (obj+0xFE bit 0, glass kind 2 → shatter +
  sound 0x2E, else debris + 0x5F3 — the 41 % 0x5F2 variant is `// [S] unverified`, kept out of
  pass/fail because it needs the game's Rand draw-order context; no channel is ever written).
  The level-01 drone-smashes-glass arm (`Vec_SqDist3D · 20` [M 0x12bfd8:23-34]) is implemented and
  unit-tested but **not yet wired into the drone move** (the API is `Breaks::drone_step`).
  Wired into `Drones::update_player_bullets` next to the props pass; kills queue as
  `Drones::break_kills` for the view's shatter/sound presentation.
- **Rotor (0x71, 2 fans)** — new `Rotor` in worldobj.rs [M rotor_init 0x18f4b8, rotor_update
  0x18f568]: the placement ROT vector is the per-tick spin (0.0349 rad here [D]), the gate
  channel (param 1, 0 here → always spins) stops it; `angle` is the getter the draw code applies.
- **Monitor (0x30, 1 console)** — new `crates/nf-game/src/monitor.rs` [M Monitor_Create 0x1b08b0,
  Monitor_Activate 0x1b0058, Monitor_Update 0x1afcc8]: E-use takes the group sensor's feed
  (state 2), the viewed tick writes the watched sensor's param-8 monitor-view channel and sets its
  +0x60 unhide, the same 0xE edge cycles/exit-cools (9 → 4 → 0), bullets destroy it (state 3 +
  the 0x6000027 blast). The monitor's use position is the placed object's glist-bounds centre (the
  placement sits at the identity transform with world-space verts [D]).

**3. The tests** — `crates/nf-view/tests/interact_07000001_live.rs`, one per KIND, each through the
real input path (walk/aim/E/fire via `step_bond`), each with a sister that fails without the
operation. 10/10 green, ~16 s:

| test | kind | sister |
|---|---|---|
| the_e_key_opens_the_entry_sliding_door | 0xDB door E-toggle | no E → CLOSED |
| a_bullet_opens_the_shoji_ss | 0xE2 (p2 = 2 bullets) | mask pins: no walk/E arms |
| bullets_shatter_the_glass_and_vases | 0x20 ×55 | unshot level: all whole |
| the_armour_pickup_sets_armour_and_writes_channel_13 | 0xF0 take | fresh load: ch 13 off |
| the_shooter_trap_fires_one_bullet_on_channel_13 | 0xD2 | ch off → never fires |
| the_channel_muxes_route_the_touch_and_take_traffic | 0xEE + 0xEC | short AND refuses |
| walking_a_touchonce_plate_raises_its_channel_once | 0xEA (ch 15) | 30 units away: off |
| the_radioroom_fans_spin_while_their_gate_is_off | 0x71 | 120 ticks × 0.0349 rad |
| e_use_on_the_monitor_takes_the_camera_feed | 0x30 | idle writes nothing |
| the_static_world_kinds_all_register | lights/ripples/env/sensor/destroy/spawner/cover counts | exact counts [D] |

Already-covered kinds ride their existing tests (cited, unchanged): 0xE8 exit →
walk_07000001_live, 0xEB AND-plate → walk_07000001_live (Bond-alone sister), 0xFB music →
music_triggers_live, 0xF0 pickups → pickups_g029_live, 0xD9 SP intro ch 0xB → walk_07000001_live.

**What ran:** `cargo test --release -p nf-game --lib` → **267 passed, 0 failed** (new: breakable
×4, monitor ×2); `cargo test --release -p nf-view --test interact_07000001_live
-- --test-threads=1` → **10 passed, 0 failed**; regression on the shared bullet path:
walk_07000001_live (2), walkreal_07000001_live (1, the bit-exact walk), door_portal_live (4),
pickups_g029_live (3), music_triggers_live (2) — **12 passed, 0 failed**. NF_SILENT set by the
harness; no audio device needed.

**Models (item 3):** the parse side is clean on this level — `nf_view::load` reports **0
unresolved** placements, 0 drones with a cross-archive skin, 3 066 236 triangles, 3678 materials
[L the load banner every test prints]. NF_TEX_TRACE is a GPU-pipeline diagnostic (white-fallback
prints in gpu.rs) and needs a wgpu device, which tests never build — the texture-fallback sweep
needs a `cargo run -p nf-view` session and belongs to the main agent's render pass.

## What's left

1. **G-050 (the shoji door drawn black)** — the black door is 0xDB placement **#14**
   (fix-npc-doors-escort.md:49); the door and SS decompiles touch no lights or draw order [M
   0x136090, 0x1a9c58], so it stays a render-side question. Needs the main agent's render + a
   `--bond 5.61,6.47,5.68,8.737,-0.083` frame comparison; suspects: the door's light pair, the
   translucent pass draw order, an escort-driven state change.
2. **RB_Create (0xD8, 4 rigid-body props)** — still draw-only; the physics port (wind, Euler,
   bullet impulses [M 0x1d6e38]) is its own task.
3. **Lightning (0x2C, 11) / the sky tick** — the creator is unported (the sky.rs per-slot machine
   exists but is only driven by headless tests); wiring `Sky::tick_group` into the live Sim is the
   remaining piece.
4. **`Breaks::drone_step` wiring** — the guards-smash-glass arm needs the per-drone move hook in
   `Drones::tick`.
5. **The escort route and its doors** — the walk tests cover the route green (walkreal is
   bit-exact); no new door defect surfaced this pass. The 0xDB secretdoor pair (#224/#225, group
   10, keep-open) is ported but has no dedicated live test yet (one placement test per KIND was
   the bar; the pair rides the door kind).
6. The 41 % break-kill sound variant needs the game's Rand draw-order context (or a live check).

## Live checks for the main agent

- **gdbwatch** on `switch_channels[0]` writes (the monitor-view channel is param 8 = 0 on this
  level [D]): while E-viewing the security TV (#105), every tick should write slot 0 — confirms
  the game really writes ch 0 (vs treating 0 as none). Address: the monitor's `Monitor_Update`
  0x1afcc8 store; access W; expect: 1 per tick while viewing, none when idle.
- **fieldprobe** at `--bond 5.61,6.47,5.68,8.737,-0.083` (G-050): frame ~4503 after the pickup
  route — confirm whether door #14's material is the white-fallback (NF_TEX_TRACE=1 in a
  `cargo run -p nf-view` session) or a lit-but-black quad; expected: the fallback log naming the
  door's material, or no fallback + a shading/draw-order cause.
- **gdbwatch** on the fan object's +0x50 rotation word (comroom_fanblades, placement #80): access
  W per tick; expect +0.0349/tick [M rotor_update 0x18f568].

## Usage

Not visible to me — this sandbox exposes no token, cost or context-window figures; the session's
subagent transcripts report their own prompt/completion splits (e.g. the census research agent:
208 777 prompt / 8 055 completion tokens), but no aggregate for this session exists anywhere I can
read.
