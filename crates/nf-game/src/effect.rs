//! Bullet impact effects: the decal FIFO (`Effect_Bullet` 0x1ce8a8 + `Effect_InitLists` 0x1cf268's
//! 200-entry pool at 0x3496e0), the per-material gas puffs (`Effect_Create` 0x1ce688 → `Gas_Init`
//! 0x1aacc8), debris (`Debris_Create` 0x1aaa00, `Debris_Update` 0x1aac00) and ricochets
//! (`Effect_RicochetProb` 0x1cf290 + the reflection in `Bullet_CollisionHandler` 0x12ce80).
//! Per-material data comes from the `EffectInfo` table (0x3037c0, parsed in
//! `nf_formats::sfx::SurfaceSounds::effects`). G-041; docs/atlas/effect.md.
//!
//! RNG discipline: every `Rand_*` the game draws at an impact is drawn here, in the game's order
//! (the collision handler runs: the material-0x80 roll [not ported — the port's HITDATA carries no
//! +0x51 second byte], then `Effect_Create`'s debris-count draw, then per-debris draws, then
//! `Effect_Bullet`'s `Rand_Rand(2)`, then the ricochet roll). `Gas_Init`'s own 5 draws per puff are
//! NOT consumed — the merged deviation documented in gas.rs (G-033's discipline).

use crate::drone::Rng;
use crate::world::V3;
use nf_formats::sfx::{EffectRecord, SurfaceSounds};

pub type Mat4 = [[f32; 4]; 4];

/// `REC_FRAME_RATE` (0x30d0cc) = 1/60 [M grenade.rs:52].
const REC: f32 = 1.0 / 60.0;
/// The decal's surface offset: `Effect_Bullet` stores `hit + normal · 0.01` [M 0x1ce9f0].
const DECAL_OFFSET: f32 = 0.01;
/// The wall-puff offset: `hit + normal · 0.05` [M 0x1ce980, 0x3d4ccccd].
const PUFF_OFFSET: f32 = 0.05;

/// One bullet-hole decal (a pool node at 0x3496e0, 0x60 bytes each): position, surface normal and the
/// `EffectInfo+4` mesh hash. Static hits ride the cel, dynamic hits the object (`HITDATA+0x54/+0x58`);
/// the port keeps only the world-space result — dynamic hosts are not modelled (no marks on moving
/// doors yet, listed in the report).
#[derive(Clone, Debug, PartialEq)]
pub struct Decal {
    pub pos: V3,
    pub normal: V3,
    /// The `EffectInfo[material]+4` decal mesh hash (0x02000122 glass, 0x02000126/127 stone/metal,
    /// 0x02000128 wood, 0x02000262 soft-generic [D ee_start_05.bin]; ICE 0x020006fd).
    pub mesh: u32,
}

/// One debris particle (`Debris_Create` 0x1aaa00's class-0x40 object): position, velocity, spin,
/// life in ticks, scale and the `EffectInfo+0x74` mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct Debris {
    pub pos: V3,
    pub vel: V3,
    /// obj data+0x20: the `Rand_FRandHalf_Vec` spin, added to the rotation every tick.
    pub spin: V3,
    /// obj data+0x30: ticks left (`Debris_Update` kills the object at ≤ 0).
    pub life: i16,
    /// obj data+0xe8: 0.4 + FRand(0.1).
    pub scale: f32,
    pub mesh: u32,
    /// obj+0x106: the alpha byte, stepped down by data+0x32 (= 1) per tick in `Debris_Update`.
    pub alpha: u8,
}

/// The sim's effect state: the 200-slot decal FIFO (oldest evicted when full [M `Effect_Bullet`
/// 0x1cea20: pool empty → `Effect_UnLink(DAT_003496ec)` = the in-use tail]) and the live debris.
#[derive(Clone, Debug, Default)]
pub struct Effects {
    pub decals: Vec<Decal>,
    pub debris: Vec<Debris>,
    /// The count of decals evicted so far (observability for tests).
    pub evicted: u32,
}

/// `Rand_FRandHalf(max)` 0x1e3810: one FRand draw, minus `max·0.5` — the signed half-range.
pub fn frand_half(rng: &mut Rng, max: f32) -> f32 {
    nf_formats::ps2float::sub(rng.frand(max), max * 0.5)
}

