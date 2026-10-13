//! probe: the truck script SP[0] on 07000005 — does it start and move the truck?
use nf_game::player::{Bond, Input};
fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }
#[test]
fn probe_truck_sp() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond = Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    }
    for t in 0..2300 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if t % 400 == 0 || t == 2299 {
            let sim = scene.sim.as_ref().unwrap();
            let p = &sim.drones.scripts.players[0];
            let playing = p.info[p.cur].as_ref().map(|s| (s.playing(), s.clock)).unwrap_or((false, 0.0));
            eprintln!("t{t}: disabled {} SP0 state {:#x} cur {} info-clock/playing {:?} ch 0x5c {} bond {:?}",
                sim.disabled, p.state, p.cur, playing, sim.drones.channels.on(0x5c) as u8, sim.bond.pos);
        }
    }
}
