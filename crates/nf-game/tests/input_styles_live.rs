//! fix-input G-005: the style × action table in `nf_game::input` against the game's own
//! `psiInput_MapInputs` (ACTION.ELF 0x10cd80), run in the R5900 interpreter over the user's ELF.
//! For every style 0..7 and a grid of synthetic pad states (every button bit, stick extremes, the
//! conditional-source buttons, the stick-menu thresholds), the 40 action floats must match bit-exact.
//! Sister: StickCompensation2 (0x17f798) and MapAnalogStick (0x17f958) against the port's curves.
//! Opt-in via `NF_ISO`.

use nf_formats::ee::Machine;
use nf_formats::iso9660::IsoImage;
use nf_game::input::{self, PadConfig, PadState};

const ACTION_ELF_ENTRY: u32 = 0x10cd80; // psiInput_MapInputs
const PLAYER_SETTING: u32 = 0x2a38c8;   // PlayerSetting[0] [M symtab]
const T_SLOT: u32 = 0x245680;           // tSlot[0] [M symtab; stride 0x180]
const MULTITAP: u32 = 0x30c7e4;         // MultitapPresent [M symtab]
const COUNT_114: u32 = 0x30c7e8;        // the 60-call rumble counter [M prologue: lw -32392(gp), _gp 0x314670]
const GAME_STATE: u32 = 0x2a3768;       // GameState [M symtab]
const STICKCOMP2: u32 = 0x17f798;
const MAP_ANALOG_STICK: u32 = 0x17f958;

fn machine() -> (Machine, Vec<u8>) {
    let iso = IsoImage::open(std::env::var("NF_ISO").unwrap()).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    (Machine::from_elf(&elf).unwrap(), elf)
}

/// Pokes one synthetic pad state into tSlot[0] and PlayerSetting[0].style, runs the game's
/// MapInputs for player 0, returns the 40 action floats.
fn game_map(m: &mut Machine, style: u16, pad: &PadState) -> [f32; 40] {
    m.poke(MULTITAP, &0u32.to_le_bytes()).unwrap();       // no multitap: player 0 -> slot 0
    m.poke(COUNT_114, &0x3cu32.to_le_bytes()).unwrap();   // skip the sceMtapGetConnection RPC [M 0x10cd80:16-19]
    m.poke(GAME_STATE + 0x50, &0u8.to_le_bytes()).unwrap(); // skip the second rumble loop [M 0x10cd80:36]
    m.poke(T_SLOT + 0x114, &99i32.to_le_bytes()).unwrap(); // slot 0 ready
    m.poke(T_SLOT + 0x124, &[1u8]).unwrap();              // buffer[0] != 0: skip the scePadSetActDirect branch
    m.poke(T_SLOT + 0x122, &pad.buttons.to_le_bytes()).unwrap();
    m.poke(T_SLOT + 0x128, &[pad.rx, pad.ry, pad.lx, pad.ly]).unwrap();
    m.poke(T_SLOT + 0x148, &[pad.rx, pad.ry, pad.lx, pad.ly]).unwrap(); // the same dead-zoned copies
    m.poke(PLAYER_SETTING + 0xe, &style.to_le_bytes()).unwrap();
    m.poke(PLAYER_SETTING + 0x14, &[0u8; 160]).unwrap();  // clear the 40 action floats
    m.call(ACTION_ELF_ENTRY, PLAYER_SETTING, 1, 4_000_000).unwrap();
    let mut out = [0f32; 40];
    for (i, v) in out.iter_mut().enumerate() {
        *v = f32::from_le_bytes(m.read(PLAYER_SETTING + 0x14 + 4 * i as u32, 4).unwrap().try_into().unwrap());
    }
    out
}

fn pad(buttons: u16, rx: u8, ry: u8, lx: u8, ly: u8) -> PadState {
    PadState { buttons, rx, ry, lx, ly, connected: true }
}

/// The probe grid: covers every button bit alone, stick extremes, the style-conditional buttons
/// (L1 0x4 / R1 0x8), the stick-menu thresholds (±0.5 → bytes 63/191) and action 38's triple.
fn grid() -> Vec<PadState> {
    let mut v = Vec::new();
    let bit = |i: u16| 1u16 << i;
    for i in 0..16u16 { v.push(pad(bit(i), 0x7f, 0x7f, 0x7f, 0x7f)); }
    v.push(pad(0, 0x7f, 0x7f, 0x7f, 0x7f));           // sticks centred
    v.push(pad(0, 255, 1, 200, 60));                  // full + partial deflection
    v.push(pad(0, 1, 255, 60, 200));                  // the other diagonal
    v.push(pad(0, 24, 230, 230, 24));                 // the dead-zone/threshold edges
    for mask in [0x4u16, 0x8, 0x0c] {                 // style 2's L1 source swap, style 6's ±1 strafe
        v.push(pad(mask, 255, 1, 200, 60));
        v.push(pad(mask, 0x7f, 0x7f, 0x7f, 0x7f));
    }
    v.push(pad(0x1000, 24, 0x7f, 0x7f, 0x7f));        // action 38: LX < −0.8 && RX > 0.8 && Up
    v.push(pad(0x1000, 230, 0x7f, 24, 0x7f));
    v
}

