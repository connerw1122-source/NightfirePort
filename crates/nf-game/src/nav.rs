//! AI network and routes (docs/systems/drone-navigation.md): map block 0x05 paths and boundaries
//! (`AIPath_*`, `AIBounds_*`), move tests (`NDrone2_MoveTest`, `AINetwork_BoundsTest`), node searches
//! (`NodeSearchCel`), A* (`AINetwork_DoAStarPath`) and the link creep that produces a drone's move target
//! (`LinkCreep_*`, `AINetwork_FollowRoute`).
//!
//! *Deviation*: cels (rooms) and portals are not decoded, so the level is one cel; a collision-world line of sight
//! at +1 stands in for the portal connectivity part of `BoundsTest` (`Collide_StraddleCels`).

use crate::world::{CollisionWorld, V3};
use nf_formats::aipath::AiNetwork;

/// `Vec_Dist2D`.
pub fn dist2d(a: V3, b: V3) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
fn dist3(a: V3, b: V3) -> f32 { ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }
fn lerp(a: V3, b: V3, t: f32) -> V3 { [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t] }

#[derive(Clone, Debug)]
pub struct NavNode {
    /// Disc position with y + 0.4 (`AIPath_Parse`).
    pub pos: V3,
    /// MAPNODE +4: 1 wait, 2 door, 4 kick, 8 action anim, 0x20 ally wait, 0x40 fiddle, 0x10000.. Mayhew lines.
    pub flags: u32,
}

#[derive(Clone, Debug)]
pub struct NavLink { pub a: u16, pub b: u16, pub len: f32 }

#[derive(Clone, Debug)]
pub struct NavPath {
    pub name: String,
    pub kind: u32,
    pub nodes: Vec<NavNode>,
    pub links: Vec<NavLink>,
    /// Per node: its links in link order (`AIPath_Prepare`).
    pub adj: Vec<Vec<u16>>,
}

/// A boundary wall (BLINK) between two posts; heights from a ray up (`AIBounds_Link2Cel`).
#[derive(Clone, Debug)]
pub struct Wall { pub a: V3, pub b: V3, pub ha: f32, pub hb: f32, pub flags: u32 }

pub struct Network {
    pub paths: Vec<NavPath>,
    pub walls: Vec<Wall>,
    pub bounds: bool,
    slope_dy: f32,
    max_slope: f32,
}

/// Node flags that make a node "special" (the drone must reach it: `LinkCreep_Calc`).
pub const SPECIAL: u32 = 0xF007F;

impl Network {
    /// `AIPath_Parse` + `AIPath_BindNodes` + `Drone_PostLoad_Init` (door / kick walls, passable walls).
    pub fn build(net: &AiNetwork, world: &CollisionWorld, level: u32) -> Network {
        let mut paths = Vec::new();
        let mut walls = Vec::new();
        let mut bounds = false;
        for p in &net.paths {
            let pos = |n: &nf_formats::aipath::PathNode| n.game_pos();
            if p.is_boundary() {
                bounds = true;
                let posts: Vec<(V3, f32)> = p.nodes.iter().map(|n| {
                    let q = pos(n);
                    let up = [q[0], q[1] + 256.0, q[2]];
                    let h = world.segment(q, up).map(|t| (t * 256.0).max(2.0)).unwrap_or(256.0);
                    (q, h)
                }).collect();
                for l in &p.links {
                    let (Some(a), Some(b)) = (posts.get(l.a as usize), posts.get(l.b as usize)) else { continue };
                    walls.push(Wall { a: a.0, b: b.0, ha: a.1, hb: b.1, flags: 0 });
                }
            } else {
                let nodes: Vec<NavNode> = p.nodes.iter().map(|n| NavNode { pos: pos(n), flags: match n.raw[0] { 0x2468_abce => 0, f => f } }).collect();
                let links = p.links.iter().map(|l| NavLink { a: l.a, b: l.b, len: dist3(nodes[l.a as usize].pos, nodes[l.b as usize].pos) }).collect();
                paths.push(NavPath { name: p.name.clone(), kind: p.kind, adj: p.adjacency(), nodes, links });
            }
        }
        let (slope_dy, max_slope) = if level == 0x0700_0008 { (1.5, 0.7854) } else { (2.0, 0.8727) };
        let mut n = Network { paths, walls, bounds, slope_dy, max_slope };
        n.init_walls();
        n
    }

