//! Probe: does the guided-missile camera ever release on 07000005? (the D-block from 0x2d)
use nf_view::panel;

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x2c]).expect("load");
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let q = vec![("do".to_string(), "weapon".to_string()), ("id".to_string(), "0x2c".to_string())];
    let (msg, sw) = panel::apply(&mut scene, &q);
    println!("give: {msg} sw={sw:?}");
    for _ in 0..90 { scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { switch_to: Some(0x2c), ..Default::default() }, false, false, false); }
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    println!("held={:02x} — firing", scene.sim.as_ref().unwrap().gun.weapon);
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false); }
    for t in 0..2400u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if t % 100 == 0 {
            let bullets: Vec<String> = sim.drones.player_bullets.iter().map(|b| format!("{{w={:#04x} dead={} trav={:.0} range={:.0} guided={}}}", b.weapon, b.dead, b.travelled, b.range, b.guided_cam)).collect();
            println!("t{t}: missile_cam={} riding={} restore={} disabled={} bullets=[{}]",
                sim.missile_cam.is_some(), sim.missile_cam.as_ref().map(|c| c.riding).unwrap_or(false),
                sim.missile_cam.as_ref().map(|c| c.restore).unwrap_or(0), sim.disabled, bullets.join(", "));
        }
        if t == 600 || t == 1200 || t == 1800 {
            // try a switch: does the input land?
            scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { switch_to: Some(0x02), ..Default::default() }, false, false, false);
            println!("  t{}: switch_to(02) → held={:02x} req={:02x}", t, scene.sim.as_ref().unwrap().gun.weapon, scene.sim.as_ref().unwrap().gun.switch_request);
        }
    }
}
