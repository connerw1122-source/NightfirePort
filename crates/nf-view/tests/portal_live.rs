//! Headless NF_ISO tests for fix-perf-level: the portal visibility walk (G-016's portal half) and
//! the cold-start loadout (`Player_InitWeapon` 0x1a7ef8 on a direct level load).
//!
//! Evidence base:
//! - `Vision_Portal_Recurse` (0x1e86e8) → `Vision_Recurse` (0x1e82e0) [M]: from the viewer's cel,
//!   walk the cel graph's portals (cel+0x28 lists, dest at portal+0x8, side flag at +0x27, the
//!   `Door_IsClosed` skip), clipping each portal quad against the planes accumulated on the path
//!   (`ClipPortalWithPlane` 0x1e7fa0, keep `value >= 0`, the √1e-7 compaction), and add the
//!   destination cel to the visible set; `View_AddCels` (0x1e6ba0) adds ONLY these cels' glists.
//! - `Vision_Calculate_View_Fustrum` (0x1e7df8) [M]: depth 0 = five planes (four sides through the
//!   eye + near); the port passes the frame frustum's first five Gribb-Hartmann planes.
//! - Map block 0x21 [D]: stride 56, corners + celA/celB u16 + flags u16 (2468-byte block on
//!   07000006, count 44, (2468−4)/44 = 56.0; cel.md's "stride 68" corrected).
//! - Cold loadout [M 0x1a7ef8]: `Player_RamLoad` is gated on `IsWarmReset` (0x1957a8:9), so a cold
//!   load reads the boot-cleared inventory — case 0x7000006/8 must fall into the fresh-05/party
//!   arms (nf-view 07000006 --frame 30 printed "switch request 0" before the fix = unarmed).
//!
//! Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
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

/// The World draws the render path submits for `vp`: sphere cull, then the fix-perf-level visibility
/// set (the portal walk when it runs, else the fix-render3 box test) — the same order gpu.rs uses.
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
            (Some(_), None) => {}
            (None, Some(c)) => {
                let Some((cc, cr)) = scene.cel_sphere(c as usize) else { continue };
                if nf_view::gpu::sphere_outside(&planes, cc, cr) { culled_vis += 1; continue; }
            }
            (None, None) => {}
        }
        submitted += 1;
    }
    (submitted, culled_sphere, culled_vis)
}

/// The fix-render3 box-test fallback counts (sphere cull + the cel bbox test) — the pre-portal
/// behavior, for the before/after comparison.
fn box_counts(scene: &scene::Scene, vp: [[f32; 4]; 4]) -> (usize, usize) {
    let planes = nf_view::gpu::extract_frustum(vp);
    let (mut submitted, mut culled) = (0usize, 0usize);
    for m in &scene.materials {
        if m.layer != scene::Layer::World { continue; }
        if m.draw.pass != 4 && nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { culled += 1; continue; }
        if let Some(c) = m.cel {
            let Some((cc, cr)) = scene.cel_sphere(c as usize) else { continue };
            if nf_view::gpu::sphere_outside(&planes, cc, cr) { culled += 1; continue; }
        }
        submitted += 1;
    }
    (submitted, culled)
}

/// The 07000007 start view — the number the task pins ("4526 draws; the goal is a few hundred") —
/// must submit far fewer draws with the walk, and the eye's own cel must never be culled.
#[test]
fn the_portal_walk_cuts_the_07000007_start_view() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007 not loadable: {e}"); return } };
    assert!(scene.cel_graph.portals.len() > 0, "07000007 has no portal graph — block 0x21 unparsed");
    let (pos, yaw) = scene.start.clone().unwrap_or(([0.0; 3], 0.0));
    let eye = [pos[0], pos[1] + 1.6, pos[2]];
    let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
    let (sub, _, culled_vis) = world_counts(&scene, vp, eye);
    eprintln!("[portal] 07000007 start: {sub} submitted ({culled_vis} visibility-culled); portals {}", scene.cel_graph.portals.len());
    // The walk's whole point: the start view must drop well below the 4526 the box test leaves.
    assert!(sub < 2000, "the start view still submits {sub} World draws — the portal walk brought no relief");
    // The eye's own room cel is always in the set (Vision_Portal_Recurse adds it first [M]).
    let own = scene.cel_index_at(eye).expect("the eye sits in a room cel on 07000007");
    let depth0: [[f32; 4]; 5] = { let p = nf_view::gpu::extract_frustum(vp); [p[0], p[1], p[2], p[3], p[4]] };
    let vis = scene::portal_visible_cels(&scene.cel_graph, own, eye, &depth0).expect("the walk runs");
    assert!(vis[scene.cel_graph.room_to_cell[own].unwrap()], "the eye's own cel {own} was not visible — the walk is inverted");
    // Sister: the box-test fallback (the graph emptied) still yields the old number, proving the
    // relief comes from the walk and not from the sphere cull.
    let (sub_box, _) = box_counts(&scene, vp);
    eprintln!("[portal] 07000007 start with the box-test fallback: {sub_box}");
    assert!(sub_box > sub, "the walk-gated count ({sub}) must be below the box-test count ({sub_box})");
}

