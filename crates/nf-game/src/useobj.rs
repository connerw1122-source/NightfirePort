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

/// A test World: no anims; the model table only knows hash 0x11 (a panel at (5,0,0), r 0.5).
struct TestWorld;
impl crate::nis::World for TestWorld {
    fn anim_length(&self, _script: u32) -> Option<u16> { None }
    fn discrete0(&self, _skin: u32, _script: u32, _frame: f32) -> Option<nf_formats::anim::Mat4> { None }
    fn glist_sphere(&self, hash: u32) -> Option<([f32; 3], f32)> {
        match hash { 0x11 => Some(([5.0, 0.0, 0.0], 0.5)), _ => None }
    }
}

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
    /// param 5: the inhibit gate — usable only while this channel is OFF (0 = always allowed)
    /// [M 0x1b02a8:22 `(*(ushort *)(iVar2 + 0x14) == 0 || (switch_channels[+0x14] == '\0'))`].
    pub gate: u8,
    /// param 3: the rest value the auto-return flips toward [M 0x1adea8].
    pub rest: u8,
    /// param 4: the busy flag — with it set, the switch activates only while its channel has
    /// never changed (`Switch_Activate` 0x1b02a8:12-16 returns when +0x18 != 0; +0x18 is bumped
    /// on every channel change in `Switch_Update` 0x1adea8:22,29) — a one-shot latch, not a
    /// re-armable cooldown.
    pub busy: bool,
    /// param 2: the auto-toggle delay in frames (0 = none) [M 0x1adea8].
    pub auto_delay: u16,
    /// `obj+0xF4`: the stored value [M 0x1adcc0 creates it from param 3].
    pub value: u8,
    /// `inst+0x18`: the change latch — bumped on every channel change [M 0x1adea8:22,29].
    pub latch: u8,
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
    /// `Switch_GetNearest` (0x1b0398) + `Switch_GetSwitchChannel` (0x1b0430): the nearest switch
    /// object's channel (squared 3-D distance to the given position; the game walks the SwitchList in
    /// creation order). 0 when there is no switch.
    pub fn nearest_switch_channel(&self, pos: V3) -> u8 {
        let mut best = None;
        for s in &self.switches {
            let d = (s.pos[0] - pos[0]).powi(2) + (s.pos[1] - pos[1]).powi(2) + (s.pos[2] - pos[2]).powi(2);
            if best.is_none_or(|(b, _)| d < b) { best = Some((d, s.ch)); }
        }
        best.map_or(0, |(_, ch)| ch)
    }

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
                    auto_delay: par(2) as u16, value: par(3) as u8, auto_ct: 0,
                    latch: 0,
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
            // The stored value follows the channel; every change bumps the latch [M 0x1adea8:20-23].
            let cur = ch.v.get(s.ch as usize).copied().unwrap_or(0);
            if s.value != cur {
                s.value = cur;
                s.latch = s.latch.saturating_add(1);
            }
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
    /// `scripts` are the level's ScriptPlayers (`Drones::scripts`); `w` is the level data (for the
    /// entity/model sphere lookups). Returns whether anything activated (the game's `bVar3` — it
    /// gates nothing else in the port).
    ///
    /// The SP reach (elevator-buttons-09): the game dispatches `SP_Activate` only on a hit object
    /// with `obj+0xE4 != 0 && class == 0` [M 0x1a5170:121-122] — a script ENTITY of that SP
    /// (`Script_EntityStart` 0x1d8d78:108-110 stores the owner handle). The SP body (class 0x1B)
    /// is inert to the pick. So each mask-4 SP tests:
    ///   1. its live entity spheres (`Player::entity_use_targets`) [M];
    ///   2. else its own model sphere (`model` — e.g. the elevator panel, whose art is baked
    ///      absolute in the model [D gfx bounds]) [S stand-in];
    ///   3. else its placement matrix (the original stand-in, for SPs whose scripts have created
    ///      nothing yet) [S stand-in].
    pub fn player_activate(
        &mut self, scripts: &mut crate::nis::Scripts, ch: &mut Channels, head: V3, look: V3, frame: u32,
        w: &dyn crate::nis::World,
    ) -> bool {
        let p = [head[0] + look[0], head[1] + look[1], head[2] + look[2]];
        let near = |q: V3| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2) <= 1.0;
        let in_sphere = |c: [f32; 3], r: f32| {
            let d = (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2);
            d <= (1.0 + r) * (1.0 + r)
        };
        let mut used = false;
        // SP_Activate 0x1dc338: mask & 4, the gate channel, and the out-channel gate when
        // mask & 1; then `ext+0x2C = 1` — the SP's own update plays it and holds its out.
        for spi in 0..scripts.players.len() {
            {
                let sp = &mut scripts.players[spi];
                if sp.deleted || sp.mask & 4 == 0 { continue; }
                if sp.gate != 0 && !ch.on(sp.gate as u32) { continue; }
                if sp.mask & 1 != 0 && !ch.on(sp.out as u32) { continue; }
                // 1. The entity targets [M 0x1a5170:121-122 + 0x1d8d78:108-110].
                let targets = sp.entity_use_targets(&scripts.objects, w);
                let hit = if !targets.is_empty() {
                    targets.iter().any(|&(c, r)| in_sphere(c, r))
                } else if let Some((c, r)) = (sp.model != 0)
                    .then(|| w.glist_sphere(sp.model))
                    .flatten() {
                    // 2. The SP's own model sphere [S stand-in — see the doc above]. Exclusive:
                    // a resolved model IS the SP's world body, the (often origin) placement
                    // matrix must not add a second chance.
                    in_sphere(c, r)
                } else {
                    // 3. The placement matrix (the original stand-in).
                    let m = sp.matrix;
                    near([m[3][0], m[3][1], m[3][2]])
                };
                if hit {
                    sp.activated = true;
                    // SP_Activate 0x1dc338:84-86's second write: the current SCRIPTINFO's
                    // `+0x9C = 0` (the port's `delay`). Without it a script held by an event 3
                    // (`+0x9C = p0 − 1` [M 0x1d94c0:98-117]) never resumes: 0x0600059B's stream 1
                    // carries `Event{kind 3, p0 = 0}` at t=0 [D nis dump], so the preload leaves
                    // the hold latched and the activation must clear it for the FX to play.
                    if let Some(s) = sp.info[sp.cur].as_mut() { s.delay = 0; }
                    used = true;
                }
            }
        }
        // Switch_Activate 0x1b02a8: gate on, value matches the channel, idle; a class-3
        // activator never toggles ch 0x61 — the port's use path is always the player (class 3),
        // so the alarm channel is excluded here exactly as the game's check does.
        for s in self.switches.iter_mut() {
            if s.busy && s.latch != 0 { continue; }
            // The inhibit gate: usable while the gate channel is OFF [M 0x1b02a8:22].
            if s.gate != 0 && ch.on(s.gate as u32) { continue; }
            if s.ch == 0 { continue; } // [M 0x1b02a8:24 — a channel-less switch never activates]
            if s.ch == 0x61 { continue; }
            let cur = ch.v.get(s.ch as usize).copied().unwrap_or(0);
            if s.value != cur { continue; }
            if near(s.pos) {
                let v = if cur != 0 { 0u8 } else { 1u8 };
                ch.set(s.ch as u32, v, frame);
                s.value = v;
                // The toggle's mirror mismatch bumps +0x18 next Update [M 0x1adea8:20-23]; the
                // port syncs the mirror here, so it bumps the latch at the same point.
                s.latch = s.latch.saturating_add(1);
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
        u3.player_activate(&mut scripts, &mut ch3, [9.0, 1.0, 10.0], [1.0, 0.0, 0.0], 0, &TestWorld);
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
        assert!(u.player_activate(&mut scripts, &mut ch, [0.0, 1.6, 0.0], [0.0, -1.0, 0.0], 1, &TestWorld));
        assert!(ch.on(9), "the switch toggles ch 9 [M 0x1b02a8]");
        assert!(!ch.on(0x7c), "the alarm pull is out of reach here");
        // E at the alarm pull (ch 0x7C): the player CAN raise the alarm with it.
        assert!(u.player_activate(&mut scripts, &mut ch, [5.0, 1.6, 0.0], [0.0, -1.0, 0.0], 2, &TestWorld));
        assert!(ch.on(0x7c), "the alarm pull toggles ch 0x7C for the player [M 0x1b02a8]");
        // A switch whose channel IS 0x61 refuses the class-3 player [M 0x1b02a8's first gate].
        assert!(!u.player_activate(&mut scripts, &mut ch, [10.0, 1.6, 0.0], [0.0, -1.0, 0.0], 3, &TestWorld));
        assert!(!ch.on(0x61), "the player cannot toggle a ch-0x61 switch");
    }

    /// SP E-activation: mask & 4 SPs activate (the elevator), gated channels refuse, and the
    /// activated SP's own update holds its OUT channel [M 0x1dc338 + 0x1dbda8].
    #[test]
    fn sp_activation_gates_and_fires() {
        // A mask-4 SP with out ch 0x39 at the use point, and a gated twin.
        let sp = |out: u16, gate: u16, mask: u16| sp_with_model(out, gate, mask, 0);
        let mut scripts = crate::nis::Scripts::default();
        scripts.players = vec![sp(0x39, 0, 4), sp(0x40, 0x50, 4)];
        let ps: [nf_formats::placement::Placement; 0] = [];
        let mut u = UseObjects::create(&ps);
        let mut ch = chans(&[]);
        // In front of the first SP's matrix origin, looking at it.
        assert!(u.player_activate(&mut scripts, &mut ch, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1, &TestWorld));
        assert!(scripts.players[0].activated, "the mask-4 SP activates [M 0x1dc338]");
        assert!(!scripts.players[1].activated, "the gated SP is out of reach anyway");
        // The gated twin, reached with its gate OFF: refused.
        let mut scripts2 = crate::nis::Scripts::default();
        scripts2.players = vec![sp(0x40, 0x50, 4)];
        let mut u2 = UseObjects::create(&ps);
        let mut ch2 = chans(&[]);
        u2.player_activate(&mut scripts2, &mut ch2, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1, &TestWorld);
        assert!(!scripts2.players[0].activated, "gate ch 0x50 off → no activation [M 0x1dc338]");
        // And with the gate ON: activates.
        let mut ch3 = chans(&[0x50]);
        u2.player_activate(&mut scripts2, &mut ch3, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1, &TestWorld);
        assert!(scripts2.players[0].activated, "gate on → activates");
    }

    /// The model-sphere fallback [S stand-in, elevator-buttons-09]: a mask-4 SP whose placement
    /// matrix is the origin (like 07000009's #70) activates through its own model's world-baked
    /// sphere instead — and NOT through a model the World cannot resolve.
    #[test]
    fn sp_model_sphere_fallback_replaces_origin_matrix() {
        // model 0x11 = the panel at (5,0,0) r 0.5 (TestWorld).
        let mut scripts = crate::nis::Scripts::default();
        scripts.players = vec![sp_with_model(0x39, 0, 4, 0x11)];
        let ps: [nf_formats::placement::Placement; 0] = [];
        let mut u = UseObjects::create(&ps);
        let mut ch = chans(&[]);
        // Near the panel (the use point head+look lands within 1.0 + 0.5 of (5,0,0)), NOT near
        // the origin matrix.
        assert!(u.player_activate(&mut scripts, &mut ch, [5.0, 0.3, 0.4], [0.0, 0.0, 1.0], 1, &TestWorld),
            "the model-baked panel is the use target");
        assert!(scripts.players[0].activated);
        // Far from the panel: no activation.
        let mut scripts2 = crate::nis::Scripts::default();
        scripts2.players = vec![sp_with_model(0x39, 0, 4, 0x11)];
        let mut u2 = UseObjects::create(&ps);
        let mut ch2 = chans(&[]);
        assert!(!u2.player_activate(&mut scripts2, &mut ch2, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1, &TestWorld),
            "the origin placement matrix must NOT activate a model-baked SP");
        assert!(!scripts2.players[0].activated);
        // An SP whose model the World cannot resolve falls back to the placement matrix (the
        // original stand-in — the existing behaviour the older SPs rely on).
        let mut scripts3 = crate::nis::Scripts::default();
        scripts3.players = vec![sp_with_model(0x39, 0, 4, 0xdead)];
        let mut u3 = UseObjects::create(&ps);
        let mut ch3 = chans(&[]);
        assert!(u3.player_activate(&mut scripts3, &mut ch3, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1, &TestWorld),
            "no model sphere → the placement matrix stand-in holds");
    }

    /// A Player literal with a tunable model hash (the SP test's constructor, field on its own
    /// line per the merge discipline).
    fn sp_with_model(out: u16, gate: u16, mask: u16, model: u32) -> crate::nis::Player {
        crate::nis::Player {
            placement: 0, scripts: [1, 0], modes: [64, 0], stay: false, mask, out, gate,
            scale: 1.0, matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            cur: 0, state: 1, activated: false, countdown: 0, info: [None, None], deleted: false,
            live_matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            subs: Vec::new(),
            hit_points: 0.0,
            damage: 0.0,
            tick_damage: 0.0,
            // +0x36: nothing hides a placed/fusebox/useobj SP at create [M 0x1db9a8].
            hidden: false,
            model,
        }
    }

    /// docs/fields/switch.md +0x14: the inhibit gate is usable while the channel is OFF
    /// [M 0x1b02a8:22]. Fail-first: the old port required the channel ON.
    #[test]
    fn switch_inhibit_gate_blocks_only_while_on() {
        let mut u = UseObjects::default();
        u.switches.push(SwitchObj { pos: [0.0, 0.5, -0.5], ch: 9, gate: 7, rest: 0, busy: false,
            auto_delay: 0, value: 0, latch: 0, auto_ct: 0 });
        // Gate channel OFF: the switch activates.
        let mut scripts = crate::nis::Scripts::default();
        let mut ch = Channels::default();
        let used = u.player_activate(&mut scripts, &mut ch, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1, &TestWorld);
        assert!(used && ch.on(9), "gate channel OFF → the switch activates");
        // Sister: the same switch with its gate channel ON never activates.
        let mut u2 = UseObjects::default();
        u2.switches.push(SwitchObj { pos: [0.0, 0.5, -0.5], ch: 9, gate: 7, rest: 0, busy: false,
            auto_delay: 0, value: 0, latch: 0, auto_ct: 0 });
        let mut scripts2 = crate::nis::Scripts::default();
        let mut ch2 = chans(&[7]);
        let used2 = u2.player_activate(&mut scripts2, &mut ch2, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1, &TestWorld);
        assert!(!used2 && !ch2.on(9), "gate channel ON → blocked");
        // Sister 2: gate 0 always allows.
        let mut u3 = UseObjects::default();
        u3.switches.push(SwitchObj { pos: [0.0, 0.5, -0.5], ch: 9, gate: 0, rest: 0, busy: false,
            auto_delay: 0, value: 0, latch: 0, auto_ct: 0 });
        let mut scripts3 = crate::nis::Scripts::default();
        let mut ch3 = Channels::default();
        assert!(u3.player_activate(&mut scripts3, &mut ch3, [0.0, 0.5, -0.5], [0.0, 0.0, 1.0], 1, &TestWorld),
            "no gate → activates");
    }

    /// docs/fields/switch.md +0x18: with param 4 (busy) set the switch is a ONE-SHOT — its latch
    /// (+0x18) bumps on the first channel change and never resets [M 0x1adea8:22,29; gate
    /// 0x1b02a8:12-16]. Fail-first: the old port re-armed after a 60-tick cooldown.
    #[test]
    fn switch_busy_is_one_shot_never_rearms() {
        let mut u = UseObjects::default();
        u.switches.push(SwitchObj { pos: [0.0, 0.5, -0.5], ch: 9, gate: 0, rest: 0, busy: true,
            auto_delay: 0, value: 0, latch: 0, auto_ct: 0 });
        let mut scripts = crate::nis::Scripts::default();
        let mut ch = Channels::default();
        let head = [0.0, 0.5, -0.5]; let look = [0.0, 0.0, 1.0];
        assert!(u.player_activate(&mut scripts, &mut ch, head, look, 1, &TestWorld), "first use activates");
        // Let the tick see the channel change (the game's mirror test bumps +0x18).
        u.tick(&[], &mut ch, 2);
        assert_eq!(u.switches[0].latch, 1, "the channel change bumped the latch");
        assert!(!u.player_activate(&mut scripts, &mut ch, head, look, 3, &TestWorld), "second use blocked");
        // After 61 ticks the game is still dead (the latch never resets).
        for f in 4..70 { u.tick(&[], &mut ch, f); }
        assert!(!u.player_activate(&mut scripts, &mut ch, head, look, 70, &TestWorld),
            "a busy switch never re-arms");
    }
}
