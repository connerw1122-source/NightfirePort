//! fix-cutscene-models: the cut-scene actor draw census (the REGRESSION probe).
//!
//! The playtest (1013-1758): in the cut-scenes only Bond draws — on 07000008 "Kiko is the only
//! one showing besides me", on 07000007 "only my player character". The port draws a script actor
//! only while `Sim::script_anim(src, skin_hash)` resolves a live `Obj::Anim` (scene.rs
//! `frame_state`'s `hidden` collapse; scene.rs:4052) AND its pose
//! (`script_pose_together` → `world_pose` → `skin_matrices`) is finite. This probe runs each
//! scene through its real trigger and dumps the census: per script character — the anim object
//! live? the pose finite? — plus the live Obj::Anim list.
//!
//! Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(archive: &str) -> Option<nf_view::scene::Scene> {
    let iso = iso()?;
    match nf_view::load(&iso, archive, false) {
        Ok(s) => Some(s),
        Err(e) => { eprintln!("{archive}: {e}"); None }
    }
}

fn playing(s: &nf_view::scene::Scene, hash: u32) -> bool {
    s.sim.as_ref().unwrap().drones.scripts.players.iter().any(|p| p.scripts.contains(&hash) && p.state == 2 && p.info[p.cur].as_ref().is_some_and(|i| i.playing()))
}

/// The census over one running scene: every pre-built script character (scene.rs builds one per
/// `Cmd::Anim` stream cmd at load) must (a) have its live `Obj::Anim` (the `hidden` gate) and
/// (b) pose finite matrices. Prints per-character lines and returns (drawn, hidden, nan).
fn census(scene: &nf_view::scene::Scene, tag: &str) -> (usize, usize, usize) {
    let sim = scene.sim.as_ref().unwrap();
    let (mut drawn, mut hidden, mut nan) = (0, 0, 0);
    for (ci, c) in scene.characters.iter().enumerate() {
        let Some(src) = c.script else { continue };
        let Some((script, frame)) = sim.script_anim(src, c.skin.hashcode) else {
            hidden += 1;
            eprintln!("[{tag}] char {ci} src {:?} skin {:08x}: NO live Obj::Anim (frame_state hidden)", src, c.skin.hashcode);
            continue;
        };
        let pose = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &c.skeleton, &c.skin, script, frame);
        let Some(local): Option<nf_game::anim::Pose> = pose else {
            hidden += 1;
            eprintln!("[{tag}] char {ci} src {:?} skin {:08x}: script_pose_together None for {script:08x} @ {frame}", src, c.skin.hashcode);
            continue;
        };
        // The pose is Vec<([f32; 3] translation, [f32; 4] rotation)>: every component must be
        // finite, and the quats non-zero (a zeroed quat poses garbage).
        let finite = local.iter().all(|(t, q)| t.iter().all(|v| v.is_finite()) && q.iter().all(|v| v.is_finite()) && q.iter().any(|v| *v != 0.0));
        if !finite {
            nan += 1;
            eprintln!("[{tag}] char {ci} src {:?} skin {:08x}: NON-FINITE pose {script:08x} @ {frame}", src, c.skin.hashcode);
        } else { drawn += 1; }
    }
    let anims = sim.drones.scripts.objects.list.iter().flatten().filter(|o| matches!(o, nf_game::nis::Obj::Anim { .. })).count();
    let srcs: Vec<String> = sim.drones.scripts.objects.list.iter().flatten().filter_map(|o| match o {
        nf_game::nis::Obj::Anim { src, skin, script, frame, .. } => Some(format!("src {:?} skin {:08x} script {script:08x} @ {frame}", src, skin)),
        _ => None,
    }).collect();
    eprintln!("[{tag}] census: drawn {drawn} hidden {hidden} nan {nan}; live Obj::Anim {anims}: {:?}", srcs);
    (drawn, hidden, nan)
}

/// 07000007's Kimiko scene (0x06000073, TouchOnce #412 [D fix_cutscenes2_live]): Kiko must draw
/// beside Bond mid-scene.
#[test]
fn probe_kimiko_scene_actor_census() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0073), "the Kimiko scene did not start from trigger 412");
    for extra in [60u32, 120, 180] {
        for _ in 0..extra { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
        census(&scene, &format!("kimiko +{extra}"));
    }
}

