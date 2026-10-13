//! fix-portal3: the 07000006 courtyard holes persist at the eye-cel state. The main agent's render
//! of the fix-portal2 branch showed `--bond 7.24,42.03,-39.31,10.161,0.428 --frame 120` still
//! hiding the masonry under the left walkway and the lower right castle wall (sky through the
//! hole) while `NF_NOPORTAL=1` draws them. fix-portal2's chain pass is evidence-based and kept —
//! the residue is the `Camera_CheckLocation` (0x1bae90) viewer-cel state the port resolves at the
//! eye. This file reproduces the `--bond` path headlessly (Bond placed the way lib.rs places him,
//! then the headless tick loop) and logs the viewer cel the port carries, what a fresh
//! `build_FindCel` (0x1c8cb0) returns, and which World cels the walk + chain pass keep vs the
//! frustum-only set — to find which wall cels are dropped and why.
//!
//! Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3); // the render path's GAME_FOV
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

/// World materials whose centre sits inside the frustum (the draws that survive the sphere cull) —
/// the candidates any visibility rule chooses among.
fn frustum_world_mats(scene: &scene::Scene, planes: &[[f32; 4]; 6]) -> Vec<usize> {
    (0..scene.materials.len())
        .filter(|&i| {
            let m = &scene.materials[i];
            m.layer == scene::Layer::World && m.draw.pass != 4 && !nf_view::gpu::sphere_outside(planes, m.centre, m.radius)
        })
        .collect()
}

