//! The SimpleScripts' keyframe animation (`SS_Create` 0x1a9c58, `SS_Update` 0x1a9948; placement type
//! 0xE2). The game creates one object per 0xE2 whose animation key count (`level_tag+2` =
//! `extra[7] & 0xffff`) is non-zero, and `SS_Update` poses it every frame with `KeyFrame_Interp`
//! (0x1e1140) over the placement's 0x19 key list (`m_paths[record+0x44]` = `extra[8] & 0xffff`),
//! from key 0 to key `count−1` at the current playhead. The SS's posed matrix joins the sim's
//! set_dynamic collision world with its placement object's glist hashcode (`Ss.hash`, fix-party-
//! mission item 7: 07000008's safe door ch 136 and doorway bolt ch 137 are solid like the doors).
//!
//! The playhead is STATE-driven, not channel-seeked (grates-02: the disassembly at 0x1a9b90-0x1a9c18
//! confirms the decomp word for word; the old channel-seek model was the port's invention): state 1
//! sweeps and holds its last posed matrix after the wrap, 0x40 re-sweeps only while `SS_Operate`
//! (presence, or the channel word ≠ 0 and set) says so, 8 loops forever, everything else holds. The
//! channel (param 3) is an output written from the operate result while it is 0-valued — and the
//! latch `SS_Activate` 0x1a9ea8 makes on E for ch ≠ 0.
//!
//! Playhead rate [M, fix-0c-hatch]: `SS_Create` sets ext+0x04 = `(key count − 1) / rate` (the rate
//! word `extra[7] >> 16`, default 0x4B = 75 when zero) and `SS_Update` adds that step (×
//! `FRAME_RATE_MUL` = 1.0 at 60 Hz [M GS_SetRefreshRate 0x17d2b8:15]) per tick — the rate word is
//! the sweep's whole duration in ticks, so the lift leaves (rate 0 → 75) take 75 ticks and the
//! screens (rate 1) swap at once, matching the disc data.

use crate::door::{interp, parse_keys};

const IDENT: Mat4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
use crate::triggers::Channels;
use crate::world::V3;
use nf_formats::placement::Placement;

pub type Mat4 = crate::door::Mat4;

#[derive(Clone, Debug)]
pub struct Ss {
    pub placement: usize,
    /// Param 3: the switch channel that drives the playhead (`SS_Update`'s +0x10 read).
    pub channel: u16,
    /// The 0x19 key list (`m_paths[record+0x44]`).
    pub keys: Vec<crate::door::Key>,
    /// `extra[7] >> 16`, 0 → 75 (`SS_Create`'s default divisor).
    pub rate: u32,
    /// Param 6: the spline-interpretation flag (`SS_Create`'s +0x26).
    pub spline: bool,
    /// `SS_Create` 0x1a9c58: ext+0x22 = param 0 — the state (`SS_Activate` 0x1a9ea8 arms it).
    pub state: u16,
    /// ext+0x1E = param 2 — the activate mask (bit 2 = E-usable, `SS_Activate`'s gate).
    pub mask: u16,
    /// The playhead in key units, 0..=count−1.
    pub t: f32,
    /// The posed matrix this tick (at rest: key 0).
    pub matrix: Mat4,
    /// The placement's object glist hashcode (0 when it names nothing): the set_dynamic collision
    /// mesh (`Collide_PickObj` walks the cel-linked objects; fix-party-mission item 7 — the safe
    /// door and the doorway bolt are solid like the doors).
    pub hash: u32,
    /// ext+0x08 = param 5 — the sweep's rest point in key units, snapped to whole steps
    /// (`SS_Create` tail: `step * (int)((float)param5 / step)` [M 0x1a9dc8-0x1a9dd8 disasm]).
    /// Every SS on the shipped levels carries 0, so the landing window (below) never fires and
    /// the wrap ends every sweep.
    pub rest: f32,
    /// ext+0x24 = param 1 — the wrap flag: 0 → the wrap re-arms to 0x40 (re-operable), non-zero →
    /// the wrap disarms to 0 [M 0x1a9948:121-126].
    pub wrap_flag: u16,
    /// ext+0x14 = param 4 — the operate sound id (0xffff = none; `Sound_Play3D` at the 0x40
    /// operate arm [M 0x1a9948:86-88]). Unplayed headlessly; kept for the field map.
    pub sound: u16,
}

