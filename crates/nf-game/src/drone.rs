//! Drone (guard) AI: the standing-combat part of `NDrone2_*`, ported from ACTION.ELF. Spec: docs/systems/drone-ai.md.
//!
//! Field names carry the game's `Drone+0x…` offsets. One `tick` = one 60 Hz game update (`FRAME_RATE_DIV` = 1).
//! Ported here: state machine (enter / exit / update / anim-complete / timer messages), vision (visibility score, line
//! of sight round-robin, reaction time, `EnemyLookForOpponent`), the Idle → Attack → Combat → AimStandFire ⇄ reload
//! chain, turning (`NDrone2_Steer`), upper-body aim, firing (`DroneWeap_*`), animation calls (`DroneAnim_CallAnim`).
//! Not yet: combat moves (steps, strafes, rolls, cover), patrols, hearing, drone-to-drone alerts, bullets/damage.

use crate::trig::{angle_difference, atan2_approx, rotate_to_bearing};
use crate::world::{CollisionWorld, V3};
use nf_formats::anim::Script;
use nf_formats::drone::{DroneAnimData, PropertyTables};
use std::collections::HashMap;

pub mod state {
    pub const GLOBAL: u8 = 0x00;
    pub const IDLE: u8 = 0x04;
    pub const ALERT: u8 = 0x05;
    pub const INIT_PATROL: u8 = 0x06;
    pub const PATROL: u8 = 0x07;
    pub const ATTACK: u8 = 0x56;
    pub const COMBAT: u8 = 0x58;
    pub const COMBAT_NO_MOVE: u8 = 0x59;
    pub const COMBAT_OUT_OF_RANGE: u8 = 0x5a;
    pub const COMBAT_NEW_SIGHTING: u8 = 0x5b;
    pub const COMBAT_NO_SIGHT: u8 = 0x5c;
    pub const NO_OPPONENT: u8 = 0x60;
    pub const AIM_STAND: u8 = 0x67;
    pub const AIM_STAND_FIRE: u8 = 0x68;
    pub const AIM_STAND_RELOAD: u8 = 0x69;
}

/// Messages (`*(int*)MsgObject`).
pub mod msg {
    pub const ENTER: u8 = 1;
    pub const EXIT: u8 = 2;
    pub const UPDATE: u8 = 3;
    pub const ANIM_DONE: u8 = 5;
    pub const TIMER_C: u8 = 0xc;
}

/// Anim states (`DroneAnimStates` index).
pub mod anim_state {
    pub const AIM_STAND: u16 = 0x0a;
    pub const STAND_IDLE: u16 = 0x24;
    pub const RELOAD: u16 = 0x52;
}

/// Read-only game data the AI consults.
pub struct Data<'a> {
    pub tables: &'a DroneAnimData,
    pub props: &'a PropertyTables,
    pub scripts: &'a HashMap<u32, Script>,
    pub weapons: &'a nf_formats::weapon::WeaponData,
    pub world: &'a CollisionWorld,
    pub level: u32,
}

/// Bond as the drones see him.
#[derive(Clone, Copy, Debug, Default)]
pub struct Player {
    /// Object position (feet level + collision offset; for the viewer: the camera's ground point).
    pub pos: V3,
    /// The point drones aim at and test sight against (`Drone_View_Bones`: chest / head; one point here).
    pub target: V3,
}

/// Deterministic stand-in for `Rand_FRand` (the game's generator state is not modelled).
#[derive(Clone, Debug)]
pub struct Rng(u32);
impl Rng {
    pub fn new(seed: u32) -> Rng { Rng(seed | 1) }
    pub fn frand(&mut self, max: f32) -> f32 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 17; self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32 * max
    }
}

/// Animation side of a drone: `DroneAnim_*` over the danim tables, playing scripts.
#[derive(Clone, Debug)]
pub struct Animator {
    pub danim: u16,
    pub script: Option<u32>,
    /// Ticks since the script started (script time = 1 + this).
    pub time: u32,
    /// `Drone+0x56A` anim state of the playing danim, `+0x568` its class (2 = aiming).
    pub state: u16,
    pub class: u8,
    /// Pending `DroneAnim_CallAnim`: (anim state, AI state on completion, message on completion).
    pub call: Option<(u16, u8, u8)>,
    pub started_call: bool,
}

