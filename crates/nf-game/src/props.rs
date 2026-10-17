//! The photograph targets and the breakable doors: placement 0xFD (`CamSubject_Create` 0x1bc5d8,
//! `CamSubject_Update` 0x1bc668) and 0xE1 (`Destroy_Create` 0x12c318, `Destroy_CollisionHandler`
//! 0x12bd48, `Destroy_Update` 0x12c4f8).
//!
//! The game's contact model: the collision pass registers every object overlapping the prop's glist
//! into its contact list (obj+0xD0); the per-tick update walks that list:
//! - `CamSubject_Update` 0x1bc668 writes `switch_channels[obj+0xF6] = 1` (obj+0xF6 = placement
//!   param 0 [M 0x1bc5d8:26]) for every contacting object of class 5 (a bullet, obj+0xFF == 5)
//!   whose weapon byte reads 0x54 — the camera gadget ([M 0x1bc668:14-24: the bullet's BU datum's
//!   weapon_data+2 == 0x54]). The 0700000c jetpack display (#414/#454) writes ch 0x1c and the
//!   0700000d laser display (#494/#495) ch 0x1e [D placements].
//! - `Destroy_CollisionHandler` 0x12bd48 subtracts each contacting bullet's damage from the tag's
//!   health (+0x04 = placement param 3 [M 0x12c318:36]); at health < 0 it writes
//!   `switch_channels[tag+0x12] = 1` (tag+0x12 = placement param 0 [M 0x12c318:35]) and smashes.
//!   The 0700000d "C5" doors (#93/#96 `bigdoor01/02`) carry ch 0x1f and health 1000/10000
//!   [D placements; Destroy_Update 0x12c4f8's second arm — an external channel kill — and the
//!   p6 countdown arm are not ported: both are inert here, p6 = 0 and no writer for the tag+0x10
//!   channel exists on the disc's placements [D]].
//!
//! The port checks the bullet's flight segment against each prop's bounds box (the glist bounds
//! the collision pass would register) in `Drones::update_player_bullets` — one check per tick per
//! bullet, the channel latching on the first overlap, as the game's contact list does.

use crate::triggers::Channels;
use crate::world::V3;
use nf_formats::level::Level;
use nf_formats::placement::{object_matrix, transform};

#[derive(Clone, Debug)]
pub struct CamSubject {
    pub placement: usize,
    pub lo: V3,
    pub hi: V3,
    /// `obj+0xF6` — the channel the photograph writes [M 0x1bc5d8:26].
    pub channel: u8,
    pub hit: bool,
}

