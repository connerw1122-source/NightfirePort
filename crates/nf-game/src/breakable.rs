//! The breakable scenery: placement 0x20 (`Break_Create` 0x12c230, `Break_Update` 0x12bfd8,
//! `Break_ApplyDamage` 0x12c128, `Break_Kill` 0x12b970).
//!
//! Semantics [M]:
//! - `Break_Create` 0x12c230: class-0xC object, `obj+0xFF = 0x20`; tag+0x00 = placement param 0
//!   (the break kind; 2 = glass), tag+0x04 = param 1 as f32 (the health; 0 dies on the first
//!   bullet), tag+0x08 = param 2 == 0 ? 0 : ~param2 as u16 (the bullet filter word).
//! - `Break_Update` 0x12bfd8: per tick (obj+0xFE bit 0 clear) the contact list is filtered with
//!   `Collide_FilterBullets(obj+0xD0, filter | 0x106)` and walked: a class-0x05 (bullet) contact
//!   subtracts the node's damage float from tag+0x04; a class-0x02 (drone) contact damages ONLY
//!   when `GameState+0x12 == 0x7000001` AND kind == 2 — the damage is
//!   `Vec_SqDist3D(contact+0x30, contact+0x40) · 20.0` (the drone's squared step this tick):
//!   guards walking through the castle's glass smash it. Health < 0 → `Break_Kill`.
//! - `Break_Kill` 0x12b970: `obj+0xFE |= 1` (dead); kind == 2 → sound 0x2E and
//!   `ShatterPolyRecurse` per triangle of `psiGetTriList(obj)`; otherwise the sound is 0x5F2 when
//!   `Rand_Rand(100) < 0x29` (41%) else 0x5F3, then `Drone_BrokenObject` + `Debris_Create_Loop`.
//!   NO channel is written anywhere [M 0x12b970 — no switch_channels store].
//!
//! Port deviations (documented, none tested as game behaviour):
//! - the shatter/debris visuals and `Drone_BrokenObject`'s pieces are the renderer/props layer's
//!   work (the main agent's render check owns the look); the sim exposes `dead` + the kill effect.
//! - the 41% sound variant needs the game's `Rand_Rand` draw-order context; the port keeps the
//!   deterministic 0x5F3 arm and does not consume the shared RNG (`// [S] unverified`).
//! - the filter word's weapon-bit layout is not evidenced (all 55 placements on 07000001 carry
//!   param 2 = 0 [D]); the port applies the mask as an exclude filter only when non-zero.

use crate::world::V3;
use nf_formats::level::Level;
use nf_formats::placement::{object_matrix, transform};

/// One kill effect for the caller to present (sound + shatter/debris) — `Break_Kill` 0x12b970's
/// observable outputs. The game plays the sound at obj+0x80 (the placed object's position).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BreakEffect {
    pub placement: usize,
    /// The kill position (the bounds centre — the port's stand-in for obj+0x80).
    pub pos: V3,
    /// tag+0x00: 2 = glass (shatter + sound 0x2E), other = debris (sound 0x5F3; the 41% 0x5F2
    /// variant is `// [S] unverified` — see the module header).
    pub kind: u8,
    pub sound: u16,
}

#[derive(Clone, Debug)]
pub struct Break {
    pub placement: usize,
    pub lo: V3,
    pub hi: V3,
    /// tag+0x00 — placement param 0's low byte [M 0x12c230:21 `*puVar1 = *(param_4 + 0x2c)`].
    pub kind: u8,
    /// tag+0x04 — placement param 1's bits as f32 [M 0x12c230:23-29].
    pub health: f32,
    /// tag+0x08 — param 2 == 0 ? 0 : !(param 2) as u16 [M 0x12c230:31-39].
    pub filter: u16,
    /// obj+0xFE bit 0 — set by Break_Kill [M 0x12b970], gates Break_Update [M 0x12bfd8:14].
    pub dead: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Breaks {
    pub list: Vec<Break>,
}

/// One placement's target object's bounds placed by the placement transform (same rule as
/// crate::props::bounds — the glist bounds the collision pass would register).
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
    if corners.is_empty() { return ([0.0; 3], [0.0; 3]) }
    let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
    for v in corners {
        let v = transform(&mx, v);
        for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); }
    }
    (lo, hi)
}