#[derive(Clone, Debug)]
pub struct Drone {
    pub skin: u32,
    pub anim_set: u16,
    pub weapon: u32,
    pub pos: V3,
    /// obj+0x54 and `Drone+0x694`.
    pub yaw: f32,
    pub desired_yaw: f32,
    /// Upper-body aim yaw (`AnimObjectAimAt`, anim+0xF4) relative to the body.
    pub aim_yaw: f32,
    pub alert: f32,
    pub alert_floor: f32,
    pub props: [u32; 3],
    /// `+0xE8` sight range, `+0xE4` half field of view, `+0xF0` engage range, `+0xFC` combat sight range.
    pub sight_range: f32,
    pub half_fov: f32,
    pub engage_range: f32,
    pub combat_range: f32,
    /// Stats bytes `+0xB4` accuracy, `+0xB5` aggression, `+0xB8` reaction.
    pub accuracy: u8,
    pub aggression: u8,
    pub reaction: u8,
    // Vision (`+0x228` bits 2 = line of sight, 4 = seen this tick, 8 = ever seen; `+0x230`, `+0x270`, `+0x274`).
    pub los: bool,
    pub seen: bool,
    pub ever_seen: bool,
    pub visibility: f32,
    pub seen_ticks: u32,
    pub unseen_ticks: u32,
    pub vision_on: bool,
    // Geometry to the opponent (`Drone_GetOpponentInfo`): +0x1A0 distance, +0x1C4 yaw to it.
    pub dist: f32,
    pub yaw_to_opp: f32,
    // State machine.
    pub state: u8,
    pub prev_state: u8,
    pub next_state: Option<u8>,
    pub entered: u32,
    pub timer_c: Option<u32>,
    pub anim: Animator,
    // Firing (`+0x3B` firing, `+0x3C` burst done, `+0xBC0` burst, `+0xBC4` next shot, `+0xBC8` last shot, clip).
    pub firing: bool,
    pub burst_done: bool,
    pub burst: i32,
    pub next_shot: u32,
    pub last_shot: u32,
    pub clip: i32,
    pub clip_max: i32,
    /// Frames of the shots fired (for flashes; the viewer drains it).
    pub shots: Vec<u32>,
    /// Trace of (frame, state) changes, for comparisons with PCSX2.
    pub log: Vec<(u32, u8)>,
}

impl Drone {
    /// From a placement (parameters by index) after `NDrone2_DefaultInit` / `DoModeSettingsNEW`.
    pub fn new(d: &Data, pos: V3, yaw: f32, param: impl Fn(u32) -> u32, rng: &mut Rng) -> Drone {
        let skin = param(0);
        let setup = nf_formats::drone::drone_setup(skin, param(9), d.level);
        let (start_danim, anim_set) = nf_formats::drone::start_danim(setup, 0);
        let (normal, _alert) = nf_formats::drone::behaviour_words(&param);
        let alert = nf_formats::drone::ALERTNESS_LEVELS[d.props.get(0x20, &normal) as usize & 3];
        let weapon = nf_formats::drone::weapon_id(skin, param(9));
        // Sight constants by class (parameter 14), `NDrone2_DefaultInit`; e4 = π/2 for every class.
        let (e8, f0_base, f0_rand, fc_add) = match param(14) { 1 => (12.0, 12.0, 3.0, 1.0), 2 => (16.0, 12.0, 3.0, 1.0),
            3 => (40.0, 25.0, 6.0, 100.0), 4 => (1000.0, 75.0, 8.0, 1000.0), 5 => (0.0, 12.0, 4.0, 1.0), _ => (24.0, 12.0, 4.0, 1.0) };
        let f0 = f0_base + rng.frand(f0_rand);
        let fc = match param(14) { 3 => fc_add + rng.frand(6.0), 4 => 1000.0, _ => f0 + fc_add + rng.frand(f0_rand) };
        let clip = match weapon { 0x16 | 0x18..=0x1b => 0x40, 0x1e | 0x1f | 0x24 | 0x25 => 5, _ => 0x40 };
        let mut dr = Drone {
            skin, anim_set, weapon, pos, yaw, desired_yaw: yaw, aim_yaw: 0.0, alert, alert_floor: alert, props: normal,
            sight_range: e8, half_fov: crate::trig::HALF_PI, engage_range: f0, combat_range: fc,
            // `drone_stats` for the guards' type is all zero at run time (07000005 snapshot) → defaults.
            accuracy: 5, aggression: 0, reaction: 0,
            los: false, seen: false, ever_seen: false, visibility: 0.0, seen_ticks: 0, unseen_ticks: 0, vision_on: false,
            dist: 0.0, yaw_to_opp: 0.0,
            state: state::GLOBAL, prev_state: 0, next_state: None, entered: 0, timer_c: None,
            anim: Animator { danim: 0, script: None, time: 0, state: 0, class: 0, call: None, started_call: false },
            firing: false, burst_done: false, burst: 0, next_shot: 0, last_shot: 0, clip, clip_max: clip,
            shots: Vec::new(), log: Vec::new(),
        };
        dr.set_danim(d, start_danim);
        dr
    }

