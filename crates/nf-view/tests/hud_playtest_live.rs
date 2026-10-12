//! The HUD fixes from the user's playtest of fix-hud (2026-10-11), each against the decompile:
//! - the weapon-switch labels: sprite +0x38 is the draw LAYER (0x1C = drawn, 0xFF = hidden); while `BL+0x95E`
//!   runs only the name shows (no ammo text, no rounds strip), at rest the ammo + the fire mode
//!   [M HUD_UpdateAmmoPane 0x19a7f0:176-189, 218-226, 248-300];
//! - the pickup bar: three 1/5-texel columns of the HUD sheet 0x0300007A (u 190 / 185 / 191, v 0..56), not its
//!   top-left 256×26 corner [L ee_start_05.bin 0x2FC460; M HUD_UpdateStatusPane 0x19c310:340-346];
//! - the crouch icon at screen (48, 48), the game's runtime position [M HUD_UpdateHealthPane 0x19b9c8:309-310];
//! - a hit from straight ahead sets both sides (0xC) [M Player_DealWithObjHit 0x194458:202].
//! The overlay is a pure function of the Sim's state, so each test sets that state and reads the quads.
//! Opt-in via `NF_ISO` (except the direction-bit test).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Every quad (6 vertices) drawn from texture `hash`, as (screen x0, y0, x1, y1, u0, v0, u1, v1) in the 512×448
/// space (aspect 4:3, so no widening).
fn quads(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, hash: u32) -> Vec<[f32; 8]> {
    let Some(ti) = scene.hud_textures.iter().position(|(h, _)| *h == hash) else { return Vec::new() };
    let mut out = Vec::new();
    for r in f.runs.iter().filter(|r| r.0 == ti + 1) {
        for q in f.v[r.2 as usize..(r.2 + r.3) as usize].chunks(6) {
            let (a, b) = (q[0], q[2]);
            out.push([(a.p[0] + 1.0) * 256.0, (1.0 - a.p[1]) * 224.0, (b.p[0] + 1.0) * 256.0, (1.0 - b.p[1]) * 224.0, a.uv[0], a.uv[1], b.uv[0], b.uv[1]]);
        }
    }
    out
}

/// Font glyph quads whose centre lies in the screen box (each shadowed glyph is two quads: shadow + text).
fn glyphs_in(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, x0: f32, y0: f32, x1: f32, y1: f32) -> usize {
    quads(scene, f, 0x0300_0003).iter().filter(|q| {
        let (cx, cy) = ((q[0] + q[2]) / 2.0, (q[1] + q[3]) / 2.0);
        cx > x0 && cx < x1 && cy > y0 && cy < y1
    }).count()
}

#[test]
fn a_weapon_switch_shows_the_name_alone_then_the_ammo_and_fire_mode() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let lang = scene.sim.as_ref().unwrap().lang.clone().expect("USATxt.dat");
    let (w, name_len, mode_len) = {
        let s = scene.sim.as_ref().unwrap();
        let w = s.gun.weapon as usize;
        let n = lang.bind(s.weapons.name_label(w)).unwrap_or("").chars().filter(|c| !c.is_whitespace()).count();
        let m = lang.bind(s.weapons.fire_mode_label(w, 0)).unwrap_or("").chars().filter(|c| !c.is_whitespace()).count();
        (w, n, m)
    };
    eprintln!("[hud] weapon {w}: name {name_len} glyphs, fire mode {mode_len}");
    assert!(name_len > 0 && mode_len > 0 && name_len != mode_len, "the two labels must differ in length to tell them apart");
    // At rest (timer 0): the ammo text ("7-41", y ≈ 412), the rounds strip, and the fire mode (y ≈ 432).
    assert_eq!(scene.sim.as_ref().unwrap().hud.weapon_label_timer, 0, "the level-start timer ran out");
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let (ammo_rest, label_rest) = (glyphs_in(&scene, &f, 380.0, 395.0, 512.0, 420.0), glyphs_in(&scene, &f, 380.0, 422.0, 512.0, 446.0));
    let strip_rest = quads(&scene, &f, 0x0300_0128).len();
    eprintln!("[hud] at rest: ammo glyphs {ammo_rest}, label glyphs {label_rest}, strip quads {strip_rest}");
    assert!(ammo_rest >= 3, "the ammo count draws at rest");
    assert!(strip_rest >= 1, "the rounds strip draws at rest");
    assert_eq!(label_rest, 2 * mode_len, "at rest the label is the fire mode only (no ghost of the name)");
    // During the switch timer: the name alone.
    scene.sim.as_mut().unwrap().hud.weapon_label_timer = 30;
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let (ammo_sw, label_sw) = (glyphs_in(&scene, &f, 380.0, 395.0, 512.0, 420.0), glyphs_in(&scene, &f, 380.0, 422.0, 512.0, 446.0));
    let strip_sw = quads(&scene, &f, 0x0300_0128).len();
    eprintln!("[hud] switching: ammo glyphs {ammo_sw}, label glyphs {label_sw}, strip quads {strip_sw}");
    assert_eq!(ammo_sw, 0, "no ammo count while the name shows");
    assert_eq!(strip_sw, 0, "no rounds strip while the name shows");
    assert_eq!(label_sw, 2 * name_len, "the label is the weapon name only");
}