    /// `NDrone_InitDoorNodes` / `InitKickNodes` (walls crossed by a link between two door / kick nodes → 0x800 /
    /// 0x1000), then `AINetwork_InitPassableBoundries` (walls crossed by any path link → 0x400, with the height
    /// adjustment of `LineIntersectsBoundry(adjust = 1)` as written).
    fn init_walls(&mut self) {
        for p in &self.paths {
            for l in &p.links {
                let (a, b) = (&p.nodes[l.a as usize], &p.nodes[l.b as usize]);
                for (bit, flag) in [(2, 0x800), (4, 0x1000)] {
                    if a.flags & bit != 0 && b.flags & bit != 0 {
                        for w in &mut self.walls { if intersects(w, a.pos, b.pos).is_some() { w.flags |= flag } }
                    }
                }
            }
        }
        for p in &self.paths {
            for l in &p.links {
                let (a, b) = (p.nodes[l.a as usize].pos, p.nodes[l.b as usize].pos);
                for w in &mut self.walls {
                    if w.flags & 0x1800 == 0 && intersect_adjust(w, a, b, true).is_some() { w.flags |= 0x400 }
                }
            }
        }
    }

    /// `GetBoundsPushVectorForSphere(r, pos, mask 0x1C00)` (drone collision): walls without the mask bits whose closest
    /// point Q (on the 3D segment) is within r of the feet, with the feet between Q.y − 0.4 and Q.y + height, push the
    /// feet out horizontally to distance r (the push direction / length is *suggested*: the decomp loses the stores).
    pub fn push_out(&self, feet: V3, r: f32) -> V3 {
        let mut p = feet;
        for w in self.walls.iter().filter(|w| w.flags & 0x1c00 == 0) {
            let q = closest_on_segment(w.a, w.b, p);
            let d2 = (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2);
            let h = w.ha.min(w.hb);
            if d2 > r * r || p[1] > q[1] + h || p[1] < q[1] - 0.4 { continue; }
            let (dx, dz) = (p[0] - q[0], p[2] - q[2]);
            let l = (dx * dx + dz * dz).sqrt();
            if l < 1e-6 { continue; }
            p[0] = q[0] + dx / l * r;
            p[2] = q[2] + dz / l * r;
        }
        p
    }

    /// `NDrone2_MoveTest(A, B, …, near)`: 1 passable, 0 blocked, −1 blocked with a hit (only when `want_hit`).
    pub fn move_test(&self, world: &CollisionWorld, a: V3, b: V3, near: bool, want_hit: bool) -> (i32, Option<V3>) {
        let dy = b[1] - a[1];
        let h2 = (b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2);
        if h2 + dy * dy > 900.0 || dy.abs() > 4.0 { return (0, None); }
        if dy.abs() > self.slope_dy && dy.abs().atan2(h2.sqrt()) > self.max_slope { return (0, None); }
        // Deviation: portal connectivity → static collision line of sight at +1.
        if !world.line_of_sight([a[0], a[1] + 1.0, a[2]], [b[0], b[1] + 1.0, b[2]]) { return (0, None); }
        if !self.bounds { return (1, None); }
        self.bounds_test(a, b, near, want_hit)
    }

    /// `AINetwork_BoundsTest` over every wall (one cel).
    pub fn bounds_test(&self, a: V3, b: V3, near: bool, want_hit: bool) -> (i32, Option<V3>) {
        let mut best: Option<(f32, V3)> = None;
        for w in &self.walls {
            if let Some(i) = intersects(w, a, b) {
                if !want_hit { return (0, None); }
                let d = dist3([i[0], a[1], i[2]], a);
                if best.is_none_or(|(bd, _)| d < bd) { best = Some((d, [i[0], w.a[1], i[2]])) }
                continue;
            }
            if near && !want_hit {
                let close = |p: V3| ((p[0] - a[0]).powi(2) + (p[2] - a[2]).powi(2) < 0.16) || ((p[0] - b[0]).powi(2) + (p[2] - b[2]).powi(2) < 0.16);
                if close(w.a) || close(w.b) { continue; }
                let pa = closest_on_segment(a, b, w.a);
                let pb = closest_on_segment(a, b, w.b);
                // As written: post a compares a squared distance, post b a plain one (both against 0.16).
                let da2 = (pa[0] - w.a[0]).powi(2) + (pa[2] - w.a[2]).powi(2);
                let db = ((pb[0] - w.b[0]).powi(2) + (pb[2] - w.b[2]).powi(2)).sqrt();
                if (da2 < 0.16 && (pa[1] - w.a[1]).abs() < 2.0) || (db < 0.16 && (pb[1] - w.a[1]).abs() < 2.0) { return (0, None); }
            }
        }
        match best { Some((_, h)) => (-1, Some(h)), None => (1, None) }
    }

