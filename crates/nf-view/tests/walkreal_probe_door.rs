//! walkreal-a probe 15: guard 15 (skin 0x5000036) at (−23.3, 6.3, 14.4) — why do the bullets miss
//! at 2.7? Dump his hit boxes vs the aim ray. Diagnostic.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn dxz(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

#[test]
fn probe_guard15_boxes() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..4000 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0xb) { break; } }
    for _ in 0..6000 { sim.step_bond(Input::default(), false, false, false); }
    let gi = sim.drones.list.iter().position(|d| d.skin == 0x0500_0036 && d.pos[0] < -20.0).map(|v| v as i32).unwrap_or(-1) as usize;
    if gi == 0 || gi >= sim.drones.list.len() { eprintln!("no such guard"); return; }
    let d = &sim.drones.list[gi];
    eprintln!("guard {gi}: skin {:#x} pos {:?} health {} started {} enabled {}", d.skin, d.pos, d.health, d.started, d.enabled);
    let boxes = sim.drones.list[gi].hit_boxes(&nf_game::drone::Data {
        tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
        world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons,
        nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes,
        sfx: sim.sfx.as_ref(), models: &sim.models, surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(),
    });
    eprintln!("{} boxes:", boxes.len());
    for b in boxes.iter().take(20) {
        eprintln!("  part {:3}: centre ({:.2},{:.2},{:.2}) half {:?}", b.part, b.m[3][0], b.m[3][1], b.m[3][2], b.half);
    }
    eprintln!("guard crouched? anim state {:#x} danim {}", sim.drones.list[gi].anim.state, sim.drones.list[gi].anim.danim);
}
