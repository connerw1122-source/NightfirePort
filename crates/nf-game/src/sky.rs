//! The sky's timed parts (GAPS G-019): the lightning state machine and the 0x0700001B rotating sky.
//!
//! Everything here is `View_DrawSky` (0x1e5ad0) and its helpers, ported operation by operation:
//! - the per-slot lightning timers [M 0x1e5ad0:92-158, 0x1e79a0 View_AddSkyObj];
//! - `RotMatrix` [M 0x1e2368], `Concat` [M 0x1e4a90], `ApplyMatrixLVI` [M 0x1e4960],
//!   `Vec_Spherical_2_Cartesian` [M 0x1e3948];
//! - the 0x0700001B rotation block [M 0x1e5ad0:186-198] (`rotatosky.168` at 0x30d878).
//!
//! The render-side consumers live in nf-view (`scene.rs` sky part / `gpu.rs` sky pipeline); this module owns
//! the state and the arithmetic so it can be tested headless. Rounding: the EE FPU chop
//! (`nf_formats::ps2float`), like `trig.rs`. Trig: the game's `PS2Sinf` (`trig::sinf`, bit-exact).
//!
//! Frame wiring (from `Game_Draw` 0x1e6f50): `View_DrawSky(viewer, 1)` runs before the world list and
//! `View_DrawSky(viewer, 0)` after `psiPostSolid` — the argument is the slot's sky-group id, which is
//! placement parameter 4 [M 0x1d04f0 case 0x2a: the level param area +0x2c.. holds the placement's
//! parameter words, so param 0 = slot, 1 = flash variant, 2 = the lightning interval parameter,
//! 3 = the flash length, 4 = the sky group]. Each call ticks only the slots whose sky id matches, in slot
//! order; `Sky::tick_group` reproduces that.

use crate::drone::Rng;
use crate::trig::sinf;
use nf_formats::ps2float::{add_noguard, mul};

/// `FRAME_RATE` [L 0x30d0d0 = 60.0f in work/ee_start_05.bin].
pub const FRAME_RATE: f32 = 60.0;
/// The sky-slot count of the `View_DrawSky` loop (`uVar12 < 0x15`) [M 0x1e5ad0:224].
pub const MAX_SLOTS: usize = 0x15;
/// `0.00069444446` — the rotating sky's per-count angle [M 0x1e5ad0:192].
const ROTATOSKY_ANGLE: f32 = 0.00069444446;

/// One `GfxList` sky slot. The game keeps the draw handles at `slot*0x40` and the lightning state at
/// `slot*0x40+0x40` [M 0x1e79a0]; here they are one record. `View_AddSkyObj` [M 0x1e79a0] fills it from a
/// type-0x2A placement: param 0 → slot index, param 1 → flash-variant flag (which selects +0x14/+0x1c and
/// enables the timers), param 2 → `interval` (× `FRAME_RATE`, truncated to short), param 3 → `flash_len`,
/// param 4 → `skyid`. `View_AddSkyObj` also zeroes `flash`/`dark`/`thunder` and sets the object's flag 0x10.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkySlot {
    /// +0x10: the normal variant's glist (param_2 when not a flash placement).
    pub glist: u32,
    /// +0x14: the flash variant's glist (param_2 of the flash placement).
    pub flash_glist: u32,
    /// +0x18: the normal variant's object.
    pub obj: u32,
    /// +0x1c: the flash variant's object.
    pub flash_obj: u32,
    /// +0x20: the glist-path translation (placement translation).
    pub translation: [f32; 3],
    /// +0x30: the glist-path Euler angles (RotMatrix input, radians).
    pub rotation: [f32; 3],
    /// +0x40: placement param 2 × `FRAME_RATE` — the `Rand_Rand` argument for the next dark interval.
    pub interval: u16,
    /// +0x42: placement param 3 — the flash length in ticks.
    pub flash_len: i16,
    /// +0x44: flash countdown (> 0 = flashing this tick).
    pub flash: i16,
    /// +0x46: dark countdown (reaching 0 starts the next flash; 0 with a flash variant = flash at once).
    pub dark: u16,
    /// +0x48: thunder countdown (the sound fires when it reaches 0).
    pub thunder: i16,
    /// +0x4a: the sky group this slot draws in (`View_DrawSky`'s argument).
    pub skyid: u8,
    /// The drawn variant's glist flags word (`celglist+0x44`): bit 0x2 = viewer-centred, bit 0x8 = the
    /// camera-rotation path that skips the slot rotation and the 0x0700001B spin [M 0x1e5ad0:182-206].
    pub flags: u32,
}

