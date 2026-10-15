//! fix-08-walls (101h): two 07000008 symptoms from the 1013-1758 playtest at the safe/briefcase
//! area (~(155, 43, -113)).
//!
//! (1) "I walked into a wall and it threw me onto a zip line on the other side of the wall" — the
//! jump-grab's arm test was a flat 12.0-unit radius around each third icon [M `Player_HandleJump`
//! 0x1916b0:118-147 reads `BLData+0x95F`, written by `ThirdIcon_Update` 0x196e80 off the icon's
//! obj+0xd0 chain], so jumping while hugging the guardhouse wall armed 07000008's type-3
//! "Hand_Over_Hand" icon (1.9 units away on the far side) and `Player_CollWire` 0x197130 yanked
//! Bond onto the zipline across the wall (its nearest-wire-within-1000 scan has no LOS of its own
//! [M 0x1916b0:120-144 — the decompile's scan loop is complete: class ',' + `Vec_Dist3D` only]).
//! The game's arm region is the icon's straddle-cel set [M `Controls_StraddleTest` 0x134778 →
//! `Collide_StraddleCels` 0x1eac98 → `Intersect_Portal` 0x1ea7b8 v0x101 → the chain nodes
//! `Collide_Update` 0x1ea398:160-233]: the icon's world sphere (r 1.164 [D probe]) BFS-walks
//! portals from its own cel, so a wall with no portal keeps the contexts apart. Fixed in
//! scene.rs (`icon_cels` / `icon_straddle_cels` + the FindCel arming test) and wire.rs
//! (`Icon::centre`/`radius`).
//!
//! (2) "the big wall is still clipping into the missile room" — `Room_Outside_3` (the castle's east
//! exterior cel, graph cell 15) carries the exterior wall slabs whose triangles cross the whole
//! `Room_Guardhouse_F2` interior (mats with tris-box x 155.0..166.5 over the room's x 154.5..163.1
//! [D per-tri probe]; fix-safe08.md measured the protrusion). The game hides the exterior cel
//! indoors because `View_AddCels` 0x1e6ba0 draws only the `Vision_Portal_Recurse` 0x1e86e8 set —
//! this file pins the port's game-mask (`portal_visible_cels_game_from_cell`, the fix-portal2
//! port) to the same exclusion at the user's spot. The 1013-1758 playtest build predates the
//! portal-sweep merge (fix-portal-sweep landed 2026-10-14 01:10 UTC, the playtest is 17:58 UTC
//! [git log]); at master the mask already excludes cell 15 from the missile room at every sampled
//! view — pinned here as the regression guard the task asks for, with live checks listed in the
//! report for the main agent's render.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::{Input, SS_WALK, SS_ZIPLINE};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_08(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, "07000008.bin", false).expect("load 07000008");
    let idle = Input::default();
    for _ in 0..600 { if !scene.sim.as_ref().unwrap().disabled { break; } scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(idle, false, false, false); }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the intro never released Bond");
    scene
}

/// The icon's world sphere is the object's glist bounds sphere, not a made-up scope [M
/// `Control_BuildWorldSph` 0x133070 via `build_LinkToRoom` 0x1c9680:40], and its straddle set on
/// 07000008 is exactly the west room's cel (the r-1.164 sphere crosses no portal).
#[test]
fn the_wire_icons_straddle_set_is_its_own_west_room_cel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load_08(&iso);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.icons.len(), 1, "07000008 has exactly one third icon [D probe]");
    let icon = &sim.icons[0];
    assert_eq!(icon.ctx, 3, "the Hand_Over_Hand icon is the cable context [D param 0]");
    assert!((icon.radius - 1.164).abs() < 0.05, "the icon's world sphere is its object bounds (r 1.164 [D probe]), got {}", icon.radius);
    assert_eq!(icon.centre, icon.pos, "the icon object's bounds centre is the placement origin [D probe]");
    assert_eq!(sim.icon_cels[0], vec![12u32 as usize].iter().map(|&v| v).collect::<Vec<_>>(),
        "the r-1.164 sphere crosses no portal: the straddle set is the icon's own cel (the west room, graph cell 12)");
    // Sisters: the FindCel cel of the two playtest sides.
    let east = nf_view::scene::find_cell_raycast(&sim.cel_graph, [144.0, 45.0, -104.0], Some(&sim.world));
    let west = nf_view::scene::find_cell_raycast(&sim.cel_graph, [132.0, 46.7, -104.0], Some(&sim.world));
    assert_eq!(east, Some(43usize), "the guardhouse-balcony spot finds the east junction room (cell 43)");
    assert_eq!(west, Some(12usize), "the west ledge spot finds the icon's own room (cell 12)");
    assert!(!sim.icon_cels[0].contains(&east.expect("east resolves")), "the wall side must not arm [M 0x1eac98]");
    assert!(sim.icon_cels[0].contains(&west.expect("west resolves")), "the icon's side arms [M 0x1eac98]");
}

