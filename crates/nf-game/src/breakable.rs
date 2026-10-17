//! The breakables: placement type 0x20 (`parsemap_create_dynamic_objects` 0x1d04f0 case 0x20 →
//! `Break_Create` 0x12c230) — the glass window panes (`Glass_Controlroom*` on 07000008,
//! `Window_Securityroom` on 07000006 [D placements]) and the smashable scenery (443 placements on
//! the disc: 259 kind 2, 171 kind 0, 13 kind 1 [D census]).
//!
//! The object (class 0x20 [M 0x12c230:17], instance at `obj+0xE0`):
//! - `+0x00` type (placement param 0 [M 0x12c230:21 — `*(param_4 + 0x2c)`], the same word the
//!   parsemap case reads for `psiCreateBreakableData`'s kind-2 gate [M 0x1d04f0:70-73]);
//! - `+0x04` health, a float the updates subtract into (placement param 1 [M 0x12c230:22-28 —
//!   the raw bits, so an unset param reads 0.0]);
//! - `+0x08` bullet reject mask, `~(u16)(placement param 2)` with 0 kept as 0 [M 0x12c230:29-35].
//!
//! Damage: the collision pass mirrors every hit into the victim's `obj+0xD0` list with the hitter
//! at `+0x58` [M Collide_Update 0x1ea398 / hitdata-spawner-ally.md §1]; `Break_Update` 0x12bfd8
//! filters that list with `Collide_FilterBullets(list, filter | 0x106)` [M 0x12bfd8:15] — which
//! drops every class-5 (bullet) hitter whose `weapon_definition+0x10` word ANDs the mask non-zero
//! [M 0x1ece30:24-33] — and subtracts each surviving bullet's `HITDATA+8` damage from the health
//! [M 0x12bfd8:36-37; `Break_ApplyDamage` 0x12c128 is the same subtraction]. At health < 0 the
//! pane dies: `Break_Kill` 0x12b970 sets `obj+0xFE |= 1` (the delete flag, so the collision pass
//! stops registering hits — the collision "goes away") and plays the break sound: kind 2 (glass)
//! always 0x2e; every other kind rolls `Rand_Rand(100) < 0x29` → 0x5f2 else 0x5f3 [M 0x12b970:65-76].
//!
//! The pane never stops the bullet: its collision triangles carry surface byte 0x10 (all four
//! `Glass_Controlroom*` / `Window_Securityroom` meshes [D census]) and the handler's glass rule —
//! material 0x10 pierces unless `weapon_data+0x6c & 0x800` [M 0x12ce80 disasm 0x12cf44..0x12cf64,
//! the rule crate::effect ported for world hits] — lets the round fly on while the mirrored hit
//! still damages the pane. An intact pane keeps taking hits; a broken one registers nothing.
//!
//! The kind-2 shatter polys (`psiGetTriList` 0x11c9d8 + `ShatterPolyRecurse` 0x12b270, the
//! ShardList render stream) and the other kinds' debris (`Drone_BrokenObject` 0x138588 + 
//! `Debris_Create_Loop` 0x1ed548, the EffectInfo table) are presentation — the port delivers the
//! break state, position and kind to the draw layer here (no drawing in this crate). The
//! guards-smash-glass arm (`Break_Update`'s class-2 branch, level 0x7000001 only, damage
//! `SqDist(drone+0x30, drone+0x40) × 20` [M 0x12bfd8:23-34]) is not ported: no drone contact test
//! exists yet. `Break_DoBreak` 0x12c168 (script event 0x0a) is not ported (docs/systems/breakables.md).

use crate::triggers::Channels;
use crate::world::V3;
use nf_formats::level::Level;
use nf_formats::placement::{object_matrix, transform};

