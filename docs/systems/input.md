# Input: pad → actions → movement, aim, camera (on-foot engine, ACTION.ELF)

Status: **draft from the decomp** (Ghidra headless export of ACTION.ELF, SHA-1 `e7ffdcb3…`). Addresses are this
build's. Nothing here has been confirmed by a live PCSX2 trace yet; the trace checks are GAPS G-003.
Function names are the game's own (ACTION.ELF `.symtab`). Driving (DRIVING.ELF) has its own input path, not covered
here (GAPS G-004).

Tags: *measured* = read from our decomp/ELF/disc; *suggested* = from another source, not re-checked on our build.

## 1. Pipeline overview

```
main (0x109080) loop ─ vblank pacing (§6) ─ GameFlow_Main
  Input_Update (0x17ee88)                         once per game update
    clear 40 action floats per player
    psiInput_PollDevices (0x10ca78)                8 pad slots: scePadRead → buttons, linear dead zone (§2)
    psiInput_MapInputs (0x10cd80, players, …)      controller style → action floats (§3)
    per player: StickCompensation2/3 (§4), invert (§5), clamp [-1,1], hold counters
    Input_Check4Combo (0x17ebe0)
  … Player_Move (0x191dd0) / Player_Aiming (0x1a1d28) / Player_AutoAim (0x1a7c98) read actions via
    Input_Actionf(player, action, mask) (0x17f300) and Input_Action (0x17f208, value×100 as u16)
```

## 2. Pad read and dead zone — `ps2_handle_pad` 0x10c738, `psiInput_PollDevices` 0x10ca78 (*measured*)

- 8 pad slots in `tSlot` (stride 0x180; loop ends at 0x246280). `ps2_handle_pad` is a state machine over
  `scePadGetState`/`scePadInfoMode`/`scePadSetMainMode(…,1,3)` (analog mode, locked)/`scePadInfoAct`/
  `scePadEnterPressMode`; state 99 = ready.
- `scePadRead` buffer at slot+0x124. Buttons: `~(data[2]<<8 | data[3])` stored at slot+0x122 (held) and
  slot+0x120 (newly pressed). Analog bytes at slot+0x128..0x12b.