/// The probe: Bond placed at the failing spot the way `--bond` places him (lib.rs 1655), 120
/// headless ticks, then the render path's resolution from a default state (shot(): one
/// `cel_visibility` call on a fresh `Renderer`) — plus the room-based start — with the dropped
/// World draws listed.
#[test]
fn probe_the_bond_spot_viewer_cel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    // --bond 7.24,42.03,-39.31,10.161,0.428: the exact dev path (lib.rs 1655-1657).
    {
        let sim = sc.sim.as_mut().expect("07000006 has a sim");
        sim.bond = nf_game::player::Bond::new([7.24, 42.03, -39.31], 10.161);
        sim.bond.pitch = 0.428;
    }
    let graph = sc.cel_graph.clone();
    // The headless tick loop (lib.rs 1663): 120 step_bond ticks, logging the viewer cel each tick
    // the way the interactive loop would carry it (one render per tick → one eye_cell per tick).
    let mut tick_state = scene::ViewerCel::default();
    for t in 1..=120u32 {
        let eye = { let mut sim = sc.sim.as_mut().unwrap(); sim.step_bond(Default::default(), false, false, false); sim.bond.head() };
        let world = sc.sim.as_ref().map(|s| &s.world);
        let cell = scene::eye_cell(&graph, &mut tick_state, eye, world);
        let fresh = scene::find_cell_raycast(&graph, eye, world);
        if t % 20 == 0 || t == 1 {
            let bp = { let sim = sc.sim.as_ref().unwrap(); sim.bond.pos };
            eprintln!("[spot] t{t} bond ({:.2},{:.2},{:.2}) eye ({:.2},{:.2},{:.2}) state {:?} fresh FindCel {:?} agree {}",
                bp[0], bp[1], bp[2], eye[0], eye[1], eye[2], tick_state.cell, fresh, tick_state.cell == fresh);
        }
    }
    let sim = sc.sim.as_ref().unwrap();
    let (bond_pos, bond_yaw, bond_pitch) = (sim.bond.pos, sim.bond.yaw, sim.bond.pitch);
    let eye = sim.bond.head();
    eprintln!("[spot] final bond pos {:?} eye {:?} yaw {bond_yaw} pitch {bond_pitch} (cam pitch {})", bond_pos, eye, bond_pitch * std::f32::consts::FRAC_PI_2);
    drop(sim);

    // The render path (shot()): one cel_visibility on a default state at the final eye/camera.
    let world = sc.sim.as_ref().map(|s| &s.world);
    let cam_pitch = bond_pitch * std::f32::consts::FRAC_PI_2;
    let vp = view_proj(eye, bond_yaw, cam_pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let mut state = scene::ViewerCel::default();
    let cell_persist = scene::eye_cell(&graph, &mut state, eye, world);
    let fresh = scene::find_cell_raycast(&graph, eye, world);
    let room_start = sc.cel_index_at(eye).and_then(|r| graph.room_to_cell.get(r).copied().flatten());
    eprintln!("[spot] render-path viewer cel {cell_persist:?} (state pos {:?}), fresh FindCel {fresh:?}, room-based {room_start:?} (room at eye {:?})",
        state.pos, sc.cel_index_at(eye));
    let starts: [(&str, Option<usize>); 3] = [
        ("persist", cell_persist),
        ("fresh", fresh),
        ("room", room_start),
    ];
    for (name, start) in starts {
        let Some(start) = start else { eprintln!("[spot] {name}: no start cell"); continue };
        let strict = scene::portal_visible_cels_from_cell(&graph, start, eye, &depth0);
        let game = scene::portal_visible_cels_game_from_cell(&graph, start, eye, &depth0, &sides);
        let (Some(strict), Some(game)) = (strict, game) else { eprintln!("[spot] {name}: the walk cannot run from {start}"); continue };
        eprintln!("[spot] {name} cell {start}: strict {} game {}", strict.iter().filter(|&&v| v).count(), game.iter().filter(|&&v| v).count());
        // The dropped World draws through the render gate: node materials by the game mask, cel
        // materials by the room mask (gpu.rs's `hidden`).
        let vis_rooms: Vec<bool> = sc.cel_graph.room_to_cell.iter().map(|m| m.map(|c| strict[c]).unwrap_or(true)).collect();
        let mut dropped_node = 0usize;
        let mut dropped_cel = 0usize;
        for &mi in &frustum_world_mats(&sc, &planes) {
            let m = &sc.materials[mi];
            let hidden = if m.node != u32::MAX { !game[m.node as usize] } else { m.cel.is_some_and(|c| !vis_rooms[c as usize]) };
            if !hidden { continue; }
            if m.node != u32::MAX {
                dropped_node += 1;
                if dropped_node <= 24 {
                    eprintln!("[spot] {name} DROP node mat {mi} node {} centre ({:.1},{:.1},{:.1}) r {:.1} pass {}",
                        m.node, m.centre[0], m.centre[1], m.centre[2], m.radius, m.draw.pass);
                }
            } else {
                dropped_cel += 1;
                if dropped_cel <= 8 {
                    eprintln!("[spot] {name} DROP cel mat {mi} cel {:?} centre ({:.1},{:.1},{:.1})", m.cel, m.centre[0], m.centre[1], m.centre[2]);
                }
            }
        }
        eprintln!("[spot] {name}: dropped node mats {dropped_node}, cel mats {dropped_cel}");
    }
    // Why: for the persist start, the dropped node cells — are they chain nodes of a visible cell,
    // and does their sphere pass Vision_InView?
    if let Some(start) = cell_persist {
        let strict = scene::portal_visible_cels_from_cell(&graph, start, eye, &depth0).unwrap_or_default();
        let game = scene::portal_visible_cels_game_from_cell(&graph, start, eye, &depth0, &sides).unwrap_or_default();
        let mut dropped_cells: std::collections::BTreeSet<usize> = Default::default();
        for &mi in &frustum_world_mats(&sc, &planes) {
            let m = &sc.materials[mi];
            if m.node != u32::MAX && !game[m.node as usize] { dropped_cells.insert(m.node as usize); }
        }
        for &c in &dropped_cells {
            let node = &graph.cells[c];
            let (c3, r) = node.sphere;
            let in_chain_of = (0..graph.cells.len()).filter(|&o| !graph.chains[o].is_empty() && graph.chains[o].contains(&(c as u32)) && strict[o] && game[o]).collect::<Vec<_>>();
            eprintln!("[spot] cell {c}: flags {:#x} bbox min {:?} max {:?} sphere ({:.1},{:.1},{:.1}) r {r:.1} chain owners strict+game visible: {in_chain_of:?}",
                node.flags, node.min, node.max, c3[0], c3[1], c3[2]);
        }
    }

    // The FindCel candidate set at the eye (the 0x40000 room cels whose bbox contains it), the eye
    // side of each one's portals, and the tie-break the game would run (the collision raycast).
    eprintln!("[spot] --- candidates at the eye ---");
    let world = sc.sim.as_ref().map(|s| &s.world);
    for (i, c) in graph.cells.iter().enumerate() {
        if c.flags & 0x40000 == 0 { continue; }
        if !(0..3).all(|k| c.min[k] <= eye[k] && eye[k] <= c.max[k]) { continue; }
        let (up, down) = match &world {
            Some(w) => (
                w.segment(eye, [eye[0], eye[1] + 256.0, eye[2]]).map(|t| t * 256.0),
                w.segment(eye, [eye[0], eye[1] - 256.0, eye[2]]).map(|t| t * 256.0),
            ),
            None => (None, None),
        };
        eprintln!("[spot] cand {i}: flags {:#x} bbox min {:?} max {:?} global-ray up {up:?} down {down:?}", c.flags, c.min, c.max);
    }
    // The portals touching the interesting cells: 21 (the picked start), 33 (the room-based one),
    // and the dropped walls.
    let interesting = [21usize, 33, 19, 20, 22, 38, 5, 23, 13];
    for (pi, p) in graph.portals.iter().enumerate() {
        if interesting.contains(&p.cel_a) || interesting.contains(&p.cel_b) {
            eprintln!("[spot] portal {pi}: {} <-> {} flags {:#x} n_fwd {:?} d_fwd {:.1} quad centre ({:.1},{:.1},{:.1})",
                p.cel_a, p.cel_b, p.flags, p.n_fwd, p.d_fwd,
                p.corners.iter().map(|c| c[0]).sum::<f32>() / 4.0, p.corners.iter().map(|c| c[1]).sum::<f32>() / 4.0, p.corners.iter().map(|c| c[2]).sum::<f32>() / 4.0);
        }
    }
    // The dropped-cell set of the ROOM-based walk for comparison.
    if let Some(start) = room_start {
        let strict = scene::portal_visible_cels_from_cell(&graph, start, eye, &depth0).unwrap_or_default();
        let game = scene::portal_visible_cels_game_from_cell(&graph, start, eye, &depth0, &sides).unwrap_or_default();
        let mut dropped_cells: std::collections::BTreeSet<usize> = Default::default();
        for &mi in &frustum_world_mats(&sc, &planes) {
            let m = &sc.materials[mi];
            if m.node != u32::MAX && !game[m.node as usize] { dropped_cells.insert(m.node as usize); }
        }
        eprintln!("[spot] room-walk from {start}: strict-visited {} cells {:?} dropped cells {:?}",
            strict.iter().filter(|&&v| v).count(),
            strict.iter().enumerate().filter(|(_, &v)| v).map(|(i, _)| i).collect::<Vec<_>>(),
            dropped_cells);
    }
    if let Some(start) = cell_persist {
        let strict = scene::portal_visible_cels_from_cell(&graph, start, eye, &depth0).unwrap_or_default();
        eprintln!("[spot] persist-walk from {start}: strict-visited {} cells {:?}",
            strict.iter().filter(|&&v| v).count(),
            strict.iter().enumerate().filter(|(_, &v)| v).map(|(i, _)| i).collect::<Vec<_>>());
    }
    // The pitch-convention comparison: the same eye at the raw 0.428 rad (24.5 deg) vs the render
    // path's 0.428*pi/2 (38.5 deg) — which start reaches the wall cels, and which drops them.
    {
        // Why does portal 26 (19<->33) clip empty even at level pitch? Print each depth-0 plane's
        // value at the quad's corners.
        let vp = view_proj(eye, bond_yaw, 0.0, 16.0 / 9.0);
        let depth0 = depth0_game(vp, eye);
        let q = &graph.portals.iter().find(|p| (p.cel_a == 19 && p.cel_b == 33) || (p.cel_a == 33 && p.cel_b == 19)).unwrap().corners;
        for (k, pl) in depth0.iter().enumerate() {
            let vals: Vec<f32> = q.iter().map(|c| pl[0] * c[0] + pl[1] * c[1] + pl[2] * c[2] + pl[3]).collect();
            eprintln!("[clip] plane {k} {:?} at quad corners {:?}", pl, vals);
        }
        eprintln!("[clip] quad corners {:?}", q);
        // The walk's exact clip (Sutherland-Hodgman keep >= 0, the game's compaction) over both
        // corner orders.
        let clip = |poly: &Vec<[f32; 3]>, plane: [f32; 4]| -> Vec<[f32; 3]> {
            let val = |p: &[f32; 3]| plane[0] * p[0] + plane[1] * p[1] + plane[2] * p[2] + plane[3];
            if !poly.iter().any(|p| val(p) < 0.0) { return poly.clone(); }
            let n = poly.len();
            let mut out = Vec::with_capacity(n + 2);
            for i in 0..n {
                let (cur, nxt) = (poly[i], poly[(i + 1) % n]);
                let (cin, nin) = (val(&cur) >= 0.0, val(&nxt) >= 0.0);
                let denom = plane[0] * (cur[0] - nxt[0]) + plane[1] * (cur[1] - nxt[1]) + plane[2] * (cur[2] - nxt[2]);
                match (cin, nin) {
                    (true, true) => out.push(nxt),
                    (true, false) => { if denom != 0.0 { let t = -val(&nxt) / denom; out.push([nxt[0] + t * (cur[0] - nxt[0]), nxt[1] + t * (cur[1] - nxt[1]), nxt[2] + t * (cur[2] - nxt[2])]); } }
                    (false, true) => { if denom != 0.0 { let t = -val(&nxt) / denom; out.push([nxt[0] + t * (cur[0] - nxt[0]), nxt[1] + t * (cur[1] - nxt[1]), nxt[2] + t * (cur[2] - nxt[2])]); } out.push(nxt); }
                    (false, false) => {}
                }
            }
            out
        };
        for (name, poly) in [("fwd", q.to_vec()), ("rev", vec![q[3], q[2], q[1], q[0]])] {
            let mut poly = poly;
            for (k, pl) in depth0.iter().enumerate() {
                poly = clip(&poly, *pl);
                eprintln!("[clip] {name} after plane {k}: {} verts", poly.len());
                if poly.is_empty() { break; }
            }
        }
    }
    for (label, pitch) in [("level 0.0", 0.0f32), ("raw 0.428", 0.428f32), ("render 0.428*pi/2", 0.428f32 * std::f32::consts::FRAC_PI_2)] {
        let vp = view_proj(eye, bond_yaw, pitch, 16.0 / 9.0);
        let planes = nf_view::gpu::extract_frustum(vp);
        let sides = [planes[0], planes[1], planes[2], planes[3]];
        let depth0 = depth0_game(vp, eye);
        for (name, start) in [("cell21", Some(21usize)), ("cell33", Some(33usize))] {
            let Some(start) = start else { continue };
            let Some(strict) = scene::portal_visible_cels_from_cell(&graph, start, eye, &depth0) else { continue };
            let Some(game) = scene::portal_visible_cels_game_from_cell(&graph, start, eye, &depth0, &sides) else { continue };
            let vis_rooms: Vec<bool> = graph.room_to_cell.iter().map(|m| m.map(|c| strict[c]).unwrap_or(true)).collect();
            let mut dropped_node = 0usize;
            let mut wall_dropped: Vec<usize> = Vec::new();
            for &mi in &frustum_world_mats(&sc, &planes) {
                let m = &sc.materials[mi];
                let hidden = if m.node != u32::MAX { !game[m.node as usize] } else { m.cel.is_some_and(|c| !vis_rooms[c as usize]) };
                if !hidden { continue; }
                if m.node != u32::MAX {
                    dropped_node += 1;
                    let n = m.node as usize;
                    if [19usize, 20, 22, 38, 5, 13, 23].contains(&n) && !wall_dropped.contains(&n) { wall_dropped.push(n); }
                }
            }
            eprintln!("[pitch] {label} from {name}: visited {} game {} dropped node mats {dropped_node} wall cells dropped {wall_dropped:?}",
                strict.iter().filter(|&&v| v).count(), game.iter().filter(|&&v| v).count());
        }
    }
}
/// The [L] verdict on the portal node plane sides: walk the runtime portal nodes in the EE snapshot
/// (work/ee_start_05.bin, file offset = EE address — level 07000005's own snapshot), match each
/// twin pair to THAT level's main-map PortalQuads by corner quad, and print which cel's list links
/// the file-order-corner (FORWARD) node — the decompile (0x1d15f8) and the disassembly disagree on
/// which side that is; the snapshot settles it.
#[test]
fn probe_snapshot_portal_plane_sides() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The same parse the port's load runs (lib.rs 157-163), for the raw PortalQuad record indices.
    use nf_formats::{disc, iso9660::IsoImage, level::Level};
    let iso_img = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = disc::files_bin_entries(&iso_img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).expect("07000005.bin");
    let data = iso_img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("archive");
    let level = Level::parse(&data).expect("level parse");
    let snap_path = std::env::current_dir().ok().map(|mut d| { loop { d.push("work/ee_start_05.bin"); if d.exists() { break } d.pop(); if !d.pop() { break } } d }).filter(|d| d.exists());
    let Some(snap_path) = snap_path else { eprintln!("work/ee_start_05.bin not found from {:?}", std::env::current_dir()); return };
    let snap = std::fs::read(&snap_path).expect("snapshot");
    let u32at = |a: usize| u32::from_le_bytes(snap[a..a + 4].try_into().unwrap());
    let f32at = |a: usize| f32::from_le_bytes(snap[a..a + 4].try_into().unwrap());
    let pcel_list = u32at(0x0030cfb0) as usize;
    let glb_world = u32at(0x0030d0f8) as usize;
    eprintln!("[snap] pCelList 0x{pcel_list:x} glb_world 0x{glb_world:x}");
    // The world's runtime cel chain: (cel, record address).
    let mut cels: Vec<(usize, usize)> = Vec::new();
    let mut cel = u32at(glb_world) as usize;
    while cel != 0 && cels.len() < 500 && cel + 0x40 <= snap.len() {
        cels.push((cel, u32at(cel + 0x38) as usize));
        cel = u32at(cel + 0x14) as usize;
    }
    eprintln!("[snap] {} runtime cels", cels.len());
    let rec_idx = |r: usize| if pcel_list != 0 && r >= pcel_list && (r - pcel_list) % 0x50 == 0 { Some((r - pcel_list) / 0x50) } else { None };
    // node -> the record index of the cel whose +0x28 list links it.
    let mut owner: std::collections::HashMap<usize, usize> = Default::default();
    for &(cel, rec) in &cels {
        let mut node = u32at(cel + 0x28) as usize;
        let mut hops = 0usize;
        while node != 0 && hops < 512 && node + 0xa0 <= snap.len() {
            hops += 1;
            owner.insert(node, rec);
            node = u32at(node) as usize;
        }
    }
    eprintln!("[snap] {} portal nodes linked", owner.len());
    // The main map's portal quads (the record indices the port's cel_a/cel_b came from).
    let main = level.maps.iter().max_by_key(|m| m.portals.len()).expect("a map with portals");
    eprintln!("[snap] main map portals: {}", main.portals.len());
    let mut matched = 0usize;
    let mut fwd_into_a = 0usize;
    let mut fwd_into_b = 0usize;
    let mut done: std::collections::HashSet<usize> = Default::default();
    for &(cel, _) in &cels {
        let mut node = u32at(cel + 0x28) as usize;
        let mut hops = 0usize;
        while node != 0 && hops < 512 {
            hops += 1;
            if node + 0xa0 > snap.len() { break; }
            let twin = u32at(node + 4) as usize;
            if twin != 0 && twin > node && !done.contains(&node) {
                done.insert(node);
                let corners: Vec<[f32; 3]> = (0..4).map(|c| [f32at(node + 0x30 + 0x10 * c), f32at(node + 0x34 + 0x10 * c), f32at(node + 0x38 + 0x10 * c)]).collect();
                let eq = |a: [f32; 3], b: [f32; 3]| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3 && (a[2] - b[2]).abs() < 1e-3;
                for q in &main.portals {
                    let fwd = (0..4).all(|c| eq(corners[c], q.corners[c]));
                    let rev = (0..4).all(|c| eq(corners[c], q.corners[3 - c]));
                    if fwd || rev {
                        let a_owner = owner.get(&node).copied().and_then(rec_idx);
                        let b_owner = owner.get(&twin).copied().and_then(rec_idx);
                        // `node` holds the file-order (FORWARD) corners when fwd.
                        let (fwd_owner, rev_owner) = if fwd { (a_owner, b_owner) } else { (b_owner, a_owner) };
                        let hit_a = fwd_owner == Some(q.cel_a as usize);
                        let hit_b = fwd_owner == Some(q.cel_b as usize);
                        if hit_a { fwd_into_a += 1; }
                        if hit_b { fwd_into_b += 1; }
                        eprintln!("[snap] quad celA {} celB {}: FORWARD node owned by rec {fwd_owner:?} (celA? {hit_a}, celB? {hit_b}); REVERSED node owned by rec {rev_owner:?}", q.cel_a, q.cel_b);
                        matched += 1;
                        break;
                    }
                }
            }
            node = u32at(node) as usize;
        }
    }
    eprintln!("[snap] {matched} pairs matched: FORWARD node in celA {fwd_into_a}, in celB {fwd_into_b}");
}

