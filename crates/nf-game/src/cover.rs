//! Cover nodes (docs/systems/drone-cover.md): placements 0xE5 (`Drone_CoverCornerNode`) and 0xE6
//! (`Drone_CoverLowNode`), their per-target usability bits (`Drone_IsCoverNodeUsable`, refreshed by
//! `Drone_ProcessCoverNodes`), the positions a drone takes at them (`NDrone2_GetPosForCornerCover`) and the cover
//! anim table (`CCModes`). The drone side (choosing, claiming, the 0x8C / 0x93–0x9C states) is in drone.rs.

use crate::nav::Network;
use crate::trig::{angle_difference, atan2_approx, cosf, sinf, HALF_PI, PI};
use crate::world::{CollisionWorld, V3};
use nf_formats::placement::Placement;

#[derive(Clone, Debug)]
pub struct CoverNode {
    /// 0 corner, 1 low.
    pub kind: u8,
    /// Switch channel that must be on / off (0 = none).
    pub on: u8,
    pub off: u8,
    /// Low nodes: idle and aim anim states (+3 / +4).
    pub idle_anim: u16,
    pub aim_anim: u16,
    /// Corner: side-1 depth and side-2 offset (+5 / +6 × 0.1), the corner angle A (+8).
    pub depth: f32,
    pub offset: f32,
    pub angle: f32,
    /// +0xC: corner 4 crouch, 8 stand, 0x20 step-out, 0x40 lean, 0x80 side 2, 0x100 side 1; low 1, 2.
    pub flags: u32,
    pub pos: V3,
    pub yaw: f32,
    /// The emitter's navigation path and its distance bytes per node (`+0x40`, `AINetwork_EmitPath`).
    pub path: Option<usize>,
    pub dist: Vec<u8>,
}

/// Run-time state per node: owner (`+0x14`, flag 0x10) and the usability bits for target slot 0 (Bond, `+0x90`).
#[derive(Clone, Debug, Default)]
pub struct CoverRt {
    pub owner: Option<usize>,
    pub bits: u32,
}

/// The low-node anim table at 0x2F4540.
const LOW_ANIMS: [u16; 5] = [0x11, 5, 0xA, 7, 0x1D];

impl CoverNode {
    /// From a placement whose flags word is 0xE5 / 0xE6 (`parsemap_create_dynamic_objects`); parameters by index.
    pub fn from_placement(p: &Placement) -> Option<CoverNode> {
        let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
        let mut n = CoverNode { kind: 0, on: param(4) as u8, off: param(5) as u8, idle_anim: 0, aim_anim: 0, depth: 0.0, offset: 0.0,
            angle: 0.0, flags: 0, pos: p.translation, yaw: p.rotation[1], path: None, dist: Vec::new() };
        match p.flags {
            0xe5 => {
                n.flags |= match param(0) { 1 => 8, 2 => 4, _ => 0xc };
                n.flags |= match param(1) { 1 => 0x20, 2 => 0x40, _ => 0x60 };
                n.flags |= match param(2) { 1 => 0x80, 2 => 0x100, _ => 0x180 };
                n.angle = if param(3) == 0 { f32::from_bits(0x3f49_0fdb) } else { param(3) as f32 * 0.017_453_294 };
                n.depth = if param(6) == 0 { 8 } else { param(6) as u8 } as f32 * 0.1;
                n.offset = if param(7) == 0 { 8 } else { param(7) as u8 } as f32 * 0.1;
            }
            0xe6 => {
                n.kind = 1;
                if param(0) == 2 {
                    n.flags |= 1;
                    n.idle_anim = LOW_ANIMS.get(param(1) as usize).copied().unwrap_or(0x11);
                    n.aim_anim = LOW_ANIMS.get(param(2) as usize).copied().unwrap_or(0xa);
                } else {
                    n.idle_anim = 0x11;
                    n.aim_anim = if param(0) == 1 { 5 } else { 0xa };
                }
                if param(3) == 1 { n.flags |= 2 }
            }
            _ => return None,
        }
        Some(n)
    }

    pub fn is_corner(&self) -> bool { self.kind == 0 }

    /// `Drone_IsCoverNodeUsable(t, node)` (0x1386d8) for Bond: rebuilds `rt.bits`; returns whether a line-of-sight ray
    /// was cast (the round robin stops there). `target` = his position, `head` = his eye.
    pub fn usable(&self, rt: &mut CoverRt, world: &CollisionWorld, ch: &crate::triggers::Channels, level: u32, target: V3, head: V3) -> bool {
        rt.bits = 0;
        let sw = |c: u8| ch.on(c as u32);
        if (self.on != 0 && !sw(self.on)) || (self.off != 0 && sw(self.off)) { return false; }
        let occupied = rt.owner.is_some();
        let deg = |d: f32| d * PI / 180.0;
        let (f13, f14) = if self.is_corner() {
            if occupied { (0.77 * self.angle, self.angle + deg(10.0)) } else { (0.66 * self.angle, self.angle + deg(5.0)) }
        } else {
            let (a, b) = if level == 0x0700_000d { (75.0, 80.0) } else { (30.0, 45.0) };
            if occupied { (deg(a + 5.0), deg(b + 5.0)) } else { (deg(a), deg(b)) }
        };
        let d = ((target[0] - self.pos[0]).powi(2) + (target[1] - self.pos[1]).powi(2) + (target[2] - self.pos[2]).powi(2)).sqrt();
        if d < 4.0 { rt.bits |= 0x80; return false; }
        let diff = angle_difference(self.yaw, atan2_approx(target[0] - self.pos[0], target[2] - self.pos[2]));
        if !self.is_corner() {
            if diff.abs() > f14 { rt.bits |= 0x200; return false; }
            rt.bits |= 0x48;
        } else if self.flags & 0x100 != 0 && HALF_PI - f13 < diff && diff < HALF_PI + f14 {
            rt.bits |= 2 | if diff < HALF_PI { 0x10 } else { 0 };
        } else if self.flags & 0x80 != 0 && (diff > PI - f14 || diff < f13 - PI) {
            rt.bits |= 4 | 0x20;
        } else {
            rt.bits |= 0x200;
            return false;
        }
        let h = if level == 0x0700_0011 { 1.15 } else { 1.5 };
        if !world.line_of_sight(head, [self.pos[0], self.pos[1] + h, self.pos[2]]) { rt.bits = rt.bits & !0x70 | 0x400; }
        true
    }

