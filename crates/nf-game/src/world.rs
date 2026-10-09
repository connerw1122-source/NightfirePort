//! Static collision world: every placed object's collision geometry (map block 0x2E, docs/formats/collision.md)
//! transformed to world space. Used for line of sight and floors. The game tests rays against each cel's box tree
//! (`Intersect_RayGeom`); here each object keeps its triangles behind one AABB — same answers, simpler culling.

use nf_formats::level::{Instance, Level};
use nf_formats::placement::transform;

pub type V3 = [f32; 3];

struct Object {
    lo: V3,
    hi: V3,
    tris: std::ops::Range<usize>,
}

pub struct CollisionWorld {
    tris: Vec<[V3; 3]>,
    objects: Vec<Object>,
}

fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }

impl CollisionWorld {
    /// From the level's instances (as drawn: cel placements; skipped-cel blockers included — they block).
    pub fn build(level: &Level, instances: &[Instance]) -> CollisionWorld {
        let (mut tris, mut objects) = (Vec::new(), Vec::new());
        for inst in instances {
            let Some(c) = &level.maps[inst.map].objects[inst.object].collision else { continue };
            let start = tris.len();
            let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
            for t in c.triangles() {
                let w = t.map(|p| transform(&inst.matrix, p));
                for p in &w { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } }
                tris.push(w);
            }
            if tris.len() > start { objects.push(Object { lo, hi, tris: start..tris.len() }); }
        }
        CollisionWorld { tris, objects }
    }

    pub fn triangle_count(&self) -> usize { self.tris.len() }

    /// No geometry (tests).
    pub fn empty() -> CollisionWorld { CollisionWorld { tris: Vec::new(), objects: Vec::new() } }

    /// One object holding these world-space triangles (tests).
    pub fn from_triangles(tris: Vec<[V3; 3]>) -> CollisionWorld {
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for p in tris.iter().flatten() { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } }
        let n = tris.len();
        CollisionWorld { tris, objects: vec![Object { lo, hi, tris: 0..n }] }
    }

    /// First hit along the segment `from → to` as a fraction in [0, 1] (Möller–Trumbore, both faces).
    pub fn segment(&self, from: V3, to: V3) -> Option<f32> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<f32>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        for o in &self.objects {
            if (0..3).any(|k| o.hi[k] < lo[k] || o.lo[k] > hi[k]) { continue; }
            for t in &self.tris[o.tris.clone()] {
                let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                let p = cross(d, e2);
                let det = dot(e1, p);
                if det.abs() < 1e-12 { continue; }
                let inv = 1.0 / det;
                let s = sub(from, t[0]);
                let u = dot(s, p) * inv;
                if !(0.0..=1.0).contains(&u) { continue; }
                let q = cross(s, e1);
                let v = dot(d, q) * inv;
                if v < 0.0 || u + v > 1.0 { continue; }
                let tt = dot(e2, q) * inv;
                if (0.0..=1.0).contains(&tt) && best.is_none_or(|b| tt < b) { best = Some(tt); }
            }
        }
        best
    }

    /// First hit along `from → to`: the fraction in [0, 1] and the unit normal of the triangle hit (facing `from`).
    pub fn segment_hit(&self, from: V3, to: V3) -> Option<(f32, V3)> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<(f32, V3)>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        for o in &self.objects {
            if (0..3).any(|k| o.hi[k] < lo[k] || o.lo[k] > hi[k]) { continue; }
            for t in &self.tris[o.tris.clone()] {
                let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                let p = cross(d, e2);
                let det = dot(e1, p);
                if det.abs() < 1e-12 { continue; }
                let inv = 1.0 / det;
                let s = sub(from, t[0]);
                let u = dot(s, p) * inv;
                if !(0.0..=1.0).contains(&u) { continue; }
                let q = cross(s, e1);
                let v = dot(d, q) * inv;
                if v < 0.0 || u + v > 1.0 { continue; }
                let tt = dot(e2, q) * inv;
                if (0.0..=1.0).contains(&tt) && best.is_none_or(|b| tt < b.0) {
                    let n = cross(e1, e2);
                    let l = dot(n, n).sqrt().max(1e-12);
                    let n = if dot(n, d) > 0.0 { [-n[0] / l, -n[1] / l, -n[2] / l] } else { [n[0] / l, n[1] / l, n[2] / l] };
                    best = Some((tt, n));
                }
            }
        }
        best
    }

    /// The player's capsule against the static geometry (`Intersect_CylGeom` 0x109da0, docs/systems/player.md): the
    /// segment `a`–`b` with radius `r`, triangle by triangle, each push applied before the next triangle. Per triangle:
    /// the segment point where it meets the plane (clamped; the point nearest the centroid when parallel), the closest
    /// triangle point to it, and when that lies on an edge the segment point nearest it; closer than `r` → pushed out
    /// along the separation. Returns the summed push and whether a walkable face (normal y > 0.7) touched below `b`.
    /// *Deviation*: both faces count (the game's triangles are one-sided by their stored plane; the port's winding is
    /// not known to match it).
    pub fn capsule_push(&self, a: V3, b: V3, r: f32) -> (V3, bool) {
        let (mut a, mut b) = (a, b);
        let mut total = [0f32; 3];
        let mut walkable = false;
        let grow = r + 0.1;
        let mut lo = [0f32; 3];
        let mut hi = [0f32; 3];
        for k in 0..3 { lo[k] = a[k].min(b[k]) - grow; hi[k] = a[k].max(b[k]) + grow; }
        for o in &self.objects {
            if (0..3).any(|k| o.hi[k] < lo[k] || o.lo[k] > hi[k]) { continue; }
            for t in &self.tris[o.tris.clone()] {
                let tl = [0, 1, 2].map(|k| t[0][k].min(t[1][k]).min(t[2][k]));
                let th = [0, 1, 2].map(|k| t[0][k].max(t[1][k]).max(t[2][k]));
                let (sl, sh) = ([0, 1, 2].map(|k| a[k].min(b[k]) - grow), [0, 1, 2].map(|k| a[k].max(b[k]) + grow));
                if (0..3).any(|k| th[k] < sl[k] || tl[k] > sh[k]) { continue; }
                let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
                let nl = dot(n, n).sqrt();
                if nl < 1e-12 { continue; }
                let n = [n[0] / nl, n[1] / nl, n[2] / nl];
                let ab = sub(b, a);
                let denom = dot(n, ab);
                let mut p = if denom.abs() > 0.0002 {
                    let tt = (-(dot(n, sub(a, t[0]))) / denom).clamp(0.0, 1.0);
                    [a[0] + ab[0] * tt, a[1] + ab[1] * tt, a[2] + ab[2] * tt]
                } else {
                    let c = [0, 1, 2].map(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.0);
                    closest_on_segment(a, b, c)
                };
                let (q, on_edge) = closest_on_triangle(p, t);
                if on_edge { p = closest_on_segment(a, b, q); }
                let v = sub(p, q);
                let dist = dot(v, v).sqrt();
                if dist >= r { continue; }
                // The side the capsule is on: along the separation, or (crossing the face) the normal toward the top.
                let dir = if dist > 1e-6 { [v[0] / dist, v[1] / dist, v[2] / dist] } else if dot(n, sub(a, q)) >= 0.0 { n } else { [-n[0], -n[1], -n[2]] };
                let push = [dir[0] * (r - dist), dir[1] * (r - dist), dir[2] * (r - dist)];
                for k in 0..3 { a[k] += push[k]; b[k] += push[k]; total[k] += push[k]; }
                if n[1].abs() > 0.7 && q[1] < b[1] { walkable = true; }
            }
        }
        (total, walkable)
    }

    /// Clear line between two points (`Collide_LineOfSight` stand-in: static geometry only).
    pub fn line_of_sight(&self, a: V3, b: V3) -> bool { self.segment(a, b).is_none() }

    /// Height of the first collision surface straight below `p` within `depth` (starting 0.5 above it).
    pub fn floor_below(&self, p: V3, depth: f32) -> Option<f32> {
        let from = [p[0], p[1] + 0.5, p[2]];
        let to = [p[0], p[1] - depth, p[2]];
        self.segment(from, to).map(|t| from[1] + (to[1] - from[1]) * t)
    }
}

