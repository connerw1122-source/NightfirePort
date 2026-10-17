//! The gameplay placement types the port drew as static geometry: the channel inverter (0x41), the
//! turret shooter (0xd2), the gated dynamic object (0x3b) and the third-person camera spline (0xe7).
//! All four are created by `parsemap_create_dynamic_objects` 0x1d04f0 from the placements' params
//! (the level_tag record, docs/fields/level-tag.md) and updated per tick by `control_funcs[class]`
//! through `control_movement_object_handler` 0x1336f8's update pass. None of the four creators
//! draws from the shared RNG (checked on each create's decompile), so creating them here does not
//! move the stream.
//!
//! Per-object layout follows the game's `obj_tag` (docs/atlas/object.md): the placement params the
//! creators copy into obj+0xF4/+0xF6 or the instance at obj+0xE0, the draw-flags word obj+0xF0 from
//! the glist's +0x44 (docs/fields/glist.md), the control flags obj+0xFE (bit 0 = delete me, applied
//! by the delete pass 0x1336f8:86-87 the same tick) and the tint/alpha byte obj+0x106.

use crate::world::V3;

fn dist3(a: V3, b: V3) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// InverterTrigger (placement 0x41 → `InverterTrigger_Create` 0x1b3040, class 0x53).
/// `InverterTrigger_Update` 0x1b30a8: while the IN channel is set, the OUT channel is forced to 0 —
/// the inverter that lets a script's 1 mean "off" (07000006/12/14).
#[derive(Clone, Debug)]
pub struct Inverter {
    /// obj+0xF4 = param 0 (u16): the channel watched.
    pub ch_in: u16,
    /// obj+0xF6 = param 1 (u16): the channel held low (`Update` refuses 0).
    pub ch_out: u16,
}

impl Inverter {
    /// `InverterTrigger_Update` 0x1b30a8: `if (switch_channels[in] && out) switch_channels[out] = 0;`
    pub fn tick(&self, channels: &mut crate::triggers::Channels, frame: u32) {
        if channels.on(self.ch_in as u32) && self.ch_out != 0 {
            channels.set(self.ch_out as u32, 0, frame);
        }
    }
}

/// Shooter (placement 0xd2 → `Shooter_Create` 0x17d030, class 0x4e): a one-shot fixed gun.
/// `Shooter_Update` 0x17d0c0: while the arm channel (param 0, obj+0xF4) is set it fires one
/// `Bullet_init` 0x12c5a0 bullet with the weapon at param 1 (obj+0xF6), along the object matrix's
/// forward (`Mat_GetDir` on obj+0x90) — or straight down (0, −1, 0) when the weapon record's class
/// halfword at +0x02 is 0x3b — then sets obj+0xFE bit 0, and the movement handler's delete pass
/// (0x1336f8:86-87) removes the object that same tick: one bullet per placement, on the arm edge.
#[derive(Clone, Debug)]
pub struct Shooter {
    /// The object position (obj+0x30 = the placement translation).
    pub pos: V3,
    /// The matrix forward (`Mat_GetDir(obj+0x90)`): the placement's RotZYX forward, column 2.
    pub dir: V3,
    /// obj+0xF6 = param 1 (u16): the `weapon_data` record index.
    pub weapon: u16,
    /// obj+0xF4 = param 0 (u16): the arm channel.
    pub arm: u16,
    /// obj+0xFE bit 0: set when fired; the game's delete pass removes the object, so never again.
    pub fired: bool,
}

