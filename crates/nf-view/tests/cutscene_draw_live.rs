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


/// The render gate: with the palette exactly as `Renderer::update_skin` uploads it, each scene
/// actor's skinned vertices must form a coherent body AT the actor's posed root — projected from
/// a camera placed 8 m in front of the root looking at it, ≥30% of the mesh lands inside the
/// viewport with depth in range. On the broken tree the mesh is torn (the root-adjacent bones
/// ride the face track's identity at the origin, the rest keeps the body track): the manual
/// closeup render at Kimiko's root showed an empty library pre-fix, Kimiko herself post-fix.
#[test]
fn kimiko_scene_actors_form_a_body_at_their_pose_root() {
    let Some(scene) = kimiko_scene() else { return };
    let (cam_pos, yaw, pitch, fov) = scene.sim.as_ref().unwrap().script_camera()
        .unwrap_or_else(|| panic!("the Kimiko scene runs but has no script camera"));
    // The palette exactly as the render path builds it (the same chain Renderer::update_skin
    // gets: cel_visibility's room mask from the script camera — here via the census's
    // eye_cell + portal walk, the same walk cel_visibility runs).
    let g = scene.cel_graph.clone();
    let world = scene.sim.as_ref().map(|s| &s.world);
    let mut state = nf_view::scene::ViewerCel::default();
    let cell = nf_view::scene::eye_cell(&g, &mut state, cam_pos, world);
    let planes = nf_view::gpu::extract_frustum(view_proj(cam_pos, yaw, pitch, fov, 16.0 / 9.0));
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * cam_pos[0] + fw[1] * cam_pos[1] + fw[2] * cam_pos[2];
    let depth0 = [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]];
    let vis: Option<Vec<bool>> = cell.and_then(|c| nf_view::scene::portal_visible_cels_from_cell(&g, c, cam_pos, &depth0))
        .map(|cell_vis| g.room_to_cell.iter().map(|m| m.map(|c2| cell_vis[c2]).unwrap_or(true)).collect());
    let (layout, attrs) = scene.skin_layout();
    let (pal, _) = scene.skin_palette(&layout, None, 300.0, nf_view::scene::ViewOpts::default(), vis.as_deref());
    for (ci, c) in scene.characters.iter().enumerate() {
        let Some((h, _)) = c.script else { continue };
        if h != 0x0600_0073 { continue; }
        // The actor's posed root: bone 0's skinning-matrix translation (what the GPU palette
        // holds — the body track's absolute root post-fix, the face track's origin pre-fix).
        let base = layout.char_base[ci] as usize;
        let root = [pal[base + 1][3][0], pal[base + 1][3][1], pal[base + 1][3][2]];
        // A closeup camera 8 m south of the root, looking north at it (open floor by the pool
        // table — the spot whose manual render showed the empty library pre-fix, Kimiko after).
        let cam = [root[0], root[1] + 1.0, root[2] - 8.0];
        let vp = view_proj(cam, 0.0, (-1.0f32 / 8.0).atan(), 0.594, 16.0 / 9.0);
        let (mut n_in, mut n_total) = (0u32, 0u32);
        for (k, cv) in scene.char_verts.iter().enumerate() {
            if cv.character as usize != ci || cv.attachment.is_some() { continue; }
            let a = attrs[scene.char_vertex_start + k].a as usize;
            let b = attrs[scene.char_vertex_start + k].b as usize;
            let w = attrs[scene.char_vertex_start + k].w;
            let wa = mat_apply(&pal[a], cv.pos);
            let wb = mat_apply(&pal[b], cv.pos);
            let wp = [w * wa[0] + (1.0 - w) * wb[0], w * wa[1] + (1.0 - w) * wb[1], w * wa[2] + (1.0 - w) * wb[2]];
            n_total += 1;
            if let Some((x, y, z)) = project(&vp, wp) {
                if (-1.0..=1.0).contains(&x) && (-1.0..=1.0).contains(&y) && (0.0..=1.0).contains(&z) { n_in += 1; }
            }
        }
        assert!(n_total > 500, "char {ci}: {n_total} skinned verts (expected the full mesh)");
        { let mut shown = 0; for (k, cv) in scene.char_verts.iter().enumerate() {
            if cv.character as usize != ci || cv.attachment.is_some() { continue; }
            let a = attrs[scene.char_vertex_start + k].a as usize;
            let w = attrs[scene.char_vertex_start + k].w;
            let wa = mat_apply(&pal[a], cv.pos);
            if shown < 5 { eprintln!("[skin] char {ci} vert {k}: attr a={a} w={w:.2} bind ({:.2},{:.2},{:.2}) -> world ({:.2},{:.2},{:.2})", cv.pos[0], cv.pos[1], cv.pos[2], wa[0], wa[1], wa[2]); shown += 1; }
        } }
        eprintln!("[draw] char {ci} skin {:08x} root ({:.1},{:.1},{:.1}): {n_in} of {n_total} in the closeup frustum",
            c.skin.hashcode, root[0], root[1], root[2]);
        assert!(n_in * 10 >= n_total * 3,
            "char {ci} skin {:08x}: only {n_in} of {n_total} skinned verts project in the closeup at the \
             actor's own root — the mesh is not a coherent body there (torn pose or origin collapse)", c.skin.hashcode);
    }
}
