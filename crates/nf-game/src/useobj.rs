//! The use-object classes of `Player_Activate` (0x1a5170) that the Tower mission (map-tower)
//! needs, beyond the doors/triggers the port already had:
//!
//! - **PCQWorm** (placement type **0x35** → `PCQWorm_Create` 0x1b07a8, class 0x49): the Q-Worm
//!   target computer. `PCQWorm_Update` 0x1aee20 state 0 waits for a class-5 object (a BULLET,
//!   `Bullet_init` 0x12c5a0 sets `obj+0xFF = 5`) in the PC's contact list whose weapon reads
//!   **0x58** (the Q-Worm gadget, weapon-data table [L]) and then, next update, writes
//!   `switch_channels[param1] = 1` and advances to state 2 (param 0 = the keycode count; both
//!   Tower PCs carry 0 → no keycode message, one-tick install). `PCQWorm_Activate` 0x1b0840 (the
//!   E path) starts the install only when param 2 != 0 (both Tower PCs carry 0 → E does nothing).
//! - **Switch** (placement type **0x29** → `Switch_Create` 0x1adcc0, class 0x39): param 0 = the
//!   switch channel, 2 = auto-toggle delay, 3 = the rest value, 4 = busy tracking, 5 = the gate
//!   channel. `Switch_Activate` 0x1b02a8 (reached from `SP_Activate` 0x1dc338 through the
//!   switch's owner link, or from the game's own use dispatch): toggles the channel when the
//!   gate is on (or absent), the stored value matches the channel, and the switch's script is
//!   idle. `Switch_Update` 0x1adea8 flips the stored value back after `param2` frames (none of
//!   the Tower switches set it). A class-3 activator can never toggle ch 0x61 [M 0x1b02a8].
//! - **ScriptPlayer E-activation** (`SP_Activate` 0x1dc338): an SP whose mask has bit 2 (4) is
//!   player-activatable — the use sets `ext+0x2C = 1` (the `activated` flag its own
//!   `SP_Update` 0x1dbda8 consumes as trigger bit 4), gated on the OUT channel when mask also
//!   has bit 1 (the gate channel is `SP_Update`'s own per-tick early return, not an activate
//!   gate [M 0x1dbda8:60-62 — the port keeps the stricter check]). NOTE: in the game
//!   `Player_Activate` dispatches to `SP_Activate` only for class-0 objects while 0xD9 placements
//!   create class-0x1B objects [M 0x1db9a8:71] — the E path cannot reach the Tower SPs either;
//!   this arm is the port's stand-in for the (unresolved) elevator/keyboard activation, and the
//!   level docs record the open question. On the Tower levels this is the
//!   elevator (`SCRIPTPLAYER_Elevators`, 07000009 #70: mask 4, out ch 0x39 — the activation is
//!   what objective 2's AND mux #274 waits for) and the security keyboard (0700000a #60).
//!   The SP's use-target here is its placement matrix; the game keys the SP object's matrix from
//!   its script (`SP_Update` → `Script_Update` → obj+0x90), which the port's ScriptPlayers do not
//!   model — placement transforms that sit at the world origin (both of the above) therefore
//!   cannot be hit by the use ray, and the level docs record where the walkthrough drives the
//!   resulting channel instead.
//!
//! Every fix cites the game address above; spec: docs/levels/0700000{9,a,b}.md.

use crate::triggers::Channels;
use crate::world::V3;

/// Whether the segment a→b passes within `r` of `c` (the bullet-through-contact test).
fn seg_sphere(a: V3, b: V3, c: V3, r: f32) -> bool {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if len2 <= 0.0 { 0.0 } else { (ab[0] * ac[0] + ab[1] * ac[1] + ab[2] * ac[2]).clamp(0.0, len2) / len2 };
    let p = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
    (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2) <= r * r
}

