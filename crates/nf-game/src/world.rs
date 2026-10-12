//! Static collision world: every placed object's collision geometry (map block 0x2E, docs/formats/collision.md)
//! transformed to world space. Used for line of sight and floors. The game tests rays against each cel's box tree
//! (`Intersect_RayGeom`); here each object keeps its triangles behind one AABB — same answers, simpler culling.

use nf_formats::level::{Instance, Level};
use nf_formats::placement::transform;
use std::collections::HashMap;

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
    /// Each triangle's surface byte (`HITDATA+0x50`; `& 0x3F` indexes `EffectInfo`).
    mats: Vec<u8>,
    objects: Vec<Object>,
    /// Static only (`chunk`): bounds over runs of consecutive `objects` (`tris` = an object index range), so a query
    /// skips whole runs. Empty = test every object.
    groups: Vec<Object>,
}

impl Geo {
    fn push(&mut self, tris: impl IntoIterator<Item = ([V3; 3], u8)>) {
        let start = self.tris.len();
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for (w, m) in tris {
            for p in &w { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } }
            self.tris.push(w);
            self.mats.push(m);
        }
        if self.tris.len() > start { self.objects.push(Object { lo, hi, tris: start..self.tris.len() }); }
    }

    /// Split every object into runs of `n` consecutive triangles with their own bounds. Static geometry only: its
    /// objects carry no identity, so the queries (`near`, `segment_in`) return exactly the same hits, but a
    /// segment no longer tests every triangle of a large room mesh whose overall box it touches. Mesh triangles
    /// are stored in strip order, so consecutive runs are spatially compact. (07000007: the per-drone `drawn`
    /// ray cost 12 of a 15 ms tick before this.)
    fn chunk(&mut self, n: usize) {
        let mut out = Vec::with_capacity(self.tris.len() / n + self.objects.len());
        for o in &self.objects {
            let mut s = o.tris.start;
            while s < o.tris.end {
                let e = (s + n).min(o.tris.end);
                let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
                for t in &self.tris[s..e] { for p in t { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } } }
                out.push(Object { lo, hi, tris: s..e });
                s = e;
            }
        }
        self.objects = out;
        self.groups.clear();
        let mut s = 0;
        while s < self.objects.len() {
            let e = (s + 32).min(self.objects.len());
            let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
            for o in &self.objects[s..e] { for k in 0..3 { lo[k] = lo[k].min(o.lo[k]); hi[k] = hi[k].max(o.hi[k]); } }
            self.groups.push(Object { lo, hi, tris: s..e });
            s = e;
        }
    }
}

/// Static geometry, plus moving objects (script animations' discrete parts: docs/systems/script-player.md) placed each
/// tick with `set_dynamic` — `Collide_PickObj` (0x1eae80) tests any object linked into a cel with collision data
/// (`obj+0xD8`) whatever moves it.
pub struct CollisionWorld {
    stat: Geo,
    /// Objects whose glist flags carry 0x1000 (the "BlockingPoly_PlayerMissile" family): the game's
    /// `Collide_Pick` (0x1eb320 disasm 0x1eb480-0x1eb49c) skips collision nodes whose cel flags word
    /// (cel+0x94 = the glist flags) has bit 0x1000 when the query's HITTEST+0x98 carries bit 0x20 —
    /// the actor-movement queries — while ray/sight queries (flags 0x18 et al) still hit them. On
    /// 07000008 these objects are a bullet cage over the whole map (a lid at y 52.8, a floor at
    /// y −40 and two walls over the station walkway) that must not block Bond on foot [D level data;
    /// the level is playable in the game across that wall]. They stay solid to `segment` (bullets,
    /// sight, the floor rays).
    missile: Geo,
    dynamic: std::sync::RwLock<Moving>,
    /// Collision triangles (object space) of every hashed glist object, for `set_dynamic`.
    shapes: std::collections::HashMap<u32, Vec<[V3; 3]>>,
    shape_mats: std::collections::HashMap<u32, Vec<u8>>,
}

