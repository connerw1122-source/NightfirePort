//! fix-zoom-overlays: the zoom overlays draw as the game draws them — one frame, not a 2×2 grid.
//!
//! The user's playtest (2026-10-13): zoomed with the Micro-Camera (0x54, 07000005) and the AIMS-20
//! (0x1a, 07000016 — [L weapon_data 0x2bf150+0x38 = "AIMS-20", the Burst rifle] bound over USATxt.dat)
//! the port drew four copies of the frame with a black cross through the middle. The game's sprites
//! carry flag 0x400, and `View_DrawSprites` 0x1e5290:52-162 splits such a sprite into
//! `tiles = (du+1)/texW × (dv+1)/texH`, each tile `w/tilesX × h/tilesY` screen px sampling
//! `(u, v, du>>tilesX−1, dv>>tilesY−1)` of the texture, the U flip alternating per X tile (start
//! parity = the sprite's 0x40 bit) and the V flip per Y tile (0x80) — the mirror repeat. The zoom
//! designs are quarter textures [D 07000005.bin / 07000016.bin: the viewfinder 0x03000055 is 128×64,
//! the OICW circle 0x03000141 is 64×64, the scope 0x03000034 is 256×256], so the mirror completes
//! them into ONE frame [M + D]. The port drew the sprite once with the full UV extents, which its
//! renderer wraps — an unmirrored 2×2 repeat: the grid.
//!
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// One textured quad of the frame, in the game's 512×448 screen space (exact at aspect 4/3, where
/// `Anchor::Centre` and `Anchor::Stretch` coincide): (x0, y0, x1, y1, uv0, uv1, uv2, uv3) with the
/// quad's [TL, TR, BR, TL, BR, BL] vertices.
struct Q { x0: f32, y0: f32, x1: f32, y1: f32, uv: [[f32; 2]; 4] }

fn quads_of(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame, hash: u32, blend: u8) -> Vec<Q> {
    let Some(ti) = scene.hud_textures.iter().position(|(h, _)| *h == hash) else { return Vec::new() };
    let mut out = Vec::new();
    for r in &f.runs {
        if r.0 != ti + 1 || r.1 != blend { continue; }
        for k in 0..(r.3 as usize / 6) {
            let v: Vec<_> = f.v[r.2 as usize + k * 6..r.2 as usize + k * 6 + 6].iter().map(|q| (q.p, q.uv)).collect();
            let xs = [v[0].0[0], v[1].0[0]];
            let ys = [v[0].0[1], v[2].0[1]];
            let sx = |n: f32| (n + 1.0) * 256.0;
            let sy = |n: f32| (1.0 - n) * 224.0;
            out.push(Q {
                x0: sx(xs[0].min(xs[1])), x1: sx(xs[0].max(xs[1])),
                y0: sy(ys[0].max(ys[1])), y1: sy(ys[0].min(ys[1])),
                uv: [v[0].1, v[1].1, v[2].1, v[5].1],
            });
        }
    }
    out.sort_by(|a, b| (a.y0, a.x0).partial_cmp(&(b.y0, b.x0)).unwrap());
    out
}

fn approx(a: f32, b: f32, tol: f32) -> bool { (a - b).abs() <= tol }