/// Fail-first repro: standing on the EAST balcony of the guardhouse (the safe/briefcase area side),
/// within the old 12.0 arm radius of the far-side icon (11.1 units) but across the wall, jumping
/// must do a plain jump — not grab the zipline that hangs on the west side.
#[test]
fn the_guardhouse_wall_stops_the_wire_grab() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_08(&iso);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([144.0, 45.0, -104.0], 0.0);
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 0xff,
        "the wall side reads no icon [M ThirdIcon_Update 0x196e80 walks the straddle chain only] (got {})", sim.bond.icon_ctx);
    for t in 0..8 { sim.step_bond(Input { jump: true, ..Input::default() }, false, false, false); }
    assert_eq!(sim.bond.substate, SS_WALK,
        "jumping against the guardhouse wall must not grab the far-side zipline [M 0x1916b0 reads +0x95F == 3 only; L the 1013-1758 playtest] (substate {})", sim.bond.substate);
    assert!(sim.bond.wire.is_none(), "no wire was grabbed");
    assert!(sim.bond.pos[0] > 140.0, "Bond stayed on the east side (got {:?})", sim.bond.pos);
}

/// Sister: on the icon's OWN side (the west ledge under the zipline's high anchor) the jump-grab
/// still fires — the fix narrows the arm region to the game's straddle set without killing the
/// intended cable crossing.
#[test]
fn the_hand_over_hand_grab_still_fires_on_the_icons_side() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_08(&iso);
    let sim = scene.sim.as_mut().unwrap();
    // fix-wallhug-prompt: the spot stays the [D probe spot] — 3.47 (sphere centre) from the icon
    // (133.43, 48.33, −101.21), inside the PICK-SPHERE shell 2.5 + 1.1638 = 3.66 [M Collide_Pick
    // 0x1eb320 LAB_001eb4e4 — the picker radius is the hittest's own +0x8C, a bigger pick sphere
    // than the object's world sphere 1.0576 [L M_CONTROL+0x8C]; the pins bracket it 2.31..5.9,
    // see scene.rs]. The old [S] 12.0/6.0 window masked the pick sphere entirely.
    sim.bond = nf_game::player::Bond::new([132.0, 46.7, -104.0], 0.0);
    for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.bond.icon_ctx, 3, "the west ledge reads the cable icon [D probe spot; the pick-sphere shell 2.5 + 1.1638 = 3.66 covers the 3.47 spot]");
    for t in 0..8 { sim.step_bond(Input { jump: true, ..Input::default() }, false, false, false); }
    assert_eq!(sim.bond.substate, SS_ZIPLINE, "the intended zipline grab fires on the icon's side (substate {})", sim.bond.substate);
    assert_eq!(sim.bond.wire, Some(0), "the level's only wire is grabbed");
}

