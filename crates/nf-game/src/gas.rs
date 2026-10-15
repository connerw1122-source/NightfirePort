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
//! - Velocity by type [M 0x1aacc8:89-141]: type 4 = the given velocity · REC_FRAME_RATE with a +[0,0.05)
//!   x/z jitter (2 draws, 0x1ab21c/0x1ab248); type 7 with a NULL velocity = `Env_GetWindVel` + the
//!   `(rand−0.1, rand, rand−0.1)·0.2` spread (0x3e4ccccd = 0.2, 3 draws, 0x1ab160/0x1ab184/0x1ab1a4);
//!   the default = (0, REC_FRAME_RATE, 0)?? — the decomp's `pfVar1[1] = fVar10` with fVar10 = REC_FRAME_RATE.
//! - The decompile drops `Gas_Init`'s 11th argument (the life, a stack ushort) at EVERY call site — the
//!   disassembly passes it [M 0x1aad24 `lhu 0xc0(sp)`; the searchlight site 0x1cbce0/0x1cbcb0 = 0x80].
//! - Update (`gases_smoke` 0x1ab2d0) per type: 0 = vel damped ×0.95/frame + slight gravity, scale grows;
//!   4 = pos += vel (+ gravity·0.01), vel damped; 7 = scale grows, pure buoyancy drift (no own velocity);
//!   9/10 = hidden emitters (not ported); the tail fades rgba and kills at life ≤ 0 (obj+0xFE |= 1).
//!
//! RNG discipline (fix-gas-rng): `Gas_Init` 0x1aacc8's own draws are 2 shared-stream draws per puff
//! — the spin `Rand_FRand(π/2 · REC_FRAME_RATE)` (0x1ab060) and the sign coin `Rand_Rand(0x100)`
//! (0x1ab08c, negate at ≥ 0x80) — plus the velocity branch: type 7 with a NULL velocity pointer
//! takes `Env_GetWindVel` + 3 × `Rand_FRand(0.2)` (0x1ab160/0x1ab184/0x1ab1a4); type 4 with a
//! NON-NULL pointer adds 2 × `Rand_FRand(0.05)` (0x1ab21c/0x1ab248). The callers add their own
//! draws before the call (the muzzle's quaternion-z `Rand_FRand(2π)` 0x1a43ec, the searchlight's
//! rot-z 0x1cbca0, the trail's 6.25 0x12cc18). A muzzle or searchlight puff therefore costs the
//! stream exactly 6 draws (1 caller + 2 spin + 3 wind — kind 7 + NULL vel); an `Effect_Bullet`
//! impact puff costs 2 + 3 for its kind-7 entries (the game passes the velocity pointer NULL
//! there [M 0x1ce8a8 a3 = 0], so kind 4 entries fall to the rising default with no extra draws).
//! The real-stream sites below (`init_muzzle_smoke`, `init_effect_puff`, and the merged
//! searchlight/trail/exhaust fns) consume those draws from the sim's game RNG before the pool-cap
//! check — the game's RNG has no notion of the port's cap, and a skipped draw would shift the
//! shared stream. The remaining counter-hash sites (`init`/`init_ex`: the drone.rs kind-5 puffs,
//! the 0xb crate puff, the world_fx ripples) are each their own game call site whose caller-side
//! draws are not re-derived yet — they keep the hash until their own sites are re-based (listed in
//! docs/systems/gas.md).

use crate::world::V3;

/// The one shared dust quad (`smoke_1_dust`), every `Gas_Init` caller's mesh [M 0x1aacc8:81 callers;
/// D gfx-list 07000029 entry 64 obj 0].
pub const SMOKE_QUAD: u32 = 0x0200_0039;

