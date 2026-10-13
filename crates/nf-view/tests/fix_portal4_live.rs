//! fix-portal4: the 07000006 courtyard wall cels against the main agent's live ground truth
//! (docs/live/2026-10-13-portal-ground-truth.md). The real game's main view at
//! `--bond 7.24,42.03,-39.31,10.161,0.428` holds EXACTLY two cels [L gdbcall 0x1e86e8,
//! frames 27567-27569]: the eye cel 0x0179f230 (class 0x0804c047, sphere (4.5, 43.5, -42.1)
//! r 11.4 — the port's cell 33 per fix-portal3) at depth 0, and 0x0179fb30 (class 0x0804c047,
//! sphere (-13.0, 49.0, -37.0) r 44.0) at depth 1 — one portal hop. 0x0179f230's +0x18 chain is
//! empty; 0x0179fb30's chain holds the 40 generic nodes tabulated in the ground-truth doc. The
//! game draws the courtyard walls from that room's own glist / chain; the port hides wall cells
//! 20/22/38/5/13/23. This file settles, from evidence:
//!   1. which port cell is 0x0179fb30 (sphere match);
//!   2. whether the port's walk reaches it from cell 33 at this camera;
//!   3. how the port's `CelGraph::chains` for that cell compares to the 40 live nodes;
//!   4. which of the port's wall cells are the live chain nodes.
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

/// The live main-view visible set at the courtyard spot [L ground-truth doc]: (cel class, sphere
/// centre, radius) — the eye cel 0x0179f230 and the big room 0x0179fb30 (depth 1).
const LIVE_EYE_SPHERE: ([f32; 3], f32) = ([4.5, 43.5, -42.1], 11.4);
const LIVE_BIG_SPHERE: ([f32; 3], f32) = ([-13.0, 49.0, -37.0], 44.0);

/// The big room's 40 live `+0x18` chain nodes [L ground-truth doc]: (class word, sphere centre,
/// radius, runtime address). The 28 railing nodes are abbreviated — the doc gives their span
/// (x -0.7..6.6 step 0.5 at y 46.5, z -32.8 / -26.2, r 0.4); they are matched by "railing node"
/// membership, not one-by-one.
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

fn near(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= tol)
}

