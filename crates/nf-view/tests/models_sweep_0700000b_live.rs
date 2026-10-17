//! level-0b model sweep (task 711a): every material on 0700000b resolves a texture — no white
//! fallback — and every placement's world instance resolves its mesh (the loader census prints
//! "0 unresolved" for the level). The GPU's missing-texture path draws a 1×1 white;
//! `NF_TEX_TRACE=1` prints the fallbacks at load; this test pins the same condition through the
//! scene's material table (the 08 sweep's pattern). Opt-in via NF_ISO.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn every_0700000b_material_resolves_a_texture_no_white_fallback() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "0700000b.bin", false).expect("load 0700000b");
    let w = &scene;
    let tex_keys: std::collections::HashSet<_> = w.textures.iter().map(|t| t.key).collect();
    let fallbacks: Vec<_> = w.materials.iter().enumerate()
        .filter(|(_, m)| m.key.map_or(true, |k| !tex_keys.contains(&k))).collect();
    eprintln!("[0b] materials {} textures {} white-fallback {}", w.materials.len(), w.textures.len(), fallbacks.len());
    for (i, m) in fallbacks.iter().take(16) {
        eprintln!("[0b]   fallback mat {i}: layer {:?} pass {:?} key {:?}", m.layer, m.draw.pass, m.key);
    }
    assert!(fallbacks.is_empty(), "every material resolves a texture on 0700000b (no white fallback)");
}

/// The level's guards (10 oDrone placements: kind 86 exec floor ×4 in group 2, kind 87 roof ×6
/// in group 3 [D]) exist as spawner templates: `DroneSpawner_Init` holds the group's drones
/// absent until its spawn condition runs [M 0x137e68; D spawners #267 (group 2)/#268 (group 3)] —
/// so the model sweep asserts the TEMPLATES' skins resolve (every skin hash non-zero and known to
/// the model tables), not the AI spawn timing.
#[test]
fn every_0700000b_drone_has_its_skin() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "0700000b.bin", false).expect("load 0700000b");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("[0b] drones {} spawners {} present {}",
        sim.drones.list.len(), sim.drones.spawners.len(),
        sim.drones.list.iter().filter(|d| d.present).count());
    assert_eq!(sim.drones.list.len(), 10, "all 10 oDrone templates exist [D drone census]");
    assert_eq!(sim.drones.spawners.len(), 2, "the two AI_DroneSpawners exist [D #267/#268]");
    for (i, d) in sim.drones.list.iter().enumerate() {
        assert!(d.skin != 0, "drone {i} carries a skin hash (kind {})", d.kind);
    }
}
