//! fix-aims20-lights-render probe: the BUILT scene's aims-light materials — which key did the
//! light-mesh batches get, and is its texture in scene.textures (and is it red)?
//! Run: cargo run --release --example probe_aims_scene_mat -- 07000005.bin

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let level_name = std::env::args().nth(1).unwrap_or_else(|| "07000005.bin".into());
    let scene = nf_view::load_dev(&iso, &level_name, true).unwrap_or_else(|e| panic!("load: {e}"));
    println!("scene textures: {}", scene.textures.len());
    // The aims light materials: Effect layer, and the light mesh's group — find materials whose
    // vertex count matches the mesh (8 verts → 12 indices... per bone) and print all Effect keys.
    let mut eff_keys: std::collections::BTreeMap<(Option<(usize, u32)>, u8, u32), Vec<usize>> = std::collections::BTreeMap::new();
    for (i, m) in scene.materials.iter().enumerate() {
        if m.layer == nf_view::scene::Layer::Effect {
            eff_keys.entry((m.key, m.draw.blend, m.index_count)).or_default().push(i);
        }
    }
    for (k, idxs) in &eff_keys {
        println!("  effect key={:?} blend={} indices={} mats={:?}", k.0, k.1, k.2, idxs);
    }
    // The red texture (map holding 0x02000132, tex 12): where does it live in the scene's textures?
    for (i, t) in scene.textures.iter().enumerate() {
        let (mi, ti) = t.key;
        let mut r = 0u64; let n = (t.rgba.len() / 4).max(1);
        for px in t.rgba.chunks(4) { r += px[0] as u64; }
        let gr = t.rgba.chunks(4).filter(|p| p[0] > p[1] + 30 && p[0] > p[2] + 30).count();
        if gr > n / 4 { println!("  RED texture at scene.textures[{i}] key=({mi},{ti}) size={}x{} reddish={gr}/{n}", t.width, t.height); }
    }
    // The view models: which character index is the 0x1a?
    println!("view_models: {:?}", scene.view_models);
    // The aims attachments and their bones.
    let aims: Vec<_> = scene.attachments.iter().enumerate().filter(|(_, a)| a.aims_light.is_some()).collect();
    println!("aims attachments: {} (bones {:?})", aims.len(), aims.iter().map(|(_, a)| a.aims_light.unwrap()).collect::<Vec<_>>());
    // The (10,12) texture as the scene built it:
    for (i, t) in scene.textures.iter().enumerate() {
        if t.key == (10, 12) {
            let n = (t.rgba.len() / 4).max(1);
            let mut sums = [0u64; 4];
            for px in t.rgba.chunks(4) { for k in 0..4 { sums[k] += px[k] as u64; } }
            println!("  scene.textures[{i}] key=(10,12) {}x{} avg_rgba=({},{},{},{})", t.width, t.height,
                sums[0]/n as u64, sums[1]/n as u64, sums[2]/n as u64, sums[3]/n as u64);
            let mut pal = std::collections::BTreeMap::new();
            for px in t.rgba.chunks(4) { *pal.entry([px[0],px[1],px[2],px[3]]).or_insert(0usize) += 1; }
            let mut v: Vec<_> = pal.into_iter().collect();
            v.sort_by(|a,b| b.1.cmp(&a.1));
            for (c, n) in v.iter().take(8) { println!("    colour {:?} x{}", c, n); }
        }
    }
}

// (appended by the second probe run — print the (10,12) texture's colour from the built scene)