impl Shooter {
    /// `Shooter_Update` 0x17d0c0. Spawns at most one bullet (the game deletes the object right
    /// after); `weapon_type_3b` is the record's +0x02 halfword (the game re-checks per fire).
    pub fn tick(
        &mut self,
        channels: &crate::triggers::Channels,
        weapons: &nf_formats::weapon::WeaponData,
        rng: &mut crate::drone::Rng,
        out: &mut Vec<crate::bullet::Bullet>,
    ) {
        if self.fired || !channels.on(self.arm as u32) {
            return;
        }
        self.fired = true;
        let Some(w) = weapons.records.get(self.weapon as usize) else { return };
        let f = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        // Mat_GetDir(obj+0x90); the 0x3b-class weapons fire straight down (0x17d0c0:20-24).
        let dir: V3 = if u16::from_le_bytes([w[2], w[3]]) == 0x3b {
            [0.0, -1.0, 0.0]
        } else {
            self.dir
        };
        // `Bullet_init` 0x12c5a0: the spread sphere (weapon +0x24, flags +0x68) around the direction,
        // then the record's damage/range/speed (+0x0C/+0x1C/+0x20).
        let flags68 = u32::from_le_bytes(w[0x68..0x6c].try_into().unwrap());
        let dir = crate::bullet::spread(dir, f(0x24), flags68, rng);
        out.push(crate::bullet::Bullet {
            pos: self.pos,
            dir,
            speed: f(0x20),
            travelled: 0.0,
            range: f(0x1c),
            damage: f(0x0c),
            owner: usize::MAX,
            state: 0,
            weapon: self.weapon,
            flyby: flags68 & 0x40_0000 != 0,
            bounces: 0,
        });
    }
}

/// DynamicObject (placement 0x3b → `DynamicObject_Create` 0x18dbf8, class 0x52): a placement-gated
/// prop with a distance fade. `DynamicObject_Update` 0x18d3b8 drives three effects:
/// - the gate channel A (param 0): when present and off the object is inert (early out);
/// - channel B (param 1) with the mode byte (param 3): mode 0 — armed B deletes the object
///   (obj+0xFE bit 0, the delete pass 0x1336f8 removes it that tick); mode 1 — armed B ORs the
///   draw-flags 0x30 on (obj+0xF0), otherwise restores the glist's base word;
/// - the distance fade (param 2, instance+8): inside the radius obj+0x106 (the alpha byte) rises
///   linearly from 0 at the radius to 255 at half of it.
#[derive(Clone, Debug)]
pub struct DynObject {
    /// The object position (obj+0x80 world sphere centre = the placement translation).
    pub pos: V3,
    /// instance[0] = param 0 (u16): the gate channel.
    pub ch_a: u16,
    /// instance[2] = param 1 (u16): the action channel.
    pub ch_b: u16,
    /// instance[8] = param 2 (u16): the fade radius (`Update` casts it to int; 0 = no fade).
    pub dist: u16,
    /// instance[10] = param 3 (u8): the mode byte.
    pub mode: u8,
    /// obj+0xF0 base = the glist's +0x44 draw-flags word (`DynamicObject_Create` 0x18dbf8:15-16).
    pub base_flags: u32,
    /// obj+0xF0 (the live word: base restored, or base | 0x30 in mode 1 with B armed).
    pub flags: u32,
    /// obj+0x106, the alpha byte written by the distance fade (init 0xFF, object.md +0x103..0x106).
    pub alpha: u8,
    /// obj+0xFE bit 0 (mode 0 + B armed → deleted by the movement pass that tick).
    pub dead: bool,
}

impl DynObject {
    /// `DynamicObject_Update` 0x18d3b8. `viewer` is `glb_viewer+0xC0` (the camera position).
    pub fn tick(&mut self, viewer: V3, channels: &crate::triggers::Channels) {
        if self.ch_a != 0 {
            if !channels.on(self.ch_a as u32) {
                return;
            }
            self.flags = self.base_flags; // *(u32*)(obj+0xF0) = instance[4..6]
        }
        // The mode byte splits B's effect: 0 = delete on armed B, else flags 0x30 while armed.
        if self.mode == 0 {
            if self.ch_b != 0 && channels.on(self.ch_b as u32) {
                self.dead = true; // obj+0xFE |= 1 [M 0x18d3b8:26]
            }
        } else if self.ch_b == 0 || !channels.on(self.ch_b as u32) {
            self.flags = self.base_flags; // 0x18d3b8:32-33, 38-39
        } else {
            self.flags |= 0x30; // 0x18d3b8:34-36
        }
        // The distance fade: alpha = clamp(d/radius − 0.5, 0, …) · 2 · 255 [M 0x18d3b8:40-49].
        if self.dist != 0 {
            let d = dist3(self.pos, viewer);
            if d < self.dist as f32 {
                let f = (d / self.dist as f32 - 0.5).max(0.0);
                self.alpha = ((f + f) * 255.0) as u8;
            }
        }
    }
}

