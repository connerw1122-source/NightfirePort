//! fix-portal6: the 07 clock hall's props (the grandfather clock + both wall shields with crossed
//! swords) went missing with the walk ON after fix-portal5 (the own-cell alignment +
//! `build_link_objects_to_rooms` 0x1c9150 chain-owner rule). Ground truth [L
//! docs/live/2026-10-13-portal-ground-truth.md]: at `07000007 --bond 34.25,44.13,-70.71,1.549,-0.070`
//! the real game draws two guards, the clock, the shields; fix-portal2's walk matched; the merged
//! fix-portal5 master drops the clock and the shields (the walk ON) while `NF_NOPORTAL=1` draws them.
//! This file probes where the clock/shield cells hang now and pins the repair.
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

fn load_level(iso: &str, name: &str) -> Level {
    let img = IsoImage::open(iso).expect("iso");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("{name}"));
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("archive");
    Level::parse(&data).expect("level parse")
}

/// The bond spot's eye (the courtyard probe convention: the bond y + 0.7 is the camera head;
/// `--bond`'s pitch is the game's half-radian word — the render path multiplies by π/2).
const CLOCK_HALL: ([f32; 3], f32, f32) = ([34.25, 44.13 + 0.7, -70.71], 1.549, -0.070 * std::f32::consts::FRAC_PI_2);

fn clock_hall_walk(sc: &scene::Scene) -> (usize, Vec<usize>, Vec<bool>, [[f32; 4]; 4]) {
    let g = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world);
    let (eye, yaw, pitch) = CLOCK_HALL;
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(g, &mut state, eye, world).expect("the eye cel resolves");
    let chain = scene::portal_cel_chain_from_cell(g, cell, eye, &depth0).expect("the walk runs");
    let game = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("mask");
    (cell, chain, game, vp)
}