impl Breaks {
    /// `Break_Create` over the main map's 0x20 placements (placement order, like the game's
    /// `parsemap_create_dynamic_objects` case 0x20 [M 0x1d04f0]).
    pub fn create(level: &Level) -> Breaks {
        let Some(m) = level.main_map() else { return Breaks::default() };
        let mut list = Vec::new();
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            if p.flags & 0xff != 0x20 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let (lo, hi) = bounds(level, m, p);
            let p2 = param(2);
            list.push(Break {
                placement: i,
                lo,
                hi,
                kind: (param(0) & 0xff) as u8,
                health: f32::from_bits(param(1)),
                filter: if p2 == 0 { 0 } else { !(p2 as u16) },
                dead: false,
            });
        }
        Breaks { list }
    }

    fn seg_box(a: V3, b: V3, lo: V3, hi: V3) -> bool {
        // Same rule as crate::props::seg_box: the bullet segment's AABB against the bounds grown
        // by 0.1 (the contact registers on the sphere touching the glist bounds).
        let (g, mut mn, mut mx) = (0.1, [0.0; 3], [0.0; 3]);
        for k in 0..3 {
            mn[k] = a[k].min(b[k]);
            mx[k] = a[k].max(b[k]);
            if mx[k] < lo[k] - g || mn[k] > hi[k] + g { return false; }
        }
        true
    }

    /// The bullet flight this tick against the breakables ([M Break_Update 0x12bfd8:44-48 — the
    /// class-5 contact subtracts its damage; the filter word is an exclude mask, inert on
    /// 07000001 where every param 2 is 0 [D]]). Returns the kills for presentation.
    pub fn bullet_hit(&mut self, a: V3, b: V3, weapon: u16, damage: f32) -> Vec<BreakEffect> {
        let mut kills = Vec::new();
        for br in self.list.iter_mut().filter(|b| !b.dead) {
            if br.filter != 0 && br.filter & (1u16 << (weapon & 15)) != 0 { continue; }
            if Self::seg_box(a, b, br.lo, br.hi) {
                br.health -= damage;
                if br.health < 0.0 {
                    br.dead = true;
                    kills.push(Self::effect(br));
                }
            }
        }
        kills
    }

    /// The drone-contact arm ([M Break_Update 0x12bfd8:23-34 — only on GameState+0x12 ==
    /// 0x7000001, only kind == 2, damage = the drone's squared step × 20]). `level_01` is the
    /// caller's `GameState+0x12 == 0x07000001` check.
    pub fn drone_step(&mut self, drone_from: V3, drone_to: V3, level_01: bool) -> Vec<BreakEffect> {
        let mut kills = Vec::new();
        if !level_01 { return kills; }
        let sq = (drone_to[0] - drone_from[0]).powi(2)
            + (drone_to[1] - drone_from[1]).powi(2)
            + (drone_to[2] - drone_from[2]).powi(2);
        if sq == 0.0 { return kills; }
        let dmg = sq * 20.0;
        for br in self.list.iter_mut().filter(|b| !b.dead && b.kind == 2) {
            // The contact is the drone's body inside the glass bounds this tick.
            let inside = drone_to.iter().zip(br.lo).zip(br.hi).all(|((&v, l), h)| v >= l && v <= h);
            if inside {
                br.health -= dmg;
                if br.health < 0.0 {
                    br.dead = true;
                    kills.push(Self::effect(br));
                }
            }
        }
        kills
    }

    fn effect(br: &Break) -> BreakEffect {
        let pos = [(br.lo[0] + br.hi[0]) / 2.0, (br.lo[1] + br.hi[1]) / 2.0, (br.lo[2] + br.hi[2]) / 2.0];
        let sound = if br.kind == 2 { 0x2E } else { 0x5F3 };
        BreakEffect { placement: br.placement, pos, kind: br.kind, sound }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(kind: u8, health: f32) -> Breaks {
        Breaks { list: vec![Break { placement: 0, lo: [4.0, 7.0, -3.0], hi: [6.0, 9.0, -1.0], kind, health, filter: 0, dead: false }] }
    }

    #[test]
    fn the_first_bullet_kills_a_zero_health_vase_with_the_debris_sound() {
        // The 07000001 vases carry params [(2,0)] only — kind 0, health 0.0 [D placements dump].
        let mut b = one(0, 0.0);
        let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x1e, 25.0);
        assert!(b.list[0].dead, "health 0 - damage < 0 → Break_Kill [M 0x12bfd8:44-50]");
        assert_eq!(kills.len(), 1);
        assert_eq!(kills[0].sound, 0x5F3, "non-glass → 0x5F3 (the 41% 0x5F2 variant is unported, [S])");
        assert_eq!(kills[0].kind, 0);
    }

    #[test]
    fn glass_shatters_with_sound_0x2e_and_the_sister_vase_survives_a_miss() {
        // radioroom_glass/comroom_glass carry params [(0,2),(1,0),(2,0)] — kind 2 [D placements].
        let mut b = one(2, 0.0);
        let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x1e, 25.0);
        assert_eq!(kills[0].sound, 0x2E, "glass → sound 0x2E [M 0x12b970: uVar5 = 0x2e]");
        // Sister: a shot wide of the box never touches the health.
        let mut b = one(2, 0.0);
        let kills = b.bullet_hit([50.0, 8.0, -4.0], [50.0, 8.0, -2.0], 0x1e, 25.0);
        assert!(kills.is_empty() && !b.list[0].dead, "a miss never damages the breakable");
    }

    #[test]
    fn a_health_pool_takes_repeated_hits_and_only_dies_below_zero() {
        // Destroy_test's rule: exactly 0.0 is NOT a kill [M 0x12bfd8:50 `fVar5 < 0.0`].
        let mut b = one(0, 50.0);
        for i in 0..5 {
            let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x1e, 10.0);
            assert!(kills.is_empty() && !b.list[0].dead, "hit {i}: {50 - 10*(i+1)} left");
        }
        let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x1e, 10.0);
        assert_eq!(kills.len(), 1, "the 6th crosses below zero");
    }

    #[test]
    fn a_guard_walking_through_glass_smashes_it_only_on_level_01() {
        // [M Break_Update 0x12bfd8:23-34: GameState+0x12 == 0x7000001 and kind == 2 gate the
        // drone arm; damage = squared step × 20 — a 0.5-step guard crosses 5.0, a vase (kind 0)
        // never takes drone damage.]
        let mut b = one(2, 0.0);
        let kills = b.drone_step([5.0, 8.0, -1.5], [5.3, 8.0, -1.5], true);
        assert_eq!(kills.len(), 1, "0.09 · 20 = 1.8 damage kills the 0-health glass");
        let mut v = one(0, 0.0);
        assert!(v.drone_step([5.0, 8.0, -1.5], [5.3, 8.0, -1.5], true).is_empty(),
            "kind != 2 never takes drone damage [M 0x12bfd8:31-34]");
        let mut g = one(2, 0.0);
        assert!(g.drone_step([5.0, 8.0, -1.5], [5.3, 8.0, -1.5], false).is_empty(),
            "off 07000001 the drone arm is dead [M 0x12bfd8:24]");
    }
}
