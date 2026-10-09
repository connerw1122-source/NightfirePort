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
    pub const AIM_BACKOFF: u8 = 0x6d;
    pub const AIM_CROUCH: u8 = 0x6f;
    pub const AIM_CROUCH_FIRE: u8 = 0x70;
    pub const AIM_CROUCH_RELOAD: u8 = 0x71;
    pub const STEP_AIM_LEFT: u8 = 0x73;
    pub const STEP_AIM_RIGHT: u8 = 0x74;
    pub const STRAFE_AIM_LEFT: u8 = 0x75;
    pub const STRAFE_AIM_RIGHT: u8 = 0x76;
    pub const STRAFE_DODGE_LEFT: u8 = 0x77;
    pub const STRAFE_DODGE_RIGHT: u8 = 0x78;
    pub const ROLL_LEFT_CROUCH: u8 = 0x79;
    pub const ROLL_RIGHT_CROUCH: u8 = 0x7a;
}

/// Messages (`*(int*)MsgObject`).
pub mod msg {
    pub const ENTER: u8 = 1;
    pub const EXIT: u8 = 2;
    pub const UPDATE: u8 = 3;
    /// Idle timeout (`SetIdleTimeOut`) and the wait-node message.
    pub const IDLE_TIMEOUT: u8 = 4;
    pub const ANIM_DONE: u8 = 5;
    pub const TIMER_C: u8 = 0xc;
    pub const FORCED_ATTACK: u8 = 0x1f;
}

/// Anim states (`DroneAnimStates` index).
pub mod anim_state {
    pub const AIM_STAND: u16 = 0x0a;
    pub const STAND_IDLE: u16 = 0x24;
    pub const RELOAD: u16 = 0x52;
    pub const BACKOFF: u16 = 0x04;
    pub const CROUCH_AIM: u16 = 0x05;
    pub const STRAFE_LEFT: u16 = 0x0c;
    pub const STRAFE_RIGHT: u16 = 0x0d;
    pub const DODGE_LEFT: u16 = 0x29;
    pub const ROLL_LEFT: u16 = 0x53;
    pub const ROLL_RIGHT: u16 = 0x54;
    pub const STEP_LEFT: u16 = 0x5a;
    pub const STEP_RIGHT: u16 = 0x5b;
    pub const ALERT_IDLE: u16 = 0x21;
    pub const ALERT_LOOK: u16 = 0x22;
    pub const ALERT_WALK: u16 = 0x35;
    pub const ALERT_PATROL_LOOK: u16 = 0x36;
    pub const WALK: u16 = 0x37;
    pub const LOOK_AROUND: u16 = 0x4b;
}

/// Read-only game data the AI consults.
pub struct Data<'a> {
    pub tables: &'a DroneAnimData,
    pub props: &'a PropertyTables,
    pub scripts: &'a HashMap<u32, Script>,
    pub weapons: &'a nf_formats::weapon::WeaponData,
    pub world: &'a CollisionWorld,
    pub level: u32,
    /// The level's animation sequences by hashcode (root motion).
    pub sequences: &'a HashMap<u32, nf_formats::anim::Sequence>,
    /// Skins and skeletons (muzzle position from the posed skeleton).
    pub skins: &'a HashMap<u32, nf_formats::character::Skin>,
    pub skeletons: &'a HashMap<u8, nf_formats::character::Skeleton>,
    /// AI paths and boundaries (block 0x05).
    pub nav: &'a crate::nav::Network,
}

