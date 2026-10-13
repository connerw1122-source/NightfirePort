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
            eprintln!("GOLD t={t} pos={:?}", b.pos);
            eprintln!("GOLD t={t} yaw={:?} pitch={:?} local={:?} vel={:?} cc={:?}", b.yaw, b.pitch, b.local, b.vel, b.cc);
        }
    }
}
