# The driving missions' PC keyboard + mouse mapping — drive-keyboard-input

Companion to `docs/systems/driving-input.md` (the game's pad path, G-004). That doc ends at the
`{action, device, value}` message; this one answers the PC question: **which key drives which
driving action, and what the game itself does to a digital input on the way**. Code facts are [M]
(function + address in DRIVING.ELF, decomp at `work/decomp/DRIVING.ELF/`, vaddr = file offset +
0x107080), disc facts [D], USATxt facts [M] (parsed from the disc's `USATxt.dat` inside
`FILES.BIN`), and every port-side design choice is [S] — the shipped binding data
(`data\Control\Master.def`, in the mission VIVs) is not extractable from the slim ISO
[driving-input.md §11], so the shipped key bindings are unknown and the PC table below is the
port's design onto the game's own actions.

## 1. The steering ramp — the answer to "how does the game ramp a d-pad steer"

**There is no ramp in the input path** [M-negative, driving-input.md §8: no accumulator exists in
0x167788–0x169708]. **There is no d-pad steer at all in this build**: the dedicated digital steer
actions STEERLEFT/STEERRIGHT (4/5) and STEERLEFTDIGITAL/STEERRIGHTDIGITAL/STEERUPDIGITAL/
STEERDOWNDIGITAL (8/9/0xa/0xb) have **no consumer anywhere in DRIVING.ELF** [M-negative — the
car's GAMEACTION drain `FUN_0018add8` 0x18add8 switches only on 1, 2, 3, 0xc–0x11, 0x16, 0x17,
0x1a, 0x1b, 0x1c, 0x1d–0x21; all seven queue-drain sites were enumerated
(0x18add8 car, 0x20c078 WeaponMan, 0x206ca0 MissionMan, 0x23cbc0 HUD, 0x175d78 GLOB, 0x1d6128
WorldCamera, 0x193818 POV) and none reads 4/5/8/9/0xa/0xb]. Steering reaches the car **only** as
GAMEACTION_STEER (1) with a signed float — the analog LX axis value.

The ramp lives in the **vehicle layer**, in the same drain, and it shapes *any* steer float the
same way, analog or digital 0/1 [M 0x18add8, the tail at 0x18b1f0–0x18b318, disassembly checked
0x18b224–0x18b308]:

1. The raw steer `car+0x2c4` (last STEER message value) is chopped at the vehicle dead zone:
   `if (−PAD_DEAD_ZONE < raw && raw < PAD_DEAD_ZONE) raw = 0` — `DAT_00331988` = **0.2** [M .data
   word 0.2; the `Physics\Physical\default.tun` key `PAD_DEAD_ZONE=0.2` [D, RefPack-decompressed
   from MIS01.VIV]]. No rescale afterwards (same shape as the input-side chop).
2. Target = `car+0x438` (a script/AI override) when non-zero, else the chopped raw. `+0x438`'s
   only writer is the input-clear `FUN_001872d8` (sets 0), so in shipped flow the target is
   always the chopped raw [M + S for "never non-zero in shipped flow"].
3. `car+0x240 = move_toward(car+0x240, target, 0.25)` — `FUN_002104a8` 0x2104a8 (68 bytes):

   ```c
   float move_toward(float cur, float target, float step) {   // 0x2104a8 [M]
       if (target < cur) { float r = cur - step; return (r < target) ? target : r; }
       else              { float r = cur + step; return (target < r) ? target : r; }
   }
   ```

   The **0.25** step is a code immediate (`0x3e800000` at the caller), not a tuning key — no
   STEER*/TURN*/RAMP* key exists anywhere in the tuning string data 0x360000–0x37b000
   [M-negative].

**Numbers at the fixed 60 Hz**: digital full lock (|steer| 0→1) takes **4 ticks = 66.7 ms**
(0.25/0.5/0.75/1.0); full-left → full-right **8 ticks = 133.3 ms**; releasing ramps back to
centre in 4 ticks. A digital 0/1 steer is ramped exactly like a full analog deflection — which is
why the PC port can steer with plain keys and inherit the game's feel. The engine consumes
`+0x240` raw: `slip = steer(+0x240) · MAXSTEERING · steer_scale` in the engine step `FUN_0018cef8`
(:458–474), with `steer_scale = max(1 − min(1,(|v|−MODSTEERSPEED)/25), MINSTEER)`. The full-lock
flag `car+0x2d4 = |raw steer| > 0.9` (strict) is computed pre-chop in the same tail [M 0x18add8
:189-194] and feeds the wheelspin detect.

The vertical twin (`STEERVERTICAL` 2 / `STEERVERTICALINV` 3 → `+0x444`, ramped into `+0x244` with
the same 0.25) has **no physics consumer** in the decomp [M-negative] — nothing maps to it on PC.

## 2. Every driving action, its consumer and its semantics

Consumers [M]; the dispatch is SendGameMessage 0x165850 → listener queues → per-object drains.

| action (id) | consumer | semantics | what it does |
|---|---|---|---|
| STEER (1) | car drain 0x18add8 case 1 → raw `+0x2c4` | **value-hold**: the last message value persists; shaped by the ramp §1 | steering |
| GAS (0x16) | car drain case 0x16 → `+0x278`; then every tick `+0x278 = min(2·v, 1.0)` [M :179-181] | value-hold; digital 1.0 = full gas from the first tick | throttle |
| GASBRAKE (0x17) | car drain case 0x17 [M :119-135] | single-axis split at ±PAD_DEAD_ZONE: `v < −0.2 → gas-side(+0x280) = −v`, `v > 0.2 → brake-side(+0x284) = v`, else both 0. **Negative axis = gas, positive = brake** (the stick's pull-back = pedal-down convention) | combined pedal axis (pad alternative; unused on PC) |
| BRAKE (0x1a) | car drain case 0x1a → `+0x27c`, raw, no gain | value-hold | brake; the engine's own reverse: brake held ≈30 ticks (`+0x29a`) with forward speed < 2.5 → reverse state `+0x29c` [M FUN_0018cef8 :184-208] — brake **is** reverse, no separate key |
| HANDBRAKE (0x1b) / HANDBRAKERELEASE (0x1c) | car drain cases 0x1b/0x1c [M :144-152] | **hold pair**: `+0x259 = 1` / `= 0`. While set, throttle `+0x250` and brake `+0x254` are zeroed (unless the car descriptor's `+0xbc` ≠ 0) [M :231-238]; `+0x29f` gets a camera-relative direction bit (velocity · cam-forward dot, computed at the drain's head — level context, not input) | E-brake |
| FIREPRIMARY (0x1d) / RELEASE (0x1e) | car drain [M :153-158]: set/clear `+0x25a`, set gated by the HUD singleton `FUN_0023c618()->+0x2a0 == 0`; WeaponMan 0x20c078: `[9]` hold flag + per-frame `FUN_0020d678` | **hold pair** | fire the car's gun |
| FIRESECONDARY (0x1f) / RELEASE (0x20) | car drain set/clear `+0x25b` (same gate); WeaponMan `[0xa]` hold; on WeaponMan's press edge with a POV entity active, `*(POVent+0x70c) = 1` one-shot | **hold pair** | fire the equipped secondary weapon |
| TOGGLESECONDARY (0x21) / UP (0x22) | WeaponMan 0x20c078 (identical case fallthrough) → `FUN_0020d4c0(w,1)`: index `+4 = list[(cur+1)%n]`, switch cooldown `+0x50 = time + DAT_00339f4c/3`, completes a pending reload | **edge, one-shot per message** (the car drain's own case 0x21 is a deliberate no-op) | next weapon |
| TOGGLESECONDARYDOWN (0x23) | WeaponMan → `FUN_0020d4c0(w,0)`: `list[(cur−1+n)%n]` | **edge, one-shot** | previous weapon |
| FIREGADGET (0x24) / RELEASE (0x25) | WeaponMan `[0xb]` hold; while held `FUN_0020d700` runs **every frame** (the gadget re-fires), "empty" sound on the press edge with ammo < 1 | **hold pair** | use gadget |
| CHANGECAMERA (0x26) | MissionMan 0x206ca0 → `FUN_00208540(this,3)`: one-shot request flag `+0x258 = 1` (the camera-mode cycler `FUN_001ba5c0` also handles 0x26/0x27 fwd, 0x28 back) | **edge, one-shot per message** | camera view |
| CAMLOOKBACK (0x29) / CAMLOOKBACKRELEASE (0x2a) | camera drain 0x1d6128 forwards to the chase-cam dispatch `FUN_001bddc8` case 0x29 [M 001bddc8]: latches `controller+0x48 = 1` (release only honoured for standard chase modes, mode table DAT_003339bc); the camera swings behind while latched | **hold pair** (latch until release) | look back |
| POVRELOAD (0x3f) | WeaponMan: if `[9]==0 && [0xa]==0` → `FUN_0020e868` reload (ammo refill + sound) | **edge, one-shot** | reload |
| CAMZOOMBUTTON (0x2b) | **no consumer in DRIVING.ELF** [M-negative] | dead here | — |
| CAMROTATEX/X2 (0x2c/0x2f), CAMROTATEY/YINV (0x2d/0x2e), CAMROTATEY2/Y2INV (0x30/0x31) | MissionMan axis slots [M 00206ca0], merged by larger \|v\| into the camera's yaw/pitch (`cam->+0x700/+0x704`, dead zone 0.2) | **axis** (value applied per message, 0 = release) | right-stick camera orbit — pad-only; no PC key (the task's camera actions are CHANGECAMERA/CAMLOOKBACK) |
| WHEELSPIN (0x18/0x19), LEANLEFT/RIGHT (0x3e/0x3d), AIMZOOM* (0x37–0x3a), CAMSPIN/CAMCENTRE/CAMZOOMINOUT (0x32–0x34), AUTODRIVELOCK (0x3b, MissionMan `+0x354` hold), RESUMEAUTODRIVE (0x40) | 0x3b/0x3c pinned at MissionMan; the rest have **no consumer in DRIVING.ELF** [M-negative] | — | not mapped on PC |

No type-8/9 auto-repeat binding is needed by any of this: holds are value/flag holds, one-shots
are edges [M the consumers above].

## 3. The PC mapping (the port's design, [S] throughout)

Keys synthesize the game's own raw pad slot (`KeyboardMouse::to_raw_pad` → `poll_device`
0x169288 port) so the whole shipped device path runs unchanged; the values then flow through the
port's binding table (`pc_driving_bindings`) into the game's actions. Sign conventions come from
PollDevice itself [M 0x169218 + the LY negation]: LX 255 = +1 (right), the analog-button
pressures are `min(1, b/192)` [M 0x169248]. Gas/brake ride the **analog-button pressures** (not
the LY axis) so each pedal is its own value — a single axis cannot split into GAS/BRAKE with
per-value bindings without sending one action the other's sign (GASBRAKE exists in the game
precisely because the pad shares one axis).

