//! fix-interp-ghosting: the playtest gun/arm smear while moving (2026-10-13, 07000005). With the
//! 120 fps interpolation (G-043) the draw loop blends the previous tick's skin palette into the
//! latest one (`lerp_palette`) while the camera blends by angle (`lerp_cam`). The view model (the
//! hands and gun) is posed in world space from each tick's camera ([M `Player_PositionGun`
//! 0x1a3b40] via `Sim::gun_matrix`), so mid-turn its component-wise-lerped matrices do not match
//! the angle-lerped camera: the gun swims and smears against the view. A component-wise lerp of a
//! rotation also is not a rotation — it shears and shrinks mid-way (rows of two 30°-apart
//! rotations average to length cos(15°) ≈ 0.966).
//!
//! The contract (the task's own words, tested here):
//! 1. at alpha ½ between two ticks with a 30° camera turn, the view model's CAMERA-space matrix
//!    equals the average of its two tick matrices (no swim);
//! 2. an interpolated world matrix stays orthonormal (sister: the old component lerp fails both).
//!
//! Test 1 runs the real input path (`nf_view::load` + `Sim::step_bond`, a real 30° `Input.turn`
//! tick on 07000005), builds the two ticks' palettes through `Scene::skin_palette` exactly as the
//! draw loop does, and pushes them through the draw loop's blend path
//! (`lerp_palette` + `reattach_view_model`). NF_ISO opt-in.

use glam::Vec3;
use nf_formats::anim::{mul as mat_mul, Mat4};
use nf_view::Camera;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Wrap an angle into −π..π.
fn wrap_pi(a: f32) -> f32 {
    let mut a = a;
    while a > std::f32::consts::PI { a -= std::f32::consts::TAU; }
    while a < -std::f32::consts::PI { a += std::f32::consts::TAU; }
    a
}

/// The camera basis the first-person gun is posed from: `gun_matrix` reads the viewer's yaw/pitch
/// and eye (`bond.head()`) — the same values the tick `Camera` holds — and its axes are
/// right = f×Y, up = right×f, fwd = f (the `cam_axes` basis; `Player_PositionGun` 0x1a3b40).
/// Returned as rows (right, up, fwd) with the eye as translation: the port's row-vector matrices
/// (`anim::transform` = v·M).
fn cam_rows(pos: [f32; 3], yaw: f32, pitch: f32) -> Mat4 {
    let f = [pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()];
    let r = [-f[2], 0.0, f[0]]; // f × Y, then normalized
    let rl = (r[0] * r[0] + r[2] * r[2]).sqrt();
    let r = [r[0] / rl, 0.0, r[2] / rl];
    let u = [r[1] * f[2] - r[2] * f[1], r[2] * f[0] - r[0] * f[2], r[0] * f[1] - r[1] * f[0]];
    [[r[0], r[1], r[2], 0.0], [u[0], u[1], u[2], 0.0], [f[0], f[1], f[2], 0.0], [pos[0], pos[1], pos[2], 1.0]]
}

fn trans(t: [f32; 3]) -> Mat4 {
    [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [t[0], t[1], t[2], 1.0]]
}

/// Transpose the 3×3 block (the rotation), keeping the translation row.
fn rot_transpose(m: &Mat4) -> Mat4 {
    let mut r = *m;
    for i in 0..3 { for j in 0..3 { r[i][j] = m[j][i]; } }
    r[3] = [m[3][0], m[3][1], m[3][2], 1.0];
    r
}

/// world → view: v_view = (v_world − eye) · Rᵀ (rows of the result = right/up/fwd dots).
fn c_of(cam: &Camera) -> Mat4 {
    let p = [cam.pos.x, cam.pos.y, cam.pos.z];
    let r = cam_rows([0.0; 3], cam.yaw, cam.pitch); // rotation only — the eye goes in the T(−p) half
    mat_mul(&trans([-p[0], -p[1], -p[2]]), &rot_transpose(&r))
}

/// view → world would be C⁻¹ = R · T(eye) — the production reattach builds its own inverse.

fn cam_of(pos: Vec3, yaw: f32, pitch: f32, zoom: f32) -> Camera {
    Camera { pos, yaw, pitch, fov: std::f32::consts::FRAC_PI_3 / zoom }
}

/// The camera's screen axes (the draw loop's `cam_axes`), for `ViewOpts`.
fn axes(cam: &Camera) -> ([f32; 3], [f32; 3]) {
    let f = [cam.pitch.cos() * cam.yaw.sin(), cam.pitch.sin(), cam.pitch.cos() * cam.yaw.cos()];
    let r = [-f[2], 0.0, f[0]];
    let rl = (r[0] * r[0] + r[2] * r[2]).sqrt();
    let r = [r[0] / rl, 0.0, r[2] / rl];
    let u = [r[1] * f[2] - r[2] * f[1], r[2] * f[0] - r[0] * f[2], r[0] * f[1] - r[1] * f[0]];
    (r, u)
}

