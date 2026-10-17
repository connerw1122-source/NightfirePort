//! The first-person sleeve is `AnimSleeveGetEntity(0)` 0x1c6590: the FIRST present entity of the fixed
//! SleeveEnts list @0x2c6ed8 [M], and each level archive holds its own Bond's sleeve [D: 07000008 holds
//! only 0x020001f7 of the eight]. The port's weapon-model merges lent other levels' sleeves that sit
//! earlier in the list, so on 08 the rocket launcher drew a grey glove instead of the tuxedo cuff and
//! bare hand (PCSX2 side by side, 2026-10-17). Opt-in: `NF_ISO`.

const SLEEVES: [u32; 8] = [0x0200_09d0, 0x0200_0900, 0x0200_01f7, 0x0200_0929, 0x0200_09e6, 0x0200_09ff, 0x0200_0a14, 0x0200_0a13];

/// The sleeve the launcher's first-person skin (0x05000046, the `+0xDC` of 0x2d) resolves to.
fn launcher_sleeve(scene: &nf_view::scene::Scene) -> Option<u32> {
    let sim = scene.sim.as_ref().unwrap();
    let sk = sim.skins.get(&0x0500_0046)?;
    sk.main.iter().flatten().copied().find(|h| SLEEVES.contains(h))
}

#[test]
fn the_view_model_keeps_the_levels_own_sleeve() {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    // The give path lends the launcher's assets from other archives (the borrowed-sleeve route).
    let scene = nf_view::load_weapons(&iso, "07000008.bin", false, &[0x2d]).expect("load 08");
    assert_eq!(launcher_sleeve(&scene), Some(0x0200_01f7), "08's launcher must wear 08's own sleeve 0x020001f7");
    // Sister: a level whose archive holds several sleeves keeps the game's first-present pick.
    let scene = nf_view::load(&iso, "07000029.bin", false).expect("load 29");
    assert_eq!(launcher_sleeve(&scene), Some(0x0200_01f7), "29 holds 0x020001f7 first in SleeveEnts order");
}
