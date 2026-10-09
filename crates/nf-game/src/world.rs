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

/// The moving set: geometry, each object's id and matrices (last tick, this tick), and the matrices by id from the
/// last `set_dynamic` (for the next one's "last tick").
#[derive(Default)]
struct Moving {
    geo: Geo,
    movers: Vec<(u64, Mat4, Mat4)>,
    last: std::collections::HashMap<u64, Mat4>,
}

type Mat4 = nf_formats::anim::Mat4;

/// How the ground under a point moved over the last tick (`Control_InheritVelocity` 0x133e88): the translation, the
/// change of heading (atan2 of the matrix's +Z x / z) and the object's position (the pivot of that turn).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundMotion { pub delta: V3, pub turn: f32, pub pivot: V3 }

#[derive(Default)]
struct Geo {
    tris: Vec<[V3; 3]>,
    objects: Vec<Object>,
}

impl Geo {
    fn push(&mut self, tris: impl IntoIterator<Item = [V3; 3]>) {
        let start = self.tris.len();
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for w in tris {
            for p in &w { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } }
            self.tris.push(w);
        }
        if self.tris.len() > start { self.objects.push(Object { lo, hi, tris: start..self.tris.len() }); }
    }
}

/// Static geometry, plus moving objects (script animations' discrete parts: docs/systems/script-player.md) placed each
/// tick with `set_dynamic` — `Collide_PickObj` (0x1eae80) tests any object linked into a cel with collision data
/// (`obj+0xD8`) whatever moves it.
pub struct CollisionWorld {
    stat: Geo,
    dynamic: std::sync::RwLock<Moving>,
    /// Collision triangles (object space) of every hashed glist object, for `set_dynamic`.
    shapes: std::collections::HashMap<u32, Vec<[V3; 3]>>,
}

