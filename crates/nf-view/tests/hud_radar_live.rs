//! fix-hud-radar: the HUD radar — the SP truth and the MP projection.
//!
//! What the game shows [M unless tagged]:
//! - The SP HUD has NO radar. `HUD_Init` 0x199e88:32-37 walks `PaneList` whenever
//!   MPSettings+0x384 == 0 ([L] ee_start_05.bin @0x2a47a0+0x384 = 0), and the SP PaneList slots
//!   13/14 are NULL ([L] docs/fields/hud-panes.md, decoded from 0x2be5b8; [L] the SP snapshots'
//!   HUD 0x16c8630 pane array — panes 13/14 absent). The radar exists only as MPPaneList[14]
//!   (`HUD_CreateRadar` 0x19f4e8 + `HUD_RadarUpdate` 0x19f6e8 + `MP_GetRadarObjects` 0x189840).
//! - The bottom-left ring is the health pane's sprite 15 [M 0x19b9c8:205-207]: template (4,4)
//!   87×86, uv (102,113,85,86) on sheet 0x0300007a, layer 9 [L HealthSprInfo[15] @0x2bd050+15·0x2c],
//!   tinted the health colour with alpha 140 every frame.
//! - The radar's blips (MP): the projection of `HUD_RadarUpdate` 0x19f6e8:108-153, verified against
//!   the disassembly 0x19f758-0x19fa60 (the decompiler's scratch-vec aliases are wrong there): the
//!   view-space direction's negated x/z × min(len·0.02, 1) around the ring centre; the height cue
//!   (dy·0.125 clamped ±1) is a dead store (0x319574 has no reader); the blips carry flag 0x800
//!   (drawn centred [M 0x1e5290:60-75]), tex 0x0300016f, uv from TexUV.291 @0x2becf0 [L], layer 0x1c;
//!   the ring sprite hides unless MPSettings+player·0x30+0x28 (radar on) [M 0x19f6e8:102-105] and its
//!   created (512,0) rect clamps to the viewer's top-right by `HUD_FixMP` 0x19b110:73-92.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_view::hud::{radar_project, radar_ring_layer, radar_ring_rect, RadarObj};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A level loaded with Bond in control at the spawn (the same warm-up the other live tests use).
fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    let idle = nf_game::player::Input::default();
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the intro never released Bond");
    scene
}

/// The HUD quads in PIXEL space (the vertices are NDC; invert `Hud::ndc` at a 4:3 window:
/// x = (nx+1)·256, y = (1−ny)·224) as (layer, tex_hash, x, y, w, h, argb-ish colour).
fn quads_px(scene: &nf_view::scene::Scene, frame: &nf_view::gpu::HudFrame) -> Vec<(i32, u32, f32, f32, f32, f32, [f32; 4])> {
    let _ = scene;
    let mut out = Vec::new();
    for (ti, _blend, start, count) in frame.runs.iter().copied() {
        for v in frame.v[start as usize..(start + count) as usize].chunks_exact(6) {
            // NDC corners of the first triangle's first and third vertex.
            let (a, b) = (v[0].p, v[2].p);
            let x = (a[0] + 1.0) * 256.0;
            let y = (1.0 - a[1]) * 224.0;
            let w = (b[0] - a[0]) * 256.0;
            let h = (b[1] - a[1]) * -224.0;
            out.push((0, ti as u32, x, y, w, h, v[0].c));
        }
    }
    out
}

/// The health pane's ring quad: at (20,341) 87×86, sheet uv (102,113,85,86), the health colour
/// (0x52a88b → the GS 0.5/0.65/0.545 scale) at alpha 140 [L HealthSprInfo[15]; M 0x19b9c8:205-207].
fn find_ring(scene: &nf_view::scene::Scene, frame: &nf_view::gpu::HudFrame) -> Option<(f32, f32, f32, f32, [f32; 4])> {
    // The sheet texture index+1 in the runs.
    let ti = scene.hud_textures.iter().position(|(k, _)| *k == 0x0300_007a)? as u32 + 1;
    quads_px(scene, frame).into_iter().find(|(_, t, x, y, w, h, _)| *t == ti && (*x - 20.0).abs() < 2.0 && (*y - 341.0).abs() < 2.0 && (*w - 87.0).abs() < 2.0 && (*h - 86.0).abs() < 2.0).map(|(_, _, x, y, w, h, c)| (x, y, w, h, c))
}

