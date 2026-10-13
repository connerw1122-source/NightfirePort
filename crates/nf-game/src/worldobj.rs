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

/// The four families, created from the main map's placements in placement order (the game creates
/// in `parsemap_block_map_data_dynamic` 0x1d1b68's walk order; no RNG in any of the four creates).
#[derive(Clone, Debug, Default)]
pub struct WorldObjects {
    pub inverters: Vec<Inverter>,
    pub shooters: Vec<Shooter>,
    pub dynobjs: Vec<DynObject>,
    pub thirdcams: Vec<ThirdCam>,
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
    }
}