/// ThirdCam (placement 0xe7 → `ThirdCam_Create` 0x1bc510, class 0xC): a third-person camera riding
/// a 0x19 key-list spline. The object registers itself in `ThirdPersonCameras[cam_id]`; the
/// camera/View side picks it by id and rides `ThirdCam_GetPos` 0x1bc590's `Spline_Interp`.
#[derive(Clone, Debug)]
pub struct ThirdCam {
    /// instance[0] = param 0 (u16): the `ThirdPersonCameras` slot.
    pub cam_id: u16,
    /// instance[4] = the level_tag +0x08 pointer: the 0x19 key list (`m_paths[record+0x44]`).
    pub keys: Vec<crate::door::Key>,
    /// instance[8] = the level_tag +0x02 key count.
    pub key_count: u16,
    /// instance[10] = param 1 (u8).
    pub flag: u8,
}

impl ThirdCam {
    /// `ThirdCam_GetPos` 0x1bc590: `Spline_Interp(keys, count·t, 0, count, m, 0, 2)` — the spline
    /// position at parameter t ∈ [0, 1]. The game clamps every neighbour index into [0, count], so
    /// time past the last key rests on the final key; the port rides `door::interp` with the last
    /// key index (its `last` is an inclusive index) and lets the clamp do the same. Render-side
    /// today (no camera consumer in the Sim).
    pub fn get_pos(&self, t: f32) -> Option<crate::door::Mat4> {
        if self.keys.len() < 2 {
            return None;
        }
        let last = (self.key_count.max(1) - 1).min(self.keys.len() as u16 - 1).max(1);
        crate::door::interp(&self.keys, 0, last, self.key_count as f32 * t, true)
    }
}

