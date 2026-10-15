//! The pad input path [M docs/systems/input.md]: the game's own mapping from PS2 pad bits and
//! analog bytes to the 40 action floats, the stick compensation and the turn/aim ramps.
//!
//! - `psiInput_MapInputs` 0x10cd80: controller style (PlayerSetting+0xe) -> actions, as data in
//!   `styles_gen` (generated from the decomp by scripts/gen_styles.py).
//! - `psiInput_PollDevices` 0x10ca78: the per-axis dead zone (threshold 40, scale 1.4597701 = 127/87)
//!   that overwrites the raw bytes in place — G-012.
//! - `StickCompensation2/3` 0x17f798/0x17f850 with `MapAnalogStick` 0x17f958 and the game's curve
//!   table `gAnalogStickMappingFunction` (129 bytes at 0x2a3840; read from the user's ELF, never committed).
//! - `AccelFunc0` 0x1a9218: the ramp ("acceleration") helper; `Player_Aiming` 0x1a1d28 inlines the same
//!   shape for its edge/free-look states (+0x8f4/+0x8f8/+0x900, scope +0x8ec/+0x8f0).

mod styles_gen;

/// The driving levels' input path from DRIVING.ELF (G-004) [M docs/systems/driving-input.md]:
/// the pure PollDevice/binding port; the driving levels stay unwired.
pub mod driving;

/// One dead-zoned axis byte's tSlot offset [M docs/systems/input.md §2, WORKLOG 0006].
pub const AX_RX: u16 = 0x128;
pub const AX_RY: u16 = 0x129;
pub const AX_LX: u16 = 0x12a;
pub const AX_LY: u16 = 0x12b;

/// One analog action source in a controller style [M psiInput_MapInputs 0x10cd80 case bodies].
#[derive(Clone, Copy, Debug)]
pub struct AnaSrc {
    /// The dead-zoned byte this action reads: the tSlot analog offset (AX_*).
    pub byte: u16,
    pub negate: bool,
    /// When this button mask is held, the action reads `cond_set_byte` instead of `byte`
    /// (style 2's strafe: L1 swaps RX in; style 6's free look: R1 — the same byte both ways, generated 0).
    pub cond_mask: u16,
    pub cond_set_byte: u16,
    /// +1.0 added to the action while this mask is held (style 6's strafe, L1).
    pub add_mask: u16,
    /// −1.0 added while this mask is held (style 6's strafe, R1).
    pub sub_mask: u16,
}

/// One (action, mask) button write [M 0x10cd80]: the action takes the masked bits as a float
/// (e.g. mask 0x40 held -> +64.0) before Input_Update's clamp.
#[derive(Clone, Copy, Debug)]
pub struct BtnSrc {
    pub action: u8,
    pub mask: u16,
}

/// One controller style [M psiInput_MapInputs 0x10cd80, cases 0..7].
#[derive(Clone, Copy, Debug)]
pub struct StyleMap {
    /// Actions 0..5 (turn, strafe, forward, crosshair X/Y, free look).
    pub analog: [AnaSrc; 6],
    /// The (action, mask) button writes.
    pub buttons: &'static [BtnSrc],
    /// The ±1 axis action: (action, minus-one mask, plus-one mask) — d-pad Up/Down in most styles,
    /// Triangle/Cross in styles 4 and 5 [M the 0xbf800000/0x3f800000 writes of each case].
    pub dpad: (u8, u16, u16),
    /// Style 5: the action 7/8 writes sit inside `if ((buttons & M) == 0)`'s then-branch, so both are
    /// skipped while the mask is held [M case 5 decomp lines 268-275].
    pub skip: Option<u16>,
}

/// A PS2 pad snapshot post-`scePadRead` [M ps2_handle_pad 0x10c738: the buffer at slot+0x124]:
/// the buttons as the game stores them (~(data[2]<<8|data[3]), held bits set) and the four RAW
/// analog bytes (0 = left/up, 255 = right/down).
#[derive(Clone, Copy, Debug, Default)]
pub struct PadState {
    pub buttons: u16,
    pub rx: u8,
    pub ry: u8,
    pub lx: u8,
    pub ly: u8,
    /// slot state 99 = ready [M psiInput_PollDevices 0x10ca78]; false = disconnected (buttons 0, sticks 0x7f).
    pub connected: bool,
}

