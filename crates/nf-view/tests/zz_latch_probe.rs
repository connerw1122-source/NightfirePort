//! zz_latch_probe (scratch — uncommitted): WHY do only two of the four Lock_Create (kind 0x2E)
//! hinge latches on 0700000c's vent hatch draw? NOT a test — diagnostics with eprintln; run with
//! --nocapture.
//!
//! Replicates, with the port's own pub fns/data:
//!   1. the MISC sweep's skip gates (scene.rs ~4950-5033) per 0x2E placement;
//!   2. add_instance's group-centre computation + room-cel assignment (scene.rs 4008/4037:
//!      centre = transform(matrix, obj.centre), cel = find_cel(room_cels, centre));
//!   3. gpu.rs's draw-time World culls for chain=u32::MAX draws: the A-11e frustum sphere test
//!      (gpu.rs 780) and the strict room-cel mask from the portal walk (gpu.rs 758-793 +
//!      cel_visibility gpu.rs 531-563), from the walkway camera at (5.8, 15.8, -107.5)
//!      looking at the hatch at (5.8, 16.3, -112.3).

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_formats::placement::{object_matrix, transform};
use nf_view::scene::{
    eye_cell, find_cel, near_los_rescue, portal_cel_chain_from_cell,
    portal_visible_cels_game_from_cell, vision_in_view, ViewerCel,
};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

const MISC_KINDS: [u8; 15] = [0x2e, 0x30, 0x34, 0x35, 0x3b, 0x71, 0xd5, 0xd8, 0xe1, 0xe3, 0xda, 0xfa, 0xfd, 0xfe, 0x4a];
const FLAGS_COPYING: [u8; 11] = [0x30, 0x34, 0x35, 0x3b, 0x71, 0xda, 0xe1, 0xe3, 0xfa, 0xfd, 0x4a];

type M4 = [[f32; 4]; 4]; // column-major: m[col][row] (glam's to_cols_array_2d layout)

fn mul(a: &M4, b: &M4) -> M4 {
    let mut o = [[0.0f32; 4]; 4];
    for c in 0..4 { for r in 0..4 { o[c][r] = (0..4).map(|k| a[k][r] * b[c][k]).sum(); } }
    o
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: [f32; 3]) -> [f32; 3] { let l = dot(a, a).sqrt().max(1e-12); [a[0] / l, a[1] / l, a[2] / l] }

/// lib.rs Camera::view_proj (fov π/3 = GAME_FOV, near 0.05, far 4000, wgpu depth 0..1) for eye→target.
fn view_proj(eye: [f32; 3], target: [f32; 3], fov: f32, aspect: f32) -> M4 {
    let d = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
    let len = dot(d, d).sqrt();
    let yaw = d[0].atan2(d[2]);
    let pitch = (d[1] / len).clamp(-1.0, 1.0).asin();
    let fw = norm([pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()]);
    let s = norm(cross(fw, [0.0, 1.0, 0.0]));
    let u = cross(s, fw);
    let (near, far) = (0.05f32, 4000.0f32);
    let f = 1.0 / (fov * 0.5).tan();
    let proj: M4 = [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
    ];
    let view: M4 = [
        [s[0], u[0], -fw[0], 0.0],
        [s[1], u[1], -fw[1], 0.0],
        [s[2], u[2], -fw[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(fw, eye), 1.0],
    ];
    mul(&proj, &view)
}

/// gpu.rs extract_frustum (Gribb-Hartmann; near = row3+row2, far = row3−row2; inside ≥ 0).
fn extract_frustum(vp: &M4) -> [[f32; 4]; 6] {
    let row = |i: usize| [vp[0][i], vp[1][i], vp[2][i], vp[3][i]];
    let r3 = row(3);
    let combos = [(0usize, 1.0f32), (0, -1.0), (1, 1.0), (1, -1.0), (2, 1.0), (2, -1.0)];
    let mut planes = [[0.0f32; 4]; 6];
    for (k, (i, sgn)) in combos.iter().enumerate() {
        let ri = row(*i);
        let p = [r3[0] + sgn * ri[0], r3[1] + sgn * ri[1], r3[2] + sgn * ri[2], r3[3] + sgn * ri[3]];
        let n = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-12);
        planes[k] = [p[0] / n, p[1] / n, p[2] / n, p[3] / n];
    }
    planes
}