    fn prop(&self, d: &Data, id: usize) -> bool { d.props.get(id, &self.props) != 0 }

    // ---- animation ---------------------------------------------------------------------------------------------

    /// `DroneAnim_SetDAnimInternal`: start a danim's script for our anim set.
    fn set_danim(&mut self, d: &Data, danim: u16) {
        let info = d.tables.info.get(danim as usize).copied().unwrap_or([0; 24]);
        self.anim.danim = danim;
        self.anim.state = u16::from_le_bytes([info[8], info[9]]);
        self.anim.class = info[0xc];
        self.anim.script = d.tables.script_for(danim, self.anim_set);
        self.anim.time = 0;
    }

    /// `DroneAnim_CallAnim(blend, state, variant, next AI state, completion message)`.
    fn call_anim(&mut self, state: u16, next_ai: u8, done_msg: u8) {
        self.anim.call = Some((state, next_ai, done_msg));
        self.anim.started_call = false;
    }

    /// Advances the script; at its end continues with the danim's follow-on (`Drone_AnimInfo +10`); handles a pending
    /// call (`DroneAnim_CallHandler`): start the transition danim, complete when the anim state is reached.
    fn update_anim(&mut self, d: &Data) -> Option<(u8, u8)> {
        self.anim.time += 1;
        let len = self.anim.script.and_then(|h| d.scripts.get(&h)).map(|s| s.length as u32).unwrap_or(1);
        if self.anim.time >= len {
            let info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
            let next = u16::from_le_bytes([info[10], info[11]]);
            if next != 0 && next != self.anim.danim { self.set_danim(d, next) } else { self.anim.time = 0 }
        }
        let (want, next_ai, done_msg) = self.anim.call?;
        // CallFullyComplete when the playing danim's anim state (+0x56A, set as the danim starts) is the requested one.
        if self.anim.state == want {
            self.anim.call = None;
            return Some((next_ai, done_msg));
        }
        if !self.anim.started_call {
            self.anim.started_call = true;
            let danim = d.tables.danim_for_state(want, self.anim.state).or_else(|| d.tables.danim_for_state(want, 0));
            if let Some(da) = danim.filter(|&x| x > 0) { self.set_danim(d, da as u16) }
        }
        None
    }

    // ---- state machine -----------------------------------------------------------------------------------------

    fn set_state(&mut self, s: u8) { if s != 0 { self.next_state = Some(s) } }

    /// `Drone_SM_RouteMsgDCV`: deliver, then apply pending changes (exit → enter, repeating).
    fn send(&mut self, d: &Data, rng: &mut Rng, frame: u32, m: u8) {
        if !self.handle(d, rng, frame, self.state, m) { self.handle(d, rng, frame, state::GLOBAL, m); }
        let mut guard = 0;
        while let Some(n) = self.next_state.take() {
            self.handle(d, rng, frame, self.state, msg::EXIT);
            self.prev_state = self.state;
            self.state = n;
            self.vision_on = false; // RouteMsgDCV clears 0x4f8 0x10000 on every change
            self.entered = frame;
            self.log.push((frame, n));
            self.handle(d, rng, frame, n, msg::ENTER);
            guard += 1;
            if guard > 16 { break; }
        }
    }

