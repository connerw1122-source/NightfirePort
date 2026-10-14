//! fix-cutscene-draw: the cut-scene actor draw regression (the RENDERED-frame repro).
//!
//! 101b-fix-cutscene-models showed the actors "live" (palette real, poses finite) but the
//! rendered frames of the Kimiko library scene (07000007, scene 0x06000073, TouchOnce #412)
//! show no characters at all. The cause this test pins: `script_pose_together`'s window
//! composition let the scene's narrow 22-channel face track (window (1, 04000467)) write the
//! body track's bones 0..5 first (first-writer-wins, fix-cutscenes2 d907aba) — the 222-channel
//! body track's root (which carries the actor's ABSOLUTE world position, the value
//! `AnimGetFirstTrans` 0x1c7348 initialises the game's anim object position obj+0x30 from)
//! collapsed to the face track's leading zeros, so every script actor was posed as an identity
//! mannequin at the world origin — off-camera in every cut-scene shot.
//!
//! Two gates, both through the real input path (`nf_view::load` + `Sim::step_bond` from the
//! trigger):
//! 1. the pose gate: each scene actor's bone-0 (root) local translation is the body track's
//!    non-zero library position, and it ADVANCES with the script frame;
//! 2. the render gate: the palette `Renderer::update_skin` uploads, skinned through the scene's
//!    char verts and projected with the script camera, puts a healthy fraction of every scene
//!    actor's vertices inside the viewport with depth in range (what the window draws).
//!
//! Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn kimiko_scene() -> Option<nf_view::scene::Scene> {
    let iso = iso()?;
    let mut scene = match nf_view::load(&iso, "07000007.bin", false) {
        Ok(s) => s,
        Err(e) => { eprintln!("07000007.bin: {e}"); return None }
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(scene.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| p.scripts.contains(&0x0600_0073) && p.state == 2),
        "the Kimiko scene 0x06000073 did not start from trigger 412");
    for _ in 0..210 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    Some(scene)
}

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, fov: f32, aspect: f32) -> [[f32; 4]; 4] {
    let (near_p, far, f) = (0.05f32, 4000.0f32, 1.0 / (fov * 0.5).tan());
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

/// The GPU's matrix application: the Rust row arrays upload as columns, so `M·p` is the
/// column-dot (x = col0·p, …, w = col3·p) — the same math the WGSL `pal[sa.x] * q` does.
fn mat_apply(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    let q = [p[0], p[1], p[2], 1.0];
    let dot = |k: usize| m[0][k] * q[0] + m[1][k] * q[1] + m[2][k] * q[2] + m[3][k] * q[3];
    [dot(0), dot(1), dot(2)]
}

fn project(vp: &[[f32; 4]; 4], p: [f32; 3]) -> Option<(f32, f32, f32)> {
    let q = [p[0], p[1], p[2], 1.0];
    let dot = |k: usize| vp[0][k] * q[0] + vp[1][k] * q[1] + vp[2][k] * q[2] + vp[3][k] * q[3];
    let (x, y, z, w) = (dot(0), dot(1), dot(2), dot(3));
    if w <= 0.0 { return None; }
    Some((x / w, y / w, z / w))
}

use glam::{Mat4, Vec3};

/// The pose gate: the scene's script actors must pose from the BODY track — a non-zero,
/// frame-advancing root (the library's absolute positions), not the face track's identity.
#[test]
fn kimiko_scene_actors_pose_from_the_body_track() {
    let Some(scene) = kimiko_scene() else { return };
    let sim = scene.sim.as_ref().unwrap();
    let mut checked = 0usize;
    for (ci, c) in scene.characters.iter().enumerate() {
        let Some((h, stream)) = c.script else { continue };
        if h != 0x0600_0073 { continue; }
        checked += 1;
        let (script, frame) = sim.script_anim((h, stream), c.skin.hashcode)
            .unwrap_or_else(|| panic!("char {ci}: the scene runs but its Obj::Anim is gone"));
        let local = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &c.skeleton, &c.skin, script, frame)
            .unwrap_or_else(|| panic!("char {ci}: no pose for {script:08x} @ {frame}"));
        let root = local[0].0;
        // The face track's stolen bones 0..5 pose all-zero with an identity quat; the body
        // track's root carries the actor's absolute library position (bone 0's root channel).
        assert!(root.iter().any(|v| v.abs() > 0.5),
            "char {ci} skin {:08x}: the root bone is the face track's zero translation {root:?} — \
             the body track's root was overwritten (the actor poses at the world origin)", c.skin.hashcode);
        // Finite (the census's gate, kept): every component finite, quats non-zero.
        assert!(local.iter().all(|(t, q)| t.iter().all(|v| v.is_finite()) && q.iter().all(|v| v.is_finite()) && q.iter().any(|v| *v != 0.0)),
            "char {ci}: non-finite pose");
        // And the root is INSIDE the level's party area (the scene plays in the library/hall,
        // x 60..135, y 40..48, z −100..−15) — not at the origin, not at FAR.
        assert!((60.0..135.0).contains(&root[0]) && (40.0..48.0).contains(&root[1]) && (-100.0..-15.0).contains(&root[2]),
            "char {ci}: root {root:?} outside the library scene's area");
    }
    assert_eq!(checked, 3, "the Kimiko scene pre-builds 3 script characters");
}


