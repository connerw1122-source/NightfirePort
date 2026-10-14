//! fix-portal5: the courtyard wall attribution — the game draws the wall geometry through room
//! 0x0179fb30 (the port's cell 21) with VisCelCount = 2 [L docs/live/2026-10-13-portal-ground-truth.md],
//! while the port keys the wall meshes under ROOM cells 13/20/22/38 (fix-portal4's census) and hides
//! them. The mechanism found in the decompile [M]:
//!   - `View_AddCels` 0x1e6ba0 walks the depth-sorted VisCelList threaded through cel+0x20
//!     (`vision_generate_display_list` 0x1e8848), pushes each visible cel's own glist (list-0 key
//!     -90000+10k, gated only by glist flag 0x10), then walks each cel's +0x18 chain;
//!   - per visible cel AND per chain node it calls `View_AddObjects` 0x1e62d8(cel, viewer) which
//!     walks **the cel's own object list** (head `cel+0x8`, threaded `obj+0x8` — the list
//!     `build_LinkToRoom` 0x1c9680 fills via `control_link_object_to_cel` 0x1346b8), adding every
//!     object whose glist passes {not (flags & 0x110 && !glist), channel mask, not glist & 0x8010,
//!     Vision_InView} with ±dist² keys;
//!   - `View_AddForcedObjects` 0x1e5fb8 is the complement: it draws ONLY glist-flag-0x8000 objects
//!     (the cel-creating placements' models) from the global list; `View_AddObjects` skips them.
//! So a non-0x8000 model linked into room 21's cel+0x8 list draws whenever 21 is visible. This file
//! probes where the port's wall meshes' placements actually sit, and pins the fix.
//!
//! Opt-in via `NF_ISO`.

use nf_formats::disc;
use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

use nf_view::scene;

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

fn near(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= tol)
}

/// The live main-view 40-node chain's 12 named nodes [L ground-truth doc]: (sphere centre, radius,
/// runtime address). The 28 railing nodes are matched by membership (r < 0.9 at y 46.5 on the two
/// railing lines z -32.8 / -26.2).
const LIVE_BIG_CHAIN_HEAD: [([f32; 3], f32, &str); 12] = [
    ([3.0, 46.6, -32.8], 4.5, "0x179e930"),
    ([3.0, 46.6, -26.2], 4.5, "0x179b4b0"),
    ([3.0, 51.1, -33.7], 17.6, "0x179b270"),
    ([-26.0, 48.9, -59.3], 2.3, "0x1793830"),
    ([-23.0, 48.9, -59.3], 2.3, "0x1793770"),
    ([-20.0, 48.9, -59.3], 2.3, "0x17936b0"),
    ([-6.5, 48.9, -59.3], 2.3, "0x17935f0"),
    ([4.7, 46.0, -27.0], 8.4, "0x17926f0"),
    ([-31.5, 49.7, -26.0], 1.5, "0x1792570"),
    ([-18.5, 52.1, -59.0], 15.5, "0x17923f0"),
    ([-34.0, 51.3, -41.5], 20.0, "0x1790cb0"),
    ([-17.0, 46.5, -38.6], 1.5, "0x17905f0"),
];


fn load_level(iso: &str, name: &str) -> Level {
    let img = IsoImage::open(iso).expect("iso");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("{name}"));
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("archive");
    Level::parse(&data).expect("level parse")
}

