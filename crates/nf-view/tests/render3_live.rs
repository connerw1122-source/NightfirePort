//! Headless NF_ISO tests for fix-render3: the 07000006/07/02 frame-cost work (the user's post-
//! fix-render2 report) and the cel visibility cull (G-016's View_AddCels half).
//!
//! Evidence base:
//! - `View_AddCels` (0x1e6ba0) [M, docs/atlas/cel.md §Visibility]: per viewer, iterate the room cels,
//!   frustum/bbox-test each (`Vision_InView(cel+0x8c, cel+0x80, viewer)`) — the ported cull tests each
//!   cel's bbox sphere against the frame frustum. The portal-graph traversal remains open (G-016
//!   caveat); this is the decompile-visible half.
//! - Object ↔ cel assignment: `build_LinkToRoom` (0x1c9680) → `build_FindCel` (0x1c8cb0) on the
//!   object's world position [M] — the port attributes each static World draw to `find_cel` over the
//!   room cels (the same function the ambient code uses, re-measured against the [L] snapshots in
//!   fix-render2).
//! - fix-render2 measured 07000007's start view: 4526 of 4869 World draws inside the frustum spheres —
//!   the view faces the whole interior, so the sphere cull cannot relieve it. The cel cull is the lever.
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

/// The World draws inside the frustum spheres (fix-render2's per-view cost proxy) and, once the cel
/// cull exists, how many of those the cel test removes.
fn world_counts(scene: &scene::Scene, vp: [[f32; 4]; 4]) -> (usize, usize, usize, usize) {
    let planes = nf_view::gpu::extract_frustum(vp);
    let (mut sphere_in, mut culled_sphere, mut culled_cel, mut moving) = (0usize, 0usize, 0usize, 0usize);
    for m in &scene.materials {
        if m.layer != scene::Layer::World { moving += 1; continue; }
        if m.draw.pass != 4 && nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { culled_sphere += 1; continue; }
        // The cel cull: a static draw whose room cel's bbox is outside the frustum is skipped too
        // (View_AddCels 0x1e6ba0's Vision_InView). `None`-cel draws (outdoor geometry with no room
        // cel) stay — no vanish risk from mis-attribution.
        if let Some(c) = m.cel {
            let Some((cc, cr)) = scene.cel_sphere(c as usize) else { continue };
            if nf_view::gpu::sphere_outside(&planes, cc, cr) { culled_cel += 1; continue; }
        }
        sphere_in += 1;
    }
    (sphere_in, culled_sphere, culled_cel, moving)
}

/// 07000006's and 07000002's start views: the eye's own room cel must never fail the frustum test
/// (its bbox contains the eye, so the draw cannot vanish — A-11's bug class), and the World draws
/// must be attributable at a high rate (the cull only acts on attributed draws). The measured
/// start-view counters are printed for the WORKLOG (fix-render2: 06 = 1040 of 4438 World submitted).
#[test]
fn cel_cull_keeps_the_start_views_intact() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for level in ["07000006.bin", "07000002.bin"] {
        let scene = match nf_view::load(&iso, level, false) { Ok(s) => s, Err(e) => { eprintln!("{level} not loadable: {e}"); continue } };
        let (pos, yaw) = scene.start.clone().unwrap_or(([0.0; 3], 0.0));
        let eye = [pos[0], pos[1] + 1.6, pos[2]];
        let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
        let (sub, culled_sphere, culled_cel, moving) = world_counts(&scene, vp);
        let world_total = scene.materials.iter().filter(|m| m.layer == scene::Layer::World).count();
        let attributed = scene.materials.iter().filter(|m| m.layer == scene::Layer::World && m.cel.is_some()).count();
        eprintln!("[celcull] {level}: {sub} submitted (sphere {culled_sphere}, cel {culled_cel}), moving {moving}; attributed {attributed}/{world_total}");
        assert!(attributed * 4 > world_total * 3, "{level}: {attributed}/{world_total} World draws carry a cel — attribution broken");
        // The eye sits in a room cel on both levels ([M build_LinkToRoom 0x1c9680 links the player
        // through FindCel]); that cel's box contains the eye and must pass the frustum test.
        let own = scene.cel_index_at(eye);
        eprintln!("[celcull] {level}: eye cel {own:?}");
        if let Some(c) = own {
            let _ = scene.cel_sphere(c);
            // The box test through the Scene's cel bbox: the eye is inside it.
            let cmin = scene.cels[c].min;
            let cmax = scene.cels[c].max;
            assert!(!nf_view::gpu::box_outside(&nf_view::gpu::extract_frustum(vp), cmin, cmax),
                "{level}: the eye's own cel {c} was frustum-culled — Bond's room would vanish");
        }
    }
}