    fn handle(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8, m: u8) -> bool {
        use state::*;
        match (s, m) {
            (GLOBAL, msg::ENTER) => { self.set_state(if self.prop(d, 0x24) || self.prop(d, 0x25) { INIT_PATROL } else { IDLE }); true }
            (INIT_PATROL, msg::ENTER) => { self.set_state(IDLE); true } // no AI paths yet: InitPatrol's fallback
            (IDLE | ALERT | PATROL, msg::ENTER) => {
                self.vision_on = true;
                if s == IDLE && self.alert >= 0.66 { self.set_state(ALERT) }
                true
            }
            (IDLE | ALERT | PATROL, msg::UPDATE) => { if let Some(n) = self.look_for_opponent(d) { self.set_state(n) } true }
            (ATTACK, msg::ENTER) => {
                // DroneFunc_FirstAttack: alertness 1, combat sight; (first-sight combat moves not ported yet)
                self.alert = 1.0;
                self.sight_range = self.sight_range.max(self.combat_range);
                self.half_fov = 0.0;
                self.set_state(if self.prop(d, 0x18) && self.dist < self.engage_range { AIM_STAND } else { COMBAT });
                true
            }
            (COMBAT, msg::ENTER) => { self.alert = 1.0; true }
            (COMBAT | COMBAT_NO_MOVE | COMBAT_OUT_OF_RANGE, msg::UPDATE) => {
                if self.unseen_ticks < 16 { self.set_state(COMBAT_NEW_SIGHTING) } else { self.set_state(COMBAT_NO_SIGHT) }
                true
            }
            (COMBAT_NEW_SIGHTING, msg::ENTER) => { self.set_state(AIM_STAND_FIRE); true }
            (AIM_STAND, msg::ENTER) => { self.call_anim(anim_state::AIM_STAND, 0, msg::ANIM_DONE); self.firing = false; true }
            (AIM_STAND, msg::ANIM_DONE) => { self.set_state(if self.unseen_ticks < 16 { AIM_STAND_FIRE } else { COMBAT_NO_SIGHT }); true }
            (AIM_STAND_FIRE, msg::ENTER) => {
                self.call_anim(anim_state::AIM_STAND, 0, 0);
                self.vision_on = true;
                if self.unseen_ticks < 16 { self.fire(d, rng, frame, true) } else { self.set_state(COMBAT_NO_SIGHT) }
                true
            }
            (AIM_STAND_FIRE, msg::UPDATE) => {
                self.turn_to_opponent(0.524);
                if self.unseen_ticks >= 16 { self.set_state(COMBAT_NO_SIGHT); return true; }
                if self.firing && !self.burst_done { return true; }
                if self.clip < 1 { self.set_state(AIM_STAND_RELOAD); return true; }
                self.fire(d, rng, frame, true);
                true
            }
            (AIM_STAND_FIRE, msg::TIMER_C) => { self.fire(d, rng, frame, true); true }
            (AIM_STAND_RELOAD, msg::ENTER) => {
                self.firing = false;
                self.clip = self.clip_max;
                self.call_anim(anim_state::RELOAD, AIM_STAND_FIRE, 0);
                true
            }
            (AIM_STAND_RELOAD, msg::UPDATE) => { self.turn_to_opponent(0.9); true }
            (COMBAT_NO_SIGHT, msg::ENTER) => { self.firing = false; self.call_anim(anim_state::AIM_STAND, 0, 0); true }
            (COMBAT_NO_SIGHT, msg::UPDATE) => { if self.unseen_ticks < 16 { self.set_state(COMBAT_NEW_SIGHTING) } true }
            (NO_OPPONENT, msg::ENTER) => { self.set_state(IDLE); true }
            (_, msg::ENTER | msg::EXIT | msg::UPDATE) => true,
            _ => false,
        }
    }

    /// `DroneVision_EnemyLookForOpponent` (Attack when seen and the drone may react to the player).
    fn look_for_opponent(&self, d: &Data) -> Option<u8> {
        (self.seen && self.prop(d, 0x3c) && (self.alert >= 0.66 || self.prop(d, 0x51))).then_some(state::ATTACK)
    }