    /// `AINetwork_BoundsNodeTest(r, A, B)`: posts only.
    pub fn bounds_node_test(&self, r: f32, a: V3, b: V3) -> bool {
        let r2 = r * r;
        for w in &self.walls {
            for n in [w.a, w.b] {
                let near_end = |p: V3| (n[0] - p[0]).powi(2) + (n[1] - p[1]).powi(2) + (n[2] - p[2]).powi(2) < 1.3 * r2;
                let thr = if near_end(a) || near_end(b) { 0.5 * r2 } else { r2 };
                let p = closest_on_segment(a, b, n);
                if (p[0] - n[0]).powi(2) + (p[1] - n[1]).powi(2) + (p[2] - n[2]).powi(2) < thr && (p[1] - n[1]).abs() < 2.0 { return false; }
            }
        }
        true
    }

    /// `AINetwork_NavPathForPosition`: the path (kind & 0xD = 0) with the node nearest to `pos` (2D); nodes within
    /// 4 units in height stand in for "in the drone's cel", else all.
    pub fn nav_path_for(&self, pos: V3) -> Option<usize> {
        let mut best: Option<(bool, f32, usize)> = None;
        for (pi, p) in self.paths.iter().enumerate().filter(|(_, p)| p.kind & 0xd == 0) {
            for n in &p.nodes {
                let key = ((n.pos[1] - pos[1]).abs() < 4.0, dist2d(n.pos, pos), pi);
                if best.is_none_or(|b| (key.0 && !b.0) || (key.0 == b.0 && key.1 < b.1)) { best = Some(key) }
            }
        }
        best.map(|b| b.2)
    }

    /// `NodeSearchCel` candidates: the path's nodes in the box (±√r² or ±15 in x/z, ±2 in y) with dist2D ≤ r²
    /// (the linear distance against r², as written), nearest first; equal distances keep reverse file order.
    fn candidates(&self, path: usize, pos: V3, r2: f32) -> Vec<usize> {
        let half = if r2 > 0.0 { r2.sqrt() } else { 15.0 };
        let p = &self.paths[path];
        let mut c: Vec<(f32, usize)> = (0..p.nodes.len()).rev().filter_map(|i| {
            let n = p.nodes[i].pos;
            let inside = (n[0] - pos[0]).abs() <= half && (n[2] - pos[2]).abs() <= half && (n[1] - pos[1]).abs() <= 2.0;
            let d = dist2d(n, pos);
            (inside && (r2 <= 0.0 || d <= r2)).then_some((d, i))
        }).collect();
        c.sort_by(|a, b| a.0.total_cmp(&b.0));
        c.into_iter().map(|(_, i)| i).collect()
    }

    /// `AINetwork_NodesForPosition` for one path (mode 0): the first candidate with `MoveTestFromNode(node → pos)`
    /// = 1 is found; the first failure is the fallback.
    pub fn target_nodes(&self, world: &CollisionWorld, path: usize, pos: V3, r2: f32) -> TargetNodes {
        let mut t = TargetNodes::default();
        for i in self.candidates(path, pos, r2) {
            if self.move_test(world, self.paths[path].nodes[i].pos, pos, false, false).0 == 1 { t.found = Some(i as u16); break; }
            if t.fallback.is_none() { t.fallback = Some(i as u16) }
        }
        t
    }

