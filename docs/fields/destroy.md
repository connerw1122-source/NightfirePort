# Destroy (the destroyed-state object, 0x18 bytes at obj+0x110; class 0x21)

Proof levels: see [README.md](README.md). Instance size 0x18 from `Destroy_Create` 0x12c318:16
(`control_create_object(0x18,…)`); class byte 0x21 at 0x12c318:22. Created by placement 0xE1
(dispatch 0x1d04f0:358-363, gated on its `param_7 != 0`). Level: ee_start_05 (07000005 at start)
unless noted.

The game pairs this with the breakable (`Break_Create` 0x12bfd8, class 0x20, another worker's
lane): Destroy is the object that *reacts* — it takes the bullet damage, smashes (model swap +
explosion), then after a delay fires its output channel and re-classes to 1 (a draw-only object).

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | the glist id this object was created with (`glist+0x40`, cleared to 0 when `obj+0xF0 & 0x10`); `Destroy_Update` compares it against the *current* `obj+0xD8`'s `+0x40` — a change means the paired breakable swapped the model, which triggers the smash watch below | [M1 create 0x12c318:25-30,58-60] |
| +0x04 | f32 | health/armour: bullet damage is subtracted from it; `< 0.0` → smash | [M* create 0x12c318:34-40 (p3 read as float bits); Destroy_CollisionHandler 0x12bd48:31,36] |
| +0x08 | i32 | blast-radius word (float bits) = `(int)(param5 · 0.01)`; passed to `Explode_Create` as the 2nd float arg | [M1 create 0x12c318:48; read 0x12bb60:73] |
| +0x0C | s32 | the post-smash countdown: create stores `(int)(param6 · −FRAME_RATE)` (negative = not counting); on destruction it is negated (positive), `Destroy_Update` decrements it while > 0 and at 0 fires the channel and re-classes | [M* create 0x12c318:57; Update 0x12c4f8:17-25; CollisionHandler 0x12bd48:41-45 (negate); Smash 0x12bb60:31] |
| +0x10 | u16 | watch channel (param 1): when the glist id changed and `switch_channels[+0x10]` is on → clear it and `Destroy_Smash` | [M* create 0x12c318:31; Update 0x12c4f8:29-32] |
| +0x12 | u16 | output channel (param 0): set to 1 when the countdown expires (`Destroy_Update`) or immediately on bullet kill when the timer is 0 (`Destroy_CollisionHandler`) or on a script smash with `p0 != 0` (`Destroy_Smash`) | [M* Update 0x12c4f8:23; CollisionHandler 0x12bd48:42; Smash 0x12bb60:40] |
| +0x14 | u16 | bullet filter for the object's HITDATA list: `Collide_FilterBullets(obj+0xD0, +0x14)` = `~(u16)param4` (0 when param 4 is 0) | [M* create 0x12c318:41-47; read 0x12bd48:22] |
| +0x16 | u8 | explosion kind (param 2, low byte): 0 = no explosion; 1 = script 0x6000027, scale 4; 2 = script 0x600004f, scale 2; anything else = script 0x6000052, scale 3; the kind (as a float, ×10.0) also goes to `Explode_Create`'s 3rd arg | [M* create 0x12c318:33; read 0x12bb60:57-75] |
| +0x17 | u8 | smashed latch: `Destroy_Smash` returns immediately when it is 1, then sets it | [M* 0x12bb60:17-20] |

`piVar1[3]` in the decompile is +0x0C, `piVar1[4]` +0x10 (int*-indexed).

## The three destroy paths

- **Bullet** (`Destroy_CollisionHandler` 0x12bd48, `control_funcs[0x21].collide` @0x26f9e0): filter
  the HITDATA list by +0x14; for each bullet (other obj class 5): `+0x04 -= bullet->inst+8`;
  a bullet whose ammo record `+2 == 0x67` sets an extra tint decrement; damage `< 0` → fire the
  channel (or negate the timer) and `Destroy_Smash(obj, bullet->inst+0x30, 0)`.
- **Watch** (`Destroy_Update` 0x12c4f8, `control_funcs[0x21].update`): if `+0x12 != 0` the timer
  runs (see +0x0C); else if glist id changed && `+0x10 != 0` && channel on → clear +0x10, Smash.
- **Script/other** (`Destroy_Smash` 0x12bb60): latch, delete the map light at the object sphere
  (`Light_FindMapLight(obj+0x8C)` → `Light_Delete`), `Effect_RemoveSObjs`, `Control_SetGList(obj)`
  (model swap to the destroyed glist) when `+0x00 != 0`, negate the timer, fire `+0x12` when the
  countdown is 0, re-class to 0x01 (07000002-only: `PlrStat_LogShotHitScenery` when the glist
  matches hash 0x20001d9's), then the explosion by +0x16. The 2nd arg (u16) is unused in the C.

## Param map (from the creator)

| level_tag param | Instance offset |
|---|---|
| p0 (+0x2C) u16 | +0x12 output channel |
| p1 (+0x30) u16 | +0x10 watch channel |
| p2 (+0x34) u8 | +0x16 explosion kind |
| p3 (+0x38) u32 (float bits) | +0x04 health |
| p4 (+0x3C) u16 (inverted) | +0x14 bullet filter |
| p5 (+0x40) u32 ·0.01 | +0x08 blast radius |
| p6 (+0x44) u32 ·−FRAME_RATE | +0x0C post-smash countdown |

The dispatcher passes only 4 args (0x1d04f0:358-363); the decompiler's 5th param of
`Destroy_Create` (a second `level_tag*`, stored to +0x00 when `glist+0x40 == 0`) is an artifact of
the register being live — no caller sets it meaningfully [M1 0x12c318:27-29; S on the fallback].

## Port check (crates/nf-game/src/props.rs + grep)

The port has no Destroy object at all: `grep -rn "Destroy" crates/nf-game/src/` finds only
comments in props.rs about destroyed props and `props.rs`'s `destroyed` flag on its own
prop model. Nothing reads placement 0xE1 (grep `0xe1|0xE1` in crates/nf-formats/src/placement.rs
→ absent). **Missing feature, not a mismatch**: bullet-destroyable scenery whose debris model
swap, explosion script and delayed channel all come from this struct.

## [L-read] spot checks (work/ee_start_05.bin)

07000005 creates no class-0x21 objects (class histogram in docs/atlas/object.md has no 0x21) —
the level's breakables use the Break class instead. Cross-checked in
`work/ee_start_07000002.bin` instead: [L] class-0x21 objects exist there (the level uses
shot-out scenery); instance +0x04 health values are large-positive floats and +0x0C is negative
(not yet smashed) — consistent with the create-time stores. (Values recorded in the session
transcript; the doc keeps addresses only.)

## Live checks

No probe needed until the port models 0xE1: the struct is inert unless the level ships a
Destroy placement. When it does:

1. Find the object live: `DynamicObjList 0x2705a0 → head+0x14 → obj+0xFF == 0x21 → obj+0xE0`.
2. `watch` instance+0x04 (health) while shooting the prop — it should decrease by each bullet's
   damage; the screen shows the prop's model swap + explosion when it goes negative.
3. `poke` instance+0x0C to a small positive value after a smash — the output channel
   (`switch_channels[+0x12]`) should turn on ~that many frames later (drive whatever the channel
   opens, e.g. a door, and watch it open).
