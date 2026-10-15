//! The ladder (`Ladder_Create` 0x1966c0, placement type 0x22, class 0x2D).
//!
//! `Ladder_Create` [M disasm 0x1966c0-0x19675c] calls `control_create_object(4, pos, rot, 0)` — instance size
//! 4 bytes: `inst+0x00 = 0` (`sh zero,0(v0)`), `inst+0x02 = *(u16*)(level_tag+0x30)` = placement param 1
//! (`lhu v1,48(s1)` / `sh v1,2(v0)`); `obj+0xFF = 0x2D` (`addiu a2,zero,45` / `sb a2,255(s0)`);
//! `obj+0xD8 = celglist`; `build_LinkToRoom(obj,0,glb_world)`; `Controls_StraddleTest(obj)`.
//! The rotation quaternion is passed through **unmodified** — the wire atlas's "rot.z is negated … as for
//! ladders" side note is wrong for ladders (no negation in `Ladder_Create` or its dispatch case [M
//! 0x1d04f0 case 0x22]). No param is ever read back by any ladder code (only the wire's channel pattern
//! matches this instance layout), so the port keeps `channel` for completeness.
//!
//! The runtime geometry comes from the object's world sphere (`Control_BuildWorldSph` 0x133070 pattern, the
//! same computation `Bond::sphere_centre` ports): `world = local_centre` (the glist bounds centre,
//! `MapObject.centre`) through the object matrix + translation, `radius = MapObject.radius * |scale|`
//! [M centre/radius = glist+0x20/+0x30 per docs/formats/level.rs; the ladder sphere's +0x84 (centre y) and
//! +0x8C (radius) are what `Player_CollLadder` 0x196118 and `Player_Climb` 0x196310 clamp against].
//!
//! Spec: docs/atlas/ladder.md (this session), docs/systems/player.md.

use crate::world::V3;
use nf_formats::level::{Level, MapObject};
use nf_formats::placement::Placement;

/// A ladder object (class 0x2D, `Ladder_Create` 0x1966c0).
#[derive(Clone, Debug)]
pub struct Ladder {
    /// `obj+0x80..0x8C`: the world-sphere centre (`Control_BuildWorldSph` 0x133070).
    pub centre: V3,
    /// `obj+0x8C`: the world-sphere radius.
    pub radius: f32,
    /// `obj+0x54`: the yaw from the object matrix's forward row (row 2 = (sin y, ·, cos y)).
    pub yaw: f32,
    /// `inst+0x02` = placement param 1 (`level_tag+0x30`) [M 0x196710-0x196714]; no reader found.
    pub channel: u16,
}

impl Ladder {
    /// `Ladder_Create` (0x1966c0) from a placement (type 0x22) plus the placement's object bounds.
    /// The yaw convention matches `Bond::look` (0 faces +Z, forward = (sin, ·, cos)); the object matrix's
    /// yaw-only rotation carries the glist bounds centre into the world.
    pub fn create(p: &Placement, obj: Option<&MapObject>) -> Option<Ladder> {
        let o = obj?;
        let yaw = p.rotation[1];
        let (s, c) = yaw.sin_cos();
        // row-vector matrix rows: row0 = (cos,0,−sin), row2 = (sin,0,cos) [M placement::object_matrix].
        let centre = [
            p.translation[0] + o.centre[0] * c + o.centre[2] * s,
            p.translation[1] + o.centre[1],
            p.translation[2] - o.centre[0] * s + o.centre[2] * c,
        ];
        let scale = p.scale();
        let s_max = scale.iter().fold(1.0f32, |a, &v| a * v.max(1e-6).abs());
        Some(Ladder { centre, radius: o.radius * s_max, yaw, channel: param1(p) })
    }

    /// `BLData+0x8A0` = `centre.y + radius + 0.85` — the top dismount threshold [M Player_CollLadder
    /// 0x196118 (`+0x84 + +0x8C + 0.85`), recomputed per tick in `Player_Climb` 0x196310].
    pub fn top(&self) -> f32 { self.centre[1] + self.radius + 0.85 }
    /// `BLData+0x8A4` = `centre.y − radius + 1.0` — the bottom dismount threshold [M same functions].
    pub fn bottom(&self) -> f32 { self.centre[1] - self.radius + 1.0 }

