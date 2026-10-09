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
        out.push(AiVolume { m: matrix(rot, pos), half, factor: nf_formats::ps2float::mul(param(0) as f32, 0.01), kind, placement: i });
    }
    out
}

/// The created object's matrix (`control_create_object`): row vectors, v' = v · Rx(x) · Rz(z) · Ry(y) + pos —
/// *measured*: fits the live matrix of the 07000005 volume rotated about all three axes (placement 39) to 2e-5;
/// `RotMatrix`'s order (used for map placements) misses it by 0.015. The function that builds it is not read (U-035).
pub fn matrix(rot: [f32; 3], pos: V3) -> Mat4 {
    let r = |axis: usize, a: f32| -> [[f32; 3]; 3] {
        let (s, c) = a.sin_cos();
        match axis { 0 => [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]], 1 => [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]], _ => [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]] }
    };
    let mul = |a: [[f32; 3]; 3], b: [[f32; 3]; 3]| -> [[f32; 3]; 3] { std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum())) };
    let m = mul(mul(r(0, rot[0]), r(2, rot[2])), r(1, rot[1]));
    [[m[0][0], m[0][1], m[0][2], 0.0], [m[1][0], m[1][1], m[1][2], 0.0], [m[2][0], m[2][1], m[2][2], 0.0], [pos[0], pos[1], pos[2], 1.0]]
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
