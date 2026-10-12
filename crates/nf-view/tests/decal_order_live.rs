//! R2 (user playtest, 07000007 nf-shots 002/003/005): baked shadow decals left navy holes in the walls up close. They
//! share the walls' list-0 draw state, and the front-to-back sort drew a decal before the wall it lies on, so its
//! depth write rejected the wall. The renderer now draws alpha-shaped list-0 textures after the opaque ones
//! (`gpu::alpha_shaped`). This pins the classification on the real data. Opt-in: NF_ISO.
use nf_view::scene::Layer;

#[test]
fn shadow_decal_textures_are_alpha_shaped_and_walls_are_not() {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    // The decal textures fix-render4 named [D 07000007.bin map 0x2f]: 002f:003e and 002f:0055.
    for idx in [0x3e, 0x55] {
        let t = scene.textures.iter().find(|t| t.key == (0x2f, idx)).expect("the shadow decal texture is loaded");
        assert!(nf_view::gpu::alpha_shaped(&t.rgba), "002f:{idx:04x} (a shadow decal) is alpha-shaped");
    }
    // Sister: most of the world's list-0 draws stay in the opaque group (front-to-back), so the reorder is narrow.
    let world: Vec<_> = scene.materials.iter().filter(|m| m.layer == Layer::World && m.draw.pass == 1).collect();
    let late = world.iter().filter(|m| m.key.and_then(|k| scene.textures.iter().find(|t| t.key == k)).is_some_and(|t| nf_view::gpu::alpha_shaped(&t.rgba))).count();
    eprintln!("[r2] {late} of {} list-0 World draws are alpha-shaped (drawn after the opaque ones)", world.len());
    assert!(late * 10 < world.len(), "fewer than 10% of list-0 draws move to the late group");
    // Sister for the predicate: a fully opaque texture is not shaped; a half-transparent one is.
    let opaque: Vec<u8> = (0..64).flat_map(|_| [10, 20, 30, 255]).collect();
    let shape: Vec<u8> = (0..64).flat_map(|i| [0, 0, 0, if i < 32 { 0 } else { 255 }]).collect();
    assert!(!nf_view::gpu::alpha_shaped(&opaque));
    assert!(nf_view::gpu::alpha_shaped(&shape));
}
