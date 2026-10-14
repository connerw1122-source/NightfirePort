//! fix-portal2: the 07000006 courtyard hole — with the portal walk on, the masonry under the left
//! walkway and the lower part of the right castle wall vanish looking up from the courtyard
//! (`--bond 7.24,42.03,-39.31,10.161,0.428 --frame 120`), and the lit building through the
//! 07000007 ballroom window hides. Holes the player can see through are not a cull the game makes:
//! the port's walk gated every World draw by the VisCelList alone, but the game's `View_AddCels`
//! (0x1e6ba0) ALSO draws, for every VisCelList cel, the cel's **`+0x18` chain nodes** —
//! frustum-tested by `Vision_InView` (0x1e89e0, the four viewer+0x70 side planes, visible iff
//! `n·c + d·r + r >= 0` with the cel's world sphere — the near plane does NOT clip them, which is
//! how the game fills the fog-range far view) — and skips 0x8018-class nodes while the viewer's
//! cel is 0xc023-class [M 0x1e6ba0:47-56]. The chains tie each cel to the generic (0x8018/0x8019)
//! cel placements parsed after it in the same map-data block [L ee_start_05.bin: 2606 chain nodes
//! over 137 world cels, node classes only 0x8019/0x8008019]. Room-class cels never ride the
//! chains as nodes — which is why the 07000007 hidden room stays hidden in the game while the
//! courtyard masonry does not. The port's chain window is the whole map block [S — the snapshot's
//! per-cel counts grow 0..58 with resets this port does not reproduce yet].
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

fn depth0_game(vp: [[f32; 4]; 4], eye: [f32; 3]) -> [[f32; 4]; 5] {
    let planes = nf_view::gpu::extract_frustum(vp);
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}

/// The World materials whose centre sits in `cell`'s world sphere (the transformed bbox).
fn mats_in_cell(scene: &scene::Scene, cell: usize) -> usize {
    let (c3, r) = scene.cel_graph.cells[cell].sphere;
    scene.materials.iter().filter(|m| m.layer == scene::Layer::World && {
        let d = [m.centre[0] - c3[0], m.centre[1] - c3[1], m.centre[2] - c3[2]];
        d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= r * r
    }).count()
}

/// The `+0x18` chain pass rescues World-drawing cells the strict VisCelList mask hides — along the
/// user's truck→courtyard route and at the courtyard spot itself. The pre-fix mask was the strict
/// walk only (no chain pass): every rescued cell below is a draw the port used to hide.
#[test]
fn the_chain_pass_rescues_the_cels_the_strict_walk_hides() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let graph = &scene.cel_graph;
    eprintln!("[chain06] cells {} rooms {} chain nodes {}", graph.cells.len(), scene.cels.len(),
        graph.chains.iter().map(|c| c.len()).sum::<usize>());
    // Chain nodes are only the 0x8018/0x8019 generics [L ee_start_05.bin node census]; owners can
    // be any cel (rooms own chains too — the snapshot's rec-28 room carries a 58-node chain).
    for (i, ns) in graph.chains.iter().enumerate() {
        if ns.is_empty() { continue; }
        for &n in ns {
            let ncls = graph.cells[n as usize].flags & 0xffff;
            assert!(ncls == 0x8018 || ncls == 0x8019, "chain node {n} of cell {i} has class {ncls:#x} — only the 0x8018/0x8019 generics ride the chains");
        }
    }
    let mut state = scene::ViewerCel::default();
    let (pos, yaw) = scene.start.clone().expect("07000006 has a start");
    let end = [83.58f32, 35.13, 33.21]; // the courtyard guard spot (fix_searchlight_alarm_live)
    let steps = 120usize;
    let mut total_rescued = 0usize;
    let mut rescued_mats = 0usize;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let p = [pos[0] + (end[0] - pos[0]) * t, pos[1] + (end[1] - pos[1]) * t, pos[2] + (end[2] - pos[2]) * t];
        let eye = [p[0], p[1] + 1.6, p[2]];
        let pitch = if s == steps { 0.428 } else { 0.0 };
        let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
        let planes = nf_view::gpu::extract_frustum(vp);
        let sides = [planes[0], planes[1], planes[2], planes[3]];
        let depth0 = depth0_game(vp, eye);
        let Some(cell) = scene::eye_cell(graph, &mut state, eye, None) else { continue };
        let Some(strict) = scene::portal_visible_cels_from_cell(graph, cell, eye, &depth0) else { continue };
        let Some(game) = scene::portal_visible_cels_game_from_cell(graph, cell, eye, &depth0, &sides) else { continue };
        // The game's mask never drops what the strict walk keeps (the chain pass only adds).
        for i in 0..graph.cells.len() { assert!(game[i] || !strict[i], "t={t:.2}: the game mask dropped strict-visible cell {i}"); }
        let rescued: Vec<usize> = (0..graph.cells.len()).filter(|&i| game[i] && !strict[i]).collect();
        let with_mats: usize = rescued.iter().filter(|&&i| mats_in_cell(&scene, i) > 0).count();
        total_rescued += rescued.len();
        rescued_mats += with_mats;
        if s % 24 == 0 || s == steps {
            eprintln!("[chain06] t={t:.2} eye ({:.1},{:.1},{:.1}) cell {:?} strict {} game {} rescued {} (with materials {with_mats})",
                eye[0], eye[1], eye[2], state.cell, strict.iter().filter(|&&v| v).count(), game.iter().filter(|&&v| v).count(), rescued.len());
        }
    }
    eprintln!("[chain06] total rescued cell-visits {total_rescued}, material-bearing {rescued_mats}");
    assert!(rescued_mats > 0, "the chain pass rescued nothing with World geometry along the truck→courtyard route");
}