/// 07000005 at the grapple spot: guards in the yard around it [D the level's placements]. The game
/// shows the health ring bottom-left and NO radar blips (the radar pane does not exist in SP —
/// PaneList 13/14 NULL [L hud-panes.md; L the SP snapshot's pane array]). The ring quad must be the
/// health sheet's ring at the game's numbers, and neither radar texture may appear.
#[test]
fn the_sp_hud_on_05_shows_the_health_ring_and_no_radar() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso, "07000005.bin");
    let frame = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    // The radar textures are not even loaded into the SP overlay's texture set: the radar pane
    // never existed here [L the SP pane array; M 0x199e88].
    assert!(scene.hud_textures.iter().all(|(k, _)| *k != 0x0300_0088 && *k != 0x0300_016f), "a radar texture is loaded in SP");
    let ring = find_ring(&scene, &frame).expect("the health ring quad");
    let (x, y, w, h, c) = ring;
    assert!((x - 20.0).abs() < 0.01 && (y - 341.0).abs() < 0.01, "ring at ({x},{y}), the game puts it at (20,341) [L HealthSprInfo[15] + pane (16,337)]");
    assert!((w - 87.0).abs() < 0.01 && (h - 86.0).abs() < 0.01, "ring {w}×{h}, the game's is 87×86 [L]");
    // The health colour 0x52a88b on the GS scale (0x52/0x80, 0xa8/0x80, 0x8b/0x80) and the ring's
    // alpha 140 → drawn 140>>1 = 70 → 70/128 [M 0x19b9c8:205-207].
    assert!((c[0] - 0x52 as f32 / 128.0).abs() < 0.02 && (c[1] - 0xa8 as f32 / 128.0).abs() < 0.02 && (c[2] - 0x8b as f32 / 128.0).abs() < 0.02, "ring colour {c:?}, the health tint 0x52a88b on the GS /128 scale [M 0x19b9c8:169-176]");
    let alpha = (c[3] * 128.0 * 2.0).round() as i32;
    assert!((alpha - 140).abs() <= 2, "ring alpha {alpha}, the game writes 140 [M 0x19b9c8:206]");
}

/// Sister on 07000006 (the second story level, guards at the gatehouse): same SP truth — no radar,
/// the health ring at the game's numbers.
#[test]
fn the_sp_hud_on_06_shows_the_health_ring_and_no_radar() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load(&iso, "07000006.bin");
    let frame = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert!(scene.hud_textures.iter().all(|(k, _)| *k != 0x0300_0088 && *k != 0x0300_016f), "a radar texture is loaded in SP");
    assert!(find_ring(&scene, &frame).is_some(), "the health ring is missing on 06");
}

