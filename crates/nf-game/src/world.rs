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

    /// Clear line between two points (`Collide_LineOfSight` stand-in: static geometry only).
    pub fn line_of_sight(&self, a: V3, b: V3) -> bool { self.segment(a, b).is_none() }

    /// Height of the first collision surface straight below `p` within `depth` (starting 0.5 above it).
    pub fn floor_below(&self, p: V3, depth: f32) -> Option<f32> {
        let from = [p[0], p[1] + 0.5, p[2]];
        let to = [p[0], p[1] - depth, p[2]];
        self.segment(from, to).map(|t| from[1] + (to[1] - from[1]) * t)
    }
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