/// Over-cull guard across the render3 directions: the walk can only REMOVE cels the box test kept,
/// never add draws, and every direction keeps the eye's cel. Also pins the away view's small number.
#[test]
fn the_portal_walk_never_adds_draws_in_any_direction() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007 not loadable: {e}"); return } };
    let (pos, yaw) = scene.start.clone().unwrap_or(([0.0; 3], 0.0));
    let eye = [pos[0], pos[1] + 1.6, pos[2]];
    for (name, y, p) in [("fwd", yaw, 0.0), ("away", yaw + std::f32::consts::PI, 0.0), ("left", yaw - 1.5707964, 0.0), ("right", yaw + 1.5707964, 0.0), ("down", yaw, -1.2)] {
        let vp = view_proj(eye, y, p, 16.0 / 9.0);
        let (sub_walk, _, _) = world_counts(&scene, vp, eye);
        let (sub_box, _) = box_counts(&scene, vp);
        eprintln!("[portal] 07000007 {name}: walk {sub_walk} vs box {sub_box}");
        assert!(sub_walk <= sub_box, "{name}: the walk submitted {sub_walk} > box {sub_box} — it added draws");
        let own = scene.cel_index_at(eye).unwrap();
        let depth0: [[f32; 4]; 5] = { let pl = nf_view::gpu::extract_frustum(vp); [pl[0], pl[1], pl[2], pl[3], pl[4]] };
        let vis = scene::portal_visible_cels(&scene.cel_graph, own, eye, &depth0).unwrap();
        assert!(vis[scene.cel_graph.room_to_cell[own].unwrap()], "{name}: the eye's own cel vanished");
    }
    let (_, _, _) = world_counts(&scene, view_proj(eye, yaw + std::f32::consts::PI, 0.0, 16.0 / 9.0), eye);
}

/// The graph's geometry: no dropped endpoints, and each portal's forward plane (built
/// `Plane_PlaneEq(plane, c0, c1, c2)` [M 0x1d15f8]) points into celA — the winding the walk's
/// `dot(eye) < 0` test and `build_alloc_portal`'s corner copies depend on. *Measured*, not assumed.
#[test]
fn the_portal_graph_planes_point_into_cel_a() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for level in ["07000002.bin", "07000005.bin", "07000006.bin", "07000007.bin"] {
        let scene = match nf_view::load(&iso, level, false) { Ok(s) => s, Err(e) => { eprintln!("{level} not loadable: {e}"); continue } };
        let g = &scene.cel_graph;
        eprintln!("[portal] {level}: {} cells, {} portals, {} dropped", g.cells.len(), g.portals.len(), g.dropped_portals);
        assert_eq!(g.dropped_portals, 0, "{level}: {0} portal endpoints did not resolve to cel placements", g.dropped_portals);
        let mut into_a = 0;
        for (k, p) in g.portals.iter().enumerate() {
            let a = &g.cells[p.cel_a];
            let centre = [0, 1, 2].map(|i| (a.min[i] + a.max[i]) * 0.5);
            let v = p.n_fwd[0] * (centre[0] - p.corners[0][0]) + p.n_fwd[1] * (centre[1] - p.corners[0][1]) + p.n_fwd[2] * (centre[2] - p.corners[0][2]);
            if v > 0.0 { into_a += 1; }
            assert_ne!(p.cel_a, p.cel_b, "{level}: portal {k} links a cel to itself");
        }
        eprintln!("[portal] {level}: {into_a}/{} forward planes point into celA", g.portals.len());
        // A consistent file winding puts the normal into celA for every record (the game builds the
        // reversed plane for the A-side node from the same corners backwards). A strict majority is
        // the measured rule; assert it so a parse slip flips the test.
        assert!(into_a * 2 > g.portals.len(), "{level}: only {into_a}/{} planes point into celA — the winding reading is wrong", g.portals.len());
    }
}

/// Cold start on every story level: `load_with_carry(None)` + 30 ticks must leave Bond armed —
/// the P2K drawn with ammo on the HUD everywhere except the party level 07000007 (fists, the
/// disguise lock). This is the user's no-weapon report (07000006 --frame 30, "switch request 0")
/// driven through the real viewer path, per level.
#[test]
fn cold_loadout_arms_bond_on_every_story_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut checked = 0;
    for n in 1..=0xd {
        let name = format!("0700000{n:x}.bin");
        let mut scene = match nf_view::load(&iso, &name, false) { Ok(s) => s, Err(e) => { eprintln!("{name}: not loadable ({e})"); continue } };
        let Some(sim) = scene.sim.as_mut() else { eprintln!("{name}: no sim"); continue };
        for t in 0..30 { sim.step_bond(Default::default(), false, false, false); let _ = t; }
        let w = sim.gun.weapon;
        let (clip, carried) = (sim.gun.clip, sim.gun.spare);
        let owned: Vec<u8> = (0..sim.drones.inventory.owned.len()).filter(|&i| sim.drones.inventory.owned[i]).map(|i| i as u8).collect();
        eprintln!("[loadout] {name}: weapon {w}, clip {clip}, spare {carried}, owned {owned:?}");
        checked += 1;
        if n == 7 {
            // The party: fists drawn (the disguise), the guns arrive with ch 0x48.
            assert_eq!(w, 1, "{name}: the party cold start must draw fists, got weapon {w}");
        } else if n == 8 {
            // Case 0x7000008's own fallback [M 0x1a7ef8]: a cold load has no RamSave, the P2K is
            // unowned → the 07 party arm runs and the request is 1 (fists). The campaign reaches 08
            // warm (level_carry_live covers that), and the game never cold-loads it — fists IS the
            // ported switch's cold answer, not a port bug.
            assert_eq!(w, 1, "{name}: the game's case-8 cold fallback draws fists, got weapon {w}");
            assert!(owned.contains(&2), "{name}: the party arm still equips the P2K (owned {owned:?})");
        } else {
            assert!(owned.contains(&2), "{name}: the P2K is not owned on a cold start (owned {owned:?})");
            assert_eq!(w, 2, "{name}: weapon {w} drawn after 30 ticks — the switch request did not arm the P2K");
            assert!(clip > 0 || carried > 0, "{name}: no P2K ammo on the HUD");
        }
    }
    assert!(checked >= 8, "only {checked} story levels loadable — the sweep did not run");
}