/// One Q-Worm target PC (`PCQWorm_Create` 0x1b07a8's ext: `[0]=param0 keycodes, [1]=param1
/// channel, [2]=param2 E-usable, [3]=param3`).
pub struct PcQworm {
    /// The object's world position (the placement transform; the Tower PCs carry real ones [D]).
    pub pos: V3,
    /// param 1: the channel `PCQWorm_Update` writes when the install completes [M 0x1aee20].
    pub ch: u8,
    /// param 2: non-zero makes `PCQWorm_Activate` (the E path) start the install [M 0x1b0840].
    pub e_usable: u8,
    /// param 3: `PCQWorm_Update` 0x1aee20 scans for the bullet only when it is non-zero
    /// (0x1aee20:49-51); both Tower PCs carry 1, the decoy office PCs carry 0 with no channel.
    pub armed: bool,
    /// `obj+0xF4`: 0 idle, 1 installing, ≥2 done [M 0x1aee20].
    pub state: u8,
}

/// One switch (`Switch_Create` 0x1adcc0's ext: +0x12 = param0 channel, +0x10 = param2 auto-delay,
/// +0x16 = param3 rest value, +0x19 = param4 busy, +0x14 = param5 gate).
pub struct SwitchObj {
    pub pos: V3,
    /// param 0: the channel this switch toggles.
    pub ch: u8,
    /// param 5: the gate channel that must be ON (0 = always allowed) [M 0x1b02a8].
    pub gate: u8,
    /// param 3: the rest value the auto-return flips toward [M 0x1adea8].
    pub rest: u8,
    /// param 4: the busy flag (the script-idle gate; the port stands in a cooldown).
    pub busy: bool,
    /// param 2: the auto-toggle delay in frames (0 = none) [M 0x1adea8].
    pub auto_delay: u16,
    /// `obj+0xF4`: the stored value [M 0x1adcc0 creates it from param 3].
    pub value: u8,
    /// The stand-in for the switch script's busy state (frames since the last toggle).
    pub cooldown: u16,
    /// The auto-return frame counter (`obj+0xF6`) [M 0x1adea8].
    pub auto_ct: u16,
}

/// The level's use objects, in placement order.
#[derive(Default)]
pub struct UseObjects {
    pub pcs: Vec<PcQworm>,
    pub switches: Vec<SwitchObj>,
}

impl UseObjects {
    /// `Switch_Create` 0x1adcc0:39-43's load beat: `switch_channels[param0] = param3` (+ the time
    /// stamp) at level load. The Tower switches all carry rest 0 (no-op) — ported for fidelity.
    pub fn init_channels(&self, ch: &mut Channels, frame: u32) {
        for s in &self.switches {
            if s.ch != 0 && s.rest != 0 {
                ch.set(s.ch as u32, s.rest, frame);
                ch.time[s.ch as usize] = frame;
            }
        }
    }

    /// `parsemap_create_dynamic_objects` 0x1d04f0 cases 0x35 / 0x29, in placement order.
    pub fn create(ps: &[nf_formats::placement::Placement]) -> UseObjects {
        let mut u = UseObjects::default();
        for p in ps {
            if p.flags & 0xff != 0x35 && p.flags & 0xff != 0x29 { continue; }
            let par = |k: u32| p.params.iter().find(|(i, _)| *i == k).map_or(0, |&(_, v)| v);
            let pos = p.translation;
            if p.flags & 0xff == 0x35 {
                // PCQWorm_Create 0x1b07a8: ext[1] = param 1 (the channel), ext[2] byte = param 2.
                u.pcs.push(PcQworm { pos, ch: par(1) as u8, e_usable: par(2) as u8, armed: par(3) != 0, state: 0 });
            } else {
                // Switch_Create 0x1adcc0: +0x12 = param 0, +0x10 = param 2, +0x16 = param 3,
                // +0x19 = param 4, +0x14 = param 5; obj+0xF4 starts at param 3.
                u.switches.push(SwitchObj {
                    pos, ch: par(0) as u8, gate: par(5) as u8, rest: par(3) as u8, busy: par(4) != 0,
                    auto_delay: par(2) as u16, value: par(3) as u8, cooldown: 0, auto_ct: 0,
                });
            }
        }
        u
    }