    /// `AINetwork_DoAStarPath` from `start` toward `dest` whose goal nodes are `goal`; `used` = link used-counts
    /// this frame. Returns (0 real goal / 1 fallback goal, node list, link list) or the failure code (4, 5, 6).
    pub fn astar(&self, world: &CollisionWorld, path: usize, start: V3, dest: V3, goal: &TargetNodes, used: &[u16]) -> Result<(u8, Vec<u16>, Vec<u16>), u8> {
        let p = &self.paths[path];
        let (goal_node, result) = match (goal.found, goal.fallback) { (Some(g), _) => (g, 0), (None, Some(g)) => (g, 1), _ => return Err(4) };
        const OPEN: u32 = 0x4000_0000;
        let n = p.nodes.len();
        let (mut flags, mut g, mut f) = (vec![0u32; n], vec![0f32; n], vec![0f32; n]);
        let mut parent: Vec<Option<(u16, u16)>> = vec![None; n];
        let mut open: Vec<usize> = Vec::new();
        // Start node: the first candidate (box ±25) reachable from the start.
        let mut started = false;
        for i in self.candidates(path, start, 625.0) {
            match self.move_test(world, start, p.nodes[i].pos, false, false).0 {
                1 => {
                    g[i] = dist2d(start, p.nodes[i].pos);
                    f[i] = g[i] + dist2d(p.nodes[i].pos, dest);
                    flags[i] |= OPEN;
                    open.push(i);
                    started = true;
                    break;
                }
                -1 => break,
                _ => {}
            }
        }
        if !started { return Err(5); }
        while !open.is_empty() {
            let cur = open.remove(0);
            flags[cur] &= !OPEN;
            if cur == goal_node as usize {
                let (mut nodes, mut links) = (vec![cur as u16], Vec::new());
                let mut k = cur;
                while let Some((pn, l)) = parent[k] {
                    links.push(l);
                    nodes.push(pn);
                    k = pn as usize;
                    if nodes.len() > n { break; }
                }
                nodes.reverse();
                links.reverse();
                return Ok((result, nodes, links));
            }
            for &li in &p.adj[cur] {
                let l = &p.links[li as usize];
                let nb = if l.a as usize == cur { l.b } else { l.a } as usize;
                let cost = g[cur] + l.len * (used.get(li as usize).copied().unwrap_or(0) as f32 + 1.0);
                if flags[nb] & 0xC000_0000 != 0 && g[nb] <= cost { continue; }
                parent[nb] = Some((cur as u16, li));
                g[nb] = cost;
                f[nb] = cost + dist2d(p.nodes[nb].pos, dest);
                if flags[nb] & OPEN == 0 {
                    let at = open.iter().position(|&o| f[o] > f[nb]).unwrap_or(open.len());
                    open.insert(at, nb);
                    flags[nb] |= OPEN;
                }
            }
            // As built: the expanded node is marked with the open bit (never re-queued, g/parent still writable).
            flags[cur] |= OPEN;
        }
        Err(6)
    }
}

impl Network {
    /// `AINetwork_NodeSearch` (mode 1, limit 625): the first node (nearest first, 2D) reachable from `pos`.
    pub fn node_from(&self, world: &CollisionWorld, path: usize, pos: V3) -> Option<u16> {
        for i in self.candidates(path, pos, 625.0) {
            match self.move_test(world, pos, self.paths[path].nodes[i].pos, false, false).0 {
                1 => return Some(i as u16),
                -1 => return None,
                _ => {}
            }
        }
        None
    }

    /// `AINetwork_EmitPath(limit 255)` (0x158cf8): path distances from `pos` to every node of `path` (a flood from the
    /// first reachable node, link cost len × (used + 1), the same list quirks as A*), stored as bytes:
    /// trunc(g × 255 / 256) capped at 254, 0xFF = not reached.
    pub fn emit(&self, world: &CollisionWorld, path: usize, pos: V3) -> Vec<u8> {
        let p = &self.paths[path];
        let n = p.nodes.len();
        const OPEN: u32 = 0x4000_0000;
        let (mut flags, mut g) = (vec![0u32; n], vec![255.0f32; n]);
        let Some(start) = self.node_from(world, path, pos) else { return vec![0xff; n] };
        let s = start as usize;
        g[s] = dist2d(pos, p.nodes[s].pos);
        flags[s] |= OPEN;
        let mut open = vec![s];
        while !open.is_empty() {
            let cur = open.remove(0);
            flags[cur] &= !OPEN;
            for &li in &p.adj[cur] {
                let l = &p.links[li as usize];
                let nb = if l.a as usize == cur { l.b } else { l.a } as usize;
                let cost = g[cur] + l.len;
                if cost > 255.0 { continue; }
                if flags[nb] & 0xC000_0000 != 0 && g[nb] <= cost { continue; }
                g[nb] = cost;
                if flags[nb] & OPEN == 0 {
                    let at = open.iter().position(|&o| g[o] > cost).unwrap_or(open.len());
                    open.insert(at, nb);
                    flags[nb] |= OPEN;
                }
            }
            flags[cur] |= OPEN;
        }
        g.iter().map(|&v| if v == 255.0 { 0xff } else { ((v / 256.0).abs() * 255.0).min(254.0) as u8 }).collect()
    }
}

/// Goal nodes of a target in one path (`AITarget` found / fallback flags).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TargetNodes { pub found: Option<u16>, pub fallback: Option<u16> }

fn closest_on_segment(a: V3, b: V3, p: V3) -> V3 {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    if len2 <= 0.0 { return a; }
    let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1] + (p[2] - a[2]) * d[2]) / len2).clamp(0.0, 1.0);
    lerp(a, b, t)
}