/// The [L] model-identity census on the 05 start snapshot: every runtime cel's glist carries the
/// model's LOCAL header (sphere at glist+0x10..0x1c, bbox at +0x20..0x40, draw flags at +0x44 —
/// `parsemap_block_map_data_static` 0x1d1998 copies them to the cel). Match each glist against the
/// port's parsed objects by that header; then, for every runtime ROOM cel, print which port object
/// the game's glist is vs the object the port's room placement resolved to. A mismatch = the port's
/// placement->model attribution differs from the game's.
#[test]
fn probe_portal5_glist_model_match() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let level = load_level(&iso, "07000005.bin");
    let m = level.main_map().expect("main map");
    let snap_path = ["work/", "../work/", "../../work/"].iter().map(|pre| std::path::PathBuf::from(format!("{pre}ee_start_05.bin"))).find(|d| d.exists());
    let Some(snap_path) = snap_path else { eprintln!("ee_start_05.bin not found"); return };
    let snap = std::fs::read(&snap_path).expect("snapshot");
    let u32at = |a: usize| u32::from_le_bytes(snap[a..a + 4].try_into().unwrap());
    let f32at = |a: usize| f32::from_le_bytes(snap[a..a + 4].try_into().unwrap());

    // The port's objects (ALL maps — the game's model array is parse-wide).
    let mut objects: Vec<(usize, usize, &str, [f32; 3], f32, [f32; 3], [f32; 3], u32)> = Vec::new();
    for (mi, map) in level.maps.iter().enumerate() {
        for (oi, o) in map.objects.iter().enumerate() {
            objects.push((mi, oi, o.name.trim(), o.centre, o.radius, o.min, o.max, o.flags));
        }
    }
    let near = |a: [f32; 3], b: [f32; 3], tol: f32| a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= tol);

    let glb_world = u32at(0x30d0f8) as usize;
    let mut cels: Vec<usize> = Vec::new();
    let mut c = u32at(glb_world) as usize;
    while c != 0 && cels.len() < 800 && c + 0xb0 <= snap.len() { cels.push(c); c = u32at(c + 0x14) as usize; }
    eprintln!("[glist] runtime cels {}", cels.len());

    let mut matched = 0usize; let mut unmatched = 0usize;
    let mut unmatched_glists: Vec<u32> = Vec::new();
    let mut g2obj: std::collections::HashMap<usize, usize> = Default::default();
    for (ci, &cel) in cels.iter().enumerate() {
        let g = u32at(cel + 0x38) as usize;
        if g == 0 || g + 0x48 > snap.len() { continue; }
        let gc = [f32at(g + 0x10), f32at(g + 0x14), f32at(g + 0x18)];
        let gr = f32at(g + 0x1c);
        let gb = ([f32at(g + 0x20), f32at(g + 0x24), f32at(g + 0x28)], [f32at(g + 0x30), f32at(g + 0x34), f32at(g + 0x38)]);
        let gf = u32at(g + 0x44);
        let hit = objects.iter().position(|(_, _, _, oc, or, omin, omax, of)|
            near(gc, *oc, 0.05) && (gr - *or).abs() < 0.05 && gf == *of
            && (0..3).all(|k| (gb.0[k] - omin[k]).abs() < 0.05 && (gb.1[k] - omax[k]).abs() < 0.05));
        match hit {
            Some(h) => { matched += 1; g2obj.insert(ci, h); }
            None => { unmatched += 1; if unmatched_glists.len() < 8 { unmatched_glists.push(g as u32); } }
        }
    }
    eprintln!("[glist] glist->port-object header matches {matched}, unmatched {unmatched} (sample glists {unmatched_glists:?})");

    // The rooms: the runtime ROOM cels' glist model vs the port's room placement object.
    for (ci, &cel) in cels.iter().enumerate() {
        let cls = u32at(cel + 0x3c);
        if (cls & 0xffff) != 0xc047 && (cls & 0xffff) != 0xc023 { continue; }
        let g = u32at(cel + 0x38) as usize;
        if g == 0 || g + 0x48 > snap.len() { continue; }
        let gc = [f32at(g + 0x10), f32at(g + 0x14), f32at(g + 0x18)];
        let gr = f32at(g + 0x1c);
        let game_obj = g2obj.get(&ci).map(|&h| {
            let (mi, oi, name, _, _, _, _, _) = objects[h];
            format!("maps[{mi}].objects[{oi}] \"{name}\"")
        }).unwrap_or_else(|| "<no header match>".into());
        // The port's room placements: main-map 0xc023/0xc047 placements in parse order — match the
        // runtime room cel by the placement's world sphere (the port's CelNode match, fix-portal4).
        let sc3 = [f32at(cel + 0x80), f32at(cel + 0x84), f32at(cel + 0x88)];
        let sr = f32at(cel + 0x8c);
        let mut port_obj = "<no sphere match>".to_string();
        for p in &level.maps[m].placements {
            if !matches!(p.flags & 0xffff, 0xc023 | 0xc047) { continue; }
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
            let Some((om, oi)) = target else { continue };
            let o = &level.maps[om].objects[oi];
            // the placement-transformed sphere, the way parse_cel_graph derives it
            let mtx = nf_formats::placement::object_matrix(p.rotation, p.translation);
            let mut lo = [f32::MAX; 3]; let mut hi = [f32::MIN; 3];
            for cx in [o.min[0], o.max[0]] { for cy in [o.min[1], o.max[1]] { for cz in [o.min[2], o.max[2]] {
                let w = nf_formats::placement::transform(&mtx, [cx, cy, cz]);
                for k in 0..3 { lo[k] = lo[k].min(w[k]); hi[k] = hi[k].max(w[k]); }
            } } }
            let c3 = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5, (lo[2] + hi[2]) * 0.5];
            let d = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
            let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() * 0.5;
            if near(sc3, c3, 0.6) && (sr - r).abs() < 1.0 {
                port_obj = format!("objects[{oi}] \"{}\" (flags {:08x})", o.name.trim(), o.flags);
                break;
            }
        }
        eprintln!("[room] cel {:#x} class {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1}: GAME glist model {game_obj} | PORT placement object {port_obj}",
            cel, cls, sc3[0], sc3[1], sc3[2], sr);
    }
}