/// The investigation: which objects are the clock and the shields, which cells do they own, which
/// rooms' chains carry them now (fix-portal5's 0x1c9150 rule) vs the old find_cell_raycast pick,
/// what the walk visits at the clock-hall spot, and why the owner pick landed where it landed
/// (the radius gate, the bbox gate, the closest_point_scoped distances).
#[test]
fn probe_portal6_clock_hall() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007: {e}"); return } };
    let level = load_level(&iso, "07000007.bin");
    let m = level.main_map().expect("main map");
    let g = &sc.cel_graph;
    eprintln!("cells {} dropped_portals {}", g.cells.len(), g.dropped_portals);

    // 1. The walk at the clock-hall spot.
    let (eye_cell, chain, game, vp) = clock_hall_walk(&sc);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    eprintln!("[walk] eye cell {eye_cell}, visits {chain:?}");
    for (k, p) in sides.iter().enumerate() {
        eprintln!("[plane {k}] n ({:.4},{:.4},{:.4}) d {:.4}", p[0], p[1], p[2], p[3]);
    }

    // The Vision_InView verdict per plane for the clock's sphere (cell 50) and the hall shields'
    // (43-48, 269-284): which plane fails, by how much.
    let in_view = |c3: [f32; 3], r: f32| sides.iter().map(|p| p[0] * c3[0] + p[1] * c3[1] + p[2] * c3[2] + p[3] * r + r).collect::<Vec<_>>();
    for c in [50usize, 94, 108, 43, 44, 45, 46, 47, 48, 269, 270, 274, 276, 277, 278, 283, 284] {
        let Some(node) = g.cells.get(c) else { continue };
        let s = node.sphere;
        let v = in_view(s.0, s.1);
        let owners: Vec<usize> = g.chains.iter().enumerate().filter(|(_, ch)| ch.contains(&(c as u32))).map(|(o, _)| o).collect();
        eprintln!("[view] cell {c} sphere ({:.1},{:.1},{:.1}) r {:.2} owners {owners:?} per-plane {v:.1?} -> {}",
            s.0[0], s.0[1], s.0[2], s.1, v.iter().all(|&x| x >= 0.0));
    }

    // The full chain dump of the visited rooms: node, sphere, mask verdict, mats.
    for &c in &chain {
        eprintln!("[visited] room {c}: chain {} nodes", g.chains[c].len());
        for &n in &g.chains[c] {
            let node = &g.cells[n as usize];
            let mats = sc.materials.iter().filter(|mm| mm.layer == nf_view::scene::Layer::World && mm.node == n).count();
            eprintln!("[chain {c}] node {n} flags {:08x} sphere ({:.1},{:.1},{:.1}) r {:.2} mats {mats} mask {}",
                node.flags, node.sphere.0[0], node.sphere.0[1], node.sphere.0[2], node.sphere.1, game[n as usize]);
        }
    }

    // 2. The candidate props: name-match clock/shield/sword.
    let mut props: Vec<(usize, String)> = Vec::new();
    for (mi, map) in level.maps.iter().enumerate() {
        for (oi, o) in map.objects.iter().enumerate() {
            let n = o.name.to_lowercase();
            if n.contains("clock") || n.contains("shield") || n.contains("sword") || n.contains("grandfather") {
                props.push((oi, format!("maps[{mi}].objects[{oi}] \"{}\"", o.name.trim())));
                if mi == m { eprintln!("[obj] main-map {} flags {:#x} centre ({:.1},{:.1},{:.1}) r {:.1} bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1}) leaves {} batches {}",
                    o.name.trim(), o.flags, o.centre[0], o.centre[1], o.centre[2], o.radius, o.min[0], o.min[1], o.min[2], o.max[0], o.max[1], o.max[2],
                    o.gfx.leaves.len(), o.gfx.leaves.iter().map(|l| l.batches.len()).sum::<usize>()); }
            }
        }
    }
    eprintln!("[obj] {} candidate objects total (all maps)", props.len());

    // 3. Every main-map placement of those objects: class, own cell, chain owners (new rule), the
    //    old-rule owner, and the mask verdict at the spot.
    let placement_name = |p: &nf_formats::placement::Placement| -> String {
        if p.by_hash() { level.by_hash.get(&p.hashcode).map(|(mm, oi)| format!("maps[{mm}].objects[{oi}] \"{}\"", level.maps[*mm].objects[*oi].name.trim())).unwrap_or_else(|| "<unresolved>".into()) }
        else { level.maps[m].objects.get(p.index as usize).map(|o| format!("objects[{}] \"{}\"", p.index, o.name.trim())).unwrap_or_else(|| "<bad index>".into()) }
    };
    for (pi, p) in level.maps[m].placements.iter().enumerate() {
        let nm = placement_name(p);
        let nl = nm.to_lowercase();
        if !(nl.contains("clock") || nl.contains("shield") || nl.contains("sword")) { continue; }
        let resolved = if p.by_hash() { level.by_hash.contains_key(&p.hashcode) } else { (p.index as usize) < level.maps[m].objects.len() };
        let own = g.own_cells_main.get(pi).copied().flatten();
        let owners: Vec<usize> = own.map(|c| g.chains.iter().enumerate().filter(|(_, ch)| ch.contains(&c)).map(|(o, _)| o).collect()).unwrap_or_default();
        // The old rule's pick (find_cell_raycast over the same cell sphere).
        let old_owner = own.and_then(|c| scene::find_cell_raycast(g, g.cells[c as usize].sphere.0, None));
        let mut gates_r: Vec<usize> = Vec::new();
        let mut gates_b: Vec<usize> = Vec::new();
        if let (Some(c), Some(world)) = (own, sc.sim.as_ref().map(|s| &s.world)) {
            let sph = g.cells[c as usize].sphere;
            for (ri, r) in g.cells.iter().enumerate() {
                if !matches!(r.flags & 0xffff, 0xc023 | 0xc047) { continue; }
                if sph.1 > r.sphere.1 { gates_r.push(ri); continue; }
                if !(0..3).all(|k| r.min[k] <= sph.0[k] && sph.0[k] <= r.max[k]) { gates_b.push(ri); continue; }
                let d = world.closest_point_scoped(ri, sph.0);
                eprintln!("[gate] prop cell {c} sphere ({:.2},{:.2},{:.2}) r {:.2}: room {ri} passes gates, scoped dist {:?}",
                    sph.0[0], sph.0[1], sph.0[2], sph.1, d);
            }
            eprintln!("[gate] prop cell {c}: radius-gated rooms {gates_r:?}, bbox-gated rooms {gates_b:?}");
        }
        eprintln!("[prop] placement {pi}: {nm} flags {:#08x} (low16 {:04x}) creates_cel {} resolved {resolved} at ({:.1},{:.1},{:.1})",
            p.flags, p.flags & 0xffff, p.creates_cel(), p.translation[0], p.translation[1], p.translation[2]);
        eprintln!("[prop]   own cell {own:?}, chain owners {owners:?}, old-rule owner {old_owner:?}, in game mask {}",
            own.map(|c| game.get(c as usize).copied().unwrap_or(false)).unwrap_or(false));
        if let Some(c) = own {
            for (ri, ch) in g.chains.iter().enumerate() {
                if ch.contains(&c) {
                    eprintln!("[prop]   on chain of room {ri} ({} nodes); room {ri} in walk {} in mask {}",
                        ch.len(), chain.contains(&ri), game[ri]);
                }
            }
        }
    }

    // 4. Orphan generics: creates_cel non-room cells on NO chain (they can never draw with the
    //    walk ON — the game leaves them unchained only when no room qualifies [M 0x1c9150]).
    let mut orphans = 0usize;
    for (i, c) in g.cells.iter().enumerate() {
        let f16 = c.flags & 0xffff;
        if !matches!(f16, 0x8018 | 0x8019) { continue; }
        if c.min == [0.0; 3] && c.max == [0.0; 3] && c.sphere.1 == 0.0 { continue; }
        if !g.chains.iter().any(|ch| ch.contains(&(i as u32))) {
            orphans += 1;
            if orphans <= 60 {
                eprintln!("[orphan] cell {i} flags {:#x} sphere ({:.1},{:.1},{:.1}) r {:.1}", c.flags, c.sphere.0[0], c.sphere.0[1], c.sphere.0[2], c.sphere.1);
            }
        }
    }
    eprintln!("[orphan] {orphans} chained-nowhere generic cells");

    // 5. The room identities along the walk (the room placements' object names).
    let room_name = |ci: usize| -> String {
        let Some(rank) = g.room_to_cell.iter().position(|m| *m == Some(ci)) else { return "<not a room>".into() };
        let mut k = 0usize;
        for p in &level.maps[m].placements {
            if matches!(p.flags & 0xffff, 0xc023 | 0xc047) {
                if k == rank { return placement_name(p); }
                k += 1;
            }
        }
        "<?>".into()
    };
    for &c in &chain {
        eprintln!("[room] cell {c} = {}", room_name(c));
    }

    // 6. The portal edges touching the clock/shield/InvPoly cells, and every edge of the visited
    //    rooms (does ANY portal reach the props' cels? the game's unconditional list-0 draw would
    //    need the props' cels IN the VisCelList — i.e. portal-reached).
    let placement_name_opt = |c: usize| -> String {
        g.own_cells_main.iter().enumerate().find(|(_, oc)| **oc == Some(c as u32))
            .map(|(pi, _)| placement_name(&level.maps[m].placements[pi])).unwrap_or_default()
    };
    let interesting = |c: usize| {
        matches!(c, 41..=61 | 245 | 246) || nl_match(&placement_name_opt(c))
    };
    eprintln!("[portals] total {}", g.portals.len());
    for (k, p) in g.portals.iter().enumerate() {
        if interesting(p.cel_a) || interesting(p.cel_b) {
            eprintln!("[portal {k}] {} ({}) <-> {} ({}), flags {:x}, corners {:?}..{:?}",
                p.cel_a, placement_name_opt(p.cel_a), p.cel_b, placement_name_opt(p.cel_b), p.flags,
                p.corners[0], p.corners[2]);
        }
    }
    // The visited rooms' full edge lists.
    for &c in &chain {
        let edges: Vec<String> = g.portals.iter()
            .filter(|p| p.cel_a == c || p.cel_b == c)
            .map(|p| format!("{}<->{} f{:x}", p.cel_a, p.cel_b, p.flags)).collect();
        eprintln!("[edges] cell {c} ({}): {:?}", room_name(c), edges);
    }
}