| PC key | pad synthesis | value (idx) | bindings → action | game semantics kept |
|---|---|---|---|---|
| A / ← | LX byte 0 | ALX (0) | Always → STEER (1), value −1.0 | the §1 ramp: ±0.2 chop, 0.25/tick, 4 ticks to lock |
| D / → | LX byte 255 | ALX (0) | Always → STEER (1), value +1.0 | same |
| (released) | LX byte 127 | ALX | value 0.0 (chopped) | ramps back 4 ticks |
| W | △ pressure 255 | AButtonTriangle (8) | Always → GAS (0x16) | min(2v,1) clamp → full gas tick 1 |
| S | ✕ pressure 255 | AButtonCross (10) | Always → BRAKE (0x1a) | raw hold; reverse is the engine's ≈30-tick brake-at-standstill state |
| Space | ✕ bit 0x0040 | DButtonCross (22) | Pressed → HANDBRAKE (0x1b), Released → HANDBRAKERELEASE (0x1c) | hold pair |
| Left mouse | R1 bit 0x0008 | DButtonR1 (19) | Pressed → FIREPRIMARY (0x1d), Released → FIREPRIMARYRELEASE (0x1e) | hold pair |
| B | R2 bit 0x0002 | DButtonR2 (17) | Pressed → FIRESECONDARY (0x1f), Released → FIRESECONDARYRELEASE (0x20) | hold pair (B = the port's on-foot Alt. Fire key, kept consistent) |
| Right mouse | ○ bit 0x0020 | DButtonCircle (21) | Pressed → FIREGADGET (0x24), Released → FIREGADGETRELEASE (0x25) | hold pair, re-fires while held |
| E | L1 bit 0x0004 | DButtonL1 (18) | Pressed/Released → FIREGADGET (0x24/0x25) | second gadget key |
| Q | Select bit 0x0100 | DButtonSelect (24) | Pressed → TOGGLESECONDARY (0x21) | edge one-shot, next |
| Wheel up | L3 bit 0x0200 | DButtonL3 (25) | Pressed → TOGGLESECONDARYUP (0x22) | edge one-shot, next |
| Wheel down | R3 bit 0x0400 | DButtonR3 (26) | Pressed → TOGGLESECONDARYDOWN (0x23) | edge one-shot, previous |
| C | Start bit 0x0800 | DButtonStart (27) | Pressed → CHANGECAMERA (0x26) | edge one-shot |
| Tab / middle mouse (hold) | □ bit 0x0080 | DButtonSquare (23) | Pressed → CAMLOOKBACK (0x29), Released → CAMLOOKBACKRELEASE (0x2a) | hold latch |
| R | d-pad Up bit 0x1000 | DButtonUp (28) | Pressed → POVRELOAD (0x3f) | edge one-shot |
| Esc | — | — | the frontend's pause (not a GAMEACTION; label 0x315) | — |

The arrows steer through the analog axis like A/D because the digital steer actions are dead ids
(§1) — synthesizing d-pad bits for them would do nothing. The d-pad Right/Down/Left bits (29/30/31)
stay unbound. All 16 pad buttons/pressures used: L2 (16) unbound, R2 = B, L1 = E, R1 = LMB,
△ pressure = W, ○ = RMB, ✕ pressure = S, ✕ bit = Space, □ = Tab/MMB, Select = Q, L3 = wheel up,
R3 = wheel down, Start = C, d-pad Up = R.

## 4. The labels (USATxt.dat, [M] parsed from the disc)

The driving controls' text block, bank 0, contiguous ids 0x30a–0x315 (2801-entry table, fixups
partition it; `Lang::parse`/`Lang::bind` in `crates/nf-formats/src/hud.rs:166-241`, `Txt_BindLabel`
0x1cad60 = `fixups[id>>24] + (id & 0xffffff)`):