impl SkySlot {
    /// `View_AddSkyObj` [M 0x1e79a0:26-36]: the interval parameter scaled by the frame rate, truncated to
    /// short, and the timers cleared. `flash_variant` selects which draw handles the placement fills.
    pub fn from_placement(slot: usize, skyid: u8, flash_variant: bool, interval_param: u16, flash_len: u16,
                          glist: u32, obj: u32, translation: [f32; 3], rotation: [f32; 3]) -> SkySlot {
        let interval = mul(interval_param as f32, FRAME_RATE) as i16;
        let _ = slot; // the slot index only selects the game's array position
        SkySlot {
            glist: if flash_variant { 0 } else { glist },
            flash_glist: if flash_variant { glist } else { 0 },
            obj: if flash_variant { 0 } else { obj },
            flash_obj: if flash_variant { obj } else { 0 },
            translation,
            rotation,
            interval: if flash_variant { interval as u16 } else { 0 },
            flash_len: if flash_variant { flash_len as i16 } else { 0 },
            flash: 0,
            dark: 0,
            thunder: 0,
            skyid,
            flags: 0,
            ..Default::default()
        }
    }
}

/// The thunder sound event (`Sound_Play(100.0f, id, 0, 0)` [M 0x1e5ad0:104-107]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThunderSound {
    /// Id 0x5d: `LightningType == 0`.
    Normal,
    /// Id 0x5e: `LightningType == 1` (the reflicker roll).
    Reflicker,
}

/// What one slot tick decided (the renderer draws the normal variant when `drew_flash` is false).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SlotTick {
    /// The flash variant was drawn this tick (the glist path when `flash_glist != 0`, else the object path).
    pub drew_flash: bool,
    /// The thunder sound fired this tick.
    pub thunder: Option<ThunderSound>,
    /// `SuperHappyLightningFlag` was raised (the lightning bolts draw; `Lightning_Update` 0x18cf68).
    pub super_happy: bool,
    /// How many `Rand_Rand` draws this tick consumed (0, 1 or 2) — pins the game's RNG order.
    pub rng_draws: u32,
}

/// The three globals of the lightning block: `LightningType` [L 0x30d0b9], `Reflicker`,
/// `SuperHappyLightningFlag` [L 0x30d0b8]. `SuperHappyLightningFlag` is cleared at the top of every
/// `View_DrawSky` call [M 0x1e5ad0:93] and raised during flash ticks of reflicker-type lightning.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LightningGlobals {
    pub lightning_type: u8,
    pub reflicker: bool,
    pub super_happy: bool,
}

impl LightningGlobals {
    /// The per-call clear of `SuperHappyLightningFlag` [M 0x1e5ad0:93].
    pub fn begin_draw_call(&mut self) { self.super_happy = false; }
}

/// The lightning arithmetic of one slot, one `View_DrawSky` tick [M 0x1e5ad0:100-158]. Call order per frame:
/// `begin_draw_call`, then the group's slots in slot order. The RNG consumes at most two `Rand_Rand` draws:
/// a flashing tick rolls `Rand_Rand(100)`; a flash-start tick rolls `Rand_Rand(interval)` then
/// `Rand_Rand(100)` (and on the non-reflicker branch `Rand_Rand(120)`).
pub fn tick_slot(slot: &mut SkySlot, g: &mut LightningGlobals, rng: &mut Rng) -> SlotTick {
    let mut out = SlotTick::default();
    // The thunder countdown runs every tick [M 0x1e5ad0:100-109]: decrement while positive; the sound
    // fires on the tick the timer leaves 1.
    let t = slot.thunder;
    if t > 0 {
        slot.thunder = t - 1;
        if t == 1 {
            out.thunder = Some(if g.lightning_type == 1 { ThunderSound::Reflicker } else { ThunderSound::Normal });
        }
    }
    if slot.flash < 1 {
        // The dark path [M 0x1e5ad0:110-146]. Without a flash variant the timers never run at all
        // (the jump to LAB_001e5d6c): no draws, no sound scheduling.
        if slot.flash_glist == 0 && slot.flash_obj == 0 {
            return out;
        }
        g.reflicker = false;
        let s = slot.dark;
        slot.dark = s.wrapping_sub(1);
        // `(int)((uint)(ushort)(sVar1 - 1U) << 0x10) < 1` — as i32 this is negative when the u16's top bit
        // is set, so a fresh 0 timer (wrap to 0xffff) starts a flash on the slot's first tick, like 1.
        let d16 = s.wrapping_sub(1) as u16;
        if d16 == 0 || d16 & 0x8000 != 0 {
            slot.flash = slot.flash_len;
            // Rand_Rand(interval) — the next dark interval [M 0x1e5ad0:124-125].
            slot.dark = rng.rand(slot.interval as u32) as u16;
            out.rng_draws += 1;
            // Rand_Rand(100) — the reflicker roll [M 0x1e5ad0:126-143].
            let roll = rng.rand(100);
            out.rng_draws += 1;
            if roll < 0x33 {
                g.reflicker = true;
                slot.thunder = mul(FRAME_RATE, 0.16666667f32) as i16;
                g.lightning_type = 1;
            } else {
                g.lightning_type = 0;
                // Rand_Rand(2 × FRAME_RATE), + FRAME_RATE × 0.5 [M 0x1e5ad0:134-142].
                let n = rng.rand((FRAME_RATE + FRAME_RATE) as u32);
                out.rng_draws += 1;
                slot.thunder = add_noguard(n as f32, mul(FRAME_RATE, 0.5), true) as i16;
            }
            out.drew_flash = true; // iVar11 = iVar2: the flash variant draws on the start tick
        }
    } else {
        // The flash path [M 0x1e5ad0:148-157]: decrement, then the ~50 % flicker roll.
        slot.flash -= 1;
        let roll = rng.rand(100);
        out.rng_draws += 1;
        if roll > 0x32 {
            out.drew_flash = true;
            if g.lightning_type == 1 {
                g.super_happy = true;
            }
        }
    }
    out
}