/// The four mirror tiles of one overlay sprite form one frame: the tiles partition `rect`, each
/// samples the whole texture once, and the flips mirror — TL (U+, V+), TR (U−, V+), BL (U+, V−),
/// BR (U−, V−) with the start parity from the sprite's 0x40/0x80 bits (all the zoom panes start 0).
fn assert_one_frame(quads: &[Q], rect: (f32, f32, f32, f32), tile: (f32, f32), uv_span: (f32, f32), what: &str) {
    assert_eq!(quads.len(), 4, "{what}: the sprite draws as four mirrored tiles, got {}", quads.len());
    let want = [(rect.0, rect.1), (rect.0 + tile.0, rect.1), (rect.0, rect.1 + tile.1), (rect.0 + tile.0, rect.1 + tile.1)];
    for (q, w) in quads.iter().zip(want) {
        assert!(approx(q.x0, w.0, 0.5) && approx(q.x1, w.0 + tile.0, 0.5) && approx(q.y0, w.1, 0.5) && approx(q.y1, w.1 + tile.1, 0.5),
            "{what}: tile at ({},{})..({},{}) should be ({},{})..({},{})", q.x0, q.y0, q.x1, q.y1, w.0, w.1, w.0 + tile.0, w.1 + tile.1);
        // Each tile samples the texture once (its UV span is one texture, not the wrapped repeat).
        let (du, dv) = ((q.uv[1][0] - q.uv[0][0]).abs(), (q.uv[2][1] - q.uv[0][1]).abs());
        assert!(approx(du, uv_span.0, 0.01) && approx(dv, uv_span.1, 0.01),
            "{what}: tile uv span ({du},{dv}) should be {:?}", uv_span);
    }
    // The mirror: the right tiles run U backwards, the bottom tiles V backwards (parity starts 0).
    let (tl, tr, bl, br) = (&quads[0], &quads[1], &quads[2], &quads[3]);
    assert!(tl.uv[0][0] < tl.uv[1][0] && tl.uv[0][1] < tl.uv[2][1], "{what}: top-left tile unflipped");
    assert!(tr.uv[0][0] > tr.uv[1][0] && tr.uv[0][1] < tr.uv[2][1], "{what}: top-right tile U-mirrored");
    assert!(bl.uv[0][0] < bl.uv[1][0] && bl.uv[0][1] > bl.uv[2][1], "{what}: bottom-left tile V-mirrored");
    assert!(br.uv[0][0] > br.uv[1][0] && br.uv[0][1] > br.uv[2][1], "{what}: bottom-right tile U+V-mirrored");
    // No seam: the mirrored neighbours sample the same texel at the shared edge.
    assert!(approx(tl.uv[1][0], tr.uv[0][0], 0.01) && approx(bl.uv[1][0], br.uv[0][0], 0.01),
        "{what}: the vertical centre must not jump in U");
    assert!(approx(tl.uv[2][1], bl.uv[0][1], 0.01) && approx(tr.uv[2][1], br.uv[0][1], 0.01),
        "{what}: the horizontal centre must not jump in V");
}

/// The Micro-Camera (0x54, the user's 07000005 spot): hold aim + the zoom key through
/// `Player_Zoom` 0x1a1a70 (the real input path), the viewfinder 0x03000055 draws as ONE frame —
/// four 256×224 mirror tiles over the whole 512×448 screen of the 128×64 quarter texture.
#[test]
fn the_micro_camera_viewfinder_is_one_frame() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = 0x54; // the Micro-Camera [L weapon_data 0x2bf150+0x38]
        sim.gun.switch_request = 0x54;
        sim.aiming = true;
        sim.cursor = [0.0, 0.0];
        sim.cross = [0.0, 0.0];
    }
    for _ in 0..400 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { aiming: true, zoom: true, ..Default::default() }, false, false, false);
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.zoom > 1.9, "the Micro-Camera zooms on the held key, got {}", sim.zoom);
        assert!(sim.zoom_target > 1.0);
    }
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let q = quads_of(&scene, &f, 0x0300_0055, 2);
    // [L CameraSprInfo 0x2bd600 sprite 1: (0,0) 512×448, uv (0,0,255,127), flags 0x0600]
    // [D 07000005.bin: 0x03000055 is 128×64] → tiles 256×224, per-tile uv (0,0,127,63) of 128×64.
    assert_one_frame(&q, (0.0, 0.0, 512.0, 448.0), (256.0, 224.0), (127.0 / 128.0, 63.0 / 64.0), "the Micro-Camera viewfinder");
    // Sister: no scope tiles on the Micro-Camera.
    assert!(quads_of(&scene, &f, 0x0300_0034, 2).is_empty(), "no sniper scope on the Micro-Camera");
}