/// Where do the wall meshes' placements sit, and what would the game's draw do with them?
#[test]
fn probe_portal5_wall_attribution() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let level = load_level(&iso, "07000006.bin");
    let m = level.main_map().expect("main map");
    let g = &sc.cel_graph;
    eprintln!("cells {} dropped_portals {}", g.cells.len(), g.dropped_portals);

    // 1. The wall-region material census by node (the port's current attribution).
    let wall_cells = [5usize, 13, 19, 20, 22, 23, 38];
    let mut by_node: std::collections::BTreeMap<u32, usize> = Default::default();
    for mat in sc.materials.iter().filter(|mm| mm.layer == nf_view::scene::Layer::World) {
        if wall_cells.iter().any(|&c| {
            let cell = &g.cells[c];
            (0..3).all(|k| mat.centre[k] >= cell.min[k] - 1e-4 && mat.centre[k] <= cell.max[k] + 1e-4)
        }) { *by_node.entry(mat.node).or_default() += 1; }
    }
    eprintln!("[mats] wall-region World materials by node: {:?} (total {})", by_node,
        by_node.values().sum::<usize>());

    // 2. Every main-map placement: identity, class, own cell.
    //    `own_cells_main` is by placement ordinal (None = no cel). Reverse-map cells 13/20/22/38
    //    to their placements, print the objects, then find EVERY placement of those objects.
    let own = &g.own_cells_main;
    let mut cell_to_placement: std::collections::HashMap<u32, usize> = Default::default();
    for (pi, oc) in own.iter().enumerate() {
        if let Some(c) = oc { cell_to_placement.insert(*c, pi); }
    }
    let mut wall_hashes: Vec<u32> = Vec::new();
    for &c in &wall_cells {
        let Some(&pi) = cell_to_placement.get(&(c as u32)) else { eprintln!("[cell {c}] no placement"); continue };
        let p = &level.maps[m].placements[pi];
        let name = if p.by_hash() {
            level.by_hash.get(&p.hashcode).map(|(mm, oi)| level.maps[*mm].objects[*oi].name.trim().to_string()).unwrap_or_else(|| "<unresolved>".into())
        } else {
            level.maps[m].objects.get(p.index as usize).map(|o| o.name.trim().to_string()).unwrap_or_else(|| "<bad index>".into())
        };
        let h = p.hashcode;
        eprintln!("[cell {c}] placement {pi}: by_hash {} hash {:08x} index {} flags {:08x} (low16 {:04x}) creates_cel {} at ({:.1},{:.1},{:.1}) obj \"{name}\"",
            p.by_hash(), h, p.index, p.flags, p.flags & 0xffff, p.creates_cel(), p.translation[0], p.translation[1], p.translation[2]);
        if p.by_hash() { wall_hashes.push(h); }
    }

    // 3. All placements of the wall objects (same hash), any class.
    for h in &wall_hashes {
        let mut hits = 0usize;
        for (pi, p) in level.maps[m].placements.iter().enumerate() {
            if !p.by_hash() || p.hashcode != *h { continue; }
            hits += 1;
            let own_cell = own.get(pi).copied().flatten().map(|c| c.to_string()).unwrap_or_else(|| "-".into());
            eprintln!("[obj {:08x}] placement {pi}: flags {:08x} (low16 {:04x}) creates_cel {} own_cell {own_cell} at ({:.1},{:.1},{:.1})",
                h, p.flags, p.flags & 0xffff, p.creates_cel(), p.translation[0], p.translation[1], p.translation[2]);
        }
        eprintln!("[obj {:08x}] {hits} placements", h);
    }

    // 4. The big room (cell 21) and the eye (33): identity, and chain(21)'s nodes' materials.
    for &c in &[21usize, 33] {
        let cell = &g.cells[c];
        eprintln!("[cell {c}] flags {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1} bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) chain {}",
            cell.flags, cell.sphere.0[0], cell.sphere.0[1], cell.sphere.0[2], cell.sphere.1,
            cell.min[0], cell.min[1], cell.min[2], cell.max[0], cell.max[1], cell.max[2], g.chains[c].len());
        if let Some(&pi) = cell_to_placement.get(&(c as u32)) {
            let p = &level.maps[m].placements[pi];
            let name = if p.by_hash() {
                level.by_hash.get(&p.hashcode).map(|(mm, oi)| level.maps[*mm].objects[*oi].name.trim().to_string()).unwrap_or_else(|| "<unresolved>".into())
            } else { format!("index {}", p.index) };
            eprintln!("[cell {c}] placement {pi} obj \"{name}\" hash {:08x}", p.hashcode);
        }
        let mut node_mats: std::collections::BTreeMap<u32, usize> = Default::default();
        for &n in &g.chains[c] {
            let k = sc.materials.iter().filter(|mm| mm.layer == nf_view::scene::Layer::World && mm.node == n).count();
            if k > 0 { node_mats.insert(n, k); }
        }
        eprintln!("[cell {c}] chain nodes with World materials: {:?} ({} of {})", node_mats, node_mats.len(), g.chains[c].len());
    }

    // 4b. The wall-material-bearing generic cells: identity, spheres, and their chain owners.
    for &c in &[41usize, 42, 43, 57, 344, 345, 346, 45, 115, 118, 339, 349, 308, 306, 304] {
        let Some(cell) = g.cells.get(c) else { continue };
        let owners: Vec<usize> = g.chains.iter().enumerate().filter(|(_, ch)| ch.contains(&(c as u32))).map(|(o, _)| o).collect();
        let nmats = sc.materials.iter().filter(|mm| mm.layer == nf_view::scene::Layer::World && mm.node == c as u32).count();
        let mut name = String::new();
        if let Some(&pi) = cell_to_placement.get(&(c as u32)) {
            let p = &level.maps[m].placements[pi];
            name = if p.by_hash() {
                level.by_hash.get(&p.hashcode).map(|(mm, oi)| level.maps[*mm].objects[*oi].name.trim().to_string()).unwrap_or_else(|| "<unresolved>".into())
            } else {
                level.maps[m].objects.get(p.index as usize).map(|o| o.name.trim().to_string()).unwrap_or_else(|| "<bad index>".into())
            };
            name = format!("\"{name}\" (placement {pi}, flags {:08x})", p.flags);
        }
        eprintln!("[wallgen] cell {c}: flags {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1} mats {nmats} owners {owners:?} obj {name}",
            cell.flags, cell.sphere.0[0], cell.sphere.0[1], cell.sphere.0[2], cell.sphere.1);
        if let Some(&pi) = cell_to_placement.get(&(c as u32)) {
            let p = &level.maps[m].placements[pi];
            if let Some((om, oi)) = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((m, p.index as usize)) } {
                let o = &level.maps[om].objects[oi];
                let leaves = o.gfx.leaves.len();
                let batches: usize = o.gfx.leaves.iter().map(|l| l.batches.len()).sum();
                eprintln!("[wallgen]   obj flags {:#x} leaves {leaves} batches {batches} skipped_by_cel_pass {} center ({:.1},{:.1},{:.1}) r {:.1}",
                    o.flags, o.skipped_by_cel_pass(), o.centre[0], o.centre[1], o.centre[2], o.radius);
            }
        }
    }

    // 4c. chain(21) in full: node id, sphere, material count, live-match mark.
    {
        let live: [([f32; 3], f32); 12] = [
            ([3.0, 46.6, -32.8], 4.5), ([3.0, 46.6, -26.2], 4.5), ([3.0, 51.1, -33.7], 17.6),
            ([-26.0, 48.9, -59.3], 2.3), ([-23.0, 48.9, -59.3], 2.3), ([-20.0, 48.9, -59.3], 2.3),
            ([-6.5, 48.9, -59.3], 2.3), ([4.7, 46.0, -27.0], 8.4), ([-31.5, 49.7, -26.0], 1.5),
            ([-18.5, 52.1, -59.0], 15.5), ([-34.0, 51.3, -41.5], 20.0), ([-17.0, 46.5, -38.6], 1.5),
        ];
        let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 0.8);
        for (k, &n) in g.chains[21].iter().enumerate() {
            let s = g.cells[n as usize].sphere;
            let nmats = sc.materials.iter().filter(|mm| mm.layer == nf_view::scene::Layer::World && mm.node == n).count();
            let live_hit = live.iter().position(|(c3, r)| near(s.0, *c3) && (s.1 - r).abs() < 1.2);
            eprintln!("[ch21] {k}: node {n} flags {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1} mats {nmats} {}", g.cells[n as usize].flags, s.0[0], s.0[1], s.0[2], s.1,
                live_hit.map(|i| format!("LIVE#{i}")).unwrap_or_else(|| "extra".into()));
        }
    }

    // 5. The own-cell alignment audit: walk the instances the way Scene::new does and compare the
    //    consumed own_cells[own_i] against the placement's TRUE own cell (own_cells_main by ordinal).
    //    fix-portal5: the audit replicates the FIXED build (same filters as the walk).
    let mut own_cells: Vec<u32> = Vec::new();
    for (pi, p) in level.maps[m].placements.iter().enumerate() {
        let resolved = if p.by_hash() { level.by_hash.contains_key(&p.hashcode) } else { (p.index as usize) < level.maps[m].objects.len() };
        if !resolved { continue; }
        if matches!(p.flags & 0xff, 0x2a | 0x22 | 0xd4) { continue; }
        if matches!(p.flags & 0xffff, 0xc023 | 0xc047) { continue; }
        own_cells.push(own.get(pi).copied().flatten().unwrap_or(u32::MAX));
    }
    let mut own_i = 0usize; let mut mis = 0usize; let mut checked = 0usize;
    let mut samples: Vec<String> = Vec::new();
    for (pi, p) in level.maps[m].placements.iter().enumerate() {
        let resolved = if p.by_hash() { level.by_hash.contains_key(&p.hashcode) } else { (p.index as usize) < level.maps[m].objects.len() };
        if !resolved { continue; }
        let room_class = matches!(p.flags & 0xffff, 0xc023 | 0xc047);
        // (Scene::new also retains-out flags&0xff in {0x2a,0x22,0xd4} — they never reach own_i.)
        if matches!(p.flags & 0xff, 0x2a | 0x22 | 0xd4) { continue; }
        if room_class { continue; }
        let got = own_cells.get(own_i).copied().unwrap_or(u32::MAX);
        let want = own.get(pi).copied().flatten().unwrap_or(u32::MAX);
        checked += 1;
        if got != want {
            mis += 1;
            if samples.len() < 8 { samples.push(format!("placement {pi} (flags {:04x}) wants cell {want}, Scene's walk consumes {got} (own_i {own_i})", p.flags & 0xffff)); }
        }
        own_i += 1;
    }
    eprintln!("[align] non-room instances checked {checked}, misaligned {mis}");
    for s in &samples { eprintln!("[align] {s}"); }
}

