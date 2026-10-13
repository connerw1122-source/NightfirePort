# Ladder instance (the climbable, 4 bytes at obj+0x110; class 0x2D)

Proof levels: see [README.md](README.md). Instance size from `Ladder_Create` 0x1966c0:11
(`control_create_object(4, …)`). Level: ee_start_05 (07000005 at start) unless noted.
`control_funcs[0x2D]` @0x26fac4 = {0,0,0} [ELF bytes] — **no update, collide or delete handler**;
the class never runs in the movement pass and the object is never updated by the framework.
All ladder behaviour lives in the player module reading the ladder's **obj_tag** fields.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | s16 | always 0 (written once at create). Read back **as a discriminator** by two functions that share the "current rideable" pointer BLData+0x884: `Player_Weapon` 0x1a24c8:33 (`**(short **)(*(int *)(iVar11 + 0x884) + 0xe0) == 0` → the substate-1 climb weapon block — a ladder keeps the weapon holstered path) and `camera_tracking` 0x1ba5e0:101 (`== 1` selects the creepwall-family camera segment; a ladder's 0 takes the else branch, :106-108). The nonzero values belong to the wire/creepwall instances that share the slot | [M*] writer `Ladder_Create` 0x1966c0:18 (`*puVar1 = 0`); readers listed; [L-read: obj 0x16e2210 → inst 0x16e2320, bytes `00 00 00 00`] |
| +0x02 | u16 | channel = **placement param 1** (`level_tag+0x30`). **No reader found anywhere** — grep over every read of the +0x884 instance plus the whole decompile: nothing dereferences a class-0x2D instance's +0x02. The port keeps it for the layout's sake (crates/nf-game/src/ladder.rs:33-34 agrees) | [M1] writer `Ladder_Create` 0x1966c0:19 (`puVar1[1] = *(undefined2 *)(param_3 + 0x30)`; `lhu v1,48(s1)` / `sh v1,2(v0)` in the disasm) [L-read: inst 0x16e2322 = 0x0000 (the 07000005 ladder's param 1 is 0)] |

That is the whole instance — 4 bytes, no padding fields, nothing else written after the create
memset. Instance size cross-check: the next object's instance in the snapshot starts 0x1B0 later
(0x110 header + 4 bytes + allocator slack).

### obj-side ladder fields (what the gameplay actually reads)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| obj+0xFF | u8 | class 0x2D — `Player_DealWithObjHit` 0x194458:108-112 dispatches hit class 0x2D → `Player_CollLadder` (0x2C → `Player_CollWire`, 0x27 → `Hurt`) | [M] writer `Ladder_Create` 0x1966c0:20 [L-read: obj 0x16e2210 +0xFF = 0x2D] |
| obj+0x54 | f32 | rot.y — the ladder yaw. Grab: `\|Vec_AngleDifference(player.yaw, ladder.yaw)\| > π/2` (0x196118:18-22); climb: yaw eased toward `ladder.yaw + π`, hard-clamped ±π/2 (0x196310:18-33); camera uses it directly | [M*] writer `control_create_object` from the placement rotation (dispatch 0x1d04f0, quaternion param 0 — nothing negates rot.z, confirming docs/atlas/ladder.md's correction) [L-read: obj 0x16e2210 +0x54 raw `da 0f c9 bf` = −1.5707964 (−π/2); matrix row 2 = (−1, 0, 0)] |
| obj+0x80..0x8C | vec3 + f32 | gfx sphere centre/radius (`Control_BuildWorldSph` 0x133070 pattern — glist bounds through the object matrix). The climb window: **top = cy + r + 0.85, bottom = cy − r + 1.0**, recomputed every tick (0x196118:23-25, 0x196310:50-53); grab window `bottom + 0.02 < pos.y < top` after raising the feet (0x196118:26-31) | [M*] readers `Player_CollLadder` 0x196118:20-24, `Player_Climb` 0x196310:50-52 [L-read: obj 0x16e2210 sphere (60.927, 29.906, 89.206) r 3.929 → window 26.98..34.68] |
| obj+0xC0 / obj+0x40 | vec3 pair | matrix row 3 / prev-pos — the riding carry `pos += ladder(+0xC0) − ladder(+0x40)` per climb tick (0x196310:36-44); zero for a static ladder | [M1] |
| obj+0xD8 | ptr | celglist (set at create, 0x1966c0:21) — feeds the sphere build and the renderer | [M1] [L-read: obj 0x16e2210 +0xD8 = 0x0170cc50] |

### BLData+0x884 — the "current rideable" pointer (who reads the ladder object)

Writer: `Player_CollLadder` 0x196118:48 (= the ladder obj on grab; also `Player_CollWire`
0x197130:220, `Player_CollCreepWall` 0x196758:128); cleared by `Player_Init` 0x1910f8:74.
Readers: `Player_Climb` 0x196310:19-52, `Player_Weapon` 0x1a24c8:33, `camera_tracking`
0x1ba5e0:78-125, `Player_CollisionHandler` 0x1949a8:46 (a hit whose object == +0x884 is
discarded — climbing ignores the ladder's own collision), `Player_Collision` 0x194df0:256,380
(+0x7b8 copy), `Player_Zipline`/`Player_Wire`/`Player_Creep` (wire/creepwall substates).

## Param map (from the creator — placement 0x22; params k at tag+0x2C+4k)

| level_tag param | Instance offset |
|---|---|
| p1 (tag+0x30) | +0x02 channel (u16, no reader) |

No other parameter is read; the rotation vector goes into the object header
(obj+0x50..0x58) untouched.

## Port mismatches (crates/nf-game/src/ladder.rs; report only)

No functional disagreement found — `Ladder::create`, `top()`/`bottom()` (+0.85/+1.0 constants)
and the `channel: inst+0x02`-with-no-reader note all match the decompile. Deviations already
flagged in the port's own comments, listed for the record:

1. **ladder.rs:72-82 `capsule_hits`** — the grab volume is a stand-in [S]: the game's hit comes
   from `Collide_Update` 0x1ea398's per-object HITTEST shape (`Player_DealWithObjHit` →
   `Player_CollLadder`); the shape is not recoverable from the decompile this pass. The real
   gates the port does honour are the yaw window (0x196118:18-22) and the y window
   (0x196118:23-31).
2. **ladder.rs:41-42 `let o = obj?;`** — a ladder placement whose map object fails to resolve
   is DROPPED by `Ladder::all` (ladder.rs:98-101), while the game always creates the object
   (a glist-less ladder would get the 0.1 default sphere radius, `Control_BuildWorldSph`
   0x133070). Robustness deviation; no 07000005 impact (its ladder resolves).
3. **ladder.rs:52 radius scale** — the port folds all three placement scales into the radius
   (`s_max` product); the game uses the single obj+0xE8 scale against the glist radius
   (`glist+0x1C · abs(+0xE8)`). Same value whenever the placement scales are uniform [S].
4. Field-level agreement: the port's `channel` = inst+0x02 (ladder.rs:33-34, 88) matches the
   creator; its "no reader found" comment is confirmed by this pass's sweep.

## [L-read] spot checks (ee_start_05.bin)

07000005 has exactly one ladder (class 0x2D; my walk finds it at obj **0x16e2210**, inst
0x16e2320 — docs/atlas/ladder.md's "no live instance is in the snapshot" is outdated: the
object IS live at start and its sphere matches the disc placement (60.9, 29.9, 89.2)):

| Field | Address | Raw | Interpreted |
|---|---|---|---|
| inst +0x00/+0x02 | 0x16e2320 / 0x16e2322 | `00 00` / `00 00` | discriminator 0 (ladder), channel 0 |
| obj+0x54 yaw | 0x16e2264 | `da 0f c9 bf` | −1.5707964 (−π/2); matrix row 2 = (−1, 0, 0) |
| obj+0x84 sphere cy | 0x16e2294 | `8c 3f ef 41` | 29.9060 |
| obj+0x8C radius | 0x16e229C | `c6 6f 7b 40` | 3.9287 → climb window 26.98..34.68 |
| obj+0xFF class | 0x16e230F | `2D` | 0x2D |
| obj+0xFC drawn | 0x16e230C | `02` | drawn last frame |

## Live checks

Probe chain: `DynamicObjList 0x2705a0 → u32 0x2705b4 → obj+0x14 …; class 0x2D → obj 0x16e2210;
inst = u32 obj+0xE0 = 0x16e2320`.

1. **Grab wiring**: walk to the player obj (`PLAYERS 0x2D88E0 → obj → obj+0xE0 = BLData`) and
   `watch <BLData>+0x884 4` while walking Bond into the ladder at (60.93, 29.91, 89.20) — on
   grab it should flip 0 → 0x16e2210 (the ladder obj), substate (obj+0xF6) → 1, weapon → fists.
2. **Climb window**: the thresholds derive from the sphere — bottom 26.98, top 34.68. Climbing
   past `top` should dismount with the +0.6 forward lunge (`Player_Climb` 0x196310:54-62).
3. **Negative check for +0x02** (the channel with no reader): `python -I tools/trace/fieldprobe.py
   poke 0x16e2322 2 0x1234 --shot out.png` — expect **no gameplay change of any kind** (nothing
   reads it; the poke only proves the sweep). Revert with poke 0.
4. The discriminator +0x00: `poke 0x16e2320 2 1` while Bond is climbing should make
   `Player_Weapon` treat the rideable as non-ladder (weapon logic takes the other branch) and
   `camera_tracking` take the creepwall segment branch — a live confirmation of the [M*]
   reading if anyone needs it.