impl PadState {
    /// A neutral (disconnected) pad's bytes [M 0x10ca78:61-66].
    pub fn disconnected() -> PadState { PadState { buttons: 0, rx: 0x7f, ry: 0x7f, lx: 0x7f, ly: 0x7f, connected: false } }
}

/// The game's analog dead zone [M psiInput_PollDevices 0x10ca78:27-58] — G-012.
///
/// Per axis (in place, both the raw +0x128..0x12b bytes and the curve copy at +0x148..0x14b):
/// `v = byte − 127`; |v| < 40 -> 0; else `(v ∓ 40) × 1.4597701` (127/87, full deflection stays ≈127);
/// result + 127. The curve copy written just before (`gAnalogStickMappingFunction[|v|]·sign/2 + 0x7f`)
/// is overwritten in the same iteration and has no effect [M 0x10ca78:37-55; docs/systems/input.md §2].
/// The game reads the raw bytes ONLY inside this loop — between `scePadRead` (ps2_handle_pad 0x10c738,
/// which touches just the button bytes +0x126/0x127) and the loop nothing writes +0x128..0x12b, so the
/// raw values live only there (the live check G-012 asked for).
pub fn dead_zone(b: u8) -> u8 {
    let v = b as i32 - 127;
    let m = if v > 0 {
        if v >= 40 { ((v - 40) as f32 * 1.4597701) as i32 } else { 0 }
    } else if v < 0 {
        if v <= -40 { ((v + 40) as f32 * 1.4597701) as i32 } else { 0 }
    } else { 0 };
    // the game: `(char)(int)fVar10 + 0x7f` — the float truncates to int, then char arithmetic wraps
    (m as i8 as u8).wrapping_add(0x7f)
}

/// The game's curve table and the style, bound once [M gAnalogStickMappingFunction 0x2a3840].
/// The 129 bytes come from the user's ACTION.ELF (`from_action_elf`); the game never reads them
/// through a symbol the port could miss — they are straight data.
#[derive(Clone)]
pub struct PadConfig {
    /// `gAnalogStickMappingFunction` (the used copy at 0x2a3840).
    pub curve: Vec<u8>,
    /// PlayerSetting+0xe [M Input_ChangeControllerStyle 0x17f5f0].
    pub style: u16,
    /// The look-invert flag [M PlayerSetting+0x0, Input_Update's negate of actions 4/5].
    pub invert_look: bool,
    /// Player 0 maps Start|Cross onto action 25 [M 0x10cd80:415-417].
    pub player0: bool,
}

impl PadConfig {
    /// Reads the curve table from ACTION.ELF by symbol [M 0x2a3840, 129 bytes].
    pub fn from_action_elf(elf: &[u8]) -> Option<PadConfig> {
        let e = nf_formats::elf::Elf::parse(elf).ok()?;
        let syms = e.symbols(elf).ok()?;
        let addr = syms.iter().find(|s| s.name == "gAnalogStickMappingFunction" && s.size == 129)?.value;
        let segs = e.load_segments(elf).ok()?;
        let curve = segs.iter().find_map(|(va, b, _)| {
            (addr >= *va && (addr - *va) as usize + 129 <= b.len())
                .then(|| b[(addr - *va) as usize..][..129].to_vec())
        })?;
        Some(PadConfig { curve, style: 0, invert_look: false, player0: true })
    }
}