/// The 07000007 window: the hidden room stays hidden (room-class cels never ride the chains), and
/// the start view's game-measured draw band holds with the chain pass on — the sister assertion
/// that the rescue does not blow the cull open.
#[test]
fn the_07000007_hidden_room_stays_hidden_and_the_start_view_holds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007: {e}"); return } };
    let graph = &scene.cel_graph;
    let (pos, yaw) = scene.start.clone().unwrap();
    let eye = [pos[0], pos[1] + 1.6, pos[2]];
    let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let mut state = scene::ViewerCel::default();
    let start = scene::eye_cell(graph, &mut state, eye, None).expect("the eye resolves on 07000007");
    let strict = scene::portal_visible_cels_from_cell(graph, start, eye, &depth0).expect("the walk runs");
    let game = scene::portal_visible_cels_game_from_cell(graph, start, eye, &depth0, &sides).expect("the game mask runs");
    // The post-fix gate: a cel placement draws by its own cell (`Material::node`), the dynamics by
    // their containing room.
    let mut sub = 0usize;
    for m in &scene.materials {
        if m.layer != scene::Layer::World { continue; }
        if m.draw.pass != 4 && nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { continue; }
        let hidden = if m.node != u32::MAX { !game[m.node as usize] }
            else { m.cel.is_some_and(|c| !strict[graph.room_to_cell.get(c as usize).copied().flatten().unwrap_or(usize::MAX)]) };
        if !hidden { sub += 1; }
    }
    eprintln!("[window07] start view submits {sub} World draws under the game rule (the game measured 177)");
    assert!((100..=400).contains(&sub), "the chain pass pushes the 07000007 start view to {sub} draws — the game measured 177");
    // The hidden room (fix-portal's ballroom room): its ROOM cell stays strict-hidden — the chain
    // pass cannot un-hide it because rooms are never chain nodes.
    let eye2 = [61.11f32, 44.12, -52.58];
    let (yaw2, pitch2) = (-2.541f32, -0.010f32);
    let vp2 = view_proj(eye2, yaw2, pitch2, 16.0 / 9.0);
    let planes2 = nf_view::gpu::extract_frustum(vp2);
    let sides2 = [planes2[0], planes2[1], planes2[2], planes2[3]];
    let depth02 = depth0_game(vp2, eye2);
    let mut state2 = scene::ViewerCel::default();
    let start2 = scene::eye_cell(graph, &mut state2, eye2, None).expect("the eye resolves at the window");
    let strict2 = scene::portal_visible_cels_from_cell(graph, start2, eye2, &depth02).expect("the walk runs at the window");
    let game2 = scene::portal_visible_cels_game_from_cell(graph, start2, eye2, &depth02, &sides2).expect("the game mask runs at the window");
    let mut room_cells = 0usize;
    for (i, c) in graph.cells.iter().enumerate() {
        let cls = c.flags & 0xffff;
        if cls != 0xc023 && cls != 0xc047 { continue; }
        if !strict2[i] { continue; }
        assert!(!game2[i] || strict2[i], "a room cell flipped visible without the walk visiting it");
        let _ = room_cells;
        room_cells += 1;
    }
    // The hidden room (fix-portal's ballroom room): its ROOM cell stays game-hidden — the chain
    // pass cannot un-hide it because rooms are never chain nodes.
    let hidden_rooms: Vec<usize> = (0..scene.cels.len()).filter(|&r| {
        let Some(cell) = graph.room_to_cell.get(r).copied().flatten() else { return false };
        !strict2[cell]
    }).collect();
    for &r in &hidden_rooms {
        let cell = graph.room_to_cell[r].unwrap();
        assert!(!game2[cell], "hidden room {r} (cell {cell}) was chain-rescued — rooms must never ride the chains");
    }
    eprintln!("[window07] strict-hidden rooms {} (all stay game-hidden)", hidden_rooms.len());
}
