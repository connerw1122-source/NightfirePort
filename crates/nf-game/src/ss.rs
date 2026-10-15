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
    /// ext+0x1E = param 2 — the activate mask (bit 2 = E-usable, `SS_Activate`'s gate) AND the
    /// `SS_Operate` contact mask (ext+0x1E & the contact bits: player 0x8 / drone 0x10 / bullet
    /// 0x2, `SS_Operate` 0x1a9df8:22-39; bit 0x1 gates the self-latch, `SS_Update` 0x1a9948:20).
    pub mask: u16,
    /// ext+0x24 = param 1 — the rearm word: after a completed sweep the state becomes 0x40
    /// (re-armed) when this is 0, 0 (dead) otherwise [M `SS_Update` 0x1a9948:121-126].
    pub rearm: u16,
    /// ext+0x08 = param 5 — the duration float, quantized down to whole steps at create
    /// (`SS_Create` 0x1a9c58 last line `puVar2[2] = step * (int)(duration/step)`); the playing
    /// state's exact-landing epsilon compares against it [M 0x1a9948:109-113].
    pub duration: f32,
    /// The playhead in key units, 0..=count−1.
    pub t: f32,
    /// The posed matrix this tick (at rest: key 0).
    pub matrix: Mat4,
    /// The placement's object glist hashcode (0 when it names nothing): the set_dynamic collision
    /// mesh (`Collide_PickObj` walks the cel-linked objects; fix-party-mission item 7 — the safe
    /// door and the doorway bolt are solid like the doors).
    pub hash: u32,
}

/// One `KeyFrame_Interp`/`Spline_Interp` pose into the SS's matrix (`SS_Update` 0x1a9948:49-58,
/// 92-101 — the key-frame form when ext+0x26 == 0, the spline form otherwise).
fn interp_at(s: &mut Ss, t: f32, last_k: u16) {
    if let Some(m) = interp(&s.keys, 0, last_k, t, s.spline) { s.matrix = m; }
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
            // The duration word quantizes down to whole steps (`SS_Create` 0x1a9c58's last line);
            // 0 (the disc's every value [D census]) stays 0 and disables the epsilon arm.
            let raw_duration = param(5) as f32;
            let step0 = last as f32 / rate as f32;
            let duration = if raw_duration > 0.0 && step0 > 0.0 { step0 * (raw_duration / step0) as i32 as f32 } else { 0.0 };
            list.push(Ss {
                placement: i,
                channel: param(3) as u16,
                keys,
                rate,
                spline: param(6) != 0,
                state: param(0) as u16,
                mask: param(2) as u16,
                rearm: param(1) as u16,
                duration,
                t: 0.0,
                matrix,
                hash: hash(p),
            });
        }
        SsList { list }
    }

    /// `SS_Update` 0x1a9948's state machine, ported whole (fix-0c-hatch-latches; the old scrub
    /// model — toward the last key while the channel is on, back while off — plays scripts in
    /// reverse and never re-arms, which the game never does [M 0x1a9948:48-130]).
    ///
    /// The driver is `SS_Operate` 0x1a9df8: contact types player(3)→0x8 / drone(2)→0x10 /
    /// bullet(5)→0x2 against the SS's mask word (ext+0x1E = param 2); no contact match → the
    /// channel path: **1 iff channel != 0 AND switch_channels[channel] != 0** [M 0x1a9df8:46-51].
    /// A channel-0 SS (the 0c winches #351/#364/#377, mask 0x2 [D]) is therefore driven ONLY by
    /// contacts — a bullet hit — never by the channel array (fix-0c-vent2's read-stand-in
    /// corrected). The self-latch [M 0x1a9948:20-23] writes `switch_channels[ch] = cond` only
    /// when the mask's bit 0x1 is CLEAR, the channel word is non-zero and the channel value is 0.
    ///
    /// States [M 0x1a9948:48-130]: **0x40** armed — a cond tick starts the sweep (state 1);
    /// **1** playing — `KeyFrame_Interp`/`Spline_Interp` at the playhead into `obj+0x90`, then
    /// `t += step` (`SS_Create`'s ext+0x04 = (keys−1)/rate [M 0x1a9c58:51]; ×`FRAME_RATE_MUL` =
    /// 1.0 at 60 Hz [M `GS_SetRefreshRate` 0x17d2b8:15], the 2-key case adds step alone — the
    /// same value); the exact-landing epsilon (`duration < t && |t−duration| < 0.0002`,
    /// :109-113) disarms to 0x40; the overshoot wrap (`last < t`, :119-128) resets t to 0 and
    /// the state to **0x40 when param 1 (rearm) is 0, 0 (dead) otherwise** — the POSE holds the
    /// last key either way (states 0/0x40 never interp). **8** free-loops with a bare wrap;
    /// **0** and the rest are inert.
    ///
    /// `contacts` is per-SS Operate contact bits (bit 0 = the player, bit 1 = a bullet) — the
    /// game's contact list (`obj+0xD0`) is built by the collision pass for every overlapping
    /// body; the port feeds the player's capsule and the player-bullet segments (the drone/bullet
    /// sources are inert on the disc's masks: every disc SS carries 0x1 or 0x2 [D census]).
    pub fn tick(&mut self, ch: &mut Channels, contacts: &[u8], frame: u32) {
        for (i, s) in self.list.iter_mut().enumerate() {
            if s.keys.len() < 2 { continue; }
            let last = (s.keys.len() - 1) as f32;
            let last_k = (s.keys.len() - 1) as u16;
            let step = last / s.rate as f32;
            let c = contacts.get(i).copied().unwrap_or(0);
            let contact = (s.mask & 0x8 != 0 && c & 1 != 0) || (s.mask & 0x2 != 0 && c & 2 != 0);
            let cond = contact || (s.channel != 0 && ch.on(s.channel as u32));
            // The self-latch: mask bit 0x1 clear, channel non-zero, channel value 0 [M 0x1a9948:20-23].
            if s.mask & 1 == 0 && s.channel != 0 && !ch.on(s.channel as u32) {
                ch.set(s.channel as u32, cond as u8, frame);
            }
            match s.state {
                // Armed: a cond tick starts the sweep (the sound handle is the stub arm) [M :85-90].
                // The arming tick falls into the shared interp+advance (the C's flow-through), so the
                // pose starts at the current playhead (0) and advances in the same tick.
                0x40 => if cond { s.state = 1; interp_at(s, s.t.min(last), last_k); s.t += step; },
                // Free loop: interp + advance, bare wrap [M :48-71].
                8 => { interp_at(s, s.t, last_k); s.t += step; if s.t > last { s.t = 0.0; } }
                // Playing [M :92-128].
                1 => {
                    interp_at(s, s.t.min(last), last_k);
                    s.t += step;
                    // The exact-landing epsilon [M :109-113]: an exact duration disarms without a wrap.
                    if s.duration > 0.0 && s.t > s.duration && (s.t - s.duration).abs() < 0.0002 { s.state = 0x40; }
                    if s.t > last {
                        s.t = 0.0;
                        s.state = if s.rearm == 0 { 0x40 } else { 0 }; // [M :119-128]
                    }
                }
                // 0 and the rest: inert — no interp at all, the matrix stays wherever the last
                // playing tick left it (the at-rest key 0 before the first play, the last key
                // after a completed sweep) [M :73-77, the 0/0x40 arms have no interp].
                _ => {}
            }
        }
    }

    /// The SS of a placement index (the render's per-instance lookup).
    pub fn of_placement(&self, placement: usize) -> Option<usize> {
        self.list.iter().position(|s| s.placement == placement)
    }
}