/// The big room's own model: what the port's cell 21 (placement 26, index 20) resolves to, its
/// World material count, and where the wall-region materials sit per node (21 and 33 included).
#[test]
fn probe_portal5_big_room_model() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let level = load_level(&iso, "07000006.bin");
    let m = level.main_map().expect("main map");
    for idx in [19usize, 20, 21, 22, 37, 4, 12, 18] {
        let o = &level.maps[m].objects[idx];
        let batches: usize = o.gfx.leaves.iter().map(|l| l.batches.len()).sum();
        eprintln!("[obj{idx}] \"{}\" flags {:#x} leaves {} batches {} centre ({:.1},{:.1},{:.1}) r {:.1} bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1})",
            o.name.trim(), o.flags, o.gfx.leaves.len(), batches, o.centre[0], o.centre[1], o.centre[2], o.radius,
            o.min[0], o.min[1], o.min[2], o.max[0], o.max[1], o.max[2]);
    }
    // Per-node World material totals for every node, top 20 by count.
    let g = &sc.cel_graph;
    let mut by_node: std::collections::BTreeMap<u32, usize> = Default::default();
    for mat in sc.materials.iter().filter(|mm| mm.layer == nf_view::scene::Layer::World) {
        *by_node.entry(mat.node).or_default() += 1;
    }
    let mut v: Vec<(u32, usize)> = by_node.clone().into_iter().collect();
    v.sort_by_key(|(n, c)| std::cmp::Reverse(*c));
    eprintln!("[nodes] top: {:?}", &v[..v.len().min(24)]);
    for (n, c) in v.iter().take(24) {
        let cell = g.cells.get(*n as usize);
        let (fl, sp) = cell.map(|c| (c.flags, c.sphere)).unwrap_or((0, ([0.0; 3], 0.0)));
        eprintln!("[node {n}] {c} mats, flags {fl:08x}, sphere ({:.1},{:.1},{:.1}) r {:.1}", sp.0[0], sp.0[1], sp.0[2], sp.1);
    }
    let m21 = by_node.get(&21).copied().unwrap_or(0);
    let m33 = by_node.get(&33).copied().unwrap_or(0);
    eprintln!("[bigroom] cell 21 World materials {m21}, cell 33 {m33}");
}

