//! fix-portal2: the 07000006 courtyard hole — with the portal walk on, the masonry under the left
//! walkway and the lower part of the right castle wall vanish looking up from the courtyard
//! (`--bond 7.24,42.03,-39.31,10.161,0.428`), and the lit building through the 07000007 ballroom
//! window hides. Holes the player can see through are not a cull the game makes: the port's walk
//! gates every World draw by the VisCelList alone, but the game's `View_AddCels` (0x1e6ba0) ALSO
//! draws, for every VisCelList cel, the cel's **`+0x18` chain nodes** — frustum-tested by
//! `Vision_InView` (0x1e89e0, the four viewer+0x70 side planes, visible iff `n·c + d·r + r >= 0`
//! with the node's sphere) — and skips 0x8018-class nodes while the viewer's cel is 0xc023-class
//! [M 0x1e6ba0:47-56]. The chains tie each cel to the generic (0x8018/0x8019) cel placements
//! parsed after it in the same map-data block [L ee_start_05.bin: 2606 chain nodes over 137 world
//! cels, node classes only 0x8019/0x8008019, the walk-visited rooms' chains carrying the block's
//! later generics]. Room-class cels (0xc023/0xc047) never ride the chains — which is why the
//! 07000007 hidden room stays hidden in the game while the courtyard masonry does not.
//!
//! Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The parsed level (placements), read the same way `nf_view::load` reads it.
fn load_level(iso_path: &str, archive: &str) -> Option<nf_formats::level::Level> {
    let img = nf_formats::iso9660::IsoImage::open(iso_path).ok()?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive))?;
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()?;
    nf_formats::level::Level::parse(&data).ok()
}

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

/// `Vision_InView` (0x1e89e0) [M]: visible iff for each of the four viewer+0x70 side planes
/// `n·c + d·r + r >= 0` — the SIMD dot of the (c, r) VECTOR against the (n, d) plane plus the
/// radius slack (`if (dot < -param_1) return 0`, param_1 = the sphere radius at +0x8c).
fn vision_in_view(c: [f32; 3], r: f32, sides: &[[f32; 4]; 4]) -> bool {
    sides.iter().all(|p| p[0] * c[0] + p[1] * c[1] + p[2] * c[2] + p[3] * r + r >= 0.0)
}

/// The `cel+0x18` chain per cell, rebuilt from the parse [L ee_start_05.bin]: per map-data block
/// (one `parsemap_block_map_data_static` run per map), every generic-class cel placement
/// (0x8018/0x8019 — the classes `View_AddCels`' inner path names) parsed after a block cel hangs
/// on that cel's chain; walk order = parse order.
fn build_chains(level: &nf_formats::level::Level, graph: &scene::CelGraph) -> Vec<Vec<u32>> {
    let mut chains: Vec<Vec<u32>> = vec![Vec::new(); graph.cells.len()];
    let mut main_seen = 0usize; // cells pushed so far, mirroring parse_cel_graph's order
    for map in &level.maps {
        let mut block: Vec<usize> = Vec::new();
        for p in &map.placements {
            if !p.creates_cel() { continue; }
            let idx = main_seen;
            main_seen += 1;
            let flags16 = (p.flags & 0xffff) as u16;
            if matches!(flags16, 0x8018 | 0x8019) {
                for &b in &block { chains[b].push(idx as u32); }
            }
            block.push(idx);
        }
    }
    chains
}

