//! The gas/smoke puffs: `Gas_Init` 0x1aacc8 (creator) and `gases_smoke` 0x1ab2d0 (control class 0xE's
//! update, docs/atlas/object.md:108). Every caller passes the same mesh hash 0x0200039 (`smoke_1_dust`,
//! a 4-vertex/2-triangle quad [D gfx-list]) via `hashtable_set_object_to_entity_gfx` [M 0x1aacc8:81], so
//! a puff is a growing, fading, spinning instance of one shared quad.
//!
//! Game model [M 0x1aacc8]:
//! - obj+0xE8 = scale start (param 1); data+0x34 = (param 2 − param 1) / life (the growth rate);
//!   data+0x38 = life (param 11, ticks — the update decrements it by FRAME_RATE_MUL = 1 per tick);
//!   data+0x3C byte = the behaviour type (param 8).
//! - data+0x20..0x2C = the current rgba as floats, started from the packed param 9 (e.g. the muzzle
//!   0x48403080 = r 0x48 g 0x40 b 0x30 a 0x80) and moved toward the packed param 10 (the game's callers
//!   pass an integer whose float bits are ~0, i.e. fade to zero) by data+0x10..0x1C rates each tick
//!   [M 0x1aacc8:56-71, 0x1ab2d0 tail]; obj+0x103..0x106 carry the byte form the draw reads.
//! - data+0x30 = the spin rate: `Rand_FRand(π/2 · REC_FRAME_RATE)`, negated when `Rand_Rand(0x100) > 0x7f`
//!   [M 0x1aacc8:72-80] — 2 shared-RNG draws per puff.
//! - Velocity by type [M 0x1aacc8:89-141]: type 4 = the given velocity · REC_FRAME_RATE with a ±0.05
//!   x/z jitter (2 draws); type 7 = `Env_GetWindVel` + jitter (0x3e4ccccd = 0.2, 3 draws); the default
//!   = (0, REC_FRAME_RATE, 0)?? — the decomp's `pfVar1[1] = fVar10` with fVar10 = REC_FRAME_RATE.
//! - Update (`gases_smoke` 0x1ab2d0) per type: 0 = vel damped ×0.95/frame + slight gravity, scale grows;
//!   4 = pos += vel (+ gravity·0.01), vel damped; 7 = scale grows, pure buoyancy drift (no own velocity);
//!   9/10 = hidden emitters (not ported); the tail fades rgba and kills at life ≤ 0 (obj+0xFE |= 1).
//!
//! RNG discipline: the game consumes 5 shared-RNG draws per puff (2 spin, 2-3 velocity). The port does
//! not consume them — the shared bit-exact stream must not gain draws (G-033's discipline, searchlight.rs
//! note) — and derives the jitter from a counter hash instead. Same visible behaviour, deterministic
//! without touching the stream; the deviation is listed in docs/models/misc.md.

use crate::world::V3;

/// The one shared dust quad (`smoke_1_dust`), every `Gas_Init` caller's mesh [M 0x1aacc8:81 callers;
/// D gfx-list 07000029 entry 64 obj 0].
pub const SMOKE_QUAD: u32 = 0x0200_0039;

/// REC_FRAME_RATE (0x30d0cc) = 1/60 [M grenade.rs:52].
const REC_FRAME_RATE: f32 = 1.0 / 60.0;
/// WldGravity [L grenade.rs:51; M the global the update's `_WldGravity` reads].
const WLD_GRAVITY: V3 = [0.0, -9.8, 0.0];

#[derive(Clone, Debug, PartialEq)]
pub struct Gas {
    pub pos: V3,
    pub vel: V3,
    /// The puff's mesh hash (`Gas_Init`'s hashtable_set_object_to_entity_gfx argument); the callers'
    /// common case is SMOKE_QUAD, the `Effect_Bullet` entries carry their own (0x02000124 hard-surface,
    /// 0x02000129 water [D EffectInfo@0x3037c0]). fix-impacts, additive field.
    pub mesh: u32,
    pub scale: f32,
    pub scale_rate: f32,
    /// The current rgba as floats (data+0x20), byte-clamped into the draw colour each tick.
    pub rgba: [f32; 4],
    pub rgba_rate: [f32; 4],
    /// data+0x30: the spin rate (radians/tick); the draw billboards the quad with it.
    pub spin: f32,
    /// data+0x38: ticks left; the update kills the object at ≤ 0.
    pub life: f32,
    /// data+0x3C byte: 0 = static damped, 4 = given velocity, 7 = buoyant drift.
    pub kind: u8,
}

