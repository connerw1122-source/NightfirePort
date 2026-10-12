//! map-finale: the 0700004a walkthrough (docs/levels/0700004a.md). The Tower-2 elevator: the shaft's
//! TouchOnce plates fire their floor channels as Bond descends (teleporting between floors is fine
//! for walking), the first plate's ch 0x4d reveals "Gain access to lobby" [L record 4a], and the
//! mission stays open (ch 0x4a's completing writer is not ported — the doc's §6). Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_descent_plates_fire_and_the_objective_reveals() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700004a.bin", false).expect("load 0700004a");
    let sim = scene.sim.as_mut().unwrap();
    let p = sim.bond.pos;
    assert!(p[0] > 2.0 && p[0] < 6.0 && p[1] > -9.0 && p[1] < -6.0,
        "the arrival spawns at the shaft's top lobby (#1 [D]), got {p:?}");
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..100 { sim.step_bond(Input::default(), false, false, false); }
    // The shaft's first plate #277 (0.90-5.18, -9.87..-7.65, -5.74..-1.01, out ch 0x4d [D]).
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [3.0, -6.5, -3.4], 0.0);
    let mut input = Input::default();
    input.forward = 1.0;
    for _ in 1..=120 { sim.step_bond(input, false, false, false); if sim.drones.channels.on(0x4d) { break; } }
    for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x4d), "the shaft's first plate fires ch 0x4d [D trigger #277]");
        assert_eq!(sim.drones.mission.states[17][0], 3, "ch 0x4d reveals 'Gain access to lobby' [L record 4a]");
    }
    // Ride the shaft: the floor plates fire in descent order — the shaft descends in Y (the plates
    // stack at x 0.86-5.2, z -5.8..-1.0 [D triggers]); teleporting between floors is fine for walking.
    for (y, ch) in [(-29.5f32, 0x3du32), (-40.0, 0x17u32), (-60.0, 0x18u32), (-71.0, 0x3fu32), (-80.0, 0x19u32), (-83.5, 0x45u32), (-100.5, 0x1au32), (-103.5, 0x3eu32), (-120.5, 0x1bu32), (-124.0, 0x4fu32), (-156.0, 0x84u32)] {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [3.0, y + 2.0, -3.4], 0.0);
        let mut input = Input::default();
        input.forward = 1.0;
        for _ in 1..=60 { sim.step_bond(input, false, false, false); }
        let fired = sim.drones.channels.on(ch);
        assert!(fired, "the descent plate near y {y} fired ch {ch:#x} [D triggers]");
    }
    let sim = scene.sim.as_ref().unwrap();
    // Sister: the completing channel 0x4a has no ported writer (the doc's §6) — the mission stays open.
    assert!(!sim.drones.channels.on(0x4a), "ch 0x4a never fires without the lift's lobby arrival");
    assert!(!sim.drones.channels.on(100), "the Tower-2 mission stays open on this part");
    assert_eq!(sim.drones.level_exit, None, "no level exit on 0700004a [D — 13 touch plates only]");
}
