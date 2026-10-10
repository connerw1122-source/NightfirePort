//! Switch channels and trigger objects (docs/systems/triggers.md): placements 0xEA TouchOnce, 0xEB Touch, 0xEC
//! MultiplexIn (AND), 0xED MultiplexSIn (ordered AND), 0xEE MultiplexOut, 0xEF MultiplexOrIn (OR), 0xDC Trigger (type
//! = p0). Volumes are the trigger model's collision triangles at the placement transform; contacts are capsules
//! (Bond: radius 0.55 from feet + 0.55 to feet + 1.86; drones: *suggested* radius 0.4 from the feet to the head).

use crate::world::V3;
use nf_formats::level::Level;
use nf_formats::placement::{object_matrix, transform};

pub mod kind {
    pub const TOUCH_ONCE: u8 = 1;
    pub const TOUCH: u8 = 2;
    pub const AND: u8 = 3;
    pub const SEQ_AND: u8 = 4;
    pub const OUT: u8 = 5;
    pub const LOAD_LEVEL: u8 = 10;
    pub const OR: u8 = 0xc;
}

#[derive(Clone, Debug)]
pub struct Trigger {
    /// Position in the main map's placement list (objects update in reverse placement order).
    pub placement: usize,
    pub kind: u8,
    pub out: u8,
    /// Contact mask (8 player, 0x10 drone, 0x18 either with the filter list).
    pub mask: u32,
    /// Touch: filter list; muxes: input channels; Out: target channels.
    pub list: Vec<u32>,
    pub lock: u8,
    pub hide: bool,
    pub tris: Vec<[V3; 3]>,
    lo: V3,
    hi: V3,
    /// TouchOnce removed itself (its channel was on).
    pub gone: bool,
    /// Made by a script (`Script_CreateEntity` 0xEA / 0xEB): the script object's index; its volume follows the object.
    pub script: Option<usize>,
    /// `LOAD_LEVEL` (0xE8): the next level's hashcode (placement parameter 0x20), requested when Bond touches it
    /// (`Trigger_Activate` 0x1b2318 → `ResetMap_LevelToLoad` 0x1d3800) [M].
    pub next_level: u32,
}

/// A body that can touch triggers: capsule axis from `a` to `b`, radius, drone (true) or Bond, drone skin hash.
#[derive(Clone, Copy, Debug)]
pub struct Toucher { pub a: V3, pub b: V3, pub r: f32, pub drone: bool, pub skin: u32 }

impl Toucher {
    /// Bond in the normal stance (`Player_Collision` 0x194DF0, hip height 1.035 read live).
    pub fn player(feet: V3) -> Toucher { Toucher { a: [feet[0], feet[1] + 0.55, feet[2]], b: [feet[0], feet[1] + 1.86, feet[2]], r: 0.55, drone: false, skin: 0 } }
}

/// The channel table (`switch_channels`) with the frame each channel was last set (`switch_channels_time`).
#[derive(Clone, Debug)]
pub struct Channels { pub v: Vec<u8>, pub time: Vec<u32> }

impl Default for Channels { fn default() -> Channels { Channels { v: vec![0; 256], time: vec![0; 256] } } }

impl Channels {
    pub fn on(&self, c: u32) -> bool { c != 0 && self.v.get(c as usize).is_some_and(|&x| x != 0) }
    pub fn set(&mut self, c: u32, value: u8, frame: u32) {
        if c == 0 || c > 255 { return; }
        if value != 0 && self.v[c as usize] == 0 { self.time[c as usize] = frame; }
        self.v[c as usize] = value;
    }
}

fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn lerp(a: V3, b: V3, t: f32) -> V3 { [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t] }

/// Closest distance between point p and triangle t.
fn point_triangle(p: V3, t: &[V3; 3]) -> f32 {
    let (e0, e1) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let n = cross(e0, e1);
    let nn = dot(n, n);
    if nn > 1e-12 {
        let d = dot(sub(p, t[0]), n) / nn;
        let q = sub(p, [n[0] * d, n[1] * d, n[2] * d]);
        let inside = (0..3).all(|k| { let (u, v) = (t[k], t[(k + 1) % 3]); dot(cross(sub(v, u), sub(q, u)), n) >= 0.0 });
        if inside { return (d * d * nn).sqrt(); }
    }
    (0..3).map(|k| segment_segment(p, p, t[k], t[(k + 1) % 3])).fold(f32::INFINITY, f32::min)
}

