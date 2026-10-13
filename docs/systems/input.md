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
- Pad layout (*measured*, PCSX2 trace WORKLOG 0006 — the user pressed each input in a known order; matches the
  standard libpad layout): slot+0x128 RX, +0x129 RY, +0x12a LX, +0x12b LY (0 = left/up, 255 = right/down);
  buttons (game's u16 at slot+0x122): 0x0100 Select, 0x0200 L3, 0x0400 R3, 0x0800 Start, 0x1000 Up,
  0x2000 Right, 0x4000 Down, 0x8000 Left, 0x0001 L2, 0x0002 R2, 0x0004 L1, 0x0008 R1, 0x0010 △, 0x0020 ○,
  0x0040 ✕, 0x0080 □.
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

**Style 0 buttons → actions** (*measured* in the decomp and confirmed live, WORKLOG 0006):

| Button | Actions set | | Button | Actions set |
|---|---|---|---|---|
| ✕ | 8, 31 (+ 25 for player 0) | | Up | 6 = −1, 11, 26 |
| ○ | 12, 33 | | Down | 6 = +1, 10, 27 |
| □ | 14, 32 | | Left | 15, 28 |
| △ | 7, 34 | | Right | 16, 29 |
| L1 | 19 | | Select | 13 |
| R1 | 9 | | Start | 30 (+ 25 for player 0) |
| L2 | 16 | | L3, R3 | none in style 0 |
| R2 | 14 | | | |

Live, a held button briefly shows its raw masked bit as the action value (e.g. ✕ → action 31 = +64.0) before the
clamp at the end of `Input_Update`, exactly as the decomp has it.

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
- **Live check** (WORKLOG 0006, `tools/trace/pad_log.py`): recomputing actions 3/4 from the logged RX/RY with this
  formula in f32 (table read from our ELF) reproduces 364 of 407 samples exactly, 34 more against a stick value from
  ±3 samples (the stick moved between the two PINE reads), and 9 are torn mid-update reads (one axis 0 or
  uncompensated). Example: right stick hard left (RX 1) → −0.984 before the curve, −0.980 after (table[125]/255).
- Not verifiable from the trace: the §2 dead zone, because the game overwrites the raw bytes in place (GAPS G-012).

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
- **Live (PCSX2 2.8.2, level 0x07000001, 15 s of gameplay sampled at 2 kHz, WORKLOG 0006):** 819 updates over 829
  vblanks: 809 one-vblank (60 Hz) and 10 two-vblank (30 Hz) updates, i.e. 98.8 % at 60 Hz. PCSX2 ran at 55.3
  vblanks/s (~92 % speed). Other levels and real hardware remain open (U-013). The movie folder is named
  `MOVIES/30_FPS`.

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

## 9. The port's pad path (fix-input)

`nf_game::input` carries the game's pad path as code + generated data; `nf-view`'s `gamepad` cargo
feature feeds a real pad into it. Everything below cites the decomp; the [L] tests run the game's own
functions in the R5900 interpreter over the user's ACTION.ELF (`crates/nf-game/tests/input_styles_live.rs`,
`input_ramp_live.rs`).

1. **Dead zone** (`input::dead_zone`): `psiInput_PollDevices` 0x10ca78 — per axis, `v = byte − 127`,
   |v| < 40 → 0, else `(v ∓ 40) × 1.4597701` (= 127/87), trunc → char arithmetic `+ 0x7f`, written to
   both +0x128..0x12b and the curve copy +0x148..0x14b. The raw byte exists only between `scePadRead`
   (`ps2_handle_pad` 0x10c738 touches just the button bytes) and this loop — G-012's "read the code"
   answer; the curve-mapped write one iteration earlier is dead code.
2. **Style table** (`input::map_inputs`, data in `input/styles_gen.rs`): generated from the decomp of
   `psiInput_MapInputs` 0x10cd80 by `scripts/gen_styles.py` (each field cites its decomp line): the six
   analog sources of actions 0..5 per style (style 2's L1 swaps the strafe byte, style 6 adds ±1.0 from
   L1/R1), the (action, mask) button writes, the ±1 d-pad axis (styles 4/5 use Triangle/Cross), style 5's
   L1-gated skip of actions 7/8, and the common tail (Start/d-pad/face, the LX/LY ±0.5 menu overwrites,
   action 38's LX<−0.8 && RX>0.8 && Up, player 0's Start|Cross → 25). Verified bit-exact against the
   game's own function, 8 styles × 21+ synthetic pad states.
3. **Stick compensation** (`input::compensate`): `StickCompensation2` 0x17f798 (radial) /
   `StickCompensation3` 0x17f850 (axis snap) with `MapAnalogStick` 0x17f958 — the game multiplies by the
   literal 0.003921569 (not ÷255; the constant's rounding differs by 1 ulp). Per style: Comp2(3,4) then
   Comp3(0,2) for styles 2/5, else Comp2(3,4), Comp2(0,5), Comp3(1,2). The curve table
   (`gAnalogStickMappingFunction`, 129 bytes at 0x2a3840) is read from the user's ELF, never committed.
4. **Ramps** (`input::AccelState`): `AccelFunc0` 0x1a9218 — the state moves toward `base` (|stick| <
   hi) or `base·mul` by `(base·mul − base)/steps` per tick, |stick| ≤ lo resets to base, clamp
   ±base(1+mul); the target is UNSIGNED, the stick's sign rides the multiply outside (`Player_Move`
   writes obj+0x34 = (−action0) × state × FRAME_RATE_MUL [M 0x191dd0:143-148]). `Player_Aiming` 0x1a1d28
   inlines the same shape for its states: +0x8f4/+0x8f8 (aim edge-turn X/Y), +0x900 (free-look pitch),
   +0x8ec/+0x8f0 (scope). `Player_Init` 0x1910f8:57-63 zeroes them all. Verified tick-by-tick against
   AccelFunc0 in the interpreter (the EE float-arg convention: six floats in f12..f17, the state pointer
   in a0 — decoded from the prologue).
5. **The pad driver** (`input::PadDriver`): binds the config, the tuning values and one set of ramp
   states per player; `tick` runs dead zone → map → compensate → invert → clamp [M `Input_Update`
   0x17ee88's order] and produces the port's `player::Input`: turn/look through the ramps (free look),
   the aim-mode crosshair at `Plr_AimSpeed × 0.125` with the ramped edge-turn beyond the box, jump/
   crouch on the newly-pressed edge (+0x120's semantics), fire 9, aim mode 19, zoom = action 6 in aim
   mode (`Player_Zoom` 0x1a1a70). Not modelled yet: the hold-counter active flags (actions 20/21, the
   game's pad weapon cycling) — the pad's weapon switch rides the port's keyboard mechanisms for now.
6. **The backend** (`nf-view/src/gamepad.rs`, `--features gamepad`): `gilrs` 0.11 (MIT OR Apache-2.0)
   drains the OS pad and maps it onto the PS2 bits/bytes (Cross 0x40 … D-pad; sticks to raw bytes,
   0 = left/up), then `PadDriver::merge` runs the game's path and merges the result into the keyboard's
   `Input` (axes add+clamp, buttons OR). No pad connected → no-op: the keyboard/mouse path is untouched
   (the recorded-trace test pins it bit-exact). Master builds without the feature and without gilrs.