#[test]
fn the_pickup_bar_is_the_sheet_s_dark_column_with_caps() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    // Sister: no message, no bar.
    assert!(!quads(&scene, &f, 0x0300_007a).iter().any(|q| (q[1] - 364.0).abs() < 0.5 && (q[3] - 390.0).abs() < 0.5), "no bar quads (y 364..390) without a message");
    scene.sim.as_mut().unwrap().hud.msgs.push_back(nf_view::scene::Msg { text: "Picked up 6x .357 Magnum rounds".into(), time: 180.0 });
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let bar: Vec<[f32; 8]> = quads(&scene, &f, 0x0300_007a).into_iter().filter(|q| (q[1] - 364.0).abs() < 0.5).collect();
    for q in &bar { eprintln!("[hud] bar quad x {:.1}..{:.1} y {:.1}..{:.1} u {:.4}..{:.4} v {:.4}..{:.4}", q[0], q[2], q[1], q[3], q[4], q[6], q[5], q[7]); }
    assert_eq!(bar.len(), 3, "middle + two caps");
    let tex = 256.0;
    let mid = bar.iter().find(|q| (q[2] - q[0] - 256.0).abs() < 0.5).expect("a 256-wide middle");
    assert!((mid[0] - 128.0).abs() < 0.5 && (mid[3] - 390.0).abs() < 0.5, "the middle spans (128, 364)..(384, 390)");
    assert!((mid[4] * tex - 190.0).abs() < 1e-3 && (mid[6] * tex - 191.0).abs() < 1e-3, "the middle samples the 1-texel column u 190");
    assert!((mid[7] * tex - 56.0).abs() < 1e-3, "over v 0..56");
    assert!(bar.iter().any(|q| (q[2] - 128.0).abs() < 0.5 && (q[4] * tex - 185.0).abs() < 1e-3), "the left cap (u 185) ends at x 128");
    assert!(bar.iter().any(|q| (q[0] - 384.0).abs() < 0.5 && (q[4] * tex - 191.0).abs() < 1e-3), "the right cap (u 191) starts at x 384");
    // The old bug: the top-left 256×26 corner of the sheet (its 007 coin and health arcs).
    assert!(!bar.iter().any(|q| q[4] == 0.0 && (q[6] * tex - 256.0).abs() < 1e-3), "not the sheet's top strip");
}

#[test]
fn the_crouch_icon_sits_top_left_at_48_48() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let input = nf_game::player::Input { crouch: true, ..Default::default() };
    for _ in 0..12 { scene.sim.as_mut().unwrap().step_bond(input, false, false, false); }
    assert!(scene.sim.as_ref().unwrap().bond.crouched, "the crouch input crouched Bond");
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let icon = quads(&scene, &f, 0x0300_0173);
    assert_eq!(icon.len(), 1, "one crouch icon");
    let q = icon[0];
    let (cx, cy) = ((q[0] + q[2]) / 2.0, (q[1] + q[3]) / 2.0);
    eprintln!("[hud] crouch icon centre ({cx:.1}, {cy:.1}), size {:.0}×{:.0}", q[2] - q[0], q[3] - q[1]);
    assert!((cx - 48.0).abs() < 0.5 && (cy - 48.0).abs() < 0.5, "centred on the game's runtime (48, 48)");
    // Sister: not the template's (336, 37) the first port used.
    assert!(cx < 128.0, "in the left quarter of the screen");
}

#[test]
fn a_hit_from_straight_ahead_glows_both_sides() {
    let mut h = nf_game::bullet::PlayerHealth::default();
    let eye = [0.0, 1.5, 0.0];
    // Yaw 0 faces +Z: a sniper 60 units ahead, slightly above.
    h.pain_from(2.0, 1, 0x0700_0005, [0.0, 3.0, 60.0], eye, 0.0);
    assert_eq!(h.dir_bits, 0xC, "neither axis reaches 0.5: both sides glow");
    // Sister: a hit from the side still picks one side.
    h.pain_from(2.0, 1, 0x0700_0005, [4.0, 1.5, 0.0], eye, 0.0);
    assert_eq!(h.dir_bits, 0x4);
}

#[test]
fn the_pickup_bar_stays_centred_under_its_text_on_a_widescreen_window() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    scene.sim.as_mut().unwrap().hud.msgs.push_back(nf_view::scene::Msg { text: "Picked up Winter Covert Sniper".into(), time: 180.0 });
    // In NDC (x in −1..1) at 16:9: the bar's centre and the text's centre are both the screen's centre.
    let centre = |aspect: f32| -> (f32, f32) {
        let f = nf_view::hud::overlay(&scene, aspect);
        let ti = |h: u32| scene.hud_textures.iter().position(|(hh, _)| *hh == h).unwrap() + 1;
        let span = |tex: usize, y0: f32, y1: f32| {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for r in f.runs.iter().filter(|r| r.0 == tex) {
                for v in &f.v[r.2 as usize..(r.2 + r.3) as usize] {
                    let sy = (1.0 - v.p[1]) * 224.0;
                    // The bar's columns only (u 185..196 of the sheet); the health arcs share the texture.
                    let bar_col = tex != ti(0x0300_007a) || (185.0..=196.0).contains(&(v.uv[0] * 256.0));
                    if sy >= y0 && sy <= y1 && bar_col { lo = lo.min(v.p[0]); hi = hi.max(v.p[0]); }
                }
            }
            (lo + hi) / 2.0
        };
        (span(ti(0x0300_007a), 363.5, 390.5), span(ti(0x0300_0003), 366.0, 388.0))
    };
    let (bar, text) = centre(16.0 / 9.0);
    eprintln!("[hud] 16:9 bar centre {bar:.3}, text centre {text:.3} (NDC)");
    assert!(bar.abs() < 0.01, "the bar is centred on a widescreen window");
    assert!((bar - text).abs() < 0.03, "the text sits on the bar");
}