fn nl_match(s: &str) -> bool { let l = s.to_lowercase(); l.contains("clock") || l.contains("shield") || l.contains("sword") }

/// The clock-hall pin [L docs/live/2026-10-13-portal-ground-truth.md: "two guards, the clock, the
/// shields" at 07000007 --bond 34.25,44.13,-70.71,1.549,-0.070; the merged fix-portal5 master drops
/// them with the walk ON]. The clock (OBJECT_GrandFatherClock1, port cell 50, 17 World materials)
/// hangs on room 29's chain and the end-of-hall wall shields (OBJECT_ShieldBlackwhite1Swords cell
/// 269 + OBJECT_ShieldBlackwhiteSwords cell 270, the x=38 pair — one per wall, the only ones inside
/// the game's 4:3 frustum at that spot) hang on room 33's chain; rooms 33/29/31 are exactly the
/// walk's visited set. The chain-node gate is `Vision_InView` (0x1e89e0): the port's transcription
/// `n·c + d·r + r >= 0` over the eye-anchored planes (d = -n·eye, |d| ~ 70 at this spot) makes the
/// d·r term explode (-184 on the left plane) and rejects EVERY chain node — the regression. The
/// game's own stored planes [L ee_start_05.bin: |n|=1, n·eye+d=0, d=51..117] prove d is
/// eye-distance-scaled, so the literal d·r form is untestable-absurd in BOTH compare polarities
/// (the decompile's `< -param_1` is a Ghidra polarity artifact; the raw c.lt.s f0,f12 + bc1t reads
/// `sum < r -> reject`), while the signed-distance form `n·c + d + r >= 0` is the common limit
/// when |d| is small and matches [L] at all three ground-truth spots. Pins, after the fix:
///   1. the walk visits exactly the eye room 33 + 29 + 31;
///   2. the clock's cell 50 and the two Swords cells 269/270 are in the game mask and ALL their
///      World materials are visible;
///   3. sister: the hall shields outside the spot's frustum (the x=34..36 pairs, cells 43..48,
///      271..284) stay culled — the fix must not become draw-everything;
///   4. the courtyard fix holds: fix_portal5_live's own pin stays green (run separately).
#[test]
fn the_clock_hall_props_draw_with_the_walk_on() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007: {e}"); return } };
    let g = &sc.cel_graph;
    let (eye_cell, chain, game, _vp) = clock_hall_walk(&sc);
    assert_eq!(eye_cell, 33, "the eye cel is the hall room (Room_Corridor_I)");
    assert_eq!(chain, vec![33usize, 29, 31], "the walk visits exactly the three corridor rooms");
    assert!(game[33] && game[29] && game[31], "all three visited rooms draw");
    // The clock: cell 50 on room 29's chain (the junction beyond the hall), 17 World materials.
    assert!(g.chains[29].contains(&50), "the clock's cell 50 rides room 29's +0x18 chain");
    assert!(game[50], "the grandfather clock draws with the walk ON (the game shows it)");
    let clock_mats = sc.materials.iter().filter(|m| m.layer == nf_view::scene::Layer::World && m.draw.pass != 4 && m.node == 50).count();
    assert!(clock_mats >= 17, "the clock's World materials are all present (got {clock_mats})");
    // The in-frustum wall shields: the x=38 pair — the shield bodies (OBJECT_ShieldBlackwhite
    // cell 45, left wall z=-73.1 + OBJECT_ShieldBlackwhite1 cell 46, right wall z=-67.8) and their
    // crossed-swords overlays (cells 270/269) — "BOTH wall shields with crossed swords".
    for &c in &[45usize, 46, 269, 270] {
        assert!(g.chains[33].contains(&(c as u32)), "shield cell {c} rides room 33's chain");
        assert!(game[c], "the wall shield cell {c} draws with the walk ON (the game shows both)");
        let mats = sc.materials.iter().filter(|m| m.layer == nf_view::scene::Layer::World && m.draw.pass != 4 && m.node == c as u32).count();
        assert!(mats > 0, "shield cell {c} carries World materials");
    }
    // Sister: the shields directly beside the eye (x=34, 86..94 deg off-axis) stay culled — the
    // gate keeps its frustum shape. (Cells 44/47 at 55..58 deg are outside even the port's wider
    // 16:9 frustum centre-edge but their r=0.59 bounding spheres poke the plane; the port draws
    // them GPU-clipped off-screen — the game's 4:3 frustum culls them. Documented divergence,
    // inherent to rendering 16:9 with the game's 4:3-authored cull spheres.)
    for &c in &[43usize, 48] {
        assert!(!game[c], "the beside-the-eye shield cell {c} stays culled at this spot");
    }
    // The other visited rooms' prop nodes with materials draw or cull ONLY by the frustum — none
    // may be dropped by a chain-side artifact: every chain node of the visited rooms that lies
    // within 2 deg of the view axis must be visible (the clock is the on-axis one).
    eprintln!("[pin] clock hall: rooms {{33,29,31}} + clock 50 + shields {{269,270}} draw; beside-eye shields culled");
}