/// gpu.rs depth0_game: the four fov side planes + the 200-unit near quad (normal back at the eye).
fn depth0_game(planes: &[[f32; 4]; 6], eye: [f32; 3]) -> [[f32; 4]; 5] {
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}

fn sphere_outside(planes: &[[f32; 4]; 6], c: [f32; 3], r: f32) -> bool {
    planes.iter().any(|p| p[0] * c[0] + p[1] * c[1] + p[2] * c[2] + p[3] < -r)
}

fn pt(p: [f32; 3]) -> String { format!("({:.3},{:.3},{:.3})", p[0], p[1], p[2]) }

// Scratch probe kept for its data citations (109n); no assertions — not part of the suite run.
#[ignore]
#[test]
fn probe_latch_skip_reasons() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set; skipping"); return };
    let level = Level::parse(&read_archive(&iso, "0700000c.bin")).expect("parse 0700000c");
    let m = level.main_map().expect("main map");
    let scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let cels = scene.cels.clone();
    let cg = scene.cel_graph.clone();
    let world = scene.sim.as_ref().map(|s| &s.world);
    eprintln!("== scene: {} room cels, {} cells, {} portals, dropped_portals {} ==",
        cels.len(), cg.cells.len(), cg.portals.len(), cg.dropped_portals);
    eprintln!("scene.misc_drawn = {:?} (0x2e = {:?})", scene.misc_drawn, scene.misc_drawn.get(&0x2e));

    // ---------- (a) every 0x2E placement in the main map ----------
    eprintln!("\n== (a) main-map placements with (flags & 0xff) == 0x2e ==");
    let mut latch_pis: Vec<usize> = Vec::new();
    for (pi, p) in level.maps[m].placements.iter().enumerate() {
        if p.flags & 0xff != 0x2e { continue; }
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
        let (tag, obj) = match target {
            Some((tm, to)) => (format!("map {tm} obj #{to}"), Some(&level.maps[tm].objects[to])),
            None => ("RESOLUTION-FAILED".to_string(), None),
        };
        match &obj {
            Some(o) => {
                let leaves = o.gfx.leaves.len();
                let leaves_with_batches = o.gfx.leaves.iter().filter(|l| !l.batches.is_empty()).count();
                let batches: usize = o.gfx.leaves.iter().map(|l| l.batches.len()).sum();
                eprintln!("#{pi} idx {} hash {:08x} by_hash {} creates_cel {} flags {:#010x} trans {} rot {} params {:?} -> {tag} \"{}\" obj.flags {:#x} leaves {leaves} (with batches {leaves_with_batches}) batches {batches} centre {} r {:.3} min {} max {}",
                    p.index, p.hashcode, p.by_hash(), p.creates_cel(), p.flags, pt(p.translation), pt(p.rotation), p.params, o.name.trim(), o.flags, pt(o.centre), o.radius, pt(o.min), pt(o.max));
            }
            None => eprintln!("#{pi} idx {} hash {:08x} by_hash {} creates_cel {} flags {:#010x} trans {} rot {} params {:?} -> {tag}",
                p.index, p.hashcode, p.by_hash(), p.creates_cel(), p.flags, pt(p.translation), pt(p.rotation), p.params),
        }
        if p.flags & 0xff == 0x2e { latch_pis.push(pi); }
    }

    // duplicate-hash census: every map holding an object with one of the latch hashcodes.
    let hashes: Vec<u32> = latch_pis.iter().map(|&pi| level.maps[m].placements[pi].hashcode).collect();
    eprintln!("\n== (a2) hash census: objects with a latch hashcode, over ALL maps (by_hash picks ONE) ==");
    for h in hashes.iter().collect::<std::collections::BTreeSet<_>>() {
        eprintln!(" hash {h:08x}: by_hash -> {:?}", level.by_hash.get(h));
        for (mi, map) in level.maps.iter().enumerate() {
            for (oi, o) in map.objects.iter().enumerate() {
                if o.hashcode == *h {
                    let with_batches = o.gfx.leaves.iter().filter(|l| !l.batches.is_empty()).count();
                    eprintln!("   map {mi} obj #{oi} \"{}\" flags {:#x} leaves-with-batches {with_batches} centre {}", o.name.trim(), o.flags, pt(o.centre));
                }
            }
        }
    }

    // ---------- (b) the sweep's skip-gate evaluation for the hinge latches ----------
    eprintln!("\n== (b) MISC-sweep gate evaluation (scene.rs 4950-5033) for hinge latches ==");
    for &pi in &latch_pis {
        let p = &level.maps[m].placements[pi];
        let is_latch = p.index >= 139 && p.index <= 142
            || level.by_hash.get(&p.hashcode).map(|&(tm, to)| level.maps[tm].objects[to].name.to_lowercase().contains("hingelock")).unwrap_or(false)
            || p.translation[2] < -109.0; // anywhere near/at the hatch wall
        if !is_latch { continue; }
        let kind = (p.flags & 0xff) as u8;
        let gate_3b = kind == 0x3b && p.params.iter().find(|(i, _)| *i == 0).is_some_and(|&(_, v)| v != 0);
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
        let Some((tm, to)) = target else {
            eprintln!("#{pi}: SWEEP-SKIP resolution-failed (by_hash {} hash {:08x} index-field {})", p.by_hash(), p.hashcode, p.index);
            continue;
        };
        let o = &level.maps[tm].objects[to];
        let f10 = o.flags & 0x10 != 0;
        let f400 = o.flags & 0x400 != 0;
        let f800 = o.flags & 0x800 != 0;
        let f1 = o.flags & 1 != 0;
        let gfx_empty = !o.gfx.leaves.iter().any(|l| !l.batches.is_empty());
        let gate_copy = f10 && FLAGS_COPYING.contains(&kind);
        let nf_hide = std::env::var("NF_HIDE").ok();
        let gate_hide = nf_hide.as_ref().map(|h| h.split(',').any(|n| !n.is_empty() && o.name.contains(n))).unwrap_or(false);
        let drawn = !(gate_3b || gate_copy || gfx_empty || f400 || gate_hide);
        eprintln!("#{pi} \"{}\" (map {tm} obj #{to}): kind {kind:02x} 0x10 {f10} (copying-gate {gate_copy}) 0x400 {f400} 0x800 {f800} 0x1(alpha-list) {f1} gfx-empty {gfx_empty} NF_HIDE {nf_hide:?} hide-gate {gate_hide} 3b-gate {gate_3b} => SWEEP {}",
            o.name.trim(), if drawn { "DRAWS (add_instance called)" } else { "SKIPS" });
    }

    // ---------- (c)+(d) the render-side cull: centre -> room cel -> cell -> walk visibility ----------
    eprintln!("\n== (c) per-latch add_instance centre (scene.rs 4008) -> find_cel room cel (4037) -> room_to_cell cell ==");
    struct Latch { pi: usize, centre: [f32; 3], radius: f32, rc: Option<usize>, cell: Option<usize>, name: String, creates_cel: bool, node: u32 }
    let mut latches: Vec<Latch> = Vec::new();
    for &pi in &latch_pis {
        let p = &level.maps[m].placements[pi];
        let Some((tm, to)) = (if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None }) else { continue };
        let o = &level.maps[tm].objects[to];
        let mat = object_matrix(p.rotation, p.translation);
        let centre = transform(&mat, o.centre);
        let col_len = |c: usize| (mat[0][c] * mat[0][c] + mat[1][c] * mat[1][c] + mat[2][c] * mat[2][c]).sqrt();
        let radius = o.radius * col_len(0).max(col_len(1)).max(col_len(2));
        let rc = find_cel(&cels, centre);
        let cell = rc.and_then(|i| cg.room_to_cell.get(i).copied().flatten());
        // all containing room cels (find_cel takes the smallest volume)
        let mut cands: Vec<(usize, f32)> = Vec::new();
        for (i, c) in cels.iter().enumerate() {
            let inside = (0..3).all(|k| c.min[k] <= centre[k] && centre[k] <= c.max[k]);
            if inside { cands.push((i, (c.max[0] - c.min[0]) * (c.max[1] - c.min[1]) * (c.max[2] - c.min[2]))); }
        }
        cands.sort_by(|a, b| a.1.total_cmp(&b.1));
        let cands_s: Vec<String> = cands.iter().take(6).map(|&(i, v)| format!("cel {i} vol {v:.2} x{}..{} y{}..{} z{}..{}",
            cels[i].min[0], cels[i].max[0], cels[i].min[1], cels[i].max[1], cels[i].min[2], cels[i].max[2])).collect();
        let node = cg.own_cells_main.get(pi).copied().flatten().unwrap_or(u32::MAX);
        eprintln!("#{pi} \"{}\": placement trans {} obj.centre {} -> group centre {} r {:.3}; containing room cels [{}]; picked cel {:?} -> cell {:?}; own_cells_main node {node:#x} (creates_cel {})",
            o.name.trim(), pt(p.translation), pt(o.centre), pt(centre), radius, cands_s.join(" | "), rc, cell, p.creates_cel());
        latches.push(Latch { pi, centre, radius, rc, cell, name: o.name.trim().to_string(), creates_cel: p.creates_cel(), node });
    }

    // cameras: given eye, the head-height eye, wide aspect variant
    let eye_given = [5.8f32, 15.8, -107.5];
    let eye_head = [5.8f32, 15.8 + 1.6, -107.5];
    let target = [5.8f32, 16.3, -112.3];
    for (label, eye, aspect) in [
        ("eye(5.8,15.8,-107.5) aspect 16:9", eye_given, 16.0f32 / 9.0),
        ("eye(5.8,15.8,-107.5) aspect 1.0", eye_given, 1.0),
        ("eye+1.6 head aspect 16:9", eye_head, 16.0 / 9.0),
    ] {
        eprintln!("\n== (c) camera [{label}] -> target (5.8,16.3,-112.3) ==");
        let vp = view_proj(eye, target, std::f32::consts::FRAC_PI_3, aspect);
        let planes = extract_frustum(&vp);
        let d0 = depth0_game(&planes, eye);
        let sides = [planes[0], planes[1], planes[2], planes[3]];
        let mut state = ViewerCel::default();
        let ec = eye_cell(&cg, &mut state, eye, world);
        match ec {
            Some(ec) => {
                let cn = &cg.cells[ec];
                eprintln!("eye cell {ec}: bbox x{}..{} y{}..{} z{}..{} flags {:#x} sphere {} r {:.2}",
                    cn.min[0], cn.max[0], cn.min[1], cn.max[1], cn.min[2], cn.max[2], cn.flags, pt(cn.sphere.0), cn.sphere.1);
                let chain_topo = portal_cel_chain_from_cell(&cg, ec, eye, &[[0.0f32; 4]; 5]).unwrap_or_default();
                let chain_real = portal_cel_chain_from_cell(&cg, ec, eye, &d0).unwrap_or_default();
                eprintln!("strict chain: topo (no depth-0 frustum) {} cels {:?}; real (game depth-0) {} cels {:?}",
                    chain_topo.len(), chain_topo, chain_real.len(), chain_real);
                // gpu.rs 560: room-cel mask = room_to_cell -> cell in the strict chain, unwrap_or(true)
                let room_vis = |chain: &Vec<usize>, rc: Option<usize>| -> Option<bool> {
                    let cell = rc.and_then(|i| cg.room_to_cell.get(i).copied().flatten())?;
                    Some(chain.contains(&cell))
                };
                // game mask (+ rescue) for the node != u32::MAX path
                let mut game = portal_visible_cels_game_from_cell(&cg, ec, eye, &d0, &sides);
                if let (Some(gm), Some(w)) = (game.as_mut(), world) { near_los_rescue(&cg, gm, eye, &sides, w, 30.0); }
                for l in &latches {
                    let frustum = sphere_outside(&planes, l.centre, l.radius);
                    let vis_topo = room_vis(&chain_topo, l.rc);
                    let vis_real = room_vis(&chain_real, l.rc);
                    let hidden_real = l.rc.is_some() && !vis_real.unwrap_or(true);
                    let hidden_topo = l.rc.is_some() && !vis_topo.unwrap_or(true);
                    // the node path only applies when the placement ALSO rides the static loop (creates_cel)
                    let node_hidden = l.node != u32::MAX && game.as_ref().map(|g| !g[l.node as usize]).unwrap_or(false);
                    let verdict = if frustum { "FRUSTUM-CULLED" }
                        else if hidden_real { if hidden_topo { "CEL-CULLED (cell not in walk — topological, frustum-independent)" } else { "CEL-CULLED (depth-0 frustum only)" } }
                        else if node_hidden { "NODE-CULLED (game mask)" } else { "SUBMITTED" };
                    eprintln!("  #{:<3} \"{}\" centre {} cel {:?} cell {:?}: frustum-out {frustum}, cell in topo {} / real {} ; node-game {} => {verdict}",
                        l.pi, l.name, pt(l.centre), l.rc, l.cell, vis_topo.unwrap_or(false), vis_real.unwrap_or(false),
                        game.as_ref().map(|g| l.node != u32::MAX && !g[l.node as usize]).unwrap_or(false));
                }
                // the walk's own Vision_InView on each latch's room-cel sphere, and the chain membership
                for l in &latches {
                    if let Some(cell) = l.cell {
                        let cn = &cg.cells[cell];
                        eprintln!("  latch #{} cell {cell}: bbox x{}..{} y{}..{} z{}..{} flags {:#x}; Vision_InView(sides) {}; in topo {} in real {}",
                            l.pi, cn.min[0], cn.max[0], cn.min[1], cn.max[1], cn.min[2], cn.max[2], cn.flags,
                            vision_in_view(cn.sphere.0, cn.sphere.1, &sides), chain_topo.contains(&cell), chain_real.contains(&cell));
                    }
                }
            }
            None => eprintln!("eye cell: NONE (eye outside every room cel) — renderer keeps the box test"),
        }
    }

    // ---------- (e) portals near the hatch wall ----------
    eprintln!("\n== (e) portals with a corner near the hatch plane (|z +112.25| < 3, |x - 5.8| < 8, y 13..20) ==");
    for (i, p) in cg.portals.iter().enumerate() {
        let cx = p.corners.iter().map(|c| c[0]).sum::<f32>() / 4.0;
        let cy = p.corners.iter().map(|c| c[1]).sum::<f32>() / 4.0;
        let cz = p.corners.iter().map(|c| c[2]).sum::<f32>() / 4.0;
        if (cz + 112.25).abs() < 3.0 && (cx - 5.8).abs() < 8.0 && (13.0..20.0).contains(&cy) {
            let fa = &cg.cells[p.cel_a];
            let fb = &cg.cells[p.cel_b];
            eprintln!("portal #{i}: a {} (flags {:#x} room {}) <-> b {} (flags {:#x} room {}) side-flags {:#04x} n_fwd {} d {:.3} corners {} {} {} {}",
                p.cel_a, fa.flags, fa.flags & 0x40000 != 0, p.cel_b, fb.flags, fb.flags & 0x40000 != 0, p.flags, pt(p.n_fwd), p.d_fwd,
                pt(p.corners[0]), pt(p.corners[1]), pt(p.corners[2]), pt(p.corners[3]));
        }
    }
    eprintln!("\n.done");
}