/// `Rand_FRand_MVar2(min, off)` 0x1e38b0: one FRand draw of `min`, minus `off` — range [−off, min−off].
pub fn frand_mvar2(rng: &mut Rng, min: f32, off: f32) -> f32 {
    nf_formats::ps2float::sub(rng.frand(min), off)
}

/// `Rand_FRandHalf_Vec(scale, out)` 0x1dfef0: three FRandHalf draws into a vector.
pub fn frand_half_vec(rng: &mut Rng, scale: f32) -> V3 {
    [frand_half(rng, scale), frand_half(rng, scale), frand_half(rng, scale)]
}

/// `Vec_MakeOrthonormalBasis` 0x1e4b78: two vectors orthonormal against `n`; `Effect_Create` stores
/// `u + v` into `HITDATA+0x20` [M 0x1ce740] and `Debris_Create` spreads the debris velocity with it.
pub fn orthonormal_basis_pair(n: V3) -> (V3, V3) {
    if n[0].abs() <= n[1].abs() {
        let il = 1.0 / (n[2] * n[2] + n[1] * n[1]).sqrt();
        let a = [0.0, -n[2] * il, n[1] * il];
        let b = [n[1] * a[2] - n[2] * a[1], -n[0] * a[2], n[0] * a[1]];
        (a, b)
    } else {
        let il = 1.0 / (n[2] * n[2] + n[0] * n[0]).sqrt();
        let a = [n[2] * il, 0.0, -n[0] * il];
        let b = [n[1] * a[2], n[2] * a[0] - n[0] * a[2], -n[1] * a[0]];
        (a, b)
    }
}

