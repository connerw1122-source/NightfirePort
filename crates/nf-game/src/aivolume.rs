//! AI volumes (`Drone_AIVolume_Create` 0x1376c0, placements 0xF5 / 0xF8, e.g. `AI_ShadowVolume`): oriented boxes that
//! scale how visible Bond is to drones (`Drone_ProcessOpponents` 0x1390c0 → `Drone+0x22C`). Spec:
//! docs/systems/drone-ai.md "Perception".

use crate::world::V3;
use nf_formats::anim::Mat4;
use nf_formats::level::Level;

#[derive(Clone, Debug)]
pub struct AiVolume {
    /// Rows 0–2 the box axes, row 3 the centre (the created object's matrix, `obj+0x90`).
    pub m: Mat4,
    /// Half-extents: |(glist max − min) / 2| × the placement's scale (+0x34).
    pub half: [f32; 3],
    /// Parameter 0 × 0.01 (`+0x20`): the visibility factor.
    pub factor: f32,
    /// `+0x24` from parameter 1: 0 → 0 (visibility box, the only kind `Drone_ProcessOpponents` uses), 1 → 2, 2 → 1,
    /// 4 → 3, other values leave 0.
    pub kind: u16,
    pub placement: usize,
    /// fix-script-objects: a SCRIPT-created volume's owner (script hash, stream) — its matrix
    /// refreshes from the live `Obj::Entity` every tick (the game keys obj+0x90 from the script
    /// stream from the creation tick on [M Script_Interp 0x1d9a08:106-133]). `None` = a placement.
    pub src: Option<(u32, u8)>,
}

/// The main map's AI volumes. On 07000005 (and for parameter 1 = 4) the creator negates the rotation's z and lifts the
/// position by 1.5 × scale y.
pub fn build(level: &Level, level_id: u32) -> Vec<AiVolume> {
    let Some(m) = level.main_map() else { return Vec::new() };
    let map = &level.maps[m];
    let mut out = Vec::new();
    for (i, p) in map.placements.iter().enumerate().filter(|(_, p)| matches!(p.flags, 0xf5 | 0xf8)) {
        let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((m, p.index as usize)) };
        // `Drone_AIVolume_Create` returns without a glist.
        let Some(obj) = target.and_then(|(om, oi)| level.maps[om].objects.get(oi)) else { continue };
        let scale = p.scale();
        let (mut pos, mut rot) = (p.translation, p.rotation);
        if level_id == 0x0700_0005 || param(1) == 4 {
            rot[2] = -rot[2];
            pos[1] += nf_formats::ps2float::mul(scale[1], 1.5);
        }
        let half = [0, 1, 2].map(|k| ((obj.max[k] - obj.min[k]) * 0.5).abs() * scale[k]);
        let kind = match param(1) { 0 => 0, 1 => 2, 2 => 1, 4 => 3, _ => 0 };
        out.push(AiVolume { m: matrix(rot, pos), half, factor: nf_formats::ps2float::mul(param(0) as f32, 0.01), kind, placement: i, src: None });
    }
    out
}

/// The created object's matrix: `control_create_object` without a quaternion → `View_RotTransMatrix` (0x1e5168):
/// sines from `PS2Sinf3` (cosines as sin(a + π/2), the sum on the FPU), products and sums on the FPU; row vectors
/// row 0 = (cy·cz, sz, −cz·sy), row 1 = (sx·sy − cx·cy·sz, cx·cz, cy·sx + cx·sy·sz),
/// row 2 = (cx·sy + cy·sx·sz, −cz·sx, cx·cy − sx·sy·sz) — the order Rx·Rz·Ry, which a fit to the live matrix of the
/// tilted 07000005 volume had found first (WORKLOG 0032 / 0038).
pub fn matrix(rot: [f32; 3], pos: V3) -> Mat4 {
    use crate::trig::{cosf, sinf};
    use nf_formats::ps2float::{add_noguard, mul};
    let add = |a: f32, b: f32| add_noguard(a, b, true);
    let (cx, sx) = (cosf(rot[0]), sinf(rot[0]));
    let (cy, sy) = (cosf(rot[1]), sinf(rot[1]));
    let (cz, sz) = (cosf(rot[2]), sinf(rot[2]));
    [
        [mul(cy, cz), sz, -mul(cz, sy), 0.0],
        [add(mul(sx, sy), -mul(mul(cx, cy), sz)), mul(cx, cz), add(mul(cy, sx), mul(mul(cx, sy), sz)), 0.0],
        [add(mul(cx, sy), mul(mul(cy, sx), sz)), -mul(cz, sx), add(mul(cx, cy), -mul(mul(sx, sy), sz)), 0.0],
        [pos[0], pos[1], pos[2], 1.0],
    ]
}

impl AiVolume {
    /// `Intersect_PointOOBox`: the point within the box (in its axes, |local| ≤ half).
    pub fn contains(&self, p: V3) -> bool {
        let d = [p[0] - self.m[3][0], p[1] - self.m[3][1], p[2] - self.m[3][2]];
        (0..3).all(|r| {
            let a = [self.m[r][0], self.m[r][1], self.m[r][2]];
            let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-12);
            ((d[0] * a[0] + d[1] * a[1] + d[2] * a[2]) / l).abs() <= self.half[r]
        })
    }
}

/// `Drone_ProcessOpponents`: the product of the factors of the visibility boxes (kind 0) holding `p`; ×0.75 when Bond
/// crouches (player `obj+0xF6` = 4).
pub fn visibility_factor(volumes: &[AiVolume], p: V3, crouched: bool) -> f32 {
    let mut f = 1.0f32;
    for v in volumes.iter().filter(|v| v.kind == 0) {
        if v.contains(p) { f = nf_formats::ps2float::mul(f, v.factor); }
    }
    if crouched { f = nf_formats::ps2float::mul(f, 0.75); }
    f
}
