//! The helicopter (`Copter_Create` 0x17d340, `Copter_Update` 0x17dbb8, `Copter_Move` 0x17d628;
//! placement type 0x2F — fix-objects2). 07000008's `Helicopter-Body` (#45) is the mission's last
//! writer: gated on ch 5 (which the ride's end cinematic 0x06000078 writes), it flies its script
//! and its arrival writes ch 0x4B — "Escape with Zoe in gondola" completes and mux #276 fires
//! ch 100, ending the level into 0x0730008 [L mission record 3; M Mission_Update 0x182c60:85-116].
//!
//! The game's model [M]:
//! - `Copter_Create` (0x17d340 disasm 0x17d49c-0x17d4d0): record+0x70 = param 6, +0x72 = param 7,
//!   +0x74 = param 8, +0x76 = param 9; +0x48/+0x54 hold its scripts (params 4/5 — 0x06000051 the
//!   parts assembly, 0x06000050 the flight [D #45; the level bin's type-7 entries]), loaded through
//!   a hidden `SP_Create` (0x1dc7e0) whose script entities register as copter paths
//!   (`Copter_Register` 0x17e530).
//! - `Copter_Update` (0x17dbb8 head): a non-zero gate channel that is off returns at once — the
//!   copter does nothing until the gate opens; then the +0x76 countdown pulses +0x74 (:16-22).
//! - The move states: `Copter_Move` (0x17d628) advances the path playhead (record+0x5C) and, when
//!   the frames run out, the state machine advances; cases 4/5/6 write
//!   `switch_channels[record+0x72] = 1` (:249-268) — the arrival channel.
//! - The combat states (case 2: `Music_Event(2,1)`, `Copter_FireMissile`/`FireGun`, the cone-sphere
//!   player detection) are the attack helicopter's — not ported; on 07000008 the copter is the
//!   extraction and its gate only opens at the ride's end.
//!
//! Port status: the channel machine is exact (gate, the countdown pulse, the arrival write); the
//! flight is timed by the flight script's Nis length (0x06000050, 299 ticks [D]) — `Copter_Move`'s
//! frame counter reaches the same end [S: the exact per-path advance is not modelled]. The visuals
//! ride the port's script-player machinery: a synthetic player (mode 1, plays once) animates the
//! flight script's entities (the body 0x02000427 type 0x2F and its parented parts [D script
//! streams]) from the placement's matrix, and they join the collision world through the normal
//! script-entity path.

use crate::triggers::Channels;
use crate::world::V3;
use nf_formats::placement::{object_matrix, Placement};
use nf_formats::anim::Mat4;

#[derive(Clone, Debug)]
pub struct Copter {
    pub placement: usize,
    /// The placement's world matrix (the hidden SP's frame — the script's keys are local to it).
    pub matrix: Mat4,
    /// record+0x70 = param 6: the gate channel [M 0x17d340:0x17d4ac, 0x17dbb8 head].
    pub gate: u16,
    /// record+0x72 = param 7: written on the move's completion [M 0x17dbb8:249-268].
    pub out: u16,
    /// record+0x74/+0x76 = params 8/9: the countdown pulse written when the countdown expires
    /// [M 0x17dbb8:16-22]. Both 0 on 07000008 [D #45] — inert there, implemented anyway.
    pub arrival: u16,
    pub countdown: u16,
    /// The scripts (params 4/5): [assembly, flight] [D #45 = 0x06000051 / 0x06000050].
    pub scripts: [u32; 2],
    /// Idle until the gate's rising edge (the game's early return).
    pub started: bool,
    /// Ticks since the flight began; `out` fires at the flight script's length.
    pub flight_t: u32,
    pub flight_len: u32,
    pub done: bool,
}

impl Copter {
    /// `Copter_Create` over the main map's 0x2F placements.
    pub fn create(placements: &[Placement]) -> Vec<Copter> {
        let mut out = Vec::new();
        for (i, p) in placements.iter().enumerate() {
            if p.flags & 0xff != 0x2f { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            out.push(Copter {
                placement: i,
                matrix: object_matrix(p.rotation, p.translation),
                gate: param(6) as u16,
                out: param(7) as u16,
                arrival: param(8) as u16,
                countdown: param(9) as u16,
                scripts: [param(4), param(5)],
                started: false,
                flight_t: 0,
                flight_len: 0,
                done: false,
            });
        }
        out
    }

    /// `Copter_Update`'s channel machine. `flight_len` = the flight script's Nis length (the game's
    /// path frame count). Returns `(started_flight, arrived)`:
    /// - `started_flight` — the gate just rose; the caller starts the flight script's player;
    /// - `arrived` — the move completed this tick; `out` has been written [M cases 4/5/6].
    pub fn tick(&mut self, ch: &mut Channels, flight_len: u32, frame: u32) -> (bool, bool) {
        // The gate: a non-zero gate channel that is off stops everything [M 0x17dbb8 head].
        if self.gate != 0 && !ch.on(self.gate as u32) { return (false, false); }
        // The countdown pulse [M 0x17dbb8:16-22].
        let mut arrived = false;
        if self.countdown != 0 {
            self.countdown -= 1;
            if self.countdown == 0 && self.arrival != 0 && !self.done { ch.set(self.arrival as u32, 1, frame); }
        }
        if self.done { return (false, false); }
        if !self.started {
            self.started = true;
            self.flight_len = flight_len;
            self.flight_t = 0;
            if self.flight_len == 0 {
                // No flight script: the move is already complete [M Copter_Move's +0x54 == 0 → 1].
                self.done = true;
                if self.out != 0 { ch.set(self.out as u32, 1, frame); }
                return (false, true);
            }
            return (true, false);
        }
        // The flight: the playhead advances per tick; at the end the state machine writes `out`.
        if self.flight_t < self.flight_len { self.flight_t += 1; }
        if self.flight_t >= self.flight_len {
            self.done = true;
            if self.out != 0 { ch.set(self.out as u32, 1, frame); }
            arrived = true;
        }
        (false, arrived)
    }

    /// The copter's world position (the matrix row 3) — the viewer's draw/cull helpers.
    pub fn pos(&self) -> V3 { [self.matrix[3][0], self.matrix[3][1], self.matrix[3][2]] }
}
