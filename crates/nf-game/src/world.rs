//! Static collision world: every placed object's collision geometry (map block 0x2E, docs/formats/collision.md)
//! transformed to world space. Used for line of sight and floors. The game tests rays against each cel's box tree
//! (`Intersect_RayGeom`); here each object keeps its triangles behind one AABB — same answers, simpler culling.

use nf_formats::level::{Instance, Level};
use nf_formats::placement::transform;
use std::collections::HashMap;

pub type V3 = [f32; 3];

#[derive(Clone)]
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
    /// Per triangle: the owning glist node's bullet-pass flag — the placement's glist flags bit
    /// 0x40 (`celglist+0x44` → `cel+0x94` [M parsemap_block_map_data_static 0x1d1998:103]).
    /// `Collide_Intersect` 0x1ebaa8:143-145 ORs it into `HITDATA+0x51` bit 1 on every hit of the
    /// node, and the bullet-family queries (HITTEST+0x98 bit 0x8 — the aim trace 0x20a/0x208,
    /// `Bullet_homing`'s LOS 8) drop such hits outright (disasm 0x1ebff0-0x1ec010), so bullets fly
    /// through the whole node. fix-foliage-bullets.
    pass: Vec<bool>,
    objects: Vec<Object>,
    /// Static only (`chunk`): bounds over runs of consecutive `objects` (`tris` = an object index range), so a query
    /// skips whole runs. Empty = test every object.
    groups: Vec<Object>,
    /// Static only (`chunk`): for each chunked run, the pushed object (instance) it came from — the
    /// cel-scope bucketing key (fix-safe08b). One entry per `objects` entry after `chunk`.
    owners: Vec<u32>,
}

impl Geo {
    fn push(&mut self, tris: impl IntoIterator<Item = ([V3; 3], u8)>) { self.push_flags(tris, false) }

    fn push_flags(&mut self, tris: impl IntoIterator<Item = ([V3; 3], u8)>, node_pass: bool) {
        let start = self.tris.len();
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for (w, m) in tris {
            for p in &w { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } }
            self.tris.push(w);
            self.mats.push(m);
            self.pass.push(node_pass);
        }
        if self.tris.len() > start { self.objects.push(Object { lo, hi, tris: start..self.tris.len() }); }
    }

    /// Whether a hit on triangle `k` carries the game's bullet-pass bit (`HITDATA+0x51` bit 1):
    /// the node's glist flags bit 0x40 OR the surface byte's bit 0x40 — `Collide_Intersect`
    /// 0x1ebaa8 sets the flag from either (the node word at :143-145, the surface byte at
    /// disasm 0x1ebf78-0x1ebf80, no query-flag gate on either). The surface byte's bit 0x80 does
    /// NOT set it for the bullet queries: that OR is guarded by HITTEST+0x98 bit 0x1
    /// (disasm 0x1ebf8c-0x1ebf94), which the 0x20a/0x208 bullet flags clear — their 50/50 stays
    /// in the walk's roll (`Collide_Update` 0x1ea398:217-220, `Bullet_CollisionHandler` 0x12cfd0).
    /// fix-foliage-bullets.
    fn bullet_pass(&self, k: usize) -> bool { self.pass[k] || self.mats[k] & 0x40 != 0 }

    /// Split every object into runs of `n` consecutive triangles with their own bounds. Static geometry only: its
    /// objects carry no identity, so the queries (`near`, `segment_in`) return exactly the same hits, but a
    /// segment no longer tests every triangle of a large room mesh whose overall box it touches. Mesh triangles
    /// are stored in strip order, so consecutive runs are spatially compact. (07000007: the per-drone `drawn`
    /// ray cost 12 of a 15 ms tick before this.)
    fn chunk(&mut self, n: usize) {
        let mut out = Vec::with_capacity(self.tris.len() / n + self.objects.len());
        let mut owners = Vec::with_capacity(self.tris.len() / n + self.objects.len());
        for (si, o) in self.objects.iter().enumerate() {
            let mut s = o.tris.start;
            while s < o.tris.end {
                let e = (s + n).min(o.tris.end);
                let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
                for t in &self.tris[s..e] { for p in t { for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); } } }
                out.push(Object { lo, hi, tris: s..e });
                owners.push(si as u32);
                s = e;
            }
        }
        self.objects = out;
        self.owners = owners;
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

/// One block-0x21 portal as the two triangles `Intersect_Portal` (0x1ea7b8) tests (its `Index_189`
/// indexing: corners 0,1,2 then 0,2,3), with the two cells it joins.
#[derive(Clone)]
pub struct ScopePortal {
    pub tris: [[V3; 3]; 2],
    pub a: usize,
    pub b: usize,
    /// The portal's reference point: the 4 corners' average, stored at portal+0x70 by
    /// `build_alloc_portal` 0x1c8928 (`puVar2[0x1c..0x1e] = (c0+c1+c2+c3)·0.25`) — the point
    /// `Intersect_Portal` 0x1ea7b8's 0x800 path projects the query segment against (fix-08-collision2;
    /// the port used corner 0, which missed crossings on long doorways).
    pub centre: V3,
}

/// The cel graph the game scopes collision queries with: one bbox per runtime cel (the parse-wide
/// pCelList records) and the portals between them. Plain data — `nf-view` maps its parsed graph in
/// at `CollisionWorld::build_scoped`.
#[derive(Clone, Default)]
pub struct ScopeGraph {
    pub cells: Vec<([f32; 3], [f32; 3])>,
    pub portals: Vec<ScopePortal>,
    /// The runtime cel's room-cel bit (`cel+0x3c & 0x40000`): `build_FindCel` 0x1c8cb0's candidate
    /// walk collects ONLY room cels (both the `glb_world` `gbl_room_cels` branch and the per-world
    /// chain branch filter the flag), so the relink's FindCel fallback must too. fix-swim-entry.
    pub room: Vec<bool>,
}

/// The per-cell static buckets + the graph, built once (`build_scoped`).
struct ScopeData {
    graph: ScopeGraph,
    /// `stat.objects` (chunked-run) indices per cell.
    cells: Vec<Vec<u32>>,
    /// Runs whose instance mapped to no cell: tested by every scoped query (the port's safety net —
    /// the game leaves such objects unlinked and untested, but a mapping miss here must not eat a wall).
    always: Vec<u32>,
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
    /// The cel scope (`build_scoped`): when set, the actor queries (`capsule_push`, the floor ray,
    /// the take ray) test only the cels the game's straddle walk keeps — `Collide_StraddleCels`
    /// 0x1eac98 via `Intersect_Portal` 0x1ea7b8, cap 0x7e cels, the start cel always kept.
    scope: Option<ScopeData>,
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

/// The segment's AABB.
fn box_of(from: V3, to: V3) -> (V3, V3) {
    ([from[0].min(to[0]), from[1].min(to[1]), from[2].min(to[2])], [from[0].max(to[0]), from[1].max(to[1]), from[2].max(to[2])])
}

/// Whether the segment `from → to` crosses the triangle `t` (Möller–Trumbore, t in [0, 1]).
fn seg_tri_hit(from: V3, to: V3, t: &[V3; 3]) -> bool {
    let d = sub(to, from);
    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 { return false; }
    let inv = 1.0 / det;
    let s = sub(from, t[0]);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) { return false; }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < 0.0 || u + v > 1.0 { return false; }
    let tt = dot(e2, q) * inv;
    (0.0..=1.0).contains(&tt)
}

impl CollisionWorld {
    /// From the level's instances (as drawn: cel placements; skipped-cel blockers included — they block).
    pub fn build(level: &Level, instances: &[Instance]) -> CollisionWorld {
        Self::build_scoped(level, instances, None)
    }

