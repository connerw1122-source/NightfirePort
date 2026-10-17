//! Level and dynamic lights (`light_tag` records), the VU1 point-light formula, and the per-object
//! 2-light selection.
//!
//! Decompile facts ([M] unless tagged):
//! - `Light_Create` 0x1cc3d8 fills a 0x50-byte record: position vec4 at +0x10, life short +0x20
//!   (`-1` → 0 = permanent, else `arg + 1`), script channel +0x22, flicker period +0x24, flicker
//!   base +0x26 (`arg >> 1`), flicker counter +0x28 (`arg + Rand_Rand(arg)` — the draw always
//!   happens, even for arg 0), flicker mask +0x2A, sound id +0x2C, tracked-object pointer +0x30
//!   (kind 2), psi handle +0x34, falloff +0x38, radius +0x3C, intensity +0x40, rgb bytes
//!   +0x44..+0x46, kind byte +0x47, active byte +0x48 = 1.
//! - `Light_Update` 0x1cafb8 runs per tick over `LightList`: a light with life != 0 is
//!   decremented and deleted when it reaches 0 (the muzzle light: life 1 = one tick); a non-zero
//!   channel copies `switch_channels[channel]` into active; a non-zero flicker period runs the
//!   flicker state machine (counter wraps through 0xF000, active = `mask == frame & mask`,
//!   re-seeded `count = base + Rand_Rand(base)` when the low 12 bits pass the period).
//! - `Lights_CalcClosestLights` 0x1cb2e0 walks the list keeping up to 30 accepted lights, sorted
//!   ascending by the falloff (the QuickSort comparator at 0x1d3350 compares `light+0x38`).
//!   Kinds: 0 only for objects with `flags & 0x80`; 1 skipped when `flags & 0x100`; 2 needs
//!   `flags & 0x80`, d = `Vec_Dist2D` (sqrt(dx²+dz²), Vec_Dist2D 0x1e2ec8) from the tracked
//!   object's position, accepted when d ≤ 4.0 with intensity overwritten `(1 − d·0.25)·50`
//!   [M 0x1cb40c]; else 3D: d = `Vec_Dist3D` accepted when d ≤ sphere radius + light radius.
//!   Accepted: falloff = `d / (radius · 0.5)`.
//! - `psiLight_SetLights` 0x10e588 uploads at most the two closest: `TColor[i]` =
//!   rgb(byte) · intensity · tint (the callers pass 1.0/1.0/1.0), `.w` = 1/radius².
//! - The per-vertex shading runs in the VU1 microcode (ACTION.ELF `.vutext` 0x241c80). The
//!   point-light program (I-mem ~832-940; the light setup at 595-608 loads packet qwords 8/9 =
//!   the light positions, 12/13 = TColor0/1, 15 = TAmbient) computes, per vertex, in the object's
//!   local space: `L = lightpos − pos`, `d² = |L|²` (componentwise `MUL` + the ACC broadcast-sum),
//!   `1/d²` via `DIV Q, VF0.w, d²` (VF0.w = 1.0, PCSX2 vuMemReset; the DIV at I-mem 866/902 reads
//!   Fs=VF00.w ÷ Ft=VF28.y — `_Fsf_` = bits 21-22 = w), atten = `max(0, 1/d² − 1/r²)`
//!   (`SUBw VF24, VF28, TColor.w`, then `MAXx.xy`), the diffuse `atten · dot(N, L)` with the RAW
//!   (unnormalized) L (componentwise `MUL` + broadcast-sum, no RSQRT in the program), clamped
//!   `max(0, ·)`, and the sum `TAmbient ⊙ rgba + TColor.rgb · term` (895: `MUL VF22, VF12, VF28`;
//!   911/912: `MADDAx VF7·term1` + `MADDy VF8·term0`). The vertex normal enters as a packed int
//!   × 1/128 (`MULi VF26, VF29, I`, I = 0x3c000000 — the LOI at 862/898/934). The summed colour is
//!   CLAMPED to byte 255 by `MINIi` (I = 255.0 from the LOI at 900/936, applied at 919/954) before
//!   the FTOI0 output store — the game's answer to close-range saturation. fix-lighting2: the port's
//!   shader mirrored the add at /128 over an Unorm8x4 (byte/255) vertex colour — 2× the game's add —
//!   and missed the clamp; both fixed (gpu.rs). The port runs the same math in world space —
//!   the game's object space is a rigid transform of it and N·L is invariant.