/// The courtyard pin [L docs/live/2026-10-13-portal-ground-truth.md + M 0x1e6ba0/0x1c9150/0x1e62d8]:
/// at `--bond 7.24,42.03,-39.31,10.161,0.428` the game's main view holds VisCelCount = 2 (the eye
/// cel 0x0179f230 = port cell 33, and 0x0179fb30 = port cell 21) and still draws the courtyard
/// walls — through room 21's own glist (the port's cell-21 materials, Room_Courtyards_G) and its
/// 40-node +0x18 chain. This test pins, after the fix-portal5 repairs (the own-cell alignment + the
/// build_link_objects_to_rooms 0x1c9150 owner rule):
///   1. the walk visits exactly [33, 21];
///   2. cell 33's chain is empty (the live eye cel's is);
///   3. cell 21's chain carries the live 40 — the 12 named nodes by sphere + exactly 28 railing
///      nodes — the 28 railing nodes by membership;
///   4. the wall/room materials the game draws at this spot — every World material whose node is
///      cell 33's and cell 21's own models — the wall masonry and castle wall at VisCelCount = 2
///      — are ALL visible in the game mask, and no wall ROOM cell leaks into the visible set;
///   5. sister: the courtyard generics own their own materials (cell 39/40/41/42's material counts
///      equal their placements' object batch counts — the misaligned build keyed them elsewhere).
#[test]
fn the_courtyard_walls_draw_with_the_walk_on() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let g = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world);
    let eye = [7.24f32, 42.73, -39.31];
    let vp = view_proj(eye, 10.161, 0.428 * std::f32::consts::FRAC_PI_2, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(g, &mut state, eye, world).expect("resolves");
    assert_eq!(cell, 33, "the eye cel resolves to port cell 33 (the live 0x0179f230)");
    let chain = scene::portal_cel_chain_from_cell(g, cell, eye, &depth0).expect("walk runs");
    assert_eq!(chain, vec![33usize, 21], "the walk visits exactly the live two-room set (VisCelCount = 2)");
    assert!(g.chains[33].is_empty(), "cell 33's +0x18 chain must be empty — the live eye cel's is");
    // The live 40 on cell 21: the 12 named nodes by sphere, 28 railings, no other extras beyond
    // the known 9 (counted, not asserted away — their game-side owner is a live check).
    let ch21 = &g.chains[21];
    for (k, (c3, r, addr)) in LIVE_BIG_CHAIN_HEAD.iter().enumerate() {
        assert!(ch21.iter().any(|&n| { let s = g.cells[n as usize].sphere; near(s.0, *c3, 0.8) && (s.1 - r).abs() < 1.2 }),
            "live chain node {k} ({addr}) missing from the port's chain for cell 21");
    }
    let railings = ch21.iter().filter(|&&n| {
        let s = g.cells[n as usize].sphere;
        s.1 < 0.9 && s.0[1] > 45.5 && s.0[1] < 47.5 && ((s.0[2] + 32.8).abs() < 0.8 || (s.0[2] + 26.2).abs() < 0.8)
    }).count();
    assert_eq!(railings, 28, "the port's chain holds {railings} railing nodes — the live chain holds 28");
    let game = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("mask");
    assert!(game[33] && game[21], "both rooms draw");
    // Every live-40 chain node passes Vision_InView (the game draws them all).
    let live40: Vec<u32> = {
        let mut v: Vec<u32> = Vec::new();
        for &(c3, r, _) in LIVE_BIG_CHAIN_HEAD.iter() {
            if let Some(&n) = ch21.iter().find(|&&n| { let s = g.cells[n as usize].sphere; near(s.0, c3, 0.8) && (s.1 - r).abs() < 1.2 }) { v.push(n); }
        }
        for &n in ch21.iter() {
            let s = g.cells[n as usize].sphere;
            let is_rail = s.1 < 0.9 && s.0[1] > 45.5 && s.0[1] < 47.5 && ((s.0[2] + 32.8).abs() < 0.8 || (s.0[2] + 26.2).abs() < 0.8);
            if is_rail { v.push(n); }
        }
        v.sort_unstable(); v.dedup(); v
    };
    assert_eq!(live40.len(), 40, "the live 40 resolve to {} port nodes", live40.len());
    // The game's courtyard draw set at VisCelCount = 2: the two rooms' own glists (drawn
    // unconditionally — the wall masonry and castle wall are Room_Courtyards_G's and the eye
    // room's own geometry) plus chain nodes that pass Vision_InView. The port's mask must keep
    // both rooms and every one of their materials.
    assert!(game[33] && game[21], "both rooms draw");
    for &room in &[33usize, 21] {
        let mats = sc.materials.iter().filter(|m| m.layer == scene::Layer::World && m.draw.pass != 4 && m.node == room as u32).count();
        assert!(mats > 0, "cell {room} carries World materials");
        let hidden = sc.materials.iter().filter(|m| m.layer == scene::Layer::World && m.draw.pass != 4 && m.node == room as u32 && !game[m.node as usize]).count();
        assert_eq!(hidden, 0, "cell {room}'s {mats} World materials must all be visible — the game draws the room's whole glist");
    }
    // No wall ROOM leaks into the visible set (the game's VisCelList is exactly {33, 21}).
    for &c in &[5usize, 13, 19, 20, 22, 23, 38] {
        assert!(!game[c], "wall room cell {c} must stay hidden at the courtyard (VisCelCount = 2)");
    }
    // fix-portal5's attribution pin: the courtyard generic cells own their own materials — the
    // World material count at each node equals its placement object's batch count. The misaligned
    // build keyed these materials under other placements' cells (379/765 instances shifted), so
    // this failed before the fix (RED first).
    let level = load_level(&iso, "07000006.bin");
    let mm = level.main_map().expect("main map");
    let cell_to_placement: std::collections::HashMap<u32, usize> = g.own_cells_main.iter().enumerate()
        .filter_map(|(pi, oc)| oc.map(|c| (c, pi))).collect();
    for &c in &[39usize, 40, 41, 42, 43, 57, 344, 345, 346] {
        let Some(&pi) = cell_to_placement.get(&(c as u32)) else { continue };
        let p = &level.maps[mm].placements[pi];
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < level.maps[mm].objects.len() { Some((mm, p.index as usize)) } else { None };
        let Some((om, oi)) = target else { continue };
        let want = level.maps[om].objects[oi].gfx.leaves.iter().map(|l| l.batches.len()).sum::<usize>();
        let got = sc.materials.iter().filter(|m| m.layer == scene::Layer::World && m.node == c as u32).count();
        assert_eq!(got, want, "cell {c}'s placement object must own its {want} batches as node-keyed materials (got {got})");
    }
    eprintln!("[pin] courtyard: rooms {{33, 21}} draw with all their materials, chain(21) = {} nodes (the live 40 present), extras {} (the helicopter parts + 4 railing dots, counted)", ch21.len(), ch21.len() - 40);
}
