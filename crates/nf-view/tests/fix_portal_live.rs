//! fix-portal: the portal visibility walk's collapse on 07000006 (truck → courtyard) and the
//! partygoers drawn outside on 07000007, driven headless through `nf_view::load` (the real viewer
//! path's data) + the renderer's own cull order (`gpu.rs::render`).
//!
//! Evidence base ([M] unless noted):
//! - `Vision_Portal_Recurse` 0x1e86e8 → `Vision_Recurse` 0x1e82e0: the walk (recursion budget 100,
//!   depth cap 45, `VisCelCount < 0x40` insert cap, the eye on the portal plane's negative side,
//!   `ClipPortalWithPlane` 0x1e7fa0 keep `value >= 0`).
//! - `Vision_Calculate_View_Fustrum` 0x1e7df8 (disassembled): depth 0 = the four side planes through
//!   the eye + a near plane through the 200-unit quad corners (c1,c3,c2) whose normal points BACK at
//!   the eye — it keeps `z <= 200`, clipping everything beyond 200 units (the fog range).
//! - `Camera_CheckLocation` 0x1bae90: the viewer's cel is persistent state — `Cel_ObjectLeftCel`
//!   0x1e0d58 (the move raycast against the current cel's portals) relinks directly; else
//!   `build_point_in_bounding_box` 0x1c9500 keeps it; else `build_FindCel` 0x1c8cb0; when FindCel
//!   finds nothing the PREVIOUS cel is kept (and a first-ever miss falls back to the world's first
//!   cel). The port re-picked from scratch every frame with a smallest-volume tie-break instead.
//! - `build_FindCel` 0x1c8cb0: among containing room cels the tie-break is a vertical RAYCAST
//!   (nearest geometry hit above, mirrored below, within 256 units), not the smallest volume.
//! - `View_AddCels` 0x1e6ba0 / `View_AddObjects` 0x1e62d8 / `View_DrawObjList` 0x1e69a0: only the
//!   visible cels' glists AND their linked objects draw (each object's cel via `build_LinkToRoom`
//!   0x1c9680 → `build_FindCel`; `Vision_InView` 0x1e89e0 = the 4-plane sphere test with radius).
//!
//! Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near, far, fov) = (0.05f32, 4000.0f32, 1.0471976f32); // 60°, the render path's GAME_FOV
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
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

/// The World draws the render path submits for `vp` — the same order gpu.rs::render uses (sphere
/// cull, then the visibility set: the portal walk when it runs, else the fix-render3 box test).
fn depth0_game(vp: [[f32; 4]; 4], eye: [f32; 3]) -> [[f32; 4]; 5] {
    // The game's depth-0 [M 0x1e7df8]: the four GH side planes + the near plane keeping
    // fw·p <= fw·eye + 200 (fw = the GH near plane's forward normal).
    let planes = nf_view::gpu::extract_frustum(vp);
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}

/// The room-level visibility along a walk, replicating gpu.rs::cel_visibility headlessly: the
/// persistent `Camera_CheckLocation` 0x1bae90 state carried across the route's ticks.
fn route_vis(scene: &scene::Scene, state: &mut scene::ViewerCel, eye: [f32; 3], vp: [[f32; 4]; 4]) -> Option<Vec<bool>> {
    let depth0 = depth0_game(vp, eye);
    if scene.cel_graph.portals.is_empty() { return None; }
    scene::eye_cell(&scene.cel_graph, state, eye).and_then(|cell| {
        scene::portal_visible_cels_from_cell(&scene.cel_graph, cell, eye, &depth0)
            .map(|cell_vis| scene.cel_graph.room_to_cell.iter().map(|m| m.map(|c| cell_vis[c]).unwrap_or(true)).collect())
    })
}