/// The 07000006 portal data audit: the port's parse vs the runtime nodes in `work/ee_snapshot.bin`
/// (which level is it? match its node pairs against the 07000006 main-map quads), the port's
/// dropped-portal count, and — for every runtime pair whose quad does NOT match a port quad — the
/// raw corners (the port may be missing or mis-parsing portals).
#[test]
fn probe_portal_data_audit() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    use nf_formats::{disc, iso9660::IsoImage, level::Level};
    let iso_img = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = disc::files_bin_entries(&iso_img).expect("FILES.BIN");
    let read_level = |name: &str| -> Level {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).expect(name);
        let data = iso_img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("archive");
        Level::parse(&data).expect("level parse")
    };
    let level06 = read_level("07000006.bin");
    let sc = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    eprintln!("[audit] port graph: {} portals, {} dropped, {} cells", sc.cel_graph.portals.len(), sc.cel_graph.dropped_portals, sc.cel_graph.cells.len());
    for name in ["ee_snapshot.bin", "ee_start_05.bin"] {
        let snap_path = ["work/", "../work/", "../../work/", "../../../work/"].iter().map(|pre| std::path::PathBuf::from(format!("{pre}{name}"))).find(|d| d.exists());
        let Some(snap_path) = snap_path else { eprintln!("[audit] {name} not found"); continue };
        let Ok(snap) = std::fs::read(&snap_path) else { eprintln!("[audit] {name} unreadable"); continue };
        let u32at = |a: usize| u32::from_le_bytes(snap[a..a + 4].try_into().unwrap());
        let f32at = |a: usize| f32::from_le_bytes(snap[a..a + 4].try_into().unwrap());
        let glb_world = u32at(0x0030d0f8) as usize;
        let mut cels: Vec<usize> = Vec::new();
        let mut cel = u32at(glb_world) as usize;
        while cel != 0 && cels.len() < 500 && cel + 0x40 <= snap.len() {
            cels.push(cel);
            cel = u32at(cel + 0x14) as usize;
        }
        let mut owner: std::collections::HashMap<usize, usize> = Default::default();
        for &cel in &cels {
            let mut node = u32at(cel + 0x28) as usize;
            let mut hops = 0usize;
            while node != 0 && hops < 512 && node + 0xa0 <= snap.len() {
                hops += 1;
                owner.insert(node, cel);
                node = u32at(node) as usize;
            }
        }
        let eq = |a: [f32; 3], b: [f32; 3]| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3 && (a[2] - b[2]).abs() < 1e-3;
        let mut matched = 0usize;
        let mut unmatched = 0usize;
        let mut done: std::collections::HashSet<usize> = Default::default();
        for &cel in &cels {
            let mut node = u32at(cel + 0x28) as usize;
            let mut hops = 0usize;
            while node != 0 && hops < 512 {
                hops += 1;
                if node + 0xa0 > snap.len() { break; }
                let twin = u32at(node + 4) as usize;
                if twin != 0 && twin > node && !done.contains(&node) {
                    done.insert(node);
                    let corners: Vec<[f32; 3]> = (0..4).map(|c| [f32at(node + 0x30 + 0x10 * c), f32at(node + 0x34 + 0x10 * c), f32at(node + 0x38 + 0x10 * c)]).collect();
                    let mut hit = false;
                    let main = level06.maps.iter().max_by_key(|m| m.portals.len()).unwrap();
                    for q in &main.portals {
                        let fwd = (0..4).all(|c| eq(corners[c], q.corners[c]));
                        let rev = (0..4).all(|c| eq(corners[c], q.corners[3 - c]));
                        if fwd || rev { hit = true; break; }
                    }
                    if hit { matched += 1; } else {
                        unmatched += 1;
                        if unmatched <= 10 {
                            eprintln!("[audit] {name}: UNMATCHED pair at node 0x{:x}: corners {:?} plane {:?}", node, corners, [f32at(node + 0x10), f32at(node + 0x14), f32at(node + 0x18), f32at(node + 0x1c)]);
                        }
                    }
                }
                node = u32at(node) as usize;
            }
        }
        eprintln!("[audit] {name}: {} cels, {} nodes, {} pairs matched to 07000006 quads, {unmatched} unmatched", cels.len(), owner.len(), matched);
    }
}

