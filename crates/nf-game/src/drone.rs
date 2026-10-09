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
    pub const WAIT_SWITCH: u8 = 0x01;
    pub const IDLE: u8 = 0x04;
    pub const ALERT: u8 = 0x05;
    pub const INIT_PATROL: u8 = 0x06;
    /// Sniper mode (DroneTypeSettings 2 / 0x19).
    pub const SNIPER_IDLE: u8 = 0x29;
    pub const SNIPER_AIM: u8 = 0x2a;
    pub const SNIPER_FIRE: u8 = 0x2b;
    pub const SNIPER_RELOAD: u8 = 0x2c;
    /// Surrender (docs/systems/drone-surrender.md): hands up, kept up, lowered; then the sidearm is drawn.
    pub const SURRENDER_ANIM: u8 = 0x3f;
    pub const SURRENDERED: u8 = 0x40;
    pub const UNSURRENDER_ANIM: u8 = 0x41;
    pub const DRAW_WEAPON: u8 = 0x66;
    pub const PATROL: u8 = 0x07;
    pub const ATTACK: u8 = 0x56;
    pub const COMBAT: u8 = 0x58;
    pub const COMBAT_NO_MOVE: u8 = 0x59;
    pub const COMBAT_OUT_OF_RANGE: u8 = 0x5a;
    pub const COMBAT_NEW_SIGHTING: u8 = 0x5b;
    pub const COMBAT_NO_SIGHT: u8 = 0x5c;
    pub const COMBAT_DRONE_AHEAD: u8 = 0x5e;
    pub const COMBAT_NO_ROUTE: u8 = 0x5f;
    pub const NO_OPPONENT: u8 = 0x60;
    pub const DRONE_STUCK: u8 = 0x7f;
    pub const GO_TO_GOAL: u8 = 0x63;
    /// Hit and death (docs/systems/drone-damage.md).
    pub const DEATH_ANIM: u8 = 0x44;
    pub const SPECIAL_DEATH: u8 = 0x46;
    pub const DEAD: u8 = 0x47;
    pub const FADE: u8 = 0x48;
    pub const FADE_FAST: u8 = 0x49;
    pub const BULLET_IMPACT: u8 = 0x55;
    pub const ALERT_TO_POSITION: u8 = 0x62;
    pub const SEARCH_AREA: u8 = 0x65;
    pub const HEARD_NOISE: u8 = 0xa2;
    pub const HEARD_NOISE_AWARE: u8 = 0xa3;
    pub const HEARD_NOISE_SUSPECT: u8 = 0xa4;
    pub const HEARD_NOISE_ALERT: u8 = 0xa5;
    pub const RUN_FOR_COVER: u8 = 0x8c;
    pub const UNDER_COVER_INIT: u8 = 0x93;
    pub const UNDER_COVER_IDLE: u8 = 0x94;
    pub const UNDER_COVER_AIM: u8 = 0x95;
    pub const UNDER_COVER_FIRE: u8 = 0x96;
    pub const UNDER_COVER_RETURN: u8 = 0x99;
    pub const UNDER_COVER_TYPE_CHANGE: u8 = 0x9a;
    pub const UNDER_COVER_LEAVE: u8 = 0x9b;
    pub const UNDER_COVER_LEAVE_NOW: u8 = 0x9c;
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
    /// A sound was heard (`DroneFunc_HandleSoundAlerts`); a turn finished (HeardNoise*); SearchArea's anim steps.
    pub const SOUND: u8 = 0x14;
    pub const TURNED: u8 = 0x23;
    pub const SEARCH_LOOKED: u8 = 0x24;
    pub const SEARCH_DONE: u8 = 0x25;
    pub const ANIM_DONE: u8 = 5;
    pub const TIMER_C: u8 = 0xc;
    pub const FORCED_ATTACK: u8 = 0x1f;
    /// `Drone_Message(0xE)`: enable (WaitSwitch → the drone's start state).
    pub const ENABLE: u8 = 0x0e;
    /// The alarm (`Drone_InitComms` → `Drone_Message(0, 0x1E)` every 30 frames while `switch_channels[0x96]`).
    pub const ALARM: u8 = 0x1e;
    /// A bullet hit (`Drone_BulletHit` → msg 8 with the HITDATA).
    pub const BULLET: u8 = 8;
}

/// Anim states (`DroneAnimStates` index).
pub mod anim_state {
    pub const AIM_STAND: u16 = 0x0a;
    /// Hands up (Surrender_Anim) and drawing the sidearm (DrawWeapon).
    pub const SURRENDER: u16 = 0x5c;
    pub const DRAW_WEAPON: u16 = 0x46;
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
    pub const RUN: u16 = 0x1e;
    pub const RUN_AIM: u16 = 0x08;
    pub const WALK_AIM: u16 = 0x0f;
    pub const STRAFE_LEFT_ALT: u16 = 0x29;
    pub const STRAFE_RIGHT_ALT: u16 = 0x2a;
    pub const ROLL_LEFT_ALT: u16 = 0x55;
    pub const ROLL_RIGHT_ALT: u16 = 0x56;
    pub const IMPACT: u16 = 0x4c;
    pub const DEATH: u16 = 0x40;
    pub const DEATH_SHOT: u16 = 0x42;
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
    /// Cover nodes (placements 0xE5 / 0xE6).
    pub covers: &'a [crate::cover::CoverNode],
    /// The level's tuning globals (`ReadTuningVars`) and the hit-box tables (`HumanCollisionBones`).
    pub tuning: &'a nf_formats::tuning::Tuning,
    pub hitboxes: &'a nf_formats::hitbox::CollisionBones,
    /// AI volumes (placements 0xF5 / 0xF8): Bond's visibility factor.
    pub volumes: &'a [crate::aivolume::AiVolume],
    /// Hearing data (`SFXOutputData`) for sounds the world makes (pickups); `None`: silent.
    pub sfx: Option<&'a nf_formats::sfx::SfxTable>,
    /// Glist bounds of the level's models by hashcode (dropped weapons' size).
    pub models: &'a HashMap<u32, crate::pickup::Model>,
}

/// A bullet that hit a drone (HITDATA): the part (bone id), the ray (`+0x30`), the point, damage (`+0x08`), weapon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit { pub part: i16, pub ray: V3, pub point: V3, pub damage: f32, pub weapon: u16 }

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

/// The player as an AI target (`DAT_00298fd0`): his position when the nodes were found, the frame (`+0x54`), and per
/// path the found / fallback node. Rebuilt when he has moved ≥ 2.0 (`BuildAITarget`).
#[derive(Clone, Debug, Default)]
pub struct PlayerTarget {
    pub pos: V3,
    pub stamp: u32,
    pub nodes: Vec<crate::nav::TargetNodes>,
}

/// The game's generator (`Rand_Rand` 0x1e3738, `Rand_FRand` 0x1e3780): two 16-bit multiply-with-carry streams, x × 18000
/// and y × 30903 (`a_318` / `b_319`, read from RAM), state `SEED_X` / `SEED_Y` (0x30d0a0 / 0x30d0a4). The whole game
/// shares it (particles, sounds …), so a live sequence cannot be reproduced; seeds pick a state.
#[derive(Clone, Debug)]
pub struct Rng { pub x: u32, pub y: u32 }
impl Rng {
    /// The state of a level-start snapshot of 07000005 (work/ee_start_05.bin), mixed with `seed`.
    pub fn new(seed: u32) -> Rng { Rng { x: 0x0659_5adb ^ seed.wrapping_mul(0x9e37_79b9), y: 0x208a_5de8 ^ seed } }
    fn step_x(&mut self) { self.x = 18000u32.wrapping_mul(self.x & 0xffff).wrapping_add(self.x >> 16); }
    fn step_y(&mut self) { self.y = 30903u32.wrapping_mul(self.y & 0xffff).wrapping_add(self.y >> 16); }
    /// `Rand_FRand(max)`: x then y step; (x << 16) + (y & 0xFFFF) converted by `cvt.s.w` (truncating; values with the
    /// top bit set are halved, converted and doubled), × 2⁻³², × max.
    pub fn frand(&mut self, max: f32) -> f32 {
        self.step_x();
        self.step_y();
        let v = (self.x << 16).wrapping_add(self.y & 0xffff);
        let cvt = |i: u32| -> f32 { let bits = 32 - i.leading_zeros(); if bits <= 24 { i as f32 } else { let sh = bits - 24; ((i >> sh) << sh) as f32 } };
        let f = if (v as i32) >= 0 { cvt(v) } else { let h = cvt((v >> 1) | (v & 1)); nf_formats::ps2float::add(h, h) };
        nf_formats::ps2float::mul(max, nf_formats::ps2float::mul(f, f32::from_bits(0x2f80_0000)))
    }
    /// `Rand_Rand(n)`: y then x step; ((y & 0xFFFF) × n) >> 16.
    pub fn rand(&mut self, n: u32) -> u32 {
        self.step_y();
        self.step_x();
        ((self.y & 0xffff).wrapping_mul(n)) >> 16
    }
}

/// Animation side of a drone: `DroneAnim_*` over the danim tables, playing scripts.
#[derive(Clone, Debug)]
pub struct Animator {
    pub danim: u16,
    pub script: Option<u32>,
    /// Ticks since the script started (script time = 1 + this).
    pub time: u32,
    /// Script time with fractions, and the playback speed (`Drone_AnimInfo +0x14`, e.g. 1.5 for lean-out 176).
    pub ftime: f32,
    pub speed: f32,
    /// `Drone+0x56A` anim state of the playing danim, `+0x568` its class (2 = aiming).
    pub state: u16,
    pub class: u8,
    /// Pending `DroneAnim_CallAnim`: (anim state, AI state on completion, message on completion).
    pub call: Option<(u16, u8, u8)>,
    /// The call's variant (`DroneAnim_CallAnim`'s third argument: consecutive danims of a record, e.g. 0x4B × 3).
    pub variant: u16,
    /// `+0x57B`: a danim of the called state started (the request itself is spent); `+0x55C`: the state playing when
    /// the call was made.
    pub reached: bool,
    pub call_from: u16,
    /// Frames left of the blend the call waits for (`+0x578` with the blend-out script `+0x538`).
    pub blend_wait: u32,
    /// `+0x57A`: a one-shot script (info flag bit 0 clear, `SetHTAnim`) still running — class-1 calls wait for its end;
    /// `held`: a one-shot without a follow-on ended and holds its last frame (`AnimScriptIsStopped`).
    pub once: bool,
    pub held: bool,
    /// The follow-on of a raw script (`DroneAnim_SetScript` `+0x594`), used when no danim plays.
    pub follow_on: u16,
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
    /// The second behaviour block (parameters 23–25), active after `FirstAttack` when parameter 5 = 100 (`+0xC48`):
    /// `DroneFunc_Set2ndBehaviour` points `+0x4D8` at `+0x4E8`.
    pub alert_props: Option<[u32; 3]>,
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
    /// `+0x22C`: Bond's visibility factor from the AI volumes he stands in (`Drone_ProcessOpponents`).
    pub light: f32,
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
    /// Combat route (`Drone+0x860`), the drone's navigation path (`+0x954`), the move quad (`+0x3C6`), the anim yaw
    /// offset (`+0x4C4`), CombatNoRoute's alternation (`+0xCD8`).
    pub croute: Option<crate::nav::Route>,
    pub nav_path: Option<Option<usize>>,
    pub move_quad: u8,
    pub yaw_offset: f32,
    pub no_route_odd: bool,
    /// The player's target nodes (`NDrone2_NavNodeCache`) and this frame's link used-counts, lent by `Drones::tick`.
    pub target: PlayerTarget,
    pub links_used: Vec<Vec<u16>>,
    /// Switch channel the drone waits for (`+0x134`, placement parameter 3; 0 = none), whether that channel is on
    /// this tick (`switch_channels`, lent by `Drones::tick`), and whether the drone is enabled (`NDrone2_Enable`:
    /// disabled drones have object flags 0x30 — not drawn — and no vision).
    pub switch_channel: u8,
    pub channel_on: bool,
    /// Position in the main map's placement list (update order: newest first).
    pub placement: usize,
    /// Spawner group (placement parameter 4); present in the world (spawner templates are not until spawned); the
    /// state machine has had its first enter.
    pub group: u32,
    pub present: bool,
    pub started: bool,
    /// Drawn last frame (`obj+0xFC`, set by the renderer's cel visibility; *suggested* stand-in: within 60° of Bond's
    /// view and in line of sight from his eye), set by `Drones::tick`.
    pub drawn: bool,
    pub enabled: bool,
    /// Index in `Drones::list` (cover ownership) and the state being entered during an exit message.
    pub index: usize,
    pub exiting_to: u8,
    /// A message the drone sends itself with no delay (`Drone_SM_SendMsgSelf` → `RouteMsgDCV`), routed after the
    /// handler that sent it.
    pub self_msg: Option<u8>,
    /// Cover: node (`+0xBB4`), side (`+0xBB8 & 3`), type (`+0xBBB`), leave flags (`+0xBB8` 4 / 8), the in-cover point
    /// (`+0x830`), the goal point (`+0x710`), `+0x4F8` 0x800 (claimed this run) / 0x1000 (has cover), `+0xCD8` tries.
    pub cover: Option<usize>,
    pub cover_side: u8,
    pub cover_type: u8,
    pub cover_leave: u8,
    pub cover_point: V3,
    pub goal: Option<V3>,
    pub cover_claimed: bool,
    pub has_cover: bool,
    pub cover_tries: u32,
    /// Lent by `Drones::tick`: the cover nodes' run-time state and the enabled drones' positions; the enabled drones in
    /// list order with whether they see Bond (`SetGoalToDroneWithLosToOpponent`).
    pub cover_rt: Vec<crate::cover::CoverRt>,
    pub others: Vec<Option<V3>>,
    pub peers: Vec<(usize, V3, bool)>,
    /// The state `GoToGoalPosition` returns to (`+0x5A2`); the second switch (`+0x135`, parameter 6) and its tag
    /// (`+0x13A`, parameter 8).
    pub return_state: u8,
    pub second_channel: u8,
    pub second_tag: u32,
    /// Hearing (docs/systems/drone-alerts.md): loudness this frame (`+0x510`), its alertness share (`+0x514`), the
    /// last heard frame and peak (`+0xB38` / `+0xB3C`), the sound's position (`+0xAF0`), the noise level (`+0x43`,
    /// never reset), "investigating" (`+0x33`), "turned" (`+0x34`), the state and home to return to (`+0x5A0` /
    /// `+0x5B0`).
    pub loudness: f32,
    pub sound_share: f32,
    pub heard_frame: u32,
    pub heard_peak: f32,
    pub sound_pos: V3,
    pub noise_level: u8,
    pub investigating: bool,
    pub turned: bool,
    pub resume_state: u8,
    pub home: V3,
    /// Damage (`NDrone2_DefaultInit`, docs/systems/drone-damage.md): health `+0xAC` / max `+0xB0`, damage-taken
    /// multiplier `+0x14C`, damageable `+0x14`, type `+0xD8`, the last damage `+0x150`, head shot `+0x3A`, big hit
    /// `+0x1C`, the last flinch `+0x540`, the last hit `+0x470`, the state after a flinch `+0x118`, hits taken `+0x1D8`.
    pub health: f32,
    pub max_health: f32,
    pub damage_taken: f32,
    pub damageable: bool,
    pub kind: u16,
    pub last_damage: f32,
    pub silent_kill: bool,
    pub big_hit: bool,
    pub flinch_at: Option<u32>,
    pub hit: Option<Hit>,
    pub impact_resume: u8,
    pub hits_taken: u32,
    /// Lent by `Drones`: the global last flinch (`DAT_00298FC0`), whether Bond aims (L1, player `anim+0x60` bit 0),
    /// and whether this drone is the oldest corpse beyond the limit (`DroneFunc_DeadDrone`).
    pub global_flinch: Option<u32>,
    pub bond_aiming: bool,
    pub cull: bool,
    /// Lent by `Drones`: Bond holds a gun (`DroneWeap_OpponentHasWeapon` / `NDrone2_ObjectIsArmed`: weapon byte 2–0x46
    /// or 0x6A–0x71). `+0x1D0`: AngleDiff(Bond's yaw, bearing Bond → drone).
    pub bond_armed: bool,
    pub opp_facing: f32,
    /// Bond's chest (`Player::target`) for this drone's own line-of-sight tests.
    pub opp_target: V3,
    /// `+0x4F8` 0x4000000 (a sighting happened), 0x8000000 (it was this drone's own) and 0x2000000 (`FirstAttack`
    /// done).
    pub sighted: bool,
    pub own_sight: bool,
    pub first_attack_done: bool,
    /// Corpse (`obj+0xF0 & 0x20`: no longer hit or seeing), the death channel to raise (`+0xCD8` pending, `+0x13C`
    /// = placement parameter 12), the special-death script (`+0xBD4`, parameter 13), alpha (`obj+0x106` / `+0xCFC`).
    pub corpse: bool,
    pub death_pending: bool,
    pub death_channel: u8,
    pub special_death: u32,
    /// The spot a special death plays at (`+0xBE0`: the object's position at creation; `+0xBF4`: its yaw) and the
    /// stationary flag (`+0x4F8 & 0x10`: set at creation for special-death scripts other than 0x0600021F; blocks combat
    /// moves and routes like property 0x40).
    pub special_spot: V3,
    pub special_yaw: f32,
    pub stationary: bool,
    /// A captain (`+0x19`).
    pub captain: bool,
    /// `+0x1A`: the special death's free root motion — the whole root translation (vertical too) moves the object,
    /// which is then the hip (`DroneAnim_PreTransCallback` keeps the vertical part and adds the motion to the slide
    /// target `+0xBE0`); collision, gravity (`+0x4F8 & 0x20 / 0x80`) and the AI-boundary push-out (`& 0x80000000`,
    /// `bounded`) are off.
    pub free_root: bool,
    pub bounded: bool,
    /// Pickups this drone made this tick (`DroneWeap_DropWeapon`, `DroneFunc_DropGrenade`), collected by `Drones`.
    pub drops: Vec<crate::pickup::Pickup>,
    /// Drone modes (`behaviour_util_get`: placement parameter 15 = type `+0xC7`, 16 = the normal block's mode `+0xC8`,
    /// 21 = the alert block's `+0xC9`; `NDrone2_GetDTYPENEW` → mode `+0xC5`, alternate mode `+0xC6`), the team `+0x44`,
    /// the alternate (attack) state `+0x5A4`, and the alert block's alertness `+0x504`.
    pub dtype: u8,
    pub mode_normal: u8,
    pub mode_alert: u8,
    pub mode: u8,
    pub alt_mode: u8,
    pub team: u8,
    pub alt_state: u8,
    pub alert_block: f32,
    pub alpha: f32,
    /// A switch channel to raise after the drone's update (`SetDeathChannel`).
    pub raise: Option<u8>,
    /// Trace of (frame, state) changes, for comparisons with PCSX2.
    pub log: Vec<(u32, u8)>,
}