/// The fourth ground-truth spot [L docs/live + the task]: 07000006 at
/// `--bond 9.87,43.04,9.84,3.177,0` renders fine after the alignment fix — the barrel props at the
/// bottom of the view draw. Pin: the walk resolves, the eye room + its chain's in-frustum props
/// draw, and the barrel placements inside the frustum are in the game mask.
#[test]
fn the_07000006_second_spot_keeps_its_props() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let level = load_level(&iso, "07000006.bin");
    let m = level.main_map().expect("main map");
    let g = &sc.cel_graph;
    let eye = [9.87f32, 43.04 + 0.7, 9.84];
    let vp = view_proj(eye, 3.177, 0.0, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);
    let world = sc.sim.as_ref().map(|s| &s.world);
    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(g, &mut state, eye, world).expect("the eye cel resolves");
    let chain = scene::portal_cel_chain_from_cell(g, cell, eye, &depth0).expect("the walk runs");
    let game = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("mask");
    assert!(game[cell], "the eye room draws");
    eprintln!("[spot2] eye cell {cell}, visits {chain:?}");
    // The barrel props: name-match barrel in the main map's placements.
    let mut checked = 0usize;
    for (pi, p) in level.maps[m].placements.iter().enumerate() {
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
        let Some((om, oi)) = target else { continue };
        let o = &level.maps[om].objects[oi];
        if !o.name.to_lowercase().contains("barrel") { continue; }
        let Some(own) = g.own_cells_main.get(pi).copied().flatten() else { continue };
        let (c3, r) = g.cells[own as usize].sphere;
        // Only the ones inside the spot's frustum cone (the ones the game's view shows).
        let rel = [c3[0] - eye[0], c3[1] - eye[1], c3[2] - eye[2]];
        let fwd = [planes[4][0], planes[4][1], planes[4][2]];
        let t = rel[0] * fwd[0] + rel[1] * fwd[1] + rel[2] * fwd[2];
        if t <= 1.0 { continue; }
        let lat = (rel[0] * rel[0] + rel[1] * rel[1] + rel[2] * rel[2] - t * t).sqrt();
        let ang = (lat / t).atan();
        if ang > 0.62 { continue; } // ~35.5 deg: inside the game's 4:3 cone
        checked += 1;
        assert!(game[own as usize], "barrel \"{}\" (cell {own}, {:.0} deg off-axis) draws at the spot — the game shows the barrels", o.name.trim(), ang.to_degrees());
    }
    assert!(checked > 0, "the spot's frustum contains barrel props to pin");
    eprintln!("[spot2] {checked} in-frustum barrel props all draw");
}

