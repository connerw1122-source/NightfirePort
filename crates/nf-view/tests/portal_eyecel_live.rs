//! fix-portal-eyecel: the trench eye cel must resolve like the game's, and the near walls must
//! draw on the RENDERED path.
//!
//! The playtest spot `07000005 --bond 16.67,30.04,58.60,1.755,-0.001` culled the near trench
//! walls: the eye cel resolved to the giant outdoor room (the port's cell 14) instead of the
//! trench room (cell 15), so `View_AddCels` (0x1e6ba0) never visited the rooms whose `+0x18`
//! chains hold the trench generics and the walls vanished (sky through stone). The game's eye
//! cel is 15 there because `build_FindCel` (0x1c8cb0)'s tie-break casts its up/down rays through
//! `Collide_RayIntersect(from, to, CANDIDATE, 0, 0, …, 1, 0x90)` — the owner filter (arg7 = 1)
//! exact-tests only hits whose static-geometry cel (`HITDATA+0x54`) is the passed candidate, so
//! each ray sees the candidate's OWN chain geometry. [L ee_start_05.bin]: every trench generic
//! (58 nodes) rides the trench room's chain (game room 17 = the port's cell 15); the outdoor
//! room's chain (41 nodes, game room 18 = the port's cell 14) holds nothing near the trench.
//!
//! The port's stand-in scoped the SHARED collision world (straddle buckets), so the tie-break
//! rays saw nothing of the trench through either candidate and the FIRST candidate (14) stood.
//! The fix: the collision scope's per-cell buckets are re-keyed by the same chain owners
//! `build_chains` already computes (`build_link_objects_to_rooms` 0x1c9150's collision side),
//! and the tie-break queries the candidate's own bucket only (`CollisionWorld::segment_hit_cell`).
//!
//! This test drives the real draw-list path (`gpu::Renderer::cel_visibility`'s inputs:
//! `scene::eye_cell` + `portal_visible_cels_game_from_cell`) at the trench spot and asserts the
//! near stone-wall instances the frustum keeps are kept by the walk mask — it FAILS on master
//! (eye cell 14, the wall nodes dropped). Opt-in via NF_ISO.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near_p, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near_p - far), -1.0],
        [0.0, 0.0, near_p * far / (near_p - far), 0.0],
    ]);
    let cp = pitch.cos();
    let fw = Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
    let s = fw.cross(Vec3::Y).normalize();
    let u = s.cross(fw);
    let view = Mat4::from_cols_array_2d(&[
        [s.x, u.x, -fw.x, 0.0],
        [s.y, u.y, -fw.y, 0.0],
        [s.z, u.z, -fw.z, 0.0],
        [-s.dot(Vec3::from(pos)), -u.dot(Vec3::from(pos)), fw.dot(Vec3::from(pos)), 1.0],
    ]);
    (proj * view).to_cols_array_2d()
}

/// The playtest's trench spot (the probe convention: bond y + 0.7 is the camera head, the pitch
/// is the game's half-radian word — the render path multiplies by π/2).
const TRENCH: ([f32; 3], f32, f32) = ([16.67, 30.04 + 0.7, 58.60], 1.755, -0.001 * std::f32::consts::FRAC_PI_2);

/// The trench eye cel is 15 (the game's room 17) — `build_FindCel` 0x1c8cb0's tie-break over the
/// per-cel collision sets — and the near trench-wall generics the frustum keeps with clear LOS
/// are kept by the walk mask the renderer gates World draws with (the exact `--shot` draw list
/// input), WITHOUT the fix-portal-sweep rescue.
#[test]
fn trench_eye_cel_is_15_and_the_near_walls_draw() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000005.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000005: {e}"); return } };
    let g = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world);
    let (eye, yaw, pitch) = TRENCH;
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = [planes[0], planes[1], planes[2], planes[3], {
        let fw = [planes[4][0], planes[4][1], planes[4][2]];
        let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
        [-fw[0], -fw[1], -fw[2], fe + 200.0]
    }];

    // The eye cel: the game's FindCel resolves the trench room (the port's cell 15), not the
    // giant outdoor room (14) the shared-world stand-in picked.
    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(g, &mut state, eye, world).expect("the eye cel resolves");
    eprintln!("[eyecel] trench eye cell {cell}");
    assert_eq!(cell, 15, "the trench eye cel must be the trench room (cell 15), got {cell}");

    // The walk from the real eye cel keeps the trench rooms.
    let game = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("the game mask");
    assert!(game[15] && game[24], "the walk from cell 15 must visit the trench rooms 15 and 24");

    // The RENDERED path's World-draw verdict for the near trench-wall generics: every node-keyed
    // draw within 45 units of the eye that the frustum keeps and whose LOS to the eye is clear
    // and whose node rides a chain the walk visits must be KEPT by the mask. On master the eye
    // cel 14 never visits rooms 15/24, so the trench walls (the 0x8019 nodes of room 15's chain
    // around the eye) are all dropped — this is the playtest's see-through-stone.
    let mats: Vec<&scene::Material> = sc.materials.iter().filter(|m| m.layer == scene::Layer::World).collect();
    let d2 = |c: [f32; 3]| (c[0] - eye[0]).powi(2) + (c[1] - eye[1]).powi(2) + (c[2] - eye[2]).powi(2);
    let hidden: Vec<(&scene::Material, f32)> = mats.iter().filter_map(|m| {
        if m.node == u32::MAX { return None; }                       // node-keyed generics only
        let d = d2(m.centre).sqrt();
        if d > 45.0 { return None; }
        // The frustum keeps it (`Vision_InView` 0x1e89e0's signed-distance form).
        if !sides.iter().all(|p| p[0] * m.centre[0] + p[1] * m.centre[1] + p[2] * m.centre[2] + p[3] + m.radius >= 0.0) { return None; }
        // Clear static LOS from the eye to the draw's centre (the walk's own criterion).
        let w = world?;
        if w.segment_hit_static(eye, m.centre).is_some() { return None; }
        // The node rides a chain the walk visits (the game's View_AddCels keeps it then).
        let owner = g.chains.iter().enumerate().find(|(_, ch)| ch.contains(&m.node)).map(|(o, _)| o);
        let Some(o) = owner else { return None };
        if !game[o] { return None; }
        let kept = !game[m.node as usize] == false; // the mask's node verdict: hidden iff !game[node]
        if kept { None } else { Some((*m, d)) }
    }).collect();
    eprintln!("[walls] {} near visited-chain wall draws hidden by the mask", hidden.len());
    for (m, d) in hidden.iter().take(8) {
        eprintln!("[hidden] node {} centre ({:.1},{:.1},{:.1}) r {:.1} d {:.1}", m.node, m.centre[0], m.centre[1], m.centre[2], m.radius, d);
    }
    assert!(hidden.is_empty(), "{} near trench-wall generics (visited chains, clear LOS) are culled at the trench spot", hidden.len());
}