/// REC_FRAME_RATE (0x30d0dc — GS_SetRefreshRate 0x17d2b8 sets it = 1/FRAME_RATE; 0x30d0cc is
/// FRAME_RATE_INT) = 1/60 [M symtab; grenade.rs:52].
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

    /// `Gas_Init` 0x1aacc8 minus the RNG draws (see the module doc). LEGACY counter-hash form —
    /// kept for the not-yet-rebased sites (drone.rs's kind-5 puffs, the 0xb crate puff, the
    /// world_fx ripples); the re-based real-stream sites are `init_muzzle_smoke` and
    /// `init_effect_puff`. `p1`/`p2` = the scale start/end,
    /// `rgba` = the packed param 9 (r, g, b, a), the target rgba is the callers' ~0 (fade out),
    /// `yaw` = the quaternion z the callers pass (a random half-open π for the muzzle/searchlight,
    /// unused by the port's billboard draw beyond the spin), `vel` = the type-4 velocity vector.
    #[allow(clippy::too_many_arguments)]
    pub fn init(&mut self, pos: V3, yaw: f32, vel: V3, p1: f32, p2: f32, kind: u8, rgba: [u8; 4], life: f32) {
        self.init_ex(crate::gas::SMOKE_QUAD, pos, yaw, vel, p1, p2, kind, rgba, 0, life);
    }

    /// `Gas_Init` for the broken-searchlight site (`Searchlight_Update` 0x1cb900:373-385), consuming the
    /// game's 6 shared-RNG draws in order [M disassembly]:
    /// 1. the caller's rotation draw `Rand_FRand(π)` (0x1cbca0, the rot-z euler — 0x40c90fdb);
    /// 2. the spin `Rand_FRand(π/2 · REC_FRAME_RATE)` (0x1ab060);
    /// 3. the spin sign `Rand_Rand(0x100)` — negate when ≥ 0x80 (0x1ab08c, decomp :77-80);
    /// 4-6. the type-7/NULL-vel wind spread `Rand_FRand(0.2)` ×3 (0x1ab160/0x1ab184/0x1ab1a4):
    ///     `vel = (wind.x + r − 0.1, wind.y + r, wind.z + r − 0.1) · REC_FRAME_RATE`, wind =
    ///     `Env_GetWindVel` 0x17c048 = 0 with no cel wind radiator (none ported — [S] the cel wind
    ///     system stays unported). Fixed args of the site: p1 0.05, p2 0.25, type 7, rgba 0x48403080,
    ///     end 1 (~0 float bits), life 128 ticks (the decomp-dropped 11th arg = 0x80 [M 0x1cbce0]).
    /// The draws are consumed BEFORE the pool-cap check — the game's RNG has no notion of the port's
    /// cap, and a skipped draw would shift the shared stream.
    pub fn init_searchlight_smoke(&mut self, rng: &mut crate::drone::Rng, pos: V3) {
        let yaw = rng.frand(std::f32::consts::PI);
        let spin_mag = rng.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE);
        let spin = if rng.rand(0x100) >= 0x80 { -spin_mag } else { spin_mag };
        let (rx, ry, rz) = (rng.frand(0.2), rng.frand(0.2), rng.frand(0.2));
        if self.puffs.len() >= Self::CAP { return; }
        let vel = [rx - 0.1, ry, rz - 0.1].map(|j| mul_ee(j, REC_FRAME_RATE));
        self.puffs.push(Gas {
            pos,
            vel,
            mesh: SMOKE_QUAD,
            scale: 0.05,
            scale_rate: (0.25 - 0.05) / 128.0,
            rgba: [0x48 as f32, 0x40 as f32, 0x30 as f32, 0x80 as f32],
            rgba_rate: [-0x48 as f32 / 128.0, -0x40 as f32 / 128.0, -0x30 as f32 / 128.0, -0x80 as f32 / 128.0],
            spin,
            life: 128.0,
            kind: 7,
        });
        let _ = yaw; // the rot-z euler: stored to obj+0x50/0x60 in the game; the port's billboard draw reads only the spin
    }

    /// The projectile smoke-trail site (`Bullet_trail_effect_smoke` 0x12cba8 fast arm, called from
    /// `Bullet_DoTrails` 0x12d890 when the weapon record's `+0x6C` carries bit 0x2 [M decomp
    /// :144-146]), every 4th game tick (`(GameState tick + obj[0xec]) & 3 == 0` [M disasm
    /// 0x12cbf4-0x12cc08 — obj+0xec is the creation tick, so the sum is the current tick]) and
    /// behind the `MPSettings` trail-detail gate (word 384, 0 = trails on; .bss zero at boot [L
    /// MPSettings 0x2a47a0+384, `LS_LoadMPSettings` never sets it on this build]).
    /// Fixed args [M disasm 0x12cc10-0x12cc74]: scale 0.1 → 0.5, mesh 0x0200001f, kind 7,
    /// rgba 0x404040ff → 0x40404001, life 120 ticks (the decomp-dropped 11th arg = 0x78 [M
    /// 0x12cc60 `sw $v0, ($sp)` with v0 = 0x78]), velocity (0, 0, 0) — the game passes a non-NULL
    /// zero vector so `Gas_Init`'s given-velocity branch stores it with NO wind draws [M 0x1aacc8
    /// :106-111]. Draws [M]: the caller's `Rand_FRand(6.25)` (0x12cc18, 0x40c90fdb) + the 2 spin
    /// draws inside `Gas_Init` (0x1aacc8:72-80) = 3 per puff, consumed before the pool-cap check
    /// (the searchlight precedent). The kind-7 update is pure buoyancy — the puff hangs and fades.
    /// Records with bit 0x2 [L ee_start_05.bin @ 0x2bf150 sweep]: 0x1b (the AR's GL), 0x36 (the
    /// stun grenade), 0x69 — all gravity bullets (+0x6C & 8), so the port's Grenade path.
    pub fn init_trail_smoke(&mut self, rng: &mut crate::drone::Rng, pos: V3) {
        let yaw = rng.frand(6.25);
        let spin_mag = rng.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE);
        let spin = if rng.rand(0x100) >= 0x80 { -spin_mag } else { spin_mag };
        if self.puffs.len() >= Self::CAP { return; }
        self.puffs.push(Gas {
            pos,
            vel: [0.0; 3],
            mesh: 0x0200_001f,
            scale: 0.1,
            scale_rate: (0.5 - 0.1) / 120.0,
            rgba: [0x40 as f32, 0x40 as f32, 0x40 as f32, 0xff as f32],
            rgba_rate: [0.0, 0.0, 0.0, (1.0 - 255.0) / 120.0],
            spin,
            life: 120.0,
            kind: 7,
        });
        let _ = yaw; // the rotation draw: stored to obj+0x50/0x60; the port's billboard reads only the spin
    }

    /// The rocket-exhaust site (`Bullet_DoTrails` 0x12d890 bit 0x20 arm, decomp :152-166), every 3rd
    /// game tick (`% 3 == 0` [M disasm 0x12d9f4-0x12da14]) behind the same MPSettings gate.
    /// Fixed args [M disasm 0x12da34-0x12da84]: scale 0.1 → 0.4, mesh SMOKE_QUAD 0x02000039, kind 0
    /// (the damped-velocity drift), rgba 0x7f7f7f7f → 0x80808000, life 64 ticks (v0 = 0x40 at
    /// 0x12da38), velocity (0, 0, `Rand_FRand(6.25)`) — the caller's draw is stored as the z of the
    /// velocity vector [M 0x12da2c jal Rand_FRand; 0x12da28/0x12da30 zero stores; 0x12da6c
    /// `swc1 $f0, 0x28($sp)`], so kind 0's `vel · REC_FRAME_RATE` gives a ≤0.104/tick drift.
    /// Draws: the caller's 1 + the 2 spin draws = 3 per puff. Records with bit 0x20 [L sweep]:
    /// 0x2a/0x2b (the Militek MGL twins).
    pub fn init_exhaust_puff(&mut self, rng: &mut crate::drone::Rng, pos: V3) {
        let vz = rng.frand(6.25);
        let spin_mag = rng.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE);
        let spin = if rng.rand(0x100) >= 0x80 { -spin_mag } else { spin_mag };
        if self.puffs.len() >= Self::CAP { return; }
        self.puffs.push(Gas {
            pos,
            vel: [0.0, 0.0, vz * REC_FRAME_RATE],
            mesh: SMOKE_QUAD,
            scale: 0.1,
            scale_rate: (0.4 - 0.1) / 64.0,
            rgba: [0x7f as f32, 0x7f as f32, 0x7f as f32, 0x7f as f32],
            rgba_rate: [
                (0x80 as f32 - 0x7f as f32) / 64.0,
                (0x80 as f32 - 0x7f as f32) / 64.0,
                (0x80 as f32 - 0x7f as f32) / 64.0,
                (0x00 as f32 - 0x7f as f32) / 64.0,
            ],
            spin,
            life: 64.0,
            kind: 0,
        });
    }

    /// The player's muzzle-smoke site (`Player_MuzzleFlash` 0x1a4298, the 0x68 & 0x2000 == 0 arm),
    /// fix-gas-rng: consumes `Gas_Init`'s internal draws in the game's order from the sim's shared
    /// RNG — the spin `Rand_FRand(π/2 · REC_FRAME_RATE)` (0x1ab060), the sign coin `Rand_Rand(0x100)`
    /// (0x1ab08c, negate at ≥ 0x80), then the kind-7/NULL-vel wind spread `Rand_FRand(0.2)` ×3
    /// (0x1ab160/0x1ab184/0x1ab1a4) — 5 draws; the caller's quaternion-z `Rand_FRand(2π)`
    /// (0x1a43ec, 0x40c90fdb) is drawn by the scene right before this call, keeping the game's
    /// order. Fixed args [M disasm 0x1a4404-0x1a442c]: scale 0.05 → 0.1 (f12 0x3d4ccccd /
    /// f13 0x3dcccccd), velocity pointer NULL (a3 = 0) so kind 7 takes the wind branch, mesh
    /// SMOKE_QUAD (t0 0x2000039), kind 7 (t1), rgba start 0x48403080 (t2), rgba end 1 (t3 — the
    /// game unpacks its bytes, so the alpha end byte is 1, not 0), life 0x80 = 128 ticks (the
    /// stack word the decompile drops). `Env_GetWindVel` 0x17c048 returns 0 — the port's cels
    /// carry no wind radiators (none ported, the searchlight precedent [S]).
    /// Draws are consumed BEFORE the pool-cap check (the game's RNG has no notion of the cap).
    pub fn init_muzzle_smoke(&mut self, rng: &mut crate::drone::Rng, pos: V3) {
        let spin_mag = rng.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE); // 0x1ab060
        let spin = if rng.rand(0x100) >= 0x80 { -spin_mag } else { spin_mag };  // 0x1ab08c
        let (rx, ry, rz) = (rng.frand(0.2), rng.frand(0.2), rng.frand(0.2));    // 0x1ab160/184/1a4
        if self.puffs.len() >= Self::CAP { return; }
        let inv = 1.0 / 128.0; // the game's 1/life reciprocal form [M 0x1aacc8:51]
        self.puffs.push(Gas {
            pos,
            vel: [mul_ee(rx - 0.1, REC_FRAME_RATE), mul_ee(ry, REC_FRAME_RATE), mul_ee(rz - 0.1, REC_FRAME_RATE)],
            mesh: SMOKE_QUAD,
            scale: 0.05,
            scale_rate: (0.1 - 0.05) * inv,
            rgba: [0x48 as f32, 0x40 as f32, 0x30 as f32, 0x80 as f32],
            rgba_rate: [
                (0.0 - 0x48 as f32) * inv,
                (0.0 - 0x40 as f32) * inv,
                (0.0 - 0x30 as f32) * inv,
                (1.0 - 0x80 as f32) * inv,
            ],
            spin,
            life: 128.0,
            kind: 7,
        });
    }

    /// The `Effect_Bullet` 0x1ce8a8 impact-puff site (the gas loop at 0x1ce94c-0x1cea90, 3 entries
    /// per surface record), fix-gas-rng: consumes `Gas_Init`'s internal draws from the sim's shared
    /// RNG — the spin (0x1ab060), the sign coin (0x1ab08c), and for kind-7 entries the wind spread
    /// ×3 (0x1ab160/0x1ab184/0x1ab1a4). The game passes the velocity pointer NULL at this site
    /// (a3 = 0 [M 0x1ce8a8 call]), so the dispatch is the NULL arm: kind 7 → wind; kinds 9/10 →
    /// the position-copy arm (LAB_001ab1e4, kind 10 first halves the life [M 0x1aacc8:122]); every
    /// other kind (4 included — the given-velocity jitter branch needs a NON-NULL pointer) → the
    /// rising default (0, REC_FRAME_RATE, 0) [M 0x1aacc8:118-128]. Kind 0xb scales the spin ×0.25
    /// [M :74-76]. The life gate runs BEFORE the draws (the game's entry `param_11 != 0` check
    /// [M :31] — a 0-life entry creates no object and draws nothing); the pool cap stays after
    /// them. rgba = the entry's packed start (param 9), rgba_end = the entry's packed end
    /// (param 10), rates = (end byte − start byte) · 1/life in the game's reciprocal form.
    #[allow(clippy::too_many_arguments)]
    pub fn init_effect_puff(&mut self, rng: &mut crate::drone::Rng, mesh: u32, pos: V3, kind: u8,
                            rgba: [u8; 4], rgba_end: u32, p1: f32, p2: f32, life: f32) {
        if life <= 0.0 { return; } // the game's entry gate — no object, no draws [M 0x1aacc8:31]
        let mut spin_mag = rng.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE); // 0x1ab060
        if kind == 0xb { spin_mag *= 0.25; } // [M 0x1aacc8:74-76]
        let spin = if rng.rand(0x100) >= 0x80 { -spin_mag } else { spin_mag };      // 0x1ab08c
        let mut life = life;
        let vel = if kind == 7 {
            let (rx, ry, rz) = (rng.frand(0.2), rng.frand(0.2), rng.frand(0.2));    // 0x1ab160/184/1a4
            [mul_ee(rx - 0.1, REC_FRAME_RATE), mul_ee(ry, REC_FRAME_RATE), mul_ee(rz - 0.1, REC_FRAME_RATE)]
        } else if kind == 9 || kind == 10 {
            if kind == 10 { life *= 0.5; } // [M 0x1aacc8:122]
            pos                           // LAB_001ab1e4: the vel fields copy the position
        } else {
            [0.0, REC_FRAME_RATE, 0.0]    // the rising default [M 0x1aacc8:118-128]
        };
        if self.puffs.len() >= Self::CAP { return; }
        let inv = 1.0 / life;
        let end = [(rgba_end >> 24) as u8, (rgba_end >> 16) as u8, (rgba_end >> 8) as u8, rgba_end as u8];
        self.puffs.push(Gas {
            pos, vel, mesh,
            scale: p1,
            scale_rate: (p2 - p1) * inv,
            rgba: rgba.map(f32::from),
            rgba_rate: [
                (f32::from(end[0]) - f32::from(rgba[0])) * inv,
                (f32::from(end[1]) - f32::from(rgba[1])) * inv,
                (f32::from(end[2]) - f32::from(rgba[2])) * inv,
                (f32::from(end[3]) - f32::from(rgba[3])) * inv,
            ],
            spin,
            life,
            kind,
        });
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