    /// `DroneMove_TurnToObj(target, threshold)`: re-aim the desired yaw when off by more than the threshold.
    fn turn_to_opponent(&mut self, threshold: f32) {
        if angle_difference(self.yaw_to_opp, self.desired_yaw).abs() > threshold { self.desired_yaw = self.yaw_to_opp; }
    }

    // ---- firing -------------------------------------------------------------------------------------------------

    fn weapon_field_u16(&self, d: &Data, off: usize) -> u16 {
        d.weapons.records.get(self.weapon as usize).map(|r| u16::from_le_bytes([r[off], r[off + 1]])).unwrap_or(0)
    }

    /// `DroneWeap_Fire(c)`.
    fn fire(&mut self, d: &Data, rng: &mut Rng, frame: u32, c: bool) {
        if c && !self.firing { self.next_bullet_time(d, rng, frame, true); }
        self.firing = true;
        self.burst_done = false;
    }

    /// `DroneWeap_BurstDelay` (frames).
    fn burst_delay(&self, rng: &mut Rng) -> u32 {
        let k = match self.aggression { 0 => 1.66, 1 => 1.33, 3 => 0.66, 4 => 0.33, _ => 1.0 };
        let mut dly = ((45.0 * k * (0.5 + rng.frand(0.5))) as u32 as f32).max(15.0);
        if self.dist < 4.0 { dly = 15.0 } else { dly = dly.min(60.0); if self.dist > 30.0 { dly = 60.0 } }
        dly as u32
    }

    /// `DroneWeap_NextBulletTime(new burst / start)`.
    fn next_bullet_time(&mut self, d: &Data, rng: &mut Rng, frame: u32, new_burst: bool) {
        let interval = self.weapon_field_u16(d, 0x40) as i16 as i32;
        let i = interval.max(0) as u32;
        if new_burst {
            self.burst = (self.weapon_field_u16(d, 0x2c) as i32).max(1);
            self.last_shot += self.burst_delay(rng);
        }
        self.next_shot = (self.last_shot + i).max(frame);
    }

    /// `DroneWeap_Ready2Fire`: aiming class, close enough, or the gun within 20° of the target.
    fn ready_to_fire(&self) -> bool {
        if self.anim.class != 2 { return false; }
        if self.dist < 2.0 { return true; }
        angle_difference(self.yaw + self.aim_yaw, self.yaw_to_opp).abs() < 0.349
    }

    /// `DroneWeap_HandleFiring` → `DoFiring` → `FireWeapon`.
    fn handle_firing(&mut self, d: &Data, rng: &mut Rng, frame: u32) {
        if !self.firing { return; }
        self.burst_done = false;
        let mut more = false;
        if self.burst == 0 { self.next_bullet_time(d, rng, frame, true); }
        if self.unseen_ticks < 0x10 || self.prop(d, 0x17) {
            more = true;
            if self.next_shot <= frame && self.ready_to_fire() {
                self.shots.push(frame);
                self.last_shot = frame;
                self.clip -= 1;
                self.burst -= 1;
                if self.burst < 1 { more = false } else { self.next_bullet_time(d, rng, frame, false) }
            }
        }
        if !more { self.burst_done = true; }
    }

    // ---- per tick -----------------------------------------------------------------------------------------------

    /// `Drone_GetOpponentInfo`: distance and yaw to the player.
    fn opponent_info(&mut self, p: &Player) {
        let dx = p.pos[0] - self.pos[0];
        let dz = p.pos[2] - self.pos[2];
        let dy = p.pos[1] - self.pos[1];
        self.dist = (dx * dx + dy * dy + dz * dz).sqrt();
        self.yaw_to_opp = atan2_approx(dx, dz);
    }

    /// `DroneVision_OpponentVisibility` (+0x230): range scaled by alertness, cone with a 22.5° pad.
    fn visibility(&self) -> f32 {
        if self.dist < 2.0 && self.sight_range > 0.0 { return 1.0; }
        let r = self.sight_range * (2.0 * self.alert + 1.0);
        if r <= 0.0 { return 0.0; }
        let mut vis = 0.7 - (self.dist - r) / r;
        if self.half_fov != 0.0 {
            let margin = self.half_fov + 0.392_699_1 - angle_difference(self.yaw, self.yaw_to_opp).abs();
            if margin < 0.0 { return 0.0; }
            vis *= crate::trig::sinf(margin / self.half_fov * crate::trig::HALF_PI);
        }
        vis.max(0.0)
    }