/// Whether the segment `from + t·d`, t in [0, 1], passes through the box (slab test, a little padding).
fn seg_box(from: V3, d: V3, lo: V3, hi: V3) -> bool {
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for k in 0..3 {
        let (l, h) = (lo[k] - 1e-3, hi[k] + 1e-3);
        if d[k].abs() < 1e-12 { if from[k] < l || from[k] > h { return false; } continue; }
        let inv = 1.0 / d[k];
        let (mut a, mut b) = ((l - from[k]) * inv, (h - from[k]) * inv);
        if a > b { std::mem::swap(&mut a, &mut b); }
        t0 = t0.max(a); t1 = t1.min(b);
        if t0 > t1 { return false; }
    }
    true
}

/// The objects the segment `from → to` passes through the bounds of (through `groups` when present).
fn along<'a>(g: &'a Geo, from: V3, to: V3) -> impl Iterator<Item = &'a Object> + 'a {
    let d = sub(to, from);
    let all: Box<dyn Iterator<Item = &'a Object>> = if g.groups.is_empty() { Box::new(g.objects.iter()) } else {
        Box::new(g.groups.iter().filter(move |gr| seg_box(from, d, gr.lo, gr.hi)).flat_map(move |gr| g.objects[gr.tris.clone()].iter()))
    };
    all.filter(move |o| seg_box(from, d, o.lo, o.hi))
}

/// The objects whose bounds overlap `lo..hi` (through `groups` when present).
fn overlapping<'a>(g: &'a Geo, lo: V3, hi: V3) -> impl Iterator<Item = &'a Object> + 'a {
    let out = move |o: &Object| (0..3).any(|k| o.hi[k] < lo[k] || o.lo[k] > hi[k]);
    let all: Box<dyn Iterator<Item = &'a Object>> = if g.groups.is_empty() { Box::new(g.objects.iter()) } else {
        Box::new(g.groups.iter().filter(move |gr| !(0..3).any(|k| gr.hi[k] < lo[k] || gr.lo[k] > hi[k])).flat_map(move |gr| g.objects[gr.tris.clone()].iter()))
    };
    all.filter(move |o| !out(o))
}