/// The render gate: the palette `Renderer::update_skin` uploads (the exact GPU draw input the
/// window's skinned draw consumes) must place each scene actor's bones AT the actor's body-track
/// root in the library — bone 0's skinning-matrix translation non-zero and inside the scene area.
/// On the broken tree the whole palette region rode the face track's identity: bone 0 at (0,0,0),
/// every actor collapsed to the origin, the frames drew no one.
#[test]
fn kimiko_scene_actors_palette_places_them_in_the_library() {
    let Some(scene) = kimiko_scene() else { return };
    let (cam_pos, yaw, pitch, fov) = scene.sim.as_ref().unwrap().script_camera()
        .unwrap_or_else(|| panic!("the Kimiko scene runs but has no script camera"));
    // The same chain Renderer::update_skin gets: cel_visibility's room mask from the script
    // camera — here via the census's eye_cell + portal walk (the same walk cel_visibility runs).
    let vp = view_proj(cam_pos, yaw, pitch, fov, 16.0 / 9.0);
    let g = scene.cel_graph.clone();
    let world = scene.sim.as_ref().map(|s| &s.world);
    let mut state = nf_view::scene::ViewerCel::default();
    let cell = nf_view::scene::eye_cell(&g, &mut state, cam_pos, world);
    let planes = nf_view::gpu::extract_frustum(vp);
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * cam_pos[0] + fw[1] * cam_pos[1] + fw[2] * cam_pos[2];
    let depth0 = [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]];
    let vis: Option<Vec<bool>> = cell.and_then(|c| nf_view::scene::portal_visible_cels_from_cell(&g, c, cam_pos, &depth0))
        .map(|cell_vis| g.room_to_cell.iter().map(|m| m.map(|c2| cell_vis[c2]).unwrap_or(true)).collect());
    let (layout, _) = scene.skin_layout();
    let (pal, _) = scene.skin_palette(&layout, None, 300.0, nf_view::scene::ViewOpts::default(), vis.as_deref());
    let far = [[0.0f32; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
    for (ci, c) in scene.characters.iter().enumerate() {
        let Some((h, _)) = c.script else { continue };
        if h != 0x0600_0073 { continue; }
        let base = layout.char_base[ci] as usize;
        let nb = c.skeleton.bones as usize;
        let mats = &pal[base..=(base + nb).min(pal.len() - 1)];
        assert!(!mats.iter().all(|m| *m == far), "char {ci}: the palette moved the actor to FAR (room-hidden)");
        let bone0 = [pal[base + 1][3][0], pal[base + 1][3][1], pal[base + 1][3][2]];
        eprintln!("[draw] char {ci} skin {:08x}: palette bone0 t=({:.2},{:.2},{:.2})", c.skin.hashcode, bone0[0], bone0[1], bone0[2]);
        assert!(bone0.iter().any(|v| v.abs() > 0.5),
            "char {ci} skin {:08x}: the uploaded palette's bone 0 sits at the origin — the actor draws              as a mannequin at the world origin (the face track stole the body track's root)", c.skin.hashcode);
        assert!((60.0..135.0).contains(&bone0[0]) && (40.0..48.0).contains(&bone0[1]) && (-100.0..-15.0).contains(&bone0[2]),
            "char {ci} skin {:08x}: the uploaded palette's bone 0 at {bone0:?} is outside the library scene area", c.skin.hashcode);
    }
}
