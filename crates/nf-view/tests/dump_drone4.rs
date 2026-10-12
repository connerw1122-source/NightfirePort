//! fix-drone4 dump: the C2-class "drawn but never attacks / can't be damaged" drones on 07000002 and
//! 07000006, driven through `nf_view::load` + `Sim::step_bond`. Bond is stood at the reported spots,
//! then every drawn drone near him is dumped with present/started/enabled/in-llist/held/drawn/team/
//! mode/dtypenew/channel, and the closest drawn one is SHOT to test hittability.
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn dump(sim: &nf_view::scene::Sim, tag: &str) {
    eprintln!("--- {tag} @ tick {} bond ({:.1},{:.1},{:.1}) ---", sim.drones.frame, sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]);
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if !dr.present || !dr.drawn { continue }
        let d = [(dr.pos[0] - sim.bond.pos[0]), (dr.pos[1] - sim.bond.pos[1]), (dr.pos[2] - sim.bond.pos[2])];
        let dist = (d[0] * d[0] + d[2] * d[2]).sqrt();
        if dist > 25.0 { continue }
        eprintln!("[d{i:02}] skin {:09x} dist {dist:.1} team {} mode {} dtypenew {} state {:#x} ch {} chon {} hp {:.0} present {} started {} enabled {} llist {} held {} drawn {}",
            dr.skin, dr.team, dr.mode, dr.dtype, dr.state, dr.switch_channel, dr.channel_on as u8, dr.health,
            dr.present, dr.started, dr.enabled, sim.drones.llist.contains(&i) as u8, dr.anim.held as u8, dr.drawn);
    }
}

fn shoot_closest_drawn(sim: &mut nf_view::scene::Sim, ticks: u32) {
    let target = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.drawn && dr.health > 0.0)
        .min_by(|a, b| {
            let da = (a.1.pos[0] - sim.bond.pos[0]).powi(2) + (a.1.pos[2] - sim.bond.pos[2]).powi(2);
            let db = (b.1.pos[0] - sim.bond.pos[0]).powi(2) + (b.1.pos[2] - sim.bond.pos[2]).powi(2);
            da.partial_cmp(&db).unwrap()
        });
    let Some((ti, tp)) = target else { eprintln!("no drawn living drone to shoot"); return };
    eprintln!("[shoot] target d{ti:02} skin {:09x} at {:?}", sim.drones.list[ti].skin, tp);
    let hp0 = sim.drones.list[ti].health;
    for _ in 0..ticks {
        let tp = sim.drones.list[ti].pos;
        let e = sim.bond.head();
        let d = [tp[0] - e[0], tp[1] - e[1], tp[2] - e[2]];
        sim.bond.yaw = d[0].atan2(d[2]);
        sim.bond.pitch = (d[1] / (d[0] * d[0] + d[2] * d[2]).sqrt().max(1e-4)).atan() / (std::f32::consts::FRAC_PI_2);
        sim.gun.weapon = 2; // the P2K
        sim.step_bond(Input::default(), true, false, false);
        if sim.drones.list[ti].health < hp0 { break; }
    }
    let dr = &sim.drones.list[ti];
    eprintln!("[shoot] d{ti:02} hp {:.1} -> {:.1} state {:#x} — {}", hp0, dr.health, dr.state,
        if dr.health < hp0 { "DAMAGED" } else { "UNTOUCHED" });
}

#[test]
fn dump_07000002_bathhouse_c2() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    dump(sim, "start");
    for _ in 0..2 { sim.step_bond(Default::default(), false, false, false); }
    // The bathhouse civilian woman [D placement] at (−24.33, 1.08, 31.92): stand Bond beside her.
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [-20.0, 1.0, 34.0], 0.0);
    sim.gun.weapon = 2;
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    dump(sim, "bathhouse pre-channel");
    // Bond walked here in play, stepping on the TOUCH_ONCE plates 341/383 that raise ch 21.
    let f = sim.drones.frame;
    sim.drones.channels.set(21, 1, f);
    for _ in 0..180 { sim.step_bond(Default::default(), false, false, false); }
    dump(sim, "bathhouse ch21 raised");
    shoot_closest_drawn(sim, 240);
    dump(sim, "after shoot");
}

#[test]
fn dump_07000006_stationary_guards_c2() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..2 { sim.step_bond(Default::default(), false, false, false); }
    dump(sim, "start");
    // Walk Bond forward 900 ticks (real inputs) toward the gatehouse.
    let mut input = Input::default();
    input.forward = 1.0;
    for _ in 0..900 { sim.step_bond(input, false, false, false); }
    dump(sim, "after 900 walk");
    shoot_closest_drawn(sim, 240);
    dump(sim, "after shoot");
}