#[derive(Clone, Debug)]
pub struct Destroy {
    pub placement: usize,
    pub lo: V3,
    pub hi: V3,
    /// `tag+0x12` — the channel the death writes [M 0x12c318:35].
    pub channel: u8,
    /// `tag+0x04` — the bullet-damage pool [M 0x12c318:36, 0x12bd48:44-48].
    pub health: f32,
    pub dead: bool,
    /// `tag+0x16` — the smash type byte (placement param 2's low byte [M 0x12c3c8 disasm
    /// `lbu v1,52(s0)` → `sb v1,22(a0)`]): `Destroy_Smash` 0x12bc50-0x12bd20 routes the blast
    /// script on it — 1 → 0x6000027 (radius ×4), 2 → 0x600004f (×2), other non-zero → 0x6000052
    /// (×3); a type-0 tag plays NO script (`if (cVar4 != 0)` [M 0x12bc50]).
    pub kind: u8,
    /// `tag+0x08` — the smash blast's damage float (placement param 5 [M 0x12c424 disasm
    /// `lw v0,64(s0)` → decoded → tag+8]; `Destroy_Smash` passes it as `Explode_Create`'s f13-slot
    /// damage [M 0x12bd04 `lwc1 f13,8(s1)`]). The disc's destroyables all carry small positive
    /// ints here (10…3000 [D census]), the only branch the port decodes.
    pub blast_damage: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Props {
    pub subjects: Vec<CamSubject>,
    pub destroy: Vec<Destroy>,
}

/// One placement's target object's bounds placed by the placement transform (the game's contact
/// is against the placed glist), in placement order. Objects whose gfx bounds are unset (anim-only
/// meshes: min/max stay at the parser's ±1e7 sentinel) fall back to their collision triangles.
pub(crate) fn bounds(level: &Level, m: usize, p: &nf_formats::placement::Placement) -> (V3, V3) {
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

impl Props {
    /// `CamSubject_Create` / `Destroy_Create` over the main map's placements (placement order, like
    /// the game's `parsemap_create_dynamic_objects`).
    pub fn create(level: &Level) -> Props {
        let Some(m) = level.main_map() else { return Props::default() };
        let mut props = Props::default();
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let (lo, hi) = bounds(level, m, p);
            match p.flags & 0xff {
                // `CamSubject_Create` 0x1bc5d8: class 0x48, obj+0xF6 = placement +0x2C (param 0).
                0xfd => props.subjects.push(CamSubject { placement: i, lo, hi, channel: param(0) as u8, hit: false }),
                // `Destroy_Create` 0x12c318: tag+0x12 = param 0, tag+0x04 (health) = param 3
                // (as f32; the tag holds a float the handler subtracts into), tag+0x16 (kind) =
                // param 2's low byte, tag+0x08 (smash blast damage) = param 5 [M disasm].
                0xe1 => props.destroy.push(Destroy { placement: i, lo, hi, channel: param(0) as u8, health: param(3) as f32, dead: false, kind: (param(2) & 0xff) as u8, blast_damage: param(5) as f32 }),
                _ => {}
            }
        }
        props
    }

    fn seg_box(a: V3, b: V3, lo: V3, hi: V3) -> bool {
        // The segment's AABB against the prop's bounds grown by 0.1 — the game's contact registers
        // on the bullet's sphere touching the glist bounds, and the world surface often coincides
        // with the box's near face (the flight dies there first).
        let (g, mut mn, mut mx) = (0.1, [0.0; 3], [0.0; 3]);
        for k in 0..3 {
            mn[k] = a[k].min(b[k]);
            mx[k] = a[k].max(b[k]);
            if mx[k] < lo[k] - g || mn[k] > hi[k] + g { return false; }
        }
        true
    }

    /// The bullet's flight this tick against the props ([M CamSubject_Update 0x1bc668 — class-5
    /// contacts with weapon 0x54 write the channel; Destroy_CollisionHandler 0x12bd48 — every
    /// class-5 contact subtracts its damage, death writes the channel]). Returns the tick's
    /// destroyable smashes that carry a blast (`Destroy_Smash` 0x12bb60's `Explode_Create` for a
    /// non-zero type byte): `(smash position, radius, damage, kind)` — the caller queues the blast
    /// (the game's blast runs inside the smash, on the bullet's owner as the attacker).
    pub fn bullet_hit(&mut self, a: V3, b: V3, weapon: u16, damage: f32, ch: &mut Channels, frame: u32) -> Vec<(V3, f32, f32, u8)> {
        let mut smashes = Vec::new();
        for s in self.subjects.iter_mut().filter(|s| !s.hit) {
            if weapon == 0x54 && Self::seg_box(a, b, s.lo, s.hi) {
                s.hit = true;
                ch.set(s.channel as u32, 1, frame);
            }
        }
        for d in self.destroy.iter_mut().filter(|d| !d.dead) {
            if Self::seg_box(a, b, d.lo, d.hi) {
                d.health -= damage;
                if d.health < 0.0 {
                    d.dead = true;
                    ch.set(d.channel as u32, 1, frame);
                    // `Destroy_Smash` 0x12bb60: the blast only for a non-zero type byte [M
                    // 0x12bc50]. Radius [M 0x12bcdc `lwc1 f12,140(s0)` × the type mult 4/2/3
                    // (0x12bc74/0x12bc84/0x12bc94)] reads the placed obj's +0x8C sphere — a field
                    // the port doesn't parse; the bounds' half-diagonal stands in [*suggested*],
                    // live check in the report. Position: the obj's own `+0x80` [M 0x12bd08 a1 =
                    // s3 = obj+0x80]; the port uses the bounds centre.
                    if d.kind != 0 {
                        let c = [(d.lo[0] + d.hi[0]) / 2.0, (d.lo[1] + d.hi[1]) / 2.0, (d.lo[2] + d.hi[2]) / 2.0];
                        let ext = [(d.hi[0] - d.lo[0]) / 2.0, (d.hi[1] - d.lo[1]) / 2.0, (d.hi[2] - d.lo[2]) / 2.0];
                        let sphere = (ext[0] * ext[0] + ext[1] * ext[1] + ext[2] * ext[2]).sqrt();
                        let mult = match d.kind { 1 => 4.0f32, 2 => 2.0, _ => 3.0 };
                        smashes.push((c, sphere * mult, d.blast_damage, d.kind));
                    }
                }
            }
        }
        smashes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_props() -> Props {
        Props {
            subjects: vec![CamSubject { placement: 0, lo: [4.0, 7.0, -111.0], hi: [6.0, 9.0, -109.0], channel: 0x1c, hit: false }],
            destroy: vec![Destroy { placement: 1, lo: [133.0, 4.0, -270.0], hi: [137.0, 7.0, -268.0], channel: 0x1f, health: 100.0, dead: false,
            kind: 1, blast_damage: 100.0 }],
        }
    }

    #[test]
    fn camera_bullet_writes_the_subject_channel() {
        let mut p = box_props();
        let mut ch = Channels::default();
        // The camera's (weapon 0x54) shot through the 0700000c jetpack display's box.
        p.bullet_hit([5.0, 8.0, -112.0], [5.0, 8.0, -108.0], 0x54, 1.0, &mut ch, 10);
        assert!(ch.on(0x1c), "the photograph writes ch 0x1c [M 0x1bc668:20-22]");
        // A sister: the same shot with a gun does nothing (the weapon byte gates the subject).
        let mut p = box_props();
        let mut ch = Channels::default();
        p.bullet_hit([5.0, 8.0, -112.0], [5.0, 8.0, -108.0], 0x1e, 1.0, &mut ch, 10);
        assert!(!ch.on(0x1c), "a non-camera bullet never photographs the subject");
    }

    #[test]
    fn bullets_whittle_the_door_until_it_writes_its_channel() {
        let mut p = box_props();
        let mut ch = Channels::default();
        // 9 hits of 10 damage leave the 100-health door standing; the 10th lands exactly on 0.0,
        // which the game's `fVar9 < 0.0` does NOT count [M 0x12bd48:46].
        for f in 0..10 {
            p.bullet_hit([135.0, 5.5, -270.5], [135.0, 5.5, -267.5], 0x1e, 10.0, &mut ch, f);
            assert!(!ch.on(0x1f), "hit {f}: the door stands");
            assert!(!p.destroy[0].dead);
        }
        // The 11th crosses below zero → the death channel.
        p.bullet_hit([135.0, 5.5, -270.5], [135.0, 5.5, -267.5], 0x1e, 10.0, &mut ch, 10);
        assert!(ch.on(0x1f), "the death writes ch 0x1f [M 0x12bd48:51-57]");
        assert!(p.destroy[0].dead);
        // A sister: a shot that misses the box does nothing.
        let mut p = box_props();
        let mut ch = Channels::default();
        p.bullet_hit([20.0, 5.5, -270.5], [20.0, 5.5, -267.5], 0x1e, 10.0, &mut ch, 1);
        assert!(!ch.on(0x1f) && !p.destroy[0].dead, "a miss never damages the door");
    }
}
