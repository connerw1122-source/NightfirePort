//! probe: the 04 target's damageable/kind — why does the hit deal no damage?
#[path = "common/walker.rs"]
mod walker;
use nf_game::player::Input;
fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }
#[test]
fn probe_target_flags() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000004.bin", false).expect("load");
    for _ in 0..3000 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let ti = sim.drones.list.iter().position(|d| d.skin == 0x0500_001e).expect("target");
    let d = &sim.drones.list[ti];
    eprintln!("target: damageable {} kind {} health {:.1} max {:.1} hits_taken {} state {:#x} team {}",
        d.damageable, d.kind, d.health, d.max_health, d.hits_taken, d.state, d.team);
    let data = nf_game::drone::Data {
        tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
        world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons,
        nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes,
        sfx: sim.sfx.as_ref(), models: &sim.models, surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(),
    };
    eprintln!("prop 0x2B (damageable?): {}", d.prop(&data, 0x2b as usize));
    for p in [0x00u32, 0x01, 0x02, 0x11, 0x12, 0x13] { eprintln!("prop {p:#x}: {}", d.prop(&data, p as usize)); }
}
