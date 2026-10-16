//! The driving levels' input path from DRIVING.ELF [M docs/systems/driving-input.md] — G-004.
//!
//! `PS2PadDevice::PollDevice` 0x169288 builds a 32-float value message from the raw pad slot
//! bytes; the binding evaluator `FUN_00168678` turns value changes into `{action, device, value}`
//! messages through typed condition helpers. Everything here is a pure port of the decompiled
//! arithmetic (bit-exact constants from the disassembly); the driving levels stay unwired.
//!
//! Compared with the on-foot path (`super::input`, docs/systems/input.md §2) the dead zone is a
//! different mechanism: float-space chop `−0.3 < v < 0.3 → 0` [M 0x167c48] with no rescale, not
//! the byte-space threshold 40 + scale 127/87.

/// `FUN_00169218` 0x169218: stick byte → normalized float.
///
/// The multiplier is the disassembly literal 0x3c008081 [M docs/systems/driving-input.md §3],
/// which is exactly `2.0f32/255.0f32` (the division rounds the mantissa up to 0x008081); the
/// bit pattern is kept so the constant is pinned to the binary, not to a re-derivation.
pub fn stick_to_f32(b: u8) -> f32 {
    (b as f32) * f32::from_bits(0x3c00_8081) - 1.0
}

/// `FUN_00169248` 0x169248: analog button pressure byte → float, `min(1.0, b·(1/192))`.
///
/// 0x3baaaaab is exactly 1/192; the cap is the game's `1.0 <= v` test [M 0x169248 disasm].
pub fn pressure_to_f32(b: u8) -> f32 {
    let v = (b as f32) * f32::from_bits(0x3baa_aaab);
    if 1.0f32 <= v { 1.0 } else { v }
}

/// `FUN_00167c48` 0x167c48: the dead-zone chop — `if lo < v && v < hi { 0.0 }` (strict both
/// sides) [M 0x167c48 disasm]. PollDevice calls it with (−0.3, 0.3) on all four axes and never
/// rescales afterwards [M 0x169384 area].
pub fn chop(v: f32, lo: f32, hi: f32) -> f32 {
    if lo < v && v < hi { 0.0 } else { v }
}

/// The PollDevice dead-zone bounds [M 0x16931c/0x169330: 0xbe99999a / 0x3e99999a].
pub const DEAD_ZONE_LO: f32 = f32::from_bits(0xbe99_999a); // −0.3
pub const DEAD_ZONE_HI: f32 = f32::from_bits(0x3e99_999a); // +0.3

/// Raw pad-slot bytes as PollDevice reads them [M 0x259b78: the 0x20-byte struct at 0x419840
/// per slot; layout in docs/systems/driving-input.md §2].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawPadSlot {
    /// The slot's mode word [M 0x259b90 reads +0x0; 4 = digital mode, no pressure bytes].
    pub mode: u32,
    /// Active-high button bits at +0x0c, on-foot bit layout (0x0001 L2 … 0x8000 Left) [M].
    pub buttons: u16,
    /// Sticks at +0x10..0x13: 0 = left/up, 255 = right/down [M].
    pub rx: u8,
    pub ry: u8,
    pub lx: u8,
    pub ly: u8,
    /// Pressure bytes at +0x14..0x1f, libpad order: Right, Left, Up, Down, △, ○, ✕, □, L1, R1,
    /// L2, R2 [M 0x169288 sequential loop].
    pub pressures: [u8; 12],
}

/// Value indices in the 32-float message [M InputTable registration 0x168b08 name order].
pub mod value {
    pub const ALX: usize = 0;
    pub const ALY: usize = 1;
    pub const ARX: usize = 2;
    pub const ARY: usize = 3;
    /// AButtonRight..AButtonR2 = 4..15 in libpad pressure order [M].
    pub const ABUTTON_FIRST: usize = 4;
    pub const ABUTTON_LAST: usize = 15;
    /// DButtonL2..DButtonLeft = 16..31, bit i of the button u16 → 16+i [M].
    pub const DBUTTON_FIRST: usize = 16;
    pub const DBUTTON_LAST: usize = 31;
    /// Message length in floats [M PollDevice clears 0x80 bytes].
    pub const COUNT: usize = 32;
}

/// `FUN_00169288` 0x169288 — the bit-exact 32-float message build.
///
/// Order matters and is pinned: the all-sticks-zero guard zeroes only values 0..3 (buttons and
/// pressures still build) [M 0x1692c8–0x169318]; axes are LX, −LY, RX, −RY before the chop [M
/// 0x16931c–0x16937c]; the chop (−0.3, 0.3) applies to the four axes only [M 0x169384–0x1693b4];
/// the digital bit values always fill 16..31 [M 0x1693c8 area]; and digital mode (4) copies the
/// DButton floats {29,31,28,30,20,21,22,23,18,19,16,17} over values 4..15, while analog mode
/// fills 4..15 from the pressure bytes [M 0x169400 area + 0x169248].
pub fn poll_device(pad: &RawPadSlot) -> [f32; value::COUNT] {
    let mut v = [0.0f32; value::COUNT];

    if pad.lx == 0 && pad.ly == 0 && pad.rx == 0 && pad.ry == 0 {
        // guard leaves v[0..4] at 0.0
    } else {
        v[value::ALX] = stick_to_f32(pad.lx);
        v[value::ALY] = -stick_to_f32(pad.ly);
        v[value::ARX] = stick_to_f32(pad.rx);
        v[value::ARY] = -stick_to_f32(pad.ry);
        for i in 0..4 {
            v[i] = chop(v[i], DEAD_ZONE_LO, DEAD_ZONE_HI);
        }
    }

    for i in 0..16 {
        v[value::DBUTTON_FIRST + i] = if pad.buttons & (1 << i) != 0 { 1.0 } else { 0.0 };
    }

    if pad.mode == 4 {
        // Digital-mode remap: AButton slots take the DButton floats [M 0x169400 area].
        const REMAP: [usize; 12] = [29, 31, 28, 30, 20, 21, 22, 23, 18, 19, 16, 17];
        for (dst, src) in REMAP.iter().enumerate() {
            v[value::ABUTTON_FIRST + dst] = v[*src];
        }
    } else {
        for (i, &b) in pad.pressures.iter().enumerate() {
            v[value::ABUTTON_FIRST + i] = pressure_to_f32(b);
        }
    }
    v
}

/// Record kind [M InputTable 0x167788 third argument: 0 axis, 1 analog button, 2 digital button].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Axis,
    AButton,
    DButton,
}

impl Kind {
    fn num(self) -> u32 {
        match self {
            Kind::Axis => 0,
            Kind::AButton => 1,
            Kind::DButton => 2,
        }
    }
}

/// One 0xc-byte binding entry `{type, arg, action}` [M 0x168678 entry reads].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Binding {
    /// The condition type 0..9 [M docs/systems/driving-input.md §5].
    pub kind: BindingKind,
    /// The threshold argument (types 3/4; ignored elsewhere) [M entry[1]].
    pub arg: f32,
    /// The GAMEACTION id sent when the condition fires [M entry[2]; validated 1..0x40 by
    /// SendGameMessage 0x165850].
    pub action: u8,
}

/// The condition types [M the switch at 0x168678; thresholds pinned in the helpers below].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BindingKind {
    /// 0: send whenever the change gate lets the frame through.
    Always,
    /// 1: pressed edge [M 0x1678f8].
    Pressed,
    /// 2: released edge [M 0x1679c8].
    Released,
    /// 3: positive crossing at `arg` [M 0x167a98].
    CrossPos,
    /// 4: negative crossing at `arg` [M 0x167ad8].
    CrossNeg,
    /// 5: returned to centre, |cur| ≤ 0.2 after |prev| > 0.2 [M 0x167b18, 0x3e4ccccd].
    Centred,
    /// 6/7: the global flag read (caller supplies it; the game reads *(u32*)(0x3c8780+0x20),
    /// semantics unpinned) [M 0x1664c0/0x166d78].
    GlobalSet,
    GlobalClear,
    /// 8: pressed edge + auto-repeat (state machine below) [M case 8].
    PressRepeat,
    /// 9: released edge + auto-repeat [M case 9].
    ReleaseRepeat,
}

/// `FUN_001678f8` 0x1678f8 — pressed edge [M].
pub fn pressed_edge(kind: Kind, cur: f32, prev: f32) -> bool {
    match kind {
        Kind::DButton => cur == 1.0 && prev == 0.0,
        Kind::AButton | Kind::Axis => cur >= 0.5 && prev < 0.5,
    }
}

