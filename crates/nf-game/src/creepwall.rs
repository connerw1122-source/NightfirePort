//! The creepwall — the crawlable vent duct (ACTION.ELF). There is no grate class: the crawlable vent
//! is a **creepwall** (placement 0x3F → `CreepWall_Create` 0x196fc8, class 0x2B = '+'), and the grate
//! panel itself is an ordinary door (class 0x1C, `Door_Activate` 0x135be0) [M, fix-ladder2's census].
//! The use path: `Player_Activate` 0x1a5170:66-97 — with the third-icon context 6, the use key scans
//! every object for class '+' and calls `Player_CollCreepWall` 0x196758 on the NEAREST within 1000
//! units (no capsule test). The crawl: substate 7, `Player_Creep` 0x1969d0 — `Music_Event(0xD,1)`
//! and `switch_channels[params+6] = 1` every tick, the exit at the wall's ends (the input direction
//! decides), the channel cleared on the way out.

use crate::world::V3;

#[derive(Clone, Debug)]
pub struct CreepWall {
    /// The placement index (the port's identity for the object).
    pub placement: u32,
    /// The placement transform's translation (the object's pos vec, `obj+0x80` [M 0x196758:78-79]).
    pub pos: V3,
    /// The placement's yaw (`obj+0x54`); the wall's axis runs along `yaw + π/2`
    /// [M 0x196758:80-86 + 0x1969d0:99-100: `Vec_Spherical_2_Cartesian(len, yaw + 1.5707964, 0)`].
    pub yaw: f32,
    /// The axis half-length (`obj+0x8c`). `CreepWall_Create` sets no +0x8c itself — it arrives from
    /// `control_create_object`'s placement transform; the port takes the placement scale's x [S].
    pub half_len: f32,
    /// The switch channel held while Bond crawls: the 4th param word's low half (level+0x38 →
    /// the object's BLData u16[3], read as `*(short *)(params + 6)` [M 0x196fc8:19-22 +
    /// 0x1969d0:86-88]).
    pub ch: u16,
}

impl CreepWall {
    /// The wall's axis endpoints: pos ± fwd(yaw + π/2) · half_len [M 0x196758:80-90: the two vectors
    /// `p + d` and `p − d` with `d = Cartesian(len, yaw + π/2, 0)`].
    pub fn endpoints(&self) -> (V3, V3) {
        // G-046 site (exact-sine-sites-3) [M Vec_Spherical_2_Cartesian 0x1e3948:22-26]: with the call
        // `S2C(half_len, wall.yaw + π/2, 0)` (the +π/2 an EE FPU add at the caller, 0x1969d0:99-100),
        // the pair is `PS2Sinf2(θ + π/2, θ)` → (cos θ, sin θ) plus `PS2Sinf2(φ + π/2, φ)` for φ = 0,
        // and the lanes are `(len·sin θ)·cos φ` / `(len·cos θ)·cos φ` — so z's sine is the SECOND
        // no-guard add `sin(θ + π/2)` and both lanes multiply by the game's own `sinf(π/2)` (`cos φ`).
        // The ± endpoints are EE FPU adds/subs (0x1969d0:102-105).
        if crate::trig::host_trig() {
            let (s, c) = (self.yaw + std::f32::consts::FRAC_PI_2).sin_cos();
            let d = [s * self.half_len, 0.0, c * self.half_len];
            (
                [self.pos[0] + d[0], self.pos[1], self.pos[2] + d[2]],
                [self.pos[0] - d[0], self.pos[1], self.pos[2] - d[2]],
            )
        } else {
            use nf_formats::ps2float as pf;
            let half = crate::trig::HALF_PI;
            let theta = crate::trig::add(self.yaw, half);
            let (sin_t, cos_t) = (crate::trig::sinf(theta), crate::trig::sinf(crate::trig::add(theta, half)));
            let cos_p = crate::trig::sinf(crate::trig::add(0.0, half)); // PS2Sinf2(0 + π/2, 0) lane 0
            let dx = pf::mul(pf::mul(self.half_len, sin_t), cos_p);
            let dz = pf::mul(pf::mul(self.half_len, cos_t), cos_p);
            (
                [crate::trig::add(self.pos[0], dx), self.pos[1], crate::trig::add(self.pos[2], dz)],
                [crate::trig::sub(self.pos[0], dx), self.pos[1], crate::trig::sub(self.pos[2], dz)],
            )
        }
    }

    /// The closest point of the axis segment to `p` (the game's
    /// `vecutil_calculate_closest_point_on_line` 0x196758:91-92 / 0x1969d0:106-107).
    pub fn closest_point(&self, p: V3) -> V3 {
        let (a, b) = self.endpoints();
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let l2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
        let t = if l2 > 0.0 {
            (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1] + (p[2] - a[2]) * ab[2]) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t]
    }
}

fn param(p: &nf_formats::placement::Placement, k: u32) -> u32 {
    p.params.iter().find(|(i, _)| *i == k).map_or(0, |&(_, v)| v)
}

/// Every creepwall placement of a level (`flags & 0xff == 0x3F`), mirroring the wire sweep
/// (`wire::all`) [M 0x1d04f0 case 0x3F → `CreepWall_Create` 0x196fc8]. The wall's pos/yaw come from
/// the placement transform (`control_create_object`'s vec pair); the half-length is the placement
/// scale's x [S — the game's obj+0x8c arrives from that transform; no Create-side write]; the
/// channel is param 3 [M 0x196fc8 reads placement+0x2c/0x30/0x34/0x38 into BLData u16[0..3], and
/// `Player_Creep` reads `*(short *)(params + 6)` = u16[3] as the switch channel].
pub fn all(level: &nf_formats::level::Level) -> Vec<CreepWall> {
    let mut out = Vec::new();
    for m in level.maps.iter() {
        for p in &m.placements {
            if p.flags & 0xff != 0x3f { continue; }
            let scale = p.scale();
            out.push(CreepWall {
                placement: u32::from(p.index),
                pos: p.translation,
                yaw: p.rotation[1],
                half_len: scale[0].abs().max(1.0),
                ch: param(p, 3) as u16,
            });
        }
    }
    out
}