/// The EE's `mul.s`: the port's ps2float::mul (the game's velocity scale runs on the EE FPU).
fn mul_ee(a: f32, b: f32) -> f32 { nf_formats::ps2float::mul(a, b) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searchlight_smoke_consumes_six_shared_draws_in_order() {
        // [M 0x1cbca0, 0x1ab060, 0x1ab08c, 0x1ab160, 0x1ab184, 0x1ab1a4] — the puff's fields must equal
        // the six draws replayed on a twin stream, bit for bit.
        let mut rng = crate::drone::Rng::new(0x5e4c04);
        let mut pool = GasPool::default();
        pool.init_searchlight_smoke(&mut rng, [1.0, 2.0, 3.0]);
        assert_eq!(pool.puffs.len(), 1);
        let g = &pool.puffs[0];
        assert_eq!(g.life, 128.0, "the decomp-dropped 11th arg = 0x80 [M 0x1cbce0]");
        assert_eq!(g.kind, 7);
        assert_eq!(g.mesh, SMOKE_QUAD);
        assert_eq!(g.scale, 0.05);
        assert_eq!(g.scale_rate, (0.25 - 0.05) / 128.0);
        assert_eq!(g.rgba, [0x48 as f32, 0x40 as f32, 0x30 as f32, 0x80 as f32]);
        // vel = (wind + (r−0.1), wind + r, wind + (r−0.1)) · REC with wind 0 — three draws 4..6,
        // replayed in order after the spin draws.
        let mut r = crate::drone::Rng::new(0x5e4c04);
        let _rot_z = r.frand(std::f32::consts::PI); // draw 1, the caller's euler
        let spin = r.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE);
        let sign = r.rand(0x100);
        let expect_spin = if sign >= 0x80 { -spin } else { spin };
        let (rx, ry, rz) = (r.frand(0.2), r.frand(0.2), r.frand(0.2));
        assert_eq!(g.spin.to_bits(), expect_spin.to_bits(), "the spin = draw 2 with the draw-3 sign");
        assert_eq!(g.vel[0].to_bits(), mul_ee(rx - 0.1, REC_FRAME_RATE).to_bits());
        assert_eq!(g.vel[1].to_bits(), mul_ee(ry, REC_FRAME_RATE).to_bits());
        assert_eq!(g.vel[2].to_bits(), mul_ee(rz - 0.1, REC_FRAME_RATE).to_bits());
        // The whole stream advanced by exactly 6 draws: the twin consumed the same six in the same
        // order, so a seventh replay continues both streams identically.
        let mut a = crate::drone::Rng::new(0x5e4c04);
        let mut pool2 = GasPool::default();
        pool2.init_searchlight_smoke(&mut a, [0.0; 3]);
        let mut b = crate::drone::Rng::new(0x5e4c04);
        for _ in 0..6 { b.frand(1.0); }
        let a7 = a.frand(1.0).to_bits();
        let b7 = b.frand(1.0).to_bits();
        assert_eq!(a7, b7, "the pool consumed exactly the game's 6 draws");
        assert_eq!(pool.puffs[0].pos, [1.0, 2.0, 3.0], "the puff sits at the head");
    }

    #[test]
    fn muzzle_smoke_consumes_the_games_five_internal_draws_in_order() {
        // [M 0x1ab060, 0x1ab08c, 0x1ab160, 0x1ab184, 0x1ab1a4] — the puff's fields must equal the
        // five draws replayed on a twin stream, bit for bit (the caller's 2π draw is the scene's,
        // one slot earlier in the game's order).
        let mut rng = crate::drone::Rng::new(0x5e4c04);
        let mut pool = GasPool::default();
        pool.init_muzzle_smoke(&mut rng, [1.0, 2.0, 3.0]);
        assert_eq!(pool.puffs.len(), 1);
        let g = &pool.puffs[0];
        // Fixed args [M 0x1a4404-0x1a442c]: 0.05 → 0.1 over 0x80 ticks, kind 7, SMOKE_QUAD,
        // rgba 0x48403080 with the end-1 bytes (alpha end = 1, not 0).
        assert_eq!(g.life, 128.0, "the stack word the decompile drops = 0x80 ticks");
        assert_eq!(g.kind, 7);
        assert_eq!(g.mesh, SMOKE_QUAD);
        assert_eq!(g.scale, 0.05);
        assert_eq!(g.rgba, [0x48 as f32, 0x40 as f32, 0x30 as f32, 0x80 as f32]);
        assert_eq!(g.rgba_rate[3], (1.0 - 0x80 as f32) / 128.0, "the end-1 bytes reach the alpha rate");
        // vel = (r−0.1, r, r−0.1) · REC with wind 0 — draws 3..5, replayed in order.
        let mut r = crate::drone::Rng::new(0x5e4c04);
        let spin = r.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE);
        let sign = r.rand(0x100);
        let expect_spin = if sign >= 0x80 { -spin } else { spin };
        let (rx, ry, rz) = (r.frand(0.2), r.frand(0.2), r.frand(0.2));
        assert_eq!(g.spin.to_bits(), expect_spin.to_bits(), "the spin = draw 1 with the draw-2 sign");
        assert_eq!(g.vel[0].to_bits(), mul_ee(rx - 0.1, REC_FRAME_RATE).to_bits());
        assert_eq!(g.vel[1].to_bits(), mul_ee(ry, REC_FRAME_RATE).to_bits());
        assert_eq!(g.vel[2].to_bits(), mul_ee(rz - 0.1, REC_FRAME_RATE).to_bits());
        // The stream advanced by exactly the 5 internal draws: a twin that makes the same five then
        // one more continues both streams identically; 4 or 6 draws land elsewhere.
        for k in [4u32, 6] {
            let mut a = crate::drone::Rng::new(0x5e4c04);
            let mut pool2 = GasPool::default();
            pool2.init_muzzle_smoke(&mut a, [0.0; 3]);
            let mut b = crate::drone::Rng::new(0x5e4c04);
            for _ in 0..k { b.frand(1.0); }
            let a7 = a.frand(1.0).to_bits();
            let b7 = b.frand(1.0).to_bits();
            assert_ne!(a7, b7, "the pool consumed {k} draws, not the game's 5");
        }
    }

    #[test]
    fn effect_puffs_consume_the_game_draws_per_kind() {
        // Kind 7 (NULL vel): 5 draws — spin, sign, wind ×3 [M 0x1ab060/0x1ab08c/0x1ab160-1a4].
        let mut rng = crate::drone::Rng::new(0x5e4c04);
        let mut pool = GasPool::default();
        pool.init_effect_puff(&mut rng, 0x123, [4.0, 5.0, 6.0], 7, [0x40, 0x40, 0x40, 0xff], 0x40404001, 0.1, 0.5, 60.0);
        assert_eq!(pool.puffs.len(), 1);
        let g = &pool.puffs[0];
        let mut r = crate::drone::Rng::new(0x5e4c04);
        let spin = r.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE);
        let sign = r.rand(0x100);
        let (rx, ry, rz) = (r.frand(0.2), r.frand(0.2), r.frand(0.2));
        assert_eq!(g.spin.to_bits(), (if sign >= 0x80 { -spin } else { spin }).to_bits());
        assert_eq!(g.vel[0].to_bits(), mul_ee(rx - 0.1, REC_FRAME_RATE).to_bits());
        assert_eq!(g.vel[1].to_bits(), mul_ee(ry, REC_FRAME_RATE).to_bits());
        assert_eq!(g.vel[2].to_bits(), mul_ee(rz - 0.1, REC_FRAME_RATE).to_bits());
        // Kind 0 (NULL vel): the rising default (0, REC, 0), 2 draws only [M 0x1aacc8:118-128].
        let mut rng = crate::drone::Rng::new(0x5e4c04);
        let mut pool = GasPool::default();
        pool.init_effect_puff(&mut rng, 0x123, [0.0; 3], 0, [0x40; 4], 0, 0.1, 0.5, 60.0);
        let g = &pool.puffs[0];
        assert_eq!(g.vel, [0.0, REC_FRAME_RATE, 0.0], "kind 0 rises at REC, no wind draws");
        let mut r = crate::drone::Rng::new(0x5e4c04);
        let _spin = r.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE);
        let _sign = r.rand(0x100);
        let a2 = r.frand(1.0).to_bits(); // draw #3 on a twin that made the same 2
        let mut b2 = crate::drone::Rng::new(0x5e4c04);
        for _ in 0..2 { b2.frand(1.0); }
        assert_eq!(a2, b2.frand(1.0).to_bits(), "the kind-0 puff advanced the stream exactly 2 draws");
        // Kind 4 with the site's NULL velocity pointer: the given-velocity jitter branch needs a
        // NON-NULL pointer, so the game falls to the rising default [M 0x1aacc8:91-128].
        let mut rng = crate::drone::Rng::new(0x5e4c04);
        let mut pool = GasPool::default();
        pool.init_effect_puff(&mut rng, 0x123, [0.0; 3], 4, [0x40; 4], 0, 0.1, 0.5, 60.0);
        assert_eq!(pool.puffs[0].vel, [0.0, REC_FRAME_RATE, 0.0], "NULL-vel kind 4 rises, no jitter draws");
        // Kind 0xb: the spin is quartered [M 0x1aacc8:74-76].
        let mut rng = crate::drone::Rng::new(0x5e4c04);
        let mut pool = GasPool::default();
        pool.init_effect_puff(&mut rng, 0x123, [0.0; 3], 0xb, [0x40; 4], 0, 0.1, 0.5, 60.0);
        let mut r = crate::drone::Rng::new(0x5e4c04);
        let spin = r.frand(std::f32::consts::FRAC_PI_2 * REC_FRAME_RATE) * 0.25;
        let sign = r.rand(0x100);
        let _ = (r.frand(0.2), r.frand(0.2), r.frand(0.2));
        assert_eq!(pool.puffs[0].spin.to_bits(), (if sign >= 0x80 { -spin } else { spin }).to_bits());
        // A 0-life entry creates no object and draws nothing (the game's entry gate [M :31]).
        let mut rng = crate::drone::Rng::new(0x5e4c04);
        let mut pool = GasPool::default();
        pool.init_effect_puff(&mut rng, 0x123, [0.0; 3], 7, [0x40; 4], 0, 0.1, 0.5, 0.0);
        assert!(pool.puffs.is_empty());
        let mut r = crate::drone::Rng::new(0x5e4c04);
        assert_eq!(rng.frand(1.0).to_bits(), r.frand(1.0).to_bits(), "the 0-life gate drew nothing");
    }

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
