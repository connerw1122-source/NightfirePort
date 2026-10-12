//! fix-render4: the 07000002 "drawn but never attacks / can't be damaged" pass the user asked for
//! (after fix-drone2, no R pressed). Runs the real viewer path (nf_view::load + Sim::step_bond),
//! walks Bond to the bathhouse, steps him over the TOUCH_ONCE plates 341/383 (the ones play raises
//! channel 21 through), then dumps EVERY drawn drone that either takes no damage or isn't running
//! AI, with present/started/enabled/in_llist/held/drawn/team/mode/dtypenew/channel. Also prints the
//! plate placements so the walk is the real trigger path, not a channel poked by hand.
//! Opt-in via NF_ISO.

use nf_formats::{disc, iso9660::IsoImage, level::Level};
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn parse_level(iso_path: &str, archive: &str) -> Level {
    let img = IsoImage::open(iso_path).expect("open ISO");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive entry");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
    Level::parse(&data).expect("parse level")
}

fn dump_drawn_but_broken(sim: &nf_view::scene::Sim, tag: &str) {
    eprintln!("--- {tag} @ tick {} bond ({:.1},{:.1},{:.1}) ---", sim.drones.frame, sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2]);
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if !dr.present || !dr.drawn { continue }
        let d = [(dr.pos[0] - sim.bond.pos[0]), (dr.pos[1] - sim.bond.pos[1]), (dr.pos[2] - sim.bond.pos[2])];
        let dist = (d[0] * d[0] + d[2] * d[2]).sqrt();
        if dist > 30.0 { continue }
        eprintln!("[d{i:02}] skin {:09x} dist {dist:.1} team {} mode {} dtypenew {} state {:#x} ch {} chon {} hp {:.0} present {} started {} enabled {} llist {} held {} drawn {}",
            dr.skin, dr.team, dr.mode, dr.dtype, dr.state, dr.switch_channel, dr.channel_on as u8, dr.health,
            dr.present, dr.started, dr.enabled, sim.drones.llist.contains(&i) as u8, dr.anim.held as u8, dr.drawn);
    }
}

/// Shoot the closest drawn living drone for `ticks`, return true if its health dropped.
fn shoot_closest_drawn(sim: &mut nf_view::scene::Sim, ticks: u32) -> bool {
    let target = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.drawn && dr.health > 0.0)
        .min_by(|a, b| {
            let da = (a.1.pos[0] - sim.bond.pos[0]).powi(2) + (a.1.pos[2] - sim.bond.pos[2]).powi(2);
            let db = (b.1.pos[0] - sim.bond.pos[0]).powi(2) + (b.1.pos[2] - sim.bond.pos[2]).powi(2);
            da.partial_cmp(&db).unwrap()
        });
    let Some((ti, _)) = target else { eprintln!("[shoot] no drawn living drone"); return false };
    let hp0 = sim.drones.list[ti].health;
    eprintln!("[shoot] target d{ti:02} skin {:09x} mode {} team {}", sim.drones.list[ti].skin, sim.drones.list[ti].mode, sim.drones.list[ti].team);
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
    let damaged = sim.drones.list[ti].health < hp0;
    eprintln!("[shoot] d{ti:02} hp {hp0:.1} -> {:.1} state {:#x} — {}", sim.drones.list[ti].health, sim.drones.list[ti].state,
        if damaged { "DAMAGED" } else { "UNTOUCHED" });
    damaged
}

#[test]
fn bathhouse_guard_real_walk_and_dump() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let level = parse_level(&iso, "07000002.bin");
    let m = level.main_map().expect("main map");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();

    // The plates' real placements (the trigger objects play steps on).
    for pi in [341usize, 383] {
        if pi < level.maps[m].placements.len() {
            let p = &level.maps[m].placements[pi];
            eprintln!("[plate {pi}] kind {:#x} at ({:.2},{:.2},{:.2}) params {:?}", p.flags & 0xff, p.translation[0], p.translation[1], p.translation[2], p.params);
        }
    }

    dump_drawn_but_broken(sim, "start");
    // Stand Bond beside the bathhouse civilian woman [D placement] (−24.33, 1.08, 31.92), then walk
    // him over the plates: real trigger path (the trigger system runs inside step_bond).
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [-20.0, 1.0, 34.0], 0.0);
    sim.gun.weapon = 2;
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    dump_drawn_but_broken(sim, "bathhouse, before the plates");

    // Walk to plate 341, then 383 (their real translations), 400 ticks each with real forward input.
    for pi in [341usize, 383] {
        let t = level.maps[m].placements[pi].translation;
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [t[0], t[1], t[2] - 2.0], 0.0);
        let mut input = Input::default();
        input.forward = 1.0;
        for _ in 0..400 { sim.step_bond(input, false, false, false); }
        eprintln!("[walk] over plate {pi}: ch21 = {}", sim.drones.channels.on(21));
        dump_drawn_but_broken(sim, &format!("after plate {pi}"));
    }
    let _damaged = shoot_closest_drawn(sim, 240);
    dump_drawn_but_broken(sim, "after shoot");
}