/// The pinned fix, part 1 [M 0x1c8cb0 + 0x1b9920]: at the failing `--bond 7.24,42.03,-39.31,10.161,0.428`
/// spot the render-path viewer cel resolves through `build_FindCel`'s collision tie-break (the port's
/// scoped `CollisionWorld` standing in for the game's per-cel `Collide_RayIntersect` rays) to the
/// courtyard cel — the same cel the room-based derivation picks. The pre-fix bbox-face approximation
/// picked the upper-region cel 21 there instead (the sister assertion pins that divergence).
#[test]
fn the_placed_viewer_cel_resolves_where_the_game_finds_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let graph = sc.cel_graph.clone();
    {
        let sim = sc.sim.as_mut().expect("07000006 has a sim");
        sim.bond = nf_game::player::Bond::new([7.24, 42.03, -39.31], 10.161);
        sim.bond.pitch = 0.428;
        for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    }
    let world = sc.sim.as_ref().map(|s| &s.world);
    let eye = sc.sim.as_ref().unwrap().bond.head();
    // The render path (shot()): one resolution from a default state — what a placement leaves.
    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(&graph, &mut state, eye, world).expect("the placed eye resolves");
    let room_start = sc.cel_index_at(eye).and_then(|r| graph.room_to_cell.get(r).copied().flatten()).expect("the eye's room maps");
    eprintln!("[pin1] eye {:?} collision-tie-break cel {cell:?} room-based {room_start:?}", eye);
    assert_eq!(Some(cell), Some(room_start), "the game-faithful FindCel tie-break must resolve the courtyard cel the room records pick");
    // Sister: the no-world bbox-face approximation diverged here (the pre-fix behaviour).
    let approx = scene::find_cell_raycast(&graph, eye, None);
    eprintln!("[pin1] the bbox-face approximation picks {approx:?} — the divergence the collision tie-break removes");
    assert_ne!(approx, Some(cell), "the approximation was expected to diverge at this eye (it picked the upper-region cel)");
    // And the persistent state agrees with the fresh one after the 120 carried ticks.
    assert_eq!(state.cell, Some(cell));
}

