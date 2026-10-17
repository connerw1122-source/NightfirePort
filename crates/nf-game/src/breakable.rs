//! The breakable scenery: placement 0x20 (`Break_Create` 0x12c230, `Break_Update` 0x12bfd8,
//! `Break_ApplyDamage` 0x12c128, `Break_Kill` 0x12b970).
//!
//! Semantics [M]:
//! - `Break_Create` 0x12c230: class-0xC object, `obj+0xFF = 0x20`; tag+0x00 = placement param 0
//!   (the break kind; 2 = glass), tag+0x04 = param 1 as f32 (the health; 0 dies on the first
//!   bullet), tag+0x08 = param 2 == 0 ? 0 : ~param2 as u16 (the bullet reject mask).
//! - `Break_Update` 0x12bfd8: per tick (obj+0xFE bit 0 clear) the contact list is filtered with
//!   `Collide_FilterBullets(obj+0xD0, filter | 0x106)` [M :15] and walked: a class-0x05 (bullet)
//!   contact subtracts the node's damage float from tag+0x04; a class-0x02 (drone) contact
//!   damages ONLY when `GameState+0x12 == 0x7000001` AND kind == 2 — the damage is
//!   `Vec_SqDist3D(contact+0x30, contact+0x40) · 20.0` (the drone's squared step this tick):
//!   guards walking through the castle's glass smash it. Health < 0 → `Break_Kill`.
//! - `Collide_FilterBullets` 0x1ece30:24-33: a class-5 hitter whose `weapon_definition+0x10`
//!   word ANDs the passed mask non-zero is dropped (disc words: guns 0x0008, Laser 0x0002,
//!   watch 0x0100 [L dump]) — the 0x106 base mask rejects the laser/watch classes even on a pane
//!   with no filter param, while the guns' 0x0008 passes both.
//! - `Break_Kill` 0x12b970: `obj+0xFE |= 1` (dead — the pane registers nothing further and
//!   leaves the world, the collision "goes away" [M :60]); kind == 2 → sound 0x2E and
//!   `ShatterPolyRecurse` per triangle of `psiGetTriList(obj)`; otherwise the sound is 0x5F2
//!   when `Rand_Rand(100) < 0x29` (41%) else 0x5F3 [M :65-76 — the roll is the only RNG draw,
//!   taken only when a non-glass pane dies; a glass kill draws nothing], then
//!   `Drone_BrokenObject` + `Debris_Create_Loop`. NO channel is written anywhere
//!   [M 0x12b970 — no switch_channels store].
//!
//! The census [D, `probe_breakables`]: 443 placements on the disc — 259 kind 2, 171 kind 0,
//! 13 kind 1. The 07000008 control room carries three `Glass_Controlroom*` panes (kind 2,
//! health 0, no filter), 07000006 one `Window_Securityroom` (kind 2, health 0, filter param
//! 0xf8); 07000005 has none.
//!
//! Bullets never stop on a pane: the panes' collision triangles all carry surface byte 0x10
//! (glass) [D census of the pane meshes] and `Bullet_CollisionHandler` 0x12ce80's glass rule —
//! material 0x10 pierces unless `weapon_data+0x6c & 0x800` [M disasm 0x12cf44..0x12cf64, ported
//! for world hits by fix-rocket-glass-08 in crate::effect] — lets the round fly on while the
//! mirrored hit still damages the pane. A shot shatters the pane and flies on; a broken one
//! registers nothing and blocks no body (glass-breach): the static bake excludes the 0x20
//! placements (world.rs) and `Drones` injects each INTACT pane per tick through `set_dynamic`
//! (drone.rs) — `Break_Kill`'s obj+0xFE |= 1 takes the pane out of the world and the guards
//! walk through the opening.
//!
//! Port deviations (documented, none tested as game behaviour):
//! - the shatter/debris visuals and `Drone_BrokenObject`'s pieces are the renderer/props layer's
//!   work (the main agent's render check owns the look); the sim exposes `dead` + the kill
//!   effect.
//! - the contact stand-ins [S — the game contacts the placed glist's triangles; the port's
//!   glist-bounds rule is crate::props's]: the bullet's flight segment vs the bounds grown 0.1;
//!   the class-2 body's movement segment vs the kind-2 bounds grown 0.7 (the port's blocked
//!   standoff is the contact the game resolves); the scripted kill's point vs the bounds grown
//!   0.2 (the game's sphere: centre obj+0x80, radius obj+0x8c).

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
    /// tag+0x00: 2 = glass (shatter + sound 0x2E), other = debris (sound 0x5F2/0x5F3 by the
    /// `Rand_Rand(100) < 0x29` roll [M 0x12b970:65-76]).
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
    /// The placement transform (`object_matrix`) — the draw matrix while alive; the renderer
    /// stops drawing a dead one (the movement handler's delete pass removes the object
    /// [M 0x12b970:41 — obj+0xFE bit 0]).
    pub matrix: [[f32; 4]; 4],
    /// The collision-shape key the World indexes the pane's tris under (the object's hashcode, or
    /// the anonymous `(map, object)` synthetic [S — world.rs's own rule]) — the per-tick
    /// `set_dynamic` injection looks the INTACT pane up by it (glass-breach: the intact pane
    /// blocks bodies, the broken one injects nothing).
    pub shape_key: u32,
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
    /// `parsemap_create_dynamic_objects` case 0x20 [M 0x1d04f0]; no RNG draw at create).
    pub fn create(level: &Level) -> Breaks {
        let Some(m) = level.main_map() else { return Breaks::default() };
        let mut list = Vec::new();
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            if p.flags & 0xff != 0x20 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let (lo, hi) = bounds(level, m, p);
            let p2 = param(2);
            // The collision-shape key: the target object's hashcode, or world.rs's anonymous
            // (map, object) synthetic when it is u32::MAX [S — the shapes index's own rule].
            let (om, oo) = if p.by_hash() { level.by_hash.get(&p.hashcode).copied().unwrap_or((m, p.index as usize)) } else { (m, p.index as usize) };
            let hc = level.maps[om].objects[oo].hashcode;
            let shape_key = if hc == u32::MAX { 0x8000_0000 | ((om as u32) << 16) | oo as u32 } else { hc };
            list.push(Break {
                placement: i,
                lo,
                hi,
                kind: (param(0) & 0xff) as u8,
                health: f32::from_bits(param(1)),
                filter: if p2 == 0 { 0 } else { !(p2 as u16) },
                dead: false,
                matrix: object_matrix(p.rotation, p.translation),
                shape_key,
            });
        }
        Breaks { list }
    }

    fn seg_box(a: V3, b: V3, lo: V3, hi: V3) -> bool {
        // Same rule as crate::props::seg_box: the bullet segment's AABB against the bounds grown
        // by 0.1 (the contact registers on the sphere touching the glist bounds).
        Self::seg_box_grow(a, b, lo, hi, 0.1)
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

    fn centre(br: &Break) -> V3 {
        [(br.lo[0] + br.hi[0]) / 2.0, (br.lo[1] + br.hi[1]) / 2.0, (br.lo[2] + br.hi[2]) / 2.0]
    }

    /// Whether the placement's breakable is dead (`obj+0xFE & 1`) — the draw layer skips it and
    /// nothing registers hits on it any more [M `Break_Kill` 0x12b970:60].
    pub fn broken_at(&self, placement: usize) -> bool {
        self.list.iter().any(|b| b.placement == placement && b.dead)
    }

    /// The bullet flight this tick against the breakables ([M Break_Update 0x12bfd8:36-51 — the
    /// class-5 contacts filtered by `Collide_FilterBullets(list, filter | 0x106)` — a hitter
    /// whose `weapon_definition+0x10` word ANDs the mask is dropped [M 0x1ece30:24-33] — then
    /// each survivor's damage subtracts; death at health < 0 with `Break_Kill`'s sound
    /// [M 0x12b970:60-76]]). `weapon_mask` is the bullet's weapon-definition +0x10 word (guns
    /// 0x0008, Laser 0x0002, watch 0x0100 [L dump]). The roll draws the shared stream ONLY when a
    /// non-glass pane dies — the game's draw count exactly (`Rand_Rand(100)` inside `Break_Kill`'s
    /// kind != 2 branch [M 0x12b970:66-68]); the glass panes (kind 2) never draw. Returns the
    /// kills for presentation.
    pub fn bullet_hit(&mut self, a: V3, b: V3, weapon_mask: u16, damage: f32, rng: &mut crate::drone::Rng) -> Vec<BreakEffect> {
        let mut kills = Vec::new();
        for br in self.list.iter_mut().filter(|b| !b.dead) {
            // `Collide_FilterBullets` [M 0x1ece30:24-33]: the hitter's weapon word against the
            // pane's mask | 0x106 — non-zero AND = the hit is dropped.
            if weapon_mask & (br.filter | 0x106) != 0 { continue; }
            if !Self::seg_box(a, b, br.lo, br.hi) { continue; }
            br.health -= damage;
            if br.health < 0.0 {
                br.dead = true;
                // `Break_Kill` 0x12b970:65-76: kind 2 plays 0x2e with no draw; the others roll
                // `Rand_Rand(100) < 0x29` → 0x5f2, else 0x5f3.
                let sound = if br.kind == 2 {
                    0x2E
                } else if rng.rand(100) < 0x29 {
                    0x5F2
                } else {
                    0x5F3
                };
                kills.push(BreakEffect { placement: br.placement, pos: Self::centre(br), kind: br.kind, sound });
            }
        }
        kills
    }

    /// The drone-contact arm ([M Break_Update 0x12bfd8:23-34 — only on GameState+0x12 ==
    /// 0x7000001, only kind == 2, damage = the drone's squared step × 20]). `level_01` is the
    /// caller's `GameState+0x12 == 0x07000001` check; (`drone_from`, `drone_to`) is the game's
    /// (+0x40, +0x30) pair — the movement pass copies +0x30 → +0x40 after each move
    /// [M 0x1336f8:212]. The port's contact is the movement segment against the bounds grown 0.7
    /// [*suggested* — the blocked standoff IS the contact the game resolves]. Kind-2 kills play
    /// 0x2e with NO RNG draw (the shared stream is untouched by glass kills [M 0x12b970:65-76]).
    pub fn drone_step(&mut self, drone_from: V3, drone_to: V3, level_01: bool) -> Vec<BreakEffect> {
        let mut kills = Vec::new();
        if !level_01 { return kills; }
        let sq = (drone_to[0] - drone_from[0]).powi(2)
            + (drone_to[1] - drone_from[1]).powi(2)
            + (drone_to[2] - drone_from[2]).powi(2);
        if sq == 0.0 { return kills; }
        let dmg = sq * 20.0;
        for br in self.list.iter_mut().filter(|b| !b.dead && b.kind == 2) {
            if !Self::seg_box_grow(drone_from, drone_to, br.lo, br.hi, 0.7) { continue; }
            br.health -= dmg;
            if br.health < 0.0 {
                br.dead = true;
                kills.push(BreakEffect { placement: br.placement, pos: Self::centre(br), kind: br.kind, sound: 0x2E });
            }
        }
        kills
    }

    /// `Break_DoBreak` 0x12c168 (NIS event 0x0a, the scripted kill): walk the breakables in
    /// placement order — the game walks the object list [M :39-51] — and kill the FIRST intact
    /// one whose world sphere contains `pos` (the game's sphere: centre `obj+0x80`, radius
    /// `obj+0x8c`; the port's stand-in is the bounds grown 0.2 [*suggested* — the glist
    /// world-sphere radius for a flat pane sits between the thin axis and the long axis]).
    /// The kill is the standard `Break_Kill` (obj+0xFE |= 1, the sound) [M :61; 0x12b970:60-76].
    pub fn do_break(&mut self, pos: V3, rng: &mut crate::drone::Rng) -> Vec<BreakEffect> {
        let mut kills = Vec::new();
        let Some(i) = self.list.iter().position(|b| !b.dead && Self::seg_box_grow(pos, pos, b.lo, b.hi, 0.2)) else { return kills };
        let br = &mut self.list[i];
        br.dead = true;
        let sound = if br.kind == 2 {
            0x2E
        } else if rng.rand(100) < 0x29 {
            0x5F2
        } else {
            0x5F3
        };
        kills.push(BreakEffect { placement: br.placement, pos: Self::centre(br), kind: br.kind, sound });
        kills
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(kind: u8, health: f32) -> Breaks {
        Breaks { list: vec![Break { placement: 0, lo: [4.0, 7.0, -3.0], hi: [6.0, 9.0, -1.0], kind, health, filter: 0, dead: false, matrix: [[0.0; 4]; 4], shape_key: 0 }] }
    }

    #[test]
    fn the_first_bullet_kills_a_zero_health_vase_with_the_debris_roll() {
        // The 07000001 vases carry params [(2,0)] only — kind 0, health 0.0 [D placements dump].
        // Break_Kill rolls Rand_Rand(100) < 0x29 → 0x5f2 else 0x5f3 [M 0x12b970:65-76] — the old
        // unconditional-0x5F3 pin was the unported-[S] stopgap and contradicted the decompile.
        let mut b = one(0, 0.0);
        let mut rng = crate::drone::Rng::new(1);
        // A pistol round (weapon word 0x0008 [L dump] — 0x0008 & 0x106 == 0 passes the base mask).
        let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x0008, 25.0, &mut rng);
        assert!(b.list[0].dead, "health 0 - damage < 0 → Break_Kill [M 0x12bfd8:44-50]");
        assert_eq!(kills.len(), 1);
        assert_eq!(kills[0].kind, 0);
        assert!(kills[0].sound == 0x5F2 || kills[0].sound == 0x5F3,
            "the kill rolled the debris sound [M 0x12b970:66-68], got {:#x}", kills[0].sound);
    }

    #[test]
    fn glass_shatters_with_sound_0x2e_and_the_sister_vase_survives_a_miss() {
        // radioroom_glass/comroom_glass carry params [(0,2),(1,0),(2,0)] — kind 2 [D placements].
        let mut b = one(2, 0.0);
        let mut rng = crate::drone::Rng::new(1);
        let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x0008, 25.0, &mut rng);
        assert_eq!(kills[0].sound, 0x2E, "glass → sound 0x2E [M 0x12b970: uVar5 = 0x2e]");
        // Sister: a shot wide of the box never touches the health.
        let mut b = one(2, 0.0);
        let kills = b.bullet_hit([50.0, 8.0, -4.0], [50.0, 8.0, -2.0], 0x0008, 25.0, &mut rng);
        assert!(kills.is_empty() && !b.list[0].dead, "a miss never damages the breakable");
    }

    #[test]
    fn a_health_pool_takes_repeated_hits_and_only_dies_below_zero() {
        // Destroy_test's rule: exactly 0.0 is NOT a kill [M 0x12bfd8:50 `fVar5 < 0.0`].
        let mut b = one(0, 50.0);
        let mut rng = crate::drone::Rng::new(1);
        for i in 0..5 {
            let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x0008, 10.0, &mut rng);
            assert!(kills.is_empty() && !b.list[0].dead, "hit {}: {} left", i, 50 - 10 * (i + 1));
        }
        let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x0008, 10.0, &mut rng);
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

    #[test]
    fn the_laser_and_watch_words_are_rejected_by_the_base_mask_and_the_gun_passes() {
        // Collide_FilterBullets is called with `filter | 0x106` even on a no-filter pane [M
        // 0x12bfd8:15]; a hitter whose weapon word ANDs it is dropped [M 0x1ece30:24-33]. The
        // disc's words: guns 0x0008 (passes — 0x0008 & 0x106 == 0), Laser 0x0002 and watch 0x0100
        // (both rejected) [L dump].
        for (word, breaks) in [(0x0002u16, false), (0x0100, false), (0x0008, true)] {
            let mut b = one(2, 0.0);
            let mut rng = crate::drone::Rng::new(1);
            let kills = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], word, 25.0, &mut rng);
            assert_eq!(kills.is_empty(), !breaks, "word {word:#x}: the base 0x106 mask filters it [M 0x1ece30:24-33]");
            assert_eq!(b.list[0].dead, breaks);
        }
    }

    #[test]
    fn the_panes_filter_param_rejects_matching_weapon_words() {
        // A pane with filter param 2 = 0xf8 (the 07000006 Window_Securityroom [D placements])
        // stores ~0xf8 = 0xff07; a weapon word ANDing filter | 0x106 is dropped [M
        // 0x1ece30:24-33]. Word 0x0008: 0x0008 & (0xff07 | 0x106) == 0 → passes; word 0x0007
        // (bits 0-2 sit in the mask) is dropped.
        let mut b = Breaks { list: vec![Break { placement: 0, lo: [0.0, 0.0, 0.0], hi: [2.0, 2.0, 2.0], kind: 2, health: 0.0, filter: 0xff07, dead: false, matrix: [[0.0; 4]; 4], shape_key: 0 }] };
        let mut rng = crate::drone::Rng::new(1);
        let kills = b.bullet_hit([1.0, 1.0, -1.0], [1.0, 1.0, 3.0], 0x0008, 10.0, &mut rng);
        assert_eq!(kills.len(), 1, "the gun's word passes the 0xf8-derived mask");
        let mut b2 = Breaks { list: vec![Break { placement: 0, lo: [0.0, 0.0, 0.0], hi: [2.0, 2.0, 2.0], kind: 2, health: 0.0, filter: 0xff07, dead: false, matrix: [[0.0; 4]; 4], shape_key: 0 }] };
        let kills2 = b2.bullet_hit([1.0, 1.0, -1.0], [1.0, 1.0, 3.0], 0x0007, 10.0, &mut rng);
        assert!(kills2.is_empty() && !b2.list[0].dead, "a word inside the mask never damages the pane [M 0x1ece30:24-33]");
    }

    #[test]
    fn a_broken_pane_registers_nothing_but_the_next_one_still_breaks() {
        // 07000008's control-room panes: kind 2, health 0, no filter [D placements] — two flat
        // planes (123 the x = 105.3 pane, 128 the z = −122 pane).
        let pane = |pl: usize, lo: V3, hi: V3| Break { placement: pl, lo, hi, kind: 2, health: 0.0, filter: 0, dead: false, matrix: [[0.0; 4]; 4], shape_key: 0 };
        let mut b = Breaks { list: vec![
            pane(123, [105.3, 34.2, -122.0], [105.3, 36.4, -120.1]),
            pane(128, [105.3, 34.2, -122.0], [109.2, 36.4, -122.0]),
        ] };
        let mut rng = crate::drone::Rng::new(1);
        // The round runs along z at x = 105.3, crossing only pane 123's plane.
        let kills = b.bullet_hit([105.3, 35.0, -121.5], [105.3, 35.0, -120.0], 0x0008, 10.0, &mut rng);
        assert_eq!(kills.len(), 1, "the first round shatters it [M 0x12bfd8:49-51]");
        assert!(b.broken_at(123), "obj+0xFE & 1 is set [M 0x12b970:60]");
        // The second round through the SAME spot: the dead pane registers no hit [M 0x12b970:60].
        let kills = b.bullet_hit([105.3, 35.0, -121.5], [105.3, 35.0, -120.0], 0x0008, 10.0, &mut rng);
        assert!(kills.is_empty(), "a broken pane is gone from the world");
        // The sister: the OTHER intact pane still breaks — the system is alive.
        let kills = b.bullet_hit([107.0, 35.0, -123.0], [107.0, 35.0, -121.0], 0x0008, 10.0, &mut rng);
        assert_eq!(kills.len(), 1, "the neighbouring pane still shatters [D 07000008 has 3 panes]");
        assert!(b.broken_at(128));
    }

    #[test]
    fn a_drone_step_drains_sqdist_times_twenty_and_kills_the_glass() {
        // 07000001's pane 136 bounds [D nf_view bounds dump].
        let pane = |pl: usize, kind: u8, health: f32| Break { placement: pl, lo: [-1.91, 5.34, -1.167], hi: [-0.09, 7.95, -1.167], kind, health, filter: 0, dead: false, matrix: [[0.0; 4]; 4], shape_key: 0 };
        let mut b = Breaks { list: vec![pane(136, 2, 0.0)] };
        // The level gate: the class-2 arm exists on 0x0700001 ALONE [M 0x12bfd8:24].
        let ev0 = b.drone_step([-1.0, 6.3, -0.5], [-1.0, 6.3, -1.6], false);
        assert!(ev0.is_empty() && !b.broken_at(136), "another level never smashes by contact [M 0x12bfd8:24]");
        // A motionless brush drains nothing [M :27 — SqDist(pos, prev) = 0].
        let ev1 = b.drone_step([-1.0, 6.3, -1.0], [-1.0, 6.3, -1.0], true);
        assert!(ev1.is_empty() && !b.broken_at(136), "a stationary contact survives the health-0 pane");
        // The tick's movement through the pane plane kills it [M :26-27, :49-51].
        let ev2 = b.drone_step([-1.0, 6.3, -0.5], [-1.0, 6.3, -1.6], true);
        assert_eq!(ev2.len(), 1, "the crossing shatters the health-0 pane");
        assert_eq!(ev2[0].sound, 0x2e, "kind 2 plays 0x2e, no roll [M 0x12b970:65-70]");
        assert_eq!(ev2[0].kind, 2);
        assert!(b.broken_at(136), "obj+0xFE |= 1 [M 0x12b970:60]");
        // Sister: a NON-glass kind is immune to the class-2 arm (the kind gate [M :25]).
        let mut b2 = Breaks { list: vec![pane(5, 0, 0.0)] };
        let ev3 = b2.drone_step([-1.0, 6.3, -0.5], [-1.0, 6.3, -1.6], true);
        assert!(ev3.is_empty() && !b2.broken_at(5), "the smash arm is kind-2 only [M 0x12bfd8:25]");
        // Sister: the drain is exactly SqDist·20 — a 0.3 step (d2 0.09 → 1.8) dents a 100 pool.
        let mut b3 = Breaks { list: vec![pane(6, 2, 100.0)] };
        let ev4 = b3.drone_step([-1.0, 6.3, -0.5], [-1.0, 6.3, -0.8], true);
        assert!(ev4.is_empty() && !b3.broken_at(6));
        assert!((b3.list[0].health - 98.2).abs() < 1e-4, "health -= SqDist·20 [M 0x12bfd8:27], got {}", b3.list[0].health);
        // The RNG stream is untouched by a glass kill (0x2e is unconditional [M 0x12b970:65-70]).
        let mut rng_a = crate::drone::Rng::new(0x5eed);
        let _ = rng_a.rand(100);
        let mut b4 = Breaks { list: vec![pane(7, 2, 0.0)] };
        let _ = b4.drone_step([-1.0, 6.3, -0.5], [-1.0, 6.3, -1.6], true);
        let mut rng_b = crate::drone::Rng::new(0x5eed);
        let _ = rng_b.rand(100);
        let bb = rng_b.rand(100);
        let mut rng_c = crate::drone::Rng::new(0x5eed);
        let _ = rng_c.rand(100);
        // (no draw here — the glass kill is unconditional) — then the same next draw.
        let cc = rng_c.rand(100);
        assert_eq!(bb, cc, "the glass kill drew nothing between the two rolls");
    }

    #[test]
    fn do_break_kills_the_first_intact_pane_containing_the_point() {
        // The game walks the object list and kills the FIRST class-0x20 sphere containing the
        // point [M Break_DoBreak 0x12c168:39-51,61]; the port's order is placement order.
        let pane = |pl: usize, lo: V3, hi: V3| Break { placement: pl, lo, hi: [hi[0], hi[1], hi[2]], kind: 2, health: 0.0, filter: 0, dead: false, matrix: [[0.0; 4]; 4], shape_key: 0 };
        let mut b = Breaks { list: vec![
            pane(10, [-1.91, 5.34, -1.167], [-0.09, 7.95, -1.167]),
            pane(11, [5.0, 5.34, -1.167], [6.8, 7.95, -1.167]),
        ] };
        let mut rng = crate::drone::Rng::new(7);
        // The point sits in pane 11 only (pane 10's grow 0.2 reaches x −0.09+0.2 = 0.11 < 5).
        let ev = b.do_break([5.9, 6.3, -1.167], &mut rng);
        assert_eq!(ev.len(), 1, "the scripted kill fires [M 0x12c168:39-61]");
        assert!(b.broken_at(11) && !b.broken_at(10), "the containing pane died, the far one lives");
        assert_eq!(ev[0].sound, 0x2e);
        // Sister: no pane at the point → nothing dies.
        let mut b2 = Breaks { list: vec![pane(10, [-1.91, 5.34, -1.167], [-0.09, 7.95, -1.167])] };
        let ev2 = b2.do_break([100.0, 6.3, -1.167], &mut rng);
        assert!(ev2.is_empty() && !b2.broken_at(10), "a point outside every sphere kills nothing");
        // Sister: a BROKEN pane is skipped (the first INTACT one [M :45 — the FE&1 gate]).
        let mut broken = pane(12, [-1.91, 5.34, -1.167], [-0.09, 7.95, -1.167]);
        broken.dead = true;
        let mut b3 = Breaks { list: vec![broken, pane(13, [-1.91, 5.34, -1.167], [-0.09, 7.95, -1.167])] };
        let ev3 = b3.do_break([-1.0, 6.3, -1.167], &mut rng);
        assert!(b3.broken_at(13) && ev3.len() == 1, "the kill skips the already-broken pane");
    }

    #[test]
    fn pooled_health_takes_more_than_one_round_and_the_roll_picks_the_sound() {
        // The kind-0 breakable: 30 health. The game draws Rand_Rand(100) at the kill [M
        // 0x12b970:66-68] — the shared stream must see exactly that draw count.
        let mut b = one(0, 30.0);
        b.list[0].filter = 0xfff8; // filter | 0x106 = 0xfffe — only bit 0 passes; the word below is 0x0001.
        let mut rng = crate::drone::Rng::new(0x5eed);
        for _ in 0..2 {
            let ev = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x0001, 10.0, &mut rng);
            assert!(ev.is_empty(), "30 health survives 10-damage rounds (health < 0 kills [M 0x12bfd8:49])");
        }
        // The 3rd lands exactly on 0.0, which the game's `< 0.0` does NOT count [M 0x12bfd8:49;
        // the same rule props.rs pins for Destroy_CollisionHandler 0x12bd48:46].
        let ev = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x0001, 10.0, &mut rng);
        assert!(ev.is_empty(), "health exactly 0.0 survives");
        let ev = b.bullet_hit([5.0, 8.0, -4.0], [5.0, 8.0, -2.0], 0x0001, 10.0, &mut rng);
        assert_eq!(ev.len(), 1);
        assert!(ev[0].sound == 0x5f2 || ev[0].sound == 0x5f3, "the roll picks 0x5f2/0x5f3 [M 0x12b970:65-75]");
        assert_eq!(ev[0].kind, 0);
    }
}