/// Hurt volume (placement 0xE4 → `Hurt_Create` 0x17e870, class byte 0x27): a damage volume whose
/// per-contact damage and pain type ride the placement params. The game's damage path is the
/// collision pass: every tick the toucher's bounding sphere overlaps the volume's glist bounds, a
/// HITDATA lands in both contact lists (`Collide_Update` 0x1ea398:160-234), and the victim's
/// handler deals it — for Bond `Player_DealWithObjHit` 0x194458:94-101 case 0x27:
/// `Player_HandlePain(hitdata+8 + Hurt_GetDamage(v), BL, Hurt_GetType(v), -1)`. `hitdata+8` is
/// copied from the toucher's `HITTEST+0x90` (the weapon damage slot; `Collide_Intersect`
/// 0x1ebaa8:322) — the player's ground HITTEST+0x90 is `BL+0x7D0`, which nothing writes, so the
/// volume's param damage is the whole pain amount. The update pass (`Hurt_Update` 0x17e5d0,
/// `control_funcs[0x27]`) runs the state machine below; it holds no damage code of its own.
///
/// Gates on the damage, from the decompile: the volume is skipped as a collision candidate while
/// `obj+0xF0` carries bit 0x20 (`Collide_PickObj` skips `& 0x20`), which `Hurt_Create` 0x17e870:56-58
/// sets when the gate channel exists and is off, `Hurt_Update` clears/sets as the channel flips.
/// Nothing else gates it: the state machine below drives the key-list animation and the one-shot
/// expiry only — a state-0 volume with a satisfied mask still damages on contact, because both the
/// contact build and `Player_DealWithObjHit` read neither `obj+0xF4` (state) nor the mask.
#[derive(Clone, Debug)]
pub struct Hurt {
    /// The volume's world triangles (the placement object's glist, transformed by the placement
    /// matrix — the same resolution `triggers::build` uses). The game's glist is attached at create
    /// (`Control_SetGList` 0x17e870:52). The key-list animation moves the game's obj+0x90 matrix and
    /// therefore its geometry; the port keeps the triangles at the placement matrix — *deviation*
    /// ([S] unverified against a live volume; the only animated disc placements are 07000046's seven
    /// bullet-masked one-shots), the frame math below still runs so one-shot expiry keeps time.
    pub tris: Vec<[V3; 3]>,
    /// The triangle bounds (the mesh test's reject box).
    pub lo: [f32; 3],
    /// (with `lo`) The triangle bounds.
    pub hi: [f32; 3],
    /// Param 0 (u8) → instance+0x14: the pain type (`Hurt_GetType` 0x17ea50). Disc values: 1 and 6.
    pub kind: u8,
    /// Param 1 (u32, int→float at create 0x17e870:32-38) → instance+0x10: the per-contact damage
    /// (`Hurt_GetDamage` 0x17ea18). Disc: 1 (07000002's trickle) and 105 / 1000 (the death volumes).
    pub damage: f32,
    /// Param 2 (u16) → instance+0x18: the gate channel (0 = always active).
    pub channel: u16,
    /// Param 3 (u16, 0 → 0xFFFF at create 0x17e870:40-44) → instance+0x1A: the loop-start sound id
    /// (`Sound_Play3D` at frame 0, 0x17e5d0:119-123). Not modelled in the Sim's audio.
    pub sound: u16,
    /// Param 4 (u16) → instance+0x16: the state-0 arming mask over the contact classes — 8 = class 3
    /// (player), 0x10 = class 2 (drone), 2 = class 5 (bullet) [M 0x17e5d0:62-99]. Gates the
    /// ANIMATION arm only (see the type doc).
    pub mask: u16,
    /// Param 5 (u8) → instance+0x15: 1 = one sweep then spent; 0 = loop forever [M 0x17e5d0:129-132].
    pub oneshot: u8,
    /// level_tag+0x02 (placement+0x40 u16) → instance+0x0C as count−1: the key count (0 = never
    /// animates — `Hurt_Update` state 1 returns at once, 0x17e5d0:106-108).
    pub count: u16,
    /// level_tag+0x04 (placement+0x42 u16): the playback divisor of
    /// `count·REC_FRAME_RATE·(FRAME_RATE/rate)` [M 0x17e870:47-50; REC_FRAME_RATE = 1/60].
    pub rate: u16,
    /// The 0x19 key list (level_tag+0x08 = `m_paths[placement+0x44]`) — parsed for the frame math;
    /// the port does not move the triangles with it (see `tris`).
    pub keys: Vec<crate::door::Key>,
    /// instance+0x00: the anim frame (create zeroes it).
    pub frame: f32,
    /// instance+0x04: the per-tick increment `(count−1)·(1/60)·(60/rate)` (0 when count ≤ 1 or
    /// rate = 0 — the game's float division by a zero rate would give inf; no disc placement has
    /// rate 0 with keys, so the port guards to 0.0 [S]).
    pub step: f32,
    /// obj+0xF4: 0 armed / 1 running / 2 spent (`Hurt_Update`'s state short).
    pub state: u16,
    /// obj+0xF0 bit 0x20: the channel gate — collision candidates with the bit are skipped, so no
    /// contacts and no damage [M Collide_PickObj skip; create 0x17e870:56-58, update 0x17e5d0:29-36].
    pub inactive: bool,
}