/// The fix-render3 box-test fallback's submitted count (the pre-portal behavior).
fn box_counts(scene: &scene::Scene, vp: [[f32; 4]; 4]) -> (usize, usize) {
    let planes = nf_view::gpu::extract_frustum(vp);
    let (mut submitted, mut culled) = (0usize, 0usize);
    for m in &scene.materials {
        if m.layer != scene::Layer::World { continue; }
        if m.draw.pass != 4 && nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { culled += 1; continue; }
        if let Some(c) = m.cel {
            if !scene.cels.iter().any(|_| true) { continue; }
            let cel = &scene.cels[c as usize];
            if nf_view::gpu::box_outside(&planes, cel.min, cel.max) { culled += 1; continue; }
        }
        submitted += 1;
    }
    (submitted, culled)
}

fn world_counts(scene: &scene::Scene, vp: [[f32; 4]; 4], _eye: [f32; 3], vis: Option<&[bool]>) -> (usize, usize, usize) {
    let planes = nf_view::gpu::extract_frustum(vp);
    let visible: Option<&[bool]> = vis;
    let (mut submitted, mut culled_sphere, mut culled_vis) = (0usize, 0usize, 0usize);
    for m in &scene.materials {
        if m.layer != scene::Layer::World { continue; }
        if m.draw.pass != 4 && nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { culled_sphere += 1; continue; }
        match (visible, m.cel) {
            (Some(vis), Some(c)) => if !vis[c as usize] { culled_vis += 1; continue; },
            _ => {}
        }
        submitted += 1;
    }
    (submitted, culled_sphere, culled_vis)
}

/// The 07000006 collapse walk, now fixed: the truck spawn to the courtyard (the user's report:
/// the whole level hid with the walk on — empty night sky over water). Per tick the visible World
/// draw count must stay well above zero while the frustum sees geometry, the eye's room cel must
/// always stay visible, and the persistent `Camera_CheckLocation` cel must follow the walk through
/// the gatehouse (29 → 36 → 37), which the old per-frame smallest-volume re-pick could not.
#[test]
fn the_07000006_walk_keeps_the_level_visible() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006 not loadable: {e}"); return } };
    let (pos, yaw) = scene.start.clone().expect("07000006 has a start");
    let end = [83.58f32, 35.13, 33.21]; // the courtyard guard spot (fix_searchlight_alarm_live)
    let steps = 120usize;
    let mut state = scene::ViewerCel::default();
    let mut min_sub = usize::MAX;
    let mut min_visited = usize::MAX;
    let mut cells_seen = std::collections::BTreeSet::new();
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let p = [pos[0] + (end[0] - pos[0]) * t, pos[1] + (end[1] - pos[1]) * t, pos[2] + (end[2] - pos[2]) * t];
        let eye = [p[0], p[1] + 1.6, p[2]];
        let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
        let vis = route_vis(&scene, &mut state, eye, vp);
        let (sub, _, cv) = world_counts(&scene, vp, eye, vis.as_deref());
        let visited = vis.as_ref().map(|v| v.iter().filter(|&&v| v).count()).unwrap_or(0);
        if let Some(cell) = state.cell { cells_seen.insert(cell); }
        min_sub = min_sub.min(sub);
        min_visited = min_visited.min(visited);
        if s % 24 == 0 { eprintln!("[walk06] t={t:.2} eye ({:.1},{:.1},{:.1}) cell {:?} visited {visited} walk {sub} (vis-culled {cv})", eye[0], eye[1], eye[2], state.cell); }
        // The eye's own room never culls (Vision_Portal_Recurse adds it first [M 0x1e86e8]).
        if let (Some(v), Some(room)) = (&vis, scene.cel_index_at(eye)) {
            assert!(v[room], "t={t:.2}: the eye's room {room} culls — the walk is inverted");
        }
    }
    eprintln!("[walk06] min walk draws {min_sub}, min visited rooms {min_visited}, cells followed {cells_seen:?}");
    // The collapse pinned: the walk kept at least this many World draws on every tick (the user's
    // "empty night sky over water" was the walk submitting ~nothing; the fixed walk's minimum along
    // this route is the courtyard's own shell looking out at the water, 104 draws).
    assert!(min_sub >= 100, "the walk collapsed to {min_sub} draws along the truck→courtyard route");
    assert!(min_visited >= 1, "the walk visited nothing at some tick");
    // The state machine followed the path: the truck area (29) and the courtyard (37); the
    // gatehouse (36) only when the path crosses its box (the straight line skims it).
    assert!(cells_seen.contains(&29) && cells_seen.contains(&37),
        "the viewer cel did not follow the walk: {cells_seen:?} (Camera_CheckLocation's relink chain broken)");
}

