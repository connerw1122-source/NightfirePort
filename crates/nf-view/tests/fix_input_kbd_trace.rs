//! fix-input: the keyboard/mouse regression trace. A fixed sequence of `Input`s runs through the real
//! load path on 07000005; Bond's state is compared against golden values recorded from the
//! pre-fix-input tree (the gamepad backend must not move the keyboard path by a bit).
//! Opt-in via `NF_ISO`.

use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The scripted trace: (tick, Input) — the keyboard path's shapes: full-deflection axes, mouse-turn
/// radians, edge presses (jump/crouch), aim mode, fire.
fn input_at(t: u32) -> Input {
    let mut i = Input::default();
    if (20..60).contains(&t) { i.forward = 1.0; }
    if (60..120).contains(&t) { i.forward = 1.0; i.strafe = 1.0; }
    if (120..180).contains(&t) { i.forward = 0.6; i.aiming = true; }
    if (30..50).contains(&t) { i.turn = 0.02; }
    if (90..130).contains(&t) { i.turn = -0.03; }
    if (130..180).contains(&t) { i.turn = 0.01; i.look = -0.004; }
    if t == 45 { i.jump = true; }
    if t == 80 { i.crouch = true; }
    if (100..110).contains(&t) { i.fire = true; }
    if t == 150 { i.jump = true; }
    i
}

#[test]
fn keyboard_trace_unchanged() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().expect("sim");
    sim.bond = Bond::stand_at(&sim.world, [sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]], 0.6);
    for t in 0..180u32 {
        sim.step_bond(input_at(t), (100..110).contains(&t), false, false);
        if t % 30 == 29 || t == 179 {
            let b = &sim.bond;
            // The goldens were recorded from the pre-fix-input tree (the gamepad backend must not
            // move the keyboard/mouse path by a bit). Bit-exact on every recorded field.
            let (pos, yaw, pitch, local, vel, cc) = match t {
                29 => ([105.3352f32, 24.977415, 109.382454], 0.6f32, 0.0f32, [0.0f32, 0.09372571], [0.0f32, 0.0, 0.0], 1.0347183f32),
                59 => ([107.22346, 25.675138, 111.11418], 0.99999964, 0.0, [0.0, 0.1], [0.0, 1.9599992, 0.0], 1.0347183),
                89 => ([110.6332, 26.184004, 111.018906], 0.99999964, 0.0, [0.074907154, 0.1], [0.0, -1.4700007, 0.0], 1.0347183),
                119 => ([113.98246, 24.18317, 112.40501], 0.10000008, 0.0, [0.07499989, 0.1], [0.0, -6.3700027, 0.0], 1.0347183),
                149 => ([114.27505, 19.732336, 112.73504], 0.027819457, 0.0, [0.0, 0.0], [0.0, -11.269996, 0.0], 1.0347183),
                179 => ([114.27505, 16.816837, 112.73504], 0.072180696, -0.053007107, [0.0, 0.0], [0.0, 0.0, 0.0], 1.0347183),
                _ => unreachable!(),
            };
            for k in 0..3 { assert_eq!(b.pos[k].to_bits(), pos[k].to_bits(), "t={t} pos[{k}] = {} (want {})", b.pos[k], pos[k]); }
            assert_eq!(b.yaw.to_bits(), yaw.to_bits(), "t={t} yaw = {} (want {yaw})", b.yaw);
            assert_eq!(b.pitch.to_bits(), pitch.to_bits(), "t={t} pitch");
            for k in 0..2 { assert_eq!(b.local[k].to_bits(), local[k].to_bits(), "t={t} local[{k}]"); }
            for k in 0..3 { assert_eq!(b.vel[k].to_bits(), vel[k].to_bits(), "t={t} vel[{k}]"); }
            assert_eq!(b.cc.to_bits(), cc.to_bits(), "t={t} cc");
        }
    }
}
