//! Bullets against drones: the hit boxes (`AnimGetCollData` 0x1c4ac8), the ray test (`Collide_Jointy` 0x1ec180) and the
//! damage (`NDrone2_HitDamage` 0x145558). Spec: docs/systems/drone-damage.md.

use crate::world::V3;
use nf_formats::anim::{self, Mat4};
use nf_formats::hitbox::CollisionBone;
use nf_formats::tuning::Tuning;

/// One oriented box: rows 0–2 the other bone's axes, row 3 the centre; half-extents in those axes; the part (bone).
#[derive(Clone, Copy, Debug)]
pub struct HitBox { pub m: Mat4, pub half: [f32; 3], pub part: i16 }

/// The drone's object matrix as nf-view draws it: yaw about +Y, x/z from the drone, y from its feet plus the hip height
/// (`Drone+0xA0` = hip − 0.02, at least 0.2; 1.0 without a hip).
pub fn drone_height(hip: f32) -> f32 { if hip == 0.0 { 1.0 } else { (hip - 0.02).max(0.2) } }

/// `AnimGetCollData`: boxes from the bone world matrices (`palette`, world space) per collision-bone entry. Entry 0's
/// centre is the bone plus (0, bone.y − other.y, 0); the others sit midway between bone and other. The box axes are a
/// copy of the other bone's matrix. Extents × skin scale.
pub fn boxes(table: &[CollisionBone], palette: &[Mat4], skin: &nf_formats::character::Skin) -> Vec<HitBox> {
    let mut out = Vec::with_capacity(table.len());
    for (k, e) in table.iter().enumerate() {
        let b = e.bone as usize;
        let o = if e.other < 0 { match skin.parent(b) { Some(p) => p, None => continue } } else { e.other as usize };
        let (Some(mb), Some(mo)) = (palette.get(b), palette.get(o)) else { continue };
        let mut m = *mo;
        let (tb, to) = ([mb[3][0], mb[3][1], mb[3][2]], [mo[3][0], mo[3][1], mo[3][2]]);
        let c = if k == 0 { [tb[0], tb[1] + (tb[1] - to[1]), tb[2]] } else { [0, 1, 2].map(|i| (tb[i] + to[i]) * 0.5) };
        m[3] = [c[0], c[1], c[2], 1.0];
        out.push(HitBox { m, half: [e.half[0] * skin.scale[0], e.half[1] * skin.scale[1], e.half[2] * skin.scale[2]], part: e.bone });
    }
    out
}

/// World bone matrices of a posed skeleton: `local` with the root translation zeroed (`AnimSeqTick` takes it as root
/// motion), then placed by `object` (row vectors: world = model · object).
pub fn palette(local: &mut [([f32; 3], [f32; 4])], skin: &nf_formats::character::Skin, object: &Mat4) -> Vec<Mat4> {
    if let Some(r) = local.first_mut() { r.0 = [0.0; 3]; }
    anim::world_pose(local, skin).iter().map(|w| anim::mul(w, object)).collect()
}

/// `Intersect_RayBox` in the box's frame: the segment `start + t·ray`, t in [0, 1], against [−half, half]; the entry t
/// (0 when the start is inside).
pub fn ray_box(start: V3, ray: V3, b: &HitBox) -> Option<f32> {
    let c = [b.m[3][0], b.m[3][1], b.m[3][2]];
    let (mut t0, mut t1) = (0f32, 1f32);
    for k in 0..3 {
        let axis = [b.m[k][0], b.m[k][1], b.m[k][2]];
        let l2 = axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2];
        if l2 <= 0.0 { return None; }
        let p = ((start[0] - c[0]) * axis[0] + (start[1] - c[1]) * axis[1] + (start[2] - c[2]) * axis[2]) / l2.sqrt();
        let d = (ray[0] * axis[0] + ray[1] * axis[1] + ray[2] * axis[2]) / l2.sqrt();
        let h = b.half[k];
        if d.abs() < 1e-12 {
            if p < -h || p > h { return None; }
        } else {
            let (a, z) = ((-h - p) / d, (h - p) / d);
            let (lo, hi) = if a < z { (a, z) } else { (z, a) };
            t0 = t0.max(lo);
            t1 = t1.min(hi);
            if t0 > t1 { return None; }
        }
    }
    Some(t0)
}

/// `Collide_Jointy` for a ray: the first box in list order the segment hits (not the nearest), with its t.
pub fn first_box_hit(start: V3, ray: V3, boxes: &[HitBox]) -> Option<(i16, f32)> {
    boxes.iter().find_map(|b| ray_box(start, ray, b).map(|t| (b.part, t)))
}

/// What `NDrone2_HitDamage` needs from the drone.
#[derive(Clone, Copy, Debug)]
pub struct Target { pub damageable: bool, pub armour: u8, pub mode: u8, pub skin: u32, pub kind: u16 }

/// `NDrone2_HitDamage`'s result: health lost, the value recorded in `+0x150`, and whether `+0x3A` (head shot, silent
/// kill) was set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damage { pub loss: f32, pub recorded: f32, pub head: bool }