/// Looking at the castle door from the truck spawn, the walk reaches the castle rooms through it:
/// the door portal (cells 29 ↔ 34, quad centre (55,44,30)) opens and the recursion carries into
/// the castle's network — the multi-hop propagation the depth-1 edge planes must keep alive.
#[test]
fn the_walk_propagates_through_the_castle_door() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let (pos, _) = scene.start.clone().unwrap();
    let door = [55.0f32, 44.0, 30.0]; // portal 39's quad centre (cells 29 <-> 34)
    let eye = [pos[0], pos[1] + 1.6, pos[2]];
    let yaw = (door[0] - eye[0]).atan2(door[2] - eye[2]);
    let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
    let mut state = scene::ViewerCel::default();
    let vis = route_vis(&scene, &mut state, eye, vp).expect("the walk runs from the truck spawn");
    let visited = vis.iter().filter(|&&v| v).count();
    eprintln!("[door06] resolved cell {:?}, visited rooms {visited}", state.cell);
    // The door OPENS: from the truck area the walk must reach the castle side (cell 34, room 29 —
    // the big central hall the 29<->34 portal joins). The next portals sit ~68° off this view axis,
    // so a deeper reach needs a turn; the crossing itself is the pinned rule.
    assert!(visited >= 2, "the walk reached only {visited} rooms — the castle door portal never opened");
    let castle = scene.cel_graph.room_to_cell.iter().position(|m| *m == Some(34)).expect("cell 34 maps to a room");
    assert!(vis[castle], "the castle hall (room {castle}, cell 34) is not visible through its own door");
}