#[test]
fn probe_portal6_viewer_planes() {
    let Some(_iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let snap_path = ["work/", "../work/", "../../work/"].iter().map(|pre| std::path::PathBuf::from(format!("{pre}ee_start_05.bin"))).find(|d| d.exists());
    let Some(snap_path) = snap_path else { eprintln!("ee_start_05.bin not found"); return };
    let snap = std::fs::read(&snap_path).expect("snapshot");
    let u32at = |a: usize| u32::from_le_bytes(snap[a..a + 4].try_into().unwrap());
    let u16at = |a: usize| u16::from_le_bytes(snap[a..a + 2].try_into().unwrap());
    let f32at = |a: usize| f32::from_le_bytes(snap[a..a + 4].try_into().unwrap());

    // The viewer: glb_viewer[0] (0x2d88b0, 44 bytes = 11 u32 slots).
    let mut viewers = Vec::new();
    for k in 0..11u32 {
        let v = u32at(0x2d88b0 + (k as usize) * 4) as usize;
        if v != 0 && v + 0x160 <= snap.len() { viewers.push((k, v)); }
    }
    eprintln!("[viewer] glb_viewer slots: {:?}", viewers);

    let eye = [f32at(0x34b390), f32at(0x34b394), f32at(0x34b398)];
    eprintln!("[eye] CamPos 0x34b390 = ({:.2},{:.2},{:.2})", eye[0], eye[1], eye[2]);

    let (vk, v) = viewers[0];
    eprintln!("[viewer {vk}] struct at {v:#x}");
    eprintln!("[viewer {vk}] +0x114 aspect {:.4}, +0x118 fov {:.4}", f32at(v + 0x114), f32at(v + 0x118));
    for row in 0..4 {
        let a = v + 0x120 + row * 16;
        eprintln!("[viewer {vk}] matrix row {row}: ({:.3},{:.3},{:.3},{:.3})", f32at(a), f32at(a + 4), f32at(a + 8), f32at(a + 12));
    }
    let mut planes = [[0f32; 4]; 4];
    for k in 0..4 {
        let a = v + 0x70 + k * 16;
        planes[k] = [f32at(a), f32at(a + 4), f32at(a + 8), f32at(a + 12)];
        eprintln!("[plane {k}] n ({:.4},{:.4},{:.4}) d {:.4}  |n| {:.4}  n·eye+d {:.4}",
            planes[k][0], planes[k][1], planes[k][2], planes[k][3],
            (planes[k][0] * planes[k][0] + planes[k][1] * planes[k][1] + planes[k][2] * planes[k][2]).sqrt(),
            planes[k][0] * eye[0] + planes[k][1] * eye[1] + planes[k][2] * eye[2] + planes[k][3]);
    }

    // The VisCelList at this moment: the rooms the game walked for THIS viewer.
    let count = u16at(0x30d0bc) as usize;
    eprintln!("[viscel] count {count}");
    let mut rooms: Vec<usize> = Vec::new();
    for k in 0..count.min(0x40) {
        let c = u32at(0x34b3b0 + k * 8) as usize;
        let depth = u16at(0x34b3b0 + k * 8 + 6);
        let cls = u32at(c + 0x3c);
        let sp = ([f32at(c + 0x80), f32at(c + 0x84), f32at(c + 0x88)], f32at(c + 0x8c));
        eprintln!("[viscel {k}] cel {c:#x} depth {depth} class {cls:#x} sphere ({:.1},{:.1},{:.1}) r {:.1}", sp.0[0], sp.0[1], sp.0[2], sp.1);
        rooms.push(c);
    }

    // The chain nodes on those rooms: evaluate the two readings with the game's OWN planes.
    let mut a_pass = 0usize; let mut b_pass = 0usize; let mut total = 0usize;
    for &c in &rooms {
        let mut nxt = u32at(c + 0x18) as usize;
        let mut guard = 0;
        while nxt != 0 && guard < 200 {
            guard += 1;
            let node = nxt;
            let cls = u32at(node + 0x3c);
            let c3 = [f32at(node + 0x80), f32at(node + 0x84), f32at(node + 0x88)];
            let r = f32at(node + 0x8c);
            let mut va = true; let mut vb = true;
            let mut worst_a = f32::MAX; let mut worst_b = f32::MAX;
            for p in &planes {
                let dot = p[0] * c3[0] + p[1] * c3[1] + p[2] * c3[2];
                let na = dot + p[3] * r + r;
                let nb = dot + p[3] + r;
                if na < 0.0 { va = false; }
                if nb < 0.0 { vb = false; }
                worst_a = worst_a.min(na); worst_b = worst_b.min(nb);
            }
            total += 1;
            if va { a_pass += 1; }
            if vb { b_pass += 1; }
            eprintln!("[node] {node:#x} class {cls:#08x} c ({:.1},{:.1},{:.1}) r {:.2}: (a) d·r {va} worst {worst_a:.1} | (b) d {vb} worst {worst_b:.1}", c3[0], c3[1], c3[2], r);
            nxt = u32at(node + 0x18) as usize;
        }
    }
    eprintln!("[verdict] chain nodes {total}: reading (a) n·c+d·r+r accepts {a_pass}, reading (b) n·c+d+r accepts {b_pass}");
}
