# Player physics — `WldGravity`, the per-level gravity vector

The port carried a hard-coded `GRAVITY: f32 = −9.8` in `crates/nf-game/src/player.rs` (fix-world-gravity).
The game has no such constant: every physics consumer reads the global vector **`WldGravity`**
(0x2d8890..0x2d889c, 16 bytes [M `work/action-symtab.tsv`: `002d8890 16 1 1 4 WldGravity`]).
Bond's jump/fall is one of its readers (`Player_Collision` 0x194df0), and the death-anim pick
reads its y (`Player_ChangeSubState` 0x190630 case 0xd/0xe).

## Where the value comes from

`WldGravity` has exactly **two writers** in ACTION.ELF (grep `WldGravity` / `DAT_002d8890..8` — no
writer in DRIVING.ELF):

1. **Boot** [M `bootup` 0x1c7e78:67]: `WldGravity = 0` (the pre-load state; the EE snapshots never
   show it — every level load runs 2 below first).
2. **Every `Player_Init`** [M 0x1910f8:152-172, disasm 0x19151c-0x19161c — the decompile's stores
   verified in the disasm]: first the unconditional default write
   - `0x2d8890 (x) = 0` (`sw zero, −30576(a0)`, a0 = 0x2e0000, 0x191540),
   - `0x2d8894 (y) = 0xc11ccccd = −9.8f` (`lui at,0xc11c; ori at,at,0xcccd; mtc1 at,f0; swc1 f0,4(s0)`, 0x19151c-0x191548),
   - `0x2d8898 (z) = 0` (`sw zero,8(s0)`, 0x191550),

   then a switch on `*(u32*)(param_4 + 0x38)` — **param 3 (p3) of the player-start placement's
   0xB0 `level_tag` block** (`Player_Start` 0x190fd0 passes `paramblock@0x2ba1c4 + i·0xe0`; the
   read is the disasm's `lw a2,56(s5)` 0x191580; docs/fields/level-tag.md's +0x2C+4k param map):

   | p3 | arm | WldGravity after |
   |---|---|---|
   | **2** | `Player_StandAtNewPosition(…, 8)` + anim 0x6000a00, then **`Vec_Zero(&WldGravity)`** (`jal 0x1e2d58`, a0 = s0 = 0x2d8890, 0x191618) | **(0, 0, 0) — zero gravity** |
   | 1 | pose 3 + anim 0x6000a00 | (0, −9.8, 0) |
   | 3 | pose 4 + anim 0x60009ff + `BL+0x90c = 0x3ee66666 (0.45)` | (0, −9.8, 0) |
   | 0 / ≥4 | pose 0 + anim 0x6000a00 | (0, −9.8, 0) |

   The kind byte that makes the start selectable is written at registration: the dispatcher's
   type-0x24 case calls `Player_AddNewStartPos(…, kind=1, …)` [M `parsemap_create_dynamic_objects`
   0x1d04f0:96; 0x2d starts get kind 0 and `Player_Start`'s `+0x22 == 1` filter skips them, :159].

Everything else — `Player_Collision` 0x194df0:107/161 (the fall, `vel += WldGravity ·
REC_FRAME_RATE`, REC_FRAME_RATE = 1/60), `Player_ChangeSubState` 0x190630:68-73, `Player_HandleDeath`
0x190bb0:15 (zero-g nudges the corpse +0.001 per axis), `RB_EulerEvolve` 0x1d7810:85 (rigid bodies
copy it), `Bullet_update` 0x12e7b0:256 (grenades), `Bullet_casing_update`, `Debris_Update`,
`Pickup_Update`, `gases_smoke`, `Car_Update`, `Drone_CollisionHandler`, `UpdateAllShards` — reads.

## The death-anim rule

`Player_ChangeSubState` 0x190630 case 0xd/0xe [M :68-73]: `WldGravity.y == 0.0` (the exact float
test — the zero-g write is `Vec_Zero`, and any non-zero y, e.g. −0.001, is "ground") plays anim
**0x60008e0** (the space death), else **0x6000151** (the ground death). The port exposes this as
`Bond::death_script()` (crates/nf-game/src/player.rs) for the death-anim branch.

## The per-level table (the disc's data)

From every `0700xxxx.bin` archive on the disc (36 parse; the port's `Level` parser, the same
placement records the loader expands): the type-0x24 player starts' **p3** per level. Rule output:
p3 == 2 → (0, 0, 0); anything else → (0, −9.8, 0). [L ee_start_05.bin / ee_start_07000001.bin /
ee_start_0700000a.bin / ee_snapshot.bin @ 0x2d8890..9c all read (0, −9.8, 0) — consistent.]

| Level | starts (placement # = p3) | WldGravity.y |
|---|---|---|
| 07000001 | #0 = 0 | −9.8 |
| 07000002 | #0 = 0 | −9.8 |
| 07000003 | #0 = 0 | −9.8 |
| 07000004 | #0 = 0 | −9.8 |
| 07000005 | #0 = 0, #0 = 0 | −9.8 |
| 07000006 | #176 = 0, #178 = 0, #178 = 0 | −9.8 |
| 07000007 | #0 = 0, #0 = 0 | −9.8 |
| 07000008 | #0 = 0, #0 = 0 | −9.8 |
| 07000009 | #62 = 0 | −9.8 |
| 0700000a | #0 = 0 | −9.8 |
| 0700000b | #150 = 0 | −9.8 |
| 0700000c | #0 = 0 | −9.8 |
| 0700000d | #0 = 0 | −9.8 |
| 07000011 | #117 = 0 | −9.8 |
| 07000012 | #0 = 0 | −9.8 |
| 07000013 | #7 = 0 | −9.8 |
| **07000014** | #284 = **3**, #284 = **3** | −9.8 (the p3==3 arm: pose 4 + BL+0x90c = 0.45) |
| 07000015 | #137 = 0 | −9.8 |
| 07000016 | #163 = 0 | −9.8 |
| **0700001b** | #112 = **2** | **0.0 — zero gravity (the spacewalk)** |
| 07000024 | #149 = 0 ×3 | −9.8 |
| 07000025 | #3 = 0 | −9.8 |
| 07000027 | #64 = 0 | −9.8 |
| 07000028 | #0 = 0 | −9.8 |
| 07000029 | #64 = 0 | −9.8 |
| 07000041 | #0 = 0 | −9.8 |
| 07000043 | #1 = 0 | −9.8 |
| 07000046 | #1 = 0 | −9.8 |
| 07000049 | #1 = 0 | −9.8 |
| 0700004a | #43 = 0 | −9.8 |

No type-0x24 start (no `Player_Init` on the SP path → the vector keeps its previous value; the
port's load leaves the constructor default (0, −9.8, 0)): 07000023 and 07000026 (the MP arenas —
`MP_Start`'s own `Player_Init` hands the spawn block, out of this task's SP scope [S]), 07000048,
0700004b, 07000500, 07000999. 07000023 having none matches mp-core's WORKLOG note.

**0700001b is the disc's only zero-g level** (the spacewalk; p3 == 2 on its single start).

## The port

- `Bond.world_gravity: [f32; 3]` (crates/nf-game/src/player.rs), constructor default (0, −9.8, 0)
  — the game's default write; `Bond::death_script()` implements the 0x190630 rule.
- The fall (`Bond::physics_tick`) reads `self.world_gravity[1] / 60.0` — bit-exact `GRAVITY/60`
  on every −9.8 level, 0 on 0700001b. The jump launch is NOT gravity-derived in the game
  (`Player_HandleJump` 0x1916b0:169-183 writes the facing-scaled launch with a hard −0.4f lane —
  the port's `−GRAVITY·0.4` stays a constant, unchanged).
- The load path (nf-view `load_impl`, the `Player_Start`/`Player_Init` analogue) sets
  `sim.bond.world_gravity` from the PICKED start's p3: `p3 == 2 → [0;3], else [0, −9.8, 0]`
  [M 0x1910f8:152-172]. `LevelStart` carries `p3` (the placement's param 3).
- Not ported (out of scope, noted): the other WldGravity consumers (grenades/casings/pickups/
  gas/rigid bodies keep their −9.8 constants), the MP spawn-block path, the boot zero.

## Live checks for the main agent

1. **PCSX2, 0700001b**: break on `Vec_Zero` (0x1e2d58) at the level's spawn — expect exactly one
   call with a0 = 0x2d8890 during `Player_Init` (0x1910f8); then read 0x2d8894 — expect
   0x00000000 (vs 0xc11ccccd on 07000005).
2. **PCSX2, death on 0700001b** (a guard kill in the spacewalk): `Player_ChangeSubState` 0x190630
   case 0xd/0xe must queue anim hash **0x60008e0** (gdbwatch on the `AnimScriptAdd` call
   0x1c6d40's a1) — 07000005's death queues 0x6000151.
3. **nf-view `--bond` spots**: `--bond 22.97777,-1.900582,3.132896,0,0` on 0700001b — Bond at the
   port's spawn (the station interior); a jump there should rise and hang far longer than on
   07000005's snowfield (the station rooms are low — he head-bumps; the outdoor spacewalk area is
   the clean float spot, reachable through the level's own flow).