    /// The WALK form of the grab pick (`Player_Collision` 0x194df0's capsule switch case 0/6 [M]).
    /// Two stages:
    ///
    /// Stage 1 — the game's pick sphere. `Collide_PickObj` 0x1eae80:152-168 [M] tests the player's
    /// HITTEST capsule segment against the object's world sphere inflated ×1.5 plus the capsule
    /// radius (mode word BLData+0x7D4 = 0x800). The walk arm's HITTEST [M 0x194df0 case 0/6]:
    /// radius `+0x7CC = 0x3e8ccccd` (0.275 — the caller passes `player::WALK_PICK_RADIUS`), segment
    /// A = pos + up·0.55 (0x3f0ccccd) and B = pos + up·(+0x7CC − cc) — the game reads the B offset's
    /// base FROM the same radius slot it just wrote, so `b = pos + (capsule_radius − cc)` here.
    /// With the stand cc 1.0347 the segment spans 0.55..−0.76. This is what lets a DEEP walker
    /// below the bounds centre trigger the grab — `Player_CollLadder` 0x196118:23-31 then lifts
    /// pos.y to `bottom + 0.02`, the climb-out assist.
    ///
    /// Stage 2 — the climb-plane bound. The game's pick reach is narrowed past the sphere by the
    /// straddled-cel scoping (`Collide_Pick` 0x1eb320:125-134/318-328 tests only objects linked to the
    /// cels the player straddles) and the glist mesh refine (`Collide_Intersect` 0x1ebaa8, run per
    /// candidate in `Collide_Update` 0x1ea398:84-93, not ported): a tall ladder's bounds sphere is its
    /// half-HEIGHT (r 5.75 on the 0700000c yard ladder, y span 0.8..12.1 [D probes]), so the bare ×1.5
    /// sphere reaches ~9 units sideways, and a ladder's pickable body is its thin rung plane through
    /// the axis plus the rails' lateral span. The port models the refine as the frontal/lateral
    /// decomposition of the offset from the axis ([S] — the stand-in for cel scoping + mesh refine,
    /// replacing the pre-109i bare radial column `min(radius,1)+capsule`): `|frontal| ≤ capsule_radius`
    /// (the capsule reaches the rung plane and no farther) and `|lateral| ≤ 0.4 + capsule_radius`
    /// (the rails' span ±~0.4 of the axis). With the WALK arm's own 0.275 the walk grab fires only
    /// when the walk capsule touches the rung plane — a walker brushing past, a stander 0.5+ off the
    /// plane or a just-dismounted dry stander at the dismount offset 0.5 never re-grabs, as in the
    /// game (fix-ladder-water2; the pre-109i2 port passed the swim arm's 0.55 here and re-grabbed a
    /// dry stander at 0.5). The SWIM arm keeps 0.55 [M 0x194df0 case 3]: that bound is the
    /// dismount-into-water escape — the dismount leaves Bond ~0.5 off the plane facing the ladder,
    /// the swim entry's 5-tick regrab [M Player_InWater 0x195e50] buys ~0.2 of drift at the
    /// `Player_Move` 0.2/tick ramp [M 0x191dd0:108-117], and 0.5+0.2 > 0.55 clears the plane reach
    /// while the old bare `min(radius,1)+capsule` column (1.55 on the hut ladder) re-grabbed him every
    /// cycle — the user's "infinitely latching onto the ladder", playtest 1014-1725. The yaw window in
    /// `Player_CollLadder` 0x196118:20 gates the sides (a walker brushing past faces across the
    /// ladder, |diff| = π/2, no grab).
    pub fn capsule_hits(&self, pos: V3, capsule_radius: f32, cc: f32) -> bool {
        self.pick_hit(pos, 0.55, capsule_radius - cc, capsule_radius)
    }

    /// The DEFAULT arm of the pick (`Player_Collision` 0x194df0's capsule switch default case [M],
    /// which covers the crouch substate 4 and the other non-walk/non-swim substates that reach
    /// `Player_CollLadder` through the walk tick): radius `+0x7CC = 0x3f0ccccd` (0.55), segment
    /// A = pos + up·0.275 (0x3e8ccccd), B = pos + up·(0.55 − cc). Stage 1/stage 2 as in
    /// `capsule_hits`. (fix-ladder-water2: this arm was implicit before — the port fed every
    /// non-swim substate one shape.)
    pub fn capsule_hits_default(&self, pos: V3, capsule_radius: f32, cc: f32) -> bool {
        self.pick_hit(pos, 0.275, 0.55 - cc, capsule_radius)
    }