/// `FUN_001679c8` 0x1679c8 — released edge [M]. Note the axis falls at −0.5, not 0.5.
pub fn released_edge(kind: Kind, cur: f32, prev: f32) -> bool {
    match kind {
        Kind::DButton => cur == 0.0 && prev == 1.0,
        Kind::AButton => cur < 0.5 && prev >= 0.5,
        Kind::Axis => cur < -0.5 && prev >= -0.5,
    }
}

/// `FUN_00167a98` 0x167a98 — positive crossing at `thr` [M].
pub fn cross_pos(thr: f32, cur: f32, prev: f32) -> bool {
    cur >= thr && prev < thr
}

/// `FUN_00167ad8` 0x167ad8 — negative crossing at `thr` [M].
pub fn cross_neg(thr: f32, cur: f32, prev: f32) -> bool {
    cur < thr && prev >= thr
}

/// `FUN_00167b18` 0x167b18 — returned to centre within `dz` [M; PollDevice's caller passes 0.2].
pub fn centred(dz: f32, cur: f32, prev: f32) -> bool {
    (-dz <= cur && cur <= dz) && (prev < -dz || dz < prev)
}

/// `FUN_001677f8` 0x1677f8 — statically down [M].
pub fn is_down(kind: Kind, cur: f32) -> bool {
    match kind {
        Kind::DButton => cur == 1.0,
        Kind::AButton | Kind::Axis => cur >= 0.5,
    }
}

/// `FUN_00167878` 0x167878 — statically up (the axis threshold is −0.5) [M].
pub fn is_up(kind: Kind, cur: f32) -> bool {
    match kind {
        Kind::DButton => cur == 0.0,
        Kind::AButton => cur < 0.5,
        Kind::Axis => cur < -0.5,
    }
}

/// The `{action, device, value}` message SendGameMessage forwards to listeners [M 0x165850].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionMsg {
    pub action: u8,
    pub device: u32,
    pub value: f32,
}

/// Auto-repeat scratch — the game keeps this per record in message buffers C/D as ints
/// [M 0x168678 writes through the record's +0x2c/+0x30 pointers; 3 = disconnected, set on
/// unplug by 0x167e80].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RepeatState {
    pub last_tick: i32,
    /// 0 idle, 1 just fired, 2 repeating, 3 disconnected [M the state reads in cases 8/9].
    pub state: i32,
}

/// The repeat delays the container carries [M container +0xa8 / +0xac, read by cases 8/9].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RepeatDelays {
    pub first: i32,
    pub rest: i32,
}

/// One value's bindings plus its auto-repeat scratch (the game stores the scratch in the
/// InputTable record) [M].
pub struct ValueBindings {
    pub kind: Kind,
    pub bindings: Vec<Binding>,
    pub repeat: RepeatState,
}

/// `FUN_00168678` 0x168678 for one value — the binding evaluation, bit-exact.
///
/// `changed` is the raw A-vs-B float inequality of the message buffers [M 0x168678:
/// `cur != prev`]; `send_all` is container+4 [M]; `now` is container+0x28 [M]; `global_flag` is
/// the type-6/7 read (caller-supplied) [M 0x1664c0/0x166d78]. The change gate: when
/// `!send_all && !changed`, only the repeat types (8/9) evaluate [M the first test at
/// 0x168678].
pub fn evaluate_value(
    vb: &mut ValueBindings,
    cur: f32,
    prev: f32,
    changed: bool,
    send_all: bool,
    now: i32,
    delays: &RepeatDelays,
    global_flag: bool,
    device: u32,
    out: &mut Vec<ActionMsg>,
) {
    let send = |out: &mut Vec<ActionMsg>, action: u8| {
        // SendGameMessage's range check [M 0x165850: 0 < id < 0x41].
        if action > 0 && (action as u32) < 0x41 {
            out.push(ActionMsg { action, device, value: cur });
        }
    };
    for b in &vb.bindings {
        let fire = match b.kind {
            BindingKind::Always => changed || send_all,
            BindingKind::Pressed => (changed || send_all) && pressed_edge(vb.kind, cur, prev),
            BindingKind::Released => (changed || send_all) && released_edge(vb.kind, cur, prev),
            BindingKind::CrossPos => (changed || send_all) && cross_pos(b.arg, cur, prev),
            BindingKind::CrossNeg => (changed || send_all) && cross_neg(b.arg, cur, prev),
            BindingKind::Centred => (changed || send_all) && centred(0.2, cur, prev),
            BindingKind::GlobalSet => (changed || send_all) && global_flag,
            BindingKind::GlobalClear => (changed || send_all) && !global_flag,
            BindingKind::PressRepeat | BindingKind::ReleaseRepeat => {
                // Types 8/9 run regardless of the change gate [M the gate's type filter].
                let release = b.kind == BindingKind::ReleaseRepeat;
                let edge = if release { released_edge(vb.kind, cur, prev) } else { pressed_edge(vb.kind, cur, prev) };
                let held = if release { is_up(vb.kind, cur) } else { is_down(vb.kind, cur) };
                let mut fired = false;
                if edge {
                    vb.repeat.last_tick = now;
                    vb.repeat.state = 1;
                    fired = true;
                } else if held && vb.repeat.state != 3 {
                    let delay = if vb.repeat.state == 1 { delays.first } else { delays.rest };
                    if vb.repeat.last_tick.checked_add(delay).map_or(false, |t| t < now) {
                        vb.repeat.last_tick = now;
                        vb.repeat.state = 2;
                        fired = true;
                    }
                }
                fired
            }
        };
        if fire {
            send(out, b.action);
        }
    }
}

/// GAMEACTION ids from the .data table at 0x32c098 [M docs/systems/driving-input.md §7].
pub mod action {
    pub const STEER: u8 = 1;
    pub const STEERVERTICAL: u8 = 2;
    pub const STEERVERTICALINV: u8 = 3;
    pub const STEERLEFT: u8 = 4;
    pub const STEERRIGHT: u8 = 5;
    pub const STEERUP: u8 = 6;
    pub const STEERDOWN: u8 = 7;
    pub const STEERLEFTDIGITAL: u8 = 8;
    pub const STEERRIGHTDIGITAL: u8 = 9;
    pub const STEERUPDIGITAL: u8 = 0xa;
    pub const STEERDOWNDIGITAL: u8 = 0xb;
    pub const GAS: u8 = 0x16;
    pub const GASBRAKE: u8 = 0x17;
    pub const WHEELSPIN: u8 = 0x18;
    pub const WHEELSPINRELEASE: u8 = 0x19;
    pub const BRAKE: u8 = 0x1a;
    pub const HANDBRAKE: u8 = 0x1b;
    pub const HANDBRAKERELEASE: u8 = 0x1c;
    pub const FIREPRIMARY: u8 = 0x1d;
    pub const FIREPRIMARYRELEASE: u8 = 0x1e;
    pub const FIRESECONDARY: u8 = 0x1f;
    pub const FIRESECONDARYRELEASE: u8 = 0x20;
    pub const TOGGLESECONDARY: u8 = 0x21;
    pub const TOGGLESECONDARYUP: u8 = 0x22;
    pub const TOGGLESECONDARYDOWN: u8 = 0x23;
    pub const FIREGADGET: u8 = 0x24;
    pub const FIREGADGETRELEASE: u8 = 0x25;
    pub const CHANGECAMERA: u8 = 0x26;
    pub const CAMLOOKBACK: u8 = 0x29;
    pub const CAMLOOKBACKRELEASE: u8 = 0x2a;
    pub const POVRELOAD: u8 = 0x3f;
    pub const RESUMEAUTODRIVE: u8 = 0x40;
    /// The SendGameMessage bound [M 0x165850: `0 < id < 0x41`].
    pub const MAX: u32 = 0x40;
}

