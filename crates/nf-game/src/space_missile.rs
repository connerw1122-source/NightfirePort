//! The 0700001b SpaceMissiles (`Create_SpaceMissile` 0x1a9ef0 / `SpaceMissile_Update` 0x1aa118,
//! update id 0x4f [M 0x1a9ef0:18]).
//!
//! Eight kind-0x39 placements (#129, #131–137 [D]): each carries p0 = the launch-anim script
//! (record +0x2c), p1 = the launched-away script (ext+0x04), p2 = the disarmed script (ext+0x08),
//! p3 = the state-0 countdown seed (ext+0x14, f32), p4 = the missile index 1..8 (ext+0x0c),
//! p5 = the abort channel 199 (ext+0x0e), p6 = the per-missile channel 21..28 (ext+0x10),
//! p7 = the launch-order gate 19 (ext+0x12) [D placement dump 2026-10-17; M 0x1a9ef0 param map].
//!
//! State machine (the jump table `PTR_LAB_002fcc30` at 0x2fcc30, five entries [M disasm]:
//! 0→0x1aa1c8, 1→0x1aa23c, 2→0x1aa25c, 3→0x1aa33c, 4→0x1aa3e0):
//! - **state 0** [M 0x1aa1c8]: return while `switch_channels[ext+0x12]` (p7 = 19) is clear; then
//!   `ext+0x14 -= 512.0` (the constant at 0x31d0dc [M]) and stay while the result is >= 0
//!   (c.lt.s/bc1f 0x1aa200-208); on < 0: `MissileDeploy[p4−1] = 1` (0x1aa238, base 0x30cd70 at
//!   0x1aa218) and `obj+0xf4 = 1` (state 1, 0x1aa21c).
//! - **state 1** [M 0x1aa23c]: `Script_Update(ext[0])` per tick; when the script signals done
//!   (`script+0x9c != 0`, 0x1aa244-24c) → state 2. The port's object-side script VM is not wired
//!   into `Drones`, so the play is a tick counter against the script's decoded length [S — the
//!   lengths and the in-stream channel writes are [D] decoded; the flow pins are the [M] states].
//! - **state 2** [M 0x1aa25c]: `switch_channels[p4]` (the console channel 1..8) set → free the p0
//!   script, load+play+update the **p2** script (ext+0x08), `obj+0xf6 = 0`, `MissileDeploy[p4−1]=2`,
//!   state 3; not set → the **p1** script (ext+0x04), `obj+0xf6 = 1`, `MissileDeploy[p4−1]=3`,
//!   state 3 (0x1aa278-338).
//! - **state 3** [M 0x1aa33c]: `Script_Update` + `Script_IsPlaying` per tick; when the script ends:
//!   `obj+0xf6 == 0` → `switch_channels[p6]` (21..28) = 1, else → `switch_channels[p5]` (199) = 1,
//!   both with `switch_channels_time[ch] = frame` (0x1aa354-3d0); state 4, ext[0] freed (0x1aa3d4).
//!   ch 21..28 feed AND #139 (21 & 22 & 27 → ch 9, the "Disable the missiles" objective [D #139]);
//!   ch 199 fails the mission (`Mission_Update` 0x1827d8:113-115, FailLabel 0x0200_005a [M]).
//! - **state 4**: the shared epilogue only — the object is inert (the outer guard also stops the
//!   per-tick `Script_Update` block in states 0 and 4 [M 0x1aa118:13]).
//!
//! `MissileDeploy` is the 8 bytes at 0x30cd70 [M 0x1aa218/0x1aa2c8/0x1aa324], zeroed per
//! `Create_SpaceMissile` [M 0x1a9ef0:62], read by `HUD_UpdateSpacePane` 0x19d3b8 (the eight lamps:
//! 1 → the flashing "in flight" tex, 3 → the dim launched tex [M hud.rs]).
//!
//! The per-missile p0 in-stream writes are [D] (script decode 2026-10-17): #129 0600092a (3300
//! ticks) → ch 31 @t359 · #131 0600092b (3300) → 32 · #132 06000929 (1800) → 33 · #133 06000a30
//! (1800) → ch 192 @t1, no gate write · #134 06000927 (1800) → 35 · #135 0600092c (2400) → 36 ·
//! #136 0600092e (3300) → 37 · #137 0600092d (2400) → 38. Gate 34's writer is 06000928 @t359 (SP
//! #200's script) [D]; SP #200 waits on ch 191 (SP #199's out) whose own gate 190 has **no writer**
//! in the level's 77 scripts [D sweep] — missile 4's console chain therefore never runs from disc
//! data alone. The p1/p2 script lengths are not pinned beyond their last decoded events (ch 99
//! @t1000 / ch 0x2a @t1130 [D]); the port uses those ticks as the state-3 waits [S, live-check
//! queued] — the tests only assert inside generous windows.

use crate::triggers::Channels;

/// The p0 launch-script lengths per missile index 1..8, in ticks [D decode 2026-10-17].
pub const P0_LEN: [u32; 8] = [3300, 3300, 1800, 1800, 1800, 2400, 3300, 2400];
/// The gate channel each p0 script writes at t359 (0 = none) [D].
pub const P0_GATE: [u16; 8] = [31, 32, 33, 0, 35, 36, 37, 38];
/// #133's 06000a30 writes ch 192 (0xc0) at t1 instead of a gate [D].
pub const P0_ALT_WRITE: u16 = 192;
/// [S] the p1 (launched-away) script wait: its last decoded event is ch 99 @t1000 [D].
pub const P1_LEN: u32 = 1001;
/// [S] the p2 (disarmed) script wait: its last decoded event is ch 0x2a @t1130 [D].
pub const P2_LEN: u32 = 1131;
/// The state-0 countdown decrement, the float at 0x31d0dc [M].
pub const COUNTDOWN_STEP: f32 = 512.0;

