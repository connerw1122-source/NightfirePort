//! probe: the gadget cycle under the runner's load path
use nf_game::player::Input;
#[test]
fn probe_gadget_cycle() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let ids: Vec<u8> = (1..0x72u8).filter(|&i| i != 0x47).collect();
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &ids).expect("load");
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("held={:#04x} sw={} rel={} fire={}", sim.gun.weapon, sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8);
    for g in [0x4Au8, 0x4eu8, 0x50u8, 0x54u8] {
        eprintln!("owned[{g:#04x}] = {}", sim.drones.inventory.owned.get(g as usize).copied().unwrap_or(false));
    }
    for press in 0..6 {
        scene.sim.as_mut().unwrap().step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
        for t in 0..60 {
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
            let sim = scene.sim.as_ref().unwrap();
            if t % 10 == 0 { eprintln!("  press{press} t{t}: held={:#04x} sw={} rel={}", sim.gun.weapon, sim.gun.switch_state, sim.gun.reload_state); }
        }
    }
}
