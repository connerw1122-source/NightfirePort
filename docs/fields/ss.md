# SIMPLESCRIPT_tag (the SS instance, 0x2C bytes at obj+0x110; class 0x25)

Proof levels: see [README.md](README.md). Instance size 0x2C from `control_create_object(0x2c, …)` in
`SS_Create` 0x1a9c58 [M 001a9c58_SS_Create…c:16]; class byte `obj+0xFF = 0x25` [M :52]. Placement type 0xE2
[M 0x1d04f0:365-369]. Level for the [L] rows: **ee_start_07000001** (07000005 has no class-0x25 object — the
05 snapshot's class histogram has no 0x25 entry; 07000001 has 16).

The whole block is memset 0 by `control_create_object`, then every field except +0x1E/+0x28 is written by
`SS_Create`. `SS_Update` casts the instance to `float*` — `pfVar2[k]` = byte 0x4·k; `(int)pfVar2 + 0xHH`
forms are plain byte offsets.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | f32 | the playhead in key units ("time"). Advanced by +0x04 (or +0x04·FRAME_RATE_MUL) each interp tick; wrapped to 0 past +0x1C | w create 0 [M 0x1a9c58:26], `*pfVar2 += pfVar2[1]` [M 0x1a9948:61-64,103-107], wrap `*pfVar2 = 0.0` [M :70,120]; r as `KeyFrame_Interp`/`Spline_Interp` arg 1 [M :51,56,93,98] and vs +0x08/+0x1C [M :66,109,119]. **[L-read: 0.0 on all 16 SS of 07000001]** |
| +0x04 | f32 | the per-tick step = (+0x1C count−1) / speed, speed = level_tag+0x04 (u16, 0 → 0x4B = 75) | w [M 0x1a9c58:51]. **[L-read: step = cnt1/rate exactly — 0.04 = 3/75, 0.093333 = 7/75, 0.16 = 8/50, 0.14 = 7/50]** |
| +0x08 | f32 | the hold/end time: param 5 (u32 at level_tag+0x40) quantised DOWN to a whole multiple of +0x04 (`+0x04 · trunc(param5 / +0x04)`); the state-1 run stops (→ 0x40) when the playhead lands on it within 0.0002 | w [M 0x1a9c58:40-46,57], r [M 0x1a9948:109-113]. **[L-read: 30.0, 29.9733, 59.92, 44.96, 44.94 — all exact step multiples (param 5 = 750, 562, 642, 281, 321)]** |
| +0x0C | ptr | the framelist (key array, 0x1C bytes/key: vec3 pos + quat wxyz) passed to `KeyFrame_Interp`/`Spline_Interp` arg 2 = `level_tag+0x08` (u32) copied verbatim | w [M 0x1a9c58:27]; r [M 0x1a9948:51,56,93,98 (`pfVar2[3]`)]. **[L-read: 0xb04704..0xb03a64 — level-heap pointers]** |
| +0x10 | u32 | switch channel (param 3, level_tag+0x38). OUTPUT: while off, `SS_Update` writes it = `SS_Operate`'s result (0/1). Also `SS_Operate`'s input gate for the 0x40 state. 0 = none (writes skipped) | w none at create (0); r/w [M 0x1a9948:21-23 (`switch_channels[(int)pfVar2[4]] = (char)lVar4`)], r [M 0x1a9df8:47-48 (SS_Operate), 0x1a9ea8:11-12 (SS_Activate writes 1)]. **[L-read: 0 on all 16 (param 3 unset) — no channel traffic]** |
| +0x14 | u32 | sound id (param 4, level_tag+0x3C; 0 → 0xFFFF at create), played at obj+0x80 by the 0x40 state | w [M 0x1a9c58:35-39]; r [M 0x1a9948:86-88 (`Sound_Play3D(100.0, −1.0, −1.0, pfVar2[5], obj+0x80, …)`)]. **[L-read: 0xFFFF ×16 (no sound param)]** |
| +0x18 | f32 | the sound handle returned by `Sound_Play3D`; ≠ 0 → `Sound_IsFinnished` polled, cleared when finished or when the anim ends | w 0 at create [M 0x1a9c58:49]; r/w [M 0x1a9948:30-47,88,111,127 (`pfVar2[6]`)]. **[L-read: 0 ×16]** |
| +0x1C | s16 | last key index = level_tag+0x02 − 1 (the key count−1). Doubles as the speed-mode selector: == 1 → the playhead advances a whole step per tick (no FRAME_RATE_MUL); also the interp clamp/wrap bound | w [M 0x1a9c58:28]; r [M 0x1a9948:51,56,60,66,93,98,103,119 (`*(short/undefined2*)(pfVar2+7)`)]. **[L-read: 3,4,7,8 → key counts 4/5/8/9]** |
| +0x1E | u16 | operate mask — which touching classes make `SS_Operate` return "operating": bit 1 (0x2) bullet (class 5), bit 3 (0x8) player (class 3), bit 4 (0x10) drone (class 2); **bit 0 (0x1) = don't write the switch channel**; **bit 2 (0x4) = the `SS_Activate` use-key gate**. = param 2 (level_tag+0x34) | w [M 0x1a9c58:33]; r [M 0x1a9948:20 (`& 1` gate)], [M 0x1a9df8:46 (`& uVar5`)], [M 0x1a9ea8:9 (`& 4`)]. **[L-read: 0x2 on all 16 — param 2 = 2, bullets count as "operating"]** |
| +0x20 | u16 | param 0 copy (level_tag+0x2C). **No reader found anywhere** — the initial state is written separately to +0x22; keep as writer-only | w [M 0x1a9c58:31]. **[L-read: 64 (0x40) ×16 = the same value +0x22 starts with]** |
| +0x22 | u16 | the state machine; **initial value = param 0**: 0 idle, 1 animating, 8 looping wrap, 0x40 sound-then-animate. `SS_Activate` (use key, mask & 4) sets 1 from any non-zero state | w [M 0x1a9c58:30], [M 0x1a9948:89 (→1), :112 (→0x40), :122/:125 (→0x40/0)], [M 0x1a9ea8:10 (→1)]; r switch [M 0x1a9948:37,46,48,81,112,121-125]; r gate [M 0x1a9ea8:9]. **[L-read: 64 (0x40) on all 16 — the level's SSs sit in the sound-then-animate state at start]** |
| +0x24 | u16 | param 1 (level_tag+0x30): the end-of-run selector — after a state-1 run finishes: == 0 → state 0x40 (loop via the sound state), ≠ 0 → state 0 (one-shot, dead; `SS_Activate` cannot restart state 0) | w [M 0x1a9c58:32]; r [M 0x1a9948:121]. **[L-read: 0 ×16 → loop forever through 0x40]** |
| +0x26 | u16 (s16) | spline flag (param 6, level_tag+0x44): 0 → `KeyFrame_Interp`, ≠ 0 → `Spline_Interp(…, 0, 2)` | w [M 0x1a9c58:47]; r [M 0x1a9948:49,78,90]. **[L-read: 0 ×16 (no splines on 07000001)]** |
| +0x28 | u16 | "interpolated last tick" latch: set 1 after every interp tick; while set, `SS_Update` ORs obj+0xFA 0x20 (moved) each tick. Never cleared except at create | w 0 (memset), [M 0x1a9948:129]; r [M :24,27]. **[L-read: 0 ×16 (never interpolated yet at snapshot time)]** |
| +0x2A | u16 (s16) | param 7 (level_tag+0x48): when ≠ 0 and the object has a cel, `*(cel+0x90) = *(obj+0x84)` after the update (a cel-side position scratch — purpose not chased) | w [M 0x1a9c58:50]; r [M 0x1a9948:67,75,82,132-136]. **[L-read: 0 ×16]** |

obj_tag header: `obj+0xFA |= 0x44` at create [M 0x1a9c58:55] — [L-read 0x44 ×16]; per-tick `\|= 0x20` while
the +0x28 latch is set [M 0x1a9948:33,59,102]. No delete handler in `control_funcs[0x25]` (update only).

## The state machine (`SS_Update` 0x1a9948) — what each state does

```
SS_Operate(obj, ss) [M 0x1a9df8]: bits from the obj+0xD0 HITDATA chain: class 3 → 0x8, class 2 → 0x10,
  class 5 → 0x2. If (ss+0x1E & bits) != 0 → 1 ("operating": something of a masked class is touching).
  Else: +0x10 == 0 or switch_channels[+0x10] == 0 → 0; else 1.
per tick:
  if !(+0x1E & 1) and +0x10 != 0 and channel off → switch_channels[+0x10] = operate result   (:20-23)
  if +0x28 → obj+0xFA |= 0x20; poll +0x18 sound (clear when finished)                        (:24-47)
  switch (+0x22):
    0    → nothing (falls to the +0x2A tail)                                                 (:73-76)
    1    → interp (+0x26 picks key/spline); +0x00 += +0x04 (·1 if +0x1C==1 else ·FRAME_RATE_MUL);
           if +0x00 lands exactly on +0x08 (|Δ|<0.0002) → +0x18=0, +0x22=0x40;
           if +0x00 > +0x1C → +0x00=0, +0x22 = (+0x24==0 ? 0x40 : 0), +0x18=0; +0x28=1       (:73-129)
    8    → interp; advance; if +0x00 <= +0x1C → done this tick, else +0x00=0 (wrap, no sound) (:48-71)
    0x40 → operate result must be 1 (else skip); Sound_Play3D(+0x14) → +0x18; +0x22=1; then
           the same interp+advance as state 1                                                 (:80-129)
  tail: if +0x2A != 0 and obj+0x20 (cel) != 0 → *(cel+0x90) = *(obj+0x84)                     (:131-137)
```

`SS_Activate` 0x1a9ea8 (the only activation path — called from `Player_Activate` 0x1a5170:139 on the
use-key hit, class '%'): `if (+0x22 != 0 && (+0x1E & 4)) { +0x22 = 1; if (+0x10) switch_channels[+0x10] = 1; }`
— note its mask bit is 4 on +0x1E (bit 2), a third use of that word.

## Param map (from `SS_Create` 0x1a9c58; param k = level_tag+0x2C+4k)

| param | level_tag | Instance offset |
|---|---|---|
| — | +0x02 (u16 key count) | +0x1C = count − 1; +0x04 = (+0x1C)/speed |
| — | +0x04 (u16 speed) | divisor of +0x04 (0 → 75) |
| — | +0x08 (u32 ptr) | +0x0C framelist pointer |
| 0 | +0x2C | +0x22 (initial state) and +0x20 (copy, no reader) |
| 1 | +0x30 | +0x24 (loop selector) |
| 2 | +0x34 | +0x1E (operate/activate mask: 0x2 bullet, 0x8 player, 0x10 drone, 0x1 no-channel-write, 0x4 activate-gate) |
| 3 | +0x38 | +0x10 (switch channel) |
| 4 | +0x3C | +0x14 (sound id, 0 → 0xFFFF) |
| 5 | +0x40 | +0x08 (hold time, quantised to +0x04 multiples) |
| 6 | +0x44 | +0x26 (spline flag) |
| 7 | +0x48 | +0x2A (cel-sync flag) |

## [L-read] spot checks (ee_start_07000001.bin, 16 class-0x25 objects, all values ×16)

1. **+0x22 == +0x20 == 64** — the initial state is param 0, live value 0x40 ("sound-then-animate").
2. **+0x04 == +0x1C / speed** — 0.04 = 3/75, 0.053333 = 4/75, 0.093333 = 7/75, 0.16 = 8/50, 0.14 = 7/50 (per-placement rates 75/75/50/50; census over all 16: 8× 3/75, 1× 4/75, 3× 7/75, 2× 8/50, 2× 7/50).
3. **+0x08 is an exact step multiple** — hold/step = 750, 642, 562, 321, 281 across the 16 ⇒ param 5 = 750, 562, 642, 281, 321 ticks (the `+0x04 · trunc(param5/+0x04)` down-quantise, 0x1a9c58:57).
4. **+0x14 = 0xFFFF, +0x10 = 0, +0x24 = 0, +0x26 = 0, +0x2A = 0, +0x00 = 0.0, +0x18 = 0, +0x28 = 0**, obj+0xFA = 0x44.

## Live checks

No SS objects exist on 07000005 — load **07000001** (16 of them, e.g. obj 0x18fabd0, SS 0x18face0, at
(−23.9, 5.3, 16.7); windows/panels by height). Walk as usual, filter obj+0xFF == 0x25, instance = u32 at
obj+0xE0.

1. **State machine**: watch `SS+0x22` while shooting one of the panels (+0x1E = 0x2 means a bullet touch
   makes SS_Operate return 1). Expected per the decompile: 0x40 → 1 on the operate tick, the playhead
   (+0x00) advances from 0 by +0x04 per tick, and when it lands on +0x08 the state returns to 0x40 (a
   one-cycle animation per bullet hit — or looping if a bullet stays in contact).
2. **Channel write-back**: set +0x10 to a live channel (e.g. poke 4 there) and watch `switch_channels[ch]`
   flip to the operate result each tick while it is 0. If the game never writes it, the +0x1E bit-0 gate or
   the `pfVar2[4] != 0.0` float-compare path differs from the reading.
3. **+0x26 spline**: poke 1 → the pose should come out `Spline_Interp` (visibly different path between the
   same keys). Diff screenshots at the same playhead.
4. **+0x1C == 1 speed mode**: poke +0x1C = 1 → the playhead should advance a full key per tick instead of
   count/75 — a 07000001 SS with count 4 finishes in 3 ticks instead of 75.

## Port mismatches (crates/nf-game/src/ss.rs @ master; report only)

1. **The playhead model is wrong in mechanism** (the port flags it [S] — ss.rs:15-18 — the decompile
   disagrees with the mechanism, not just the details). Port (`ss.rs:86-88`):
   ```rust
   let target = if s.channel != 0 && ch.on(s.channel as u32) { last } else { 0.0 };
   if s.t < target { s.t = (s.t + step).min(target); }
   else if s.t > target { s.t = (s.t - step).max(target); }
   ```
   The game has no target-seek and never reads the channel as a playhead gate: the playhead advances in
   states 1/8/0x40 unconditionally (`+0x00 += +0x04`, 0x1a9948:61-64/103-107), wraps to 0 past +0x1C
   (:70/:119-120), and the channel (+0x10) is an OUTPUT (`switch_channels[+0x10] = SS_Operate()` while off,
   :21-23) plus `SS_Operate`'s gate for the 0x40 sound state only (:85).
2. **The doc comment states the wrong gate** — `ss.rs:5-6` "The playhead advances while the SS's switch
   channel (param 3) is on and returns while it is off": nothing in 0x1a9948 advances or returns the
   playhead on the channel; the channel is written, not read, outside `SS_Operate`'s 0x40 gate.
3. **Missing entirely**: state +0x22 machine (port SSs animate every tick unconditionally), the one-shot
   hold at +0x08, the +0x24 loop selector, the 0x40 sound state (+0x14/+0x18), `SS_Operate`'s touch gate
   (+0x1E vs the obj+0xD0 hit list), the channel write-back, `SS_Activate` (use-key activation, mask bit 2
   of +0x1E), the +0x1C==1 per-tick-key speed mode, and the +0x2A → cel+0x90 sync.
4. What the port gets right: create gating on key count ≠ 0 [M 0x1a9c58:15], rate default 75, spline =
   param 6, channel = param 3 (as a value), key list from the placement's path block. The rate→step
   direction (`1/rate` per tick, ss.rs:85) matches +0x04 = count−1/rate when FRAME_RATE_MUL = 1 [L].