/// `psiInput_MapInputs` 0x10cd80 as data-driven Rust: the dead-zoned bytes + buttons of one slot
/// through one style -> the 40 action floats. `player0` adds the Start|Cross action-25 write.
/// Actions the decomp never writes (17-24, 35-37, 39) stay 0.0 — the game's memory would keep the
/// previous tick's value there, but every reader masks through the active/hold flags (0x104/0x12c),
/// which the port's Input does not model; see docs/systems/input.md §3.
pub fn map_inputs(style: u16, pad: &PadState, cfg: &PadConfig) -> [f32; 40] {
    let mut acts = [0.0f32; 40];
    let st = &styles_gen::STYLES[(style as usize).min(7)];
    let b = |byte: u16| -> f32 {
        // the +0x148..0x14b copies hold the same dead-zoned values as +0x128..0x12b [M 0x10ca78:53-55]
        let byte = match byte { 0x148 => AX_RX, 0x149 => AX_RY, 0x14a => AX_LX, 0x14b => AX_LY, other => other };
        (match byte { AX_RX => pad.rx, AX_RY => pad.ry, AX_LX => pad.lx, AX_LY => pad.ly, _ => 0x7f }) as f32
    };
    // the analog actions 0..5 [M the case bodies]
    let skip = st.skip.unwrap_or(0);
    for (act, src) in st.analog.iter().enumerate() {
        let mut byte = src.byte;
        if src.cond_mask != 0 && pad.buttons & src.cond_mask != 0 { byte = src.cond_set_byte; }
        let mut v = (b(byte) - 127.0) * 0.0078125;
        if src.negate { v = -v; }
        if src.add_mask != 0 && pad.buttons & src.add_mask != 0 { v += 1.0; }
        if src.sub_mask != 0 && pad.buttons & src.sub_mask != 0 { v -= 1.0; }
        // style 5: actions 7 and 8 are only written while the skip mask is CLEAR
        if st.skip.is_some() && (act == 7 || act == 8) && pad.buttons & skip != 0 { continue; }
        acts[act] = v;
    }
    // the d-pad ±1 axis [M the 0xbf800000/0x3f800000 writes]
    let (dact, dneg, dpos) = st.dpad;
    if pad.buttons & dneg != 0 { acts[dact as usize] = -1.0; }
    if pad.buttons & dpos != 0 { acts[dact as usize] = 1.0; }
    // the button writes [M the case's `(float)(buttons & MASK)` stores]
    for btn in st.buttons {
        if st.skip.is_some() && (btn.action == 7 || btn.action == 8) && pad.buttons & skip != 0 { continue; }
        acts[btn.action as usize] = (pad.buttons & btn.mask) as f32;
    }
    // the common tail [M 0x10cd80:377-422]
    for &(act, mask) in styles_gen::TAIL_BUTTONS {
        acts[act as usize] = (pad.buttons & mask) as f32;
    }
    if cfg.player0 {
        for &(act, mask) in styles_gen::TAIL_P0_BUTTONS {
            acts[act as usize] = (pad.buttons & mask) as f32;
        }
    }
    // the stick-menu overwrites [M 0x10cd80:384-409]: LX ±0.5 -> 28/29, LY ±0.5 -> 26/27
    let (lx_neg, lx_pos, ly_neg, ly_pos) = styles_gen::TAIL_STICK_MENU;
    let lx = (b(AX_LX) - 127.0) * 0.0078125;
    let ly = (b(AX_LY) - 127.0) * 0.0078125;
    if lx < -0.5 { acts[lx_neg as usize] = 1.0; }
    if lx > 0.5 { acts[lx_pos as usize] = 1.0; }
    if ly < -0.5 { acts[ly_neg as usize] = 1.0; }
    if ly > 0.5 { acts[ly_pos as usize] = 1.0; }
    // action 38: LX < −0.8 && RX > 0.8 && Up [M 0x10cd80:418-422]
    let rx = (b(AX_RX) - 127.0) * 0.0078125;
    if lx < -0.8 && rx > 0.8 && pad.buttons & 0x1000 != 0 { acts[38] = 1.0; }
    acts
}

/// `MapAnalogStick` 0x17f958: `i = (int)(x·127)`, |i| clamped to 128, then the SIGNED integer
/// `±curve[|i|]` converts to float and multiplies by the literal 0.003921569 (not ÷255 — the
/// constant's rounding differs by 1 ulp and the interpreter test pins it).
fn map_analog_stick(x: f32, curve: &[u8]) -> f32 {
    let i = (x * 127.0) as i32;
    let mut a = i.abs();
    if a > 128 { a = 128; }
    let v = if i < 0 { -(curve[a as usize] as i32) } else { curve[a as usize] as i32 };
    v as f32 * 0.003921569
}

/// Test hook: `map_analog_stick` for the live tests (the crate pins the constant's rounding).
#[doc(hidden)]
pub fn map_analog_stick_for_test(x: f32, curve: &[u8]) -> f32 { map_analog_stick(x, curve) }