impl Ss {
    /// `SS_Activate` (0x1a9ea8) — the use key on the SS's class-0x25 object (`Player_Activate`
    /// 0x1a5170 case '%'): a state armed at create (ext+0x22 != 0) and the E mask bit (ext+0x1E & 4)
    /// snap the state to 1 and latch the channel on (the playhead then runs to the last key).
    pub fn activate(&mut self, ch: &mut crate::triggers::Channels, frame: u32) -> bool {
        if self.state == 0 || self.mask & 4 == 0 { return false; }
        self.state = 1;
        if self.channel != 0 { ch.set(self.channel as u32, 1, frame); }
        true
    }

    /// The object's world position (the posed matrix's translation).
    pub fn pos(&self) -> V3 { [self.matrix[3][0], self.matrix[3][1], self.matrix[3][2]] }
}

/// Every animated SimpleScript of the level, in placement order.
#[derive(Clone, Debug, Default)]
pub struct SsList { pub list: Vec<Ss> }

impl SsList {
    /// `SS_Create` over the main map's 0xE2 placements with a non-zero key count. The sphere argument
    /// is unused; the pose comes from the key list alone (key 0 = the placement transform [D]).
    pub fn create(placements: &[Placement], path_blocks: &[&[u8]], mut hash: impl FnMut(&Placement) -> u32) -> SsList {
        let mut list = Vec::new();
        for (i, p) in placements.iter().enumerate() {
            if p.flags & 0xff != 0xe2 { continue; }
            let count = (p.extra[7] & 0xffff) as usize;
            if count == 0 { continue; } // `SS_Create` creates nothing without keys [M 0x1a9c58].
            let index = (p.extra[8] & 0xffff) as usize;
            let Some(payload) = path_blocks.get(index) else { continue };
            let keys = parse_keys(payload);
            if keys.is_empty() { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let rate = (p.extra[7] >> 16) as u32;
            let rate = if rate == 0 { 75 } else { rate };
            let last = (count.min(keys.len()) - 1) as u16;
            let matrix = interp(&keys, 0, last, 0.0, param(6) != 0).unwrap_or(IDENT);
            // ext+0x08's snap [M 0x1a9dbc-0x1a9dd8 disasm]: step * (int)(rest / step).
            let step = last as f32 / rate as f32;
            let rest = if step > 0.0 { step * ((param(5) as f32 / step) as i32) as f32 } else { 0.0 };
            list.push(Ss {
                placement: i,
                channel: param(3) as u16,
                keys,
                rate,
                spline: param(6) != 0,
                state: param(0) as u16,
                mask: param(2) as u16,
                t: 0.0,
                matrix,
                hash: hash(p),
                rest,
                wrap_flag: param(1) as u16,
                sound: param(4) as u16,
            });
        }
        SsList { list }
    }

    /// `SS_Update` 0x1a9948 — the playhead is **state-driven**, not channel-seeked [M, the
    /// disassembly at 0x1a9b90-0x1a9c18 confirms the decomp word for word]:
    ///
    /// * state **1** (armed by `SS_Activate` 0x1a9ea8 or the 0x40 operate arm): pose at `t`,
    ///   advance `t += step * FRAME_RATE_MUL` (2-key scripts add `step` alone [M 0x1a9948:103-108];
    ///   `FRAME_RATE_MUL` is 1.0 at the port's fixed 60 Hz [M GS_SetRefreshRate 0x17d2b8:15]),
    ///   then the landing check (`rest < t && |t − rest| < 0.0002` → state 0x40 [M 0x1a9948:109-115])
    ///   and the wrap check (`(int)last_key < (int)t` → `t = 0`, state = `wrap_flag == 0 ? 0x40 : 0`
    ///   [M 0x1a9948:119-128]). States 0/0x40 never re-pose the matrix, so **the object holds its
    ///   last posed pose after the wrap** — a one-shot grate (p1=1) ends disarmed at state 0 with
    ///   the open pose held; a looping prop (p1=0) re-arms at 0x40.
    /// * state **0x40** (armed rest): re-sweeps only while `SS_Operate` returns non-zero — the
    ///   presence bits (mask 0x2/0x8/0x10 vs the obj+0xD0 contact chain [M 0x1a9df8:12-46]) or the
    ///   channel path (channel word ≠ 0 and `switch_channels[ch]` ≠ 0 [M 0x1a9df8:47-51]). The
    ///   port has no contact chain, so only the channel half is live; every mask-0x2/0x8/0x10 SS
    ///   rests until its channel (if any) is set.
    /// * state **8**: the infinite loop — pose, advance, wrap `t` to 0 past the last key, stay 8
    ///   [M 0x1a9948:48-71].
    /// * every other state: nothing (the pose stays) [M 0x1a9948:74-83].
    ///
    /// The channel is an OUTPUT, never a playhead driver: with mask bit 0 clear and the channel
    /// word non-zero, `SS_Update` writes the operate result into a 0-valued channel
    /// [M 0x1a9948:20-23] — unported here (it needs the presence result; the E latch in
    /// `activate` covers the ch≠0 write the game makes there [M 0x1a9ea8:11-13]).
    ///
    /// The per-tick step is `SS_Create`'s ext+0x04 = **`(key count − 1) / rate`**, computed once at
    /// create [M 0x1a9c58:51], so the rate word is the sweep's whole duration in ticks:
    /// 0700000c's `ventmesh-front` #138 (6 keys, rate 50 [D]) reaches its last key in 50 ticks
    /// (fix-0c-hatch; the old 1/rate model was five times slow).
    pub fn tick(&mut self, ch: &Channels) {
        for s in &mut self.list {
            if s.keys.len() < 2 { continue; }
            let last = (s.keys.len() - 1) as u16;
            let step = last as f32 / s.rate as f32;
            // `SS_Operate` 0x1a9df8, channel half (the presence half needs the obj+0xD0 contact
            // chain — unported; see the module report). Channel word 0 means "no channel" [M
            // 0x1a9df8:47-48 — the `== 0` short-circuit], so a ch-0 SS never re-arms here.
            let operate = s.channel != 0 && ch.on(s.channel as u32);
            match s.state {
                1 => Self::sweep(s, last, step),
                0x40 if operate => Self::sweep(s, last, step), // the sound arm [M 0x1a9948:86-89] is unplayed
                8 => {
                    // The loop: pose, advance, wrap t to 0 past the last key, stay 8.
                    if let Some(m) = interp(&s.keys, 0, last, s.t, s.spline) { s.matrix = m; }
                    s.t += step;
                    if (last as i32) < (s.t as i32) { s.t = 0.0; }
                }
                _ => {}
            }
        }
    }

    /// The shared state-1 sweep body [M 0x1a9948:92-129]: pose, advance, landing, wrap.
    fn sweep(s: &mut Ss, last: u16, step: f32) {
        if let Some(m) = interp(&s.keys, 0, last, s.t, s.spline) { s.matrix = m; }
        s.t += step; // FRAME_RATE_MUL is 1.0 at 60 Hz [M 0x17d2b8:15]; the 2-key case is then identical
        // Landing: rest < t && |t − rest| < 0.0002 → armed rest at 0x40 [M 0x1a9948:109-115 +
        // disasm 0x1a9b9c-0x1a9bd4]. With the shipped rest = 0 this never fires.
        if s.rest < s.t && (s.t - s.rest).abs() < 0.0002 { s.state = 0x40; }
        // Wrap: (int)last_key < (int)t (signed trunc) → t = 0, state per the wrap flag
        // [M 0x1a9948:119-128 + disasm 0x1a9bdc-0x1a9c10]. The pose is NOT rewritten here —
        // states 0/0x40 hold the last posed matrix, so the object stays at its end pose.
        if (last as i32) < (s.t as i32) {
            s.t = 0.0;
            s.state = if s.wrap_flag == 0 { 0x40 } else { 0 };
        }
    }

    /// The SS of a placement index (the render's per-instance lookup).
    pub fn of_placement(&self, placement: usize) -> Option<usize> {
        self.list.iter().position(|s| s.placement == placement)
    }
}