| id | text | | id | text |
|---|---|---|---|---|
| 0x30a | Gas | | 0x310 | Next Weapon |
| 0x30b | Brake | | 0x311 | Use Gadget |
| 0x30c | Steer Left-Right | | 0x312 | Look back |
| 0x30d | Fire | | 0x313 | Camera View |
| 0x30e | "Cycle Weapon " | | 0x314 | E-Brake |
| 0x30f | Previous Weapon | | 0x315 | Pause |

Bank 1: 0x100032a `Brake/Reverse`, 0x100032b `Handbrake`. No horn/honk/throttle/accelerate/reload
control label exists in the whole table [M-negative, whole-word scan]. The frontend's
controller-style list (`Menu_DisplayControllerStyleList` 0x1f44d8, 8 styles) has **no driving
style** — the 0x30a block is never bound by ACTION.ELF's decomp; it is the driving manual/controls
vocabulary. `PC_DRIVING_CONTROLS` in `crates/nf-view/src/menu.rs` (next to the on-foot
`PC_CONTROLS`) carries (key, label id, fallback) rows for the main agent's draw pass; the two rows
without a game label (B Alt. Fire, R Reload) use id 0 + fallback text like the on-foot table does.

## 5. The port (`crates/nf-game/src/input/driving.rs`)