/// The missile room's visible-instance set excludes the exterior wall: from the user's spot inside
/// `Room_Guardhouse_F2` (graph cell 48), the game-mask walk (`Vision_Portal_Recurse` 0x1e86e8 +
/// the `View_AddCels` 0x1e6ba0 chain pass, the fix-portal2 port) never marks the east exterior
/// room (cell 15, `Room_Outside_3` — the slabs whose tris cross the room interior [D]) visible,
/// under the port's own camera (fov pi/3, the lib.rs view_proj) at any sampled yaw, 4:3 or 16:9.
#[test]
fn the_missile_room_visible_set_excludes_the_exterior_wall() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load_08(&iso);
    let g = &scene.cel_graph;
    let world = scene.sim.as_ref().map(|s| &s.world);
    let eye = [155.4f32, 44.0, -113.6];
    let mut state = nf_view::scene::ViewerCel::default();
    let start = nf_view::scene::eye_cell(g, &mut state, eye, world);
    assert_eq!(start, Some(48usize), "the eye resolves to the missile room's own cel");
    let start = start.unwrap();
    for &aspect in [16.0f32 / 9.0, 4.0 / 3.0].iter() {
        for step in 0..24 {
            let yaw = step as f32 * std::f32::consts::PI / 12.0;
            let planes = frustum(eye, yaw, 0.0, aspect);
            let depth0 = [planes[0], planes[1], planes[2], planes[3], planes[4]];
            let sides = [planes[0], planes[1], planes[2], planes[3]];
            let mut cv = nf_view::scene::portal_visible_cels_game_from_cell(g, start, eye, &depth0, &sides)
                .expect("the walk runs from the missile room");
            if let Some(w) = world { nf_view::scene::near_los_rescue(g, &mut cv, eye, &sides, w, 30.0); }
            assert!(!cv[15], "yaw {yaw}: the east exterior room (cell 15, Room_Outside_3) stays hidden [M View_AddCels 0x1e6ba0 draws only the recurse set]");
            assert!(cv[48], "yaw {yaw}: the missile room itself stays visible (sister)");
        }
    }
    // Sister at the object level: every World material riding node 15 is culled under the yaw-0
    // mask, while the room's own furniture generics (nodes 98..116) draw — the exclusion is the
    // exterior wall, not a blanket cull.
    let planes = frustum(eye, 0.0, 0.0, 16.0 / 9.0);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = [planes[0], planes[1], planes[2], planes[3], planes[4]];
    let mut cv = nf_view::scene::portal_visible_cels_game_from_cell(g, start, eye, &depth0, &sides).unwrap();
    if let Some(w) = world { nf_view::scene::near_los_rescue(g, &mut cv, eye, &sides, w, 30.0); }
    let mut node15 = 0usize; let mut furniture = 0usize;
    for m in &scene.materials {
        if m.layer != nf_view::scene::Layer::World { continue; }
        if m.node == 15 { node15 += 1; }
        if (98..=116).contains(&m.node) { furniture += 1; }
    }
    assert!(node15 > 0, "the exterior room's World materials exist (the probe found the wall slabs)");
    assert!(furniture > 0, "the guardhouse furniture generics exist");
    for m in scene.materials.iter().filter(|m| m.layer == nf_view::scene::Layer::World && m.node == 15) {
        assert!(cv[15] == false, "the exterior material at {:?} is culled", m.centre);
    }
}

/// The port's camera frustum (lib.rs `Camera::view_proj` + gpu.rs `extract_frustum`), verbatim math.
fn frustum(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 6] {
    let fov = std::f32::consts::FRAC_PI_3;
    let (near, far, f) = (0.05f32, 4000.0f32, 1.0 / (fov * 0.5).tan());
    let (sp, cp) = pitch.sin_cos();
    let fw = [yaw.sin() * cp, sp, yaw.cos() * cp];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let norm = |a: [f32; 3]| { let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt(); [a[0] / l, a[1] / l, a[2] / l] };
    let s = norm(cross(fw, [0.0, 1.0, 0.0]));
    let u = cross(s, fw);
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let view = [
        [s[0], u[0], -fw[0], 0.0],
        [s[1], u[1], -fw[1], 0.0],
        [s[2], u[2], -fw[2], 0.0],
        [-dot(s, pos), -dot(u, pos), dot(fw, pos), 1.0],
    ];
    let proj = [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
    ];
    let mut m = [[0.0f32; 4]; 4];
    for r in 0..4 { for c in 0..4 { m[r][c] = (0..4).map(|k| proj[r][k] * view[k][c]).sum(); } }
    let row = |i: usize| [m[0][i], m[1][i], m[2][i], m[3][i]];
    let add = |a: [f32; 4], b: [f32; 4]| [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]];
    let sub = |a: [f32; 4], b: [f32; 4]| [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]];
    let nrm = |p: [f32; 4]| { let l = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt(); [p[0] / l, p[1] / l, p[2] / l, p[3] / l] };
    let mut pl = [row(3); 6];
    pl[0] = nrm(add(row(3), row(0)));
    pl[1] = nrm(sub(row(3), row(0)));
    pl[2] = nrm(add(row(3), row(1)));
    pl[3] = nrm(sub(row(3), row(1)));
    pl[4] = nrm(row(2));
    pl[5] = nrm(add(row(3), [0.0, 0.0, 1.0, 0.0]));
    pl
}
