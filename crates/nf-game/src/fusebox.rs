//! The fuseboxes (`FuseBox_Create` 0x1b0448, `FuseBox_Update` 0x1adfe8, `FuseBox_Register` 0x1b0970;
//! placement type 0x31 with a script hash in param 0 — 07000014's `OBJECT_FuseBox1`, the security
//! sabotage of "Sabotage base security and escape from room").
//!
//! The game's model [M]:
//! - `FuseBox_Create` stores param 1 → +0x54, param 2 → +0x56, param 3 → +0x5C, param 4 → +0x5E
//!   (two channel words per wire group), sets class 0x3C, and `SP_Create`s the box's script
//!   (param 0, mode 0x40 = preloaded) with `FuseBox_Register` as the create callback.
//! - `FuseBox_Register` files every entity the script creates into the box's two wire groups
//!   (10 slots each at +0x04/+0x2C) and counts them at +0x58/+0x5A.
//! - `FuseBox_Update` runs per frame: a wire whose hit list is non-empty (`Collide_FilterBullets`
//!   0xfffd — the wires die to bullets) is removed; removing a **group 1** wire hurts Bond 5.0
//!   (`Player_Hurt(0x40a00000, …)` — the live wire) unless +0x56 == 0. When a group's wire count
//!   reaches 0, both of its channel words are written (`switch_channels[ch] = 1` + timestamp) and
//!   cleared: group 0 → param 1 then param 3, group 1 → param 2 then param 4.
//!
//! Port stand-ins [S, noted in the report]: the wire sub-objects and their bullet-filter collisions
//! are not ported (the SP-entity machinery does not create class-5 hittables), so the wires are
//! modelled as one abstract wire per group cut by the E-interaction (`Player_Activate` 0x1a5170
//! reaches the box's class-0 SP first — the lid script 0x06000A3B — and the port cuts one wire per
//! press once the box is open). The channel writes, the group order (safe wire first, live wire
//! second) and the 5.0 live-wire hurt follow `FuseBox_Update` exactly.

use crate::triggers::Channels;
use nf_formats::placement::Placement;
use crate::world::V3;

pub struct FuseBox {
    pub placement: usize,
    /// The box's world position (the placement translation).
    pub pos: V3,
    /// +0x54/+0x56/+0x5C/+0x5E: group 0's two channels, then group 1's (placement params 1–4).
    pub ch: [u16; 4],
    /// One wire per group (see the stand-in note). `true` = still cuttable.
    pub wires: [bool; 2],
    /// The lid: closed until the first E (`Player_Activate` → the box's SP, script 0x06000A3B).
    pub opened: bool,
    /// Ticks since the lid opened (the script runs 0x3F = 63 ticks [D nis decode]).
    pub open_t: u32,
    pub gone: bool,
}

/// One 5.0-damage live-wire hurt (`Player_Hurt(0x40a00000 = 5.0f32, …)`) [M 0x1adfe8].
pub const LIVE_WIRE_HURT: f32 = 5.0;

impl FuseBox {
    /// `FuseBox_Create` over the main map's 0x31 placements that carry a script hash in param 0
    /// (07000014's `OBJECT_FuseBox1`, script 0x06000A3B). The E-interaction reach is the box's
    /// world sphere (`Control_BuildWorldSph`'s object centre + radius, the same helper the doors use).
    pub fn create(placements: &[Placement], object: impl Fn(&Placement) -> Option<(V3, f32)>) -> (Vec<FuseBox>, Vec<(V3, f32)>) {
        let mut out = Vec::new();
        let mut spheres = Vec::new();
        for (i, p) in placements.iter().enumerate() {
            if p.flags & 0xff != 0x31 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let script = param(0);
            // The game's fuseboxes preload a wire/lid script; a 0x31 without one is another class of
            // object (the pickup boxes — `Pickup_Create`'s own case) and keeps its old handling.
            if script < 0x0600_0000 { continue; }
            let (centre, radius) = object(p).unwrap_or((p.translation, 1.0));
            let pos = [p.translation[0] + centre[0], p.translation[1] + centre[1], p.translation[2] + centre[2]];
            out.push(FuseBox {
                placement: i,
                pos,
                ch: [param(1) as u16, param(2) as u16, param(3) as u16, param(4) as u16],
                wires: [true, true],
                opened: false,
                open_t: 0,
                gone: false,
            });
            spheres.push((pos, radius.max(1.0)));
        }
        (out, spheres)
    }

    /// The E-interaction (`player_use_doors`'s sphere, the same shape `Player_Activate` 0x1a5170
    /// casts: 1.0 at head + look). Returns the hurt this press dealt (0.0 = none).
    pub fn player_use(&mut self, head: V3, look: V3, ch: &mut Channels, frame: u32) -> f32 {
        if self.gone { return 0.0; }
        let use_point = [head[0] + look[0], head[1] + look[1], head[2] + look[2]];
        let r = 1.0f32;
        let d = [use_point[0] - self.pos[0], use_point[1] - self.pos[1], use_point[2] - self.pos[2]];
        if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > r * r { return 0.0; }
        if !self.opened {
            // First press: the lid script (0x06000A3B, 63 ticks) — the wires register over it
            // (`FuseBox_Register` 0x1b0970 through the SP's create callback).
            self.opened = true;
            self.open_t = 0;
            return 0.0;
        }
        // One wire per press, group 0 (safe) before group 1 (live) — `FuseBox_Update`'s group scan
        // order [M 0x1adfe8]. Cutting the live wire hurts 5.0 [M 0x1adfe8 Player_Hurt(0x40a00000)].
        let Some(g) = (0..2).find(|&g| self.wires[g]) else { return 0.0 };
        self.wires[g] = false;
        let hurt = if g == 1 && self.ch[1] != 0 { LIVE_WIRE_HURT } else { 0.0 };
        // The group emptied: write both of its channel words with the timestamp, then clear [M 0x1adfe8].
        if !self.wires[0] {
            for &c in &[self.ch[0], self.ch[2]] {
                if c != 0 { ch.set(c as u32, 1, frame); }
            }
        }
        if !self.wires[1] {
            for &c in &[self.ch[1], self.ch[3]] {
                if c != 0 { ch.set(c as u32, 1, frame); }
            }
        }
        if !self.wires[0] && !self.wires[1] { self.gone = true; }
        hurt
    }
}
