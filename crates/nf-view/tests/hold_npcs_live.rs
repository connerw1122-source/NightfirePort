//! `--hold-npcs` (dev, the port's own): held guards stay where they stand but Bond's bullets still
//! hit them — so a render check can aim at a guard and get the same frame every run.
//! Opt-in: set `NF_ISO` to the disc image.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn shoot(hold: bool) -> (f32, f32, [f32; 3], [f32; 3], usize) {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let iso = iso().unwrap();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 05");
    let sim = scene.sim.as_mut().unwrap();
    sim.drones.hold_npcs = hold;
    // The render spot: guard #6 in the trench, the crosshair on his chest.
    sim.bond = nf_game::player::Bond::new([96.9, 17.9, 98.8], 0.635);
    sim.bond.pitch = -0.22;
    let p0 = sim.drones.list[6].pos;
    for _ in 0..58 { sim.step_bond(Default::default(), false, false, false); }
    let hp0 = sim.drones.list[6].health;
    sim.step_bond(nf_game::player::Input { fire: true, ..Default::default() }, true, false, false);
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    let puffs = sim.drones.gas.puffs.iter().filter(|p| p.mesh == 0x0200_08bc).count();
    (hp0, sim.drones.list[6].health, p0, sim.drones.list[6].pos, puffs)
}

#[test]
fn held_guards_stay_put_and_still_take_hits() {
    if iso().is_none() { return; }
    let (hp0, hp1, p0, p1, puffs) = shoot(true);
    let moved = ((p1[0] - p0[0]).powi(2) + (p1[2] - p0[2]).powi(2)).sqrt();
    assert!(moved < 1e-4, "a held guard moved {moved:.3} units");
    assert!(hp1 < hp0, "the shot at a held guard did no damage ({hp0} -> {hp1})");
    assert_eq!(puffs, 1, "the hit spawns the blood puff");
}

/// Sister: the same 240 ticks with Bond standing at the spot — unheld, the guard leaves his spot;
/// held, he doesn't.
#[test]
fn unheld_guards_move() {
    if iso().is_none() { return; }
    let walk = |hold: bool| {
        let mut scene = nf_view::load(&iso().unwrap(), "07000005.bin", false).expect("load 05");
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.hold_npcs = hold;
        sim.bond = nf_game::player::Bond::new([96.9, 17.9, 98.8], 0.635);
        let p0 = sim.drones.list[6].pos;
        for _ in 0..240 { sim.step_bond(Default::default(), false, false, false); }
        let p1 = sim.drones.list[6].pos;
        ((p1[0] - p0[0]).powi(2) + (p1[2] - p0[2]).powi(2)).sqrt()
    };
    let (free, held) = (walk(false), walk(true));
    assert!(free > 0.05, "the unheld guard should move, moved {free:.3}");
    assert!(held < 1e-4, "the held guard moved {held:.3}");
}