/// Closest distance between segments p1–q1 and p2–q2.
fn segment_segment(p1: V3, q1: V3, p2: V3, q2: V3) -> f32 {
    let (d1, d2, r) = (sub(q1, p1), sub(q2, p2), sub(p1, p2));
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let (s, t) = if a <= 1e-12 && e <= 1e-12 { (0.0, 0.0) } else if a <= 1e-12 { (0.0, (f / e).clamp(0.0, 1.0)) } else {
        let c = dot(d1, r);
        if e <= 1e-12 { ((-c / a).clamp(0.0, 1.0), 0.0) } else {
            let b = dot(d1, d2);
            let den = a * e - b * b;
            let mut s = if den > 1e-12 { ((b * f - c * e) / den).clamp(0.0, 1.0) } else { 0.0 };
            let mut t = (b * s + f) / e;
            if t < 0.0 { t = 0.0; s = (-c / a).clamp(0.0, 1.0); } else if t > 1.0 { t = 1.0; s = ((b - c) / a).clamp(0.0, 1.0); }
            (s, t)
        }
    };
    let (c1, c2) = (lerp(p1, q1, s), lerp(p2, q2, t));
    dot(sub(c1, c2), sub(c1, c2)).sqrt()
}

/// Whether the segment a–b crosses triangle t.
fn segment_hits_triangle(a: V3, b: V3, t: &[V3; 3]) -> bool {
    let d = sub(b, a);
    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 { return false; }
    let s = sub(a, t[0]);
    let u = dot(s, p) / det;
    if !(0.0..=1.0).contains(&u) { return false; }
    let q = cross(s, e1);
    let v = dot(d, q) / det;
    if v < 0.0 || u + v > 1.0 { return false; }
    (0.0..=1.0).contains(&(dot(e2, q) / det))
}

impl Trigger {
    /// A trigger of `kind` from its parameters (placement or script entity: parameter k at `level_tag+0x2C+4k`) and its
    /// world triangles; `None` for a Touch on channel 0x7D (only live on 0x07001500).
    pub fn from_params(placement: usize, kind: u8, q: impl Fn(u32) -> u32, tris: Vec<[V3; 3]>) -> Option<Trigger> {
        let (out_ch, mask, list, lock, hide, next_level) = match kind {
            kind::TOUCH_ONCE => (q(0), q(1), (2..=8).map(&q).collect(), q(7), q(8) != 0, 0),
            kind::TOUCH => (q(0), q(1), (2..=8).map(&q).collect(), q(6), q(7) != 0, 0),
            // `LOAD_LEVEL` (0xE8): out = p0 (the switch channel it sets), next_level = p0x20 (the next level hashcode).
            kind::LOAD_LEVEL => (q(0), q(9), (1..=8).filter(|&k| k != 7).map(&q).collect(), q(10), q(11) != 0, q(0x20)),
            _ => (q(0), q(9), (1..=8).filter(|&k| k != 7).map(&q).collect(), q(10), q(11) != 0, 0),
        };
        if matches!(kind, kind::TOUCH | kind::TOUCH_ONCE) && out_ch == 0x7d { return None; }
        let mut t = Trigger { placement, kind, out: out_ch as u8, mask, list, lock: lock as u8, hide, tris: Vec::new(), lo: [0.0; 3], hi: [0.0; 3], gone: false, script: None, next_level };
        t.set_tris(tris);
        Some(t)
    }