/// One kind-0x39 placement's missile object (the ext block at obj+0xE0 plus obj+0xf4/0xf6 [M]).
#[derive(Clone, Debug, PartialEq)]
pub struct SpaceMissile {
    pub placement: usize,
    /// ext+0x0c: the missile index 1..8 (p4) — the console channel AND the MissileDeploy slot [M].
    pub index: u16,
    /// ext+0x0e: the abort channel (p5 = 199 on all eight [D]).
    pub fail_channel: u16,
    /// ext+0x10: the per-missile done channel (p6 = 21..28 [D]).
    pub done_channel: u16,
    /// ext+0x12: the launch-order gate (p7 = 19 on all eight [D]).
    pub gate_channel: u16,
    /// ext+0x14: the state-0 countdown (p3: 105/40/20/0/60/105/105/60 [D]).
    pub countdown: f32,
    /// obj+0xf4: the state 0..4 [M].
    pub state: u16,
    /// obj+0xf6: 0 = the disarm branch, 1 = the launch branch (set in state 2 [M 0x1aa2bc/0x1aa31c]).
    pub launch_flag: u16,
    /// [S] the current script's tick counter (the state-1/3 script waits).
    pub timer: u32,
}

impl SpaceMissile {
    /// `SpaceMissile_Update` 0x1aa118, one tick. `deploy` is `MissileDeploy` (8 bytes, 0x30cd70).
    pub fn tick(&mut self, ch: &mut Channels, deploy: &mut [u8; 8], frame: u32) {
        match self.state {
            0 => {
                // [M 0x1aa1c8-1e4]: the gate first — no countdown while switch_channels[p7] is clear.
                if self.gate_channel != 0 && !ch.on(self.gate_channel as u32) {
                    return;
                }
                // [M 0x1aa1ec-20c]: countdown -= 512.0; stay while >= 0 (c.lt.s f0,f2 + bc1f).
                self.countdown -= COUNTDOWN_STEP;
                if self.countdown >= 0.0 {
                    return;
                }
                // [M 0x1aa210-238]: MissileDeploy[p4−1] = 1; obj+0xf4 = 1.
                if let Some(slot) = self.deploy_slot() {
                    deploy[slot] = 1;
                }
                self.state = 1;
                self.timer = 0;
            }
            1 => {
                // [M 0x1aa23c-258]: the p0 script plays; done → state 2. The in-stream writes are [D].
                self.timer += 1;
                if self.timer == 1 && self.index == 4 {
                    ch.set(P0_ALT_WRITE as u32, 1, frame); // [D 06000a30 t1 → ch 192]
                }
                let gate = P0_GATE[(self.index.clamp(1, 8) - 1) as usize];
                if self.timer == 359 && gate != 0 {
                    ch.set(gate as u32, 1, frame); // [D t359 → ch 31..38]
                }
                if self.timer >= P0_LEN[(self.index.clamp(1, 8) - 1) as usize] {
                    self.state = 2; // [M 0x1aa250-258: script+0x9c != 0 → 2]
                }
            }
            2 => {
                // [M 0x1aa25c-338]: the console channel decides the branch.
                if ch.on(self.index as u32) {
                    self.launch_flag = 0; // [M 0x1aa2bc] — the p2 (disarmed) script
                    if let Some(slot) = self.deploy_slot() {
                        deploy[slot] = 2; // [M 0x1aa2cc-338, a0 = 2]
                    }
                    self.timer = 0;
                    self.timer = P2_LEN.saturating_sub(1); // [S] the p2 wait (see module doc)
                } else {
                    self.launch_flag = 1; // [M 0x1aa31c] — the p1 (launched-away) script
                    if let Some(slot) = self.deploy_slot() {
                        deploy[slot] = 3; // [M a0 = 3]
                    }
                    self.timer = P1_LEN.saturating_sub(1); // [S] the p1 wait (see module doc)
                }
                self.state = 3; // [M 0x1aa2c0/0x1aa328]
            }
            3 => {
                // [M 0x1aa33c-3d0]: the branch script plays; on end the channel write + state 4.
                self.timer += 1;
                if self.timer >= self.wait_len() {
                    if self.launch_flag == 0 {
                        ch.set(self.done_channel as u32, 1, frame); // [M 0x1aa354-398: ch[p6]]
                    } else {
                        ch.set(self.fail_channel as u32, 1, frame); // [M 0x1aa360-3c4: ch[p5] = 199]
                    }
                    self.state = 4; // [M 0x1aa3d4-3d8]
                }
            }
            _ => {} // [M 0x1aa3e0]: state 4 is the epilogue — inert.
        }
    }

    fn deploy_slot(&self) -> Option<usize> {
        if (1..=8).contains(&self.index) {
            Some(self.index as usize - 1)
        } else {
            None
        }
    }

    fn wait_len(&self) -> u32 {
        if self.launch_flag == 0 { P2_LEN } else { P1_LEN }
    }
}