/// The tick camera + palette the way the draw loop builds them at a tick boundary: opts axes from
/// the tick camera, palette from `Scene::skin_palette` (no visibility cull — the view model's cel
/// is the camera's).
fn tick_palette(scene: &mut nf_view::scene::Scene, layout: &nf_view::scene::SkinLayout) -> (Camera, Vec<Mat4>) {
    let sim = scene.sim.as_ref().unwrap();
    let cam = cam_of(Vec3::from_array(sim.bond.head()), sim.bond.yaw, sim.bond.pitch * std::f32::consts::FRAC_PI_2, sim.zoom);
    let (r, u) = axes(&cam);
    let opts = nf_view::scene::ViewOpts { reinforcements: false, firing: false, right: r, up: u };
    let (pal, _) = scene.skin_palette(layout, None, sim.drones.frame as f32, opts, None);
    (cam, pal)
}

/// The first-person view model's palette entries: the object matrix and its bones, plus the
/// attachment slots hung on the view model character (the rigid gun parts, the muzzle flash) —
/// the same set the draw loop re-attaches.
fn view_model_entries(scene: &nf_view::scene::Scene, layout: &nf_view::scene::SkinLayout) -> Vec<usize> {
    let sim = scene.sim.as_ref().unwrap();
    let ci = *scene.view_models.get(&sim.gun.weapon)
        .unwrap_or_else(|| panic!("no first-person view model for the held weapon {}", sim.gun.weapon));
    let base = layout.char_base[ci] as usize;
    let nb = scene.characters[ci].skeleton.bones as usize;
    let mut out: Vec<usize> = (base..=base + nb).collect();
    let att = layout.attach_base as usize;
    for (k, a) in scene.attachments.iter().enumerate() {
        if a.character as usize == ci { out.push(att + k); }
    }
    out
}

const TRANS_TOL: f32 = 5e-4;
const ROT_TOL: f32 = 2e-3;

fn assert_close(label: &str, k: usize, expected: &Mat4, got: &Mat4) {
    let (mt, jt) = worst_trans(expected, got);
    assert!(mt <= TRANS_TOL, "{label}: entry {k} translation[{jt}] off by {mt:.6} (> {TRANS_TOL}) — the view model swims against the interpolated camera\nexpected row3 {:?}\ngot      row3 {:?}", expected[3], got[3]);
    let (mr, ri, rj) = worst_rot(expected, got);
    assert!(mr <= ROT_TOL, "{label}: entry {k} rotation[{ri}][{rj}] off by {mr:.6} (> {ROT_TOL}) — the blended view model pose is not the pose average (the mesh shears/shrinks mid-turn)\nexpected {:?}\ngot      {:?}", expected[ri], got[ri]);
}

fn worst_trans(a: &Mat4, b: &Mat4) -> (f32, usize) {
    let (mut m, mut j) = (0.0f32, 0usize);
    for c in 0..3 { let d = (a[3][c] - b[3][c]).abs(); if d > m { m = d; j = c; } }
    (m, j)
}

fn worst_rot(a: &Mat4, b: &Mat4) -> (f32, usize, usize) {
    let (mut m, mut mi, mut mj) = (0.0f32, 0usize, 0usize);
    for i in 0..3 { for j in 0..3 { let d = (a[i][j] - b[i][j]).abs(); if d > m { m = d; mi = i; mj = j; } } }
    (m, mi, mj)
}