    /// `PCQWorm_Update` 0x1aee20 + `Switch_Update` 0x1adea8, once per tick. `bullets` are this
    /// tick's player-bullet shots as (from, to, weapon) — the game reads the PC's class-5 contact
    /// list (a bullet with weapon 0x58 entering the object's box); the port's equivalent is the
    /// segment-vs-sphere test over each bullet's path [M 0x1aee20's 0x58 test].
    pub fn tick(&mut self, bullets: &[(V3, V3, u16)], ch: &mut Channels, frame: u32) {
        for pc in self.pcs.iter_mut() {
            match pc.state {
                0 => {
                    // State 0: a Q-Worm bullet (weapon 0x58) reaching the PC begins the install
                    // [M 0x1aee20: the class-5 toucher with ext+0x44 == 0x58] — only when param 3
                    // is set (0x1aee20:49-51). The contact box is the PC's glist; the port stands
                    // in the placement sphere r = 0.7 (the Q-WORM_PC_1 box's circumscribed radius,
                    // [D placements 0x35]).
                    if pc.armed {
                        let hit = bullets.iter().any(|&(a, b, w)| w == 0x58
                            && seg_sphere(a, b, pc.pos, 0.7));
                        if hit { pc.state = 1; }
                    }
                }
                1 => {
                    // State 1: the keycode message (param 0 keys; presentation) and the channel
                    // write, then state 2 [M 0x1aee20 LAB_001aefec]. The Tower PCs carry param 0
                    // = 0 → the write lands the tick after the hit.
                    if pc.ch != 0 { ch.set(pc.ch as u32, 1, frame); }
                    pc.state = 2;
                }
                _ => {}
            }
        }
        for s in self.switches.iter_mut() {
            if s.cooldown > 0 { s.cooldown -= 1; }
            // The stored value follows the channel [M 0x1adea8's first block].
            let cur = ch.v.get(s.ch as usize).copied().unwrap_or(0);
            if s.value != cur { s.value = cur; }
            // The auto-return [M 0x1adea8's second block]: after param2 frames off the rest
            // value, flip the channel back.
            if s.auto_delay != 0 && cur != s.rest {
                s.auto_ct += 1;
                if s.auto_ct > s.auto_delay {
                    let v = if cur != 0 { 0u8 } else { 1u8 };
                    ch.set(s.ch as u32, v, frame);
                    s.auto_ct = 0;
                }
            } else {
                s.auto_ct = 0;
            }
        }
    }