/// The PC keyboard+mouse → raw pad bytes mapping for the driving missions
/// [S — the shipped bindings live in data\Control\Master.def/FrontEnd.def, not extractable here;
/// the design and its per-row evidence is docs/systems/driving-pc-input.md §3].
///
/// Sign conventions come from PollDevice itself [M 0x169218 + the LY negation at 0x169288]:
/// LX byte 255 (right) → ALX +1, byte 0 → −1; the analog-button pressures are min(1, b/192)
/// [M 0x169248]. Steering rides the analog LX axis because the digital steer actions (4/5/8/9/
/// 0xa/0xb) have no consumer in DRIVING.ELF [M-negative, driving-pc-input.md §1] — the vehicle
/// ramp (`CarInputStage`) shapes the digital 0/1 exactly like a full stick deflection. Gas and
/// brake ride the analog-button **pressures** (not the LY axis) so each pedal is its own value;
/// a single shared axis would send one action the other's sign (GASBRAKE exists for the pad's
/// shared axis, splitting at ±0.2 with negative = gas [M 0x18add8 :119-135]). Nothing here wires
/// the driving levels — a level calls [`PcDrivingInput::tick`] each 60 Hz step [S].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KeyboardMouse {
    pub throttle_up: bool,       // W → △ pressure 255 → GAS 0x16
    pub throttle_down: bool,     // S → ✕ pressure 255 → BRAKE 0x1a (reverse is the engine's
                                 //     ≈30-tick brake-at-standstill state [M 0x18cef8 :184-208])
    pub steer_left: bool,        // A / ← → LX 0 → STEER 1, value −1.0
    pub steer_right: bool,       // D / → → LX 255 → STEER 1, value +1.0
    pub handbrake: bool,         // Space → ✕ bit 0x0040 → HANDBRAKE 0x1b / RELEASE 0x1c
    pub fire_primary: bool,      // left mouse → R1 bit 0x0008 → FIREPRIMARY 0x1d / 0x1e
    pub fire_secondary: bool,    // B → R2 bit 0x0002 → FIRESECONDARY 0x1f / 0x20
    pub gadget: bool,            // right mouse → ○ bit 0x0020 → FIREGADGET 0x24 / 0x25
    pub gadget_alt: bool,        // E → L1 bit 0x0004 → FIREGADGET 0x24 / 0x25
    pub cycle_weapon: bool,      // Q → Select bit 0x0100 → TOGGLESECONDARY 0x21 (edge)
    pub cycle_weapon_up: bool,   // wheel up → L3 bit 0x0200 → TOGGLESECONDARYUP 0x22 (edge)
    pub cycle_weapon_down: bool, // wheel down → R3 bit 0x0400 → TOGGLESECONDARYDOWN 0x23 (edge)
    pub camera_view: bool,       // C → Start bit 0x0800 → CHANGECAMERA 0x26 (edge)
    pub look_back: bool,         // Tab / middle mouse (hold) → □ bit 0x0080 → CAMLOOKBACK 0x29 / 0x2a
    pub reload: bool,            // R → d-pad Up bit 0x1000 → POVRELOAD 0x3f (edge)
}

/// The pad buttons/pressures the mapping synthesizes [M the bit layout and the pressure order
/// of PollDevice — buttons bit i → value 16+i; pressures Right,Left,Up,Down,△,○,✕,□,L1,R1,L2,R2
/// → values 4..15].
pub mod pc_key_pad {
    pub const DPAD_UP_BIT: u16 = 0x1000;
    pub const R2_BIT: u16 = 0x0002;
    pub const L1_BIT: u16 = 0x0004;
    pub const R1_BIT: u16 = 0x0008;
    pub const CIRCLE_BIT: u16 = 0x0020;
    pub const CROSS_BIT: u16 = 0x0040;
    pub const SQUARE_BIT: u16 = 0x0080;
    pub const SELECT_BIT: u16 = 0x0100;
    pub const LSTICK_BIT: u16 = 0x0200;
    pub const RSTICK_BIT: u16 = 0x0400;
    pub const START_BIT: u16 = 0x0800;
    /// Pressure slot indexes into `RawPadSlot.pressures` [M the libpad order].
    pub const PRESS_TRIANGLE: usize = 4;
    pub const PRESS_CROSS: usize = 6;
    /// Analog-button message indexes (value::ABUTTON_FIRST + slot − 4 offset already applied
    /// here: pressures[4] = △ → message value 8) [M the 4..15 order].
    pub const VAL_TRIANGLE: usize = 8;
    pub const VAL_CROSS: usize = 10;
    /// DButton message indexes (16 + bit position) [M the 16..31 order].
    pub const VAL_R2: usize = 17;
    pub const VAL_L1: usize = 18;
    pub const VAL_R1: usize = 19;
    pub const VAL_CIRCLE: usize = 21;
    pub const VAL_DBN_CROSS: usize = 22;
    pub const VAL_SQUARE: usize = 23;
    pub const VAL_SELECT: usize = 24;
    pub const VAL_LSTICK: usize = 25;
    pub const VAL_RSTICK: usize = 26;
    pub const VAL_START: usize = 27;
    pub const VAL_DPAD_UP: usize = 28;
}

impl KeyboardMouse {
    /// Digital extremes: pressed axis bytes 0/255, released 127 (centre — reads 0.0 after the
    /// input-side ±0.3 chop [M 0x167c48]); buttons are bits; pedals are full pressures [S].
    pub fn to_raw_pad(&self) -> RawPadSlot {
        let lx = match (self.steer_left, self.steer_right) {
            (true, false) => 0u8,
            (false, true) => 255u8,
            _ => 127u8,
        };
        let mut buttons = 0u16;
        if self.handbrake { buttons |= pc_key_pad::CROSS_BIT; }
        if self.fire_primary { buttons |= pc_key_pad::R1_BIT; }
        if self.fire_secondary { buttons |= pc_key_pad::R2_BIT; }
        if self.gadget { buttons |= pc_key_pad::CIRCLE_BIT; }
        if self.gadget_alt { buttons |= pc_key_pad::L1_BIT; }
        if self.cycle_weapon { buttons |= pc_key_pad::SELECT_BIT; }
        if self.cycle_weapon_up { buttons |= pc_key_pad::LSTICK_BIT; }
        if self.cycle_weapon_down { buttons |= pc_key_pad::RSTICK_BIT; }
        if self.camera_view { buttons |= pc_key_pad::START_BIT; }
        if self.look_back { buttons |= pc_key_pad::SQUARE_BIT; }
        if self.reload { buttons |= pc_key_pad::DPAD_UP_BIT; }
        let mut pressures = [0u8; 12];
        if self.throttle_up { pressures[pc_key_pad::PRESS_TRIANGLE] = 255; }
        if self.throttle_down { pressures[pc_key_pad::PRESS_CROSS] = 255; }
        RawPadSlot {
            // Analog mode: the pressure path must run (mode 4 would remap the DButton bits over
            // the AButton slots and lose the pedals) [M 0x169400 area].
            mode: 0,
            buttons,
            rx: 127,
            ry: 127,
            lx,
            ly: 127,
            pressures,
        }
    }
}

/// `FUN_002104a8` 0x2104a8 — the vehicle layer's move-toward, bit-exact [M the decompile]:
/// step toward `target` from `cur`, clamping at the target; the else-branch's re-add is the
/// decompile's own shape and is float-identical.
pub fn move_toward(cur: f32, target: f32, step: f32) -> f32 {
    if target < cur {
        let r = cur - step;
        if r < target { target } else { r }
    } else {
        let r = cur + step;
        if target < r { target } else { r }
    }
}

/// `DAT_00331988` — PAD_DEAD_ZONE, the **vehicle-layer** steer dead zone [M .data word 0.2;
/// the shipped `Physics\Physical\default.tun` key `PAD_DEAD_ZONE=0.2`; note it is narrower than
/// the input-side ±0.3 chop and applies again on top of it].
pub const VEHICLE_DEAD_ZONE: f32 = 0.2;
/// The steer ramp step — a code immediate `0x3e800000` at the ramp call [M 0x18add8 tail;
/// no tuning key exists, M-negative]. Digital full lock takes 4 ticks (66.7 ms at 60 Hz).
pub const STEER_RAMP_STEP: f32 = 0.25;

