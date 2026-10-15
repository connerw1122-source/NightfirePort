//! The pause/fail menus' burning woman (`psiDecompressWoman` 0x1202a8, crate::woman). The disc's 0x030000db is a
//! placeholder; the game decodes the level archive's kind-16 "woman" mask over the fire texture 0x030000da into it.
//! PCSX2 (07000005 fail menu, 2026-10-15): a fiery woman holding a pistol. Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_woman_decodes_into_the_pause_figure_texture() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let w = scene.woman.as_mut().expect("07000005 carries the kind-16 woman entry and the fire texture");
    // Frame 100: the figure covers a good part of the 128×128 frame and its texels come from the fire texture.
    let mut frames = 1;
    while frames < 100 { if w.tick() { frames += 1; } }
    let lit = w.indices.iter().filter(|&&i| i != 0).count();
    eprintln!("frame 100: {lit} lit texels of 16384");
    assert!(lit > 2000 && lit < 9000, "a woman-sized silhouette ({lit})");
    // Her outline: the lit texels sit in a body-shaped band — none in the frame's top-left corner.
    let corner = (0..20).flat_map(|y| (0..20).map(move |x| y * 128 + x)).filter(|&k| w.indices[k] != 0).count();
    assert_eq!(corner, 0, "the top-left corner is empty");
    // The rgba frame goes through the fire palette: lit texels are warm (red ≥ blue).
    let rgba = w.rgba();
    let warm = (0..128 * 128).filter(|&k| w.indices[k] != 0).filter(|&k| rgba[k * 4] >= rgba[k * 4 + 2]).count();
    assert!(warm * 10 >= lit * 9, "the figure is fire-coloured ({warm}/{lit})");
    // Sister: the stream loops after 0x15c frames back to frame 0.
    let first = { let mut w0 = nf_view::load(&iso, "07000005.bin", false).unwrap().woman.unwrap(); w0.indices.clone() };
    let mut n = 0;
    while n < nf_view::woman::FRAMES as usize - 99 { if w.tick() { n += 1; } }
    assert_eq!(w.indices, first, "the loop restarts at frame 0 after 0x15c frames");
}