    /// New world triangles (a moving trigger).
    pub fn set_tris(&mut self, tris: Vec<[V3; 3]>) {
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for t in &tris { for v in t { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } } }
        (self.tris, self.lo, self.hi) = (tris, lo, hi);
    }

    /// Whether a capsule touches the trigger's triangles.
    pub fn touched_by(&self, c: &Toucher) -> bool {
        let grow = c.r + 0.1;
        let lo = [c.a[0].min(c.b[0]) - grow, c.a[1].min(c.b[1]) - grow, c.a[2].min(c.b[2]) - grow];
        let hi = [c.a[0].max(c.b[0]) + grow, c.a[1].max(c.b[1]) + grow, c.a[2].max(c.b[2]) + grow];
        if (0..3).any(|k| hi[k] < self.lo[k] || lo[k] > self.hi[k]) { return false; }
        self.tris.iter().any(|t| {
            segment_hits_triangle(c.a, c.b, t) || point_triangle(c.a, t) <= c.r || point_triangle(c.b, t) <= c.r
                || (0..3).any(|k| segment_segment(c.a, c.b, t[k], t[(k + 1) % 3]) <= c.r)
        })
    }

    /// `Trigger_Hit` (0x1B1FC8) for a Touch / TouchOnce trigger.
    fn hit(&self, touchers: &[Toucher]) -> bool {
        let touching: Vec<&Toucher> = touchers.iter().filter(|c| self.touched_by(c)).collect();
        let player = touching.iter().any(|c| !c.drone);
        let drones: Vec<&&Toucher> = touching.iter().filter(|c| c.drone).collect();
        match self.mask {
            8 => player,
            0x10 => !drones.is_empty(),
            0x18 => {
                if !(player || !drones.is_empty()) { return false; }
                let filters: Vec<u32> = self.list.iter().copied().filter(|&v| v != 0).collect();
                if filters.is_empty() { return true; }
                filters.iter().any(|&f| match f {
                    0xb => !drones.is_empty(),
                    0xe => player,
                    h => drones.iter().any(|c| c.skin & 0xffff == h & 0xffff),
                })
            }
            m => (m & 8 != 0 && player) || (m & 0x10 != 0 && !drones.is_empty()),
        }
    }

    /// `Trigger_Update` (0x1B1A78) for every type except Touch (run in `calc`).
    pub fn update(&mut self, ch: &mut Channels, touchers: &[Toucher], frame: u32) {
        if self.gone || (self.lock != 0 && ch.on(self.lock as u32)) { return; }
        match self.kind {
            kind::TOUCH_ONCE => {
                if ch.on(self.out as u32) { self.gone = true; return; }
                if self.hit(touchers) { ch.set(self.out as u32, 1, frame); }
            }
            kind::AND | kind::OR => {
                let inputs: Vec<u32> = self.list.iter().copied().filter(|&c| c != 0).collect();
                let v = if self.kind == kind::AND { !inputs.is_empty() && inputs.iter().all(|&c| ch.on(c)) } else { inputs.iter().any(|&c| ch.on(c)) };
                ch.set(self.out as u32, v as u8, frame);
            }
            kind::SEQ_AND => {
                // Ordered AND: each input set no more than 4 frames after the previous one.
                let inputs: Vec<u32> = self.list.iter().copied().filter(|&c| c != 0).collect();
                let ok = !inputs.is_empty() && inputs.iter().all(|&c| ch.on(c))
                    && inputs.windows(2).all(|w| { let (a, b) = (ch.time[w[0] as usize], ch.time[w[1] as usize]); b >= a && b - a <= 4 });
                ch.set(self.out as u32, ok as u8, frame);
            }
            kind::OUT => {
                let v = ch.v.get(self.out as usize).copied().unwrap_or(0);
                for &t in &self.list { if t != 0 { ch.set(t, v, frame); } }
            }
            // `LOAD_LEVEL` (0xE8): a one-shot touch trigger that sets its switch channel when Bond enters the volume,
            // then signals the next-level request (`Trigger_Activate` → `ResetMap_LevelToLoad`). The caller reads
            // `next_level` after `update` to drive the transition [M 0x1b2318].
            kind::LOAD_LEVEL => {
                if self.gone { return; }
                if self.hit(touchers) { ch.set(self.out as u32, 1, frame); self.gone = true; }
            }
            _ => {}
        }
    }
}

/// Every trigger of the level's main map.
pub fn build(level: &Level) -> Vec<Trigger> {
    let Some(m) = level.main_map() else { return Vec::new() };
    let map = &level.maps[m];
    let mut out = Vec::new();
    for (i, p) in map.placements.iter().enumerate() {
        let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
        let kind = match p.flags { 0xea => kind::TOUCH_ONCE, 0xeb => kind::TOUCH, 0xec => kind::AND, 0xed => kind::SEQ_AND, 0xee => kind::OUT,
            0xef => kind::OR, 0xe8 => kind::LOAD_LEVEL, 0xdc => param(0) as u8, _ => continue };
        let raw = p.flags == 0xdc;
        let shift = raw as u32; // a raw Trigger (0xDC) carries its type in p0 and the rest one later (*suggested*)
        let q = |k: u32| param(k + shift);
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
        let mx = object_matrix(p.rotation, p.translation);
        let tris: Vec<[V3; 3]> = target.and_then(|(om, oi)| level.maps[om].objects[oi].collision.as_ref())
            .map(|c| c.triangles().into_iter().map(|t| t.map(|v| transform(&mx, v))).collect()).unwrap_or_default();
        // Touch triggers on channel 0x7D only work on level 0x07001500.
        if let Some(t) = Trigger::from_params(i, kind, q, tris) { out.push(t); }
    }
    out
}