/// The vehicle-layer input stage — the car's GAMEACTION drain `FUN_0018add8` 0x18add8, input
/// block only (cases 1, 0x16, 0x17, 0x1a, 0x1b/0x1c, 0x1d–0x21) plus the per-tick tail
/// [M 0x18b1f0–0x18b318]. Field comments carry the car offsets. The wheelspin state machine
/// (+0x29b/+0x299) and the handbrake direction bit +0x29f (a velocity · camera-forward dot
/// computed at the drain's head) are level context, not input, and stay out.
#[derive(Clone, Debug, PartialEq)]
pub struct CarInputStage {
    /// +0x2c4 — the last STEER message value (raw, pre-chop; the chop writes back here).
    pub raw_steer: f32,
    /// +0x278 — the last GAS value; every tick `min(2·v, 1.0)` [M :179-181].
    pub gas: f32,
    /// +0x280 — GASBRAKE's gas side (negative axis, negated) [M :124-127].
    pub gasbrake_gas: f32,
    /// +0x284 — GASBRAKE's brake side (positive axis) [M :128-131].
    pub gasbrake_brake: f32,
    /// +0x27c — the last BRAKE value (raw, no gain) [M :137-142].
    pub brake: f32,
    /// +0x259 — the handbrake latch (0x1b sets, 0x1c clears) [M :144-152].
    pub handbrake: bool,
    /// +0x25a — the primary-fire flag (car side) [M :153-158].
    pub fire_primary: bool,
    /// +0x25b — the secondary-fire flag (car side) [M :159-173].
    pub fire_secondary: bool,
    /// The HUD singleton gate `FUN_0023c618()->+0x2a0 != 0` blocks the 0x1d/0x1f **set** (the
    /// release always clears) [M :153-158]; a level supplies it. Default false.
    pub hud_blocks_fire: bool,
    /// The car descriptor's `+0xbc != 0` lets a latched handbrake keep throttle/brake [M :231-238].
    /// Default false (the game's default: handbrake zeroes both pedals).
    pub handbrake_keeps_drive: bool,
    /// +0x438 — the script/AI steer override target; 0 = follow the raw steer [M :245-254; its
    /// only writer is the input-clear FUN_001872d8, so shipped flow never overrides].
    pub steer_override: f32,
    // --- outputs (the tail's writes) ---
    /// +0x240 — the ramped steer the engine consumes (`FUN_0018cef8` :458-474).
    pub steer: f32,
    /// +0x260 — last tick's ramped steer (written pre-ramp each tick) [M :250-251].
    pub steer_prev: f32,
    /// +0x250 — throttle after the totals and the exclusion.
    pub throttle: f32,
    /// +0x254 — brake after the totals and the exclusion.
    pub brake_pedal: f32,
    /// +0x2bc — throttle total `max(gas, gasbrake_gas)` [M :182-184].
    pub throttle_total: f32,
    /// +0x2c0 — brake total `max(brake, gasbrake_brake)` [M :185-188].
    pub brake_total: f32,
    /// +0x2d4 — full-lock flag, `|raw_steer| > 0.9` strictly, computed pre-chop [M :189-194].
    pub full_lock: bool,
}

impl Default for CarInputStage {
    fn default() -> Self {
        CarInputStage {
            raw_steer: 0.0,
            gas: 0.0,
            gasbrake_gas: 0.0,
            gasbrake_brake: 0.0,
            brake: 0.0,
            handbrake: false,
            fire_primary: false,
            fire_secondary: false,
            hud_blocks_fire: false,
            handbrake_keeps_drive: false,
            steer_override: 0.0,
            steer: 0.0,
            steer_prev: 0.0,
            throttle: 0.0,
            brake_pedal: 0.0,
            throttle_total: 0.0,
            brake_total: 0.0,
            full_lock: false,
        }
    }
}

impl CarInputStage {
    /// The drain's message switch, input cases only [M 0x18add8 :74-176]. Messages for actions
    /// this stage does not model (camera, weapon, HUD) are ignored here — the levels' other
    /// queues consume those.
    pub fn action(&mut self, m: &ActionMsg) {
        match m.action {
            action::STEER => self.raw_steer = m.value,
            action::GAS => self.gas = m.value,
            action::GASBRAKE => {
                // Single-axis split at ±VEHICLE_DEAD_ZONE; negative = gas [M :119-135].
                if m.value < -VEHICLE_DEAD_ZONE {
                    self.gasbrake_brake = 0.0;
                    self.gasbrake_gas = -m.value;
                } else if VEHICLE_DEAD_ZONE < m.value {
                    self.gasbrake_gas = 0.0;
                    self.gasbrake_brake = m.value;
                } else {
                    self.gasbrake_gas = 0.0;
                    self.gasbrake_brake = 0.0;
                }
            }
            action::BRAKE => self.brake = m.value,
            action::HANDBRAKE => self.handbrake = true,
            action::HANDBRAKERELEASE => self.handbrake = false,
            action::FIREPRIMARY => {
                if !self.hud_blocks_fire {
                    self.fire_primary = true;
                }
            }
            action::FIREPRIMARYRELEASE => self.fire_primary = false,
            action::FIRESECONDARY => {
                if !self.hud_blocks_fire {
                    self.fire_secondary = true;
                }
            }
            action::FIRESECONDARYRELEASE => self.fire_secondary = false,
            _ => {}
        }
    }

    /// The drain's per-tick tail [M 0x18add8 :179-270] — run once per 60 Hz step after the
    /// messages. Order is the game's: gas clamp, totals, full lock, exclusion, handbrake
    /// zeroing, the steer chop + override + ramp (the vertical twin +0x244 has no physics
    /// consumer and is not modelled).
    pub fn tick(&mut self) {
        // gas ×2 gain + clamp [M :179-181]: `+0x278 = (1.0 < 2v) ? 1.0 : 2v`.
        let doubled = self.gas + self.gas;
        self.gas = if 1.0 < doubled { 1.0 } else { doubled };
        // Totals [M :182-188].
        self.throttle_total = if self.gas < self.gasbrake_gas { self.gasbrake_gas } else { self.gas };
        self.brake_total = if self.brake < self.gasbrake_brake { self.gasbrake_brake } else { self.brake };
        // Full lock, strict, pre-chop [M :189-194].
        self.full_lock = 0.9 < self.raw_steer.abs();
        // Pedals + the exclusion: the smaller is zeroed, ties zero the gas [M :223-230:
        // `if (+0x2c0 < +0x2bc) brake=0 else gas=0`].
        self.throttle = self.throttle_total;
        self.brake_pedal = self.brake_total;
        if self.brake_total < self.throttle_total {
            self.brake_pedal = 0.0;
        } else {
            self.throttle = 0.0;
        }
        // Handbrake zeroes both pedals unless the descriptor allows the drive [M :231-238].
        if self.handbrake && !self.handbrake_keeps_drive {
            self.brake_pedal = 0.0;
            self.throttle = 0.0;
        }
        // Steer: chop (write-back), override, then the 0.25 ramp [M :239-256].
        if -VEHICLE_DEAD_ZONE < self.raw_steer && self.raw_steer < VEHICLE_DEAD_ZONE {
            self.raw_steer = 0.0;
        }
        let target = if self.steer_override != 0.0 { self.steer_override } else { self.raw_steer };
        self.steer_prev = self.steer;
        self.steer = move_toward(self.steer, target, STEER_RAMP_STEP);
    }
}

/// The PC driving binding table [S — the design of docs/systems/driving-pc-input.md §3]: one
/// `ValueBindings` per message value, the game's own evaluator runs it unchanged. Holds are
/// Always (the car keeps the last value) or Pressed/Released pairs; the one-shots are Pressed
/// edges; nothing needs auto-repeat [M the consumers in §2 of the doc].
pub fn pc_driving_bindings() -> Vec<ValueBindings> {
    use pc_key_pad as kp;
    let mut v: Vec<ValueBindings> = (0..value::COUNT)
        .map(|_| ValueBindings { kind: Kind::DButton, bindings: Vec::new(), repeat: RepeatState::default() })
        .collect();
    let mut set = |i: usize, kind: Kind, bs: Vec<Binding>| {
        v[i].kind = kind;
        v[i].bindings = bs;
    };
    let edge = |press: u8, release: u8| vec![
        Binding { kind: BindingKind::Pressed, arg: 0.0, action: press },
        Binding { kind: BindingKind::Released, arg: 0.0, action: release },
    ];
    set(value::ALX, Kind::Axis, vec![Binding { kind: BindingKind::Always, arg: 0.0, action: action::STEER }]);
    set(kp::VAL_TRIANGLE, Kind::AButton, vec![Binding { kind: BindingKind::Always, arg: 0.0, action: action::GAS }]);
    set(kp::VAL_CROSS, Kind::AButton, vec![Binding { kind: BindingKind::Always, arg: 0.0, action: action::BRAKE }]);
    set(kp::VAL_R2, Kind::DButton, edge(action::FIRESECONDARY, action::FIRESECONDARYRELEASE));
    set(kp::VAL_L1, Kind::DButton, edge(action::FIREGADGET, action::FIREGADGETRELEASE));
    set(kp::VAL_R1, Kind::DButton, edge(action::FIREPRIMARY, action::FIREPRIMARYRELEASE));
    set(kp::VAL_CIRCLE, Kind::DButton, edge(action::FIREGADGET, action::FIREGADGETRELEASE));
    set(kp::VAL_DBN_CROSS, Kind::DButton, edge(action::HANDBRAKE, action::HANDBRAKERELEASE));
    set(kp::VAL_SQUARE, Kind::DButton, edge(action::CAMLOOKBACK, action::CAMLOOKBACKRELEASE));
    set(kp::VAL_SELECT, Kind::DButton, vec![Binding { kind: BindingKind::Pressed, arg: 0.0, action: action::TOGGLESECONDARY }]);
    set(kp::VAL_LSTICK, Kind::DButton, vec![Binding { kind: BindingKind::Pressed, arg: 0.0, action: action::TOGGLESECONDARYUP }]);
    set(kp::VAL_RSTICK, Kind::DButton, vec![Binding { kind: BindingKind::Pressed, arg: 0.0, action: action::TOGGLESECONDARYDOWN }]);
    set(kp::VAL_START, Kind::DButton, vec![Binding { kind: BindingKind::Pressed, arg: 0.0, action: action::CHANGECAMERA }]);
    set(kp::VAL_DPAD_UP, Kind::DButton, vec![Binding { kind: BindingKind::Pressed, arg: 0.0, action: action::POVRELOAD }]);
    v
}

