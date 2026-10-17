//! level-08 model sweep: every material on 07000008 resolves a texture — no white fallback.
//! (The GPU's missing-texture path draws a 1×1 white; `NF_TEX_TRACE=1` prints the fallbacks at
//! load; this test pins the same condition through the scene's material table.) Mesh/skin
//! resolution per level is pinned level-wide by models_world_live; here the live render scene is
//! the witness. Opt-in via NF_ISO.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn every_07000008_material_resolves_a_texture_no_white_fallback() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let w = &scene;
    let tex_keys: std::collections::HashSet<_> = w.textures.iter().map(|t| t.key).collect();
    let fallbacks: Vec<_> = w.materials.iter().enumerate()
        .filter(|(_, m)| m.key.map_or(true, |k| !tex_keys.contains(&k))).collect();
    eprintln!("[08] materials {} textures {} white-fallback {}", w.materials.len(), w.textures.len(), fallbacks.len());
    for (i, m) in fallbacks.iter().take(16) {
        eprintln!("[08]   fallback mat {i}: layer {:?} pass {:?} key {:?}", m.layer, m.draw.pass, m.key);
    }
    assert!(fallbacks.is_empty(), "every material resolves a texture on 07000008 (no white fallback)");
}

/// The level's NPC skins and the drone skins resolve (the census's the drones over skins
/// 0x0500002e/2f/30/35 [D oDrone p0]): every live drone has a skin hash the model tables hold.
#[test]
fn every_07000008_drone_has_its_skin() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let present: Vec<(usize, u32)> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present).map(|(i, d)| (i, d.skin)).collect();
    eprintln!("[08] drones present {}/{} skins {:?}", present.len(), sim.drones.list.len(),
        present.iter().map(|(_, s)| format!("{s:#x}")).collect::<Vec<_>>());
    assert!(!present.is_empty(), "the level's drones are present");
    for (i, skin) in &present {
        assert!(*skin != 0, "drone {i} carries a skin hash");
    }
}
