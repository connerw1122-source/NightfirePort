//! Hurt volumes (placement 0xE4, class 0x27) — `Hurt_Create` 0x17e870 / `Hurt_Update` 0x17e5d0.
//!
//! Spec: docs/systems/hurt-volumes.md (written from the decompile; every field cites it).
//! The volume's geometry is the placement object's collision glist; a body intersecting it takes
//! `Hurt_GetDamage` 0x17ea18 (+0x10 = param 1) through the victim's pain pipeline with
//! `Hurt_GetType` 0x17ea50 (+0x14 = param 0) — the fusebox live-wire hurt is the port's precedent
//! (`drone.rs` `pain_generic`). The state machine (0 armed → 1 animating → 2 retired) drives the
//! activation sound and the one-shot retirement; the damage itself rides the COLLIDE contact and
//! applies every tick a body intersects while the object lives.
//!
//! 07000008's #42–44 are the level's kill floor: three 2-triangle quads at y −24.10 tiling the
//! valley (x −55.8..286.1, z −260.4..−110.9 [D the placement collision]), type 6 (fall), 1000
//! damage, no gate, who 0x18 = classes 3+2 (drones AND the player), one-shot 0 [D params]. LIVE
//! at load: no gate + who ≠ 0 → they start on the first body contact
//! (the who-1 inert law never fires; 0x18 & {8, 0x10, 2} does).

use crate::world::V3;
use nf_formats::level::Level;
use nf_formats::placement::{object_matrix, transform};
use crate::triggers::{Channels, Toucher};

fn point_triangle(p: V3, t: &[V3; 3]) -> f32 {
    // Closest distance from p to triangle t: the plane distance inside, the edge/vertex
    // fallback on the border (the trigger walk's contact shape).
    let (a, b, c) = (t[0], t[1], t[2]);
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let n = cross(ab, ac);
    let nlen = len(n);
    if nlen < 1e-9 { return f32::INFINITY; }
    let dist_n = (dot(ap, n) / nlen).abs();
    if bary_inside(p, t, n) { return dist_n; }
    let mut best = f32::INFINITY;
    for k in 0..3 {
        let (q, r) = (t[k], t[(k + 1) % 3]);
        let e = [r[0] - q[0], r[1] - q[1], r[2] - q[2]];
        let el2 = e[0] * e[0] + e[1] * e[1] + e[2] * e[2];
        if el2 < 1e-12 { continue; }
        let tproj = (((p[0] - q[0]) * e[0] + (p[1] - q[1]) * e[1] + (p[2] - q[2]) * e[2]) / el2).clamp(0.0, 1.0);
        let dx = q[0] + e[0] * tproj - p[0]; let dy = q[1] + e[1] * tproj - p[1]; let dz = q[2] + e[2] * tproj - p[2];
        best = best.min((dx * dx + dy * dy + dz * dz).sqrt());
    }
    best
}

fn bary_inside(p: V3, t: &[V3; 3], n: V3) -> bool {
    let mut signs = [0.0f32; 3];
    for k in 0..3 {
        let (a, b) = (t[k], t[(k + 1) % 3]);
        let e = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
        signs[k] = dot(cross(e, w), n);
    }
    (signs[0] >= 0.0 && signs[1] >= 0.0 && signs[2] >= 0.0) || (signs[0] <= 0.0 && signs[1] <= 0.0 && signs[2] <= 0.0)
}

fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn len(a: V3) -> f32 { dot(a, a).sqrt() }

fn segment_hits_triangle(a: V3, b: V3, t: &[V3; 3]) -> bool {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let (e1, e2) = ([t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]], [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]]);
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 { return false; }
    let s = [a[0] - t[0][0], a[1] - t[0][1], a[2] - t[0][2]];
    let u = dot(s, p) / det;
    if !(0.0..=1.0).contains(&u) { return false; }
    let q = cross(s, e1);
    let v = dot(d, q) / det;
    if v < 0.0 || u + v > 1.0 { return false; }
    (0.0..=1.0).contains(&(dot(e2, q) / det))
}