    /// `NDrone2_GetPosForCornerCover(node, bits, approach)` (0x152028): the in-cover point (or the approach point
    /// before it) and the side (1, 2, 3 = low); `None` without usable bits.
    pub fn position(&self, bits: u32, approach: bool) -> Option<(V3, u8)> {
        let mut side = 0;
        for (bit, s) in [(2, 1), (4, 2), (8, 3)] { if bits & bit != 0 { side = s } }
        let (x, z) = match side {
            1 => (if approach { -0.7 } else { 0.1 }, -self.depth),
            2 => (self.offset, if approach { 0.7 } else { -0.1 }),
            3 => (0.0, if approach { -1.1 } else { -0.3 }),
            _ => return None,
        };
        let (s, c) = (sinf(self.yaw), cosf(self.yaw));
        Some(([self.pos[0] + x * c + z * s, self.pos[1], self.pos[2] - x * s + z * c], side))
    }
}

/// Every cover node of the level's main map, with its emitter (navigation path for its position, distance bytes).
pub fn build(placements: &[Placement], nav: &Network, world: &CollisionWorld) -> Vec<CoverNode> {
    placements.iter().filter_map(CoverNode::from_placement).map(|mut n| {
        n.path = nav.nav_path_for(n.pos);
        if let Some(p) = n.path { n.dist = nav.emit(world, p, n.pos); }
        n
    }).collect()
}

/// Cover types (`Drone+0xBBB`).
pub mod kind {
    pub const CROUCH_LEAN: u8 = 0;
    pub const CROUCH_STEP: u8 = 1;
    pub const STAND_LEAN: u8 = 2;
    pub const STAND_STEP: u8 = 3;
    pub const AS_LOW: u8 = 4;
}

/// `CCModes` (0x270768): per cover type and side, the anim state of each column (0 enter, 1 idle, 2 aim, 3 —, 4
/// return, 5 change).
pub fn cc_anim(cover_type: u8, side: u8, column: usize) -> u16 {
    const T: [[u16; 6]; 9] = [
        [0x10, 0x10, 0x13, 0x58, 0x10, 0x1a], [0x10, 0x10, 0x14, 0x58, 0x10, 0x1a],
        [0x10, 0x10, 0x15, 0x58, 0x10, 0x1a], [0x10, 0x10, 0x16, 0x58, 0x10, 0x1a],
        [0x1a, 0x1a, 0x17, 0x58, 0x1a, 0x1a], [0x1a, 0x1a, 0x18, 0x58, 0x1a, 0x1a],
        [0x1a, 0x1a, 0x1b, 0x58, 0x1a, 0x1a], [0x1a, 0x1a, 0x1c, 0x58, 0x1a, 0x1a],
        [0x11, 0x11, 0x0a, 0x58, 0x11, 0x0a],
    ];
    let row = if cover_type >= kind::AS_LOW { 8 } else { cover_type as usize * 2 + (side.clamp(1, 2) as usize - 1) };
    T[row][column.min(5)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corner(yaw: f32) -> CoverNode {
        CoverNode::from_placement(&Placement { index: 0, hashcode: u32::MAX, flags: 0xe5, translation: [0.0; 3], rotation: [0.0, yaw, 0.0], params: vec![(3, 46)] }).unwrap()
    }

    #[test]
    fn corner_sides_and_positions() {
        let n = corner(0.0);
        assert_eq!(n.flags, 0xc | 0x60 | 0x180);
        assert!((n.depth - 0.8).abs() < 1e-6 && (n.angle - 46f32.to_radians()).abs() < 1e-5);
        let w = CollisionWorld::empty();
        let mut rt = CoverRt::default();
        // Bond to the node's +x side (yaw to him π/2 from the node's facing): side 1.
        assert!(n.usable(&mut rt, &w, &Default::default(), 0x0700_0005, [10.0, 0.0, 0.0], [10.0, 1.5, 0.0]));
        assert_eq!(rt.bits & 0xe, 2);
        // Behind the node: side 2.
        n.usable(&mut rt, &w, &Default::default(), 0x0700_0005, [0.0, 0.0, -10.0], [0.0, 1.5, -10.0]);
        assert_eq!(rt.bits & 0xe, 4);
        // In front: unusable, no ray.
        assert!(!n.usable(&mut rt, &w, &Default::default(), 0x0700_0005, [0.0, 0.0, 10.0], [0.0, 1.5, 10.0]));
        assert_eq!(rt.bits, 0x200);
        // Side 2 in-cover point: x = offset 0.8, z = −0.1 in the node's frame.
        let (p, side) = n.position(4, false).unwrap();
        assert_eq!(side, 2);
        assert!((p[0] - 0.8).abs() < 1e-5 && (p[2] + 0.1).abs() < 1e-5, "{p:?}");
        assert_eq!(cc_anim(kind::STAND_LEAN, 2, 2), 0x18);
    }
}