/// `rotatosky.168` [L 0x30d878 = 0 in every snapshot] and the rotation block [M 0x1e5ad0:186-198].
/// Only level 0x0700001B runs it; the counter advances once per slot pass that reaches the block
/// (both of 0700001b's slots have glist flags 0x222, so two passes per frame), and only while the
/// game is not paused (`GameState[0x50] == 0`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RotatingSky {
    pub count: u32,
}

impl RotatingSky {
    /// One slot pass: returns the spin angle in radians (`(float)count × 0.00069444446`), or `None` on
    /// other levels. The increment happens before the read [M 0x1e5ad0:189-192].
    pub fn slot_pass(&mut self, level: u32, paused: bool) -> Option<f32> {
        if level != 0x0700_001b {
            return None;
        }
        if !paused {
            self.count = self.count.wrapping_add(1);
        }
        Some(mul(self.count as f32, ROTATOSKY_ANGLE))
    }
}

/// The 0x0700001B sun-vector arguments, recomputed every slot pass [M 0x1e5ad0:187]: radius 1000.0 and
/// the two f32 literals `0x409f2c8e` / `0xbf29c920` kept at their exact bits. `Env_Create` 0x17b968 fills
/// the same vector from the level's +0x54/+0x58 degree parameters at load; the sky block overwrites it
/// every frame with these hardcoded angles and the rotated matrix applied.
pub const ROTATING_SKY_SUN_ARGS: (f32, f32, f32) =
    (1000.0, f32::from_bits(0x409f_2c8e), f32::from_bits(0xbf29_c920));

/// `Vec_Spherical_2_Cartesian(r, theta, phi)` [M 0x1e3948]: `PS2Sinf2` is the game's sinf; the two
/// arguments are `(θ + π/2, θ)` and `(φ + π/2, φ)`.
pub fn spherical_2_cartesian(r: f32, theta: f32, phi: f32) -> [f32; 3] {
    let ct = sinf(add(theta, HALF_PI)); // fStack_50
    let st = sinf(theta);               // fStack_4c
    let cp = sinf(add(phi, HALF_PI));   // fStack_48
    let sp = sinf(phi);                 // fStack_44
    [mul(mul(r, st), cp), mul(r, sp), mul(mul(r, ct), cp)]
}

const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);

fn add(a: f32, b: f32) -> f32 { add_noguard(a, b, true) }