/// The per-player stick compensation [M Input_Update 0x17ee88; docs/systems/input.md §4]:
/// styles 2 and 5: Comp2(3,4), Comp3(0,2); all others: Comp2(3,4), Comp2(0,5), Comp3(1,2).
pub fn compensate(acts: &mut [f32; 40], style: u16, curve: &[u8]) {
    let radial = |a: f32, b: f32| -> (f32, f32) {
        let m = (a * a + b * b).sqrt();
        if m.abs() > 0.0002 {
            let k = map_analog_stick(m, curve) / m;
            (a * k, b * k)
        } else { (a, b) }
    };
    // Comp3 0x17f850: if one axis is within ±0.707107 and the other beyond, the other becomes ±1
    let snap = |a: f32, b: f32| -> (f32, f32) {
        let (x, y) = (a, b);
        if x.abs() <= 0.707107 && y.abs() > 0.707107 { return (x, if y > 0.0 { 1.0 } else { -1.0 }); }
        if y.abs() <= 0.707107 && x.abs() > 0.707107 { return (if x > 0.0 { 1.0 } else { -1.0 }, y); }
        (x, y)
    };
    let (a3, a4) = radial(acts[3], acts[4]);
    acts[3] = a3; acts[4] = a4;
    if style == 2 || style == 5 {
        let (a0, a2) = snap(acts[0], acts[2]);
        acts[0] = a0; acts[2] = a2;
    } else {
        let (a0, a5) = radial(acts[0], acts[5]);
        acts[0] = a0; acts[5] = a5;
        let (a1, a2) = snap(acts[1], acts[2]);
        acts[1] = a1; acts[2] = a2;
    }
}

/// The pad path's action values for one tick: dead zone -> map -> compensate -> invert -> clamp,
/// the order `Input_Update` 0x17ee88 runs [M; docs/systems/input.md §1, §4, §5].
pub fn actions(pad: &PadState, cfg: &PadConfig) -> [f32; 40] {
    let mut a = map_inputs(cfg.style, pad, cfg);
    compensate(&mut a, cfg.style, &cfg.curve);
    if cfg.invert_look { a[4] = -a[4]; a[5] = -a[5]; }
    for v in a.iter_mut() { *v = v.clamp(-1.0, 1.0); }
    a
}

/// `AccelFunc0` 0x1a9218 as state [M; docs/systems/input.md §7]: the current speed moves toward
/// `base` (|stick| < hi) or `base·mul` (beyond) by `(base·mul − base)/steps` per call; |stick| ≤ lo
/// resets to `base`; the state clamps to ±base(1+mul). `Player_Aiming` inlines the same shape for
/// its +0x8f4/+0x8f8/+0x900/+0x8ec/+0x8f0 states with lo 0.05 hi 0.95 [M 0x1a1d28:33-64, 109-144, 198-232].
#[derive(Clone, Copy, Debug, Default)]
pub struct AccelState {
    pub state: f32,
}

impl AccelState {
    pub fn new(base: f32) -> AccelState { AccelState { state: base } }
    /// One tick; returns the new speed. The target is `base` (|stick| < hi) or `base·mul` —
    /// UNSIGNED, like the game's [M 0x1a9218: the target never flips sign; the stick's sign rides
    /// the multiply outside]. The negative-state branch exists only for the clamp's defence.
    pub fn step(&mut self, stick: f32, base: f32, mul: f32, steps: f32, lo: f32, hi: f32) -> f32 {
        let target = if stick.abs() < hi { base } else { base * mul };
        let d = (base * mul - base) / steps;
        let mut s = self.state;
        if s < target {
            s += d;
            if target < s { s = target; }
        } else if target < s {
            s += d;
            if s < target { s = target; }
        }
        if stick.abs() <= lo { s = base; }
        let lim = if s >= 0.0 { base + base * mul } else { -(base + base * mul) };
        if (lim >= 0.0 && s > lim) || (lim < 0.0 && s < lim) { s = lim; }
        self.state = s;
        s
    }
}

/// The port's one-tick player input [M nf_game::player::Input].
use crate::player::Input;

