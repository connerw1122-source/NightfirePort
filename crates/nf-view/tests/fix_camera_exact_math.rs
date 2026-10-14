//! fix-exact-math-swap3: the frame-170 NIS camera matrix equals the value the decompile's arithmetic
//! gives. The merge held because "renders of master vs the branch differ by far more than float
//! precision can explain"; the dump (this file's `dump` sibling) shows the two trees' cameras differ
//! by a few ulps and the renders are pixel-identical, so the test pins the camera matrix to the
//! game's own arithmetic (the exact path):
//!   - `RotMatrix` 0x1e2368 / `Quat_QuatTransToMat` 0x1e3fa8: chop products, no-guard FPU adds
//!     (ps2float), PS2Sinf (0x120700) — proven bit-exact against the interpreter samples and the 40
//!     recorded calls (docs/formats/animation.md "Arithmetic", `diff_calls.rs`);
//!   - `psiBuildMatrixPalette` 0x111498: the VU0 ACC chain (chopped products, guard-bit sums).
//! The recompute here is independent of the sim's own matrix code: it reads the camera stream's key
//! inputs (`Script_GetInterp` 0x1da158's arguments) and re-runs the key interpolation + the inlined
//! decompile arithmetic. On a tree whose sim does NOT run the decompile's arithmetic (master's host
//! math), the bit assertion fails; the sister assertion (≤ 2e-6) passes on both and documents that
//! master differs only by precision.
//!
//! The camera path: `Script_Interp` case 4 (0x1d9a08:190-200) → `Script_GetInterp` keys →
//! `Quat_QuatTransToMat` → `Concat` with the script base (the SP placement's `RotMatrix` matrix).

use nf_game::nis::Obj;

type Mat4 = [[f32; 4]; 4];

/// `psiBuildMatrixPalette` 0x111498's product/sum chain (the exact `anim::mul`, inlined so the test
/// does not share the sim's code path).
fn decomp_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    use nf_formats::ps2float::{add, mul};
    let mut r = [[0f32; 4]; 4];
    for i in 0..4 { for j in 0..4 {
        let mut acc = mul(a[i][0], b[0][j]);
        for k in 1..4 { acc = add(acc, mul(a[i][k], b[k][j])); }
        r[i][j] = acc;
    } }
    r
}

/// `Quat_QuatTransToMat` 0x1e3fa8 on the EE FPU (the exact `quat_trans_to_mat`, inlined).
fn decomp_quat_trans_to_mat(q: [f32; 4], t: [f32; 3]) -> Mat4 {
    use nf_formats::ps2float as pf;
    let [x, y, z, w] = q;
    let fadd = |a, b| pf::add_noguard(a, b, true);
    let fsub = |a: f32, b: f32| pf::add_noguard(a, -b, true);
    let m = pf::mul;
    let (y2, z2, x2) = (fadd(y, y), fadd(z, z), fadd(x, x));
    let (xx, xw) = (m(x2, x), m(x2, w));
    let (zy, zz) = (m(y2, y), m(z2, z));
    [
        [fsub(1.0, fadd(zy, zz)), fadd(m(y2, x), m(z2, w)), fsub(m(z2, x), m(y2, w)), 0.0],
        [fsub(m(y2, x), m(z2, w)), fsub(1.0, fadd(xx, zz)), fadd(m(z2, y), xw), 0.0],
        [fadd(m(z2, x), m(y2, w)), fsub(m(z2, y), xw), fsub(1.0, fadd(xx, zy)), 0.0],
        [t[0], t[1], t[2], 1.0],
    ]
}