/// The AIMS-20 (0x1a, the user's 07000016 rifle — the Burst one): its OICW zoom circle
/// 0x03000141 (64×64 quarter [D 07000016.bin]) draws as ONE circle — four 64×64 mirror tiles
/// inside the 128×128 block at (192,160). Before the fix the port drew the sprite once with the
/// wrapped UV: the 2×2 grid the user saw.
#[test]
fn the_aims20_zoom_circle_is_one_frame() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000016.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.weapon = 0x1a;
    sim.zoom_target = sim.weapons.zoom(0x1a as usize).max(2.0);
    sim.zoom = sim.zoom_target;
    drop(sim);
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let q = quads_of(&scene, &f, 0x0300_0141, 1);
    // [L OICWSprInfo 0x2be008 sprite 1: (192,160) 128×128, uv (1,1,127,127), flags 0x0500]
    // → tiles 64×64, per-tile uv (1,1,63,63) of 64×64 (span 63/64 of the texture).
    assert_one_frame(&q, (192.0, 160.0, 320.0, 288.0), (64.0, 64.0), (63.0 / 64.0, 63.0 / 64.0), "the AIMS-20 zoom circle");
    // Sister: the subtractive full-screen tint (sprite 0, flags 0x0200 — no tiling) stays one quad.
    let tint = quads_of(&scene, &f, 0x0300_0002, 2);
    assert_eq!(tint.len(), 1, "the OICW tint is a single full-screen quad");
}

/// The scoped rifles (0x1e Winter Tactical Sniper and 0x11 Delta Repeater — the driver's 0700000c /
/// 07000016 spots — plus 0x24 Winter Covert Sniper): the default Sight pane draws the scope as ONE
/// oval — four 224×224 mirror tiles of the 256×256 texture in the 448×448 block at (32,0). The
/// port's scope already matched PCSX2 on 0700000c: this pins it while the shared tiler lands.
#[test]
fn the_sniper_scope_stays_one_frame() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000016.bin", false).expect("load");
    for w in [0x1eu8, 0x11, 0x24] {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.weapon = w;
        sim.zoom_target = sim.weapons.zoom(w as usize).max(2.0);
        sim.zoom = sim.zoom_target;
        drop(sim);
        let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
        let q = quads_of(&scene, &f, 0x0300_0034, 2);
        // [L SightSprInfo 0x2bd558 sprite 0: (32,0) 448×448, uv (0,0,511,511), flags 0x0600]
        // → tiles 224×224, per-tile uv (0,0,255,255) of 256×256.
        assert_one_frame(&q, (32.0, 0.0, 480.0, 448.0), (224.0, 224.0), (255.0 / 256.0, 255.0 / 256.0), &format!("the scope of {w:#04x}"));
    }
}

/// The Phoenix Samurai's laser frame (0x32, LaserSprInfo 0x2be288 — flags 0x0100, no 0x400): six
/// solid strips at the game's exact rects, whole and untiled. Sister guard: the mirror tiler must
/// not leak into sprites without the flag.
#[test]
fn the_laser_frame_strips_stay_whole() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000016.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.weapon = 0x32;
    sim.zoom_target = sim.weapons.zoom(0x32 as usize).max(2.0);
    sim.zoom = sim.zoom_target;
    drop(sim);
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let q = quads_of(&scene, &f, 0x0300_0002, 1);
    let mut want = [(140.0, 64.0, 372.0, 68.0), (140.0, 384.0, 372.0, 388.0), (192.0, 223.0, 232.0, 225.0), (280.0, 223.0, 320.0, 225.0), (255.0, 160.0, 257.0, 200.0), (255.0, 248.0, 257.0, 288.0)];
    want.sort_by(|a, b| (a.1, a.0).partial_cmp(&(b.1, b.0)).unwrap());
    assert_eq!(q.len(), want.len(), "six strips");
    for (g, w) in q.iter().zip(want) {
        assert!(approx(g.x0, w.0, 0.5) && approx(g.y0, w.1, 0.5) && approx(g.x1, w.2, 0.5) && approx(g.y1, w.3, 0.5),
            "strip ({},{})..({},{}) should be ({},{})..({},{})", g.x0, g.y0, g.x1, g.y1, w.0, w.1, w.2, w.3);
    }
}

