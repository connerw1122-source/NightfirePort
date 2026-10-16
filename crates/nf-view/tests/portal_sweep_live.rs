//! fix-portal-sweep: portal culling must never hide what the player can see, on every level.
//!
//! The playtest found 07000005 `--bond 16.67,30.04,58.60,1.755,-0.001` (a snow trench between
//! stone walls near grapple point #282) with the walk ON showing through the trench walls to the
//! distant cliff: the near room itself is culled. `NF_NOPORTAL=1` draws the walls and the wall-hug
//! marker. This file (1) probes the walk at that spot headlessly, (2) runs the near-visibility
//! sweep over every on-foot level: every World cel the frustum alone keeps, within ~30 units of
//! the eye with clear line of sight (a ray against the collision world), must be kept by the walk.
//!
//! Opt-in via NF_ISO.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

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

/// The game's depth-0 pyramid (the gpu.rs `depth0_game` copy for tests).
fn depth0_game(vp: [[f32; 4]; 4], eye: [f32; 3]) -> [[f32; 4]; 5] {
    let planes = nf_view::gpu::extract_frustum(vp);
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}

/// `Vision_InView`'s signed-distance form (scene.rs).
fn in_view(c3: [f32; 3], r: f32, sides: &[[f32; 4]; 4]) -> bool {
    sides.iter().all(|p| p[0] * c3[0] + p[1] * c3[1] + p[2] * c3[2] + p[3] + r >= 0.0)
}

/// The renderer's World-draw cull under the walk (gpu.rs render): node-keyed draws go through the
/// game cell mask, distance-keyed draws (node == u32::MAX) through their room's strict mask.
fn world_draw_hidden_walk(m: &scene::Material, cell_vis: &[bool], room_vis: &[bool]) -> bool {
    if m.node != u32::MAX { !cell_vis[m.node as usize] } else { m.cel.is_some_and(|c| !room_vis[c as usize]) }
}

/// The renderer's World-draw cull with the walk OFF (`NF_NOPORTAL=1`): every draw falls through to
/// its room's frustum box test (`cel_boxes` + the 6 frustum planes).
fn world_draw_hidden_frustum(m: &scene::Material, room_vis: &[bool]) -> bool {
    m.cel.is_some_and(|c| !room_vis[c as usize])
}

/// The playtest's trench spot on 07000005 (the probe convention: bond y + 0.7 is the camera head,
/// the pitch is the game's half-radian word — the render path multiplies by π/2).
const TRENCH: ([f32; 3], f32, f32) = ([16.67, 30.04 + 0.7, 58.60], 1.755, -0.001 * std::f32::consts::FRAC_PI_2);