/// The playtest's second symptom: on 07000007 at `--bond 61.11,44.12,-52.58,-2.541,-0.010` (looking
/// out of a ballroom window) a lit room box floated in the snow, and with the walk on two
/// partygoers inside that hidden room still drew (the walk culled only World draws; the game also
/// skips the objects of hidden cels — `View_DrawObjList` 0x1e69a0 walks the visible cels' object
/// lists). Both halves pinned: the hidden room's World draws and its characters' palettes are
/// off with the walk on, and back on with `NF_NOPORTAL` (the sister assertion).
#[test]
fn the_hidden_ballroom_room_and_its_partygoers_are_skipped() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007: {e}"); return } };
    let eye = [61.11f32, 44.12, -52.58];
    let (yaw, pitch) = (-2.541f32, -0.010f32);
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    // The box test's visible rooms (what the frustum alone keeps).
    let box_vis: Vec<bool> = scene.cels.iter().map(|c| !nf_view::gpu::box_outside(&planes, c.min, c.max)).collect();
    let mut state = scene::ViewerCel::default();
    let walk_vis = route_vis(&scene, &mut state, eye, vp).expect("the walk runs on 07000007");
    // The rooms the walk hides while the box test keeps them, with World draws near the view ray
    // (the floating lit box): pick the room whose draws the walk culls and the box keeps.
    let mut hidden_rooms: Vec<usize> = Vec::new();
    for (r, v) in box_vis.iter().enumerate() {
        if *v && !walk_vis[r] { hidden_rooms.push(r); }
    }
    eprintln!("[ballroom] box-kept rooms {}, walk-kept {}, hidden-with-draws candidates {}",
        box_vis.iter().filter(|&&v| v).count(), walk_vis.iter().filter(|&&v| v).count(), hidden_rooms.len());
    let mut hidden_draws_on = 0usize;
    let mut hidden_draws_off = 0usize;
    let mut hidden_room = None;
    for &r in &hidden_rooms {
        let (on, _, _) = world_counts(&scene, vp, eye, Some(&walk_vis));
        let (off, _, _) = world_counts(&scene, vp, eye, None);
        let _ = (on, off);
        // The room's own World draws under each mask.
        let d_on = scene.materials.iter().filter(|m| m.layer == scene::Layer::World && m.cel == Some(r as u16)
            && m.draw.pass != 4 && !nf_view::gpu::sphere_outside(&planes, m.centre, m.radius)).count();
        let d_off = d_on; // with the walk off, only the sphere test applies — the same count
        if d_on > 0 {
            hidden_draws_on += if walk_vis[r] { d_on } else { 0 };
            hidden_draws_off += d_off;
            if hidden_room.is_none() { hidden_room = Some(r); }
        }
    }
    let room = hidden_room.expect("no hidden room with World draws behind the ballroom window");
    eprintln!("[ballroom] hidden room {room}: draws with the walk on {hidden_draws_on}, with it off {hidden_draws_off}");
    assert_eq!(hidden_draws_on, 0, "the hidden room's World draws still submit with the walk on");
    assert!(hidden_draws_off > 0, "the sister assertion broke: the room draws nothing even with the walk off");

    // The partygoers: characters whose posed position sits in the hidden room's box. Their palette
    // collapses to FAR with the walk on (the object half of the cull) and stands with it off.
    let c = &scene.cels[room];
    let layout = scene.skin_layout();
    let (pal_on, _) = scene.skin_palette(&layout.0, None, 0.0, Default::default(), Some(&walk_vis));
    let (pal_off, _) = scene.skin_palette(&layout.0, None, 0.0, Default::default(), None);
    let mut guests = 0usize;
    let in_any_hidden = |p: [f32; 3]| hidden_rooms.iter().any(|&r| {
        let c = &scene.cels[r];
        (0..3).all(|k| c.min[k] <= p[k] && p[k] <= c.max[k])
    });
    for (ci, ch) in scene.characters.iter().enumerate() {
        let p = ch.matrix[3];
        if !in_any_hidden([p[0], p[1], p[2]]) { continue; }
        let base = layout.0.char_base[ci] as usize;
        let far = |m: &[[f32; 4]; 4]| m[3][1] <= -9999.0;
        let on = far(&pal_on[base]);
        let off = far(&pal_off[base]);
        eprintln!("[ballroom] character {ci} at ({:.1},{:.1},{:.1}): palette collapsed on={on} off={off}", p[0], p[1], p[2]);
        assert!(on, "a partygoer inside the hidden room still draws with the walk on");
        assert!(!off, "the sister assertion broke: the partygoer is collapsed even with the walk off");
        guests += 1;
    }
    eprintln!("[ballroom] partygoers inside walk-hidden rooms: {guests}");
    assert!(guests >= 1, "no characters found inside any walk-hidden room — the symptom's spot moved");
}

#[test]
fn diag_cell_flags() {
    let Some(iso) = iso() else { return };
    let scene = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(_) => return };
    for (i, c) in scene.cel_graph.cells.iter().enumerate() {
        if i < 8 || (24..32).contains(&scene.cel_graph.room_to_cell.iter().position(|m| *m == Some(i)).unwrap_or(999)) {
            let room = scene.cel_graph.room_to_cell.iter().position(|m| *m == Some(i));
            eprintln!("[cflags] cell {i} room {room:?} flags {:08x} min ({:.0},{:.0},{:.0})", c.flags, c.min[0], c.min[1], c.min[2]);
        }
        if i > 95 { break; }
    }
}