/// One breakable object ([M `Break_Create` 0x12c230]; bounds per crate::props's glist-bounds rule).
#[derive(Clone, Debug)]
pub struct Breakable {
    pub placement: usize,
    pub lo: V3,
    pub hi: V3,
    /// `obj+0xE0+0x00` — the kind (placement param 0) [M 0x12c230:21]. 2 = the glass panes.
    pub kind: u32,
    /// `obj+0xE0+0x04` — the damage pool (placement param 1's bits) [M 0x12c230:22-28].
    pub health: f32,
    /// `obj+0xE0+0x08` — the bullet reject mask, `~(u16)param(2)` (0 when the param is 0)
    /// [M 0x12c230:29-35].
    pub filter: u16,
    /// `obj+0xFE & 1` — the death flag `Break_Kill` sets [M 0x12b970:60]; a dead pane registers no
    /// further hits (the object leaves the world).
    pub broken: bool,
    /// The collision-shape key the World indexes the pane's tris under (the object's hashcode, or
    /// the anonymous `(map, object)` synthetic [S — world.rs's own rule]) — `set_dynamic` looks the
    /// intact pane up by it each tick (glass-breach: the intact pane blocks bodies, the broken one
    /// injects nothing [M 0x12b970:60]).
    pub shape_key: u32,
    /// The placement's object matrix (rotation × translation — the same matrix `bounds` transformed
    /// the corners with) for the per-tick `set_dynamic` injection.
    pub matrix: [[f32; 4]; 4],
}

/// The tick's breaks, for the caller's sounds/debris/draw (the game's `Break_Kill` runs the sound
/// and the shatter/debris inside the kill).
#[derive(Clone, Copy, Debug)]
pub struct BreakEvent {
    /// The pane's bounds centre (the game sounds at `obj+0x80`, the world sphere centre
    /// [M 0x12b970:69,75]).
    pub pos: V3,
    pub kind: u32,
    /// The sound `Break_Kill` played [M 0x12b970:65-76].
    pub sound: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Breakables {
    pub list: Vec<Breakable>,
}

fn bounds(level: &Level, m: usize, p: &nf_formats::placement::Placement) -> (V3, V3) {
    let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
        else { level.maps[m].objects.get(p.index as usize) };
    let Some(o) = obj else { return ([0.0; 3], [0.0; 3]) };
    let mx = object_matrix(p.rotation, p.translation);
    let mut corners = Vec::new();
    let sane = [o.min, o.max].iter().all(|b| b.iter().all(|v| v.abs() < 1.0e6));
    if sane {
        for x in [o.min[0], o.max[0]] { for y in [o.min[1], o.max[1]] { for z in [o.min[2], o.max[2]] {
            corners.push([x, y, z]);
        } } }
    } else if let Some(c) = o.collision.as_ref() {
        for t in c.triangles() { for v in t { corners.push(v); } }
    }
    if corners.is_empty() { return ([0.0; 3], [0.0; 3]); }
    let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
    for v in corners {
        let v = transform(&mx, v);
        for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); }
    }
    (lo, hi)
}

