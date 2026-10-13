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
fn world_counts(scene: &scene::Scene, vp: [[f32; 4]; 4], eye: [f32; 3]) -> (usize, usize, usize) {
    let planes = nf_view::gpu::extract_frustum(vp);
    let depth0: [[f32; 4]; 5] = [planes[0], planes[1], planes[2], planes[3], planes[4]];
    let eye_room = scene.cel_index_at(eye);
    let visible: Option<Vec<bool>> = eye_room.and_then(|room| {
        if scene.cel_graph.portals.is_empty() { return None; }
        scene::portal_visible_cels(&scene.cel_graph, room, eye, &depth0)
            .map(|cell_vis| scene.cel_graph.room_to_cell.iter().map(|m| m.map(|c| cell_vis[c]).unwrap_or(true)).collect())
    });
    let (mut submitted, mut culled_sphere, mut culled_vis) = (0usize, 0usize, 0usize);
    for m in &scene.materials {
        if m.layer != scene::Layer::World { continue; }
        if m.draw.pass != 4 && nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { culled_sphere += 1; continue; }
        match (&visible, m.cel) {
            (Some(vis), Some(c)) => if !vis[c as usize] { culled_vis += 1; continue; },
            _ => {}
        }
        submitted += 1;
    }
    (submitted, culled_sphere, culled_vis)
}

/// The 07000006 collapse walk: the truck spawn toward the courtyard, logging the eye cel, the
/// walk's visited cel count and the visible draw count per tick. The user's report: the level
/// hides entirely (empty night sky over water) along this walk with NF_PORTAL=1.
#[test]
fn repro_the_07000006_walk_collapses() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006 not loadable: {e}"); return } };
    let (pos, yaw) = scene.start.clone().expect("07000006 has a start");
    // The courtyard guard area (fix_searchlight_alarm_live's guard spot) is the walk's far end.
    let end = [83.58f32, 35.13, 33.21];
    let steps = 120;
    eprintln!("[repro] start {pos:?} yaw {yaw} -> end {end:?}; cels {} rooms {} portals {}",
        scene.cel_graph.cells.len(), scene.cels.len(), scene.cel_graph.portals.len());
    let mut collapse = 0usize;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let p = [pos[0] + (end[0] - pos[0]) * t, pos[1] + (end[1] - pos[1]) * t, pos[2] + (end[2] - pos[2]) * t];
        let eye = [p[0], p[1] + 1.6, p[2]];
        let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
        let (sub, cs, cv) = world_counts(&scene, vp, eye);
        let rooms = scene.cel_index_at(eye);
        let visited = rooms.map(|room| scene::portal_cel_chain(&scene.cel_graph, room, eye, &{
            let pl = nf_view::gpu::extract_frustum(vp); [pl[0], pl[1], pl[2], pl[3], pl[4]]
        }).map(|c| c.len()).unwrap_or(0)).unwrap_or(0);
        if sub < 30 { collapse += 1; }
        if s % 6 == 0 || sub < 30 {
            eprintln!("[repro] t={t:.3} eye ({:.1},{:.1},{:.1}) room {rooms:?} visited {visited} walk {sub} (sphere {cs}, vis-culled {cv})", eye[0], eye[1], eye[2]);
        }
    }
    eprintln!("[repro] collapse ticks (walk < 30 draws): {collapse}/{steps}");
}