    /// `DroneFunc_ReactionTime` (ticks).
    fn reaction_time(&self, level: u32) -> u32 {
        if matches!(level, 0x0700_0002 | 0x0700_0003 | 0x0700_0005 | 0x0700_0009 | 0x0700_000a) {
            if self.ever_seen || self.alert >= 1.0 { 0 } else { ((1.0 - self.alert) * 30.0 * 0.01 * (200.0 - self.reaction as f32)) as u32 }
        } else { 10 }
    }

    fn head(&self) -> V3 { [self.pos[0], self.pos[1] + 0.6, self.pos[2]] }
}

/// All drones of a level plus the round-robin line-of-sight cursor.
pub struct Drones {
    pub list: Vec<Drone>,
    pub frame: u32,
    los_cursor: usize,
    pub rng: Rng,
}

impl Drones {
    pub fn new(list: Vec<Drone>, rng: Rng) -> Drones { Drones { list, frame: 0, los_cursor: 0, rng } }

    /// One 60 Hz game tick: global vision (`DroneVision_ProcessDroneSight`), then each drone's control.
    pub fn tick(&mut self, d: &Data, player: &Player) {
        self.frame += 1;
        let frame = self.frame;
        let n = self.list.len();
        for dr in &mut self.list { dr.opponent_info(player); dr.seen = false; }
        // Visibility every tick for drones with vision on; line of sight for one drone per tick (round robin).
        for dr in &mut self.list { dr.visibility = if dr.vision_on || dr.alert >= 1.0 { dr.visibility() } else { 0.0 }; if dr.visibility < 0.7 { dr.los = false; } }
        for k in 0..n {
            let i = (self.los_cursor + 1 + k) % n.max(1);
            let dr = &mut self.list[i];
            if dr.visibility >= 0.7 {
                dr.los = d.world.line_of_sight(dr.head(), player.target);
                self.los_cursor = i;
                break;
            }
        }
        for dr in &mut self.list {
            if dr.los {
                dr.seen_ticks += 1;
                dr.unseen_ticks = 0;
                if dr.seen_ticks >= dr.reaction_time(d.level) { dr.seen = true; dr.ever_seen = true; }
            } else {
                dr.seen_ticks = 0;
                dr.unseen_ticks += 1;
            }
        }
        // Per drone: timers, alertness decay, update message, turning, aim, firing, animation.
        let rng = &mut self.rng;
        for dr in &mut self.list {
            if frame == 1 { dr.send(d, rng, frame, msg::ENTER); dr.entered = frame; dr.log.push((frame, dr.state)); }
            if let Some(t) = dr.timer_c { if frame >= t { dr.timer_c = None; dr.send(d, rng, frame, msg::TIMER_C); } }
            if dr.alert < 1.0 && dr.alert > dr.alert_floor { dr.alert -= 1.0 / (60.0 * 50.0); }
            if dr.alert < dr.alert_floor { dr.alert = dr.alert_floor; }
            dr.send(d, rng, frame, msg::UPDATE);
            // NDrone2_Steer: exponential turn toward the desired yaw, 0.1 per tick.
            dr.yaw += angle_difference(dr.yaw, dr.desired_yaw) * 0.1;
            // Upper-body aim (`AnimObjectAimAt`, speed 0.2) toward the opponent while aiming; lower clamp −0.9 only.
            let want = if dr.anim.state == anim_state::AIM_STAND { angle_difference(dr.yaw, dr.yaw_to_opp).max(-0.9) } else { 0.0 };
            dr.aim_yaw = rotate_to_bearing(dr.aim_yaw, want, 0.2, crate::trig::PI);
            dr.handle_firing(d, rng, frame);
            if let Some((next_ai, done_msg)) = dr.update_anim(d) {
                if done_msg != 0 { dr.send(d, rng, frame, done_msg); }
                if next_ai != 0 { dr.set_state(next_ai); dr.send(d, rng, frame, 0); }
            }
        }
    }
}
