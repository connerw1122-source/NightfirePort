## Your task: `fix-input`, gamepad support, the 8 controller styles, and the aim ramps

The user plays with keyboard and mouse, and doesn't like tasks that need a controller. But the game is a pad game, and the port should support one. Open items: GAPS G-003 and G-005, and `docs/systems/input.md`.

1. **The 8 controller styles** (G-005): generate the full style × action table mechanically from `psiInput_MapInputs`, and store it as data in nf-game, with a test that compares it against the decompile's table in the EE snapshot ([L]).
2. **The aim and turn ramps** (G-003): `Player_Aiming` state +0x8f4/+0x8f8/+0x900, how the stick's turn speed ramps up and the aim-mode slowdown, and the dead zone (G-012: the game overwrites the raw analog bytes in place, so read the code between `scePadRead` and the dead-zone loop).
3. **A gamepad backend in nf-view:** map an XInput/DualShock-style pad onto the PS2 pad bits and analog bytes, then run the game's own mapping and curves on them, so pad play is the game's. If this needs a crate (e.g. `gilrs`), name it, its licence and size in your report, and put it behind a cargo feature `gamepad` so master builds without it until the user approves. Keyboard and mouse stay the default and must not change.
4. **Mouse look stays as it is.** Don't touch its feel; the user tuned it.

Tests: style tables vs the snapshot; a synthetic stick input through the ramp gives the decompile's turn speeds tick by tick, with a sister assertion; keyboard input still produces the exact same `Input` as before (a regression test over a recorded trace).