/// The reproduction: the walk at the trench spot, the eye cel, the visited rooms, the chain nodes,
/// and the World draw counts walk-on vs frustum-only; the near draws' verdicts individually.
#[test]
fn probe_trench_spot_07000005() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000005.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000005: {e}"); return } };
    let g = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world);
    eprintln!("[graph] cells {} portals {} dropped {} rooms {}", g.cells.len(), g.portals.len(), g.dropped_portals, g.room_to_cell.len());

    let (eye, yaw, pitch) = TRENCH;
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = depth0_game(vp, eye);

    let mut state = scene::ViewerCel::default();
    let cell = scene::eye_cell(g, &mut state, eye, world).expect("the eye cel resolves");
    let cn = &g.cells[cell];
    eprintln!("[eye] cell {cell} flags {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1} bbox ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1})",
        cn.flags, cn.sphere.0[0], cn.sphere.0[1], cn.sphere.0[2], cn.sphere.1, cn.min[0], cn.min[1], cn.min[2], cn.max[0], cn.max[1], cn.max[2]);

    let chain = scene::portal_cel_chain_from_cell(g, cell, eye, &depth0).expect("the walk runs");
    let game = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("the game mask");
    eprintln!("[walk] visits {} rooms: {:?}", chain.len(), chain);
    for &c in &chain {
        let n = &g.cells[c];
        eprintln!("[visited] cell {c} flags {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1} chain {} nodes",
            n.flags, n.sphere.0[0], n.sphere.0[1], n.sphere.0[2], n.sphere.1, g.chains[c].len());
    }

    // The strict room mask (the walk's VisCelList) and the frustum-only room mask, in the port's
    // room indexing — exactly what the renderer hands the World draws.
    let room_vis_walk: Vec<bool> = g.room_to_cell.iter().map(|m| m.map(|c| game[c]).unwrap_or(true)).collect();
    let room_vis_frustum: Vec<bool> = sc.cels.iter().map(|c| !nf_view::gpu::box_outside(&planes, c.min, c.max)).collect();

    // The World draws: counts under both paths, and every draw within 45 units of the eye.
    let mats: Vec<&scene::Material> = sc.materials.iter().filter(|m| m.layer == scene::Layer::World).collect();
    let kept_walk = mats.iter().filter(|m| !world_draw_hidden_walk(m, &game, &room_vis_walk)).count();
    let kept_frustum = mats.iter().filter(|m| !world_draw_hidden_frustum(m, &room_vis_frustum)).count();
    eprintln!("[world] {} World draws: walk keeps {kept_walk}, frustum-only keeps {kept_frustum}", mats.len());
    let d2 = |c: [f32; 3]| (c[0] - eye[0]).powi(2) + (c[1] - eye[1]).powi(2) + (c[2] - eye[2]).powi(2);
    let mut near = mats.iter().filter(|m| d2(m.centre) <= 45.0 * 45.0).collect::<Vec<_>>();
    near.sort_by_key(|m| (d2(m.centre) * 100.0) as i64);
    eprintln!("[world] {} World draws within 45 units of the eye:", near.len());
    for m in &near {
        let w = !world_draw_hidden_walk(m, &game, &room_vis_walk);
        let f = !world_draw_hidden_frustum(m, &room_vis_frustum);
        // Clear-LOS check against the collision world (the sweep's rule).
        let los = world.map(|w| w.segment_hit_scoped(None, [eye[0], eye[1], eye[2]], m.centre).is_none());
        eprintln!("[near] centre ({:.1},{:.1},{:.1}) r {:.1} node {} cel {:?}: walk {} frustum {} los {:?}",
            m.centre[0], m.centre[1], m.centre[2], m.radius, m.node, m.cel, w, f, los);
    }

    // The per-cell frustum verdict for every cell whose sphere is near the eye: is the cell's own
    // frustum test fine while the walk drops it?
    for (i, c) in g.cells.iter().enumerate() {
        if c.min == [0.0; 3] && c.max == [0.0; 3] && c.sphere.1 == 0.0 { continue; }
        let dc = (c.sphere.0[0] - eye[0]).hypot((c.sphere.0[1] - eye[1]).hypot(c.sphere.0[2] - eye[2]));
        if dc > 60.0 { continue; }
        let owners: Vec<usize> = g.chains.iter().enumerate().filter(|(_, ch)| ch.contains(&(i as u32))).map(|(o, _)| o).collect();
        eprintln!("[cell {i}] flags {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1} dist {dc:.1} frustum {} game-mask {} owners {:?}",
            c.flags, c.sphere.0[0], c.sphere.0[1], c.sphere.0[2], c.sphere.1, in_view(c.sphere.0, c.sphere.1, &sides), game[i], owners);
    }

    // The eye-cel candidates: every room-class (0x40000) cel whose bbox holds the eye — what
    // FindCel had to choose between, and the port's tie-break pick.
    eprintln!("[findcel] eye {eye:?}");
    for (i, c) in g.cells.iter().enumerate() {
        if c.flags & 0x40000 == 0 { continue; }
        if !(0..3).all(|k| c.min[k] <= eye[k] && eye[k] <= c.max[k]) { continue; }
        eprintln!("[findcel] candidate cell {i} flags {:08x} sphere ({:.1},{:.1},{:.1}) r {:.1}", c.flags, c.sphere.0[0], c.sphere.0[1], c.sphere.0[2], c.sphere.1);
    }
    // The portal edges around 14, 15, 24: is 15 even portal-reachable from the eye cel?
    for c in [14usize, 15, 24] {
        let edges: Vec<String> = g.portals.iter()
            .filter(|p| p.cel_a == c || p.cel_b == c)
            .map(|p| format!("{}<->{} f{:x}", p.cel_a, p.cel_b, p.flags)).collect();
        eprintln!("[edges] cell {c}: {:?}", edges);
    }

    // The experiment: the walk FROM cell 15 (as if the eye cel resolved there) — do the trench
    // nodes draw and does the near set survive? Same for 14 as the baseline.
    for start in [14usize, 15] {
        let chain15 = scene::portal_cel_chain_from_cell(g, start, eye, &depth0).expect("walk");
        let game15 = scene::portal_visible_cels_game_from_cell(g, start, eye, &depth0, &sides).expect("mask");
        let room15: Vec<bool> = g.room_to_cell.iter().map(|m| m.map(|c| game15[c]).unwrap_or(true)).collect();
        let kept15 = mats.iter().filter(|m| !world_draw_hidden_walk(m, &game15, &room15)).count();
        let near_dropped = near.iter().filter(|m| world_draw_hidden_walk(m, &game15, &room15)).count();
        eprintln!("[from {start}] visits {:?} keeps {kept15} World draws, near-dropped {near_dropped}", chain15);
    }

    // The fix (fix-portal-sweep): the shipped mask = the walk + the near-LOS rescue. The trench
    // spot's near set must survive from the REAL eye cel (14).
    let mut shipped = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("mask");
    let rescued = match world {
        Some(w) => scene::near_los_rescue(g, &mut shipped, eye, &sides, w, 30.0),
        None => 0,
    };
    let room_shipped: Vec<bool> = g.room_to_cell.iter().map(|m| m.map(|c| shipped[c]).unwrap_or(true)).collect();
    let kept_shipped = mats.iter().filter(|m| !world_draw_hidden_walk(m, &shipped, &room_shipped)).count();
    let near_dropped_shipped = near.iter().filter(|m| world_draw_hidden_walk(m, &shipped, &room_shipped)).count();
    eprintln!("[shipped] rescue restored {rescued} cels; keeps {kept_shipped} World draws (frustum-only 1809, raw walk 719); near-dropped {near_dropped_shipped}");
    // The near draws that remain hidden under the shipped mask while the frustum alone keeps them
    // AND the LOS is clear — the sweep's failure set at this spot (must be empty for node-keyed
    // generics; distance-keyed draws ride their room's strict mask and are reported, not asserted).
    let mut still = 0usize;
    for m in &near {
        if !world_draw_hidden_walk(m, &shipped, &room_shipped) { continue; }
        if world_draw_hidden_frustum(m, &room_vis_frustum) { continue; }
        let los = world.map(|w| w.segment_hit_scoped(None, [eye[0], eye[1], eye[2]], m.centre).is_none()).unwrap_or(false);
        if los && m.node != u32::MAX { still += 1; eprintln!("[still-hidden] node {} centre ({:.1},{:.1},{:.1})", m.node, m.centre[0], m.centre[1], m.centre[2]); }
    }
    eprintln!("[shipped] near node-keyed failures at the trench spot: {still}");
    // The sweep's criterion at this spot, cel-level: every generic the frustum keeps, within 30
    // units with clear static LOS, must be in the shipped mask.
    let mut cel_fails = 0usize;
    for (i, c) in g.cells.iter().enumerate() {
        if shipped[i] || !matches!(c.flags & 0xffff, 0x8017 | 0x8018 | 0x8019) { continue; }
        if c.min == [0.0; 3] && c.max == [0.0; 3] && c.sphere.1 == 0.0 { continue; }
        if !in_view(c.sphere.0, c.sphere.1, &sides) { continue; }
        if (0..3).any(|k| !c.min[k].is_finite() || !c.max[k].is_finite() || c.min[k] > c.max[k]) { continue; }
        let mut tp = [0f32; 3];
        for k in 0..3 { tp[k] = eye[k].clamp(c.min[k], c.max[k]); }
        let d = ((tp[0] - eye[0]).powi(2) + (tp[1] - eye[1]).powi(2) + (tp[2] - eye[2]).powi(2)).sqrt();
        if d > 30.0 { continue; }
        if let Some(w) = world {
            if scene::los_ray_clear(w, eye, tp, d) {
                cel_fails += 1;
                eprintln!("[cel-fail] cell {i} sphere ({:.1},{:.1},{:.1}) r {:.1}", c.sphere.0[0], c.sphere.0[1], c.sphere.0[2], c.sphere.1);
            }
        }
    }
    assert_eq!(cel_fails, 0, "near unoccluded generics culled by the shipped mask at the trench spot");
}