/// The named contract: at alpha ½ across one 30° camera-turn tick, every view model palette
/// entry's camera-space matrix equals the average of its two ticks' camera-space matrices.
/// The old path (component-wise world lerp, no camera-space re-attach) fails this — that is the
/// reported smear.
#[test]
fn view_model_stays_glued_in_camera_space_across_a_turn() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    // 07000005 has no intro (walkreal_07000005 asserts control at the start); if a script ever
    // holds Bond here, skip it the game's own way before measuring.
    if scene.sim.as_ref().unwrap().disabled {
        for _ in 0..600 {
            let s = scene.sim.as_mut().unwrap();
            if !s.disabled { break; }
            s.drones.scripts.skip_request = true;
            s.step_bond(Default::default(), false, false, false);
        }
    }
    assert!(!scene.sim.as_ref().unwrap().disabled, "the intro never released control");
    for _ in 0..5 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }

    let layout = scene.skin_layout().0;
    let (cam_a, pa) = tick_palette(&mut scene, &layout);
    let yaw_a = cam_a.yaw;

    // One real 30° turn tick (the mouse path: `Player_Update` adds `input.turn` to the yaw
    // [M player.rs, `self.yaw += input.turn`]; 30°/tick is a hard mouse flick).
    let turn = 30f32.to_radians();
    scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input { turn, ..Default::default() }, false, false, false);
    let (cam_b, pb) = tick_palette(&mut scene, &layout);
    let yaw_b = cam_b.yaw;
    assert!((wrap_pi(yaw_b - yaw_a) - turn).abs() < 1e-3,
        "the input tick must turn the view 30°: {yaw_a} → {yaw_b}");

    // The interpolated draw camera at alpha ½ (the draw loop's `lerp_cam` shape: position lerp +
    // short-arc yaw + pitch lerp).
    let cam_mid = cam_of(cam_a.pos.lerp(cam_b.pos, 0.5), nf_view::lerp_yaw(yaw_a, yaw_b, 0.5),
                         cam_a.pitch + (cam_b.pitch - cam_a.pitch) * 0.5, 1.0);
    let (c_a, c_b, c_mid) = (c_of(&cam_a), c_of(&cam_b), c_of(&cam_mid));

    let vm = view_model_entries(&scene, &layout);
    assert!(vm.len() >= 2, "the view model must contribute its object matrix plus bones");

    // The draw loop's blend path at alpha ½, then back into camera space.
    let mut drawn = nf_view::lerp_palette(&pa, &pb, 0.5);
    nf_view::reattach_view_model(&mut drawn, &pa, &pb, 0.5, &cam_a, &cam_b, &cam_mid, &vm);

    for &k in &vm {
        let rel_a = mat_mul(&pa[k], &c_a);
        let rel_b = mat_mul(&pb[k], &c_b);
        // The contract: the camera-space pose at ½ is the average of the two ticks' camera-space
        // poses (for the gun's root this is exact — standing still, the local pose is constant;
        // for the bones the per-tick anim delta is tiny, well inside the tolerance).
        let mut expected = rel_a;
        for i in 0..4 { for j in 0..4 { expected[i][j] = (rel_a[i][j] + rel_b[i][j]) * 0.5; } }
        let rel_drawn = mat_mul(&drawn[k], &c_mid);
        // Run-wide diagnostics first (the profile reads like the playtest report), then the asserts.
        let (mt, jt) = worst_trans(&expected, &rel_drawn);
        let (mr, ri, rj) = worst_rot(&expected, &rel_drawn);
        eprintln!("[interp-ghost] entry {k}: max translation delta {mt:.6} (comp {jt}), max rotation delta {mr:.6} (comp [{ri}][{rj}])");
        assert_close("camera-space average", k, &expected, &rel_drawn);
    }
}

/// Sister (no ISO): the world-matrix blend is a rotation — slerp of the rotation part + lerp of
/// the translation keeps every interpolated matrix orthonormal, and the 4-unit snap for
/// teleports/cuts is intact. The old component-wise lerp fails the orthonormal rows (two 30°-apart
/// rotations average to row length cos 15° ≈ 0.966) — the reported shear/shrink.
#[test]
fn interpolated_world_matrices_stay_orthonormal() {
    let r0 = cam_rows([1.0, 2.0, 3.0], 0.0, 0.0);
    let r1 = cam_rows([1.1, 2.0, 3.2], 30f32.to_radians(), 0.0);
    let m = nf_view::lerp_palette(&[r0], &[r1], 0.5)[0];
    // Orthonormal rows (unit, mutually orthogonal), within f32 noise of the slerp arc.
    for i in 0..3 {
        let n = (m[i][0] * m[i][0] + m[i][1] * m[i][1] + m[i][2] * m[i][2]).sqrt();
        assert!((n - 1.0).abs() < 1e-3, "row {i} length {n} — the blend is not a rotation (component lerp gives cos 15° ≈ 0.966)");
    }
    for i in 0..3 { for j in (i + 1)..3 {
        let d = m[i][0] * m[j][0] + m[i][1] * m[j][1] + m[i][2] * m[j][2];
        assert!(d.abs() < 1e-3, "rows {i}/{j} dot {d} — the blend shears");
    } }
    // Halfway round the 30° turn: the blended forward is 15° off r0's.
    let f0 = [r0[2][0], r0[2][1], r0[2][2]];
    let f1 = [r1[2][0], r1[2][1], r1[2][2]];
    let fm = [m[2][0], m[2][1], m[2][2]];
    let ang = |a: [f32; 3], b: [f32; 3]| (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).acos();
    assert!((ang(f0, fm) - 15f32.to_radians()).abs() < 1e-3,
        "the blended forward must be halfway (15°): got {}°", ang(f0, fm).to_degrees());
    assert!((ang(f1, fm) - 15f32.to_radians()).abs() < 1e-3);
    // The translation lerps, the 4-unit snap is intact.
    assert!((m[3][0] - 1.05).abs() < 1e-5 && (m[3][2] - 3.1).abs() < 1e-5, "translation must lerp");
    let far = cam_rows([1.0, 2.0, 3.0], 0.0, 0.0);
    let snapped = nf_view::lerp_palette(&[far], &[cam_rows([1.0 + 5.0, 2.0, 3.0], 0.0, 0.0)], 0.5)[0];
    assert_eq!(snapped[3][0], 6.0, "a >4-unit jump snaps to the newer tick (cuts/teleports do not smear)");
}