    /// The two pick stages over one arm's segment: stage 1 — the object's world sphere inflated
    /// ×1.5 plus the capsule radius against the closest point on the segment A..B
    /// [`Collide_PickObj` 0x1eae80:152-168, mode word BLData+0x7D4 = 0x800]; stage 2 — the
    /// mesh-scale column [`in_mesh_column`, the stand-in for the glist refine
    /// `Collide_Update` 0x1ea398:84-93 → `Collide_Intersect` 0x1ebaa8].
    fn pick_hit(&self, pos: V3, a_off: f32, b_off: f32, capsule_radius: f32) -> bool {
        let a = [pos[0], pos[1] + a_off, pos[2]];
        let b = [pos[0], pos[1] + b_off, pos[2]];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ap = [self.centre[0] - a[0], self.centre[1] - a[1], self.centre[2] - a[2]];
        let ab2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
        let t = if ab2 < 1e-9 { 0.0 } else { (ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / ab2 };
        let t = t.clamp(0.0, 1.0);
        let cx = a[0] + ab[0] * t - self.centre[0];
        let cy = a[1] + ab[1] * t - self.centre[1];
        let cz = a[2] + ab[2] * t - self.centre[2];
        if (cx * cx + cy * cy + cz * cz).sqrt() > self.radius * 1.5 + capsule_radius { return false; }
        self.in_mesh_column(pos, capsule_radius)
    }

    /// The SWIM form of the same pick (`Player_Collision` 0x194df0's substate-3 arm builds a different
    /// HITTEST [M]: capsule radius +0x7CC = 0.55 and the segment A = pos + up·(0.1 − (cc − 0.55)),
    /// B = pos + up·(0.45 − cc) — a short LOW capsule ~0.48 under the root at the stand height, unlike
    /// the walk arm's 0.55..−0.76). Stage 1 is the same ×1.5 sphere test; stage 2 is the same mesh-
    /// scale column (`in_mesh_column`). The low segment is what bounds the grab by DEPTH: the game's
    /// swim pick reaches ~0.48 below the root, so a bed-hugging root too far under the bounds centre
    /// misses the ×1.5 sphere — the game grabs swimmers in the water column, not off the pond bed.
    pub fn capsule_hits_swim(&self, pos: V3, capsule_radius: f32, cc: f32) -> bool {
        self.pick_hit(pos, 0.1 - (cc - 0.55), 0.45 - cc, capsule_radius)
    }

    /// Stage 2, shared by every pick form: the offset from the ladder axis decomposed along the
    /// ladder's facing (`fwd(yaw) = (sin, ·, cos)`, the same convention as the grab snap
    /// `centre + fwd·0.5` [M 0x196118:36-44]) — `|frontal| ≤ capsule_radius` (the rung plane: the
    /// form's own capsule radius — the walk arm 0.275 [M 0x194df0 case 0/6], the swim/default arms
    /// 0.55 [M case 3 / default]) and `|lateral| ≤ 0.4 + capsule_radius` (the rails' span). See
    /// `capsule_hits`'s doc.
    fn in_mesh_column(&self, pos: V3, capsule_radius: f32) -> bool {
        let dx = pos[0] - self.centre[0];
        let dz = pos[2] - self.centre[2];
        let (s, c) = self.yaw.sin_cos();
        let frontal = dx * s + dz * c;
        let lateral = dx * c - dz * s;
        frontal.abs() <= capsule_radius && lateral.abs() <= 0.4 + capsule_radius
    }
}

/// Placement param 1: `params.iter().find(|(i, _)| *i == 1)` [M the params are (index, value) pairs,
/// `level_tag+0x2C+4k` addressed by INDEX (docs/systems/levels.md §2 pickup note); `Ladder_Create` reads
/// the word at `level_tag+0x30` = param 1].
fn param1(p: &Placement) -> u16 { p.params.iter().find(|(i, _)| *i == 1).map_or(0, |&(_, v)| v as u16) }

/// Every ladder placement of a level (`flags & 0xff == 0x22`), resolved against its map object.
/// Mirrors the wire sweep in nf-view's `Scene::build` [M 0x1d04f0 case 0x22 → Ladder_Create].
pub fn all(level: &Level) -> Vec<Ladder> {
    let main = level.main_map();
    let mut out = Vec::new();
    for (mi, m) in level.maps.iter().enumerate() {
        for p in &m.placements {
            if p.flags & 0xff != 0x22 { continue; }
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < m.objects.len() { Some((mi, p.index as usize)) } else { None };
            let obj = obj.and_then(|(om, oi)| level.maps.get(om).and_then(|map| map.objects.get(oi)));
            if let Some(l) = Ladder::create(p, obj) { out.push(l); }
        }
    }
    let _ = main;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder(yaw: f32) -> Ladder {
        Ladder { centre: [10.0, 3.0, 20.0], radius: 1.5, yaw, channel: 0 }
    }

    #[test]
    fn thresholds_match_the_decompile_constants() {
        let l = ladder(0.0);
        // top = 3.0 + 1.5 + 0.85 [M 0x196118]; bottom = 3.0 − 1.5 + 1.0.
        assert!((l.top() - 5.35).abs() < 1e-6, "{}", l.top());
        assert!((l.bottom() - 2.5).abs() < 1e-6, "{}", l.bottom());
        // Sister: a different radius moves both thresholds.
        let l2 = Ladder { radius: 2.5, ..ladder(0.0) };
        assert!((l2.top() - l.top()).abs() > 0.5 && (l2.bottom() - l.bottom()).abs() > 0.5);
    }

    #[test]
    fn grab_zone_is_the_game_pick_shape() {
        let l = ladder(0.0);
        let cc = 1.0347f32;
        let wr = crate::player::WALK_PICK_RADIUS; // 0.275 [M Player_Collision 0x194df0 case 0/6]
        // On the rung plane at mid height: the walk capsule (r 0.275) touches it — hit (stage 1 dist
        // ≈ 0.6 ≤ 1.5·1.5 + 0.275; stage 2 frontal 0.2 ≤ 0.275).
        assert!(l.capsule_hits([10.0, 3.0, 20.2], wr, cc));
        // Sister: 0.4 off the plane is past the walk capsule's reach (0.4 > 0.275) — the game's
        // walk HITTEST refine only fires on capsule-vs-rung contact [M 0x194df0 case 0/6 +
        // Collide_Intersect 0x1ebaa8], so a walker parked 0.4 off the plane does not grab.
        assert!(!l.capsule_hits([10.0, 3.0, 20.4], wr, cc));
        // 2 units to the side: the bare stage-1 sphere reaches (2.0 ≤ 2.775) [M 0x1eae80:152-168], but the
        // game narrows the reach past the sphere (the straddled-cel scoping Collide_Pick 0x1eb320:125-134
        // + the glist mesh refine Collide_Intersect 0x1ebaa8 — both unported); the port's stage-2
        // axis-column bound stands in: lateral 2.0 > 0.4 + 0.275 → no hit. (The bare stage 1 here is what
        // re-grabbed Bond off the 0700000c ladder top — the 102h regression.)
        assert!(!l.capsule_hits([12.0, 3.0, 20.0], wr, cc));
        // 4 units to the side: beyond both stages: no hit.
        assert!(!l.capsule_hits([14.0, 3.0, 20.0], wr, cc));
        // DEEP below the sphere bottom (the climb-out assist): the walk segment (down to
        // pos + 0.275 − cc = pos − 0.76 [M case 0/6 reads the B base from the radius slot]) against
        // the ×1.5 sphere reaches — the grab y-snap then lifts him to bottom + 0.02 [M 0x196118:23-31].
        assert!(l.capsule_hits([10.0, 0.9, 20.0], wr, cc));
        // Sister: a shallow radius ladder stops reaching deep (the pick tracks the sphere).
        let thin = Ladder { radius: 0.6, ..ladder(0.0) };
        assert!(!thin.capsule_hits([10.0, 0.9, 20.0], wr, cc), "the inflated pick reached a thin ladder's bed depth");
        // Sister (the swim2 regression, the 0700000c yard ladder's real geometry — y span 0.8..12.1
        // [D probes] → cy 5.55, r 5.75): the top-exit dismount leaves Bond ~2.9 from the axis at
        // walkway height when regrab expires. The bare stage-1 sphere HITS him there (dist 6.4 ≤ 9.03)
        // and yanked him back into the climb; the stage-2 column bound rejects.
        let yard = Ladder { centre: [9.7, 5.55, -187.3], radius: 5.75, yaw: 0.0, channel: 0 };
        // Walking INTO the ladder at walkway height (crossing the rung plane) still grabs — the
        // walk capsule's own reach [M 0x194df0 case 0/6].
        assert!(yard.capsule_hits([9.7, 11.736, -187.1], wr, cc), "the walk-on grab at the walkway height must still reach");
        // The just-dismounted spot (frontal 0.372, lateral 0.249): past the walk capsule's plane
        // reach (0.372 > 0.275) — the game's walk HITTEST does not touch the rungs there either, so
        // no re-grab (fix-ladder-water2; the pre-109i2 0.55 stand-in grabbed this spot).
        assert!(!yard.capsule_hits([9.949, 11.736, -186.928], wr, cc),
            "the just-dismounted stander 0.372 off the plane re-grabbed — the walk arm's reach is the walk capsule's 0.275");
        assert!(!yard.capsule_hits([9.285, 11.736, -190.167], wr, cc),
            "the dismounted walker 2.9 from the axis re-grabbed (the 102h limit cycle)");
    }

    #[test]
    fn swim_pick_is_the_game_swim_capsule_with_the_mesh_scale_reach() {
        let l = ladder(0.0); // centre (10, 3, 20), r 1.5 → bottom 2.5, top 5.35
        let cc = 1.0347f32; // the stand root height [M Player_Collision 0x194df0 cc = AnimObj+0xCC]
        // The swim segment [M 0x194df0 case 3]: A = pos + 0.1 − (cc − 0.55) = pos − 0.3847,
        // B = pos + 0.45 − cc = pos − 0.5847. On the axis at the grab-snap distance (0.5, the game's
        // own snap offset 0x196118:36-44): stage 1 (dist ≈ 1.17 ≤ 1.5·1.5 + 0.55) and stage 2
        // (0.5 ≤ 0.55) both hit — the pond-bed swimmer still grabs the ladder he swims INTO.
        assert!(l.capsule_hits_swim([10.0, 3.0, 20.5], 0.55, cc));
        // 0.62 off the plane: past the mesh-scale stage 2 (0.55) → no hit. This is the dismount
        // escape: the back-stick drift (0.2/tick ramp [M 0x191dd0], ~0.2 over the swim entry's
        // 5-tick regrab [M 0x195e50]) clears 0.5 + 0.2 > 0.55 and the cycle never re-latches.
        assert!(!l.capsule_hits_swim([10.0, 3.0, 20.62], 0.55, cc));
        // Sister: the WALK form at the same 0.62 is past its plane reach too — the walk arm's own
        // radius 0.275 [M 0x194df0 case 0/6] bounds it (fix-ladder-water2).
        assert!(!l.capsule_hits([10.0, 3.0, 20.62], crate::player::WALK_PICK_RADIUS, cc));
        // Sister: at 0.5 (the grab snap's own offset [M 0x196118:36-44]) the SWIM form still reaches
        // (the pond grab), but the WALK form must NOT — the game's walk capsule (r 0.275) parked at
        // the dismount offset 0.5 does not touch the rungs, so a dry stander there never re-grabs
        // (fix-ladder-water2; the pre-109i2 code passed 0.55 here and re-grabbed him).
        assert!(l.capsule_hits_swim([10.0, 3.0, 20.5], 0.55, cc));
        assert!(!l.capsule_hits([10.0, 3.0, 20.5], crate::player::WALK_PICK_RADIUS, cc),
            "the dry stander at the dismount offset 0.5 re-grabbed through the walk arm (the pre-109i2 0.55 reach)");
        // And the walk arm DOES grab the crossing walker at the plane (frontal ≤ 0.275).
        assert!(l.capsule_hits([10.0, 3.0, 20.2], crate::player::WALK_PICK_RADIUS, cc));
        // Sister: the lateral span — 2.0 to the SIDE of the axis is past the rails (0.4 + 0.275).
        assert!(!l.capsule_hits([12.0, 3.0, 20.0], crate::player::WALK_PICK_RADIUS, cc));
        // DEEP below the sphere bottom (the pond-bed swimmer at the root ~2.0): the low swim segment
        // still reaches the ×1.5 sphere (dist ≈ 2.05 ≤ 2.8) — the climb-out assist keeps working.
        assert!(l.capsule_hits_swim([10.0, 2.0, 20.3], 0.55, cc));
        // Sister: a thin ladder stops the deep swim reach (the pick tracks the sphere): at the root
        // 0.9 the swim segment (0.52..0.32) sits 2.49 from the sphere centre — past the thin ×1.5
        // reach 1.45, while the full ladder still reaches (2.49 ≤ 2.8).
        let thin = Ladder { radius: 0.6, ..ladder(0.0) };
        assert!(!thin.capsule_hits_swim([10.0, 0.9, 20.0], 0.55, cc));
    }
}
