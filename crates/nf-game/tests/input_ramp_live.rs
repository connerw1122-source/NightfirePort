//! fix-input G-003: the turn/aim ramps against the game's own code in the R5900 interpreter.
//! `AccelFunc0` (0x1a9218 — the helper `Player_Move` drives the yaw ramp with, state BL+0x8fc)
//! runs tick by tick over synthetic stick sequences; the port's `input::AccelState` must reproduce
//! the speed bit-exact every tick, and the returned-state + clamp semantics must agree.
//! Sister: the |stick| ≤ 0.05 reset and the |stick| ≥ 0.95 target switch (two sequences that must
//! diverge). Opt-in via `NF_ISO`.

use nf_formats::ee::Machine;
use nf_formats::iso9660::IsoImage;
use nf_game::input::AccelState;

const ACC_FUNC0: u32 = 0x1a9218;
const SCRATCH: u32 = 0x2a3d90; // PlayerSetting[3]'s tail — unused by these tests [M symtab 0x2a38c8 + 3*0x158 + 0x100]

/// One call of the game's AccelFunc0 (0x1a9218) with the EE's float-arg convention: the six floats
/// in f12..f17, the state pointer in a0. Returns the new speed; the state word at SCRATCH holds it.
fn game_accel(m: &mut Machine, state: f32, stick: f32, base: f32, mul: f32, steps: f32, lo: f32, hi: f32) -> f32 {
    // The EE's float-arg convention, decoded from the prologue [M 0x1a9218: `mul.s f3,f13,f14`
    // = base·mul, `abs.s f0,f12` = |stick|, `lwc1 f1,0(a0)` = the state through a0]: the six
    // floats sit in f12..f17 in order and the pointer in a0. The game reads *a0 and returns the
    // new speed WITHOUT storing (the caller does: `*(float *)(BL+0x8fc) = fVar5` [M 0x191dd0:146]).
    m.poke(SCRATCH, &state.to_le_bytes()).unwrap();
    m.set_fpr(12, stick);
    m.set_fpr(13, base);
    m.set_fpr(14, mul);
    m.set_fpr(15, steps);
    m.set_fpr(16, lo);
    m.set_fpr(17, hi);
    m.call(ACC_FUNC0, SCRATCH, 0, 200_000).unwrap();
    m.fpr(0)
}

#[test]
fn accel_func0_turn_speeds_tick_by_tick() {
    let Some(_) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(std::env::var("NF_ISO").unwrap()).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let mut m = Machine::from_elf(&elf).unwrap();
    // The no-aim yaw values the game runs with on 07000005 (ReadTuningVars' live values
    // [M docs/systems/input.md §7]: X 0.04, Mul 2, Steps 120).
    let (base, mul, steps) = (0.04f32, 2.0f32, 120.0f32);
    let mut port = AccelState { state: base };
    let mut game = base;
    // Ramp up at full stick: the speed climbs (base·mul − base)/Steps = 0.04/120 per tick.
    // The interpreter's div.s rounds through f64 [M nf-formats/src/ee.rs 0x03: `(fs / ft) as f32`],
    // so the per-tick step can differ from the real EE's IEEE-direct divide (and from the port's
    // f32 division) by 1 ulp. The thresholds, reset, clamp and sign are bit-checked below.
    let ulps = |x: f32, y: f32| (x.to_bits() as i64 - y.to_bits() as i64).abs();
    for t in 0..200u32 {
        let g = game_accel(&mut m, game, 1.0, base, mul, steps, 0.05, 0.95);
        let p = port.step(1.0, base, mul, steps, 0.05, 0.95);
        assert!(ulps(g, p) <= 1, "tick {t}: game {g} port {p}");
        game = g;
    }
    // After `steps` ticks the state must sit at base·mul (the ramp's ceiling).
    assert_eq!(game, base * mul, "the ramp reaches base·mul and holds");
    // Sister A: |stick| ≤ 0.05 resets the state to base — drop the stick and the speed snaps back.
    let g = game_accel(&mut m, game, 0.0, base, mul, steps, 0.05, 0.95);
    let p = port.step(0.0, base, mul, steps, 0.05, 0.95);
    assert!(ulps(g, p) <= 1);
    assert_eq!(g, base, "the reset returns exactly base");
    // Sister B: |stick| < 0.95 never ramps — 40 ticks at 0.9 leave the state at base.
    let mut slow = AccelState { state: base };
    for _ in 0..40 {
        let g = game_accel(&mut m, slow.state, 0.9, base, mul, steps, 0.05, 0.95);
        let p = slow.step(0.9, base, mul, steps, 0.05, 0.95);
        assert!(ulps(g, p) <= 1);
        assert_eq!(g, base, "target stays base below hi");
    }
    // Sister C: the ramp STATE has no sign of its own — the target is base·mul either way and the
    // stick's sign rides the multiply outside (obj+0x34 = (−action0) × state [M 0x191dd0:143-148]).
    // A negative stick from `base` still climbs to +base·mul.
    let mut neg = AccelState { state: base };
    for _ in 0..130 {
        let g = game_accel(&mut m, neg.state, -1.0, base, mul, steps, 0.05, 0.95);
        let p = neg.step(-1.0, base, mul, steps, 0.05, 0.95);
        assert!(ulps(g, p) <= 1);
        assert!(g > 0.0, "the state is unsigned; game {g}");
    }
    assert_eq!(neg.state, base * mul, "the ramp target is unsigned");
}