/// First hit of `from → to` in one geometry set (fraction).
fn segment_in(g: &Geo, from: V3, to: V3) -> Option<f32> {
    let d = sub(to, from);
    let mut best = None::<f32>;
    for k in 0..3 { if from[k].min(to[k]).is_nan() { return None; } }
    for o in &g.objects {
        if (0..3).any(|k| o.hi[k] < from[k].min(to[k]) || o.lo[k] > from[k].max(to[k])) { continue; }
        for t in &g.tris[o.tris.clone()] {
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

fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }

impl CollisionWorld {
    /// From the level's instances (as drawn: cel placements; skipped-cel blockers included — they block).
    pub fn build(level: &Level, instances: &[Instance]) -> CollisionWorld {
        let mut stat = Geo::default();
        for inst in instances {
            let Some(c) = &level.maps[inst.map].objects[inst.object].collision else { continue };
            stat.push(c.triangles().into_iter().map(|t| t.map(|p| transform(&inst.matrix, p))));
        }
        let shapes = level.by_hash.iter()
            .filter_map(|(&h, &(m, o))| level.maps[m].objects[o].collision.as_ref().map(|c| (h, c.triangles()))).collect();
        CollisionWorld { stat, dynamic: Default::default(), shapes }
    }

    pub fn triangle_count(&self) -> usize { self.stat.tris.len() }

    /// No geometry (tests).
    pub fn empty() -> CollisionWorld { Self::from_triangles(Vec::new()) }

    /// One object holding these world-space triangles (tests).
    pub fn from_triangles(tris: Vec<[V3; 3]>) -> CollisionWorld {
        let mut stat = Geo::default();
        stat.push(tris);
        CollisionWorld { stat, dynamic: Default::default(), shapes: Default::default() }
    }

    /// Glist object `hash`'s collision triangles in object space.
    pub fn shape(&self, hash: u32) -> Option<&[[V3; 3]]> { self.shapes.get(&hash).map(Vec::as_slice) }

    /// Whether glist object `hash` has collision geometry.
    pub fn has_shape(&self, hash: u32) -> bool { self.shapes.contains_key(&hash) }

    /// Replaces the moving objects: each `(id, glist hash, row-vector world matrix)`; the id (stable from tick to
    /// tick) pairs an object with its matrix of the previous call.
    pub fn set_dynamic(&self, parts: &[(u64, u32, Mat4)]) {
        let mut d = self.dynamic.write().unwrap();
        let (mut geo, mut movers, mut last) = (Geo::default(), Vec::new(), std::collections::HashMap::new());
        for &(id, h, m) in parts {
            last.insert(id, m);
            let Some(t) = self.shapes.get(&h) else { continue };
            let n = geo.objects.len();
            geo.push(t.iter().map(|t| t.map(|p| transform(&m, p))));
            if geo.objects.len() > n { movers.push((id, d.last.get(&id).copied().unwrap_or(m), m)); }
        }
        *d = Moving { geo, movers, last };
    }

    /// When the first collision surface along `from → to` belongs to a moving object, how that object moved over the
    /// last tick (`Player_FeetOnPoint` 0x194188 → `Control_InheritVelocity`).
    pub fn ground_motion(&self, from: V3, to: V3) -> Option<GroundMotion> {
        let stat = self.stat_segment(from, to);
        let d = self.dynamic.read().unwrap();
        let mut best: Option<(f32, usize)> = None;
        for (i, o) in d.geo.objects.iter().enumerate() {
            let g = Geo { tris: d.geo.tris[o.tris.clone()].to_vec(), objects: vec![Object { lo: o.lo, hi: o.hi, tris: 0..o.tris.len() }] };
            if let Some(t) = segment_in(&g, from, to) { if best.is_none_or(|b| t < b.0) { best = Some((t, i)); } }
        }
        let (t, i) = best?;
        if stat.is_some_and(|s| s < t) { return None; }
        let (_, prev, cur) = d.movers[i];
        let heading = |m: &Mat4| m[2][0].atan2(m[2][2]);
        let mut turn = heading(&cur) - heading(&prev);
        if turn > std::f32::consts::PI { turn -= std::f32::consts::TAU; } else if turn < -std::f32::consts::PI { turn += std::f32::consts::TAU; }
        Some(GroundMotion { delta: [cur[3][0] - prev[3][0], cur[3][1] - prev[3][1], cur[3][2] - prev[3][2]], turn, pivot: [cur[3][0], cur[3][1], cur[3][2]] })
    }

    fn stat_segment(&self, from: V3, to: V3) -> Option<f32> { segment_in(&self.stat, from, to) }

    /// Every triangle of the objects whose box meets `lo`..`hi`, static first.
    fn near(&self, lo: V3, hi: V3, mut f: impl FnMut(&[V3; 3])) {
        let dynamic = self.dynamic.read().unwrap();
        for g in [&self.stat, &dynamic.geo] {
            for o in &g.objects {
                if (0..3).any(|k| o.hi[k] < lo[k] || o.lo[k] > hi[k]) { continue; }
                for t in &g.tris[o.tris.clone()] { f(t); }
            }
        }
    }

    /// First hit along the segment `from → to` as a fraction in [0, 1] (Möller–Trumbore, both faces).
    pub fn segment(&self, from: V3, to: V3) -> Option<f32> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<f32>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        self.near(lo, hi, |t| {
            let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
            let p = cross(d, e2);
            let det = dot(e1, p);
            if det.abs() < 1e-12 { return; }
            let inv = 1.0 / det;
            let s = sub(from, t[0]);
            let u = dot(s, p) * inv;
            if !(0.0..=1.0).contains(&u) { return; }
            let q = cross(s, e1);
            let v = dot(d, q) * inv;
            if v < 0.0 || u + v > 1.0 { return; }
            let tt = dot(e2, q) * inv;
            if (0.0..=1.0).contains(&tt) && best.is_none_or(|b| tt < b) { best = Some(tt); }
        });
        best
    }

    /// First hit along `from → to`: the fraction in [0, 1] and the unit normal of the triangle hit (facing `from`).
    pub fn segment_hit(&self, from: V3, to: V3) -> Option<(f32, V3)> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<(f32, V3)>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        self.near(lo, hi, |t| {
            let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
            let p = cross(d, e2);
            let det = dot(e1, p);
            if det.abs() < 1e-12 { return; }
            let inv = 1.0 / det;
            let s = sub(from, t[0]);
            let u = dot(s, p) * inv;
            if !(0.0..=1.0).contains(&u) { return; }
            let q = cross(s, e1);
            let v = dot(d, q) * inv;
            if v < 0.0 || u + v > 1.0 { return; }
            let tt = dot(e2, q) * inv;
            if (0.0..=1.0).contains(&tt) && best.is_none_or(|b| tt < b.0) {
                let n = cross(e1, e2);
                let l = dot(n, n).sqrt().max(1e-12);
                let n = if dot(n, d) > 0.0 { [-n[0] / l, -n[1] / l, -n[2] / l] } else { [n[0] / l, n[1] / l, n[2] / l] };
                best = Some((tt, n));
            }
        });
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
        self.near(lo, hi, |t| {
                let tl = [0, 1, 2].map(|k| t[0][k].min(t[1][k]).min(t[2][k]));
                let th = [0, 1, 2].map(|k| t[0][k].max(t[1][k]).max(t[2][k]));
                let (sl, sh) = ([0, 1, 2].map(|k| a[k].min(b[k]) - grow), [0, 1, 2].map(|k| a[k].max(b[k]) + grow));
                if (0..3).any(|k| th[k] < sl[k] || tl[k] > sh[k]) { return; }
                let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
                let nl = dot(n, n).sqrt();
                if nl < 1e-12 { return; }
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
                if dist >= r { return; }
                // The side the capsule is on: along the separation, or (crossing the face) the normal toward the top.
                let dir = if dist > 1e-6 { [v[0] / dist, v[1] / dist, v[2] / dist] } else if dot(n, sub(a, q)) >= 0.0 { n } else { [-n[0], -n[1], -n[2]] };
                let push = [dir[0] * (r - dist), dir[1] * (r - dist), dir[2] * (r - dist)];
                for k in 0..3 { a[k] += push[k]; b[k] += push[k]; total[k] += push[k]; }
                if n[1].abs() > 0.7 && q[1] < b[1] { walkable = true; }
        });
        (total, walkable)
    }

    /// Clear line between two points (`Collide_LineOfSight` stand-in: collision geometry, moving objects included).
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
        let stat = Geo { tris, objects: vec![Object { lo: [-10.0; 3], hi: [10.0; 3], tris: 0..n }] };
        CollisionWorld { stat, dynamic: Default::default(), shapes: Default::default() }
    }

    #[test]
    fn moving_objects_collide_where_they_are() {
        // One unit triangle in object space as glist 7, placed at x = 5 by the matrix.
        let mut w = world(Vec::new());
        w.shapes.insert(7, vec![[[0.0, 0.0, -1.0], [0.0, 1.0, 1.0], [0.0, -1.0, 1.0]]]);
        let at = |x: f32| [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [x, 0.0, 0.0, 1.0]];
        w.set_dynamic(&[(1, 7, at(5.0))]);
        assert!((w.segment([0.0, 0.0, 0.3], [10.0, 0.0, 0.3]).unwrap() - 0.5).abs() < 1e-5);
        // The ground moved 0.5 along x since the last call.
        w.set_dynamic(&[(1, 7, at(5.5))]);
        let g = w.ground_motion([0.0, 0.0, 0.3], [10.0, 0.0, 0.3]).unwrap();
        assert!((g.delta[0] - 0.5).abs() < 1e-6 && g.turn.abs() < 1e-6, "{g:?}");
        // Sister: moved away, the same segment is clear; an unknown glist adds nothing.
        w.set_dynamic(&[(1, 7, at(50.0)), (2, 8, at(5.0))]);
        assert!(w.segment([0.0, 0.0, 0.3], [10.0, 0.0, 0.3]).is_none());
        assert!(w.ground_motion([0.0, 0.0, 0.3], [10.0, 0.0, 0.3]).is_none());
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