/// First hit of `from → to` in one geometry set (fraction).
fn segment_in(g: &Geo, from: V3, to: V3) -> Option<f32> {
    let d = sub(to, from);
    let mut best = None::<f32>;
    for k in 0..3 { if from[k].min(to[k]).is_nan() { return None; } }
    let (lo, hi): (V3, V3) = ([from[0].min(to[0]), from[1].min(to[1]), from[2].min(to[2])], [from[0].max(to[0]), from[1].max(to[1]), from[2].max(to[2])]);
    let _ = (lo, hi);
    for o in along(g, from, to) {
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
        let mut missile = Geo::default();
        for inst in instances {
            let Some(c) = &level.maps[inst.map].objects[inst.object].collision else { continue };
            // The 0x1000-flagged objects (the player-missile cage) never block the actor capsule [M
            // Collide_Pick 0x1eb320 — the cel flags 0x1000 skip for queries with HITTEST+0x98 bit 0x20].
            if level.maps[inst.map].objects[inst.object].flags & 0x1000 != 0 {
                missile.push(c.triangles_with_material().into_iter().map(|(t, m)| (t.map(|p| transform(&inst.matrix, p)), m)));
                continue;
            }
            stat.push(c.triangles_with_material().into_iter().map(|(t, m)| (t.map(|p| transform(&inst.matrix, p)), m)));
        }
        // Index collision shapes by hashcode, AND by (map,object) packed into a synthetic key for objects whose
        // hashcode is u32::MAX (anonymous objects — e.g. 4/5 doors on 07000001 have hashcode=MAX, so the hashcode
        // lookup fails and bullets pass through). The synthetic key is 0x80000000 | (map << 16) | object [S].
        let mut shapes: HashMap<u32, Vec<[V3; 3]>> = HashMap::new();
        let mut shape_mats: HashMap<u32, Vec<u8>> = HashMap::new();
        for (&h, &(m, o)) in &level.by_hash {
            if let Some(c) = level.maps[m].objects[o].collision.as_ref() {
                shapes.insert(h, c.triangles());
                shape_mats.insert(h, c.triangles_with_material().into_iter().map(|t| t.1).collect());
            }
        }
        // Also index by (map, object) for anonymous objects (hashcode == MAX) — doors placed by-index [S].
        for (m, map) in level.maps.iter().enumerate() {
            for (o, obj) in map.objects.iter().enumerate() {
                if obj.hashcode == u32::MAX {
                    if let Some(c) = obj.collision.as_ref() {
                        let key = 0x8000_0000 | ((m as u32) << 16) | o as u32;
                        shapes.insert(key, c.triangles());
                        shape_mats.insert(key, c.triangles_with_material().into_iter().map(|t| t.1).collect());
                    }
                }
            }
        }
        stat.chunk(16);
        missile.chunk(16);
        CollisionWorld { stat, missile, dynamic: Default::default(), shapes, shape_mats }
    }

    pub fn triangle_count(&self) -> usize { self.stat.tris.len() }

    /// No geometry (tests).
    pub fn empty() -> CollisionWorld { Self::from_triangles(Vec::new()) }

    /// One object holding these world-space triangles (tests).
    pub fn from_triangles(tris: Vec<[V3; 3]>) -> CollisionWorld {
        let mut stat = Geo::default();
        stat.push(tris.into_iter().map(|t| (t, 0)));
        CollisionWorld { stat, missile: Geo::default(), dynamic: Default::default(), shapes: Default::default(), shape_mats: Default::default() }
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
            let mats = self.shape_mats.get(&h);
            geo.push(t.iter().enumerate().map(|(k, t)| (t.map(|p| transform(&m, p)), mats.and_then(|v| v.get(k).copied()).unwrap_or(0))));
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
            let g = Geo { tris: d.geo.tris[o.tris.clone()].to_vec(), mats: d.geo.mats[o.tris.clone()].to_vec(), objects: vec![Object { lo: o.lo, hi: o.hi, tris: 0..o.tris.len() }], groups: Vec::new() };
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
    /// Every triangle whose object's bounds the segment `from → to` passes through (`near` for segments).
    fn near_seg(&self, from: V3, to: V3, mut f: impl FnMut(&[V3; 3], u8)) {
        let dynamic = self.dynamic.read().unwrap();
        for g in [&self.stat, &self.missile, &dynamic.geo] {
            for o in along(g, from, to) {
                for k in o.tris.clone() { f(&g.tris[k], g.mats[k]); }
            }
        }
    }

    /// The actor-capsule neighbourhood: static geometry MINUS the 0x1000 missile cage (the game's
    /// actor queries skip those nodes [M Collide_Pick 0x1eb320:0x1eb480-0x1eb49c]), dynamic objects included.
    fn near_actor(&self, lo: V3, hi: V3, mut f: impl FnMut(&[V3; 3], u8)) {
        let dynamic = self.dynamic.read().unwrap();
        for g in [&self.stat, &dynamic.geo] {
            for o in overlapping(g, lo, hi) {
                for k in o.tris.clone() { f(&g.tris[k], g.mats[k]); }
            }
        }
    }

    fn near(&self, lo: V3, hi: V3, mut f: impl FnMut(&[V3; 3], u8)) {
        let dynamic = self.dynamic.read().unwrap();
        for g in [&self.stat, &self.missile, &dynamic.geo] {
            for o in overlapping(g, lo, hi) {
                for k in o.tris.clone() { f(&g.tris[k], g.mats[k]); }
            }
        }
    }

    /// First hit along the segment `from → to` as a fraction in [0, 1] (Möller–Trumbore, both faces).
    pub fn segment(&self, from: V3, to: V3) -> Option<f32> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<f32>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        let _ = (lo, hi); self.near_seg(from, to, |t, _| {
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
    pub fn segment_hit(&self, from: V3, to: V3) -> Option<(f32, V3)> { self.segment_surface(from, to).map(|h| (h.0, h.1)) }

    /// `segment_hit` with the surface byte of the triangle hit (`Intersect_RayGeom` → `HITDATA+0x50`).
    pub fn segment_surface(&self, from: V3, to: V3) -> Option<(f32, V3, u8)> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<(f32, V3, u8)>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        let _ = (lo, hi); self.near_seg(from, to, |t, mat| {
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
                best = Some((tt, n, mat));
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
        self.near_actor(lo, hi, |t, _| {
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
        let stat = Geo { mats: vec![0; n], tris, objects: vec![Object { lo: [-10.0; 3], hi: [10.0; 3], tris: 0..n }], groups: Vec::new() };
        CollisionWorld { stat, missile: Geo::default(), dynamic: Default::default(), shapes: Default::default(), shape_mats: Default::default() }
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