    /// The E-use dispatch (`Player_Activate` 0x1a5170's class walk): SPs (mask & 4, via
    /// `SP_Activate` 0x1dc338) and switches (class 0x39, `Switch_Activate` 0x1b02a8) within the
    /// use sphere (radius 1.0 at head + look [M 0x1a5170's `Collide_SphereIntersect(1.0, …)`]).
    /// `scripts` are the level's ScriptPlayers (`Drones::scripts`). Returns whether anything
    /// activated (the game's `bVar3` — it gates nothing else in the port).
    pub fn player_activate(
        &mut self, scripts: &mut crate::nis::Scripts, ch: &mut Channels, head: V3, look: V3, _frame: u32,
    ) -> bool {
        let p = [head[0] + look[0], head[1] + look[1], head[2] + look[2]];
        let near = |q: V3| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2) <= 1.0;
        let mut used = false;
        // SP_Activate 0x1dc338: mask & 4, the gate channel, and the out-channel gate when
        // mask & 1; then `ext+0x2C = 1` — the SP's own update plays it and holds its out.
        for sp in scripts.players.iter_mut() {
            if sp.deleted || sp.mask & 4 == 0 { continue; }
            if sp.gate != 0 && !ch.on(sp.gate as u32) { continue; }
            if sp.mask & 1 != 0 && !ch.on(sp.out as u32) { continue; }
            let m = sp.matrix;
            if near([m[3][0], m[3][1], m[3][2]]) {
                sp.activated = true;
                used = true;
            }
        }
        // Switch_Activate 0x1b02a8: gate on, value matches the channel, idle; a class-3
        // activator never toggles ch 0x61 — the port's use path is always the player (class 3),
        // so the alarm channel is excluded here exactly as the game's check does.
        for s in self.switches.iter_mut() {
            if s.busy && s.cooldown > 0 { continue; }
            if s.gate != 0 && !ch.on(s.gate as u32) { continue; }
            if s.ch == 0 { continue; } // [M 0x1b02a8:24 — a channel-less switch never activates]
            if s.ch == 0x61 { continue; }
            let cur = ch.v.get(s.ch as usize).copied().unwrap_or(0);
            if s.value != cur { continue; }
            if near(s.pos) {
                let v = if cur != 0 { 0u8 } else { 1u8 };
                ch.set(s.ch as u32, v, _frame);
                s.value = v;
                if s.busy { s.cooldown = 60; }
                used = true;
            }
        }
        // PCQWorm_Activate 0x1b0840: only when param 2 != 0 (the Tower PCs: 0 → E does nothing).
        for pc in self.pcs.iter_mut() {
            if pc.e_usable == 0 || pc.state != 0 { continue; }
            if near(pc.pos) {
                pc.state = 1;
                used = true;
            }
        }
        used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chans(on: &[u8]) -> Channels {
        let mut c = Channels::default();
        for &k in on { c.set(k as u32, 1, 0); }
        c
    }

    fn placement(flags: u32, pos: [f32; 3], params: &[(u32, u32)]) -> nf_formats::placement::Placement {
        nf_formats::placement::Placement {
            index: 0, hashcode: 0, flags, translation: pos, rotation: [0.0; 3],
            params: params.to_vec(), extra: [0; 9],
        }
    }

    /// The Q-Worm PC: a weapon-0x58 bullet reaching it writes its channel one tick later
    /// [M 0x1aee20]; a normal bullet does nothing; E does nothing when param 2 = 0 [M 0x1b0840].
    #[test]
    fn pcqworm_installs_on_qworm_bullet_only() {
        let ps = [placement(0x35, [10.0, 1.0, 10.0], &[(0, 0), (1, 0x48), (2, 0), (3, 1)])];
        let mut u = UseObjects::create(&ps);
        let mut ch = chans(&[]);
        let bullets = [( [10.0, 1.0, 10.4], [10.0, 1.0, 12.0], 0x58u16 )];
        u.tick(&bullets, &mut ch, 1);
        u.tick(&[], &mut ch, 2);
        assert!(ch.on(0x48), "the Q-Worm bullet installs the virus (ch 0x48) [M 0x1aee20]");
        // Sister: a rifle bullet must not.
        let mut u2 = UseObjects::create(&ps);
        let mut ch2 = chans(&[]);
        let rifle = [( [10.0, 1.0, 10.4], [10.0, 1.0, 12.0], 2u16 )];
        u2.tick(&rifle, &mut ch2, 1);
        u2.tick(&[], &mut ch2, 2);
        assert!(!ch2.on(0x48), "a normal bullet does not install [M 0x1aee20's 0x58 test]");
        // E with param 2 = 0 does nothing either.
        let mut scripts = crate::nis::Scripts::default();
        let mut u3 = UseObjects::create(&ps);
        let mut ch3 = chans(&[]);
        u3.player_activate(&mut scripts, &mut ch3, [9.0, 1.0, 10.0], [1.0, 0.0, 0.0], 0);
        assert!(!ch3.on(0x48) && u3.pcs[0].state == 0, "E cannot install when param 2 = 0 [M 0x1b0840]");
    }