/// One 60 Hz step of the whole PC driving chain: keyboard/mouse → raw pad slot →
/// `poll_device` → the game's binding evaluator → the action messages (returned) → the car's
/// vehicle-layer input stage. This is the flow a driving level runs per tick where the engine
/// polls (`FUN_00168280`) and drains (`FUN_0018add8`) [S — the levels stay unwired].
pub struct PcDrivingInput {
    /// The keyboard/mouse state for this tick (the caller fills it from its device events).
    pub km: KeyboardMouse,
    /// The car-side stage; after `tick`, its getters are the level's car inputs.
    pub stage: CarInputStage,
    bindings: Vec<ValueBindings>,
    prev: [f32; value::COUNT],
    cur: [f32; value::COUNT],
}

impl Default for PcDrivingInput {
    fn default() -> Self {
        PcDrivingInput {
            km: KeyboardMouse::default(),
            stage: CarInputStage::default(),
            bindings: pc_driving_bindings(),
            prev: [0.0; value::COUNT],
            cur: [0.0; value::COUNT],
        }
    }
}

impl PcDrivingInput {
    /// Poll, evaluate, deliver. `now` is the frame counter (container+0x28 [M 0x168060]; the PC
    /// table binds no auto-repeat, so it only pins the repeat stamps). Returns this tick's
    /// `{action, device, value}` messages — the same list the car's queue would hold.
    pub fn tick(&mut self, now: i32) -> Vec<ActionMsg> {
        self.cur = poll_device(&self.km.to_raw_pad());
        let delays = RepeatDelays { first: 0, rest: 0 };
        let mut out = Vec::new();
        for (i, vb) in self.bindings.iter_mut().enumerate() {
            let (cur, prev) = (self.cur[i], self.prev[i]);
            let changed = cur != prev;
            evaluate_value(vb, cur, prev, changed, false, now, &delays, false, 0, &mut out);
        }
        self.prev = self.cur;
        for m in &out {
            self.stage.action(m);
        }
        self.stage.tick();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(action: u8) -> Binding {
        Binding { kind: BindingKind::Always, arg: 0.0, action }
    }

    #[test]
    fn stick_constant_is_the_disassembly_literal() {
        // [M 0x169218: lui 0x3c00, ori 0x8081]. The literal is exactly f32(2/255): 2/255 =
        // 2^-7 × (1 + 32896.502/2^23) rounds the mantissa up to 32897 = 0x008081. The sister
        // pins that the decimal-adjacent neighbours are NOT this pattern.
        assert_eq!(f32::to_bits(2.0f32 / 255.0f32), 0x3c00_8081);
        assert_ne!(f32::to_bits(1.0f32 / 127.0f32), 0x3c00_8081);
        assert_eq!(f32::from_bits(0x3c00_8081), 2.0f32 / 255.0f32);
    }

    #[test]
    fn stick_golden_table() {
        // b·C − 1 with C = 0x3c008081; golden values computed in f32 exactly as the function.
        let c = f32::from_bits(0x3c00_8081);
        for b in [0u8, 1, 64, 88, 127, 128, 166, 200, 254, 255] {
            let expect = (b as f32) * c - 1.0f32;
            assert_eq!(stick_to_f32(b), expect, "b={b}");
        }
        // Endpoints + centre sisters.
        assert_eq!(stick_to_f32(0), -1.0);
        assert_eq!(stick_to_f32(255), 1.0);
        assert!(stick_to_f32(127) < 0.0 && stick_to_f32(128) > 0.0);
    }

    #[test]
    fn pressure_golden_table_and_cap() {
        for b in [0u8, 1, 96, 191, 192, 193, 255] {
            let expect = (b as f32) * f32::from_bits(0x3baa_aaab);
            let expect = if 1.0f32 <= expect { 1.0 } else { expect };
            assert_eq!(pressure_to_f32(b), expect, "b={b}");
        }
        // Sister: 191 just under, 192 exactly 1.0, 255 clamps to 1.0 (never 1.328).
        assert!(pressure_to_f32(191) < 1.0);
        assert_eq!(pressure_to_f32(192), 1.0);
        assert_eq!(pressure_to_f32(255), 1.0);
    }

    #[test]
    fn chop_is_strict_and_no_rescale() {
        // [M 0x167c48: lo < v && v < hi → 0] — the sisters pin both edges and the missing rescale.
        assert_eq!(chop(-0.3, DEAD_ZONE_LO, DEAD_ZONE_HI), -0.3); // lo itself survives
        assert_eq!(chop(0.3, DEAD_ZONE_LO, DEAD_ZONE_HI), 0.3); // hi itself survives
        assert_eq!(chop(-0.300_000_02, DEAD_ZONE_LO, DEAD_ZONE_HI), -0.300_000_02);
        assert_eq!(chop(0.300_000_02, DEAD_ZONE_LO, DEAD_ZONE_HI), 0.300_000_02);
        assert_eq!(chop(0.29, DEAD_ZONE_LO, DEAD_ZONE_HI), 0.0);
        assert_eq!(chop(-0.29, DEAD_ZONE_LO, DEAD_ZONE_HI), 0.0);
        // Full deflection survives at 1.0 (no 127/87-style re-stretch exists).
        assert_eq!(chop(1.0, DEAD_ZONE_LO, DEAD_ZONE_HI), 1.0);
        assert_eq!(chop(-1.0, DEAD_ZONE_LO, DEAD_ZONE_HI), -1.0);
    }

    #[test]
    fn poll_zero_slot_gives_the_disconnected_shape() {
        // All sticks zero → axes 0; buttons/pressures still build [M 0x1692c8 guard].
        let pad = RawPadSlot { buttons: 0xffff, pressures: [255; 12], ..Default::default() };
        let v = poll_device(&pad);
        assert!(v[0..4].iter().all(|&x| x == 0.0));
        assert!(v[value::ABUTTON_FIRST..=value::ABUTTON_LAST].iter().all(|&x| x == 1.0));
        assert!(v[value::DBUTTON_FIRST..=value::DBUTTON_LAST].iter().all(|&x| x == 1.0));
        // Sister: the same pad with centre sticks axes full message.
        let v2 = poll_device(&RawPadSlot { buttons: 0xffff, pressures: [255; 12], rx: 127, ry: 127, lx: 127, ly: 127, mode: 0 });
        assert!(v2[0..4].iter().all(|&x| chop(x, DEAD_ZONE_LO, DEAD_ZONE_HI) == 0.0));
    }

    #[test]
    fn poll_axis_signs_and_dead_zone() {
        // LX right (255) → +1; LY up (0) → +1 (the negation); centre → chopped 0.
        let pad = RawPadSlot { lx: 255, ly: 0, rx: 255, ry: 0, ..Default::default() };
        let v = poll_device(&pad);
        assert_eq!(v[value::ALX], 1.0);
        assert_eq!(v[value::ALY], 1.0);
        assert_eq!(v[value::ARX], 1.0);
        assert_eq!(v[value::ARY], 1.0);
        // Just past the chop edge stays un-rescaled [M no DeadZoneScale in the path].
        let pad = RawPadSlot { lx: 166, ..Default::default() };
        let v = poll_device(&pad);
        let raw = stick_to_f32(166);
        assert!(raw > DEAD_ZONE_HI);
        assert_eq!(v[value::ALX], raw);
        // Inside the zone zeroes.
        let pad = RawPadSlot { lx: 150, ..Default::default() };
        assert_eq!(poll_device(&pad)[value::ALX], 0.0);
    }

    #[test]
    fn poll_digital_bits_and_remap() {
        // Bit layout sister: L2 = bit 0 → DButton index 16 [M the name order of 0x168b08].
        let pad = RawPadSlot { buttons: 0x0001, mode: 4, ..Default::default() };
        let v = poll_device(&pad);
        assert_eq!(v[16], 1.0);
        assert_eq!(v[17], 0.0);
        // Digital mode copies DButton floats over the AButton slots: AButtonRight(4) ← v[29].
        let pad = RawPadSlot { buttons: 1 << 13, mode: 4, ..Default::default() }; // Right
        let v = poll_device(&pad);
        assert_eq!(v[4], 1.0, "AButtonRight ← DButtonRight in digital mode");
        assert_eq!(v[29], 1.0);
        // Sister: analog mode ignores the bits for the AButton slots.
        let v = poll_device(&RawPadSlot { buttons: 1 << 13, mode: 0, ..Default::default() });
        assert_eq!(v[4], 0.0);
        // The exact remap order [M 0x169400 area]: {29,31,28,30,20,21,22,23,18,19,16,17}.
        let all = RawPadSlot { buttons: 0xffff, mode: 4, ..Default::default() };
        let v = poll_device(&all);
        for (dst, src) in [29usize, 31, 28, 30, 20, 21, 22, 23, 18, 19, 16, 17].iter().enumerate() {
            assert_eq!(v[4 + dst], v[*src], "slot {dst}");
        }
    }

    #[test]
    fn edge_helpers_match_the_decompile() {
        use Kind::{AButton, Axis, DButton};
        // Pressed [M 0x1678f8].
        assert!(pressed_edge(DButton, 1.0, 0.0));
        assert!(!pressed_edge(DButton, 1.0, 1.0)); // sister: held, not an edge
        assert!(pressed_edge(AButton, 0.5, 0.4999));
        assert!(!pressed_edge(AButton, 0.4999, 0.0));
        assert!(pressed_edge(Axis, 0.6, 0.4));
        // Released [M 0x1679c8] — the axis falls at −0.5, the sister pins the asymmetry.
        assert!(released_edge(DButton, 0.0, 1.0));
        assert!(released_edge(AButton, 0.4, 0.6));
        assert!(released_edge(Axis, -0.6, -0.4));
        assert!(!released_edge(Axis, 0.0, 0.6));
        // Crossings [M 0x167a98/0x167ad8].
        assert!(cross_pos(0.5, 0.5, 0.4999));
        assert!(!cross_pos(0.5, 0.5, 0.5));
        assert!(cross_neg(0.5, 0.4999, 0.5));
        assert!(!cross_neg(0.5, 0.5, 0.4));
        // Centred [M 0x167b18 with 0.2]: inclusive cur, exclusive prev.
        assert!(centred(0.2, 0.2, 0.8));
        assert!(centred(0.2, -0.2, -0.8));
        assert!(!centred(0.2, 0.2, 0.1)); // sister: prev already centred
        assert!(!centred(0.2, 0.200_000_02, 0.8));
        // Static up/down [M 0x1677f8/0x167878].
        assert!(is_down(Axis, 0.5) && !is_down(Axis, 0.4999));
        assert!(is_up(Axis, -0.500_000_1) && !is_up(Axis, -0.5));
    }

    #[test]
    fn evaluate_change_gate_and_repeat() {
        let mk = |kinds: &[BindingKind]| ValueBindings {
            kind: Kind::DButton,
            bindings: kinds
                .iter()
                .map(|&k| Binding { kind: k, arg: 0.0, action: action::HANDBRAKE })
                .collect(),
            repeat: RepeatState::default(),
        };
        let delays = RepeatDelays { first: 10, rest: 3 };
        let mut out;

        // Unchanged value: only the repeat types run [M the gate].
        let mut vb = mk(&[BindingKind::Always, BindingKind::PressRepeat]);
        out = Vec::new();
        evaluate_value(&mut vb, 1.0, 1.0, false, false, 0, &delays, false, 0, &mut out);
        assert!(out.is_empty(), "unchanged gates Always");

        // Pressed edge fires the repeat and the edge together on the changed frame.
        out = Vec::new();
        evaluate_value(&mut vb, 1.0, 0.0, true, false, 0, &delays, false, 0, &mut out);
        assert_eq!(out.len(), 2, "Always + PressRepeat on the edge");
        assert_eq!(vb.repeat.state, 1);
        // Held past the first delay: state 1 → first delay.
        out = Vec::new();
        evaluate_value(&mut vb, 1.0, 1.0, false, false, 10, &delays, false, 0, &mut out);
        assert!(out.is_empty(), "10 is not > last(0)+10");
        out = Vec::new();
        evaluate_value(&mut vb, 1.0, 1.0, false, false, 11, &delays, false, 0, &mut out);
        assert_eq!(out.len(), 1, "repeat after the first delay");
        assert_eq!(vb.repeat.state, 2);
        // State 2 → rest delay (sister: the second delay applies). The game's comparison is
        // strict `last + delay < now` [M 0x168678], so now == last+delay does not fire.
        out = Vec::new();
        evaluate_value(&mut vb, 1.0, 1.0, false, false, 13, &delays, false, 0, &mut out);
        assert!(out.is_empty());
        out = Vec::new();
        evaluate_value(&mut vb, 1.0, 1.0, false, false, 14, &delays, false, 0, &mut out);
        assert!(out.is_empty(), "14 is not > 11+3 (strict compare)");
        out = Vec::new();
        evaluate_value(&mut vb, 1.0, 1.0, false, false, 15, &delays, false, 0, &mut out);
        assert_eq!(out.len(), 1);
        // Disconnected (state 3) never repeats [M the != 3 test].
        vb.repeat.state = 3;
        out = Vec::new();
        evaluate_value(&mut vb, 1.0, 1.0, false, false, 1000, &delays, false, 0, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn evaluate_global_and_send_range() {
        let mut vb = ValueBindings {
            kind: Kind::Axis,
            bindings: vec![
                Binding { kind: BindingKind::GlobalSet, arg: 0.0, action: action::STEERUP },
                Binding { kind: BindingKind::GlobalClear, arg: 0.0, action: action::STEERDOWN },
            ],
            repeat: RepeatState::default(),
        };
        let delays = RepeatDelays { first: 1, rest: 1 };
        let mut out = Vec::new();
        evaluate_value(&mut vb, 0.9, 0.0, true, false, 0, &delays, true, 7, &mut out);
        assert_eq!(out, vec![ActionMsg { action: action::STEERUP, device: 7, value: 0.9 }]);
        let mut out = Vec::new();
        evaluate_value(&mut vb, 0.9, 0.0, true, false, 0, &delays, false, 7, &mut out);
        assert_eq!(out, vec![ActionMsg { action: action::STEERDOWN, device: 7, value: 0.9 }]);
        // SendGameMessage drops ids outside 1..0x40 [M 0x165850]; 0x40 itself passes.
        let mut vb = ValueBindings {
            kind: Kind::DButton,
            bindings: vec![msg(0), msg(0x41), msg(action::RESUMEAUTODRIVE)],
            repeat: RepeatState::default(),
        };
        let mut out = Vec::new();
        evaluate_value(&mut vb, 1.0, 0.0, true, false, 0, &delays, false, 0, &mut out);
        assert_eq!(out, vec![ActionMsg { action: action::RESUMEAUTODRIVE, device: 0, value: 1.0 }]);
    }
    #[test]
    fn keyboard_mouse_maps_onto_the_pad_conventions() {
        // [S bindings, docs/systems/driving-pc-input.md §3] — the sisters pin the game-side sign
        // conventions: LX 255 = ALX +1 (right), the pedals are the △/✕ pressures (independent
        // values, so GAS never sees BRAKE's sign), the buttons are the DButton bits.
        let km = KeyboardMouse { throttle_up: true, steer_right: true, handbrake: true, ..Default::default() };
        let v = poll_device(&km.to_raw_pad());
        assert_eq!(v[value::ALX], 1.0, "D = steer right = ALX +1");
        assert_eq!(v[value::ALY], 0.0, "W/S no longer ride LY — the pedals are pressures");
        assert_eq!(v[pc_key_pad::VAL_TRIANGLE], 1.0, "W = △ pressure 255 → 1.0");
        assert_eq!(v[pc_key_pad::VAL_CROSS], 0.0, "S released → ✕ pressure 0");
        assert_eq!(v[pc_key_pad::VAL_CROSS], v[10]);
        assert_eq!(v[16 + 6], 1.0, "Space = ✕ bit held (DButton index 22)");
        let km = KeyboardMouse { throttle_down: true, steer_left: true, gadget_alt: true, ..Default::default() };
        let v = poll_device(&km.to_raw_pad());
        assert_eq!(v[value::ALX], -1.0, "A = steer left = ALX −1");
        assert_eq!(v[pc_key_pad::VAL_CROSS], 1.0, "S = ✕ pressure 255");
        assert_eq!(v[pc_key_pad::VAL_TRIANGLE], 0.0, "W released");
        assert_eq!(v[pc_key_pad::VAL_L1], 1.0, "E = L1 bit (DButton 18)");
        // Released keys sit at byte 127 → inside the chop → 0.
        let v = poll_device(&KeyboardMouse::default().to_raw_pad());
        assert_eq!(v[value::ALX], 0.0);
        assert_eq!(v[pc_key_pad::VAL_TRIANGLE], 0.0);
        assert_eq!(v[pc_key_pad::VAL_CROSS], 0.0);
        // Analog mode (0) so the pressure path runs [M 0x169400: mode 4 would remap the bits
        // over the pedal values].
        assert_eq!(KeyboardMouse::default().to_raw_pad().mode, 0);
    }

    #[test]
    fn move_toward_matches_the_decompile() {
        // [M 0x2104a8] — step up clamped at the target, step down clamped, exact comparisons.
        assert_eq!(move_toward(0.0, 1.0, 0.25), 0.25);
        assert_eq!(move_toward(0.9, 1.0, 0.25), 1.0, "0.9+0.25 > 1.0 clamps");
        assert_eq!(move_toward(1.0, 0.0, 0.25), 0.75);
        assert_eq!(move_toward(-0.9, -1.0, 0.25), -1.0);
        assert_eq!(move_toward(0.5, 0.5, 0.25), 0.5, "already at target");
        // Sister: a step that would pass the target lands ON the target (never overshoots).
        assert_eq!(move_toward(0.2, 0.3, 0.25), 0.3);
    }

    #[test]
    fn steer_ramp_golden_60hz() {
        // [M 0x18add8 tail: chop ±0.2 write-back, then move_toward step 0.25] — a digital
        // full-lock steer takes exactly 4 ticks (66.7 ms at the fixed 60 Hz).
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::STEER, device: 0, value: 1.0 });
        s.tick();
        assert_eq!(s.steer, 0.25);
        s.tick();
        assert_eq!(s.steer, 0.5);
        s.tick();
        assert_eq!(s.steer, 0.75);
        assert_ne!(s.steer, 1.0, "sister: tick 3 is not full lock yet");
        s.tick();
        assert_eq!(s.steer, 1.0, "tick 4 = full lock");
        assert_eq!(s.steer_prev, 0.75, "steer_prev (+0x260) is last tick's value");
        // Held full lock stays.
        s.tick();
        assert_eq!(s.steer, 1.0);
        // Release: the STEER 0.0 message, then 4 ticks back.
        s.action(&ActionMsg { action: action::STEER, device: 0, value: 0.0 });
        for expected in [0.75f32, 0.5, 0.25, 0.0] {
            s.tick();
            assert_eq!(s.steer, expected);
        }
        // Full-left mirror.
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::STEER, device: 0, value: -1.0 });
        for _ in 0..3 {
            s.tick();
        }
        assert_eq!(s.steer, -0.75);
        s.tick();
        assert_eq!(s.steer, -1.0);
        // Full-left → full-right sweep = 8 ticks: 4 back through centre, 4 out to +1.
        s.action(&ActionMsg { action: action::STEER, device: 0, value: 1.0 });
        for k in 1..=8u32 {
            s.tick();
            let expect = match k {
                1..=3 => -1.0 + 0.25 * k as f32,
                4 => 0.0,
                _ => 0.25 * (k - 4) as f32,
            };
            assert_eq!(s.steer, expect, "sweep tick {k}");
        }
    }

    #[test]
    fn vehicle_dead_zone_writeback_and_full_lock_edge() {
        // [M :189-194 full lock strict on the RAW value pre-chop; :241-244 the chop writes 0
        // back into +0x2c4 when inside ±0.2, no rescale].
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::STEER, device: 0, value: 0.15 });
        s.tick();
        assert_eq!(s.raw_steer, 0.0, "inside ±0.2 → written back to 0");
        assert_eq!(s.steer, 0.0);
        // The chop edge itself survives (strict comparisons).
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::STEER, device: 0, value: 0.2 });
        s.tick();
        assert_eq!(s.raw_steer, 0.2);
        assert_eq!(s.steer, 0.2, "one tick to 0.2 (within one step)");
        // Full lock: 0.9 is NOT > 0.9; anything past is.
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::STEER, device: 0, value: 0.9 });
        s.tick();
        assert!(!s.full_lock, "0.9 is not full lock (strict >)");
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::STEER, device: 0, value: -0.900_000_1 });
        s.tick();
        assert!(s.full_lock, "just past −0.9 is");
        // The override (+0x438) wins when non-zero [M :245-254].
        let mut s = CarInputStage::default();
        s.steer_override = 0.5;
        s.action(&ActionMsg { action: action::STEER, device: 0, value: -1.0 });
        s.tick();
        assert_eq!(s.steer, 0.25, "ramps toward the override, not the raw steer");
    }

    #[test]
    fn gas_doubles_then_clamps() {
        // [M :179-181: `+0x278 = min(2v, 1.0)` each tick] — a held analog 0.3 compounds to
        // 1.0; the PC's digital 1.0 is full from the first tick.
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 0.3 });
        s.tick();
        assert_eq!(s.gas, 0.6);
        s.tick();
        assert_eq!(s.gas, 1.0, "0.6 → 1.2 clamps to 1.0");
        s.tick();
        assert_eq!(s.gas, 1.0, "held stays clamped");
        assert_eq!(s.throttle, 1.0, "throttle total (+0x2bc) = max(gas, gasbrake_gas)");
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 1.0 });
        s.tick();
        assert_eq!(s.gas, 1.0);
        assert_eq!(s.throttle, 1.0);
        // Release: value 0.0 → min(0,1) = 0.
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 0.0 });
        s.tick();
        assert_eq!(s.gas, 0.0);
        assert_eq!(s.throttle, 0.0);
        // GASBRAKE's split: negative axis = gas side, positive = brake side [M :119-135].
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::GASBRAKE, device: 0, value: -1.0 });
        s.tick();
        assert_eq!(s.throttle_total, 1.0, "−1 → gas side +1");
        assert_eq!(s.brake_total, 0.0);
        s.action(&ActionMsg { action: action::GASBRAKE, device: 0, value: 0.5 });
        s.tick();
        assert_eq!(s.throttle_total, 0.0, "the old gas side cleared");
        assert_eq!(s.brake_total, 0.5);
    }

    #[test]
    fn gas_brake_exclusion_and_handbrake() {
        // [M :223-230: the smaller of the gas/brake totals is zeroed, ties zero the gas;
        // :231-238 the handbrake zeroes both unless the descriptor keeps the drive].
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 1.0 });
        s.action(&ActionMsg { action: action::BRAKE, device: 0, value: 1.0 });
        s.tick();
        assert_eq!(s.throttle_total, 1.0);
        assert_eq!(s.brake_total, 1.0);
        assert_eq!(s.throttle, 0.0, "tie zeroes the gas");
        assert_eq!(s.brake_pedal, 1.0);
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 1.0 });
        s.action(&ActionMsg { action: action::BRAKE, device: 0, value: 0.5 });
        s.tick();
        assert_eq!(s.brake_pedal, 0.0, "smaller brake zeroed");
        assert_eq!(s.throttle, 1.0);
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 0.4 });
        s.action(&ActionMsg { action: action::BRAKE, device: 0, value: 1.0 });
        s.tick();
        assert_eq!(s.throttle, 0.0, "smaller gas zeroed");
        assert_eq!(s.brake_pedal, 1.0);
        // Handbrake: both pedals zero.
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 1.0 });
        s.action(&ActionMsg { action: action::HANDBRAKE, device: 0, value: 1.0 });
        s.tick();
        assert!(s.handbrake);
        assert_eq!(s.throttle, 0.0);
        assert_eq!(s.brake_pedal, 0.0);
        // ...unless the descriptor keeps the drive [M :231-235].
        let mut s = CarInputStage::default();
        s.handbrake_keeps_drive = true;
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 1.0 });
        s.action(&ActionMsg { action: action::HANDBRAKE, device: 0, value: 1.0 });
        s.tick();
        assert_eq!(s.throttle, 1.0);
        // Release restores the pedals.
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::GAS, device: 0, value: 1.0 });
        s.action(&ActionMsg { action: action::HANDBRAKE, device: 0, value: 1.0 });
        s.tick();
        s.action(&ActionMsg { action: action::HANDBRAKERELEASE, device: 0, value: 0.0 });
        s.tick();
        assert!(!s.handbrake);
        assert_eq!(s.throttle, 1.0);
    }

    #[test]
    fn fire_flags_and_the_hud_gate() {
        // [M :153-173: 0x1d/0x1f set +0x25a/+0x25b gated by the HUD singleton word, the
        // releases always clear].
        let mut s = CarInputStage::default();
        s.action(&ActionMsg { action: action::FIREPRIMARY, device: 0, value: 1.0 });
        assert!(s.fire_primary);
        s.action(&ActionMsg { action: action::FIREPRIMARYRELEASE, device: 0, value: 0.0 });
        assert!(!s.fire_primary);
        s.hud_blocks_fire = true;
        s.action(&ActionMsg { action: action::FIREPRIMARY, device: 0, value: 1.0 });
        assert!(!s.fire_primary, "the HUD gate blocks the set");
        s.fire_primary = true;
        s.action(&ActionMsg { action: action::FIREPRIMARYRELEASE, device: 0, value: 0.0 });
        assert!(!s.fire_primary, "the release clears through the gate");
        s.action(&ActionMsg { action: action::FIRESECONDARY, device: 0, value: 1.0 });
        assert!(!s.fire_secondary, "same gate on the secondary");
    }

    /// One tick with exactly one key state; returns the messages and the stage.
    fn one_key(km: KeyboardMouse, now: i32) -> (Vec<ActionMsg>, CarInputStage) {
        let mut pc = PcDrivingInput::default();
        pc.km = km;
        let msgs = pc.tick(now);
        (msgs, pc.stage)
    }

    #[test]
    fn every_key_reaches_its_driving_action() {
        // docs/systems/driving-pc-input.md §3 — each key's first tick produces exactly its
        // action (the press edge or the axis value), with the value PollDevice normalized.
        use action::*;
        let cases: [(KeyboardMouse, u8, f32); 15] = [
            (KeyboardMouse { steer_left: true, ..Default::default() }, STEER, -1.0),
            (KeyboardMouse { steer_right: true, ..Default::default() }, STEER, 1.0),
            (KeyboardMouse { throttle_up: true, ..Default::default() }, GAS, 1.0),
            (KeyboardMouse { throttle_down: true, ..Default::default() }, BRAKE, 1.0),
            (KeyboardMouse { handbrake: true, ..Default::default() }, HANDBRAKE, 1.0),
            (KeyboardMouse { fire_primary: true, ..Default::default() }, FIREPRIMARY, 1.0),
            (KeyboardMouse { fire_secondary: true, ..Default::default() }, FIRESECONDARY, 1.0),
            (KeyboardMouse { gadget: true, ..Default::default() }, FIREGADGET, 1.0),
            (KeyboardMouse { gadget_alt: true, ..Default::default() }, FIREGADGET, 1.0),
            (KeyboardMouse { cycle_weapon: true, ..Default::default() }, TOGGLESECONDARY, 1.0),
            (KeyboardMouse { cycle_weapon_up: true, ..Default::default() }, TOGGLESECONDARYUP, 1.0),
            (KeyboardMouse { cycle_weapon_down: true, ..Default::default() }, TOGGLESECONDARYDOWN, 1.0),
            (KeyboardMouse { camera_view: true, ..Default::default() }, CHANGECAMERA, 1.0),
            (KeyboardMouse { look_back: true, ..Default::default() }, CAMLOOKBACK, 1.0),
            (KeyboardMouse { reload: true, ..Default::default() }, POVRELOAD, 1.0),
        ];
        for (km, act, val) in cases {
            let (msgs, _s) = one_key(km, 0);
            assert_eq!(msgs.len(), 1, "one action per key: {act}");
            assert_eq!(msgs[0].action, act, "key → action");
            assert_eq!(msgs[0].value, val, "key → value");
        }
        // Sister: no key, no message.
        let (msgs, _) = one_key(KeyboardMouse::default(), 0);
        assert!(msgs.is_empty());
    }

    #[test]
    fn hold_keys_do_not_repeat_and_releases_fire_the_release_action() {
        // The game's semantics [M §2 of the doc]: the hold pairs set/clear flags (no
        // auto-repeat exists on any driving binding), the value-holds send on change only.
        use action::*;
        let mut pc = PcDrivingInput::default();
        pc.km = KeyboardMouse { fire_primary: true, handbrake: true, throttle_up: true, ..Default::default() };
        let first = pc.tick(0);
        assert_eq!(first.len(), 3, "FIREPRIMARY + HANDBRAKE + GAS on the press tick");
        for now in 1..10i32 {
            let msgs = pc.tick(now);
            assert!(msgs.is_empty(), "held keys never repeat (tick {now})");
        }
        // The stage held: gas stays 1.0 through the ×2 clamp, flags stay set.
        assert_eq!(pc.stage.gas, 1.0);
        assert!(pc.stage.fire_primary && pc.stage.handbrake);
        assert_eq!(pc.stage.throttle, 0.0, "the handbrake zeroes the pedals [M :231-238]");
        // Release everything: exactly the RELEASE ids + the value drops.
        pc.km = KeyboardMouse::default();
        let last = pc.tick(10);
        let mut acts: Vec<u8> = last.iter().map(|m| m.action).collect();
        acts.sort_unstable();
        assert_eq!(acts, vec![GAS, HANDBRAKERELEASE, FIREPRIMARYRELEASE], "sorted ids 0x16, 0x1c, 0x1e");
        assert_eq!(pc.stage.fire_primary, false);
        assert_eq!(pc.stage.gas, 0.0);
        // The edge one-shots fire once per press too (Q = TOGGLESECONDARY).
        let mut pc = PcDrivingInput::default();
        pc.km = KeyboardMouse { cycle_weapon: true, ..Default::default() };
        assert_eq!(pc.tick(0).len(), 1);
        for now in 1..5i32 {
            assert!(pc.tick(now).is_empty());
        }
        pc.km = KeyboardMouse::default();
        assert!(pc.tick(5).is_empty(), "the release of a press-only binding sends nothing");
    }

    #[test]
    fn full_chain_keyboard_to_car_60hz() {
        // W + D held through 5 ticks, then released: the game's ramp numbers end to end.
        let mut pc = PcDrivingInput::default();
        pc.km = KeyboardMouse { throttle_up: true, steer_right: true, ..Default::default() };
        for (tick, expect_steer) in [(0, 0.25), (1, 0.5), (2, 0.75), (3, 1.0)] {
            let msgs = pc.tick(tick);
            if tick == 0 {
                assert_eq!(msgs.len(), 2, "STEER + GAS on the first tick");
            } else {
                assert!(msgs.is_empty(), "held: no new messages");
            }
            assert_eq!(pc.stage.steer, expect_steer, "tick {tick}");
            assert_eq!(pc.stage.throttle, 1.0);
        }
        // Full lock reached: the engine's steer input +0x240 is 1.0.
        assert_eq!(pc.stage.steer, 1.0);
        assert!(pc.stage.full_lock);
        // Release both: the ramp walks back 4 ticks, gas drops to 0 immediately.
        pc.km = KeyboardMouse::default();
        let msgs = pc.tick(4);
        assert_eq!(msgs.len(), 2, "STEER 0.0 + GAS 0.0");
        assert_eq!(pc.stage.steer, 0.75);
        assert_eq!(pc.stage.throttle, 0.0);
        for (tick, expect) in [(5, 0.5), (6, 0.25), (7, 0.0)] {
            pc.tick(tick);
            assert_eq!(pc.stage.steer, expect, "release tick {tick}");
        }
        assert!(!pc.stage.full_lock);
    }
}