- `move_toward` — `FUN_002104a8` bit-exact [M].
- `CarInputStage` — the vehicle-layer input block of the drain `FUN_0018add8` as a per-tick
  struct: `action(&ActionMsg)` ports the switch's input cases (1, 0x16, 0x17, 0x1a, 0x1b/0x1c,
  0x1d–0x21), `tick()` ports the tail exactly (gas ×2 clamp, totals `+0x2bc = max(GAS, gas-side)`,
  `+0x2c0 = max(BRAKE, brake-side)`, the smaller-of-gas/brake zeroed with ties zeroing gas
  [M :223-230], handbrake zeroing [M :231-238], the ±0.2 chop write-back, the 0.25 steer ramp,
  full-lock pre-chop). Field names carry the car offsets in their doc comments. The wheelspin
  state machine and the `+0x29f` camera-relative bit stay out (level context, not input).
- `KeyboardMouse` — the §3 table as fields; `to_raw_pad` synthesizes the slot; mode 0 (analog, so
  the pressure path runs).
- `pc_driving_bindings()` — the §3 binding table as `ValueBindings` (kinds Axis/AButton/DButton,
  types Always/Pressed/Released).
- `PcDrivingInput::tick(now)` — one 60 Hz step of the whole chain: `to_raw_pad` → `poll_device` →
  `evaluate_value` per value (the change gate; no send-all) → the action messages (returned, and
  pushed into the stage) → `stage.tick()`. Getters expose the stage (steer/throttle/brake/
  handbrake/fire flags) for the level loop and the draw code. The driving levels themselves stay
  unwired (as in driving-input.md §12); this is the flow a level calls.