Files: `crates/nf-game/src/player.rs` and `input` modules, `crates/nf-view/src/lib.rs` (the backend, minimal), `docs/systems/input.md`; update G-003, G-005 and G-012.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-input`.

---
claimed-by: sandbox6 at 2026-10-13T13:41Z

---
## Report (done, 2026-10-13T15:45Z)


---
## Report (done, 2026-10-13T15:46Z)

# fix-input — gamepad support, the 8 controller styles, the aim ramps

Branch `s/fix-input`. Closes G-005 and G-012, and the ramp half of G-003.

## What's done

1. **The 8 controller styles (G-005), as generated data in nf-game.** `scripts/gen_styles.py` parses the
   decomp of `psiInput_MapInputs` (ACTION.ELF 0x10cd80) and emits `crates/nf-game/src/input/styles_gen.rs`:
   per style the six analog sources of actions 0..5 (byte offset, sign, the conditional bytes — style 2's L1
   swaps the strafe source 0x128↔0x12a, style 6 adds ±1.0 from L1/R1), every (action, mask) button write,
   the ±1 d-pad axis (styles 4/5 map Triangle/Cross onto it), style 5's L1-gated skip of actions 7/8, and
   the shared tail (Start/d-pad/face writes, action 13's per-style mask, the LX/LY ±0.5 menu overwrites,
   action 38's LX<−0.8 && RX>0.8 && Up triple, player 0's Start|Cross → action 25). The generator hard-fails
   on anything it cannot read; nothing was transcribed from memory.
2. **The dead zone (G-012), answered from the code.** `psiInput_PollDevices` 0x10ca78: threshold 40, scale
   1.4597701 (=127/87), written in place to +0x128..0x12b and to the curve copy +0x148..0x14b (which it
   overwrites — dead code). The raw bytes exist only between `scePadRead` (`ps2_handle_pad` 0x10c738 touches
   just the button bytes +0x126/0x127) and the loop, so the code pins the behaviour; ported as
   `input::dead_zone` with the exact f32 outputs (trunc-then-char-wrap) unit-tested.
3. **The ramps (G-003).** `AccelFunc0` 0x1a9218 = `input::AccelState`: the state moves toward `base`
   (|stick| < hi) or `base·mul` by `(base·mul − base)/steps` per tick, |stick| ≤ lo resets, clamp
   ±base(1+mul), target UNSIGNED (the stick's sign rides the multiply outside [M 0x191dd0:143-148]).
   `Player_Aiming` 0x1a1d28 inlines the same shape for +0x8f4/+0x8f8 (aim edge-turn), +0x900 (free-look
   pitch), +0x8ec/+0x8f0 (scope); `Player_Init` 0x1910f8:57-63 zeroes all six states.
4. **The gamepad backend** behind the `gamepad` cargo feature: `gilrs` 0.11.2 (licence MIT OR Apache-2.0;
   ~6 crates + `libudev` on Linux — needs the system `libudev-dev`) drains the OS pad in
   `crates/nf-view/src/gamepad.rs` and maps it onto the PS2 pad bits and raw analog bytes (the §2 layout),
   then `nf_game::input::PadDriver::tick` runs the game's own path — dead zone → style mapping →
   `StickCompensation2/3` (the curve table read from the user's ACTION.ELF; the game's literal 0.003921569
   multiply, not ÷255) → invert → clamp → ramps — and merges into the keyboard's `Input` (axes add+clamp,
   buttons OR). No pad connected → a no-op. **Keyboard and mouse are untouched**: the recorded-trace test
   pins 180 ticks of scripted keyboard input bit-exact against goldens recorded from the pre-change tree.
5. `docs/systems/input.md` gained §9 documenting the port's pad path; G-003/G-005/G-012 updated in
   `docs/GAPS.md`.

## How it was verified

- **[L] Style table**: `input_styles_live.rs` runs the game's own `psiInput_MapInputs` in the R5900
  interpreter over the user's ACTION.ELF (8 styles × 21+ synthetic pad states: every button bit alone,
  stick extremes and threshold edges, the L1/R1 conditional paths, the action-38 triple) and compares all
  40 action floats bit-exact. Sister: style 0 vs 1 on Cross→action 8, style 4's right-stick forward,
  style 6's L1/R1 strafe offset — the styles differ exactly where the decomp says.
- **[L] Curves**: `MapAnalogStick` (f12→f0) and `StickCompensation2` (two float pointers) run in the
  interpreter and match to ±1 ulp (the interpreter's `div.s` rounds through f64 — `nf-formats/src/ee.rs`
  — while the real EE and the port divide IEEE-direct; the curve lookup and sign are exact).
- **[L] Ramps**: `input_ramp_live.rs` runs `AccelFunc0` tick by tick (EE float-arg convention: six floats
  in f12..f17, the state pointer in a0 — decoded from the prologue `mul.s f3,f13,f14` / `abs.s f0,f12` /
  `lwc1 f1,0(a0)`), 200 ramp-up ticks + reset + sub-hi + unsigned-target sisters, state and return value
  matched per tick.
- **Keyboard regression**: `fix_input_kbd_trace.rs` — a scripted 180-tick keyboard/mouse trace on
  07000005 through the real load path, compared bit-exact (pos/yaw/pitch/local/vel/cc every 30 ticks)
  against goldens recorded from the pre-change tree.
- Unit tests: the dead zone's exact outputs; `AccelState`'s shape (step/reset/clamp/target).
- Full suite (`NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p
  nf-audio -p nf-view -- --test-threads=1`): 119 test binaries ok; the only failures are the 3 allowed
  `disc_golden` trio on the slim ISO. Both feature-off (the default, no gilrs) and feature-on
  (`--features gamepad`) builds compile and link.

## What's left

- **A real pad playtest** (the main agent's render check): the backend compiles but no pad device exists in
  this sandbox, so the end-to-end pad feel is unverified. Suggested `--bond` spots on 07000005, each after
  plugging a pad: `--bond 105.34,25.6,109.38,0.6,0` — hold the left stick forward: Bond walks the same lane
  the keyboard trace does; push the right stick: the yaw ramps up over ~120 ticks (half-speed at first,
  full speed when pinned); tap Triangle (jump) and Cross (crouch) per style 0's table.
- The gamepad's **weapon switching** needs the hold-counter active-flag logic of `Input_Update` (actions
  20/21, threshold 46); the pad currently shares the keyboard's cycle mechanisms (Q/E/number keys).
- The scope branch's ramp states (+0x8ec/+0x8f0) are carried in `PadDriver` but the driver's scope path is
  a stub (the game's `Plr_ScopeSpeed_*` values ride the same `AccelState::step`); wire the `scoped` flag to
  `weapon_data+0x68` bit 0x40 when the port grows scoped-weapon aim mode.
- `PadDriver`'s style is fixed at construction (`PadConfig::style`, default 0); `Input_ChangeControllerStyle`
  0x17f5f0's flush-on-change semantics are not modelled.
- `Cargo.lock` gained the optional `gilrs` subtree (recorded, so master's `cargo fetch` warms it; the build
  without the feature is unchanged).

## Live checks for the main agent

1. [L] `psiInput_MapInputs` style writes: set style 4 in the pause menu, hold R1 — the fire action
   (`Input_Actionf(slot, 9, 1)` read at 0x1a1d28:196 or the muzzle) should fire, since style 4's action 9
   is L1 (mask 0x10, case 4 line 240) not R1. Confirms the interpreter result on real hardware.
2. [L] G-012 residual: PCSX2 watch on tSlot+0x128..0x12b (0x2457a8) between the `scePadRead` return
   (0x10c85c) and the `psiInput_PollDevices` loop (0x10cb90) — the bytes should be the raw 0..255 values,
   and after the loop the dead-zoned ones (127 at rest, 0/255 at full deflection).