/// `NDrone2_HitDamage` (0x145558) for a hit of `dmg` (HIT+8 · `Drone+0x14C`) on `part` with `weapon`.
/// `difficulty` = `GameState+0x28`. A drone with `+0x14` = 0 loses nothing; unless forced it only records `dmg`.
pub fn hit_damage(t: &Tuning, target: Target, dmg: f32, part: i16, weapon: u16, forced: bool, difficulty: u32, level: u32) -> Damage {
    use nf_formats::ps2float::mul;
    let mut part = if part < 0 { 1 } else { part };
    if part == 5 && matches!(weapon, 1 | 0x4a..=0x4d) { part = 1; }
    if level == 0x0700_001b && part != 5 { part = 1; }
    if !target.damageable && !forced { return Damage { loss: 0.0, recorded: dmg, head: false }; }
    let a = target.armour;
    let mut head = false;
    let mut d = dmg;
    match part {
        5 => {
            let f = if target.mode == 0xd || target.skin == 0x0500_00ba || (target.kind == 0x10 && level == 0x0700_0014) {
                t.DroneDamage_Torso + t.DroneDamage_Torso
            } else { head = true; t.DroneDamage_Head };
            d = mul(d, f);
            if a & 4 != 0 { d = mul(d, t.DroneArmour_Helmet); }
        }
        0x14 | 0x15 | 0x17 | 0x20 | 0x23 | 0x27 => {
            d = mul(d, t.DroneDamage_Arms);
            if a & 8 != 0 { d = mul(d, t.DroneArmour_Combat); }
            if a & 2 != 0 { d = mul(d, t.DroneArmour_Jacket); }
        }
        0x31..=0x38 => {
            d = mul(d, t.DroneDamage_Legs);
            if a & 8 != 0 { d = mul(d, t.DroneArmour_Combat); }
        }
        _ => {
            d = mul(d, t.DroneDamage_Torso);
            if a & 8 != 0 { d = mul(d, t.DroneArmour_Combat); }
            if a & 2 != 0 { d = mul(d, t.DroneArmour_Jacket); }
            if a & 1 != 0 { d = mul(d, t.DroneArmour_Vest); }
        }
    }
    let k = match difficulty { 1 => t.DroneDamage_Easy, 3 | 4 => t.DroneDamage_Hard, _ => t.DroneDamage_Normal };
    let d = mul(d, k);
    Damage { loss: if target.damageable { d } else { 0.0 }, recorded: d, head }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_box(c: V3, half: [f32; 3], part: i16) -> HitBox {
        HitBox { m: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [c[0], c[1], c[2], 1.0]], half, part }
    }

    #[test]
    fn rays_hit_boxes_in_list_order() {
        let a = unit_box([0.0, 0.0, 5.0], [0.5; 3], 4);
        let b = unit_box([0.0, 0.0, 2.0], [0.5; 3], 5);
        // The far box comes first in the list and wins although the near one is hit earlier along the ray.
        assert_eq!(first_box_hit([0.0, 0.0, 0.0], [0.0, 0.0, 10.0], &[a, b]), Some((4, 0.45)));
        assert_eq!(first_box_hit([0.0, 0.0, 0.0], [0.0, 0.0, 10.0], &[b, a]), Some((5, 0.15)));
        // Missing to the side, and stopping short.
        assert_eq!(first_box_hit([1.0, 0.0, 0.0], [0.0, 0.0, 10.0], &[a, b]), None);
        assert_eq!(first_box_hit([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], &[a, b]), None);
        // Starting inside: t = 0.
        assert_eq!(ray_box([0.0, 0.0, 2.1], [0.0, 0.0, 1.0], &b), Some(0.0));
    }

    #[test]
    fn hit_damage_per_part_and_difficulty() {
        let mut t = Tuning::default();
        (t.DroneDamage_Easy, t.DroneDamage_Normal, t.DroneDamage_Hard) = (1.5, 1.0, 0.8);
        (t.DroneDamage_Head, t.DroneDamage_Legs, t.DroneDamage_Arms, t.DroneDamage_Torso) = (10.0, 0.75, 1.0, 1.0);
        let g = Target { damageable: true, armour: 0, mode: 0, skin: 0, kind: 2 };
        // P2K 3.5 on 07000005: torso Easy 5.25, legs Normal 2.625, head 35 and a silent kill; fists on the head = torso.
        let h = |part, weapon, diff| { let r = hit_damage(&t, g, 3.5, part, weapon, false, diff, 0x0700_0005); (r.loss, r.head) };
        assert_eq!(h(4, 2, 1), (5.25, false));
        assert_eq!(h(0x35, 2, 2), (2.625, false));
        assert_eq!(h(5, 2, 2), (35.0, true));
        assert_eq!(h(5, 1, 2), (3.5, false));
        assert_eq!(h(4, 2, 3), (2.8, false));
        let off = hit_damage(&t, Target { damageable: false, ..g }, 3.5, 4, 2, false, 2, 0x0700_0005);
        assert_eq!((off.loss, off.recorded), (0.0, 3.5));
    }
}