/// `LineIntersectsBoundry`: 2D intersection of the segment A→B with the wall; hit when the segment's height there is
/// within [wall − 2.4, wall + height]. With `adjust`, a segment more than 2 above the wall sets both post heights to
/// its height − 0.5 (as written). Returns the intersection point.
fn intersects(w: &Wall, a: V3, b: V3) -> Option<V3> { intersect_adjust(&mut w.clone(), a, b, false) }

impl Network {
    /// `AINetwork_FurthestPosition(A, B)` (0x15a8d8): the intersection of the segment A→B with any boundary wall (no
    /// flag filter) nearest to A, at post a's height.
    pub fn first_wall_hit(&self, a: V3, b: V3) -> Option<V3> {
        let mut best: Option<(f32, V3)> = None;
        for w in &self.walls {
            if let Some(i) = intersects(w, a, b) {
                let d = ((i[0] - a[0]).powi(2) + (i[2] - a[2]).powi(2)).sqrt();
                if best.is_none_or(|(bd, _)| d < bd) { best = Some((d, [i[0], w.a[1], i[2]])) }
            }
        }
        best.map(|b| b.1)
    }
}

fn intersect_adjust(w: &mut Wall, a: V3, b: V3, adjust: bool) -> Option<V3> {
    let (r, s) = ([b[0] - a[0], b[2] - a[2]], [w.b[0] - w.a[0], w.b[2] - w.a[2]]);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-12 { return None; } // parallel: the game returns 1 without a point (U-030); treated as no hit
    let q = [w.a[0] - a[0], w.a[2] - a[2]];
    let t = (q[0] * s[1] - q[1] * s[0]) / den;
    let u = (q[0] * r[1] - q[1] * r[0]) / den;
    if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) { return None; }
    let i = [a[0] + r[0] * t, 0.0, a[2] + r[1] * t];
    let y_line = a[1] + (b[1] - a[1]) * t;
    let y_wall = w.a[1] + (w.b[1] - w.a[1]) * u;
    let h_wall = w.ha + (w.hb - w.ha) * u;
    if adjust && y_line - y_wall > 2.0 { w.ha = y_line - 0.5; w.hb = y_line - 0.5; }
    (y_wall - 2.4 <= y_line && y_line <= y_wall + h_wall).then_some([i[0], y_line, i[2]])
}


// ---- routes ---------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteMode { OneWay, OneWayEnd, Loop }

/// The link creep (`route+0xA0..`): from `a` toward `b` in `steps` steps of `vec`.
#[derive(Clone, Debug, Default)]
pub struct Creep { pub a: V3, pub b: V3, pub vec: V3, pub step: i32, pub steps: i32, pub special: Option<u16> }

impl Creep {
    /// `LinkCreep_ForNodes`: steps of 0.4 (trunc(trunc(|d|) × 2.5)), or one step of d when |d| ≤ 0.4.
    fn new(a: V3, b: V3, special: Option<u16>) -> Creep {
        let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let (vec, steps) = if len > 0.4 { (d.map(|x| x * 0.4 / len), (len.trunc() * 2.5).trunc() as i32) } else { (d, 1) };
        Creep { a, b, vec, step: 0, steps, special }
    }
    fn point(&self, k: i32) -> V3 { [self.a[0] + self.vec[0] * k as f32, self.a[1] + self.vec[1] * k as f32, self.a[2] + self.vec[2] * k as f32] }
}

/// `AIRoute`.
#[derive(Clone, Debug)]
pub struct Route {
    pub path: usize,
    pub mode: RouteMode,
    pub nodes: Vec<u16>,
    /// Links between consecutive nodes (for the used-counts).
    pub links: Vec<u16>,
    pub index: i32,
    pub start: V3,
    pub dest: V3,
    /// Current move target (`route+0x60`, copied to `Drone+0x670`).
    pub target: V3,
    pub remaining: f32,
    pub radius: f32,
    pub lookahead: f32,
    pub creep: Creep,
    /// The player-target stamp the route was computed for (`route+8`).
    pub stamp: u32,
}

/// `AINetwork_FollowRoute` results.
pub mod follow {
    pub const MOVING: u8 = 0;
    pub const DIRECT: u8 = 2;
    pub const ARRIVED: u8 = 3;
    pub const NO_PATH: u8 = 7;
    pub const BLOCKED: u8 = 0xc;
}