/// Diagnosis: at each collapse tick, every room cel containing the eye (volume, portal degree,
/// room_to_cell backlink) — the game's Camera_CheckLocation keeps the PREVIOUS cel when the eye
/// leaves none / FindCel misses, so the persistent choice is the giant exterior cel, not the
/// smallest-volume one.
#[test]
fn diag_containing_rooms_and_degrees() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006 not loadable: {e}"); return } };
    let g = &scene.cel_graph;
    let deg = |cell: usize| g.portals.iter().filter(|p| p.cel_a == cell || p.cel_b == cell).count();
    for room in 0..scene.cels.len() {
        let Some(cell) = g.room_to_cell.get(room).copied().flatten() else { continue };
        let c = &scene.cels[room];
        let vol: f32 = (0..3).map(|k| c.max[k] - c.min[k]).product();
        eprintln!("[diag] room {room:2} cell {cell:3} deg {} vol {:.0} min ({:.0},{:.0},{:.0}) max ({:.0},{:.0},{:.0})",
            deg(cell), vol, c.min[0], c.min[1], c.min[2], c.max[0], c.max[1], c.max[2]);
    }
    let (pos, _) = scene.start.clone().unwrap();
    let end = [83.58f32, 35.13, 33.21];
    for s in [70usize, 78, 90] {
        let t = s as f32 / 120.0;
        let eye = [pos[0] + (end[0] - pos[0]) * t, pos[1] + (end[1] - pos[1]) * t + 1.6, pos[2] + (end[2] - pos[2]) * t];
        eprintln!("[diag] t={t:.2} eye ({:.1},{:.1},{:.1}) containing rooms:", eye[0], eye[1], eye[2]);
        for (room, c) in scene.cels.iter().enumerate() {
            let inside = (0..3).all(|k| c.min[k] <= eye[k] && eye[k] <= c.max[k]);
            if inside {
                let cell = g.room_to_cell.get(room).copied().flatten();
                eprintln!("[diag]   room {room} cell {cell:?} deg {} vol {:.0}", cell.map(deg).unwrap_or(0),
                    (0..3).map(|k| c.max[k] - c.min[k]).product::<f32>());
            }
        }
    }
}

/// The game's FindCel room list = the world cel chain filtered by the 0x40000 BIT (build_FindCel
/// 0x1c8cb0 `*puVar1 & 0x40000`), not the two exact flag words 0xc023/0xc047 the port's
/// `parse_room_cels` matches. Count both over every cel placement of every map.
#[test]
fn diag_room_flag_bit_vs_exact_words() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let iso_img = nf_formats::iso9660::IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso_img).expect("FILES.BIN");
    for level in ["07000005.bin", "07000006.bin", "07000007.bin"] {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level)) else { eprintln!("{level}: absent"); continue };
        let data = iso_img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
        let lvl = nf_formats::level::Level::parse(&data).expect("parse");
        let _main = lvl.main_map();
        let (mut bit_main, mut exact, mut cels_main) = (0usize, 0usize, 0usize);
        let mut words = std::collections::BTreeMap::new();
        for (mi, map) in lvl.maps.iter().enumerate() {
            for p in &map.placements {
                if p.flags & 0x8000 == 0 { continue; }
                if mi == 0 { cels_main += 1; }
                *words.entry(format!("{:08x}", p.flags & 0xffff)).or_insert(0usize) += 1;
                if p.flags & 0x40000 != 0 { if mi == 0 { bit_main += 1; } }
                if mi == 0 && matches!(p.flags & 0xffff, 0xc023 | 0xc047) { exact += 1; }
            }
        }
        eprintln!("[diag] {level}: map0 cel placements {cels_main}, 0x40000 bit {bit_main}, exact-word rooms {exact}");
        let top: Vec<_> = words.iter().filter(|(w, _)| w.starts_with("c") || w.starts_with("0")).take(14).collect();
        eprintln!("[diag] {level}: flag words (all maps): {:?}", top);
    }
}

/// Every portal of 07000006's graph: (cel_a, cel_b, flags) with the room backlinks — the castle
/// door between the courtyard and the hall must be in this list for the walk to cross it.
#[test]
fn diag_portal_list() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let g = &scene.cel_graph;
    let cell_to_room: Vec<Option<usize>> = {
        let mut v = vec![None; g.cells.len()];
        for (r, c) in g.room_to_cell.iter().enumerate() { if let Some(c) = c { v[*c] = Some(r); } }
        v
    };
    for (k, p) in g.portals.iter().enumerate() {
        let centre = (0..3).map(|i| p.corners.iter().map(|c| c[i]).sum::<f32>() / 4.0).collect::<Vec<f32>>();
        eprintln!("[portals] {k:2}: cells {}(room {:?}) <-> {}(room {:?}) flags {:x} quad centre ({:.0},{:.0},{:.0})",
            p.cel_a, cell_to_room[p.cel_a], p.cel_b, cell_to_room[p.cel_b], p.flags, centre[0], centre[1], centre[2]);
    }
    eprintln!("[portals] cells {} rooms {} portals {} dropped {}", g.cells.len(), scene.cels.len(), g.portals.len(), g.dropped_portals);
}