/// The chain-builder rule, verified against the 05 start snapshot [L ee_start_05.bin]. The live
/// ground truth (07000006, gdb) showed: the big room 0x0179fb30's +0x18 chain holds 40 generic
/// nodes whose sphere centres ALL sit inside the room's world bbox, while the eye cel
/// 0x0179f230's chain is empty even though a generic sphere (3.0, 46.6, −32.8) sits inside its
/// world sphere (r 11.4) — that node's centre is 0.6 ABOVE the eye bbox's top face. Hypothesis:
/// a generic cel hangs on the room cel(s) whose world bbox contains the generic's sphere centre
/// (parse order preserved). This probe walks the snapshot's world cel list (+0x14), every cel's
/// +0x18 chain, and checks the hypothesis against every chain in the level.
#[test]
fn probe_snapshot_chain_rule() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for name in ["ee_start_05.bin", "ee_snapshot.bin"] {
        let snap_path = ["work/", "../work/", "../../work/"].iter().map(|pre| std::path::PathBuf::from(format!("{pre}{name}"))).find(|d| d.exists());
        let Some(snap_path) = snap_path else { eprintln!("[rule] {name} not found"); continue };
        let Ok(snap) = std::fs::read(&snap_path) else { eprintln!("[rule] {name} unreadable"); continue };
        let u32at = |a: usize| u32::from_le_bytes(snap[a..a + 4].try_into().unwrap());
        let f32at = |a: usize| f32::from_le_bytes(snap[a..a + 4].try_into().unwrap());
        let glb_world = u32at(0x0030d0f8) as usize;
        let mut cels: Vec<usize> = Vec::new();
        let mut cel = u32at(glb_world) as usize;
        while cel != 0 && cels.len() < 800 && cel + 0x90 <= snap.len() {
            cels.push(cel);
            cel = u32at(cel + 0x14) as usize;
        }
        let class = |c: usize| u32at(c + 0x3c);
        // The record's world bbox [L ee_start_05.bin cel 0x16f12a0 vs the port's cell 136]: two
        // vec4s — min.xyz at +0x40 (+0x4c pad 0), max.xyz at +0x50 (+0x5c pad 0).
        let bbox = |c: usize| ([f32at(c + 0x40), f32at(c + 0x44), f32at(c + 0x48)], [f32at(c + 0x50), f32at(c + 0x54), f32at(c + 0x58)]);
        let centre = |c: usize| [f32at(c + 0x80), f32at(c + 0x84), f32at(c + 0x88)];
        let inside = |p: [f32; 3], bb: ([f32; 3], [f32; 3])| {
            (0..3).all(|k| p[k] >= bb.0[k] - 1e-3 && p[k] <= bb.1[k] + 1e-3)
        };
        // Walk every cel's +0x18 chain [M 0x1e6ba0: the head is cel+0x18 and each node's NEXT is
        // node+0x18 — the chain threads through the cel records' +0x18 field; the VisCelList
        // threads through +0x20].
        // Chains threaded through +0x18 [M 0x1e6ba0]: a member's +0x18 is its SUCCESSOR, so the
        // real chain owners are the cels with a successor that nobody points AT (no predecessor),
        // and the members are the cels with a predecessor. Build both, then test the containment
        // rule on true chains only.
        let mut node_set: std::collections::HashSet<usize> = Default::default();
        let mut max_owner_count = 0usize;
        let mut max_owner: usize = 0;
        let mut chain_of: std::collections::HashMap<usize, Vec<usize>> = Default::default();
        let mut succ: std::collections::HashMap<usize, usize> = Default::default();
        let mut has_pred: std::collections::HashSet<usize> = Default::default();
        for (ci, &c) in cels.iter().enumerate() {
            let mut nodes: Vec<usize> = Vec::new();
            let mut node = u32at(c + 0x18) as usize;
            let mut hops = 0usize;
            while node != 0 && hops < 600 && node + 0x90 <= snap.len() && !nodes.contains(&node) {
                nodes.push(node);
                node_set.insert(node);
                node = u32at(node + 0x18) as usize;
                hops += 1;
            }
            if let Some(&first) = nodes.first() {
                succ.insert(c, first);
                has_pred.insert(first);
                for w in nodes.windows(2) { has_pred.insert(w[1]); }
            }
            if nodes.is_empty() { continue; }
            chain_of.insert(c, nodes.clone());
            if nodes.len() > max_owner_count { max_owner_count = nodes.len(); max_owner = ci; }
        }
        let mut true_owner_classes: std::collections::BTreeMap<u32, usize> = Default::default();
        let mut member_count = 0usize;
        let mut member_classes: std::collections::BTreeMap<u32, usize> = Default::default();
        for &c in &cels {
            let is_owner = succ.contains_key(&c) && !has_pred.contains(&c);
            if is_owner { *true_owner_classes.entry(class(c)).or_default() += 1; }
            if has_pred.contains(&c) {
                member_count += 1;
                *member_classes.entry(class(c)).or_default() += 1;
            }
        }
        // Containment of members in the owner's bbox, plus the tie rule for members inside more
        // than one room's bbox, plus the orphans (generics in no chain): are they in no room bbox?
        let mut violations = 0usize;
        let mut viol_samples: Vec<String> = Vec::new();
        let mut multi = 0usize; let mut owner_is_first = 0usize; let mut owner_is_last = 0usize;
        let mut owner_neither = 0usize;
        let mut orphans_outside = 0usize; let mut orphans_inside = 0usize;
        let mut samples: Vec<String> = Vec::new();
        let rooms: Vec<usize> = cels.iter().copied().filter(|&c| (class(c) & 0xffff) == 0xc047).collect();
        let chain_members: std::collections::HashSet<usize> = has_pred.iter().copied().collect();
        for (&owner, nodes) in chain_of.iter() {
            if has_pred.contains(&owner) { continue; } // not a true owner
            let obb = bbox(owner);
            for &n in nodes {
                let nc = centre(n);
                if !inside(nc, obb) {
                    violations += 1;
                    if viol_samples.len() < 6 {
                        viol_samples.push(format!("owner 0x{owner:x} (class {:08x}) bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) <- node 0x{n:x} centre ({:.1},{:.1},{:.1})",
                            class(owner), obb.0[0], obb.0[1], obb.0[2], obb.1[0], obb.1[1], obb.1[2], nc[0], nc[1], nc[2]));
                    }
                }
            }
        }
        for &n in &has_pred {
            let nc = centre(n);
            let containers: Vec<usize> = rooms.iter().copied().filter(|&r| inside(nc, bbox(r))).collect();
            if containers.len() > 1 {
                multi += 1;
                // The TRUE owner: the no-predecessor cel whose chain reaches n.
                let owner = chain_of.iter().find(|(&c, ns)| !has_pred.contains(&c) && ns.contains(&n)).map(|(&c, _)| c);
                let oidx = owner.and_then(|o| containers.iter().position(|&r| r == o));
                match oidx {
                    Some(0) => owner_is_first += 1,
                    Some(k) if k == containers.len() - 1 => owner_is_last += 1,
                    Some(_) => owner_neither += 1,
                    None => owner_neither += 1,
                }
                if samples.len() < 12 {
                    let vol = |c: usize| { let bb = bbox(c); (bb.1[0] - bb.0[0]) * (bb.1[1] - bb.0[1]) * (bb.1[2] - bb.0[2]) };
                    samples.push(format!("node 0x{n:x} ({:.1},{:.1},{:.1}): containers {} (vol {}) owner {:?} (room #{})",
                        nc[0], nc[1], nc[2],
                        containers.iter().map(|&r| format!("0x{r:x}")).collect::<Vec<_>>().join(","),
                        containers.iter().map(|&r| format!("{:.0}", vol(r))).collect::<Vec<_>>().join("/"),
                        owner.map(|o| format!("0x{o:x}")), oidx.map(|k| k.to_string()).unwrap_or_else(|| "?".into())));
                }
            }
        }
        for &c in &cels {
            if (class(c) & 0xffff) != 0x8019 || chain_members.contains(&c) || succ.contains_key(&c) { continue; }
            let nc = centre(c);
            if rooms.iter().any(|&r| inside(nc, bbox(r))) { orphans_inside += 1; } else { orphans_outside += 1; }
        }
        eprintln!("[rule] {name}: multi-bbox members {multi} (owner-first {owner_is_first}, owner-last {owner_is_last}, neither {owner_neither}), orphan generics outside-any-room {orphans_outside}, inside-a-room {orphans_inside}");
        for s in &samples { eprintln!("[rule] {name}: MULTI {s}"); }
        // Empirical bbox layout: match snapshot room cels to the port's parsed cells by world
        // sphere (+0x80..0x8c) and print the record's +0x40..0x60 raw floats against the port's
        // known world bbox.
        for lvl in ["07000005.bin", "07000007.bin"] {
            let Ok(sc) = nf_view::load(&iso, lvl, false) else { continue };
            let g = &sc.cel_graph;
            let mut matched = 0usize;
            for (ci, &c) in cels.iter().enumerate() {
                let sc3 = centre(c);
                let sr = f32at(c + 0x8c);
                let Some(pi) = g.cells.iter().position(|p| near(p.sphere.0, sc3, 0.35) && (p.sphere.1 - sr).abs() < 0.5) else { continue };
                matched += 1;
                if matched <= 6 && !has_pred.contains(&c) {
                    let raw: Vec<f32> = (0..8).map(|k| f32at(c + 0x40 + 4 * k)).collect();
                    eprintln!("[rule] {name} {lvl}: snapshot cel #{ci} 0x{c:x} class {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1} | port cell {pi} bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) | rec+0x40 {:?}",
                        class(c), sc3[0], sc3[1], sc3[2], sr, g.cells[pi].min[0], g.cells[pi].min[1], g.cells[pi].min[2], g.cells[pi].max[0], g.cells[pi].max[1], g.cells[pi].max[2], raw);
                }
            }
            eprintln!("[rule] {name}: {matched} snapshot cels sphere-matched to {lvl}'s port cells");
        }
        let cel_set: std::collections::HashSet<usize> = cels.iter().copied().collect();
        let nodes_also_cels = node_set.iter().filter(|n| cel_set.contains(n)).count();
        eprintln!("[rule] {name}: world cels {} (also-nodes {nodes_also_cels}), TRUE owners {} ({:08x?}), members {member_count} ({:08x?}), max owner count {max_owner_count} (owner #{max_owner})",
            cels.len(), true_owner_classes.values().sum::<usize>(), true_owner_classes, member_classes);
        eprintln!("[rule] {name}: containment violations {violations}");
        for s in &viol_samples { eprintln!("[rule] {name}: VIOLATION {s}"); }
    }
}