/// The walk must hold on the other story levels' routes too (the task's 07000005/07000007 walks).
/// 07000007's start view pins the GAME-MEASURED number — fix-perf-level measured 4526 → **177**
/// World draws on master's walk; the fixed walk reproduces it (a strong cross-check that the
/// depth-0 planes and the walk match the game). The forward walk stays inside the playable start
/// region; the 07000005 truck-deck walk pins the never-near-zero floor.
#[test]
fn the_walk_holds_on_the_07000005_and_07000007_routes() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    {
        let scene = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007: {e}"); return } };
        let (pos, yaw) = scene.start.clone().unwrap();
        let eye = [pos[0], pos[1] + 1.6, pos[2]];
        let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
        let mut state = scene::ViewerCel::default();
        let vis = route_vis(&scene, &mut state, eye, vp).expect("the walk runs on 07000007");
        let (sub, _, cv) = world_counts(&scene, vp, eye, Some(&vis));
        eprintln!("[walk-route] 07000007 start: {sub} submitted ({cv} vis-culled), the game measured 177");
        assert!((100..=400).contains(&sub), "07000007's start view submits {sub} draws — the game measured 177 on the walk");
        // The box test keeps ~4500: the walk is doing the work, and it never ADDS draws.
        let (boxsub, _) = box_counts(&scene, vp);
        assert!(sub <= boxsub, "the walk ({sub}) added draws over the box test ({boxsub})");
        // A short forward walk inside the start room: the eye's room never culls.
        for s in 1..=6usize {
            let e2 = [eye[0] + yaw.sin() * s as f32, eye[1], eye[2] + yaw.cos() * s as f32];
            let vp2 = view_proj(e2, yaw, 0.0, 16.0 / 9.0);
            let v2 = route_vis(&scene, &mut state, e2, vp2);
            if let (Some(v), Some(room)) = (&v2, scene.cel_index_at(e2)) {
                assert!(v[room], "step {s}: the eye's room {room} culls");
            }
        }
    }
    {
        let scene = match nf_view::load(&iso, "07000005.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000005: {e}"); return } };
        let (pos, yaw) = scene.start.clone().unwrap();
        let end = [pos[0] + yaw.sin() * 60.0, pos[1], pos[2] + yaw.cos() * 60.0];
        let mut state = scene::ViewerCel::default();
        let mut min_sub = usize::MAX;
        for s in 0..=60usize {
            let t = s as f32 / 60.0;
            let eye = [pos[0] + (end[0] - pos[0]) * t, pos[1] + 1.6, pos[2] + (end[2] - pos[2]) * t];
            let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
            let vis = route_vis(&scene, &mut state, eye, vp);
            let (sub, _, _) = world_counts(&scene, vp, eye, vis.as_deref());
            min_sub = min_sub.min(sub);
            if let (Some(v), Some(room)) = (&vis, scene.cel_index_at(eye)) {
                assert!(v[room], "07000005 t={t:.2}: the eye's room {room} culls");
            }
        }
        eprintln!("[walk-route] 07000005: min walk draws {min_sub}");
        assert!(min_sub >= 300, "07000005: the walk collapsed to {min_sub} draws along the route");
    }
}

/// Why does the 07000007 route's walk drop to 8 draws at t=0.22 while the eye sits in cell 33?
/// Trace cell 33's portals along the route: eye-side test, the move raycast, the clipped poly.
#[test]
fn diag_07000007_cell33_portals() {
    let Some(iso) = iso() else { return };
    let scene = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("{e}"); return } };
    let g = &scene.cel_graph;
    let (pos, yaw) = scene.start.clone().unwrap();
    let end = [61.11f32, 44.12, -52.58];
    let mut state = scene::ViewerCel::default();
    for s in 0..=30usize {
        let t = s as f32 / 140.0;
        let eye = [pos[0] + (end[0] - pos[0]) * t, pos[1] + (end[1] - pos[1]) * t + 1.6, pos[2] + (end[2] - pos[2]) * t];
        let cell = scene::eye_cell(g, &mut state, eye);
        let ps: Vec<String> = g.portals.iter().enumerate()
            .filter(|(_, p)| p.cel_a == state.cell.unwrap_or(0) || p.cel_b == state.cell.unwrap_or(999))
            .map(|(k, p)| {
                let (n, d) = if p.cel_a == state.cell.unwrap_or(0) { ([-p.n_fwd[0], -p.n_fwd[1], -p.n_fwd[2]], -p.d_fwd) } else { (p.n_fwd, p.d_fwd) };
                let v = n[0] * eye[0] + n[1] * eye[1] + n[2] * eye[2] + d;
                format!("p{k}(dest {} eye {v:.1})", if p.cel_a == state.cell.unwrap_or(0) { p.cel_b } else { p.cel_a })
            }).collect();
        if s % 4 == 0 { eprintln!("[c33] t={t:.3} eye ({:.1},{:.1},{:.1}) cell {cell:?} portals {}", eye[0], eye[1], eye[2], ps.join(" ")); }
    }
}