/// `RotMatrix(angles)` [M 0x1e2368]: the game's Euler rotation matrix, `PS2Sinf3` = the game's sinf per
/// argument. Row-major rows `[0..3] [4..7] [8..11]`, entry 4k+i as in the decompile.
pub fn rot_matrix(v: [f32; 3]) -> [f32; 12] {
    let cx = sinf(add(v[0], HALF_PI)); // fStack_60
    let sx = sinf(v[0]);               // fStack_5c
    let cy = sinf(add(v[1], HALF_PI)); // fStack_58
    let cz = sinf(add(v[2], HALF_PI)); // fStack_54
    let sz = sinf(v[2]);               // fStack_50
    let sy = sinf(v[1]);               // afStack_4c[0]
    let mut m = [0.0f32; 12];
    m[0] = mul(cy, cz);
    m[1] = add(mul(mul(sx, sy), cz), mul(cx, sz));
    m[2] = add(mul(mul(cx, -sy), cz), mul(sx, sz));
    m[4] = mul(cy, -sz);
    m[5] = add(mul(cx, cz), -mul(mul(sx, sy), sz));
    m[6] = add(mul(sx, cz), -mul(mul(cx, -sy), sz));
    m[8] = sy;
    m[9] = mul(-sx, cy);
    m[10] = mul(cx, cy);
    m
}

/// `Concat(a, b)` [M 0x1e4a90]: `b = a · b` on the rotation rows (translation rows add; the sky keeps
/// them zero — `Mat_CopyRot` copies a rotation-only matrix).
pub fn concat(a: &[f32; 12], b: &mut [f32; 12]) {
    let mut out = [0.0f32; 12];
    for r in 0..3 {
        for c in 0..3 {
            out[r * 4 + c] = add(add(mul(a[r * 4], b[c]), mul(a[r * 4 + 1], b[4 + c])), mul(a[r * 4 + 2], b[8 + c]));
        }
    }
    *b = out;
}

/// `ApplyMatrixLVI(m, v)` [M 0x1e4960]: `v = m · v` with the decompile's index pattern.
pub fn apply_matrix_lvi(m: &[f32; 12], v: &mut [f32; 3]) {
    let (x, y, z) = (v[0], v[1], v[2]);
    v[0] = add(add(mul(m[0], x), mul(m[4], y)), mul(m[8], z));
    v[1] = add(add(mul(m[1], x), mul(m[5], y)), mul(m[9], z));
    v[2] = add(add(mul(m[2], x), mul(m[6], y)), mul(m[10], z));
}

/// The full 0x0700001B slot-pass matrix [M 0x1e5ad0:182-196]: the camera rotation (`Mat_CopyRot` of the
/// global at 0x2d6230), the slot's own Euler rotation, then the spin — `spin · slot · camera`.
pub fn rotating_sky_matrix(cam_rot: &[f32; 12], slot_rotation: [f32; 3], spin: f32) -> [f32; 12] {
    let mut m = *cam_rot;
    let slot = rot_matrix(slot_rotation);
    concat(&slot, &mut m);
    let spin_m = rot_matrix([spin, 0.0, 0.0]);
    concat(&spin_m, &mut m);
    m
}

/// The sky state a frame tick advances: the lightning slots and the rotating-sky counter.
#[derive(Clone, Debug, Default)]
pub struct Sky {
    pub slots: [SkySlot; MAX_SLOTS],
    pub globals: LightningGlobals,
    pub rotating: RotatingSky,
}

impl Sky {
    /// Tick one group's slots in slot order, like one `View_DrawSky(viewer, group)` call
    /// [M 0x1e6f50 / 0x1e5ad0:96]. Returns the per-slot ticks of the slots in that group and the last
    /// spin angle the rotation block produced (the sun vector is rotated by the last slot's matrix).
    pub fn tick_group(&mut self, group: u8, level: u32, paused: bool, rng: &mut Rng) -> (Vec<(usize, SlotTick)>, Option<f32>) {
        self.globals.begin_draw_call();
        let mut out = Vec::new();
        let mut last_spin: Option<f32> = None;
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.skyid != group || (slot.glist == 0 && slot.obj == 0 && slot.flash_glist == 0 && slot.flash_obj == 0) {
                continue;
            }
            let t = tick_slot(slot, &mut self.globals, rng);
            out.push((i, t));
            // The rotation block runs inside the same per-slot body, on the glist path, for slots whose
            // glist flags lack bit 0x8 [M 0x1e5ad0:182-198] — both 0700001b slots (flags 0x222) qualify,
            // so the counter advances twice per frame. The spin angle is consumed at draw time via
            // `rotating_sky_matrix`.
            if slot.glist != 0 && slot.flags & 0x8 == 0 {
                last_spin = self.rotating.slot_pass(level, paused);
            }
        }
        (out, last_spin)
    }

    /// Convenience wrapper returning just the slot ticks.
    pub fn tick_group_slots(&mut self, group: u8, level: u32, paused: bool, rng: &mut Rng) -> Vec<(usize, SlotTick)> {
        self.tick_group(group, level, paused, rng).0
    }
}