/// The game's World-glist visibility set at `eye`: the VisCelList walk plus the rescued chain
/// nodes (headless — the same rule the renderer's mask implements after the fix).
fn game_visible(graph: &scene::CelGraph, chains: &[Vec<u32>], start: usize, eye: [f32; 3], depth0: &[[f32; 4]; 5], sides: &[[f32; 4]; 4]) -> Vec<bool> {
    let mut vis = vec![false; graph.cells.len()];
    let chain = scene::portal_cel_chain_from_cell(graph, start, eye, depth0).unwrap_or_default();
    for &c in &chain { vis[c] = true; }
    let start_flags = graph.cells[start].flags;
    for &c in &chain {
        for &n in &chains[c] {
            let n = n as usize;
            if vis[n] { continue; }
            let node = &graph.cells[n];
            // `View_AddCels` 0x1e6ba0:47-56: 0x8018-class nodes are skipped while the viewer's cel
            // is 0xc023-class.
            if node.flags & 0xffff == 0x8018 && start_flags & 0xffff == 0xc023 { continue; }
            let c3 = [(node.min[0] + node.max[0]) * 0.5, (node.min[1] + node.max[1]) * 0.5, (node.min[2] + node.max[2]) * 0.5];
            let r = ((node.max[0] - node.min[0]) * (node.max[0] - node.min[0])
                + (node.max[1] - node.min[1]) * (node.max[1] - node.min[1])
                + (node.max[2] - node.min[2]) * (node.max[2] - node.min[2])).sqrt() * 0.5;
            if vision_in_view(c3, r, sides) { vis[n] = true; }
        }
    }
    vis
}

/// The user's 07000006 spot: the walk keeps the courtyard but drops the masonry cels the game
/// draws through the `+0x18` chain pass. Lists the cells the frustum keeps and the walk drops,
/// then asserts the port's visibility covers the game-rule rescue — it does not today (the hole).
#[test]
fn the_07000006_courtyard_walls_stay_visible_at_the_users_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let Some(level) = load_level(&iso, "07000006.bin") else { eprintln!("07000006 level unreadable"); return };
    let scene = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let eye = [7.24f32, 42.03, -39.31];
    let (yaw, pitch) = (10.161f32, 0.428f32);
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let graph = &scene.cel_graph;
    let mut state = scene::ViewerCel::default();
    let start = scene::eye_cell(graph, &mut state, eye).expect("the eye resolves to a cel at the courtyard spot");
    let visited = scene::portal_visible_cels_from_cell(graph, start, eye, &depth0).expect("the walk runs");
    let chains = build_chains(&level, graph);
    let game = game_visible(graph, &chains, start, eye, &depth0, &sides);

    // The census the chain model rests on: which classes the level's cel placements carry.
    let mut census: std::collections::BTreeMap<u16, usize> = Default::default();
    for map in &level.maps { for p in &map.placements { if p.creates_cel() { *census.entry((p.flags & 0xffff) as u16).or_default() += 1; } } }
    let chain_nodes: usize = chains.iter().map(|c| c.len()).sum();
    eprintln!("[hole06] cells {} rooms {} creates_cel census {:?} chain nodes {chain_nodes}",
        graph.cells.len(), scene.cels.len(), census);

    // The frustum keeps, the walk drops: the cells with World materials the 4-plane test passes
    // but the VisCelList mask hides — the hole's contents.
    let mut dropped: Vec<usize> = Vec::new();
    for (i, c) in graph.cells.iter().enumerate() {
        if visited[i] || (c.flags & 0x40000 == 0 && c.min == c.max) { continue; }
        let c3 = [(c.min[0] + c.max[0]) * 0.5, (c.min[1] + c.max[1]) * 0.5, (c.min[2] + c.max[2]) * 0.5];
        let r = ((c.max[0] - c.min[0]) * (c.max[0] - c.min[0]) + (c.max[1] - c.min[1]) * (c.max[1] - c.min[1]) + (c.max[2] - c.min[2]) * (c.max[2] - c.min[2])).sqrt() * 0.5;
        if !vision_in_view(c3, r, &sides) { continue; }
        let mats = scene.materials.iter().filter(|m| m.layer == scene::Layer::World
            && m.centre[0] >= c.min[0] && m.centre[0] <= c.max[0]
            && m.centre[1] >= c.min[1] && m.centre[1] <= c.max[1]
            && m.centre[2] >= c.min[2] && m.centre[2] <= c.max[2]).count();
        if mats > 0 { dropped.push(i); }
    }
    eprintln!("[hole06] frustum-kept walk-dropped cells with World materials: {:?}", dropped);

    // The game's chain pass rescues cells the walk dropped — the masonry under the walkway and the
    // lower castle wall live in generic cels hung on the visited rooms' +0x18 chains.
    let rescued: Vec<usize> = (0..graph.cells.len()).filter(|&i| game[i] && !visited[i]).collect();
    let rescued_with_mats: Vec<usize> = rescued.iter().copied().filter(|&i| {
        let c = &graph.cells[i];
        scene.materials.iter().any(|m| m.layer == scene::Layer::World
            && m.centre[0] >= c.min[0] && m.centre[0] <= c.max[0]
            && m.centre[1] >= c.min[1] && m.centre[1] <= c.max[1]
            && m.centre[2] >= c.min[2] && m.centre[2] <= c.max[2])
    }).collect();
    eprintln!("[hole06] chain-rescued cells {} (with World materials: {:?})", rescued.len(), rescued_with_mats);
    assert!(!rescued_with_mats.is_empty(),
        "the chain pass rescues nothing with World geometry at the courtyard spot — the chain model does not hold here");

    // THE HOLE: the port's walk-visible set must cover every rescued cell (the game draws them);
    // today's mask (VisCelList only) does not — this is the failing assertion pre-fix.
    for &i in &rescued_with_mats {
        assert!(visited[i], "cell {i} is chain-rescued (the game draws it) but the port's walk hides it");
    }
}