/// Bond as the drones see him.
#[derive(Clone, Copy, Debug, Default)]
pub struct Player {
    /// Object position (feet level + collision offset; for the viewer: the camera's ground point).
    pub pos: V3,
    /// The point drones aim at and test sight against (`Drone_View_Bones`: chest / head; one point here).
    pub target: V3,
    /// Bond's crosshair ray (eye, unit direction) — `Check_Target` (0x1a7810) marks drones it hits as "aimed at".
    pub eye: V3,
    pub aim: V3,
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
    /// `Rand_Rand(n)`: an integer in [0, n) (0 for n = 0).
    pub fn rand(&mut self, n: u32) -> u32 { (self.frand(1.0) * n as f32) as u32 % n.max(1) }
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
    /// Root translation at the previous tick (root motion).
    pub root_prev: Option<V3>,
    /// `+0x2A` Bond's crosshair on this drone (until the given frame) and `+0x17` another drone within 3 units.
    pub aimed_at: bool,
    pub aimed_until: u32,
    pub bunched: bool,
    /// `obj+0xEC` (Rand 10000 at creation; miss wobble phase), `+0x41` sight lost since the last shot, `+0x100` damage multiplier.
    pub wobble: u32,
    pub lost_sight: bool,
    pub damage_mul: f32,
    /// `+0x4FC` alert bits others can see (0x10 attacking, 1 shouted, …; observers get the source's bits << 16),
    /// `+0x524` frame the attack began, `+0x4F8 & 0x20000` already alerted by another drone, `+0x4F8 & 0x2000` may notice.
    pub alert_bits: u32,
    pub attack_frame: u32,
    pub alerted_by_drone: bool,
    pub watching: bool,
    /// Patrol route (`Drone+0x970`), the last wait node (`+0xBAE`), the idle timeout and a delayed message.
    pub route: Option<crate::nav::Route>,
    pub last_wait: Option<u16>,
    pub idle_timeout: Option<u32>,
    pub delayed: Option<(u32, u8)>,
    /// State-machine id (`Drone+0x108`, a creation counter) and the stand-idle hold (`+0x59C`).
    pub sm_id: u32,
    pub stand_until: u32,
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
            shots: Vec::new(), log: Vec::new(), root_prev: None, aimed_at: false, aimed_until: 0, bunched: false, wobble: rng.rand(10000), lost_sight: true, damage_mul: 1.0, alert_bits: 0, attack_frame: 0, alerted_by_drone: false, watching: false,
            route: None, last_wait: None, idle_timeout: None, delayed: None, sm_id: 0, stand_until: 0,
        };
        dr.set_danim(d, start_danim);
        dr
    }

    fn prop(&self, d: &Data, id: usize) -> bool { d.props.get(id, &self.props) != 0 }

    /// `NDrone2_FeetPos`: the feet sphere's centre (object y − 1 + r 0.4 with the standing height 1.0).
    pub fn feet(&self) -> V3 { [self.pos[0], self.pos[1] - 0.6, self.pos[2]] }

    // ---- patrols and idling ------------------------------------------------------------------------------------

    /// `NDrone2_AssignAIPath(mask)`: the nearest node (2D, < 0.3) of a path with `kind & mask` → `InitAIPath`.
    /// *Deviation*: "in the drone's cel" is replaced by "within 2 units of the feet in height".
    fn assign_path(&mut self, d: &Data, mask: u32) -> bool {
        let feet = self.feet();
        let mut best: Option<(f32, usize, u16)> = None;
        for (pi, p) in d.nav.paths.iter().enumerate().filter(|(_, p)| p.kind & mask != 0) {
            for (ni, n) in p.nodes.iter().enumerate() {
                if (n.pos[1] - feet[1]).abs() > 2.0 { continue; }
                let dist = crate::nav::dist2d(self.pos, n.pos);
                if dist < best.map_or(0.3, |b| b.0) { best = Some((dist, pi, ni as u16)) }
            }
        }
        let Some((_, pi, ni)) = best else { return false };
        self.route = Some(crate::nav::Route::for_path(d.nav, pi, ni));
        self.last_wait = None;
        true
    }

    /// `SetIdleTimeOut(base, n)`: msg 4 after (base + Rand n) seconds.
    fn set_idle_timeout(&mut self, rng: &mut Rng, frame: u32, base: u32, n: u32) {
        self.idle_timeout = Some(frame + (base + rng.rand(n)) * 60);
    }

    /// Requests an anim state unless it is playing or pending, or another call is pending (the "respect" form).
    /// Calling the state that is already playing zeroes any pending call (`ZeroAnimCall`): [L] a patrolling guard's
    /// walk request cancels the look-around its wait node asked for in the same tick (work/patrol_trace.tsv: the
    /// guard walks past node 23 without stopping).
    fn request_anim(&mut self, d: &Data, anim: u16) {
        if self.anim.state == anim { self.anim.call = None; return; }
        if self.anim.call.is_some() || !self.can_do(d, anim) { return; }
        self.call_anim(anim, 0, 0);
    }

    /// `SetStandIdleAnim(force)`: in Idle one of 0x24..0x26 (Rand 3) held 5 + Rand 5 seconds; elsewhere 0x24.
    fn set_stand_idle(&mut self, d: &Data, rng: &mut Rng, frame: u32, force: bool) {
        if self.state != state::IDLE { self.request_anim(d, anim_state::STAND_IDLE); return; }
        if !force && frame < self.stand_until { return; }
        let mut anim = anim_state::STAND_IDLE + rng.rand(3) as u16;
        if !self.can_do(d, anim) { anim = anim_state::STAND_IDLE }
        self.stand_until = frame + (5 + rng.rand(5)) * 60;
        if self.anim.call.is_none() && self.anim.state != anim { self.call_anim(anim, 0, 0) }
    }

    /// `SetWalkIdleAnim`: in Patrol (SM id + 1) % 3 + 0x37, or 0x37 when the set lacks it.
    fn walk_anim(&self, d: &Data) -> u16 {
        let a = ((self.sm_id + 1) % 3) as u16 + anim_state::WALK;
        if self.can_do(d, a) { a } else { anim_state::WALK }
    }

    /// `UpdatePatrolRoute` → `NDrone2_FollowRoute` → `AINetwork_FollowRoute`: move target and desired yaw, the walk
    /// anim, wait nodes (`ReachedDestNode` flag 1: msg 4 a tick later every other lap, alertness < 0.33).
    fn update_patrol_route(&mut self, d: &Data, _rng: &mut Rng, frame: u32) {
        let feet = self.feet();
        let Some(route) = self.route.as_mut() else { self.request_anim(d, anim_state::STAND_IDLE); return };
        let (code, reached) = route.follow(d.nav, feet);
        let target = route.target;
        if let Some(node) = reached {
            let flags = d.nav.paths[route.path].nodes[node as usize].flags;
            if flags & 1 != 0 {
                if self.alert < 0.33 && self.last_wait != Some(node) {
                    self.last_wait = Some(node);
                    self.delayed = Some((frame + 1, msg::IDLE_TIMEOUT));
                } else {
                    self.last_wait = None;
                }
            }
        }
        let alerted = self.prop(d, 0x25) && self.alert >= 0.66;
        match code {
            crate::nav::follow::MOVING => { let a = if alerted { anim_state::ALERT_WALK } else { self.walk_anim(d) }; self.request_anim(d, a) }
            crate::nav::follow::NO_PATH => self.request_anim(d, anim_state::STAND_IDLE),
            _ => { let a = if self.alert >= 0.66 { anim_state::ALERT_IDLE } else { self.walk_anim(d) }; self.request_anim(d, a) }
        }
        self.desired_yaw = atan2_approx(target[0] - self.pos[0], target[2] - self.pos[2]);
    }

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
        let before = (self.anim.script, self.anim.danim);
        self.anim.time += 1;
        let len = self.anim.script.and_then(|h| d.scripts.get(&h)).map(|s| s.length as u32).unwrap_or(1);
        let mut restarted = false;
        // A one-shot danim of the requested state that just finished completes the call.
        let mut finished_wanted = false;
        let mut wrap_end = None;
        if self.anim.time >= len {
            // The wrap tick repeats the script's last step (last frame − the one before): [L] aligned per-tick steps of
            // a live walk cycle (115 ticks, work/patrol_trace.tsv) show the step before the wrap twice.
            self.anim.time = len.saturating_sub(1);
            let last = self.root_value(d);
            self.anim.time = len.saturating_sub(2);
            wrap_end = self.root_value(d).zip(last).map(|(a, b)| [b[0] - a[0], 0.0, b[2] - a[2]]);
            finished_wanted = self.anim.call.is_some_and(|(w, _, _)| w == self.anim.state && self.anim.started_call);
            let info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
            let next = u16::from_le_bytes([info[10], info[11]]);
            if next != 0 && next != self.anim.danim { self.set_danim(d, next) } else { self.anim.time = 0 }
            restarted = true;
        }
        let same = before == (self.anim.script, self.anim.danim);
        self.root_motion(d, restarted || !same, if same { wrap_end } else { None });
        let (want, next_ai, done_msg) = self.anim.call?;
        // CallFullyComplete: the requested state is playing as a loop (class 2: aim, crouch-aim, idle …), or a one-shot
        // danim of it (a step, a roll, a reload) has played to its end. *Suggested* from the trace (steps last
        // their animation); `DroneAnim_CallHandler`'s exact test is not read yet.
        if finished_wanted || (self.anim.state == want && self.anim.class == 2) {
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

    /// The sequence and frame the playing script shows (opcode-0 command covering the script time).
    pub fn script_frame(&self, d: &Data) -> Option<(u32, u32)> {
        let script = d.scripts.get(&self.anim.script?)?;
        let t = (1 + self.anim.time) as u16;
        let (a, _, q) = script.sequences().find(|(a, b, _)| *a <= t && t <= *b).or_else(|| script.sequences().next())?;
        Some((q, (t.saturating_sub(a) + 1) as u32))
    }

    /// The root translation at the playing script frame.
    fn root_value(&self, d: &Data) -> Option<V3> {
        self.script_frame(d).and_then(|(q, f)| d.sequences.get(&q).filter(|s| s.channels.len() >= 3).map(|s| {
            let f = f.clamp(1, s.frames.max(1) as u32);
            [s.value(0, f), s.value(1, f), s.value(2, f)]
        }))
    }

    /// Root motion (`AnimSeqTick` + `AnimFrameResolve`): the root translation's change since the last tick, rotated
    /// by the object's yaw (`RotMatrix`: world = l.x·(cos y, 0, −sin y) + l.z·(sin y, 0, cos y)) and added to the
    /// position. No motion on the tick a new script starts. Vertical motion is replaced by the drone's height rule
    /// (`DroneAnim_PreTransCallback`), so only x/z move here.
    fn root_motion(&mut self, d: &Data, restarted: bool, wrap_end: Option<V3>) {
        let root = self.root_value(d);
        // A script that wrapped onto itself moves by its first step (`wrap_end`): [L] without it a patrolling guard
        // fell 2 % behind the live one (work/patrol_trace.tsv).
        let delta = match (root, self.root_prev, restarted, wrap_end) {
            (Some(r), Some(prev), false, _) => Some([r[0] - prev[0], r[2] - prev[2]]),
            (Some(_), Some(_), true, Some(step)) => Some([step[0], step[2]]),
            _ => None,
        };
        if let (Some([lx, lz]), Some(_)) = (delta, root) {
            let (s, c) = (crate::trig::sinf(self.yaw), crate::trig::cosf(self.yaw));
            let step = [lx * c + lz * s, -lx * s + lz * c];
            let to = [self.pos[0] + step[0], self.pos[1], self.pos[2] + step[1]];
            // Walls: the move is cancelled when the body (0.4 above the feet sphere … head) would cross collision.
            let probe = |y: f32| d.world.line_of_sight([self.pos[0], y, self.pos[2]], [to[0] + step[0].signum() * 0.4 * (step[0].abs() > 1e-6) as i32 as f32, y, to[2] + step[1].signum() * 0.4 * (step[1].abs() > 1e-6) as i32 as f32]);
            if probe(self.pos[1]) && probe(self.pos[1] + 0.5) {
                self.pos = to;
                if let Some(f) = d.world.floor_below([to[0], self.pos[1], to[2]], 2.0) { self.pos[1] = f + 1.0; }
            }
        }
        self.root_prev = root;
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
            // ... and the timers (C/D; the idle timeout too: *suggested*, it is re-armed by every enter that uses it).
            self.timer_c = None;
            self.idle_timeout = None;
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
            (INIT_PATROL, msg::ENTER) => {
                let patrols = (self.prop(d, 0x24) || self.prop(d, 0x25)) && self.assign_path(d, 4);
                self.set_state(if patrols { PATROL } else { IDLE });
                true
            }
            (IDLE | ALERT | PATROL, msg::ENTER) => {
                self.vision_on = true;
                self.watching = true;
                match s {
                    IDLE if self.alert >= 0.66 => self.set_state(ALERT),
                    IDLE => {
                        self.set_stand_idle(d, rng, frame, true);
                        if self.prop(d, 0x46) { self.set_idle_timeout(rng, frame, 45, 5) }
                    }
                    ALERT => {
                        self.request_anim(d, anim_state::ALERT_IDLE);
                        if self.prop(d, 0x46) { self.set_idle_timeout(rng, frame, 45, 5) }
                    }
                    _ => if self.prop(d, 0x46) {
                        let (base, n) = if self.prop(d, 0x25) && self.alert >= 0.66 { (5, 10) } else { (45, 10) };
                        self.set_idle_timeout(rng, frame, base, n);
                    },
                }
                true
            }
            (IDLE | ALERT, msg::UPDATE) => {
                if let Some(n) = self.look_for_opponent(d) { self.set_state(n) }
                else if s == IDLE { self.set_stand_idle(d, rng, frame, false) } else { self.request_anim(d, anim_state::ALERT_IDLE) }
                true
            }
            (PATROL, msg::UPDATE) => {
                self.update_patrol_route(d, rng, frame);
                if let Some(n) = self.look_for_opponent(d) { self.set_state(n) }
                true
            }
            // Idle timeouts (prop 0x46) and wait nodes: look around (`0x4B`, variant Rand 3 — variants not modelled),
            // then back to the state, which re-arms the timeout.
            (IDLE | ALERT | PATROL, msg::IDLE_TIMEOUT) => {
                if self.prop(d, 0x46) {
                    let alerted = self.alert >= 0.66;
                    let (anim, back) = match s {
                        PATROL if self.prop(d, 0x25) && alerted => (anim_state::ALERT_PATROL_LOOK, PATROL),
                        PATROL => (anim_state::LOOK_AROUND, PATROL),
                        IDLE if alerted => (anim_state::ALERT_LOOK, IDLE),
                        IDLE => (anim_state::LOOK_AROUND, IDLE),
                        _ => (anim_state::ALERT_LOOK, ALERT),
                    };
                    let _variant = rng.rand(3);
                    if self.can_do(d, anim) { self.call_anim(anim, back, 0) }
                    if s != PATROL { self.set_idle_timeout(rng, frame, 45, 5) }
                }
                true
            }
            // Forced attack (`Drone_Message(0x1F)` → `DroneFunc_DoForcedAttack`): another drone was seen attacking.
            (IDLE | ALERT | PATROL, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            (ATTACK, msg::ENTER) => {
                // DroneFunc_FirstAttack → FirstSightState: alertness 1, combat sight; props 0x1B (50 %) / 0x19 pick a
                // combat move; otherwise AimStand (prop 0x18, in range) or Combat.
                self.alert = 1.0;
                // FirstAttack: the attack is visible to others (`+0x4FC |= 0x10`, enemy side), shouts with prop 0x33.
                if self.alert_bits & 0x10 == 0 { self.attack_frame = frame; }
                self.alert_bits |= 0x10 | if self.prop(d, 0x33) { 1 } else { 0 };
                self.sight_range = self.sight_range.max(self.combat_range);
                self.half_fov = 0.0;
                let first = if self.prop(d, 0x1b) { if rng.rand(2) == 0 { 0 } else { self.choose_combat_move(d, rng) } }
                    else if self.prop(d, 0x18) { 0 } else if self.prop(d, 0x19) { self.choose_combat_move(d, rng) } else { 0 };
                self.set_state(if first != 0 { first } else if self.prop(d, 0x18) && self.dist < self.engage_range { AIM_STAND } else { COMBAT });
                true
            }
            (COMBAT, msg::ENTER) => { self.alert = 1.0; true }
            (COMBAT | COMBAT_NO_MOVE | COMBAT_OUT_OF_RANGE, msg::UPDATE) => {
                if self.unseen_ticks < 16 { self.set_state(COMBAT_NEW_SIGHTING) } else { self.set_state(COMBAT_NO_SIGHT) }
                true
            }
            (COMBAT_NEW_SIGHTING, msg::ENTER) => {
                let crouch = self.prop(d, 0x11) && self.can_do(d, anim_state::CROUCH_AIM) && self.dist >= 6.0 && rng.rand(2) != 0;
                self.set_state(if crouch { AIM_CROUCH_FIRE } else { AIM_STAND_FIRE });
                true
            }
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
                if self.dist <= 4.0 && self.can_backoff(d) { self.set_state(AIM_BACKOFF); return true; }
                if self.unseen_ticks >= 16 { self.set_state(COMBAT_NO_SIGHT); return true; }
                if self.firing && !self.burst_done { return true; }
                match self.combat_state(d, rng, frame, AIM_STAND_FIRE) { 0 => self.fire(d, rng, frame, true), n => self.set_state(n) }
                true
            }
            (AIM_STAND_FIRE | AIM_CROUCH_FIRE, msg::TIMER_C) => { self.fire(d, rng, frame, true); true }
            (AIM_STAND_RELOAD | AIM_CROUCH_RELOAD, msg::ENTER) => {
                self.firing = false;
                self.clip = self.clip_max;
                self.call_anim(anim_state::RELOAD, if s == AIM_STAND_RELOAD { AIM_STAND_FIRE } else { AIM_CROUCH_FIRE }, 0);
                true
            }
            (AIM_STAND_RELOAD | AIM_CROUCH_RELOAD, msg::UPDATE) => { self.turn_to_opponent(0.9); true }
            // Crouching: AimCrouch plays the crouch-aim anim and continues in AimCrouchFire.
            (AIM_CROUCH, msg::ENTER) => { self.call_anim(anim_state::CROUCH_AIM, AIM_CROUCH_FIRE, 0); true }
            (AIM_CROUCH, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            (AIM_CROUCH_FIRE, msg::ENTER) => { self.call_anim(anim_state::CROUCH_AIM, 0, 0); self.vision_on = true; self.fire(d, rng, frame, true); true }
            (AIM_CROUCH_FIRE, msg::UPDATE) => {
                self.desired_yaw = self.yaw_to_opp;
                if self.dist <= 4.0 && self.can_backoff(d) { self.set_state(AIM_BACKOFF); return true; }
                if self.firing && !self.burst_done { return true; }
                if frame < self.entered + 60 { return true; }
                match self.combat_state(d, rng, frame, AIM_CROUCH_FIRE) {
                    0 => if self.clip > 0 { self.fire(d, rng, frame, true) } else { self.set_state(AIM_CROUCH_RELOAD) },
                    n => self.set_state(n),
                }
                true
            }
            // Steps: one anim, then back to AimStandFire; no firing while stepping.
            (STEP_AIM_LEFT | STEP_AIM_RIGHT, msg::ENTER) => {
                self.call_anim(if s == STEP_AIM_LEFT { anim_state::STEP_LEFT } else { anim_state::STEP_RIGHT }, AIM_STAND_FIRE, 0);
                self.firing = false;
                true
            }
            (STEP_AIM_LEFT | STEP_AIM_RIGHT, msg::UPDATE) => {
                let a = if s == STEP_AIM_LEFT { anim_state::STEP_LEFT } else { anim_state::STEP_RIGHT };
                if !self.can_do(d, a) { self.set_state(AIM_STAND_FIRE) } else { self.desired_yaw = self.yaw_to_opp }
                true
            }
            // Strafes fire while moving; dodges do not. Both end on a 2–5 s / 2–3 s timer or when blocked.
            (STRAFE_AIM_LEFT | STRAFE_AIM_RIGHT | STRAFE_DODGE_LEFT | STRAFE_DODGE_RIGHT, msg::ENTER) => {
                let (anim, secs, fires) = match s {
                    STRAFE_AIM_LEFT => (anim_state::STRAFE_LEFT, rng.rand(4) + 2, true),
                    STRAFE_AIM_RIGHT => (anim_state::STRAFE_RIGHT, rng.rand(4) + 2, true),
                    STRAFE_DODGE_LEFT => (anim_state::DODGE_LEFT, rng.rand(2) + 2, false),
                    _ => (anim_state::STRAFE_RIGHT, rng.rand(2) + 2, false), // StrafeDodgeRight calls anim 0xD (shipped)
                };
                self.call_anim(anim, 0, 0);
                self.timer_c = Some(frame + secs * 60);
                if fires { self.fire(d, rng, frame, true) } else { self.firing = false }
                true
            }
            (STRAFE_AIM_LEFT | STRAFE_AIM_RIGHT | STRAFE_DODGE_LEFT | STRAFE_DODGE_RIGHT, msg::UPDATE) => {
                self.desired_yaw = self.yaw_to_opp;
                let left = matches!(s, STRAFE_AIM_LEFT | STRAFE_DODGE_LEFT);
                let dodge = matches!(s, STRAFE_DODGE_LEFT | STRAFE_DODGE_RIGHT);
                if self.anim.class == 2 && (!self.can_move(d, if left { 1.0 } else { -1.0 } * if dodge { 3.6 } else { 1.8 }, 0.0) || self.unseen_ticks > 15) {
                    self.set_state(AIM_STAND);
                } else if !dodge && self.burst_done { self.fire(d, rng, frame, true) }
                true
            }
            (STRAFE_AIM_LEFT | STRAFE_AIM_RIGHT | STRAFE_DODGE_LEFT | STRAFE_DODGE_RIGHT | AIM_BACKOFF, msg::TIMER_C) => { self.set_state(AIM_STAND); true }
            (ROLL_LEFT_CROUCH | ROLL_RIGHT_CROUCH, msg::ENTER) => {
                self.call_anim(if s == ROLL_LEFT_CROUCH { anim_state::ROLL_LEFT } else { anim_state::ROLL_RIGHT }, AIM_CROUCH_FIRE, 0);
                true
            }
            (ROLL_LEFT_CROUCH | ROLL_RIGHT_CROUCH, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            (AIM_BACKOFF, msg::ENTER) => {
                self.call_anim(anim_state::BACKOFF, 0, 0);
                self.timer_c = Some(frame + (rng.rand(4) + 2) * 60);
                self.fire(d, rng, frame, true);
                true
            }
            (AIM_BACKOFF, msg::UPDATE) => {
                if !(self.dist <= self.engage_range && self.unseen_ticks < 16 && self.can_backoff(d)) { self.set_state(AIM_STAND); return true; }
                self.desired_yaw = self.yaw_to_opp;
                if self.firing && !self.burst_done { return true; }
                self.fire(d, rng, frame, true);
                let m = self.choose_combat_move(d, rng);
                self.set_state(m);
                true
            }
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

    /// `DroneAnim_CanDoAnimState`: the anim state's own danim has a script in our anim set (or set 0).
    fn can_do(&self, d: &Data, anim: u16) -> bool {
        d.tables.states.get(anim as usize).is_some_and(|(own, _)| d.tables.script_for(*own, self.anim_set).is_some())
    }

    /// The `NDrone2_Can*` tests: the point `local` (x = left, z = forward) from the feet must be reachable
    /// (`NDrone2_MoveTest`: |dy| ≤ 4 and not across a boundary). *Deviation*: the game tests AI-network boundaries;
    /// we require a floor there and no collision between at knee and chest height.
    fn can_move(&self, d: &Data, x: f32, z: f32) -> bool {
        let (s, c) = (crate::trig::sinf(self.yaw), crate::trig::cosf(self.yaw));
        let feet = [self.pos[0], self.pos[1] - 1.0, self.pos[2]];
        let to = [feet[0] + x * c + z * s, feet[1], feet[2] - x * s + z * c];
        let Some(floor) = d.world.floor_below([to[0], feet[1] + 2.0, to[2]], 6.0) else { return false };
        if (floor - feet[1]).abs() > 4.0 { return false; }
        [0.5, 1.4].iter().all(|&h| d.world.line_of_sight([feet[0], feet[1] + h, feet[2]], [to[0], floor + h, to[2]]))
    }

    fn can_backoff(&self, d: &Data) -> bool { self.prop(d, 0x06) && self.can_do(d, anim_state::BACKOFF) && self.can_move(d, 0.0, -1.8) }

    /// `NDrone2_ChooseCombatMove` (0x150c18): an option list (dodge, strafe, roll, step, crouch, stand) picked at
    /// random, each case falling through to the next when it fails; left/right order random.
    fn choose_combat_move(&mut self, d: &Data, rng: &mut Rng) -> u8 {
        use state::*;
        if self.prop(d, 0x40) { return 0; }
        let (aimed, bunched) = (self.aimed_at, self.bunched);
        if rng.rand(15) >= 7 && !aimed && !bunched { return 0; }
        let mut opts = Vec::new();
        if aimed && self.prop(d, 0x13) && self.can_do(d, anim_state::DODGE_LEFT) { opts.push(1) }
        if (aimed || bunched) && self.prop(d, 0x41) && self.can_do(d, anim_state::STRAFE_LEFT) { opts.push(2) }
        if aimed && self.prop(d, 0x2e) && self.can_do(d, anim_state::ROLL_LEFT) { opts.push(3) }
        if self.prop(d, 0x42) && self.can_do(d, anim_state::STEP_LEFT) { opts.push(4) }
        if !aimed {
            if self.prop(d, 0x11) && self.can_do(d, anim_state::CROUCH_AIM) && self.dist >= 6.0 && rng.rand(2) == 0 { opts.push(5) }
            opts.push(6);
        }
        if opts.is_empty() { return 0; }
        let mut case = opts[rng.rand(opts.len() as u32) as usize];
        let pair = |me: &Self, rng: &mut Rng, dist: f32, l: u8, r: u8| -> u8 {
            let order = if rng.rand(2) != 0 { [(l, dist), (r, -dist)] } else { [(r, -dist), (l, dist)] };
            order.iter().find(|(_, x)| me.can_move(d, *x, 0.0)).map(|(s, _)| *s).unwrap_or(0)
        };
        let result = loop {
            let r = match case {
                1 if aimed && self.prop(d, 0x13) => pair(self, rng, 3.6, STRAFE_DODGE_LEFT, STRAFE_DODGE_RIGHT),
                2 if !aimed => { case = 4; continue; }
                2 if self.prop(d, 0x41) => pair(self, rng, 1.8, STRAFE_AIM_LEFT, STRAFE_AIM_RIGHT),
                3 if aimed && self.prop(d, 0x2e) && self.can_do(d, anim_state::ROLL_LEFT) => pair(self, rng, 5.2, ROLL_LEFT_CROUCH, ROLL_RIGHT_CROUCH),
                4 if self.prop(d, 0x42) && self.can_do(d, anim_state::STEP_LEFT) => pair(self, rng, 1.8, STEP_AIM_LEFT, STEP_AIM_RIGHT),
                5 if self.prop(d, 0x11) && self.can_do(d, anim_state::CROUCH_AIM) && self.dist >= 6.0 && rng.rand(2) == 0 => AIM_CROUCH,
                6 => break if rng.rand(4) != 0 { AIM_STAND } else { 0 },
                7.. => break 0,
                _ => 0,
            };
            if r != 0 { break r; }
            case += 1;
        };
        if (result == AIM_STAND && self.state == AIM_STAND_FIRE) || (result == AIM_CROUCH && self.state == AIM_CROUCH_FIRE) || result == self.state { 0 } else { result }
    }

    /// `DroneFunc_CombatState` (0x148338) for the firing states (no routes or cover in this port).
    fn combat_state(&mut self, d: &Data, rng: &mut Rng, frame: u32, from: u8) -> u8 {
        use state::*;
        if from == AIM_STAND_FIRE && self.clip <= 0 { return AIM_STAND_RELOAD; }
        if self.unseen_ticks >= 16 { return COMBAT_NO_SIGHT; }
        if from == AIM_STAND_FIRE {
            if self.anim.class == 2 && self.anim.state == anim_state::AIM_STAND {
                let m = self.choose_combat_move(d, rng);
                if m != 0 { return m; }
            } else { self.call_anim(anim_state::AIM_STAND, 0, 0); }
        } else {
            if frame > self.entered + 120 {
                let m = self.choose_combat_move(d, rng);
                if m != 0 { return m; }
            }
            self.call_anim(anim_state::CROUCH_AIM, 0, 0);
        }
        0
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
    fn handle_firing(&mut self, d: &Data, rng: &mut Rng, frame: u32, p: &Player, motion: (bool, bool), owner: usize) -> Option<crate::bullet::Bullet> {
        if !self.firing { return None; }
        self.burst_done = false;
        let mut more = false;
        let mut bullet = None;
        if self.burst == 0 { self.next_bullet_time(d, rng, frame, true); }
        if self.unseen_ticks < 0x10 || self.prop(d, 0x17) {
            more = true;
            if self.next_shot <= frame && self.ready_to_fire() {
                bullet = self.fire_weapon(d, rng, frame, p, motion, owner);
                self.shots.push(frame);
                self.last_shot = frame;
                self.clip -= 1;
                self.burst -= 1;
                if self.burst < 1 { more = false } else { self.next_bullet_time(d, rng, frame, false) }
            }
        }
        if !more { self.burst_done = true; }
        bullet
    }

    /// The muzzle (`AnimGetBoneWorldTrans(obj, weapon datum id, 1)` = datum id + 1) in world space, from the posed
    /// skeleton; falls back to chest height in front of the drone.
    pub fn muzzle(&self, d: &Data) -> V3 {
        let posed = (|| {
            let (q, f) = self.script_frame(d)?;
            let seq = d.sequences.get(&q)?;
            let skin = d.skins.get(&self.skin)?;
            let skel = d.skeletons.get(&skin.skeleton)?;
            let (_, datum) = d.weapons.third_person(self.weapon as u8)?;
            let id = if datum == 0xff { 0 } else { datum as u32 } + 1;
            let dt = skin.datums.iter().find(|x| x.id == id && x.bone >= 0)?;
            let mut local = nf_formats::anim::local_pose(seq, skel, skin, f.clamp(1, seq.frames.max(1) as u32));
            local[0].0 = [0.0; 3];
            let world = nf_formats::anim::world_pose(&local, skin);
            let obj = nf_formats::placement::object_matrix([0.0, self.yaw, 0.0], self.pos);
            let m = nf_formats::anim::mul(&nf_formats::anim::mul(&nf_formats::anim::quat_trans_to_mat(dt.q, dt.t), world.get(dt.bone as usize)?), &obj);
            Some([m[3][0], m[3][1], m[3][2]])
        })();
        posed.unwrap_or_else(|| {
            let (s, c) = (crate::trig::sinf(self.yaw), crate::trig::cosf(self.yaw));
            [self.pos[0] + s * 0.5, self.pos[1] + 0.4, self.pos[2] + c * 0.5]
        })
    }

    /// `DroneWeap_FireWeapon` + `DoBulletAccuracy`: hit roll, the miss wobble, the cone (`Bullet_init`), damage
    /// (`Drone_ModBulletDamage`: ×2 under 4 units, × `+0x100`).
    fn fire_weapon(&mut self, d: &Data, rng: &mut Rng, frame: u32, p: &Player, (moving, first_moved): (bool, bool), owner: usize) -> Option<crate::bullet::Bullet> {
        let w = d.weapons.records.get(self.weapon as usize)?;
        let f32_at = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        // Hit chance: p = 100 − 5·accuracy, ×2 under TooClose (3), ×0.25 first moved, ×0.75 moving, × level
        // difficulty accuracy (07000005–08: 0.8 / 0.9 / 1.3; difficulty 1 as read live), ×2 again under TooClose.
        let mut pct = 100.0 - 5.0 * self.accuracy as f32;
        let close = self.dist < 3.0;
        if close { pct *= 2.0 }
        if first_moved { pct *= 0.25 }
        if moving { pct *= 0.75 }
        pct *= level_accuracy(d.level, DIFFICULTY);
        if close { pct *= 2.0 }
        let mut hit = rng.frand(100.0) < pct;
        if !close { hit &= self.seen_ticks >= 120 && !self.lost_sight; }
        if self.dist < 3.0 { hit = true; }
        self.lost_sight = false;
        // Miss: deterministic wobble (0.5 sin a, 1.5 sin(a + π/2), 1.5 sin 0.02t), a = 0.01t, t = frame + obj+0xEC,
        // rotated by the yaw toward the target.
        let off = if hit { [0.0; 3] } else {
            let t = (frame + self.wobble) as f32;
            let a = 0.01 * t;
            let l = [0.5 * crate::trig::sinf(a), 1.5 * crate::trig::sinf(a + crate::trig::HALF_PI), 1.5 * crate::trig::sinf(0.02 * t)];
            let (s, c) = (crate::trig::sinf(self.yaw_to_opp), crate::trig::cosf(self.yaw_to_opp));
            [l[0] * c + l[2] * s, l[1], -l[0] * s + l[2] * c]
        };
        let muzzle = self.muzzle(d);
        let aim = [p.target[0] + off[0] - muzzle[0], p.target[1] + off[1] - muzzle[1], p.target[2] + off[2] - muzzle[2]];
        let flags68 = u32::from_le_bytes(w[0x68..0x6c].try_into().unwrap());
        let dir = crate::bullet::spread(aim, f32_at(0x24), flags68, rng);
        let mut damage = f32_at(0x0c) * self.damage_mul;
        if self.dist < 4.0 { damage *= 2.0 }
        Some(crate::bullet::Bullet { pos: muzzle, dir, speed: f32_at(0x20), travelled: 0.0, range: f32_at(0x1c), damage, owner, state: 0 })
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

/// Difficulty (`GameState+0x28`); 1 in the user's live sessions.
pub const DIFFICULTY: u32 = 1;

/// Per-level accuracy factors by difficulty (`ReadTuningVars` 0x1d3a80).
pub fn level_accuracy(level: u32, difficulty: u32) -> f32 {
    let (e, n, h) = match level & 0xff {
        0x01..=0x04 | 0x14..=0x16 => (0.5, 0.7, 1.0),
        0x05..=0x08 => (0.8, 0.9, 1.3),
        0x09..=0x0b => (0.8, 1.0, 1.0),
        0x0c | 0x0d | 0x1b => (0.6, 0.8, 1.2),
        0x11..=0x13 | 0x4a => (0.7, 0.8, 1.0),
        0x21..=0x29 | 0x4b | 0x4c => (0.8, 1.0, 1.5),
        _ => (0.5, 1.0, 2.0),
    };
    match difficulty { 1 => e, 3 | 4 => h, _ => n }
}

/// All drones of a level plus the round-robin line-of-sight cursor.
pub struct Drones {
    pub list: Vec<Drone>,
    pub frame: u32,
    los_cursor: usize,
    pub rng: Rng,
    pub bullets: Vec<crate::bullet::Bullet>,
    pub health: crate::bullet::PlayerHealth,
    /// Every hit on Bond: (tick, shooting drone, health lost).
    pub player_hits: Vec<(u32, usize, f32)>,
    /// Impacts this tick (for effects).
    pub impacts: Vec<crate::bullet::BulletEvent>,
    /// Bond's previous position and the tick he started moving (`DoOpponentTargetting` +0x1DC / +0x1DD).
    prev_player: Option<V3>,
    moved_since: Option<u32>,
}

impl Drones {
    pub fn new(mut list: Vec<Drone>, rng: Rng) -> Drones {
        // `Drone_SM_InitObject` numbers state machines in creation order (the base is not known: 0 here).
        for (i, dr) in list.iter_mut().enumerate() { dr.sm_id = i as u32; }
        Drones { list, frame: 0, los_cursor: 0, rng, bullets: Vec::new(), health: Default::default(), player_hits: Vec::new(), impacts: Vec::new(), prev_player: None, moved_since: None }
    }

    /// One 60 Hz game tick: global vision (`DroneVision_ProcessDroneSight`), then each drone's control.
    pub fn tick(&mut self, d: &Data, player: &Player) {
        self.frame += 1;
        let frame = self.frame;
        let n = self.list.len();
        for dr in &mut self.list { dr.opponent_info(player); dr.seen = false; }
        // `Check_Target` (0x1a7810): every 6 ticks Bond's 50-unit crosshair ray marks the first drone it hits (body
        // cylinder r 0.4, feet to 1.8 up; *deviation*: the game tests the drone's bounding sphere / boxes) for 30 ticks.
        if frame % 6 == 0 {
            let far = [player.eye[0] + player.aim[0] * 50.0, player.eye[1] + player.aim[1] * 50.0, player.eye[2] + player.aim[2] * 50.0];
            let wall = d.world.segment(player.eye, far).unwrap_or(1.0);
            let hit = self.list.iter().enumerate().filter_map(|(i, dr)| {
                let vol = crate::bullet::PlayerVolume { feet: [dr.pos[0], dr.pos[1] - 1.0, dr.pos[2]], height: 1.8, radius: 0.4 };
                crate::bullet::segment_cylinder(player.eye, far, &vol).filter(|&t| t < wall).map(|t| (i, t))
            }).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i);
            if let Some(i) = hit { self.list[i].aimed_at = true; self.list[i].aimed_until = frame + 30; }
        }
        for dr in &mut self.list { if dr.aimed_at && frame >= dr.aimed_until { dr.aimed_at = false; } }
        // `DroneMove_NoBunching`: another drone within 3 units.
        let positions: Vec<V3> = self.list.iter().map(|x| x.pos).collect();
        for (i, dr) in self.list.iter_mut().enumerate() {
            dr.bunched = positions.iter().enumerate().any(|(j, p)| j != i && ((p[0] - dr.pos[0]).powi(2) + (p[2] - dr.pos[2]).powi(2)) < 9.0);
        }
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
                dr.lost_sight = true;
            }
        }
        // Drones noticing drones that attack (`DroneVision_FindAlertedDrones` → `ConsiderAlerted` →
        // `DroneCanSeeDrone`): source attacking ≥ 1 s with visible alert bits; observer watching (Idle/Alert), not yet
        // alerted, whose props accept the bits (0x10 always; 2 → 0x27, 4 → 0x4F, 1 → 0x28, 8 → 0x26/0x29), within
        // 50 units, in its view (±e4, ≤ e8, under 2 always) and with head-to-head line of sight. At most 4 per tick.
        let mut alerts = Vec::new();
        'pairs: for (oi, o) in self.list.iter().enumerate() {
            if !o.watching || o.alerted_by_drone || !matches!(o.state, state::IDLE | state::ALERT | state::PATROL) { continue; }
            for (si, src) in self.list.iter().enumerate() {
                if si == oi || src.alert_bits == 0 || frame < src.attack_frame + 60 { continue; }
                let b = src.alert_bits;
                let accepts = b & 0x10 != 0 || (b & 2 != 0 && o.prop(d, 0x27)) || (b & 4 != 0 && o.prop(d, 0x4f))
                    || (b & 1 != 0 && o.prop(d, 0x28)) || (b & 8 != 0 && (o.prop(d, 0x26) || o.prop(d, 0x29)));
                if !accepts { continue; }
                let (dx, dy, dz) = (src.pos[0] - o.pos[0], src.pos[1] - o.pos[1], src.pos[2] - o.pos[2]);
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                if dist > 50.0 { continue; }
                let in_view = dist < 2.0 || ((o.half_fov == 0.0 || angle_difference(o.yaw, atan2_approx(dx, dz)).abs() <= o.half_fov) && dist <= o.sight_range);
                if in_view && d.world.line_of_sight(o.head(), src.head()) {
                    alerts.push((oi, b));
                    if alerts.len() > 4 { break 'pairs; }
                    continue 'pairs;
                }
            }
        }
        for (oi, b) in alerts {
            let o = &mut self.list[oi];
            o.alerted_by_drone = true;
            o.alert_bits |= b << 16;
            o.send(d, &mut self.rng, frame, msg::FORCED_ATTACK);
        }
        // Bond's motion (`DroneWeap_DoOpponentTargetting`): moving, and "first moved" for 2 s after he starts.
        let moving = self.prev_player.is_some_and(|q| (q[0] - player.pos[0]).abs() + (q[2] - player.pos[2]).abs() > 1e-4);
        if moving && self.moved_since.is_none() { self.moved_since = Some(frame) }
        if !moving { self.moved_since = None }
        let first_moved = self.moved_since.is_some_and(|f| frame < f + 120);
        self.prev_player = Some(player.pos);
        // Per drone: timers, alertness decay, update message, turning, aim, firing, animation.
        let rng = &mut self.rng;
        for (index, dr) in self.list.iter_mut().enumerate() {
            if frame == 1 { dr.send(d, rng, frame, msg::ENTER); dr.entered = frame; dr.log.push((frame, dr.state)); }
            if let Some(t) = dr.timer_c { if frame >= t { dr.timer_c = None; dr.send(d, rng, frame, msg::TIMER_C); } }
            if let Some(t) = dr.idle_timeout { if frame >= t { dr.idle_timeout = None; dr.send(d, rng, frame, msg::IDLE_TIMEOUT); } }
            if let Some((t, m)) = dr.delayed { if frame >= t { dr.delayed = None; dr.send(d, rng, frame, m); } }
            if dr.alert < 1.0 && dr.alert > dr.alert_floor { dr.alert -= 1.0 / (60.0 * 50.0); }
            if dr.alert < dr.alert_floor { dr.alert = dr.alert_floor; }
            dr.send(d, rng, frame, msg::UPDATE);
            // NDrone2_Steer: exponential turn toward the desired yaw, 0.1 per tick.
            dr.yaw += angle_difference(dr.yaw, dr.desired_yaw) * 0.1;
            // Upper-body aim (`AnimObjectAimAt`, speed 0.2) toward the opponent while aiming; lower clamp −0.9 only.
            let want = if dr.anim.class == 2 && dr.state != state::IDLE { angle_difference(dr.yaw, dr.yaw_to_opp).max(-0.9) } else { 0.0 };
            dr.aim_yaw = rotate_to_bearing(dr.aim_yaw, want, 0.2, crate::trig::PI);
            if let Some(b) = dr.handle_firing(d, rng, frame, player, (moving, first_moved), index) { self.bullets.push(b); }
            if let Some((next_ai, done_msg)) = dr.update_anim(d) {
                if done_msg != 0 { dr.send(d, rng, frame, done_msg); }
                if next_ai != 0 { dr.set_state(next_ai); dr.send(d, rng, frame, 0); }
            }
        }
        // Bullets: move, collide with the world and Bond (`Bullet_update` + `Collide_Update`), damage
        // (`Player_HandlePain`); the hit flash fades by 1 per tick (HUD).
        self.impacts.clear();
        let vol = crate::bullet::PlayerVolume { feet: [player.pos[0], player.pos[1] - 0.4, player.pos[2]], height: 1.8, radius: 0.25 };
        for b in &mut self.bullets {
            if let Some(ev) = b.update(d.world, &vol, &mut self.rng) {
                if let crate::bullet::BulletEvent::Player { damage, .. } = ev {
                    let before = self.health.health;
                    self.health.pain(damage, DIFFICULTY, d.level);
                    self.player_hits.push((frame, b.owner, before - self.health.health));
                }
                self.impacts.push(ev);
            }
        }
        self.bullets.retain(|b| b.state < 2);
        self.health.flash = (self.health.flash - 1.0).max(0.0);
    }
}
