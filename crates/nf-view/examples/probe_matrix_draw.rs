//! Probe: why does the draw refuse for the grenade/gadget records on the merged load?
//! Prints the switch machine's state per 10 ticks after a give + switch_to for one id.
use nf_view::panel;
use nf_view::scene::Scene;

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let level = std::env::args().nth(1).unwrap_or_else(|| "07000005.bin".into());
    let wid = u8::from_str_radix(std::env::args().nth(2).unwrap_or_else(|| "34".into()).trim_start_matches("0x"), 16).expect("wid");
    let mut scene: Scene = nf_view::load(&iso, &level, false).expect("load");
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // give through the panel (the matrix's give path)
    let q = vec![("do".to_string(), "weapon".to_string()), ("id".to_string(), format!("0x{wid:02x}"))];
    let (msg, sw) = panel::apply(&mut scene, &q);
    println!("give: {msg} sw={sw:?}");
    let clip = scene.sim.as_ref().unwrap().weapons.clip_size(wid as usize);
    let fp = scene.sim.as_ref().unwrap().weapons.first_person(wid as usize);
    let owned = scene.sim.as_ref().unwrap().drones.inventory.owned.get(wid as usize).copied();
    let t = scene.sim.as_ref().unwrap().weapons.ammo_type(wid as usize);
    let carried = scene.sim.as_ref().unwrap().drones.inventory.carried.get(t as usize).copied();
    println!("clip={clip} fp={fp:08x} owned={owned:?} ammo_type={t} carried={carried:?}");
    // draw-fire the Sentinel first (the matrix's preceding cell), then try the target id
    let pre = std::env::args().nth(3).map(|x| u8::from_str_radix(x.trim_start_matches("0x"), 16).unwrap()).unwrap_or(0);
    if pre != 0 {
        for i in 0..60 {
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { switch_to: Some(pre), ..Default::default() }, false, false, false);
            if scene.sim.as_ref().unwrap().gun.weapon == pre { break; }
        }
        for _ in 0..25 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
        for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false); }
        for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
        println!("pre drawn: weapon={:02x}", scene.sim.as_ref().unwrap().gun.weapon);
    }
    for i in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { switch_to: Some(wid), ..Default::default() }, false, false, false);
        if i % 10 == 0 {
            let sim = scene.sim.as_ref().unwrap();
            println!("t{i}: weapon={:02x} req={:02x} switch_state={} reload={} fire={} clip={}", sim.gun.weapon, sim.gun.switch_request,
                sim.gun.switch_state, sim.gun.reload_state, sim.gun.fire_state as u8, sim.gun.clip);
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    println!("end: weapon={:02x} vm={}", sim.gun.weapon, scene.view_models.contains_key(&wid) as u8);
}