/// The radar projection's numbers, hand-derived from `HUD_RadarUpdate` 0x19f6e8 (disasm-checked):
/// view = R·(p − cam); dir = the UNIT view vector, x and z negated [M disasm 0x19f888-0x19f8b0];
/// scale = min(len·0.02, 1) [M :122-125]; x = dir.x·scale·(ring_w·0.45) + ring.x + 0.5 + ring_w/2,
/// y = dir.z·scale·(ring_h·0.45) + ring.y + 0.5 + ring_h/2 [M :130-136]; the height cue
/// (dy·0.125 clamp ±1) is a DEAD STORE [M disasm 0x19f88c-0x19f8d4 → 0x319574, no reader].
#[test]
fn the_radar_projection_matches_the_game() {
    let rot = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let cam = [10.0, 20.0, 30.0];
    let ring = (100.0, 50.0, 64.0, 64.0); // the ring sprite's runtime rect (x, y, w, h)
    let objs = [
        // 25 units along −z from the camera, level with Bond: len 25 → scale 0.5; dir = (0,0,−1),
        // negated z → +1 → the blip sits BELOW the centre in the game's signs.
        RadarObj { pos: [10.0, 20.0, 5.0], argb: 0x7f7f7fff, kind: 0 },
        // 60 units along +z — past the 50-unit range clamp: scale 1; the negated unit z −1 puts
        // it ABOVE the centre in the game's signs (the opposite side of blip 0).
        RadarObj { pos: [10.0, 20.0, 90.0], argb: 0xd22d35ff, kind: 0 },
        // 8 units above Bond — the height cue (1.0) is computed and dropped: the SAME y as a blip
        // level with Bond at the same horizontal offset.
        RadarObj { pos: [10.0, 28.0, 5.0], argb: 0x7f7f7fff, kind: 2 },
    ];
    let blips = radar_project(rot, cam, 20.0, ring, &objs);
    assert_eq!(blips.len(), 3);
    let cx = ring.0 + 0.5 + ring.2 * 0.5; // 132.5
    let cy = ring.1 + 0.5 + ring.3 * 0.5; // 82.5
    // Blip 0: dir (0,0,−1) → (−dir.x, −dir.z) = (0, 1); scale 0.5; the half-extent 64·0.45 = 28.8.
    assert!((blips[0].x - cx).abs() < 1e-4, "blip 0 x {}, the game's formula gives the centre {cx}", blips[0].x);
    assert!((blips[0].y - (cy + 0.5 * 28.8)).abs() < 1e-4, "blip 0 y {}, the game's formula gives {} [M 0x19f6e8:133-136]", blips[0].y, cy + 14.4);
    // Blip 1: dir (0,0,1) → negated z −1; scale clamped 1 (len 60 · 0.02 = 1.2 > 1).
    assert!((blips[1].y - (cy - 28.8)).abs() < 1e-4, "blip 1 y {}, the 50-unit clamp pins it at the ring's edge [M 0x19f6e8:122-125]", blips[1].y);
    assert!((blips[1].x - cx).abs() < 1e-4);
    // Blip 2: the same horizontal offset as blip 0 but 8 units UP — the dead height cue must not
    // move it: identical y to blip 0.
    assert!((blips[2].y - blips[0].y).abs() < 1e-4, "blip 2 y {} — the height cue is a dead store in the retail code [M disasm 0x19f88c-0x19f8d4]", blips[2].y);
    // The blip sizes/uv from TexUV.291 [L 0x2becf0]: the players 4×4, an uplink 8×8; layer 0x1c.
    assert_eq!((blips[0].w, blips[0].h), (4.0, 4.0));
    assert_eq!((blips[2].w, blips[2].h), (8.0, 8.0));
    assert!(blips.iter().all(|b| b.layer == 0x1c), "the blip layer [M 0x19f6e8:152]");
    assert_eq!(blips[1].argb, 0xd22d35ff, "the blip colour rides the RObj [M 0x189840:58-63]");
}

/// Sisters: the ring's layer gate and the HUD_FixMP clamp — the created (512,0) ring lands at the
/// viewer's top-right corner [M 0x19f6e8:102-105; M 0x19b110:73-92; L viewer+0xf0..fc = 0,512,0,448].
#[test]
fn the_ring_layer_gate_and_the_fixmp_clamp() {
    assert_eq!(radar_ring_layer(false), 0xff, "radar off → the ring hidden [M 0x19f6e8:102-105]");
    assert_eq!(radar_ring_layer(true), 0x1e);
    // A 100×100 ring texture on the full-screen viewer: x = max(512,0) = 512 → 512−100; y stays 0.
    let (x, y, w, h) = radar_ring_rect(100.0, 100.0, (0.0, 512.0, 0.0, 448.0));
    assert!((x - 412.0).abs() < 1e-4 && y == 0.0 && w == 100.0 && h == 100.0, "the ring clamps to ({x},{y}) — the viewer's top-right [M 0x19b110:73-92]");
    // A split-screen right-half viewer (left 256): the ring clamps inside THAT rect.
    let (x, y, _, _) = radar_ring_rect(100.0, 100.0, (256.0, 512.0, 0.0, 224.0));
    assert!((x - 412.0).abs() < 1e-4, "split-screen: the same clamp inside the half [M 0x19b110:47-71]");
}