impl Drone {
    /// From a placement (parameters by index) after `NDrone2_DefaultInit` / `DoModeSettingsNEW`.
    pub fn new(d: &Data, pos: V3, yaw: f32, param: impl Fn(u32) -> u32, rng: &mut Rng) -> Drone {
        let skin = param(0);
        let setup = nf_formats::drone::drone_setup(skin, param(9), d.level);
        let (start_danim, anim_set) = nf_formats::drone::start_danim(setup, 0);
        let (normal, alert_words) = nf_formats::drone::behaviour_words(&param);
        let alert_props = (param(5) == 100).then_some(alert_words);
        let alert = nf_formats::drone::ALERTNESS_LEVELS[d.props.get(0x20, &normal) as usize & 3];
        let weapon = nf_formats::drone::weapon_id(skin, param(9));
        // Sight constants by class (parameter 14), `NDrone2_DefaultInit`; e4 = π/2 for every class.
        let (e8, f0_base, f0_rand, fc_add) = match param(14) { 1 => (12.0, 12.0, 3.0, 1.0), 2 => (16.0, 12.0, 3.0, 1.0),
            3 => (40.0, 25.0, 6.0, 100.0), 4 => (1000.0, 75.0, 8.0, 1000.0), 5 => (0.0, 12.0, 4.0, 1.0), _ => (24.0, 12.0, 4.0, 1.0) };
        let f0 = f0_base + rng.frand(f0_rand);
        let fc = match param(14) { 3 => fc_add + rng.frand(6.0), 4 => 1000.0, _ => f0 + fc_add + rng.frand(f0_rand) };
        let clip = match weapon { 0x16 | 0x18..=0x1b => 0x40, 0x1e | 0x1f | 0x24 | 0x25 => 5, _ => 0x40 };
        let mut dr = Drone {
            skin, anim_set, weapon, pos, yaw, desired_yaw: yaw, aim_yaw: 0.0, alert, alert_floor: alert, props: normal, alert_props,
            sight_range: e8, half_fov: crate::trig::HALF_PI, engage_range: f0, combat_range: fc,
            // `drone_stats` for the guards' type is all zero at run time (07000005 snapshot) → defaults.
            accuracy: 5, aggression: 0, reaction: 0,
            los: false, seen: false, ever_seen: false, visibility: 0.0, seen_ticks: 0, unseen_ticks: 0, vision_on: false,
            dist: 0.0, yaw_to_opp: 0.0,
            state: state::GLOBAL, prev_state: 0, next_state: None, entered: 0, timer_c: None,
            anim: Animator { danim: 0, script: None, time: 0, state: 0, class: 0, call: None, reached: false, call_from: 0, blend_wait: 0, ftime: 0.0, speed: 1.0, variant: 0, once: false, held: false, follow_on: 0 },
            firing: false, burst_done: false, burst: 0, next_shot: 0, last_shot: 0, clip, clip_max: clip,
            shots: Vec::new(), log: Vec::new(), root_prev: None, aimed_at: false, aimed_until: 0, bunched: false, wobble: rng.rand(10000), lost_sight: true, damage_mul: 1.0, alert_bits: 0, light: 1.0, attack_frame: 0, alerted_by_drone: false, watching: false,
            route: None, last_wait: None, idle_timeout: None, delayed: None, sm_id: 0, stand_until: 0,
            croute: None, nav_path: None, move_quad: 0, yaw_offset: 0.0, no_route_odd: false, target: PlayerTarget::default(), links_used: Vec::new(),
            switch_channel: param(3) as u8, channel_on: false, placement: 0, group: param(4), present: true, started: false, drawn: true, enabled: true, index: 0, exiting_to: 0, self_msg: None,
            cover: None, cover_side: 0, cover_type: 0, cover_leave: 0, cover_point: pos, goal: None, cover_claimed: false, has_cover: false, cover_tries: 0,
            cover_rt: Vec::new(), others: Vec::new(), peers: Vec::new(), return_state: 0, second_channel: param(6) as u8, second_tag: param(8),
            loudness: 0.0, sound_share: 0.0, heard_frame: 0, heard_peak: 0.0, sound_pos: pos, noise_level: 0, investigating: false, turned: false, resume_state: 0, home: pos,
            // `drone_stats[type]+8` is 0 for these guards → 10 (captains, difficulty ≥ 2 only, are not modelled).
            health: 10.0, max_health: 10.0, damage_taken: 1.0, damageable: !matches!(skin, 0x0500_003b | 0x0500_003c | 0x0500_0053), kind: setup.kind,
            last_damage: 0.0, silent_kill: false, big_hit: false, flinch_at: None, hit: None, impact_resume: 0, hits_taken: 0,
            global_flinch: None, bond_aiming: false, cull: false, bond_armed: true, opp_facing: 0.0, opp_target: [0.0; 3], sighted: false, own_sight: false, first_attack_done: false,
            corpse: false, death_pending: false, death_channel: param(12) as u8, special_death: param(13), special_spot: pos, special_yaw: yaw,
            stationary: param(13) != 0 && param(13) != 0x0600_021f, alpha: 255.0, raise: None,
            captain: false, drops: Vec::new(), free_root: false, bounded: true, dtype: param(15) as u8, mode_normal: param(16) as u8, mode_alert: param(21) as u8, mode: 0, alt_mode: 0, team: 1, alt_state: 0,
            alert_block: alert_props.map_or(0.0, |w| nf_formats::drone::ALERTNESS_LEVELS[d.props.get(0x20, &w) as usize & 3]),
        };
        dr.init_properties(d, &param);
        dr.type_setup(d);
        dr.captain_setup(d);
        dr.set_danim(d, start_danim);
        dr
    }

    /// `NDrone2_DefaultInit` after the mode settings: a captain (`+0x19`) by difficulty — property 9 on 1, 5 on 2, 8 on
    /// 3 — for kinds 1, 2, 3, 6, 7, each with its captain skin (0x5000038, 0x500002F, 0x5000006, 0x5000093, 0x50000B2);
    /// health × `DroneCaptain_Mod_Health`, `+0x100` = `Mod_BulletDamage`, accuracy `+0xB4` × 1 / `Mod_BulletAccuracy`
    /// (truncated). Type 0xC drops properties 0x14–0x16 (kind 9 also gets 0x4C).
    fn captain_setup(&mut self, d: &Data) {
        use nf_formats::ps2float::mul;
        let by_difficulty = match DIFFICULTY { 1 => self.prop(d, 9), 2 => self.prop(d, 5), 3 => self.prop(d, 8), _ => false };
        if by_difficulty {
            let skin = match self.kind { 1 => Some(0x0500_0038), 2 => Some(0x0500_002f), 3 => Some(0x0500_0006), 6 => Some(0x0500_0093), 7 => Some(0x0500_00b2), _ => None };
            if let Some(skin) = skin {
                self.captain = true;
                self.skin = skin;
                let t = d.tuning;
                self.health = mul(self.health, t.DroneCaptain_Mod_Health);
                self.max_health = self.health;
                self.damage_mul = t.DroneCaptain_Mod_BulletDamage;
                self.accuracy = (self.accuracy as f32 * (1.0 / t.DroneCaptain_Mod_BulletAccuracy)) as u8;
            }
        }
        if self.dtype == 0xc {
            for p in [0x14, 0x15, 0x16] { d.props.set(p, &mut self.props, 0); }
            if self.kind == 9 { d.props.set(0x4c, &mut self.props, 1); }
        }
    }

    /// `NDrone2_GetDTYPENEW` (single player): the mode for type `ty` and a block's mode `block` (0 = the base).
    fn dtype_of(&self, d: &Data, ty: u8, block: u8) -> u8 {
        if block == 0 && self.prop(d, 0x10) { return 0x10; }
        let lvl7 = d.level == 0x0700_0007;
        match ty {
            0..=9 if lvl7 && block == 0 => return if self.prop(d, 0x4b) { 0x16 } else { 0x13 },
            10 | 11 if block == 0 => return if self.alert < 1.0 { if self.damageable { 9 } else { 0x12 } } else { 10 },
            0xc if block == 0 => return if self.kind == 0xc { 0x11 } else { 0xc },
            0xd if block == 0 => return if self.kind == 0xf { 0x17 } else if self.alert < 1.0 { 9 } else { 10 },
            0xe if block == 0 => return if self.prop(d, 0x4e) { 5 } else if self.alert >= 1.0 { 10 } else { 9 },
            0xf if block == 0 => return 0xd,
            0x10 => return 0x1e,
            _ => {}
        }
        match block { 2 => if self.alert >= 1.0 { 0x19 } else { 2 }, 3 => if self.prop(d, 0x56) { 0xe } else { 7 }, 4 => 4, 5 => 0x15, _ => 0 }
    }

    /// `NDrone2_GetDroneTypeAttackTypeFriend` (0x14cbd8) at creation: modes, team, the type's init function (the
    /// sniper's sets the stationary flag), the alternate state `+0x5A4`; a second tag `+0x13A` is dropped when both
    /// blocks have the same mode, and set to 100 when there is an alternate state but no tag. (The 07000007 property
    /// writes inside `GetDTYPENEW` are not modelled.)
    fn type_setup(&mut self, d: &Data) {
        if self.mode_normal == self.mode_alert || self.second_tag == 0x65 { self.second_tag = 0; }
        self.mode = self.dtype_of(d, self.dtype, self.mode_normal);
        self.alt_mode = if self.second_tag == 0 || self.second_tag == 0x65 { 0 }
            else if self.second_channel == 0 && self.alert_floor >= 1.0 { 0 }
            else { self.dtype_of(d, self.dtype, self.mode_alert) };
        self.team = match self.dtype { 10 | 11 | 0xd | 0xe => 3, 0xc => 2, _ => 1 };
        let row = |m: u8| d.props.types.get(m as usize);
        if row(self.mode).and_then(|t| t.init.as_deref()).is_some_and(|n| n.starts_with("NDrone2_initDTYPE_Sniper")) { self.stationary = true; }
        self.alt_state = if self.alt_mode == 0 { 0 } else {
            let a = row(self.alt_mode).map_or(0, |t| t.alt);
            (if a != 0 { a } else { row(self.mode).map_or(0, |t| t.alt) }) as u8
        };
        if self.alt_mode == 0 && self.alt_state == 0 && (self.second_channel != 0 || self.alert_block >= 1.0) {
            self.alt_state = match self.start_state(d) { 4 | 5 | 6 | 8 | 0x30 | 0x31 | 0x36 | 0x37 | 0x85 | 0x89 => 0x56, 0x10 | 0x2e | 0x32 => 0x12, 0x29 => 0x2a, 0xa6 => 0xa7, _ => 0 };
        }
        if self.alt_state != 0 && self.second_tag == 0 { self.second_tag = 100; }
    }

    fn prop(&self, d: &Data, id: usize) -> bool { d.props.get(id, &self.props) != 0 }

    /// "Stays put": `+0x4F8 & 0x10` or property 0x40, tested together throughout the decomp (`DroneFunc_CombatState`,
    /// `ChooseCombatMove`, CombatNoRoute, AimStandFire, HeardNoiseSuspect / Alert, …).
    fn stays(&self, d: &Data) -> bool { self.stationary || self.prop(d, 0x40) }

    /// `Drone+0x5A2`, the state a drone starts in once enabled.
    /// Idle / Alert / InitPatrol become Attack when the drone is fully alert (`GetDroneTypeAttackTypeFriend`): [L] the
    /// 0xA4 waiters 271 / 272 (alertness 1) went straight into combat when the channel pulsed (WORKLOG 0028).
    /// The mode's start state from `DroneTypeSettings`, or for −1 by mode (2 → SniperIdle, or SniperAim when fully
    /// alert; 4 → 8, 7 → 0x85, 0x10 → 0x37, 0x13 → 0x30, 0x16 → 0x31, 0x18 → 0x36; else InitPatrol with property
    /// 0x24 / 0x25, Idle without); then, at alertness 1, the attack versions.
    fn start_state(&self, d: &Data) -> u8 {
        let t = d.props.types.get(self.mode as usize).map_or(-1, |t| t.start);
        let s: u16 = if t >= 0 { t as u16 } else {
            match self.mode {
                2 => if self.alert < 1.0 { 0x29 } else { 0x2a },
                4 => 8, 7 => 0x85, 0x10 => 0x37, 0x13 => 0x30, 0x16 => 0x31, 0x18 => 0x36,
                _ => if self.prop(d, 0x24) || self.prop(d, 0x25) { 6 } else { 4 },
            }
        };
        if self.alert >= 1.0 {
            return match s { 4 | 5 | 6 | 0x30 | 0x31 | 0x36 | 0x56 => 0x56, 0x10 | 0x12 | 0x2e | 0x32 | 0xb5 => 0x12, 0x29 | 0x2a => 0x2a, 0xa6 | 0xa7 => 0xa7, x => x as u8 };
        }
        s as u8
    }