/// Every on-foot level's on-foot level list (the levels_sweep_live convention).
fn on_foot_levels(img: &nf_formats::iso9660::IsoImage) -> Vec<String> {
    use nf_formats::disc;
    let (entries, _) = disc::files_bin_entries(img).unwrap();
    let mut names: Vec<String> = entries.iter()
        .map(|e| e.name.to_ascii_lowercase())
        .filter(|n| n.starts_with("0700") && n.ends_with(".bin"))
        .filter(|n| {
            let code = u32::from_str_radix(&n[..8], 16).unwrap_or(0) & 0xff;
            !(0x23..=0x29).contains(&code) && code != 0x00 && n != "07000500.bin" && n != "07000999.bin"
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The sweep (fix-portal-sweep item 2): for every on-foot level, the eye at the loaded spawn and
/// at a grid of points along the level's nav nodes (capped at 150 per level, 4 yaw directions
/// each), asserting that every World cel the frustum alone keeps AND that lies within 30 units of
/// the eye with clear line of sight (the segment to the closest point of the cel's bbox against
/// the collision world) is kept by the shipped mask — the walk (`Vision_Portal_Recurse` 0x1e86e8
/// → `Vision_Recurse` 0x1e82e0 + `View_AddCels` 0x1e6ba0's chain pass) plus the near-LOS rescue.
/// Near, unoccluded generic geometry must never be culled. The RAW walk's failures print per
/// level (counts only — the documented causes); the shipped mask must be clean everywhere.
#[test]
fn sweep_every_level_near_los_never_culled() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let levels = on_foot_levels(&img);
    eprintln!("[sweep] {} on-foot levels", levels.len());
    let (mut spots_total, mut dirs_total, mut shipped_failures) = (0usize, 0usize, Vec::new());
    for lv in &levels {
        let scene = match nf_view::load(&iso, lv, false) { Ok(s) => s, Err(e) => { eprintln!("[sweep] {lv}: load failed {e}"); continue } };
        let g = scene.cel_graph.clone();
        if g.portals.is_empty() { eprintln!("[sweep] {lv}: no portals, skipped"); continue }
        let world = scene.sim.as_ref().map(|s| &s.world);
        let waypoints: Vec<[f32; 3]> = scene.sim.as_ref().map(|s| s.nav.paths.iter().flat_map(|p| p.nodes.iter().map(|n| n.pos)).collect()).unwrap_or_default();
        let mut spots: Vec<[f32; 3]> = scene.sim.as_ref().map(|s| vec![s.bond.pos]).unwrap_or_default();
        let stride = (waypoints.len() / 150).max(1);
        spots.extend(waypoints.into_iter().step_by(stride));
        let (mut checked) = (0usize);
        for spot in &spots {
            let eye = [spot[0], spot[1] + 0.7, spot[2]];
            for k in 0..4u32 {
                let yaw = k as f32 * std::f32::consts::FRAC_PI_2;
                let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
                let planes = nf_view::gpu::extract_frustum(vp);
                let sides = [planes[0], planes[1], planes[2], planes[3]];
                let depth0 = depth0_game(vp, eye);
                let mut state = scene::ViewerCel::default();
                let Some(cell) = scene::eye_cell(&g, &mut state, eye, world) else { continue };
                checked += 1;
                let Some(mut mask) = scene::portal_visible_cels_game_from_cell(&g, cell, eye, &depth0, &sides) else { continue };
                // faster-live-tests: the old RAW-WALK diagnostic scan here re-ran los_ray_clear over
                // every near generic cell purely to PRINT raw-failure counts — the shipped loop
                // below reuses the identical filters, so the diagnostic doubled the sweep's raycasts
                // without feeding any assertion (the shipped-mask assert at the end is the pin).
                // The shipped mask must be clean.
                if let Some(w) = world { scene::near_los_rescue(&g, &mut mask, eye, &sides, w, 30.0); }
                for (i, c) in g.cells.iter().enumerate() {
                    if mask[i] || !matches!(c.flags & 0xffff, 0x8017 | 0x8018 | 0x8019) { continue; }
                    if !in_view(c.sphere.0, c.sphere.1, &sides) { continue; }
                    let mut tp = [0f32; 3];
                    if (0..3).any(|k| !c.min[k].is_finite() || !c.max[k].is_finite() || c.min[k] > c.max[k]) { continue; }
                    for j in 0..3 { tp[j] = eye[j].clamp(c.min[j], c.max[j]); }
                    let d = ((tp[0] - eye[0]).powi(2) + (tp[1] - eye[1]).powi(2) + (tp[2] - eye[2]).powi(2)).sqrt();
                    if d > 30.0 { continue; }
                    if let Some(w) = world {
                        if scene::los_ray_clear(w, eye, tp, d) {
                            shipped_failures.push((lv.clone(), i, eye, yaw));
                        }
                    }
                }
            }
        }
        spots_total += spots.len();
        dirs_total += checked;
        eprintln!("[sweep] {lv}: {} spots, {checked} directions", spots.len());
    }
    eprintln!("[sweep] pinned: {spots_total} spots, {dirs_total} spot-directions over {} levels", levels.len());
    eprintln!("[sweep] shipped-mask failures: {} ({:?})", shipped_failures.len(), shipped_failures.iter().take(8).map(|(l, c, _, _)| format!("{l} cell {c}")).collect::<Vec<_>>());
    assert!(dirs_total >= 400, "the sweep must cover a meaningful number of spot-directions, got {dirs_total}");
    assert!(shipped_failures.is_empty(), "near unoccluded generics culled by the shipped mask: {}", shipped_failures.len());
}

/// The four ground-truth spots (docs/live/2026-10-13-portal-ground-truth.md) against the SHIPPED
/// mask (the walk + the near-LOS rescue). The rescue is additive, so the only spot where it can
/// do harm is the ballroom window (the game HIDES the lit building there): the test asserts the
/// rescue restores nothing at that spot, keeps the 06 courtyard masonry solid, and keeps the 07
/// clock-hall props (the grandfather clock cell 50, the shields 43-48) visible.
#[test]
fn ground_truth_spots_stay_put() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // (level, eye, yaw, pitch, assert kind)
    let spots: [(&str, [f32; 3], f32, f32, u8); 4] = [
        ("07000006.bin", [7.24, 42.03 + 0.7, -39.31], 10.161, 0.428 * std::f32::consts::FRAC_PI_2, 0), // the courtyard
        ("07000006.bin", [0.0, 0.0, 0.0], 0.0, 0.0, 3), // the 06 start (filled below from the spawn)
        ("07000007.bin", [61.11, 44.12 + 0.7, -52.58], -2.541, -0.010 * std::f32::consts::FRAC_PI_2, 1), // the window
        ("07000007.bin", [34.25, 44.13 + 0.7, -70.71], 1.549, -0.070 * std::f32::consts::FRAC_PI_2, 2), // the clock hall
    ];
    let mut loaded: Vec<(&str, scene::Scene)> = Vec::new();
    for (lv, eye, yaw, pitch, kind) in spots {
        let sc = match loaded.iter().find(|(l, _)| *l == lv) {
            Some((_, s)) => s,
            None => {
                let s = nf_view::load(&iso, lv, false).expect("load");
                loaded.push((lv, s));
                &loaded[loaded.len() - 1].1
            }
        };
        let g = &sc.cel_graph;
        let world = sc.sim.as_ref().map(|s| &s.world);
        let eye = if kind == 3 { let mut e = sc.sim.as_ref().expect("sim").bond.pos; e[1] += 0.7; e } else { eye };
        let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
        let planes = nf_view::gpu::extract_frustum(vp);
        let sides = [planes[0], planes[1], planes[2], planes[3]];
        let depth0 = depth0_game(vp, eye);
        let mut state = scene::ViewerCel::default();
        let cell = scene::eye_cell(g, &mut state, eye, world).expect("the eye cel resolves");
        let mut mask = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("mask");
        let rescued = match world {
            Some(w) => scene::near_los_rescue(g, &mut mask, eye, &sides, w, 30.0),
            None => 0,
        };
        let added: Vec<usize> = (0..g.cells.len()).filter(|&i| !scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).unwrap()[i] && mask[i]).collect();
        match kind {
            0 => {
                eprintln!("[gt] 06 courtyard: eye cell {cell}, rescue {rescued} {added:?}");
                // The ground-truth walk verdict (the eye cel 33, the masonry solid) is pinned by
                // fix_portal3_live; the rescue must add nothing at the verified spot.
                assert_eq!(cell, 33, "the courtyard spot's eye cel (the ground-truth cell 33)");
                assert_eq!(rescued, 0, "the rescue must add nothing at the 06 courtyard spot");
            }
            1 => {
                eprintln!("[gt] 07 window: eye cell {cell}, rescue {rescued} {added:?}");
                assert_eq!(rescued, 0, "the rescue must restore nothing at the ballroom window (the hidden building stays hidden)");
            }
            2 => {
                eprintln!("[gt] 07 clock hall: eye cell {cell}, rescue {rescued} {added:?}");
                // fix_portal6_live's pinned set: the clock (50) and the in-frustum shields
                // (45/46 + the crossed-swords overlays 269/270).
                assert_eq!(cell, 33, "the clock hall's eye cel (the corridor room)");
                assert!(mask[50], "the grandfather clock cell 50 must stay visible");
                for c in [45usize, 46, 269, 270] { assert!(mask[c], "the shield cell {c} must stay visible"); }
            }
            _ => eprintln!("[gt] 06 start: eye cell {cell}, rescue {rescued}"),
        }
    }
}