- Analog-byte → button bit mapping assumed from the standard libpad layout (*suggested*: Sony libpad / ps2sdk
  `libpad.h`; to verify with a PCSX2 trace, UNKNOWNS U-010):
  `data[4..7]` = RX, RY, LX, LY → slot+0x128 RX, +0x129 RY, +0x12a LX, +0x12b LY;
  buttons (game's u16): 0x0100 Select, 0x0200 L3, 0x0400 R3, 0x0800 Start, 0x1000 Up, 0x2000 Right,
  0x4000 Down, 0x8000 Left, 0x0001 L2, 0x0002 R2, 0x0004 L1, 0x0008 R1, 0x0010 △, 0x0020 ○, 0x0040 ✕, 0x0080 □.
- **Dead zone (linear, per axis):** for each of the 4 bytes, `v = byte − 127`;
  `|v| < 40 → 0`, else `(v ∓ 40) × 1.4597701` (= 127/87, so full deflection stays ≈127); result + 0x7f is written
  to both slot+0x128+i and slot+0x148+i. Per axis, not radial.
- A curve-mapped copy (`gAnalogStickMappingFunction[|v|]·sign / 2 + 0x7f`) is computed into slot+0x148+i first but
  **overwritten** in the same iteration, so it has no effect (*measured*; dead code in this build).
- Disconnected/not-ready slots: buttons 0, analog bytes 0x7f.

## 3. Controller styles → actions — `psiInput_MapInputs` 0x10cd80 (*measured*)

Per-player record `PlayerSetting` (0x2a38c8, 4 × 0x158 bytes):

| Offset | Meaning (*measured* use) |
|---|---|
| +0x000 | look-invert flag (byte; non-zero → negate actions 4 and 5, §5) |
| +0x001 / +0x002 | auto-aim enabled, single-player / multiplayer (byte; read by `Player_AutoAim`, §7) |
| +0x00e | controller style (u16, 0..7; set by `Input_ChangeControllerStyle` 0x17f5f0, which also flushes input) |
| +0x014 | 40 action floats (`GameActions` index 0..39) |
| +0x0b4 | 40 u16 hold counters (saturate 0x7fff) |
| +0x104 | 40 "active" bytes (what `Input_Action[f]` masks) |
| +0x12c | 40 per-action flag bytes (bits 0x01, 0x04, 0x10 gate re-triggering) |
| +0x154 | "any action active" byte |
| +0x155 | slot connected; +0x156 player index |

Analog actions are `(byte − 127) × 0.0078125` (÷128), from the dead-zoned bytes. Mapping for **style 0** (the
first case; full table for all 8 styles to be generated mechanically, GAPS G-005):

| Action | Source (style 0) | Consumer (*measured*) |
|---|---|---|
| 0 | +(RX − 127)/128 | `Player_Move`: yaw turn (negated), ramped by `Plr_NoAimTurnSpeed_X*` |
| 1 | −(LX − 127)/128 | `Player_Move`: strafe |
| 2 | −(LY − 127)/128 | `Player_Move`: forward/back |
| 3 | +(RX − 127)/128 | `Player_Aiming`: crosshair X |
| 4 | −(RY − 127)/128 | `Player_Aiming`: crosshair Y |
| 5 | −(RY − 127)/128 | `Player_Aiming`: free-look pitch |
| 6 | Up → −1, Down → +1 | (d-pad axis) |
| 7.. | buttons (value = masked bit as float, then clamped to 1) | names TBD (UNKNOWNS U-011) |

Other styles swap sticks/buttons (e.g. style 4 drives movement from the right stick and look from the left; style 6
adds L1/R1 as ±1 strafe). Common to all styles: actions 26–31 and 34–35 come from Start/d-pad/face buttons and from
LX/RY beyond ±0.5 (menu navigation); action 38 (0xac) fires when LX < −0.8, RX > 0.8 and Up are held together.
`Input_Update` passes `MPSettings`+428 as the player count, or 4 when `GameState`+12 == 0x07000048.

## 4. Stick compensation — `Input_Update` 0x17ee88 (*measured*)

- `StickCompensation2(a, b)` (0x17f798): radial. `m = sqrt(a²+b²)`; if `|m| > 0.0002`:
  `a,b *= MapAnalogStick(m)/m`.
- `MapAnalogStick(x)` (0x17f958): `i = (int)(x·127)`, `|i|` clamped to 128,
  `out = sign · gAnalogStickMappingFunction[|i|] / 255` (table at **0x2a3840**, 129 bytes; an identical copy at
  0x2455c0 is not used by this function). The table starts with a run of zeros (an extra radial dead zone) and
  reaches 255 at the end. Its bytes are game data: the port reads them from the user's ELF, never commits them.
- `StickCompensation3(a, b)` (0x17f850): axis snap. If one axis is within ±0.707107 and the other is beyond it,
  the other becomes exactly ±1.
- Application per player: styles 2 and 5: Comp2(3,4), Comp3(0,2). All other styles: Comp2(3,4), Comp2(0,5),
  Comp3(1,2).

## 5. Invert, clamp, hold counters (*measured*)

- `PlayerSetting[p]+0` non-zero → actions 4 and 5 negated (vertical look invert).
- Every action clamped to [−1, 1].
- Hold counter increments while the action is non-zero; 46 (0x2e) is a threshold used for the active flag logic.

## 6. Frame pacing and the logic tick — `main` 0x109080, `GS_SetRefreshRate` 0x17d2b8 (*measured*)

- `VBlankInt` (0x124088) counts vblanks (`VBlankCount`); `PS2FrameRate` = 60.
- Each main-loop pass: `PS2FramesToSkip = VBlankCount − GameCount`; while it is negative the loop waits (the update
  finished early); then it is clamped to 1..2. If game+draw time exceeds 262.5 scanlines (EE timer 0 in H-blank mode,
  `REG_RCNT0_MODE = 0x83`; one NTSC field) — or the draw alone does — the skip is forced to 2 for the next 10 passes.
  Live (PINE, level 0x07000001, WORKLOG 0005): snapshots show skip −1 (waiting) or 1, game ≈ 33–51 + draw ≈ 34–38
  scanlines, i.e. well under one field. Then
  `GS_SetRefreshRate(60 / skip)` sets `FRAME_RATE = 60 or 30`, `FRAME_RATE_MUL = 60 / FRAME_RATE` (1 or 2),
  `REC_FRAME_RATE = 1/FRAME_RATE`, and `GameFlow_Main` runs **one** update.
- So the game logic is **variable-step**: one update per 1 or 2 vblanks, scaled by `FRAME_RATE_MUL`. During a
  background FMV the skip is computed on even vblanks and clamped to 2.
- **Not frame-rate independent:** the turn-speed ramps (§7) advance a fixed amount *per update*, so at 30 Hz a ramp
  takes twice as long in wall-clock time as at 60 Hz, while per-update deltas are multiplied by `FRAME_RATE_MUL`.
  The crosshair smoothing (×0.5 per update, §7) is also per update.
- How often real hardware runs at 30 vs 60 in gameplay is unknown (UNKNOWNS U-013; measure `PS2FramesToSkip` with
  PCSX2/PINE). The movie folder is named `MOVIES/30_FPS`.

## 7. Aiming, turning, auto-aim (*measured*)

Live tuning values are the constants stored by `ReadTuningVars` (0x1d3a80), not the `.sdata` initialisers:

| Variable | Live | `.sdata` init | Meaning (comment in FILES.BIN `TuningVars.txt`) |
|---|---|---|---|
| Plr_AimSpeed_X / _Y | 0.1275 / 0.136 | 0.15 / 0.16 | cursor speed while aiming |
| Plr_AimTurnSpeed_X / _Y | 0.05 / 0.06 | same | turn speed when the cursor reaches the aim-box edge |
| Plr_ScopeSpeed_X / _Y | 0.015 / 0.011, Mul 3, Steps 120 | same | scoped-weapon turn speed |
| Plr_NoAimTurnSpeed_X / _Y | 0.04 / 0.01, Mul 2, Steps 120 | same | turn speed when not aiming |
| Autoaim_Angle_H / _V | 0.12 / 0.22 rad | — | auto-aim cone |
| Autoaim_Range | 25.0 m | 35.0 | |
| Autoaim_LockOnMul | 1.4 | — | |
| Autoaim_EasyMul / NormalMul / HardMul | 1.9 / 1.0 / 0.0 | — | Hard = no auto-aim |

**Ramp ("acceleration")**, used for yaw (`Player_Move`, via a helper with arguments
`(stick, base, Mul, Steps, 0.05, 0.95, &state)`), free-look pitch, edge-turn and scope: per update, the current
speed moves toward a target by `(base·Mul − base)/Steps`; the target is `base·Mul` when |stick| ≥ 0.95
(free look) or the cursor is pinned at the box edge (aim mode), else `base`; |stick| ≤ 0.05 resets the speed to
`base`; the speed is clamped to ±(base + base·Mul). Angle delta = stick × speed × `FRAME_RATE_MUL` (× π/2 for
pitch).

**Free look (not aiming)** — `Player_Aiming` first branch: pitch (obj+0x8a8) += action5 × ramped
`Plr_NoAimTurnSpeed_Y` × `FRAME_RATE_MUL` × π/2.

**Manual aim (aim button held)** — second branch: crosshair (+0x118, +0x11c) += action3/4 × `Plr_AimSpeed_*` ×
`FRAME_RATE_MUL` × 0.125, clamped to the box ±0.296 (X) and ±0.4 (Y). When pinned at an edge, the view turns at
the ramped `Plr_AimTurnSpeed_*` with `Plr_ScopeSpeed_*_Mul` as the multiplier, divided by the zoom (+0x8d0);
underwater (`surfaced == 0`) yaw ×0.4, pitch ×0.7. A third branch (weapon flag 0x40: scoped) uses `Plr_ScopeSpeed_*`.

**Auto-aim** — `Player_AutoAim` 0x1a7c98: enabled from `PlayerSetting[p]+1` (single player) or `+2`
(multiplayer, `MPSettings`+384 ≠ 0); weapons 0x50/0x51 force it on. When enabled and not manually aiming it scans
targets with `Check_AutoAim` (0x1a73f8) within `Autoaim_Range`; the crosshair offsets decay ×0.5 per update; while
manually aiming, the smoothed cursor (+0x120/+0x124) moves halfway to the raw cursor per update. Details of
`Check_AutoAim` and the lock-on (`Autoaim_LockOnMul`) are GAPS G-006.

## 8. Port design consequences

1. **Controller path = this document, bit for bit.** Port `ps2_handle_pad` → `MapInputs` → compensation →
   `Player_*` exactly, including the dead pre-curve write's absence of effect, the per-update ramps and
   `FRAME_RATE_MUL`.
2. **Mouse look bypasses §2–§4 and the ramps of §7.** Raw mouse deltas (counts) × sensitivity → yaw/pitch radians
   added directly to the player's yaw/pitch state (the same fields `Player_Move`/`Player_Aiming` write), with optional
   invert. No dead zone, no `MapAnalogStick`, no ramp, no `FRAME_RATE_MUL`. In manual-aim mode the mouse moves the
   crosshair within the box first (as the stick does) and turns the view at the edge with the same raw delta; the
   exact mouse behaviour in aim mode is a design decision recorded in docs/PLAN.md (DEVIATION D-002).
3. **Auto-aim with mouse:** configurable; **default off with mouse** — a documented deviation (DEVIATION D-001).
   With a controller it stays as the original (difficulty multipliers above).
4. **Tick:** the original is variable-step 60/30 Hz (§6). The port runs logic at a fixed 60 Hz (`FRAME_RATE_MUL` = 1,
   the original's fast path), samples input every rendered frame, accumulates mouse deltas into the next tick and
   interpolates rendering. Reproducing the 30 Hz fallback is optional (a "PS2 pacing" mode) once U-013 is measured.
