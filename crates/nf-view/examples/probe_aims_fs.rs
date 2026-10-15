//! fix-aims20-lights-render probe: frame_state's attach matrices for the aims lights after a strafe
//! walk — are they collapsed (the gates) or posed? Run: cargo run --release --example probe_aims_fs
use nf_view::scene::{ViewOpts, Layer};

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x1a]).unwrap();
    let sim = scene.sim.as_mut().unwrap();
    let input = nf_game::player::Input { strafe: 1.0, ..Default::default() };
    for _ in 0..60 { sim.step_bond(input, false, false, false); }
    println!("gun.weapon={:#x} aims_idle={} aims_raise={} frame={}", sim.gun.weapon, sim.drones.aims_idle, sim.drones.aims_raise, sim.drones.frame);
    println!("clip[0x1a]={:?} clip[0x1b]={:?}", sim.drones.inventory.clip.get(0x1a), sim.drones.inventory.clip.get(0x1b));
    println!("view_models: {:?}", scene.view_models);
    let opts = ViewOpts::default();
    let fs = scene.frame_state(None, 0.0, opts);
    for (i, a) in scene.attachments.iter().enumerate() {
        if a.aims_light.is_some() {
            let m = fs.attach.get(i).copied();
            let t = m.map(|m| m[3]);
            println!("attach[{i}] aims_light={:?} bone={} matrix_t={:?} collapsed={:?}",
                a.aims_light, a.bone, t, t.map(|t| (t[0], t[1], t[2])));
        }
    }
    // The aims material's draw call: is it in the draw list with a sane centre?
    let aims_mats: Vec<_> = scene.materials.iter().enumerate()
        .filter(|(_, m)| m.layer == Layer::Effect && m.key == Some((10, 12)) && m.draw.blend == 1 && m.index_count == 96)
        .collect();
    for (i, m) in aims_mats { println!("material[{i}] centre={:?} radius={:.3} pass={} blend={} first={} count={}", m.centre, m.radius, m.draw.pass, m.draw.blend, m.first_index, m.index_count); }
}