#[derive(Default)]
pub struct GasPool {
    pub puffs: Vec<Gas>,
    /// The spawn counter feeding the deterministic jitter (the game's 5 RNG draws, un-consumed).
    counter: u32,
}

/// `Rand_Rand(0x100) > 0x7f`-quality sign/offset from the counter: a 32-bit bit-mix (xxhash-style), no
/// shared-stream involvement. The game's draws come from Rand_FRand — the port substitutes this [S].
fn jitter(counter: u32, salt: u32) -> f32 {
    let mut h = counter ^ salt;
    h = h.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    // 0..1
    (h & 0xffff) as f32 / 65536.0
}

impl GasPool {
    /// Slot cap: the muzzle puffs live 1 tick life... the game's searchlight break re-fires every 7th
    /// frame with the same parameters, so a few dozen live at once in play; 128 covers grenade chains.
    pub const CAP: usize = 128;

    /// `Gas_Init` 0x1aacc8 minus the RNG draws (see the module doc). `p1`/`p2` = the scale start/end,
    /// `rgba` = the packed param 9 (r, g, b, a), the target rgba is the callers' ~0 (fade out),
    /// `yaw` = the quaternion z the callers pass (a random half-open π for the muzzle/searchlight,
    /// unused by the port's billboard draw beyond the spin), `vel` = the type-4 velocity vector.
    #[allow(clippy::too_many_arguments)]
    pub fn init(&mut self, pos: V3, yaw: f32, vel: V3, p1: f32, p2: f32, kind: u8, rgba: [u8; 4], life: f32) {
        self.init_ex(crate::gas::SMOKE_QUAD, pos, yaw, vel, p1, p2, kind, rgba, 0, life);
    }

    /// The full form the `Effect_Bullet` gas entries need [M disasm 0x1ce964..0x1cea90]: the entry's own
    /// mesh hash (the callers' SMOKE_QUAD is only the common case) and the packed rgba END (the game
    /// fades start→end; `init`'s callers pass ~0). Fix-impacts hook, additive.
    #[allow(clippy::too_many_arguments)]
    pub fn init_ex(&mut self, mesh: u32, pos: V3, yaw: f32, vel: V3, p1: f32, p2: f32, kind: u8, rgba: [u8; 4], rgba_end: u32, life: f32) {
        if life <= 0.0 || self.puffs.len() >= Self::CAP { return; }
        let n = self.counter.wrapping_add(0x9e37_79b9);
        self.counter = n;
        // The spin rate: Rand_FRand(π/2 · REC_FRAME_RATE), sign from the coin [M 0x1aacc8:72-80]. The
        // game's value is per-tick; the port keeps the magnitude and derives the sign deterministically.
        let spin = jitter(n, 1) * (std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE)
            * if jitter(n, 2) > 0.5 { 1.0 } else { -1.0 };
        // Velocity by type [M 0x1aacc8:89-141]: 4 = given · REC + ±0.05 x/z jitter; 7 = the wind
        // (Env_GetWindVel — the port's rooms carry no wind radiators for these sources [S]) + the
        // (rand−0.1, rand, rand−0.1) spread over 0.2; else the decomp's (0, REC, 0) default.
        let vel = match kind {
            4 => [vel[0] * REC_FRAME_RATE + (jitter(n, 3) - 0.025), vel[1] * REC_FRAME_RATE, vel[2] * REC_FRAME_RATE + (jitter(n, 4) - 0.025)],
            7 => [(jitter(n, 3) - 0.1) * 0.2, jitter(n, 5) * 0.2, (jitter(n, 4) - 0.1) * 0.2],
            _ => [0.0, REC_FRAME_RATE, 0.0],
        };
        let _ = yaw;
        // The fade target: the entry's packed rgba end (the game's param 10 [M 0x1aacc8:56-71]).
        let end = [(rgba_end >> 24) as u8, (rgba_end >> 16) as u8, (rgba_end >> 8) as u8, rgba_end as u8];
        self.puffs.push(Gas {
            pos, vel,
            mesh,
            scale: p1,
            scale_rate: (p2 - p1) / life,
            rgba: rgba.map(f32::from),
            rgba_rate: [
                (f32::from(end[0]) - f32::from(rgba[0])) / life,
                (f32::from(end[1]) - f32::from(rgba[1])) / life,
                (f32::from(end[2]) - f32::from(rgba[2])) / life,
                (f32::from(end[3]) - f32::from(rgba[3])) / life,
            ],
            spin,
            life,
            kind,
        });
        // The mesh rides alongside for the draw (the game passes it to the gfx hashtable); the port's
        // billboard draw reads it from the puff via the pool index.
        if let Some(g) = self.puffs.last_mut() { g.mesh = mesh; }
    }