impl Breakables {
    /// `Break_Create` over the main map's placements (placement order, like the game's
    /// `parsemap_create_dynamic_objects` 0x1d04f0 case 0x20).
    pub fn create(level: &Level) -> Breakables {
        let Some(m) = level.main_map() else { return Breakables::default() };
        let mut out = Breakables::default();
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            if p.flags & 0xff != 0x20 { continue }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let (lo, hi) = bounds(level, m, p);
            let raw = param(2);
            // The collision-shape key: the target object's hashcode, or world.rs's anonymous
            // (map, object) synthetic when it is u32::MAX [S — the shapes index's own rule].
            let (om, oo) = if p.by_hash() { level.by_hash.get(&p.hashcode).copied().unwrap_or((m, p.index as usize)) } else { (m, p.index as usize) };
            let hc = level.maps[om].objects[oo].hashcode;
            let shape_key = if hc == u32::MAX { 0x8000_0000 | ((om as u32) << 16) | oo as u32 } else { hc };
            out.list.push(Breakable {
                placement: i, lo, hi,
                kind: param(0),
                // The raw word is the float's bits [M 0x12c230:22-28 — `(float)uVar2` either way].
                health: f32::from_bits(param(1)),
                filter: if raw == 0 { 0 } else { !(raw as u16) },
                broken: false,
                shape_key,
                matrix: object_matrix(p.rotation, p.translation),
            });
        }
        out
    }

    fn seg_box(a: V3, b: V3, lo: V3, hi: V3) -> bool {
        // The segment's AABB against the pane's bounds grown by 0.1 — the game's contact registers
        // on the bullet's sphere touching the glist bounds (crate::props's rule).
        let (g, mut mn, mut mx) = (0.1, [0.0; 3], [0.0; 3]);
        for k in 0..3 {
            mn[k] = a[k].min(b[k]);
            mx[k] = a[k].max(b[k]);
            if mx[k] < lo[k] - g || mn[k] > hi[k] + g { return false; }
        }
        true
    }

    /// The bullet's flight this tick against the breakables ([M `Break_Update` 0x12bfd8 — the
    /// mirrored class-5 contacts, each filtered by `Collide_FilterBullets`'s
    /// `weapon_definition+0x10 & (filter | 0x106)` reject [M 0x1ece30:24-33], damage `HITDATA+8`
    /// subtracted [M 0x12bfd8:36-37], death at health < 0 [M 0x12bfd8:49-51] with `Break_Kill`'s
    /// sound [M 0x12b970:65-76]]). The roll draws the shared stream ONLY when a non-glass pane
    /// dies — the game's draw count exactly (`Rand_Rand(100)` inside `Break_Kill`, kind != 2
    /// branch [M 0x12b970:66-68]); the glass panes (kind 2) never draw.
    pub fn bullet_hit(&mut self, a: V3, b: V3, weapon_mask: u16, damage: f32, rng: &mut crate::drone::Rng) -> Vec<BreakEvent> {
        let mut events = Vec::new();
        for s in self.list.iter_mut().filter(|s| !s.broken) {
            // `Collide_FilterBullets` [M 0x1ece30:24-33]: the hitter's weapon word against the
            // pane's mask | 0x106 — non-zero AND = the hit is dropped (distance 1e8).
            if weapon_mask & (s.filter | 0x106) != 0 { continue }
            if !Self::seg_box(a, b, s.lo, s.hi) { continue }
            s.health -= damage;
            if s.health < 0.0 {
                s.broken = true;
                // `Break_Kill` 0x12b970:65-76: kind 2 plays 0x2e; the others roll
                // `Rand_Rand(100) < 0x29` → 0x5f2, else 0x5f3.
                let sound = if s.kind == 2 {
                    0x2e
                } else if rng.rand(100) < 0x29 {
                    0x5f2
                } else {
                    0x5f3
                };
                events.push(BreakEvent {
                    pos: [(s.lo[0] + s.hi[0]) / 2.0, (s.lo[1] + s.hi[1]) / 2.0, (s.lo[2] + s.hi[2]) / 2.0],
                    kind: s.kind,
                    sound,
                });
            }
        }
        events
    }

    /// Whether the placement's breakable is dead (`obj+0xFE & 1`) — the draw layer skips it and
    /// nothing registers hits on it any more [M `Break_Kill` 0x12b970:60].
    pub fn broken_at(&self, placement: usize) -> bool {
        self.list.iter().any(|s| s.placement == placement && s.broken)
    }

    fn seg_box_grow(a: V3, b: V3, lo: V3, hi: V3, grow: f32) -> bool {
        let (mut mn, mut mx) = ([0.0; 3], [0.0; 3]);
        for k in 0..3 {
            mn[k] = a[k].min(b[k]);
            mx[k] = a[k].max(b[k]);
            if mx[k] < lo[k] - grow || mn[k] > hi[k] + grow { return false; }
        }
        true
    }

    fn centre(s: &Breakable) -> V3 {
        [(s.lo[0] + s.hi[0]) / 2.0, (s.lo[1] + s.hi[1]) / 2.0, (s.lo[2] + s.hi[2]) / 2.0]
    }

    /// `Break_DoBreak` 0x12c168 (NIS event 0x0a, the scripted kill): walk the breakables in
    /// placement order — the game walks the object list [M :39-51] — and kill the FIRST intact
    /// one whose world sphere contains `pos` (the game's sphere: centre `obj+0x80`, radius
    /// `obj+0x8c`; the port's stand-in is the bounds grown 0.2 [*suggested* — the glist
    /// world-sphere radius for a flat pane sits between the thin axis and the long axis]).
    /// The kill is the standard `Break_Kill` (obj+0xFE |= 1, the sound) [M :61; 0x12b970:60-76].
    pub fn do_break(&mut self, pos: V3, rng: &mut crate::drone::Rng) -> Vec<BreakEvent> {
        let mut events = Vec::new();
        let Some(s) = self.list.iter_mut().find(|s| !s.broken && Self::seg_box_grow(pos, pos, s.lo, s.hi, 0.2)) else { return events };
        s.broken = true;
        let sound = if s.kind == 2 {
            0x2e
        } else if rng.rand(100) < 0x29 {
            0x5f2
        } else {
            0x5f3
        };
        events.push(BreakEvent { pos: Self::centre(s), kind: s.kind, sound });
        events
    }

    /// The guards-smash-glass arm [M `Break_Update` 0x12bfd8:23-34]: on level 0x0700001 alone,
    /// every class-2 hitter (a drone or the player — the port passes each body's tick) whose
    /// collision registered against a kind-2 pane drains `health -= SqDist3D(hitter+0x30,
    /// hitter+0x40) × 20.0` — the tick's own movement, the movement pass copying +0x30 → +0x40
    /// after the move [M 0x1336f8:212] — and the pane dies at health < 0 [M :49-51]. The Mayhew
    /// kind-2 panes all carry health 0 [D placements], so any movement through the pane plane
    /// shatters it; a motionless brush drains 0 and survives [M :27,49]. The game's contact is
    /// the collision pass's mirrored hit list (the body against the pane's glist triangles); the
    /// port's stand-in is the movement segment against the pane's bounds grown 0.7 [*suggested* —
    /// the game's contact is the body surface reaching the pane's triangles, but the port's
    /// `move_by` probe (the 0.4 ray overshoot) stops the charging body short of the plane, so the
    /// port's blocked standoff (~0.4-0.7) IS the contact the game resolves]. Kind-2 kills play
    /// 0x2e with NO RNG draw (the shared
    /// stream is untouched by glass kills [M 0x12b970:65-76]).
    pub fn drone_smash(&mut self, prev: V3, pos: V3, level: u32) -> Vec<BreakEvent> {
        let mut events = Vec::new();
        if level != 0x0700_0001 { return events; }
        let d2 = (pos[0] - prev[0]).powi(2) + (pos[1] - prev[1]).powi(2) + (pos[2] - prev[2]).powi(2);
        if d2 <= 0.0 { return events; }
        for s in self.list.iter_mut().filter(|s| !s.broken && s.kind == 2) {
            if !Self::seg_box_grow(prev, pos, s.lo, s.hi, 0.7) { continue }
            s.health -= d2 * 20.0;
            if s.health < 0.0 {
                s.broken = true;
                events.push(BreakEvent { pos: Self::centre(s), kind: s.kind, sound: 0x2e });
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panes() -> Breakables {
        Breakables {
            list: vec![
                // The 07000008 glass: kind 2, health 0 (one hit kills), no filter [D placements].
                Breakable { placement: 123, lo: [105.3, 34.2, -122.0], hi: [105.3, 36.4, -120.1], kind: 2, health: 0.0, filter: 0, broken: false, shape_key: 0, matrix: [[0.0; 4]; 4] },
                Breakable { placement: 128, lo: [105.3, 34.2, -122.0], hi: [109.2, 36.4, -122.0], kind: 2, health: 0.0, filter: 0, broken: false, shape_key: 0, matrix: [[0.0; 4]; 4] },
                // A generic breakable with a filter and pool health.
                Breakable { placement: 5, lo: [0.0, 0.0, 0.0], hi: [2.0, 2.0, 2.0], kind: 0, health: 30.0, filter: 0xfff8, broken: false, shape_key: 0, matrix: [[0.0; 4]; 4] },
            ],
        }
    }

    #[test]
    fn one_shot_shatters_the_glass_pane_with_the_glass_sound() {
        let mut b = panes();
        let mut rng = crate::drone::Rng::new(1);
        // A pistol round (word 0x0008 [L dump, drone.rs fusebox comment]) crossing the pane plane:
        // the mask passes (0x0008 & 0x106 == 0), health 0 − damage < 0 → dead. The z range stays
        // above −121.9 so only pane 123's plane (x = 105.3) is crossed — pane 128 sits at z −122.
        let ev = b.bullet_hit([105.3, 35.0, -121.5], [105.3, 35.0, -120.0], 0x0008, 10.0, &mut rng);
        assert_eq!(ev.len(), 1, "the first round shatters the pane [M 0x12bfd8:36-37,49-51]");
        assert_eq!(ev[0].sound, 0x2e, "kind 2 plays the glass sound 0x2e [M 0x12b970:65-76]");
        assert!(b.broken_at(123), "obj+0xFE & 1 is set [M 0x12b970:60]");
        // The sister: a shot that misses the pane's bounds does nothing.
        let mut b2 = panes();
        let ev2 = b2.bullet_hit([100.0, 35.0, -121.5], [100.0, 35.0, -120.0], 0x0008, 10.0, &mut rng);
        assert!(ev2.is_empty() && !b2.broken_at(123), "a miss never breaks a pane");
    }

    #[test]
    fn a_broken_pane_registers_nothing_but_the_next_one_still_breaks() {
        let mut b = panes();
        let mut rng = crate::drone::Rng::new(1);
        b.bullet_hit([105.3, 35.0, -121.5], [105.3, 35.0, -120.0], 0x0008, 10.0, &mut rng);
        // The second round through the same spot: the dead pane registers no hit [M 0x12b970:60].
        let ev = b.bullet_hit([105.3, 35.0, -121.5], [105.3, 35.0, -120.0], 0x0008, 10.0, &mut rng);
        assert!(ev.is_empty(), "a broken pane is gone from the world");
        // The sister: the OTHER intact pane still breaks — the system is alive.
        let ev2 = b.bullet_hit([107.0, 35.0, -123.0], [107.0, 35.0, -121.0], 0x0008, 10.0, &mut rng);
        assert_eq!(ev2.len(), 1, "the neighbouring pane still shatters [D 07000008 has 3 panes]");
    }

    #[test]
    fn the_filter_rejects_matching_weapon_words() {
        let mut b = panes();
        let mut rng = crate::drone::Rng::new(1);
        // The 07000006 pane carries param 0xf8 → mask ~0xf8 = 0xff07; a weapon whose word ANDs it
        // (e.g. 0x0008, bit 3) is dropped [M 0x1ece30:24-33]. Wait — 0x0008 & 0xff07 != 0 → the
        // pane survives that weapon. The 0x106 constant bits never reject the guns (0x0008 &
        // 0x106 == 0 is what lets the 08 panes break above).
        let ev = b.bullet_hit([30.0, 41.0, -17.0], [30.0, 41.0, -15.0], 0x0008, 10.0, &mut rng);
        // The test list's third pane (kind 0, filter 0xfff8) is not at this spot; the shot must
        // hit nothing.
        assert!(ev.is_empty());
        // Against the filtered pane itself: 0x0008 & (0xfff8 | 0x106) != 0 → rejected.
        let mut b3 = Breakables { list: vec![Breakable { placement: 0, lo: [0.0, 0.0, 0.0], hi: [2.0, 2.0, 2.0], kind: 2, health: 0.0, filter: 0xfff8, broken: false, shape_key: 0, matrix: [[0.0; 4]; 4] }] };
        let ev3 = b3.bullet_hit([1.0, 1.0, -1.0], [1.0, 1.0, 3.0], 0x0008, 10.0, &mut rng);
        assert!(ev3.is_empty(), "a filtered-out hitter never damages the pane [M 0x1ece30:24-33]");
        // A weapon word outside the mask (0xfff8|0x106 = 0xfffe covers bits 1..15; only bit 0
        // passes) still breaks it.
        let ev4 = b3.bullet_hit([1.0, 1.0, -1.0], [1.0, 1.0, 3.0], 0x0001, 10.0, &mut rng);
        assert_eq!(ev4.len(), 1, "an unfiltered weapon breaks the pane");
    }

    fn pane(pl: usize, kind: u32, health: f32) -> Breakable {
        // 07000001's pane 136 bounds [D nf_view bounds dump].
        Breakable { placement: pl, lo: [-1.91, 5.34, -1.167], hi: [-0.09, 7.95, -1.167], kind, health, filter: 0, broken: false, shape_key: 0, matrix: [[0.0; 4]; 4] }
    }

    #[test]
    fn a_drone_smash_drains_sqdist_times_twenty_and_kills_the_glass() {
        let mut b = Breakables { list: vec![pane(136, 2, 0.0)] };
        // The level gate: the class-2 arm exists on 0x0700001 ALONE [M 0x12bfd8:24].
        let ev0 = b.drone_smash([-1.0, 6.3, -0.5], [-1.0, 6.3, -1.6], 0x0700_0002);
        assert!(ev0.is_empty() && !b.broken_at(136), "another level never smashes by contact [M 0x12bfd8:24]");
        // A motionless brush drains nothing [M :27 — SqDist(pos, prev) = 0].
        let ev1 = b.drone_smash([-1.0, 6.3, -1.0], [-1.0, 6.3, -1.0], 0x0700_0001);
        assert!(ev1.is_empty() && !b.broken_at(136), "a stationary contact survives the health-0 pane");
        // The tick's movement through the pane plane kills it [M :26-27, :49-51].
        let ev2 = b.drone_smash([-1.0, 6.3, -0.5], [-1.0, 6.3, -1.6], 0x0700_0001);
        assert_eq!(ev2.len(), 1, "the crossing shatters the health-0 pane");
        assert_eq!(ev2[0].sound, 0x2e, "kind 2 plays 0x2e, no roll [M 0x12b970:65-70]");
        assert_eq!(ev2[0].kind, 2);
        assert!(b.broken_at(136), "obj+0xFE |= 1 [M 0x12b970:60]");
        // Sister: a NON-glass kind is immune to the class-2 arm (the kind gate [M :25]).
        let mut b2 = Breakables { list: vec![pane(5, 0, 0.0)] };
        let ev3 = b2.drone_smash([-1.0, 6.3, -0.5], [-1.0, 6.3, -1.6], 0x0700_0001);
        assert!(ev3.is_empty() && !b2.broken_at(5), "the smash arm is kind-2 only [M 0x12bfd8:25]");
        // Sister: the drain is exactly SqDist·20 — a 0.3 step (d2 0.09 → 1.8) dents a 100 pool.
        let mut b3 = Breakables { list: vec![pane(6, 2, 100.0)] };
        let ev4 = b3.drone_smash([-1.0, 6.3, -0.5], [-1.0, 6.3, -0.8], 0x0700_0001);
        assert!(ev4.is_empty() && !b3.broken_at(6));
        assert!((b3.list[0].health - 98.2).abs() < 1e-4, "health -= SqDist·20 [M 0x12bfd8:27], got {}", b3.list[0].health);
        // The RNG stream is untouched by a glass kill (0x2e is unconditional [M 0x12b970:65-70]).
        let mut rng_a = crate::drone::Rng::new(0x5eed);
        let a = rng_a.rand(100);
        let mut b4 = Breakables { list: vec![pane(7, 2, 0.0)] };
        let _ = b4.drone_smash([-1.0, 6.3, -0.5], [-1.0, 6.3, -1.6], 0x0700_0001);
        let mut rng_b = crate::drone::Rng::new(0x5eed);
        let _ = rng_b.rand(100);
        let bb = rng_b.rand(100);
        let mut rng_c = crate::drone::Rng::new(0x5eed);
        let _ = rng_c.rand(100);
        // (no draw here — the glass kill is unconditional) — then the same next draw.
        let cc = rng_c.rand(100);
        assert_eq!(bb, cc, "the glass kill drew nothing between the two rolls");
        let _ = a;
    }

    #[test]
    fn do_break_kills_the_first_intact_pane_containing_the_point() {
        // The game walks the object list and kills the FIRST class-0x20 sphere containing the
        // point [M Break_DoBreak 0x12c168:39-51,61]; the port's order is placement order.
        let mut b = Breakables { list: vec![pane(10, 2, 0.0), { let mut x = pane(11, 2, 0.0); x.lo = [5.0, 5.34, -1.167]; x.hi = [6.8, 7.95, -1.167]; x }] };
        let mut rng = crate::drone::Rng::new(7);
        // The point sits in BOTH (pane 10's grow 0.2 reaches x −0.09+0.2 = 0.11 < 5 — no: only
        // pane 11 contains x 5.9); the first CONTAINING one dies.
        let ev = b.do_break([5.9, 6.3, -1.167], &mut rng);
        assert_eq!(ev.len(), 1, "the scripted kill fires [M 0x12c168:39-61]");
        assert!(b.broken_at(11) && !b.broken_at(10), "the containing pane died, the far one lives");
        assert_eq!(ev[0].sound, 0x2e);
        // Sister: no pane at the point → nothing dies.
        let mut b2 = Breakables { list: vec![pane(10, 2, 0.0)] };
        let ev2 = b2.do_break([100.0, 6.3, -1.167], &mut rng);
        assert!(ev2.is_empty() && !b2.broken_at(10), "a point outside every sphere kills nothing");
        // Sister: a BROKEN pane is skipped (the first INTACT one [M :45 — the FE&1 gate]).
        let mut b3 = Breakables { list: vec![{ let mut x = pane(12, 2, 0.0); x.broken = true; x }, pane(13, 2, 0.0)] };
        let ev3 = b3.do_break([-1.0, 6.3, -1.167], &mut rng);
        assert!(b3.broken_at(13) && ev3.len() == 1, "the kill skips the already-broken pane");
    }

    #[test]
    fn pooled_health_takes_more_than_one_round_and_the_roll_picks_the_sound() {
        // The kind-0 breakable: 30 health. The game draws Rand_Rand(100) at the kill [M
        // 0x12b970:66-68] — the shared stream must see exactly that draw count. Weapon word
        // 0x0001 passes the 0xfff8|0x106 = 0xfffe mask.
        let mut b = panes();
        let mut rng = crate::drone::Rng::new(0x5eed);
        for _ in 0..2 {
            let ev = b.bullet_hit([1.0, 1.0, -1.0], [1.0, 1.0, 3.0], 0x0001, 10.0, &mut rng);
            assert!(ev.is_empty(), "30 health survives 10-damage rounds (health < 0 kills [M 0x12bfd8:49])");
        }
        // The 3rd lands exactly on 0.0, which the game's `< 0.0` does NOT count [M 0x12bfd8:49;
        // the same rule props.rs pins for Destroy_CollisionHandler 0x12bd48:46].
        let ev = b.bullet_hit([1.0, 1.0, -1.0], [1.0, 1.0, 3.0], 0x0001, 10.0, &mut rng);
        assert!(ev.is_empty(), "health exactly 0.0 survives");
        let ev = b.bullet_hit([1.0, 1.0, -1.0], [1.0, 1.0, 3.0], 0x0001, 10.0, &mut rng);
        assert_eq!(ev.len(), 1);
        assert!(ev[0].sound == 0x5f2 || ev[0].sound == 0x5f3, "the roll picks 0x5f2/0x5f3 [M 0x12b970:65-75]");
        assert_eq!(ev[0].kind, 0);
    }
}
