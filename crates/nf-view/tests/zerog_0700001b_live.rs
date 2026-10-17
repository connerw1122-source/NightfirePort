//! zero-g-movement-1b: Bond's spacewalk substate against the real level. `Player_ZeroG` 0x192ab0
//! runs as dispatch cases 8/9 [M Player_Update 0x193600:238-241]; the level's zero gravity is the
//! p3 == 2 start [M Player_Init 0x1910f8 — pinned by fix_world_gravity_live]. These tests drive
//! the real input path (`Sim::step_bond`) on 0700001b with the substate entered the way the
//! game's script does (ChangeSubState 9 — the port's tests assign it directly, the swim-entry
//! precedent, until the C_CHCHZEROG 0x216388 msg-0x4b wire lands; see
//! docs/reports/zero-g-movement-1b.md): the thrusters move Bond along the object matrix, `vel`
//! (BL+0x50) stays zero [M 0x192ab0:71], and nothing pulls him down.
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_game::player::Input;
use nf_view::scene::Sim;

/// One step that skips any script cut-scene holding Bond (the Walker harness's move: the game's
/// own skip action while `disabled` — the arrival chain re-disables between scripts).
fn step(sim: &mut Sim, input: Input) {
    if sim.disabled { sim.drones.scripts.skip_request = true; }
    sim.step_bond(input, false, false, false);
}

/// Load 0700001b and skip the intro script chain until Bond stays enabled for 60 straight ticks.
fn loaded_sim(scene: &mut nf_view::scene::Scene) -> &mut Sim {
    let sim = scene.sim.as_mut().unwrap();
    sim.disabled = false;
    let mut enabled_run = 0u32;
    for _ in 0..2400 {
        step(sim, Input::default());
        enabled_run = if sim.disabled { 0 } else { enabled_run + 1 };
        if enabled_run >= 60 { return sim; }
    }
    panic!("the arrival script chain never ended (Bond still disabled)");
}

#[test]
fn zero_g_on_0700001b_thrusts_along_the_look_and_hangs() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("skip: NF_ISO not set"); return };
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 1b");
    let sim = loaded_sim(&mut scene);
    assert!(!sim.disabled, "Bond enabled before the spacewalk");
    // Enter the spacewalk substate the way the game's script does (ChangeSubState 9 [M
    // C_CHCHZEROG 0x216388]; the port assigns it directly — the swim-entry precedent).
    sim.bond.substate = nf_game::player::SS_ZEROG_9;
    let y0 = sim.bond.pos[1];
    // Idle: `Vec_Zero(BLData+0x50)` every tick [M 0x192ab0:71] — Bond hangs at the spawn.
    for _ in 0..30 { step(sim, Input::default()); }
    assert!(!sim.disabled, "no cut-scene re-disabled the spacewalk");
    assert_eq!(sim.bond.substate, nf_game::player::SS_ZEROG_9, "the substate holds");
    assert_eq!(sim.bond.vel, [0.0; 3], "vel stays zero in zero-g [M 0x192ab0:71]");
    assert!((sim.bond.pos[1] - y0).abs() < 1e-3, "the spawn hangs: {} vs {}", y0, sim.bond.pos[1]);
    // The vertical thruster (the jump key held = action 7) lifts him [M :136-138]; the rise
    // saturates at the ±0.05 smoother [M :110-112].
    for _ in 0..60 { step(sim, Input { lift: 1.0, ..Default::default() }); }
    assert!(sim.bond.pos[1] > y0 + 1.0, "lifted: {} → {} (sub {}, disabled {})",
        y0, sim.bond.pos[1], sim.bond.substate, sim.disabled);
    assert_eq!(sim.bond.vel, [0.0; 3], "no gravity sneaks into vel");
    // Sister: the same lift input in the WALK substate does not lift — the spawn stands, and the
    // substate never enters 8/9 on its own.
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 1b (sister)");
    let sim = loaded_sim(&mut scene);
    let y0 = sim.bond.pos[1];
    for _ in 0..60 { step(sim, Input { lift: 1.0, ..Default::default() }); }
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "walk stays walk");
    assert!(sim.bond.pos[1] <= y0 + 0.01, "walk ignores the lift: {} vs {}", y0, sim.bond.pos[1]);
}