/// One hurt volume (`Hurt_Create` 0x17e870).
pub struct Hurt {
    pub placement: usize,
    /// +0x10 = param 1 [M Hurt_GetDamage 0x17ea18].
    pub damage: f32,
    /// +0x14 = param 0 [M Hurt_GetType 0x17ea50; 6 = fall in the port's damage table].
    pub dtype: u8,
    /// +0x15 = param 5: 1 → state 2 (retire) after the first pass.
    pub one_shot: bool,
    /// +0x16 = param 4: the AND-mask over the contact class bits (1 always, 2 class 5, 8 class 3,
    /// 0x10 class 2/the player) [M 0x17e5d0 state 0].
    pub who: u16,
    /// +0x18 = param 2: gate channel (0 = none; nonzero → the channel VALUE is the who-mask).
    pub gate: u16,
    /// +0x1a = param 3, 0xffff when 0: the activation sound id [M 0x17e5d0 state 1 at t == 0].
    pub sound: u16,
    pub tris: Vec<[V3; 3]>,
    pub lo: [f32; 3],
    pub hi: [f32; 3],
    /// obj+0xF4: 0 armed, 1 animating, 2 retired [M 0x17e5d0].
    pub state: u8,
    /// +0x00 the anim clock; +0x04 the per-tick increment; +0x0C the key count (the pose spline
    /// itself is not ported: every LIVE row in the survey is an invisible kill floor / steam
    /// volume — the framelist pose has no drawn target here [S unverified]).
    pub t: f32,
    pub inc: f32,
    pub keys: u16,
    /// Set when the volume applies damage (the live test's mechanism witness — the game's
    /// equivalent is the collide contact on obj+0xD0).
    pub hits: u32,
    pub gone: bool,
}

impl Hurt {
    /// `Hurt_Update` 0x17e5d0 state 0's who-mask law. `collected` = the OR of the contact class
    /// bits of the bodies touching THIS volume this tick (class 3 → 8, class 2 → 0x10, class 5 → 2).
    pub fn who_mask(&self, ch: &Channels, collected: u16) -> u16 {
        if self.gate != 0 {
            // Gated: the channel VALUE is the mask [M 0x17e5d0: `uVar5 = switch_channels[gate]`].
            ch.v.get(self.gate as usize).copied().unwrap_or(0) as u16
        } else if self.who == 0 {
            1 // No gate + who 0 → starts at once [M].
        } else {
            self.who & collected // No gate + who ≠ 0 → fires only on the collected class bits [M].
        }
    }

    /// Whether a toucher capsule reaches this volume's triangles (the trigger walk's tests — the
    /// game's contact is the body sphere against the glist bounds [M the collide pass]).
    pub fn touched_by(&self, c: &Toucher) -> bool {
        let grow = c.r + 0.1;
        if (0..3).any(|k| c.a[k].max(c.b[k]) + grow < self.lo[k] || c.a[k].min(c.b[k]) - grow > self.hi[k]) { return false; }
        self.tris.iter().any(|t| {
            segment_hits_triangle(c.a, c.b, t) || point_triangle(c.a, t) <= c.r || point_triangle(c.b, t) <= c.r
        })
    }
}

/// Build from the 0xE4 placements (`Hurt_Create` 0x17e870: params at tag+0x2C+4k, the same layout
/// SoundTrigger uses; the volume = the placement object's collision glist, posed by the placement
/// matrix [M Control_SetGList + build_LinkToRoom]).
pub fn build(level: &Level) -> Vec<Hurt> {
    let Some(m) = level.main_map() else { return Vec::new() };
    let map = &level.maps[m];
    let mut out = Vec::new();
    for (i, p) in map.placements.iter().enumerate() {
        if p.flags & 0xff != 0xe4 { continue; }
        let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
        let mx = object_matrix(p.rotation, p.translation);
        let tris: Vec<[V3; 3]> = target.and_then(|(om, oi)| level.maps[om].objects[oi].collision.as_ref())
            .map(|c| c.triangles().into_iter().map(|t| t.map(|v| transform(&mx, v))).collect()).unwrap_or_default();
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for t in &tris { for v in t { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } } }
        out.push(Hurt {
            placement: i,
            damage: param(1) as f32,
            dtype: param(0) as u8,
            one_shot: param(5) == 1,
            who: param(4) as u16,
            gate: param(2) as u16,
            sound: if param(3) == 0 { 0xffff } else { param(3) as u16 },
            tris, lo, hi,
            state: 0,
            t: 0.0,
            inc: 0.0,
            keys: 0,
            hits: 0,
            gone: false,
        });
    }
    out
}

