//! fix-space-pane: the 0700001b (Equinox, space) missile-status pane — the white "T" the real game
//! draws at the top right from the level start [L PCSX2 capture 2026-10-16, gameplay start]. The
//! port drew nothing there.
//!
//! Game path [M unless tagged]: pane 20 of `PaneList` 0x2be5b8 is `SpacePane` 0x2be598 = {x 0, y 0,
//! flags 0, `HUD_CreateDefault`, `HUD_UpdateSpacePane` 0x19d3b8, `SpaceSprInfo` 0x2be3b0, 11 sprites}
//! [L ACTION.ELF]. The update enables the pane while `GameState+0xC == 0x0700001b` and disables it on
//! every other level [M 0x19d3b8:11-19]; sprites 1..8 change texture/colour by `MissileDeploy[k−1]`
//! and switch channel k, sprite 0 flashes on channel 9, sprites 9/10 are 0x7f7f7f7f unless channels
//! 0x18 / 0x1C flash them. Textures 0x03000177..7B live in 0700001b.bin [D tex-list entry 61].
//!
//! Numbers and hashes only. Opt-in via NF_ISO; run with NF_SILENT=1.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

const SPACE_TEX: [u32; 3] = [0x0300_0179, 0x0300_0178, 0x0300_017b];

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    let idle = Input::default();
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    scene
}

/// Whether the overlay has a run drawing texture `hash` (`HudFrame.runs`' tex = hud_textures index + 1).
fn drawn(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, hash: u32) -> bool {
    match scene.hud_textures.iter().position(|(h, _)| *h == hash) {
        Some(ti) => f.runs.iter().any(|(t, _, _, _)| *t == ti + 1),
        None => false,
    }
}

#[test]
fn the_0700001b_missile_pane_draws_at_the_level_start() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso, "0700001b.bin");
    let t = scene.hud.as_ref().expect("hud tables");
    // The table as the ELF holds it [L SpaceSprInfo 0x2be3b0]: 11 entries, the T at (342, 48) tex 0x03000179,
    // the lamps at x 369/427, y 93..156, the boxes at (369, 48) / (415, 48) tex 0x0300017B.
    assert_eq!(t.space.len(), 11);
    assert_eq!((t.space[0].x, t.space[0].y, t.space[0].texture), (342, 48, 0x0300_0179));
    let lamps: Vec<(i16, i16)> = t.space[1..9].iter().map(|s| (s.x, s.y)).collect();
    assert_eq!(lamps, vec![(369, 93), (427, 93), (369, 114), (427, 114), (369, 135), (427, 135), (369, 156), (427, 156)]);
    assert!(t.space[1..9].iter().all(|s| s.texture == 0x0300_0178));
    assert_eq!((t.space[9].x, t.space[10].x, t.space[9].texture, t.space[10].texture), (369, 415, 0x0300_017b, 0x0300_017b));
    for h in SPACE_TEX { assert!(scene.hud_textures.iter().any(|(k, _)| *k == h), "texture {h:#x} not loaded"); }
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    for h in SPACE_TEX { assert!(drawn(&scene, &f, h), "pane 20 sprite texture {h:#x} not drawn on 0700001b"); }
}

#[test]
fn the_missile_pane_is_level_gated() {
    // Sister: the same update disables pane 20 on every other level [M 0x19d3b8:16-19].
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso, "07000005.bin");
    let t = scene.hud.as_ref().expect("hud tables");
    assert!(nf_view::hud::space_pane(&t.space, 0x0700_0005, |_| false, 0).is_empty());
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    for h in SPACE_TEX { assert!(!drawn(&scene, &f, h), "pane 20 texture {h:#x} drawn on 07000005"); }
}

#[test]
fn the_missile_pane_channel_rules() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("iso");
    let elf = img.read_file(img.find("/ACTION.ELF").expect("ACTION.ELF")).expect("read");
    let t = nf_formats::hud::HudTables::from_action_elf(&elf).expect("tables");
    let l = 0x0700_001b;
    // Nothing on: the template textures; the boxes 0x7f7f7f7f [M 0x19d3b8 tail].
    let idle = nf_view::hud::space_pane(&t.space, l, |_| false, 0);
    assert_eq!(idle.len(), 11);
    assert_eq!(idle[3].0.texture, 0x0300_0178);
    assert_eq!((idle[9].1, idle[10].1), (0x7f7f_7f7f, 0x7f7f_7f7f));
    // Channel 3 on (MissileDeploy[2] = 0): lamp 3 → tex 0x0300017A, 0xff0000ff [M 0x19d3b8 second loop].
    let ch3 = nf_view::hud::space_pane(&t.space, l, |k| k == 3, 0);
    assert_eq!((ch3[3].0.texture, ch3[3].1), (0x0300_017a, 0xff00_00ff));
    assert_eq!(ch3[4].0.texture, 0x0300_0178);
    // Channel 9 flashes the T: 0xff000080 on the first quarter-second of each half second, else 0x7f7f7f7f.
    let on9 = |k: u32| k == 9;
    assert_eq!(nf_view::hud::space_pane(&t.space, l, on9, 0)[0].1, 0xff00_0080);
    assert_eq!(nf_view::hud::space_pane(&t.space, l, on9, 15)[0].1, 0x7f7f_7f7f);
    // Channel 0x1C flashes box 10 only.
    let on1c = nf_view::hud::space_pane(&t.space, l, |k| k == 0x1c, 3);
    assert_eq!((on1c[9].1, on1c[10].1), (0x7f7f_7f7f, 0xff00_0080));
}