    /// Property writes at creation (first block): `DoModeSettingsNEW` clears 0x1A (sets 0x1A and 0x56 on 07000009/0A,
    /// leaves it on 07000007); `NDrone2_DefaultInit` in single player clears 0x17, and on 07000005/06 gives SG5 guards
    /// (parameter 9 = 0x16) props 0, 1, 6, 0x41, 0x42, clears 0x13 and sets the engage range `+0xF0` to 10. [L] the
    /// level-start snapshot of 07000005 (work/ee_start_05.bin) differs from the placement words by exactly these bits.
    /// Mode-specific writes (modes 0xC, 0x11, mission scripts) are not ported.
    fn init_properties(&mut self, d: &Data, param: &impl Fn(u32) -> u32) {
        let set = |w: &mut [u32; 3], id: usize, v: u32| d.props.set(id, w, v);
        if param(5) >= 0x24 {
            match d.level {
                0x0700_0007 => {}
                0x0700_0009 | 0x0700_000a => { set(&mut self.props, 0x1a, 1); set(&mut self.props, 0x56, 1); }
                _ => set(&mut self.props, 0x1a, 0),
            }
        }
        set(&mut self.props, 0x17, 0);
        if matches!(d.level, 0x0700_0005 | 0x0700_0006) && param(9) == 0x16 {
            for id in [0, 1, 6, 0x41, 0x42] { set(&mut self.props, id, 1); }
            set(&mut self.props, 0x13, 0);
            self.engage_range = 10.0;
        }
    }

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
        self.call_anim(d, anim, 0, 0);
    }

    /// `SetStandIdleAnim(force)`: in Idle one of 0x24..0x26 (Rand 3) held 5 + Rand 5 seconds; elsewhere 0x24.
    fn set_stand_idle(&mut self, d: &Data, rng: &mut Rng, frame: u32, force: bool) {
        if self.state != state::IDLE { self.request_anim(d, anim_state::STAND_IDLE); return; }
        if !force && frame < self.stand_until { return; }
        let mut anim = anim_state::STAND_IDLE + rng.rand(3) as u16;
        if !self.can_do(d, anim) { anim = anim_state::STAND_IDLE }
        self.stand_until = frame + (5 + rng.rand(5)) * 60;
        if self.anim.call.is_none() && self.anim.state != anim { self.call_anim(d, anim, 0, 0) }
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

    /// `DroneAnim_SetDAnimInternal` (0x13aee8): start a danim's script for our anim set. With no script, or for a drone
    /// that was not drawn (`obj+0xFC` = 0) given a danim with info flag 4 of class 0, 3–7 or 9, the danim ends at once
    /// (`SetEndAnim(0x1E)`) and its follow-on (`+10`, when not itself) starts instead — e.g. an off-screen guard's
    /// look-around 265 (class 3, flags 6) goes straight to alert idle 244.
    fn set_danim(&mut self, d: &Data, danim: u16) {
        let mut danim = danim;
        for _ in 0..8 {
            let info = d.tables.info.get(danim as usize).copied().unwrap_or([0; 24]);
            self.anim.danim = danim;
            self.anim.state = u16::from_le_bytes([info[8], info[9]]);
            self.anim.class = info[0xc];
            self.anim.script = d.tables.script_for(danim, self.anim_set);
            self.anim.time = 0;
            self.anim.ftime = 0.0;
            let speed = f32::from_le_bytes([info[0x14], info[0x15], info[0x16], info[0x17]]);
            self.anim.speed = if speed > 0.0 && speed.is_finite() { speed } else { 1.0 };
            self.anim.blend_wait = 0;
            self.anim.once = info[0] & 1 == 0;
            self.anim.held = false;
            self.anim.follow_on = 0;
            // A danim of the called state (0x41..0x44, 0x50 and 0x6F count as 0x40) or of state 0 marks the call
            // reached (`+0x57B`) and zeroes the request (`ZeroAnimCall`; the completion action `+0x596`/`+0x598` stays).
            let want = self.anim.call.filter(|_| !self.anim.reached).map_or(0, |(w, _, _)| match w { 0x41..=0x44 | 0x50 | 0x6f => 0x40, w => w });
            if self.anim.state == 0 || self.anim.state == want { self.anim.reached = true; }
            // A new script has no previous root: no motion until its second tick (a call starting a danim after this
            // tick's root motion would otherwise move by the old sequence's root minus the new one's).
            self.root_prev = None;
            let skip = self.anim.script.is_none() || (!self.drawn && info[0] & 4 != 0 && matches!(self.anim.class, 0 | 3..=7 | 9));
            let next = u16::from_le_bytes([info[10], info[11]]);
            if !(skip && next != 0 && next != danim) { break; }
            danim = next;
        }
    }

    /// `DroneAnim_SetScript(script, state, follow-on, …, next AI state, message, flags)` (0x13e088): play a script by
    /// hashcode as anim state `state`, class 1, the call already reached with the given completion; flags 0 = one-shot.
    fn play_script(&mut self, hash: u32, state: u16, follow_on: u16, next_ai: u8, done_msg: u8) {
        self.anim.danim = 0;
        self.anim.state = state;
        self.anim.class = 1;
        self.anim.script = Some(hash);
        self.anim.time = 0;
        self.anim.ftime = 0.0;
        self.anim.speed = 1.0;
        self.anim.blend_wait = 0;
        self.anim.once = true;
        self.anim.held = false;
        self.anim.follow_on = follow_on;
        self.anim.call = Some((state, next_ai, done_msg));
        self.anim.reached = true;
        self.root_prev = None;
    }

    /// `DroneAnim_CallAnim(blend, state, variant, next AI state, completion message)` (0x13ab58).
    fn call_anim(&mut self, d: &Data, state: u16, next_ai: u8, done_msg: u8) {
        self.call_anim_variant(d, state, 0, next_ai, done_msg);
    }

    /// `DroneAnim_CallAnim` with a variant (consecutive danims of a record, e.g. 0x4B × 3). A state the anim set
    /// cannot do is replaced (0x22 → 0x21, 0x0B → 0x0A …, else 0x21; 0x24 when even that is missing). The playing
    /// state completes at once (`CallFullyComplete`: the AI state is queued and the message routed now), or with no
    /// completion action zeroes the call (`ZeroAnimCall`); the same request again keeps the running call; anything
    /// else replaces it (`+0x55C` = the state playing now, `+0x57B` cleared).
    fn call_anim_variant(&mut self, d: &Data, state: u16, variant: u16, next_ai: u8, done_msg: u8) {
        self.yaw_offset = 0.0; // `DroneAnim_CallAnim`'s yaw-offset argument is 0 except in SetCombatMoveAnim (*suggested*)
        let mut state = state;
        if state == 0 || !self.can_do(d, state) {
            state = match state {
                0xb | 0xc | 0xd | 0x29 | 0x2a | 0x3d | 0x4a | 0x4f | 0x52..=0x56 | 0x5d | 0x5e | 0x61 => 10,
                0xf | 0x36 => 0x35,
                0x1f | 0x62 => 0x1e,
                0x25..=0x27 => 0x24,
                0x4c => self.anim.state,
                0x57 => if self.can_do(d, 0x11) { 0x11 } else if self.can_do(d, 0x19) { 0x19 } else { 0x21 },
                _ => 0x21,
            };
            if !self.can_do(d, state) { state = anim_state::STAND_IDLE; }
        }
        if state == self.anim.state {
            if next_ai == 0 && done_msg == 0 { self.anim.call = None; return; }
            self.anim.call = None;
            self.anim.reached = false;
            self.set_state(next_ai);
            if done_msg != 0 { self.self_msg = Some(done_msg); }
            return;
        }
        if self.anim.call == Some((state, next_ai, done_msg)) && !self.anim.reached && self.anim.variant == variant { return; }
        self.anim.call = Some((state, next_ai, done_msg));
        self.anim.reached = false;
        self.anim.call_from = self.anim.state;
        self.anim.variant = variant;
    }

    /// `DroneAnim_CallHandler` (0x13add0), every frame after the anim tick: once a danim of the requested state started
    /// (`+0x57B`) and no blend is waited for (`+0x578`), the call completes when that danim's class is 1, 2 or 9;
    /// until the state is reached the transition is looked up again from the state playing now
    /// (`DroneAnim_CanSetAnimCall`: not while a class-5 danim of another state than the call's start plays) and set.
    fn call_handler(&mut self, d: &Data) -> Option<(u8, u8)> {
        self.anim.blend_wait = self.anim.blend_wait.saturating_sub(1);
        if self.anim.reached && self.anim.blend_wait == 0 && (matches!(self.anim.class, 2 | 9) || (self.anim.class == 1 && !self.anim.once)) {
            self.anim.reached = false;
            let (_, next_ai, done_msg) = self.anim.call.take()?;
            return Some((next_ai, done_msg));
        }
        let (want, _, _) = self.anim.call?;
        if self.anim.reached { return None; }
        let (cur, class) = (self.anim.state, self.anim.class);
        if class == 5 && cur != self.anim.call_from { return None; }
        if matches!(cur, 0xc | 0xd | 0x29 | 0x2a) && want == 10 && class == 5 { return None; }
        if let Some(t) = d.tables.transition(want, cur, self.anim.variant).filter(|t| t.danim > 0) {
            self.set_danim(d, t.danim as u16);
            self.anim.blend_wait = if t.b5 != 0 { t.b4 as u32 } else { 0 };
        }
        None
    }

    /// Advances the script; at its end continues with the danim's follow-on (`Drone_AnimInfo +10`); handles a pending
    /// call (`DroneAnim_CallHandler`): start the transition danim, complete when the anim state is reached.
    fn update_anim(&mut self, d: &Data) -> Option<(u8, u8)> {
        // `Drone_Control` / `NDrone2_DoAnimation` (0x149f88): a drone that was not drawn (`obj+0xFC` = 0) playing a danim
        // with info flag 4 and no blend does not animate; it slides along its yaw by the danim's speed (`Drone_AnimInfo
        // +4`, e.g. 0.02 for walk 392). [L] work/switch_trace.tsv: a Dr_17 patroller behind the castle moved exactly
        // 0.02 per frame with no gait while the drawn patroller near the gate shows the walk's 115-tick gait.
        let info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
        if !self.drawn && info[0] & 4 != 0 && self.anim.blend_wait == 0 {
            let v = f32::from_le_bytes([info[4], info[5], info[6], info[7]]);
            if v != 0.0 && v.is_finite() { self.move_by(d, [crate::trig::sinf(self.yaw) * v, crate::trig::cosf(self.yaw) * v]); }
            self.root_prev = None;
            return self.call_handler(d);
        }
        if self.anim.held { self.root_prev = None; return self.call_handler(d); }
        let before = (self.anim.script, self.anim.danim);
        // Playback speed (`Drone_AnimInfo +0x14`, passed to the script by `SetHTAnim`): [L] lean-out 176 (51 frames at
        // 1.5) lasted 35 ticks live (work/chase_trace.tsv).
        self.anim.ftime += self.anim.speed;
        self.anim.time = self.anim.ftime as u32;
        let len = self.anim.script.and_then(|h| d.scripts.get(&h)).map(|s| s.length as u32).unwrap_or(1);
        let mut restarted = false;
        let mut wrap_end = None;
        if self.anim.time >= len {
            // The wrap tick repeats the script's last step (last frame − the one before): [L] aligned per-tick steps of
            // a live walk cycle (115 ticks, work/patrol_trace.tsv) show the step before the wrap twice.
            self.anim.time = len.saturating_sub(1);
            let last = self.root_value(d);
            self.anim.time = len.saturating_sub(2);
            wrap_end = self.root_value(d).zip(last).map(|(a, b)| [b[0] - a[0], 0.0, b[2] - a[2]]);
            let info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
            let next = if self.anim.danim == 0 { self.anim.follow_on } else { u16::from_le_bytes([info[10], info[11]]) };
            if next != 0 && next != self.anim.danim { self.set_danim(d, next) }
            else if self.anim.once {
                // `AnimEndCallback` + `CheckForAnimComplete`: the script stops on its last frame; `+0x57A` clears.
                self.anim.once = false;
                self.anim.held = true;
                self.anim.time = len.saturating_sub(1);
                self.anim.ftime = self.anim.time as f32;
            } else {
                self.anim.ftime = (self.anim.ftime - len as f32).max(0.0);
                self.anim.time = self.anim.ftime as u32;
            }
            restarted = true;
        }
        let same = before == (self.anim.script, self.anim.danim);
        self.root_motion(d, restarted || !same, if same { wrap_end } else { None });
        // [L] work/chase_trace.tsv: UnderCoverInit reaches loop 171 at once through state 0's generic record (blend 30,
        // wait) and completes 31 ticks later; UnderCoverAim's transition 176 (class 5) leads into loop 182 and completes
        // then.
        self.call_handler(d)
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
            if self.free_root {
                // Unblocked, with the vertical change; the slide target moves along.
                let ly = match (root, self.root_prev, restarted, wrap_end) {
                    (Some(r), Some(prev), false, _) => r[1] - prev[1],
                    _ => 0.0,
                };
                let m = [step[0], ly, step[1]];
                for k in 0..3 { self.pos[k] += m[k]; self.special_spot[k] += m[k]; }
            } else {
                self.move_by(d, step);
            }
        }
        self.root_prev = root;
    }

    /// Moves x/z by `step` unless the body (0.4 above the feet sphere … head) would cross collision, then stands on the
    /// floor below.
    fn move_by(&mut self, d: &Data, step: [f32; 2]) {
        let to = [self.pos[0] + step[0], self.pos[1], self.pos[2] + step[1]];
        let probe = |y: f32| d.world.line_of_sight([self.pos[0], y, self.pos[2]], [to[0] + step[0].signum() * 0.4 * (step[0].abs() > 1e-6) as i32 as f32, y, to[2] + step[1].signum() * 0.4 * (step[1].abs() > 1e-6) as i32 as f32]);
        if probe(self.pos[1]) && probe(self.pos[1] + 0.5) {
            self.pos = to;
            if let Some(f) = d.world.floor_below([to[0], self.pos[1], to[2]], 2.0) { self.pos[1] = f + 1.0; }
        }
    }

    // ---- damage and death (docs/systems/drone-damage.md) ---------------------------------------------------------

    /// Each state's `DroneFunc_HandleImpact(msg, resume, allow)` for msg 8: (resume state, 0 = the current one; whether
    /// a flinch is allowed). `None`: the state ignores bullets.
    fn impact_args(s: u8) -> Option<(u8, bool)> {
        use state::*;
        Some(match s {
            IDLE | ALERT | PATROL | ALERT_TO_POSITION | SEARCH_AREA | HEARD_NOISE_AWARE | HEARD_NOISE_SUSPECT | HEARD_NOISE_ALERT
                | RUN_FOR_COVER | UNDER_COVER_INIT | UNDER_COVER_LEAVE_NOW => (ATTACK, false),
            UNDER_COVER_IDLE | UNDER_COVER_AIM | UNDER_COVER_FIRE | UNDER_COVER_RETURN => (UNDER_COVER_LEAVE_NOW, false),
            AIM_STAND | AIM_BACKOFF | STEP_AIM_LEFT | STEP_AIM_RIGHT | STRAFE_AIM_LEFT | STRAFE_AIM_RIGHT | STRAFE_DODGE_LEFT
                | STRAFE_DODGE_RIGHT | ROLL_LEFT_CROUCH | ROLL_RIGHT_CROUCH => (AIM_STAND_FIRE, true),
            AIM_STAND_FIRE | AIM_CROUCH_FIRE | COMBAT_NO_MOVE | COMBAT_NO_ROUTE | SNIPER_FIRE => (0, true),
            SNIPER_IDLE => (SNIPER_AIM, false),
            SNIPER_AIM => (SNIPER_FIRE, true),
            SNIPER_RELOAD => (SNIPER_FIRE, false),
            // Surrendered (0x1626a0 / 0x162950): any bullet kills (`HandleImpact(0x44, 0)`); Unsurrender (0x162a70)
            // `HandleImpact(0, 0)`; DrawWeapon *suggested* the same.
            SURRENDER_ANIM | SURRENDERED => (DEATH_ANIM, false),
            UNSURRENDER_ANIM | DRAW_WEAPON => (0, false),
            AIM_CROUCH | AIM_STAND_RELOAD | AIM_CROUCH_RELOAD | COMBAT_OUT_OF_RANGE | COMBAT_NO_SIGHT | COMBAT_DRONE_AHEAD
                | GO_TO_GOAL | DRONE_STUCK => (0, false),
            _ => return None,
        })
    }

    /// Msg 8 in state `s`: the BulletImpact state calls `NDrone2_BulletImpact(…, 0, 0)` itself; the others go through
    /// `DroneFunc_HandleImpact`.
    fn on_bullet(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8) -> bool {
        if s == state::BULLET_IMPACT { self.bullet_impact(d, rng, frame, 0, false); return true; }
        let Some((resume, allow)) = Self::impact_args(s) else { return false };
        // `DroneFunc_HandleImpact` case 8 (0x1469b8): resume = given or the current state; type 0xC → 0x6E.
        let valid = |x: u8| x != 0 && !(0x53..=0x55).contains(&x);
        let u10 = if resume != 0 { resume } else { self.state };
        let mut p3 = if self.kind == 0xc { 0x6e } else { resume };
        if valid(u10) { p3 = u10; }
        if !self.bullet_impact(d, rng, frame, p3, allow) {
            if valid(u10) { self.impact_resume = u10; }
            self.set_state(state::BULLET_IMPACT);
        }
        true
    }

    /// `NDrone2_BulletImpact` (0x1461c8): damage; dead → Death_Anim; alive with `allow` and no flinch for 2 s (this
    /// drone's and the global stamp) → `LocationImpactAnim` and false (the caller enters BulletImpact); else the resume
    /// state when it is another one.
    fn bullet_impact(&mut self, d: &Data, rng: &mut Rng, frame: u32, p3: u8, allow: bool) -> bool {
        let valid = |x: u8| x != 0 && !(0x53..=0x55).contains(&x);
        let u8_ = if p3 != 0 { p3 } else { self.state };
        let p3 = if p3 == self.state { 0 } else { p3 };
        self.do_hit_effects(d);
        if self.health <= 0.0 { self.set_state(state::DEATH_ANIM); return true; }
        if allow {
            self.big_hit = self.last_damage > self.max_health * 0.25 && self.health > self.max_health * 0.333;
            // The game's frame counter runs from boot (108153 at a 07000005 start [L]), so unset stamps are long past.
            if self.flinch_at.is_none_or(|f| frame >= f + 120) && self.global_flinch.is_none_or(|f| frame >= f + 120) {
                if valid(u8_) { self.impact_resume = u8_; }
                self.location_impact_anim(d, rng);
                self.global_flinch = Some(frame);
                self.flinch_at = Some(frame);
                return false;
            }
        }
        if valid(p3) { self.set_state(p3); }
        true
    }

    /// `NDrone2_DoHitEffects` → `NDrone2_HitDamage` with HIT+8 · `+0x14C` (the hit sounds 0x601 / 0x413 weigh 0).
    fn do_hit_effects(&mut self, d: &Data) {
        let Some(h) = self.hit else { return };
        self.hits_taken += 1;
        let target = crate::damage::Target { damageable: self.damageable, armour: 0, mode: 0, skin: self.skin, kind: self.kind };
        let r = crate::damage::hit_damage(d.tuning, target, nf_formats::ps2float::mul(h.damage, self.damage_taken), h.part, h.weapon, false, DIFFICULTY, d.level);
        if r.head { self.silent_kill = true; }
        self.health -= r.loss;
        self.last_damage = r.recorded;
    }

    /// `DroneAnim_LocationImpactAnim` (0x13c8b8): the flinch 0x4C, variant 5 + Rand(8), or by the part hit when Bond
    /// aims; then the state `+0x118`.
    fn location_impact_anim(&mut self, d: &Data, rng: &mut Rng) {
        let v = if !self.bond_aiming { 5 + rng.rand(8) } else {
            match self.hit.map_or(0, |h| h.part) { 4 | 5 => 5, 0x12..=0x15 => 8, 0x1e | 0x1f | 0x20 | 0x23 => 9, 0x31..=0x33 => 13, 0x35..=0x37 => 12, _ => 6 + rng.rand(2) }
        };
        self.call_anim_variant(d, anim_state::IMPACT, v as u16, self.impact_resume, 0);
    }

    /// `DroneAnim_LocationDeathAnim` (0x13c780): shot in the head or chest while Bond aims → 0x42 (variant 1 when the
    /// bullet travels the way the drone faces); otherwise 0x40, variant Rand(4) when the playing danim's `info+0x10`
    /// is set. Then `next`.
    fn location_death_anim(&mut self, d: &Data, rng: &mut Rng, next: u8) {
        if let Some(h) = self.hit.filter(|h| self.bond_aiming && matches!(h.part, 4 | 5)) {
            let a = atan2_approx(h.ray[0], h.ray[2]);
            let v = if angle_difference(a, self.yaw).abs() <= crate::trig::HALF_PI { 1 } else { 0 };
            self.call_anim_variant(d, anim_state::DEATH_SHOT, v, next, 0);
            return;
        }
        self.silent_kill = false;
        let info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
        let v = if info[0x10] != 0 { rng.rand(4) as u16 } else { 0 };
        self.call_anim_variant(d, anim_state::DEATH, v, next, 0);
    }

    /// `DroneFunc_SetDeathChannel` (0x13e660): raise `+0x13C` (0x63 = mission-fail reasons, not modelled).
    fn set_death_channel(&mut self) {
        if self.death_channel != 0 && self.death_channel != 0x63 { self.raise = Some(self.death_channel); }
    }

    /// Death_Anim 0x44, SpecialDeath 0x46, Dead 0x47, Fade 0x48, FadeFast 0x49 (0x1637f8, 0x163fe8, 0x172cd8, 0x172dd8).
    fn handle_death(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8, m: u8) -> Option<bool> {
        use state::*;
        if !matches!(s, DEATH_ANIM | SPECIAL_DEATH | DEAD | FADE | FADE_FAST) { return None; }
        Some(match (s, m) {
            (_, 0) | (_, 10) => true,
            (DEATH_ANIM, msg::ENTER) => {
                self.corpse = true;
                self.vision_on = false;
                self.firing = false;
                if self.special_death != 0 { self.set_state(SPECIAL_DEATH); return Some(true); }
                self.death_pending = true; // `OnInitDeath` (death talk, mission-fail props: not modelled)
                self.drop_grenades(d);
                self.location_death_anim(d, rng, DEAD);
                true
            }
            // SpecialDeath 0x46 (0x163c68): at its spot (within 1.0 in 2D, facing within 90° of the spot's yaw; script
            // 0x0600021F also needs Bond 2 units away or more) the drone raises its death channel at once and plays the
            // script as anim state 0x40, sliding to the spot (rate 0.05), follow-on 0xBF, then Dead; anywhere else the
            // script is dropped (0x0600021F also drops the death channel) and the normal death plays.
            (SPECIAL_DEATH, msg::ENTER) => {
                self.corpse = true;
                self.vision_on = false;
                self.firing = false;
                let near = ((self.pos[0] - self.special_spot[0]).powi(2) + (self.pos[2] - self.special_spot[2]).powi(2)).sqrt() < 1.0;
                let facing = angle_difference(self.special_yaw, self.yaw).abs() < crate::trig::HALF_PI;
                let bond_ok = self.special_death != 0x0600_021f || self.dist >= 2.0;
                if bond_ok && near && facing {
                    self.set_death_channel();
                    self.death_pending = false;
                    // The object is the hip from here (feet + `Drone+0xA0`; the port's position is otherwise feet + 1).
                    let hip = self.root_value(d).map_or(0.0, |r| r[1]);
                    self.pos[1] += crate::damage::drone_height(hip) - 1.0;
                    self.free_root = true;
                    self.bounded = false;
                    self.play_script(self.special_death, anim_state::DEATH, 0xbf, DEAD, msg::ANIM_DONE);
                    self.timer_c = Some(frame + 60);
                    return Some(true);
                }
                if self.special_death == 0x0600_021f { self.death_channel = 0; }
                self.special_death = 0;
                self.set_state(DEATH_ANIM);
                true
            }
            (SPECIAL_DEATH, msg::UPDATE) => {
                let to = self.special_spot;
                if self.free_root { self.slide_free(to, 0.05); } else { self.slide_to(d, to, 0.05); }
                if !self.firing || matches!(self.anim_set, 4 | 9) { self.drop_weapon(d, rng); }
                true
            }
            (SPECIAL_DEATH, msg::ANIM_DONE) => { self.set_state(DEAD); true }
            (DEATH_ANIM, msg::EXIT) => { if self.death_pending { self.set_death_channel(); } true }
            (DEATH_ANIM, msg::TIMER_C) => { if self.death_pending { self.set_death_channel(); self.death_pending = false; } true }
            (SPECIAL_DEATH, msg::TIMER_C) => { self.death_pending = true; true }
            // Update: the gun drops while the firing flag (`+0x3B`) is clear or the anim set is 4 / 9.
            (DEATH_ANIM, msg::UPDATE) => {
                if !self.firing || matches!(self.anim_set, 4 | 9) { self.drop_weapon(d, rng); }
                true
            }
            (DEAD, msg::ENTER) => {
                self.drop_weapon(d, rng); // `NDrone2_SetAsDead`
                self.corpse = true;
                self.health = 0.0; // `NDrone2_SetAsDead`
                self.vision_on = false;
                self.firing = false;
                self.timer_c = Some(frame + 300); // `DAT_00298F58` = 10 · FRAME_RATE_INT
                // Prop 0x31 and no silent kill: `DroneAlertToObject(msg 0x15, corpse)` — not modelled (G-026).
                true
            }
            (DEAD, msg::UPDATE) => {
                if !self.drawn { self.present = false; self.enabled = false; return Some(true); }
                if self.cull { self.set_state(FADE_FAST); }
                true
            }
            (DEAD, msg::TIMER_C) => { self.set_state(FADE); true }
            (FADE | FADE_FAST, msg::ENTER) => { self.corpse = true; true }
            (FADE | FADE_FAST, msg::UPDATE) => {
                // `NDrone2_FadeOut`: 0.6 per tick (8 in FadeFast); gone one tick after reaching 0, or when not drawn.
                let gone = !self.drawn || {
                    if self.alpha > 0.0 { self.alpha = (self.alpha - if s == FADE_FAST { 8.0 } else { 0.6 }).max(0.0); false } else { true }
                };
                if gone { self.present = false; self.enabled = false; return Some(true); }
                if s == FADE && self.cull { self.set_state(FADE_FAST); }
                true
            }
            (_, msg::EXIT) => true,
            _ => false,
        })
    }

    /// The bone matrices in world space at the playing script frame (root translation removed; object at the drone's
    /// x/z and yaw, y = feet + hip height − 0.02 as nf-view draws it). *Deviation*: the upper-body aim
    /// (`AnimObjectAimAt`) is not applied.
    pub fn palette(&self, d: &Data) -> Option<Vec<nf_formats::anim::Mat4>> {
        let (q, f) = self.script_frame(d)?;
        let seq = d.sequences.get(&q)?;
        let skin = d.skins.get(&self.skin)?;
        let skel = d.skeletons.get(&skin.skeleton)?;
        let mut local = nf_formats::anim::local_pose(seq, skel, skin, f.clamp(1, seq.frames.max(1) as u32));
        let hip = local[0].0[1];
        let obj = nf_formats::placement::object_matrix([0.0, self.yaw, 0.0], [self.pos[0], self.draw_y(hip), self.pos[2]]);
        Some(crate::damage::palette(&mut local, skin, &obj))
    }

    /// `DroneWeap_DropWeapon` (0x17a2a0, docs/systems/pickups.md): once — the weapon byte (`anim+0x62`) is cleared. At
    /// the held gun's world matrix (its datum on the posed skeleton; the drone's position when the ray from there to
    /// the gun hits geometry), the base weapon with (clip · (0.5 or 0.25 + FRand(0.5))) rounds; weapon 0x1A also drops
    /// its grenade launcher's rounds (0x1B, ammo, hidden).
    fn drop_weapon(&mut self, d: &Data, rng: &mut Rng) {
        use nf_formats::ps2float::{add, mul};
        let w = self.weapon as u8;
        if (w as i8) < 1 { return; }
        let w = w as usize;
        if d.weapons.model(w) == 0 { self.weapon = 0; return; }
        let mut m = self.weapon_matrix(d);
        self.weapon = 0;
        if d.world.segment(self.pos, [m[3][0], m[3][1], m[3][2]]).is_some() { m[3] = [self.pos[0], self.pos[1], self.pos[2], 1.0]; }
        let n = d.weapons.clip_size(w);
        let f = add(rng.frand(0.5), if n < 10 { 0.5 } else { 0.25 });
        let base = d.weapons.base(w);
        let rounds = mul(n as f32, f) as i32 as i16;
        self.drops.extend(crate::pickup::Pickup::create_simple(&m, base, rounds, crate::pickup::kind::WEAPON, d.weapons, d.models));
        if base == 0x1a {
            let f = add(rng.frand(0.5), 0.25);
            let rounds = mul(d.weapons.clip_size(0x1b) as f32, f) as i32 as i16;
            if let Some(mut p) = crate::pickup::Pickup::create_simple(&m, 0x1b, rounds, crate::pickup::kind::AMMO, d.weapons, d.models) {
                p.hidden = true;
                self.drops.push(p);
            }
        }
    }

    /// `AnimGetBoneWorldTrans(obj, datum, 0, …)` for the held weapon's datum (`+0x50`, 0xFF → 0): datum · bone world;
    /// the drone's object matrix when it has no pose or no such datum.
    fn weapon_matrix(&self, d: &Data) -> nf_formats::anim::Mat4 {
        let obj = nf_formats::placement::object_matrix([0.0, self.yaw, 0.0], self.pos);
        let datum = d.weapons.third_person(self.weapon as u8).map_or(0, |(_, id)| if id == 0xff { 0 } else { id as u32 });
        let (Some(pal), Some(skin)) = (self.palette(d), d.skins.get(&self.skin)) else { return obj };
        let Some(dt) = skin.datums.iter().find(|x| x.id == datum && x.bone >= 0) else { return obj };
        let Some(bone) = pal.get(dt.bone as usize) else { return obj };
        nf_formats::anim::mul(&nf_formats::anim::quat_trans_to_mat(dt.q, dt.t), bone)
    }

    /// `DroneFunc_OnInitDeath` → `DroneFunc_DropGrenade`: a captain drops grenades at its own matrix — weapon 0x35 with
    /// 2 on 07000001–08, 0x34 with 1 on 07000009–0B, 11–16, 4A.
    fn drop_grenades(&mut self, d: &Data) {
        if !self.captain { return; }
        let (id, n) = match d.level {
            0x0700_0001..=0x0700_0008 => (0x35, 2),
            0x0700_0009..=0x0700_000b | 0x0700_0011..=0x0700_0016 | 0x0700_004a => (0x34, 1),
            _ => return,
        };
        let m = nf_formats::placement::object_matrix([0.0, self.yaw, 0.0], self.pos);
        self.drops.extend(crate::pickup::Pickup::create_simple(&m, id, n, crate::pickup::kind::WEAPON, d.weapons, d.models));
    }

    /// The object's height as drawn (the hip): feet + `Drone+0xA0` from the root height, or the free object itself.
    pub fn draw_y(&self, hip: f32) -> f32 {
        if self.free_root { self.pos[1] } else { self.pos[1] - 1.0 + crate::damage::drone_height(hip) }
    }

    /// `AnimGetCollData`: the drone's hit boxes now.
    pub fn hit_boxes(&self, d: &Data) -> Vec<crate::damage::HitBox> {
        let (Some(pal), Some(skin)) = (self.palette(d), d.skins.get(&self.skin)) else { return Vec::new() };
        crate::damage::boxes(d.hitboxes.for_skin(self.skin), &pal, skin)
    }

    // ---- hearing (docs/systems/drone-alerts.md) -------------------------------------------------------------------

    /// `DroneVision_EnemyAlerts(msg 0x14)` (0x177c18) → `DroneVision_AlertSound` (0x177800): saves the return state and
    /// home the first time, then HeardNoise when the sound's share > 0.2 or alertness > 0.66. Ignored without prop 0x1F,
    /// more than 50 units from the opponent, or when already attacking.
    fn enemy_alerts(&mut self, d: &Data, s: u8) -> Option<u8> {
        if !self.investigating {
            self.resume_state = s;
            self.home = self.feet();
        }
        if !self.prop(d, 0x1f) || self.dist > 50.0 || self.alert_bits & 0x10 != 0 { return None; }
        (self.sound_share > 0.2 || self.alert > 0.66).then_some(state::HEARD_NOISE)
    }

    /// SniperIdle's alert messages through `DroneVision_EnemyAlerts`: a sound as for any guard, the alarm → Attack.
    fn sniper_alerts(&mut self, d: &Data, s: u8, m: u8) -> Option<u8> {
        if m == msg::ALARM {
            if !self.investigating { self.resume_state = s; self.home = self.feet(); }
            return Some(state::ATTACK);
        }
        self.enemy_alerts(d, s)
    }

    /// AlertToPosition's move (`MoveToAlertPosition`): run when the route is 10 or longer, else the alert walk;
    /// arrived → the return state (SearchArea after Suspect); 6 → NoRoute; failure → back to `+0x5A0`.
    fn alert_to_position_update(&mut self, d: &Data) {
        match self.move_to_goal(d) {
            0..=2 => {
                let far = self.croute.as_ref().is_some_and(|r| r.remaining >= 10.0);
                if far { self.call_anim(d, anim_state::RUN, 0, 0) }
                else if !matches!(self.anim.state, anim_state::ALERT_WALK | anim_state::WALK_AIM) { self.call_anim(d, anim_state::ALERT_WALK, 0, 0) }
            }
            3 => { let r = if self.return_state != 0 { self.return_state } else { state::SEARCH_AREA }; self.set_state(r) }
            6 => self.set_state(state::COMBAT_NO_ROUTE),
            10 => {}
            _ => { self.investigating = false; let r = if self.resume_state != 0 { self.resume_state } else { self.start_state(d) }; self.set_state(r) }
        }
    }

    // ---- cover (docs/systems/drone-cover.md) ---------------------------------------------------------------------

    /// The anim predicates of `NDrone2_CoverNodeOK` / `DroneAnim_CoverAnim` for a corner node: which cover types this
    /// drone can use there (crouch lean, crouch step, stand lean, stand step) and whether crouching is forced.
    fn cover_types(&self, d: &Data, n: &crate::cover::CoverNode) -> ([bool; 4], bool) {
        let p = |id| self.prop(d, id);
        let lean = p(0xa) || p(0xb) || p(0xd);
        let crouch_ok = (p(0x10) || p(0xf)) && n.flags & 4 != 0;
        let stand_ok = n.flags & 8 != 0;
        let c = n.is_corner();
        ([crouch_ok && lean && c && n.flags & 0x40 != 0 && self.can_do(d, 0x13),
          crouch_ok && p(0xc) && c && n.flags & 0x20 != 0 && self.can_do(d, 0x15),
          lean && stand_ok && c && n.flags & 0x40 != 0 && self.can_do(d, 0x17),
          p(0xc) && stand_ok && c && n.flags & 0x20 != 0 && self.can_do(d, 0x1b)],
         p(0x10) || (c && n.flags & 8 == 0))
    }

    /// `CanUseCornerAsLowCover`: a low node or a crouch-only corner.
    fn as_low(n: &crate::cover::CoverNode) -> bool { !n.is_corner() || n.flags & 0xc == 4 }

    /// `NDrone2_CoverNodeOK(node, side, needLos)` (0x151d68): the side (1, 2, 3) or 0.
    fn cover_node_ok(&self, d: &Data, ni: usize, side: u8, need_los: bool) -> u8 {
        let (Some(n), Some(rt)) = (d.covers.get(ni), self.cover_rt.get(ni)) else { return 0 };
        let bits = rt.bits;
        if bits & 0xe == 0 || (need_los && bits & 0x70 == 0) { return 0; }
        let t = self.target.pos;
        let dist = ((t[0] - n.pos[0]).powi(2) + (t[1] - n.pos[1]).powi(2) + (t[2] - n.pos[2]).powi(2)).sqrt();
        if dist < 2.0 || (self.prop(d, 0x44) && dist > 1.5 * self.combat_range) { return 0; }
        let out = if n.is_corner() {
            let (types, forced) = self.cover_types(d, n);
            let any = if forced { types[0] || types[1] } else { types.iter().any(|&x| x) };
            if !(any || Self::as_low(n)) { return 0; }
            match side {
                1 => if bits & 2 != 0 { 1 } else { 0 },
                2 => if bits & 4 != 0 { 2 } else { 0 },
                _ => if bits & 2 != 0 { 1 } else if bits & 4 != 0 { 2 } else { 0 },
            }
        } else if self.prop(d, 0xf) && bits & 8 != 0 { 3 } else { 0 };
        if out == 0 { return 0; }
        if !self.has_cover {
            let crowded = self.others.iter().enumerate().any(|(j, p)| j != self.index && p.is_some_and(|p| {
                ((p[0] - n.pos[0]).powi(2) + (p[1] - n.pos[1]).powi(2) + (p[2] - n.pos[2]).powi(2)).sqrt() < 1.5
            }));
            if crowded { return 0; }
        }
        out
    }

    /// `NDrone2_DisownCover`.
    fn disown_cover(&mut self) {
        if let Some(rt) = self.cover.and_then(|c| self.cover_rt.get_mut(c)) { if rt.owner == Some(self.index) { rt.owner = None } }
        self.cover = None;
        self.cover_claimed = false;
        self.has_cover = false;
    }

    /// `NDrone2_FindCover(needLos)` (0x152548): the unoccupied node on the drone's network with the smallest path
    /// distance from its nav node (no farther than the opponent), usable from the drone's side; claims it.
    fn find_cover(&mut self, d: &Data, need_los: bool) -> bool {
        let feet = self.feet();
        let Some(path) = *self.nav_path.get_or_insert_with(|| d.nav.nav_path_for(feet)) else { return false };
        let Some(from) = d.nav.node_from(d.world, path, feet) else { return false };
        let mut best: Option<(u8, usize, u8)> = None;
        for (i, n) in d.covers.iter().enumerate() {
            if n.path != Some(path) || self.cover_rt.get(i).is_none_or(|r| r.owner.is_some()) { continue; }
            let dd = n.dist.get(from as usize).copied().unwrap_or(0xff);
            if dd == 0xff || best.is_some_and(|b| dd >= b.0) || (dd as f32).powi(2) > self.dist * self.dist { continue; }
            let side = self.cover_node_ok(d, i, 0, need_los);
            if side != 0 { best = Some((dd, i, side)); }
        }
        let Some((_, i, side)) = best else { return false };
        self.disown_cover();
        let n = &d.covers[i];
        let bits = self.cover_rt[i].bits;
        let Some((goal, _)) = n.position(bits, true) else { return false };
        self.cover = Some(i);
        self.croute = None;
        self.goal = Some(goal);
        self.cover_rt[i].owner = Some(self.index);
        self.cover_side = side;
        self.cover_point = n.position(bits, false).map_or(goal, |p| p.0);
        self.cover_claimed = true;
        self.has_cover = true;
        true
    }

    /// `NDrone2_CoverAvailable(needLos)` (0x155a38).
    fn cover_available(&mut self, d: &Data, need_los: bool) -> bool {
        if !self.prop(d, 0x30) { return false; }
        if self.cover_claimed { return true; }
        if self.seen && self.dist < 4.0 { return false; }
        self.find_cover(d, need_los)
    }

    /// `NDrone2_MoveToGoalPosition(goal)` (0x1517d8): a route to the goal point (target nodes searched with r = 0),
    /// kept until invalidated; 3 within 2.0 of the route's end, else follow (0xC blocked, else 0).
    fn move_to_goal(&mut self, d: &Data) -> u8 {
        let Some(goal) = self.goal else { return 8 };
        let feet = self.feet();
        let path = *self.nav_path.get_or_insert_with(|| d.nav.nav_path_for(feet));
        if !self.croute.as_ref().is_some_and(|r| r.stamp == u32::MAX) {
            let nodes = path.map(|p| d.nav.target_nodes(d.world, p, goal, 0.0)).unwrap_or_default();
            let used: &[u16] = path.and_then(|p| self.links_used.get(p)).map_or(&[], |v| v.as_slice());
            match crate::nav::Route::to_position(d.nav, d.world, path, feet, goal, &nodes, used) {
                Ok((_, mut r)) => { r.stamp = u32::MAX; self.croute = Some(r) }
                Err(code) => { self.croute = None; return code }
            }
        }
        let route = self.croute.as_mut().unwrap();
        route.remaining = route.distance(d.nav, feet);
        if route.remaining < 2.0 { return 3; }
        let (code, _) = route.follow(d.nav, feet);
        if let Some(used) = self.links_used.get_mut(route.path) { route.mark_used(used) }
        let t = route.target;
        self.desired_yaw = atan2_approx(t[0] - self.pos[0], t[2] - self.pos[2]);
        match code { 3 | 0xc => code, _ => 0 }
    }

    /// `NDrone2_SetGoalToCloserPosition` (0x155e10): where the straight line to the opponent first meets an AI-network
    /// wall, if a route there exists and the drone is not already within 2.0 of it.
    fn set_goal_to_closer_position(&mut self, d: &Data) -> bool {
        let Some(p) = d.nav.first_wall_hit(self.pos, self.target.pos) else { return false };
        let (old_goal, old_route) = (self.goal, self.croute.take());
        self.goal = Some(p);
        if self.move_to_goal(d) < 3 { return true; }
        self.goal = old_goal;
        self.croute = old_route;
        false
    }

    /// `NDrone2_SetGoalToDroneWithLosToOpponent` (0x1543f8): the first other drone (list order) that sees the opponent
    /// and wins Rand(4) == 0 and can be routed to (`MoveToGoalPosition` 0 or 2).
    fn set_goal_to_drone_with_los(&mut self, d: &Data, rng: &mut Rng) -> bool {
        for (i, pos, sees) in self.peers.clone() {
            if i == self.index || !sees || rng.rand(4) != 0 { continue; }
            let (old_goal, old_route) = (self.goal, self.croute.take());
            self.goal = Some(pos);
            if matches!(self.move_to_goal(d), 0 | 2) { return true; }
            self.goal = old_goal;
            self.croute = old_route;
        }
        false
    }

    /// `NDrone2_SlideToPosition(target, rate, y = false)`: x/z toward the point (snapping within 0.01), then the
    /// drone's height rule puts it back on the floor.
    /// `NDrone2_SlideToPosition(to, rate, 1)`: in 3D, no floor (the special death).
    fn slide_free(&mut self, to: V3, rate: f32) {
        let dist = ((to[0] - self.pos[0]).powi(2) + (to[1] - self.pos[1]).powi(2) + (to[2] - self.pos[2]).powi(2)).sqrt();
        if dist > 0.01 { for k in 0..3 { self.pos[k] += (to[k] - self.pos[k]) * rate; } } else { self.pos = to; }
    }

    fn slide_to(&mut self, d: &Data, to: V3, rate: f32) {
        let dist = ((to[0] - self.pos[0]).powi(2) + (to[2] - self.pos[2]).powi(2)).sqrt();
        if dist > 0.01 {
            self.pos[0] += (to[0] - self.pos[0]) * rate;
            self.pos[2] += (to[2] - self.pos[2]) * rate;
        } else {
            self.pos[0] = to[0];
            self.pos[2] = to[2];
        }
        // Stay on the floor without climbing: only surfaces within 0.3 of the current feet count (the game's
        // collision pushes the feet sphere; walls next to cover points have sloped faces our probe could climb).
        if let Some(f) = d.world.floor_below([self.pos[0], self.pos[1] - 0.7, self.pos[2]], 0.6) { self.pos[1] = f + 1.0; }
    }

    /// `NDrone2_SetAngleToCover`: low node or as-low cover → the opponent; side 1 → node yaw − π/2; side 2 → node yaw.
    fn set_angle_to_cover(&mut self, d: &Data) {
        let Some(n) = self.cover.and_then(|c| d.covers.get(c)) else { return };
        self.desired_yaw = if !n.is_corner() || self.cover_type == crate::cover::kind::AS_LOW { self.yaw_to_opp }
            else if self.cover_side == 1 { n.yaw - crate::trig::HALF_PI } else { n.yaw };
    }

    /// `DroneAnim_CoverAnim(mode, next state)` (0x13c078): picks the cover type (mode 6, or 7 = maybe change) and plays
    /// the `CCModes` column; false when no anim fits.
    fn cover_anim(&mut self, d: &Data, rng: &mut Rng, mode: u8, next: u8) -> bool {
        use crate::cover::{cc_anim, kind};
        let Some(n) = self.cover.and_then(|c| d.covers.get(c)).cloned() else { return false };
        if !n.is_corner() {
            let anim = match mode { 0 | 6 | 1 | 4 => n.idle_anim, 2 => n.aim_anim, 3 => 0x58, _ => return false };
            let anim = if self.can_do(d, anim) { anim } else if anim == 5 { if self.can_do(d, 7) { 7 } else { 0xa } } else { anim };
            self.call_anim(d, anim, next, 0);
            return true;
        }
        let (types, forced) = self.cover_types(d, &n);
        let column = match mode {
            6 => {
                self.cover_leave = 0;
                let c: Vec<u8> = if forced { [kind::CROUCH_LEAN, kind::CROUCH_STEP].into_iter().filter(|&t| types[t as usize]).collect() }
                    else { [kind::STAND_LEAN, kind::STAND_STEP].into_iter().filter(|&t| types[t as usize]).collect() };
                self.cover_type = if c.is_empty() { if Self::as_low(&n) { kind::AS_LOW } else { return false } } else { c[rng.rand(c.len() as u32) as usize] };
                0
            }
            7 => {
                if rng.rand(2) != 0 { return false; }
                let mut c: Vec<u8> = if forced { [kind::CROUCH_LEAN, kind::CROUCH_STEP].into_iter().filter(|&t| types[t as usize]).collect() }
                    else { [kind::STAND_LEAN, kind::STAND_STEP].into_iter().filter(|&t| types[t as usize]).collect() };
                if !forced && self.dist > 8.0 { c.extend([kind::CROUCH_LEAN, kind::CROUCH_STEP].into_iter().filter(|&t| types[t as usize])); }
                let new = if c.is_empty() { kind::AS_LOW } else { c[rng.rand(c.len() as u32) as usize] };
                if new == self.cover_type { return false; }
                let anim = cc_anim(self.cover_type, self.cover_side, 5);
                self.cover_type = new;
                if anim >= 0x77 || !self.can_do(d, anim) { return false; }
                self.call_anim(d, anim, next, 0);
                return true;
            }
            m => m as usize,
        };
        let anim = cc_anim(self.cover_type, self.cover_side, column);
        if anim >= 0x77 || !self.can_do(d, anim) { return false; }
        self.call_anim(d, anim, next, 0);
        true
    }

    /// The cover states (0x8C, 0x93–0x9C); `None` when the state is not a cover state.
    fn handle_cover(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8, m: u8) -> Option<bool> {
        use state::*;
        let in_cover_ok = |me: &Self, los: bool| me.cover.is_some_and(|c| me.cover_node_ok(d, c, me.cover_side, los) != 0);
        let far_from_goal = |me: &Self| me.goal.is_none_or(|g| ((g[0] - me.pos[0]).powi(2) + (g[2] - me.pos[2]).powi(2)).sqrt() > 1.5);
        Some(match (s, m) {
            (RUN_FOR_COVER, msg::ENTER) => { self.vision_on = true; if self.goal.is_none() { self.set_state(ATTACK) } true }
            (RUN_FOR_COVER, msg::UPDATE) => {
                if !in_cover_ok(self, false) || (self.seen && self.dist < 4.0) { self.set_state(ATTACK); return Some(true); }
                match self.move_to_goal(d) {
                    0 | 2 => {
                        let remaining = self.croute.as_ref().map_or(0.0, |r| r.remaining);
                        if remaining < 1.0 { let p = self.cover_point; self.slide_to(d, p, 0.1); }
                        self.set_combat_move_anim(d, rng, remaining);
                    }
                    3 => self.set_state(UNDER_COVER_INIT),
                    10 => {}
                    0xc => self.face_and_aim(d),
                    _ => self.set_state(ATTACK),
                }
                true
            }
            (RUN_FOR_COVER, msg::EXIT) => {
                if self.exiting_to != UNDER_COVER_INIT { self.disown_cover() }
                self.cover_claimed = false;
                true
            }
            (UNDER_COVER_INIT, msg::ENTER) => {
                self.vision_on = true;
                self.cover_claimed = false;
                if !self.cover_anim(d, rng, 6, UNDER_COVER_IDLE) { /* as built: the drone then waits here */ }
                true
            }
            (UNDER_COVER_INIT, msg::UPDATE) => { let p = self.cover_point; self.slide_to(d, p, 0.08); self.set_angle_to_cover(d); true }
            (UNDER_COVER_INIT, msg::EXIT) => { if self.exiting_to != UNDER_COVER_IDLE { self.disown_cover() } true }
            (UNDER_COVER_IDLE, msg::ENTER) => {
                self.vision_on = true;
                self.cover_anim(d, rng, 1, 0);
                self.clip = self.clip_max;
                if !self.prop(d, 0x10) { self.timer_c = Some(frame + rng.frand(150.0) as u32); }
                self.cover_tries = 0;
                true
            }
            (UNDER_COVER_IDLE, msg::UPDATE) => {
                if self.prop(d, 0x10) {
                    let bond = self.target.pos;
                    if self.seen || ((bond[0] - self.pos[0]).powi(2) + (bond[2] - self.pos[2]).powi(2)).sqrt() < 3.0 {
                        d.props.set(0x10, &mut self.props, 0);
                        self.set_state(UNDER_COVER_LEAVE_NOW);
                    }
                    return Some(true);
                }
                let p = self.cover_point;
                self.slide_to(d, p, 0.08);
                self.set_angle_to_cover(d);
                if !in_cover_ok(self, false) || far_from_goal(self) { self.set_state(UNDER_COVER_LEAVE_NOW) }
                true
            }
            (UNDER_COVER_IDLE, msg::TIMER_C) => {
                if rng.rand(5) == 0 && self.cover_available(d, true) { self.set_state(RUN_FOR_COVER) }
                else if in_cover_ok(self, true) { self.set_state(UNDER_COVER_AIM) }
                else if self.cover_available(d, true) { self.set_state(RUN_FOR_COVER) }
                else {
                    self.cover_tries += 1;
                    if self.cover_tries < 3 { self.timer_c = Some(frame + rng.frand(150.0) as u32) } else { self.set_state(ATTACK) }
                }
                true
            }
            (UNDER_COVER_IDLE, msg::EXIT) => { d.props.set(0x10, &mut self.props, 0); true }
            (UNDER_COVER_AIM, msg::ENTER) => {
                if !self.cover_anim(d, rng, 2, UNDER_COVER_FIRE) { self.set_state(UNDER_COVER_LEAVE_NOW) }
                true
            }
            (UNDER_COVER_FIRE, msg::ENTER) => { self.fire(d, rng, frame, true); true }
            (UNDER_COVER_FIRE, msg::UPDATE) => {
                if !in_cover_ok(self, true) {
                    let leans = self.cover.and_then(|c| d.covers.get(c)).is_some_and(|n| n.is_corner())
                        && matches!(self.cover_type, crate::cover::kind::CROUCH_LEAN | crate::cover::kind::STAND_LEAN);
                    self.set_state(if leans { UNDER_COVER_RETURN } else { UNDER_COVER_LEAVE_NOW });
                } else if self.aimed_at && rng.rand(3) == 0 {
                    self.set_state(UNDER_COVER_RETURN);
                } else if self.seen && self.burst_done {
                    self.fire(d, rng, frame, true);
                }
                true
            }
            (UNDER_COVER_RETURN, msg::ENTER) => { if !self.cover_anim(d, rng, 4, UNDER_COVER_TYPE_CHANGE) { self.set_state(UNDER_COVER_LEAVE) } true }
            (UNDER_COVER_RETURN, msg::UPDATE) => { if !in_cover_ok(self, false) || far_from_goal(self) { self.cover_leave |= 4 } true }
            (UNDER_COVER_TYPE_CHANGE, msg::ENTER) => {
                if self.cover_leave & 4 != 0 { self.set_state(UNDER_COVER_LEAVE_NOW) }
                else if !in_cover_ok(self, true) && self.cover_available(d, true) { self.set_state(RUN_FOR_COVER) }
                else if !self.cover_anim(d, rng, 7, UNDER_COVER_IDLE) { self.set_state(UNDER_COVER_IDLE) }
                true
            }
            (UNDER_COVER_LEAVE, msg::ENTER) => { self.set_state(UNDER_COVER_LEAVE_NOW); true }
            (UNDER_COVER_LEAVE_NOW, msg::ENTER) => {
                self.cover_leave |= 8;
                self.disown_cover();
                self.call_anim(d, anim_state::AIM_STAND, ATTACK, 0);
                true
            }
            (UNDER_COVER_LEAVE_NOW, msg::EXIT) => { self.disown_cover(); true }
            (c, msg::ENTER | msg::EXIT | msg::UPDATE) if c == RUN_FOR_COVER || (UNDER_COVER_INIT..=UNDER_COVER_LEAVE_NOW).contains(&c) => true,
            _ => return None,
        })
    }

    // ---- state machine -----------------------------------------------------------------------------------------

    fn set_state(&mut self, s: u8) { if s != 0 { self.next_state = Some(s) } }

    /// `Drone_SM_RouteMsgDCV`: deliver, then apply pending changes (exit → enter, repeating).
    fn send(&mut self, d: &Data, rng: &mut Rng, frame: u32, m: u8) {
        if !self.handle(d, rng, frame, self.state, m) { self.handle(d, rng, frame, state::GLOBAL, m); }
        if let Some(m2) = self.self_msg.take() { self.send(d, rng, frame, m2); }
        let mut guard = 0;
        while let Some(n) = self.next_state.take() {
            self.exiting_to = n;
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
            if let Some(m2) = self.self_msg.take() { self.send(d, rng, frame, m2); }
            guard += 1;
            if guard > 16 { break; }
        }
    }

    fn handle(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8, m: u8) -> bool {
        use state::*;
        if m == msg::BULLET { return self.on_bullet(d, rng, frame, s); }
        // BulletImpact 0x55 (0x173098): enter turns vision on; the flinch call ends it (state `+0x118`).
        if s == BULLET_IMPACT && matches!(m, 0 | msg::ENTER | msg::UPDATE) { if m == msg::ENTER { self.vision_on = true; } return true; }
        if let Some(r) = self.handle_death(d, rng, frame, s, m) { return r; }
        if let Some(r) = self.handle_cover(d, rng, frame, s, m) { return r; }
        match (s, m) {
            // `NDrone2_DSTATE_Global` enter: a drone with a switch channel that is still off waits (WaitSwitch); else its
            // start state `+0x5A2` (InitPatrol with props 0x24/0x25, else Idle). Modes 0x1E / 0x17 and start scripts
            // (PlayScript 3) are not ported.
            (GLOBAL, msg::ENTER) => {
                self.return_state = self.start_state(d);
                if self.switch_channel != 0 && !self.channel_on { self.set_state(WAIT_SWITCH); return true; }
                self.set_state(self.start_state(d));
                true
            }
            (WAIT_SWITCH, msg::ENTER) => { self.enabled = false; self.vision_on = false; self.watching = false; true }
            (WAIT_SWITCH, msg::UPDATE) => { if self.channel_on { self.enabled = true; let s = self.start_state(d); self.set_state(s) } true }
            (WAIT_SWITCH, msg::ENABLE) => { self.enabled = true; let s = self.start_state(d); self.set_state(s); true }
            (INIT_PATROL, msg::ENTER) => {
                let patrols = (self.prop(d, 0x24) || self.prop(d, 0x25)) && self.assign_path(d, 4);
                self.set_state(if patrols { PATROL } else { IDLE });
                true
            }
            (IDLE | ALERT | PATROL, msg::ENTER) => {
                self.vision_on = true;
                self.watching = true;
                // Idle / Alert: prop 0x1D → a mission path (`AssignAIPath(8)` → EnemyMission 0x1B, not ported; none in
                // 07000005), else the property is cleared.
                if s != PATROL && self.prop(d, 0x1d) && !self.assign_path(d, 8) { d.props.set(0x1d, &mut self.props, 0); }
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
                    let variant = if anim == anim_state::LOOK_AROUND { rng.rand(3) as u16 } else { 0 };
                    if self.can_do(d, anim) { self.call_anim_variant(d, anim, variant, back, 0) }
                    if s != PATROL { self.set_idle_timeout(rng, frame, 45, 5) }
                }
                true
            }
            // Forced attack (`Drone_Message(0x1F)` → `DroneFunc_DoForcedAttack`): another drone was seen attacking.
            (IDLE | ALERT | PATROL, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            // The alarm through `DroneVision_EnemyAlerts` case 0x1E (0x177c18): save the return state and home the first
            // time, then Attack (`+0x4F8` |= 0x4000000 | 0x40000000; drone modes 9 / 0x12 / 0x1E, not modelled, ignore it).
            (IDLE | ALERT | PATROL | ALERT_TO_POSITION | SEARCH_AREA | HEARD_NOISE_AWARE | HEARD_NOISE_SUSPECT | HEARD_NOISE_ALERT, msg::ALARM) => {
                if !self.investigating { self.resume_state = s; self.home = self.feet(); }
                self.set_state(ATTACK);
                true
            }
            (IDLE | ALERT | PATROL | ALERT_TO_POSITION | SEARCH_AREA | HEARD_NOISE_AWARE, msg::SOUND) => {
                // Aware (0x16af20): a share above 0.1 → Suspect, else nothing.
                if s == HEARD_NOISE_AWARE { if self.sound_share > 0.1 { self.set_state(HEARD_NOISE_SUSPECT) } return true; }
                if matches!(s, ALERT_TO_POSITION | SEARCH_AREA) && self.sound_share > 0.2 { self.set_state(ATTACK); return true; }
                if let Some(n) = self.enemy_alerts(d, s) { self.set_state(n); }
                true
            }
            // Sniper (0x15fab0 / 0x15fc18 / 0x15fd88 / 0x15ff80). SniperIdle: anim 0x0E, stationary, look for Bond.
            (SNIPER_IDLE, msg::ENTER) => { self.vision_on = true; self.stationary = true; self.call_anim(d, 0x0e, 0, 0); true }
            (SNIPER_IDLE, msg::UPDATE) => { if let Some(n) = self.look_for_opponent(d) { self.set_state(n) } true }
            (SNIPER_IDLE, msg::SOUND | msg::ALARM) => { if let Some(n) = self.sniper_alerts(d, s, m) { self.set_state(n) } true }
            (SNIPER_IDLE, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            // SniperAim: anim 0x0A, sight and combat range 1000; Bond in sight (`+0x228 & 4`) → SniperFire.
            (SNIPER_AIM, msg::ENTER) => {
                self.vision_on = true;
                self.stationary = true;
                self.call_anim(d, anim_state::AIM_STAND, 0, 0);
                self.combat_range = 1000.0;
                self.sight_range = 1000.0;
                true
            }
            (SNIPER_AIM, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; if self.seen { self.set_state(SNIPER_FIRE) } true }
            // SniperFire: aim and fire (`DroneWeap_Fire`; the first shot after a 60-tick timer); empty clip → reload;
            // out of sight 16 ticks → SniperAim. (Alternative attacks — `CanAltAttack` — are not modelled.)
            (SNIPER_FIRE, msg::ENTER) => {
                self.stationary = true;
                self.desired_yaw = self.yaw_to_opp;
                self.call_anim(d, anim_state::AIM_STAND, 0, 0);
                self.vision_on = true;
                self.firing = false;
                self.timer_c = Some(frame + 60);
                true
            }
            (SNIPER_FIRE, msg::TIMER_C) => { self.firing = true; true }
            (SNIPER_FIRE, msg::UPDATE) => {
                self.desired_yaw = self.yaw_to_opp;
                if self.clip < 1 { self.set_state(SNIPER_RELOAD); }
                else if self.unseen_ticks >= 0x10 { self.set_state(SNIPER_AIM); }
                true
            }
            (SNIPER_FIRE, msg::EXIT) => { self.firing = false; true }
            // SniperReload: 20 ticks, then the reload anim 0x52 → SniperFire; the clip refills on exit.
            // Surrender_Anim (0x1626a0): drop the gun, unarmed (`ChangeWeapon(1)`: weapon 1, anim set 0xC), hands up (anim
            // 0x5C → Surrendered), turned away from Bond; every 15 ticks the "surrendered" alert bit (`+0x4FC |= 4`) that
            // calm guards seeing him react to (prop 0x4F). The 0x0600021F special death is cancelled.
            (SURRENDER_ANIM, msg::ENTER) => {
                // Order as read: the call (with the armed anim set), the drop, then `ChangeWeapon(1)`.
                self.firing = false;
                self.call_anim(d, anim_state::SURRENDER, SURRENDERED, 0);
                self.drop_weapon(d, rng);
                self.weapon = 1;
                self.anim_set = 0xc;
                self.clip = 0;
                self.timer_c = Some(frame + 15);
                if self.special_death == 0x0600_021f { self.special_death = 0; self.death_channel = 0; }
                true
            }
            (SURRENDER_ANIM, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp + crate::trig::PI; true }
            (SURRENDER_ANIM, msg::TIMER_C) => { self.alert_bits |= 4; self.timer_c = Some(frame + 15); true }
            // Surrendered (0x162950): faces Bond once; hands up while Bond keeps him covered (`CheckUnsurrender`).
            (SURRENDERED, msg::ENTER) => { self.desired_yaw = self.yaw_to_opp; true }
            (SURRENDERED, msg::UPDATE) => { if self.check_unsurrender(d) { self.set_state(UNSURRENDER_ANIM); } true }
            // Unsurrender_Anim (0x162a70): face Bond, aim-stand anim → Attack (which, unarmed, draws the sidearm).
            (UNSURRENDER_ANIM, msg::ENTER) => { self.desired_yaw = self.yaw_to_opp; self.call_anim(d, anim_state::AIM_STAND, ATTACK, 0); true }
            (UNSURRENDER_ANIM, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            // DrawWeapon (0x166000): anim 0x46 → the return state `+0x5A2` (*suggested* InitPatrol when unset, [L] 6 for
            // most 07000005 guards); on exit, still unarmed → `ChangeWeapon(6)`: weapon 6, anim set 4, 16 rounds.
            (DRAW_WEAPON, msg::ENTER) => {
                let next = if self.return_state != 0 { self.return_state } else { INIT_PATROL };
                self.call_anim(d, anim_state::DRAW_WEAPON, next, 0);
                true
            }
            (DRAW_WEAPON, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            (DRAW_WEAPON, msg::EXIT) => { if (self.weapon as u8) <= 1 { self.weapon = 6; self.anim_set = 4; self.clip = 16; } true }
            (SNIPER_RELOAD, msg::ENTER) => { self.vision_on = true; self.stationary = true; self.firing = false; self.timer_c = Some(frame + 20); true }
            (SNIPER_RELOAD, msg::TIMER_C) => { self.call_anim(d, anim_state::RELOAD, SNIPER_FIRE, 0); true }
            (SNIPER_RELOAD, msg::EXIT) => { self.clip = self.clip_max; true }
            (SNIPER_RELOAD, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            // The noise chain (`NDrone2_DSTATE_HeardNoise*`).
            (HEARD_NOISE, msg::ENTER) => { self.investigating = true; self.set_state(HEARD_NOISE_AWARE); true }
            (HEARD_NOISE_AWARE, msg::ENTER) => {
                self.investigating = true;
                if self.noise_level < 2 {
                    self.noise_level = 2;
                    self.vision_on = true;
                    let back = if self.resume_state != 0 { self.resume_state } else { self.start_state(d) };
                    self.call_anim_variant(d, anim_state::ALERT_LOOK, 1, back, 0);
                } else { self.set_state(HEARD_NOISE_SUSPECT); }
                true
            }
            (HEARD_NOISE_AWARE | HEARD_NOISE_SUSPECT | HEARD_NOISE_ALERT | ALERT_TO_POSITION | SEARCH_AREA, msg::UPDATE) => {
                if let Some(n) = self.look_for_opponent(d) { self.set_state(n); return true; }
                if s == ALERT_TO_POSITION { self.alert_to_position_update(d); }
                true
            }
            (HEARD_NOISE_SUSPECT, msg::ENTER) => {
                if self.noise_level >= 3 { self.set_state(HEARD_NOISE_ALERT); return true; }
                self.noise_level = 3;
                self.investigating = true;
                self.vision_on = true;
                let back = if self.resume_state != 0 { self.resume_state } else { self.start_state(d) };
                if self.stays(d) { self.call_anim_variant(d, anim_state::ALERT_LOOK, 1, back, 0); return true; }
                // `SetSoundAlertRoute`: the goal becomes the sound (radius 2.0).
                self.goal = Some(self.sound_pos);
                self.croute = None;
                if self.turned && self.prop(d, 0x21) { self.set_state(ALERT_TO_POSITION); return true; }
                let to = atan2_approx(self.sound_pos[0] - self.pos[0], self.sound_pos[2] - self.pos[2]);
                if angle_difference(self.yaw, to).abs() > 2.356_194_5 {
                    self.call_anim(d, TURN, 0, msg::TURNED);
                } else {
                    self.turned = true;
                    self.desired_yaw = to;
                    if self.prop(d, 0x21) { self.return_state = SEARCH_AREA; self.call_anim(d, anim_state::ALERT_LOOK, ALERT_TO_POSITION, 0); }
                    else { self.call_anim_variant(d, anim_state::ALERT_LOOK, 1, back, 0); }
                }
                true
            }
            // Suspect's turn finished (msg 0x23, 0x16b0e0): already turned with prop 0x21 → AlertToPosition; else face the
            // sound; without prop 0x21 look about (variant 1) and go back; with it SearchArea follows AlertToPosition,
            // reached through the short look (0x0B) when standing in alert idle, else at once.
            (HEARD_NOISE_SUSPECT, msg::TURNED) => {
                if self.turned && self.prop(d, 0x21) { self.set_state(ALERT_TO_POSITION); return true; }
                self.turned = true;
                self.desired_yaw = atan2_approx(self.sound_pos[0] - self.pos[0], self.sound_pos[2] - self.pos[2]);
                let back = if self.resume_state != 0 { self.resume_state } else { self.start_state(d) };
                if !self.prop(d, 0x21) { self.call_anim_variant(d, anim_state::ALERT_LOOK, 1, back, 0); return true; }
                self.return_state = SEARCH_AREA;
                if self.anim.state == anim_state::ALERT_IDLE { self.call_anim(d, LOOK_SHORT, ALERT_TO_POSITION, 0) }
                else { self.set_state(ALERT_TO_POSITION) }
                true
            }
            // Suspect: any further sound → Attack. Alert (0x16b540) has no sound case (the global state's applies).
            (HEARD_NOISE_SUSPECT, msg::SOUND) => { self.set_state(ATTACK); true }
            (HEARD_NOISE_ALERT, msg::ENTER) => {
                self.noise_level = 4;
                self.investigating = true;
                self.vision_on = true;
                if self.stays(d) { self.call_anim_variant(d, anim_state::ALERT_LOOK, 1, ATTACK, 0); return true; }
                self.timer_c = Some(frame + 60);
                if angle_difference(self.yaw, self.yaw_to_opp).abs() <= 2.356_194_5 {
                    self.turned = true;
                    self.desired_yaw = self.yaw_to_opp;
                    self.call_anim(d, LOOK_SHORT, ATTACK, 0);
                } else { self.call_anim(d, TURN, 0, msg::TURNED); }
                true
            }
            (HEARD_NOISE_ALERT, msg::TURNED) => { self.turned = true; self.desired_yaw = self.yaw_to_opp; self.call_anim(d, LOOK_SHORT, ATTACK, 0); true }
            (HEARD_NOISE_ALERT, msg::TIMER_C) => { self.set_state(ATTACK); true }
            // AlertToPosition 0x62: walk / run to the sound.
            (ALERT_TO_POSITION, msg::ENTER) => { self.vision_on = true; self.call_anim(d, anim_state::WALK_AIM, 0, 0); self.timer_c = Some(frame + 300); true }
            (ALERT_TO_POSITION, msg::TIMER_C) => { true }
            // SearchArea 0x65: look around, turn, look, give up and walk home (`+0x5B0`) through GoToGoalPosition.
            (SEARCH_AREA, msg::ENTER) => { self.vision_on = true; self.call_anim(d, anim_state::ALERT_LOOK, 0, 0); self.timer_c = Some(frame + 180); true }
            (SEARCH_AREA, msg::TIMER_C) => { self.call_anim(d, TURN, 0, msg::SEARCH_LOOKED); true }
            (SEARCH_AREA, msg::SEARCH_LOOKED) => { self.call_anim(d, LOOK_SHORT, 0, msg::SEARCH_DONE); true }
            (SEARCH_AREA, msg::SEARCH_DONE) => {
                self.alert = 0.0;
                self.turned = false;
                self.return_state = if self.resume_state != 0 { self.resume_state } else { self.start_state(d) };
                self.goal = Some(self.home);
                self.croute = None;
                self.set_state(GO_TO_GOAL);
                true
            }
            // `DroneFunc_FirstAttack` with an alternate state (`+0x5A4`) and no second switch: take the alternate mode
            // (mode 2 becomes 0x19, sniper alert, whose init keeps the drone stationary) and go to that state.
            (ATTACK, msg::ENTER) if self.alt_state != 0 && self.second_channel == 0 && self.alt_state != ATTACK && self.alt_state != 0xb4 => {
                self.alert = 1.0;
                self.return_state = ATTACK;
                if self.alert_bits & 0x10 == 0 { self.attack_frame = frame; }
                self.alert_bits |= 0x10 | if self.prop(d, 0x33) { 1 } else { 0 };
                self.sight_range = self.sight_range.max(self.combat_range);
                self.half_fov = 0.0;
                if self.alt_mode != 0 { self.mode = self.alt_mode; }
                if self.mode == 2 { self.mode = 0x19; self.stationary = true; }
                let st = self.alt_state;
                self.alt_mode = 0;
                self.alt_state = 0;
                self.set_state(st);
                true
            }
            // `DroneFunc_FirstAttack` (0x148010), once per drone, after its own first sighting: `NDrone2_CheckSurrender` →
            // alertness 1, `+0x4F8 |= 0x6000000`, `+0x524` = now if unset, sight widened — and Surrender_Anim, without
            // the shout (`+0x4FC |= 0x10` is skipped).
            (ATTACK, msg::ENTER) if !self.first_attack_done && self.own_sight && self.check_surrender(d) => {
                self.first_attack_done = true;
                self.alert = 1.0;
                self.second_channel = 0;
                self.second_tag = 0;
                if self.attack_frame == 0 { self.attack_frame = frame; }
                self.sight_range = self.sight_range.max(self.combat_range);
                self.half_fov = 0.0;
                self.set_state(SURRENDER_ANIM);
                true
            }
            // Attack without a gun (weapon byte ≤ 1, after a surrender): `DroneFunc_AttackNoWeapon` (0x14af80) → DrawWeapon
            // (`+0x45` > 1; [L] 10 for every 07000005 guard).
            (ATTACK, msg::ENTER) if (self.weapon as u8) <= 1 => {
                self.first_attack_done = true;
                self.alert = 1.0;
                self.set_state(DRAW_WEAPON);
                true
            }
            (ATTACK, msg::ENTER) => {
                self.first_attack_done = true;
                // DroneFunc_FirstAttack → FirstSightState: alertness 1, combat sight; props 0x1B (50 %) / 0x19 pick a
                // combat move; otherwise AimStand (prop 0x18, in range) or Combat. FirstAttack also clears the second
                // switch and sets the return state to Attack (`+0x5A2` = 0x56, *suggested* for these guards).
                self.alert = 1.0;
                self.second_channel = 0;
                self.return_state = ATTACK;
                // FirstAttack: the attack is visible to others (`+0x4FC |= 0x10`, enemy side), shouts with prop 0x33.
                if self.alert_bits & 0x10 == 0 { self.attack_frame = frame; }
                self.alert_bits |= 0x10 | if self.prop(d, 0x33) { 1 } else { 0 };
                self.sight_range = self.sight_range.max(self.combat_range);
                self.half_fov = 0.0;
                let first = if self.prop(d, 0x1b) { if rng.rand(2) == 0 { 0 } else { self.choose_combat_move(d, rng) } }
                    else if self.prop(d, 0x18) { 0 } else if self.prop(d, 0x19) { self.choose_combat_move(d, rng) } else { 0 };
                // `FirstAttack` may switch to the second behaviour block (`+0x5A4` ≠ 0, not modelled); [L] the 07000005
                // gate guard still used its first block after a fight (`+0x4D8` = `+0x4DC`, WORKLOG 0021), so no switch.
                self.set_state(if first != 0 { first } else if self.prop(d, 0x18) && self.dist < self.engage_range { AIM_STAND } else { COMBAT });
                true
            }
            (COMBAT, msg::ENTER) => { self.alert = 1.0; true }
            (COMBAT_OUT_OF_RANGE | COMBAT_NO_SIGHT, msg::ENTER) => { self.alert = 1.0; self.vision_on = true; true }
            (COMBAT | COMBAT_OUT_OF_RANGE | COMBAT_NO_SIGHT | COMBAT_DRONE_AHEAD, msg::UPDATE) => {
                let n = self.combat_state(d, rng, frame, s);
                self.set_state(n);
                true
            }
            // CombatNoRoute: stand, face, fire; every second try to route again or give up (`+0xCD8` alternates).
            (COMBAT_NO_ROUTE, msg::ENTER) => {
                self.vision_on = true;
                self.no_route_odd = false;
                self.call_anim(d, anim_state::ALERT_IDLE, 0, 0);
                self.timer_c = Some(frame + 60);
                self.croute = None;
                self.fire(d, rng, frame, true);
                true
            }
            (COMBAT_NO_ROUTE, msg::UPDATE) => {
                self.desired_yaw = self.yaw_to_opp;
                if !self.seen {
                    match self.choose_combat_move(d, rng) {
                        0 | AIM_STAND | AIM_CROUCH => self.call_anim(d, anim_state::ALERT_IDLE, 0, 0),
                        m => self.set_state(m),
                    }
                } else {
                    self.call_anim(d, anim_state::AIM_STAND, 0, 0);
                    self.fire(d, rng, frame, true);
                }
                true
            }
            (COMBAT_NO_ROUTE, msg::TIMER_C) => {
                self.no_route_odd = !self.no_route_odd;
                if !self.stays(d) && self.cover_available(d, true) { self.set_state(RUN_FOR_COVER); return true; }
                let next = if self.stays(d) { 0 } else if self.no_route_odd {
                    match self.move_to_object(d, 2.0, true) { 0..=2 => COMBAT, 10 => return true, _ => 0 }
                } else if self.set_goal_to_drone_with_los(d, rng) { GO_TO_GOAL } else { 0 };
                if next != 0 { self.set_state(next); return true; }
                // Fallback: visible → Rand(2) ≠ 0 and in range → AimStand, else the closer position; not visible → look
                // about and try again in a second.
                if self.seen {
                    if rng.rand(2) != 0 && self.dist < self.engage_range { self.set_state(AIM_STAND); return true; }
                    if self.set_goal_to_closer_position(d) { self.set_state(GO_TO_GOAL); return true; }
                }
                self.call_anim(d, anim_state::ALERT_IDLE, 0, 0);
                self.timer_c = Some(frame + 60);
                true
            }
            // GoToGoalPosition 0x63 (`NDrone2_DSTATE_GoToGoalPosition` 0x165640): run to the goal (`+0x710`), looking for
            // the opponent on the way; blocked → aim or look about; arrived or failed → DeleteMe (prop 0x59) or the
            // return state `+0x5A2`. Deaf meanwhile (`+0x4F8` 0x400000; hearing is not ported).
            (GO_TO_GOAL, msg::ENTER) => { self.vision_on = true; true }
            (GO_TO_GOAL, msg::UPDATE) => {
                if self.seen && !(self.ever_seen && self.dist >= self.engage_range) {
                    if let Some(n) = self.look_for_opponent(d) { self.set_state(n); return true; }
                }
                match self.move_to_goal(d) {
                    0..=2 => self.call_anim(d, anim_state::RUN, 0, 0),
                    9 | 0xc => self.face_and_aim(d),
                    10 => {}
                    _ => {
                        if self.prop(d, 0x59) { self.present = false; self.enabled = false; }
                        else { let r = if self.return_state != 0 { self.return_state } else { self.start_state(d) }; self.set_state(r); }
                    }
                }
                true
            }
            // DroneStuck: `DAT_00299024` is only ever 0, so the update's escape never fires; the enter goes to NoRoute.
            (DRONE_STUCK, msg::ENTER) => { self.vision_on = true; self.set_state(COMBAT_NO_ROUTE); true }
            (COMBAT_NO_MOVE, msg::UPDATE) => {
                if self.unseen_ticks < 16 { self.set_state(COMBAT_NEW_SIGHTING) } else { self.set_state(COMBAT_NO_SIGHT) }
                true
            }
            (COMBAT_NEW_SIGHTING, msg::ENTER) => {
                let crouch = self.prop(d, 0x11) && self.can_do(d, anim_state::CROUCH_AIM) && self.dist >= 6.0 && rng.rand(2) != 0;
                self.set_state(if crouch { AIM_CROUCH_FIRE } else { AIM_STAND_FIRE });
                true
            }
            (AIM_STAND, msg::ENTER) => { self.call_anim(d, anim_state::AIM_STAND, 0, msg::ANIM_DONE); self.firing = false; true }
            (AIM_STAND, msg::ANIM_DONE) => { self.set_state(if self.unseen_ticks < 16 { AIM_STAND_FIRE } else { COMBAT_NO_SIGHT }); true }
            (AIM_STAND_FIRE, msg::ENTER) => {
                self.call_anim(d, anim_state::AIM_STAND, 0, 0);
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
                self.call_anim(d, anim_state::RELOAD, if s == AIM_STAND_RELOAD { AIM_STAND_FIRE } else { AIM_CROUCH_FIRE }, 0);
                true
            }
            (AIM_STAND_RELOAD | AIM_CROUCH_RELOAD, msg::UPDATE) => { self.turn_to_opponent(0.9); true }
            // Crouching: AimCrouch plays the crouch-aim anim and continues in AimCrouchFire.
            (AIM_CROUCH, msg::ENTER) => { self.call_anim(d, anim_state::CROUCH_AIM, AIM_CROUCH_FIRE, 0); true }
            (AIM_CROUCH, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            (AIM_CROUCH_FIRE, msg::ENTER) => { self.call_anim(d, anim_state::CROUCH_AIM, 0, 0); self.vision_on = true; self.fire(d, rng, frame, true); true }
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
                self.call_anim(d, if s == STEP_AIM_LEFT { anim_state::STEP_LEFT } else { anim_state::STEP_RIGHT }, AIM_STAND_FIRE, 0);
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
                self.call_anim(d, anim, 0, 0);
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
                self.call_anim(d, if s == ROLL_LEFT_CROUCH { anim_state::ROLL_LEFT } else { anim_state::ROLL_RIGHT }, AIM_CROUCH_FIRE, 0);
                true
            }
            (ROLL_LEFT_CROUCH | ROLL_RIGHT_CROUCH, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            (AIM_BACKOFF, msg::ENTER) => {
                self.call_anim(d, anim_state::BACKOFF, 0, 0);
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
            (NO_OPPONENT, msg::ENTER) => { self.set_state(IDLE); true }
            (_, msg::ENTER | msg::EXIT | msg::UPDATE) => true,
            _ => false,
        }
    }

    /// `DroneVision_EnemyLookForOpponent` (Attack when seen and the drone may react to the player).
    /// `DroneVision_EnemyLookForOpponent` (0x177ed0): Attack when Bond is seen (prop 0x3C) and the drone is alert
    /// (≥ 0.66) or has prop 0x51; the first such sighting marks it as the drone's own (`+0x4F8 |= 0xC000000`).
    fn look_for_opponent(&mut self, d: &Data) -> Option<u8> {
        let go = self.seen && self.prop(d, 0x3c) && (self.alert >= 0.66 || self.prop(d, 0x51));
        if go && !self.sighted { self.sighted = true; self.own_sight = true; }
        go.then_some(state::ATTACK)
    }

    /// `NDrone2_CheckSurrender` (0x14a060, inlined in `FirstSightState`): prop 0x43, alertness < 0.66, Bond within
    /// 2.0, more than 120° off the drone's facing (behind it), in line of sight, armed, and facing the drone within 45°.
    fn check_surrender(&self, d: &Data) -> bool {
        self.prop(d, 0x43) && self.alert < 0.66 && self.dist < 2.0 && angle_difference(self.yaw, self.yaw_to_opp).abs() > 2.094_395_2
            && self.seen && self.bond_armed && self.opp_facing.abs() < std::f32::consts::FRAC_PI_4
    }

    /// `NDrone2_CheckUnsurrender` (0x14a168): farther than 12, or beyond 2 with Bond unarmed, or Bond not facing the
    /// drone within 90°, or no line of sight.
    fn check_unsurrender(&self, d: &Data) -> bool {
        self.dist > 12.0 || (self.dist > 2.0 && !self.bond_armed) || self.opp_facing.abs() > std::f32::consts::FRAC_PI_2
            || !d.world.line_of_sight(self.head(), self.opp_target)
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
        if self.stays(d) { return 0; }
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

    /// `DroneFunc_CombatState` (0x148338): the combat decision for 0x58 / 0x5A / 0x5C / 0x5E / 0x68 / 0x70. Every
    /// call routes to the opponent first (`MoveToObject` with radius min(engage range, 6); following the route except
    /// from the firing states). Cover (0x8C) and grenades are not ported yet.
    fn combat_state(&mut self, d: &Data, rng: &mut Rng, frame: u32, from: u8) -> u8 {
        use state::*;
        if self.in_transition() { return 0; }
        let follow = !matches!(from, AIM_STAND_FIRE | AIM_CROUCH_FIRE);
        let mv = self.move_to_object(d, self.engage_range.min(6.0), follow);
        let routed = mv < 4 || mv == 10;
        let out_of_range = self.engage_range <= self.dist;
        let no_move = self.stays(d);
        match from {
            COMBAT => {
                if no_move { return COMBAT_NO_MOVE; }
                if out_of_range { return COMBAT_OUT_OF_RANGE; }
                if !self.seen { return if self.unseen_ticks > 15 { COMBAT_NO_SIGHT } else { COMBAT_NO_ROUTE }; }
                COMBAT_NEW_SIGHTING
            }
            COMBAT_OUT_OF_RANGE => {
                if no_move { return COMBAT_NO_MOVE; }
                if self.cover_available(d, true) { return RUN_FOR_COVER; }
                if !out_of_range { return COMBAT; }
                self.moving_result(d, rng, mv, true)
            }
            COMBAT_NO_SIGHT => {
                if no_move { return COMBAT_NO_MOVE; }
                if self.cover_available(d, true) { return RUN_FOR_COVER; }
                if self.seen { return COMBAT_NEW_SIGHTING; }
                if self.unseen_ticks > 15 { return self.moving_result(d, rng, mv, false); }
                self.face_and_aim(d);
                0
            }
            COMBAT_DRONE_AHEAD => {
                if routed { return COMBAT; }
                self.face_and_aim(d);
                0
            }
            AIM_STAND_FIRE => {
                if self.clip < 1 { return AIM_STAND_RELOAD; }
                if routed && out_of_range { return COMBAT_OUT_OF_RANGE; }
                if self.unseen_ticks > 15 { return COMBAT_NO_SIGHT; }
                if self.cover_available(d, true) { return RUN_FOR_COVER; }
                if self.anim.class == 2 && self.anim.state == anim_state::AIM_STAND {
                    let m = self.choose_combat_move(d, rng);
                    if m != 0 { return m; }
                }
                self.call_anim(d, anim_state::AIM_STAND, 0, 0);
                0
            }
            _ => {
                if routed && out_of_range { return COMBAT_OUT_OF_RANGE; }
                if self.unseen_ticks > 15 { return COMBAT_NO_SIGHT; }
                if self.cover_available(d, true) { return RUN_FOR_COVER; }
                if frame > self.entered + 120 {
                    let m = self.choose_combat_move(d, rng);
                    if m != 0 { return m; }
                }
                self.call_anim(d, anim_state::CROUCH_AIM, 0, 0);
                0
            }
        }
    }

    /// `MoveToObject` results while moving (0x5A; 0x5C after 15 unseen ticks): 0/1/2 move (combat move anims in
    /// 07000005/06 or from 0x5C, else run), 3/9/0xB/0xC face and aim, 5 stuck, 10 wait, else no route.
    fn moving_result(&mut self, d: &Data, rng: &mut Rng, mv: u8, out_of_range_state: bool) -> u8 {
        match mv {
            0..=2 => {
                let remaining = self.croute.as_ref().map_or(0.0, |r| r.remaining);
                if !out_of_range_state || matches!(d.level, 0x0700_0005 | 0x0700_0006) { self.set_combat_move_anim(d, rng, remaining) }
                else { self.call_anim(d, anim_state::RUN, 0, 0) }
                0
            }
            3 | 9 | 0xb | 0xc => { self.face_and_aim(d); 0 }
            5 => state::DRONE_STUCK,
            10 => 0,
            _ => state::COMBAT_NO_ROUTE,
        }
    }

    /// `NDrone2_SetAngleToObj(opponent)`, then aim (anim 0xA, firing while visible) or look about (0x21).
    fn face_and_aim(&mut self, d: &Data) {
        self.desired_yaw = self.yaw_to_opp;
        if self.seen { self.firing = true; self.call_anim(d, anim_state::AIM_STAND, 0, 0) } else { self.call_anim(d, anim_state::ALERT_IDLE, 0, 0) }
    }

    /// `DroneAnim_InTransition` (0x13e3b0): the playing danim's class is above 3 and not 8.
    fn in_transition(&self) -> bool {
        self.anim.class > 3 && self.anim.class != 8
    }

    /// `NDrone2_MoveToObject(radius, opponent, follow)` (0x151618): verify / compute the combat route to the player's
    /// feet, then 3 when the route is shorter than 2.0, else follow it (move target, desired yaw). Results as the game:
    /// 0 moving, 2 direct, 3 there, 4/5/6/8 failures, 0xC blocked. The radius (target node search) is fixed at 6 in
    /// `Drones::tick`.
    fn move_to_object(&mut self, d: &Data, _radius: f32, follow: bool) -> u8 {
        let feet = self.feet();
        let path = *self.nav_path.get_or_insert_with(|| d.nav.nav_path_for(feet));
        // `AINetwork_VerifyRouteTarget`: still valid while the player target has not been rebuilt.
        if !self.croute.as_ref().is_some_and(|r| r.stamp == self.target.stamp) {
            let goal = path.and_then(|p| self.target.nodes.get(p).copied()).unwrap_or_default();
            let used: &[u16] = path.and_then(|p| self.links_used.get(p)).map_or(&[], |v| v.as_slice());
            match crate::nav::Route::to_position(d.nav, d.world, path, feet, self.target.pos, &goal, used) {
                Ok((_, mut r)) => { r.stamp = self.target.stamp; self.croute = Some(r) }
                Err(code) => { self.croute = None; return code }
            }
        }
        let route = self.croute.as_mut().unwrap();
        route.remaining = route.distance(d.nav, feet);
        if route.remaining < 2.0 { return 3; }
        if !follow { return 0; }
        let (mut code, _) = route.follow(d.nav, feet);
        if route.nodes.is_empty() && code == 0 { code = 2 }
        if let Some(used) = self.links_used.get_mut(route.path) { route.mark_used(used) }
        let t = route.target;
        self.desired_yaw = atan2_approx(t[0] - self.pos[0], t[2] - self.pos[2]);
        match code { 0 | 1 => 0, 2 | 3 | 0xc => code, _ => 0 }
    }

    /// `DroneAnim_GetMoveQuad(a, previous)` (0x13ce00): 0 forward, 1 / 3 sideways, 2 backward, with hysteresis.
    fn move_quad(a: f32, prev: u8) -> u8 {
        let side = |lim: f32| if a.abs() > lim { if a > 0.0 { 3 } else { 1 } } else { 0 };
        match prev {
            0 => if a.abs() > 2.356_194_7 { 2 } else { side(0.925_024_5) },
            1 => if a.abs() > 2.495_821 { 2 } else if a > 0.785_398_2 { 3 } else if a < -0.925_024_5 { 1 } else { 0 },
            2 => if a.abs() > 2.216_568_2 { 2 } else { side(0.785_398_2) },
            3 => if a.abs() > 2.495_821 { 2 } else if a > 0.925_024_5 { 3 } else if a < -0.785_398_2 { 1 } else { 0 },
            _ => 0,
        }
    }

    /// `DroneAnim_SetCombatMoveAnim(route distance)` (0x13cfb0): the moving anim from the angle between the move target
    /// and the opponent (run-and-aim forward, back off, strafe / roll / step sideways), with the body turned by the
    /// anim's yaw offset (±π/2, π) so the anim's own direction follows the route. The "drone ahead" flags of
    /// `DroneMove_NoBunching` (`+0x31C`) are not ported.
    fn set_combat_move_anim(&mut self, d: &Data, rng: &mut Rng, route_dist: f32) {
        use anim_state::*;
        let Some(route) = self.croute.as_ref() else { self.call_anim(d, RUN, 0, 0); return };
        let t = route.target;
        let next = route.next_node_turn(d.nav, self.feet());
        let a = angle_difference(atan2_approx(t[0] - self.pos[0], t[2] - self.pos[2]), self.yaw_to_opp);
        let quad = Self::move_quad(a, self.move_quad);
        self.move_quad = 0;
        let fire_ok = self.unseen_ticks < 180;
        let (mut anim, mut offset, mut fire) = (RUN, 0.0f32, false);
        match quad {
            1 | 3 => {
                let left = quad == 1;
                let (strafe, alt, step, sign) = if left { (STRAFE_LEFT, STRAFE_LEFT_ALT, STEP_LEFT, 1.0) } else { (STRAFE_RIGHT, STRAFE_RIGHT_ALT, STEP_RIGHT, -1.0) };
                if self.prop(d, 0) || self.prop(d, 0x41) {
                    let cur = self.anim.state;
                    let s = if cur == strafe || cur == alt { Some(cur) } else if self.can_do(d, strafe) { Some(strafe) } else { None };
                    if let Some(s) = s {
                        self.move_quad = quad;
                        (anim, offset, fire) = (s, sign * crate::trig::HALF_PI, fire_ok);
                        if route_dist > 2.0 && route_dist < 2.5 && self.prop(d, 0x2e) {
                            let (r1, r2) = if left { (ROLL_LEFT, ROLL_LEFT_ALT) } else { (ROLL_RIGHT, ROLL_RIGHT_ALT) };
                            // As built: with both rolls available the right side picks 0x53 (the left roll) or 0x56.
                            let roll = match (self.can_do(d, r1), self.can_do(d, r2)) {
                                (true, true) => Some(if rng.rand(2) != 0 { ROLL_LEFT } else { r2 }),
                                (true, false) => Some(r1),
                                (false, true) => Some(r2),
                                _ => None,
                            };
                            if let Some(r) = roll { anim = r; fire = false; }
                        }
                    }
                } else if self.prop(d, 0x42) && self.can_do(d, step) {
                    if let Some((_, turn)) = next.filter(|(nd, _)| *nd < 3.0) {
                        let limit = if self.anim.state == step { 2.792_527 } else { 2.617_994 };
                        if turn.abs() < limit { self.move_quad = quad; (anim, offset) = (step, sign * crate::trig::HALF_PI); }
                    }
                }
            }
            2 => {
                if (self.prop(d, 0) || self.prop(d, 6)) && self.can_do(d, BACKOFF) {
                    self.move_quad = 2;
                    (anim, offset, fire) = (BACKOFF, crate::trig::PI, fire_ok);
                }
            }
            _ => {
                let pick = if (self.prop(d, 1) || self.prop(d, 0)) && self.can_do(d, RUN_AIM) { Some(RUN_AIM) }
                    else if self.prop(d, 2) && self.can_do(d, WALK_AIM) { Some(WALK_AIM) } else { None };
                if let Some(p) = pick { (anim, fire) = (p, fire_ok); }
            }
        }
        self.firing = fire;
        self.call_anim(d, anim, 0, 0);
        self.yaw_offset = offset;
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
        self.opp_target = p.target;
        let bond_yaw = atan2_approx(p.aim[0], p.aim[2]);
        self.opp_facing = angle_difference(bond_yaw, atan2_approx(-dx, -dz));
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
        // `DroneVision_OpponentVisibility` (0x175af0): × `+0x22C` (the AI-volume factor) unless the drone has alert
        // bits and full alertness. [L] 07000005 start: Bond stands in a 0.25 shadow volume (WORKLOG 0032).
        if !(self.alert_bits != 0 && self.alert >= 1.0) && self.light != 1.0 { vis *= self.light; }
        vis.max(0.0)
    }

    /// `DroneFunc_ReactionTime` (ticks).
    fn reaction_time(&self, level: u32) -> u32 {
        if matches!(level, 0x0700_0002 | 0x0700_0003 | 0x0700_0005 | 0x0700_0009 | 0x0700_000a) {
            // ((1 − alertness) × 30) × 0.01 × (200 − stat), each multiply chopped (EE FPU): 59 for these guards
            // (nearest rounding would give 60).
            use nf_formats::ps2float::{add_noguard, mul};
            if self.ever_seen || self.alert >= 1.0 { 0 } else {
                let k = mul(mul(mul(add_noguard(1.0, -self.alert, true), 30.0), 0.01), add_noguard(200.0, -(self.reaction as f32), true));
                k as u32
            }
        } else { 10 }
    }

    fn head(&self) -> V3 { [self.pos[0], self.pos[1] + 0.6, self.pos[2]] }
}

fn seg_point_dist2(a: V3, b: V3, p: V3) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let l2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if l2 > 0.0 { ((ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let q = [a[0] + ab[0] * t - p[0], a[1] + ab[1] * t - p[1], a[2] + ab[2] * t - p[2]];
    q[0] * q[0] + q[1] * q[1] + q[2] * q[2]
}

/// One of Bond's bullets (`Player_WeaponInitBullet` → `Bullet_init` / `Bullet_update` with owner = player).
#[derive(Clone, Debug)]
pub struct PlayerBullet { pub pos: V3, pub dir: V3, pub speed: f32, pub travelled: f32, pub range: f32, pub damage: f32, pub weapon: u16, pub first: bool, pub dead: bool }

/// What one of Bond's bullets hit: a drone (index, part) or the world, and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerImpact { Drone { drone: usize, part: i16, at: V3 }, World(V3) }

/// A sound in the hearing list (`Sound_Play3D`: position, inner / outer radius, weight).
#[derive(Clone, Copy, Debug)]
pub struct Sound { pub id: u32, pub pos: V3, pub inner: f32, pub outer: f32, pub weight: f32 }

/// Anim states of the noise chain: turn (0x61) and a short look (0xB).
const TURN: u16 = 0x61;
const LOOK_SHORT: u16 = 0x0b;

/// Stand-in for the renderer's "drawn" (`obj+0xFC`) and `Drone_PositionVisible`: within 60° of Bond's aim (or within
/// 2 units) and any of five body points in line of sight from his eye (thin occluders hide one ray, not a cel).
pub fn visible_to(player: &Player, world: &CollisionWorld, pos: V3, head: V3) -> bool {
    let v = [pos[0] - player.eye[0], pos[2] - player.eye[2]];
    let (a, l) = ([player.aim[0], player.aim[2]], (v[0] * v[0] + v[1] * v[1]).sqrt());
    let al = (a[0] * a[0] + a[1] * a[1]).sqrt();
    let side = [-v[1] / l.max(1e-6) * 0.5, v[0] / l.max(1e-6) * 0.5];
    let pts = [head, pos, [pos[0] + side[0], pos[1], pos[2] + side[1]], [pos[0] - side[0], pos[1], pos[2] - side[1]], [pos[0], pos[1] - 0.8, pos[2]]];
    l < 2.0 || (al > 1e-6 && (v[0] * a[0] + v[1] * a[1]) / (l * al) > 0.5 && pts.iter().any(|&q| world.line_of_sight(player.eye, q)))
}

/// Every drone placement of the level's main map, in placement order (`Drone_Create`); spawner templates included
/// (`Drones::attach_level` marks them absent). Positions on the collision floor below the placement.
pub fn level_drones(d: &Data, level: &nf_formats::level::Level, rng: &mut Rng) -> Vec<Drone> {
    let Some(m) = level.main_map() else { return Vec::new() };
    level.maps[m].placements.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0x0f).map(|(i, p)| {
        let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
        let floor = d.world.floor_below(p.translation, 3.0).unwrap_or(p.translation[1] - 1.0);
        let mut dr = Drone::new(d, [p.translation[0], floor + 1.0, p.translation[2]], p.rotation[1], param, rng);
        dr.placement = i;
        dr
    }).collect()
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
    /// The game's drone list (0x298E14, `LList_Add` appends): drones created at load in placement order, spawned ones
    /// appended — the order of the line-of-sight round robin.
    pub llist: Vec<usize>,
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
    /// The player as an AI target and this frame's link used-counts per path.
    pub target: PlayerTarget,
    used: Vec<Vec<u16>>,
    /// `switch_channels` and the level's trigger objects (docs/systems/triggers.md).
    pub channels: crate::triggers::Channels,
    pub triggers: Vec<crate::triggers::Trigger>,
    pub spawners: Vec<crate::spawners::Spawner>,
    /// Cover nodes' run-time state, `Drone_ProcessCoverNodes`' shared cursor and its frame counter.
    pub cover_rt: Vec<crate::cover::CoverRt>,
    /// Sounds made since the last frame (`Sound_Play3D`'s list; weights zeroed after the drones heard them).
    pub sounds: Vec<Sound>,
    /// Bond's bullets in flight, what they hit this tick, the global flinch stamp (`DAT_00298FC0`) and whether Bond
    /// aims (L1).
    pub player_bullets: Vec<PlayerBullet>,
    pub player_impacts: Vec<PlayerImpact>,
    pub last_flinch: Option<u32>,
    /// The last alarm broadcast (`DAT_0029AA88`) and whether one went out (`DAT_0029AA85`, 07000014 sends only one).
    pub last_alarm: Option<u32>,
    /// Auto-aim: the last target (`BL+0x114`) and the crosshair offset as (yaw, pitch) radians (`BL+0x120` / `+0x124`,
    /// screen units in the game; *approximation*: small angles).
    pub aim_target: Option<usize>,
    pub aim_offset: [f32; 2],
    pub alarm_sent: bool,
    pub bond_aiming: bool,
    /// Bond's weapon byte (player `anim+0x62`; the P2K = 2).
    pub bond_weapon: u8,
    /// Pickups in the world, Bond's inventory and what he took (tick, what) — docs/systems/pickups.md.
    pub pickups: Vec<crate::pickup::Pickup>,
    pub inventory: crate::pickup::Inventory,
    pub taken: Vec<(u32, crate::pickup::Taken)>,
    cover_cursor: usize,
    cover_counter: i32,
}

impl Drones {
    pub fn new(mut list: Vec<Drone>, rng: Rng) -> Drones {
        // `Drone_SM_InitObject` numbers state machines in creation order (the base is not known: 0 here).
        for (i, dr) in list.iter_mut().enumerate() { dr.sm_id = i as u32; dr.index = i; }
        let llist = (0..list.len()).collect();
        Drones { list, frame: 0, los_cursor: 0, llist, rng, bullets: Vec::new(), health: Default::default(), player_hits: Vec::new(), impacts: Vec::new(), prev_player: None, moved_since: None, target: PlayerTarget::default(), used: Vec::new(), channels: Default::default(), triggers: Vec::new(), spawners: Vec::new(), cover_rt: Vec::new(), sounds: Vec::new(), cover_cursor: 0, cover_counter: 0, player_bullets: Vec::new(), player_impacts: Vec::new(), last_flinch: None, bond_aiming: false, bond_weapon: 2, last_alarm: None, alarm_sent: false, aim_target: None, aim_offset: [0.0; 2],
            pickups: Vec::new(), inventory: crate::pickup::Inventory::level_start(0x73, 34), taken: Vec::new() }
    }

    /// Loads the level's trigger objects and gives each drone its placement index (matched by position among the
    /// main map's drone placements), which orders the per-frame object updates.
    pub fn attach_level(&mut self, level: &nf_formats::level::Level) {
        self.triggers = crate::triggers::build(level);
        let Some(m) = level.main_map() else { return };
        for dr in &mut self.list {
            if let Some(i) = level.maps[m].placements.iter().position(|p| p.flags & 0xff == 0x0f
                && (p.translation[0] - dr.pos[0]).abs() < 1e-3 && (p.translation[2] - dr.pos[2]).abs() < 1e-3) { dr.placement = i; }
        }
        // Spawners: their group's drones become templates, absent until spawned (`DroneSpawner_Init`).
        self.spawners.clear();
        for (i, p) in level.maps[m].placements.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xf1) {
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
            if param(0) <= 1 || param(1) == 0 { continue; }
            let templates: Vec<usize> = self.list.iter().enumerate().filter(|(_, x)| x.group == param(1)).map(|(k, _)| k).collect();
            if let Some(sp) = crate::spawners::Spawner::new(i, param, templates.clone(), &self.channels) {
                for &k in &templates { self.list[k].present = false; self.list[k].enabled = false; }
                self.llist.retain(|i| !templates.contains(i));
                self.spawners.push(sp);
            }
        }
    }

    /// `Sound_Play3D(id, pos)`: enters a sound for the drones to hear next frame (hearing data from `SFXOutputData`).
    /// Bond fires `weapon` from `from` along `dir` (`Player_WeaponInitBullet`: head → aim point): one bullet per pellet
    /// (`+0x18`) with the record's damage `+0x0C`, range `+0x1C`, speed `+0x20` and spread `+0x24`; and the shot's
    /// sound (the first-person fire script's, 0x403 for the P2K) for the drones' hearing.
    pub fn fire_player(&mut self, d: &Data, from: V3, dir: V3, weapon: u8, sound: nf_formats::sfx::SfxHearing) {
        let Some(w) = d.weapons.records.get(weapon as usize) else { return };
        let f = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let flags68 = u32::from_le_bytes(w[0x68..0x6c].try_into().unwrap());
        for _ in 0..u32::from_le_bytes(w[0x18..0x1c].try_into().unwrap()).clamp(1, 16) {
            let dir = crate::bullet::spread(dir, f(0x24), flags68, &mut self.rng);
            self.player_bullets.push(PlayerBullet { pos: from, dir, speed: f(0x20), travelled: 0.0, range: f(0x1c), damage: f(0x0c), weapon: weapon as u16, first: true, dead: false });
        }
        self.play_sound(from, sound);
    }

    /// Bond's bullets for one tick (`Bullet_update`, `Collide_Update`): the segment of max(step, 1) units against the
    /// world and every live drone's boxes (`Collide_PickObj` sphere r 1.05 around the hips, then `Collide_Jointy`); the
    /// nearest wins. A drone hit sends msg 8 (`Drone_BulletHit`).
    fn update_player_bullets(&mut self, d: &Data) {
        self.player_impacts.clear();
        let frame = self.frame;
        let mut bullets = std::mem::take(&mut self.player_bullets);
        for b in &mut bullets {
            if b.dead { continue; }
            if b.travelled > b.range { b.dead = true; continue; }
            let mut step = if b.first { (self.rng.frand(0.5) + 0.5) * b.speed } else { b.speed };
            b.first = false;
            b.travelled += step;
            if b.travelled > b.range { step -= b.travelled - b.range; }
            let l = step.max(1.0);
            let ray = [b.dir[0] * l, b.dir[1] * l, b.dir[2] * l];
            let end = [b.pos[0] + ray[0], b.pos[1] + ray[1], b.pos[2] + ray[2]];
            let tw = d.world.segment(b.pos, end);
            let mut best: Option<(usize, i16, f32)> = None;
            for (i, dr) in self.list.iter().enumerate() {
                if !dr.present || !dr.enabled || dr.corpse { continue; }
                let c = [dr.pos[0], dr.pos[1], dr.pos[2]];
                if seg_point_dist2(b.pos, end, c) > 1.05 * 1.05 * 4.0 { continue; }
                if let Some((part, t)) = crate::damage::first_box_hit(b.pos, ray, &dr.hit_boxes(d)) {
                    if best.is_none_or(|(_, _, bt)| t < bt) { best = Some((i, part, t)); }
                }
            }
            let at = |t: f32| [b.pos[0] + ray[0] * t, b.pos[1] + ray[1] * t, b.pos[2] + ray[2] * t];
            match (best, tw) {
                (Some((i, part, t)), w) if w.is_none_or(|w| t < w) => {
                    b.dead = true;
                    let hit = Hit { part, ray, point: at(t), damage: b.damage, weapon: b.weapon };
                    self.player_impacts.push(PlayerImpact::Drone { drone: i, part, at: hit.point });
                    let rng = &mut self.rng;
                    let dr = &mut self.list[i];
                    dr.hit = Some(hit);
                    dr.global_flinch = self.last_flinch;
                    dr.bond_aiming = self.bond_aiming;
                    dr.send(d, rng, frame, msg::BULLET);
                    self.last_flinch = dr.global_flinch;
                }
                (_, Some(w)) => { b.dead = true; self.player_impacts.push(PlayerImpact::World(at(w))); }
                _ => b.pos = [b.pos[0] + b.dir[0] * step, b.pos[1] + b.dir[1] * step, b.pos[2] + b.dir[2] * step],
            }
        }
        bullets.retain(|b| !b.dead);
        self.player_bullets = bullets;
    }

    /// `Sound_Play3D` (0x1dd548) with the table's radii: the sound enters the hearing list for the next tick. Looped
    /// sounds are never heard (`Sound_Alertness` skips them).
    /// `Player_AutoAim` (0x1a7c98) / `Check_AutoAim` (0x1a73f8), once per tick when auto-aim is on and Bond is not in aim
    /// mode: the nearest qualifying drone's bone 2 within `Autoaim_Range` (25): drawn, alive, not type 0xB; |yaw to it −
    /// Bond's yaw| < `Autoaim_Angle_H` · k and |pitch to it − Bond's pitch| < `Autoaim_Angle_V` · k with k = weapon
    /// `+0x14` / 100 · (1 − d / 25) · the difficulty multiplier (Easy 1.9, Normal 1, Hard 0), ×1.4 (`LockOnMul`) for the
    /// last target; line of sight from the head. A target snaps the crosshair offset onto it; the offset then halves
    /// every tick. Returns the direction the next bullet takes (`Player_GetAimingPoint`), given Bond's view `look`.
    pub fn auto_aim(&mut self, d: &Data, head: V3, look: V3, weapon: u8) -> V3 {
        let t = d.tuning;
        let yaw = look[0].atan2(look[2]);
        let pitch = look[1].clamp(-1.0, 1.0).asin();
        let wf = d.weapons.records.get(weapon as usize).map_or(100.0, |w| f32::from_le_bytes(w[0x14..0x18].try_into().unwrap()));
        let diff = match DIFFICULTY { 1 => t.Autoaim_EasyMul, 3 | 4 => t.Autoaim_HardMul, _ => t.Autoaim_NormalMul };
        let mut best = t.Autoaim_Range;
        let mut found = None;
        for (i, dr) in self.list.iter().enumerate() {
            if !dr.present || !dr.enabled || !dr.drawn || dr.corpse || dr.health <= 0.0 || dr.kind == 0xb { continue; }
            let Some(p) = dr.palette(d).and_then(|m| m.get(2).map(|r| [r[3][0], r[3][1], r[3][2]])) else { continue };
            let dv = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
            let dist = (dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]).sqrt();
            if dist > best || dist <= 0.0 { continue; }
            let mut k = wf * 0.01 * (1.0 - dist / t.Autoaim_Range) * diff;
            if matches!(weapon, 0x50 | 0x51) { k = 1.0; }
            let (mut h, mut v) = (t.Autoaim_Angle_H * k, t.Autoaim_Angle_V * k);
            if self.aim_target == Some(i) { h *= t.Autoaim_LockOnMul; v *= t.Autoaim_LockOnMul; }
            if h == 0.0 && v == 0.0 { continue; }
            let dyaw = angle_difference(yaw, dv[0].atan2(dv[2]));
            if dyaw.abs() >= h { continue; }
            let dpitch = angle_difference(pitch, (dv[1] / dist).clamp(-1.0, 1.0).asin());
            if dpitch.abs() >= v || !d.world.line_of_sight(head, p) { continue; }
            best = dist;
            found = Some((i, dyaw, dpitch));
        }
        match found {
            Some((i, dyaw, dpitch)) => { self.aim_target = Some(i); self.aim_offset = [dyaw, dpitch]; }
            None => { self.aim_target = None; }
        }
        let (y, p) = (yaw + self.aim_offset[0], pitch + self.aim_offset[1]);
        // The offset is halved after use (the game halves it at the end of `Player_AutoAim`; `Player_GetAimingPoint`
        // doubles it back, so a fresh target is aimed at exactly).
        self.aim_offset = [self.aim_offset[0] * 0.5, self.aim_offset[1] * 0.5];
        [p.cos() * y.sin(), p.sin(), p.cos() * y.cos()]
    }

    pub fn play_sound(&mut self, pos: V3, h: nf_formats::sfx::SfxHearing) {
        if h.looped { return; }
        self.sounds.push(Sound { id: h.id, pos, inner: h.inner, outer: h.outer, weight: h.weight });
    }

    /// `Pickup_Update` for every pickup: the fall, Bond's touch (`Pickup_Handler`; refused pickups stay, and the
    /// lifetime does not run that tick), the lifetime. Taken: sound 0xF5 at the pickup, deleted (no respawn).
    /// `hashtable_getitem` of the first-person model stands in as "the level has that skin".
    fn update_pickups(&mut self, d: &Data, player: &Player, frame: u32) {
        let alive = self.health.health > 0.0;
        for k in 0..self.pickups.len() {
            let p = &mut self.pickups[k];
            p.fall(d.world);
            if alive && p.touches(d.world, player.pos, player.target) {
                let loaded = d.skins.contains_key(&d.weapons.first_person(p.weapon as usize));
                if let Some(t) = self.inventory.take(d.weapons, p, loaded) {
                    p.gone = true;
                    let (at, sound) = ([p.m[3][0], p.m[3][1], p.m[3][2]], p.sound);
                    self.taken.push((frame, t));
                    if let (Some(id), Some(sfx)) = (sound, d.sfx) { self.play_sound(at, sfx.get(id)); }
                }
                continue;
            }
            if p.lifetime != 0 {
                p.lifetime -= 1;
                if p.lifetime == 0 { p.gone = true; }
            }
        }
        self.pickups.retain(|p| !p.gone);
    }

    /// Turns a switch channel on (callers and tests).
    pub fn set_channel(&mut self, c: u8) { let f = self.frame; self.channels.set(c as u32, 1, f); }

    /// One 60 Hz game tick: global vision (`DroneVision_ProcessDroneSight`), then each drone's control.
    pub fn tick(&mut self, d: &Data, player: &Player) {
        self.frame += 1;
        let frame = self.frame;
        // `Drone_ProcessOpponents` (0x1390c0): Bond's visibility factor from the AI volumes holding him (crouching
        // ×0.75 is not modelled: the port's Bond does not crouch), then each drone's opponent info.
        let light = crate::aivolume::visibility_factor(d.volumes, player.pos, false);
        let armed = (2..0x47).contains(&self.bond_weapon) || (0x6a..=0x71).contains(&self.bond_weapon);
        for dr in self.list.iter_mut().filter(|x| x.present) { dr.light = light; dr.opponent_info(player); dr.seen = false; dr.bond_armed = armed; }
        self.update_player_bullets(d);
        // `Drone_InitComms`: corpses (0x0F, 0x47, 0x48 + 300, 0xBF by the time their state began); beyond 3
        // (`DAT_00298F5C`) the oldest fades fast (`DroneFunc_DeadDrone`).
        let mut corpses = 0;
        let mut oldest: Option<(u32, usize)> = None;
        for (i, dr) in self.list.iter().enumerate().filter(|(_, x)| x.present) {
            let key = match dr.state { 0x0f | state::DEAD | 0xbf => dr.entered, state::FADE => dr.entered + 300, _ => continue };
            corpses += 1;
            if key < oldest.map_or(frame, |o| o.0) { oldest = Some((key, i)); }
        }
        for (i, dr) in self.list.iter_mut().enumerate() { dr.cull = corpses > 3 && oldest.is_some_and(|o| o.1 == i); }
        // `Check_Target` (0x1a7810): every 6 ticks Bond's 50-unit crosshair ray marks the first drone it hits (body
        // cylinder r 0.4, feet to 1.8 up; *deviation*: the game tests the drone's bounding sphere / boxes) for 30 ticks.
        if frame % 6 == 0 {
            let far = [player.eye[0] + player.aim[0] * 50.0, player.eye[1] + player.aim[1] * 50.0, player.eye[2] + player.aim[2] * 50.0];
            let wall = d.world.segment(player.eye, far).unwrap_or(1.0);
            let hit = self.list.iter().enumerate().filter(|(_, dr)| dr.enabled).filter_map(|(i, dr)| {
                let vol = crate::bullet::PlayerVolume { feet: [dr.pos[0], dr.pos[1] - 1.0, dr.pos[2]], height: 1.8, radius: 0.4 };
                crate::bullet::segment_cylinder(player.eye, far, &vol).filter(|&t| t < wall).map(|t| (i, t))
            }).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i);
            if let Some(i) = hit { self.list[i].aimed_at = true; self.list[i].aimed_until = frame + 30; }
        }
        for dr in &mut self.list { if dr.aimed_at && frame >= dr.aimed_until { dr.aimed_at = false; } }
        // `DroneMove_NoBunching`: another drone within 3 units.
        let positions: Vec<Option<V3>> = self.list.iter().map(|x| x.enabled.then_some(x.pos)).collect();
        for (i, dr) in self.list.iter_mut().enumerate() {
            dr.bunched = positions.iter().enumerate().any(|(j, p)| j != i && p.is_some_and(|p| ((p[0] - dr.pos[0]).powi(2) + (p[2] - dr.pos[2]).powi(2)) < 9.0));
        }
        // Visibility every tick for drones with vision on; line of sight for one drone per tick (round robin).
        for dr in &mut self.list { dr.visibility = if dr.present && !dr.corpse && (dr.vision_on || dr.alert >= 1.0) { dr.visibility() } else { 0.0 }; if dr.visibility < 0.7 { dr.los = false; } }
        let ln = self.llist.len();
        for k in 0..ln {
            let slot = (self.los_cursor + 1 + k) % ln.max(1);
            let dr = &mut self.list[self.llist[slot]];
            if dr.visibility >= 0.7 {
                dr.los = d.world.line_of_sight(dr.head(), player.target);
                self.los_cursor = slot;
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
        // `Drone_InitComms` order: `ProcessDroneSight` (vision, line of sight, `FindAlertedDrones`), the 0x97 check, then
        // every drone's `HandleSoundAlerts`, `Sound_ZeroAlertness`, the alarm.
        // Drones noticing drones that attack (`DroneVision_FindAlertedDrones` → `ConsiderAlerted` →
        // `DroneCanSeeDrone`): source attacking ≥ 1 s with visible alert bits; observer watching (Idle/Alert), not yet
        // alerted, whose props accept the bits (0x10 always; 2 → 0x27, 4 → 0x4F, 1 → 0x28, 8 → 0x26/0x29), within
        // 50 units, in its view (±e4, ≤ e8, under 2 always) and with head-to-head line of sight. At most 4 per tick.
        let mut alerts = Vec::new();
        'pairs: for (oi, o) in self.list.iter().enumerate() {
            if !o.watching || o.alerted_by_drone || !matches!(o.state, state::IDLE | state::ALERT | state::PATROL) { continue; }
            for (si, src) in self.list.iter().enumerate() {
                if si == oi || !src.enabled || src.alert_bits == 0 || frame < src.attack_frame + 60 { continue; }
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
        // `Drone_InitComms` after the vision pass: an enabled (`+0x4F8 & 0x100`, set by DefaultInit / Enable), living
        // (`& 0x600` clear) drone at alertness ≥ 1 within 30 units of Bond (not team 2) raises `switch_channels[0x97]`.
        if self.list.iter().any(|x| x.present && x.enabled && !x.corpse && x.alert >= 1.0 && x.dist < 30.0) { self.channels.set(0x97, 1, frame); }
        // `DroneFunc_HandleSoundAlerts` (every drone, every frame) over the sounds made since the last frame, then
        // `Sound_ZeroAlertness`: each sound alerts once.
        let sounds = std::mem::take(&mut self.sounds);
        if !sounds.is_empty() {
            for dr in self.list.iter_mut().filter(|x| x.present && x.enabled && !x.corpse) {
                if !dr.prop(d, 0x1f) || dr.state == state::GO_TO_GOAL { continue; }
                // `Sound_Alertness` (0x1ddc48, read in the disassembly): per sound with weight within the outer radius
                // (squared distances compared), weight × (1 inside the inner radius, else (outer − d) / (outer − inner)),
                // summed; id 0x248 counts as weight 10 everywhere inside its outer radius; ids 0x11C / 0x11D are
                // ignored on 07000009 / 0A. The "peak" is the running maximum of the sum and the loudest sound the one
                // that raised it. The result is max(sum, 0) as a float.
                use nf_formats::ps2float::{add_noguard, mul};
                let (mut sum, mut peak, mut loudest) = (0f32, 0f32, None);
                for snd in &sounds {
                    let (mut inner, mut weight) = (snd.inner, snd.weight);
                    if weight == 0.0 { continue; }
                    let dx = [snd.pos[0] - dr.pos[0], snd.pos[1] - dr.pos[1], snd.pos[2] - dr.pos[2]];
                    let d2 = add_noguard(add_noguard(mul(dx[0], dx[0]), mul(dx[1], dx[1]), true), mul(dx[2], dx[2]), true);
                    if mul(snd.outer, snd.outer) < d2 { continue; }
                    let id = snd.id & 0xfffff;
                    if (0x11c..0x11e).contains(&id) && matches!(d.level, 0x0700_0009 | 0x0700_000a) { continue; }
                    if id == 0x248 { inner = snd.outer; weight = 10.0; }
                    let dd = d2.sqrt();
                    let f = if dd < inner { 1.0 } else { add_noguard(snd.outer, -dd, true) / add_noguard(snd.outer, -inner, true) };
                    sum = add_noguard(sum, mul(f, weight), true);
                    if peak < sum { peak = sum; loudest = Some(snd); }
                }
                dr.loudness = sum.max(0.0);
                if dr.loudness <= 0.0 { continue; }
                // `DroneFunc_HandleSoundAlerts` (0x148a30): share = 0.01 · loudness, added to alertness unclamped.
                dr.sound_share = mul(dr.loudness, 0.01);
                dr.alert = add_noguard(dr.alert, dr.sound_share, true);
                if let Some(snd) = loudest {
                    if frame > dr.heard_frame + 60 || peak > dr.heard_peak {
                        dr.sound_pos = snd.pos;
                        dr.heard_frame = frame;
                        dr.heard_peak = peak;
                        dr.send(d, &mut self.rng, frame, msg::SOUND);
                    }
                }
            }
        }
        // The alarm: while `switch_channels[0x96]`, every 30 frames (and once only on 07000014) msg 0x1E to every drone.
        if self.channels.on(0x96) && self.last_alarm.is_none_or(|t| frame > t + 30) && (d.level != 0x0700_0014 || !self.alarm_sent) {
            self.alarm_sent = true;
            self.last_alarm = Some(frame);
            for dr in self.list.iter_mut().filter(|x| x.present && x.enabled) { dr.send(d, &mut self.rng, frame, msg::ALARM); }
        }
        // Bond's motion (`DroneWeap_DoOpponentTargetting`): moving, and "first moved" for 2 s after he starts.
        let moving = self.prev_player.is_some_and(|q| (q[0] - player.pos[0]).abs() + (q[2] - player.pos[2]).abs() > 1e-4);
        if moving && self.moved_since.is_none() { self.moved_since = Some(frame) }
        if !moving { self.moved_since = None }
        let first_moved = self.moved_since.is_some_and(|f| frame < f + 120);
        self.prev_player = Some(player.pos);
        // `NDrone2_NavNodeCache` / `BuildAITarget`: the player's goal nodes per navigation path, rebuilt when he has
        // moved 2.0 or more (search radius 6 = min(engage range, 6) for these guards); `AINetwork_ClearLinkFlags`.
        let moved = self.target.stamp == 0 || {
            let q = self.target.pos;
            (q[0] - player.pos[0]).powi(2) + (q[1] - player.pos[1]).powi(2) + (q[2] - player.pos[2]).powi(2) >= 4.0
        };
        if moved {
            self.target.pos = player.pos;
            self.target.stamp = frame;
            self.target.nodes = d.nav.paths.iter().enumerate().map(|(pi, p)| {
                if p.kind & 0xd == 0 { d.nav.target_nodes(d.world, pi, player.pos, 36.0) } else { Default::default() }
            }).collect();
        }
        self.used = d.nav.paths.iter().map(|p| vec![0; p.links.len()]).collect();
        // `Drone_ProcessCoverNodes` (every 5 frames from the first): for Bond, walk the nodes from the shared cursor
        // until one casts a line-of-sight ray. *Deviation*: always run (the game only for allocated target slots).
        if self.cover_rt.len() != d.covers.len() { self.cover_rt = vec![Default::default(); d.covers.len()]; }
        self.cover_counter -= 2;
        if self.cover_counter < 1 && !d.covers.is_empty() {
            self.cover_counter = 10;
            for _ in 0..d.covers.len() {
                let i = self.cover_cursor;
                self.cover_cursor = (self.cover_cursor + 1) % d.covers.len();
                if d.covers[i].usable(&mut self.cover_rt[i], d.world, &self.channels, d.level, player.pos, player.eye) { break; }
            }
        }
        let positions: Vec<Option<V3>> = self.list.iter().map(|x| x.enabled.then_some(x.pos)).collect();
        let peers: Vec<(usize, V3, bool)> = self.llist.iter().map(|&i| (i, self.list[i].pos, self.list[i].seen)).filter(|&(i, _, _)| self.list[i].enabled).collect();
        // `Trigger_Calc`, then every object in the control list (newest first = reverse placement order): triggers and
        // drones interleaved, so a channel a trigger pulses is seen only by the objects updated after it.
        let touchers: Vec<crate::triggers::Toucher> = std::iter::once(crate::triggers::Toucher::player([player.pos[0], player.pos[1] - 0.4, player.pos[2]]))
            .chain(self.list.iter().filter(|x| x.enabled).map(|x| crate::triggers::Toucher { a: [x.pos[0], x.pos[1] - 1.0, x.pos[2]], b: x.head(), r: 0.4, drone: true, skin: x.skin }))
            .collect();
        crate::triggers::calc(d.level, &mut self.triggers, &mut self.channels, &touchers, frame);
        let mut order: Vec<(usize, u8, usize)> = self.list.iter().enumerate().filter(|(_, x)| x.present).map(|(i, x)| (x.placement, 0, i))
            .chain(self.triggers.iter().enumerate().filter(|(_, t)| t.kind != crate::triggers::kind::TOUCH).map(|(j, t)| (t.placement, 1, j)))
            .chain(self.spawners.iter().enumerate().map(|(j, sp)| (sp.placement, 2, j)))
            .collect();
        order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        // Pickups are created at run time, so they head the control list: `Pickup_Update` before the placed objects.
        self.update_pickups(d, player, frame);
        // Per drone: timers, alertness decay, update message, turning, aim, firing, animation.
        for &(_, what, index) in &order {
            if what == 1 { self.triggers[index].update(&mut self.channels, &touchers, frame); continue; }
            if what == 2 {
                let list = &self.list;
                let rng = &mut self.rng;
                let action = self.spawners[index].update(&mut self.channels, frame, |i| list[i].present,
                    |i, min| { let q = list[i].pos; let dd = ((q[0] - player.pos[0]).powi(2) + (q[2] - player.pos[2]).powi(2)).sqrt();
                        dd >= min && !visible_to(player, d.world, q, [q[0], q[1] + 0.6, q[2]]) },
                    &mut |n| rng.rand(n));
                match action {
                    crate::spawners::Action::Spawn(i) => { let dr = &mut self.list[i]; dr.present = true; dr.enabled = true; dr.started = false; self.llist.push(i); }
                    crate::spawners::Action::Remove(v) => for i in v { self.list[i].present = false; self.list[i].enabled = false; self.llist.retain(|&k| k != i); },
                    crate::spawners::Action::None => {}
                }
                continue;
            }
            let rng = &mut self.rng;
            let dr = &mut self.list[index];
            if !dr.present { continue; }
            if dr.target.stamp != self.target.stamp { dr.target = self.target.clone(); }
            dr.channel_on = self.channels.on(dr.switch_channel as u32);
            dr.drawn = dr.enabled && visible_to(player, d.world, dr.pos, dr.head());
            dr.links_used = std::mem::take(&mut self.used);
            dr.cover_rt = std::mem::take(&mut self.cover_rt);
            dr.others = positions.clone();
            dr.peers = peers.clone();
            // `NDrone2_PreDroneControl`: the second switch (`+0x135`) with a tag (`+0x13A`): 0xC deletes the drone,
            // otherwise the alternate type's state (`+0x5A4`; Attack for these guards, *suggested*).
            if dr.second_channel != 0 && dr.second_tag != 0 && self.channels.on(dr.second_channel as u32) {
                dr.second_channel = 0;
                if dr.second_tag == 0xc { dr.present = false; dr.enabled = false; continue; }
                dr.second_tag = 0;
                dr.enabled = true;
                dr.set_state(if dr.alt_state != 0 { dr.alt_state } else { state::ATTACK });
            }
            if !dr.started { dr.started = true; dr.send(d, rng, frame, msg::ENTER); dr.entered = frame; dr.log.push((frame, dr.state)); }
            if let Some(t) = dr.timer_c { if frame >= t { dr.timer_c = None; dr.send(d, rng, frame, msg::TIMER_C); } }
            if let Some(t) = dr.idle_timeout { if frame >= t { dr.idle_timeout = None; dr.send(d, rng, frame, msg::IDLE_TIMEOUT); } }
            if let Some((t, m)) = dr.delayed { if frame >= t { dr.delayed = None; dr.send(d, rng, frame, m); } }
            if dr.alert < 1.0 && dr.alert > dr.alert_floor { dr.alert -= 1.0 / (60.0 * 50.0); }
            if dr.alert < dr.alert_floor { dr.alert = dr.alert_floor; }
            dr.send(d, rng, frame, msg::UPDATE);
            self.used = std::mem::take(&mut dr.links_used);
            self.cover_rt = std::mem::take(&mut dr.cover_rt);
            // NDrone2_Steer: exponential turn toward the desired yaw, 0.1 per tick, with the anim's yaw offset added
            // around it (`ControlSTANDARD`): the body faces desired − offset.
            let y = dr.yaw + dr.yaw_offset;
            dr.yaw = y + angle_difference(y, dr.desired_yaw) * 0.1 - dr.yaw_offset;
            // Upper-body aim (`AnimObjectAimAt`, speed 0.2) toward the opponent while aiming; lower clamp −0.9 only.
            let want = if dr.anim.class == 2 && dr.state != state::IDLE { angle_difference(dr.yaw, dr.yaw_to_opp).max(-0.9) } else { 0.0 };
            dr.aim_yaw = rotate_to_bearing(dr.aim_yaw, want, 0.2, crate::trig::PI);
            if !dr.corpse { if let Some(b) = dr.handle_firing(d, rng, frame, player, (moving, first_moved), index) { self.bullets.push(b); } }
            if let Some((next_ai, done_msg)) = dr.update_anim(d) {
                // `SetEndAIState`: the AI state is queued, then the message routed (which applies the queued state).
                dr.set_state(next_ai);
                if done_msg != 0 || next_ai != 0 { dr.send(d, rng, frame, done_msg); }
            }
            if let Some(c) = dr.raise.take() { self.channels.set(c as u32, 1, frame); }
            for mut p in dr.drops.drain(..) { p.from = Some(index); self.pickups.push(p); }
            // `NDrone2_Collision`: the feet sphere (r 0.4) is pushed out of the AI-network boundaries.
            if dr.enabled && dr.bounded && d.nav.bounds {
                let p = d.nav.push_out(dr.feet(), 0.4);
                dr.pos[0] = p[0];
                dr.pos[2] = p[2];
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