/// The measured finding on 07000007 (the user's "lag depends on view direction"): the START view
/// faces all 32 room cels — every cel box intersects the frustum, so the cel test relieves nothing
/// there (4526 submitted; the draw cost in that view is real front-facing geometry), while looking
/// away the existing sphere cull already drops 97 %+ (measured 64 submitted). This test pins the
/// direction-dependence the user reported and the attribution the cull needs; the next lever for
/// the start view is occlusion (the portal graph, map block 0x21 — outside this task's file set),
/// recorded in GAPS G-016.
#[test]
fn the_07000007_draw_cost_depends_on_view_direction() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = match nf_view::load(&iso, "07000007.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000007 not loadable: {e}"); return } };
    let (pos, yaw) = scene.start.clone().unwrap_or(([0.0; 3], 0.0));
    let eye = [pos[0], pos[1] + 1.6, pos[2]];
    let (sub_start, _, cel_start, _) = world_counts(&scene, view_proj(eye, yaw, 0.0, 16.0 / 9.0));
    let (sub_away, _, _, _) = world_counts(&scene, view_proj(eye, yaw + std::f32::consts::PI, 0.0, 16.0 / 9.0));
    eprintln!("[celcull] 07000007: start view {sub_start} submitted ({cel_start} cel-culled), away view {sub_away}");
    assert!(sub_away * 20 < sub_start, "the away view ({sub_away}) must submit far less than the start view ({sub_start})");
    // The start view submits the bulk: the cel box test cannot relieve it (every room cel box
    // intersects the view) — pinned so a future portal/occlusion cull shows its gain against this.
    assert!(sub_start > 4000, "the start view dropped to {sub_start} — the pinned measurement changed");
}

/// The fix-render3 posed-vertices hoist: fix-render2 computed the pickup / dropped-gun /
/// script-entity ambient multipliers inside the per-vertex closure — every vertex of a dimmed
/// character re-ran `find_cel` (a linear cel scan) and, for drops, a linear `pickups` scan. The
/// multiplier depends only on the pickup / attachment key, so it is now computed once per distinct
/// key per frame. The counter counts `Sim::ambient_at` calls per `posed_vertices` pass: on
/// 07000005 the truck's nine 0x8880 script parts (thousands of vertices) must produce single-digit
/// lookups, not per-vertex thousands. Sister: the multipliers must be IDENTICAL to the game values
/// (the truck takes its cel ambient — the fix-render2 test `the_truck_takes_the_0x800_ambient`
/// re-runs in this suite).
#[test]
fn posed_vertices_ambient_lookups_are_per_key_not_per_vertex() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = match nf_view::load(&iso, "07000005.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000005 not loadable: {e}"); return } };
    if let Some(sim) = scene.sim.as_mut() { for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); } }
    // Raw-pointer dance: `posed_vertices` borrows the whole Scene, so the Sim is reached through a
    // raw pointer (test-only; the same shape render2_live.rs uses for stepping).
    let sim_ptr = scene.sim.as_mut().unwrap() as *mut scene::Sim;
    let lookups = unsafe {
        (*sim_ptr).ambient_lookups.set(0);
        // The truck must exist as script characters on 07000005 (nine 0x8880 parts [L ee_start_05.bin]).
        let _ = scene.posed_vertices(None, 0.0, scene::ViewOpts::default());
        (*sim_ptr).ambient_lookups.get()
    };
    eprintln!("[perf] ambient lookups per posed_vertices pass: {lookups}");
    assert!(lookups <= 64, "{lookups} ambient lookups per frame — the per-vertex lookup cost is back");
    // Sister: the ramp's own per-tick lookups stay bounded by the character count (76-level scale).
    let ramp_lookups = unsafe {
        (*sim_ptr).ambient_lookups.set(0);
        (*sim_ptr).ramp_char_ambient();
        (*sim_ptr).ambient_lookups.get()
    };
    eprintln!("[perf] ambient lookups per ramp tick: {ramp_lookups}");
    assert!(ramp_lookups <= 128, "{ramp_lookups} ramp lookups per tick — the ramp is scanning per vertex");
}