/// The per-tick update: the contact scan + state machine [M 0x17e5d0] plus the damage pass.
/// Returns the (volume index, toucher index, damage, type) applications — the caller feeds them
/// to the victims' pain pipelines (Bond: `Drones::health.pain_generic`; a drone: its own).
pub fn update(hurts: &mut [Hurt], ch: &Channels, touchers: &[Toucher]) -> Vec<(usize, usize, f32, u8)> {
    let mut apps = Vec::new();
    for (hix, h) in hurts.iter_mut().enumerate() {
        if h.gone || h.state == 2 { continue; }
        let mut collected: u16 = 0;
        let mut touched: Vec<usize> = Vec::new();
        for (ti, c) in touchers.iter().enumerate() {
            if h.touched_by(c) {
                touched.push(ti);
                collected |= if c.drone { 8 } else { 0x10 }; // class 3 → 8, class 2 → 0x10 [M].
            }
        }
        if h.state == 0 && h.who_mask(ch, collected) != 0 {
            // The anim starts [M 0x17e5d0: `uVar5 != 0` → state = 1]; the activation sound fires
            // on the animating path's t == 0 beat [M state 1] — the port's audio hook rides the
            // map-sound pass (sound 0xffff = none on every LIVE survey row).
            h.state = 1;
        }
        if h.state == 1 {
            h.t += h.inc;
            if h.keys != 0 && h.t > h.keys as f32 {
                h.t = 0.0;
                if h.one_shot { h.state = 2; }
            }
        }
        if h.state == 2 { continue; }
        for ti in touched {
            h.hits += 1;
            apps.push((hix, ti, h.damage, h.dtype));
        }
    }
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(y: f32) -> Hurt {
        let (a, b, c, d) = ([-1.0, y, -1.0], [1.0, y, -1.0], [1.0, y, 1.0], [-1.0, y, 1.0]);
        Hurt { placement: 0, damage: 1000.0, dtype: 6, one_shot: false, who: 0x18, gate: 0, sound: 0xffff,
            tris: vec![[a, b, c], [a, c, d]], lo: [-1.0, y, -1.0], hi: [1.0, y, 1.0],
            state: 0, t: 0.0, inc: 0.0, keys: 0, hits: 0, gone: false }
    }

    /// The 07000008 row law [D placements #42-44; M 0x17e5d0]: type 6, 1000 damage, no gate,
    /// who 0x18 (drones AND the player), one-shot 0.
    #[test]
    fn the_08_row_law() {
        let h = quad(-24.1);
        assert_eq!((h.dtype, h.damage as u32, h.gate, h.who, h.one_shot), (6, 1000, 0, 0x18, false));
        let ch = Channels::default();
        assert_eq!(h.who_mask(&ch, 0), 0, "nothing touching → still armed");
        assert_eq!(h.who_mask(&ch, 0x10), 0x10, "the player's class bit");
        assert_eq!(h.who_mask(&ch, 8), 8, "a drone's class bit");
        assert_eq!(h.who_mask(&ch, 2), 0, "class 5's bit 2 is not in the who mask");
        // The who-1 inert law (07000006's #373) [M 1 & collected == 0 always — collected ⊆ {8,0x10,2}]:
        let mut inert = quad(0.0); inert.who = 1;
        assert_eq!(inert.who_mask(&ch, 0x10 | 8 | 2), 0);
    }

    /// A falling body takes the damage the tick it touches; a standing body above the quad does
    /// not [M the collide contact law]. A drone toucher fires it too (who bit 8) [M 0x18 = 3+2].
    #[test]
    fn the_fall_touches_and_the_stand_does_not() {
        let mut h = quad(-24.1);
        let ch = Channels::default();
        let above = [Toucher::player([0.0, 20.0, 0.0])];
        assert!(update(std::slice::from_mut(&mut h), &ch, &above).is_empty());
        assert_eq!(h.state, 0);
        let falling = [Toucher::player([0.0, -24.9, 0.0])]; // capsule bottom −24.35, within 0.55 of the quad
        let apps = update(std::slice::from_mut(&mut h), &ch, &falling);
        assert_eq!(apps.len(), 1);
        assert_eq!((apps[0].2 as u32, apps[0].3), (1000, 6));
        assert_eq!(h.state, 1);
        assert_eq!(h.hits, 1);
        let mut d = quad(-24.1);
        let drone = [Toucher { a: [0.0, -24.5, 0.0], b: [0.0, -23.5, 0.0], r: 0.4, drone: true, skin: 0, contain: true }];
        assert_eq!(update(std::slice::from_mut(&mut d), &ch, &drone).len(), 1);
    }

    /// The one-shot law: param 5 = 1 → the volume retires after its first pass [M 0x17e5d0 state 2].
    #[test]
    fn the_one_shot_retires() {
        let mut h = quad(0.0);
        h.one_shot = true;
        h.inc = 2.0;
        h.keys = 3;
        let ch = Channels::default();
        let on = [Toucher::player([0.0, 0.0, 0.0])];
        let mut hurt = std::slice::from_mut(&mut h);
        assert_eq!(update(hurt, &ch, &on).len(), 1);
        hurt[0].t = 4.0; // past the key count: the anim's last tick retires it (the port's order:
        // the clock advances, then the retire check, then the damage — the retire tick's collide
        // contact lands on the retired object [S unverified: the game's collide/update order])
        assert_eq!(update(hurt, &ch, &on).len(), 0, "retired: state 2 stops everything");
        assert_eq!(hurt[0].state, 2);
    }
}