#[test]
fn style_table_matches_psiinput_mapinputs_bit_exact() {
    let Some(_) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let (mut m, elf) = machine();
    let cfg = PadConfig::from_action_elf(&elf).expect("curve symbol");
    let mut checked = 0usize;
    for style in 0..8u16 {
        for p in grid() {
            let game = game_map(&mut m, style, &p);
            let port = input::map_inputs(style, &p, &cfg);
            for (act, (g, r)) in game.iter().zip(port.iter()).enumerate() {
                assert_eq!(g.to_bits(), r.to_bits(), "style {style} pad {p:?} action {act}: game {g} port {r}");
            }
            checked += 1;
        }
    }
    assert!(checked >= 8 * 21, "grid ran {checked} states");
}

#[test]
fn sister_styles_differ_where_the_decomp_says() {
    let Some(_) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let (mut m, elf) = machine();
    let cfg = PadConfig::from_action_elf(&elf).expect("curve symbol");
    // Style 0: Cross (0x40) -> action 8 (+64.0). Style 1: action 8 is L2 (0x1) — Cross leaves it 0
    // and L2 sets it [M case 0 line 87 vs case 1 line 121].
    let cross = pad(0x40, 0x7f, 0x7f, 0x7f, 0x7f);
    assert_eq!(game_map(&mut m, 0, &cross)[8], 64.0);
    assert_eq!(game_map(&mut m, 1, &cross)[8], 0.0);
    assert_eq!(input::map_inputs(0, &cross, &cfg)[8], 64.0);
    assert_eq!(input::map_inputs(1, &cross, &cfg)[8], 0.0);
    // Style 4 drives movement from the right stick and look from the left [M case 4:213-215]:
    // action 2 (forward) reads RY (0x129) instead of LY (0x12b).
    let fwd = pad(0, 0x7f, 255, 0x7f, 0x7f);
    assert_eq!(game_map(&mut m, 4, &fwd)[2], -1.0);
    assert_eq!(game_map(&mut m, 0, &fwd)[2], 0.0);
    assert_eq!(input::map_inputs(4, &fwd, &cfg)[2], -1.0);
    assert_eq!(input::map_inputs(0, &fwd, &cfg)[2], 0.0);
    // Style 6 adds L1 (+1.0) / R1 (−1.0) onto the strafe [M case 6:291-304] — sticks centred,
    // so the strafe is exactly ±1.0.
    let l1 = pad(0x4, 0x7f, 0x7f, 0x7f, 0x7f);
    assert_eq!(game_map(&mut m, 6, &l1)[1], 1.0);
    assert_eq!(input::map_inputs(6, &l1, &cfg)[1], 1.0);
}

#[test]
fn stick_compensation_matches_the_game() {
    let Some(_) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let (mut m, elf) = machine();
    let cfg = PadConfig::from_action_elf(&elf).expect("curve symbol");
    // MapAnalogStick 0x17f958: f12 in, f0 out.
    for x in [0.0f32, 0.1, 0.35, 0.5, 0.707107, 0.9, 0.98, 1.0, -0.98, -1.0] {
        m.set_fpr(12, x);
        m.call(MAP_ANALOG_STICK, 0, 0, 100_000).unwrap();
        let game = m.fpr(0);
        // the port's MapAnalogStick through the same curve
        let port = {
            let mut i = (x * 127.0) as i32;
            let sign = if i < 0 { -1.0f32 } else { 1.0 };
            let mut a = i.abs();
            if a > 128 { a = 128; }
            sign * cfg.curve[a as usize] as f32 / 255.0
        };
        assert_eq!(game.to_bits(), port.to_bits(), "MapAnalogStick({x}): game {game} port {port}");
    }
    // StickCompensation2 0x17f798: two float pointers.
    let scratch = 0x2a3d90u32; // inside PlayerSetting[3] — unused by these tests
    for (a, b) in [(0.9f32, 0.1f32), (1.0, 1.0), (0.2, -0.2), (0.0, 0.0), (-0.98, 0.05), (0.05, -0.98)] {
        m.poke(scratch, &a.to_le_bytes()).unwrap();
        m.poke(scratch + 4, &b.to_le_bytes()).unwrap();
        m.call(STICKCOMP2, scratch, scratch + 4, 200_000).unwrap();
        let (ga, gb) = (
            f32::from_le_bytes(m.read(scratch, 4).unwrap().try_into().unwrap()),
            f32::from_le_bytes(m.read(scratch + 4, 4).unwrap().try_into().unwrap()),
        );
        // the port's radial compensation through the crate's MapAnalogStick (the game multiplies
        // by 0.003921569, not /255 — pinned by the MapAnalogStick loop above)
        let m_abs = (a * a + b * b).sqrt();
        let (pa, pb) = if m_abs > 0.0002 {
            let k = input::map_analog_stick_for_test(m_abs, &cfg.curve) / m_abs;
            (a * k, b * k)
        } else { (a, b) };
        // ±1 ulp: the game's magnitude comes out of the EE's madd/sqrt pairing, whose last bit
        // differs from Rust's `(a*a)+(b*b).sqrt()`; the curve lookup and the sign are exact.
        let ulps = |x: f32, y: f32| (x.to_bits() as i64 - y.to_bits() as i64).abs();
        assert!(ulps(ga, pa) <= 1, "Comp2 a=({a},{b}): game {ga} port {pa}");
        assert!(ulps(gb, pb) <= 1, "Comp2 b=({a},{b}): game {gb} port {pb}");
    }
}