/// The 07000007 window: the hidden room stays hidden — room-class cels never ride the +0x18
/// chains (the chain nodes on ee_start_05 are only 0x8019/0x8008019), so the rescue cannot un-hide
/// it, and the 07000007 start view's game-measured draw count keeps its band.
#[test]
fn the_07000007_hidden_room_stays_hidden_and_the_start_view_holds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let Some(level) = load_level(&iso, "07000007.bin") else { eprintln!("07000007 level unreadable"); return };
    let scene = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007: {e}"); return } };
    let graph = &scene.cel_graph;
    let chains = build_chains(&level, graph);
    // No room-class cell is ever a chain node [L ee_start_05.bin node census].
    for (i, ns) in chains.iter().enumerate() {
        if !ns.is_empty() {
            assert!(graph.cells[i].flags & 0xffff != 0xc023 && graph.cells[i].flags & 0xffff != 0xc047,
                "room-class cell {i} carries a chain — rooms are VisCelList members, not chain nodes");
        }
    }
    // The start view: the rescue must not blow past the game-measured band.
    let (pos, yaw) = scene.start.clone().unwrap();
    let eye = [pos[0], pos[1] + 1.6, pos[2]];
    let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let mut state = scene::ViewerCel::default();
    let start = scene::eye_cell(graph, &mut state, eye).expect("the eye resolves on 07000007");
    let game = game_visible(graph, &chains, start, eye, &depth0, &sides);
    let mut sub = 0usize;
    for m in &scene.materials {
        if m.layer != scene::Layer::World { continue; }
        if m.draw.pass != 4 && nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { continue; }
        // The post-fix gate: the placement's own cell when it has one, else the containing room.
        let mut hidden = false;
        if m.node != u32::MAX {
            hidden = !game[m.node as usize];
        } else if let Some(c) = m.cel {
            // the room mask from the strict VisCelList walk (the game's object-cull shape)
            let visited = scene::portal_visible_cels_from_cell(graph, start, eye, &depth0).unwrap();
            let room_vis: Vec<bool> = graph.room_to_cell.iter().map(|rm| rm.map(|cc| visited[cc]).unwrap_or(true)).collect();
            hidden = !room_vis[c as usize];
        }
        if !hidden { sub += 1; }
    }
    eprintln!("[window07] start view submits {sub} World draws under the game rule (the game measured 177)");
    assert!((100..=400).contains(&sub), "the chain rescue pushes the 07000007 start view to {sub} draws — the game measured 177");
}