impl Hurt {
    /// `Hurt_Create` 0x17e870 for every main-map 0xE4 placement, in placement order (the game's
    /// dynamic-object walk; the create draws no RNG — checked on the decompile). `path_blocks` is
    /// the main map's 0x19 payloads; `tris_of` resolves the placement's object to world triangles.
    pub fn create(
        placements: &[nf_formats::placement::Placement],
        path_blocks: &[&[u8]],
        tris_of: impl Fn(&nf_formats::placement::Placement) -> Vec<[V3; 3]>,
    ) -> Vec<Hurt> {
        let mut out = Vec::new();
        for p in placements {
            if p.flags & 0xff != 0xe4 { continue; }
            let param = |k: u32| -> u32 { p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0) };
            let tris = tris_of(p);
            let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
            for t in &tris { for v in t { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } } }
            // The frame increment [M 0x17e870:47-50]: (count−1)·REC_FRAME_RATE·(FRAME_RATE/rate),
            // REC_FRAME_RATE = 1/60 [M docs/fields/pickup.md +0x00], FRAME_RATE = 60 at play.
            let count = (p.extra[7] & 0xffff) as u16; // level_tag+0x02 [M 0x1d1b68:91]
            let rate = (p.extra[7] >> 16) as u16; // level_tag+0x04 [M 0x1d1b68:90]
            let step = if count > 1 && rate != 0 { (count - 1) as f32 / rate as f32 } else { 0.0 };
            let index = (p.extra[8] & 0xffff) as usize; // level_tag+0x08 = m_paths[placement+0x44]
            let keys = path_blocks.get(index).map(|b| crate::door::parse_keys(b)).unwrap_or_default();
            let channel = param(2) as u16;
            out.push(Hurt {
                tris, lo, hi,
                kind: param(0) as u8,
                damage: param(1) as f32,
                channel,
                sound: if param(3) == 0 { 0xffff } else { param(3) as u16 },
                mask: param(4) as u16,
                oneshot: param(5) as u8,
                count, rate, keys,
                frame: 0.0,
                step,
                state: 0,
                // The gate: a channel that exists and is off disables the volume at create
                // [M 0x17e870:56-58] — `switch_channels[]` is all zero at level load.
                inactive: channel != 0,
            });
        }
        out
    }

    /// `Hurt_Update` 0x17e5d0 (the `control_funcs[0x27]` update pass): the channel gate, the
    /// state-0 arm check, the state-1 key-list frame advance and the one-shot expiry.
    pub fn tick(&mut self, channels: &crate::triggers::Channels) {
        if self.state == 2 { return; } // the class byte dropped to 1 (0x17e5d0:42-45) — inert
        if self.channel != 0 {
            if channels.on(self.channel as u32) {
                self.inactive = false; // 0x17e5d0:29-31
            } else {
                self.inactive = true; // 0x17e5d0:34-36
            }
        }
        if self.state == 0 {
            if self.channel != 0 && !channels.on(self.channel as u32) { return; } // 0x17e5d0:51-54
            // The arm read [M 0x17e5d0:56-99]: a channel present reads THE CHANNEL; no channel with
            // mask 0 arms at once; no channel with a mask reads the contact classes — the port has
            // no per-class contact list for volumes, so a masked arm never fires here ([S] stand-in;
            // the disc's masked no-channel placements arm on bullets — class 5 — which the port's
            // volumes do not see). Arming starts the animation only (see the struct doc).
            let arm = if self.channel != 0 { channels.on(self.channel as u32) } else { self.mask == 0 };
            if arm { self.state = 1; }
            if self.count == 0 { return; } // nothing to animate
        }
        // State 1: the frame advance [M 0x17e5d0:106-132]. A channel present and OFF falls through
        // after the state-0 reset (0x17e5d0:109-116 runs the body once on the off edge); the port's
        // early return above already covered that arm, so the body runs whenever state 1 is reached.
        if self.state != 1 { return; }
        // `Sound_Play3D` at frame 0 (0x17e5d0:119-123) — not modelled (Sim audio).
        // `Spline_Interp(frame, keys, …, obj+0x90, 0, 2)` moves the game's matrix (0x17e5d0:124-125);
        // the port's triangles stay at the placement matrix (see `tris`) but the frame keeps time.
        self.frame += self.step;
        if self.frame > self.count.saturating_sub(1) as f32 {
            self.frame = 0.0;
            if self.oneshot == 1 { self.state = 2; } // the class byte drops next tick (0x17e5d0:42-45)
        }
    }

    /// The damage pass: whether the player's capsule contacts this volume's triangles — the port's
    /// stand-in for the game's per-tick contact list (the same model `Trigger::touched_by` ships:
    /// AABB reject, containment for small-triangle volumes, then the capsule-vs-mesh tests).
    /// `Trigger::touched_by` 0x1b1fc8's geometry core, re-expressed for the volume.
    pub fn touched_by(&self, c: &crate::triggers::Toucher) -> bool {
        let grow = c.r + 0.1;
        let lo = [c.a[0].min(c.b[0]) - grow, c.a[1].min(c.b[1]) - grow, c.a[2].min(c.b[2]) - grow];
        let hi = [c.a[0].max(c.b[0]) + grow, c.a[1].max(c.b[1]) + grow, c.a[2].max(c.b[2]) + grow];
        if (0..3).any(|k| hi[k] < self.lo[k] || lo[k] > self.hi[k]) { return false; }
        let contain_ok = self.tris.iter().all(|t| (0..3).all(|k| {
            let a = t[k];
            let b = t[(k + 1) % 3];
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)) < 24.0 * 24.0
        }));
        if c.contain && contain_ok
            && (0..3).all(|k| c.a[k] >= self.lo[k] && c.a[k] <= self.hi[k])
            && (0..3).all(|k| c.b[k] >= self.lo[k] && c.b[k] <= self.hi[k]) {
            return true;
        }
        self.tris.iter().any(|t| {
            crate::triggers::segment_hits_triangle(c.a, c.b, t) || crate::triggers::point_triangle(c.a, t) <= c.r
                || crate::triggers::point_triangle(c.b, t) <= c.r
                || (0..3).any(|k| crate::triggers::segment_segment(c.a, c.b, t[k], t[(k + 1) % 3]) <= c.r)
        })
    }
}