/// The pad driver: the game's pad path tick by tick, bound to the tuning globals `ReadTuningVars`
/// (0x1d3a80) sets. Ramp states mirror BLData: +0x8fc yaw (Player_Move's AccelFunc0), +0x900 free-look
/// pitch, +0x8f4/+0x8f8 aim edge-turn, +0x8ec/+0x8f0 scope [M 0x191dd0:144-146, 0x1a1d28].
pub struct PadDriver {
    pub cfg: PadConfig,
    yaw: AccelState,
    free_look: AccelState,
    aim_x: AccelState,
    aim_y: AccelState,
    /// The scope branch's states [M 0x1a1d28:198-232]; driven once a scoped weapon reaches the driver
    /// (the caller passes `scoped`; see docs/reports/fix-input.md).
    #[allow(dead_code)]
    scope_x: AccelState,
    #[allow(dead_code)]
    scope_y: AccelState,
    prev_buttons: u16,
    /// The mirrored aim-box cursor (BL+0x118/0x11c) [M Player_Aiming writes; the Sim clamps ±0.296/±0.4].
    cursor_x: f32,
    cursor_y: f32,
}

impl PadDriver {
    pub fn new(cfg: PadConfig) -> PadDriver {
        PadDriver { cfg, yaw: AccelState::default(), free_look: AccelState::default(), aim_x: AccelState::default(),
            aim_y: AccelState::default(), scope_x: AccelState::default(), scope_y: AccelState::default(), prev_buttons: 0,
            cursor_x: 0.0, cursor_y: 0.0 }
    }

    /// One tick of the game's pad path. `tuning` supplies the live Plr_* values; `zoom` is BL+0x8d0,
    /// `underwater` the surfaced==0 flag, `scoped` weapon_data+0x68 bit 0x40. Returns the port's Input
    /// (turn radians, look in π/2 units) plus the raw actions for the caller's HUD/menus.
    pub fn tick(&mut self, pad: &PadState, tuning: &nf_formats::tuning::Tuning, zoom: f32, underwater: bool, scoped: bool) -> (Input, [f32; 40]) {
        let a = actions(pad, &self.cfg);
        let newly = pad.buttons & !self.prev_buttons;
        self.prev_buttons = pad.buttons;
        let mut input = Input::default();
        // Style 0's button actions [M the case 0 table; the port's conventions: 7 jump, 8 crouch,
        // 9 fire, 19 (0x13) aim mode]. Jump/crouch press on the newly-pressed edge, like the
        // keyboard path and the game's +0x120 "newly pressed" word [M ps2_handle_pad 0x10c738:89].
        input.jump = a[7] != 0.0 && newly & self.mask_of(pad, 7) != 0;
        input.crouch = a[8] != 0.0 && newly & self.mask_of(pad, 8) != 0;
        input.fire = a[9] != 0.0;
        input.aiming = a[19] != 0.0;
        // Player_Zoom 0x1a1a70: action 6 while aim mode is on.
        input.zoom = input.aiming && a[6] != 0.0;
        if input.aiming {
            // Manual aim [M Player_Aiming 0x1a1d28:78-194]: the crosshair moves inside the box at
            // Plr_AimSpeed × 0.125; pinned at an edge, the view turns at the ramped Plr_AimTurnSpeed.
            let dx = a[3] * tuning.Plr_AimSpeed_X * 0.125;
            let dy = a[4] * tuning.Plr_AimSpeed_Y * 0.125;
            let pin_x = a[3] * 0.125 != 0.0 && ((self.cursor_x + dx).abs() > 0.296);
            let pin_y = a[4] * 0.125 != 0.0 && ((self.cursor_y + dy).abs() > 0.4);
            if pin_x {
                let speed = self.aim_x.step(a[3] * 0.125, tuning.Plr_AimTurnSpeed_X, tuning.Plr_ScopeSpeed_X_Mul, tuning.Plr_ScopeSpeed_X_Steps, 0.05, 0.95);
                let mut t = a[3] * 0.125 * speed / zoom;
                if underwater { t *= 0.4; }
                input.turn = -t * std::f32::consts::FRAC_PI_2;
            } else {
                self.cursor_x = (self.cursor_x + dx).clamp(-0.296, 0.296);
                input.turn = -dx * 0.76966; // the Sim's aim block converts back to cursor units
            }
            if pin_y {
                let speed = self.aim_y.step(a[4] * 0.125, tuning.Plr_AimTurnSpeed_Y, tuning.Plr_ScopeSpeed_Y_Mul, tuning.Plr_ScopeSpeed_Y_Steps, 0.05, 0.95);
                let mut l = a[4] * 0.125 * speed / zoom;
                if underwater { l *= 0.7; }
                input.look = l;
            } else {
                self.cursor_y = (self.cursor_y + dy).clamp(-0.4, 0.4);
                input.look = dy * 0.57724 / std::f32::consts::FRAC_PI_2;
            }
        } else {
            // Free look [M Player_Move 0x191dd0:144-146 + Player_Aiming 0x1a1d28:31-74]: the yaw
            // takes −action0 through AccelFunc0 (Plr_NoAimTurnSpeed_X), the pitch takes action 5
            // through the same shape (Plr_NoAimTurnSpeed_Y).
            let turn_in = -a[0];
            let speed = self.yaw.step(turn_in, tuning.Plr_NoAimTurnSpeed_X, tuning.Plr_NoAimTurnSpeed_X_Mul, tuning.Plr_NoAimTurnSpeed_X_Steps, 0.05, 0.95);
            input.turn = turn_in * speed;
            let pspeed = self.free_look.step(a[5], tuning.Plr_NoAimTurnSpeed_Y, tuning.Plr_NoAimTurnSpeed_Y_Mul, tuning.Plr_NoAimTurnSpeed_Y_Steps, 0.05, 0.95);
            input.look = a[5] * pspeed;
            self.cursor_x *= 0.5;
            self.cursor_y *= 0.5;
        }
        let _ = scoped; // the scope branch (Plr_ScopeSpeed_*) rides the same AccelStates; see the report
        (input, a)
    }