impl Route {
    fn new(path: usize, mode: RouteMode, nodes: Vec<u16>, links: Vec<u16>, start: V3, dest: V3) -> Route {
        Route { path, mode, nodes, links, index: 0, start, dest, target: start, remaining: 0.0, radius: 2.0, lookahead: 2.0, creep: Creep::default(), stamp: 0 }
    }

    /// `NDrone2_InitAIPath` from `first`: the node list follows the links (a 2-link node leaves by the link not
    /// leading back; the first node of a kind-4 path without 0x10 leaves by its second link; others re-append
    /// themselves) until it holds the path's node count. Mode 2 for patrol paths, else 1.
    pub fn for_path(net: &Network, path: usize, first: u16) -> Route {
        let p = &net.paths[path];
        let mut nodes = vec![first];
        let mut links = Vec::new();
        let mut prev: Option<u16> = None;
        let mut cur = first;
        while nodes.len() < p.nodes.len() {
            let adj = &p.adj[cur as usize];
            let mut next = None;
            if adj.len() == 2 {
                let pick = if nodes.len() == 1 && p.kind & 4 != 0 && p.kind & 0x10 == 0 { Some(adj[1]) } else {
                    adj.iter().copied().find(|&li| { let l = &p.links[li as usize]; let o = if l.a == cur { l.b } else { l.a }; Some(o) != prev })
                };
                if let Some(li) = pick { let l = &p.links[li as usize]; next = Some((if l.a == cur { l.b } else { l.a }, li)); }
            }
            match next {
                Some((o, li)) => { prev = Some(cur); cur = o; nodes.push(o); links.push(li); }
                None => nodes.push(cur),
            }
        }
        let mode = if p.kind & 4 != 0 { RouteMode::Loop } else { RouteMode::OneWayEnd };
        let first_pos = p.nodes[first as usize].pos;
        let mut r = Route::new(path, mode, nodes, links, first_pos, first_pos);
        // Mode 1's end point (+0x9B0): the last node + 0.5 × the last segment's direction.
        if let [.., x, y] = r.nodes[..] {
            let (a, b) = (p.nodes[x as usize].pos, p.nodes[y as usize].pos);
            let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
            r.dest = [b[0] + 0.5 * d[0] / l, b[1] + 0.5 * d[1] / l, b[2] + 0.5 * d[2] / l];
        }
        r.calc(net, 0);
        r.target = r.creep.a;
        r
    }

    /// A combat route to `dest` (`AINetwork_CalcRoute`): direct when `MoveTest(start, dest, near)` passes, else A* +
    /// `OptimiseRoute`. Err = the failure code (4/5/6/8).
    pub fn to_position(net: &Network, world: &CollisionWorld, path: Option<usize>, start: V3, dest: V3, goal: &TargetNodes, used: &[u16]) -> Result<(u8, Route), u8> {
        if net.move_test(world, start, dest, true, false).0 == 1 {
            let mut r = Route::new(path.unwrap_or(0), RouteMode::OneWay, Vec::new(), Vec::new(), start, dest);
            r.creep = Creep::new(start, dest, None);
            r.target = start;
            return Ok((follow::DIRECT, r));
        }
        let Some(pi) = path.filter(|&p| !net.paths[p].nodes.is_empty()) else { return Err(8) };
        let (res, mut nodes, mut links) = net.astar(world, pi, start, dest, goal, used)?;
        // OptimiseRoute: trim the first / last node when the next one is reachable directly.
        let p = &net.paths[pi];
        let pair_ok = |x: u16, y: u16| { let (fx, fy) = (p.nodes[x as usize].flags, p.nodes[y as usize].flags); !((fx & 2 != 0 && fy & 2 != 0) || (fx & 4 != 0 && fy & 4 != 0)) };
        if nodes.len() > 1 && pair_ok(nodes[0], nodes[1]) && net.move_test(world, start, p.nodes[nodes[1] as usize].pos, true, false).0 == 1 {
            nodes.remove(0);
            links.remove(0);
        }
        let n = nodes.len();
        if n > 1 && pair_ok(nodes[n - 2], nodes[n - 1]) && net.move_test(world, dest, p.nodes[nodes[n - 2] as usize].pos, true, false).0 == 1 {
            nodes.pop();
            links.pop();
        }
        let mut r = Route::new(pi, RouteMode::OneWay, nodes, links, start, dest);
        r.calc(net, 0);
        r.target = r.creep.a;
        Ok((res, r))
    }

    fn node_pos(&self, net: &Network, k: usize) -> V3 { net.paths[self.path].nodes[self.nodes[k] as usize].pos }