    /// `gases_smoke` 0x1ab2d0 for the ported types (0, 4, 7): motion, growth, fade, the life countdown
    /// and the kill. Types 9/10 (hidden emitters) and 0xb are not ported; no game caller in the port's
    /// scope uses them yet.
    pub fn update(&mut self) {
        self.puffs.retain_mut(|g| {
            match g.kind {
                // Case 0 [M 0x1ab2d0 case 0]: vel damped ×0.95, gravity scaled −0.02, pos += vel.
                0 => {
                    for k in 0..3 { g.vel[k] *= 0.95; }
                    for k in 0..3 { g.pos[k] += g.vel[k] + WLD_GRAVITY[k] * -0.02 * REC_FRAME_RATE; }
                }
                // Case 4 [M 0x1ab2d0 case 4]: pos += vel + gravity·0.01, vel damped.
                4 => {
                    for k in 0..3 { g.pos[k] += g.vel[k] + WLD_GRAVITY[k] * 0.01 * REC_FRAME_RATE; }
                    for k in 0..3 { g.vel[k] *= 0.95 - REC_FRAME_RATE; }
                }
                // Case 7 [M 0x1ab2d0 case 7]: no own velocity; gravity·0.05 drift.
                7 => {
                    for k in 0..3 { g.pos[k] += WLD_GRAVITY[k] * 0.05 * REC_FRAME_RATE; }
                }
                _ => {}
            }
            g.scale += g.scale_rate;
            for k in 0..4 { g.rgba[k] = (g.rgba[k] + g.rgba_rate[k]).clamp(0.0, 255.0); }
            g.life -= 1.0;
            g.life > 0.0
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_puff_grows_fades_and_dies() {
        let mut pool = GasPool::default();
        pool.init([1.0, 2.0, 3.0], 0.0, [0.0; 3], 0.05, 0.10, 7, [0x48, 0x40, 0x30, 0x80], 8.0);
        assert_eq!(pool.puffs.len(), 1);
        let g0 = pool.puffs[0].clone();
        pool.update();
        let g1 = &pool.puffs[0];
        assert!(g1.scale > g0.scale, "the puff grows");
        assert!(g1.rgba[3] < g0.rgba[3], "the puff fades");
        let mut n = 0;
        while !pool.puffs.is_empty() { pool.update(); n += 1; assert!(n < 20, "the puff outlived its life"); }
        assert_eq!(n, 7, "life 8 = 8 ticks (1 spent before the count)");
    }

    #[test]
    fn the_pool_is_deterministic_and_capped() {
        let mut a = GasPool::default();
        let mut b = GasPool::default();
        for i in 0..64 { a.init([i as f32, 0.0, 0.0], 0.0, [0.0; 3], 0.05, 0.1, 7, [72, 64, 48, 128], 4.0); }
        for i in 0..64 { b.init([i as f32, 0.0, 0.0], 0.0, [0.0; 3], 0.05, 0.1, 7, [72, 64, 48, 128], 4.0); }
        assert_eq!(a.puffs, b.puffs, "same calls, same puffs (no wall-clock)");
        for _ in 0..200 { a.init([0.0; 3], 0.0, [0.0; 3], 0.05, 0.1, 7, [72, 64, 48, 128], 4.0); }
        assert_eq!(a.puffs.len(), GasPool::CAP, "the pool caps");
    }
}