/// On a window wider than 4:3 the zoomed frame keeps the game's 4:3 shape, centred, with the sides
/// black (the game's overlay covered its whole screen; the port's wide window shows the world
/// beside the 4:3 frame without them). At 4:3 itself no bars.
#[test]
fn the_zoomed_frame_centres_with_black_side_bars_on_wide_windows() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.weapon = 0x54;
    sim.zoom_target = sim.weapons.zoom(0x54 as usize).max(2.0);
    sim.zoom = sim.zoom_target;
    drop(sim);
    let black_bars = |aspect: f32| -> Vec<(f32, f32, f32, f32)> {
        let f = nf_view::hud::overlay(&scene, aspect);
        // Blend 0 on the solid texture: only the bars (the OICW tint and the scope strips are
        // blend 2, the laser strips blend 1; the crosshair is another texture).
        quads_of(&scene, &f, 0x0300_0002, 0).iter().map(|q| (q.x0, q.y0, q.x1, q.y1)).collect()
    };
    // The black colour [0,0,0,1] with blend 0: only the bars (the subtractive strips are blend 2).
    let bars = black_bars(2.0);
    assert_eq!(bars.len(), 2, "two side bars on a 2:1 window");
    for (x0, y0, x1, y1) in bars {
        assert!(approx(y0, 0.0, 0.5) && approx(y1, 448.0, 0.5), "the bars span the full height");
        let at_frame_edge = approx(x1.min(512.0 - x0), (1.0 - 4.0 / 3.0 / 2.0) * 256.0, 1.0);
        let at_window_edge = approx(x0, 0.0, 0.5) || approx(x1, 512.0, 0.5);
        assert!(at_frame_edge && at_window_edge, "bar ({x0},{y0})..({x1},{y1}) hugs the window edge and stops at the 4:3 frame");
    }
    assert!(black_bars(4.0 / 3.0).is_empty(), "no bars on a 4:3 window");
}

/// The overlay selection per weapon id [M `HUD_UpdateCrossHair` 0x19e6d0:106-147 — the switch runs
/// on `wid − 0x1a` with cases 0/1, 0x18/0x19, 0x3a/0x3b and the default only; there is no case
/// 0x38/0x39] and the name table [L ee_start_05/07000001/0700000a weapon_data+0x38 over USATxt.dat:
/// 0x52/0x53 = "Phoenix Ronin", 0x54/0x55 = "Micro-Camera"]: the Ronin twins fall to the default
/// Sight pane like any other scoped weapon. (fix-viewmodel-zoom mapped them to the Camera pane on
/// the belief they were the micro camera — the name table refutes it.) Pure — no ISO.
#[test]
fn the_overlay_selection_follows_the_game_switch() {
    use nf_view::hud::{zoom_overlay, ZoomOverlay};
    assert_eq!(zoom_overlay(0x1a), ZoomOverlay::Oicw, "the AIMS-20");
    assert_eq!(zoom_overlay(0x1b), ZoomOverlay::Oicw, "the AIMS-20 twin");
    assert_eq!(zoom_overlay(0x32), ZoomOverlay::Laser, "the Phoenix Samurai");
    assert_eq!(zoom_overlay(0x33), ZoomOverlay::Laser);
    assert_eq!(zoom_overlay(0x54), ZoomOverlay::Camera, "the Micro-Camera");
    assert_eq!(zoom_overlay(0x55), ZoomOverlay::Camera, "the twin (the game rewrites 0x55 to 0x54)");
    assert_eq!(zoom_overlay(0x52), ZoomOverlay::Sight, "the Phoenix Ronin — no case 0x38 in the switch");
    assert_eq!(zoom_overlay(0x53), ZoomOverlay::Sight, "the Ronin twin");
    assert_eq!(zoom_overlay(0x1e), ZoomOverlay::Sight, "the Winter Tactical Sniper — default");
    assert_eq!(zoom_overlay(0x11), ZoomOverlay::Sight, "the Delta Repeater — default");
    assert_eq!(zoom_overlay(0x24), ZoomOverlay::Sight, "the Winter Covert Sniper — default");
}