/// The four families, created from the main map's placements in placement order (the game creates
/// in `parsemap_block_map_data_dynamic` 0x1d1b68's walk order; no RNG in any of the four creates).
#[derive(Clone, Debug, Default)]
pub struct WorldObjects {
    pub inverters: Vec<Inverter>,
    pub shooters: Vec<Shooter>,
    pub dynobjs: Vec<DynObject>,
    pub thirdcams: Vec<ThirdCam>,
    /// The hurt volumes (placement 0xE4, `Hurt_Create` 0x17e870) — filled by [`Hurt::create`]
    /// from `attach_level` (the WorldObjects::create closure's shape carries no triangles).
    pub hurts: Vec<Hurt>,
}

impl WorldObjects {
    /// The placement scan (`p.flags & 0xff` kinds 0x41 / 0xd2 / 0x3b / 0xe7). `path_blocks` is the
    /// main map's 0x19 key-list payloads (`m_paths`), indexed by the placement's +0x44 word.
    /// `glist` resolves the placement's object to its glist's +0x44 draw-flags word (the word
    /// `DynamicObject_Create` copies to obj+0xF0) and its sphere centre (`celglist+0x20..+0x2C`,
    /// docs/fields/glist.md) — the world sphere obj+0x80 the fade reads starts at the matrix
    /// translation + the glist centre (0x1336f8 pass 6 rebuilds it from the same two).
    pub fn create(
        placements: &[nf_formats::placement::Placement],
        path_blocks: &[&[u8]],
        glist: impl Fn(&nf_formats::placement::Placement) -> (u32, V3),
    ) -> WorldObjects {
        let mut w = WorldObjects::default();
        for p in placements {
            let param = |k: u32| -> u32 { p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0) };
            match p.flags & 0xff {
                0x41 => {
                    // InverterTrigger_Create 0x1b3040: p0 → obj+0xF4, p1 → obj+0xF6 (u16 each).
                    w.inverters.push(Inverter { ch_in: param(0) as u16, ch_out: param(1) as u16 });
                }
                0xd2 => {
                    // Shooter_Create 0x17d030: p0 → obj+0xF4, p1 → obj+0xF6.
                    let m = nf_formats::placement::object_matrix(p.rotation, p.translation);
                    w.shooters.push(Shooter {
                        pos: p.translation,
                        // Mat_GetDir(obj+0x90): the matrix's forward column (identity → +z).
                        dir: [m[0][2], m[1][2], m[2][2]],
                        weapon: param(1) as u16,
                        arm: param(0) as u16,
                        fired: false,
                    });
                }
                0x3b => {
                    // DynamicObject_Create 0x18dbf8: p0/p1 (u16), p2 (u16), p3 (u8); obj+0xF0 = the
                    // glist's +0x44 word (0x18dbf8:15-16), |0x30 when p0 != 0 (0x18dbf8:27-29).
                    let (base, centre) = glist(p);
                    let m = nf_formats::placement::object_matrix(p.rotation, p.translation);
                    w.dynobjs.push(DynObject {
                        // obj+0x80 (the world sphere the fade distance reads): the matrix translation
                        // plus the rotated glist centre (Control_BuildWorldSph's steady state).
                        pos: nf_formats::placement::transform(&m, centre),
                        ch_a: param(0) as u16,
                        ch_b: param(1) as u16,
                        dist: param(2) as u16,
                        mode: param(3) as u8,
                        base_flags: base,
                        flags: if param(0) != 0 { base | 0x30 } else { base },
                        alpha: 0xff,
                        dead: false,
                    });
                }
                0xe7 => {
                    // ThirdCam_Create 0x1bc510: cam id = p0 (u16), key count = the level_tag +0x02
                    // half (placement +0x40), the key list = m_paths[placement+0x44], flag = p1 (u8).
                    let count = (p.extra[7] & 0xffff) as u16;
                    let index = (p.extra[8] & 0xffff) as usize;
                    let keys = path_blocks.get(index).map(|b| crate::door::parse_keys(b)).unwrap_or_default();
                    w.thirdcams.push(ThirdCam { cam_id: param(0) as u16, keys, key_count: count, flag: param(1) as u8 });
                }
                _ => {}
            }
        }
        w
    }

    /// One tick, in the game's update-pass order (the DynamicObjList walk 0x1336f8 pass 4): the
    /// inverters, shooters and dynamic objects are plain `control_funcs` updates. `viewer` is the
    /// camera position for the fade; spawned shooter bullets join the drone-bullet list.
    pub fn tick(
        &mut self,
        channels: &mut crate::triggers::Channels,
        viewer: V3,
        weapons: &nf_formats::weapon::WeaponData,
        rng: &mut crate::drone::Rng,
        bullets: &mut Vec<crate::bullet::Bullet>,
        frame: u32,
    ) {
        for inv in &self.inverters {
            inv.tick(channels, frame);
        }
        for sh in &mut self.shooters {
            sh.tick(channels, weapons, rng, bullets);
        }
        for ob in &mut self.dynobjs {
            ob.tick(viewer, channels);
        }
        self.dynobjs.retain(|ob| !ob.dead);
        // The hurt volumes' update pass (`Hurt_Update` 0x17e5d0, control_funcs[0x27]): the channel
        // gate + the state-0 arm + the state-1 frame advance. The damage itself is the collision
        // pass's (`hurt_pain`, called from the Sim after Trigger_Calc with the touchers built).
        for h in &mut self.hurts {
            h.tick(channels);
        }
    }

    /// The hurt volumes' damage pass — the port's stand-in for `Player_CollisionHandler`
    /// 0x1949a8:141-152 walking Bond's fresh contact list and `Player_DealWithObjHit` 0x194458:94-101
    /// case 0x27: every ACTIVE volume (not spent, gate clear) the player capsule contacts deals
    /// `(Hurt_GetDamage, Hurt_GetType)` this tick. Spent one-shots left out: the game's state-2 write
    /// drops the class byte to 1, so `Player_DealWithObjHit` no longer matches 0x27 [M 0x17e5d0:42-45].
    pub fn hurt_pain(&self, toucher: &crate::triggers::Toucher) -> Vec<(f32, u32)> {
        self.hurts.iter()
            .filter(|h| h.state != 2 && !h.inactive && h.touched_by(toucher))
            .map(|h| (h.damage, h.kind as u32))
            .collect()
    }
}