/// The pin [L ground-truth doc + M 0x1e6ba0/0x1c8cb0]: at the exact courtyard spot the port's
/// visible room set is {cell 33, cell 21 (= 0x0179fb30)}, cell 33's chain is EMPTY, and cell 21's
/// chain carries the live 40 courtyard generics — the 12 named nodes by sphere and the 28-node
/// railing rows — while the old per-map-block parse-order chain's far-away nodes are gone
/// (sister). The 9 extra nodes the port's FindCel tie-break still hangs on 21 (the game hangs
/// them on the wall rooms — the collision-assignment divergence) are counted, not asserted away.
#[test]
fn the_courtyard_visible_set_and_chain_match_the_live_ground_truth() {
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
    assert_eq!(cell, 33, "the eye cel resolves to the port's cell 33 (the live 0x0179f230)");
    let chain = scene::portal_cel_chain_from_cell(g, cell, eye, &depth0).expect("walk runs");
    assert_eq!(chain, vec![33usize, 21], "the walk visits exactly the live two-room set (VisCelCount = 2)");
    let game = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("mask");
    assert!(game[33] && game[21], "both rooms draw");
    // The eye cel's chain is empty [L ground-truth doc].
    assert!(g.chains[33].is_empty(), "cell 33's +0x18 chain must be empty — the live eye cel's is");
    // The big room's chain carries the live 40: the 12 named nodes by sphere [L ground-truth doc].
    let ch21 = &g.chains[21];
    for (k, (c3, r, addr)) in LIVE_BIG_CHAIN_HEAD.iter().enumerate() {
        assert!(ch21.iter().any(|&n| { let s = g.cells[n as usize].sphere; near(s.0, *c3, 0.8) && (s.1 - r).abs() < 1.2 }),
            "live chain node {k} ({addr}) missing from the port's chain for cell 21");
    }
    // ... and the 28 railing nodes: r < 0.9 at y 46.5 on the two railing lines.
    let railings = ch21.iter().filter(|&&n| {
        let s = g.cells[n as usize].sphere;
        s.1 < 0.9 && s.0[1] > 45.5 && s.0[1] < 47.5 && ((s.0[2] + 32.8).abs() < 0.8 || (s.0[2] + 26.2).abs() < 0.8)
    }).count();
    assert_eq!(railings, 28, "the port's chain holds {railings} railing nodes — the live chain holds 28");
    // Sister: the old per-map-block parse-order chain hung far-away generics on cell 21 (node 58,
    // sphere (88.1, 35.7, 28.0) r 13.5, the other end of the map, was in the pre-fix chain) — the
    // bbox+FindCel rule must not.
    let far = g.cells.iter().position(|c| near(c.sphere.0, [88.1, 35.7, 28.0], 1.0)).expect("node 58 exists");
    assert!(!ch21.contains(&(far as u32)), "the far generic (old model's chain node) must not hang on cell 21");
    eprintln!("[pin] chain(21) = {} nodes: the live 40 all present, {railings} railings; extras {}",
        ch21.len(), ch21.len() - 40);
}