    /// `LinkCreep_Calc(index)`.
    fn calc(&mut self, net: &Network, i: i32) {
        let count = self.nodes.len() as i32;
        if count == 0 { self.creep = Creep::new(self.start, self.dest, None); return; }
        let j = match self.mode { RouteMode::Loop => (i + 1).rem_euclid(count), _ => (i + 1).min(count - 1) };
        self.creep = if i < 0 {
            Creep::new(self.start, self.node_pos(net, j as usize), None)
        } else if i == j || i >= count {
            Creep::new(self.node_pos(net, i.min(count - 1) as usize), self.dest, None)
        } else {
            let node = self.nodes[i as usize];
            let special = (net.paths[self.path].nodes[node as usize].flags & SPECIAL != 0).then_some(node);
            Creep::new(self.node_pos(net, i as usize), self.node_pos(net, j as usize), special)
        };
    }

    /// `AINetwork_SetupNextNode(±1)`.
    fn next_node(&mut self, net: &Network, dir: i32) {
        let count = self.nodes.len() as i32;
        self.index += dir;
        match self.mode {
            RouteMode::Loop => { if self.index >= count { self.index = 0 } if self.index < 0 { self.index = count - 1 } }
            _ => {
                if self.index >= count {
                    self.index = count;
                    self.creep = Creep::new(self.target, self.dest, None);
                    return;
                }
                if self.index < -1 { self.index = -1 }
            }
        }
        self.calc(net, self.index);
        self.target = self.creep.a;
    }

    /// `DroneMove_DistanceToNextRouteNode` and `DroneMove_NextRouteAngle`: the next node's 2D distance and the turn
    /// there (between feet → node and node → the one after, or the dest); `None` past the last node.
    pub fn next_node_turn(&self, net: &Network, feet: V3) -> Option<(f32, f32)> {
        let next = (self.index + 1).max(0) as usize;
        let n = self.node_pos(net, *[next].iter().find(|&&k| k < self.nodes.len())?);
        let after = if next + 1 < self.nodes.len() { self.node_pos(net, next + 1) } else { self.dest };
        let a = crate::trig::atan2_approx(n[0] - feet[0], n[2] - feet[2]);
        let b = crate::trig::atan2_approx(after[0] - n[0], after[2] - n[2]);
        Some((dist2d(feet, n), crate::trig::angle_difference(a, b)))
    }

    /// `GetRouteDistance`.
    pub fn distance(&self, net: &Network, feet: V3) -> f32 {
        let next = (self.index + 1).max(0) as usize;
        if next >= self.nodes.len() { return dist2d(feet, self.dest); }
        let p = &net.paths[self.path];
        let mut d = dist2d(feet, self.node_pos(net, next));
        for &l in self.links.iter().skip(next) { d += p.links[l as usize].len; }
        d + dist2d(self.node_pos(net, self.nodes.len() - 1), self.dest)
    }

    /// `AINetwork_FollowRoute` + `LinkCreep_Handler`: updates the move target. Returns the result code and the node
    /// reached when a special node was reached this tick (`NDrone2_ReachedDestNode`).
    pub fn follow(&mut self, net: &Network, feet: V3) -> (u8, Option<u16>) {
        self.remaining = self.distance(net, feet);
        let count = self.nodes.len() as i32;
        if self.index >= count && self.mode != RouteMode::Loop && self.remaining < self.radius { return (follow::ARRIVED, None); }
        let mut reached = None;
        if let Some(node) = self.creep.special {
            let pos = net.paths[self.path].nodes[node as usize].pos;
            self.target = pos;
            if dist2d(feet, pos) >= 0.4 { return (follow::MOVING, None); }
            self.creep.special = None;
            reached = Some(node);
        }
        // Blocked toward the target: step back (`LinkCreep_Decrement`).
        if net.bounds && (feet[0] - self.target[0]).powi(2) + (feet[1] - self.target[1]).powi(2) + (feet[2] - self.target[2]).powi(2) >= 1.0
            && !net.bounds_node_test(0.2, feet, self.target) {
            if self.creep.step >= 1 && net.bounds_node_test(0.1, feet, self.creep.point(self.creep.step - 1)) {
                self.creep.step -= 1;
                self.target = self.creep.point(self.creep.step);
                return (follow::MOVING, reached);
            }
            return (follow::BLOCKED, reached);
        }
        // `LinkCreep_Increment`.
        if self.creep.step >= self.creep.steps {
            self.next_node(net, 1);
            return (follow::MOVING, reached);
        }
        let q = self.creep.point(self.creep.step + 1);
        let d = dist2d(feet, q);
        if d < self.lookahead && (d < 0.4 || net.bounds_node_test(0.4, feet, q)) {
            self.creep.step += 1;
            self.target = q;
        }
        (follow::MOVING, reached)
    }