    /// The button mask whose action is `act` (for the newly-pressed edge).
    fn mask_of(&self, _pad: &PadState, act: u8) -> u16 {
        let st = &styles_gen::STYLES[(self.cfg.style as usize).min(7)];
        for b in st.buttons { if b.action == act { return b.mask; } }
        0
    }

    /// The mirrored aim-box cursor (BL+0x118/0x11c).
    pub fn cursor(&self) -> [f32; 2] { [self.cursor_x, self.cursor_y] }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dead zone's exact outputs [M 0x10ca78:38-55, f32 arithmetic: threshold 40, scale
    /// 1.4597701, trunc-then-char-wrap]. G-012: the raw byte exists only between `scePadRead`
    /// and this loop — the code pins the behaviour the live check would observe.
    #[test]
    fn dead_zone_matches_the_decomp_constants() {
        // (raw, dead-zoned): centre, the ±40 thresholds (inside -> 0, first step -> ±1), the
        // extremes (full deflection stays ≈127), and a mid value.
        let cases = [
            (0u8, 0u8), (24, 36), (86, 126), (87, 127), (88, 127), (127, 127),
            (167, 127), (168, 128), (200, 175), (230, 218), (255, 255),
        ];
        for (raw, want) in cases {
            assert_eq!(dead_zone(raw), want, "raw {raw}");
        }
        // Sister: the zone is per-axis by construction (the function takes one byte), and the
        // ±40..±127 output range covers exactly 255..0.
        assert_eq!(dead_zone(255) as i32 - dead_zone(0) as i32, 255);
    }

    /// The ramp's shape [M AccelFunc0 0x1a9218]: reset at |stick| ≤ lo, target switch at hi,
    /// clamp at ±base(1+mul), per-tick step (base·mul − base)/steps.
    #[test]
    fn accel_state_shape() {
        let (base, mul, steps) = (0.04f32, 2.0f32, 120.0f32);
        let mut a = AccelState { state: base };
        a.step(1.0, base, mul, steps, 0.05, 0.95);
        assert!((a.state - (base + (base * mul - base) / steps)).abs() < 1e-9);
        // reset
        let mut a = AccelState { state: base * mul };
        a.step(0.0, base, mul, steps, 0.05, 0.95);
        assert_eq!(a.state, base);
        // clamp ceiling
        let mut a = AccelState { state: base * mul };
        a.step(1.0, base, mul, steps, 0.05, 0.95);
        assert_eq!(a.state, base * mul);
        // below hi the target is base: no growth
        let mut a = AccelState { state: base };
        a.step(0.9, base, mul, steps, 0.05, 0.95);
        assert_eq!(a.state, base);
    }
}
