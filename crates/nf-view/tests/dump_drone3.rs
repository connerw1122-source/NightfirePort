//! fix-drone3 dump: every drone that is drawn but either not hittable or not running AI.
//! Runs the real viewer path (`nf_view::load`, the level start, then walking Bond there with real
//! inputs through `Sim::step_bond`). Dump fields: present / started / enabled / drawn / held /
//! team / mode / dtypenew (mode) / state / channel / skin / pos / health / corpse / alert.
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn dump(tag: &str, sim: &nf_view::scene::Sim, filter: fn(&nf_game::drone::Drone) -> bool) {
    eprintln!("--- dump {tag} @ tick {} bond {:?} ---", sim.drones.frame, sim.bond.pos);
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if !filter(dr) { continue }
        eprintln!(
            "[d{i:02}] skin {:09x} pos ({:.1},{:.1},{:.1}) yaw {:.2} aim {:.2} aclass {} team {} dtype {} mode {} dtypenew {} state {:#x} ch {} chon {} w {:04x} hp {:.0} present {} started {} enabled {} drawn {} held {} corpse {} alert {:.1}",
            dr.skin, dr.pos[0], dr.pos[1], dr.pos[2], dr.yaw, dr.aim_yaw, dr.anim.class, dr.team, dr.dtype, dr.mode_normal, dr.mode, dr.state,
            dr.switch_channel, dr.channel_on as u8, dr.weapon, dr.health, dr.present, dr.started, dr.enabled, dr.drawn,
            dr.anim.held as u8, dr.corpse, dr.alert);
    }
}

/// Walk forward with real inputs for n ticks.
fn walk(sim: &mut nf_view::scene::Sim, n: u32, trigger: bool) {
    let mut input = Input::default();
    input.forward = 1.0;
    for _ in 0..n { sim.step_bond(input, trigger, false, false); }
}

#[test]
fn dump_07000006_drone_states() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let interesting = |dr: &nf_game::drone::Drone| dr.present;
    dump("07000006 start", sim, interesting);
    // Idle 120 ticks: the level start settles.
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    dump("07000006 after 120 idle", sim, interesting);
    // Walk 600 ticks forward from the spawn with real inputs (through guards if any).
    walk(sim, 600, false);
    dump("07000006 after 600 walk", sim, interesting);
    walk(sim, 600, false);
    dump("07000006 after 1200 walk", sim, interesting);
    walk(sim, 600, false);
    dump("07000006 after 1800 walk", sim, interesting);
}

#[test]
fn dump_07000002_bathhouse_drone_states() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let interesting = |dr: &nf_game::drone::Drone| dr.present;
    dump("07000002 start", sim, interesting);
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    dump("07000002 after 120 idle", sim, interesting);
    // Walk toward the bathhouse (fix-drone2: ch 0x15 plates 341/383 by the civilian, ch 0x17 trio).
    walk(sim, 900, false);
    dump("07000002 after 900 walk", sim, interesting);
    walk(sim, 900, false);
    dump("07000002 after 1800 walk", sim, interesting);
}

#[test]
fn dump_07000004_party_drone_states() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000004.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let interesting = |dr: &nf_game::drone::Drone| dr.present;
    dump("07000004 start", sim, interesting);
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    dump("07000004 after 120 idle", sim, interesting);
    walk(sim, 600, false);
    dump("07000004 after 600 walk", sim, interesting);
}

#[test]
fn dump_07000005_patrol_drone_states() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let interesting = |dr: &nf_game::drone::Drone| dr.present;
    dump("07000005 start", sim, interesting);
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    dump("07000005 after 120 idle", sim, interesting);
    walk(sim, 900, false);
    dump("07000005 after 900 walk", sim, interesting);
}

#[test]
fn dump_07000007_party_drone_states() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let interesting = |dr: &nf_game::drone::Drone| dr.present;
    dump("07000007 start", sim, interesting);
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    dump("07000007 after 120 idle", sim, interesting);
    walk(sim, 600, false);
    dump("07000007 after 600 walk", sim, interesting);
}

#[test]
fn dump_07000006_searchlight_arm() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..2 { sim.step_bond(Default::default(), false, false, false); }
    let sl = &sim.drones.searchlights;
    for (i, p) in sl.parts.iter().enumerate() {
        eprintln!("[sl{i:02}] kind {} pos ({:.1},{:.1},{:.1}) state {} alarm {} on {} kill {} links {:?}", p.kind, p.pos[0], p.pos[1], p.pos[2], p.state, p.alarm, p.on, p.off, p.links);
    }
}