fn closest_on_segment(a: V3, b: V3, p: V3) -> V3 {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    let t = if l2 > 0.0 { (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0) } else { 0.0 };
    [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t]
}

/// The closest point of a triangle to `p` (Ericson, Real-Time Collision Detection 5.1.5) and whether it lies on an edge
/// or vertex rather than inside the face.
fn closest_on_triangle(p: V3, t: &[V3; 3]) -> (V3, bool) {
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 { return (a, true); }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 { return (b, true); }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 { let v = d1 / (d1 - d3); return ([a[0] + ab[0] * v, a[1] + ab[1] * v, a[2] + ab[2] * v], true); }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 { return (c, true); }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 { let w = d2 / (d2 - d6); return ([a[0] + ac[0] * w, a[1] + ac[1] * w, a[2] + ac[2] * w], true); }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bc = sub(c, b);
        return ([b[0] + bc[0] * w, b[1] + bc[1] * w, b[2] + bc[2] * w], true);
    }
    let denom = 1.0 / (va + vb + vc);
    let (v, w) = (vb * denom, vc * denom);
    ([a[0] + ab[0] * v + ac[0] * w, a[1] + ab[1] * v + ac[1] * w, a[2] + ab[2] * v + ac[2] * w], false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(tris: Vec<[V3; 3]>) -> CollisionWorld {
        let n = tris.len();
        CollisionWorld { tris, objects: vec![Object { lo: [-10.0; 3], hi: [10.0; 3], tris: 0..n }] }
    }

    #[test]
    fn segments_hit_floors_and_walls() {
        // A floor quad at y = 0 and a wall at z = 5.
        let w = world(vec![
            [[-5.0, 0.0, -5.0], [5.0, 0.0, -5.0], [5.0, 0.0, 5.0]], [[-5.0, 0.0, -5.0], [5.0, 0.0, 5.0], [-5.0, 0.0, 5.0]],
            [[-5.0, 0.0, 5.0], [5.0, 0.0, 5.0], [5.0, 3.0, 5.0]], [[-5.0, 0.0, 5.0], [5.0, 3.0, 5.0], [-5.0, 3.0, 5.0]],
        ]);
        assert_eq!(w.floor_below([1.0, 1.1, 1.0], 3.0), Some(0.0));
        assert!(w.line_of_sight([0.0, 1.0, 0.0], [0.0, 1.0, 4.9]));
        assert!(!w.line_of_sight([0.0, 1.0, 0.0], [0.0, 1.0, 6.0]));
        assert!(w.line_of_sight([0.0, 4.0, 0.0], [0.0, 4.0, 6.0]), "over the wall");
    }
}