    /// The switch: E toggles its channel when the gate allows and the value matches
    /// [M 0x1b02a8]. The alarm pulls (ch 0x7C/0x7D) are player-usable — that is how the alarm
    /// raises — while a switch whose channel IS 0x61 refuses the class-3 player activator.
    #[test]
    fn switch_toggles_channel_and_skips_alarm() {
        let ps = [
            placement(0x29, [0.0, 0.0, 0.0], &[(0, 9), (1, 0), (2, 0), (3, 0), (4, 1), (5, 0), (6, 0)]),
            placement(0x29, [5.0, 0.0, 0.0], &[(0, 0x7c), (1, 0), (2, 0), (3, 0), (4, 1), (5, 0), (6, 0)]),
            placement(0x29, [10.0, 0.0, 0.0], &[(0, 0x61), (1, 0), (2, 0), (3, 0), (4, 1), (5, 0), (6, 0)]),
        ];
        let mut u = UseObjects::create(&ps);
        let mut scripts = crate::nis::Scripts::default();
        let mut ch = chans(&[]);
        // E on the keycard lock (ch 9).
        assert!(u.player_activate(&mut scripts, &mut ch, [0.0, 1.6, 0.0], [0.0, -1.0, 0.0], 1));
        assert!(ch.on(9), "the switch toggles ch 9 [M 0x1b02a8]");
        assert!(!ch.on(0x7c), "the alarm pull is out of reach here");
        // E at the alarm pull (ch 0x7C): the player CAN raise the alarm with it.
        assert!(u.player_activate(&mut scripts, &mut ch, [5.0, 1.6, 0.0], [0.0, -1.0, 0.0], 2));
        assert!(ch.on(0x7c), "the alarm pull toggles ch 0x7C for the player [M 0x1b02a8]");
        // A switch whose channel IS 0x61 refuses the class-3 player [M 0x1b02a8's first gate].
        assert!(!u.player_activate(&mut scripts, &mut ch, [10.0, 1.6, 0.0], [0.0, -1.0, 0.0], 3));
        assert!(!ch.on(0x61), "the player cannot toggle a ch-0x61 switch");
    }

    /// SP E-activation: mask & 4 SPs activate (the elevator), gated channels refuse, and the
    /// activated SP's own update holds its OUT channel [M 0x1dc338 + 0x1dbda8].
    #[test]
    fn sp_activation_gates_and_fires() {
        // A mask-4 SP with out ch 0x39 at the use point, and a gated twin.
        let sp = |out: u16, gate: u16, mask: u16| crate::nis::Player {
            placement: 0, scripts: [1, 0], modes: [64, 0], stay: false, mask, out, gate,
            scale: 1.0, matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            cur: 0, state: 1, activated: false, countdown: 0, info: [None, None], deleted: false,
        };
        let mut scripts = crate::nis::Scripts::default();
        scripts.players = vec![sp(0x39, 0, 4), sp(0x40, 0x50, 4)];
        let ps: [nf_formats::placement::Placement; 0] = [];
        let mut u = UseObjects::create(&ps);
        let mut ch = chans(&[]);
        // In front of the first SP's matrix origin, looking at it.
        assert!(u.player_activate(&mut scripts, &mut ch, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1));
        assert!(scripts.players[0].activated, "the mask-4 SP activates [M 0x1dc338]");
        assert!(!scripts.players[1].activated, "the gated SP is out of reach anyway");
        // The gated twin, reached with its gate OFF: refused.
        let mut scripts2 = crate::nis::Scripts::default();
        scripts2.players = vec![sp(0x40, 0x50, 4)];
        let mut u2 = UseObjects::create(&ps);
        let mut ch2 = chans(&[]);
        u2.player_activate(&mut scripts2, &mut ch2, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1);
        assert!(!scripts2.players[0].activated, "gate ch 0x50 off → no activation [M 0x1dc338]");
        // And with the gate ON: activates.
        let mut ch3 = chans(&[0x50]);
        u2.player_activate(&mut scripts2, &mut ch3, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1);
        assert!(scripts2.players[0].activated, "gate on → activates");
    }
}