    /// `build` with the cel scope. Each instance's runs join **its own runtime cel's bucket**: the
    /// instances are the cel placements in parse order (`Level::instances(true)`) and the scope
    /// graph's cells are `parse_cel_graph`'s one-cell-per-placement list in the same order, so the
    /// k-th pushed instance IS the k-th cell — `parsemap_block_map_data_static` 0x1d1998 creates
    /// one runtime cel per placement whose glist IS the placement's object, and the game's
    /// per-hit cel word (`HITDATA+0x54`, the input of `Collide_SphereIntersect` 0x1ec940's
    /// candidate-scoped queries and of the chain-owner pick `build_link_objects_to_rooms`
    /// 0x1c9150 makes at load) carries that cel. (The old smallest-volume-containing-bbox rule
    /// put a room's floor in whatever tiny generic bbox held the floor's centre — the 07000006
    /// guard-room floor under the table's bbox, the courtyard floor under the fountain's — so
    /// every candidate room's bucket read empty at `build_chains` time and the generics were
    /// left unchained: the fountain, the guard-room furniture and the lockers never drew.)
    /// fix-missing-objects. `attribute_scope` re-keys the generics' runs to their chain owners
    /// after `build_chains` (the game's final attribution); instances whose own cell is out of
    /// range stay in the `always` bucket.
    pub fn build_scoped(level: &Level, instances: &[Instance], graph: Option<ScopeGraph>) -> CollisionWorld {
        let mut stat = Geo::default();
        let mut missile = Geo::default();
        // Per pushed object: its own runtime cel (the pushed ordinal — see the doc above).
        let mut obj_cell: Vec<Option<u32>> = Vec::new();
        for inst in instances {
            let Some(c) = &level.maps[inst.map].objects[inst.object].collision else { continue };
            // The node's bullet-pass flag: the placement's glist flags bit 0x40 (cel+0x94 [M
            // 0x1d1998:103]; Collide_Intersect 0x1ebaa8:143-145 turns every hit on the node into a
            // pass, and the bit-0x8 bullet queries drop those hits — bullets fly through the node).
            let node_pass = level.maps[inst.map].objects[inst.object].flags & 0x40 != 0;
            // The 0x1000-flagged objects (the player-missile cage) never block the actor capsule [M
            // Collide_Pick 0x1eb320 — the cel flags 0x1000 skip for queries with HITTEST+0x98 bit 0x20].
            if level.maps[inst.map].objects[inst.object].flags & 0x1000 != 0 {
                missile.push_flags(c.triangles_with_material().into_iter().map(|(t, m)| (t.map(|p| transform(&inst.matrix, p)), m)), node_pass);
                continue;
            }
            if graph.is_some() {
                obj_cell.push(Some(obj_cell.len() as u32));
            }
            stat.push_flags(c.triangles_with_material().into_iter().map(|(t, m)| (t.map(|p| transform(&inst.matrix, p)), m)), node_pass);
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
        // The per-cell buckets over the chunked runs (fix-safe08b). Runs of an instance that mapped to
        // no cell (or built without a graph) test in every scoped query.
        let scope = graph.map(|graph| {
            let mut cells = vec![Vec::new(); graph.cells.len()];
            let mut always = Vec::new();
            for (run, src) in stat.owners.iter().enumerate() {
                match obj_cell.get(*src as usize).copied().flatten() {
                    Some(c) if (c as usize) < cells.len() => cells[c as usize].push(run as u32),
                    _ => always.push(run as u32),
                }
            }
            ScopeData { graph, cells, always }
        });
        CollisionWorld { stat, missile, dynamic: Default::default(), shapes, shape_mats, scope }
    }

    /// Re-keys the cel-scope buckets by an explicit per-instance attribution — the collision side
    /// of the game's per-cel attribution (`build_link_objects_to_rooms` 0x1c9150 hangs every
    /// generic's collision on the room whose chain holds it; the rooms' own geometry stays with
    /// the room). `instance_cell[i]` = the cell for the i-th pushed object (the `stat.owners`
    /// source index — instances that pushed static geometry, in push order); `None` leaves the
    /// runs in the `always` bucket (the actor queries' safety net). fix-portal-eyecel: the
    /// `build_FindCel` 0x1c8cb0 tie-break reads these buckets per candidate, so the buckets must
    /// carry the game's chain owners, not the smallest-volume centre rule `build_scoped` stands
    /// in with.
    pub fn attribute_scope(&mut self, instance_cell: &[Option<u32>]) {
        let Some(sc) = &mut self.scope else { return };
        let mut cells = vec![Vec::new(); sc.graph.cells.len()];
        let mut always = Vec::new();
        for (run, src) in self.stat.owners.iter().enumerate() {
            match instance_cell.get(*src as usize).copied().flatten() {
                Some(c) if (c as usize) < cells.len() => cells[c as usize].push(run as u32),
                _ => always.push(run as u32),
            }
        }
        sc.cells = cells;
        sc.always = always;
    }

    /// Whether the world carries a cel scope (`build_scoped` with a graph).
    pub fn has_scope(&self) -> bool { self.scope.is_some() }

    /// Debug introspection (fix-08-collision): for every cell's scope bucket plus the `always`
    /// remainder, how many chunked runs whose bounds meet `lo..hi` it holds. Probes the per-cel
    /// attribution without reaching into the private `ScopeData`.
    pub fn debug_bucket_owners(&self, lo: V3, hi: V3) -> Vec<(i32, usize)> {
        let meets = |r: u32| {
            let o = &self.stat.objects[r as usize];
            !(0..3).any(|k| o.hi[k] < lo[k] || o.lo[k] > hi[k])
        };
        let Some(sc) = &self.scope else { return Vec::new() };
        let mut out: Vec<(i32, usize)> = sc.cells.iter().enumerate()
            .map(|(c, runs)| (c as i32, runs.iter().copied().filter(|&r| meets(r)).count()))
            .collect();
        out.push((-1, sc.always.iter().copied().filter(|&r| meets(r)).count()));
        out.retain(|(_, n)| *n > 0);
        out
    }

    /// Debug introspection (fix-08-collision2): the straddle set a capsule query at `a..b` with
    /// radius `r` keeps from `cell` — the `Collide_StraddleCels` result the scoped passes test —
    /// so a missing portal crossing is probeable from a test.
    pub fn debug_straddle(&self, cell: usize, a: V3, b: V3, r: f32) -> Vec<usize> {
        let mut hit = Self::capsule_portal_hit(a, b, r);
        self.straddle(cell, &mut hit)
    }

    pub fn triangle_count(&self) -> usize { self.stat.tris.len() }

    /// Every static triangle whose object's box meets `lo`..`hi`, in stored winding order (the [D]
    /// evidence path: the game's plane is the stored normal = +cross(v1−v0, v2−v0), so the winding's
    /// direction IS the game's facing — `Intersect_RayGeom` 0x10a8c8 skips `n·dir >= 0`).
    pub fn tris_near(&self, lo: V3, hi: V3) -> Vec<[[f32; 3]; 3]> {
        let mut out = Vec::new();
        for o in overlapping(&self.stat, lo, hi) {
            out.extend(self.stat.tris[o.tris.clone()].iter().copied());
        }
        out
    }

    /// No geometry (tests).
    pub fn empty() -> CollisionWorld { Self::from_triangles(Vec::new()) }

    /// One object holding these world-space triangles (tests).
    pub fn from_triangles(tris: Vec<[V3; 3]>) -> CollisionWorld {
        let mut stat = Geo::default();
        stat.push(tris.into_iter().map(|t| (t, 0)));
        CollisionWorld { stat, missile: Geo::default(), dynamic: Default::default(), shapes: Default::default(), shape_mats: Default::default(), scope: None }
    }

    /// One object holding these world-space triangles with surface bytes and per-triangle
    /// bullet-pass flags (tests; the live path derives the flag from the placement's glist flags).
    pub fn from_triangles_flags(tris: Vec<([V3; 3], u8, bool)>) -> CollisionWorld {
        let mut stat = Geo::default();
        let pass = tris.iter().map(|(_, _, p)| *p).collect::<Vec<_>>();
        stat.push_flags(tris.into_iter().map(|(t, m, _)| (t, m)), false);
        for (k, p) in pass.into_iter().enumerate() { stat.pass[k] = p; }
        CollisionWorld { stat, missile: Geo::default(), dynamic: Default::default(), shapes: Default::default(), shape_mats: Default::default(), scope: None }
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
            let g = Geo { tris: d.geo.tris[o.tris.clone()].to_vec(), mats: d.geo.mats[o.tris.clone()].to_vec(), objects: vec![Object { lo: o.lo, hi: o.hi, tris: 0..o.tris.len() }], groups: Vec::new(), owners: Vec::new(),
                pass: d.geo.pass[o.tris.clone()].to_vec(), };
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
        self.near_seg_drop_pass(from, to, false, f)
    }

    /// `near_seg` with the bullet-family's pass-flag drop (HITTEST+0x98 bit 0x8 [M 0x1ebff0]).
    fn near_seg_drop_pass(&self, from: V3, to: V3, drop_pass: bool, mut f: impl FnMut(&[V3; 3], u8)) {
        let dynamic = self.dynamic.read().unwrap();
        for g in [&self.stat, &self.missile, &dynamic.geo] {
            for o in along(g, from, to) {
                for k in o.tris.clone() {
                    if drop_pass && g.bullet_pass(k) { continue; }
                    f(&g.tris[k], g.mats[k]);
                }
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

    /// `Collide_StraddleCels` (0x1eac98): the cells the query keeps — the start cell plus, BFS, every
    /// cell across a portal the query shape straddles (`Intersect_Portal` 0x1ea7b8), cap 0x7e cells,
    /// each cell entered once. `portal_hit` = the shape test for the query's kind.
    fn straddle(&self, start: usize, portal_hit: &mut dyn FnMut(&ScopePortal) -> bool) -> Vec<usize> {
        let Some(sc) = &self.scope else { return Vec::new() };
        let mut seen = vec![false; sc.graph.cells.len()];
        let mut out = Vec::new();
        let start = if start < seen.len() { start } else { return out };
        seen[start] = true;
        out.push(start);
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        while let Some(c) = queue.pop_front() {
            if out.len() >= 0x7e { break; }
            for p in &sc.graph.portals {
                let dest = if p.a == c { Some(p.b) } else if p.b == c { Some(p.a) } else { None };
                let Some(dest) = dest else { continue };
                if dest >= seen.len() || seen[dest] { continue; }
                if !portal_hit(p) { continue; }
                seen[dest] = true;
                out.push(dest);
                queue.push_back(dest);
                if out.len() >= 0x7e { break; }
            }
        }
        out
    }

    /// The `stat.objects` runs the scoped query tests: the straddle cells' buckets plus the `always`
    /// remainder (`None` when there is no scope — the caller falls back to the flat query).
    fn scoped_runs(&self, cell: usize, portal_hit: &mut dyn FnMut(&ScopePortal) -> bool) -> Option<Vec<u32>> {
        let sc = self.scope.as_ref()?;
        let mut taken = vec![false; self.stat.objects.len()];
        let mut out = Vec::new();
        for c in self.straddle(cell, portal_hit) {
            for &r in &sc.cells[c] { if !taken[r as usize] { taken[r as usize] = true; out.push(r); } }
        }
        for &r in &sc.always { if !taken[r as usize] { taken[r as usize] = true; out.push(r); } }
        Some(out)
    }

    /// The actor-capsule portal test (`Intersect_Portal`'s 0x800 path [M 0x1ea7b8]: the point on the
    /// capsule's segment closest to the portal's +0x70 reference point, then that point's distance to
    /// the portal's two triangles against radius × 1.5). The reference point is the portal's CENTRE —
    /// the 4 corners' average `build_alloc_portal` 0x1c8928 stores at +0x70 — not a corner
    /// (fix-08-collision2: corner 0 sat a doorway's height away from the capsule's band and missed the
    /// crossing, dropping the neighbouring cel's walls out of the straddle set — the 01 escort wedge).
    fn capsule_portal_hit(a: V3, b: V3, r: f32) -> impl FnMut(&ScopePortal) -> bool {
        move |p: &ScopePortal| {
            let mut hit = false;
            for t in &p.tris {
                // The closest point on the capsule's segment (HITTEST+0x20..+0x30) to portal+0x70
                // [M 0x1ea7b8's 0x800 branch: the clamped projection against the stored centre].
                let q = closest_on_segment(a, b, p.centre);
                let (cp, _) = closest_on_triangle(q, t);
                let d = sub(q, cp);
                if dot(d, d) < (r * 1.5) * (r * 1.5) { hit = true; break; }
            }
            hit
        }
    }

    /// The ray portal test (`Intersect_Portal`'s 0x201 path [M 0x1ea7b8]: `Collide_RayTriangle` over
    /// the portal's two triangles — a hit on either straddles).
    fn ray_portal_hit(from: V3, to: V3) -> impl FnMut(&ScopePortal) -> bool {
        move |p: &ScopePortal| p.tris.iter().any(|t| seg_tri_hit(from, to, t))
    }

    /// The actor capsule against the straddle cells' static geometry (`capsule_push` over the cel
    /// scope; falls back to the flat world when there is no scope or the query's cell is unmapped).
    pub fn capsule_push_scoped(&self, cell: Option<usize>, a: V3, b: V3, r: f32) -> (V3, bool) {
        let Some(cell) = cell else { return self.capsule_push(a, b, r) };
        let Some(runs) = self.scoped_runs(cell, &mut Self::capsule_portal_hit(a, b, r)) else { return self.capsule_push(a, b, r) };
        let dynamic = self.dynamic.read().unwrap();
        let (mut a, mut b) = (a, b);
        let mut total = [0f32; 3];
        let mut walkable = false;
        let grow = r + 0.1;
        let mut lo = [0f32; 3];
        let mut hi = [0f32; 3];
        for k in 0..3 { lo[k] = a[k].min(b[k]) - grow; hi[k] = a[k].max(b[k]) + grow; }
        let stat = &self.stat;
        let mut test = |t: &[V3; 3], mat: u8, a: &mut V3, b: &mut V3, total: &mut V3, walkable: &mut bool| {
                let tl = [0, 1, 2].map(|k| t[0][k].min(t[1][k]).min(t[2][k]));
                let th = [0, 1, 2].map(|k| t[0][k].max(t[1][k]).max(t[2][k]));
                let (sl, sh) = ([0, 1, 2].map(|k| a[k].min(b[k]) - grow), [0, 1, 2].map(|k| a[k].max(b[k]) + grow));
                if (0..3).any(|k| th[k] < sl[k] || tl[k] > sh[k]) { return; }
                // Water surfaces never collide the player (the cel scoping — see capsule_push).
                if mat & 0x3f == 0xE { return; }
                let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
                let nl = dot(n, n).sqrt();
                if nl < 1e-12 { return; }
                let n = [n[0] / nl, n[1] / nl, n[2] / nl];
                // The one-sided rule for near-horizontal faces: the capsule's top end must be on the
                // front (+n) side, else the face is skipped (the game's head-on cull [M 0x109da0:321-322]).
                if n[1].abs() > 0.7 && dot(n, sub(*a, t[0])) < 0.0 { return; }
                let ab = sub(*b, *a);
                let denom = dot(n, ab);
                let mut p = if denom.abs() > 0.0002 {
                    let tt = (-(dot(n, sub(*a, t[0]))) / denom).clamp(0.0, 1.0);
                    [a[0] + ab[0] * tt, a[1] + ab[1] * tt, a[2] + ab[2] * tt]
                } else {
                    let c = [0, 1, 2].map(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.0);
                    closest_on_segment(*a, *b, c)
                };
                let (q, on_edge) = closest_on_triangle(p, t);
                if on_edge { p = closest_on_segment(*a, *b, q); }
                let v = sub(p, q);
                let dist = dot(v, v).sqrt();
                if dist >= r { return; }
                let dir = if dist > 1e-6 { [v[0] / dist, v[1] / dist, v[2] / dist] } else if dot(n, sub(*a, q)) >= 0.0 { n } else { [-n[0], -n[1], -n[2]] };
                let push = [dir[0] * (r - dist), dir[1] * (r - dist), dir[2] * (r - dist)];
                for k in 0..3 { a[k] += push[k]; b[k] += push[k]; total[k] += push[k]; }
                if n[1].abs() > 0.7 && q[1] < b[1] { *walkable = true; }
        };
        for r in runs {
            let o = &stat.objects[r as usize];
            if !(0..3).any(|k| o.hi[k] < lo[k] || o.lo[k] > hi[k]) {
                for k in o.tris.clone() { test(&stat.tris[k], stat.mats[k], &mut a, &mut b, &mut total, &mut walkable); }
            }
        }
        for g in [&dynamic.geo] {
            for o in overlapping(g, lo, hi) {
                for k in o.tris.clone() { test(&g.tris[k], g.mats[k], &mut a, &mut b, &mut total, &mut walkable); }
            }
        }
        (total, walkable)
    }

    /// The floor ray against the straddle cells' static geometry (the walk's `segment_hit`).
    pub fn segment_hit_scoped(&self, cell: Option<usize>, from: V3, to: V3) -> Option<(f32, V3)> {
        let Some(cell) = cell else { return self.segment_hit(from, to) };
        let Some(runs) = self.scoped_runs(cell, &mut Self::ray_portal_hit(from, to)) else { return self.segment_hit(from, to) };
        let d = sub(to, from);
        let mut best = None::<(f32, V3)>;
        let stat = &self.stat;
        for r in runs {
            let o = &stat.objects[r as usize];
            for k in o.tris.clone() {
                let t = &stat.tris[k];
                let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                if dot(cross(e1, e2), d) >= 0.0 { continue; } // back face [M 0x10a8c8:168]
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
                    best = Some((tt, [n[0] / l, n[1] / l, n[2] / l]));
                }
            }
        }
        let dynamic = self.dynamic.read().unwrap();
        let (blo, bhi) = box_of(from, to);
        for o in overlapping(&dynamic.geo, blo, bhi) {
            for k in o.tris.clone() {
                let t = &dynamic.geo.tris[k];
                let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                if dot(cross(e1, e2), d) >= 0.0 { continue; } // back face [M 0x10a8c8:168]
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
                    best = Some((tt, [n[0] / l, n[1] / l, n[2] / l]));
                }
            }
        }
        best
    }

    /// The first static hit of `from → to` in `cell`'s OWN scope bucket — the port's reading of
    /// `Collide_RayIntersect` (0x1ec690)'s owner filter: the query is handed the candidate cel
    /// (`build_FindCel` 0x1c8cb0's tie-break passes each candidate as the cel argument with obj
    /// lists 0 and flags 1 [M disasm 0x1c900c: a3 = 0, stack0 = 0, t2 = 1, t3 = 0x90]), and the
    /// post-pass exact-tests a static hit only when `HITDATA+0x54` (the static-geometry cel
    /// [M docs/fields/hitdata-spawner-ally.md:41, writer `Collide_Pick` 0x1eb320: `sw $s3, 0x54`
    /// = the walked chain node]) equals the passed cel [M disasm 0x1ec784-0x1ec794: hit+0x54 vs
    /// HITTEST+0x80]. So the ray sees the candidate's OWN chain geometry — the straddle collects
    /// neighbour cels' nodes but the filter rejects every one of them. No `always` runs: the game
    /// leaves unlinked objects untested. Static only: the dynamic-object hits carry +0x54 = 0 and
    /// pass the game's filter, but the port's dynamic set is not cel-keyed (Bond's own parts
    /// would tie both candidates) — a documented deviation [S].
    /// `None` = no scope, the cell is unmapped, or nothing in the cel's own set is hit.
    pub fn segment_hit_cell(&self, cell: usize, from: V3, to: V3) -> Option<(f32, V3)> {
        let sc = self.scope.as_ref()?;
        let runs = sc.cells.get(cell)?;
        let d = sub(to, from);
        let mut best = None::<(f32, V3)>;
        let stat = &self.stat;
        for r in runs {
            let o = &stat.objects[*r as usize];
            for k in o.tris.clone() {
                let t = &stat.tris[k];
                let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                if dot(cross(e1, e2), d) >= 0.0 { continue; } // back face [M 0x10a8c8:168]
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
                    best = Some((tt, [n[0] / l, n[1] / l, n[2] / l]));
                }
            }
        }
        best
    }

    /// The closest distance from `p` to `cell`'s OWN scoped static triangles plus the unmapped
    /// `always` runs — `Collide_SphereIntersect` (0x1ec940)'s per-cel closest hit the way the chain
    /// builder `build_link_objects_to_rooms` (0x1c9150) uses it: a sphere at `p` with a containing
    /// radius (4001), the query scoped so hits outside the queried cel read 1e8 while hits whose
    /// cel is 0 (unmapped instances — the port's `always` runs) still intersect. fix-portal5: this
    /// orders the candidate rooms of a generic's chain hang by their nearest collision point.
    /// `None` = no scope or the cell is unmapped.
    pub fn closest_point_scoped(&self, cell: usize, p: V3) -> Option<f32> {
        let sc = self.scope.as_ref()?;
        let runs = sc.cells.get(cell)?;
        let stat = &self.stat;
        let mut best: Option<f32> = None;
        for r in runs.iter().chain(sc.always.iter()) {
            let o = &stat.objects[*r as usize];
            // Bounding reject: the run's box against the best distance so far.
            let slack = best.unwrap_or(f32::MAX);
            if (0..3).any(|k| p[k] < o.lo[k] - slack || p[k] > o.hi[k] + slack) { continue; }
            for k in o.tris.clone() {
                let (cp, _) = closest_on_triangle(p, &stat.tris[k]);
                let d = sub(cp, p);
                let dist = dot(d, d).sqrt();
                if best.is_none_or(|b| dist < b) { best = Some(dist); }
            }
        }
        best
    }

    /// The move segment against one cell's portals — `Cel_ObjectLeftCel` 0x1e0d58 [M]: the segment
    /// `from→to` against each portal quad the cell participates in (the portal chain at `cel+0x28`),
    /// two `Collide_RayTriangle` tests per quad (the 0,1,2 / 0,2,3 split, `Index_189`), a hit within
    /// the segment returns the portal's OTHER cell. The caller relinks to it BEFORE the bbox keep
    /// runs, so a crossing wins even while the new position is still inside the old cell's box.
    pub fn cel_portal_dest(&self, cell: usize, from: V3, to: V3) -> Option<usize> {
        let sc = self.scope.as_ref()?;
        let dir = sub(to, from);
        let hit = |tri: [V3; 3]| -> bool {
            // Möller–Trumbore over the move segment [0, 1], both sides (the game's RayTriangle has
            // no back-face skip here — `Cel_ObjectLeftCel` 0x1e0d58:95-108 tests the quad twice).
            let e1 = sub(tri[1], tri[0]);
            let e2 = sub(tri[2], tri[0]);
            let pv = [dir[1] * e2[2] - dir[2] * e2[1], dir[2] * e2[0] - dir[0] * e2[2], dir[0] * e2[1] - dir[1] * e2[0]];
            let det = dot(e1, pv);
            if det.abs() < 1e-12 { return false; }
            let inv = 1.0 / det;
            let tv = sub(from, tri[0]);
            let u = dot(tv, pv) * inv;
            if !(-1e-9..=1.0 + 1e-9).contains(&u) { return false; }
            let qv = [tv[1] * e1[2] - tv[2] * e1[1], tv[2] * e1[0] - tv[0] * e1[2], tv[0] * e1[1] - tv[1] * e1[0]];
            let v = dot(dir, qv) * inv;
            if !(-1e-9..=1.0 + 1e-9).contains(&v) || u + v > 1.0 + 2e-9 { return false; }
            let t = dot(e2, qv) * inv;
            (0.0..=1.0).contains(&t)
        };
        for p in &sc.graph.portals {
            let dest = if p.a == cell { p.b } else if p.b == cell { p.a } else { continue };
            let c = &p.tris;
            if hit(c[0]) || hit(c[1]) { return Some(dest); }
        }
        None
    }

    /// `build_point_in_bounding_box` 0x1c9500 [M]: the inclusive bbox test over one runtime cel
    /// (`cel+0x40..0x58`). `None` cell → false.
    pub fn cel_contains(&self, cell: Option<usize>, p: V3) -> bool {
        let Some(cell) = cell else { return false };
        self.scope.as_ref().and_then(|sc| sc.graph.cells.get(cell))
            .is_some_and(|(lo, hi)| (0..3).all(|k| lo[k] <= p[k] && p[k] <= hi[k]))
    }

    /// `build_FindCel` 0x1c8cb0 [M] over the scope graph: the containing ROOM cels (`cel+0x3c &
    /// 0x40000`, cap 0x40 — `CelListCount_154`), then the game's tie-break when several contain the
    /// point: per candidate a collision raycast up (length `len`, 256 first, then the best-so-far
    /// distance) and down (length `min(up hit, len)`), both scoped to the CANDIDATE'S OWN geometry
    /// (`Collide_RayIntersect`'s cel argument + owner filter 1 — `segment_hit_cell`); the nearest
    /// hit distance wins (`if hit->0xc < fVar11`), and with no hit at all the FIRST candidate
    /// stands (0x1c8cb0's tail `return CelList_153`). One candidate returns directly [M
    /// `if (CelListCount_154 != 1)` guards the tie-break]; none returns NULL.
    pub fn find_cel(&self, p: V3) -> Option<usize> {
        let sc = self.scope.as_ref()?;
        let mut cands: Vec<usize> = Vec::new();
        for (i, (lo, hi)) in sc.graph.cells.iter().enumerate() {
            if !sc.graph.room.get(i).copied().unwrap_or(false) { continue; }
            if (0..3).all(|k| lo[k] <= p[k] && p[k] <= hi[k]) {
                cands.push(i);
                if cands.len() >= 0x40 { break; }
            }
        }
        if cands.len() <= 1 { return cands.first().copied(); }
        let mut best: Option<(usize, f32)> = None;
        for &i in &cands {
            let len = best.map_or(256.0, |(_, d)| d);
            let up = self.segment_hit_cell(i, p, [p[0], p[1] + len, p[2]]).map(|(t, _)| t * len);
            let dlen = up.map_or(len, |ud| ud.min(len));
            let down = self.segment_hit_cell(i, p, [p[0], p[1] - dlen, p[2]]).map(|(t, _)| t * dlen);
            match (up, down) {
                (None, None) => continue,
                (a, b) => {
                    let d = a.iter().chain(b.iter()).copied().fold(f32::MAX, f32::min);
                    if best.map_or(true, |(_, bd)| d < bd) { best = Some((i, d)); }
                }
            }
        }
        Some(best.map_or(cands[0], |(i, _)| i))
    }

    /// The player object's per-tick cel relink — `control_handle_cel_change` 0x133288 [M] as the
    /// movement handler's moved pass runs it (`Control_BuildWorldSph` then the change): the move
    /// segment `prev→pos` crossed a portal of the current cel → that portal's other cell
    /// (`Cel_ObjectLeftCel` 0x1e0d58); else `pos` still inside the current cel's box → keep
    /// (`build_point_in_bounding_box` 0x1c9500); else `build_LinkToRoom` 0x1c9680 → `build_FindCel`
    /// 0x1c8cb0 AT THE WORLD-SPHERE CENTRE (`obj+0x80`, `Control_BuildWorldSph` 0x133070 — the
    /// model's local sphere centre through the object matrix, `Bond::sphere_centre`) with the
    /// raycast tie-break; a FindCel miss leaves the object unlinked (`build_LinkToRoom` returns 0
    /// without linking — `Player_InWater` 0x195e50 reads `*(obj+0x20) == 0` → no water).
    /// Deviation [S], cited: the game's `build_LinkToRoom` 0x1c9680:41-45 passes obj+0x80..0x8c (the
    /// world-sphere centre, 0.1267 below the player position [L ee_start_05 M_CONTROL+0x80]); the port
    /// passes POS — the caller's `centre` argument — because the port's per-cell bucket attribution
    /// (`build_scoped`) is coarser than the game's per-cel chains, and at the 07000001 seam the
    /// 0.1267-shorter down-ray flips the tie-break to a root whose bucket misses the floor under Bond
    /// (fix-08-collision2's traced 01 pin, kept green). `find_cel` itself is the game's.
    pub fn player_cel_relink(&self, cur: Option<usize>, prev: V3, pos: V3, centre: V3) -> Option<usize> {
        if let Some(cell) = cur {
            if let Some(dest) = self.cel_portal_dest(cell, prev, pos) { return Some(dest); }
            if self.cel_contains(Some(cell), pos) { return Some(cell); }
        }
        self.find_cel(centre)
    }

    /// The take's LOS ray: the straddle cells' static geometry, the 0x1000 missile cage excluded
    /// (`Collide_Pick`'s bit-0x20 skip, flags 0x721 [M 0x1eb480-0x1eb49c + 0x18e148:213-214]) and the
    /// surface classes the take's query-type filter skips — `Collide_Intersect` 0x1ebaa8's tail: the
    /// HITTEST+0x98 bit 0x1 (0x721 carries it) drops every hit whose surface byte's low 6 bits are
    /// 0xd or 0xe (`(byte & 0x3f) - 0xd < 2`).
    pub fn segment_take_scoped(&self, cell: Option<usize>, from: V3, to: V3) -> Option<f32> {
        let Some(cell) = cell else { return self.segment_actor(from, to) };
        let Some(runs) = self.scoped_runs(cell, &mut Self::ray_portal_hit(from, to)) else { return self.segment_actor(from, to) };
        let d = sub(to, from);
        let mut best = None::<f32>;
        let stat = &self.stat;
        for r in runs {
            let o = &stat.objects[r as usize];
            for k in o.tris.clone() {
                let m = stat.mats[k];
                if (m as u32 & 0x3f).wrapping_sub(0xd) < 2 { continue; }
                let t = &stat.tris[k];
                let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                if dot(cross(e1, e2), d) >= 0.0 { continue; } // back face [M 0x10a8c8:168]
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
        let dynamic = self.dynamic.read().unwrap();
        let (blo, bhi) = box_of(from, to);
        for o in overlapping(&dynamic.geo, blo, bhi) {
            for k in o.tris.clone() {
                let t = &dynamic.geo.tris[k];
                let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                if dot(cross(e1, e2), d) >= 0.0 { continue; } // back face [M 0x10a8c8:168]
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

    /// First hit along the segment `from → to` as a fraction in [0, 1] (Möller–Trumbore). One-sided
    /// like every game ray test [M Intersect_RayGeom 0x10a8c8:168: `n·dir >= 0` skips the triangle].
    pub fn segment(&self, from: V3, to: V3) -> Option<f32> {
        self.segment_in_sets(from, to, true)
    }

    /// The bullet-family segment: `segment` MINUS every hit whose hit-data would carry the pass
    /// flag (the glist node's flags bit 0x40 or the surface byte's bit 0x40). The game's bullet
    /// queries run HITTEST+0x98 with bit 0x8 — the aim trace 0x20a/0x208 (`Player_WeaponInitBullet`
    /// 0x1a4bf0:154-159 [M]) and `Bullet_homing`'s LOS 8 (0x12f358:92 [M]) — and
    /// `Collide_Intersect` 0x1ebaa8 drops pass-flagged hits outright for them (fraction 1e8,
    /// disasm 0x1ebff0-0x1ec010), so the trace's first report is the first NON-pass surface and
    /// bullets fly through the flagged node (the 07000005 trees, glist flags 0x41 [D]).
    /// fix-foliage-bullets.
    pub fn segment_bullet(&self, from: V3, to: V3) -> Option<f32> {
        self.segment_in_sets_drop_pass(from, to, true, true)
    }

    /// The actor-class segment: static MINUS the 0x1000 missile cage, plus the dynamic set — the
    /// game's queries whose HITTEST+0x98 carries bit 0x20 skip the 0x1000-flagged nodes [M
    /// `Collide_Pick` 0x1eb320 disasm 0x1eb480-0x1eb49c, fix-objects2], and the take test's
    /// `Collide_LineOfSight` runs with flags 0x721 (bit 0x20 set) [M 0x18e148:214]. The flat
    /// `segment` (bullets/sight) still hits the cage. fix-safe08: the 07000008 safe take's LOS ray
    /// crossed the cage's hidden wall beside the safe and refused every take.
    pub fn segment_actor(&self, from: V3, to: V3) -> Option<f32> {
        self.segment_in_sets(from, to, false)
    }

    fn segment_in_sets(&self, from: V3, to: V3, with_missile: bool) -> Option<f32> {
        self.segment_in_sets_drop_pass(from, to, with_missile, false)
    }

    fn segment_in_sets_drop_pass(&self, from: V3, to: V3, with_missile: bool, drop_pass: bool) -> Option<f32> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<f32>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        let _ = (lo, hi);
        let dynamic = self.dynamic.read().unwrap();
        let sets: Vec<&Geo> = if with_missile { vec![&self.stat, &self.missile, &dynamic.geo] } else { vec![&self.stat, &dynamic.geo] };
        for g in sets {
            for o in along(g, from, to) {
                for k in o.tris.clone() {
                    if drop_pass && g.bullet_pass(k) { continue; } // the pass-flagged hits the bit-0x8 queries drop [M 0x1ebff0]
                    let t = &g.tris[k];
                    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                    if dot(cross(e1, e2), d) >= 0.0 { continue; } // back face [M 0x10a8c8:168]
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
        }
        best
    }

    /// First hit along `from → to`: the fraction in [0, 1] and the unit normal of the triangle hit (facing `from`).
    /// The player's GROUNDING variant: water surfaces (material 0xE) never ground him — the cel scoping
    /// of `Player_FeetOnPoint` 0x194188 (the ray tests only the current cel `obj+0x20`; the water planes
    /// live in the water cels, whose bbox tops sit at the surface below the walking root) [M] — see
    /// `capsule_push` for the full note. The flat `segment_hit` keeps water for bullets/LOS/AI.
    pub fn segment_hit(&self, from: V3, to: V3) -> Option<(f32, V3)> {
        self.segment_surface(from, to, true).map(|h| (h.0, h.1))
    }

    /// The static-only first hit along `from → to` (fix-portal-sweep): `segment_hit` minus the
    /// missile cage and the dynamic objects. The near-LOS rescue's ray must not be blocked by the
    /// player's own collision or the script-animated parts standing next to him — the game's
    /// `Vision_InView` (0x1e89e0) consults no collision at all (a pure frustum test); this ray is
    /// the port-side unocclusion guard only, so it reads the static world like the level's own
    /// glists do.
    pub fn segment_hit_static(&self, from: V3, to: V3) -> Option<(f32, V3)> {
        let d = sub(to, from);
        let mut best: Option<(f32, V3)> = None;
        for o in along(&self.stat, from, to) {
            for k in o.tris.clone() {
                let t = &self.stat.tris[k];
                let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                if dot(cross(e1, e2), d) >= 0.0 { continue; } // back face [M 0x10a8c8:168]
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
                    best = Some((tt, [n[0] / l, n[1] / l, n[2] / l]));
                }
            }
        }
        best
    }

    /// `segment_hit` with the surface byte of the triangle hit (`Intersect_RayGeom` → `HITDATA+0x50`).
    /// One-sided [M Intersect_RayGeom 0x10a8c8:150-168]: the plane is the triangle's STORED normal (=
    /// +cross(v1−v0, v2−v0), docs/formats/collision.md whole-disc check) and a triangle whose
    /// `n·dir >= 0` is skipped — a ray only hits the front face. `no_water` additionally drops the
    /// water-surface material (0xE) triangles for the player's grounding rays (the cel scoping — see
    /// `segment_hit`).
    pub fn segment_surface(&self, from: V3, to: V3, no_water: bool) -> Option<(f32, V3, u8)> {
        self.segment_surface_impl(from, to, no_water, false)
    }

    /// The bullet-family `segment_surface`: pass-flagged triangles (the glist node's bit 0x40 or
    /// the surface byte's bit 0x40) are dropped the way `Collide_Intersect` 0x1ebaa8 drops them
    /// for the bit-0x8 queries (fraction 1e8, disasm 0x1ebff0-0x1ec010) — the walk's first hit is
    /// the first non-pass surface. Used by the homing LOS (`Bullet_homing` 0x12f358:92, flags 8).
    pub fn segment_surface_bullet(&self, from: V3, to: V3) -> Option<(f32, V3, u8)> {
        self.segment_surface_impl(from, to, false, true)
    }

    /// The flight-trace hit WITH the hit's node pass flag (`HITDATA+0x51` bit 1's node source):
    /// the flight trace keeps pass-flagged hits in its list (`Collide_Update` 0x1ea398 walks them,
    /// `Bullet_CollisionHandler` 0x12ce80 sets state 1 past them), so the caller's walk needs to
    /// know which hits to carry the round past. fix-foliage-bullets.
    pub fn segment_surface_flags(&self, from: V3, to: V3, no_water: bool) -> Option<(f32, V3, u8, bool)> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<(f32, V3, u8, bool)>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        let _ = (lo, hi);
        let dynamic = self.dynamic.read().unwrap();
        for g in [&self.stat, &self.missile, &dynamic.geo] {
            for o in along(g, from, to) {
                for k in o.tris.clone() {
                    let node_pass = g.pass[k];
                    let mat = g.mats[k];
                    if no_water && mat & 0x3f == 0xE { continue; }
                    let t = &g.tris[k];
                    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
                    let n = cross(e1, e2);
                    if dot(n, d) >= 0.0 { continue; } // back face [M 0x10a8c8:168]
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
                        let l = dot(n, n).sqrt().max(1e-12);
                        best = Some((tt, [n[0] / l, n[1] / l, n[2] / l], mat, node_pass));
                    }
                }
            }
        }
        best
    }

    fn segment_surface_impl(&self, from: V3, to: V3, no_water: bool, drop_pass: bool) -> Option<(f32, V3, u8)> {
        let d = sub(to, from);
        let (mut best, mut lo, mut hi) = (None::<(f32, V3, u8)>, [0f32; 3], [0f32; 3]);
        for k in 0..3 { lo[k] = from[k].min(to[k]); hi[k] = from[k].max(to[k]); }
        let _ = (lo, hi); self.near_seg_drop_pass(from, to, drop_pass, |t, mat| {
            if no_water && mat & 0x3f == 0xE { return; } // the water planes never ground the player
            let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
            let n = cross(e1, e2);
            if dot(n, d) >= 0.0 { return; } // back face: the game's ray test skips it [M 0x10a8c8:168]
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
                let l = dot(n, n).sqrt().max(1e-12);
                best = Some((tt, [n[0] / l, n[1] / l, n[2] / l], mat));
            }
        });
        best
    }

    /// The bullet-probe sphere against the world (`Bullet_update`'s slow-move hittest form,
    /// version 0x101 [M 0x12e7b0:425-444 → `Collide_SphereIntersect` 0x1ec940 →
    /// `Intersect_PointGeom` 0x1e92d8]): the closest surface within `radius` of `centre`, as
    /// `(closest point on the triangle, stored plane normal, surface byte, distance)`.
    ///
    /// Per triangle (`Intersect_PointGeom` 0x1e92d8:183-249): the signed plane distance `sd` of
    /// the centre; faces in front by more than the radius are skipped (`radius < sd` → next tri);
    /// `Dist2Tri` 0x1dfca8 returns the closest point on the triangle and — for a centre BEHIND
    /// the face — a negative result, which the best gate skips (`fVar24 < fVar25` with the best
    /// initialised to 0.0 [M 0x1e92d8:103-109; the HITTEST flag word +0x9a bit 0x8 that would
    /// re-allow behind-faces is never set by Bullet_update's build — it writes only 0x20 at
    /// 0x12e7b0:391]) — so the probe is ONE-SIDED: it sees faces the centre is in front of or
    /// touching, never a face it has passed. The minimum-distance face wins; the hit point is
    /// Dist2Tri's closest point (HITDATA+0x40) and the normal the triangle's stored plane normal
    /// (HITDATA+0x10) — the same normal `segment_surface` computes.
    pub fn sphere_surface(&self, centre: V3, radius: f32, no_water: bool) -> Option<(V3, V3, u8, f32)> {
        let mut best: Option<(V3, V3, u8, f32)> = None;
        let mut lo = [0f32; 3];
        let mut hi = [0f32; 3];
        for k in 0..3 { lo[k] = centre[k] - radius; hi[k] = centre[k] + radius; }
        self.near(lo, hi, |t, mat| {
            if no_water && mat & 0x3f == 0xE { return; }
            let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
            let n = cross(e1, e2);
            let nl = dot(n, n).sqrt();
            if nl < 1e-12 { return; }
            let n = [n[0] / nl, n[1] / nl, n[2] / nl];
            // Signed plane distance of the centre; one-sided like the game's best gate.
            let sd = dot(n, sub(centre, t[0]));
            if sd > radius || sd < 0.0 { return; }
            let (q, _on_edge) = closest_on_triangle(centre, t);
            let d = sub(centre, q);
            let dist = dot(d, d).sqrt();
            if dist > radius { return; }
            if best.is_none_or(|b| dist < b.3) {
                best = Some((q, n, mat, dist));
            }
        });
        best
    }

    /// The player's capsule against the static geometry (`Intersect_CylGeom` 0x109da0, docs/systems/player.md): the
    /// segment `a`–`b` with radius `r`, triangle by triangle, each push applied before the next triangle. Per triangle:
    /// the segment point where it meets the plane (clamped; the point nearest the centroid when parallel), the closest
    /// triangle point to it, and when that lies on an edge the segment point nearest it; closer than `r` → pushed out
    /// along the separation. Returns the summed push and whether a walkable face (normal y > 0.7) touched below `b`.
    /// One-sided for near-horizontal faces [M Intersect_CylGeom 0x109da0:302-322 + Player_Collision 0x194df0 case 0/6]:
    /// the game's walk spine (spheres at pos+0.55 and pos+0.275−cc, 1.31 long) makes every plane within
    /// ~40° of horizontal a head-on crossing, and those the game culls by side — an up-facing floor is
    /// solid from above while a down-facing one (the garden pond's floor [D probe_vent_swim]) is passed
    /// through, and an up-facing platform is jumped through from below. The port keys the rule on the
    /// face's horizontality (|n·y| > 0.7 ≈ the game's 0.999/1.31 cone) with the capsule's top end `a` as
    /// the side reference — [S] approximation of the cull, walls and steep slopes stay double-sided as in
    /// the game's non-head-on path.
    pub fn capsule_push(&self, a: V3, b: V3, r: f32) -> (V3, bool) {
        let (mut a, mut b) = (a, b);
        let mut total = [0f32; 3];
        let mut walkable = false;
        let grow = r + 0.1;
        let mut lo = [0f32; 3];
        let mut hi = [0f32; 3];
        for k in 0..3 { lo[k] = a[k].min(b[k]) - grow; hi[k] = a[k].max(b[k]) + grow; }
        self.near_actor(lo, hi, |t, mat| {
                let tl = [0, 1, 2].map(|k| t[0][k].min(t[1][k]).min(t[2][k]));
                let th = [0, 1, 2].map(|k| t[0][k].max(t[1][k]).max(t[2][k]));
                let (sl, sh) = ([0, 1, 2].map(|k| a[k].min(b[k]) - grow), [0, 1, 2].map(|k| a[k].max(b[k]) + grow));
                if (0..3).any(|k| th[k] < sl[k] || tl[k] > sh[k]) { return; }
                // Water surfaces never collide the player [M the cel scoping: the player's collision
                // queries test only his current cel — Player_FeetOnPoint 0x194188 passes obj+0x20 and
                // Collide_SphereIntersect 0x1ec940 takes the cel; the water surface/bed planes (surface
                // material 0xE [D probe_vent_swim: every underwater hit on 07000002 is mat 14]) live in
                // the water cels, which the walking player's origin never occupies (his root stands
                // 1.03 above the bed, above the water volume's bbox top cel+0x90) — so the game's Bond
                // falls through every water bed into the swim. The port approximates the cel scoping by
                // dropping mat-&0x3f==0xE triangles from the player's capsule.
                if mat & 0x3f == 0xE { return; }
                let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
                let nl = dot(n, n).sqrt();
                if nl < 1e-12 { return; }
                let n = [n[0] / nl, n[1] / nl, n[2] / nl];
                // The one-sided rule for near-horizontal faces: the capsule's top end must be on the
                // front (+n) side, else the face is skipped (the game's head-on cull [M 0x109da0:321-322]).
                if n[1].abs() > 0.7 && dot(n, sub(a, t[0])) < 0.0 { return; }
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
        let stat = Geo { mats: vec![0; n], tris, objects: vec![Object { lo: [-10.0; 3], hi: [10.0; 3], tris: 0..n }], groups: Vec::new(), owners: Vec::new(),
            pass: vec![false; n] };
        CollisionWorld { stat, missile: Geo::default(), dynamic: Default::default(), shapes: Default::default(), shape_mats: Default::default(), scope: None }
    }

    /// fix-safe08b: the straddle walk. A wall assigned to an exterior cell with NO portal from the
    /// query's cell is never tested by the scoped actor queries (`Collide_StraddleCels` 0x1eac98:
    /// the exterior cel is not portal-visible from the bedroom); the flat queries still hit it; a
    /// portal the capsule straddles brings the wall back; and the take ray skips the surface classes
    /// the 0x721 filter drops (`Collide_Intersect` 0x1ebaa8's tail: bit 0x1 → classes 0xd/0xe).
    #[test]
    fn scoped_actor_queries_keep_only_the_straddle_cels() {
        // A wall quad at x = 2 (bedroom side, 1.5 units from the capsule) — its collision object is
        // assigned to cell 1 (the "exterior"): its world sphere centre (2, 1, 0) lies in cell 1's box.
        // Wound with the stored normal facing −x (west, at the capsule) — the disc's convention for
        // a wall that stops rays/capsules from the west side (0x10a8c8 one-sided).
        let wall = [
            [[2.0, 0.0, -2.0], [2.0, 2.0, 2.0], [2.0, 2.0, -2.0]],
            [[2.0, 0.0, -2.0], [2.0, 0.0, 2.0], [2.0, 2.0, 2.0]],
        ];
        let mut stat = Geo::default();
        for t in wall { stat.push(std::iter::once((t, 0u8))); }
        stat.chunk(16);
        let cell_centre = |lo: V3, hi: V3| [0, 1, 2].map(|k| (lo[k] + hi[k]) / 2.0);
        let mut graph = ScopeGraph {
            cells: vec![([-8.0, -8.0, -8.0], [1.9, 8.0, 8.0]), ([1.95, -8.0, -8.0], [8.0, 8.0, 8.0])],
            portals: Vec::new(),
            room: vec![true, true],
        };
        // The wall's centre (2, 1, 0) → cell 1. Bucket by the same rule build_scoped uses.
        let mut scope_cells = vec![Vec::new(); graph.cells.len()];
        let mut always = Vec::new();
        for (run, _) in stat.owners.iter().enumerate() {
            let lo = stat.objects[run].lo; let hi = stat.objects[run].hi;
            let c = cell_centre(lo, hi);
            match graph.cells.iter().position(|(lo, hi)| (0..3).all(|k| lo[k] <= c[k] && c[k] <= hi[k])) {
                Some(i) => scope_cells[i].push(run as u32),
                None => always.push(run as u32),
            }
        }
        let mk = |graph: ScopeGraph, cells: Vec<Vec<u32>>| CollisionWorld {
            stat: Geo { tris: stat.tris.clone(), mats: stat.mats.clone(), objects: stat.objects.clone(), groups: Vec::new(), owners: Vec::new(), pass: stat.pass.clone() },
            missile: Geo::default(), dynamic: Default::default(), shapes: Default::default(), shape_mats: Default::default(),
            scope: Some(ScopeData { graph, cells, always: always.clone() }),
        };
        let w = mk(graph.clone(), scope_cells.clone());
        // The capsule stands in cell 0, 1 unit west of the wall: the flat world pushes it out; the
        // scoped one does not (cell 1 is portal-unreachable).
        let (flat, _) = w.capsule_push([1.6, 1.0, 0.0], [1.6, 1.2, 0.0], 0.55);
        assert!(flat[0] < -0.1, "flat: pushed west of the wall {flat:?}");
        let (scoped, _) = w.capsule_push_scoped(Some(0), [1.6, 1.0, 0.0], [1.6, 1.2, 0.0], 0.55);
        assert_eq!(scoped, [0.0; 3], "scoped: the portal-unreachable wall is not tested");
        // The floor ray behaves the same way: a ray THROUGH the wall plane.
        assert!(w.segment_hit([1.5, 1.0, 0.0], [2.5, 1.0, 0.0]).is_some(), "flat segment_hit still sees the wall");
        assert!(w.segment_hit_scoped(Some(0), [1.5, 1.0, 0.0], [2.5, 1.0, 0.0]).is_none(), "the scoped floor ray skips the unreachable wall");
        // Sister: with a portal the capsule straddles (a quad across its path), the wall is back.
        graph.portals.push(ScopePortal { tris: [[[1.8, 0.0, -4.0], [1.8, 4.0, -4.0], [1.8, 4.0, 4.0]], [[1.8, 0.0, -4.0], [1.8, 4.0, 4.0], [1.8, 0.0, 4.0]]], a: 0, b: 1, centre: [1.8, 2.0, 0.0] });
        scope_cells[1].push(0);
        let w2 = mk(graph, scope_cells);
        let (scoped2, _) = w2.capsule_push_scoped(Some(0), [1.6, 1.0, 0.0], [1.6, 1.2, 0.0], 0.55);
        assert!(scoped2[0] < -0.1, "with the straddled portal the wall blocks again {scoped2:?}");
        // The take ray's surface filter: a mat-0xd triangle in the START cell is skipped by
        // segment_take_scoped (the 0x721 bit 0x1) while the flat segment_actor still hits it.
        let mut stat2 = Geo::default();
        let tri: [[V3; 3]; 1] = [[[-1.0, 0.0, 2.0], [0.0, 1.5, 2.0], [1.0, 0.0, 2.0]]];
        for t in tri { stat2.push(std::iter::once((t, 0xd))); }
        stat2.chunk(16);
        let w3 = CollisionWorld {
            stat: stat2, missile: Geo::default(), dynamic: Default::default(), shapes: Default::default(), shape_mats: Default::default(),
            scope: Some(ScopeData { graph: ScopeGraph { cells: vec![([-8.0; 3], [8.0; 3])], portals: Vec::new(), room: vec![true] }, cells: vec![vec![0]], always: Vec::new() }),
        };
        assert!(w3.segment_actor([0.0, 0.5, 0.0], [0.0, 0.5, 4.0]).is_some(), "flat: the mat-0xd tri blocks");
        assert!(w3.segment_take_scoped(Some(0), [0.0, 0.5, 0.0], [0.0, 0.5, 4.0]).is_none(), "the take's 0x721 filter skips the mat-0xd tri");
        assert!(w3.segment_take_scoped(None, [0.0, 0.5, 0.0], [0.0, 0.5, 4.0]).is_some(), "unmapped cell: the flat fallback keeps the old behaviour");
    }

    /// fix-foliage-bullets: the bullet-family queries (`segment_bullet`, `segment_surface_bullet`)
    /// drop every hit whose hit-data would carry the pass flag — the glist node's flags bit 0x40
    /// (`cel+0x94` [M parsemap_block_map_data_static 0x1d1998:103]) or the surface byte's bit 0x40
    /// (`Collide_Intersect` 0x1ebaa8 disasm 0x1ebf78-0x1ebf80 sets `HITDATA+0x51` bit 1 from
    /// either; bit 0x8 of the query's flags — the aim trace 0x20a/0x208, `Bullet_homing`'s LOS 8 —
    /// then drops the hit at disasm 0x1ebff0-0x1ec010) — while the flat queries still report them.
    /// The surface byte's bit 0x80 does NOT drop: its trace-side OR is gated on HITTEST+0x98
    /// bit 0x1 (disasm 0x1ebf8c-0x1ebf94), which the bullet flags clear — its 50/50 stays in the
    /// walk's roll (`Collide_Update` 0x1ea398:217-220).
    #[test]
    fn bullet_queries_drop_the_pass_flagged_hits() {
        // Rays fly +x. Three cards before a solid class-10 (0x0a) wall at x = 5:
        // x = 1 a node-flagged leaf card (class 0), x = 2 a mat-0x40 card (no node flag),
        // x = 3 a mat-0x80 card, all facing −x; the wall at x = 5 faces −x.
        let face = |x: f32, m: u8, pass: bool| -> ([[f32; 3]; 3], u8, bool) { ([[x, 0.0, -1.0], [x, 2.0, 1.0], [x, 2.0, -1.0]], m, pass) };
        let w = CollisionWorld::from_triangles_flags(vec![
            face(1.0, 0x00, true),   // the 07000005 tree nodes: glist flags 0x41 [D]
            face(2.0, 0x40, false),  // the leaf cards' material-level pass bit
            face(3.0, 0x80, false),  // the random-pass material: NOT dropped by the trace
            face(5.0, 0x0a, false),  // solid terrain
        ]);
        let from = [0.0f32, 1.0, 0.0];
        let to = [8.0f32, 1.0, 0.0];
        // The flat queries still see everything (sight, AI, the cage rays).
        assert!((w.segment(from, to).unwrap() - 1.0 / 8.0).abs() < 1e-6, "flat: the node-flagged leaf blocks");
        // The bullet trace reports only the mat-0x80 card at x = 3 (3/8 of the ray).
        let tb = w.segment_bullet(from, to).unwrap();
        assert!((tb - 3.0 / 8.0).abs() < 1e-6, "bullet: the node flag and the 0x40 byte drop, the 0x80 card is the first report (got t={tb})");
        let (_, n, m) = w.segment_surface_bullet(from, to).unwrap();
        assert_eq!(m, 0x80, "bullet surface: the first report is the 0x80 card");
        assert!(n[0] < 0.0, "the reported normal faces the ray");
        // Sister: remove the 0x80 card — the bullet trace reports the WALL behind the foliage.
        let w2 = CollisionWorld::from_triangles_flags(vec![face(1.0, 0x00, true), face(2.0, 0x40, false), face(5.0, 0x0a, false)]);
        let t2 = w2.segment_bullet(from, to).unwrap();
        assert!((t2 - 5.0 / 8.0).abs() < 1e-6, "bullet: only the wall remains (got t={t2})");
        let (_, _, m2) = w2.segment_surface_bullet(from, to).unwrap();
        assert_eq!(m2, 0x0a, "the hit surface is the class-10 wall behind the tree");
        // And the flat query still hits the leaf first there (the game's sight keeps the foliage).
        assert!((w2.segment(from, to).unwrap() - 1.0 / 8.0).abs() < 1e-6, "flat: the leaf still blocks sight");
    }

    #[test]
    fn moving_objects_collide_where_they_are() {
        // One unit triangle in object space as glist 7, placed at x = 5 by the matrix (wound so its
        // plane normal faces −x, the way the disc winds a wall that stops +x rays — 0x10a8c8).
        let mut w = world(Vec::new());
        w.shapes.insert(7, vec![[[0.0, 0.0, -1.0], [0.0, -1.0, 1.0], [0.0, 1.0, 1.0]]]);
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
        // Disc winding: the stored plane normal = +cross(v1−v0, v2−v0) faces UP on every real floor
        // (probe on the disc: 7,246 up / 0 down), and `Intersect_RayGeom` 0x10a8c8 skips a triangle
        // when dot(plane_normal, ray_dir) >= 0 — so a down-ray only hits an up-wound floor.
        let w = world(vec![
            [[-5.0, 0.0, -5.0], [5.0, 0.0, 5.0], [5.0, 0.0, -5.0]], [[-5.0, 0.0, -5.0], [-5.0, 0.0, 5.0], [5.0, 0.0, 5.0]],
            [[-5.0, 0.0, 5.0], [5.0, 3.0, 5.0], [5.0, 0.0, 5.0]], [[-5.0, 0.0, 5.0], [-5.0, 3.0, 5.0], [5.0, 3.0, 5.0]],
        ]);
        assert_eq!(w.floor_below([1.0, 1.1, 1.0], 3.0), Some(0.0));
        assert!(w.line_of_sight([0.0, 1.0, 0.0], [0.0, 1.0, 4.9]));
        assert!(!w.line_of_sight([0.0, 1.0, 0.0], [0.0, 1.0, 6.0]));
        assert!(w.line_of_sight([0.0, 4.0, 0.0], [0.0, 4.0, 6.0]), "over the wall");
    }
}