/// The evidence probe (log-only): the port's cell table against the live spheres, the big room's
/// chain vs the live 40, and the walk's reach from cell 33 at the exact camera.
#[test]
fn probe_portal4_ground_truth_match() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let g = &sc.cel_graph;
    eprintln!("cells {} portals {} dropped {}", g.cells.len(), g.portals.len(), g.dropped_portals);

    // 1. The eye cel and the big room, by sphere.
    for (name, want) in [("eye", LIVE_EYE_SPHERE), ("big", LIVE_BIG_SPHERE)] {
        let mut hits: Vec<(usize, f32)> = g.cells.iter().enumerate()
            .map(|(i, c)| { let d = (c.sphere.0[0] - want.0[0]).abs() + (c.sphere.0[1] - want.0[1]).abs() + (c.sphere.0[2] - want.0[2]).abs(); (i, d) })
            .filter(|(_, d)| *d < 3.0)
            .collect();
        hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for (i, d) in hits.iter().take(4) {
            let c = &g.cells[*i];
            eprintln!("[{name}] cell {i} L1 {d:.2}: flags {:08x} sphere ({:.1}, {:.1}, {:.1}) r {:.1} bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1})",
                c.flags, c.sphere.0[0], c.sphere.0[1], c.sphere.0[2], c.sphere.1, c.min[0], c.min[1], c.min[2], c.max[0], c.max[1], c.max[2]);
        }
    }

    // 2. The wall cells: what are they (class, sphere, chain membership)?
    for i in [5usize, 13, 19, 20, 22, 23, 38] {
        let c = &g.cells[i];
        let owners: Vec<usize> = g.chains.iter().enumerate().filter(|(_, ch)| ch.contains(&(i as u32))).map(|(o, _)| o).collect();
        eprintln!("[wall] cell {i}: flags {:08x} sphere ({:.1}, {:.1}, {:.1}) r {:.1} bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) chain-owners {:?}",
            c.flags, c.sphere.0[0], c.sphere.0[1], c.sphere.0[2], c.sphere.1, c.min[0], c.min[1], c.min[2], c.max[0], c.max[1], c.max[2], owners);
    }

    // 3. The big room's chain vs the live 40.
    let big: Option<usize> = g.cells.iter().position(|c| near(c.sphere.0, LIVE_BIG_SPHERE.0, 1.5) && (c.sphere.1 - LIVE_BIG_SPHERE.1).abs() < 2.0);
    match big {
        Some(b) => {
            let ch = &g.chains[b];
            eprintln!("[big] cell {b}: port chain {} nodes; live 40", ch.len());
            let mut live_hit = 0;
            for (k, (c3, r, addr)) in LIVE_BIG_CHAIN_HEAD.iter().enumerate() {
                let m = ch.iter().position(|&n| { let s = g.cells[n as usize].sphere; near(s.0, *c3, 0.8) && (s.1 - r).abs() < 1.2 });
                if m.is_some() { live_hit += 1; }
                eprintln!("[big] live {k} {addr} ({:.1}, {:.1}, {:.1}) r {r}: {}", c3[0], c3[1], c3[2], if m.is_some() { format!("port chain node {}", ch[m.unwrap()]) } else { "NO port chain node".into() });
            }
            eprintln!("[big] live-head matched {live_hit}/12");
            // The port's chain nodes that match NO live head entry: count railing-ish (r<1) and others.
            let mut unmatched_other: Vec<u32> = Vec::new();
            let mut rail = 0usize;
            for &n in ch {
                let s = &g.cells[n as usize].sphere;
                let head_hit = LIVE_BIG_CHAIN_HEAD.iter().any(|(c3, r, _)| near(s.0, *c3, 0.8) && (s.1 - *r).abs() < 1.2);
                let is_rail = s.1 < 0.9 && s.0[1] > 45.0 && s.0[1] < 48.0 && (s.0[2] + 32.8).abs() < 0.8 || (s.1 < 0.9 && s.0[1] > 45.0 && s.0[1] < 48.0 && (s.0[2] + 26.2).abs() < 0.8);
                if head_hit { continue; }
                if is_rail { rail += 1; } else { unmatched_other.push(n); }
            }
            eprintln!("[big] port chain: railing-like {rail}, unmatched others {:?} (spheres below)", unmatched_other);
            for &n in &unmatched_other {
                let s = &g.cells[n as usize].sphere;
                eprintln!("[big]   extra node {n}: flags {:08x} sphere ({:.2}, {:.2}, {:.2}) r {:.2}", g.cells[n as usize].flags, s.0[0], s.0[1], s.0[2], s.1);
            }
        }
        None => eprintln!("[big] NO port cell matches the live big-room sphere"),
    }

    // 4. The walk from the eye cel at the exact camera.
    let world = sc.sim.as_ref().map(|s| &s.world);
    let eye = [7.24f32, 42.73, -39.31]; // the render-path camera y (--bond y + the eye height the path uses)
    let vp = view_proj(eye, 10.161, 0.428 * std::f32::consts::FRAC_PI_2, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(g, &mut state, eye, world).expect("resolves");
    eprintln!("[walk] eye cell {cell} (live: the eye cel, port 33)");
    let chain = scene::portal_cel_chain_from_cell(g, cell, eye, &depth0).expect("walk runs");
    eprintln!("[walk] visited {} cels: {:?}", chain.len(), chain);
    if let Some(b) = big {
        eprintln!("[walk] big room cell {b} visited: {}", chain.contains(&b));
    }
    let game = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("mask");
    let walls = [19usize, 20, 22, 38, 5, 13, 23];
    eprintln!("[walk] wall cells kept by walk+chain: {:?}", walls.iter().copied().filter(|c| game[*c]).collect::<Vec<_>>());
    if let Some(b) = big {
        let vis = game[b];
        eprintln!("[walk] big room cell {b} in game mask: {vis}");
    }

    // Which nodes own the World materials in the wall regions (the port's wall-room bboxes), and
    // are those nodes visible under the fixed chains? The game draws the walls with VisCelCount=2,
    // so their geometry must ride cell 21's chain (or 21/33's own glists) — this prints where the
    // port's parse put it.
    let wall_regions: [([f32; 3], [f32; 3]); 3] = [
        (g.cells[22].min, g.cells[22].max), // the masonry under the left walkway
        (g.cells[20].min, g.cells[20].max), // the castle wall SE
        (g.cells[38].min, g.cells[38].max), // the castle wall S
    ];
    let chain21: Vec<u32> = big.map(|b| g.chains[b].clone()).unwrap_or_default();
    let mut by_node: std::collections::BTreeMap<u32, usize> = Default::default();
    for m in sc.materials.iter().filter(|m| m.layer == scene::Layer::World && m.draw.pass != 4) {
        if wall_regions.iter().any(|(lo, hi)| (0..3).all(|k| m.centre[k] >= lo[k] && m.centre[k] <= hi[k])) {
            *by_node.entry(m.node).or_default() += 1;
        }
    }
    let mut total = 0usize;
    for (node, count) in &by_node {
        let n = *node as usize;
        let (vis, who) = if n < g.cells.len() {
            let in_chain = chain21.contains(&(*node));
            (game.get(n).copied().unwrap_or(false), if in_chain { "chain(21)" } else if game.get(n).copied().unwrap_or(false) { "walk" } else { "hidden" })
        } else { (false, "??") };
        total += count;
        eprintln!("[mats] node {node} ({:08x}) sphere ({:.1},{:.1},{:.1}) r {:.1}: {count} wall-region mats, visible {vis} ({who})",
            g.cells.get(n).map_or(0, |c| c.flags), g.cells.get(n).map_or([0.0;3], |c| c.sphere.0)[0], g.cells.get(n).map_or([0.0;3], |c| c.sphere.0)[1], g.cells.get(n).map_or([0.0;3], |c| c.sphere.0)[2], g.cells.get(n).map_or(0.0, |c| c.sphere.1));
    }
    eprintln!("[mats] {total} wall-region World materials over {} nodes; visible {}", by_node.len(),
        by_node.keys().filter(|n| game.get(**n as usize).copied().unwrap_or(false)).count());
}
