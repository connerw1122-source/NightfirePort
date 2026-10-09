//! The EE music logic (ACTION.ELF): the event board (`Music_Event` 0x1de328), `UpdateMusicalEvents` (0x114e70), the
//! 07000005 updater `UpdateMFX_01CastleA` (0x112d78), `PickRandomCombat` / `PickRandomStealth`, and the attack gate
//! `NDrone2_SeenAndAttacking` (0x145150) with its `MusicVars`. Spec: docs/systems/music.md.
//!
//! Commands queued here reach the IOP player with the next frame's batch, as on the PS2.

use nf_formats::music::{MapMusic, MusicVars};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// `SFXStopMusic` + `SFXStartMusic(track, section)`.
    Start { track: i32, section: i32 },
    /// `SFXJumpToMusicMarker(section, instant)`.
    Jump { section: i32, instant: bool },
    /// `SFXFadeDown(fadeMusic)`.
    FadeDown { music: bool },
}

/// The attack gate's state (`MusicVars` 0x2707a0): attackers counted this frame, the latch, the drones counted.
#[derive(Clone, Debug, Default)]
pub struct AttackGate { pub vars: Option<MusicVars>, latch: bool, count: u16, counted: Vec<usize> }

impl AttackGate {
    /// `Drone_InitComms`' per-frame reset: the latch falls when no more than `floor` drones attacked last frame.
    pub fn new_frame(&mut self) {
        let floor = self.vars.map_or(0, |v| v.floor);
        if self.count <= floor { self.latch = false; }
        self.count = 0;
        self.counted.clear();
    }

    /// `NDrone2_SeenAndAttacking(drone)`: called by the drone combat states. `sees`: sees Bond now (`+0x16`);
    /// `since_seen`: frames since it last saw him (`+0x238`); `dist`: to him (`+0x1A0`). True = `Music_Event(2, 5)`.
    pub fn seen_and_attacking(&mut self, drone: usize, sees: bool, since_seen: u32, dist: f32) -> bool {
        let Some(v) = self.vars else { return false };
        let mut recent = since_seen < v.window * 60;
        if dist < v.distance && self.latch { recent = true; }
        if !(recent || sees) || self.counted.contains(&drone) { return false; }
        if self.counted.len() < 100 { self.counted.push(drone); }
        self.count += 1;
        if self.count >= v.count || self.latch || sees { self.latch = true; return true; }
        false
    }
}

/// The music state (`UpdateMusicalEvents` globals).
#[derive(Clone, Debug)]
pub struct Music {
    pub table: Vec<MapMusic>,
    entry: Option<MapMusic>,
    /// `EventList[64]`: one-frame pulses.
    pub events: [u32; 64],
    pub out: Vec<Cmd>,
    nis_playing: bool,
    nis_quitten: i32,
    delay_timer: i32,
    pub last_jump: i32,
    pub last_request: i32,
    pub wait_for_end: bool,
    under_attack: bool,
    count: i32,
    count2: i32,
    bonus: i32,
    which_combat: usize,
    which_stealth: usize,
    temp1: bool,
    temp2: i32,
    temp3: i32,
    temp4: bool,
}

impl Default for Music {
    fn default() -> Music {
        Music { table: Vec::new(), entry: None, events: [0; 64], out: Vec::new(), nis_playing: false, nis_quitten: 0, delay_timer: 0, last_jump: 0,
            last_request: -1, wait_for_end: false, under_attack: false, count: 0, count2: 0, bonus: 0, which_combat: 0, which_stealth: 0,
            temp1: false, temp2: 0, temp3: 0, temp4: false }
    }
}

impl Music {
    pub fn new(table: Vec<MapMusic>) -> Music { Music { table, ..Default::default() } }

    /// `Music_Event(i, v)`.
    pub fn event(&mut self, i: usize, v: u32) { if i < 64 { self.events[i] = v; } }

    fn in_set(set: &[i32], x: i32) -> bool { set.contains(&x) }

    fn jump(&mut self, s: i32, instant: bool) { self.last_request = s; self.out.push(Cmd::Jump { section: s, instant }); }

    /// `PickRandomCombat(flag)`: round robin over the combat set, unless a combat section is requested or playing.
    fn pick_combat(&mut self, flag: bool) {
        let Some(e) = &self.entry else { return };
        let set = e.combat.clone();
        if set.is_empty() || Self::in_set(&set, self.last_request) || Self::in_set(&set, self.last_jump) { return; }
        let s = set[self.which_combat % set.len()];
        self.which_combat += 1;
        self.jump(s, flag);
    }

    /// `PickRandomStealth`: only coming out of combat (the IOP reports a combat section).
    fn pick_stealth(&mut self) {
        let Some(e) = &self.entry else { return };
        let (st, cb) = (e.stealth.clone(), e.combat.clone());
        if st.is_empty() || Self::in_set(&st, self.last_request) || Self::in_set(&st, self.last_jump) || !Self::in_set(&cb, self.last_jump) { return; }
        let s = st[self.which_stealth % st.len()];
        self.which_stealth += 1;
        self.jump(s, false);
    }