    /// `UpdateLinksUsedCount`: every remaining link (from index − 1).
    pub fn mark_used(&self, used: &mut [u16]) {
        let from = (self.index - 1).max(0) as usize;
        for &l in self.links.iter().skip(from) { if let Some(u) = used.get_mut(l as usize) { *u += 1 } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(nodes: &[V3], links: &[(u16, u16)], kind: u32) -> Network {
        let nodes: Vec<NavNode> = nodes.iter().map(|&p| NavNode { pos: p, flags: 0 }).collect();
        let links: Vec<NavLink> = links.iter().map(|&(a, b)| NavLink { a, b, len: dist3(nodes[a as usize].pos, nodes[b as usize].pos) }).collect();
        let mut adj = vec![Vec::new(); nodes.len()];
        for (k, l) in links.iter().enumerate() { adj[l.a as usize].push(k as u16); adj[l.b as usize].push(k as u16); }
        Network { paths: vec![NavPath { name: "t".into(), kind, nodes, links, adj }], walls: Vec::new(), bounds: false, slope_dy: 2.0, max_slope: 0.8727 }
    }

    #[test]
    fn astar_takes_the_short_way_and_used_links_cost_more() {
        // A square 0-1-2-3 with a long detour 0-4-2.
        let n = net(&[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 0.0, 10.0], [0.0, 0.0, 10.0], [-20.0, 0.0, 30.0]], &[(0, 1), (1, 2), (2, 3), (3, 0), (0, 4), (4, 2)], 2);
        let w = CollisionWorld::empty();
        let goal = TargetNodes { found: Some(2), fallback: None };
        let (res, nodes, links) = n.astar(&w, 0, [0.0, 0.0, -1.0], [10.0, 0.0, 11.0], &goal, &[0; 6]).unwrap();
        assert_eq!(res, 0);
        assert_eq!((nodes[0], nodes[2], nodes.len()), (0, 2, 3));
        assert_eq!(links.len(), 2);
        // Busy links 0→1 and 0→3 (used 9 times each) push the route onto the detour.
        let (_, nodes, _) = n.astar(&w, 0, [0.0, 0.0, -1.0], [10.0, 0.0, 11.0], &goal, &[9, 0, 0, 9, 0, 0]).unwrap();
        assert_eq!(nodes, vec![0, 4, 2]);
        // No goal → 4.
        assert_eq!(n.astar(&w, 0, [0.0; 3], [1.0; 3], &TargetNodes::default(), &[0; 6]).unwrap_err(), 4);
    }

    #[test]
    fn walls_block_moves_and_the_creep_walks_a_loop() {
        let mut n = net(&[[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 0.0, 4.0], [0.0, 0.0, 4.0]], &[(0, 1), (1, 2), (2, 3), (3, 0)], 4);
        let w = CollisionWorld::empty();
        assert_eq!(n.move_test(&w, [0.0; 3], [2.0, 0.0, 0.0], false, false).0, 1);
        assert_eq!(n.move_test(&w, [0.0; 3], [2.0, 5.0, 0.0], false, false).0, 0, "|dy| > 4");
        n.bounds = true;
        n.walls.push(Wall { a: [1.0, 0.0, -1.0], b: [1.0, 0.0, 1.0], ha: 2.0, hb: 2.0, flags: 0 });
        assert_eq!(n.move_test(&w, [0.0; 3], [2.0, 0.0, 0.0], false, false).0, 0, "through the wall");
        assert_eq!(n.move_test(&w, [0.0; 3], [2.0, 0.0, 0.0], false, true).0, -1);
        n.walls.clear();
        n.bounds = false;
        // Patrol from node 0: kind 4 → the first node leaves by its second link (3), then around.
        let mut r = Route::for_path(&n, 0, 0);
        assert_eq!(r.nodes, vec![0, 3, 2, 1]);
        assert_eq!(r.mode, RouteMode::Loop);
        // Walk the feet onto the target each tick: the route must come back to node 0.
        let mut feet = [0.0, 0.0, 0.0];
        let mut visited = vec![];
        for _ in 0..400 {
            r.follow(&n, feet);
            feet = r.target;
            if !visited.contains(&r.index) { visited.push(r.index) }
        }
        assert_eq!(visited.len(), 4, "all four links walked: {visited:?}");
    }
}