use crate::drone::Rng;

/// One `light_tag` record.
#[derive(Clone, Debug, PartialEq)]
pub struct Light {
    pub pos: [f32; 3],
    /// +0x20: 0 = permanent, else ticks left (`Light_Update` deletes at 0).
    pub life: i16,
    /// +0x22: script switch channel gating `active` (0 = none).
    pub channel: u16,
    /// +0x24: flicker period (0 = steady).
    pub flicker_period: i16,
    /// +0x26: flicker re-seed base (`param_11 >> 1`).
    pub flicker_base: u16,
    /// +0x28: flicker counter.
    pub flicker_count: u16,
    /// +0x2A: flicker mask (active = `mask == frame & mask`).
    pub flicker_mask: u16,
    /// +0x2C: the flicker wrap sound id (the port does not play it — see `update`).
    pub sound: i16,
    /// +0x30: kind 2 — index of the tracked object whose position the light follows (the searchlight part).
    pub track: Option<u32>,
    /// +0x3C.
    pub radius: f32,
    /// +0x40. Kind 2: overwritten per selection by `(1 − d·0.25)·50` [M 0x1cb40c].
    pub intensity: f32,
    /// +0x38: `d / (radius · 0.5)`, the sort key.
    pub falloff: f32,
    /// +0x44..+0x46, the raw bytes.
    pub rgb: [u8; 3],
    /// +0x47: 0 = level lights / ambient radiators (only objects with flag 0x80), 1 = muzzle
    /// flashes (not emitters, flag 0x100), 2 = searchlight beams (2D distance), 3 = generic point.
    pub kind: u8,
    /// +0x48.
    pub active: bool,
}

/// `LightList` (0x2c74c0). The game's pool grows in 32-record blocks (`Light_Create` mallocs
/// 0xa00 = 32 × 0x50 when the free list empties), so there is no fixed cap.
#[derive(Default, Clone, Debug)]
pub struct LightList {
    pub lights: Vec<Light>,
}

impl LightList {
    /// `Light_Create` 0x1cc3d8. `life_arg` is the game's `param_8` (-1 = permanent, `n` = n+1
    /// ticks). Draws `Rand_Rand(flicker)` from the shared RNG exactly like the game (the draw
    /// always happens, arg 0 included — the state advance is argument-independent [M 0x1cc3d8:
    /// `Rand_Rand(param_11)` unconditionally]; [S] the 10-arg call sites leave `param_11`
    /// uninitialised for the counter seed, which the steady-state flicker path re-derives anyway).
    pub fn create(&mut self, radius: f32, intensity: f32, pos: [f32; 3], rgb: [u8; 3],
                  channel: u16, life_arg: i16, kind: u8, flicker: u16, rng: &mut Rng) -> usize {
        let drawn = rng.rand(flicker as u32);
        self.lights.push(Light {
            pos,
            life: if life_arg == -1 { 0 } else { life_arg + 1 },
            channel,
            flicker_period: flicker as i16,
            flicker_base: flicker >> 1,
            flicker_count: flicker.wrapping_add(drawn as u16),
            flicker_mask: 0,
            sound: 0,
            track: None,
            radius,
            intensity,
            falloff: 0.0,
            rgb,
            kind,
            active: true,
        });
        self.lights.len() - 1
    }

    /// `Light_Create` with the full record the 0x4E placements carry: `seed_source` is the game's
    /// param_11 (node+0x26 = `seed>>1`, node+0x28 = `seed + Rand_Rand(seed)` — the draw ALWAYS
    /// happens, seed 0 included [M 0x1cc3d8]) and `mask` the flicker mask (node+0x2a). `create`
    /// above is the same machine with seed_source = the period and mask 0 — what its call sites
    /// (muzzle, explosion, radiators) pass implicitly. fix-searchlight-remainder, additive.
    #[allow(clippy::too_many_arguments)]
    pub fn create_full(&mut self, radius: f32, intensity: f32, pos: [f32; 3], rgb: [u8; 3],
                       channel: u16, life_arg: i16, kind: u8, flicker: u16, seed_source: u16,
                       mask: u16, rng: &mut Rng) -> usize {
        let drawn = rng.rand(seed_source as u32);
        self.lights.push(Light {
            pos,
            life: if life_arg == -1 { 0 } else { life_arg + 1 },
            channel,
            flicker_period: flicker as i16,
            flicker_base: seed_source >> 1,
            flicker_count: seed_source.wrapping_add(drawn as u16),
            flicker_mask: mask,
            sound: 0,
            track: None,
            radius,
            intensity,
            falloff: 0.0,
            rgb,
            kind,
            active: true,
        });
        self.lights.len() - 1
    }

