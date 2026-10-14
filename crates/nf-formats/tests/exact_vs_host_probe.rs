//! fix-exact-math-swap3 probe: how far the EE-exact path can move from the host path, per function.
//! The merge held because renders moved "far more than float precision can explain"; this measures
//! each swapped function's host-vs-exact divergence over a wide domain so the camera dump can point
//! at the right layer.

use nf_formats::anim::{mul, quat_trans_to_mat, Mat4};
use nf_formats::placement::object_matrix;

/// master's host `object_matrix` (crates/nf-formats/src/placement.rs @ origin/master).
fn host_object_matrix(rot: [f32; 3], trans: [f32; 3]) -> [[f32; 4]; 4] {
    let (sx, cx) = rot[0].sin_cos();
    let (sy, cy) = rot[1].sin_cos();
    let (sz, cz) = rot[2].sin_cos();
    [
        [cy * cz, sx * sy * cz + cx * sz, cx * -sy * cz + sx * sz, 0.0],
        [cy * -sz, cx * cz - sx * sy * sz, sx * cz - cx * -sy * sz, 0.0],
        [sy, -sx * cy, cx * cy, 0.0],
        [trans[0], trans[1], trans[2], 1.0],
    ]
}

/// master's host `quat_trans_to_mat`.
fn host_quat_trans_to_mat(q: [f32; 4], t: [f32; 3]) -> Mat4 {
    let [x, y, z, w] = q;
    let (y2, z2, xx2, xw2) = (y + y, z + z, (x + x) * x, (x + x) * w);
    [
        [1.0 - (y2 * y + z2 * z), y2 * x + z2 * w, z2 * x - y2 * w, 0.0],
        [y2 * x - z2 * w, 1.0 - (xx2 + z2 * z), z2 * y + xw2, 0.0],
        [z2 * x + y2 * w, z2 * y - xw2, 1.0 - (xx2 + y2 * y), 0.0],
        [t[0], t[1], t[2], 1.0],
    ]
}

/// master's host `mul`.
fn host_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [[0f32; 4]; 4];
    for i in 0..4 { for j in 0..4 { r[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum(); } }
    r
}

fn diff(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> (f32, usize, usize) {
    let mut best = (0f32, 0usize, 0usize);
    for i in 0..4 { for j in 0..4 {
        let d = (a[i][j] - b[i][j]).abs();
        if d > best.0 { best = (d, i, j); }
    } }
    best
}

/// xorshift for reproducible quats/matrices.
struct R(u32);
impl R {
    fn next(&mut self) -> u32 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 17; self.0 ^= self.0 << 5; self.0 }
    fn f32(&mut self) -> f32 { (self.next() >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0 }
}

#[test]
fn probe_object_matrix_domain() {
    // A wide angle grid: every 0.37 rad step over [-4π, 4π] crossed with a few mixed triples —
    // NIS camera eulers land in this range (splines sweep whole turns).
    let mut worst = (0f32, [0f32; 3]);
    let mut a = -4.0 * std::f32::consts::PI;
    while a <= 4.0 * std::f32::consts::PI {
        for k in 0..7 {
            let rot = [a, a * 0.5 + k as f32 * 0.317, -a * 0.25 + k as f32 * 0.211];
            let (e, h) = (object_matrix(rot, [0.0; 3]), host_object_matrix(rot, [0.0; 3]));
            let (d, i, j) = diff(&e, &h);
            if d > worst.0 { worst = (d, rot); }
            let _ = (i, j);
        }
        a += 0.37;
    }
    println!("object_matrix max |exact-host| = {:e} at rot {:?}", worst.0, worst.1);
}

#[test]
fn probe_quat_trans_to_mat_domain() {
    let mut r = R(0x1234_5678);
    let mut worst = 0f32;
    for _ in 0..20000 {
        // Random quats, then near-identity and near-antipodal mixes.
        let q = [r.f32(), r.f32(), r.f32(), r.f32()];
        let (e, h) = (quat_trans_to_mat(q, [1.5, -2.5, 30.0]), host_quat_trans_to_mat(q, [1.5, -2.5, 30.0]));
        let (d, _, _) = diff(&e, &h);
        worst = worst.max(d);
    }
    println!("quat_trans_to_mat max |exact-host| over 20000 random quats = {:e}", worst);
}

#[test]
fn probe_mul_domain() {
    let mut r = R(0xdead_beef);
    let mut worst = 0f32;
    for _ in 0..20000 {
        let a = [[r.f32(), r.f32(), r.f32(), r.f32()]; 4];
        let b = [[r.f32(), r.f32(), r.f32(), r.f32()]; 4];
        let (e, h) = (mul(&a, &b), host_mul(&a, &b));
        let (d, _, _) = diff(&e, &h);
        worst = worst.max(d);
    }
    println!("mul max |exact-host| over 20000 random pairs = {:e}", worst);
}
