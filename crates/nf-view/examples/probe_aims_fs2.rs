//! fix-aims20-lights-render probe: the skin palette's aims entries with the AIMS-20 actually held.
//! Run: cargo run --release --example probe_aims_fs2
fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x1a]).unwrap();
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(0x1a).max(1);
        let ok = sim.drones.inventory.equip_weapon(&wd, 0x1a, rounds, true);
        sim.drones.inventory.equip_ammo(&wd, 0x1a, rounds * 3);
        sim.gun.switch_request = 0x1a;
        eprintln!("equip ok={ok}");
    }
    let input = nf_game::player::Input { strafe: 1.0, ..Default::default() };
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(input, false, false, false); }
    let s = scene.sim.as_ref().unwrap();
    eprintln!("gun.weapon={:#x} aims_idle={} clip[0x1a]={:?} frame={}", s.gun.weapon, s.drones.aims_idle, s.drones.inventory.clip.get(0x1a), s.drones.frame);
    let opts = nf_view::scene::ViewOpts::default();
    let fs = scene.frame_state(None, 0.0, opts);
    // Which characters hold the aims attachments, and are their 3x3 rotations degenerate?
    let aims: Vec<usize> = scene.attachments.iter().enumerate().filter(|(_, a)| a.aims_light.is_some()).map(|(i, _)| i).collect();
    let l = scene.skin_layout().0;
    eprintln!("attach_base={} n_att={}", l.attach_base, aims.len());
    for &i in &aims {
        let m = fs.attach[i];
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        eprintln!("attach[{i}] char={} bone={:?} det={:.5} t=({:.2},{:.2},{:.2})", scene.attachments[i].character, scene.attachments[i].aims_light, det, m[3][0], m[3][1], m[3][2]);
    }
    // Palette check.
    let (pal, cols) = scene.skin_palette(&l, None, 0.0, opts, None);
    for &i in &aims {
        let m = pal[l.attach_base as usize + i];
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        eprintln!("pal[attach+{i}] det={:.5} t=({:.2},{:.2},{:.2})", det, m[3][0], m[3][1], m[3][2]);
        eprintln!("  cols[{}]={:?}", l.col_att + i as u32, cols.get((l.col_att + i as u32) as usize));
    }
}