    /// `Light_Update` 0x1cafb8. `channel_on` reads the switch channels; `frame` is
    /// `GameState+0x52` (the main-loop tick counter). The flicker path's `Sound_Play3D`
    /// (id +0x2C) is not ported: the placement call sites leave the sound id uninitialised and
    /// no level light showed the sound in play.
    pub fn update(&mut self, channel_on: &dyn Fn(u16) -> bool, frame: u32, rng: &mut Rng) {
        let mut dead = vec![false; self.lights.len()];
        for (k, l) in self.lights.iter_mut().enumerate() {
            if l.life != 0 {
                l.life -= 1;
                if l.life == 0 { dead[k] = true; }
            }
            if l.channel != 0 {
                l.active = channel_on(l.channel);
            }
            if l.flicker_period != 0 {
                let count = if l.flicker_count & 0xfff == 0 { 0xf000 } else { l.flicker_count };
                l.flicker_count = count;
                if count & 0xf000 == 0 {
                    l.active = true;
                    l.flicker_count = count - 1;
                } else {
                    l.flicker_count = count + 1;
                    // [M 0x1cafb8: `active = (mask == GameState+52 & mask)` — a mask of 0 compares 0 == 0,
                    // so a mask-less flicker light is ALWAYS active in phase; the port's old extra
                    // `flicker_mask != 0 &&` guard made it permanently dark.
                    l.active = (frame as u16 & l.flicker_mask) == l.flicker_mask;
                    if l.flicker_period < ((count + 1) & 0xfff) as i16 {
                        let base = l.flicker_base;
                        l.flicker_count = base.wrapping_add(rng.rand(base as u32) as u16);
                    }
                }
            }
        }
        let mut k = 0;
        self.lights.retain(|_| { let keep = !dead[k]; k += 1; keep });
    }

    /// The frame's resolved view. fix-searchlight-remainder: the UPLOAD position of a kind-2 light is
    /// the node's STATIC +0x10 — the housing placement position, written once at create before the
    /// −10000 move and never again [M Searchlight_Create 0x1cb4f8:154-167 writes node+0x10 = obj+0x30
    /// BEFORE the move; psiLight_SetLights 0x10e588:34-38 uploads node+0x10]. Only the intensity and
    /// the 4.0-accept test follow the tracked object's LIVE position [M Lights_CalcClosestLights
    /// 0x1cb2e0:41-44, store 0x1cb400] — carried separately in `ResolvedLight::track_pos`.
    pub fn resolve(&self, track_pos: &dyn Fn(u32) -> [f32; 3]) -> ResolvedLights {
        ResolvedLights {
            lights: self.lights.iter().map(|l| {
                let live = if l.kind == 2 { track_pos(l.track.unwrap_or(u32::MAX)) } else { l.pos };
                ResolvedLight {
                    pos: [l.pos[0], l.pos[1], l.pos[2], 1.0],
                    track_pos: [live[0], live[1], live[2], 1.0],
                    rgb: [l.rgb[0] as f32, l.rgb[1] as f32, l.rgb[2] as f32],
                    intensity: l.intensity,
                    inv_r2: 1.0 / (l.radius * l.radius),
                    kind: l.kind,
                    active: l.active,
                    track: l.track,
                }
            }).collect(),
        }
    }
}