## 6. Tests (headless, numbers only — `cargo test -p nf-game -p nf-view`)

Inline in `input/driving.rs`: the ramp goldens (0→1 in exactly 4 ticks, ±1 sweep 8, release back
in 4, step values 0.25/0.5/0.75), the ±0.2 chop write-back and the strict 0.9 full-lock edge, the
gas ×2 clamp (0.3→0.6→1.0), the exclusion tie rule (equal gas/brake zeroes gas), the handbrake
zeroing, and per-key chain tests: each key's first tick produces exactly its action id (and only
that one for holds — no repeats over 10 held ticks) and its release produces the RELEASE id. The
full-chain test runs W+D through 5 ticks (steer 0.25→1.0, throttle 1.0) then releases (back to 0).
`crates/nf-view/tests/driving_controls_live.rs` binds every `PC_DRIVING_CONTROLS` label id from
the disc's USATxt.dat ($NF_ISO-gated) and asserts the expected words, with sister assertions
(wrong-word negatives) that can fail.

## 7. Live checks for the main agent

The shipped `Master.def` bindings are unextractable here, so these PCSX2 checks would settle what
the sandbox cannot:

1. **Steer binding shape**: in a driving mission (e.g. MIS01), set a watch on the car's raw steer
   `car+0x2c4` (car pointer at `*(u32*)(0x339190+0xdcc)`) while steering with the **left stick
   half-deflected and held still**. Expected if the shipped binding is type 0/Always-on-change:
   `+0x2c4` holds the last value and the ramped `+0x240` walks to it at 0.25/tick. If `+0x2c4`
   instead keeps re-writing, the binding repeats per frame — would change nothing for the PC port
   (digital keys re-send only on edge).
2. **GAS compounding check**: hold analog gas at a mid value with the stick still and watch
   `car+0x278`: if it walks 0.3 → 0.6 → 1.0 the GAS binding is change-gated (the ×2 clamp
   compounds); if it sits at 2·v the binding re-sends every frame. The PC port's digital W is
   unaffected either way (1.0 → min(2,1) = 1.0 on the first tick).
3. **D-pad steer negative**: press only d-pad Left/Right in a driving mission and watch
   `car+0x2c4`: expected **no change** (actions 8/9 dead, §1) — confirming the shipped config
   steers by stick only, which is what justifies mapping the PC arrows through the analog axis.

## 8. What's left

- The main agent's draw pass for a driving controls page: consume `PC_DRIVING_CONTROLS`
  (crates/nf-view/src/menu.rs, next to `PC_CONTROLS`) — data only, no drawing delivered.
- The driving level loop stays unwired (per the task): `PcDrivingInput::tick` is the entry a
  driving level calls per 60 Hz step; nothing in the level code calls it yet.
- The shipped `Master.def`/`FrontEnd.def` binding data (mission VIVs) remains unextractable on the
  slim ISO — the §3 table is the port's design [S], evidence-tagged per row.
- The wheelspin state machine (+0x29b/+0x299) and the handbrake `+0x29f` direction bit are level/
  physics context, deliberately out of the input struct (§5).