/// `Script_GetInterp` (0x1da158) over the stream's keys, then `Script_Interp` case 4's `Concat` with
/// the script base — mirrored from `nf_game::nis::Script::keys` + `interp` case 4 with the decompile's
/// matrix arithmetic in place of the sim's calls.
fn recompute_camera(sim: &nf_view::scene::Sim) -> Option<(Mat4, Mat4, f32)> {
    let (hash, _i, clock_read, first, last, flags) = sim.drones.scripts.camera_stream_inputs()?;
    // `Script::update` runs the streams' `Script_Run` (commands + `Script_Interp` at the current
    // clock) BEFORE `self.clock += 1; s.clock += 1` (nis.rs update loop) — so a post-tick read is one
    // past the clock the camera's interp consumed.
    let clock = clock_read - 1.0;
    let script = sim.drones.scripts.players.iter().find_map(|p| p.info.iter().flatten().find(|s| s.nis.hashcode == hash))?;
    let nis = &script.nis;
    let k = &nis.keys[first..=last];
    let n = k.len() - 1;
    let c = clock.floor();
    // The last key with time ≤ ⌊clock⌋ + 0.001 (−1 before the first).
    let idx = k.iter().take_while(|x| x.time <= c + 0.001).count() as isize - 1;
    let lerp = |x: f32, y: f32, t: f32| x * (1.0 - t) + y * t;
    let (pos, q) = if flags & 2 == 0 {
        let (i0, i1, t) = if idx < 0 { (0, 0, 0.0) } else if (idx as usize) < n {
            let i0 = idx as usize;
            (i0, i0 + 1, (c - k[i0].time) / (k[i0 + 1].time - k[i0].time))
        } else { (n, n, 0.0) };
        let p = [0, 1, 2].map(|j| lerp(k[i0].pos[j], k[i1].pos[j], t));
        (p, nf_game::anim::slerp(k[i0].quat, k[i1].quat, t))
    } else {
        // `GetSplineWeights`: Catmull-Rom over i−1..i+2 (clamped); the rotation slerps i → i+1 by t.
        let (ix, wt, t) = if idx < 0 || idx as usize >= n {
            let j = idx.clamp(0, n as isize) as usize;
            ([j; 4], [0.0, 1.0, 0.0, 0.0], 0.0)
        } else {
            let j = idx as usize;
            let t = (c - k[j].time) / (k[j + 1].time - k[j].time);
            let (t2, t3) = (t * t, t * t * t);
            ([j.saturating_sub(1), j, (j + 1).min(n), (j + 2).min(n)],
             [t2 - (t + t3) * 0.5, 1.0 - t2 * 2.5 + t3 * 1.5, t * 0.5 + t2 + t2 - t3 * 1.5, (t3 - t2) * 0.5], t)
        };
        let sum = |f: &dyn Fn(&nf_formats::nis::Key) -> f32| (0..4).map(|m| f(&k[ix[m]]) * wt[m]).sum::<f32>();
        let p = [sum(&|x| x.pos[0]), sum(&|x| x.pos[1]), sum(&|x| x.pos[2])];
        (p, nf_game::anim::slerp(k[ix[1]].quat, k[ix[2]].quat, t))
    };
    let keys_matrix = decomp_quat_trans_to_mat(q, pos);
    let camera = decomp_mul(&keys_matrix, &script.matrix);
    Some((camera, keys_matrix, script.matrix[3][0]))
}

fn bits(m: &Mat4) -> String {
    m.iter().map(|r| r.iter().map(|v| format!("{:08x}", v.to_bits())).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join(" | ")
}

#[test]
fn the_frame_170_camera_matrix_is_the_decompiles_arithmetic() {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_mut().unwrap();
    // The intro cut-scene runs from the level start; the viewer's --frame path steps real ticks.
    for _ in 0..170 { sim.step_bond(Default::default(), false, false, false); }
    let (m, fov) = sim.drones.scripts.camera().expect("the intro cut-scene camera at tick 170");
    let (re, keys_m, base_x) = recompute_camera(sim).expect("camera stream inputs");
    eprintln!("camera @170  {}", bits(&m));
    eprintln!("recompute    {}", bits(&re));
    eprintln!("keys matrix  {}", bits(&keys_m));
    eprintln!("base m[3][0] = {base_x:.6}  fov = {fov}");
    // The camera matrix the sim holds IS the decompile's arithmetic over the stream's keys.
    assert_eq!(bits(&m), bits(&re), "the sim's camera matrix must be the decompile's arithmetic over the stream keys");
    // Sister: the same numbers within float precision (this is the assertion master also passes —
    // the two trees differ by a few ulps here, not by the held merge's "far more").
    for i in 0..4 { for j in 0..4 {
        assert!((m[i][j] - re[i][j]).abs() <= 2e-6, "entry [{i}][{j}] diverged: {} vs {}", m[i][j], re[i][j]);
    } }
    // And the render-facing derivation is stable across the trees: the yaw bits and the fov.
    let f = m[2];
    let yaw = f[0].atan2(f[2]);
    eprintln!("derived yaw bits {:08x} pitch bits {:08x}", yaw.to_bits(), (f[1] / (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt().max(1e-6)).clamp(-1.0, 1.0).asin().to_bits());
    let _ = (fov, Obj::Camera { matrix: m, fov: 1.0, src: (0, 0) });
}
