//! The SimpleScripts' keyframe animation (`SS_Create` 0x1a9c58, `SS_Update` 0x1a9948; placement type
//! 0xE2). The game creates one object per 0xE2 whose animation key count (`level_tag+2` =
//! `extra[7] & 0xffff`) is non-zero, and `SS_Update` poses it every frame with `KeyFrame_Interp`
//! (0x1e1140) over the placement's 0x19 key list (`m_paths[record+0x44]` = `extra[8] & 0xffff`),
//! from key 0 to key `count−1` at the current playhead. The SS's posed matrix joins the sim's
//! set_dynamic collision world with its placement object's glist hashcode (`Ss.hash`, fix-party-
//! mission item 7: 07000008's safe door ch 136 and doorway bolt ch 137 are solid like the doors). The playhead advances while the SS's switch
//! channel (param 3) is on and returns while it is off — the open/closed swap of the lift screens,
//! shutters and lift doors (07000012's `sh_screen-shutter_down/up`, `lft_screen-liftopen/closed`,
//! `lft_LIFTDOOR1/2`, `sh_shuttersclosed/open_LEFT`).
//!
//! The at-rest pose is key 0 — for every pair member on 07000012 key 0 equals the placement
//! transform [D, the level's 0x19 blocks 12–20], so the covering/closed member sits in place and the
//! complementary member rests parked (its key 0 is the away position). Triggering the channel swaps
//! them: each member glides to its other key.
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
            let last = (count.min(keys.len()) - 1) as u16;
            let matrix = interp(&keys, 0, last, 0.0, param(6) != 0).unwrap_or(IDENT);
            list.push(Ss {
                placement: i,
                channel: param(3) as u16,
                keys,
                rate: if rate == 0 { 75 } else { rate },
                spline: param(6) != 0,
                state: param(0) as u16,
                mask: param(2) as u16,
                t: 0.0,
                matrix,
                hash: hash(p),
            });
        }
        SsList { list }
    }

    /// `SS_Update`'s playhead: toward the last key while the channel is on, back to key 0 while off,
    /// then the `KeyFrame_Interp` pose [M 0x1a9948 + 0x1e1140].
    ///
    /// The per-tick step is `SS_Create`'s ext+0x04 = **`(key count − 1) / rate`**, computed once at
    /// create [M 0x1a9c58:51 `puVar2[1] = (float)last / (float)rate`], and the update adds
    /// `step * FRAME_RATE_MUL` (the 2-key special case adds `step` alone, [M 0x1a9948:103-108]).
    /// `GS_SetRefreshRate` 0x17d2b8:15 sets `FRAME_RATE_MUL = 60.0 / FRAME_RATE` — 1.0 at the game's
    /// 60 Hz, which is the port's fixed tick — so the rate word is the whole sweep's duration in
    /// ticks: 0700000c's `ventmesh-front` #138 (6 keys, rate 50 [D]) opens in 50 ticks, not 250
    /// (fix-0c-hatch; the old 1/rate model was five times slow).
    pub fn tick(&mut self, ch: &Channels) {
        for s in &mut self.list {
            if s.keys.len() < 2 { continue; }
            let last = (s.keys.len() - 1) as f32;
            let step = last / s.rate as f32;
            // The channel INDEX is not special: `SS_Update` 0x1a9948:20-23 reads AND writes
            // `switch_channels[ch]` for any channel word including 0 (the 0c pulley winches
            // #351/#364/#377 listen on ch 0 [D]); only the channel VALUE gates the sweep.
            let target = if ch.on(s.channel as u32) { last } else { 0.0 };
            if s.t < target { s.t = (s.t + step).min(target); }
            else if s.t > target { s.t = (s.t - step).max(target); }
            if let Some(m) = interp(&s.keys, 0, (s.keys.len() - 1) as u16, s.t, s.spline) { s.matrix = m; }
        }
    }

    /// The SS of a placement index (the render's per-instance lookup).
    pub fn of_placement(&self, placement: usize) -> Option<usize> {
        self.list.iter().position(|s| s.placement == placement)
    }
}
