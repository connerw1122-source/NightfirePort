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
    pub const FIREGADGET: u8 = 0x24;
    pub const CHANGECAMERA: u8 = 0x26;
    pub const CAMLOOKBACK: u8 = 0x29;
    pub const RESUMEAUTODRIVE: u8 = 0x40;
    /// The SendGameMessage bound [M 0x165850: `0 < id < 0x41`].
    pub const MAX: u32 = 0x40;
}

/// The port's keyboard+mouse → raw pad bytes mapping [S — the shipped bindings live in
/// data\Control\FrontEnd.def, not extractable here; see docs/systems/driving-input.md §11].
///
/// Sign conventions come from PollDevice itself: LY byte 0 (up) → ALY +1 (gas), LY 255 → −1
/// (brake); LX 255 (right) → ALX +1 [M 0x169218 + the negations at 0x169288]. Nothing here
/// wires the driving levels — a driving level would call `poll_device` on its slot each tick
/// where the engine runs `FUN_00168280` [S].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KeyboardMouse {
    pub throttle_up: bool,    // W → LY 0
    pub throttle_down: bool,  // S → LY 255
    pub steer_left: bool,     // A → LX 0
    pub steer_right: bool,    // D → LX 255
    pub handbrake: bool,      // Space → Cross bit (0x0040) [S]
    pub fire_primary: bool,   // mouse left → △ (0x0010) [S]
    pub fire_secondary: bool, // mouse right → ○ (0x0020) [S]
    pub cam_up: bool,         // ↑ → d-pad Up bit (0x1000) [S]
    pub cam_down: bool,       // ↓ → d-pad Down bit (0x4000) [S]
    pub cam_left: bool,       // ← → d-pad Left bit (0x8000) [S]
    pub cam_right: bool,      // → → d-pad Right bit (0x2000) [S]
    pub look_back: bool,      // R → Square (0x0080) [S]
}

impl KeyboardMouse {
    /// Full-deflection digital bytes: pressed = 0/255, released = 127 (centre) [S — any value
    /// past the ±0.3 chop edge (byte ≥ 166 / ≤ 88) reads the same after PollDevice; 0/255 is
    /// the honest digital extreme].
    pub fn to_raw_pad(&self) -> RawPadSlot {
        let ly = match (self.throttle_up, self.throttle_down) {
            (true, false) => 0u8,
            (false, true) => 255u8,
            _ => 127u8,
        };
        let lx = match (self.steer_left, self.steer_right) {
            (true, false) => 0u8,
            (false, true) => 255u8,
            _ => 127u8,
        };
        let mut buttons = 0u16;
        if self.handbrake { buttons |= 0x0040; }
        if self.fire_primary { buttons |= 0x0010; }
        if self.fire_secondary { buttons |= 0x0020; }
        if self.cam_up { buttons |= 0x1000; }
        if self.cam_down { buttons |= 0x4000; }
        if self.cam_left { buttons |= 0x8000; }
        if self.cam_right { buttons |= 0x2000; }
        if self.look_back { buttons |= 0x0080; }
        RawPadSlot {
            mode: 0,
            buttons,
            rx: 127,
            ry: 127,
            lx,
            ly,
            pressures: [0; 12],
        }
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
        // [S bindings] — the sisters pin the sign conventions that come from the game: gas is
        // LY=0 (the negation at 0x169288 makes up = +1), steer right is LX=255.
        let km = KeyboardMouse {
            throttle_up: true,
            steer_right: true,
            handbrake: true,
            ..Default::default()
        };
        let v = poll_device(&km.to_raw_pad());
        assert_eq!(v[value::ALY], 1.0, "W = gas = ALY +1");
        assert_eq!(v[value::ALX], 1.0, "D = steer right = ALX +1");
        assert_eq!(v[16 + 6], 1.0, "Space = Cross (bit 6) held");
        let km = KeyboardMouse { throttle_down: true, steer_left: true, ..Default::default() };
        let v = poll_device(&km.to_raw_pad());
        assert_eq!(v[value::ALY], -1.0, "S = brake = ALY −1");
        assert_eq!(v[value::ALX], -1.0, "A = steer left = ALX −1");
        // Released keys sit at byte 127 → inside the chop → 0.
        let v = poll_device(&KeyboardMouse::default().to_raw_pad());
        assert_eq!(v[value::ALX], 0.0);
        assert_eq!(v[value::ALY], 0.0);
    }
}