/// `Trigger_Calc` (0x1B1900): level latches, then the Touch triggers (their channels cleared, then OR-ed).
pub fn calc(level: u32, triggers: &mut [Trigger], ch: &mut Channels, touchers: &[Toucher], frame: u32) {
    if level == 0x0700_0005 && ch.on(0xa4) { ch.set(0x6f, 1, frame); }
    if level == 0x0700_000b && ch.on(0x34) { ch.set(0x2d, 1, frame); }
    for t in triggers.iter().filter(|t| t.kind == kind::TOUCH) { ch.set(t.out as u32, 0, frame); }
    let hits: Vec<u8> = triggers.iter().filter(|t| t.kind == kind::TOUCH && !(t.lock != 0 && ch.on(t.lock as u32)) && t.hit(touchers)).map(|t| t.out).collect();
    for out in hits { ch.set(out as u32, 1, frame); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plate(kind: u8, out: u8, mask: u32, list: Vec<u32>) -> Trigger {
        // A 2 × 2 horizontal quad at y = 0 around the origin.
        let (a, b, c, d) = ([-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0]);
        Trigger { placement: 0, kind, out, mask, list, lock: 0, hide: false, tris: vec![[a, b, c], [a, c, d]], lo: [-1.0, 0.0, -1.0], hi: [1.0, 0.0, 1.0], gone: false, script: None, next_level: 0 }
    }

    #[test]
    fn touch_once_latches_and_muxes_follow_inputs() {
        let mut ch = Channels::default();
        let mut once = plate(kind::TOUCH_ONCE, 0x23, 8, vec![]);
        let mut or = Trigger { kind: kind::OR, out: 0x22, list: vec![0x23, 0x24], ..plate(kind::OR, 0, 0, vec![]) };
        let away = [Toucher::player([5.0, 0.0, 5.0])];
        let on = [Toucher::player([0.5, -0.3, 0.0])];
        once.update(&mut ch, &away, 1);
        or.update(&mut ch, &away, 1);
        assert!(!ch.on(0x22));
        once.update(&mut ch, &on, 2);
        or.update(&mut ch, &on, 2);
        assert!(ch.on(0x23) && ch.on(0x22));
        // Latched: stays on after Bond leaves; the TouchOnce removes itself.
        once.update(&mut ch, &away, 3);
        or.update(&mut ch, &away, 3);
        assert!(ch.on(0x22) && once.gone);
        // A drone-only plate ignores Bond.
        let drone_plate = plate(kind::TOUCH_ONCE, 0x30, 0x10, vec![]);
        assert!(!drone_plate.hit(&on));
        assert!(drone_plate.hit(&[Toucher { drone: true, ..on[0] }]));
    }

    /// `LOAD_LEVEL` (0xE8): Bond touching the exit volume sets the switch channel once and latches `gone`; the
    /// `next_level` hashcode is carried for `ResetMap_LevelToLoad`. Sister: a Bond standing away never triggers it.
    #[test]
    fn load_level_trigger_fires_once_and_carries_next_level() {
        let mut ch = Channels::default();
        let mut exit = Trigger { kind: kind::LOAD_LEVEL, out: 0x40, next_level: 0x0700_0006, ..plate(kind::LOAD_LEVEL, 0x40, 8, vec![]) };
        let away = [Toucher::player([5.0, 0.0, 5.0])];
        let on = [Toucher::player([0.5, -0.3, 0.0])];
        // Sister: away → nothing.
        exit.update(&mut ch, &away, 1);
        assert!(!ch.on(0x40) && !exit.gone);
        // On → fires once, latches.
        exit.update(&mut ch, &on, 2);
        assert!(ch.on(0x40) && exit.gone);
        assert_eq!(exit.next_level, 0x0700_0006, "carries the next level hashcode");
        // Stays gone (no re-fire).
        exit.update(&mut ch, &on, 3);
        assert!(exit.gone);
    }
}