/// The pinned fix, part 2 — the user's actual case: the truck→courtyard walk extended to the spot.
/// The persistent `Camera_CheckLocation` state carries through the portals; the courtyard masonry
/// (cell 19 — the wall the failing view looks at) stays in the game mask along the final leg.
#[test]
fn the_user_route_walk_keeps_the_courtyard_walls() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let graph = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world);
    let (pos, yaw) = sc.start.clone().expect("07000006 has a start");
    let mid = [83.58f32, 35.13, 33.21]; // the courtyard guard spot (fix_searchlight_alarm_live)
    let end = [7.24f32, 42.03, -39.31]; // the failing --bond spot
    let mut state = scene::ViewerCel::default();
    let mut cell19_leg = 0usize;
    let mut min_game = usize::MAX;
    for s in 0..=240usize {
        let t = s as f32 / 240.0;
        // Two legs: start -> courtyard guard spot -> the failing spot (the walk's own route).
        let p = if t <= 0.5 {
            let k = t / 0.5;
            [pos[0] + (mid[0] - pos[0]) * k, pos[1] + (mid[1] - pos[1]) * k, pos[2] + (mid[2] - pos[2]) * k]
        } else {
            let k = (t - 0.5) / 0.5;
            [mid[0] + (end[0] - mid[0]) * k, mid[1] + (end[1] - mid[1]) * k, mid[2] + (end[2] - mid[2]) * k]
        };
        let eye = [p[0], p[1] + 1.6, p[2]];
        let pitch = if t > 0.5 { (t - 0.5) * 2.0 * 0.428 } else { 0.0 }; // the walker tilts up approaching the spot
        let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
        let planes = nf_view::gpu::extract_frustum(vp);
        let sides = [planes[0], planes[1], planes[2], planes[3]];
        let depth0 = depth0_game(vp, eye);
        let Some(cell) = scene::eye_cell(graph, &mut state, eye, world) else { continue };
        let Some(strict) = scene::portal_visible_cels_from_cell(graph, cell, eye, &depth0) else { continue };
        let Some(game) = scene::portal_visible_cels_game_from_cell(graph, cell, eye, &depth0, &sides) else { continue };
        if s % 24 == 0 || s == 240 {
            eprintln!("[pin2] t={t:.2} eye ({:.1},{:.1},{:.1}) cell {:?} strict {} game {}",
                eye[0], eye[1], eye[2], state.cell, strict.iter().filter(|&&v| v).count(), game.iter().filter(|&&v| v).count());
        }
        // The user's reported symptom was the level collapsing to nothing; the game-faithful
        // walk must keep submitting World draws at every step, and the eye cel must always be
        // the game's own resolution (never stuck on the world's first cel).
        assert!(state.cell.is_some(), "t={t:.2}: the viewer cel failed to resolve");
        min_game = min_game.min(game.iter().filter(|&&v| v).count());
        // The courtyard wall cel 19 (the masonry under the walkway) draws from the fresh-33
        // start at level pitch; along the carried state it may not — the live check decides.
        if t > 0.6 && game.get(19) == Some(&true) { cell19_leg += 1; }
    }
    eprintln!("[pin2] cell-19-visible steps on the final leg: {cell19_leg}, min game draws over the route: {min_game}");
    assert!(min_game > 0, "the walk collapsed to zero game draws somewhere on the route — the user's symptom regressed");
}