/// 07000007's Drake fly-through (0x06000072, TouchOnce #356): the live party AND the script's
/// guests drew in the earlier playtest.
#[test]
fn probe_flythrough_actor_census() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [95.4, 49.6, -79.3];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0072), "the fly-through did not start from trigger 356");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    census(&scene, "flythrough");
    render_census(&scene, "flythrough-render");
}

/// The render-path census (the draw gate the live app runs): the script camera drives the view;
/// `cel_visibility`'s strict room mask (the portal walk from the camera position) feeds
/// `skin_palette`'s `vis_room` — a character in a room the walk hides is moved to FAR and never
/// draws. This test runs the exact chain headless (scene::skin_layout + eye_cell +
/// portal_visible_cels_from_cell + the room_to_cell mapping + skin_palette) and counts per
/// character: palette real / FAR (room-hidden) / collapse (frame_state hidden).
fn render_census(scene: &nf_view::scene::Scene, tag: &str) {
    let g = &scene.cel_graph;
    let world = scene.sim.as_ref().map(|s| &s.world);
    let cam = scene.sim.as_ref().unwrap().script_camera();
    let Some((pos, yaw, pitch, _fov)) = cam else { eprintln!("[{tag}] no script camera"); return };
    eprintln!("[{tag}] script camera at ({:.1},{:.1},{:.1}) yaw {yaw:.2} pitch {pitch:.2}", pos[0], pos[1], pos[2]);
    let vp = view_proj(pos, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let depth0 = depth0_game(vp, pos);
    let mut state = nf_view::scene::ViewerCel::default();
    let cell = nf_view::scene::eye_cell(g, &mut state, pos, world);
    eprintln!("[{tag}] eye cell {cell:?}");
    let (layout, _) = scene.skin_layout();
    let vis: Option<Vec<bool>> = cell.and_then(|c| nf_view::scene::portal_visible_cels_from_cell(g, c, pos, &depth0))
        .map(|cell_vis| g.room_to_cell.iter().map(|m| m.map(|c2| cell_vis[c2]).unwrap_or(true)).collect());
    let (pal, _) = scene.skin_palette(&layout, None, 300.0, nf_view::scene::ViewOpts::default(), vis.as_deref());
    let far = [[0.0f32; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
    let (mut real, mut farc, mut coll) = (0usize, 0usize, 0usize);
    for (ci, c) in scene.characters.iter().enumerate() {
        let Some(src) = c.script else { continue };
        let base = layout.char_base[ci] as usize;
        let nb = c.skeleton.bones as usize;
        let mats = &pal[base..=(base + nb).min(pal.len() - 1)];
        let all_far = mats.iter().all(|m| *m == far);
        let all_collapsed = mats.iter().all(|m| m[0] == [0.0; 4] && m[1] == [0.0; 4] && m[2] == [0.0; 4]);
        if all_far { farc += 1; eprintln!("[{tag}] char {ci} src {:?} skin {:08x}: palette FAR (room-hidden)", src, c.skin.hashcode); }
        else if all_collapsed { coll += 1; eprintln!("[{tag}] char {ci} src {:?} skin {:08x}: palette collapsed (frame_state hidden)", src, c.skin.hashcode); }
        else { real += 1; }
    }
    eprintln!("[{tag}] render census: real {real}, far {farc}, collapsed {coll}");
}

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

fn depth0_game(vp: [[f32; 4]; 4], eye: [f32; 3]) -> [[f32; 4]; 5] {
    let planes = nf_view::gpu::extract_frustum(vp);
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}

/// The render path at the Kimiko scene WITH the live app's persistent viewer-cel state: the
/// render loop runs `cel_visibility` (→ `eye_cell`'s persistent `ViewerCel`) every frame at the
/// draw camera, and the script camera CUTS between positions. A cut that portal-move-relinks to
/// the wrong cel (or a FindCel miss that keeps the stale cel) walks the WRONG rooms and the
/// palette's `vis_room` moves every actor to FAR — the playtest's missing cut-scene cast. This
/// test carries ONE ViewerCel across the whole scene, updating it per tick from the script
/// camera exactly like the render loop, and censuses the palette at checkpoints.
#[test]
fn probe_render_path_census_07() {
    let Some(mut scene) = load("07000007.bin") else { return };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert!(playing(&scene, 0x0600_0073), "the Kimiko scene did not start");
    let g = scene.cel_graph.clone();
    let (layout, _) = scene.skin_layout();
    let mut state = nf_view::scene::ViewerCel::default();
    let far = [[0.0f32; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
    let mut last = 0u32;
    for extra in [30u32, 30, 30, 30, 60, 60] {
        for _ in 0..(extra - last) {
            let (cam_pos, _yaw, _pitch, _fov) = scene.sim.as_ref().unwrap().script_camera().unwrap_or_else(|| {
                let p = scene.sim.as_ref().unwrap().bond.head(); (p, 0.0, 0.0, 60.0)
            });
            let world = scene.sim.as_ref().map(|s| &s.world);
            let _ = nf_view::scene::eye_cell(&g, &mut state, cam_pos, world); // the render loop's per-frame eye update
            scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        }
        last = extra;
        let (cam_pos, yaw, pitch, _fov) = scene.sim.as_ref().unwrap().script_camera().unwrap_or_else(|| {
            let p = scene.sim.as_ref().unwrap().bond.head(); (p, 0.0, 0.0, 60.0)
        });
        let vp = view_proj(cam_pos, yaw, pitch, 16.0 / 9.0);
        let depth0 = depth0_game(vp, cam_pos);
        let world = scene.sim.as_ref().map(|s| &s.world);
        // The live loop's cel_visibility with the PERSISTENT state: the eye cell it resolved.
        let cell = nf_view::scene::eye_cell(&g, &mut state, cam_pos, world);
        let vis: Option<Vec<bool>> = cell.and_then(|c| nf_view::scene::portal_visible_cels_from_cell(&g, c, cam_pos, &depth0))
            .map(|cell_vis| g.room_to_cell.iter().map(|m| m.map(|c2| cell_vis[c2]).unwrap_or(true)).collect());
        let (pal, _) = scene.skin_palette(&layout, None, 300.0, nf_view::scene::ViewOpts::default(), vis.as_deref());
        let (mut real, mut farc, mut coll) = (0, 0, 0);
        for (ci, c) in scene.characters.iter().enumerate() {
            let Some(src) = c.script else { continue };
            let base = layout.char_base[ci] as usize;
            let nb = c.skeleton.bones as usize;
            let mats = &pal[base..=(base + nb).min(pal.len() - 1)];
            if mats.iter().all(|m| *m == far) { farc += 1; eprintln!("[render +{last}] char {ci} src {src:?} skin {:08x}: FAR (eye cell {cell:?})", c.skin.hashcode); }
            else if mats.iter().all(|m| m[0] == [0.0; 4] && m[1] == [0.0; 4] && m[2] == [0.0; 4]) { coll += 1; }
            else { real += 1; }
        }
        eprintln!("[render +{last}] eye cell {cell:?} at ({:.1},{:.1},{:.1}): real {real} far {farc} collapsed {coll}", cam_pos[0], cam_pos[1], cam_pos[2]);
        // The scene's own three actors (src 100663411 = 0x06000073) must never be FAR while it plays.
        let scene_chars: Vec<usize> = scene.characters.iter().enumerate().filter(|(_, c)| c.script.map_or(false, |(h, _)| h == 0x0600_0073)).map(|(ci, _)| ci).collect();
        let far_scene = scene_chars.iter().filter(|&&ci| {
            let base = layout.char_base[ci] as usize;
            let nb = scene.characters[ci].skeleton.bones as usize;
            pal[base..=(base + nb).min(pal.len() - 1)].iter().all(|m| *m == far)
        }).count();
        assert_eq!(far_scene, 0, "the Kimiko scene's actors are room-hidden (FAR) at +{last} with the persistent viewer cel {cell:?}");
    }
}

/// The scene drive (fix-cutscene-models): EVERY cut-scene script player on 07000005/06/07/08,
/// started the game's own way — its `out` channel set on the sim's switch channels plus the SP's
/// activation flag (`SP_Update` 0x1dbda8 state 0: `flags = out-channel | activated`, `trig` on
/// `mask`) — then the census at three checkpoints. A scene counts as running at a checkpoint when
/// any of its pre-built characters has a live `Obj::Anim`; while it runs, ALL of its characters
/// must be live (no `frame_state` collapse) with finite poses. This is the regression net: the
/// playtest's "only Bond draws in cut-scenes" means some scene's actors collapse or pose NaN.
#[test]
fn every_scene_on_05_06_07_08_draws_its_actors() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for lv in ["07000005.bin", "07000006.bin", "07000007.bin", "07000008.bin"] {
        let mut scene = match nf_view::load(&iso, lv, false) { Ok(s) => s, Err(e) => { eprintln!("{lv}: {e}"); continue } };
        // The scene hashes the level pre-built actors for (scene.rs's per-Cmd::Anim characters).
        let mut hashes: Vec<u32> = scene.characters.iter().filter_map(|c| c.script.map(|(h, _)| h)).collect();
        hashes.sort(); hashes.dedup();
        if hashes.is_empty() { eprintln!("[{lv}] no script scenes"); continue }
        {
            let sim = scene.sim.as_mut().unwrap();
            let f = sim.drones.frame;
            for p in sim.drones.scripts.players.iter_mut() {
                if p.deleted { continue; }
                if !p.scripts.iter().any(|s| hashes.contains(s)) { continue; }
                if p.out != 0 { sim.drones.channels.set(p.out as u32, 1, f); }
                p.activated = true;
                if p.gate != 0 { sim.drones.channels.set(p.gate as u32, 1, f); }
            }
        }
        let mut last = 0u32;
        for extra in [90u32, 120, 180] {
            for _ in 0..(extra - last) { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
            last = extra;
            // Per hash: is the scene running (any of its chars has a live Obj::Anim) and if so,
            // do ALL its chars draw (live + finite)?
            let sim = scene.sim.as_ref().unwrap();
            for &h in &hashes {
                let chars: Vec<(usize, (u32, u8), u32)> = scene.characters.iter().enumerate()
                    .filter(|(_, c)| c.script.map_or(false, |(h2, _)| h2 == h))
                    .map(|(ci, c)| (ci, c.script.unwrap(), c.skin.hashcode)).collect();
                let live: Vec<bool> = chars.iter().map(|(_, src, skin)| sim.script_anim(*src, *skin).is_some()).collect();
                if !live.iter().any(|l| *l) { continue; } // the scene not running at this checkpoint
                let mut bad = 0usize;
                for (k, (ci, src, skin)) in chars.iter().enumerate() {
                    if live[k] {
                        let (script, frame) = sim.script_anim(*src, *skin).unwrap();
                        if let Some(pose) = nf_game::anim::script_pose_together(&sim.scripts, &sim.seqs, &scene.characters[*ci].skeleton, &scene.characters[*ci].skin, script, frame) {
                            if !pose.iter().all(|(t, q)| t.iter().all(|v| v.is_finite()) && q.iter().all(|v| v.is_finite())) { bad += 1; eprintln!("[{lv} +{extra}] scene {h:08x} char {ci} skin {skin:08x}: NON-FINITE pose"); }
                        } else { bad += 1; eprintln!("[{lv} +{extra}] scene {h:08x} char {ci} skin {skin:08x}: pose None"); }
                    } else { bad += 1; eprintln!("[{lv} +{extra}] scene {h:08x} char {ci} src {src:?} skin {skin:08x}: NO live Obj::Anim (collapsed)"); }
                }
                assert_eq!(bad, 0, "{lv} scene {h:08x}: {bad} of {} actors not drawing while the scene runs", chars.len());
            }
        }
        eprintln!("[{lv}] {} scene hashes driven: {:08x?}", hashes.len(), hashes);
    }
}