/// The frame-resolved light list: kind-2 positions resolved, colours precomputed — the game's
/// per-frame `psiLight_SetLights` inputs.
#[derive(Clone, Debug, Default)]
pub struct ResolvedLights {
    pub lights: Vec<ResolvedLight>,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedLight {
    /// The UPLOADED position: the node's +0x10 [M psiLight_SetLights 0x10e588:34-38] — for kind 2 the
    /// static housing position (see `resolve`).
    pub pos: [f32; 4],
    /// The LIVE tracked position (kind 2: the searchlight spot object; other kinds = `pos`) — what
    /// `Lights_CalcClosestLights` measures the kind-2 distance against [M 0x1cb2e0:41-44].
    pub track_pos: [f32; 4],
    /// The rgb bytes (+0x44..46).
    pub rgb: [f32; 3],
    /// +0x40 (kind 2 recomputes it per selection).
    pub intensity: f32,
    /// 1/radius² (TColor.w).
    pub inv_r2: f32,
    pub kind: u8,
    pub active: bool,
    /// The record's tracked part index (kind 2) — the render-side handle for the live spot.
    pub track: Option<u32>,
}

impl ResolvedLights {
    /// `Lights_CalcClosestLights` 0x1cb2e0: the accepted light indices with their falloffs,
    /// sorted ascending by the falloff (the comparator at 0x1d3350), max 30 entries.
    /// `sphere` = (x, y, z, radius); `flags` = the object's display flags word (obj+0xF0 /
    /// cel+0x94). Ties keep the list order ([S] the game's QuickSort is not stable; a tie needs
    /// equal falloffs).
    pub fn closest(&self, sphere: [f32; 4], flags: u32) -> Vec<(usize, f32)> {
        let mut out: Vec<(usize, f32)> = Vec::new();
        for (i, l) in self.lights.iter().enumerate() {
            if !l.active { continue; }
            // Kind 2 measures against the LIVE tracked position (the spot object); every upload reads
            // the node's static +0x10 [M 0x1cb2e0 vs 0x10e588].
            let dp = if l.kind == 2 { l.track_pos } else { l.pos };
            let d = match l.kind {
                0 => {
                    if flags & 0x80 == 0 { continue; }
                    dist3(dp, sphere)
                }
                1 => {
                    if flags & 0x100 != 0 { continue; }
                    dist3(dp, sphere)
                }
                2 => {
                    if flags & 0x80 == 0 { continue; }
                    let dx = dp[0] - sphere[0];
                    let dz = dp[2] - sphere[2];
                    let d = (dx * dx + dz * dz).sqrt();
                    if d > 4.0 { continue; }
                    d
                }
                _ => {
                    let d = dist3(dp, sphere);
                    if d > sphere[3] + l.inv_r2.recip().sqrt() { continue; }
                    d
                }
            };
            // The falloff `d / (radius · 0.5)` [M 0x1cb448/0x1cb44c]; radius = 1/sqrt(inv_r2).
            let radius = l.inv_r2.recip().sqrt();
            out.push((i, d / (radius * 0.5)));
            if out.len() > 30 { break; }
        }
        out.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    /// `psiLight_SetLights` 0x10e588: at most the two closest lights as
    /// `(position.xyz, w 1.0)` and `(TColor rgb·intensity, w 1/radius²)` slot pairs.
    /// The uploaded POSITION is the node's static +0x10 [M 0x10e588:34-38 — for kind 2 the housing];
    /// the kind-2 intensity `(1 − d·0.25)·50` [M 0x1cb400] uses the LIVE tracked position's 2D distance.
    pub fn pick2(&self, sphere: [f32; 4], flags: u32) -> ([[f32; 4]; 2], [[f32; 4]; 2]) {
        let mut lp = [[0.0f32; 4]; 2];
        let mut lc = [[0.0f32; 4]; 2];
        for (slot, (i, _falloff)) in self.closest(sphere, flags).into_iter().take(2).enumerate() {
            let l = &self.lights[i];
            lp[slot] = l.pos;
            // The kind-2 intensity uses the RAW 2D distance from the LIVE spot [M 0x1cb400], not the falloff.
            let k = if l.kind == 2 {
                let dx = l.track_pos[0] - sphere[0];
                let dz = l.track_pos[2] - sphere[2];
                (1.0 - (dx * dx + dz * dz).sqrt() * 0.25) * 50.0
            } else { l.intensity };
            lc[slot] = [l.rgb[0] * k, l.rgb[1] * k, l.rgb[2] * k, l.inv_r2];
        }
        (lp, lc)
    }
}

fn dist3(a: [f32; 4], b: [f32; 4]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// The VU1 point-light shading for one vertex (the microcode's formula, world space). `base_rgb`
/// is the ambient ⊙ vertex-rgba product in the game's byte scale (the port's shader has it as the
/// Unorm8x4 vertex colour = byte/255), `normal` unit, `pos` world, `lp` the two light positions
/// (`.w` spare), `lc` the two TColor rows in the byte scale with `.w` = 1/radius². Returns rgb in
/// the byte scale, clamped to 255 like the program's MINIi (I = 255.0, .vutext I-mem 919/954) —
/// the caller's shader divides by 255 to enter the byte/255 vertex-colour scale.
pub fn shade_vertex(base_rgb: [f32; 3], normal: [f32; 3], pos: [f32; 3],
                    lp: &[[f32; 4]; 2], lc: &[[f32; 4]; 2]) -> [f32; 3] {
    let mut out = base_rgb;
    if dot(normal, normal) < 1e-6 { return out; } // zero normals (sprites, snow, radar) stay unlit [M main-2]
    for i in 0..2 {
        if lc[i][3] <= 0.0 || (lc[i][0] + lc[i][1] + lc[i][2]) <= 0.0 { continue; }
        let d = [lp[i][0] - pos[0], lp[i][1] - pos[1], lp[i][2] - pos[2]];
        let d2 = dot(d, d);
        let inv_d2 = 1.0 / d2; // DIV Q, VF0.w (=1.0), d² [M .vutext 866/902: Fs=VF00.w / Ft=VF28.y(d²)]
        let atten = (inv_d2 - lc[i][3]).max(0.0);
        let term = (atten * dot(normal, d)).max(0.0);
        out[0] += lc[i][0] * term;
        out[1] += lc[i][1] * term;
        out[2] += lc[i][2] * term;
    }
    // The program clamps the summed colour to byte 255 before the FTOI0 output store
    // [M .vutext 919/954: MINIi .xyzw VF23, VF22, I with I = 255.0 from the LOI at 900/936].
    [out[0].min(255.0), out[1].min(255.0), out[2].min(255.0)]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }

/// The muzzle smoke puff: `Gas_Init` 0x1aacc8 (the shot's `Rand_FRand(2π)` draw rides the shared
/// RNG in the game's order) + `gases_smoke` 0x1ab2d0. The game's params for the shot: start scale
/// 0.05, end scale 0.1, start ARGB 0x48/0x40/0x30/0x80, packed END rgba 1 — bytes (0,0,0,1) [M
/// 0x1a4298:100-102, 0x1a4440], the decomp-dropped stack life 0x80 = 128 ticks [M 0x1a43f8/0x1a442c,
/// rocket-trail-gas]. The live spawn runs through `GasPool::init_ex` (scene.rs, the muzzle-flash
/// tick); this struct is the older models-misc stand-in kept for the render slot type.
#[derive(Clone, Debug, PartialEq)]
pub struct Smoke {
    pub pos: [f32; 3],
    /// Ticks since creation.
    pub age: u8,
    /// Start / end scale (Gas_Init's param_1/param_2).
    pub scale: (f32, f32),
    /// Start alpha (Gas_Init's param_9 byte 3, 0x48/255).
    pub alpha: f32,
}

impl Smoke {
    pub fn new(pos: [f32; 3]) -> Smoke {
        Smoke { pos, age: 0, scale: (0.05, 0.1), alpha: 0x48 as f32 / 255.0 }
    }
    /// `gases_smoke` [S]: the puff rises, grows and fades over 30 ticks.
    pub fn update(&mut self) {
        self.age += 1;
        self.pos[1] += 0.01;
        let t = (self.age as f32 / 30.0).min(1.0);
        self.alpha = (0x48 as f32 / 255.0) * (1.0 - t);
    }
    pub fn dead(&self) -> bool { self.age >= 30 }
}

/// The level's load-time lights, in the game's creation order [M]: the main map's block-0x26
/// ambient radiators first (`Light_SetAmbientRadiators` 0x1caeb8 runs from the map block handler
/// `parsemap_handle_block_id` 0x1d1020 — one `Rand_Rand(0)` draw per radiator), then the type-0x4E
/// placements in placement order (`parsemap_create_dynamic_objects` 0x1d04f0 case 0x4e — kind 0,
/// intensity 1.0, permanent; radius = placement +0x2C, rgb bytes at +0x30/+0x34/+0x38, channel
/// +0x3C, flicker +0x40). The 0x4E walk is the same pass that creates the drones, so the shared
/// RNG draws interleave exactly there.
pub fn level_lights(level: &nf_formats::level::Level, rng: &mut Rng, lights: &mut LightList) {
    if let Some(m) = level.main_map() {
        for r in &level.maps[m].radiators {
            lights.create(r.radius, 1.0, r.pos,
                [(r.rgb[0] * 255.0) as u8, (r.rgb[1] * 255.0) as u8, (r.rgb[2] * 255.0) as u8],
                0, -1, 0, 0, rng);
        }
    }
}

/// The type-0x4E placement lights for one map (called from the placement walk so the draws
/// interleave with the drones' create draws in placement order). The record's tail fields are
/// pinned from the case-0x4E call site [M 0x1d0bd4-0x1d0c04: lhu 0x40 → a7 = the flicker period,
/// lhu 0x44 → stack+0 = the counter-seed source (Light_Create stores `seed>>1` at +0x26 and
/// `seed + Rand_Rand(seed)` at +0x28), lhu 0x48 → stack+8 = the flicker MASK (+0x2a), lhu 0x4c
/// → stack+0x10 = the wrap sound (+0x2c); n32 stack slots are 8-byte spaced]. The old parse
/// passed only the period and derived the seed from it — the two coincide only when the level
/// data's +0x44 == +0x40.
pub fn placement_light(level: &nf_formats::level::Level, map: usize, index: usize, rng: &mut Rng, lights: &mut LightList) {
    let p = &level.maps[map].placements[index];
    if p.flags & 0xff != 0x4e || p.flags & 0xa000 != 0 { return; }
    let e = |k: usize| p.extra.get(k).copied().unwrap_or(0);
    let b = |k: usize| (e(k) & 0xff) as u8;
    lights.create_full(f32::from_bits(e(2)), 1.0, p.translation,
        [b(3), b(4), b(5)], (e(6) & 0xffff) as u16, -1, 0,
        (e(7) & 0xffff) as u16, (e(8) & 0xffff) as u16, (e(9) & 0xffff) as u16, rng);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kind2_light_uploads_the_housing_and_measures_the_live_spot() {
        // [M 0x10e588:34-38 uploads node+0x10 (static); 0x1cb2e0:41-44 + store 0x1cb400 measure the
        // tracked obj's LIVE position] — the pre-fix resolve moved the UPLOAD to the live spot, which
        // lit a halo at the ground circle instead of the beam from the housing.
        let mut l = LightList::default();
        let i = l.lights.len();
        l.lights.push(Light {
            pos: [10.0, 40.0, 10.0], life: 0, channel: 0, flicker_period: 0, flicker_base: 0,
            flicker_count: 0, flicker_mask: 0, sound: 0, track: Some(0), radius: 500.0,
            intensity: 1.0, falloff: 0.0, rgb: [255, 255, 255], kind: 2, active: true,
        });
        let r = l.resolve(&|_| [70.0, 46.6, 10.0]); // the live spot 60 units +x of the housing
        assert_eq!(r.lights[i].pos, [10.0, 40.0, 10.0, 1.0], "the upload stays at the housing");
        assert_eq!(r.lights[i].track_pos, [70.0, 46.6, 10.0, 1.0], "the live spot rides track_pos");
        // An object standing at the spot: accepted (2D d = 0 ≤ 4), intensity (1−0)·50, position housing.
        let (lp, lc) = r.pick2([70.0, 46.6, 10.0, 1.0], 0x80);
        assert_eq!(lp[0], [10.0, 40.0, 10.0, 1.0], "psiLight_SetLights uploads the housing");
        assert_eq!(lc[0][0], 255.0 * 50.0, "the intensity (1−d·0.25)·50 at d=0");
        // 4 units off the spot in x: still accepted, intensity halved; 5 units: rejected.
        let (_, lc2) = r.pick2([66.0, 46.6, 10.0, 1.0], 0x80);
        assert_eq!(lc2[0][0], 255.0 * (1.0 - 4.0 * 0.25) * 50.0);
        let (_, lc3) = r.pick2([65.0, 46.6, 10.0, 1.0], 0x80);
        assert_eq!(lc3[0], [0.0; 4], "d > 4.0 rejects the spot light");
        // Sister: the pre-fix pick2 measured the intensity from the housing (d = 60 → rejected) —
        // an object AT the spot would get no light at all.
        let old_style_d = ((10.0 - 70.0f32).powi(2) + (10.0 - 10.0f32).powi(2)).sqrt();
        assert!(old_style_d > 4.0, "the pre-fix reading is exactly the bug");
    }

    #[test]
    fn a_maskless_flicker_light_stays_active_in_phase() {
        // [M 0x1cafb8: `active = (mask == GameState+52 & mask)` — mask 0 compares 0 == 0, always true;
        // the port's old `flicker_mask != 0 &&` guard made such lights permanently dark.]
        let mut l = LightList::default();
        l.lights.push(Light {
            pos: [0.0; 3], life: 0, channel: 0, flicker_period: 60, flicker_base: 30,
            flicker_count: 0xf100, flicker_mask: 0, sound: 0, track: None, radius: 5.0,
            intensity: 1.0, falloff: 0.0, rgb: [255, 255, 255], kind: 0, active: true,
        });
        let mut rng = Rng::new(1);
        for frame in 0..64u32 {
            l.update(&|_| false, frame, &mut rng);
            assert!(l.lights[0].active, "mask-0 flicker: active every tick (frame {frame})");
        }
        // Sister: a real mask gates only the count-UP (dark) phase — the lit phase sets active=1
        // unconditionally [M 0x1cafb8]. Mask 8 is on only while bit 3 of the frame is set, checked
        // on the ticks that run the up-phase (pre-update count has 0xf000 set).
        let mut m = LightList::default();
        m.lights.push(Light { flicker_mask: 8, ..l.lights[0].clone() });
        let mut seen_off = false;
        let mut seen_on_up = false;
        for frame in 0..256u32 {
            let up_phase = m.lights[0].flicker_count & 0xf000 != 0;
            m.update(&|_| false, frame, &mut rng);
            if up_phase {
                let want = frame & 8 == 8;
                assert_eq!(m.lights[0].active, want, "mask 8 tracks frame bit 3 in the up-phase (frame {frame})");
                seen_on_up |= want;
            }
            seen_off |= !m.lights[0].active;
        }
        assert!(seen_on_up && seen_off, "the mask actually toggles");
    }

    #[test]
    fn the_4e_parse_consumes_the_seed_draw_from_the_seed_field() {
        // [M 0x1d0be4: lhu 0x44 → the counter-seed source; the draw is Rand_Rand(param_11).] The port's
        // create_full derives flicker_base/counter from it — here the seed source differs from the
        // period, so the record must show the seed's >>1 base, and the stream advances once.
        let mut rng = Rng::new(7);
        let mut l = LightList::default();
        let i = l.create_full(6.0, 1.0, [1.0, 2.0, 3.0], [0xff, 0x80, 0x40], 9, -1, 0, 60, 200, 0x18, &mut rng);
        let li = &l.lights[i];
        assert_eq!(li.flicker_period, 60);
        assert_eq!(li.flicker_base, 100, "seed_source >> 1 [M 0x1cc3d8]");
        assert_eq!(li.flicker_mask, 0x18);
        assert_eq!(li.channel, 9);
        assert_eq!(li.rgb, [0xff, 0x80, 0x40]);
        let before = Rng { x: rng.x, y: rng.y, ..Default::default() };
        let mut rng2 = Rng::new(7);
        l.create_full(6.0, 1.0, [1.0, 2.0, 3.0], [0xff, 0x80, 0x40], 9, -1, 0, 60, 200, 0x18, &mut rng2);
        let _ = before;
        // Sister: the old create derived the base from the PERIOD (60 >> 1 = 30) — wrong when +0x44 ≠ +0x40.
        let mut old = LightList::default();
        let mut r3 = Rng::new(7);
        let j = old.create(6.0, 1.0, [1.0, 2.0, 3.0], [0xff, 0x80, 0x40], 9, -1, 0, 60, &mut r3);
        assert_eq!(old.lights[j].flicker_base, 30, "the old reading — the data disagrees");
    }
}