/// Returns `u + v` (the only form the callers use).
pub fn orthonormal_basis_sum(n: V3) -> V3 {
    let (a, b) = orthonormal_basis_pair(n);
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `Bullet_CollisionHandler` 0x12ce80's per-hit pass rules [M disasm 0x12cee0..0x12cfb8]: the walk
/// decides whether a bullet PASSES THROUGH the surface it hit (a pierce keeps the bullet alive —
/// state 1, the handler walks the trace's NEXT HITDATA and spawns the exit-hole effects — while a
/// stop dies with `BU+0xF0 |= 0x10` [M 0x12d81c..0x12d82c]). In the game's order:
/// - `HITDATA+0x51` bit 1 — set by `Collide_Intersect` 0x1ebaa8 when the glist node's flags carry
///   the pass bit — forces pierce (the caller's `node_pass`);
/// - the object-hit rules (`obj+0xf0 & 0x60`, the shooter skip, class-0x28 windows vs
///   `weapon_data+0x70 & 0x100` [M 0x12cf10..0x12cf3c]) — the port's world walk sees world hits
///   only, no object arm;
/// - **the glass rule [M 0x12cf44..0x12cf64]**: `if (!(weapon_data+0x6c & 0x800) &&
///   (HITDATA+0x50 & 0x3f) == 0x10) pass = 1` — material index 0x10 (the 07000008 corridor
///   windows' surface byte) lets ANY bullet through unless the weapon's `+0x6c` carries 0x800
///   (the Laser 0x4E's +0x6c = 0x900 [L] burns the pane instead); no RNG draw;
/// - `weapon_data+2` (the weapon id) 0x50/0x51 (the Q-Claw) then skip the surface-byte rules —
///   no draw [M 0x12cf68 `if (uVar2 - 0x50 < 2)`];
/// - surface byte `& 0x40` → forced pierce, no draw [M 0x12cf7c];
/// - surface byte `& 0x80` → ONE `Rand_Rand(0x3f)` draw; bit 0 of the roll ORs into pierce
///   [M 0x12cf9c `Rand_Rand(0x3f)`, `bVar7 = bVar7 | bVar6 & 1`].
/// The material's 0x40/0x80 bits are per-triangle bytes in the level collision blob
/// (`Intersect_RayGeom` 0x10a8c8:326 copies the whole byte to `HITDATA+0x50`).
/// `flags6c` is the shooter's `weapon_data+0x6c` word (the glass-rule gate). fix-rocket-glass-08.
pub fn penetration_roll(rng: &mut Rng, material: u8, weapon_id: u16, flags6c: u32) -> bool {
    penetration_roll_node(rng, material, weapon_id, false, flags6c)
}

/// The roll with the hit's node pass flag (`HITDATA+0x51` bit 1's glist-flags source —
/// `Collide_Intersect` 0x1ebaa8:143-145): a node-flagged hit passes WITHOUT a draw (the game's
/// walk sets the pass from the flag and the 0x40 byte before any roll), and the surface byte's
/// 0x80 roll is drawn exactly when the byte carries the bit — even on a node-flagged hit, whose
/// result is then OR'd in (`Collide_Update` 0x1ea398:216-220). fix-foliage-bullets.
pub fn penetration_roll_node(rng: &mut Rng, material: u8, weapon_id: u16, node_pass: bool, flags6c: u32) -> bool {
    let mut pierce = node_pass
        // the glass rule [M 0x12cf44..0x12cf64]: material 0x10 passes before the Q-Claw's
        // type-word check, so even the grapple bolt flies through a pane (no draw).
        || flags6c & 0x800 == 0 && material & 0x3f == 0x10;
    if weapon_id == 0x50 || weapon_id == 0x51 { return pierce; }
    pierce |= material & 0x40 != 0;
    if material & 0x80 != 0 {
        pierce |= rng.rand(0x3f) & 1 != 0;
        if std::env::var_os("NF_RNGLOG").is_some() {
            eprintln!("[rnglog] penetration_roll +1 (draw #{})", rng.draws);
        }
    }
    pierce
}

impl Effects {
    pub fn new() -> Effects { Effects { decals: Vec::with_capacity(200), debris: Vec::new(), evicted: 0 } }

    /// The `EffectInfo` record of `material` (`HITDATA+0x50 & 0x3f`), None when the table is absent or
    /// the material is 0 (the game's `Effect_Create` guard [M 0x1ce6f8]).
    pub fn record<'a>(surfaces: Option<&'a SurfaceSounds>, material: u8) -> Option<&'a EffectRecord> {
        let s = surfaces?;
        let m = (material & 0x3f) as usize;
        if m == 0 { return None; }
        s.effects.get(m)
    }

    /// One bullet hit on the world, in `Bullet_CollisionHandler` 0x12ce80's order:
    /// `Effect_Create` 0x1ce688 (debris) → `Effect_Bullet` 0x1ce8a8 (gas puffs + the decal) → the
    /// ricochet roll (`Effect_RicochetProb` 0x1cf290) and reflection [M 0x12d6f8..0x12d7b8].
    ///
    /// `flags70` is `weapon_data+0x70`: bit 2 (0x2) enables the effects (the flags become 0x61 =
    /// bullet + emitter + sound), bit 4 (0x10) enables ricochets. `dir` is the bullet's direction
    /// (BU+0x00), `bounces` its ricochet count (BU+0x50, killed at 3). `owner_is_player`: the shooting
    /// object is class 3 — `Effect_Bullet` skips its `Rand_Rand(2)` for the player's own shots
    /// [M 0x1ce94c: `obj+0xe0 → +0x30`, byte `+0xff == 3`]. Returns the reflected direction when the
    /// bullet ricochets (the caller re-aims the bullet and bumps its bounce count).
    ///
    /// The impact sound itself is `EffectInfo+0x14` (`SurfaceSounds::impact`), already played by the
    /// callers' `impact_sound`.
    pub fn bullet_impact(&mut self, rng: &mut Rng, surfaces: Option<&SurfaceSounds>, gas: &mut crate::gas::GasPool,
                         fx: &mut crate::world_fx::WorldFx,
                         pos: V3, normal: V3, material: u8, dir: V3, flags70: u32, bounces: u8, owner_is_player: bool,
                         ricochet_mask: u32) -> Option<V3> {
        // `Bullet_CollisionHandler` 0x12ce80: the effect flags start 0; weapon_data+0x70 & 2 → 0x61
        // [M 0x12cf58 init, 0x12d198 `| 0x61`]. Without bit 2 the game passes flags 0: no decal, no
        // gas, no debris — only the +0x14 sound (which `Effect_Bullet`'s 0x40 arm plays; the port
        // plays it unconditionally via `impact_sound`).
        let flags: u32 = if flags70 & 0x2 != 0 { 0x61 } else { 0 };
        // Laser-flagged weapons (`+0x6c & 0x100` / `+0x70 & 0x1000`) pass flags 0x10 instead — the
        // port's laser/watch beams are hitscan stand-ins and do not route through here.
        let Some(rec) = Self::record(surfaces, material) else { return None };
        // `Effect_Create` 0x1ce688: GLASS (material & 0x3f == 0x10) negates the normal for the debris
        // basis [M 0x1ce728], restored before `Effect_Bullet`.
        let glass = (material & 0x3f) == 0x10;
        let n = normal;
        if flags & 0x20 != 0 {
            // The emitter arm [M Effect_Create 0x1ce688:81-88, the flags & 0x20 block's FIRST
            // action]: `Emitter_Create(0, 0, EffectInfo+0x84, hit_pos, normal, 0, 0, 0)` 0x17ae00 —
            // a ONE-SHOT burst emitter (rate argument 0 → the +0x4b one-shot latch) at the hit
            // point, oriented by the surface normal, spawning `EffectInfo[material]+0x84`'s def.
            // `Emitter_Create` itself consumes NO RNG (a failed def lookup → no object at all);
            // the draws come on the emitter's OWN update ticks (`Emitter_Update` 0x17b080, 4 ×
            // `Rand_FRandHalf(1.0)` per spawned particle — yaw, pitch, speed, life, that order).
            if rec.emitter != 0 {
                fx.spawn_impact(rec.emitter, pos, n);
            }
        }
        if flags & 0x20 != 0 && rec.debris_enable != 0 && rec.debris_mesh != 0 {
            // `Rand_Rand(EffectInfo+0x83)` → count [M 0x1ce7b0]; `Debris_Create` 0x1aaa00 per particle:
            let count = rng.rand(rec.debris_count as u32) + 1;
            if std::env::var_os("NF_RNGLOG").is_some() {
                eprintln!("[rnglog] debris_count +1 (draw #{})", rng.draws);
            }
            let basis = orthonormal_basis_sum(if glass { [-n[0], -n[1], -n[2]] } else { n });
            for _ in 0..count {
                let scale = rng.frand(0.1) + 0.4;                       // data+0xe8 [M 0x1aaad8]
                let life = (rec.debris_life >> 1) as u32 + rng.rand((rec.debris_life >> 1) as u32); // data+0x30 [M 0x1aaaf4]
                let half = rec.debris_speed * 0.5;
                let jitter = [frand_half(rng, rec.debris_speed), frand_half(rng, rec.debris_speed), frand_half(rng, rec.debris_speed)];
                let vel = [
                    (half * n[0] + basis[0] * jitter[0]) * REC,
                    (half * n[1] + basis[1] * jitter[1]) * REC,
                    (half * n[2] + basis[2] * jitter[2]) * REC,
                ];
                let spin = frand_half_vec(rng, rec.debris_spin);        // data+0x20 [M 0x1aab68]
                self.debris.push(Debris { pos, vel, spin, life: life as i16, scale, mesh: rec.debris_mesh, alpha: 0xff });
            }
        }
        if flags & 0x1 != 0 {
            // `Effect_Bullet`'s gas loop: `Rand_Rand(2)` unless the shooter is the player [M 0x1ce94c].
            if !owner_is_player && rng.rand(2) != 0 {
                // The 1-in-2 kill skips only the puffs; the decal below still runs.
            } else {
                for g in &rec.gas {
                    if g.mesh == 0 { continue; }
                    // Flag +0x1a: wall puffs (offset along the normal) are skipped on floor/ceiling hits
                    // [M 0x1ce990: `if (0.3 < |normal.y|) skip`].
                    let wall = g.wall_offset != 0;
                    if wall && n[1].abs() > 0.3 { continue; }
                    let p = if wall { [pos[0] + n[0] * PUFF_OFFSET, pos[1] + n[1] * PUFF_OFFSET, pos[2] + n[2] * PUFF_OFFSET] } else { pos };
                    // The velocity: wall puffs get `Vec_NormToRot(normal)` (0x1ce96c); floor puffs get
                    // zero or the euler vector by flag +0x1b. The port zeroes both ([S] — the drift is
                    // a visual; `Vec_NormToRot`/`ATAN2_APPROX` are not ported, see the report).
                    let _ = g.orient_normal;
                    let rgba = [((g.rgba[0] >> 24) & 0xff) as u8, ((g.rgba[0] >> 16) & 0xff) as u8, ((g.rgba[0] >> 8) & 0xff) as u8, (g.rgba[0] & 0xff) as u8];
                    gas.init_ex(g.mesh, p, 0.0, [0.0; 3], g.scale[0], g.scale[1], g.kind as u8, rgba, g.rgba[1], g.life as f32);
                }
            }
            // The decal: `EffectInfo+4` != 0 → take a pool node, evicting the OLDEST when the 200 are
            // in use [M 0x1cea20..0x1cea58]; `Mat_Align2Dir` + `hit + normal·0.01` [M 0x1ce9f0]. No RNG.
            if rec.decal != 0 {
                let d = Decal { pos: [pos[0] + n[0] * DECAL_OFFSET, pos[1] + n[1] * DECAL_OFFSET, pos[2] + n[2] * DECAL_OFFSET], normal: n, mesh: rec.decal };
                if self.decals.len() >= 200 { self.decals.remove(0); self.evicted += 1; }
                self.decals.push(d);
            }
        }
        // The ricochet [M 0x12d6f8..0x12d7b8]: weapon_data+0x70 & 0x10, the bullet's bounce count < 3,
        // `Effect_RicochetProb(material, mask)` — mask 8 (the port has no hit-object for world hits,
        // so the 9 variant's `obj+0x3c & 0x400000` never applies). The roll `Rand_Rand(N) & mask`
        // draws only when `EffectInfo+0x1a != 0` [M 0x1cf290].
        if flags70 & 0x10 != 0 && bounces < 3 && rec.ricochet_n != 0 {
            // The mask: 8 for a world hit, 9 when the hit object carries +0x3c & 0x400000, and 0
            // on a PIERCE contact — the roll still draws, but can never reflect [M 0x12d6f8:
            // `uVar16 = 8; … if (bVar7 != 0) uVar16 = 0;`].
            if rng.rand(rec.ricochet_n as u32) as u8 & ricochet_mask as u8 != 0 {
                if std::env::var_os("NF_RNGLOG").is_some() {
                    eprintln!("[rnglog] ricochet_roll +1 (draw #{})", rng.draws);
                }
                // Reflect: `dir + normal·(−(rest+1)·dot(dir,normal))`, normalised, with the restitution
                // read as raw f32 bits from `EffectInfo+0x1c` [M 0x12d704..0x12d7a8].
                let rest = f32::from_bits(rec.restitution_bits);
                let dot = dir[0] * n[0] + dir[1] * n[1] + dir[2] * n[2];
                let k = -(dot * rest + dot);
                let r = [dir[0] + n[0] * k, dir[1] + n[1] * k, dir[2] + n[2] * k];
                let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
                if l > 1e-12 {
                    return Some([r[0] / l, r[1] / l, r[2] / l]);
                }
            }
        }
        None
    }

    /// `Debris_Update` 0x1aac00 per particle: life −1, pos += vel, vel += WldGravity·(1/60)² while
    /// state 0, rotation += spin, the alpha byte steps down by 1 per tick; kill at life ≤ 0.
    pub fn update(&mut self) {
        for d in self.debris.iter_mut() {
            d.life -= 1;
            d.pos = [d.pos[0] + d.vel[0], d.pos[1] + d.vel[1], d.pos[2] + d.vel[2]];
            d.vel = [d.vel[0], d.vel[1] + 0.0 - 9.8 * REC * REC, d.vel[2]];
            d.alpha = d.alpha.saturating_sub(1);
        }
        self.debris.retain(|d| d.life > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_formats::sfx::EffectGas;

    fn rec(decal: u32, ricochet_n: u16) -> EffectRecord {
        EffectRecord {
            decal, ricochet_n, restitution_bits: 0x3f,
            gas: [EffectGas { mesh: 0x0200_0039, kind: 9, rgba: [0x7f7f_7f7f, 0], scale: [0.005, 0.02], life: 32, wall_offset: 1, orient_normal: 0 }, Default::default(), Default::default()],
            debris_mesh: 0x0200_012c, debris_speed: 6.2, debris_spin: 0.785, debris_life: 55, debris_enable: 1, debris_count: 5,
            emitter: 0,
        }
    }

    /// A 23-entry `EffectInfo` table with `r` at material 10 (STONE) — the parse is indexed by the
    /// material byte (`HITDATA+0x50 & 0x3f`).
    fn table(r: EffectRecord) -> SurfaceSounds {
        SurfaceSounds { impact: vec![], footstep: vec![], body: vec![],
            effects: (0..23).map(|k| if k == 10 { r } else { Default::default() }).collect(), ..Default::default() }
    }

    fn fx() -> crate::world_fx::WorldFx { crate::world_fx::WorldFx::default() }

    /// G-041: a hit on a decal-carrying material leaves one decal at `hit + normal·0.01` with the
    /// material's mesh. Sister: material 0 (NONE) and flags without weapon bit 2 leave none.
    #[test]
    fn decal_at_offset_with_material_mesh() {
        let mut e = Effects::new();
        let mut gas = crate::gas::GasPool::default();
        let r = rec(0x0200_0126, 0);
        let surf = table(r);
        let mut rng = Rng::new(1);
        let out = e.bullet_impact(&mut rng, Some(&surf), &mut gas, &mut fx(), [10.0, 2.0, 3.0], [0.0, 0.0, 1.0], 10, [0.0, 0.0, -1.0], 0x2, 0, true, 8);
        assert!(out.is_none());
        assert_eq!(e.decals.len(), 1);
        let d = &e.decals[0];
        assert_eq!(d.mesh, 0x0200_0126);
        assert!((d.pos[2] - (3.0 + DECAL_OFFSET)).abs() < 1e-6, "the decal sits 0.01 along the normal");
        assert_eq!(d.pos[0], 10.0);
        // Sister: no weapon bit 2 → flags 0 → nothing at all (and no RNG draws for gas/debris — the
        // only draws would be the ricochet's, which is gated on the flag too).
        let before = (rng.x, rng.y);
        let mut e2 = Effects::new();
        e2.bullet_impact(&mut rng, Some(&surf), &mut gas, &mut fx(), [1.0, 1.0, 1.0], [0.0, 1.0, 0.0], 10, [0.0, -1.0, 0.0], 0x0, 0, true, 8);
        assert!(e2.decals.is_empty());
        assert_eq!((rng.x, rng.y), before, "flags 0: the game draws nothing at this impact");
    }

    /// The 200-entry FIFO: the 201st decal evicts the first [M Effect_InitLists 0x1cf268, Effect_Bullet
    /// 0x1cea20 — pool full → the OLDEST in-use node is unlinked].
    #[test]
    fn the_201st_decal_evicts_the_first() {
        let mut e = Effects::new();
        let mut gas = crate::gas::GasPool::default();
        let r = rec(0x0200_0126, 0);
        let surf = table(r);
        let mut rng = Rng::new(7);
        for k in 0..201 {
            e.bullet_impact(&mut rng, Some(&surf), &mut gas, &mut fx(), [k as f32, 0.0, 0.0], [0.0, 1.0, 0.0], 10, [0.0, -1.0, 0.0], 0x2, 0, true, 8);
        }
        assert_eq!(e.decals.len(), 200, "the pool holds 200");
        assert_eq!(e.decals[0].pos[0], 1.0, "the first decal (k=0) was evicted");
        assert_eq!(e.decals[199].pos[0], 200.0, "the 201st (k=200) is the newest tail");
        assert_eq!(e.evicted, 1);
    }

    /// `Debris_Create` 0x1aaa00's draw count and physics: count = Rand_Rand(5)+1, per particle 5 draws
    /// (scale, life, 3 velocity) + 3 spin = 8. Sister: debris dies at life 0 and gains gravity.
    #[test]
    fn debris_draw_order_and_update() {
        let mut e = Effects::new();
        let mut gas = crate::gas::GasPool::default();
        let r = rec(0, 0);
        let surf = table(r);
        let mut rng = Rng::new(3);
        e.bullet_impact(&mut rng, Some(&surf), &mut gas, &mut fx(), [0.0; 3], [0.0, 1.0, 0.0], 10, [0.0, -1.0, 0.0], 0x2, 0, true, 8);
        assert!(!e.debris.is_empty(), "stone debris spawns");
        assert!(e.debris.len() <= 6, "count = Rand_Rand(5)+1 ≤ 6");
        for d in &e.debris {
            assert!((0.4..=0.5).contains(&d.scale), "scale = 0.4 + FRand(0.1) [M 0x1aaad8]");
            assert_eq!(d.alpha, 0xff);
        }
        let n = e.debris.len();
        for _ in 0..60 { e.update(); }
        assert!(e.debris.is_empty(), "life ≤ 55 ticks: all debris dead after 60");
        let _ = n;
    }

    /// `Effect_RicochetProb` 0x1cf290 + the reflection [M 0x12d704]: one `Rand_Rand(N)` draw when
    /// N != 0; a hit reflects the direction about the normal with the +0x1c restitution. Sister: a
    /// zero N draws nothing.
    #[test]
    fn ricochet_draws_once_and_reflects() {
        let mut gas = crate::gas::GasPool::default();
        // Force a hit: a big N and a fixed seed — the roll is Rand_Rand(N) & 8.
        let r = rec(0, 0xffff);
        let surf = table(r);
        // Debris disabled so only the ricochet draw happens.
        let r2 = EffectRecord { debris_enable: 0, ..rec(0, 0xffff) };
        let surf2 = table(r2);
        // A grazing shot (dot = −0.0995): the reflection removes the normal component (restitution ≈
        // 0 — the +0x1c bits are a denormal), keeping the tangent. A head-on shot would stop
        // (v' ≈ 0) — the game's own e≈0 does that, which is why ricochets read on graze hits.
        let dir = [1.0, 0.0, -0.1];
        let n = [0.0, 0.0, 1.0];
        let mut found = None;
        for seed in 0..64u32 {
            let mut e = Effects::new();
            let mut rng = Rng::new(seed);
            let before = (rng.x, rng.y);
            let out = e.bullet_impact(&mut rng, Some(&surf2), &mut gas, &mut fx(), [0.0; 3], n, 10, dir, 0x12, 0, true, 8);
            assert_ne!((rng.x, rng.y), before, "N != 0: exactly one Rand_Rand draw (seeds advanced)");
            if let Some(d) = out { found = Some((seed, d)); break; }
        }
        let (_, d) = found.expect("some seed must roll a ricochet with N=0xffff");
        // The tangent (x) is preserved, the normal component is gone: dot(d, n) ≈ 0 and d points back
        // off the wall (d·dir's normal part flipped).
        assert!((d[0] - 0.995).abs() < 0.01, "the tangential component survives");
        assert!(d[2].abs() < 0.01, "the normal component is removed (e ≈ 0)");
        assert!((d[0] * d[0] + d[1] * d[1] + d[2] * d[2] - 1.0).abs() < 1e-4, "normalised");
        // Sister: N = 0 → no draw, no reflection.
        let r3 = EffectRecord { debris_enable: 0, ..rec(0, 0) };
        let surf3 = table(r3);
        let mut e3 = Effects::new();
        let mut rng3 = Rng::new(1);
        let before3 = (rng3.x, rng3.y);
        assert!(e3.bullet_impact(&mut rng3, Some(&surf3), &mut gas, &mut fx(), [0.0; 3], n, 10, dir, 0x12, 0, true, 8).is_none());
        assert_eq!((rng3.x, rng3.y), before3, "N = 0: Effect_RicochetProb draws nothing");
    }

    /// `Vec_MakeOrthonormalBasis` 0x1e4b78: the basis sum is orthogonal to the normal.
    #[test]
    fn basis_sum_is_tangential() {
        for n in [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.6, 0.8, 0.0]] {
            let s = orthonormal_basis_sum(n);
            let dot = s[0] * n[0] + s[1] * n[1] + s[2] * n[2];
            assert!(dot.abs() < 1e-5, "u+v lies in the surface plane");
        }
    }
}