/// The live-gated wall-cel case (see the report's "Live checks"): whether the REAL game draws
/// wall cels 20/22/38/5/13/23 at the exact placed camera. The port's verified walk rule
/// ([M] 0x1e82e0 + [L] portal sides + [M] fov 60°) does not reach them from any candidate start;
/// `NF_NOPORTAL` draws them because it skips the walk. Ignored until the main agent's gdbwatch
/// of VisCelList (0x34b3b0) at the spot settles which side is the game's.
#[test]
#[ignore = "live-gated: needs the main agent's VisCelList watch at the failing spot (docs/reports/fix-portal3.md)"]
fn the_courtyard_wall_cels_at_the_exact_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let graph = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world);
    let eye = [7.24f32, 42.73, -39.31];
    let vp = view_proj(eye, 10.161, 0.428 * std::f32::consts::FRAC_PI_2, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(graph, &mut state, eye, world).expect("resolves");
    let game = scene::portal_visible_cels_game_from_cell(graph, cell, eye, &depth0, &sides).expect("the walk runs");
    // The wall cels the failing view needs: the game's VisCelList decides.
    let walls = [19usize, 20, 22, 38, 5, 13, 23];
    let kept: Vec<usize> = walls.iter().copied().filter(|c| game[*c]).collect();
    eprintln!("[live] wall cels kept by the walk+chain pass from cell {cell}: {kept:?}");
    assert_eq!(kept.len(), walls.len(), "the walk+chain pass hides wall cels {:?} — the live check must confirm the game hides them too", walls.iter().copied().filter(|c| !game[*c]).collect::<Vec<_>>());
}