    /// `UpdateMusicalEvents`, once per frame. `iop_section`: the section the IOP reported (`0x2500CC`).
    pub fn update(&mut self, iop_section: i32) {
        let e = self.events;
        if e[1] != 0 && e[0] != 0 && e[0] != 0x0700_0048 {
            if let Some(m) = self.table.iter().find(|m| m.level == e[0] && m.track != 0).cloned() {
                self.nis_playing = false;
                self.nis_quitten = 0;
                self.last_request = -1;
                self.last_jump = m.start;
                self.out.push(Cmd::Start { track: m.track, section: m.start });
                self.wait_for_end = false;
                (self.count, self.count2) = (m.calm_frames, m.alert_frames);
                (self.temp1, self.temp2, self.temp3, self.temp4) = (false, 0, 0, false);
                (self.under_attack, self.which_stealth, self.which_combat, self.bonus) = (false, 0, 0, 0);
                self.delay_timer = 10;
                self.entry = Some(m);
            }
        }
        let t = self.delay_timer - 1;
        if self.delay_timer < 0 { self.last_jump = iop_section; }
        if e[0xb] != 0 { self.nis_playing = true; }
        if e[0xc] != 0 || e[5] != 0 { self.nis_playing = false; }
        if e[5] != 0 { self.nis_quitten = 5; } else if self.nis_quitten > 0 { self.nis_quitten -= 1; }
        let Some(m) = self.entry.clone() else { self.events = [0; 64]; return };
        if !self.nis_playing {
            if e[2] == 2 { self.bonus = 600; self.under_attack = true; }
            if e[2] == 0 || e[2] == 2 {
                let b = self.bonus; self.bonus -= 1;
                if b < 0 { let c = self.count; self.count -= 1; if c < 0 { self.under_attack = false; self.count2 = m.alert_frames; } }
            } else {
                let c2 = self.count2; self.count2 -= 1;
                if c2 < 0 { self.under_attack = true; self.bonus = 0; self.count = m.calm_frames; }
            }
        }
        self.delay_timer = t;
        if e[6] != 0 { self.jump(m.on_death, true); self.out.push(Cmd::FadeDown { music: false }); self.wait_for_end = true; }
        if e[7] != 0 { self.jump(m.on_fail, true); self.out.push(Cmd::FadeDown { music: false }); self.wait_for_end = true; }
        if !self.wait_for_end && m.level == 0x0700_0005 { self.castle_a(&e, &m); }
        self.events = [0; 64];
    }

    /// `UpdateMFX_01CastleA` (07000005).
    fn castle_a(&mut self, e: &[u32; 64], m: &MapMusic) {
        let in_combat = |x: i32| m.combat.contains(&x);
        if e[2] == 3 && !self.temp4 { self.temp4 = true; self.temp3 = 1200; self.pick_combat(false); return; }
        if self.temp3 > 0 { self.temp3 -= 1; return; }
        if e[0xc] == 0x0600_0994 { self.temp1 = false; self.temp2 = 66; }
        if self.temp2 > 0 { self.temp2 -= 1; return; }
        if e[0xb] == 0x0600_0994 || self.temp1 {
            let s = if in_combat(self.last_jump) { 6 } else { 2 };
            self.jump(s, false);
            self.temp1 = true;
            return;
        }
        if self.under_attack { self.pick_combat(false); }
        else if e[0xd] != 0 { let s = if in_combat(self.last_jump) { 6 } else { 2 }; self.jump(s, false); }
        else if self.last_request != 2 { self.pick_stealth(); }
        else { self.jump(20, false); }
        if e[8] != 0 || e[3] != 0 {
            let s = if in_combat(self.last_jump) { m.end_combat } else { m.end_calm };
            self.jump(s, true);
            self.out.push(Cmd::FadeDown { music: false });
            self.wait_for_end = true;
        }
        if !self.wait_for_end && e[4] != 0 { self.jump(9, false); self.wait_for_end = true; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn castle() -> MapMusic {
        MapMusic { level: 0x0700_0005, track: 15, start: 0, on_death: 11, on_fail: 10, end_combat: 8, end_calm: 9, alert_frames: 10, calm_frames: 160, combat: vec![4, 12, 14], stealth: vec![6, 16, 18] }
    }

    #[test]
    fn alert_hysteresis_and_round_robin() {
        let mut m = Music::new(vec![castle()]);
        m.event(0, 0x0700_0005); m.event(1, 1);
        m.update(0);
        assert_eq!(m.out.drain(..).collect::<Vec<_>>(), vec![Cmd::Start { track: 15, section: 0 }]);
        // Combat on the 12th cumulative alerted frame (count2 10 → −1 after 11 decrements, the 12th sees < 0).
        let mut first = None;
        for f in 1..=20 { m.event(2, 5); m.update(0); if first.is_none() && m.out.iter().any(|c| *c == Cmd::Jump { section: 4, instant: false }) { first = Some(f); } m.out.clear(); }
        assert_eq!(first, Some(12));
        // The IOP reports section 4; 163 calm frames later stealth 6 is requested (sister: not at 162).
        let mut at = None;
        for f in 1..=170 { m.update(4); if at.is_none() && m.out.iter().any(|c| *c == Cmd::Jump { section: 6, instant: false }) { at = Some(f); } m.out.clear(); }
        assert_eq!(at, Some(163));
        // Death: the instant end sting, then nothing more.
        m.event(6, 1); m.update(6);
        assert!(m.out.contains(&Cmd::Jump { section: 11, instant: true }) && m.wait_for_end);
    }

    #[test]
    fn attack_gate_needs_two_recent_attackers_or_sight() {
        let mut g = AttackGate { vars: Some(MusicVars { level: 0x0700_0005, count: 2, floor: 0, window: 2, distance: 35.0 }), ..Default::default() };
        g.new_frame();
        assert!(!g.seen_and_attacking(0, false, 30, 50.0), "one recent attacker is not enough");
        assert!(g.seen_and_attacking(1, false, 30, 50.0), "the second one is");
        g.new_frame();
        assert!(g.seen_and_attacking(2, true, 0, 50.0), "seeing Bond is enough");
        // Sister: not seen for 2 s and far → nothing.
        g.new_frame(); g.new_frame();
        assert!(!g.seen_and_attacking(3, false, 200, 50.0));
    }
}
