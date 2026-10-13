//! debug: prev_weapon/prev_gadget through the gadget equip + the weapon cycle.
use nf_game::player::Input;
fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }
#[test]
fn probe_prev_slots() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [54.0, 2.4, -173.5];
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    eprintln!("start: weapon {:#x} prev_w {:#x} prev_g {:#x}", sim.gun.weapon, sim.gun.prev_weapon, sim.gun.prev_gadget);
    for k in 0..4 {
        sim.step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        eprintln!("gcycle {k}: weapon {:#x} prev_w {:#x} prev_g {:#x}", sim.gun.weapon, sim.gun.prev_weapon, sim.gun.prev_gadget);
        if sim.gun.weapon == 0x52 { break; }
    }
    for k in 0..3 {
        sim.step_bond(Input { weapon_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..70 { sim.step_bond(Input::default(), false, false, false); }
        eprintln!("wcycle {k}: weapon {:#x} prev_w {:#x} prev_g {:#x}", sim.gun.weapon, sim.gun.prev_weapon, sim.gun.prev_gadget);
    }
}
