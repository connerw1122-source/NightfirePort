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
    /// `NDrone2_DSTATE_PlayScript` 0x15ba68 = `NDrone2_StateFuncs[3]` [L table read]: a drone with a
    /// start script (`Drone+0x554`, param 1) plays it here before its start state.
    pub const PLAY_SCRIPT: u8 = 0x03;
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
    /// `NDrone2_DSTATE_KnockedOut_Anim` (0x172a30): the non-lethal knock-out. ENTER plays anim 0x50
    /// (the fall) with follow-on 0x47, drops the weapon, zeroes health and raises the KO flag
    /// (+0x4F8 & 0x1000000); EXIT raises the death channel on every level except {0x07000009,
    /// 0x0700000A} [M disasm 0x172b14 `sltiu`]. Entered only from `NDrone2_PunchImpact` (0x145f28).
    pub const KNOCKED_OUT_ANIM: u8 = 0x42;
    /// `NDrone2_DSTATE_Knocked_Out` (0x172b48): update/impact → Death_Anim. Never set by any C code
    /// (only the state table) — ported for completeness, not reachable in single player.
    pub const KNOCKED_OUT: u8 = 0x43;
    /// `NDrone2_DSTATE_PunchImpact` (0x172ee0): the msg-6 ENTER state (HandleImpact case 6 with
    /// allow = 1) — ENTER replays `NDrone2_PunchImpact(…, resume 0, allow 1)` (the stagger anim).
    pub const PUNCH_IMPACT: u8 = 0x53;
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
    /// Sniper cover (docs/systems/drone-cover.md): `UnderCoverFire` ENTER branches to 0x97 when the drone's anim set
    /// is 0x13 (sniper) and the level is before 0x0700000E. 0x98 is the reload.
    pub const UNDER_COVER_SNIPER_FIRE: u8 = 0x97;
    pub const UNDER_COVER_SNIPER_RELOAD: u8 = 0x98;
    /// Ambush (drone mode 0x10, `NDrone2_init_DMODE_Ambush`): 0x37 finds an ambush cover node within 2.0 → 0x93, else
    /// crouches and waits (0x38) until the opponent is visible or within 3 → 0x56.
    pub const AMBUSH_INIT: u8 = 0x37;
    pub const AMBUSH_WAIT: u8 = 0x38;
    /// Grenade throw (`NDrone2_DSTATE_GrenadeThrow` 0x2D): the throw anim (0x49) → on event 1, `DroneWeap_ThrowGrenade`
    /// → back to Attack (0x56).
    pub const GRENADE_THROW: u8 = 0x2d;
    /// The stun-grenade receiving family (`NDrone2_StateFuncs` 0x29b300 indices 0x4D/0x4E/0x4F [L table dump]).
    /// `StunGrenadeImpact` 0x172bb0: ENTER stuns (recover time 0x147188, drop weapon + ChangeWeapon(1) when
    /// kind ≠ 4, AlertStatusSet(2), anim 0x2D/0x2E with follow-on 0x4E); a bullet (msg 8/9) → Death_Anim 0x44.
    pub const STUN_GRENADE_IMPACT: u8 = 0x4d;
    /// `StunGrenadeLoop` 0x1631c0: ENTER arms the timer at frame + recover; at msg 0xC, if nobody is within 8.0
    /// (`DroneFunc_AnyoneNear(0x40000000)`) → Recover 0x4F, else re-arm frame + 5·FRAME_RATE_INT. Another flash
    /// (msg 0x18/0x19) re-arms with a fresh `DroneFunc_RecoverTime`. A bullet (msg 8/9) → 0x44; msg 6 → KO anim 0x42.
    pub const STUN_GRENADE_LOOP: u8 = 0x4e;
    /// `StunGrenadeRecover` 0x163350: ENTER sets +0x4F8 & 0x10000 and plays the get-up anim 0x76 with follow-on
    /// 0x56 (Attack) for non-civilians; bullets (msg 8/9) → 0x44.
    pub const STUN_GRENADE_RECOVER: u8 = 0x4f;
    /// The scary-object family (`DroneFunc_ConsiderExplosive` 0x1431c0 → `RunAwayFromObject` 0x86 [M 0x1431c0:194-195];
    /// `HideFromScaryObject` 0x87 waits out the flee deadline +0x324 → 0x88 [M 0x168d18:39-44];
    /// `RecoverFromScaryObject` 0x88: +0x5A0 != 0x89 → Attack 0x56 [M 0x173658:33-42]). Names per the port's
    /// msg::EXPLOSIVE doc comment (fix-explosions); the doc's "0x86 = RecoverFromScaryObject" was wrong.
    pub const RUN_AWAY_FROM_OBJECT: u8 = 0x86;
    pub const HIDE_FROM_SCARY_OBJECT: u8 = 0x87;
    pub const RECOVER_FROM_SCARY_OBJECT: u8 = 0x88;
    /// The mission-state family (`NDrone2_StateFuncs` 0x29b300; docs/WORKLOG fix-drone3): civilians and scripted
    /// characters. CivilianInit 0x10, Civilian 0x11, CivilianScared 0x12, CivilianHiding 0x13, CivilianPatrol 0x14
    /// (0x172730 / 0x15d118 / 0x15d3a0 / 0x15d520 / 0x15d610); PartyGirlInit 0x2e, PartyGirl 0x2f (0x1728c0 / 0x160140);
    /// CivilianGuard 0x30, CivilianDoorGuard 0x31 (0x1603a8 / 0x160648); the truck driver 0x32..0x35
    /// (0x160bd0 / 0x160d98 / 0x160ed0 / 0x161030); HostageIdle is 0x0d (0x15ce60 — 0x1c is AllyLeadInit, the escort
    /// entry [L NDrone2_StateFuncs 0x29b300]). CivilianMission 0x16
    /// (0x15d860) — the prop-0x1D characters WITH a kind-8 (mission) route: the party's walkers
    /// [M table entry 0x16 → 0x15d860, read from the EE snapshot].
    pub const CIVILIAN_INIT: u8 = 0x10;
    pub const CIVILIAN: u8 = 0x11;
    pub const CIVILIAN_SCARED: u8 = 0x12;
    pub const CIVILIAN_HIDING: u8 = 0x13;
    pub const CIVILIAN_PATROL: u8 = 0x14;
    pub const CIVILIAN_MISSION: u8 = 0x16;
    /// The real `NDrone2_DSTATE_HostageIdle` (0x15ce60) — `NDrone2_StateFuncs[0x0d]` [L 0x29b300].
    pub const HOSTAGE_IDLE: u8 = 0x0d;
    /// `NDrone2_DSTATE_AllyLeadInit` (0x15e4f0) — `NDrone2_StateFuncs[0x1c]` [L 0x29b300] and
    /// `DroneTypeSettings[12].start` [L 0x29aee8+12·12]: the escort lead's entry (07000001's Mayhew).
    pub const ALLY_LEAD_INIT: u8 = 0x1c;
    /// `NDrone2_DSTATE_AllyLead` (0x15e698): walk the mission AI path, waiting for Bond.
    pub const ALLY_LEAD: u8 = 0x1d;
    /// `NDrone2_DSTATE_AllyLeadWait` (0x15eda8).
    pub const ALLY_LEAD_WAIT: u8 = 0x20;
    /// `NDrone2_DSTATE_AllyLeadDone` (0x15f2e8): the route is walked — stand at the goal.
    pub const ALLY_LEAD_DONE: u8 = 0x23;
    /// `NDrone2_DSTATE_AllyLeadPlayerInWay` (0x15ea28): Bond blocks the lead route (fix-actors).
    pub const PLAYER_IN_WAY: u8 = 0x1e;
    /// `NDrone2_DSTATE_AllyLeadHide` (0x15ec08): cower while a hostile fights nearby (fix-actors).
    pub const LEAD_HIDE: u8 = 0x1f;
    /// `NDrone2_DSTATE_AllyLeadMissionWait` (0x15efc8): the route's wait-channel node (fix-actors).
    pub const LEAD_MISSION_WAIT: u8 = 0x21;
    /// `NDrone2_DSTATE_AllyLeadBondCombat` (0x15f120): hold while Bond fights nearby (fix-actors).
    pub const LEAD_BOND_COMBAT: u8 = 0x22;
    /// `NDrone2_DSTATE_AllyFollowInit` (0x15f3c8) / Follow (0x15f4e0) / FollowWait (0x15f6a0) /
    /// FollowDone (0x15f858) and `AllyGoToGoalPosition` (0x15f938) (fix-actors).
    pub const FOLLOW_INIT: u8 = 0x24;
    pub const FOLLOW: u8 = 0x25;
    pub const FOLLOW_WAIT: u8 = 0x26;
    pub const FOLLOW_DONE: u8 = 0x27;
    pub const GO_TO_GOAL_ALLY: u8 = 0x28;
    /// `NDrone2_DSTATE_OpenDoor` (0x1681f8) = `NDrone2_StateFuncs[0x81]` [L 0x29b300 + 0x81·4]. The
    /// route's door node parked the drone here: work the door (+0xBA8) until `DoorIsOpen`, then back
    /// to the saved return state `Drone+0x5A6`.
    pub const OPEN_DOOR: u8 = 0x81;
    /// `NDrone2_DSTATE_KickObject` (0x1684c0) = `NDrone2_StateFuncs[0x82]` [M 0x153358's SetState(0x82)
    /// at its kick branch; the table entry is the function itself]. The route's kick node parked the
    /// drone here: play the kick anim 0x4e, and its event 6 (msg 0x20) kicks the object in `+0xBA8`
    /// (`NDrone2_KickObject` 0x14aa00 → `SP_UnPause` 0x1dca58 for the scripted prop), grunts sfx 0x28
    /// and returns to the saved state `Drone+0x5A6`. Timer C re-kicks every 2 ticks.
    pub const KICK_OBJECT: u8 = 0x82;
    /// `NDrone2_DSTATE_ActionAnim` (0x168628) = state 0x83 [M 0x153358's action branch]. ENTER plays
    /// the drone's start script `+0x554` (`DroneAnim_SetScript(+0x554, 10, 0x32, 0x1e, 0, 5, 0)`);
    /// script-done (msg 5) → state 0x6f, msg 0x20 → the saved state `+0x5A6`.
    pub const ACTION_ANIM: u8 = 0x83;
    /// `NDrone2_DSTATE_StandFiddle` (0x1687c0) = state 0x84 [M 0x153358's fiddle branch]. ENTER plays
    /// the fiddle anim 0x23 and arms timer C at +5 ticks; the timer (and msg 0x20) → the saved state.
    pub const STAND_FIDDLE: u8 = 0x84;
    pub const PARTY_GIRL_INIT: u8 = 0x2e;
    pub const PARTY_GIRL: u8 = 0x2f;
    pub const CIVILIAN_GUARD: u8 = 0x30;
    pub const CIVILIAN_DOOR_GUARD: u8 = 0x31;
    pub const TRUCK_DRIVER_INIT: u8 = 0x32;
    pub const TRUCK_DRIVER_INIT_ALERT: u8 = 0x33;
    pub const TRUCK_DRIVER_IDLE: u8 = 0x34;
    pub const TRUCK_DRIVER_MISSION: u8 = 0x35;
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
    /// A hurt civilian's scream broadcast (`DroneFunc_SendHurtMessage` 0x145e28:36-56 broadcasts
    /// msg 0x12 with the victim as the alert source; the receivers' case 0x12 runs
    /// `DroneVision_EnemyAlerts` → prop 0x27 → `DroneVision_ShoutFromOtherDrone` [M 0x177c18:41-51]).
    pub const SCREAM: u8 = 0x12;
    /// An armed-intruder shout (`NDrone2_DroneAlertToObject(…, 0x11, Bond)` from the PartyGirl
    /// update 0x160140:63-70; the receivers' case 0x11 gates on prop 0x28 [M 0x177c18:33-40]).
    pub const SHOUT: u8 = 0x11;
    pub const TURNED: u8 = 0x23;
    pub const SEARCH_LOOKED: u8 = 0x24;
    pub const SEARCH_DONE: u8 = 0x25;
    pub const ANIM_DONE: u8 = 5;
    pub const TIMER_C: u8 = 0xc;
    /// Timer D (the sniper cover's 6-s return timer; `*(int*)(Drone+0xCD0)`).
    pub const TIMER_D: u8 = 0xd;
    /// An explosive (grenade / mine) landed near the drone (`DroneFunc_ConsiderExplosive`, msg 0x21): the cover and
    /// combat states route it to flee (state 0x86 `RunAwayFromObject`, then 0x88 `RecoverFromScaryObject` — names per
    /// the state-table map, docs/fields/dronetype-tables.md).
    pub const EXPLOSIVE: u8 = 0x21;
    pub const FORCED_ATTACK: u8 = 0x1f;
    /// `Drone_Message(0xE)`: enable (WaitSwitch → the drone's start state).
    pub const ENABLE: u8 = 0x0e;
    /// The alarm (`Drone_InitComms` → `Drone_Message(0, 0x1E)` every 30 frames while `switch_channels[0x96]`).
    pub const ALARM: u8 = 0x1e;
    /// A bullet hit (`Drone_BulletHit` → msg 8 with the HITDATA).
    pub const BULLET: u8 = 8;
    /// A punch hit (`NDrone2_DealWithObjHit` 0x13fff0 case 5 → msg 6 with the HITDATA): weapon 1
    /// (fists), 0x4B and 0x4D route here instead of msg 8; 0x4A / 0x4C are the taser (msg 7, not ported).
    pub const PUNCH: u8 = 6;
    /// "Open that door" (`DroneAnim_EventFunc` 0x13b750 event 6 → `Drone_SM_SendMsg(0x20, 0, own, own)`;
    /// docs/fields/drone-statemachine.md). Only `DSTATE_OpenDoor` handles it: retry the door activation.
    pub const DOOR: u8 = 0x20;
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
    /// The grenade-throw anim state (`DroneWeap_CanThrowGrenade`'s `DroneAnim_CanDoAnimState(0x49, 0)`).
    pub const GRENADE_THROW: u16 = 0x49;
    pub const WALK_AIM: u16 = 0x0f;
    pub const STRAFE_LEFT_ALT: u16 = 0x29;
    pub const STRAFE_RIGHT_ALT: u16 = 0x2a;
    pub const ROLL_LEFT_ALT: u16 = 0x55;
    pub const ROLL_RIGHT_ALT: u16 = 0x56;
    pub const IMPACT: u16 = 0x4c;
    pub const DEATH: u16 = 0x40;
    pub const DEATH_SHOT: u16 = 0x42;
    /// The knock-out fall (`KnockedOut_Anim` ENTER: `CallAnim(0, 0x50, 0, next 0x47)` [M 0x172aa0-aa8]).
    pub const KNOCKED_OUT_FALL: u16 = 0x50;
    /// The punch stagger (`NDrone2_PunchImpact` 0x145f28:89-94: `CallAnim(0, 0x51, 0, resume)`).
    pub const PUNCHED: u16 = 0x51;
    /// CivilianHiding's cower (`NDrone2_DSTATE_CivilianHiding` 0x15d520 ENTER: `CallAnim(0x19)`).
    pub const COWER: u16 = 0x19;
    /// The stun-grenade reactions (`StunGrenadeImpact` 0x172bb0 ENTER: `Rand_Rand(2)` → `CallAnim(0, 0x2D/0x2E,
    /// 0, 0x4E, 0, 0)` — the 4th arg is the follow-on AI state 0x4E [M disasm 0x172c34-0x172c60]).
    pub const STUNNED_A: u16 = 0x2d;
    pub const STUNNED_B: u16 = 0x2e;
    /// The get-up (`StunGrenadeRecover` 0x163350 ENTER: `CallAnim(0, 0x76, 0, 0x56 non-civilian)`).
    pub const STUN_GET_UP: u16 = 0x76;
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
    /// Surface sounds (`EffectInfo`): impacts, footsteps; `None`: silent.
    pub surfaces: Option<&'a nf_formats::sfx::SurfaceSounds>,
    /// Drone voice lines (`Drone_SfxInfo`); `None`: the drones do not talk.
    pub voices: Option<&'a [nf_formats::sfx::VoiceLine]>,
    /// `GameState+0x40` (0x2a37a8): the difficulty — the `Player_HandlePain` 0x1902c8 damage
    /// factor, the `DroneWeap_DoBulletAccuracy` 0x179740 hit chance, the captain/lock/hint gates
    /// and `PlrStat_GetScore` 0x1984b0's multiplier all read it (fix-progress; mirrored from
    /// `Drones::difficulty` in the view's `data()`).
    pub difficulty: u32,
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
    /// The player's world-sphere centre (`obj+0x80`, `Control_BuildWorldSph` 0x133070): the LOS origin of the
    /// pickup take test [M 0x18e148:212-214]. `Bond::sphere_centre()` in the viewer; `pos` as the stand-in
    /// default (the pre-fix behaviour) for callers that have no body model.
    pub sph: V3,
    /// Bond hangs on a wire or zipline (substate 6 / 0xF): no volume-containment trigger contacts (triggers.rs).
    pub hanging: bool,
    /// The collision capsule's ends (top, bottom) — the geometry the trigger contact list is built
    /// from: `Trigger_Hit` 0x1b1fc8 classifies the contacts with no geometry of its own, and
    /// `Player_Collision` 0x194df0 builds the capsule per substate off obj+0x30 with the up vector.
    /// Built at the construction site off Bond's stance (`cc`).
    pub capsule: (V3, V3),
    /// The runtime cel the queries start from (obj+0x20 — fix-safe08b): the `pCelList` cell index in
    /// the world's `ScopeGraph`; `None` = the flat (unscoped) world.
    pub cel: Option<usize>,
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
    /// Scripts still fading out after being replaced (`AnimScriptAppendBlend`), oldest first (docs/systems/anim-engine.md).
    pub outgoing: Vec<crate::anim::Outgoing>,
    /// The playing script's start-frame commands ran (`+0x78 & 0x20000000` cleared).
    pub started: bool,
    /// Event 7 cleared `+0x579`: `DroneAnim_CheckForAnimComplete` ends the one-shot before its last frame.
    pub early_end: bool,
    /// `OnStartAnim`: a class-6 danim's script cut to 15 frames (`+0xB0`) unless the hit was big (`+0x1C`).
    pub len_cap: Option<u32>,
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
    /// The aim-at pitch (`AnimObjectAimAt` 0x1bfe58 computes yaw AND pitch; the port kept the yaw only).
    /// Positive = aim up; the pose negates it for the matrix (positive rotation about +right pitches down).
    pub aim_pitch: f32,
    /// Head tracking (`AnimObjectHeadTrack` 0x1bf618 via `psiBuildMatrixPalette` 0x111498, requested by
    /// `NDrone2_DoTracking` 0x1479d8 / `DoHeadTracking` 0x1472c8): the stored, smoothed head yaw / pitch
    /// relative to the animation (the sAnimObject's +0x80 / +0x88), the 0x10-track flag stand-in, and the
    /// re-decide timer `+0x358`. Both angles positive up.
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub tracking: bool,
    pub track_recheck: u32,
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
    /// Timer D (`Drone+0xCD0`): the sniper cover's 6-s return-to-cover timer.
    pub timer_d: Option<u32>,
    /// `Drone+0x330`: an explosive's position (set by `DroneFunc_ConsiderExplosive` on msg 0x21), and the run
    /// distance `+0x350` [M 0x1431c0:71-81]: 4.0 grenade-thrower weapons 0x2A/0x2C-0x2F (deadline 2 s), 10.0
    /// flashbang 0x37 (return to the caller state), 14.0 the 0x34/0x3B-0x40/0x6C explosives (deadline 10 s),
    /// 5.0 gas 0x36 and the 0x2A explosive object. The old "5.0 stun / 8.0 mine / 12.0 frag" reading was wrong.
    pub explosive_pos: V3,
    pub explosive_run_dist: f32,
    /// `+0x2B` (scared — set by ConsiderExplosive 0x1431c0:194 and Hide's ENTER 0x168d18:24, cleared by the
    /// family's EXITs), `+0x354` (the state to enter when the flee arrives; 0x87 Hide by default, the caller
    /// state for the 0x36/0x37/0x2A-object cases [M 0x1431c0:81-176]) and `+0x324` (the flee deadline,
    /// now + 10·FRAME_RATE_INT, 2·FRAME_RATE_INT for the grenade weapons [M 0x1431c0:82, 101-119]).
    pub scared: bool,
    pub flee_return: u8,
    pub flee_deadline: Option<u32>,
    /// The msg-0x21 payload `DroneFunc_HandleExplosives` (0x1435f0:51 SendMsgSelf) delivers: (position, run
    /// distance, return state, deadline frame). The port's states read it in their msg::EXPLOSIVE arms.
    pub explosive_msg: Option<(V3, f32, u8, u32)>,
    pub anim: Animator,
    // Firing (`+0x3B` firing, `+0x3C` burst done, `+0xBC0` burst, `+0xBC4` next shot, `+0xBC8` last shot, clip).
    pub firing: bool,
    pub burst_done: bool,
    pub burst: i32,
    pub next_shot: u32,
    pub last_shot: u32,
    pub clip: i32,
    pub clip_max: i32,
    /// `Drone+0xBCC`: the grenade weapon index (0x34 frag, 0x35 stun, 0x36 smoke — the disc name labels
    /// [L USATxt.dat]) and `+0xBCE` the count (decremented on throw).
    pub grenade_weapon: u16,
    pub grenade_count: i16,
    /// Set by `DroneAnim_EventFunc` datum 1 (the grenade-throw anim's release frame); `Drones::tick` calls
    /// `throw_grenade` and clears it.
    pub pending_grenade: bool,
    /// Frames of the shots fired (for flashes; the viewer drains it).
    pub shots: Vec<u32>,
    /// Root translation at the previous tick (root motion).
    pub root_prev: Option<V3>,
    /// `+0x2A` Bond's crosshair on this drone (until the given frame) and `+0x17` another drone within 3 units.
    pub aimed_at: bool,
    pub aimed_until: u32,
    pub bunched: bool,
    /// The port's copy of the `Rand_Rand(10000)` draw `Drone_Create` 0x136be0 stores to `obj+0xEC`, used as the
    /// miss-wobble phase (`t = frame + obj+0xEC`). The game's live `obj+0xEC` is **rewritten after creation** by an
    /// unidentified writer ([L] 0x1A677 > 10000 on every 07000005 drone — drone-port-check.md #4), so the port keeps
    /// its creation-time value; the live field's later meaning is an open item. `+0x41` sight lost since the last
    /// shot, `+0x100` damage multiplier.
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
    /// `control_add_object_to_list` (0x134698) prepend bookkeeping: 0 = a placement object (list position from
    /// `placement`); n > 0 = the n-th runtime spawn (list head, newest first) [M core-loop §7 item 9].
    pub spawn_seq: u32,
    /// The tick the drone spawned (`DroneSpawner_Control` runs mid-walk, so the game's cursor has already passed
    /// the head — a fresh spawn's first update is the NEXT tick).
    pub spawn_frame: u32,
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
    /// multiplier `+0x14C`, damageable `+0x14`, **voice set `+0xD8`** (the model-id switch in DefaultInit; NOT the
    /// damage docs' "type", which is dtypenew `+0xC5` / DTYPE `+0xC7` — drone-port-check.md #3; obj+0xD8 is the glist
    /// pointer, a third field), the last damage `+0x150`, head shot `+0x3A`, big hit `+0x1C`, the last flinch
    /// `+0x540`, the last hit `+0x470`, the state after a flinch `+0x118`, hits taken `+0x1D8`.
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
    /// Corpse (`obj+0xF0 & 0x20`: no longer hit or seeing), the death channel to raise (`+0xCD8` pending — confirmed
    /// in the re-check: `NDrone2_DSTATE_Death_Anim` 0x1637f8 sets `+0xCD8 = 1` at enter and raises `+0x13C` on
    /// exit/timer while set; `+0x13C` = placement parameter 12), the special-death script (`+0xBD4`, parameter 13),
    /// alpha (`obj+0x106` / `+0xCFC`). Note `+0xCD8` is a shared scratch word: the Disabled state reads it as a resume
    /// state id (0x172440), UnderCoverIdle as a tries count (0x169e58) — drone-port-check.md #2.
    pub corpse: bool,
    pub death_pending: bool,
    pub death_channel: u8,
    /// The last hit on this drone came from the player (a bullet, a punch or an owned explosion) —
    /// `Drone+0x2B4` (last-hit-by) is the player [M `DroneFunc_OnInitDeath` 0x145bd0 reads it; the port
    /// tracks the two bullet paths separately, so a flag stands in] (fix-actors).
    pub hit_by_player: bool,
    /// The pending `OnInitDeath` fail verdict, drained by the tick (fix-actors).
    pub fail_reason: Option<crate::ally::FailReason>,
    /// The wait-channel latch `+0xCD8` (`AllyLeadMissionWait`'s and the other wait states'), and the
    /// tick's read of it (fix-actors). The source word is `mission_wait_channel` (+0x12C) [M 0x15efc8].
    pub wait_channel: u16,
    pub wait_channel_on: bool,
    /// `Drone+0x12C`: the mission-wait channel word `AllyLeadMissionWait` ENTER latches (0 → 0x99)
    /// [M 0x15efc8:17-20]. Init 0 by `Drone_SM_InitObject` 0x175838; no writer in the decompile.
    pub mission_wait_channel: u16,
    /// The stun-grenade recover deadline (frame): `StunGrenadeLoop`'s `+0xCC4 = GameState._52_4_ + (+0xCE0)`
    /// [M 0x1631c0 case 1], where `+0xCE0` is `DroneFunc_RecoverTime` 0x147188's ticks. fix-explosions.
    pub stun_until: u32,
    /// The last `update_mission_route` report was BLOCKED/NO_PATH (fix-actors; Lead's PlayerInWay entry).
    pub route_blocked: bool,
    /// The distance to the nearest started, alive, team-1 drone (the "armed drone" of
    /// `NDrone2_NearArmedDrone` 0x15ea28's calls — the port snapshots positions, so the team/started
    /// filter runs where the snapshot is built; f32::MAX = none) (fix-actors).
    pub armed_peer_dist: f32,
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
    /// `+0x54C`: the turn a finished turning danim leaves (`DroneAnim_OnEndAnim` 0x13e2c0: info flags 0x100 → π,
    /// 0x80 → π/2, 0x200 → −π/2), applied to the yaw and the desired yaw by `DroneAnim_SnapRotate` (0x13e198) after
    /// the next control update.
    pub snap: f32,
    /// `DroneAnim_EventFunc` flags `+0x20` / `+0x21` (events 0xF / 0x12 set, 0x10 clears; `PlayFiringAnim` reads
    /// `+0x21`), and the holster swap (events 3 / 4: `AnimDatumSetEntity` datum 0x28 / 100 with 0x02000312).
    pub event_20: bool,
    pub event_21: bool,
    pub holstered: bool,
    /// `+0x45`: the weapon-swap id from the placement's setup word 0x17 (`DefaultInit` 0x14b300:266);
    /// `DroneAnim_EventFunc` datum 2 passes it to `DroneWeap_ChangeWeapon` (0x17a6e0's per-weapon jump
    /// table, disasm 0x13b7dc: `a1 = lb 0x45(a2)` — the drone's byte, not the event's arg). The setup
    /// word itself is not parsed yet; tests and future setup wiring set it directly.
    pub weapon_swap: u8,
    /// `DroneAnim_EventFunc` 5 / 10 / 0xB: a `DroneWeap_CloseCombatImpact` (0x1789c0) is due (the type
    /// argument: 1 → record 0x65, 2 → 0x66); drained in `Drones::tick` (needs `Data` for the record).
    pub pending_melee: Option<u8>,
    /// `DroneAnim_EventFunc` datum 2: a `DroneWeap_ChangeWeapon(drone+0x45)` is due.
    pub pending_weapon_swap: bool,
    /// `+0x3E`: the anim datum's weapon byte is 0x1C / 0x1D (`DefaultInit` 0x14b300:801-811) — the
    /// weapons whose `DroneAnim_PlayFiringAnim` (0x13da48) fires only on the anim's `+0x21` event and
    /// plays the 0x5C recoil danim. Guards here carry 0x16 / 1 / 6, so this changes nothing for them.
    pub recoil_weapon: bool,
    /// Sounds the animation played this tick (`AnimProcessScriptCmds` op 1 → `Sound_Play3D`), for the hearing list.
    pub anim_sounds: Vec<u32>,
    /// Speech (docs/systems/speech.md): the voice set (`+0xD8`), the current line's handle and id (`+0x4D0`), whether
    /// that sound has finished (set before each update), `SpeechSFX` requests of this update (line, force), and the
    /// patrol-talk cooldown (global `0x298FC4`, lent by `Drones::tick`).
    pub voice_type: u16,
    /// `+0x15`: a female voice (the default pain / death lines).
    pub female: bool,
    /// The first Attack entry has run (*suggested* stand-in for `+0x4F8 & 0x8000000`, "first sight").
    pub attack_talked: bool,
    pub voice: Option<(u32, u32)>,
    pub voice_finished: bool,
    pub talks: Vec<(usize, bool)>,
    pub patrol_cooldown: u32,
    /// Script footstep this tick (op 4 event 0 / 4 / 5) and sounds to stop (op 4 event 3, `Sound_StopAllWithId`).
    pub footstep: bool,
    pub anim_stops: Vec<u32>,
    /// Pickups this drone made this tick (`DroneWeap_DropWeapon`, `DroneFunc_DropGrenade`), collected by `Drones`.
    pub drops: Vec<crate::pickup::Pickup>,
    /// Drone modes (`behaviour_util_get`: placement parameter 15 = type `+0xC7`, 16 = the normal block's mode `+0xC8`,
    /// 21 = the alert block's `+0xC9`; `NDrone2_GetDTYPENEW` → mode `+0xC5`, alternate mode `+0xC6`), the team `+0x44`,
    /// the alternate (attack) state `+0x5A4`, and the resting alertness `Drone+0x500` (both `+0x500` and the live
    /// `+0x50C` start at this table value — `DoModeSettingsNEW` 0x14c600:70-77; the alt profile's resting value is
    /// `alert_floor` = `Drone+0x504`, read only by `GetDroneTypeAttackTypeFriend` 0x14cbd8:165). The live alertness is
    /// the `alert` field (+0x50C).
    pub dtype: u8,
    pub mode_normal: u8,
    pub mode_alert: u8,
    pub mode: u8,
    pub alt_mode: u8,
    pub team: u8,
    pub alt_state: u8,
    pub alert_block: f32,
    /// `Drone+0xCF8`: the armed-person radius that scares a Civilian (0x15d118 ENTER: 10 on 07000007, else 20).
    pub scared_radius: f32,
    /// `+0x4f8 & 8`: the hurt scream already broadcast (`DroneFunc_SendHurtMessage` 0x145e28:57 —
    /// once per life; cleared nowhere in ACTION.ELF's readers).
    pub screamed: bool,
    /// `+0x4f8 & 0x200000`: the armed-intruder shout already made (PartyGirl update 0x160140:70).
    pub scare_shouted: bool,
    /// The scream/shout broadcast queued this tick: msg id + the alert source position
    /// (`Drone_SM_BroadcastMsg`'s alert struct carries the shouter's position 0x145e28:41-53).
    pub pending_shout: Option<(u8, V3)>,
    /// `Drone+0xBB0`: the tick of the last `NDrone2_OpenDoor` attempt — `ControlSTANDARD` re-tries
    /// only 60 ticks later [M 0x1490f0:242].
    pub door_open_at: u32,
    /// `Drone+0xBA8`: the door object this drone is working — stood in by the route's door nodes
    /// (`NDrone2_ReachedDestNode` 0x153358) or `NDrone2_FindElevatorDoor` 0x14a940 (sniper/elevator
    /// states, not ported); read by `NDrone2_OpenDoor` 0x14aa38 / `DoorIsOpen` 0x14aaa0 /
    /// `ControlSTANDARD` 0x1490f0:232 (cleared when the door is > 2.0 away) / `LinkCreep_Handler`
    /// 0x15a2c8 (the blocked test is skipped while it stands). Port index into `Doors::list`.
    pub hit_door: Option<usize>,
    /// `Drone+0x5A6`: the state `DSTATE_OpenDoor` returns to once the door is open — the state the
    /// drone was in when `ReachedDestNode` stood the door in [M 0x153358:159].
    pub door_return: u8,
    /// Outbox: the route node just reached that carries the door flag (bit 1). `Drones::tick`
    /// resolves it to the nearest door (`NDrone_InitDoorNodes` 0x139a70 binding) and stands it in
    /// `hit_door`.
    pub door_node: Option<u16>,
    /// Outbox: `NDrone2_OpenDoor` requested this tick (DSTATE_OpenDoor ENTER / timer C / msg 0x20,
    /// or `ControlSTANDARD`'s 60-tick bump check). Serviced by `Drones::tick` where the doors live.
    pub door_attempt: bool,
    /// Outbox: anim event 6 fired (`DroneAnim_EventFunc` case 6 → msg 0x20 to self).
    pub pending_door_msg: bool,
    /// The `NDrone2_DoorIsOpen` (0x14aaa0) verdict for `hit_door`, refreshed by `Drones::tick` each
    /// tick (true when there is no door in the way; else the door's state == 4).
    pub hit_door_open: bool,
    /// `Drone+0xBA8`'s kick side (`NDrone_InitKickNodes` 0x139ce0 binds the nearest class-0x1b
    /// ScriptPlayer whose script is 0x60005bb at the kick node's +0x28; `ReachedDestNode` stands it
    /// in `+0xBA8` like a door). Port index into `Scripts::players`. `NDrone2_KickObject` 0x14aa00
    /// unpauses it (`SP_UnPause` 0x1dca58); the game's one slot is shared with `hit_door`, but a
    /// route never carries a door pair and a kick pair at once, so the port keeps them apart.
    pub kick_sp: Option<usize>,
    /// Outbox: the route node just reached that carries the kick flag (bit 2). `Drones::tick`
    /// resolves it to the bound ScriptPlayer and — on the node's first crossing — enters
    /// `DSTATE_KickObject` (the game's node-flag 0x8000, shared by every drone).
    pub kick_node: Option<u16>,
    /// Outbox: the route node just reached that carries the ally-wait flag 0x20 (`ReachedDestNode`
    /// 0x153358: the `switch_channels[0x99]` gate lives in `Drones`, like the door bindings).
    pub ally_wait_node: Option<u16>,
    /// Outbox: `NDrone2_KickObject` requested this tick (DSTATE_KickObject's timer C / msg 0x20).
    /// Serviced by `Drones::tick` where the ScriptPlayers live: `SP_UnPause` 0x1dca58 on the bound
    /// player (`+0xBA8`), i.e. `activated = true` and the current script's pause cleared.
    pub kick_attempt: bool,
    /// Outbox: `Drone_SfxInfo` one-shot indices queued by the state arms (fix-room-sound) — the
    /// civilian patrol's female scream 0x48 (`NDrone2_PlaySFX(1.0, DCVars, 0x48, 0, 1)` [M
    /// 0x15d610 disasm 0x15d754-a4]). Drained by `Drones::tick` through `foley_entry` (the game's
    /// shared gates: the 30-frame post-load, the entry's repeatDelay, the stamp-before-play).
    pub foley_reqs: Vec<usize>,
    /// `comm+0x73c`'s stand-in for `BOT_handlePain` (0x125ed8): the handle of the currently playing
    /// pain grunt, stopped before the next pool play (`Sound_Stop_Ref` 0x1de370) [M 0x126168-0x1261a0].
    pub pain_grunt: Option<u32>,
    /// `Drone+0x31C`: `DroneMove_NoBunching`'s (0x150ad0) obstacle flags, rebuilt every tick by
    /// `Drones::tick` from `NDrone2_Steer`'s (0x14eba8) nearest-obstacle scan (`Drone+0x314`,
    /// the nearest drone or the player within 3.0 — `EvaluateObstacle` 0x14e6f0's sqDist < 9 gate):
    /// bit 2 = the obstacle is within 45° of the movement direction (ahead), 4 = to the side,
    /// 8/0x10 = which side (the decomp drops the ATAN2 store, so the side sign is [*suggested*];
    /// bit 2 is sign-independent). Read at the top of `DroneAnim_SetCombatMoveAnim` (0x13cfb0:35:
    /// bit 2 → state 0x5E instead of a move anim) and in the strafe selection (:78/:247/:281).
    pub no_bunching: u32,
    pub alpha: f32,
    /// A switch channel to raise after the drone's update (`SetDeathChannel`).
    pub raise: Option<u8>,
    /// `Drone+0x480`: per-tick velocity; gravity accumulates here and `pos += velocity · REC_FRAME_RATE` (`Drone_CollisionHandler`
    /// 0x140968, WORKLOG 0078). Zero on the ground; the down-ray (`Drone_FeetOnPoint`) sets the on-surface flag.
    pub velocity: V3,
    /// The port's "off the ground" flag — its own model: the down-ray (`Drone_FeetOnPoint`) missed a walkable
    /// surface within 0.2 below the feet. NOT the game's `Drone+0x4F8 & 0x4000`: the decompile has no setter for that
    /// bit — only the per-tick clear at the top of `Drone_CollisionHandler` 0x140968:70 and the `0x4200` tests in the
    /// armed-peer selection (docs/fields/drone.md, bit census) — the game's real gravity gates are bits 0x80 / 0x20 /
    /// 0x200 (the port's `bounded` / `corpse`).
    pub in_air: bool,
    /// `anim+0x60 & 8`: feet on a walkable surface this tick (`Drone_FeetOnPoint`).
    pub on_surface: bool,
    /// Trace of (frame, state) changes, for comparisons with PCSX2.
    pub log: Vec<(u32, u8)>,
    /// `Drone+0x4F8 & 0x1000000`: knocked out (`KnockedOut_Anim` ENTER [M 0x172ab0-c4]). Makes
    /// `OnInitDeath` / `LogDroneDeath` skip `PlrStat_LogEnemyDispatched` [M 0x145bd0:22,57-59].
    pub knocked_out: bool,
    /// `PlrStat_LogEnemyDisabled` ran for this drone (the KO enter, team 1 [M 0x172ac8-dc]).
    pub disabled_logged: bool,
    /// `PlrStat_LogEnemyDispatched` ran (a death without the KO flag — `OnInitDeath` 0x145bd0:57-59).
    pub dispatched_logged: bool,
    /// `Drone+0x554`: the drone's start script (placement param 1; `NDrone2_DefaultInit` 0x14b300:
    /// `+0x554 = piVar19[0xE]` = params[1], 0 when the word is 0x6000000 or one of the 0x6000892/0x600089c
    /// specials). Non-zero → `NDrone2_DSTATE_Global`'s enter starts the drone in PlayScript (3).
    pub start_script: u32,
    /// The surrender tally outbox: `PlrStat_LogEnemySurrender(0)` 0x1997b8 runs in the
    /// `Surrender_Anim` ENTER (0x1626a0) — folded into `Drones.progress` by the tick (fix-progress).
    pub surrender_logged: bool,
    /// The shot-hit-enemy tally outbox (count this tick): `NDrone2_HitDamage` 0x145558:57-60's
    /// `PlrStat_LogShotHitEnemy` (the team-1 + hit-by-the-player gate) — folded by the tick.
    pub hit_log: u32,
}

/// Normalise a vector (the grenade throw direction helper).
fn norm(v: V3) -> V3 { let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt(); if l > 0.0 { [v[0] / l, v[1] / l, v[2] / l] } else { v } }

/// Rodrigues' rotation about the unit `axis` by `ang` radians, as a 4×4 (the pose rotations of
/// `AnimObjectHeadTrack` 0x1bf618 / `AnimObjectAimAt` 0x1bfe58 — the game builds the same rotation with
/// `Quat_AxisAngleToQuat` 0x1becb8 + `Quat_QuatToMat`).
fn axis_angle_mat(axis: [f32; 3], ang: f32) -> nf_formats::anim::Mat4 {
    let (s, c) = (ang.sin(), ang.cos());
    let t = 1.0 - c;
    let [x, y, z] = axis;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y, 0.0],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x, 0.0],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
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
        // `NDrone2_DefaultInit` (0x14b300 lines 348-376): `+0xBCC` and `+0xBCE` come from **param 11** —
        // `uVar4 = piVar19[0x18]`, and params start at `piVar19[13]` (`Drone_Create` 0x136be0 copies
        // `placement+0x2C`, 0x84 bytes, to `Drone+0xC34` = `piVar19[13]`), so `piVar19[0x18]` = params[11] =
        // `placement+0x2C+44` (drone-port-check.md #1: the old `param(24)` read the alert block's middle word).
        // `*(short*)(+0xbcc) = (short)(uVar4 >> 4)`; `*(ushort*)(+0xbce) = raw & 0xf`; switch on `(raw >> 4) & 0xffff`:
        // 1→0x36(smoke), 2→0x35(stun), 4→0x34(frag), 5→count+0x3a, 6→0x6c count=1; default→0,0. The
        // constants are the game's own [M NDrone2_DefaultInit 0x14b300:351-372]; the names are the disc
        // labels [L USATxt.dat: 0x34 'Fragmentation Grenade', 0x35 'Stun Grenade', 0x36 'Smoke Grenade'].
        // On 0x7000014 a weapon in (0x34, 0x37) clamps to 0x34 (0x14b300 lines 374-376; drone.md param-11 row).
        let p11 = param(11);
        let gw_sel = ((p11 >> 4) & 0xffff) as u16;
        let gc_raw = (p11 & 0xf) as u16;
        let (mut grenade_weapon, grenade_count) = match gw_sel {
            1 => (0x36u16, gc_raw as i16),
            2 => (0x35u16, gc_raw as i16),
            4 => (0x34u16, gc_raw as i16),
            5 => ((gc_raw.wrapping_add(0x3a)) as u16, 1i16),
            6 => (0x6cu16, 1i16),
            _ => (0u16, 0i16),
        };
        if d.level == 0x0700_0014 && (0x34..0x37).contains(&grenade_weapon) { grenade_weapon = 0x34; }
        // `Drone+0x554` (NDrone2_DefaultInit 0x14b300: `+0x554 = piVar19[0xE]` = params[1]): the start
        // script. 0x6000000 and the 0x6000892/0x600089c dtypenew specials leave it 0.
        let start_script = match param(1) { 0x6000000 | 0x6000892 | 0x600089c => 0, s => s };
        let mut dr = Drone {
            skin, anim_set, weapon, pos, yaw, desired_yaw: yaw, aim_yaw: 0.0, alert, alert_floor: alert, props: normal, alert_props,
            sight_range: e8, half_fov: crate::trig::HALF_PI, engage_range: f0, combat_range: fc,
            // `drone_stats` for the guards' type is all zero at run time (07000005 snapshot) → defaults.
            accuracy: 5, aggression: 0, reaction: 0,
            los: false, seen: false, ever_seen: false, visibility: 0.0, seen_ticks: 0, unseen_ticks: 0, vision_on: false,
            dist: 0.0, yaw_to_opp: 0.0,
            state: state::GLOBAL, prev_state: 0, next_state: None, entered: 0, timer_c: None, timer_d: None,
            explosive_pos: pos, explosive_run_dist: 0.0,
            anim: Animator { danim: 0, script: None, time: 0, state: 0, class: 0, call: None, reached: false, call_from: 0, blend_wait: 0, ftime: 0.0, speed: 1.0, variant: 0, once: false, held: false, follow_on: 0, outgoing: Vec::new(), started: false, early_end: false, len_cap: None },
            firing: false, burst_done: false, burst: 0, next_shot: 0, last_shot: 0, clip, clip_max: clip,
            grenade_weapon, grenade_count, pending_grenade: false,
            shots: Vec::new(), log: Vec::new(), knocked_out: false, disabled_logged: false, dispatched_logged: false, start_script, root_prev: None, aimed_at: false, aimed_until: 0, bunched: false, wobble: rng.rand(10000), lost_sight: true, damage_mul: 1.0, alert_bits: 0, light: 1.0, attack_frame: 0, alerted_by_drone: false, watching: false,
            // fix-progress tally outboxes (fields on their own lines — merge lesson 4):
            surrender_logged: false,
            hit_log: 0,
            route: None, last_wait: None, idle_timeout: None, delayed: None, sm_id: 0, stand_until: 0,
            croute: None, nav_path: None, move_quad: 0, yaw_offset: 0.0, no_route_odd: false, target: PlayerTarget::default(), links_used: Vec::new(),
            switch_channel: param(3) as u8, channel_on: false, placement: 0, spawn_seq: 0, spawn_frame: 0, group: param(4), present: true, started: false, drawn: true, enabled: true, index: 0, exiting_to: 0, self_msg: None,
            cover: None, cover_side: 0, cover_type: 0, cover_leave: 0, cover_point: pos, goal: None, cover_claimed: false, has_cover: false, cover_tries: 0,
            cover_rt: Vec::new(), others: Vec::new(), peers: Vec::new(), return_state: 0, second_channel: param(6) as u8, second_tag: param(8),
            loudness: 0.0, sound_share: 0.0, heard_frame: 0, heard_peak: 0.0, sound_pos: pos, noise_level: 0, investigating: false, turned: false, resume_state: 0, home: pos,
            // `drone_stats[type]+8` is 0 for these guards → 10 (captains, difficulty ≥ 2 only, are not modelled).
            health: 10.0, max_health: 10.0, damage_taken: 1.0, damageable: !matches!(skin, 0x0500_003b | 0x0500_003c | 0x0500_0053), kind: setup.kind,
            last_damage: 0.0, silent_kill: false, big_hit: false, flinch_at: None, hit: None, impact_resume: 0, hits_taken: 0,
            global_flinch: None, bond_aiming: false, cull: false, bond_armed: true, opp_facing: 0.0, opp_target: [0.0; 3], sighted: false, own_sight: false, first_attack_done: false,
            corpse: false, death_pending: false, hit_by_player: false, fail_reason: None, armed_peer_dist: f32::MAX, wait_channel: 0, wait_channel_on: false, route_blocked: false,
            mission_wait_channel: 0,
            stun_until: 0,
            // fix-finale-blockers: 07000016's chaser (skin 0x0500001e, dtype 0xf, no placement death
            // channel [D]) carries 0x90 at init — the defeat stand-in documented in kiko.rs [S].
            death_channel: crate::kiko::chaser_death_channel(d.level, skin, param(15) as u8, param(12) as u8),
            special_death: param(13), special_spot: pos, special_yaw: yaw,
            stationary: param(13) != 0 && param(13) != 0x0600_021f, alpha: 255.0, raise: None,
            head_yaw: 0.0, head_pitch: 0.0, tracking: false, track_recheck: 0, aim_pitch: 0.0,
            captain: false, snap: 0.0, event_20: false, event_21: false, holstered: false, anim_sounds: Vec::new(), voice_type: voice_type(skin), female: matches!(skin & 0xffff, 0x0e | 0x0f | 0x10 | 0x11 | 0x52 | 0x15 | 0x20 | 0x22..=0x24 | 0x17 | 0x18 | 0x2b | 0x2d | 0x34 | 0x3b | 0x3c | 0x5e), attack_talked: false, voice: None, voice_finished: true, talks: Vec::new(), patrol_cooldown: 0, footstep: false, anim_stops: Vec::new(), drops: Vec::new(), free_root: false, bounded: true, dtype: param(15) as u8, mode_normal: param(16) as u8, mode_alert: param(21) as u8, mode: 0, alt_mode: 0, team: 1, alt_state: 0, scared_radius: 20.0, screamed: false, scare_shouted: false, pending_shout: None,
            velocity: [0.0; 3], in_air: false, on_surface: true,
            door_open_at: 0,
            hit_door: None, door_return: 0, door_node: None, door_attempt: false, pending_door_msg: false, hit_door_open: true,
            // fix-nav-remainder: the kick-node outbox + the ScriptPlayer binding (fields on
            // their own lines — the shared-constructor collisions, merge lesson 4).
            kick_sp: None, kick_node: None, ally_wait_node: None, kick_attempt: false, no_bunching: 0,
            // fix-room-sound: the foley one-shot outbox + the pain-grunt handle (own lines — merge lesson 4).
            foley_reqs: Vec::new(), pain_grunt: None,
            alert_block: alert_props.map_or(0.0, |w| nf_formats::drone::ALERTNESS_LEVELS[d.props.get(0x20, &w) as usize & 3]),
            // fix-cover-remainder: the scary-object flee family (fields on their own lines — merge lesson 4).
            scared: false, flee_return: 0, flee_deadline: None, explosive_msg: None,
            // fix-anim-events: `+0x45` (the setup's swap id — not parsed from the setup word yet), the
            // EventFunc outboxes, and `+0x3E` (the anim datum's weapon byte 0x1C / 0x1D, DefaultInit
            // 0x14b300:801-811; the port keys it on the drone's weapon id — the datum byte's id space).
            weapon_swap: 0, pending_melee: None, pending_weapon_swap: false,
            recoil_weapon: weapon == 0x1c || weapon == 0x1d,
        };
        dr.init_properties(d, &param);
        dr.type_setup(d);
        dr.captain_setup(d);
        dr.set_danim(d, start_danim, 0);
        dr
    }

    /// `NDrone2_DefaultInit` after the mode settings: a captain (`+0x19`) by difficulty — property 9 on 1, 5 on 2, 8 on
    /// 3 — for kinds 1, 2, 3, 6, 7, each with its captain skin (0x5000038, 0x500002F, 0x5000006, 0x5000093, 0x50000B2);
    /// health × `DroneCaptain_Mod_Health`, `+0x100` = `Mod_BulletDamage`, accuracy `+0xB4` × 1 / `Mod_BulletAccuracy`
    /// (truncated). Type 0xC drops properties 0x14–0x16 (kind 9 also gets 0x4C).
    fn captain_setup(&mut self, d: &Data) {
        use nf_formats::ps2float::mul;
        let by_difficulty = match d.difficulty { 1 => self.prop(d, 9), 2 => self.prop(d, 5), 3 => self.prop(d, 8), _ => false };
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
    /// Takes `&mut self`: the 07000007 cases WRITE properties [M 0x14c938:37-39/67-69] — dtype 10/0xb
    /// (the party guests) get prop 0x1f = 0 (deaf to sound alerts, `HandleSoundAlerts` 0x148a30:45) and
    /// prop 0x54 = 1 (the protected guest), dtype 0xd gets 0x1f = 0; the port dropped these writes and the
    /// guests heard every punch (playtest item 3: the crowd went hostile and gun-posed).
    fn dtype_of(&mut self, d: &Data, ty: u8, block: u8) -> u8 {
        if block == 0 && self.prop(d, 0x10) { return 0x10; }
        let lvl7 = d.level == 0x0700_0007;
        match ty {
            0..=9 if lvl7 && block == 0 => return if self.prop(d, 0x4b) { 0x16 } else { 0x13 },
            // GetDTYPENEW 0x14c938 case 10/0xb: below full alertness a FEMALE (+0x15) becomes mode 0x12
            // (PartyGirlInit), any other 9 (CivilianInit) — the port had the non-damageable test here.
            10 | 11 if block == 0 => {
                if lvl7 {
                    d.props.set(0x1f, &mut self.props, 0);
                    d.props.set(0x54, &mut self.props, 1);
                }
                return if self.alert < 1.0 { if self.female { 0x12 } else { 9 } } else { 10 };
            }
            0xc if block == 0 => return if self.kind == 0xc { 0x11 } else { 0xc },
            0xd if block == 0 => {
                if lvl7 { d.props.set(0x1f, &mut self.props, 0); } // [M 0x14c938:67-69]
                return if self.kind == 0xf { 0x17 } else if self.alert < 1.0 { 9 } else { 10 };
            }
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

    pub fn prop(&self, d: &Data, id: usize) -> bool { d.props.get(id, &self.props) != 0 }

    /// "Stays put": `+0x4F8 & 0x10` or property 0x40, tested together throughout the decomp (`DroneFunc_CombatState`,
    /// `ChooseCombatMove`, CombatNoRoute, AimStandFire, HeardNoiseSuspect / Alert, …).
    fn stays(&self, d: &Data) -> bool { self.stationary || self.prop(d, 0x40) }

    /// `Drone+0x5A2`, the state a drone starts in once enabled.
    /// Idle / Alert / InitPatrol become Attack when the drone is fully alert (`GetDroneTypeAttackTypeFriend`): [L] the
    /// 0xA4 waiters 271 / 272 (alertness 1) went straight into combat when the channel pulsed (WORKLOG 0028).
    /// The mode's start state from `DroneTypeSettings`, or for −1 by mode (2 → SniperIdle, or SniperAim when fully
    /// alert; 4 → 8, 7 → 0x85, 0x10 → 0x37, 0x13 → 0x30, 0x16 → 0x31, 0x18 → 0x36; else InitPatrol with property
    /// 0x24 / 0x25, Idle without); then, at alertness 1, the attack versions.
    pub fn start_state(&self, d: &Data) -> u8 {
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
        // 07000014 always asks for 0x39.
        let a = if d.level == 0x0700_0014 { 0x39 } else { ((self.sm_id + 1) % 3) as u16 + anim_state::WALK };
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
            // The door nodes: the same `ReachedDestNode` dispatch the mission route runs
            // (patrolling guards and party guests open the doors on their paths).
            let route_path = route.path;
            let next = route.nodes.iter().position(|&x| x == node).and_then(|i| route.nodes.get(i + 1)).copied();
            drop(route);
            if self.route_reached_special(d, route_path, Some(node), next) { return; } // the OpenDoor anim is set
        }
        let alerted = self.prop(d, 0x25) && self.alert >= 0.66;
        match code {
            crate::nav::follow::MOVING => { let a = if alerted { anim_state::ALERT_WALK } else { self.walk_anim(d) }; self.request_anim(d, a) }
            crate::nav::follow::NO_PATH => self.request_anim(d, anim_state::STAND_IDLE),
            _ => { let a = if self.alert >= 0.66 { anim_state::ALERT_IDLE } else { self.walk_anim(d) }; self.request_anim(d, a) }
        }
        self.desired_yaw = atan2_approx(target[0] - self.pos[0], target[2] - self.pos[2]);
    }

    /// CivilianMission's route half (`UpdateMissionRoute` 0x154a80 = `NDrone2_FollowRoute`): `moving_anim` while the
    /// route MOVES (each game DSTATE picks its own — CivilianMission walks, `DSTATE_AllyLead` 0x15e698 case 3 calls
    /// `CallAnim(0x1e)` = RUN on code 0/2), look-around/stand when the creep is blocked (the game's route creep byte
    /// 9/0xc — the port's BLOCKED code), and `true` once the route ARRIVED (the caller clears prop 0x1D and falls to
    /// Civilian).
    fn update_mission_route(&mut self, d: &Data, _rng: &mut Rng, _frame: u32, moving_anim: u16) -> bool {
        let feet = self.feet();
        let Some(route) = self.route.as_mut() else { self.request_anim(d, anim_state::STAND_IDLE); return false };
        let (code, reached) = route.follow(d.nav, feet);
        let target = route.target;
        // `NDrone2_ReachedDestNode` (0x153358, via `LinkCreep_Handler` 0x15a2c8): a reached door node
        // parks the drone in `DSTATE_OpenDoor` (fix-npc-doors-escort); the kick/action/fiddle nodes
        // dispatch the same way.
        let route_path = route.path;
        let next = reached.and_then(|n| route.nodes.iter().position(|&x| x == n)).and_then(|i| route.nodes.get(i + 1)).copied();
        drop(route);
        self.route_reached_special(d, route_path, reached, next);
        match code {
            crate::nav::follow::ARRIVED => { self.desired_yaw = atan2_approx(target[0] - self.pos[0], target[2] - self.pos[2]); true }
            crate::nav::follow::BLOCKED | crate::nav::follow::NO_PATH => {
                self.route_blocked = true;
                if self.alert >= 0.66 { self.request_anim(d, anim_state::ALERT_IDLE) } else { self.set_stand_idle(d, _rng, _frame, false) }
                false
            }
            _ => { self.route_blocked = false; self.request_anim(d, moving_anim); self.desired_yaw = atan2_approx(target[0] - self.pos[0], target[2] - self.pos[2]); false }
        }
    }

    /// `NDrone2_ReachedDestNode` (0x153358) at a reached special node — the full flag dispatch in the
    /// game's order (door 2 beats kick 4 beats action 8; 0x20 and fiddle 0x40 only when none of
    /// those). Door bit 1 with a door-node successor — the doorway's node pair [M :149-163] — stands
    /// the node's own bound door (`NDrone_InitDoorNodes` 0x139a70 wrote it at node+0x28) in
    /// `Drone+0xBA8`, saves the current state in `Drone+0x5A6` and enters `DSTATE_OpenDoor` (0x81).
    /// On the pair's far side (no door-node successor) `NDrone2_DoorIsOpen` (0x14aaa0) decides: open
    /// → the tail clears +0xBA8 [:175]; closed → stay (the opener is still working the door).
    /// Kick bit 2 with a kick-node successor [M :135-153]: stand the node's bound ScriptPlayer
    /// (`NDrone_InitKickNodes` 0x139ce0, the class-0x1b script-0x60005bb object at node+0x28) in
    /// +0xBA8 and — on the node's first crossing only (the game sets node flag 0x8000, never
    /// cleared) — save +0x5A6 and enter `DSTATE_KickObject` (0x82). Anywhere else on a kick node the
    /// tail clears the binding. Action bit 3 [M :112-124] → `DSTATE_ActionAnim` (0x83) with +0x5A6
    /// saved (no +0xBA8 use); fiddle bit 0x40 [M :96-107] → `DSTATE_StandFiddle` (0x84) likewise;
    /// ally-wait bit 0x20 [M :85-94] parks the drone in state 0x21 with the return state saved in
    /// +0x5A2 — gated on `switch_channels[0x99] == 0`, which `Drones::tick` owns like the door
    /// bindings (outbox `ally_wait_node`). The Mayhew lines (flags 0x10000/0x20000/0x40000/0x80000
    /// [M :57-77] → `Drone_MayhewTalk` 0x44/0x45/0x46/0x47) fire for every reached node that carries
    /// them, before the dispatch. The bindings and the channel gate resolve in `Drones::tick`, which
    /// owns the doors / the ScriptPlayers / the channels. Returns whether a special state was entered.
    fn route_reached_special(&mut self, d: &Data, route_path: usize, reached: Option<u16>, next: Option<u16>) -> bool {
        let Some(node) = reached else { return false };
        let flags = d.nav.paths[route_path].nodes[node as usize].flags;
        let next_kick = next.map_or(false, |n| d.nav.paths[route_path].nodes[n as usize].flags & 4 != 0);
        let next_door = next.map_or(false, |n| d.nav.paths[route_path].nodes[n as usize].flags & 2 != 0);
        // The Mayhew lines [M 0x153358:57-77]: flags 0x10000/0x40000/0x20000/0x80000 → `Drone_MayhewTalk`
        // (0x149818) 0x44/0x46/0x45/0x47 — `NDrone2_SpeechSFX(idx, 0, 0, 0)`, non-forced. The game's
        // gate (the type-0x0c drone's +0x4F8 chain, `Drone_MayhewTalk`:24-40) applies where the node
        // is reached — on 07000001 that is Mayhew's own kind-8 route, so the walker is the speaker
        // [*suggested*: the port pushes on the walker, not the type-0x0c drone].
        if flags & 0x10000 != 0 { self.talks.push((0x44, false)); }
        if flags & 0x40000 != 0 { self.talks.push((0x46, false)); }
        if flags & 0x20000 != 0 { self.talks.push((0x45, false)); }
        if flags & 0x80000 != 0 { self.talks.push((0x47, false)); }
        if flags & 2 != 0 {
            if next_door {
                self.door_node = Some(node);
                self.door_return = self.state;
                self.set_state(state::OPEN_DOOR);
                return true;
            }
            if self.hit_door_open { self.hit_door = None; } // [M 0x153358:175]
            return false;
        }
        if flags & 4 != 0 {
            // [M 0x153358:135-153]: no successor → the tail clears +0xBA8; a kick successor binds
            // this node's object (the first crossing enters 0x82); otherwise the tail clears too.
            match next {
                Some(_) if next_kick => { self.kick_node = Some(node); }
                _ => self.kick_sp = None,
            }
            return false;
        }
        if flags & 8 != 0 {
            // [M 0x153358:112-124]: +0xBAC = the node; +0x5A6 saved when not already 0x83; → 0x83.
            if self.state != state::ACTION_ANIM { self.door_return = self.state; }
            self.set_state(state::ACTION_ANIM);
            return true;
        }
        if flags & 0x20 != 0 {
            // [M 0x153358:85-94]: only while switch channel 0x99 is off (tick reads the channels);
            // +0x5A2 = the current state, → 0x21 (stay when already there).
            self.ally_wait_node = Some(node);
            return false;
        }
        if flags & 0x40 != 0 {
            // [M 0x153358:96-107]: +0xBAC = the node; +0x5A6 saved when not already 0x84; → 0x84.
            if self.state != state::STAND_FIDDLE { self.door_return = self.state; }
            self.set_state(state::STAND_FIDDLE);
            return true;
        }
        false
    }

    // ---- animation ---------------------------------------------------------------------------------------------

    /// `DroneAnim_SetDAnimInternal` (0x13aee8): start a danim's script for our anim set. With no script, or for a drone
    /// that was not drawn (`obj+0xFC` = 0) given a danim with info flag 4 of class 0, 3–7 or 9, the danim ends at once
    /// (`SetEndAnim(0x1E)`) and its follow-on (`+10`, when not itself) starts instead — e.g. an off-screen guard's
    /// look-around 265 (class 3, flags 6) goes straight to alert idle 244.
    /// `DroneAnim_SetHTAnim`'s script switch: no blend (or a drone not animating, `NDrone2_DoAnimation`) →
    /// `AnimScriptAdd`, every script deleted; else `AnimScriptAppendBlend`: the scripts already fading restart an
    /// 8-frame fade, the one just replaced fades over `blend` frames.
    /// `old_turn`: the replaced danim is a 180° turn (info flag 0x100, `+0x574`) not yet at its end — `SetHTAnim` then
    /// freezes it (speed 0), fades it over 30 frames in anim rotation, and leaves the turn it had made (`+0x54C` = its
    /// root yaw) for `SnapRotate`: the yaw takes the turn and the fading pose loses it.
    fn begin_blend(&mut self, d: &Data, blend: u16, old_turn: bool) {
        if blend == 0 || !self.drawn { self.anim.outgoing.clear(); return; }
        for o in &mut self.anim.outgoing { o.clock = 8.0; o.length = 8.0; }
        if let Some(s) = self.anim.script {
            let looping = !(self.anim.once || self.anim.held);
            let mut o = crate::anim::Outgoing { script: s, time: 1.0 + self.anim.ftime, speed: self.anim.speed, looping, clock: blend as f32, length: blend as f32, unyaw: 0.0 };
            if old_turn {
                let yaw = self.local_pose(d).and_then(|p| p.first().map(|r| crate::anim::root_yaw(r.1))).unwrap_or(0.0);
                o.speed = 0.0;
                o.clock = 30.0;
                o.length = 30.0;
                o.unyaw = yaw;
                // `SnapRotate` (applied at once, as for the end snap; U-039): the yaw = the desired yaw + the turn.
                self.desired_yaw += yaw;
                self.yaw = self.desired_yaw;
            }
            self.anim.outgoing.push(o);
            if self.anim.outgoing.len() > 31 { self.anim.outgoing.remove(0); }
        }
    }

    /// `DroneAnim_SetDAnimInternal(danim, blend, …)` (0x13aee8): the danim's state, class and call bookkeeping; then,
    /// when it has a script for the anim set (or set 0) and is not skipped (an undrawn drone, info flag 4, classes 0 /
    /// 3–7 / 9), `SetHTAnim` starts it. Otherwise `SetEndAnim` goes to the follow-on — and with none, **the previous
    /// script keeps playing** (no `SetHTAnim`): e.g. an unarmed guard (anim set 0xC) has no death or corpse script, so
    /// the last one holds instead of an empty pose.
    fn set_danim(&mut self, d: &Data, danim: u16, blend: u16) {
        // The replaced danim: a 180° turn (info flag 0x100) still running.
        let old_info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
        let old_turn = self.anim.danim != 0 && old_info[1] & 1 != 0 && self.anim.script.is_some() && !self.anim.held;
        let mut danim = danim;
        for _ in 0..8 {
            let info = d.tables.info.get(danim as usize).copied().unwrap_or([0; 24]);
            self.anim.danim = danim;
            self.anim.state = u16::from_le_bytes([info[8], info[9]]);
            self.anim.class = info[0xc];
            self.anim.blend_wait = 0;
            self.anim.follow_on = 0;
            // A danim of the called state (0x41..0x44, 0x50 and 0x6F count as 0x40) or of state 0 marks the call
            // reached (`+0x57B`) and zeroes the request (`ZeroAnimCall`; the completion action `+0x596`/`+0x598` stays).
            let want = self.anim.call.filter(|_| !self.anim.reached).map_or(0, |(w, _, _)| match w { 0x41..=0x44 | 0x50 | 0x6f => 0x40, w => w });
            if self.anim.state == 0 || self.anim.state == want { self.anim.reached = true; }
            let script = d.tables.script_for(danim, self.anim_set);
            let skip = script.is_none() || (!self.drawn && info[0] & 4 != 0 && matches!(self.anim.class, 0 | 3..=7 | 9));
            if !skip {
                self.begin_blend(d, blend, old_turn);
                self.anim.script = script;
                self.anim.time = 0;
                self.anim.ftime = 0.0;
                let speed = f32::from_le_bytes([info[0x14], info[0x15], info[0x16], info[0x17]]);
                self.anim.speed = if speed > 0.0 && speed.is_finite() { speed } else { 1.0 };
                self.anim.once = info[0] & 1 == 0;
                self.anim.held = false;
                self.anim.started = false;
                self.anim.early_end = false;
                // `DroneAnim_OnStartAnim`: class 6 without a big hit (`+0x1C`) is cut to 15 frames, end blend 5.
                self.anim.len_cap = (self.anim.class == 6 && !self.big_hit).then_some(15);
                // `DAnimFunc_Run` (the only `DroneData_AnimFunc` entry): the run danim 0xDC plays at 0.7 on 07000009 / 0A.
                if danim == 0xdc && matches!(d.level, 0x0700_0009 | 0x0700_000a) { self.anim.speed = 0.7; }
                // A new script has no previous root: no motion until its second tick (a call starting a danim after
                // this tick's root motion would otherwise move by the old sequence's root minus the new one's).
                self.root_prev = None;
                break;
            }
            let next = u16::from_le_bytes([info[10], info[11]]);
            if next == 0 || next == danim { break; }
            danim = next;
        }
    }

    /// `DroneAnim_SetScript(script, state, follow-on, …, next AI state, message, flags)` (0x13e088): play a script by
    /// hashcode as anim state `state`, class 1, the call already reached with the given completion; flags 0 = one-shot.
    fn play_script(&mut self, d: &Data, hash: u32, state: u16, follow_on: u16, next_ai: u8, done_msg: u8) {
        let old_info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
        let old_turn = self.anim.danim != 0 && old_info[1] & 1 != 0 && self.anim.script.is_some() && !self.anim.held;
        self.begin_blend(d, 8, old_turn); // `SetScript` → `SetHTAnim(…, blend 8)`
        self.anim.danim = 0;
        self.anim.state = state;
        self.anim.class = 1;
        self.anim.script = Some(hash);
        self.anim.started = false;
        self.anim.early_end = false;
        self.anim.len_cap = None;
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
            self.set_danim(d, t.danim as u16, t.b4 as u16);
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
        // The fading scripts keep playing (`AnimScriptTick`) while their blend clocks run down (`AnimFrameResolve`).
        self.anim.outgoing.retain_mut(|o| { let len = d.scripts.get(&o.script).map_or(1.0, |s| s.length as f32); o.tick(len) });
        if self.anim.held { self.root_prev = None; return self.call_handler(d); }
        let before = (self.anim.script, self.anim.danim);
        let ftime_in = self.anim.ftime;
        // `AnimScriptTick`: a new script runs the commands of its start frame first.
        if !self.anim.started { self.anim.started = true; self.script_commands(d, 0, 1, false); }
        let prev_t = 1 + self.anim.time;
        // Playback speed (`Drone_AnimInfo +0x14`, passed to the script by `SetHTAnim`): [L] lean-out 176 (51 frames at
        // 1.5) lasted 35 ticks live (work/chase_trace.tsv).
        self.anim.ftime += self.anim.speed;
        self.anim.time = self.anim.ftime as u32;
        let len = self.anim.script.and_then(|h| d.scripts.get(&h)).map(|s| s.length as u32).unwrap_or(1);
        let len = self.anim.len_cap.map_or(len, |c| len.min(c));
        // The commands between last tick's frame and this one's (`AnimProcessScriptCmds`), wrap-aware.
        let script_before = self.anim.script;
        let to = (1 + self.anim.time).min(len);
        let wrapped = self.anim.time >= len && !self.anim.once && script_before.is_some();
        self.script_commands(d, prev_t, if wrapped { 1 + self.anim.time - len } else { to }, wrapped);
        let mut restarted = false;
        let mut wrap_end = None;
        let early = self.anim.early_end && self.anim.once && self.anim.time < len;
        if early {
            // `CheckForAnimComplete` (0x13dae8): `+0x57A` set and `+0x579` cleared by event 7 → `SetEndAnim` now.
            self.anim.early_end = false;
            let info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
            let next = if self.anim.danim == 0 { self.anim.follow_on } else { u16::from_le_bytes([info[10], info[11]]) };
            if next != 0 && next != self.anim.danim {
                let end_blend = if self.anim.len_cap.is_some() { 5 } else { info[0xe] as u16 };
                self.set_danim(d, next, end_blend);
                restarted = true;
            } else { self.anim.once = false; self.anim.held = true; }
        } else if self.anim.time >= len {
            // The wrap tick repeats the script's last step (last frame − the one before): [L] aligned per-tick steps of
            // a live walk cycle (115 ticks, work/patrol_trace.tsv) show the step before the wrap twice.
            // fix-anim-blend2: the last-two-frames sampling must not write the live float clock — `AnimSeqTick`
            // 0x1c28d0 only READS script.time (+0x90) (seq.frame = time − start + 1.0); writing `ftime` here made
            // `begin_blend`'s outgoing start time (`1.0 + ftime`, the replaced script's clock) land one frame early,
            // and every blended script switch then replayed the old script's last step on the switch tick — at speed
            // 1.0 the pre-blend port moved 0 there (its integer sampler clamps to the sequence's last frame), so
            // guards drifted ~0.01 per switch and the AI timelines (fire ticks, the cover-type Rand) shifted [L
            // chase_trace seed 1: the pick flipped to the step-out type → danim 178 (90 frames at 1.0) instead of
            // lean-out 176 (51 at 1.5) → UnderCoverAim 90 vs live 37].
            let last = self.root_value_at(d, len.saturating_sub(1));
            wrap_end = self.root_value_at(d, len.saturating_sub(2)).zip(last).map(|(a, b)| [b[0] - a[0], 0.0, b[2] - a[2]]);
            let info = d.tables.info.get(self.anim.danim as usize).copied().unwrap_or([0; 24]);
            let next = if self.anim.danim == 0 { self.anim.follow_on } else { u16::from_le_bytes([info[10], info[11]]) };
            if next != 0 && next != self.anim.danim {
                // `SetEndAnim` → `OnEndAnim`: a turning danim leaves its turn for `SnapRotate` (the next danim starts
                // facing the way the animation ended).
                let flags = u32::from_le_bytes([info[0], info[1], info[2], info[3]]);
                if flags & 0x100 != 0 { self.snap = crate::trig::PI; }
                if flags & 0x80 != 0 { self.snap = crate::trig::HALF_PI; }
                if flags & 0x200 != 0 { self.snap = -crate::trig::HALF_PI; }
                // *Deviation (order)*: the game applies it in the next tick's `SnapRotate`, after that tick's control;
                // applied here at once so a state entered on the call's completion (e.g. Surrendered facing Bond) is
                // not turned by it afterwards (the order of completion vs snap is not read, U-039).
                // The follow-on blends over the ending danim's `info+0xE` frames (`+0x584`), 0 for a snapped turn.
                let mut end_blend = if self.snap != 0.0 { 0 } else { info[0xe] as u16 };
                if self.anim.len_cap.is_some() { end_blend = 5; }
                // `SnapRotate`: desired += the turn, the yaw = desired.
                if self.snap != 0.0 { self.desired_yaw += self.snap; self.yaw = self.desired_yaw; self.snap = 0.0; }
                self.set_danim(d, next, end_blend)
            }
            else if self.anim.once {
                // `AnimEndCallback` + `CheckForAnimComplete`: the script stops on its last frame; `+0x57A` clears.
                self.anim.once = false;
                self.anim.held = true;
                self.anim.time = len.saturating_sub(1);
                self.anim.ftime = self.anim.time as f32;
            } else {
                self.anim.ftime = (ftime_in - len as f32).max(0.0);
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

    /// `AnimProcessScriptCmds` for the playing script over frames (from, to] (from 0 = the start frame; `wrapped`: to the
    /// end and again from 1): op 1 `[frame, sfx]` → a sound at the drone; op 4 `[frame, event, arg]` → event 2 calls
    /// `DroneAnim_EventFunc(arg)`; events 0 / 4 / 5 a footstep (toggle / left / right foot: `ScriptFootstepType` →
    /// `AnimObjectUpdate` → `Effect_Create(floor hit, 0x42 | foot)`: the floor surface's footstep sound; footprints
    /// not ported), event 3 `Sound_StopAllWithId(arg)` [M `AnimProcessScriptCmds` 0x1c1b38; the walk scripts carry
    /// events 4 / 5, e.g. 0600025A frames 19 / 57].
    fn script_commands(&mut self, d: &Data, from: u32, to: u32, wrapped: bool) {
        let Some(script) = self.anim.script.and_then(|h| d.scripts.get(&h)) else { return };
        let len = script.length as u32;
        let hit = |f: u32| if wrapped { f > from && f <= len || f <= to } else { f > from && f <= to };
        let mut events = Vec::new();
        for c in &script.commands {
            match (c.op, c.args.as_slice()) {
                (1, [f, sfx, ..]) if hit(*f as u32) => self.anim_sounds.push(*sfx as u32),
                (4, [f, 2, arg, ..]) if hit(*f as u32) => events.push(*arg),
                (4, [f, 0 | 4 | 5, ..]) if hit(*f as u32) => self.footstep = true,
                (4, [f, 3, id, ..]) if hit(*f as u32) => self.anim_stops.push(*id as u32),
                _ => {}
            }
        }
        for e in events { self.event_func(e); }
    }

    /// `DroneAnim_EventFunc` (0x13b750): 3 / 4 holster swap, 7 `+0x579` = 0, 0xF `+0x20`, 0x12 `+0x20` and `+0x21`, 0x10
    /// clears both, 1 the grenade-throw release (sets `pending_grenade` → `Drones::tick` calls `throw_grenade`), 7 the
    /// early end of a one-shot (`+0x579`), 5 / 10 / 0xB the close-combat impacts (`DroneWeap_CloseCombatImpact` 0x1789c0
    /// with the type argument), 9 the tertiary placement — `DroneWeap_PlaceTertiary` 0x17a848 is a tail call to
    /// `DroneWeap_ThrowGrenade` — and 2 the weapon swap (`DroneWeap_ChangeWeapon` 0x17a6e0 by the drone's `+0x45` byte,
    /// disasm 0x13b7dc: `a1 = lb 0x45(a2)` in the dispatch's shared delay slot — the drone's byte, not the event arg).
    /// The game gates 1 / 5 / 6 / 9 / 0xB / 0xE on `0.0 < drone+0xAC` and `script == *(drone+0x530)`; the port's
    /// `script_commands` only ever runs the current script, so the script gate holds structurally and the health gate is
    /// explicit here. Not ported: 0xE (`Drone_SM_SendMsg(0xB, 10, …)` to the `+0x2B0` partner's SMI +0x108 — the port has
    /// no partner messaging) and the datum-entity swap behind 3 / 4 (the port's `holstered` bool stands in).
    fn event_func(&mut self, e: u16) {
        let alive = self.health > 0.0;
        match e {
            3 => self.holstered = true,
            7 => self.anim.early_end = true,
            4 => self.holstered = false,
            // `DroneAnim_EventFunc` datum 1: the grenade-throw anim's release frame → `DroneWeap_ThrowGrenade`
            // (the actual throw is deferred to `Drones::tick` so it can borrow `Data` and the RNG).
            1 if alive => self.pending_grenade = true,
            // `DroneAnim_EventFunc` event 6: "open that door" — msg 0x20 to self (the DSTATE_OpenDoor
            // handler re-activates +0xBA8; elsewhere the GLOBAL fallback drops it) [M 0x13b750:60-64].
            6 if alive => self.pending_door_msg = true,
            // Datum 2 → `DroneWeap_ChangeWeapon(drone+0x45)`; no health / script gate in the game
            // (disasm 0x13b7dc follows the jump table directly).
            2 => self.pending_weapon_swap = true,
            // 5 / 10 → `DroneWeap_CloseCombatImpact(dcv, 1)`, 0xB → (dcv, 2) [disasm 0x13b900/0x13b93c].
            5 | 10 if alive => self.pending_melee = Some(1),
            0xb if alive => self.pending_melee = Some(2),
            // Datum 9 → `DroneWeap_PlaceTertiary` 0x17a848 = `DroneWeap_ThrowGrenade` [M decomp].
            9 if alive => self.pending_grenade = true,
            0xf => self.event_20 = true,
            0x12 => { self.event_20 = true; self.event_21 = true; }
            0x10 => { self.event_20 = false; self.event_21 = false; }
            _ => {}
        }
    }

    /// `DroneWeap_CloseCombatImpact` (0x1789c0): the melee `Bullet_init(-1, obj, obj, record, pos, dir, 0xffff)` —
    /// record 99 with the 2.0 range when the weapon class short `+0xDA` is 7 (the kick), else the type's record (100,
    /// or 0x65 for type 1 / 0x66 for type 2) with the 1.75 range and the `|drone+0x1D4| ≤ 1.0471976` (60°) facing gate.
    /// No accuracy roll, no muzzle light, no sound — `Bullet_init` straight from the record. The position is obj+0x30
    /// (the drone's position — the game plays the anim sounds at the same vector), the direction the facing.
    fn close_combat_impact(&self, d: &Data, kind: u8, owner: usize) -> Option<crate::bullet::Bullet> {
        // [S] `drone+0xDA` short = the drone's weapon id: the port's `self.weapon` indexes the same records.
        let kick = self.weapon == 7;
        let record = if kick { 99usize } else { match kind { 1 => 0x65, 2 => 0x66, _ => 100 } };
        let range = if kick { 2.0f32 } else { 1.75 };
        if self.dist > range { return None; }
        if !kick && angle_difference(self.yaw, self.yaw_to_opp).abs() > 1.047_197_6 { return None; }
        let w = d.weapons.records.get(record)?;
        let f32_at = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let (s, c) = (crate::trig::sinf(self.yaw), crate::trig::cosf(self.yaw));
        Some(crate::bullet::Bullet {
            pos: self.pos, dir: [s, 0.0, c], speed: f32_at(0x20), travelled: 0.0, range: f32_at(0x1c),
            damage: f32_at(0x0c), owner, state: 0, weapon: record as u16,
            flyby: d.weapons.flyby(record), bounces: 0,
        })
    }

    /// The sequence and frame the playing script shows. The LAST-started opcode-0 window covering the
    /// script time owns the root bone (`AnimProcessScriptCmds` 0x1c1b38 starts every window; the later
    /// one overwrites — [L] the 07000006 driver's face track, started after his body track, pins his
    /// root to its all-zero track so he holds position while the body's root runs 0.57; the free
    /// `anim::script_frame` is last-wins the same way, fix-drone2).
    pub fn script_frame(&self, d: &Data) -> Option<(u32, f32)> {
        let script = d.scripts.get(&self.anim.script?)?;
        // The window follows the integer tick (the game pins the seq slots when its commands run); the frame keeps
        // the fraction — `AnimSeqTick` 0x1c28d0: seq.frame = script.time(+0x90, float) − start + 1.0. At speed 1.0
        // this is the old integer frame exactly; at a fractional speed (Drone_AnimInfo +0x14, e.g. lean-out 176 at
        // 1.5) the root motion now samples the same f/f+1 blend the pose does.
        let t = (1 + self.anim.time) as u16;
        let (a, _, q) = script.sequences().filter(|(a, b, _)| *a <= t && t <= *b).last().or_else(|| script.sequences().next())?;
        Some((q, (1.0 + self.anim.ftime - a as f32 + 1.0).max(1.0)))
    }

    /// `root_value` at an explicit script time — the wrap-end step's last-two-frames sample. Reads the sequence
    /// window at `1 + time` with the float clock `time` exactly as `AnimSeqTick` 0x1c28d0 would
    /// (seq.frame = time − start + 1.0) WITHOUT touching the playing clock (`script_frame`/`root_value`).
    fn root_value_at(&self, d: &Data, time: u32) -> Option<V3> {
        let script = d.scripts.get(&self.anim.script?)?;
        let t = (1 + time) as u16;
        let (a, _, q) = script.sequences().filter(|(a, b, _)| *a <= t && t <= *b).last().or_else(|| script.sequences().next())?;
        let f = (1.0 + time as f32 - a as f32 + 1.0).max(1.0);
        let s = d.sequences.get(&q).filter(|s| s.channels.len() >= 3)?;
        Some(crate::anim::root_translation_at(s, f))
    }

    /// The root translation at the playing script's (fractional) frame — `AnimFrameSet` 0x1c75a0's ⌊f⌋/⌊f⌋+1 blend
    /// (`anim::root_translation_at`); the posed bones sample the fraction the same way, so the stance foot stays
    /// planted instead of stepping only on whole frames.
    fn root_value(&self, d: &Data) -> Option<V3> {
        self.script_frame(d).and_then(|(q, f)| d.sequences.get(&q).filter(|s| s.channels.len() >= 3).map(|s| {
            crate::anim::root_translation_at(s, f)
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
        // Blending (`AnimFrameResolve`): the fading scripts' root steps this tick mixed in the pose order — each next
        // step by 1 − the previous weight, the current one last.
        let delta = if self.anim.outgoing.is_empty() { delta } else {
            let steps: Vec<([f32; 2], f32)> = self.anim.outgoing.iter().map(|o| {
                let at = |t: f32| -> Option<V3> {
                    let (q, f) = crate::anim::script_frame(d.scripts.get(&o.script)?, t)?;
                    let s = d.sequences.get(&q).filter(|s| s.channels.len() >= 3)?;
                    Some(crate::anim::root_translation_at(s, f))
                };
                let step = match (at(o.time), at(o.time - o.speed)) { (Some(a), Some(b)) if o.speed != 0.0 && a[0].is_finite() => [a[0] - b[0], a[2] - b[2]], _ => [0.0; 2] };
                (step, o.weight())
            }).collect();
            let mut m = steps[0].0;
            let mut w = steps[0].1;
            for (st, wn) in steps.iter().skip(1) { for k in 0..2 { m[k] += (st[k] - m[k]) * (1.0 - w); } w = *wn; }
            let cur = delta.unwrap_or([0.0; 2]);
            for k in 0..2 { m[k] += (cur[k] - m[k]) * (1.0 - w); }
            Some(m)
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

    /// Moves x/z by `step` unless the body (0.4 above the feet sphere … head) would cross collision, then leaves the
    /// height to `collision_handler` (the game's `Drone_CollisionHandler` down-ray; WORKLOG 0078). The earlier
    /// `floor_below(2.0)` stand-in is gone — a guard dying at a ledge now falls.
    fn move_by(&mut self, d: &Data, step: [f32; 2]) {
        let to = [self.pos[0] + step[0], self.pos[1], self.pos[2] + step[1]];
        let probe = |y: f32| d.world.line_of_sight([self.pos[0], y, self.pos[2]], [to[0] + step[0].signum() * 0.4 * (step[0].abs() > 1e-6) as i32 as f32, y, to[2] + step[1].signum() * 0.4 * (step[1].abs() > 1e-6) as i32 as f32]);
        if probe(self.pos[1]) && probe(self.pos[1] + 0.5) {
            self.pos = to;
        }
    }

    // ---- damage and death (docs/systems/drone-damage.md) ---------------------------------------------------------

    /// Each state's `DroneFunc_HandleImpact(msg, resume, allow)` for msg 8: (resume state, 0 = the current one; whether
    /// a flinch is allowed). `None`: the state ignores bullets.
    fn impact_args(s: u8) -> Option<(u8, bool)> {
        use state::*;
        Some(match s {
            // The BASE handler itself: `Drone_SM_RouteMsgDCV` routes an unhandled message to GLOBAL, and
            // `DroneFunc_HandleImpact` (0x1469b8) IS the base DSTATE — its case 8 (0x1469b8:134-166) runs
            // `NDrone2_BulletImpact(d, hit, resume)` with the resume 0 unconditionally (the damage applies;
            // the +0x118 stamp and the state 0x55 entry need a valid resume). Without this arm every state
            // the port has no DSTATE for (07000004's seated target, state 0xb2) swallowed the player's
            // bullets — the impacts registered, the health never moved.
            GLOBAL => (0, false),
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
            // The mission states (fix-party-mission, playtest: bullets on party guests did nothing):
            // PartyGirl 0x160140 case 8 → HandleImpact(0x12, 0); Civilian 0x15d118 case 8 → (0x12, 0);
            // CivilianHiding 0x15d520 case 8 → (0x12, 0); CivilianScared 0x15d3a0 case 8 → (+0x118, 0);
            // CivilianMission 0x15d860 case 8 → (0x56, 0); CivilianPatrol 0x15d610 case 8 → (0x56, 0);
            // CivilianDoorGuard 0x160648 case 8 → (0x56, 0); TruckDriverInit 0x160bd0 case 8 → (0x33, 0);
            // TruckDriverIdle 0x160ed0 case 8 → (0x12, 0).
            PARTY_GIRL | CIVILIAN | CIVILIAN_HIDING | TRUCK_DRIVER_IDLE => (CIVILIAN_SCARED, false),
            CIVILIAN_MISSION | CIVILIAN_PATROL | CIVILIAN_DOOR_GUARD => (ATTACK, false),
            TRUCK_DRIVER_INIT => (TRUCK_DRIVER_INIT_ALERT, false),
            CIVILIAN_SCARED => (0, false),
            _ => return None,
        })
    }

    /// Msg 8 in state `s`: the BulletImpact state calls `NDrone2_BulletImpact(…, 0, 0)` itself; the others go through
    /// `DroneFunc_HandleImpact`.
    fn on_bullet(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8) -> bool {
        self.on_impact(d, rng, frame, s, false)
    }

    /// Msg 6 in state `s` (the punch). The per-state `HandleImpact` arguments are the SAME as msg 8's for every state
    /// that handles the punch (a scan of all 250 DSTATE decomps, 2026-10-12: 46 states carry case 6, each with the
    /// resume/allow of its case 8) — only the impact handler differs (`NDrone2_PunchImpact` vs `NDrone2_BulletImpact`).
    /// The mission states pass their own pairs, mirroring the port's msg-8 arms 1:1.
    fn on_punch(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8) -> bool {
        use state::*;
        // The per-state resume/allow for the mission states (msg 6 == msg 8 [M DSTATE scan]).
        let (resume, allow) = match s {
            TRUCK_DRIVER_INIT => (TRUCK_DRIVER_INIT_ALERT, false), // 0x160bd0 case 6: HandleImpact(0x33, 0)
            TRUCK_DRIVER_INIT_ALERT => (0, false),
            TRUCK_DRIVER_IDLE | PARTY_GIRL | CIVILIAN | CIVILIAN_SCARED | CIVILIAN_HIDING => (CIVILIAN_SCARED, false),
            CIVILIAN_MISSION | CIVILIAN_GUARD | CIVILIAN_PATROL | CIVILIAN_DOOR_GUARD => (ATTACK, false), // 0x15d860 case 6 tail: HandleImpact(0x56, 0)
            HOSTAGE_IDLE => (0, false),
            _ => return self.on_impact(d, rng, frame, s, true),
        };
        self.on_punch_resume(d, rng, frame, resume, allow)
    }

    /// The shared msg-8/msg-6 path for the states that use the shared `impact_args` table. Msg 8 in the BulletImpact
    /// state calls `NDrone2_BulletImpact(…, 0, 0)` itself (0x173098); msg 6 has no case in that DSTATE and is dropped
    /// (unhandled → GLOBAL → dropped). `KnockedOut_Anim` (0x172a30 case 8/9) routes any bullet to Death_Anim — it
    /// arrives only via a scripted Message, since the bullet scan skips corpses.
    fn on_impact(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8, punch: bool) -> bool {
        // The stun family: a bullet (msg 8/9) on a stunned guard → Death_Anim 0x44 [M 0x172bb0/0x1631c0/0x163350
        // case 8/9 → `Drone_SM_SetState(0x44)`]. msg 6 (punch) has no case in the family (dropped).
        if matches!(s, state::STUN_GRENADE_IMPACT | state::STUN_GRENADE_LOOP | state::STUN_GRENADE_RECOVER) {
            if punch { return false; }
            self.set_state(state::DEATH_ANIM);
            return true;
        }
        if s == state::KNOCKED_OUT_ANIM || s == state::KNOCKED_OUT {
            // KnockedOut_Anim case 8/9 and Knocked_Out case 8/9 → Death_Anim (0x172a30 / 0x172b48); msg 6
            // has no case there (KnockedOut_Anim) and is dropped. These arrive only via a scripted Message —
            // the bullet scan skips corpses.
            if punch { return false; }
            self.set_state(state::DEATH_ANIM);
            return true;
        }
        if s == state::PUNCH_IMPACT {
            // DSTATE_PunchImpact (0x172ee0): msg 6/8 → HandleImpact(0, 0) — the punch replays without the
            // stagger (allow 0), the bullet takes the normal BulletImpact route with resume 0.
            return if punch { self.punch_impact(d, rng, frame, 0, false) } else { self.on_bullet_resume(d, rng, frame, 0, false) };
        }
        if !punch && s == state::BULLET_IMPACT { self.bullet_impact(d, rng, frame, 0, false); return true; }
        let Some((resume, allow)) = Self::impact_args(s) else { return false };
        if punch { self.on_punch_resume(d, rng, frame, resume, allow) } else { self.on_bullet_resume(d, rng, frame, resume, allow) }
    }

    /// `DroneFunc_HandleImpact` case 8 with the state's own resume/allow (the mission states pass their own:
    /// TruckDriverInit 0x160bd0 case 8 → `HandleImpact(0x33, 0)`, the civilians → their scared state, …).
    fn on_bullet_resume(&mut self, d: &Data, rng: &mut Rng, frame: u32, resume: u8, allow: bool) -> bool {
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

    /// `DroneFunc_HandleImpact` case 6 (0x1469b8:34-57): allow = 0 → `NDrone2_PunchImpact(…, resume, 0)` directly;
    /// allow ≠ 0 → state 0x53 `PunchImpact` (its ENTER replays the punch with allow = 1 → the stagger anim),
    /// with the resume stamped into +0x118 first (0x1469b8:40-53).
    fn on_punch_resume(&mut self, d: &Data, rng: &mut Rng, frame: u32, resume: u8, allow: bool) -> bool {
        if allow {
            let u10 = if resume != 0 { resume } else { self.state };
            if u10 != 0 && !(0x53..=0x55).contains(&u10) { self.impact_resume = u10; }
            self.set_state(state::PUNCH_IMPACT); // the HITDATA rides `self.hit` (the port's stand-in for the state param)
        } else {
            self.punch_impact(d, rng, frame, resume, false);
        }
        true
    }

    /// `NDrone2_PunchImpact` (0x145f28). The punch damage runs through `DoHitEffects` like a bullet; at health ≤ 0 the
    /// drone enters state 0x42 `KnockedOut_Anim` (mode 0x1E, the bot-punch mode, would go to 0xF2 — bots are not
    /// ported). Alive with `allow`: the stagger anim 0x51 with follow-on `+0x118` and both flinch stamps; otherwise
    /// the resume state (for the common (ATTACK, false) pairs this is what turns a punched guard hostile at once).
    /// `DroneFunc_SendHurtMessage` (the hurt broadcast, prop 0x32) and the level-7/9-A mission-fail reasons are not
    /// modelled, matching the port's `bullet_impact`.
    fn punch_impact(&mut self, d: &Data, _rng: &mut Rng, frame: u32, p3: u8, allow: bool) -> bool {
        let valid = |x: u8| x != 0 && !(0x53..=0x55).contains(&x);
        // 0x145f28:23-42: the value STAMPED into +0x118 is p3 (or the current state when p3 == 0) if valid;
        // the state finally ENTERED is p3 (or 0 when it equals the current state) if valid — two different
        // values (a punch while already in the resume state re-stamps but enters nothing).
        let stored = if p3 != 0 { p3 } else { self.state };
        if valid(stored) { self.impact_resume = stored; }
        let setv = if p3 == self.state { 0 } else { p3 };
        self.do_hit_effects(d);
        // 0x145f28:45-56: on 07000007 a punch on an UNARMED drone (ObjectIsArmed 0x178768 on the drone's
        // own class byte — DoModeSettingsOLD 0x14cfa0:159-163 zeroes it for the party modes 9/10/0x13) is
        // an instant mission fail (reason 5 → "Cover Lost" ch 99 [M 0x13e560]); the port's fail plumbing
        // drains `fail_reason` next to `raise` (fix-actors). Punching a guard = the playtest's automatic fail.
        if d.level == 0x0700_0007 && !self.obj_armed() {
            self.fail_reason = Some(crate::ally::mission_fail_reason(5));
        }
        if self.health <= 0.0 {
            // 0x145f28:60-84: enemy or not, mode != 0x1E → 0x42 (the KO); 0x1E → 0xF2 (bots, not ported).
            self.set_state(state::KNOCKED_OUT_ANIM);
            return true;
        }
        // `Drone_AlertStatusSet(2)` [M 0x145f28:30] — the port's alert-status-2 = alert ≥ 0.66
        // (the convention crate::drone line ~2703 uses for the stun family).
        self.alert = self.alert.max(1.0);
        self.pain_talk(d);
        // `DroneFunc_SendHurtMessage` 0x145e28:19-57 (prop 0x32 = the scream property, the +8
        // once-per-life bit, +0xc5 != 2): the hurt civilian SCREAMS — msg 0x12 broadcast with the
        // victim as the alert source. The queue drains in the shared pass (the send needs the
        // other drones, unreachable from inside this one's borrow).
        if self.health > 0.0 && self.prop(d, 0x32) && !self.screamed {
            self.screamed = true;
            self.pending_shout = Some((msg::SCREAM, self.pos));
        }
        if allow {
            // `CallAnim(0, 0x51, 0, *(Drone+0x118), 0, 0)` + both flinch stamps (0x145f28:89-94).
            self.call_anim(d, anim_state::PUNCHED, self.impact_resume, 0);
            self.global_flinch = Some(frame);
            self.flinch_at = Some(frame);
            return true;
        }
        if valid(setv) { self.set_state(setv); }
        true
    }

    /// `NDrone2_ObjectIsArmed` 0x178768 on this drone's own class byte: classes 2..0x47 and 0x6a..0x71 are
    /// armed, 0x47 and 0/1 are not. (The port's `weapon` mirrors obj+0xdc+0x62; the party bouncers carry
    /// 0/1 until their DrawWeapon exit sets 6.)
    fn obj_armed(&self) -> bool {
        (2..0x47).contains(&self.weapon) || (0x6a..=0x71).contains(&self.weapon)
    }

    /// The upper-body aim gate. The game's `AnimObjectAimAt` (0x1bfe58, from `psiBuildMatrixPalette` 0x111498)
    /// runs only while the anim object carries the 0x180 aim flag — requested by the aim/fire states — and returns
    /// at once when the drone has no weapon datum out (`AnimDatumGetWeaponInfo < 0`). The port: the aim/fire state
    /// family with a drawn weapon (holstered or unarmed = no torso aim). The exact 0x80 setter was not pinned this
    /// session (the state list is the stand-in; *suggested* for the list, *measured* for the weapon gate).
    fn aims_torso(&self) -> bool {
        use state::*;
        if self.holstered { return false; }
        if !((2..=0x46).contains(&self.weapon) || (0x6a..=0x71).contains(&self.weapon)) { return false; }
        matches!(self.state, SNIPER_AIM | SNIPER_FIRE | COMBAT | COMBAT_NO_MOVE | COMBAT_OUT_OF_RANGE | COMBAT_NO_SIGHT
            | COMBAT_NO_ROUTE | COMBAT_DRONE_AHEAD | AIM_STAND | AIM_STAND_FIRE | AIM_STAND_RELOAD | AIM_BACKOFF
            | AIM_CROUCH | AIM_CROUCH_FIRE | AIM_CROUCH_RELOAD | STEP_AIM_LEFT | STEP_AIM_RIGHT | STRAFE_AIM_LEFT | STRAFE_AIM_RIGHT)
    }

    /// `NDrone2_NearArmedPerson` (0x1787c8) [M — disassembly checked, no float read anywhere]:
    /// the float arg every caller passes (8/10/20/6, `+0xCF8`) is DEAD in the shipped build — the
    /// gate is exactly: kind 0xD (the party guests) never qualifies [0x178840]; prop 0x3c set
    /// [0x178850]; Bond in sight (`+0x228 & 4`, the reaction-acked sight — `DroneVision_HaveOpponentSight`
    /// 0x175ef0 sets it only past `DroneFunc_ReactionTime`) [0x178864]; and
    /// `DroneWeap_OpponentHasWeapon` 0x17a4b8 = `ObjectIsArmed` 0x178768 on Bond's class byte
    /// (`bond_armed`) [0x178874].
    fn near_armed_person(&self, d: &Data) -> bool {
        self.kind != 0xd && self.prop(d, 0x3c) && self.seen && self.bond_armed
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
        let r = crate::damage::hit_damage(d.tuning, target, nf_formats::ps2float::mul(h.damage, self.damage_taken), h.part, h.weapon, false, d.difficulty, d.level);
        // `NDrone2_HitDamage` 0x145558:57-60 (and the explosive arm :129-130): a hit on a team-1
        // drone from the player (+0x2B4 == the player object) logs the accuracy numerator
        // (`PlrStat_LogShotHitEnemy` 0x199840). Outboxed; the tick folds it into the tally.
        if self.team == 1 { self.hit_log = self.hit_log.saturating_add(1); }
        if r.head { self.silent_kill = true; }
        self.health -= r.loss;
        self.last_damage = r.recorded;
        // `NDrone2_HitDamage`: a hit that leaves the drone alive → `NDrone2_PainTalk`.
        if self.health > 0.0 { self.pain_talk(d); }
    }

    /// `DroneAnim_LocationImpactAnim` (0x13c8b8): the flinch 0x4C, variant 5 + Rand(8), or by the part hit when Bond
    /// aims; then the state `+0x118`.
    fn location_impact_anim(&mut self, d: &Data, rng: &mut Rng) {
        let v = if !self.bond_aiming { 5 + rng.rand(8) } else {
            match self.hit.map_or(0, |h| h.part) { 4 | 5 => 5, 0x12..=0x15 => 8, 0x1e | 0x1f | 0x20 | 0x23 => 9, 0x31..=0x33 => 13, 0x35..=0x37 => 12, _ => 6 + rng.rand(2) }
        };
        self.call_anim_variant(d, anim_state::IMPACT, v as u16, self.impact_resume, 0);
    }

    /// The "busy" test of the talk functions: a referenced line still playing; a finished one is dropped.
    fn voice_busy(&mut self) -> bool {
        if self.voice.is_some() && !self.voice_finished { return true; }
        if self.voice_finished { self.voice = None; }
        false
    }

    /// The line playing (`+0x4D0`'s node id), for "skip if the same id is the current line".
    fn current_line(&self) -> Option<u32> { self.voice.map(|v| v.1) }

    fn line_sfx(d: &Data, idx: usize) -> u32 { d.voices.and_then(|v| v.get(idx)).map_or(0, |l| l.sfx) }

    /// `NDrone2_PainTalk` (0x13eeb8): forced, skipped only when the same line is the current one.
    fn pain_talk(&mut self, d: &Data) {
        let idx = match self.voice_type { 1 => 0x12, 2 => 0x13, 3 => 0x11, 7 => 0x14, 8 | 10 => 0x18, 9 => 0x15, 0xc => 0x1b, 0x12 | 0x13 => 0x1c, 0x14 => 0x10,
            0x10 => return, // HitTalkExtra (type 0x10) not modelled
            _ => if self.female { 0x17 } else { 0xf } };
        if self.current_line() != Some(Self::line_sfx(d, idx)) { self.talks.push((idx, true)); }
    }

    /// `NDrone2_DeathTalk` (0x13eff8): forced; the second call of a death sees the first's line and is skipped.
    fn death_talk(&mut self, d: &Data) {
        let idx = match self.voice_type { 1 => 4, 2 => 5, 3 => 3, 8 | 10 => 10, 0xc => 0xd, 0x10 => 8, 0x12 | 0x13 => 0xe, 0x14 => 2, 5 | 6 => 1,
            _ => if self.female { 9 } else { 1 } };
        if self.current_line() != Some(Self::line_sfx(d, idx)) { self.talks.push((idx, true)); }
    }

    /// `NDrone2_AttackTalk` (0x13eac0) for the single-player voice sets of these levels: type 2 picks 0x37 + Rand(9)
    /// then speaks 1 in 3; type 1 0x33 + Rand(4) at 1 in 2 (0xA3 + Rand(8) always on 0x11 / 0x0B / 0x13); type 3 0xAC +
    /// Rand(7). Others: *not modelled* (level-specific sets).
    fn attack_talk(&mut self, d: &Data, rng: &mut Rng) {
        if self.voice_busy() || self.health <= 0.0 { return; }
        let (odds, idx) = match self.voice_type {
            1 if matches!(d.level, 0x0700_0011 | 0x0700_000b | 0x0700_0013) => (0, 0xa3 + rng.rand(8) as usize),
            1 => (2, 0x33 + rng.rand(4) as usize),
            2 => (3, 0x37 + rng.rand(9) as usize),
            3 => (0, 0xac + rng.rand(7) as usize),
            _ => return,
        };
        if odds == 0 || rng.rand(odds) == 0 { self.talks.push((idx, false)); }
    }

    /// `NDrone2_SeenPlayerTalk` (0x13f1d0): 1 in 3, then the level's set (05–08: type 2 0x49 + Rand(6), else 0x21;
    /// 09 / 0A: 0xC3 + Rand(9); 0x12 / 0x4A: 0xAC + Rand(7); 0x14–0x16 *not modelled*).
    fn seen_player_talk(&mut self, d: &Data, rng: &mut Rng) {
        if self.voice_busy() || self.health <= 0.0 { return; }
        if rng.rand(3) != 0 || self.voice_type == 0x10 { return; }
        let idx = match d.level {
            0x0700_0005..=0x0700_0008 => if self.voice_type == 2 { 0x49 + rng.rand(6) as usize } else { 0x21 },
            0x0700_0009 | 0x0700_000a => 0xc3 + rng.rand(9) as usize,
            0x0700_0012 | 0x0700_004a => 0xac + rng.rand(7) as usize,
            _ => return,
        };
        self.talks.push((idx, false));
    }

    /// `NDrone2_PatrolTalk` (0x13e988): every patrol frame while the global cooldown is open: Rand(2) ≠ 0 → cooldown
    /// 5 s; then only type 3 speaks (0xB3 + Rand(5), cooldown 10 s).
    fn patrol_talk(&mut self, rng: &mut Rng, frame: u32) {
        if self.health <= 0.0 || self.voice_busy() || frame < self.patrol_cooldown { return; }
        if rng.rand(2) != 0 { self.patrol_cooldown = frame + 300; return; }
        if self.voice_type != 3 { return; }
        let idx = 0xb3 + rng.rand(5) as usize;
        self.patrol_cooldown = frame + 600;
        self.talks.push((idx, false));
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
        self.death_talk(d);
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
                self.death_pending = true;
                // `DroneFunc_OnInitDeath` 0x145bd0's mission-fail half (fix-actors, corners per
                // docs/fields/drone.md): the protected-actor props 0x53/0x54 — team 1 dies → "Cover Lost"
                // (ch 99), team 2/3 → "Killed Civilians" (ch 0x60); the monitor's standing hooks turn the
                // channels into the fail [M 0x1827d8]. The KO check opens the game's team-1 arm
                // [M 0x145bd0:20-22]: a punched-out team-1 guard never fails the mission.
                if !(self.team == 1 && self.knocked_out) {
                    if let Some(r) = crate::ally::on_init_death(self.team, self.prop(d, 0x53), self.prop(d, 0x54), self.hit_by_player) {
                        self.fail_reason = Some(r); // drained by the tick, next to `raise` (fix-actors)
                    }
                }
                // `OnInitDeath` 0x145bd0:57-59: the player's kill is logged as dispatched UNLESS the KO flag
                // (+0x4F8 & 0x1000000) is set — the KO already logged EnemyDisabled instead [*suggested*: the
                // game also requires the last hit (+0x2B4) to be the player, which in SP it always is].
                if self.team == 1 && !self.knocked_out { self.dispatched_logged = true; }
                if !self.silent_kill { self.death_talk(d); }
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
                    self.play_script(d, self.special_death, anim_state::DEATH, 0xbf, DEAD, msg::ANIM_DONE);
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
            // Death_Anim's msg-2 raises the death channel every update while `+0xCD8` is set — NOT on
            // the anim's end [M 0x1637f8 case 2: `if (+0xcd8 != 0) SetDeathChannel()`] — the fix
            // that lets a drone whose death danim never completes (0700001b's Drake) still raise it.
            (DEATH_ANIM, msg::UPDATE) => {
                if !self.firing || matches!(self.anim_set, 4 | 9) { self.drop_weapon(d, rng); }
                if self.death_pending { self.set_death_channel(); }
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

    /// KnockedOut_Anim 0x42, Knocked_Out 0x43, PunchImpact 0x53 (0x172a30, 0x172b48, 0x172ee0).
    fn handle_knockout(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8, m: u8) -> Option<bool> {
        use state::*;
        if !matches!(s, KNOCKED_OUT_ANIM | KNOCKED_OUT | PUNCH_IMPACT) { return None; }
        Some(match (s, m) {
            // ENTER (0x172a30 disasm): obj+0xF0 |= 0x20 (corpse), alert status 3 (not modelled — the port's
            // `bullet_impact` skips AlertStatusSet too), health = 0, `CallAnim(0, 0x50, 0, next 0x47)`,
            // Drone+0x4F8 |= 0x1000000, `DroneWeap_DropWeapon`, and `PlrStat_LogEnemyDisabled` for enemies.
            // Unlike Death_Anim there is NO OnInitDeath here: no death talk, no grenade drop, no dispatched log.
            (KNOCKED_OUT_ANIM, msg::ENTER) => {
                self.corpse = true;
                self.vision_on = false;
                self.firing = false;
                self.health = 0.0; // [M 0x172a88 `sw zero, 0xac(s1)`]
                self.knocked_out = true; // [M 0x172ab0-c4 `| 0x1000000` on +0x4F8]
                self.call_anim(d, anim_state::KNOCKED_OUT_FALL, DEAD, 0); // [M 0x172aa0-aa8 CallAnim(0, 0x50, 0, 0x47)]
                self.drop_weapon(d, rng); // [M 0x172ac0 jal DroneWeap_DropWeapon]
                if self.team == 1 { self.disabled_logged = true; } // [M 0x172ac8-dc +0x44 == 1 → PlrStat_LogEnemyDisabled]
                true
            }
            // EXIT (0x172b00-28): the death channel on every level EXCEPT {0x07000009, 0x0700000A} —
            // `sltiu v0, level + 0xf8fffff7, 2` at 0x172b14 is UNSIGNED, so 07000005 (difference 0xfffffffc)
            // passes. Fires when the fall anim completes (0x42 → 0x47).
            (KNOCKED_OUT_ANIM, msg::EXIT) => {
                if !matches!(d.level, 0x0700_0009 | 0x0700_000a) { self.set_death_channel(); }
                true
            }
            // Update / msgs 0, 3, 10: handled, no-op (0x172a30 cases 0/3/10).
            (KNOCKED_OUT_ANIM, _) => true,
            // Knocked_Out 0x43 (0x172b48): no ENTER arm; update (case 0) and impacts (8/9) → Death_Anim.
            (KNOCKED_OUT, msg::UPDATE) => { self.set_state(DEATH_ANIM); true }
            (KNOCKED_OUT, _) => true,
            // PunchImpact 0x53 (0x172ee0): ENTER sets +0x4F8 |= 0x10000 (flag not modelled) and replays
            // `NDrone2_PunchImpact(…, resume 0, allow 1)`; update (case 3) is `NDrone2_SeenAndAttacking`
            // (face the opponent, like the grenade-throw state).
            (PUNCH_IMPACT, msg::ENTER) => { self.punch_impact(d, rng, frame, 0, true); true }
            (PUNCH_IMPACT, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            (PUNCH_IMPACT, _) => true,
            _ => true,
        })
    }

    /// The bone matrices in world space at the playing script frame (root translation removed; object at the drone's
    /// x/z and yaw, y = feet + hip height − 0.02 as nf-view draws it). *Deviation*: the upper-body aim
    /// (`AnimObjectAimAt`) is not applied.
    pub fn palette(&self, d: &Data) -> Option<Vec<nf_formats::anim::Mat4>> {
        let (world, hip) = self.object_pose(d)?;
        let obj = nf_formats::placement::object_matrix([0.0, self.yaw, 0.0], [self.pos[0], self.draw_y(hip), self.pos[2]]);
        Some(world.iter().map(|m| nf_formats::anim::mul(m, &obj)).collect())
    }

    /// The local pose shown now: the playing script at its fractional time, blended with the scripts fading out
    /// (docs/systems/anim-engine.md). Root translation kept.
    pub fn local_pose(&self, d: &Data) -> Option<crate::anim::Pose> {
        let skin = d.skins.get(&self.skin)?;
        let skel = d.skeletons.get(&skin.skeleton)?;
        let cur = crate::anim::script_pose_drone(d.scripts, d.sequences, skel, skin, self.anim.script?, 1.0 + self.anim.ftime)?;
        let outs: Vec<(crate::anim::Pose, f32)> = self.anim.outgoing.iter()
            .filter_map(|o| {
                let mut p = crate::anim::script_pose_drone(d.scripts, d.sequences, skel, skin, o.script, o.time)?;
                if o.unyaw != 0.0 { if let Some(r) = p.first_mut() { r.1 = crate::anim::unyaw(r.1, o.unyaw); } }
                Some((p, o.weight()))
            }).collect();
        Some(crate::anim::blend_playing(cur, &outs))
    }

    /// Bone matrices in object space (root translation zeroed: `AnimSeqTick` takes it as root motion) with the
    /// upper-body aim (`AnimObjectAimAt`, bone 2: bone 2 and everything below it turned by the aim yaw about the
    /// model's up axis through bone 2, plus the aim pitch about the aim's horizontal right axis — the game's
    /// second angle, mode-0 clamp ±0.9, 0x1bfe58; *approximation*: the game rotates about the animated spine's
    /// own axes), the head tracking (`AnimObjectHeadTrack` 0x1bf618: the head bone ALONE — `psiBuildMatrixPalette`
    /// 0x111498 calls it on bone 5 while the 0x30 track flags are set — yaw about the neck axis and pitch about
    /// the head's right axis, premultiplied `q_yaw ⊗ q_pitch ⊗ q_bone`; *approximation*: the port's axes are the
    /// model's), and the hip height. Drawing and hit boxes both use it.
    pub fn object_pose(&self, d: &Data) -> Option<(Vec<nf_formats::anim::Mat4>, f32)> {
        let skin = d.skins.get(&self.skin)?;
        let mut local = self.local_pose(d)?;
        let hip = local.first().map_or(0.0, |r| r.0[1]);
        if let Some(r) = local.first_mut() { r.0 = [0.0; 3]; }
        let mut world = nf_formats::anim::world_pose(&local, skin);
        let a = self.aim_yaw;
        if (a.abs() > 1e-4 || self.aim_pitch.abs() > 1e-4) && world.len() > 2 {
            let pivot = world[2][3];
            let (sn, cs) = (a.sin(), a.cos());
            let rot_y: nf_formats::anim::Mat4 = [[cs, 0.0, -sn, 0.0], [0.0, 1.0, 0.0, 0.0], [sn, 0.0, cs, 0.0], [0.0, 0.0, 0.0, 1.0]];
            // The pitch axis: the aim direction's horizontal right (forward = (sin θ, 0, cos θ) → right =
            // (cos θ, 0, −sin θ), the game's cross(up, forward)). Positive rotation about +right pitches the
            // forward DOWN, and `aim_pitch` is positive up, so the matrix angle is negated. Composed as
            // R = R_yaw · R_pitch (the game's q_yaw ⊗ q_pitch ⊗ q_bone premultiply).
            let th = self.yaw + a;
            let rp = axis_angle_mat([th.cos(), 0.0, -th.sin()], -self.aim_pitch);
            let rot = nf_formats::anim::mul(&rot_y, &rp);
            let below = |b: usize| { let mut p = Some(b); while let Some(x) = p { if x == 2 { return true; } p = skin.parent(x); } false };
            for b in 0..world.len() {
                if !below(b) { continue; }
                let mut w = world[b];
                for k in 0..3 { w[3][k] -= pivot[k]; }
                let mut r = nf_formats::anim::mul(&w, &rot);
                for k in 0..3 { r[3][k] += pivot[k]; }
                world[b] = r;
            }
        }
        let (hy, hp) = (self.head_yaw, self.head_pitch);
        if (hy.abs() > 1e-4 || hp.abs() > 1e-4) && world.len() > 5 {
            // Bone 5 only (HeadTrack rewrites just that bone's quaternion; the head carries no children).
            let pivot = world[5][3];
            let (sn, cs) = (hy.sin(), hy.cos());
            let rot_y: nf_formats::anim::Mat4 = [[cs, 0.0, -sn, 0.0], [0.0, 1.0, 0.0, 0.0], [sn, 0.0, cs, 0.0], [0.0, 0.0, 0.0, 1.0]];
            let th = self.yaw + hy;
            let rp = axis_angle_mat([th.cos(), 0.0, -th.sin()], -hp);
            let rot = nf_formats::anim::mul(&rot_y, &rp);
            let mut w = world[5];
            for k in 0..3 { w[3][k] -= pivot[k]; }
            let mut r = nf_formats::anim::mul(&w, &rot);
            for k in 0..3 { r[3][k] += pivot[k]; }
            world[5] = r;
        }
        Some((world, hip))
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

    /// `DroneFunc_OnInitDeath` → `DroneFunc_DropGrenade`: a captain drops grenades at its own matrix — the stun
    /// grenade 0x35 ×2 on 07000001–08 [M 0x145bd0:141-144 `DropGrenade(param_1, 0x35, 2)`], the frag 0x34 on
    /// 07000009–0B, 11–16, 4A [M :141 `DropGrenade(param_1, 0x34)`].
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

    /// `NDrone2_FindAmbushCover` (0x152958): the unoccupied cover node within **2.0 units** of the drone (the ambush
    /// mode's scan radius) whose anim predicates pass; claims it the same way `find_cover` does. Returns false when no
    /// node is close enough. The ambush mode (drone type 0x10) uses this instead of the network-distance search.
    fn find_ambush_cover(&mut self, d: &Data, need_los: bool) -> bool {
        let mut best: Option<(f32, usize, u8)> = None;
        for (i, n) in d.covers.iter().enumerate() {
            if self.cover_rt.get(i).is_none_or(|r| r.owner.is_some()) { continue; }
            let dd = ((n.pos[0] - self.pos[0]).powi(2) + (n.pos[1] - self.pos[1]).powi(2) + (n.pos[2] - self.pos[2]).powi(2)).sqrt();
            if dd > 2.0 || best.is_some_and(|b| dd >= b.0) { continue; }
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

    /// `DroneFunc_ConsiderExplosive` (0x1431c0, msg 0x21): an explosive landed near the drone. Gates: fear props
    /// 0x34/0x3A/0x3B [M 0x1431c0:22-30], the state gate [M 0x1431c0:40-67] and +0x2B (not already scared)
    /// [M 0x1431c0:68-70]. Stores the position (+0x330), the run distance (+0x350, per the caller's classification),
    /// the return state (+0x354 — 0x87 Hide by default, the caller state for the flashbang/gas/0x2A-object cases;
    /// 0 → the caller state [M 0x1431c0:191-193]), the deadline (+0x324) and +0x2B, then routes to
    /// `RunAwayFromObject` (0x86) [M 0x1431c0:194-195] — whose flee update this port now runs (G-026).
    fn consider_explosive(&mut self, d: &Data, frame: u32, pos: V3, resume: u8, run_dist: f32, ret: u8, deadline: u32) {
        if !self.prop(d, 0x34) && !self.prop(d, 0x3a) && !self.prop(d, 0x3b) { return; }
        // The game skips the flee when the drone is already in an explosive-aware state (0x28/0x2D/0x2E–0x4C, etc.).
        if matches!(self.state, 0x28 | 0x2d | 0x2e..=0x4c | 0x52..=0x56 | 0x62..=0x88) && self.state != 0x86 { return; }
        if self.scared { return; }
        self.explosive_pos = pos;
        self.explosive_run_dist = run_dist;
        self.flee_return = if ret != 0 { ret } else { resume }; // +0x354 == 0 → the caller state [M 0x1431c0:191-193]
        self.flee_deadline = Some(deadline); // +0x324
        self.scared = true; // +0x2B [M 0x1431c0:194]
        self.set_state(state::RUN_AWAY_FROM_OBJECT);
    }

    /// The msg-0x21 payload `DroneFunc_HandleExplosives` (0x1435f0:51 `Drone_SM_SendMsgSelf(0x21, obj)`) delivers;
    /// `resume` is the caller state each game handler passes (Idle/Alert/Patrol/Hostage 0x56 [M 0x15bf18:134],
    /// HostageIdle 0x0E, HostageHide 0, UnderCoverFire 0x9C, RunAway/Hide 0x88 [M 0x168d18 case 0x21]).
    fn take_explosive_msg(&mut self, d: &Data, frame: u32, resume: u8) {
        if let Some((pos, run, ret, dl)) = self.explosive_msg.take() {
            self.consider_explosive(d, frame, pos, resume, run, ret, dl);
        }
    }

    /// `DroneMove_FindSafetyFromScaryObject` (0x1559f8) → `FindSafetyFromScaryPosition` (0x152250): one goal, two
    /// strategies — the nav-path emitter at dist·1.8 [M 0x152250:91-134] and the straight-line fallback:
    /// dir = normalise(drone − scary), goal = scary + dir·dist with y = scary.y + 0.5, clamped into the nav
    /// network [M 0x152250:142-166]. The port uses the straight-line goal (the route build clamps it; the emitter
    /// scan is [S]-approximated by it — documented in the report), resets the route [M 0x152250:128-129] and
    /// stores +0x5A2/+0x5A0 (the return/resume states — the port keeps +0x354 in flee_return) [M 0x152250:167-174].
    fn find_safety_from_scary_object(&mut self, d: &Data) -> bool {
        let dist = self.explosive_run_dist;
        let scary = self.explosive_pos;
        let (dx, dy, dz) = (self.pos[0] - scary[0], self.pos[1] - scary[1], self.pos[2] - scary[2]);
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        if len < 0.001 { return false; }
        let goal = [scary[0] + dx / len * dist, scary[1] + 0.5, scary[2] + dz / len * dist];
        self.goal = Some(goal);
        self.croute = None; // ResetRoute + InvalidateRoute [M 0x152250:128-129]
        self.move_to_goal(d); // build the route now (the deadline bounds a dead route)
        true
    }

    /// The scary-object states (G-026): `RunAwayFromObject` 0x86 [M 0x168aa8], `HideFromScaryObject` 0x87
    /// [M 0x168d18], `RecoverFromScaryObject` 0x88 [M 0x173658].
    fn handle_scary(&mut self, d: &Data, _rng: &mut Rng, frame: u32, s: u8, m: u8) -> Option<bool> {
        use state::*;
        if !matches!(s, RUN_AWAY_FROM_OBJECT | HIDE_FROM_SCARY_OBJECT | RECOVER_FROM_SCARY_OBJECT) { return None; }
        Some(match (s, m) {
            // ENTER: +0x4F8 |= 0x10000, AlertStatusSet(1), +0x4F8 |= 0x400000, FindSafety — fail → 0x88
            // [M 0x168aa8:18-30].
            (RUN_AWAY_FROM_OBJECT, msg::ENTER) => {
                self.vision_on = true;
                if !self.find_safety_from_scary_object(d) { self.set_state(RECOVER_FROM_SCARY_OBJECT); }
                true
            }
            // UPDATE: past the deadline (+0x324) → 0x88 [M 0x168aa8:38-42]; MoveToGoalPosition (flags 0x300):
            // moving → the run anim 0x1E [M 0x168aa8:43-47], blocked → the crouch chain 0x11 → 0x19 → 0x21
            // [M 0x168aa8:53-67], arrived → the return state +0x5A2 [M 0x168aa8:48-52].
            (RUN_AWAY_FROM_OBJECT, msg::UPDATE) => {
                if self.flee_deadline.is_some_and(|t| frame > t) { self.set_state(RECOVER_FROM_SCARY_OBJECT); return Some(true); }
                match self.move_to_goal(d) {
                    0 => self.call_anim(d, 0x1e, 0, 0),
                    3 => { let st = self.flee_return; self.set_state(if st == 0 { ATTACK } else { st }); }
                    0x9 | 0xc => self.call_anim(d, 0x11, 0, 0),
                    _ => self.call_anim(d, 0x1e, 0, 0),
                }
                true
            }
            (RUN_AWAY_FROM_OBJECT, msg::EXIT) => { self.scared = false; true } // [M 0x168aa8:31-35]
            // ENTER: +0x2B = 1, AlertStatusSet(1), crouch 0x11 / else 0x19 / else 0x21 [M 0x168d18:18-43].
            (HIDE_FROM_SCARY_OBJECT, msg::ENTER) => {
                self.vision_on = true;
                self.scared = true;
                if self.can_do(d, 0x11) { self.call_anim(d, 0x11, 0, 0); }
                else if self.can_do(d, 0x19) { self.call_anim(d, 0x19, 0, 0); }
                else { self.call_anim(d, 0x21, 0, 0); }
                true
            }
            // UPDATE: wait the deadline out, then 0x88 [M 0x168d18:39-44]. EXIT: +0x2B = 0 [M 0x168d18:46-48].
            (HIDE_FROM_SCARY_OBJECT, msg::UPDATE) => {
                if self.flee_deadline.is_some_and(|t| frame > t) { self.set_state(RECOVER_FROM_SCARY_OBJECT); }
                true
            }
            (HIDE_FROM_SCARY_OBJECT, msg::EXIT) => { self.scared = false; true }
            // ENTER: the mission-bot path (+0xC5 == 0xC) is not ported; else +0x2B = 0 and +0x5A0 != 0x89 → 0x56
            // (the port has no RunToAlarm 0x89, so always Attack) [M 0x173658:22-42]. No anim is played here.
            (RECOVER_FROM_SCARY_OBJECT, msg::ENTER) => { self.scared = false; self.set_state(ATTACK); true }
            _ => false,
        })
    }

    /// `NDrone2_CoverAvailable(needLos)` (0x155a38).
    pub fn cover_available(&mut self, d: &Data, need_los: bool) -> bool {
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
        let (code, reached) = {
            let route = self.croute.as_mut().unwrap();
            route.remaining = route.distance(d.nav, feet);
            if route.remaining < 2.0 { return 3; }
            let r = route.follow(d.nav, feet);
            // `LinkCreep_Handler` (0x15a2c8) runs `ReachedDestNode` for EVERY followed route — the
            // repositioning drives through door/kick/action nodes like any other (fix-nav-remainder).
            // The successor node is the one AFTER the reached node in the route's node list (the
            // game reads `route+0x80`+1 BEFORE `SetupNextNode` advances; the port's follow advances
            // inside the same call, so the index is already past it).
            let rpath = route.path;
            let next = r.1.and_then(|n| route.nodes.iter().position(|&x| x == n)).and_then(|i| route.nodes.get(i + 1)).copied();
            drop(route);
            self.route_reached_special(d, rpath, r.1, next);
            r
        };
        if let Some(used) = self.links_used.get_mut(self.croute.as_ref().unwrap().path) { let rt = self.croute.as_mut().unwrap(); rt.mark_used(used) }
        let t = self.croute.as_ref().unwrap().target;
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

    fn slide_to(&mut self, _d: &Data, to: V3, rate: f32) {
        let dist = ((to[0] - self.pos[0]).powi(2) + (to[2] - self.pos[2]).powi(2)).sqrt();
        if dist > 0.01 {
            self.pos[0] += (to[0] - self.pos[0]) * rate;
            self.pos[2] += (to[2] - self.pos[2]) * rate;
        } else {
            self.pos[0] = to[0];
            self.pos[2] = to[2];
        }
        // The height is set by `collision_handler`'s down-ray (WORKLOG 0078); the old `floor_below(0.6)` probe
        // could climb sloped walls next to cover points.
    }

    /// `Drone_FeetOnPoint` (0x140660): a down-ray from `pos` to `pos + up·−(hip+0.6)` with collision mask 0xC (walkable +
    /// solid). When the hit's y is within 0.2 below the feet (`pos.y − hip`), the drone is on the surface: sets
    /// `on_surface` and returns the floor height (the game sets `anim+0x60 & 8`). `None` when the ray misses (in air).
    fn feet_on_point(&self, world: &CollisionWorld) -> Option<f32> {
        let hip = self.hip();
        let from = [self.pos[0], self.pos[1], self.pos[2]];
        let to = [self.pos[0], self.pos[1] - (hip + 0.6), self.pos[2]];
        let hit = world.segment(from, to)?;
        let floor = from[1] + (to[1] - from[1]) * hit;
        let feet_y = self.pos[1] - hip;
        if floor >= feet_y - 0.2 { Some(floor) } else { None }
    }

    /// `Drone_CollisionHandler` (0x140968): the per-tick collision + gravity driver. Clears the in-air flag, runs the
    /// down-ray (`feet_on_point`), and when off the surface accumulates `WldGravity · REC_FRAME_RATE` into `velocity`
    /// and steps `pos += velocity · REC_FRAME_RATE`. On the surface the velocity is zeroed and `pos.y` snaps to
    /// `floor + hip` (the port's ground convention: `pos.y = floor + 1.0`, feet at `pos.y − 0.6`). The `free_root` path
    /// (SpecialDeath) bypasses this; `bounded = false` (the AI-boundary push-out flag) also skips it. [M] constants:
    /// WldGravity (0,−9.8,0), REC_FRAME_RATE 1/60, hip+0.6 ray, 0.2 on-surface.
    pub fn collision_handler(&mut self, world: &CollisionWorld) {
        if self.free_root || self.corpse || !self.bounded { return; }
        // `+0x4F8 & 0x4000` cleared at the top.
        self.in_air = false;
        // `NDrone2_DoCollision` / `NDrone2_DoGravity` predicates: alive, & 0x20 (collision) / & 0x80 (gravity), not
        // in a death state. The port's `bounded` flag stands for `& 0x20`; gravity always runs when bounded.
        let on_surface = self.feet_on_point(world);
        self.on_surface = on_surface.is_some();
        if let Some(floor) = on_surface {
            // Snap the feet to the floor and zero the velocity (the game sets `anim+0x60 & 8`).
            self.pos[1] = floor + self.hip();
            self.velocity = [0.0; 3];
        } else {
            self.in_air = true;
            // `velocity += WldGravity · REC_FRAME_RATE` (gravity per tick²).
            self.velocity[1] += -9.8 * (1.0 / 60.0);
        }
        // `pos += velocity · REC_FRAME_RATE` (the game applies this whether or not on the surface — on the surface
        // velocity is zero, so it's a no-op there).
        let r = 1.0 / 60.0;
        self.pos[0] += self.velocity[0] * r;
        self.pos[1] += self.velocity[1] * r;
        self.pos[2] += self.velocity[2] * r;
    }

    /// `Drone+0xA0`: the hip height (the offset from `pos` to the feet). Default 1.0; the port's ground convention is
    /// `pos.y = floor + hip` and `feet() = pos − 0.6` (the 0.6 is the port's own feet offset, not the game's).
    fn hip(&self) -> f32 { 1.0 }

    /// `NDrone2_SetAngleToCover`: low node or as-low cover → the opponent; side 1 → node yaw − π/2; side 2 → node yaw.
    fn set_angle_to_cover(&mut self, d: &Data) {
        let Some(n) = self.cover.and_then(|c| d.covers.get(c)) else { return };
        self.desired_yaw = if !n.is_corner() || self.cover_type == crate::cover::kind::AS_LOW { self.yaw_to_opp }
            else if self.cover_side == 1 { n.yaw - crate::trig::HALF_PI } else { n.yaw };
    }

    /// `DroneAnim_ValidateCoverAnim` (0x13bf40): 0 passes; ≥ 0x77 passes unvalidated [M 0x13bf40:12,22]; 1..0x77
    /// is valid only when the drone's anim set has the state's row (the base set's row also counts
    /// [M 0x13bf40:14-19]); an invalid 5 substitutes 7, else 0xA [M 0x13bf40:25-38 — the record-11 gate
    /// approximated by `can_do(7)`, the port's documented stand-in]; any other invalid → 0xFFFF.
    pub fn validate_cover_anim(&self, d: &Data, anim: u16) -> u16 {
        if anim == 0 || anim >= 0x77 { return anim; }
        if self.can_do(d, anim) { return anim; }
        if anim == 5 { return if self.can_do(d, 7) { 7 } else { 0xa }; }
        0xffff
    }

    /// `DroneAnim_CoverAnim(mode, next state)` (0x13c078): picks the cover type (mode 6, or 7 = maybe change) and plays
    /// the `CCModes` column; false when no anim fits.
    fn cover_anim(&mut self, d: &Data, rng: &mut Rng, mode: u8, next: u8) -> bool {
        use crate::cover::{cc_anim, kind};
        let Some(n) = self.cover.and_then(|c| d.covers.get(c)).cloned() else { return false };
        if !n.is_corner() {
            // Mode 3's 0x58 is played raw — the game does not validate it [M 0x13c078:297-300]; every other
            // low-node anim goes through `DroneAnim_ValidateCoverAnim` [M 0x13c078:286-295, 309-316].
            let (anim, validate) = match mode { 0 | 6 | 1 | 4 => (n.idle_anim, true), 2 => (n.aim_anim, true), 3 => (0x58u16, false), _ => return false };
            let anim = if !validate { anim } else {
                let v = self.validate_cover_anim(d, anim);
                if v == 0xffff { return false; } // CoverAnim returns 0 — no anim, the state lingers [M 0x13c078:311-314]
                v
            };
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
                        // `SlideToPosition(0x3CCCCCCC = 0.025, +0x830, y = 1)` [M 0x169150:64-67] — 2.5 %/tick
                        // toward the in-cover point (the doc's "10 %/tick" was wrong; UnderCoverInit's 8 % xz
                        // [M 0x169cc0:38] is a different call). The port keeps its xz slide + floor re-snap.
                        if remaining < 1.0 { let p = self.cover_point; self.slide_to(d, p, 0.025); }
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
                    // `NDrone2_DSTATE_UnderCoverIdle` case 0xC (0x169e58): the tries count `+0xCD8` is read, then
                    // incremented, and the **old** value gates the re-arm — < 3 → timer C again (Rand_FRand(150)),
                    // else Attack. Three retries, the attack on the fourth timer tick.
                    let tries = self.cover_tries;
                    self.cover_tries += 1;
                    if tries < 3 { self.timer_c = Some(frame + rng.frand(150.0) as u32) } else { self.set_state(ATTACK) }
                }
                true
            }
            (UNDER_COVER_IDLE, msg::EXIT) => { d.props.set(0x10, &mut self.props, 0); true }
            (UNDER_COVER_AIM, msg::ENTER) => {
                if !self.cover_anim(d, rng, 2, UNDER_COVER_FIRE) { self.set_state(UNDER_COVER_LEAVE_NOW) }
                true
            }
            (UNDER_COVER_FIRE, msg::ENTER) => {
                // `NDrone2_DSTATE_UnderCoverFire` case 1 (0x16a3e0): if the drone's anim set is 0x13 (sniper) AND the
                // level is before 0x0700000E, branch to the sniper-fire state (0x97) instead of firing at once. On
                // later levels the sniper fires through `UnderCoverFire` directly.
                if self.anim_set == 0x13 && d.level < 0x0700_000e { self.set_state(UNDER_COVER_SNIPER_FIRE); return Some(true); }
                self.fire(d, rng, frame, true);
                true
            }
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
            (UNDER_COVER_FIRE, msg::EXPLOSIVE) => { self.take_explosive_msg(d, frame, UNDER_COVER_LEAVE_NOW); true }
            // `NDrone2_DSTATE_UnderCoverSniperFire` (0x16a610): fires on timer C (1 s), returns on timer D (6 s),
            // reloads (0x98) when the clip empties. `+0x2E` = the node is not low (corner); `OpponentIsAimingAtMe(7.5)`
            // + `BondIsFacingMe(45)` + Rand(3) = 0 → duck back (0x99).
            (UNDER_COVER_SNIPER_FIRE, msg::ENTER) => {
                self.vision_on = true;
                self.cover_claimed = false;
                // `+0x2E` = node.kind != 1 (corner, not low); timer C at +1 s, timer D at +6 s.
                self.timer_c = Some(frame + 60);
                self.timer_d = Some(frame + 60 * 6);
                true
            }
            (UNDER_COVER_SNIPER_FIRE, msg::UPDATE) => {
                // Clip empty → reload (0x98).
                if self.clip < 1 { self.set_state(UNDER_COVER_SNIPER_RELOAD); return Some(true); }
                // The decomp OR-splits: if OpponentIsAimingAtMe(7.5) → duck immediately (no RNG);
                // else if BondIsFacingMe(45) AND Rand(3)==0 → duck.
                if self.aimed_at && self.dist < 7.5 {
                    self.set_state(UNDER_COVER_RETURN);
                } else if self.opp_facing.abs() < 45f32.to_radians() && rng.rand(3) == 0 {
                    self.set_state(UNDER_COVER_RETURN);
                } else if !in_cover_ok(self, true) {
                    // Node no longer OK: leans → return (0x99), else leave now (0x9C).
                    let leans = self.cover.and_then(|c| d.covers.get(c)).is_some_and(|n| n.is_corner())
                        && matches!(self.cover_type, crate::cover::kind::CROUCH_LEAN | crate::cover::kind::STAND_LEAN);
                    self.set_state(if leans { UNDER_COVER_RETURN } else { UNDER_COVER_LEAVE_NOW });
                }
                true
            }
            (UNDER_COVER_SNIPER_FIRE, msg::TIMER_C) => { self.fire(d, rng, frame, true); true }
            (UNDER_COVER_SNIPER_FIRE, msg::TIMER_D) => { self.set_state(UNDER_COVER_RETURN); true }
            (UNDER_COVER_SNIPER_FIRE, msg::EXPLOSIVE) => { self.take_explosive_msg(d, frame, UNDER_COVER_LEAVE_NOW); true }
            // `NDrone2_DSTATE_UnderCoverSniperReload` (0x16a8b0): the reload anim (0x52), then back to 0x97. Refills
            // the clip from the spare (`+0xBBC = +0xBBE`).
            (UNDER_COVER_SNIPER_RELOAD, msg::ENTER) => {
                self.vision_on = true;
                self.cover_claimed = false;
                self.call_anim(d, anim_state::RELOAD, UNDER_COVER_SNIPER_FIRE, 0);
                self.clip = self.clip_max;
                true
            }
            (UNDER_COVER_SNIPER_RELOAD, msg::EXPLOSIVE) => { self.take_explosive_msg(d, frame, 0); true }
            // `NDrone2_DSTATE_AmbushInit` (0x17 2960): find an ambush cover within 2.0 → 0x93; else crouch (0x11) and
            // wait (0x38).
            (AMBUSH_INIT, msg::ENTER) => {
                self.vision_on = true;
                if self.find_ambush_cover(d, true) { self.set_state(UNDER_COVER_INIT); }
                else { self.call_anim(d, anim_state::CROUCH_AIM, 0, 0); self.set_state(AMBUSH_WAIT); }
                true
            }
            (AMBUSH_INIT, msg::UPDATE) => { true }
            // `NDrone2_DSTATE_AmbushWait` (0x161560): crouched; on the opponent visible or within 3 → 0x56 (Attack).
            (AMBUSH_WAIT, msg::ENTER) => { self.vision_on = true; self.cover_claimed = false; true }
            (AMBUSH_WAIT, msg::UPDATE) => {
                let bond = self.target.pos;
                let d2 = ((bond[0] - self.pos[0]).powi(2) + (bond[2] - self.pos[2]).powi(2)).sqrt();
                if self.seen || d2 < 3.0 { d.props.set(0x10, &mut self.props, 0); self.set_state(ATTACK); }
                true
            }
            (AMBUSH_WAIT, msg::EXIT) => { d.props.set(0x10, &mut self.props, 0); true }
            (AMBUSH_WAIT, msg::EXPLOSIVE) => { self.take_explosive_msg(d, frame, ATTACK); true }
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
            (c, msg::ENTER | msg::EXIT | msg::UPDATE) if c == RUN_FOR_COVER || (UNDER_COVER_INIT..=UNDER_COVER_LEAVE_NOW).contains(&c)
                || c == AMBUSH_INIT || c == AMBUSH_WAIT => true,
            _ => return None,
        })
    }

    /// `NDrone2_DSTATE_GrenadeThrow` (0x2D, 0x173480): the throw state. ENTER sets `+0x4F8 |= 0x10000`, alert 1, and
    /// plays the throw anim (0x49) → on event 1 (`DroneAnim_EventFunc` datum 1, which sets `pending_grenade`), the
    /// tick loop calls `throw_grenade`. UPDATE: `SeenAndAttacking`, `SetAngleToObj`. The anim's completion routes to
    /// 0x56 (Attack). msg 0x21: `ConsiderExplosive`; msg 8: `HandleImpact(0, 0)`.
    fn handle_grenade_throw(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8, m: u8) -> Option<bool> {
        use state::*;
        if s != GRENADE_THROW { return None; }
        Some(match m {
            msg::ENTER => {
                self.vision_on = true;
                self.call_anim(d, anim_state::GRENADE_THROW, ATTACK, 0);
                true
            }
            msg::UPDATE => {
                // `NDrone2_SeenAndAttacking` + `SetAngleToObj`: face the opponent.
                self.desired_yaw = self.yaw_to_opp;
                true
            }
            msg::EXPLOSIVE => { self.take_explosive_msg(d, frame, ATTACK); true }
            msg::BULLET => { self.on_bullet(d, rng, frame, s); true }
            _ => true,
        })
    }

    /// The stun-grenade receiving family (fix-explosions). `StunGrenadeImpact` 0x172bb0 case 1: the recover
    /// time (`DroneFunc_RecoverTime` 0x147188 → +0xCE0), the weapon drop + ChangeWeapon(1) when kind ≠ 4
    /// (the port's guards carry one fixed weapon — not modelled, noted in the report), `AlertStatusSet(2)`
    /// and the stunned anim (0x2D/0x2E by `Rand_Rand(2)`) with follow-on Loop 0x4E. `StunGrenadeLoop`
    /// 0x1631c0 case 1: the timer arms at frame + recover (+0xCC4) and msg 0xC branches to Recover 0x4F
    /// when nobody is within 8.0 (`DroneFunc_AnyoneNear(0x40000000)`), else re-arms frame + 5·60. Another
    /// flash (msg 0x18/0x19) re-arms a fresh recover [M 0x1631c0 case 0x18]. `StunGrenadeRecover` 0x163350
    /// case 1: +0x4F8 & 0x10000 and the get-up anim 0x76 with follow-on 0x56 (non-civilians). The bullets
    /// (msg 8/9 → 0x44) route through `on_impact`. `AlertStatusSet(2)` maps to the port's alert ≥ 0.66
    /// (the ALERTNESS_LEVELS index 2 [*suggested* mapping; the port's alert is the 0..1 alertness float]).
    fn handle_stun(&mut self, d: &Data, rng: &mut Rng, frame: u32, s: u8, m: u8) -> Option<bool> {
        use state::*;
        if !matches!(s, STUN_GRENADE_IMPACT | STUN_GRENADE_LOOP | STUN_GRENADE_RECOVER) { return None; }
        Some(match (s, m) {
            (_, 0) | (_, 10) => true,
            (STUN_GRENADE_IMPACT, msg::ENTER) => {
                // `DroneFunc_RecoverTime` 0x147188: for a non-{0x35,0x43,0x44,0x4A,0x4C} hitting weapon
                // (the stun grenade 0x35 among them) the base is `FRAME_RATE_INT·60·0.01·(200 − Drone+0xB9)`
                // = 36·(200−0) = 7200 ticks with the stats-default +0xB9 = 0 [M 0x14c600:90; the port's
                // 07000005 snapshot evidence: the stats blob is all zero at run time].
                self.stun_until = frame + 7200;
                self.alert = self.alert.max(0.66);
                self.vision_on = false;
                self.firing = false;
                // `Rand_Rand(2)`: one draw of the shared stream, then the anim with follow-on Loop [M 0x172c34].
                let a = if rng.rand(2) == 0 { anim_state::STUNNED_A } else { anim_state::STUNNED_B };
                self.call_anim(d, a, STUN_GRENADE_LOOP, 0);
                true
            }
            (STUN_GRENADE_LOOP, msg::ENTER) => { self.vision_on = false; self.firing = false; self.timer_c = Some(self.stun_until); true }
            (STUN_GRENADE_LOOP, msg::TIMER_C) => {
                // Case 0xC: anyone within 8.0 keeps him down (re-arm frame + 5·FRAME_RATE_INT = 300).
                let near = self.opp_dist() <= 8.0;
                if near { self.timer_c = Some(frame + 300); true } else { self.set_state(STUN_GRENADE_RECOVER); true }
            }
            (STUN_GRENADE_RECOVER, msg::ENTER) => {
                self.timer_c = None;
                self.call_anim(d, anim_state::STUN_GET_UP, ATTACK, 0);
                true
            }
            _ => true,
        })
    }

    /// The drone's distance to Bond (`DroneFunc_AnyoneNear`'s 8.0 test target in single player [*suggested*:
    /// the game checks every DCVars peer; the port's `dist` field is the maintained Bond distance]).
    fn opp_dist(&self) -> f32 { self.dist }

    // ---- state machine -----------------------------------------------------------------------------------------

    fn set_state(&mut self, s: u8) {
        if s != 0 {
            if std::env::var_os("NF_AILOG").is_some() {
                eprintln!("[ailog] guard {} state {:#x} -> {:#x}", self.index, self.state, s);
            }
            self.next_state = Some(s);
        }
    }

    /// `Drone_SM_RouteMsgDCV`: deliver, then apply pending changes (exit → enter, repeating).
    pub fn send(&mut self, d: &Data, rng: &mut Rng, frame: u32, m: u8) {
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
            self.timer_d = None; // RouteMsgDCV clears BOTH message timers on every change [M-dis 0x17207c-88]
            self.damage_taken = 1.0; // the hit-effect multiplier +0x14C resets on every change [M-dis 0x17207c-88]
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
        if m == msg::PUNCH { return self.on_punch(d, rng, frame, s); }
        // The scream/shout broadcasts [M 0x145e28 + 0x160140; the receivers' EnemyAlerts arms
        // 0x177c18:33-51]: case 0x12 gates on prop 0x27, case 0x11 on prop 0x28; the receiver's
        // next state comes from `DroneVision_ShoutFromOtherDrone`. The port's civilian states go
        // CivilianScared (whose ENTER runs CivilianHiding's cower 0x19 [M 0x15d3a0/0x15d520]);
        // the guard states keep their sound-alert investigation (`enemy_alerts`, the port's
        // `DroneVision_AlertSound` else-arm [M 0x177800:81-88]).
        if m == msg::SCREAM || m == msg::SHOUT {
            let hear = if m == msg::SCREAM { self.prop(d, 0x27) } else { self.prop(d, 0x28) };
            if !hear { return true; }
            let civilian = matches!(s, PARTY_GIRL | CIVILIAN | CIVILIAN_MISSION | CIVILIAN_PATROL | CIVILIAN_DOOR_GUARD | TRUCK_DRIVER_IDLE | CIVILIAN_SCARED | CIVILIAN_HIDING);
            if civilian {
                if !matches!(s, CIVILIAN_SCARED | CIVILIAN_HIDING) { self.set_state(CIVILIAN_SCARED); }
            } else if let Some(n) = self.enemy_alerts(d, s) { self.set_state(n); }
            return true;
        }
        // BulletImpact 0x55 (0x173098): enter turns vision on; the flinch call ends it (state `+0x118`).
        if s == BULLET_IMPACT && matches!(m, 0 | msg::ENTER | msg::UPDATE) { if m == msg::ENTER { self.vision_on = true; } return true; }
        // `DroneFunc_HandleExplosives`'s msg 0x21 for the states outside the cover/grenade-throw handlers: the
        // game's Idle/Alert/Patrol/Hostage handlers route it to ConsiderExplosive with their resume state
        // (0x56 [M 0x15bf18:134, 0x15c248:102, 0x15c498:105, ...], HostageIdle 0x0E [M 0x15ce60:51], HostageHide 0
        // [M 0x15cfe0:48]); the fleeing family re-enters ConsiderExplosive with 0x88 [M 0x168d18 case 0x21]. The
        // armed states (UnderCoverFire 0x9C [M 0x16a3e0], AMBUSH_WAIT, GRENADE_THROW) keep their own arms.
        if m == msg::EXPLOSIVE && !matches!(s, UNDER_COVER_FIRE | UNDER_COVER_SNIPER_FIRE | UNDER_COVER_SNIPER_RELOAD | AMBUSH_WAIT | GRENADE_THROW) {
            let resume = if matches!(s, RUN_AWAY_FROM_OBJECT | HIDE_FROM_SCARY_OBJECT) { RECOVER_FROM_SCARY_OBJECT }
                else if matches!(s, PARTY_GIRL | CIVILIAN | CIVILIAN_MISSION | CIVILIAN_PATROL | CIVILIAN_DOOR_GUARD | CIVILIAN_SCARED | CIVILIAN_HIDING) { 0 }
                else { ATTACK };
            self.take_explosive_msg(d, frame, resume);
            return true;
        }
        if let Some(r) = self.handle_death(d, rng, frame, s, m) { return r; }
        if let Some(r) = self.handle_knockout(d, rng, frame, s, m) { return r; }
        if let Some(r) = self.handle_scary(d, rng, frame, s, m) { return r; }
        if let Some(r) = self.handle_cover(d, rng, frame, s, m) { return r; }
        if let Some(r) = self.handle_grenade_throw(d, rng, frame, s, m) { return r; }
        if let Some(r) = self.handle_stun(d, rng, frame, s, m) { return r; }
        match (s, m) {
            // `NDrone2_DSTATE_Global` enter: a drone with a switch channel that is still off waits (WaitSwitch); else its
            // start state `+0x5A2` (InitPatrol with props 0x24/0x25, else Idle). Modes 0x1E and start scripts
            // (PlayScript 3) are not ported.
            (GLOBAL, msg::ENTER) => {
                self.return_state = self.start_state(d);
                if self.switch_channel != 0 && !self.channel_on { self.set_state(WAIT_SWITCH); return true; }
                // `NDrone2_DSTATE_Global` 0x15b830 msg 1: dtypenew 0x17 (the truck driver) → TruckDriverInit
                // (0x32) BEFORE the start-script branch — the driver never enters PlayScript, and
                // WaitSwitch 0x15b960 case 3 (his channel firing) routes him to 0x32 the same way. [L]
                // 05-truck's 07000006 segment: the driver (drone 13) is in state 0x32 for the whole
                // 700-frame window while the level's start-script guards sit in 3.
                if self.mode == 0x17 { self.set_state(TRUCK_DRIVER_INIT); return true; }
                // `NDrone2_DSTATE_Global` 0x15b830: a drone with a start script (`Drone+0x554`) enters
                // PlayScript (3) instead of its start state — the script plays first [L 07000001's first
                // guard is in state 3 on the trace's line 0].
                if self.start_script != 0 { self.set_state(PLAY_SCRIPT); return true; }
                self.set_state(self.start_state(d));
                true
            }
            // `NDrone2_DSTATE_PlayScript` 0x15ba68. ENTER (msg 1): `+0x4F8 |= 0x10000 | 0x2000`, then
            // SetScript(`+0x554`, …) per script — 0x60005e6/0x60008ae and the default → (0x24, 0x11a,
            // 0x1e, 0, 5, 0); 0x6000168 → (10, 0x32, 8, 0, 5, 0); 0x6000169 → (5, 0x32, 0x1e, 0, 5, 0)
            // + AttackTalk + `+0x4F8 |= 0x400000`; 0x60007ac → (10, 0x32, 0xf, 0, 5, 0) + props
            // 0x3b/0x3a cleared. The resume `+0xCD8` (0x56 ATTACK for the default/0x60005e6/0x60008ae
            // arms) is the IMPACT resume (cases 6/7/8/9 → `HandleImpact(+0xCD8)`), not the script's
            // natural end — msg 5 lands back in this state (below). Not ported: the impact resume, the
            // AttackTalk/flag side effects, and the UPDATE arm's fully-alerted
            // `NDrone2_SeenAndAttacking` (fix-ai-traces' live check 2: the game's guard alert never
            // reaches 1.0 in the PlayScript window — no [L] evidence it fires).
            (PLAY_SCRIPT, msg::ENTER) => {
                let (st, follow) = match self.start_script {
                    0x6000_168 => (10, 0x32),
                    0x6000_169 => (5, 0x32),
                    0x6000_7ac => (10, 0x32),
                    _ => (0x24, 0x11a),
                };
                self.play_script(d, self.start_script, st, follow, 0, msg::ANIM_DONE);
                true
            }
            // msg 5 (0x15ba68 case 5, the script's natural end): `+0x554 = 0`, then — the default and
            // 0x60005e6/0x60008ae arms → `SetState(+0x5A2)` (the START state, not ATTACK: the guard on
            // 07000006 whose 1244-frame start script ends settles into IDLE, fix-npcs' d15); the
            // 0x6000168/0x6000169 arms → `CallAnim(0, 10, 0, 0x68, 0, 0)` + AttackTalk and the state is
            // HELD (the 07000001 guard's script 06000169 ends into his idle-then-aim follow-up);
            // 0x60007ac → `SetDAnimNow(0xf4, 0, 1)` + the start state (no 0x60007ac drone is ported).
            (PLAY_SCRIPT, msg::ANIM_DONE) => {
                let played = self.anim.script.unwrap_or(0);
                self.start_script = 0;
                match played {
                    0x6000_168 | 0x6000_169 => { self.call_anim(d, 10, AIM_STAND_FIRE, 0); true }
                    _ => { let s = self.start_state(d); self.set_state(s); true }
                }
            }
            (PLAY_SCRIPT, msg::UPDATE) => {
                // The game's msg-3 also attacks a fully-alerted drone (`NDrone2_SeenAndAttacking`), but
                // the trace's guard stays in 3 for ~350 lines — the port's creation alert for it is
                // unresolved (the port's start_state mapped it to ATTACK; see the report), so only the
                // look runs here for now.
                if let Some(n) = self.look_for_opponent(d) { self.set_state(n) }
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
                self.patrol_talk(rng, frame);
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
                self.attack_talk(d, rng);
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
                self.attack_talk(d, rng);
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
                // `NDrone2_DSTATE_Surrender_Anim` 0x1626a0's ENTER ends with
                // `PlrStat_LogEnemySurrender(0)` — outboxed, folded into the tally by the tick.
                self.surrender_logged = true;
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
                // `FirstSightState` (the drone's own first sight) talks before its prop-0x1B roll; `FirstAttack` talks after
                // it when it chose no state, or when this is not a first sight (*suggested*: first sight = own sight on the
                // first attack).
                let first_sight = self.own_sight && !self.attack_talked;
                self.attack_talked = true;
                if first_sight { self.seen_player_talk(d, rng); } else { self.attack_talk(d, rng); }
                let first = if self.prop(d, 0x1b) { if rng.rand(2) == 0 { 0 } else { self.choose_combat_move(d, rng) } }
                    else if self.prop(d, 0x18) { 0 } else if self.prop(d, 0x19) { self.choose_combat_move(d, rng) } else { 0 };
                if first_sight && first == 0 { self.attack_talk(d, rng); }
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
                // `NDrone2_DSTATE_CombatNoRoute` case 0xC (0x164c08): when not stationary and no prop 0x40, cover
                // first (→ RunForCover); only when that fails does `+0xCD8` toggle and alternate — odd →
                // MoveToObject (0..2 → Combat), even → SetGoalToDroneWithLos (→ GoToGoal). Stationary/0x40 drones
                // skip straight to the fallback without toggling (drone-port-check.md #2 re-check).
                let next = if self.stays(d) { 0 }
                    else if self.cover_available(d, true) { RUN_FOR_COVER }
                    else {
                        self.no_route_odd = !self.no_route_odd;
                        if self.no_route_odd {
                            match self.move_to_object(d, 2.0, true) { 0..=2 => COMBAT, 10 => return true, _ => 0 }
                        } else if self.set_goal_to_drone_with_los(d, rng) { GO_TO_GOAL } else { 0 }
                    };
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
            (GO_TO_GOAL, msg::ENTER) => { self.vision_on = true; true } // guard 0x63 (fix-actors had renamed it to the ally's 0x28)
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
            (COMBAT_NO_MOVE, msg::ENTER) => { self.attack_talk(d, rng); true }
            (COMBAT_NO_MOVE, msg::UPDATE) => {
                if self.unseen_ticks < 16 { self.set_state(COMBAT_NEW_SIGHTING) } else { self.set_state(COMBAT_NO_SIGHT) }
                true
            }
            (COMBAT_NEW_SIGHTING, msg::ENTER) => {
                self.seen_player_talk(d, rng);
                let crouch = self.prop(d, 0x11) && self.can_do(d, anim_state::CROUCH_AIM) && self.dist >= 6.0 && rng.rand(2) != 0;
                self.set_state(if crouch { AIM_CROUCH_FIRE } else { AIM_STAND_FIRE });
                true
            }
            (AIM_STAND, msg::ENTER) => { self.call_anim(d, anim_state::AIM_STAND, 0, msg::ANIM_DONE); self.firing = false; true }
            (AIM_STAND, msg::ANIM_DONE) => { self.set_state(if self.unseen_ticks < 16 { AIM_STAND_FIRE } else { COMBAT_NO_SIGHT }); true }
            (AIM_STAND_FIRE, msg::ENTER) => {
                // Non-snipers talk (`+0xC5` ≠ 2; `+0x4F8 & 0x10` and prop 0x40 not modelled).
                if !self.stationary { self.attack_talk(d, rng); }
                self.call_anim(d, anim_state::AIM_STAND, 0, 0);
                self.vision_on = true;
                if self.unseen_ticks < 16 { self.fire(d, rng, frame, true) } else { self.set_state(COMBAT_NO_SIGHT) }
                true
            }
            (AIM_STAND_FIRE, msg::UPDATE) => {
                self.turn_to_opponent(0.524);
                if self.dist <= 4.0 && self.can_backoff(d) { self.set_state(AIM_BACKOFF); return true; }
                if self.unseen_ticks >= 16 { self.set_state(COMBAT_NO_SIGHT); return true; }
                if self.firing && !self.burst_done {
                    if std::env::var_os("NF_AILOG").is_some() && self.grenade_count > 0 {
                        eprintln!("[ailog] guard {} AIM_STAND_FIRE UPDATE: firing={} burst_done={} -- combat_state skipped (mid-burst)",
                            self.index, self.firing, self.burst_done);
                    }
                    return true;
                }
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
                self.vision_on = true; // `+0x4F8 |= 0x10000` (decomp 0x167590:14)
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
                self.vision_on = true; // `+0x4F8 |= 0x10000` (decomp 0x1677b0 etc.)
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
                self.vision_on = true; // `+0x4F8 |= 0x10000` (decomp 0x167e90 etc.)
                self.call_anim(d, if s == ROLL_LEFT_CROUCH { anim_state::ROLL_LEFT } else { anim_state::ROLL_RIGHT }, AIM_CROUCH_FIRE, 0);
                true
            }
            (ROLL_LEFT_CROUCH | ROLL_RIGHT_CROUCH, msg::UPDATE) => { self.desired_yaw = self.yaw_to_opp; true }
            (AIM_BACKOFF, msg::ENTER) => {
                self.vision_on = true; // `+0x4F8 |= 0x10000` (decomp 0x166b98)
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
            // ---- the mission-state family (docs/WORKLOG fix-drone3). The `NDrone2_StateFuncs` table 0x29b300 maps
            // state 0x32 → `NDrone2_DSTATE_TruckDriverInit` 0x160bd0, 0x33 → InitAlert 0x160d98, 0x34 → Idle 0x160ed0,
            // 0x2e/0x2f → PartyGirlInit 0x1728c0 / PartyGirl 0x160140, 0x30 → CivilianGuard 0x1603a8, 0x10..0x14 →
            // CivilianInit 0x172730 / Civilian 0x15d118 / CivilianScared 0x15d3a0 / CivilianHiding 0x15d520 /
            // CivilianPatrol 0x15d610, 0x1c → HostageIdle 0x15ce60. Before this block the port had NO handler for
            // any of them: the playtest's guests, driver, hostage-side NPCs and civilians stood frozen (turning only).
            // TruckDriverInit (0x160bd0) ENTER (case 1): prop 0x1d → AssignAIPath(8) in BOTH branches;
            // `+0x694 = obj+0x54` (the sit target, not ported); a start script (`+0x554`) plays FROM HERE
            // — SetScript(`+0x554`, 0x24, 0x11a, 0x1e, 0, 5, 0), `+0x554 = 0`, `+0x4F8` bits 0x20/0x80
            // cleared — and the state HOLDS until the script's done (case 5 → 0x35 when the AI-path
            // counter `+0x9f2` is set, else 0x34): [L] 05-truck's 07000006 driver (start script 0600011d)
            // sits in 0x32 for the whole 700-frame trace window. Script-less: the prop clears and it is
            // straight to TruckDriverIdle. The sound/bullet arms keep the game's routing for the
            // transient state (0x33 / HandleImpact(0x33, 0)).
            (TRUCK_DRIVER_INIT, msg::ENTER) => {
                if self.prop(d, 0x1d) { self.assign_path(d, 8); }
                if self.start_script != 0 {
                    let script = self.start_script;
                    self.start_script = 0;
                    self.play_script(d, script, 0x24, 0x11a, 0, msg::ANIM_DONE);
                    return true;
                }
                d.props.set(0x1d, &mut self.props, 0);
                self.set_state(TRUCK_DRIVER_IDLE);
                true
            }
            // 0x160bd0 case 5 (the script's done): the AI-path counter `+0x9f2` (InitAIPath 0x153720's
            // node cursor — the port models no per-node path walk, and the game's driver is never shown
            // leaving on a mission path in the traces) → 0x34; only a running mission path would take
            // 0x35. The driver steps into TruckDriverIdle ("hands behind back", the user's C1).
            (TRUCK_DRIVER_INIT, msg::ANIM_DONE) => { self.set_state(TRUCK_DRIVER_IDLE); true }
            (TRUCK_DRIVER_INIT, msg::SOUND | msg::ALARM) => { self.set_state(TRUCK_DRIVER_INIT_ALERT); true } // 0x11/0x12/0x13/0x16/0x1e → 0x33
            (TRUCK_DRIVER_INIT, msg::BULLET) => self.on_bullet_resume(d, rng, frame, TRUCK_DRIVER_INIT_ALERT, false), // HandleImpact(0x33, 0)
            // TruckDriverInitAlert (0x160d98): anim done → CivilianScared (0x12); bullets/noise HandleImpact(0, …).
            (TRUCK_DRIVER_INIT_ALERT, msg::ANIM_DONE) => { self.set_state(CIVILIAN_SCARED); true }
            (TRUCK_DRIVER_INIT_ALERT, msg::BULLET) => self.on_bullet_resume(d, rng, frame, 0, false),
            (TRUCK_DRIVER_INIT_ALERT, msg::SOUND) => self.on_bullet_resume(d, rng, frame, 0, true),
            // TruckDriverIdle (0x160ed0): stand idle; NearArmedPerson (0x1787c8 — the 8.0 arg is dead)
            // → CivilianScared (`|= 0x8000000`).
            (TRUCK_DRIVER_IDLE, msg::ENTER) => { self.vision_on = true; self.set_stand_idle(d, rng, frame, true); true }
            (TRUCK_DRIVER_IDLE, msg::UPDATE) => {
                if self.near_armed_person(d) { self.sighted = true; self.set_state(CIVILIAN_SCARED); }
                true
            }
            (TRUCK_DRIVER_IDLE, msg::BULLET) => self.on_bullet_resume(d, rng, frame, CIVILIAN_SCARED, false),
            (TRUCK_DRIVER_IDLE, msg::SOUND | msg::ALARM) => { if let Some(n) = self.enemy_alerts(d, s) { self.set_state(n); } true }
            (TRUCK_DRIVER_IDLE, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            // PartyGirlInit (0x1728c0) ENTER: prop 0x1d → AssignAIPath(8) — with a route → CivilianMission
            // (0x16, the walkers); else the prop clears and PartyGirl (0x2f) [M 0x1728c0:17-25]. The game's NIS
            // script link (`+0x9f2` there is the route count — InitAIPath 0x153720 fills it) makes the mask-8
            // check the mission-path check; the port's route slot is the same gate.
            (PARTY_GIRL_INIT, msg::ENTER) => {
                if self.prop(d, 0x1d) && self.assign_path(d, 8) { self.set_state(CIVILIAN_MISSION); return true; }
                d.props.set(0x1d, &mut self.props, 0);
                self.set_state(PARTY_GIRL);
                true
            }
            // PartyGirl (0x160140) ENTER: voice 0x12 → Civilian; else AssignAIPath(4) — with a route →
            // CivilianPatrol (0x14), without → stay [M 0x160140:16-35]; UPDATE: armed person within 10
            // (0x41200000) → CivilianScared, else stand idle (SetStandIdleAnim(1)) — PartyGirl never patrols
            // herself [M 0x160140 case 3]. The BondTalk idle lines are level voice sets, not ported.
            (PARTY_GIRL, msg::ENTER) => {
                if self.voice_type == 0x12 { self.set_state(CIVILIAN); return true; }
                self.vision_on = true;
                if self.assign_path(d, 4) { self.set_state(CIVILIAN_PATROL); }
                true
            }
            (PARTY_GIRL, msg::UPDATE) => {
                if self.near_armed_person(d) { // 0x160140:40 passes 0x41200000 — dead arg [M 0x1787c8]
                    self.sighted = true;
                    // 0x160140:63-76: prop 0x33, the 0x200000 once-bit — the scared guest SHOUTS the
                    // armed intruder to everyone (`NDrone2_DroneAlertToObject(20.0, 20.0, 0.75, …,
                    // 0x11, Bond)` — the alert source is BOND), and prop 0x52 raises the mission-fail
                    // reason 0xb [M 0x13e560].
                    if self.prop(d, 0x33) && !self.scare_shouted {
                        self.scare_shouted = true;
                        self.pending_shout = Some((msg::SHOUT, self.target.pos)); // the alert source is BOND [M 0x160140:66-70]
                    }
                    if self.prop(d, 0x52) { self.fail_reason = Some(crate::ally::mission_fail_reason(0xb)); }
                    self.set_state(CIVILIAN_SCARED);
                } else {
                    self.set_stand_idle(d, rng, frame, true);
                }
                true
            }
            (PARTY_GIRL, msg::BULLET) => self.on_bullet_resume(d, rng, frame, CIVILIAN_SCARED, false),
            (PARTY_GIRL, msg::SOUND | msg::ALARM) => { if let Some(n) = self.enemy_alerts(d, s) { self.set_state(n); } true }
            (PARTY_GIRL, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            // CivilianGuard (0x1603a8): stands a post — look-around (0x21) at alert ≥ 0.66, stand idle (0x24) below;
            // armed person within 20 (0x41a00000) or armed Bond in sight (`+0x228 & 4`) → Attack (0x56);
            // EXIT sets team 1 (case 2: `+0x44 = 1`).
            (CIVILIAN_GUARD, msg::ENTER) => {
                self.vision_on = true;
                self.return_state = ATTACK; // `+0x5A2 = 0x56`
                if self.alert >= 0.66 { self.call_anim(d, anim_state::ALERT_IDLE, 0, 0) } else { self.call_anim(d, anim_state::STAND_IDLE, 0, 0) }
                true
            }
            (CIVILIAN_GUARD, msg::EXIT) => { self.team = 1; true }
            (CIVILIAN_GUARD, msg::UPDATE) => {
                // 0x1603a8 case 3: NearArmedPerson (the 20.0 arg is dead [M 0x1787c8]) or
                // ObjectIsArmed(Bond) && sight [M 0x1603a8:39-45] — then 0x8000000 + the prop-0x33 alert.
                if self.near_armed_person(d) || (self.bond_armed && self.seen) {
                    self.sighted = true;
                    self.set_state(ATTACK);
                }
                true
            }
            (CIVILIAN_GUARD, msg::BULLET) => self.on_bullet_resume(d, rng, frame, ATTACK, false),
            (CIVILIAN_GUARD, msg::SOUND | msg::ALARM) => { if let Some(n) = self.enemy_alerts(d, s) { self.set_state(n); } true }
            (CIVILIAN_GUARD, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            // CivilianInit (0x172730) ENTER: voice 0xd → PartyGirl; prop 0x1d → AssignAIPath(8) — with a route →
            // CivilianMission (0x16); else the prop clears, AssignAIPath(4): route → CivilianPatrol (0x14),
            // none → Civilian (0x11) [M 0x172730:15-45].
            (CIVILIAN_INIT, msg::ENTER) => {
                if self.voice_type == 0xd { self.set_state(PARTY_GIRL); return true; }
                if self.prop(d, 0x1d) && self.assign_path(d, 8) { self.set_state(CIVILIAN_MISSION); return true; }
                d.props.set(0x1d, &mut self.props, 0);
                if self.assign_path(d, 4) { self.set_state(CIVILIAN_PATROL) } else { self.set_state(CIVILIAN) }
                true
            }
            // CivilianMission (0x15d860): the prop-0x1D characters with a kind-8 route — the party's walkers.
            // ENTER: voice 0x13 → KikoMission (0x19, not ported — no such voice on the ported levels); else the
            // ready flags + CallAnim(0x37) (the walk) + the BondTalk bookkeeping clear [M 0x15d860:16-27].
            // UPDATE: armed person within 10 → (voice 0x10: team 1 + Attack; else) CivilianScared; else
            // FollowRoute — arrived → the prop clears → Civilian; blocked (creep 9/0xc) → look-around at
            // alert ≥ 0.66 else stand idle; else keep walking [M 0x15d860 case 3; UpdateMissionRoute 0x154a80].
            (CIVILIAN_MISSION, msg::ENTER) => {
                // CivilianMission 0x15d860 case 1: voice 0x13 → KikoMission 0x19 (the port's skin
                // mapping: skin 0x5e → 0x13 [M 0x14b300]); unreached by the disc's placements
                // (kiko.rs) but ported for fidelity. fix-finale-blockers replaced the 0x0d stand-in.
                if self.voice_type == 0x13 { self.set_state(crate::kiko::KIKO_MISSION); return true; }
                self.call_anim(d, anim_state::WALK, 0, 0);
                true
            }
            (CIVILIAN_MISSION, msg::UPDATE) => {
                if self.near_armed_person(d) { // 0x15d860:35 passes 0x41200000 — dead arg [M 0x1787c8]
                    self.sighted = true;
                    if self.voice_type == 0x10 { self.team = 1; self.set_state(ATTACK); }
                    else { self.set_state(CIVILIAN_SCARED); }
                } else if self.route.is_some() {
                    let arrived = self.update_mission_route(d, rng, frame, self.walk_anim(d));
                    if arrived && self.next_state.is_none() { d.props.set(0x1d, &mut self.props, 0); self.set_state(CIVILIAN); }
                } else {
                    d.props.set(0x1d, &mut self.props, 0);
                    self.set_state(CIVILIAN);
                }
                true
            }
            (CIVILIAN_MISSION, msg::BULLET) => self.on_bullet_resume(d, rng, frame, CIVILIAN_SCARED, false),
            (CIVILIAN_MISSION, msg::SOUND | msg::ALARM) => { if let Some(n) = self.enemy_alerts(d, s) { self.set_state(n); } true }
            (CIVILIAN_MISSION, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            // CivilianPatrol (0x15d610): walk the patrol path (`CallAnim(0x37)`); armed person within 6 (0x40c00000)
            // → CivilianScared (the voice-0x10 team switch and the female scream 0x48 are not ported).
            (CIVILIAN_PATROL, msg::ENTER) => {
                self.vision_on = true;
                self.request_anim(d, anim_state::WALK);
                true
            }
            (CIVILIAN_PATROL, msg::UPDATE) => {
                self.update_patrol_route(d, rng, frame);
                if self.near_armed_person(d) {
                    self.sighted = true;
                    // The female scream 0x48 before the scare-state switch (`Drone+0x15` != 0; the
                    // game's CivilianInit carve-out `+0xd8 == 0x10` has no port counterpart — the
                    // port's CIVILIAN_PATROL is never entered from CIVILIAN_INIT) [M 0x15d610:60].
                    if self.female { self.foley_reqs.push(0x48); }
                    self.set_state(CIVILIAN_SCARED);
                } // 0x15d610:32 — dead arg [M 0x1787c8]
                true
            }
            (CIVILIAN_PATROL, msg::BULLET) => self.on_bullet_resume(d, rng, frame, ATTACK, false), // HandleImpact(0x56, 0)
            (CIVILIAN_PATROL, msg::SOUND | msg::ALARM) => { if let Some(n) = self.enemy_alerts(d, s) { self.set_state(n); } true }
            (CIVILIAN_PATROL, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            // Civilian (0x15d118) ENTER: stand idle; the scare radius `+0xCF8` = 10 on 07000007, else 20.
            // UPDATE: armed person within it → CivilianScared.
            (CIVILIAN, msg::ENTER) => {
                self.vision_on = true;
                self.scared_radius = if d.level == 0x0700_0007 { 10.0 } else { 20.0 };
                self.set_stand_idle(d, rng, frame, true);
                true
            }
            (CIVILIAN, msg::UPDATE) => {
                // 0x15d118:46 passes the stored +0xCF8 radius (10/20) — dead in NearArmedPerson [M 0x1787c8].
                if self.near_armed_person(d) { self.sighted = true; self.set_state(CIVILIAN_SCARED); }
                true
            }
            (CIVILIAN, msg::BULLET) => self.on_bullet_resume(d, rng, frame, CIVILIAN_SCARED, false),
            (CIVILIAN, msg::SOUND | msg::ALARM) => { if let Some(n) = self.enemy_alerts(d, s) { self.set_state(n); } true }
            (CIVILIAN, msg::FORCED_ATTACK) => { self.set_state(ATTACK); true }
            // CivilianScared (0x15d3a0) ENTER: vision on, alert 2 (`Drone_AlertStatusSet(2)`), the scared talk, then
            // CivilianHiding (0x13) — the game's alt-mode 7 run-to-point (0x64) has no node data in the port.
            (CIVILIAN_SCARED, msg::ENTER) => {
                self.vision_on = true;
                self.alert = 1.0;
                self.pain_talk(d); // the closest port line: the hurt/scared voice of the set
                self.set_state(CIVILIAN_HIDING);
                true
            }
            (CIVILIAN_SCARED, msg::BULLET) => self.on_bullet_resume(d, rng, frame, CIVILIAN_SCARED, false),
            // CivilianHiding (0x15d520) ENTER: alert 2, cower anim 0x19.
            (CIVILIAN_HIDING, msg::ENTER) => {
                self.vision_on = true;
                self.alert = 1.0;
                self.call_anim(d, anim_state::COWER, 0, 0);
                true
            }
            (CIVILIAN_HIDING, msg::BULLET) => self.on_bullet_resume(d, rng, frame, CIVILIAN_SCARED, false),
            // HostageIdle (0x15ce60 — state 0x0d [L NDrone2_StateFuncs 0x29b300]) ENTER: stand idle, alert 0;
            // UPDATE: an armed drone within 20 → HostageHide (0xe — not ported; the hostages stay here). Bullets
            // HandleImpact(0xe, …) — resume 0 keeps the state.
            (HOSTAGE_IDLE, msg::ENTER) => {
                self.alert = 0.0;
                self.call_anim(d, anim_state::STAND_IDLE, 0, 0);
                true
            }
            (HOSTAGE_IDLE, msg::BULLET) => self.on_bullet_resume(d, rng, frame, 0, false),
            // AllyLead (0x15e4f0 / 0x15e698 / 0x15eda8 / 0x15f2e8): the escort lead — 07000001's Mayhew
            // (mode 12, `DroneTypeSettings[12].start` = 0x1c [L 0x29aee8]) leads Bond along the mission
            // AI path to the bunker plate. Minimal port: the entry, the lead loop (the port's
            // `update_mission_route` = `UpdateMissionRoute` 0x154a80) and the Bond wait; the
            // flee/combat sub-states (0x1e PlayerInWay, 0x1f Hide, 0x21 MissionWait, 0x22 BondCombat,
            // 0x28 GoToGoalPosition) are not ported (038-fix-actors). The wait distances are straight-
            // line stand-ins for `DroneMove_ObjectMissionPathDistance` (0x153f50).
            (ALLY_LEAD_INIT, msg::ENTER) => {
                if self.prop(d, 0x1d) { self.assign_path(d, 8); } // [M 0x15e4f0 case 1: AssignAIPath(8)]
                if self.route.is_none() {
                    // `+0x9F2 == 0`: no route → stand idle and stay [M 0x15e4f0 case 1]
                    self.set_stand_idle(d, rng, frame, true);
                } else {
                    self.call_anim(d, anim_state::ALERT_IDLE, 0, 0); // 0x21
                }
                true
            }
            (ALLY_LEAD_INIT, msg::UPDATE) => {
                // Bond within 3.0 → AllyLeadWait [M 0x15e4f0 case 3]
                if self.dist < 3.0 { self.set_state(ALLY_LEAD_WAIT); }
                true
            }
            (ALLY_LEAD, msg::ENTER) => { self.call_anim(d, anim_state::RUN, 0, 0); true } // [M 0x15e698 case 1: 0x1e]
            (ALLY_LEAD, msg::UPDATE) => {
                // The route first: done → AllyLeadDone [M 0x15e698 case 3, FollowRoute branch 3].
                // MOVING rides the RUN anim [M 0x15e698 case 3: code 0/2 → CallAnim(0x1e)] — the old
                // walk-anim stand-in walked the whole lead at half speed and spent the test's escort
                // window on the stairs.
                if self.update_mission_route(d, rng, frame, anim_state::RUN) { self.set_state(ALLY_LEAD_DONE); return true; }
                // `ReachedDestNode` queued `DSTATE_OpenDoor` — the sub-state checks below must not
                // stomp it (the game's SetState applies at once; the port's flush is deferred).
                if self.next_state.is_some() { return true; }
                // An armed, started hostile within 10 → LeadHide (the cower) [M 0x15e698's armed-drone
                // branch + 0x15ec08; the port's armed_peer_dist snapshot].
                if self.armed_peer_dist <= 10.0 {
                    if std::env::var_os("NF_AILOG").is_some() { eprintln!("[ailog] escort {} → LeadHide (peer {:.1})", self.index, self.armed_peer_dist); }
                    self.set_state(LEAD_HIDE);
                    return true;
                }
                // Bond blocking the route → PlayerInWay [M 0x15ea28's entry; the port's route_blocked].
                if self.route_blocked && self.dist <= 3.0 { self.set_state(PLAYER_IN_WAY); return true; }
                // Bond far behind → wait for him [M 0x15e698 case 3: the path-distance rules of
                // ObjectMissionPathDistance — the port's straight-line stand-in uses 12 units; the
                // game's close-Bond wait also needs BondWithinAngle(0.52) (the "assure him" stance),
                // which a following escort does not trigger].
                if self.dist > 12.0 { self.set_state(ALLY_LEAD_WAIT); }
                true
            }
            (ALLY_LEAD_WAIT, msg::ENTER) => { self.call_anim(d, anim_state::ALERT_IDLE, 0, 0); true } // [M 0x15eda8 case 1: 0x21]
            (ALLY_LEAD_WAIT, msg::UPDATE) => {
                // Resume the lead once Bond is back in the escort band (the game's path-distance
                // resume 0x15eda8 case 3 → 0x1d; hysteresis against the 12-unit entry).
                if self.dist <= 10.0 { self.set_state(ALLY_LEAD); }
                true
            }
            // ---- the AllyLead sub-states (fix-actors; 038's brief) ---------------------------------
            // Lead's UPDATE dispatches them [M 0x15e698 case 3 + the sub-state entries]: an armed,
            // started hostile within 10 (0x41200000) cowers the escort in LeadHide; Bond blocking the
            // route (the port's `route_blocked`, set by `update_mission_route`, with Bond within 3)
            // parks him in PlayerInWay.
            (PLAYER_IN_WAY, msg::ENTER) => { self.call_anim(d, anim_state::ALERT_IDLE, 0, 0); true } // [M 0x15ea28 case 1: 0x21]
            (PLAYER_IN_WAY, msg::UPDATE) => {
                // No armed drone within 10 and Bond out of the way band → Lead [M 0x15ea28 case 3;
                // the 1.06-rad facing check is the game's nudge-aside, the port resumes on distance —
                // *suggested*, documented in docs/systems/actors.md].
                if self.armed_peer_dist > 10.0 && self.dist > 3.0 { self.set_state(ALLY_LEAD); }
                true
            }
            (LEAD_HIDE, msg::ENTER) => { self.call_anim(d, anim_state::COWER, 0, 0); true } // [M 0x15ec08 case 1: 0x19]
            (LEAD_HIDE, msg::UPDATE) => {
                // No armed drone within 14 (0x41600000) → latch and return to Lead in 2 ticks
                // [M 0x15ec08 case 3: +0xCD8 = 1, timer C = now + 2·FRAME_RATE_INT].
                if self.armed_peer_dist > 14.0 && self.timer_c.is_none() {
                    self.timer_c = Some(frame + 2);
                }
                true
            }
            (LEAD_HIDE, msg::TIMER_C) => { self.set_state(ALLY_LEAD); true }
            // AllyLeadMissionWait 0x21 (0x15efc8): the route's wait-channel node parks the escort until
            // the channel fires. The port's route nodes carry no channels — the state is reachable
            // through the forced-state path only [*suggested*; documented].
            (LEAD_MISSION_WAIT, msg::ENTER) => {
                // The game latches `Drone+0x12C`, 0 → 0x99 [M 0x15efc8 case 1]. Drone+0x12C is init 0 by
                // Drone_SM_InitObject 0x175838 and no decompiled function ever writes it, so in the game the
                // 0x99 default always fires; the port models the field (`mission_wait_channel`) for fidelity.
                self.wait_channel = if self.mission_wait_channel != 0 { self.mission_wait_channel } else { 0x99 };
                self.call_anim(d, anim_state::ALERT_IDLE, 0, 0);
                true
            }
            (LEAD_MISSION_WAIT, msg::UPDATE) => {
                if self.wait_channel_on { self.set_state(ALLY_LEAD); } // [M 0x15efc8 case 3]
                true
            }
            // AllyLeadBondCombat 0x22 (0x15f120): Bond is fighting near the escort — hold the alert
            // idle until the fight moves away, then Lead. The game's ObjectMissionPathDistance(5.0)
            // route check is the port's armed-peer + distance band [*suggested*; documented].
            (LEAD_BOND_COMBAT, msg::ENTER) => { self.call_anim(d, anim_state::ALERT_IDLE, 0, 0); true } // [M 0x15f120 case 1: 0x21]
            (LEAD_BOND_COMBAT, msg::UPDATE) => {
                if self.armed_peer_dist > 10.0 { self.set_state(ALLY_LEAD); }
                true
            }
            // ---- the AllyFollow chain (0x24–0x27, 0x15f3c8/0x15f4e0/0x15f6a0/0x15f858) -------------
            (FOLLOW_INIT, msg::ENTER) => { self.call_anim(d, anim_state::ALERT_IDLE, 0, 0); true } // [M 0x15f3c8 case 1: 0x21]
            (FOLLOW_INIT, msg::UPDATE) => {
                // Bond visible (+0x228 & 4 — the port's `seen`) → AllyFollow [M 0x15f3c8 case 3].
                if self.seen { self.set_state(FOLLOW); }
                true
            }
            (FOLLOW, msg::ENTER) => { self.call_anim(d, self.walk_anim(d), 0, 0); true }
            (FOLLOW, msg::UPDATE) => {
                // Follow the mission band: beyond 8 walk at Bond, inside it stand (the game walks the
                // route toward Bond's node; the port's band is *suggested*, documented).
                if self.dist > 8.0 { self.desired_yaw = self.yaw_to_opp; self.request_anim(d, self.walk_anim(d)); }
                else { self.set_stand_idle(d, rng, frame, false); }
                true
            }
            (FOLLOW_WAIT, msg::UPDATE) => { if self.dist <= 6.0 { self.set_state(FOLLOW); } true }
            (FOLLOW_DONE, msg::ENTER) => { self.set_stand_idle(d, rng, frame, true); true } // [M 0x15f858]
            // AllyGoToGoalPosition 0x28 (0x15f938): walk to the saved goal, then Lead (the escort's
            // only user) [*suggested*: the goal = the route's last node].
            (GO_TO_GOAL_ALLY, msg::ENTER) => { self.call_anim(d, self.walk_anim(d), 0, 0); true }
            (GO_TO_GOAL_ALLY, msg::UPDATE) => {
                if self.update_mission_route(d, rng, frame, self.walk_anim(d)) && self.next_state.is_none() { self.set_state(ALLY_LEAD); }
                true
            }
            // NDrone2_DSTATE_OpenDoor (0x1681f8) — `NDrone2_StateFuncs[0x81]` [L 0x29b300 + 0x81·4]:
            // the route's door node parked the drone here to work the door standing in +0xBA8.
            // ENTER (case 1): the door-prop drones (props 0x14/0x15/0x16 — behaviour_util_getProperty
            // on Drone+0x4D8) walk the node and play the door anim 0x4e, whose event 6 sends msg 0x20;
            // the rest the stand/alert idle by alertness (+0x50C < 1.0 → 0x24 else 0x21). Both then
            // call `NDrone2_OpenDoor` once (the outbox below) and arm timer C at now + 2 ticks
            // [M case 1 tail + LAB_001683ac: +0xCC0 = 1, +0xCC4 = GameState + 2·FRAME_RATE_INT].
            // (The game's kind == 0 gate on the 0x4e branch is not reproducible at ENTER — the port
            // resolves the door index just after the state machine; the anim choice drops that gate.)
            (state::OPEN_DOOR, msg::ENTER) => {
                if self.prop(d, 0x14) || self.prop(d, 0x15) || self.prop(d, 0x16) {
                    self.call_anim(d, 0x4e, 0, 0);
                } else if self.alert < 1.0 {
                    self.call_anim(d, anim_state::STAND_IDLE, 0, 0);
                } else {
                    self.call_anim(d, anim_state::ALERT_IDLE, 0, 0);
                }
                self.door_attempt = true;
                self.timer_c = Some(frame + 2);
                true
            }
            // UPDATE (case 3): open → the saved return state +0x5A6; closed → stay (the route creep
            // keeps walking regardless — `LinkCreep_Handler` 0x15a2c8 skips the blocked test while
            // +0xBA8 stands; the port's route resumes when the lead state returns).
            (state::OPEN_DOOR, msg::UPDATE) => {
                if self.hit_door_open { self.set_state(self.door_return); }
                true
            }
            // Timer C (case 0xC): re-activate the door and re-arm the 2-tick retry.
            (state::OPEN_DOOR, msg::TIMER_C) => {
                self.door_attempt = true;
                self.timer_c = Some(frame + 2);
                true
            }
            // Msg 0x20 (case 0x20, from the door anim's event 6): another activation attempt. The
            // grunt itself is the `Drone_SfxInfo` line played by the `pending_door_msg` drain in
            // `Drones::tick` (state-dependent: 0x27 here, 0x28 for the kick — the raw `anim_sounds`
            // pushes played `t.get(0x27/0x28)`, sound 39/40, instead of the lines' 1477/1478).
            (state::OPEN_DOOR, msg::DOOR) => {
                self.door_attempt = true;
                true
            }
            // ---- fix-nav-remainder: the route's kick / action / fiddle nodes (G-027) ----
            // `NDrone2_DSTATE_KickObject` (0x1684c0) — state 0x82 [M 0x153358:146]. ENTER (case 1):
            // the kick anim 0x4e (the `+0x4F8 |= 0x10000` "record opponent history" bit is not
            // modelled — the port's BOT_* bookkeeping doesn't exist, same as DSTATE_OpenDoor's).
            (state::KICK_OBJECT, msg::ENTER) => {
                self.call_anim(d, 0x4e, 0, 0);
                self.timer_c = Some(frame + 2); // [M 0x1684c0 case 0xc tail: +2·FRAME_RATE]
                true
            }
            // Timer C (case 0xc): kick the bound object again, re-arm at +2. The unpausing itself
            // runs in `Drones::tick` (the ScriptPlayers live there) via the `kick_attempt` outbox.
            (state::KICK_OBJECT, msg::TIMER_C) => {
                self.kick_attempt = true;
                self.timer_c = Some(frame + 2);
                true
            }
            // Msg 0x20 (case 0x20, the kick anim's event 6): the kick grunt (`NDrone2_PlaySFX`
            // 0x28 — 1.0 volume), `NDrone2_KickObject` (0x14aa00 → `SP_UnPause` 0x1dca58: the
            // bound ScriptPlayer's script unpauses — serviced by tick), and straight back to the
            // saved state `+0x5A6` [M 0x1684c0 case 0x20: `Drone_SM_SetState(+0x5a6)`].
            (state::KICK_OBJECT, msg::DOOR) => {
                // The kick grunt is the `Drone_SfxInfo[0x28]` line (sfx 1478), played through the
                // `NDrone2_PlaySFX` gates by the `pending_door_msg` drain (the same event-6 delivery
                // as the door; the state's case 0x20 picks the line) [M 0x1684c0 disasm 0x16857c-90].
                self.kick_attempt = true;
                self.set_state(self.door_return);
                true
            }
            // `NDrone2_DSTATE_ActionAnim` (0x168628) — state 0x83 [M 0x153358:119]. ENTER (case 1):
            // play the drone's start script `+0x554` (`DroneAnim_SetScript(+0x554, 10, 0x32, 0x1e,
            // 0, 5, 0)` — anim state 10, follow-on 0x32, completion msg 5).
            (state::ACTION_ANIM, msg::ENTER) => {
                if self.start_script != 0 {
                    self.play_script(d, self.start_script, 10, 0x32, 0, msg::ANIM_DONE);
                }
                true
            }
            // Case 5 (the script's done): clear +0x4F8's 0x400000 (not modelled) and → 0x6f
            // [M 0x168628 case 5: `Drone_SM_SetState(0x6f)`].
            (state::ACTION_ANIM, msg::ANIM_DONE) => { self.set_state(state::AIM_CROUCH); true }
            // Msg 0x20 (case 0x20): back to the saved state `+0x5A6`.
            (state::ACTION_ANIM, msg::DOOR) => { self.set_state(self.door_return); true }
            // Timer C (case 0xc): re-arm at +2 (nothing else).
            (state::ACTION_ANIM, msg::TIMER_C) => { self.timer_c = Some(frame + 2); true }
            // `NDrone2_DSTATE_StandFiddle` (0x1687c0) — state 0x84 [M 0x153358:101]. ENTER (case 1):
            // the fiddle anim 0x23 and timer C at +5·FRAME_RATE.
            (state::STAND_FIDDLE, msg::ENTER) => {
                self.call_anim(d, 0x23, 0, 0);
                self.timer_c = Some(frame + 5);
                true
            }
            // Timer C (case 0xc) and msg 0x20: back to the saved state `+0x5A6` [M cases 0xc/0x20].
            (state::STAND_FIDDLE, msg::TIMER_C | msg::DOOR) => { self.set_state(self.door_return); true }
            // Case 3 sets the drone's +0x38 byte (no ported reader — not modelled); UPDATE falls
            // through to the game's default (unhandled → the GLOBAL handler), like KickObject's.
            (ALLY_LEAD_DONE, msg::ENTER) => { self.call_anim(d, anim_state::ALERT_IDLE, 0, 0); true } // [M 0x15f2e8 case 1: 0x21]
            // ---- fix-finale-blockers (crates/nf-game/src/kiko.rs has the addresses and the tags) ----
            // NinjaStand 0xa6 (0x16b990) ENTER: the eyes datum then SetState(0xa7) — the BLIND_DRONES
            // cheat is not modelled, so the fall-through is the only arm.
            (0xa6, msg::ENTER) => { self.set_state(crate::kiko::NINJA_ATTACK); true }
            // NinjaAttack 0xa7 (0x16bb50): alert 1.0, the distance bands; UPDATE re-routes each tick.
            (crate::kiko::NINJA_ATTACK, msg::ENTER) => {
                self.vision_on = true;
                self.alert = 1.0;
                self.set_state(crate::kiko::ninja_band(self.dist));
                true
            }
            (crate::kiko::NINJA_ATTACK, msg::UPDATE) => { if self.next_state.is_none() { self.set_state(crate::kiko::ninja_band(self.dist)); } true }
            // The bands 0xa8/0xa9/0xaa (0x16bfc0/0x16c248/0x16c650): advance, then re-dispatch
            // through 0xa7 (the melee/throwing-star anims need the ninja set — not modelled [S]).
            (crate::kiko::NINJA_LONG_RANGE | crate::kiko::NINJA_MID_RANGE | crate::kiko::NINJA_SHORT_RANGE, msg::ENTER) => {
                self.call_anim(d, self.walk_anim(d), 0, 0);
                true
            }
            (crate::kiko::NINJA_LONG_RANGE | crate::kiko::NINJA_MID_RANGE | crate::kiko::NINJA_SHORT_RANGE, msg::UPDATE) => {
                if self.next_state.is_none() { self.set_state(crate::kiko::NINJA_ATTACK); }
                true
            }
            // StandFire 0xb2 (0x16d5b8): hold at the melee band and re-dispatch.
            (crate::kiko::NINJA_STAND_FIRE, msg::ENTER) => { self.call_anim(d, anim_state::AIM_STAND, 0, 0); true }
            (crate::kiko::NINJA_STAND_FIRE, msg::UPDATE) => { if self.next_state.is_none() { self.set_state(crate::kiko::NINJA_ATTACK); } true }
            // KikoMission 0x19 (0x15df28): the route chase — done → prop 0x1d clear → StandBlind.
            // Its sound-class msgs are ignored on 0x07000016 (case 0x11..0x1f `return 1`).
            (crate::kiko::KIKO_MISSION, msg::ENTER) => { self.vision_on = true; self.call_anim(d, 0x34, 0, 0); true }
            (crate::kiko::KIKO_MISSION, msg::UPDATE) => {
                if self.update_mission_route(d, rng, frame, 0x34) && self.next_state.is_none() {
                    d.props.set(0x1d, &mut self.props, 0);
                    self.set_state(crate::kiko::STAND_BLIND);
                }
                true
            }
            (crate::kiko::KIKO_MISSION, msg::SOUND | msg::ALARM | 0x11 | 0x12 | 0x15 | 0x16 | 0x1f) => d.level == 0x0700_0016,
            // KikoMissionRun 0x1a (0x15e0d8): route done → RunToAlarm.
            (crate::kiko::KIKO_MISSION_RUN, msg::ENTER) => { self.vision_on = true; self.call_anim(d, 0x1e, 0, 0); true }
            (crate::kiko::KIKO_MISSION_RUN, msg::UPDATE) => {
                if self.update_mission_route(d, rng, frame, anim_state::RUN) && self.next_state.is_none() { self.set_state(crate::kiko::RUN_TO_ALARM); }
                true
            }
            // StandBlind 0x18 (0x172870): the stand anim only.
            (crate::kiko::STAND_BLIND, msg::ENTER) => { self.call_anim(d, anim_state::STAND_IDLE, 0, 0); true }
            // RunToAlarm 0x89 (0x1736f8): no alarm point reachable on the ported paths → Idle (the
            // none-found arm; the port has no alarm-point registry [S]).
            (crate::kiko::RUN_TO_ALARM, msg::ENTER) => { self.vision_on = true; self.set_state(IDLE); true }
            // EnemyRunToPoint 0x85 (0x168910): the return state 0x56, on 11/12/13/41 with the player
            // not yet seen → 4/5; FindRunToPoint's none-found arm (no AI-point registry in the port,
            // the sweep levels' mode-7 drones provably stand [M 0x168910:32-36]).
            (crate::kiko::ENEMY_RUN_TO_POINT, msg::ENTER) => {
                self.vision_on = true;
                let mut ret = ATTACK;
                if matches!(d.level, 0x0700_0011 | 0x0700_0012 | 0x0700_0013 | 0x0700_0041) && !self.ever_seen {
                    ret = if self.alert < 0.66 { IDLE } else { ALERT };
                }
                self.set_state(ret);
                true
            }
            // DeleteMe 0xb4 (0x173f00): the soft removal (NDrone2_Enable(0) + obj+0xfe |= 1).
            (crate::kiko::DELETE_ME, msg::ENTER) => { self.present = false; self.enabled = false; true }
            (_, msg::ENTER | msg::EXIT | msg::UPDATE) => true,
            _ => false,
        }
    }

    /// `DroneVision_EnemyLookForOpponent` (Attack when seen and the drone may react to the player).
    /// `DroneVision_EnemyLookForOpponent` (0x177ed0): Attack when Bond is seen (prop 0x3C) and the drone is alert
    /// (≥ 0.66) or has prop 0x51; the first such sighting marks it as the drone's own (`+0x4F8 |= 0xC000000`).
    /// Friendly drones (`+0x44 == 3`, team 3: civilians, hostages, allies — dtype 10/0xb/0xd/0xe) never attack:
    /// the game returns 0 before the sight check [M 0x177ed0:31-38].
    fn look_for_opponent(&mut self, d: &Data) -> Option<u8> {
        if self.team == 3 {
            if std::env::var_os("NF_AILOG").is_some() {
                eprintln!("[ailog] guard {} look_for_opponent: team 3 — no opponent", self.index);
            }
            return None;
        }
        let go = self.seen && self.prop(d, 0x3c) && (self.alert >= 0.66 || self.prop(d, 0x51));
        if std::env::var_os("NF_AILOG").is_some() {
            eprintln!("[ailog] guard {} look_for_opponent: seen={} prop_3c={} alert={:.2} prop_51={} -> {}",
                self.index, self.seen, self.prop(d, 0x3c), self.alert, self.prop(d, 0x51), if go { "ATTACK" } else { "none" });
        }
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
    pub fn can_do(&self, d: &Data, anim: u16) -> bool {
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
        // 0x13cfb0:35-39 — `DroneAnim_SetCombatMoveAnim`'s FIRST check precedes every combat
        // move-anim choice, the option list included: a drone directly ahead → state 0x5E.
        if self.no_bunching & 2 != 0 { return state::COMBAT_DRONE_AHEAD; }
        if self.stays(d) { return 0; }
        // The gate's "opponent is aiming at me" is `DroneWeap_OpponentIsAimingAtMe` [M 0x17a500:12-29]:
        // armed Bond AND the +0x2A threat latch `Check_Target` stamps [M 0x1a7810:149-150] — the port's
        // `aimed_at` 30-tick crosshair latch. Pre-fix the armed half was missing, so an unarmed Bond
        // counted as "aiming" and threatened guards skipped their dodge/step/crouch roll.
        let aimed = self.aimed_at && self.bond_armed;
        let bunched = self.bunched;
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
    /// from the firing states). Cover (0x8C) ported (WORKLOG 0023); grenades (0x2D) ported (WORKLOG 0080).
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
                let cover = self.cover_available(d, true);
                if cover { return RUN_FOR_COVER; }
                // `DroneWeap_CanThrowGrenade` — the decomp checks `+0x568 & 0xFFFF00FF == 0xA0002`
                // (anim class 2, type 0xA = AIM_STAND). Our port checks `anim.class == 2`.
                let can_throw = self.anim.class == 2 && self.grenade_count > 0 && self.dist >= 8.0 && self.dist <= 20.0;
                if std::env::var_os("NF_AILOG").is_some() && self.grenade_count > 0 {
                    eprintln!("[ai:gate] guard {} AIM_STAND_FIRE gate: anim={}.{} dist {:.1} unseen {} cover {} clip {} grenades {}x{:#x} can_throw={}",
                        self.index, self.anim.class, self.anim.state, self.dist, self.unseen_ticks, cover, self.clip, self.grenade_count, self.grenade_weapon, can_throw);
                }
                if can_throw && self.can_throw_grenade(d, rng) { return GRENADE_THROW; }
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
                let cover = self.cover_available(d, true);
                if cover { return RUN_FOR_COVER; }
                let can_throw = self.grenade_count > 0 && self.dist >= 8.0 && self.dist <= 20.0;
                if std::env::var_os("NF_AILOG").is_some() && can_throw {
                    let roll = { let mut r = rng.clone(); r.rand(2) };
                    eprintln!("[ailog] guard {} AIM_CROUCH_FIRE: dist {:.1} cover={} grenades={} gw={:#x} anim.class={} can_do_0x49={} rng_roll={}",
                        self.index, self.dist, cover, self.grenade_count, self.grenade_weapon, self.anim.class, self.can_do(d, anim_state::GRENADE_THROW), roll);
                }
                if self.can_throw_grenade(d, rng) { return GRENADE_THROW; }
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
    pub fn in_transition(&self) -> bool {
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
        // 0x13cfb0:35-39, FIRST: a drone directly ahead (`DroneMove_NoBunching`'s +0x31C bit 2)
        // parks the mover in `DSTATE_CombatDroneAhead` (0x5E) instead of a move anim.
        if self.no_bunching & 2 != 0 { self.set_state(state::COMBAT_DRONE_AHEAD); return; }
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

    /// `DroneWeap_CanThrowGrenade` (0x17a798): the drone has grenades (`+0xBCC` / `+0xBCE` ≠ 0), `Rand(2) == 0` (50 %),
    /// an opponent exists, `8.0 ≤ dist ≤ 20.0`, and the throw anim (0x49) is in the drone's anim set.
    /// `NF_AILOG` logs every evaluation's inputs (the user's fix-drone2 ask).
    pub fn can_throw_grenade(&self, d: &Data, rng: &mut Rng) -> bool {
        let log = std::env::var_os("NF_AILOG").is_some();
        if self.grenade_weapon == 0 || self.grenade_count <= 0 {
            if log { eprintln!("[ai:ctg] guard {} weapon={} count={} -> none", self.index, self.grenade_weapon, self.grenade_count); }
            return false;
        }
        let roll = rng.rand(2);
        let dist_ok = self.dist >= 8.0 && self.dist <= 20.0;
        let cando = self.can_do(d, anim_state::GRENADE_THROW);
        let ok = roll == 0 && dist_ok && cando;
        if log {
            eprintln!("[ai:ctg] guard {} weapon={:#x} count={} dist={:.1} roll={} dist_ok={} can_do_0x49={} state={:#x} anim={}.{} -> {}",
                self.index, self.grenade_weapon, self.grenade_count, self.dist, roll, dist_ok, cando, self.state, self.anim.class, self.anim.state, ok);
        }
        ok
    }

    /// `DroneWeap_ThrowGrenade` (0x179558): decrements `+0xBCE`, picks the grenade weapon (mode 0xD sniper → 1/3 chance
    /// of the stun grenade 0x35, else 0x36 the smoke [M 0x1795a8-0x1795b4: `uVar4 = 0x35; if (Rand_Rand(3) == 0) 0x36`]),
    /// computes the throw direction from the right-hand bone (0x15) with a ±π/4 yaw lob
    /// (dist < 10 → +π/4 lob; dist > 18 → −π/4 flat), and creates a `Grenade` projectile. The muzzle falls back to the
    /// drone's chest if the skeleton's bone 0x15 is not posed.
    fn throw_grenade(&mut self, d: &Data, rng: &mut Rng, owner: usize) -> Option<crate::grenade::Grenade> {
        let w = self.grenade_weapon;
        if self.grenade_count > 0 { self.grenade_count -= 1; }
        // Mode 0xD (sniper): 1/3 chance of the smoke 0x36, else the stun 0x35 [M 0x1795a8].
        let gw = if self.mode == 0xd && rng.rand(3) == 0 { 0x36 } else { w };
        if gw == 0 { return None; }
        let wd = d.weapons.records.get(gw as usize)?;
        let f32_at = |o: usize| f32::from_le_bytes(wd[o..o + 4].try_into().unwrap());
        // The throw origin: the right-hand bone (0x15) world position, or the chest fallback.
        let origin = self.muzzle(d);
        // The throw direction: toward the opponent horizontally, with a ±π/4 PITCH lob by distance
        // (the decomp's `Mat_Rotate({π/4, 0, 0, …})` is an X-axis rotation after the yaw matrix — a vertical
        // elevation, not a yaw. Dist < 10 → +π/4 lob up; dist > 18 → −π/4 flat/down; else straight).
        let to_opp = [self.opp_target[0] - origin[0], self.opp_target[1] - origin[1], self.opp_target[2] - origin[2]];
        let dist = (to_opp[0] * to_opp[0] + to_opp[1] * to_opp[1] + to_opp[2] * to_opp[2]).sqrt();
        let horiz = norm([to_opp[0], 0.0, to_opp[2]]);
        let pitch = if dist < 10.0 { std::f32::consts::FRAC_PI_4 } else if dist > 18.0 { -std::f32::consts::FRAC_PI_4 } else { 0.0 };
        let (sp, cp) = (crate::trig::sinf(pitch), crate::trig::cosf(pitch));
        let dir = norm([horiz[0] * cp, sp, horiz[2] * cp]);
        let speed = f32_at(0x20);
        // The grenade's weapon type (`+0x02`) for the explosion's per-type behaviour.
        let weapon_type = u16::from_le_bytes([wd[2], wd[3]]);
        let radius = f32_at(0x08);
        // The fuse is BU+0xFC: 210 ticks, 300 with +0x70 & 0x10000, the mine 0x37 → 420 (7777.0 for a
        // player owner — the port keeps 420, it has no detonator)
        // (`Bullet_init` 0x12c5a0:103,196-223). `weapon_data+0x1C` (5000) is the max-travelled range — a
        // SILENT state-4 death, not the fuse [M Bullet_update case 1 0x12e848]. fix-player3: the old
        // `f32_at(0x1c).max(210.0)` passed 5000, so guard grenades never fuse-exploded (they only died at rest).
        let range = if u32::from_le_bytes(wd[0x70..0x74].try_into().unwrap()) & 0x10000 != 0 { 300.0 } else { 210.0 };
        let range = if gw == 0x37 { 420.0 } else { range };
        // The blast damage is `weapon_data+0x0C` [M 0x12cd98 disasm: `lwc1 $f14, 12($v0)` → Explode_Create's
        // f14 → EXPLODEINFO+0x28 → Explode_Propagate's amount (Explode_CollisionHandler 0x17cde0:13-15);
        // radius = f12/f13 = weapon_data+0x08]. The `addiu $t2, $zero, 0x64` (100) is the 10th argument,
        // an u8 `Explode_Create` never reads (its prologue reads only a0-a3/f12-f14/t3/stack — 0x17cbd0).
        // fix-explosions2: the disc labels name the records — 0x34 'Fragmentation Grenade' = radius 8 /
        // damage 100 (the real frag kill), 0x35 'Stun Grenade' = 10/0 (the flash branch), 0x36 'Smoke
        // Grenade' = 0/0 (the occlude/re-arm branch) [L USATxt.dat + the records].
        let damage = f32_at(0x0c);
        Some(crate::grenade::Grenade {
            pos: origin, velocity: [dir[0] * speed, dir[1] * speed, dir[2] * speed], speed,
            travelled: 0.0, range, damage, radius, weapon_type, mesh: d.weapons.projectile_mesh(self.weapon as usize),
            owner, state: 0, bounces: 0, smoke_popped: false,
        })
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
    fn handle_firing(&mut self, d: &Data, rng: &mut Rng, frame: u32, p: &Player, motion: (bool, bool), owner: usize, lights: &mut crate::light::LightList) -> Option<crate::bullet::Bullet> {
        if !self.firing { return None; }
        self.burst_done = false;
        let mut more = false;
        let mut bullet = None;
        if self.burst == 0 { self.next_bullet_time(d, rng, frame, true); }
        if self.unseen_ticks < 0x10 || self.prop(d, 0x17) {
            more = true;
            // `DroneAnim_PlayFiringAnim` (0x13da48): for the recoil weapons (`+0x3E`, set by `DefaultInit`
            // 0x14b300:801-811 for the anim datum's weapon byte 0x1C / 0x1D) the due shot fires only when
            // the anim's fire event says so (`+0x21`, set by `DroneAnim_EventFunc` 0x12 — the census: the
            // fire anims carry the 0x12/0x10/0xF events at 86% of all kind-2 traffic), with the game's
            // `+0x558 == 0` (no pending call) / `+0x538 == 0` (no blend-out) gates; the shot then plays the
            // 0x5C recoil danim, zeroes the pending call's completion action (+0x596 / +0x598), sets +0x57B
            // and clears +0x40. Non-recoil weapons fire as before (the game's `+0x3E == 0` early-out).
            let fire_now = !self.recoil_weapon
                || (self.event_21 && self.anim.call.is_none() && self.anim.outgoing.is_empty());
            if self.next_shot <= frame && self.ready_to_fire() && fire_now {
                if self.recoil_weapon {
                    self.anim.call.as_mut().into_iter().for_each(|c| { c.1 = 0; c.2 = 0; });
                    self.set_danim(d, 0x5c, 0);
                    self.anim.reached = true;
                }
                bullet = self.fire_weapon(d, rng, frame, p, motion, owner, lights);
                // `DroneWeap_FireWeapon`: `Sound_Play3D(weapon_data[w] +0x60, drone)`, skipped for 0x5EF.
                if let Some(r) = d.weapons.records.get(self.weapon as usize) {
                    let id = u16::from_le_bytes([r[0x60], r[0x61]]) as u32;
                    if id != 0x5ef && id != 0 { self.anim_sounds.push(id); }
                }
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
        // The same posed object the renderer and the hit boxes use (`palette`: blended, aimed, at the drawn height).
        let posed = (|| {
            let skin = d.skins.get(&self.skin)?;
            let (_, datum) = d.weapons.third_person(self.weapon as u8)?;
            let id = if datum == 0xff { 0 } else { datum as u32 } + 1;
            let dt = skin.datums.iter().find(|x| x.id == id && x.bone >= 0)?;
            let pal = self.palette(d)?;
            let m = nf_formats::anim::mul(&nf_formats::anim::quat_trans_to_mat(dt.q, dt.t), pal.get(dt.bone as usize)?);
            Some([m[3][0], m[3][1], m[3][2]])
        })();
        posed.unwrap_or_else(|| {
            let (s, c) = (crate::trig::sinf(self.yaw), crate::trig::cosf(self.yaw));
            [self.pos[0] + s * 0.5, self.pos[1] + 0.4, self.pos[2] + c * 0.5]
        })
    }

    /// `DroneWeap_FireWeapon` + `DoBulletAccuracy`: hit roll, the miss wobble, the cone (`Bullet_init`), damage
    /// (`Drone_ModBulletDamage`: ×2 under 4 units, × `+0x100`).
    fn fire_weapon(&mut self, d: &Data, rng: &mut Rng, frame: u32, p: &Player, (moving, first_moved): (bool, bool), owner: usize, lights: &mut crate::light::LightList) -> Option<crate::bullet::Bullet> {
        let w = d.weapons.records.get(self.weapon as usize)?;
        let f32_at = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        // Hit chance: p = 100 − 5·accuracy, ×2 under TooClose (3), ×0.25 first moved, ×0.75 moving, × level
        // difficulty accuracy (07000005–08: 0.8 / 0.9 / 1.3; difficulty 1 as read live), ×2 again under TooClose.
        let mut pct = 100.0 - 5.0 * self.accuracy as f32;
        let close = self.dist < 3.0;
        if close { pct *= 2.0 }
        if first_moved { pct *= 0.25 }
        if moving { pct *= 0.75 }
        pct *= level_accuracy(d.level, d.difficulty);
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
        // `DroneWeap_FireWeapon` 0x178f68:189 creates the muzzle light on every shot:
        // `Light_Create(radius 7.0, intensity 1.0, the muzzle, rgb = weapon_data+0x56/+0x55/+0x54,
        // channel 0, life 0 → +0x20 = 1 (one tick), kind 1)` — before the bullet's spread draws.
        lights.create(7.0, 1.0, muzzle, [w[0x56], w[0x55], w[0x54]], 0, 0, 1, 0, rng);
        let aim = [p.target[0] + off[0] - muzzle[0], p.target[1] + off[1] - muzzle[1], p.target[2] + off[2] - muzzle[2]];
        let flags68 = u32::from_le_bytes(w[0x68..0x6c].try_into().unwrap());
        let dir = crate::bullet::spread(aim, f32_at(0x24), flags68, rng);
        let mut damage = f32_at(0x0c) * self.damage_mul;
        if self.dist < 4.0 { damage *= 2.0 }
        Some(crate::bullet::Bullet { pos: muzzle, dir, speed: f32_at(0x20), travelled: 0.0, range: f32_at(0x1c), damage, owner, state: 0,
            weapon: self.weapon as u16, flyby: d.weapons.flyby(self.weapon as usize), bounces: 0 })
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
    pub fn visibility(&self) -> f32 {
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
pub struct PlayerBullet { pub pos: V3, pub dir: V3, pub speed: f32, pub travelled: f32, pub range: f32, pub damage: f32, pub weapon: u16, pub first: bool, pub dead: bool,
    /// fix-impacts: the ricochet count (`Bullet_CollisionHandler` 0x12d798's bounce counter, killed
    /// at 3). Field on its own line (merge lesson 4).
    pub bounces: u8,
    /// fix-weapon-modes: the weapon record's `+0x6C` carries the 0x20000 homing bit — `Bullet_update`
    /// 0x12e7b0:483 runs `Bullet_homing` 0x12f358 for those bullets (the AT-600 Scorpion's
    /// 'Heatseeker' record 0x2E; 'Unguided' 0x2F and the Torpedo 'Guided' 0x2C/0x2D do not set it
    /// [L ee_start_05.bin]). No RNG draws anywhere in the homing (determinism).
    pub homing: bool,
    /// `BU+0x40`: the current heat-seek target — an index into `Drones::list` (`Bullet_homing`'s
    /// `*(int*)(BU+0x40)` object pointer; none = re-scan, and the scan re-runs when the target
    /// dies [M 0x12f3b4: the `+0xfe & 1` clear-and-rescan]).
    pub target: Option<usize> }

/// What one of Bond's bullets hit: a drone (index, part) or the world, and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerImpact { Drone { drone: usize, part: i16, at: V3 }, World(V3) }

/// A sound in the hearing list (`Sound_Play3D`: position, inner / outer radius, weight).
#[derive(Clone, Copy, Debug)]
pub struct Sound { pub id: u32, pub pos: V3, pub inner: f32, pub outer: f32, pub weight: f32 }

/// The script fade overlay's state (`Camera_SetFade` 0x1bbdd8's viewer fields +0x228/+0x22C): the
/// black overlay's alpha and its per-tick rate. Ticked in `Drones::tick`; drawn by nf-view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewFade { pub alpha: f32, pub rate: f32 }

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
pub fn level_drones(d: &Data, level: &nf_formats::level::Level, rng: &mut Rng, lights: &mut crate::light::LightList) -> Vec<Drone> {
    // The map parse's ambient radiators precede every dynamic-object create [M parsemap_handle_block_id 0x1d1020].
    crate::light::level_lights(level, rng, lights);
    let Some(m) = level.main_map() else { return Vec::new() };
    level.maps[m].placements.iter().enumerate().filter_map(|(i, p)| {
        // The drones (0x0F) and the type-0x4E lights are created by the same
        // `parsemap_create_dynamic_objects` pass in placement order, so their RNG draws interleave here.
        if p.flags & 0xff == 0x4e { crate::light::placement_light(level, m, i, rng, lights); return None; }
        if p.flags & 0xff != 0x0f { return None; }
        let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
        let floor = d.world.floor_below(p.translation, 3.0).unwrap_or(p.translation[1] - 1.0);
        let mut dr = Drone::new(d, [p.translation[0], floor + 1.0, p.translation[2]], p.rotation[1], param, rng);
        dr.placement = i;
        Some(dr)
    }).collect()
}

/// Per-level accuracy factors by difficulty (`ReadTuningVars` 0x1d3a80). The difficulty itself is
/// `Drones::difficulty` (`GameState+0x40`, 0x2a37a8 — NOT +0x28; fix-progress made it runtime).
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
    /// Grenades in flight (`Bullet_init` with a gravity-affected weapon; WORKLOG 0080).
    pub grenades: Vec<crate::grenade::Grenade>,
    /// Bounced grenades awaiting the `DroneFunc_HandleExplosives` broadcast (position, weapon record id).
    pub explosive_queue: Vec<(V3, u16)>,
    /// Explosions this tick: (pos, radius, damage, owner, weapon_type) for the sound + camera shake.
    pub explosions: Vec<(V3, f32, f32, usize, u16)>,
    /// The camera-shake amount: `Explode_Create` 0x17cbd0 calls `Camera_Shake(pos, radius)` when the
    /// radius reaches 0.5 [M 0x17cc28-0x17cc44]; the port decays it 10%/tick − 0.05 [*suggested*: the
    /// game's `Camera_Shake` curve is view-side, not modelled]. The view reads it for the offset. fix-explosions.
    pub shake: f32,
    pub health: crate::bullet::PlayerHealth,
    /// Every hit on Bond: (tick, shooting drone, health lost).
    pub player_hits: Vec<(u32, usize, f32)>,
    /// Impacts this tick (for effects).
    pub impacts: Vec<crate::bullet::BulletEvent>,
    /// fix-impacts: bullet impact effects — the 200-decal FIFO and the debris
    /// (`Effect_Bullet` 0x1ce8a8, `Debris_Create` 0x1aaa00; docs/atlas/effect.md). Fields on their
    /// own lines (merge lesson 4).
    pub effects: crate::effect::Effects,
    /// fix-impacts: ejected shell casings (`Bullet_InitCasingLists` 0x12fd48's 32-slot pool,
    /// `crates/nf-game/src/casing.rs`).
    pub casings: crate::casing::Casings,
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
    /// The level's `light_tag` records (`LightList` 0x2c74c0): level placements, ambient radiators,
    /// the searchlights' kind-2 beams and the muzzle flashes (fix-lighting).
    pub lights: crate::light::LightList,
    /// The last alarm broadcast (`DAT_0029AA88`) and whether one went out (`DAT_0029AA85`, 07000014 sends only one).
    pub last_alarm: Option<u32>,
    /// The viewer's +0x2a HUD flag [M Camera_Enable 0x1b9920:27-28]: `Script_CameraStart` 0x1d9140
    /// disables the viewers' HUD (`Camera_Enable(i, 0, 0, 0)` for 0..7) and `Script_KillStream`
    /// 0x1d9860 restores it (`Camera_PopStates`). While it is 0, `Text_UpdateMsg` 0x1b11d8:16-18
    /// returns without ticking the message timers — the objective/mission banners WAIT until the
    /// cut-scene ends (playtest item 5: they used to run over the cut-scenes). The pane state
    /// machines (0x19c310 case 0) poll the same gate, so a banner arriving mid-NIS stays closed too.
    pub hud_frozen: bool,
    /// The script camera's letterbox borders (`Script_CameraStart` 0x1d9140's two border sprites of
    /// SpriteInfo 0x2c9468 at draw layer 1, the bottom at +0x26 = 0x180; `Script_KillStream` case 4
    /// hides them) — true while a script camera is live. nf-view draws two solid black 512×64 bars
    /// [L ACTION.ELF 0x2c9468: x 0, y 0, w 512, h 64, colour 0x000000ff, texture 0].
    pub letterbox: bool,
    /// The script fade overlay (`Camera_SetFade` 0x1bbdd8's viewer state, +0x228..+0x235): the black
    /// overlay's alpha and its per-tick rate (1/|duration|; negative duration = fading in from
    /// black). Armed by op 0x13, `Script_Update`'s length−20 auto fade-out and the −22 handback
    /// fade-in; ticked in `Drones::tick` (the game ticks it in the viewer update) [S: the viewer
    /// state's exact lifecycle — who clears +0x235 — is not decoded; the port parks the alpha at the
    /// target and re-arms only from an empty state].
    pub view_fade: Option<ViewFade>,
    /// Op 0x1e script messages (`Text_AddMsg(-1, 0, 4, Txt_BindLabel(label), 0, time)`, drained to
    /// the HUD like `taken`): (label, display frames).
    pub script_msgs: Vec<(u32, u16)>,
    /// The section the IOP player reports (`0x2500CC`, read by `UpdateMusicalEvents`): the audio layer
    /// writes it each frame; the music update at the tick's end consumes it (fix-party-mission item 4 —
    /// the game's `UpdateMusicalEvents` runs every frame whether or not an output device exists).
    pub iop_section: i32,
    /// `DAT_0029aa86` — the interrogate latch: 0 while the party is calm, 1 once an interrogation began
    /// (`DroneVision_BeginInterogate` 0x1782a8; `EndInterogate` 0x1782c8 never clears it on 07000007).
    /// Gates `DroneFunc_DoForcedAttack` 0x148c60's forced attacks on 07000007 (playtest item 3: one punch
    /// turned every guest hostile). The interrogate state itself is not ported yet, so only the gate is.
    pub party_loud: bool,
    /// Auto-aim: the last target (`BL+0x114`) and the crosshair offset as (yaw, pitch) radians (`BL+0x120` / `+0x124`,
    /// screen units in the game; *approximation*: small angles).
    pub aim_target: Option<usize>,
    pub aim_offset: [f32; 2],
    pub alarm_sent: bool,
    pub bond_aiming: bool,
    /// The pending level-exit request: when a `LOAD_LEVEL` (0xE8) trigger fires, the next level's hashcode
    /// (`Trigger_Activate` 0x1b2318 → `ResetMap_LevelToLoad` 0x1d3800). The main loop reads this to reload [M].
    pub level_exit: Option<u32>,
    /// The gas/smoke puffs (`Gas_Init` 0x1aacc8 / `gases_smoke` 0x1ab2d0, crates/nf-game/src/gas.rs):
    /// muzzle smoke, broken-searchlight smoke, bullet trails. Fixed slot pool, updated per tick.
    pub gas: crate::gas::GasPool,
    /// Bond's weapon byte (player `anim+0x62`; the P2K = 2).
    pub bond_weapon: u8,
    /// Weather (`DrawSnow`: a `SNOW_gen` placement, type 0xDF): the drop type and volumes from the level, built at the
    /// first tick; the generator of the first-frame fill (before the snapshot point, so not the shared one).
    pub env: Option<crate::env::Drops>,
    env_pending: Option<(u8, Vec<crate::env::RainBox>)>,
    fill_rng: Rng,
    /// Pickups in the world, Bond's inventory and what he took (tick, what) — docs/systems/pickups.md.
    pub pickups: Vec<crate::pickup::Pickup>,
    pub inventory: crate::pickup::Inventory,
    pub taken: Vec<(u32, crate::pickup::Taken)>,
    cover_cursor: usize,
    cover_counter: i32,
    /// Script players and the objects their scripts made (docs/systems/script-player.md).
    pub scripts: crate::nis::Scripts,
    /// Monotonic counter for `control_add_object_to_list`'s prepend order: each spawner spawn gets the next seq;
    /// a spawned drone's sort key is `usize::MAX - seq`, so the newest spawn updates first (core-loop §7 item 9).
    spawn_counter: u32,
    /// Player hand-offs the scripts asked for this tick (`PlayerDisable` / `PlayerEnable`), for the player's owner.
    /// The game's drone-freeze gate (`NPCGlobals`, `Drone_EnableAll` 0x1383e8): `Drone_Control` (0x139268:39-42),
    /// `DroneSpawner_Control` (0x137e68:21), `Drone_CollisionHandler` (0x140968:62) and `Drone_InitComms`' shared
    /// passes (0x139520:124) all skip while it is clear. Set by script event 5, restored by `Script_Free`'s 0x80
    /// flag (event 7) [M]; a cut-scene WITHOUT event 5 keeps its guards live — the port's `Sim::disabled` must NOT
    /// touch this (core-loop port map §7 items 1-2).
    pub npc_enabled: bool,
    /// `Drone_bDisableSystem` (0x30ca9e): `DroneSpawner_Control` returns while it is set
    /// (0x137e68:24-26), and `DroneSpawner_Create` never creates a spawner (0x1378d0:12). The
    /// writers are the options-menu "drones off" setting (`switch_NO_DRONES` 0x30ce2c —
    /// `Drone_PauseNoDraw` 0x13a710 ← `Drone_InitComms` 0x139520:287, and `ResetMap_GameInit`
    /// 0x1d5080:14); the port has no options menu, so it stays false — the gate keeps Control's
    /// early-out order faithful for whoever wires the setting (fix-spawner-p7).
    pub npc_system_disabled: bool,
    pub player_events: Vec<crate::nis::Request>,
    /// The EE sound pool (`DynamicSoundList`, docs/systems/sound.md): what is heard, as opposed to `sounds`, the AI
    /// hearing list.
    pub audio: crate::sound::Sounds,
    /// Map ambient sounds (block 0x28 of the main map).
    pub map_sounds: crate::sound::MapSounds,
    /// fix-death-restart: the death sequence (`Player_CheckForDeath` 0x190810 +
    /// `Player_HandleDeath` 0x190bb0, `HUD_UpdateBloodPane` 0x19a6a8) — the red fade, the ✕ skip
    /// and the ch 0x62 gate. Fresh at every load (the game's death state is not saved).
    pub death: crate::death::Death,
    /// fix-death-restart: this tick's action 25 (✕ / Start, player 0 [L input.md style 0]) —
    /// `Sim::step_bond`'s fire button, the death sequence's skip gate [M 0x190bb0:30].
    pub bond_fire: bool,
    /// fix-death-restart: the pre-menu auto-restart. While 180-fix-menus has no drawing, the port
    /// answers the 0x40000042 menu with Restart immediately (`fail_choose_restart`); the flag turns
    /// that off so the view can answer instead.
    pub auto_restart: bool,
    /// fix-death-restart: the pending exit is a COLD restart (`ResetMap_LevelToLoad(BaseMap, 0, 1)`
    /// [M 0x204408:139] — IsWarmReset 0: `Init_SwitchChannels` + no `Player_RamLoad` [M 0x1d5080,
    /// 0x1957a8]). The viewer's reload drops the RamSave for this exit. (fix-restart-target: this is
    /// now only the PAUSE menu's Restart — the fail/death menu's Restart is the warm arm, see
    /// `level_exit_restart`.)
    pub level_exit_cold: bool,
    /// fix-restart-target: the pending exit is the fail/death menu's Restart — `P_ENDMISSION_Handler`
    /// 0x216618:19-21, menu 0x40000042's row 0: `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` +
    /// `Player_RamLoad(Bond, 1)`. The viewer's reload re-serves the ENTRY checkpoint (the RamSave as
    /// of level entry, ch 99 clean) with the 50.0 health floor [L 0x30cc34-3c] instead of capturing
    /// the dying state — carrying the dying state is what looped the 07000007 session.
    pub level_exit_restart: bool,
    /// fix-restart-target: the level the sim is playing — `GameState._12_4_` [M 0x1d5080:6, set from
    /// the loaded level at `ResetMap_GameInit`], the fail-menu Restart's reload target [M 0x216618:20].
    /// Latched where `Mission_Init` runs (`Drones::tick`).
    pub level_current: u32,
    /// The EE music logic and the drones' attack gate (docs/systems/music.md); the alarm counters of
    /// `DroneFunc_CheckAlarmRaised` (0x1453a8: channel 0xA4, then 0x37 / 0x61).
    pub music: crate::music::Music,
    pub attack_gate: crate::music::AttackGate,
    alarm_music: [u16; 3],
    /// `EventList[2]` as of the LAST walk's end (`Music_Event(2, …)` posts: CheckAlarmRaised 0x1453a8,
    /// SeenAndAttacking 0x145150). `UpdateMusicalEvents` 0x114e70 clears the EventList once per frame
    /// (from `SFXUpdate` 0x1156c0 in `Game_Run`'s pre/post passes), so a post made later in the walk
    /// than a script player survives into the player's next read — the SP start gate
    /// (`SP_Update` 0x1dbda8:96-100) sees the LAST walk's event, not this walk's.
    pub prev_event2: u32,
    /// Speech: each line's last play frame (`Drone_SfxInfo+0x08`, global; `None` = never) and the patrol-talk cooldown.
    voice_last: Vec<Option<u32>>,
    patrol_cooldown: u32,
    /// Script stream sounds: (script, stream) → handle and the object it follows.
    pub script_sounds: Vec<((u32, u8), crate::sound::Handle, Option<usize>)>,
    /// The level's searchlights (docs/systems/world-objects.md; created in `attach_level`, updated in `tick`'s
    /// control-list order with the drones and triggers).
    pub searchlights: crate::searchlight::Searchlights,
    /// The use objects (map-tower): the Q-Worm PCs (0x35) and switches (0x29) of
    /// `Player_Activate` 0x1a5170 — crate::useobj.
    pub useobjs: crate::useobj::UseObjects,
    /// The level's doors (docs/systems/world-objects.md "Doors"); created in `attach_level`, updated in `tick`'s
    /// control-list order (`Door_Update`), the group sync (`Doors_Calc`) at the pass's end.
    pub doors: crate::door::Doors,
    /// The fuseboxes (`FuseBox_Create` 0x1b0448; map-tower2): 07000014's OBJECT_FuseBox1 sabotage boxes.
    pub fuseboxes: Vec<crate::fusebox::FuseBox>,
    /// The helicopter (`Copter_Create` 0x17d340, placement 0x2F; fix-objects2): 07000008's
    /// extraction copter — gated on ch 5, its arrival writes ch 0x4B (the last mission objective).
    pub copters: Vec<crate::copter::Copter>,
    /// The locks (`Lock_Create` 0x1ae1f8, placement 0x2E; fix-objects2): the laser-burn targets —
    /// 07000008's safe bolts (chs 134/135 → mux 136 → the safe door).
    pub locks: Vec<crate::lock::Lock>,
    /// fix-safe08: (script hash, stream) → the renderer's reserved pickup slot for that script's
    /// obj_type-0xF0 Entity (`Script_CreateEntity` 0x1d8968's `Pickup_Create` arm). The slot is
    /// built at load (scene.rs) with `gone = true` (undrawn, inert) and goes live when the script
    /// spawns it.
    pub script_pickup_slots: HashMap<(u32, u8), usize>,
    /// The auto-switch request `Pickup_Handler` → `Player_EquipWeapon`'s tail wrote this tick
    /// [M 0x1a8a58: the `DAT_002a38d2[idx·0x158]` flag (live 1 [L ee_start_05]) ∧
    /// `Player_IsBetterWeapon` → `anim+0x63 = id`]. The view applies it to the draw machine
    /// (gun.switch_request).
    pub weapon_switch_request: Option<u16>,
    /// The grapple points (placement 0x3A, `Grapple_Create` 0x198298; fix-gadgets).
    pub grapple_points: Vec<crate::gadget::GrapplePoint>,
    /// The animated SimpleScripts (`SS_Create` 0x1a9c58 / `SS_Update` 0x1a9948; map-tower2): the
    /// 07000012 lift screens, shutters and lift doors swap on their switch channels.
    pub sss: crate::ss::SsList,
    /// Bond's bullet segments of this tick (`from`, `to`): the searchlights' break check (U-041 stand-in).
    bullet_paths: Vec<([f32; 3], [f32; 3])>,
    /// The mission layer (`Mission_Update` 0x182c60, docs/systems/mission.md). `inited` latches the
    /// per-level `Mission_Init` (0x182270, from `ResetMap_GameInit`) on this level's first tick.
    pub mission: crate::mission::Mission,
    /// The previous level's mission globals (map-tower2): restored at the `Mission_Init` latch, after
    /// init, the way the game's `.data` globals sit under it [M 0x182270:68-129].
    pub mission_carry: Option<crate::mission::MissionSave>,
    /// The photograph targets and breakable doors (crate::props; placements 0xFD / 0xE1 —
    /// `CamSubject_Create` 0x1bc5d8, `Destroy_Create` 0x12c318), created in `attach_level`.
    pub props: crate::props::Props,
    mission_inited: bool,
    /// The mission layer's events this tick (`Text_AddMsg` types 2/3, the FailLabel writes, the
    /// level-end/reset decisions) — drained by the view each tick.
    pub mission_events: Vec<crate::mission::Event>,
    /// The weapon of each `bullet_paths` entry, in order (crate::useobj's 0x58 install check).
    bullet_path_weapons: Vec<u16>,
    /// The route-node → door binding (`NDrone_InitDoorNodes` 0x139a70: each door-flagged node binds
    /// its nearest door object), keyed (nav path, node id) → `Doors::list` index. Computed lazily on
    /// the first tick that needs it (the doors' positions are static).
    node_doors: Option<HashMap<(usize, u16), usize>>,
    /// The kick-node → ScriptPlayer binding (`NDrone_InitKickNodes` 0x139ce0), keyed (nav path,
    /// node id) → `Scripts::players` index. Built lazily like `node_doors`.
    node_kicks: Option<HashMap<(usize, u16), usize>>,
    /// The kick nodes' first-crossing flags — the game sets bit 0x8000 on the NODE (0x153358:141),
    /// shared by every drone and never cleared. The port keeps them here (the state machine cannot
    /// mutate the network).
    kicked_nodes: std::collections::HashSet<(usize, u16)>,
    /// The placement-gated world objects (crate::worldobj; placements 0x41 / 0xd2 / 0x3b / 0xe7 —
    /// `InverterTrigger_Create` 0x1b3040, `Shooter_Create` 0x17d030, `DynamicObject_Create` 0x18dbf8,
    /// `ThirdCam_Create` 0x1bc510), created in `attach_level`.
    pub worldobjs: crate::worldobj::WorldObjects,
    /// The world presentation FX (0x21 ripples, 0xe3 flicker, 0xd4 cloud drift, 0x4a apocalypse,
    /// 0xf2 emitters — crates/nf-game/src/world_fx.rs, fix-world-fx).
    pub fx: crate::world_fx::WorldFx,
    /// The mission-progress layer (crates/nf-game/src/progress.rs, docs/systems/progress.md):
    /// `PlrMissionStats` 0x2bcc20, `BondMoments` 0x2bcde0, the `ScoringTable` runtime, the `bonus`
    /// bitmask, the `sp_level` flags and the save codec (fix-progress). The tally hooks fold the
    /// per-drone outboxes in `tick`; the menus (180-fix-menus) read it through here.
    pub progress: crate::progress::Progress,
    /// `GameState+0x40` (0x2a37a8): the difficulty — 1 Agent / 2 Secret Agent (the boot default
    /// [M bootup 0x1c7e78:25]) / 3 00 Agent / 4 the bonus double-damage mode [M Player_HandlePain
    /// 0x1902c8:29-48]. The old `DIFFICULTY` const (hardcoded 1, commented as `GameState+0x28` —
    /// the wrong offset; the scored member is +0x40 [M PlrStat_GetScore 0x1984b0:295]) stood in
    /// for this. The difficulty page (180-fix-menus) writes it through `set_difficulty`.
    pub difficulty: u32,
}

/// `NDrone2_DefaultInit` (0x14b300): the voice set `+0xD8` from the character id (skin 0x05000000 + id).
pub fn voice_type(skin: u32) -> u16 {
    match skin & 0xffff {
        0x04 | 0x06 | 0x08 | 0x4c => 3, 0x0e | 0x0f => 0x15, 0x10 | 0x11 | 0x52 => 0x16, 0x12..=0x14 => 0x11,
        0x15 | 0x20 | 0x22..=0x24 => 0xd, 0x16 | 0x8f | 0xac => 0x14, 0x17 | 0x18 => 0x12, 0x19 | 0x21 => 0xe,
        0x1b | 0x4f => 9, 0x1e => 0xb, 0x25 => 5, 0x26 | 0x66 => 0x10, 0x2b | 0x2d | 0x34 => 8, 0x2e..=0x30 => 2,
        0x35 => 0xf, 0x36..=0x3a => 1, 0x3b | 0x3c => 0xc, 0x27..=0x2a | 0x31..=0x33 | 0x53 => 0x17,
        0x54 | 0x6d | 0x6e | 0x7e => 4, 0x5e => 0x13, 0x78 | 0x94 | 0xba => 0x18, 0x87 | 0x90 | 0x93 => 6,
        0x8c | 0xb1 | 0xb2 => 7, _ => 0,
    }
}

/// The script runtime's view of the level data: animation lengths and an animated object's discrete object 0.
impl crate::nis::World for Data<'_> {
    fn anim_length(&self, script: u32) -> Option<u16> { self.scripts.get(&script).map(|s| s.length) }
    fn level(&self) -> u32 { self.level }
    fn discrete0(&self, skin: u32, script: u32, frame: f32) -> Option<nf_formats::anim::Mat4> {
        self.discrete_parts(skin, script, frame).into_iter().next().map(|p| p.1)
    }
    /// The op-1 sounds of `script` with frame in `(from, to]`, in command order (`AnimProcessScriptCmds`
    /// 0x1c1b38 case 1 iterates the commands forward). The weapon-id remaps are skipped: they only apply when
    /// the anim object has a weapon parent (`obj+0x1c` ≠ 0, the view model / held-weapon case), which a script
    /// anim object never has [M 0x1c1b38 case 1: the swap tests all read `*(int *)(param_1 + 0x1c)`].
    fn anim_ops1(&self, script: u32, from: i32, to: i32) -> Vec<u32> {
        self.scripts.get(&script).map(|s| s.commands.iter().filter(|c| c.op == 1).filter_map(|c| match c.args.as_slice() {
            [f, sfx, ..] if (*f as i32) > from && (*f as i32) <= to => Some(*sfx as u32),
            _ => None,
        }).collect()).unwrap_or_default()
    }
}

impl Data<'_> {
    /// An animated script object's discrete objects (the skin's rigid parts): glist hash and world matrix. The root
    /// keeps its translation: the object sits at the origin and follows it (`Script_AnimStart`).
    pub fn discrete_parts(&self, skin: u32, script: u32, frame: f32) -> Vec<(u32, nf_formats::anim::Mat4)> {
        let pose = || -> Option<Vec<(u32, nf_formats::anim::Mat4)>> {
            let sk = self.skins.get(&skin)?;
            let skel = self.skeletons.get(&sk.skeleton)?;
            let (q, f) = crate::anim::script_frame(self.scripts.get(&script)?, frame)?;
            let seq = self.sequences.get(&q).filter(|q| nf_formats::anim::fits(q, skel))?;
            let world = nf_formats::anim::world_pose(&nf_formats::anim::local_pose(seq, skel, sk, f as u32), sk);
            Some(sk.extra.iter().filter_map(|&(bone, part)| Some((part?, *world.get(bone as usize)?))).collect())
        };
        // A sequence without a channel for every bone (cut-scene 0x06000997 on 0x0500002E: its sequence is not this
        // archive's full set) gives no parts.
        pose().unwrap_or_default()
    }
}

impl Drones {
    pub fn new(mut list: Vec<Drone>, rng: Rng) -> Drones {
        // `Drone_SM_InitObject` numbers state machines in creation order (the base is not known: 0 here).
        for (i, dr) in list.iter_mut().enumerate() { dr.sm_id = i as u32; dr.index = i; }
        let llist = (0..list.len()).collect();
        let mut drones = Drones { list, frame: 0, los_cursor: 0, llist, rng, lights: Default::default(), bullets: Vec::new(), grenades: Vec::new(), explosions: Vec::new(), shake: 0.0, health: Default::default(), player_hits: Vec::new(), impacts: Vec::new(), prev_player: None, moved_since: None, target: PlayerTarget::default(), used: Vec::new(), channels: Default::default(), triggers: Vec::new(), spawners: Vec::new(), cover_rt: Vec::new(), sounds: Vec::new(), cover_cursor: 0, cover_counter: 0, player_bullets: Vec::new(), player_impacts: Vec::new(), last_flinch: None, bond_aiming: false, level_exit: None, bond_weapon: 2, env: None, env_pending: None, fill_rng: Rng::new(0x5eed), last_alarm: None, alarm_sent: false, party_loud: false, iop_section: 0, hud_frozen: false, aim_target: None, aim_offset: [0.0; 2],
            pickups: Vec::new(), inventory: crate::pickup::Inventory::level_start(0x73, 34), taken: Vec::new(), scripts: Default::default(), player_events: Vec::new(), npc_enabled: true,
            npc_system_disabled: false,
            spawn_counter: 0, audio: Default::default(), script_sounds: Vec::new(), map_sounds: Default::default(), music: Default::default(), attack_gate: Default::default(), alarm_music: [0; 3], voice_last: Vec::new(), patrol_cooldown: 0, searchlights: Default::default(), doors: Default::default(), fuseboxes: Vec::new(), sss: Default::default(), bullet_paths: Vec::new(), props: Default::default(), mission: crate::mission::Mission::new(), mission_carry: None, mission_inited: false, mission_events: Vec::new(), useobjs: Default::default(), bullet_path_weapons: Vec::new(), copters: Vec::new(), locks: Vec::new(), grapple_points: Vec::new(), prev_event2: 0,
            node_doors: None,
            // fix-nav-remainder (fields on their own lines — merge lesson 4).
            node_kicks: None,
            kicked_nodes: Default::default(),
            gas: Default::default(),
            // fix-death-restart: the death sequence + the fail-menu policy (fields on their own lines
            // — the shared-constructor collisions, merge lesson 4).
            letterbox: false, view_fade: None, script_msgs: Vec::new(),
            death: Default::default(), bond_fire: false, auto_restart: true, level_exit_cold: false,
            // fix-restart-target: the fail-menu Restart's warm arm + the current level (GameState+0xC).
            level_exit_restart: false,
            level_current: 0,
            // fix-safe08: (script hash, stream) → the reserved pickup slot the renderer built for the
            // script's obj_type-0xF0 Entity (`Script_CreateEntity` 0x1d8968's Pickup_Create arm);
            // built at load in scene.rs, matched by the runtime spawn.
            script_pickup_slots: Default::default(),
            // fix-pickups-remainder: the pickup auto-switch request (fields on their own lines —
            // merge lesson 4).
            weapon_switch_request: None,
            worldobjs: Default::default(),
            // fix-world-fx: the presentation FX (fields on their own lines — merge lesson 4).
            fx: Default::default(),
            effects: crate::effect::Effects::new(),
            casings: crate::casing::Casings::new(),
            // fix-progress: the tally/scoring layer and the runtime difficulty (fields on their
            // own lines — the shared-constructor collisions, merge lesson 4).
            progress: Default::default(),
            difficulty: 2, // [M bootup 0x1c7e78:25 GameState._40_4_ = 2]
            // fix-cover-remainder: the HandleExplosives queue (own line — merge lesson 4).
            explosive_queue: Vec::new(),
    };
        drones
    }

    /// `GameState+0x40`'s setter (C_SBNFDFCTY_Handler 0x216ca8:10-17 writes the difficulty page's
    /// selection; the boot default is 2 [M 0x1c7e78:25]). The menus (180-fix-menus) call this.
    pub fn set_difficulty(&mut self, d: u32) {
        self.difficulty = d.clamp(1, 4);
        self.progress.difficulty = self.difficulty;
    }

    /// Loads the level's trigger objects and gives each drone its placement index (matched by position among the
    /// main map's drone placements), which orders the per-frame object updates.
    pub fn attach_level(&mut self, level: &nf_formats::level::Level) {
        self.triggers = crate::triggers::build(level);
        // The photograph targets (0xFD) and breakable doors (0xE1) [M CamSubject_Create 0x1bc5d8,
        // Destroy_Create 0x12c318 — the game creates them in placement order in
        // parsemap_create_dynamic_objects 0x1d04f0].
        self.props = crate::props::Props::create(level);
        // The Q-Worm PCs (0x35) and switches (0x29) of `Player_Activate` 0x1a5170 (map-tower;
        // `parsemap_create_dynamic_objects` 0x1d04f0 cases 0x35/0x29, crate::useobj).
        self.useobjs = crate::useobj::UseObjects::create(&level.maps[level.main_map().unwrap_or(0)].placements);
        self.useobjs.init_channels(&mut self.channels, self.frame);
        let Some(m) = level.main_map() else { return };
        // Searchlights (`Searchlight_Create`, placement order; every part draws `Rand_Rand(480)`, spots another
        // `Rand_Rand(0)` — the shared stream, in this order). The game interleaves these draws with the other
        // dynamic objects' creates by placement index; the port creates drones first (level_drones) and these
        // here, so absolute stream positions differ until all creators are ported (seeds re-picked, WORKLOG).
        self.searchlights = crate::searchlight::Searchlights::create(&level.maps[m].placements, &mut self.rng, |p| {
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
                else { level.maps[m].objects.get(p.index as usize) };
            obj.map_or(([0.0; 3], [0.0; 3]), |o| (o.min, o.max))
        });
        // `Searchlight_Create`'s kind-2 light record (0x1cb4f8:150-172): white, radius 500, intensity 1.0,
        // kind 2, permanent. The node's position (+0x10) is the PLACEMENT position written before the
        // −10000 move and never changed [M 0x1cb4f8:154-167]; `psiLight_SetLights` 0x10e588 uploads it
        // for the light's whole life — the port carries it as `spawn_pos` and resolves the live spot
        // separately (light.rs `ResolvedLight::track_pos`). The record's only RNG draw is the
        // `Rand_Rand(0)` flicker seed already taken in the create above.
        for (k, part) in self.searchlights.parts.iter().enumerate() {
            if part.kind != 2 { continue; }
            self.lights.lights.push(crate::light::Light {
                pos: part.spawn_pos, life: 0, channel: 0, flicker_period: 0, flicker_base: 0,
                flicker_count: part.light.unwrap_or(0) as u16, flicker_mask: 0, sound: 0,
                track: Some(k as u32), radius: 500.0, intensity: 1.0, falloff: 0.0,
                rgb: [255, 255, 255], kind: 2, active: true,
            });
        }
        // Doors (`Door_Create`, placement order — no RNG draw). The anim comes from the level's 0x19 key-list
        // blocks (`m_paths`), the sphere from the placement object's own gfx sphere (`Control_BuildWorldSph`).
        let path_blocks = level.path_blocks();
        self.doors = crate::door::Doors::create(&level.maps[m].placements, &path_blocks, |p| {
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
                else { level.maps[m].objects.get(p.index as usize) };
            obj.map(|o| {
                // When the object's hashcode is u32::MAX (anonymous by-index door), use the synthetic key
                // (0x80000000 | (map << 16) | object) that `CollisionWorld::build` indexes it under [S].
                let glist = if o.hashcode == u32::MAX {
                    let map_idx = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(mm, _)| mm).unwrap_or(m) } else { m };
                    0x8000_0000 | ((map_idx as u32) << 16) | p.index as u32
                } else { o.hashcode };
                (glist, o.centre, o.radius)
            })
        });
        self.doors.create_channels(&mut self.channels, self.frame);
        // The world objects (InverterTrigger 0x41, Shooter 0xd2, DynamicObject 0x3b, ThirdCam 0xe7;
        // parsemap_create_dynamic_objects 0x1d04f0 cases, crate::worldobj — placement order, no RNG).
        self.worldobjs = crate::worldobj::WorldObjects::create(&level.maps[m].placements, &path_blocks, |p| {
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
                else { level.maps[m].objects.get(p.index as usize) };
            obj.map_or((0, [0.0; 3]), |o| (o.flags, o.centre))
        });
        // Fuseboxes (`FuseBox_Create` 0x1b0448, placement 0x31 with a script hash): 07000014's
        // OBJECT_FuseBox1 security-sabotage boxes (map-tower2).
        let (fuseboxes, _spheres) = crate::fusebox::FuseBox::create(&level.maps[m].placements, |p| {
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
                else { level.maps[m].objects.get(p.index as usize) };
            obj.map(|o| (o.centre, o.radius))
        });
        self.fuseboxes = fuseboxes;
        // SimpleScripts (`SS_Create` 0x1a9c58): the 0xE2 placements with a non-zero key count, their
        // keys from the main map's 0x19 blocks (map-tower2).
        self.sss = crate::ss::SsList::create(&level.maps[m].placements, &path_blocks, |p| {
            // The SS's collision mesh: the placement object's glist hashcode, the same resolution
            // `Doors::create`'s closure does (the game's SS object carries its glist at create
            // [M SS_Create 0x1a9c58]; Collide_PickObj walks the cel-linked objects).
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
                else { level.maps[m].objects.get(p.index as usize) };
            obj.map(|o| if o.hashcode == u32::MAX { 0x8000_0000 | ((m as u32) << 16) | p.index as u32 } else { o.hashcode }).unwrap_or(0)
        });
        // The helicopter (`Copter_Create` 0x17d340, placement 0x2F) and the locks (`Lock_Create`
        // 0x1ae1f8, placement 0x2E) (fix-objects2). Neither draws RNG at create.
        self.copters = crate::copter::Copter::create(&level.maps[m].placements);
        self.locks = crate::lock::Lock::create(&level.maps[m].placements, |p| {
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
                else { level.maps[m].objects.get(p.index as usize) };
            obj.map(|o| (o.centre, o.radius))
        });
        // The grapple points (`Grapple_Create` 0x198298, placement 0x3A, class 0x51; fix-gadgets) —
        // the Q-Claw's fire targets, the same world-sphere shape as the ladders.
        self.grapple_points = crate::gadget::GrapplePoint::create(&level.maps[m].placements, |p| {
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
                else { level.maps[m].objects.get(p.index as usize) };
            obj.map(|o| (o.centre, o.radius))
        });
        // Weather: `Env_Create` (placement 0xDF, parameter 0 = drop type) and the `RainBox_Create` volumes (0x32: the
        // glist bounds at the placement translation).
        self.map_sounds = crate::sound::MapSounds::new(level.maps[m].sounds.clone());
        let ps = &level.maps[m].placements;
        let lib = level.nis.iter().map(|(&h, n)| (h, std::sync::Arc::new(n.clone()))).collect();
        self.scripts = crate::nis::Scripts::new(lib, ps);
        // `FuseBox_Create` 0x1b0448 ends with `SP_Create(pos, rot, param 0, 0, 0x40, 0, obj,
        // FuseBox_Register 0x1b0970)` — the box's wire/lid ScriptPlayer, mode 0x40 (the preload:
        // play+update+stop → state 1), whose entity streams are the box's VISIBLE MESH (the object
        // itself has obj+0xD8 = 0 and never draws [M 001b0448:33-35, docs/models/misc.md]). The port's
        // fusebox stand-in had no SP, so 07000008's GuardRoom fusebox — the only in-play opener of
        // the doorway bolt ch 137 — was invisible (playtest item 7's "pillar"). The synthetic player
        // primes on the first tick; its entities render and join set_dynamic. SP_Create's mask word
        // is param_6 = 0 [M 001dc7e0]; the port's prime() skips mask-0 SPs (the port-check §9
        // disabled-byte stand-in), so the player carries mask 1 — the load beat is what matters.
        for fb in &self.fuseboxes {
            let p = &level.maps[m].placements[fb.placement];
            let script = p.params.iter().find(|(k, _)| *k == 0).map_or(0, |&(_, v)| v);
            if script < 0x0600_0000 { continue; }
            let m = nf_formats::placement::object_matrix(p.rotation, p.translation);
            self.scripts.players.push(crate::nis::Player {
                placement: fb.placement,
                scripts: [script, 0],
                modes: [0x40, 0],
                stay: false,
                mask: 1,
                out: 0,
                gate: 0,
                scale: 1.0,
                matrix: m,
                cur: 0,
                state: 0,
                activated: false,
                countdown: 0,
                info: [None, None],
                deleted: false,
                live_matrix: m,
                subs: Vec::new(),
            });
        }
        if let Some(p) = ps.iter().find(|p| p.flags & 0xff == 0xdf) {
            let kind = p.params.iter().find(|(i, _)| *i == 0).map_or(0, |&(_, v)| v as u8);
            let boxes = level.instances(false).0.iter().filter(|i| i.placement.flags & 0xff == 0x32).map(|i| {
                let o = &level.maps[i.map].objects[i.object];
                let t = i.placement.translation;
                crate::env::RainBox { min: [o.min[0] + t[0], o.min[1] + t[1], o.min[2] + t[2]], max: [o.max[0] + t[0], o.max[1] + t[1], o.max[2] + t[2]] }
            }).collect();
            self.env_pending = Some((kind, boxes));
        }
        // The presentation FX (0x21/0xe3/0xd4/0x4a/0xf2 placements, fix-world-fx): the same
        // placement-order walk `parsemap_create_dynamic_objects` 0x1d04f0 runs.
        self.fx = crate::world_fx::WorldFx::attach_level(level);
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
                // A template has no object until `DroneSpawner_SpawnDrone` 0x137cb0 runs `NDrone2_CreateObj`
                // (drone.md "present / started / enabled / in-llist"): no object → no `obj+0xFC`, so the port's
                // `drawn` stand-in is false while held (and stays false until the spawn tick refreshes it).
                for &k in &templates { self.list[k].present = false; self.list[k].enabled = false; self.list[k].drawn = false; }
                self.llist.retain(|i| !templates.contains(i));
                self.spawners.push(sp);
            }
        }
    }

    /// `Sound_Play3D(id, pos)`: enters a sound for the drones to hear next frame (hearing data from `SFXOutputData`).
    /// Bond fires `weapon` from `from` along `dir` (`Player_WeaponInitBullet`: head → aim point): one bullet per pellet
    /// (`+0x18`) with the record's damage `+0x0C`, range `+0x1C`, speed `+0x20` and spread `+0x24`; and the shot's
    /// sound (the first-person fire script's, 0x403 for the P2K) for the drones' hearing.
    /// The closest approach of the segment `a`–`b` to the sphere (centre `c`, radius `r`): the classic
/// closest-point-on-segment distance test.
fn segment_hits_sphere(a: V3, b: V3, c: V3, r: f32) -> bool {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if len2 <= 0.0 { 0.0 } else { (ac[0] * ab[0] + ac[1] * ab[1] + ac[2] * ab[2]) / len2 }.clamp(0.0, 1.0);
    let d = [ac[0] - ab[0] * t, ac[1] - ab[1] * t, ac[2] - ab[2] * t];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= r * r
}

    /// The world position of a script object this tick (an entity's matrix, or the anim's
    /// discrete object 0) — the resolution the game's collision contacts carry.
    /// A script object's world position: an Entity's matrix translation, an Anim's discrete-0 root
    /// (`Script_Interp` 5's follow point). `d` for the Anim's discrete lookup.
    pub fn script_obj_pos(scripts: &crate::nis::Scripts, d: &Data, o: usize) -> Option<V3> {
        match scripts.objects.list.get(o) {
            Some(Some(crate::nis::Obj::Entity { matrix, .. })) => Some([matrix[3][0], matrix[3][1], matrix[3][2]]),
            Some(Some(crate::nis::Obj::Anim { skin, script, frame, .. })) => crate::nis::World::discrete0(d, *skin, *script, *frame).map(|m| [m[3][0], m[3][1], m[3][2]]),
            _ => None,
        }
    }

    pub fn fire_player(&mut self, d: &Data, from: V3, dir: V3, weapon: u8, sound: nf_formats::sfx::SfxHearing) {
        let Some(w) = d.weapons.records.get(weapon as usize) else { return };
        let f = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let flags68 = u32::from_le_bytes(w[0x68..0x6c].try_into().unwrap());
        for _ in 0..u32::from_le_bytes(w[0x18..0x1c].try_into().unwrap()).clamp(1, 16) {
            // `PlrStat_LogShotFired(0)` 0x1998d8 per bullet spawned (Player_WeaponInitBullet
            // 0x1a4bf0:258 runs inside the per-pellet loop) — fix-progress.
            self.progress.log_shot_fired(self.mission.status());
            let dir = crate::bullet::spread(dir, f(0x24), flags68, &mut self.rng);
            self.player_bullets.push(PlayerBullet {
                pos: from, dir, speed: f(0x20), travelled: 0.0, range: f(0x1c), damage: f(0x0c), weapon: weapon as u16, first: true, dead: false, bounces: 0,
                homing: u32::from_le_bytes(w[0x6c..0x70].try_into().unwrap()) & 0x2_0000 != 0,
                target: None,
            });
        }
        // `Bullet_init_casing` 0x12fde0 (Player_WeaponInitBullet 0x1a4bf0:306, right after the
        // bullet's spread draws): one shell per shot, ejected from the view model after the weapon's
        // +0x78-tick delay. [S] the eject origin is the muzzle `from` and the bone rotation stands in
        // for the view-model matrix (the game reads `AnimGetBoneWorldTrans(obj, 1|5, 0)` — the main
        // agent's render check owns the visual).
        let yaw = dir[0].atan2(dir[2]);
        let pitch = dir[1].clamp(-1.0, 1.0).asin();
        self.casings.eject(&d.surfaces.map(|s| &s.ammo).unwrap_or(&Default::default()), w, from, yaw, pitch, from[1], &mut self.rng);
        // Heard by the drones; the audible shot comes from the gun's fire script (op 1), played by the player's owner.
        if !sound.looped { self.sounds.push(Sound { id: sound.id, pos: from, inner: sound.inner, outer: sound.outer, weight: sound.weight }); }
    }

    /// Bond's bullets for one tick (`Bullet_update`, `Collide_Update`): the segment of max(step, 1) units against the
    /// world and every live drone's boxes (`Collide_PickObj` sphere r 1.05 around the hips, then `Collide_Jointy`); the
    /// nearest wins. A drone hit sends msg 8 (`Drone_BulletHit`).
    fn update_player_bullets(&mut self, d: &Data) {
        self.player_impacts.clear();
        self.bullet_paths.clear();
        self.bullet_path_weapons.clear();
        // `Collide_Update` 0x1ea398's first loop clears every hit list at the head of the frame —
        // the fusebox wires' hit flags are per-tick only.
        for fb in &mut self.fuseboxes { fb.clear_hits(); }
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
            self.bullet_paths.push((b.pos, end));
            // The photograph targets and breakable doors see the bullet this tick (the game's
            // collision pass registers the contact; the prop's update walks it — crate::props).
            self.props.bullet_hit(b.pos, end, b.weapon, b.damage, &mut self.channels, frame);
            // The fusebox wires see the bullet this tick (`Collide_Update` 0x1ea398 pushes the pick
            // onto the hit object's `obj+0xD0`; `FuseBox_Update` 0x1adfe8:88 filters the list with
            // `Collide_FilterBullets(list, 0xfffd)` — class-5 hitters whose category word
            // `weapon_definition+0x10` ANDs non-zero are dropped [M 0x1ece30:24-31]). Disc words
            // [L dump]: guns 0x0008 (dropped — gunfire never cuts a wire), the Laser 0x0002 (kept —
            // it cuts), the watch/Decryptor beams 0x0100 (dropped).
            let kind = d.weapons.records.get(b.weapon as usize).map_or(0u16, |r| u16::from_le_bytes([r[0x10], r[0x11]]));
            if crate::fusebox::FuseBox::hit_survives_filter(kind) {
                let mut contacts: Vec<(usize, usize, usize)> = Vec::new();
                for (fi, fb) in self.fuseboxes.iter().enumerate() {
                    for (g, k, obj) in fb.alive_wires() {
                        if let Some(p) = Self::script_obj_pos(&self.scripts, d, obj) {
                            if Self::segment_hits_sphere(b.pos, end, p, crate::fusebox::WIRE_RADIUS) {
                                contacts.push((fi, g, k));
                            }
                        }
                    }
                }
                for (fi, g, k) in contacts { self.fuseboxes[fi].mark_hit(g, k); }
            }
            // The lazer watch's beam burns the locks (`Lock_Update` 0x1ae3d8 case 0, record word 1):
            // the cell walk takes class-5 contacts whose BU+0x44 reads 0x4E/0x4F (the watch, −3/−10
            // per tick [:57-103]); `Lock_Unlock` writes `switch_channels[record+8]` [:194-195]. The
            // port's watch bullets carry the weapon id, so the segment-vs-sphere contact stands in
            // for the cell walk; the create-time burn base is 0 on this level's data [M 0x1ae1f8 —
            // 07000008's #207/#208 param 2 = 0], so the first contact completes it (playtest item 7).
            if matches!(b.weapon, 0x4e | 0x4f) {
                let hits: Vec<usize> = self.locks.iter().enumerate()
                    .filter(|(_, lk)| !lk.burned && Self::segment_hits_sphere(b.pos, end, lk.pos, lk.radius.max(0.5)))
                    .map(|(k, _)| k).collect();
                for k in hits { self.locks[k].burn(&mut self.channels, frame); }
            }
            self.bullet_path_weapons.push(b.weapon);
            let tw = d.world.segment(b.pos, end);
            let mut best: Option<(usize, i16, f32)> = None;
            for (i, dr) in self.list.iter().enumerate() {
                // `Drone_CollisionHandler` 0x140968:62 returns while `NPCGlobals == 0` (the NIS freeze:
                // drones take no damage) and `Collide_PickObj` 0x1eae80:26-30 drops `obj+0xF0 & 0x20`
                // objects (set by `NDrone2_Enable(0)`) — bullets pass through the frozen drones.
                if !self.npc_enabled || !dr.present || !dr.enabled || dr.corpse { continue; }
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
                    // `NDrone2_DoHitEffects` 0x1458e0:102-109: damage > 0.5 and the weapon's `+0x70` flags
                    // without 0x180 → the hit sound at the hit point: 0x601 when HITDATA+0x52 (the part, the
                    // port's `part` bone id) is 5 (head), else 0x413.
                    if hit.damage > 0.5 && d.weapons.hit_flags(b.weapon as usize) & 0x180 == 0 {
                        self.audio.play3d(if part == 5 { 0x601 } else { 0x413 }, hit.point, None);
                    }
                    let rng = &mut self.rng;
                    let dr = &mut self.list[i];
                    dr.hit_by_player = true; // +0x2B4 = the player [M 0x145bd0; fix-actors]
                    dr.hit = Some(hit);
                    dr.global_flinch = self.last_flinch;
                    dr.bond_aiming = self.bond_aiming;
                    // `NDrone2_DealWithObjHit` case 5 (0x13fff0:269-306): the SHOOTER is the player, so the
                    // weapon decides the message — 1 (fists), 0x4B, 0x4D → msg 6 (punch); 0x4A / 0x4C →
                    // msg 7 (taser, not ported — the Q-Claw's grapple state); anything else msg 8
                    // (`Drone_BulletHit`). Both paths bump +0x1D8 and stamp +0x2B4 (last hit by) first.
                    let m = if matches!(b.weapon, 1 | 0x4b | 0x4d) { msg::PUNCH } else { msg::BULLET };
                    dr.send(d, rng, frame, m);
                    self.last_flinch = dr.global_flinch;
                }
                (_, Some(w)) => {
                    b.dead = true;
                    let hit = at(w);
                    self.player_impacts.push(PlayerImpact::World(hit));
                    // `PlrStat_LogShotHitScenery(0)` 0x198408 (SP_Update 0x1dbda8:261; Destroy_Smash
                    // 0x12bb60:55 — the port folds the breakables into this world-hit path): the
                    // accuracy denominator nets to the enemy hits only (fix-progress).
                    self.progress.log_shot_hit_scenery(self.mission.status());
                    self.impact_sound(d, b.pos, end);
                    // `Bullet_CollisionHandler` 0x12ce80 → `Effect_Create` 0x1ce688 / `Effect_Bullet`
                    // 0x1ce8a8 / `Effect_RicochetProb` 0x1cf290 (docs/atlas/effect.md). Bond is class 3,
                    // so `Effect_Bullet` skips its `Rand_Rand(2)` [M 0x1ce94c]. A ricochet keeps the
                    // bullet alive on the reflected direction [M 0x12d704].
                    let flags70 = d.weapons.records.get(b.weapon as usize)
                        .map_or(0u32, |r| u32::from_le_bytes(r[0x70..0x74].try_into().unwrap()));
                    if let Some((_, n, m)) = d.world.segment_surface(b.pos, end, false) {
                        if let Some(dir) = self.effects.bullet_impact(&mut self.rng, d.surfaces, &mut self.gas, hit, n, m, b.dir, flags70, b.bounces, true) {
                            if b.bounces < 3 {
                                b.dead = false;
                                b.pos = hit;
                                b.dir = dir;
                                b.bounces += 1;
                                b.first = false;
                            }
                        }
                    }
                }
                _ => b.pos = [b.pos[0] + b.dir[0] * step, b.pos[1] + b.dir[1] * step, b.pos[2] + b.dir[2] * step],
            }
            // `Bullet_homing` 0x12f358 (from `Bullet_update` 0x12e7b0:483, after the move, for weapons
            // whose `+0x6C` carries 0x20000): keep/refresh the target, else scan for the nearest
            // heat-seekable object within 1000 units inside the 0.66-dot cone of the bullet's
            // direction with a clear line of sight (`Collide_LineOfSight(…, 8)` → the port's static
            // world segment), then steer: `dir += clamp((to_target − dir)·0.0333, ±0.01)` per
            // component, normalised, and re-align the facing (the port's dir is the facing).
            // Heat-seekable [M `Bullet_heatseekable` 0x12f248]: the guards (class 2) while alive —
            // `+0xFC` set, health `+0xAC` > 0, anim state ≠ 1 — plus the copter bodies (class 0) and
            // Bond (class 3); the port's targets are its live drones (the gate below). No RNG draws
            // (determinism).
            if b.homing {
                // `Bullet_heatseekable` 0x12f248 [M]: a guard (class 2) is seekable while its object
                // exists (`+0xfc != 0` — the port's `drawn`, the per-tick cull stand-in), its health
                // (`BL+0xac`) is > 0 and its view state (`+0xf4`) is not 1 (`WAIT_SWITCH` — a
                // spawner-held template waiting for its channel is not a live target). The game does
                // NOT read the +0xF0 enable byte here (the port used `enabled`) and a corpse is
                // excluded by health 0, not by its own flag.
                let seekable = |dr: &Drone| dr.drawn && dr.health > 0.0 && dr.state != state::WAIT_SWITCH;
                let mut target = b.target;
                if let Some(ti) = target {
                    if !seekable(&self.list[ti]) { target = None; }
                }
                if target.is_none() {
                    let mut best: Option<(usize, f32)> = None;
                    for (i, dr) in self.list.iter().enumerate() {
                        if !seekable(dr) { continue; }
                        let to = [dr.pos[0] - b.pos[0], dr.pos[1] - b.pos[1], dr.pos[2] - b.pos[2]];
                        let dist = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
                        if dist <= 0.0 || dist > 1000.0 { continue; }
                        if best.is_some_and(|(_, bd)| dist >= bd) { continue; }
                        let n = [to[0] / dist, to[1] / dist, to[2] / dist];
                        let dot = n[0] * b.dir[0] + n[1] * b.dir[1] + n[2] * b.dir[2];
                        if dot <= 0.66 { continue; }
                        if d.world.segment(b.pos, dr.pos).is_some() { continue; }
                        best = Some((i, dist));
                    }
                    target = best.map(|(i, _)| i);
                }
                b.target = target;
                if let Some(ti) = target {
                    let tp = self.list[ti].pos;
                    let to = [tp[0] - b.pos[0], tp[1] - b.pos[1], tp[2] - b.pos[2]];
                    let l = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
                    if l > 0.0 {
                        let steer = |c: f32, dirc: f32| ((c - dirc) * 0.0333).clamp(-0.01, 0.01);
                        let dx = b.dir[0] + steer(to[0] / l, b.dir[0]);
                        let dy = b.dir[1] + steer(to[1] / l, b.dir[1]);
                        let dz = b.dir[2] + steer(to[2] / l, b.dir[2]);
                        let m = (dx * dx + dy * dy + dz * dz).sqrt();
                        if m > 0.0 { b.dir = [dx / m, dy / m, dz / m]; }
                    }
                }
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
        let diff = match self.difficulty { 1 => t.Autoaim_EasyMul, 3 | 4 => t.Autoaim_HardMul, _ => t.Autoaim_NormalMul };
        let mut best = t.Autoaim_Range;
        let mut found = None;
        for (i, dr) in self.list.iter().enumerate() {
            // `Check_AutoAim` (0x1a73f8): skip drawn==0 (`obj+0xFC`), dead (`+0xAC <= 0`), voice set 0xB (`+0xD8`),
            // the Disabled state (`obj+0xF4 == 1`, covered by `enabled`), and — single player — **every non-enemy
            // team** (`+0x44 != 1` → skip; the MP build checks `MP_areObjectsOnSameTeam` instead, not modelled).
            // This keeps Bond's crosshair off allies, hostages and civilians (drone.md `+0x44`: 1 = enemies,
            // 10/11/13/14 → 3 friends, 12 → 2 bots) — the 07000002 "shootable civilians" report.
            if !dr.present || !dr.enabled || !dr.drawn || dr.corpse || dr.health <= 0.0 || dr.kind == 0xb || dr.team != 1 { continue; }
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

    /// `NDrone2_SpeechSFX(idx, force, 0, 0)` → `NDrone2_PlaySFX(2.0, …)` for each request of a drone's update: closed
    /// for the first 30 frames (`DronePostLoadFrame`), the line's repeat delay (global per line), no direct play for a
    /// line with a facial anim, a non-forced line refused while one is referenced; the line is stamped, played at the
    /// drone (`Sound_Play3D`: heard and entered for the AI) and becomes the drone's current line (`+0x4D0`).
    fn speech(&mut self, d: &Data, index: usize, talks: Vec<(usize, bool)>, frame: u32) {
        let Some(lines) = d.voices else { return };
        if self.voice_last.len() < lines.len() { self.voice_last.resize(lines.len(), None); }
        for (idx, force) in talks {
            let Some(l) = lines.get(idx).copied() else { continue };
            if frame < 30 || self.voice_last[idx].is_some_and(|t| frame < t + l.repeat) { continue; }
            if l.sfx == 0 || l.facial != 0 { continue; }
            if !force && self.list[index].voice.is_some() { continue; }
            self.voice_last[idx] = Some(frame);
            let pos = self.list[index].pos;
            let h = match d.sfx { Some(t) => self.play_sound_handle(pos, t.get(l.sfx)), None => self.audio.play3d(l.sfx, pos, None) };
            self.list[index].voice = (h != 0).then_some((h, l.sfx));
        }
    }

    /// `NDrone2_PlaySFX` (0x141848) for the foley lines (scale 1.0, delay enforced — the door line 0x27, the
    /// kick 0x28): the 30-frame post-load gate, the entry's `repeatDelay` (`lastFrame`), then the entry's sfx id
    /// at the drone. `lastFrame` is stamped **before** the play (rate-limited even when the play fails) [M
    /// 0x141848: disasm 0x141a1c]. The handle is not referenced (speech's +0x4D0 stays free) and +0x4D4's
    /// 180-frame talk suppression is not modelled — it only gates optional talk lines [M 0x13f1d0].
    fn foley(&mut self, d: &Data, index: usize, idx: usize, frame: u32) {
        if let Some(l) = Self::foley_entry(d.voices, &mut self.voice_last, idx, frame) {
            let pos = self.list[index].pos;
            match d.sfx { Some(t) => { self.play_sound_handle(pos, t.get(l.sfx)); } None => { self.audio.play3d(l.sfx, pos, None); } }
        }
    }

    /// The `NDrone2_PlaySFX` gate + `Drone_SfxInfo` stamp (`lastFrame` in the table, shared by every drone) —
    /// split out so the door site inside the `list.iter_mut()` loop can play through the narrow field borrows.
    fn foley_entry(lines: Option<&[nf_formats::sfx::VoiceLine]>, voice_last: &mut Vec<Option<u32>>, idx: usize, frame: u32) -> Option<nf_formats::sfx::VoiceLine> {
        let lines = lines?;
        if voice_last.len() < lines.len() { voice_last.resize(lines.len(), None); }
        let l = lines.get(idx).copied()?;
        if frame < 30 || l.sfx == 0 || voice_last[idx].is_some_and(|t| frame < t + l.repeat) { return None; }
        voice_last[idx] = Some(frame);
        Some(l)
    }

    /// `BOT_handlePain` (0x125ed8, fix-room-sound): the bots' grunt pools — the pain pool is
    /// male `0x14d..0x151` (`Rand_Rand(5)`) / female `0x152..0x153` (`Rand_Rand(2)`), the death
    /// pool male `0x154..0x156` / female `0x157..0x159` (`Rand_Rand(3)`, no playing gate on a
    /// death) [M disasm 0x1260c8-0x12617c; the decomp's `uVar8 == 0x1e` verdict branch is dead].
    /// Exactly one `Rand_Rand(n)` draw per call that reaches a pool, before the play [M 0x126168];
    /// the previous grunt is stopped and the new handle stored (`Sound_Stop_Ref` 0x1de370, the
    /// handle at `comm+0x73c`) [M 0x126178-0x1261a4]. All ten pool entries are `Drone_SfxInfo`
    /// lines (950/948/943/596/138 · 944/465 · 949/947/139 · 945/581/466), so the play runs the
    /// shared `NDrone2_PlaySFX` gates via `foley_entry`.
    ///
    /// NOT wired into the hit path: the game reaches `BOT_handlePain` only under `MPSettings+0x388`
    /// (the bots flag [M 0x145558:15-17]) and the port is single-player — `NDrone2_HitDamage`'s SP
    /// arm is `NDrone2_PainTalk` (`pain_talk`), which is what the port runs, so no `Rand_*` draws
    /// were added to the live stream and no seed-pinned test moves. *Deviation*: the "previous
    /// grunt still playing" gate (`Sound_Playing_Ref` 0x1de3a0 on the stored handle) needs the
    /// sound pool's node state, which nf-game does not expose — the port stops the previous handle
    /// and always plays.
    pub fn bot_handle_pain(&mut self, d: &Data, index: usize, died: bool, frame: u32) {
        let female = self.list[index].female;
        let (base, n) = Self::pain_pool(female, died);
        let id = base + self.rng.rand(n) as usize;
        if let Some(l) = Self::foley_entry(d.voices, &mut self.voice_last, id, frame) {
            let pos = self.list[index].pos;
            if let Some(prev) = self.list[index].pain_grunt.take() { self.audio.stop(prev); }
            let h = match d.sfx {
                Some(t) => self.play_sound_handle(pos, t.get(l.sfx)),
                None => self.audio.play3d(l.sfx, pos, None),
            };
            self.list[index].pain_grunt = (h != 0).then_some(h);
        }
    }

    /// `BOT_handlePain`'s pool pick: the verdict (1 = died, 0x1f = alive) selects the death pool
    /// (male 0x154 / female 0x157, `Rand_Rand(3)`, no playing gate) over the pain pool (male
    /// 0x14d `Rand_Rand(5)` / female 0x152 `Rand_Rand(2)`); the female byte is `Drone+0x15`
    /// [M disasm 0x1260c8-0x126168].
    fn pain_pool(female: bool, died: bool) -> (usize, u32) {
        if died { (if female { 0x157 } else { 0x154 }, 3) } else if female { (0x152, 2) } else { (0x14d, 5) }
    }

    /// `Effect_Bullet`: the impact sound of the surface the segment `from → to` hits (`EffectInfo+0x14`), at the hit.
    fn impact_sound(&mut self, d: &Data, from: V3, to: V3) {
        let (Some(s), Some((t, _, m))) = (d.surfaces, d.world.segment_surface(from, to, false)) else { return };
        let p = [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t, from[2] + (to[2] - from[2]) * t];
        if let Some(id) = nf_formats::sfx::SurfaceSounds::get(&s.impact, m) { self.audio.play3d(id, p, None); }
    }

    /// `Sound_Play3D(id, pos, …)`: heard (the sound pool) and, unless looped, entered in the drones' hearing list.
    pub fn play_sound(&mut self, pos: V3, h: nf_formats::sfx::SfxHearing) { self.play_sound_handle(pos, h); }

    /// `play_sound`, returning the sound's handle.
    pub fn play_sound_handle(&mut self, pos: V3, h: nf_formats::sfx::SfxHearing) -> crate::sound::Handle {
        let handle = self.audio.play3d(h.id, pos, None);
        if h.looped { return handle; }
        self.sounds.push(Sound { id: h.id, pos, inner: h.inner, outer: h.outer, weight: h.weight });
        handle
    }

    /// `Player_Activate` (0x1a5170): Bond's use sphere (radius 1.0 at head + 1 unit forward) activates every
    /// cel-linked object it meets. The door part calls `Door_Activate` (0x1a5310); the trigger part (class 0x1D)
    /// calls `Trigger_Activate` (0x1b2318) — routes b and c on 07000005 (the on-foot gate door and the zip-wire
    /// exit). The port's `player_use_doors` handled only doors; this adds the trigger path so pressing E on a
    /// LOAD_LEVEL or TouchOnce trigger fires it immediately [M 0x1a5170:100-108,128, 0x1b2318:34-111].
    pub fn player_use_doors(&mut self, d: &Data, head: V3, look: V3, frame: u32) {
        let (evs, _hits) = self.doors.player_use(head, look, &mut self.channels, frame);
        for (id, pos) in evs {
            match d.sfx {
                Some(t) => { let h = t.get(id as u32); self.play_sound_handle(pos, h); }
                None => { self.audio.play3d(id as u32, pos, None); }
            }
        }
        // `Player_Activate`'s class-0x1D trigger path: for every trigger within the use sphere, call
        // `Trigger_Activate` — for LOAD_LEVEL, request the level load; for TouchOnce, set the output channel.
        self.player_use_triggers(head, look, frame);
        // `Player_Activate`'s remaining classes the Tower levels need (map-tower): the mask-4
        // ScriptPlayers (`SP_Activate` 0x1dc338 — the elevator, the security keyboard), the
        // switches (0x29, `Switch_Activate` 0x1b02a8) and the Q-Worm PCs (0x35,
        // `PCQWorm_Activate` 0x1b0840) — crate::useobj.
        self.useobjs.player_activate(&mut self.scripts, &mut self.channels, head, look, frame);
    }

    /// The class-0x1D trigger half of `Player_Activate` (0x1a5170:134-137). Bond's use sphere (radius 1.0 at
    /// head + look) activates every trigger whose volume it meets. For LOAD_LEVEL: `Trigger_Activate` checks
    /// cancel channels (0x60–0x63), the key gate, sets the success/fallback channels, and requests the level
    /// load. For TouchOnce: sets the output channel. [M 0x1a5170:100-108, 0x1b2318:34-111].
    fn player_use_triggers(&mut self, head: V3, look: V3, frame: u32) {
        let use_point = [head[0] + look[0], head[1] + look[1], head[2] + look[2]];
        let r = 1.0;
        // Collect trigger indices whose bounding box meets the use sphere (stand-in for the game's
        // `Collide_SphereIntersect(1.0, ...)` against the trigger's collision mesh).
        let hits: Vec<usize> = (0..self.triggers.len()).filter(|&i| {
            let t = &self.triggers[i];
            !(0..3).any(|k| use_point[k] + r < t.lo[k] || use_point[k] - r > t.hi[k])
        }).collect();
        for i in hits {
            let (sets, next) = self.triggers[i].activate_request(&self.channels);
            for (c, v) in &sets { self.channels.set(*c as u32, *v, frame); }
            if let Some(next) = next {
                if self.level_exit.is_none() {
                    self.level_exit = Some(next);
                    self.music.event(3, 1);
                    self.triggers[i].gone = true;
                }
            } else if !sets.is_empty() {
                // TouchOnce: latch gone so it doesn't fire again.
                if self.triggers[i].kind == crate::triggers::kind::TOUCH_ONCE { self.triggers[i].gone = true; }
            }
        }
    }

    /// The script requests' consumer (sounds, the camera's music events, the drone freeze/free,
    /// the player enable/disable, the script-driven level transition). Run once before the walk
    /// (the priming beat's and the copter's requests) and after EVERY script player's
    /// `SP_Update` in the walk — event 5's `Drone_EnableAll(0)` must gate the objects after it in
    /// the same frame (`DroneSpawner_Control` returns while `NPCGlobals == 0` [M 0x137e68:21-23];
    /// in the game the call is synchronous inside `Script_EventHandler`).
    fn drain_script_requests(&mut self, d: &Data) {
        for r in std::mem::take(&mut self.scripts.requests) {
            match r {
                crate::nis::Request::PlayerDisable | crate::nis::Request::PlayerEnable { .. } => self.player_events.push(r.clone()),
                // Events 5 / 7: the game's own drone gate (`Drone_EnableAll` → `NPCGlobals`) [M 0x1383e8].
                // `Drone_EnableAll(0)` walks the whole llist calling `NDrone2_Enable(0)` per drone —
                // `obj+0xF0 |= 0x10|0x20` (not drawn, not picked) and `+0x4F8 &= ~0x1801A0` (no anim/AI)
                // [M 0x149dd0:8-15] — and sets `NPCGlobals = 0`. The re-enable (`Script_Free`'s 0x80 flag,
                // 0x1da880:41-43) skips state-1 WaitSwitch drones [M 0x1383e8:14] — they stay disabled until
                // their own switch channel wakes them [M 0x15b960:21-37]. The port's `enabled` is the
                // obj+0xF0 gate's stand-in, so the freeze mirrors per drone.
                crate::nis::Request::DroneEnable { on } => {
                    self.npc_enabled = on;
                    for dr in self.list.iter_mut().filter(|d| d.present) {
                        if !on {
                            // `NDrone2_Enable(0)` 0x149dd0:8-15: hide everyone (the anim clock write
                            // is skipped — the port's renderer poses from `anim.time` live, and the
                            // game's frozen pose is its last rendered matrix).
                            dr.enabled = false;
                        } else if dr.state != state::WAIT_SWITCH && !dr.corpse {
                            // `Drone_EnableAll(1)` 0x1383e8:14 skips WaitSwitch drones; `NDrone2_Enable(1)`
                            // 0x149dd0:16-28 refuses corpses (+0x4F8 & 0x200) and restarts the anim clock
                            // at script time 1 (+0x98 = 0x3f800000) — the re-shown characters replay from
                            // their script's first frame, not mid-stride (fix-party-mission item 6).
                            dr.enabled = true;
                            dr.anim.time = 0;
                            dr.anim.ftime = 0.0;
                        }
                    }
                }
                crate::nis::Request::Camera { script, start } => {
                    // Script_CameraStart 0x1d9140:46-49 disables the viewers' HUD; Script_KillStream
                    // 0x1d9860:71-72 restores it. The flag freezes the HUD message timers [M 0x1b11d8].
                    self.hud_frozen = start;
                    // The letterbox: `Script_CameraStart` creates the two border sprites (0x2c9468, the
                    // second at +0x26 = 0x180) at draw layer 1 and `Script_KillStream` case 4 sets
                    // their layer to 0xff — the bars show exactly while a script camera is live
                    // [M 0x1d9140:49-61, 0x1d9860:87-93]. nf-view draws them from this flag.
                    self.letterbox = start;
                    self.music.event(if start { 0xb } else { 0xc }, script)
                }
                // Op 0x13: `Camera_SetFade(-f, 4, 0xff, 0)` [M 0x1db3d0, 0x1bbdd8] — a black overlay
                // (colour 0x000000ff) whose alpha ramps 1/|f| per tick: f < 0 fades in from black,
                // f > 0 fades out to black. The game refuses a second fade while the same viewer's
                // pair (+0x234, +0x235) is active [M 0x1bbdd8:11]; the port's one-state stand-in
                // re-arms only when nothing is running [S: the viewer state's exact lifecycle is not
                // decoded]. The same state carries the script's auto-fades (Script_Update's
                // length−20 fade-out and the −22 handback fade-in) and nf-view draws it.
                crate::nis::Request::Fade { ticks } => {
                    // `Camera_SetFade(duration, 4, 0xff, 0)` [M 0x1bbdd8]: rate = 1/duration per tick;
                    // a NEGATIVE duration starts at alpha 1.0 and fades in, a positive one starts at
                    // 0.0 and fades out (op 0x13 passes the script f32 negated, so a script fade of
                    // −82 arrives as +82 = out to black [M 0x1db3d0, 0x06000071's bytes]).
                    if self.view_fade.is_none() {
                        let (alpha, rate) = if ticks < 0.0 { (1.0, 1.0 / ticks) } else if ticks > 0.0 { (0.0, 1.0 / ticks) } else { (0.0, 0.0) };
                        self.view_fade = Some(ViewFade { alpha, rate });
                    }
                }
                // Op 0x1e: `Text_AddMsg(-1, 0, 4, Txt_BindLabel(label, 0), 0, time)` [M 0x1db2b8] —
                // the raw label rides to nf-view (the Lang table lives above nf-game), the
                // `drones.taken` pattern; the HUD pane shows it for `time` frames.
                crate::nis::Request::Text { label, time } => self.script_msgs.push((label, time)),
                // `Script_SoundStart`: 2-D, or 3-D at the stream's object (its position each frame, `Script_Interp` 5).
                crate::nis::Request::Sound { id, at, src } => {
                    // `SFXStart` 0x115a68:18-20 / `SFXStart3D` 0x115bf0: a sound with id 0x172 (the NIS
                    // music stream — the cut-scene's own score) sets `SfxNisMusicTrigger`, which the level's
                    // music updater jumps on [M]. fix-party-mission item 4: the cut-scene music change.
                    if id == 0x172 { self.music.nis_trigger = true; }
                    let h = match at.and_then(|o| Self::script_obj_pos(&self.scripts, d, o)) {
                        Some(pos) => {
                            let h = self.audio.play3d(id, pos, None);
                            if let (Some(t), false) = (d.sfx, d.sfx.is_some_and(|t| t.get(id).looped)) { let e = t.get(id); self.sounds.push(Sound { id, pos, inner: e.inner, outer: e.outer, weight: e.weight }); }
                            h
                        }
                        None => self.audio.play(id),
                    };
                    self.script_sounds.retain(|s| s.0 != src);
                    self.script_sounds.push((src, h, at));
                }
                crate::nis::Request::SoundStop { src } => {
                    for s in self.script_sounds.iter().filter(|s| s.0 == src) { self.audio.stop(s.1); }
                    self.script_sounds.retain(|s| s.0 != src);
                }
                // An animation script's op-1 sound (`AnimProcessScriptCmds` 0x1c1b38 case 1):
                // `Sound_Play3D(sfx, obj+0x30, vol 100, outer 100, inner 0, pitch Rand(500) − 250, ref 0,
                // custom 0)` [M; the same call the facial path makes, docs/systems/speech.md §5]. The script
                // anim object has no weapon parent, so no id remap; the pitch draw is consumed here (the
                // shared RNG — one `Rand(500)` per fired op-1) but not applied: the EE sound layer stores the
                // pitch and never forwards it [M crates/nf-game/src/sound.rs header]. The handle is
                // discarded by the game (the sound cannot be stopped or moved).
                crate::nis::Request::AnimSound { id, at } => {
                    let _pitch = self.rng.rand(500) as i32 - 250;
                    match at.and_then(|o| Self::script_obj_pos(&self.scripts, d, o)) {
                        Some(pos) => {
                            self.audio.play3d(id, pos, None);
                            if let Some(t) = d.sfx { let e = t.get(id); if !e.looped { self.sounds.push(Sound { id: e.id, pos, inner: e.inner, outer: e.outer, weight: e.weight }); } }
                        }
                        None => { self.audio.play(id); }
                    }
                }
                // Event 0x12: the script-driven level transition. No NIS script on 07000005 fires kind 0x12 (the truck
                // script 0x06000A22 fires only 0xC and 0xD) — the actual level exit on 07000005 is the placement-0xE8
                // trigger at the castle door (see the LOAD_LEVEL arm below). This arm is the script-event path for
                // levels that do fire 0x12. Set `level_exit` so the main loop reloads the next level
                // [M Script_EventHandler 0x1d94c0:189-199].
                crate::nis::Request::LoadLevel { level } => {
                    if self.level_exit.is_none() {
                        self.level_exit = Some(level);
                        self.music.event(3, 1);
                    }
                }
                // (fix-fusebox-cut, merged after fix-script-start-tick moved this loop here.)
                // Event 0xF: the SP create-callback hand-off (`Script_EventHandler` 0x1d94c0:172-175)
                // — the current stream's entity registers with the box's `FuseBox_Register` 0x1b0970,
                // group = the event's param 0 [L NF_SCRIPTLOG: 07000008's 0x060008AD stream 2 → 0;
                // 07000014's 0x06000A1B streams 2/6 → 0/1].
                crate::nis::Request::WireRegister { player, obj, group } => {
                    if let Some(fb) = self.fuseboxes.iter_mut().find(|f| f.placement == player) {
                        fb.register_wire(obj, group);
                    }
                }
                _ => {}
            }
        }
    }

    /// `Pickup_Update` for every pickup: the fall, Bond's touch (`Pickup_Handler`; refused pickups stay, and the
    /// lifetime does not run that tick), the lifetime. Taken: sound 0xF5 at the pickup, deleted (no respawn).
    /// `hashtable_getitem` of the first-person model stands in as "the level has that skin".
    fn update_pickups(&mut self, d: &Data, player: &Player, frame: u32) {
        let alive = self.health.health > 0.0;
        for k in 0..self.pickups.len() {
            let p = &mut self.pickups[k];
            if p.gone { continue; }
            // State 2 (taken, waiting to respawn) [M 0x18e148:182-189]: nothing else that tick.
            if p.state == 2 {
                p.respawn_wait();
                continue;
            }
            // `+0x38` spin [M 0x18e148:193-201] — every tick, before the touch.
            p.spin_tick();
            p.fall(d.world);
            // The distance test runs from the player's position (+0x30) but the LOS from his world-sphere
            // centre (+0x80) — two different points [M 0x18e148:203-214 + L]. Port-check §1: the old call
            // passed `player.pos` for both, so the take-ray origin differed from the game's by construction.
            if alive && !p.hidden && p.touches(d.world, player.cel, player.pos, player.sph) {
                let loaded = d.skins.contains_key(&d.weapons.first_person(p.weapon as usize));
                // `Pickup_Handler`: the armour case writes `BLData+0x8B0`; the clock is `GameState+0x52`
                // (the 10 → 0xC remap gate) — the port's tick count stands in for it.
                let clock = frame;
                let res = self.inventory.take(d.weapons, p, loaded, &mut self.health.armour, clock);
                // `NF_PICKUPLOG=1`: the take result and why (fix-player2's drop diagnostics; shared file —
                // one env-gated line only).
                if std::env::var_os("NF_PICKUPLOG").is_some() {
                    eprintln!("[take] tick {} weapon {:#x} kind {} rounds {} loaded {} → {:?}", frame, p.weapon, p.kind, p.rounds, loaded, res);
                }
                if let Some(t) = res {
                    // The respawn tail [M 0x18e650:338-348]: respawn 0 → deleted; -1 (0xFFFF) → stays
                    // takeable; else hidden (obj+0xF0 |= 0x10), state 2, until
                    // `FRAME_RATE_INT · respawn · 10` ticks pass. The port used to delete unconditionally.
                    match p.respawn {
                        0 => p.gone = true,
                        0xFFFF => {}
                        r => { p.hidden = true; p.state = 2; p.respawn_ticks = 60 * r as u32 * 10; }
                    }
                    // `Player_EquipWeapon`'s tail [M 0x1a8a58]: on a WEAPON equip (not the ammo path —
                    // `Player_EquipAmmo` has no switch), the per-player auto-switch flag (live 1 [L]) ∧
                    // `Player_IsBetterWeapon` writes `anim+0x63 = the upgraded id`. The port's `bond_weapon`
                    // is the view's held weapon (one tick stale at most).
                    if matches!(t, crate::pickup::Taken::Weapon { .. }) {
                        let id = crate::pickup::Inventory::upgrade_weapon(p.weapon, 0, clock);
                        if crate::pickup::Inventory::is_better_weapon(d.weapons, self.bond_weapon, id) {
                            self.weapon_switch_request = Some(id);
                        }
                    }
                    // `PlrStat_LogBondBonus(0)` 0x1999e0 — `Pickup_Handler` case 6 0x18e650:234-236
                    // logs the bonus pickup in the take-effect switch (fix-progress).
                    if matches!(t, crate::pickup::Taken::Bonus) { self.progress.log_bond_bonus(self.mission.status()); }
                    // `if (+0x28) switch_channels[+0x28] = 1` (+ time stamp) [M 0x18e650:330-333].
                    if p.switch_channel != 0 { self.channels.set(p.switch_channel as u32, 1, frame); }
                    // `Pickup_Handler` overwrites `+0x2A` with the per-kind sound at take time —
                    // 0xF5 weapon/ammo, 0x141 armour, 0x14C Bond bonus (:290,300,319-321) — before
                    // `Sound_Play3D` (:334-337). A placed bonus's create sound is 0xFFFF (none); the
                    // take still plays 0x14C [M]. fix-objects: kind 6 was silent.
                    let take_sound = match t {
                        crate::pickup::Taken::Bonus => Some(0x14C),
                        crate::pickup::Taken::Armour => Some(0x141),
                        _ => p.sound,
                    };
                    let (at, sound) = ([p.m[3][0], p.m[3][1], p.m[3][2]], take_sound);
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
        // fix-objects: taken/expired pickups STAY as tombstones (`gone`). The renderer draws placed
        // pickup slot `k` with build-time placed_pickups[k]'s mesh and poses it at
        // `self.pickups[k].m` (scene.rs `add_instance` + the pickup pose step); `View_DrawObjectGlist`
        // (0x1e6780) draws each pickup object's OWN glist at its OWN matrix — the game never reorders
        // a live pickup under a drawn instance. The old `self.pickups.retain(|p| !p.gone)` shifted
        // every later slot by one on the first take, so meshes were drawn at other pickups' and
        // drops' matrices (the user's "007 model at the armour spot" / "wrong model at the drop").
        // Every consumer already filters `gone`/`hidden`.
    }

    /// Turns a switch channel on (callers and tests).
    pub fn set_channel(&mut self, c: u8) { let f = self.frame; self.channels.set(c as u32, 1, f); }

    /// This tick's player-bullet segments (`from`, `to`) and their weapon ids — the same data the
    /// PC install check reads (crate::useobj; the game's class-5 contact list [M 0x1aee20]).
    pub fn bullet_paths(&self) -> (&[(V3, V3)], &[u16]) { (&self.bullet_paths, &self.bullet_path_weapons) }

    /// One 60 Hz game tick: global vision (`DroneVision_ProcessDroneSight`), then each drone's control.
    pub fn tick(&mut self, d: &Data, player: &Player) {
        let _prof_all = crate::prof::scope("drones.tick (total)");
        let prof_pre = crate::prof::scope("drones: shared passes");
        self.frame += 1;
        // The route-node → door binding (`NDrone_InitDoorNodes` 0x139a70), on the first tick: every
        // door-flagged node binds the nearest door (the scan takes its first candidate
        // unconditionally, so effectively the nearest overall); a node with no door never binds.
        if self.node_doors.is_none() {
            let mut m: HashMap<(usize, u16), usize> = HashMap::new();
            for (pi, p) in d.nav.paths.iter().enumerate() {
                for (ni, n) in p.nodes.iter().enumerate() {
                    if n.flags & 2 == 0 || self.doors.list.is_empty() { continue; }
                    let mut best: Option<(usize, f32)> = None;
                    for (di, door) in self.doors.list.iter().enumerate() {
                        let q = (door.pos[0] - n.pos[0]).powi(2) + (door.pos[1] - n.pos[1]).powi(2) + (door.pos[2] - n.pos[2]).powi(2);
                        if best.map_or(true, |(_, bq)| q < bq) { best = Some((di, q)); }
                    }
                    if let Some((di, _)) = best { m.insert((pi, ni as u16), di); }
                }
            }
            self.node_doors = Some(m);
        }
        // The gas puffs (`gases_smoke` 0x1ab2d0 runs in the object handler every frame).
        self.gas.update();
        // fix-impacts: `Debris_Update` 0x1aac00 — debris physics (gravity, spin, fade, life).
        self.effects.update();
        // fix-impacts: `Bullet_casing_update` 0x12faa0 — casings fly, land (the material sound via
        // `Sound_Play3D`), die at 240 ticks.
        {
            let audio = &mut self.audio;
            self.casings.update(|id, pos| { audio.play3d(id as u32, pos, None); });
        }
        let frame = self.frame;
        // `Mission_Update` (0x182c60) runs from `Game_Run` (0x1c9790:17) BEFORE the object handler
        // (`control_movement_object_handler` 0x1336f8:30-35) — i.e. before this frame's triggers,
        // scripts and the player/death path, on the channels as the previous frame left them.
        // fix-objective-banner: the port used to run it at the END of the tick, so the monitor saw
        // the Kimiko trigger's ch 0x48 on the trigger tick itself and queued the announce one tick
        // before the scene's `Script_CameraStart` disabled the viewer's HUD (0x1d9140:41-43) — the
        // banner+SFX fired the moment the cut-scene started instead of at its hand-back.
        // The per-level `Mission_Init` (0x182270, from `ResetMap_GameInit` 0x1d5080:53) latches on
        // this level's first tick; the port's loads are cold (warm = false — previous parts keep
        // their `.data` states, as the game's cold path).
        if !self.mission_inited {
            self.level_current = d.level; // GameState._12_4_ [M 0x1d5080:6] — the fail-menu Restart's target
            self.mission.init(d.level, self.mission_carry.is_some(), &mut self.channels, frame);

            // `PlrStat_ResetForMission` 0x1993a0 — Mission_Init 0x182270:65 wipes PlrMissionStats
            // (0x1C0) and BondMoments (0x14) when the level is the mission's first part [M].
            if crate::mission::LEVELS.iter().find(|l| l.level == d.level).is_some_and(|l| l.part == 0) {
                self.progress.reset_for_mission();
            }
            // `NDrone2_DefaultInit` 0x14b300:609 → `PlrStat_LogEnemySpawned(0)` per created drone.
            // The port builds the level's drones before this latch, and the game creates them after
            // `Mission_Init` ([L] ee_start_05.bin: +0x18 = 13 on 07000001, Mission_Status 1) — so
            // the initial count rides here, after the part-0 reset above.
            for _ in self.list.iter() { self.progress.log_enemy_spawned(self.mission.status()); }
            // The carried mission globals restore AFTER init, exactly as the game's `.data` globals sit
            // when `Mission_Init` runs on a warm transition: init resets the current + future parts in
            // place and the carried snapshot puts the previous parts' states back [M 0x182270:68-129;
            // the payload shape is LS_MakeMission/LS_LoadMission 0x1cd720/0x1cd880; map-tower2].
            if let Some(s) = self.mission_carry.take() { self.mission.restore(&s); }
            self.mission_inited = true;
        }
        // `Mission_Update` reads the DEATH CHANNEL, not health [M 0x182c60:21 `switch_channels[0x62]`]
        // — the death sequence (`crate::death`) gates the channel on the red fade / the player's ✕.
        let dead = self.channels.on(0x62);
        let mut events = std::mem::take(&mut self.mission_events);
        self.mission.update(&mut self.channels, frame, d.level, dead, &mut events);
        // `Mission_Update` 0x182c60's music pulses [M 0x182c60:30-33 (fail → Music_Event(7,1)),
        // 0x182c60:23-28 (success → Music_Event(8,1))]: the level updaters' end-sting block reads them
        // (EventList[8]; fix-party-mission item 4). The death pulse (event 6) is emitted at the health
        // check below; the fail/success ones ride the mission's own transitions here.
        if events.iter().any(|e| matches!(e, crate::mission::Event::MissionFailed)) { self.music.event(7, 1); }
        if events.iter().any(|e| matches!(e, crate::mission::Event::MissionComplete { .. })) { self.music.event(8, 1); }
        self.mission_events = events;
        // The presentation FX tick (ripples/flicker/clouds/apocalypse/emitters, fix-world-fx): the
        // game's object handler updates them per frame in the same control pass.
        self.fx.tick(&mut self.channels, player.pos, frame, &mut self.gas);
        // Music: `Drone_InitComms` resets the attack gate; `DroneFunc_CheckAlarmRaised` (07000005 / 06) posts event 2 = 3
        // for the first 58 frames of channel 0xA4, then = 1 for the first 598 of channel 0x37 or 0x61; Bond dead →
        // event 6 (`Player_CheckForDeath`, a player-side call — runs during cut-scenes, not gated).
        self.attack_gate.new_frame();
        // CheckAlarmRaised sits inside `Drone_InitComms`' `NPCGlobals != 0` block [M 0x139520:124-284]: a cut-scene
        // that disabled the drones (event 5) also mutes the alarm music.
        if self.npc_enabled && matches!(d.level, 0x0700_0005 | 0x0700_0006) {
            // Each counter: 0 → 1 when its channel is on, then + 1 per frame (to 0xFFFF); posts while below the limit.
            let ch = &self.channels;
            let mut step = |k: usize, c: u32, limit: u16| -> bool {
                if !ch.on(c) { return false; }
                let n = &mut self.alarm_music[k];
                if *n == 0 { *n = 1; }
                if n.wrapping_sub(1) < 0xfffe { *n += 1; }
                *n < limit
            };
            if d.level == 0x0700_0005 && step(0, 0xa4, 60) { self.music.event(2, 3); }
            else if step(1, 0x37, 600) || step(2, 0x61, 600) { self.music.event(2, 1); }
        }
        // `Player_CheckForDeath` 0x190810 + `Player_HandleDeath` 0x190bb0 (fix-death-restart): the
        // death sequence gates ch 0x62 on the blood pane's cover or the player's ✕ [M 0x190bb0:29-33].
        self.death_tick(d, player.pos, frame);
        // `Game_Run`: `Env_Update` (one `Rand_Rand(20000)`, then `UpdateDrops`) before the object handler.
        if let Some((kind, boxes)) = self.env_pending.take() { self.env = Some(crate::env::Drops::new(kind, boxes, d.level, &mut Rng::new(0x5eed_0001))); }
        if let Some(env) = self.env.as_mut() {
            self.rng.rand(20000);
            env.update(&mut self.rng, &mut self.fill_rng, player.eye, atan2_approx(player.aim[0], player.aim[2]));
        }
        // `Drone_ProcessOpponents` (0x1390c0): Bond's visibility factor from the AI volumes holding him (crouching
        // ×0.75 is not modelled: the port's Bond does not crouch), then each drone's opponent info. Inside
        // `Drone_InitComms`' `NPCGlobals != 0` block [M 0x139520:124-284] — frozen drones keep their last state.
        if self.npc_enabled {
            let light = crate::aivolume::visibility_factor(d.volumes, player.pos, false);
            let armed = (2..0x47).contains(&self.bond_weapon) || (0x6a..=0x71).contains(&self.bond_weapon);
            for dr in self.list.iter_mut().filter(|x| x.present) { dr.light = light; dr.opponent_info(player); dr.seen = false; dr.bond_armed = armed; }
        }
        self.update_player_bullets(d);
        // The use objects' per-tick halves (`PCQWorm_Update` 0x1aee20's bullet install +
        // `Switch_Update` 0x1adea8's value/auto-return; map-tower, crate::useobj). The bullet
        // segments are this tick's paths — the game reads the class-5 contact list the same pass.
        let (paths, weapons) = self.bullet_paths();
        let shots: Vec<(V3, V3, u16)> = paths.iter().zip(weapons).map(|(&(a, b), &w)| (a, b, w)).collect();
        self.useobjs.tick(&shots, &mut self.channels, frame);
        // The world objects' update pass (`control_movement_object_handler` 0x1336f8 pass 4:
        // `control_funcs[+0xFF].update` — InverterTrigger 0x1b30a8, Shooter 0x17d0c0 (fires one
        // `Bullet_init` bullet into this tick's list), DynamicObject 0x18d3b8; the camera splines
        // stay static registration). The viewer position for the fade is `glb_viewer+0xC0` — the
        // player's eye here (headless stand-in [S]).
        let wo_viewer = [player.eye[0], player.eye[1], player.eye[2]];
        crate::worldobj::WorldObjects::tick(
            &mut self.worldobjs, &mut self.channels, wo_viewer, d.weapons, &mut self.rng, &mut self.bullets, frame,
        );
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
        // The hearing list is drained BEFORE the gate so sounds made during a freeze don't burst out on re-enable.
        let sounds = std::mem::take(&mut self.sounds);
        // `Drone_InitComms`' shared passes — `ProcessDroneSight`, `FindAlertedDrones`, the 0x97 check,
        // `HandleSoundAlerts`/`Sound_ZeroAlertness` and the ch 0x96 alarm rebroadcast — sit inside the
        // `NPCGlobals != 0` block [M 0x139520:124-284]: a cut-scene that fired event 5 freezes all of these.
        if self.npc_enabled {
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
                if dr.seen_ticks >= dr.reaction_time(d.level) {
                    if !dr.seen && std::env::var_os("NF_AILOG").is_some() {
                        eprintln!("[ailog] guard {} SEEN! (seen_ticks={} >= reaction_time={}, dist={:.1}, state={:#x})",
                            dr.index, dr.seen_ticks, dr.reaction_time(d.level), dr.dist, dr.state);
                    }
                    dr.seen = true; dr.ever_seen = true;
                }
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
            // `DroneFunc_DoForcedAttack` 0x148c60's gate: on 07000007 while the interrogate latch
            // (`DAT_0029aa86` — `party_loud` here) is 0, only the bouncer mode 0x13 and the
            // CivilianMission 0x16 walkers force-attack; the guests hold the party [M 0x148c60:10-13].
            if d.level == 0x0700_0007 && !self.party_loud && !matches!(o.mode, 0x13 | 0x16) { continue; }
            o.alerted_by_drone = true;
            o.alert_bits |= b << 16;
            o.send(d, &mut self.rng, frame, msg::FORCED_ATTACK);
        }
        // `Drone_InitComms` after the vision pass: an enabled (`+0x4F8 & 0x100`, set by DefaultInit / Enable), living
        // (`& 0x600` clear) drone at alertness ≥ 1 within 30 units of Bond (not team 2) raises `switch_channels[0x97]`.
        // Direct `.v` write: `Drone_InitComms` stamps NO timestamp on 0x97/0x98 [M 0x139520:198-201 — their snapshot
        // time cells still hold the Init seed]; `Channels::set` stamps like the game's trigger/script writers.
        if self.list.iter().any(|x| x.present && x.enabled && !x.corpse && x.alert >= 1.0 && x.dist < 30.0) { self.channels.v[0x97] = 1; }
        // `DroneFunc_HandleSoundAlerts` (every drone, every frame) over the sounds made since the last frame, then
        // `Sound_ZeroAlertness`: each sound alerts once.
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
        // The scream/shout broadcasts queued by the punch/scare paths this tick
        // (`Drone_SM_BroadcastMsg` 0x145e28:56 — the alert source's position rides the alert; the
        // PartyGirl armed shout's radii are 20/20/0.75 [M 0x160140:63-70], the port uses 20.0).
        let shouts: Vec<(u8, V3)> = self.list.iter_mut()
            .filter(|x| x.present && x.enabled)
            .filter_map(|x| x.pending_shout.take())
            .collect();
        for (m, pos) in shouts {
            for dr in self.list.iter_mut().filter(|x| x.present && x.enabled) {
                let dx = [dr.pos[0] - pos[0], dr.pos[1] - pos[1], dr.pos[2] - pos[2]];
                if dx[0]*dx[0] + dx[1]*dx[1] + dx[2]*dx[2] > 400.0 { continue; }
                dr.send(d, &mut self.rng, frame, m);
            }
        }
        } // end `NPCGlobals` shared-pass block
        // Bond's motion (`DroneWeap_DoOpponentTargetting`): moving, and "first moved" for 2 s after he starts.
        let moving = self.prev_player.is_some_and(|q| (q[0] - player.pos[0]).abs() + (q[2] - player.pos[2]).abs() > 1e-4);
        if moving && self.moved_since.is_none() { self.moved_since = Some(frame) }
        if !moving { self.moved_since = None }
        let first_moved = self.moved_since.is_some_and(|f| frame < f + 120);
        self.prev_player = Some(player.pos);
        // `NDrone2_NavNodeCache` / `BuildAITarget`: the player's goal nodes per navigation path, rebuilt when he has
        // moved 2.0 or more (search radius 6 = min(engage range, 6) for these guards); `AINetwork_ClearLinkFlags`.
        // Target rebuild + the cover-node walk are `Drone_InitComms` shared passes — gated on `NPCGlobals` [M 0x139520].
        let moved = self.npc_enabled && (self.target.stamp == 0 || {
            let q = self.target.pos;
            (q[0] - player.pos[0]).powi(2) + (q[1] - player.pos[1]).powi(2) + (q[2] - player.pos[2]).powi(2) >= 4.0
        });
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
        if self.npc_enabled {
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
        }
        let positions: Vec<Option<V3>> = self.list.iter().map(|x| x.enabled.then_some(x.pos)).collect();
        // The armed-peer distances for `NDrone2_NearArmedDrone`'s calls (fix-actors): the nearest
        // started, alive, team-1 drone (positions only here; the per-drone team/started filter runs
        // on the same list before the mutable borrow below).
        let armed_peer_dist: Vec<f32> = self.list.iter().enumerate().map(|(i, dr)| {
            self.list.iter().enumerate()
                .filter(|&(j, o)| j != i && o.team == 1 && o.started && !o.corpse && o.enabled)
                .map(|(_, o)| ((o.pos[0] - dr.pos[0]).powi(2) + (o.pos[1] - dr.pos[1]).powi(2) + (o.pos[2] - dr.pos[2]).powi(2)).sqrt())
                .fold(f32::MAX, f32::min)
        }).collect();
        let peers: Vec<(usize, V3, bool)> = self.llist.iter().map(|&i| (i, self.list[i].pos, self.list[i].seen)).filter(|&(i, _, _)| self.list[i].enabled).collect();
        // NoBunching's candidate snapshot (present, enabled, non-corpse — the game's chain rides
        // +0x4F8 bits the port does not model), indexed by drone index.
        let bunching_ok: Vec<bool> = self.list.iter().map(|x| x.present && x.enabled && !x.corpse).collect();
        // `Trigger_Calc`, then every object in the control list (newest first = reverse placement order): triggers and
        // drones interleaved, so a channel a trigger pulses is seen only by the objects updated after it.
        // The player toucher keeps the historical stand-in (`Toucher::player(pos − 0.4)`, reach
        // feet+0.53..feet+3.14). Two alternatives were tried and REVERTED, each breaking a shipped
        // behaviour:
        // - the game-accurate capsule (0x194df0's pos+0.55 / pos+0.275−cc, on `Player::capsule`)
        //   reaches ~0.9 lower: 07000005's truck chain stalled (the hand-off ch 0x5C never fired —
        //   no mask-8 trigger writes it [D trigger dump]; a class-5 object contact is the likely
        //   game path, trigger-port-check #11) AND the standing ride touched the ch-0x70 park plate
        //   the game only fires crouched (fix-truck's trace) — the fix_truck_live sister caught it.
        // - the union of both capsules also let the standing ride touch the park plate.
        // The stand-in's known misses (documented): floor plates within 0.53 of the floor
        // (07000005's ch-0x45 reveal plate at feet+0.47; 07000002's bunker plate 0x80 below the
        // floor — the stealth route). Revisit when object touchers land.
        let touchers: Vec<crate::triggers::Toucher> = std::iter::once(crate::triggers::Toucher { contain: !player.hanging, ..crate::triggers::Toucher::player([player.pos[0], player.pos[1] - 0.4, player.pos[2]]) })
            .chain(self.list.iter().filter(|x| x.enabled).map(|x| crate::triggers::Toucher { a: [x.pos[0], x.pos[1] - 1.0, x.pos[2]], b: x.head(), r: 0.4, drone: true, skin: x.skin, contain: true }))
            .collect();
        drop(prof_pre);
        { let _p = crate::prof::scope("drones: Trigger_Calc"); crate::triggers::calc(d.level, &mut self.triggers, &mut self.channels, &touchers, frame); }
        // `switch_channels_prev` is snapshotted right after `Trigger_Calc`, BEFORE the object walk [M 0x1336f8:52-53
        // — `prev ← hold; hold ← channels` at 0x1337cc/0x1337dc] — so `Door_Update`'s lock rising edge
        // (`prev == 0 && now != 0`, 0x136550-0x136608) sees a channel a trigger or script pulsed earlier this
        // frame (core-loop §7 item 5). It used to snapshot at the END of the tick, so a 1-tick lock pulse was
        // invisible to the edge test.
        self.doors.end_tick(&self.channels);
        // The helicopter (`Copter_Update` 0x17dbb8, fix-objects2): the gate check, the countdown
        // pulse and the arrival write; on the gate's rising edge the flight script starts as a
        // synthetic one-shot player (the game's hidden `SP_Create` inside `Copter_Create` 0x17d340).
        let mut copter_flight: Vec<(usize, u32, nf_formats::anim::Mat4)> = Vec::new();
        for c in &mut self.copters {
            let len = self.scripts.lib.get(&c.scripts[1]).map(|n| n.length as u32).unwrap_or(0);
            let (started, _) = c.tick(&mut self.channels, len, frame);
            if started { copter_flight.push((c.placement, c.scripts[1], c.matrix)); }
        }
        for (placement, script, matrix) in copter_flight { self.scripts.spawn_once(placement, script, matrix); }
        // The script players' tick head (the create-time load beat — the game does it during the
        // level load [M 0x1db9a8:81-86]): the players THEMSELVES update from the control walk
        // below, in the game's list order (fix-script-start-tick — the old all-players-here beat
        // ran a trigger-started script one tick behind the placement trigger that started it).
        let script_skip = { let _p = crate::prof::scope("drones: scripts head"); self.scripts.begin_tick(&mut self.channels, d, frame) };
        // The script fade overlay's per-tick advance (`Camera_SetFade`'s viewer state; the game ticks
        // it in the viewer update): the alpha moves by the rate each tick and parks at its end —
        // 1.0 after a fade-to-black (the screen stays black until a fade-in), 0.0 after a fade-in.
        if let Some(f) = self.view_fade.as_mut() {
            if f.rate > 0.0 { f.alpha = (f.alpha + f.rate).min(1.0); }
            else if f.rate < 0.0 { f.alpha = (f.alpha + f.rate).max(0.0); }
        }
        // Script objects with collision data join the collision world where they now are: animations' discrete
        // parts and drawn entities (`Collide_PickObj`; triggers and AI volumes are not obstacles).
        // Id: object index · 64 + part (stable while the object lives) for the ride (`ground_motion`).
        // *set_dynamic moved after the walk* (fix-script-start-tick): the walk now moves the script
        // objects, and the game's `Collide_Update` runs after the walk [M 0x1336f8:238] — the ride
        // reads this walk's poses, the same one-tick-behind-Bond lag the all-players-here beat
        // produced before. During the walk the dynamic set holds the last walk's poses (the game's
        // does too, until Collide_Update).
        let script_parts = |drones: &Drones, d: &Data| -> Vec<(u64, u32, nf_formats::anim::Mat4)> {
            let mut parts = Vec::new();
            for (oi, o) in drones.scripts.objects.list.iter().enumerate() {
                let id = |k: usize| (oi as u64) << 6 | k as u64;
                match o {
                    Some(crate::nis::Obj::Anim { skin, script, frame, .. }) => parts.extend(d.discrete_parts(*skin, *script, *frame).into_iter().enumerate().map(|(k, (h, m))| (id(k), h, m))),
                    // fix-safe08: pickups (0xF0) stay out of the collision world — the game's `Script_CreateEntity`
                    // control-unlinks the created pickup (0x1d8d54); its own glist blocked its take ray.
                    Some(crate::nis::Obj::Entity { hash, kind, matrix, .. }) if !matches!(kind, 0xea | 0xeb | 0xf5 | 0xf2 | 0xf0) => parts.push((id(0), *hash, *matrix)),
                    _ => {}
                }
            }
            // Doors are solid through their object's collision mesh at `obj+0x90` (`Collide_PickObj` walks the
            // cel-linked objects whatever moves them); the pose this tick's `Door_Update` writes lands in the
            // same set_dynamic now (the game's Collide_Update is post-walk [M 0x1336f8:238]). Id: high bit 40 + the door index.
            parts.extend(drones.doors.list.iter().enumerate().filter(|(_, x)| x.glist != 0)
                .map(|(k, x)| (0x100_0000_0000 | k as u64, x.glist, x.matrix)));
            // The SimpleScripts too (fix-party-mission item 7): the safe door (ch 136) and the doorway
            // bolt (ch 137) are solid at their posed matrix, like the doors. Id: bit 41 + the ss index.
            parts.extend(drones.sss.list.iter().enumerate().filter(|(_, x)| x.hash != 0)
                .map(|(k, x)| (0x200_0000_0000 | k as u64, x.hash, x.matrix)));
            parts
        };
        // Script triggers (`Trigger_TouchOnce` / `Trigger_Touch` made by `Script_CreateEntity`): one per live entity,
        // its volume the glist's collision at the entity's matrix this tick; gone with the object. New objects head the
        // control list, so they update before the placed ones (placement index past them all).
        let live: Vec<(usize, u8, Vec<u32>, u32, nf_formats::anim::Mat4)> = self.scripts.objects.list.iter().enumerate().filter_map(|(i, o)| match o {
            Some(crate::nis::Obj::Entity { hash, kind, params, matrix, .. }) if matches!(kind, 0xea | 0xeb) => Some((i, *kind, params.clone(), *hash, *matrix)),
            _ => None,
        }).collect();
        self.triggers.retain(|t| t.script.is_none_or(|o| live.iter().any(|l| l.0 == o)));
        for (o, kind, params, hash, m) in live {
            let tris: Vec<[V3; 3]> = d.world.shape(hash).map(|s| s.iter().map(|t| t.map(|p| nf_formats::placement::transform(&m, p))).collect()).unwrap_or_default();
            match self.triggers.iter_mut().find(|t| t.script == Some(o)) {
                Some(t) => t.set_tris(tris),
                None => {
                    let k = if kind == 0xea { crate::triggers::kind::TOUCH_ONCE } else { crate::triggers::kind::TOUCH };
                    let q = |i: u32| params.get(i as usize).copied().unwrap_or(0);
                    if let Some(mut t) = crate::triggers::Trigger::from_params(usize::MAX - o, k, q, tris) { t.script = Some(o); self.triggers.push(t); }
                }
            }
        }
        // The requests raised before the walk (the priming beat, the copter's spawned player): the
        // game's create-time effects land before any object updates. The walk drains again after
        // every script player (`drain_script_requests` from the kind-5 arm below).
        self.drain_script_requests(d);
        for k in 0..self.script_sounds.len() {
            let (_, h, at) = self.script_sounds[k];
            if let Some(p) = at.and_then(|o| Self::script_obj_pos(&self.scripts, d, o)) { self.audio.set_position(h, p); }
        }
        // `FuseBox_Update` (0x1adfe8, the class-0x3C control update) — this tick's wire hits are
        // consumed: dying wires lose their entity (`Effect_RemoveSObjs` 0x1cf360), a group-1 death
        // hurts while +0x56 == 0 and cancels +0x5C, and an emptied group writes its two channels
        // (one-shot by the ext zeroing).
        for fi in 0..self.fuseboxes.len() {
            let cuts = self.fuseboxes[fi].update(&mut self.channels, frame);
            for c in cuts {
                match c {
                    crate::fusebox::Cut::RemoveWire { obj } => {
                        if let Some(slot) = self.scripts.objects.list.get_mut(obj) { *slot = None; }
                    }
                    crate::fusebox::Cut::LiveWireHurt { obj } => {
                        // `Player_Hurt(0x40a00000, glb_players, wire+0x80)` [M 0x1adfe8:99-101] —
                        // the flat `pain_generic` the explosions use (the position is the dying
                        // wire's; the port's hurt has no radius falloff, docs/fields/fusebox.md #6).
                        let at = Self::script_obj_pos(&self.scripts, d, obj).unwrap_or(self.fuseboxes[fi].pos);
                        let before = self.health.health;
                        self.health.pain_generic(crate::fusebox::LIVE_WIRE_HURT, self.difficulty, d.level, 0);
                        self.player_hits.push((frame, usize::MAX, before - self.health.health));
                        let _ = at;
                    }
                }
            }
        }
        let audio = &self.audio;
        self.script_sounds.retain(|s| !audio.finished(s.1));
        let mut order: Vec<(usize, u8, usize)> = self.list.iter().enumerate().filter(|(_, x)| x.present).map(|(i, x)| (if x.spawn_seq != 0 { usize::MAX - x.spawn_seq as usize } else { x.placement }, 0, i))
            .chain(self.triggers.iter().enumerate().filter(|(_, t)| t.kind != crate::triggers::kind::TOUCH).map(|(j, t)| (t.placement, 1, j)))
            .chain(self.spawners.iter().enumerate().map(|(j, sp)| (sp.placement, 2, j)))
            .chain(self.searchlights.parts.iter().enumerate().filter(|(_, p)| p.kind == 1).map(|(j, p)| (p.placement, 3, j)))
            .chain(self.doors.list.iter().enumerate().map(|(j, d)| (d.placement, 4, j)))
            // The script players (fix-script-start-tick): `SP_Update` among the control funcs, in the
            // same newest-first list order [M 0x1336f8:54-68 — `control_funcs[obj+0xFF]` per object].
            .chain(self.scripts.players.iter().enumerate().filter(|(_, p)| !p.deleted).map(|(j, p)| (p.placement, 5, j)))
            .collect();
        order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        // Pickups are created at run time, so they head the control list: `Pickup_Update` before the placed objects.
        self.update_pickups(d, player, frame);
        // The actor list `Door_Update`'s proximity test (`dist² < (r1+r2)²·2` over classes
        // 2/3/0x12, skipping `obj+0xF0 & 0x10` candidates [M 0x136678-0x136684]) sees: the player
        // first (`glb_players`, the activator of every auto-activation) with radius 0.4, then every
        // live non-corpse drone with radius 0.4. The third field is the `obj+0xF0 & 0x10` skip bit:
        // modelled CLEAR for both — the 07000005 player reads 0x880 [L ee_start_05.bin]; the drones'
        // 07000005 value is not in the snapshots and the bit's writer is unported (door-port-check #2:
        // on 07000001 both are SET [L], which the port cannot reproduce yet).
        let actors: Vec<crate::door::DoorActor> = std::iter::once((player.pos, 0.4, false))
            .chain(self.list.iter().filter(|x| x.present && !x.corpse).map(|x| (x.pos, 0.4, false)))
            .collect();
        // Per drone: timers, alertness decay, update message, turning, aim, firing, animation.
        let mut talk_queue: Vec<(usize, Vec<(usize, bool)>)> = Vec::new();
        let mut door_sounds: Vec<(u32, V3)> = Vec::new();
        let prof_loop = crate::prof::scope("drones: control-list loop");
        // fix-spawner-p7: the spawner kill cleanup's draw-counter gate reads the spawned drone's
        // `obj+0xFC` [M `lbu $v0, 0xfc($s0)` 0x13805c] as of the LAST frame — the game's renderer
        // writes it and the decrement pass runs after the control walk [M 0x1336f8:239-247,
        // core-loop item 11] — so the scan sees this walk-start snapshot, not a same-walk recompute
        // (a spawned guard whose per-drone pass runs before its spawner's would otherwise read fresh).
        let drawn_prev: Vec<bool> = self.list.iter().map(|d| d.drawn).collect();
        for &(_, what, index) in &order {
            if what == 1 { let _pw = crate::prof::scope("ctl: kind 1");
                self.triggers[index].update(&mut self.channels, &touchers, frame);
                // `LOAD_LEVEL` (0xE8): when the exit trigger fires, carry the next level's hashcode for the main loop
                // to reload (`Trigger_Activate` 0x1b2318 → `ResetMap_LevelToLoad`). Also post `Music_Event(3, 1)`
                // (the end-sting) and `GS_PauseGame(1)` (the pause during transition) [M 0x1b2318:108-110].
                if self.triggers[index].kind == crate::triggers::kind::LOAD_LEVEL && self.triggers[index].gone && self.triggers[index].next_level != 0 {
                    if self.level_exit.is_none() {
                        self.level_exit = Some(self.triggers[index].next_level);
                        self.music.event(3, 1);
                    }
                }
                // NF_LEVELLOG=1: once per second, log each LOAD_LEVEL trigger's gate channel, whether the gate
                // is on, and whether the trigger has fired. Lets the user see why the level doesn't end —
                // e.g. ch 0x1D is off because the EndLevel script never ran.
                if std::env::var("NF_LEVELLOG").is_ok_and(|v| v == "1")
                    && self.triggers[index].kind == crate::triggers::kind::LOAD_LEVEL
                    && frame % 60 == 0
                {
                    let t = &self.triggers[index];
                    let gate_on = t.gate != 0 && self.channels.on(t.gate as u32);
                    eprintln!("[levellog] frame {} LOAD_LEVEL placement {} next {:08x} gate 0x{:02x}={} gone={}", frame, t.placement, t.next_level, t.gate, if gate_on { "ON" } else { "off" }, t.gone);
                }
                continue;
            }
            if what == 4 { let _pw = crate::prof::scope("ctl: kind 4");
                // `Door_Update` (docs/systems/world-objects.md "Doors"): channels, the auto-close timer, the
                // anim; its sounds (open 285 / close 284 / the locked 320 thud on 07000005) enter the pool.
                let evs = self.doors.update(index, &mut self.channels, &actors, frame);
                for (id, pos) in evs {
                    match d.sfx {
                        Some(t) => { let h = t.get(id as u32); self.play_sound_handle(pos, h); }
                        None => { self.audio.play3d(id as u32, pos, None); }
                    }
                }
                continue;
            }
            if what == 3 { let _pw = crate::prof::scope("ctl: kind 3");
                // `Searchlight_Update` (docs/systems/world-objects.md): channels, sweep/tracking, the parts, the
                // break check; its `Sound_Play3D` events (0xE5 relay, 0xE6 glass) enter the pool and the hearing list.
                let evs = self.searchlights.update(d, player, &mut self.channels, index, &self.bullet_paths, &mut self.rng, frame);
                for (id, pos) in evs {
                    match d.sfx {
                        Some(t) => { let h = t.get(id); self.play_sound_handle(pos, h); }
                        None => { self.audio.play3d(id, pos, None); }
                    }
                }
                // The broken light's smoke [M 0x1cb900:373-385: state 2, while drawn (obj+0xFC != 0),
                // every 7th frame `(GameState[52] + obj+0xEC) % 7 == 0` — obj+0xEC is GameState[52]
                // AT CREATE (control_init_object 0x134630:15), so the port's phase is
                // `(frame + created) % 7` with created = 0 (the port creates at frame 0);
                // `Gas_Init(0.05, 0.25, …, 0x2000039, 7, 0x48403080, 1, [stack] 128)` — the
                // decompile drops the 11th arg; the disassembly passes life = 0x80 = 128
                // [M 0x1cbce0/0x1cbcb0]. The puff's 6 shared-RNG draws are consumed by
                // `init_searchlight_smoke` (gas.rs — the game's formulas, bit-exact).
                let smoke: Vec<[f32; 3]> = self.searchlights.parts.iter()
                    .filter(|p| p.kind == 1 && p.state == 2 && p.drawn && (frame + p.created) % 7 == 0)
                    .map(|p| p.pos).collect();
                for pos in smoke { self.gas.init_searchlight_smoke(&mut self.rng, pos); }
                // The kill path clears a kind-2 part's light (`Searchlight_Update`'s off-channel branch
                // removes the `light_tag` record): mirror it onto the light list.
                for (k, p) in self.searchlights.parts.iter().enumerate() {
                    if p.kind == 2 && p.light.is_none() {
                        for l in self.lights.lights.iter_mut() {
                            if l.kind == 2 && l.track == Some(k as u32) { l.active = false; }
                        }
                    }
                }
                continue;
            }
            if what == 2 { let _pw = crate::prof::scope("ctl: kind 2");
                // `DroneSpawner_Control` returns immediately while the drones are disabled (`NPCGlobals == 0`,
                // [M 0x137e68:21-22]) — a cut-scene with event 5 also freezes spawners (fix-spawner-p7:
                // measured live on 07000005 — the ch-0x43 script held the p7 latch off until Script_Free).
                if !self.npc_enabled { continue; }
                // `Drone_bDisableSystem` (0x137e68:24-26): the options-menu drones-off setting —
                // false in the port (no menu); the gate keeps Control's early-out order faithful.
                if self.npc_system_disabled { continue; }
                let list = &self.list;
                let rng = &mut self.rng;
                // `DroneSpawner_Control` 0x137e68 (fix-room-sound): mode 3's first wave on the
                // power-station pair announces itself — `Rand_Rand(2)` then `NDrone2_PlaySFX(1.0,
                // NULL, 0xd6+draw, 0, 1)` [M decomp 0x1382fc-0x138324, disasm `jal 0x1e3738;
                // addiu $a0,2` 0x138304]: DCVars NULL → the zero position and ref 0. The draw
                // precedes the call, so it happens even when the entry's 600-frame repeat gate
                // (0xd6 → 1353, 0xd7 → 1318) suppresses the play; the wave-spawned gate (+0x2C
                // == 0) is the port's `wave == 0` and "none alive" is `update`'s mode-3 `may`.
                {
                    let sp = &self.spawners[index];
                    if sp.mode == 3 && sp.wave == 0 && sp.active && !sp.gone
                        && (d.level == 0x0700_000c || d.level == 0x0700_000d)
                        && sp.spawned.iter().filter(|&&i| list[i].present).count() == 0 {
                        let draw = rng.rand(2) as usize;
                        if let Some(l) = Self::foley_entry(d.voices, &mut self.voice_last, 0xd6 + draw, frame) {
                            let pos = [0.0f32; 3];
                            match d.sfx {
                                Some(t) => { let h = t.get(l.sfx); self.audio.play3d(h.id, pos, None); if !h.looped { self.sounds.push(Sound { id: h.id, pos, inner: h.inner, outer: h.outer, weight: h.weight }); } }
                                None => { self.audio.play3d(l.sfx, pos, None); }
                            }
                        }
                    }
                }
                // fix-spawner-p7: the kill gate reads the walk-start `drawn` snapshot (obj+0xFC as of
                // the last frame — see the snapshot above), plus the level id for the cleanup's split.
                let action = self.spawners[index].update(&mut self.channels, frame, d.level,
                    |i| list[i].present, |i| drawn_prev[i],
                    |i, min| { let q = list[i].pos; let dd = ((q[0] - player.pos[0]).powi(2) + (q[2] - player.pos[2]).powi(2)).sqrt();
                        dd >= min && !visible_to(player, d.world, q, [q[0], q[1] + 0.6, q[2]]) },
                    &mut |n| rng.rand(n));
                match action {
                    crate::spawners::Action::Spawn(i) => {
                        let dr = &mut self.list[i];
                        dr.present = true; dr.enabled = true; dr.started = false;
                        // `control_add_object_to_list` prepends [M 0x134698]: a drone spawned this frame updates
                        // before every placement object from its NEXT frame on, and NOT in the frame it spawned
                        // (the walk's cursor is already past the list head when the spawner runs) — core-loop §7
                        // item 9. `spawn_seq` orders the newest first; `spawn_frame` skips the spawn tick.
                        self.spawn_counter += 1;
                        dr.spawn_seq = self.spawn_counter;
                        dr.spawn_frame = frame;
                        self.llist.push(i);
                        // `NDrone2_DefaultInit` 0x14b300:609 → `PlrStat_LogEnemySpawned(0)` per created
                        // drone (fix-progress).
                        self.progress.log_enemy_spawned(self.mission.status());
                    }
                    crate::spawners::Action::Remove(v) => {
                        // 0x07000016's kill first raises each removed drone's death channel
                        // [M `jal 0x13e660` at 0x138074 → DroneFunc_SetDeathChannel 0x13e660:
                        // `switch_channels[+0x13C] = 1` + the time stamp; 0x63 = the mission-fail
                        // reasons, not modelled — the same rule as the port's set_death_channel].
                        if d.level == 0x0700_0016 {
                            for &i in &v {
                                let ch = self.list[i].death_channel;
                                if ch != 0 && ch != 0x63 { self.channels.set(ch as u32, 1, frame); }
                            }
                        }
                        for i in v { self.list[i].present = false; self.list[i].enabled = false; self.llist.retain(|&k| k != i); }
                    }
                    crate::spawners::Action::None => {}
                }
                continue;
            }
            if what == 5 { let _pw = crate::prof::scope("ctl: kind 5");
                // `SP_Update` in the walk (fix-script-start-tick): a script a placement trigger
                // started runs its first commands HERE — after the trigger wrote its channel, before
                // the spawners/drones below it in the list order [M 0x1336f8:54-68; SP_LoadScript's
                // trailing `Script_Update` 0x1dbc08:56]. The requests drain immediately: event 5's
                // `Drone_EnableAll(0)` must gate the later objects' control funcs the same frame
                // (`DroneSpawner_Control` 0x137e68:21-23 returns while NPCGlobals == 0).
                // The SP_Update state-0 start checks [M 0x1dbda8:80-106]: Bond's health
                // (BLData+0x894 [D]), the mission status (MissionState 0x30cb9c), and the LAST
                // walk's EventList[2] (the combat-music event; see prev_event2). States 1/3/6
                // retrigger without the checks [M cases 1/3/6].
                let sp_gate = crate::nis::SpStart {
                    health_ok: self.health.health > 0.0, // BLData+0x894 > 0 [D; the port's health tracker]
                    mission: self.mission.status(),
                    music2: self.prev_event2,
                };
                self.scripts.update_player(index, &mut self.channels, d, frame, script_skip, sp_gate);
                self.drain_script_requests(d);
                continue;
            }
            // A cut-scene stops the drones only when the game itself says so: `Drone_Control` returns while
            // `NPCGlobals == 0` (script event 5 → `Drone_EnableAll(0)`, undone by `Script_Free`'s 0x80 flag)
            // [M 0x139268:39-42, 0x1d94c0 case 5, 0x1da880:41-43]. `control_movement_object_handler` itself runs
            // whenever `GS_IsPaused == 0` — ScriptCam plays no role [M 0x1c9790:30-35] — so scripts, triggers and
            // doors above keep running either way (the B-10 lesson).
            if !self.npc_enabled { continue; }
            let rng = &mut self.rng;
            let dr = &mut self.list[index];
            if !dr.present { continue; }
            if dr.spawn_frame == frame { continue; }   // spawned this tick: the game's walk already passed it
            if dr.target.stamp != self.target.stamp { dr.target = self.target.clone(); }
            dr.channel_on = self.channels.on(dr.switch_channel as u32);
            dr.wait_channel_on = dr.wait_channel != 0 && self.channels.on(dr.wait_channel as u32);
            { let _p = crate::prof::scope("drone: visible_to (drawn ray)"); dr.drawn = dr.enabled && visible_to(player, d.world, dr.pos, dr.head()); }
            dr.links_used = std::mem::take(&mut self.used);
            dr.cover_rt = std::mem::take(&mut self.cover_rt);
            let _pc = crate::prof::scope("drone: clones + misc");
            dr.others = positions.clone();
            dr.peers = peers.clone();
            // The NoBunching flag computation is SUSPENDED (see the report): the scan alone regressed
            // the 07000001 death-drop tests (fix_objects_live a_death_draws_its_drop, green on master
            // 0cda458 and at the kick commit) — the flags feed only the 0x5E gate, so a zero here
            // disables the gate, not the tested kick path. Root cause next session.
            dr.no_bunching = 0;
            // `NDrone2_NearArmedDrone`'s scan (fix-actors): the nearest started, alive, hostile peer.
            // (The snapshot list was taken before the mutable borrow; the armed/started filter rides it.)
            dr.armed_peer_dist = armed_peer_dist[index];
            // `NDrone2_PreDroneControl`: the second switch (`+0x135`) with a tag (`+0x13A`): 0xC deletes the drone,
            // otherwise the alternate type's state (`+0x5A4`; Attack for these guards, *suggested*).
            if dr.second_channel != 0 && dr.second_tag != 0 && self.channels.on(dr.second_channel as u32) {
                dr.second_channel = 0;
                if dr.second_tag == 0xc { dr.present = false; dr.enabled = false; continue; }
                dr.second_tag = 0;
                dr.enabled = true;
                dr.set_state(if dr.alt_state != 0 { dr.alt_state } else { state::ATTACK });
            }
            dr.voice_finished = dr.voice.is_none_or(|v| self.audio.finished(v.0));
            dr.patrol_cooldown = self.patrol_cooldown;
            // `NDrone2_DoorIsOpen` (0x14aaa0) refreshed for this tick: no door in the way → open;
            // else the door's obj+0xF4 == 4.
            dr.hit_door_open = dr.hit_door.map_or(true, |i| self.doors.list[i].state == crate::door::OPEN);
            if !dr.started { dr.started = true; dr.send(d, rng, frame, msg::ENTER); dr.entered = frame; dr.log.push((frame, dr.state)); }
            if let Some(t) = dr.timer_c { if frame >= t { dr.timer_c = None; dr.send(d, rng, frame, msg::TIMER_C); } }
            if let Some(t) = dr.timer_d { if frame >= t { dr.timer_d = None; dr.send(d, rng, frame, msg::TIMER_D); } }
            if let Some(t) = dr.idle_timeout { if frame >= t { dr.idle_timeout = None; dr.send(d, rng, frame, msg::IDLE_TIMEOUT); } }
            if let Some((t, m)) = dr.delayed { if frame >= t { dr.delayed = None; dr.send(d, rng, frame, m); } }
            if dr.alert < 1.0 && dr.alert > dr.alert_floor { dr.alert -= 1.0 / (60.0 * 50.0); }
            if dr.alert < dr.alert_floor { dr.alert = dr.alert_floor; }
            drop(_pc);
            { let _p = crate::prof::scope("drone: UPDATE msg (AI)"); dr.send(d, rng, frame, msg::UPDATE); }
            // The route's door node dispatched inside the state machine: resolve the node → door
            // binding now (`Drones` owns the doors; the state machine cannot) — `ReachedDestNode`
            // stood the door in +0xBA8 before `DSTATE_OpenDoor`'s ENTER ran [M 0x153358].
            if let Some(node) = dr.door_node.take() {
                if let Some(path) = dr.route.as_ref().map(|r| r.path) {
                    dr.hit_door = self.node_doors.as_ref().and_then(|m| m.get(&(path, node))).copied();
                    if let Some(i) = dr.hit_door { dr.hit_door_open = self.doors.list[i].state == crate::door::OPEN; }
                }
            }
            // The route's KICK node (fix-nav-remainder): resolve the node → ScriptPlayer binding
            // (`NDrone_InitKickNodes` 0x139ce0, built lazily below) and — on the node's first
            // crossing (the game's node flag 0x8000, `ReachedDestNode` 0x153358:141) — save the
            // return state and enter `DSTATE_KickObject` (0x82). Later crossings only re-bind.
            if let Some(node) = dr.kick_node.take() {
                // The route may be the patrol/mission one (`route`) or the combat repositioning
                // one (`croute`, GoToGoalPosition) — `LinkCreep_Handler` serves both.
                if let Some(path) = dr.route.as_ref().map(|r| r.path).or_else(|| dr.croute.as_ref().map(|r| r.path)) {
                    if self.node_kicks.is_none() {
                        let mut m: HashMap<(usize, u16), usize> = HashMap::new();
                        for (pi, p) in d.nav.paths.iter().enumerate() {
                            for (ni, n) in p.nodes.iter().enumerate() {
                                if n.flags & 4 == 0 { continue; }
                                let mut best: Option<(usize, f32)> = None;
                                for (si, sp) in self.scripts.players.iter().enumerate() {
                                    if sp.scripts[0] != 0x0600_05bb && sp.scripts[1] != 0x0600_05bb { continue; }
                                    let t = sp.matrix;
                                    let q = (t[3][0] - n.pos[0]).powi(2) + (t[3][1] - n.pos[1]).powi(2) + (t[3][2] - n.pos[2]).powi(2);
                                    if best.map_or(true, |(_, bq)| q < bq) { best = Some((si, q)); }
                                }
                                if let Some((si, _)) = best { m.insert((pi, ni as u16), si); }
                            }
                        }
                        self.node_kicks = Some(m);
                    }
                    dr.kick_sp = self.node_kicks.as_ref().and_then(|m| m.get(&(path, node))).copied();
                    if !self.kicked_nodes.contains(&(path, node)) {
                        self.kicked_nodes.insert((path, node)); // [M 0x153358:141 — set once, never cleared]
                        dr.door_return = dr.state;
                        dr.set_state(state::KICK_OBJECT);
                    }
                }
            }
            // The route's ally-wait node (flag 0x20) is RESOLVED but not acted on: the game parks
            // the drone in 0x21 while `switch_channels[0x99]` is off (`ReachedDestNode` 0x153358:85-94),
            // but enabling that here regressed 07000001's exit walk (the courtyard shoji never
            // opened — the parker stood between Mayhew and the door, fix_mission_live's
            // the_07000001_exit_works_end_to_end, green on master 0cda458). The game's ch-0x99
            // timing around that node is the missing fact — live check in the report. The outbox
            // stays (the resolution proves the node dispatch), the state entry waits for it.
            let _ = dr.ally_wait_node.take();
            // The kick request (`NDrone2_KickObject` 0x14aa00): the bound ScriptPlayer's script
            // unpauses (`SP_UnPause` 0x1dca58: `+0x2c = 1`, the current script's pause cleared).
            if dr.kick_attempt {
                dr.kick_attempt = false;
                if let Some(si) = dr.kick_sp {
                    if let Some(sp) = self.scripts.players.get_mut(si) {
                        sp.activated = true;
                        if let Some(s) = sp.info[sp.cur].as_mut() { s.delay = 0; }
                    }
                }
            }
            // The ENTER / timer-C activation request (`NDrone2_OpenDoor` 0x14aa38): only a CLOSED
            // /CLOSING door activates; the attempt stamps +0xBB0. `Door_Activate(door, drone, 0)`.
            if dr.door_attempt {
                dr.door_attempt = false;
                if let Some(i) = dr.hit_door {
                    if self.doors.list[i].state & 0x12 != 0 {
                        let mut evs: Vec<crate::door::SoundEvent> = Vec::new();
                        self.doors.activate(i, &[(dr.pos, 0.4, false)], false, &mut self.channels, frame, &mut evs);
                        dr.door_open_at = frame;
                        door_sounds.extend(evs.iter().map(|&(id, pos)| (id as u32, pos)));
                    }
                }
            }
            // `NDrone2_SeenAndAttacking` from the combat states (docs/systems/music.md: the gate posts
            // `Music_Event(2, 5)`; *suggested*: once per update, the port's states numbered as the game's).
            if dr.enabled && matches!(dr.state, state::SNIPER_AIM | state::SNIPER_FIRE | state::SNIPER_RELOAD | state::COMBAT | state::COMBAT_NO_MOVE
                | state::COMBAT_OUT_OF_RANGE | state::COMBAT_NO_SIGHT | state::COMBAT_NO_ROUTE | state::ALERT_TO_POSITION | state::GO_TO_GOAL
                | state::DRONE_STUCK | state::DRAW_WEAPON | state::AIM_STAND | state::AIM_STAND_FIRE | state::AIM_STAND_RELOAD | state::AIM_BACKOFF
                | state::AIM_CROUCH | state::AIM_CROUCH_FIRE | state::AIM_CROUCH_RELOAD | state::STEP_AIM_LEFT | state::STEP_AIM_RIGHT
                | state::STRAFE_AIM_LEFT | state::STRAFE_AIM_RIGHT | state::STRAFE_DODGE_LEFT | state::STRAFE_DODGE_RIGHT | state::ROLL_LEFT_CROUCH
                | state::ROLL_RIGHT_CROUCH | state::RUN_FOR_COVER | state::UNDER_COVER_INIT | state::UNDER_COVER_IDLE | state::UNDER_COVER_AIM
                | state::UNDER_COVER_FIRE | state::UNDER_COVER_RETURN | state::UNDER_COVER_LEAVE_NOW | state::UNDER_COVER_TYPE_CHANGE | state::BULLET_IMPACT)
                && dr.team == 1
                && self.attack_gate.seen_and_attacking(index, dr.seen, dr.unseen_ticks, dr.dist) { self.music.event(2, 5); }
            self.patrol_cooldown = dr.patrol_cooldown;
            let talks = std::mem::take(&mut dr.talks);
            if !talks.is_empty() { talk_queue.push((index, talks)); }
            self.used = std::mem::take(&mut dr.links_used);
            self.cover_rt = std::mem::take(&mut dr.cover_rt);
            // NDrone2_Steer: exponential turn toward the desired yaw, 0.1 per tick, with the anim's yaw offset added
            // around it (`ControlSTANDARD`): the body faces desired − offset.
            let y = dr.yaw + dr.yaw_offset;
            dr.yaw = y + angle_difference(y, dr.desired_yaw) * 0.1 - dr.yaw_offset;
            // Upper-body aim (`AnimObjectAimAt` 0x1bfe58 via `psiBuildMatrixPalette` 0x111498: the 0x180 aim flag +
            // the weapon gate — `AnimDatumGetWeaponInfo < 0` returns). The port aims only in the game's aim/fire
            // states with a weapon in hand, and clamps the target to ±0.9 (0x3f666666) on BOTH sides — the game sets
            // its give-up flag 0x4000 beyond either bound (0x1c0368), the old port clamped only the negative side,
            // which let the torso wind all the way around (the playtest's "spine flipped" patrol look).
            let want = if dr.anim.class == 2 && dr.aims_torso() { angle_difference(dr.yaw, dr.yaw_to_opp).clamp(-0.9, 0.9) } else { 0.0 };
            dr.aim_yaw = rotate_to_bearing(dr.aim_yaw, want, 0.2, crate::trig::PI);
            // Head tracking and the aim pitch (fix-ai-remainder; G-026). `NDrone2_DoTracking` 0x1479d8:
            // `NDrone2_AimTest` 0x147918 passing (the aim/fire states — the port's `aims_torso` gate) puts
            // the 0x180 aim flag up: `AnimObjectHeadTrack` 0x1bf618 clears the 0x10 track flag
            // (`&= 0xffffffef`), the stored head angles ease back to 0 (HeadTrack's bVar5 == 2 path), and
            // `AnimObjectAimAt` 0x1bfe58 aims the spine (bone 2) with yaw AND pitch — mode-0 clamps ±0.9
            // (0x3f666666 / 0xbf666666), smoothed at `SetAiming`'s 0.2 (0x1c6878, 0x3e4ccccd). With the aim
            // off, `NDrone2_DoHeadTracking` 0x1472c8 tracks the opponent (re-decided on `+0x358`, the
            // same-target re-decide returning before the Rand(5); the tracked point is the opponent's
            // tracked bone — the port stands in Bond's eye) or, with none, one Rand(10) stops the tracking
            // and the angles ease back. HeadTrack's clamps: yaw ±1.1, 0 beyond 2.6; pitch [-1.0, +0.6];
            // speed 0.14 alerted (`DCVars+0x50c > 0.66`) else 0.07 (0x1472c8). Both angles positive up.
            let aiming = dr.anim.class == 2 && dr.aims_torso();
            if aiming {
                let dy = player.eye[1] - (dr.pos[1] + 1.2);
                let want_p = (dy / dr.dist.max(0.001)).atan().clamp(-0.9, 0.9);
                dr.aim_pitch = rotate_to_bearing(dr.aim_pitch, want_p, 0.2, crate::trig::PI);
                if dr.tracking { let _ = rng.rand(10); dr.tracking = false; } // the stop draw (0x1472c8)
                dr.head_yaw = rotate_to_bearing(dr.head_yaw, 0.0, 0.07, crate::trig::PI);
                dr.head_pitch = rotate_to_bearing(dr.head_pitch, 0.0, 0.07, crate::trig::PI);
            } else if dr.los || dr.seen {
                // `los` (+0x228 & 2) not `seen` (& 4): `DroneVision_HaveOpponentSight` sets the seen bit
                // only past the reaction time, and the game's head tracking follows Bond from the first
                // sight pass (`DoHeadTrackingBehaviour`'s +0x228 & 8 path) — the pre-attack window.
                if !dr.tracking {
                    dr.tracking = true;
                    dr.track_recheck = frame + rng.rand(5) as u32 + 2; // (Rand(5)+2)·FR·FR_DIV
                } else if frame >= dr.track_recheck {
                    dr.track_recheck = frame + rng.rand(5) as u32 + 2;
                }
                let dy = player.eye[1] - (dr.pos[1] + 1.55);
                let want_y = angle_difference(dr.yaw, dr.yaw_to_opp);
                let want_y = if want_y.abs() > 2.6 { 0.0 } else { want_y.clamp(-1.1, 1.1) };
                let want_p = (dy / dr.dist.max(0.001)).atan().clamp(-1.0, 0.6);
                let sp = if dr.alert > 0.66 { 0.14 } else { 0.07 };
                dr.head_yaw = rotate_to_bearing(dr.head_yaw, want_y, sp, crate::trig::PI);
                dr.head_pitch = rotate_to_bearing(dr.head_pitch, want_p, sp, crate::trig::PI);
            } else {
                if dr.tracking { let _ = rng.rand(10); dr.tracking = false; } // the stop draw (0x1472c8)
                dr.head_yaw = rotate_to_bearing(dr.head_yaw, 0.0, 0.07, crate::trig::PI);
                dr.head_pitch = rotate_to_bearing(dr.head_pitch, 0.0, 0.07, crate::trig::PI);
            }
            if !dr.corpse { let _pf = crate::prof::scope("drone: handle_firing"); let b = { let lights = &mut self.lights; dr.handle_firing(d, rng, frame, player, (moving, first_moved), index, lights) }; if let Some(b) = b {
                // `DroneWeap_FireWeapon` 0x178f68:264 → `Bullet_init_casing`: the guard's shell eject,
                // right after `Bullet_init`'s spread draws [S] the eject draws run inline here instead
                // of on the weapon's +0x78-tick delay expiry; origin = the muzzle, rotation = the aim
                // direction (the game reads the weapon bone's world transform).
                let (muzzle, bdir, bweapon) = (b.pos, b.dir, b.weapon);
                if let Some(w) = d.weapons.records.get(bweapon as usize) {
                    let yaw = bdir[0].atan2(bdir[2]);
                    let pitch = bdir[1].clamp(-1.0, 1.0).asin();
                    self.casings.eject(&d.surfaces.map(|s| &s.ammo).unwrap_or(&Default::default()), w, muzzle, yaw, pitch, dr.pos[1], rng);
                }
                self.bullets.push(b);
            } }
            // `ControlSTANDARD`'s door check (0x1490f0:232-248): a door standing in +0xBA8, 60 ticks
            // after the last attempt (+0xBB0 + 60 < GameState): beyond 2.0 (Vec_Dist3D of the obj
            // positions) the drone forgets it; else `NDrone2_DoorIsOpen` false → `NDrone2_OpenDoor`.
            // (The old stand-in scanned for the nearest door within 2.0 — the game only ever checks
            // the door the route's door nodes or `FindElevatorDoor` stood in.)
            if dr.enabled && !dr.corpse && frame > dr.door_open_at + 60 {
                if let Some(i) = dr.hit_door {
                    let dp = self.doors.list[i].pos;
                    if (dr.pos[0] - dp[0]).powi(2) + (dr.pos[1] - dp[1]).powi(2) + (dr.pos[2] - dp[2]).powi(2) > 4.0 {
                        dr.hit_door = None;
                    } else if !dr.hit_door_open {
                        dr.door_attempt = true;
                    }
                }
            }
            if dr.door_attempt {
                dr.door_attempt = false;
                if let Some(i) = dr.hit_door {
                    if self.doors.list[i].state & 0x12 != 0 {
                        let mut evs: Vec<crate::door::SoundEvent> = Vec::new();
                        self.doors.activate(i, &[(dr.pos, 0.4, false)], false, &mut self.channels, frame, &mut evs);
                        dr.door_open_at = frame;
                        door_sounds.extend(evs.iter().map(|&(id, pos)| (id as u32, pos)));
                    }
                }
            }
            let _pa = crate::prof::scope("drone: update_anim");
            if let Some((next_ai, done_msg)) = dr.update_anim(d) {
                // `SetEndAIState`: the AI state is queued, then the message routed (which applies the queued state).
                dr.set_state(next_ai);
                if done_msg != 0 || next_ai != 0 { dr.send(d, rng, frame, done_msg); }
            }
            // `DroneAnim_EventFunc` datum 1 (the grenade-throw release): `DroneWeap_ThrowGrenade` creates a
            // projectile (WORKLOG 0080). The throw sound 0x5EF enters the pool.
            if std::mem::take(&mut dr.pending_grenade) {
                if let Some(g) = dr.throw_grenade(d, rng, index) {
                    if let Some(t) = d.sfx { let h = t.get(0x5ef); self.audio.play3d(h.id, g.pos, None); }
                    self.grenades.push(g);
                }
            }
            // `DroneAnim_EventFunc` 5 / 10 / 0xB (fired in `update_anim` just above) → the melee
            // `Bullet_init` (`DroneWeap_CloseCombatImpact` 0x1789c0), straight into the bullet list.
            if let Some(kind) = dr.pending_melee.take() {
                if let Some(b) = dr.close_combat_impact(d, kind, index) { self.bullets.push(b); }
            }
            // `DroneAnim_EventFunc` datum 2 → `DroneWeap_ChangeWeapon(drone+0x45)` (0x17a6e0's per-weapon
            // jump table, 0x2fb740, 55 entries). The port carries the two entries the SP states verify:
            // 1 (Surrender_Anim 0x1626a0: weapon 1, anim set 0xC, empty) and 6 (DrawWeapon 0x166000's
            // EXIT: weapon 6, anim set 4, 16 rounds); the rest of the table is not ported yet.
            if std::mem::take(&mut dr.pending_weapon_swap) {
                match dr.weapon_swap {
                    1 => { dr.weapon = 1; dr.anim_set = 0xc; dr.clip = 0; }
                    6 => { dr.weapon = 6; dr.anim_set = 4; dr.clip = 16; }
                    _ => {}
                }
            }
            // The anim impact event 6 (fired in `update_anim` just above): msg 0x20 to self —
            // `DSTATE_OpenDoor` and `DSTATE_KickObject` handle it (the GLOBAL fallback drops it
            // elsewhere); the case picks its own `Drone_SfxInfo` line (0x27 door / 0x28 kick).
            if std::mem::take(&mut dr.pending_door_msg) {
                // `DroneAnim_EventFunc` datum 6 delivers msg 0x20 to self for BOTH anims' impact
                // events; the state's case 0x20 picks the `Drone_SfxInfo` line — OpenDoor 0x27 (sfx
                // 1477) [M 0x1681f8], KickObject 0x28 (sfx 1478) [M 0x1684c0]. The narrow play
                // mirrors `foley` — `&mut self` is split by the loop's `list` borrow.
                let line = if dr.state == state::KICK_OBJECT { 0x28 } else { 0x27 };
                if let Some(l) = Self::foley_entry(d.voices, &mut self.voice_last, line, frame) {
                    match d.sfx {
                        Some(t) => { let h = t.get(l.sfx); self.audio.play3d(h.id, dr.pos, None); if !h.looped { self.sounds.push(Sound { id: h.id, pos: dr.pos, inner: h.inner, outer: h.outer, weight: h.weight }); } }
                        None => { self.audio.play3d(l.sfx, dr.pos, None); }
                    }
                }
                dr.send(d, rng, frame, msg::DOOR);
            }
            // The state arms' queued `Drone_SfxInfo` one-shots (fix-room-sound): the civilian
            // patrol's female scream 0x48 (sfx 924, repeat 300) [M 0x15d610 disasm 0x15d754-a4].
            // Same narrow play as the drain above.
            for req in std::mem::take(&mut dr.foley_reqs) {
                if let Some(l) = Self::foley_entry(d.voices, &mut self.voice_last, req, frame) {
                    match d.sfx {
                        Some(t) => { let h = t.get(l.sfx); self.audio.play3d(h.id, dr.pos, None); if !h.looped { self.sounds.push(Sound { id: h.id, pos: dr.pos, inner: h.inner, outer: h.outer, weight: h.weight }); } }
                        None => { self.audio.play3d(l.sfx, dr.pos, None); }
                    }
                }
            }
            if let Some(c) = dr.raise.take() { self.channels.set(c as u32, 1, frame); }
            // `DroneFunc_OnInitDeath`'s verdict: the FailLabel event + the fail channel [M 0x13e560
            // applied per 0x145bd0's reasons; fix-actors]. The monitor's standing hooks (ch 0x60/0x61
            // → the label + ch 99) take it from here [M 0x1827d8:98-107].
            if let Some(r) = dr.fail_reason.take() {
                if r.label != 0 {
                    self.mission.fail_label = r.label;
                    self.mission_events.push(crate::mission::Event::FailLabel(r.label));
                }
                if r.channel != 0 { self.channels.set(r.channel as u32, 1, frame); }
            }
            // The animation's sounds (`Sound_Play3D` at the drone) enter the hearing list for the next tick.
            let (snd, at) = (std::mem::take(&mut dr.anim_sounds), dr.pos);
            // The script's footstep: the floor under the drone (alertness 0: not heard by the drones).
            if std::mem::take(&mut dr.footstep) {
                let feet = dr.feet();
                if let (Some(s), Some((t, _, m))) = (d.surfaces, d.world.segment_surface([feet[0], feet[1] + 0.5, feet[2]], [feet[0], feet[1] - 1.0, feet[2]], true)) {
                    if let Some(id) = nf_formats::sfx::SurfaceSounds::get(&s.footstep, m) { self.audio.play3d(id, [feet[0], feet[1] + 0.5 - 1.5 * t, feet[2]], None); }
                }
            }
            for id in std::mem::take(&mut dr.anim_stops) { self.audio.stop_all_with_id(id); }
            if let Some(t) = d.sfx { for id in snd { let h = t.get(id); self.audio.play3d(h.id, at, None); if !h.looped { self.sounds.push(Sound { id: h.id, pos: at, inner: h.inner, outer: h.outer, weight: h.weight }); } } }
            for mut p in dr.drops.drain(..) { p.from = Some(index); self.pickups.push(p); }
            // `Drone_CollisionHandler` (0x140968, WORKLOG 0078): the feet-sphere down-ray + gravity, before the
            // AI-boundary push-out (the game's `AINetwork_GetBoundsPushVectorForSphere` runs last in the handler).
            { let _p = crate::prof::scope("drone: collision_handler"); dr.collision_handler(d.world); }
            // `NDrone2_Collision`: the feet sphere (r 0.4) is pushed out of the AI-network boundaries.
            if dr.enabled && dr.bounded && d.nav.bounds {
                let p = d.nav.push_out(dr.feet(), 0.4);
                dr.pos[0] = p[0];
                dr.pos[2] = p[2];
            }
        }
        drop(prof_loop);
        // The dynamic collision set, AFTER the walk (fix-script-start-tick): the game's
        // `Collide_Update` runs post-walk [M 0x1336f8:238], so Bond's next movement reads this
        // walk's script-object and door poses (the ride's `ground_motion` set).
        // fix-safe08 (merged after fix-script-start-tick: the players now create entities inside the walk, so
        // the drain runs after it): `Script_CreateEntity` 0x1d8968's obj_type-0xF0 arm — the game calls
        // `Pickup_Create` (0x18dda0) at the stream's posed position with rest = 1 (disasm
        // 0x1d8bcc-0x1d8c20). A reserved slot (built at load from the level's scripts) goes live;
        // a spawn without one (a script outside the pre-scan) pushes a fresh pickup — takeable,
        // with the by-hash renderer fallback.
        for sp in std::mem::take(&mut self.scripts.objects.pending_pickups) {
            let activate = |p: &mut crate::pickup::Pickup, m: nf_formats::anim::Mat4| {
                p.gone = false; p.hidden = false; p.state = 1; p.vel = [0.0; 3]; p.m = m;
            };
            if let Some(&k) = self.script_pickup_slots.get(&(sp.script, sp.stream)) {
                if let Some(p) = self.pickups.get_mut(k) {
                    if p.gone && p.from.is_none() {
                        activate(p, sp.matrix);
                        continue;
                    }
                }
            }
            // No reserved slot: build from the entity params; the bounds from the entity's hash
            // (the game's glist comes from `hashtable_hashcode_to_celglist(hash)`, 0x1d8bcc).
            // Kinds 7-11 resolve through `Pickup_CreateFromSet` like every other create [M 0x18dda0:104-105].
            if (7..=11).contains(&sp.kind) {
                if let Some(mut p) = crate::pickup::Pickup::create_from_set(
                    [sp.matrix[3][0], sp.matrix[3][1], sp.matrix[3][2]], [0.0; 3], sp.kind, d.weapons, &d.models) {
                    p.m = sp.matrix;
                    self.pickups.push(p);
                }
                continue;
            }
            let bounds = d.models.get(&sp.hash).copied().unwrap_or(crate::pickup::Model { centre: [0.0; 3], radius: 0.3 });
            self.pickups.push(crate::pickup::Pickup {
                kind: sp.kind, weapon: sp.id, rounds: sp.rounds, model: sp.hash,
                bounds,
                m: sp.matrix, start: [sp.matrix[3][0], sp.matrix[3][1], sp.matrix[3][2]],
                vel: [0.0; 3], state: 1, lifetime: 0, switch_channel: sp.switch_ch,
                sound: if sp.sound == 0 || sp.sound == 0xFFFF { None } else { Some(sp.sound) },
                hidden: false, gone: false, from: None, obj: None,
                respawn: 0, respawn_ticks: 0, spin: sp.kind == crate::pickup::kind::BOND_BONUS,
            });
        }
        { let _p = crate::prof::scope("drones: set_dynamic"); d.world.set_dynamic(&script_parts(self, d)); }
        // The drones' speech requests, in update order (no random draws in `SpeechSFX` / `PlaySFX`).
        for (index, talks) in talk_queue { self.speech(d, index, talks, frame); }
        // The drones' door-open sounds (`NDrone2_OpenDoor`'s `Door_Activate` events).
        for (id, pos) in door_sounds {
            match d.sfx {
                Some(t) => { let h = t.get(id); self.play_sound_handle(pos, h); }
                None => { self.audio.play3d(id, pos, None); }
            }
        }
        // `Doors_Calc` — the control pass's last act (0x1336f8's tail): the door groups sync, the busy markers
        // clear. (`switch_channels_prev` is snapshotted after `Trigger_Calc`, at the top of the tick — see above.)
        self.doors.calc();
        // `NF_DOORLOG=1` per-tick door dump (Task A-8-2): placement, kind, channels, flags, glist,
        // whether the glist is in the collision world's `shapes` table (else bullets pass through),
        // state, pos01, open_timer, bounding sphere.
        self.doors.log_doors(Some(d.world));
        // `NF_DOORLOG=1` throttled (every 60 ticks) door dump (Task A-9-2): the four values
        // that decide whether the auto-close path can fire — state, pos01, open_timer —
        // plus the keep_open bit and whether any actor in `actors` blocks the proximity
        // check (`dist² < (r1+r2)²·2`). `actors` is the same list `doors.update` saw this
        // tick (player + live drones), so the `blocked` column matches the live decision.
        // Use this to see why a door stays OPEN in play: the unit tests park Bond far
        // outside the blocking radius, but in play Bond typically stands inside it.
        self.doors.log_doors_throttled(Some(d.world), &actors, frame);
        // Bullets: move, collide with the world and Bond (`Bullet_update` + `Collide_Update`), damage
        // (`Player_HandlePain`); the hit flash fades by 1 per tick (HUD). `Bullet_DoTrails` fly-by (sound 0x1dc) is
        // checked inside `Bullet::update` in the game's RNG order (Rand_FRand → Rand_Rand(5)).
        self.impacts.clear();
        let vol = crate::bullet::PlayerVolume { feet: [player.pos[0], player.pos[1] - 0.4, player.pos[2]], height: 1.8, radius: 0.25 };
        let viewer = player.pos;
        for b in &mut self.bullets {
            let (from, dir) = (b.pos, b.dir);
            if let Some(ev) = b.update(d.world, &vol, viewer, &mut self.rng) {
                match ev {
                    crate::bullet::BulletEvent::World(p) => {
                        let end = [p[0] + dir[0] * 0.1, p[1] + dir[1] * 0.1, p[2] + dir[2] * 0.1];
                        if let (Some(s), Some((_, n, m))) = (d.surfaces, d.world.segment_surface(from, end, false)) {
                            if let Some(id) = nf_formats::sfx::SurfaceSounds::get(&s.impact, m) { self.audio.play3d(id, p, None); }
                            // `Bullet_CollisionHandler` 0x12ce80: the effects + ricochet. The shooter is a
                            // drone (not class 3), so `Effect_Bullet` draws its `Rand_Rand(2)` [M 0x1ce94c].
                            let flags70 = d.weapons.records.get(b.weapon as usize)
                                .map_or(0u32, |r| u32::from_le_bytes(r[0x70..0x74].try_into().unwrap()));
                            if let Some(nd) = self.effects.bullet_impact(&mut self.rng, d.surfaces, &mut self.gas, p, n, m, b.dir, flags70, b.bounces, false) {
                                if b.bounces < 3 {
                                    // The bullet continues on the reflected direction [M 0x12d7a8..0x12d7f0].
                                    b.state = 1;
                                    b.pos = p;
                                    b.dir = nd;
                                    b.bounces += 1;
                                }
                            }
                        }
                    }
                    crate::bullet::BulletEvent::Player { damage, at } => {
                        let before = self.health.health;
                        // `Player_DealWithObjHit` 0x194458:201-226: the direction bits from the ATTACKER's position
                        // (HITDATA+0x30 = the shooter) in the viewer's frame.
                        let attacker = self.list.get(b.owner).filter(|d| d.present).map_or(at, |d| d.pos);
                        let yaw = player.aim[2].atan2(player.aim[0]);
                        self.health.pain_from(damage, self.difficulty, d.level, attacker, player.eye, yaw);
                        self.player_hits.push((frame, b.owner, before - self.health.health));
                    }
                    // `Bullet_DoTrails` fly-by: `Sound_Play3D(0x1dc, closest_point, 100, -1, -1, …, custom 0)` [M].
                    crate::bullet::BulletEvent::FlyBy(p) => { self.audio.play3d(0x1dc, p, None); self.sounds.push(Sound { id: 0x1dc, pos: p, inner: 0.0, outer: 0.0, weight: 0.0 }); }
                }
                self.impacts.push(ev);
            }
        }
        self.bullets.retain(|b| b.state < 2);
        // Grenades (`Bullet_update` + `Bullet_CollisionHandler`, WORKLOG 0080): move under gravity, bounce off the
        // world, explode after the fuse (`Bullet_handle_object_destruction` 0x12cd98: `+0x68 & 0x4000` and
        // `+0x70 & 0x80204`; a bounce NEVER explodes [M 0x12ce80]). fix-explosions: the blast is `Explode_Create`
        // 0x17cbd0 (radius = weapon+0x08, damage = weapon+0x0C — see `explode.rs`), the effect script 0x6000052
        // plays via the one-shot player and ITS Sound(0x3E) is the blast sound [D script stream 36]; the old
        // port's 0x4A7 was `Bullet_CollisionHandler`'s rest-impact sound of weapons 0x3B/0x45, not the blast.
        // fix-explosions2: the death branch routes on the weapon FLAGS, not the id — `Bullet_DoTrails`
        // 0x12d890's timer death [asm 0x12dd2c-0x12dd68]: `+0x70 & 0x2000` → the FLASH branch (only the stun
        // grenade 0x35 among the grenades carries the bit [L sweep of all 115 records: 0x2280; 0x59/0x5b are
        // the non-grenade carriers]); else `+0x70 & 0x20000` → the SMOKE branch (0x36 [L 0x20280]: first
        // expiry `Occlude_Create` + Sound 0x14 + the fuse re-arms to 1200 ticks [M 0x12e114-0x12e1bc], second
        // expiry dies); else → `Explode_Create(wd+8, wd+8, wd+0xC)` [M 0x12e1c0] — the frag 0x34 (8/100),
        // the mine 0x37 (8/100), the satchel family 0x3b-0x40 (8/2000). fix-explosions routed the flash on
        // id 0x36 — the disc labels name 0x35 'Stun Grenade' and 0x36 'Smoke Grenade' [L USATxt.dat].
        self.explosions.clear();
        for g in &mut self.grenades {
            while let Some(ev) = g.update(d.world, &mut self.rng) {
                match ev {
                    crate::grenade::GrenadeEvent::Bounce(p) => {
                        if let Some(t) = d.sfx { let h = t.get(0x51c); self.audio.play3d(h.id, p, None); }
                        // `DroneFunc_HandleExplosives` (0x1435f0): the kind-2 explosive entries — a guard with the
                        // fear props within the entry's radius that sees the object sends itself msg 0x21
                        // [M 0x1435f0:30-52]. The port triggers once at the bounce (the game rescans every frame
                        // while the object exists; documented in the report).
                        self.explosive_queue.push((p, g.weapon_type));
                        break;
                    }
                    crate::grenade::GrenadeEvent::Explode { pos, radius, damage, owner, weapon_type } => {
                        let flags70 = d.weapons.records.get(weapon_type as usize)
                            .map(|r| u32::from_le_bytes(r[0x70..0x74].try_into().unwrap()))
                            .unwrap_or(0);
                        if flags70 & 0x2000 != 0 {
                        // The FLASH branch (the stun grenade 0x35): `Bullet_DoTrails` 0x12d890 [M
                        // 0x12df58-0x12e130] — `Gas_Init(0.5, 1.68, …, fx 0x200001f, 0xB)`, `Camera_Shake(2.0)`,
                        // `Sound_Play3D(0x16)`, `Light_Create(10.0, 5.0, 0xff,0xff,0xff, 0, 10, 1, 0)`, then a
                        // 30-unit sphere from the blast [M 0x12dfb0]: Bond's head whitens out (dist 5→90, × the
                        // facing factor, ×0.5 without LOS → `Player_SetFlashBang` [M 0x12e014-0x12e0b8]) and the
                        // drones within 20 get msg 0x18 → `HandleImpact` case 0x18 → StunGrenadeImpact 0x4D [M
                        // 0x12e0c0, 0x1469b8 case 0x18]. NO HitDamage and NO 0x6000052 script anywhere on this
                        // path (no `Explode_Create` call — the white light and the gas puff are the visuals).
                        // The broadcast gate [M 0x12e094]: in SP (`MPSettings._388_4_ == 0`) only a PLAYER
                        // thrower (obj+0xFF == 3) broadcasts — the port is SP-only, so owner == usize::MAX.
                            self.health.flashbang = 1.0;
                            if let Some(t) = d.sfx { let h = t.get(0x16); self.audio.play3d(h.id, pos, None); }
                            self.lights.create(10.0, 5.0, pos, [0xff, 0xff, 0xff], 0, 10, 1, 0, &mut self.rng);
                            self.gas.init(pos, 0.0, [0.0; 3], 0.5, 1.68, 0xb, [0x00, 0x80, 0x80, 0x80], 30.0);
                            self.shake = self.shake.max(2.0);
                            if owner == usize::MAX {
                                for dr in self.list.iter_mut() {
                                    if !dr.present || dr.corpse || !dr.enabled { continue; }
                                    let dx = dr.pos[0] - pos[0]; let dy = dr.pos[1] - pos[1]; let dz = dr.pos[2] - pos[2];
                                    if dx * dx + dy * dy + dz * dz <= 400.0 {
                                        dr.hit_by_player = true;
                                        dr.stun_until = frame + 7200; // `DroneFunc_RecoverTime` [M 0x147188]
                                        dr.set_state(state::STUN_GRENADE_IMPACT);
                                    }
                                }
                            }
                            break;
                        }
                        if flags70 & 0x20000 != 0 {
                        // The SMOKE branch (0x36): first expiry — state back to 1, `BU+0xF6 = 1`, the fuse
                        // re-arms to 1200 ticks (0x44960000; 3600 for the 0x69 family), `Occlude_Create(pos,
                        // 1200, 1)` is the cloud and `Sound_Play3D(0x14)` pops it [M 0x12e114-0x12e1b4]; the
                        // second expiry falls through to the death [M 0x12e118 `bnel +0xF6, 0 → 0x12e1fc`].
                        // No damage, no whiteout, no script. The port has no occluder system — the cloud
                        // visual is missing (live-check item); the pop and the re-arm are the Sim half.
                            if !g.smoke_popped {
                                g.smoke_popped = true;
                                g.state = 1;
                                g.range += 1200.0;
                                if let Some(t) = d.sfx { let h = t.get(0x14); self.audio.play3d(h.id, pos, None); }
                            }
                            break;
                        }
                        // `Explode_Propagate` 0x17c518 through `explode::propagate`: the full 12-case
                        // dispatch (drones msg 9, Bond `Player_Hurt`, breakables, destroyables; the
                        // port has no copter/vehicle/MP/GT/sensor/monitor/sub lists yet). The frag
                        // 0x34 lands here with radius 8 / damage 100 [L record] — the real kill.
                        let drones: Vec<(usize, V3, f32)> = self.list.iter().enumerate()
                            .filter(|(_, x)| x.present && !x.corpse).map(|(i, x)| (i, x.pos, 0.4)).collect();
                        let hits = crate::explode::propagate(pos, radius, damage, owner,
                            &crate::explode::Targets { drones: &drones, player: player.pos, breaks: &[], destroys: &[] });
                        // Bond: `Player_Hurt` (the difficulty factor is applied in `explode_propagate`).
                        if let Some(ph) = hits.player {
                            let before = self.health.health;
                            // `Explode_Propagate` → `Player_Hurt` → `Player_HandlePain` (type 0): BL+0x967 = 0xC
                            // (both sides) [M 0x1902c8:124-130].
                            self.health.pain_generic(ph, self.difficulty, d.level, 0);
                            self.player_hits.push((frame, owner, before - self.health.health));
                        }
                        // Drones: `Drone_ExplosiveHit` → msg 9 (the port routes through `on_bullet` as an impact).
                        for (di, ddmg) in hits.drones {
                            if let Some(dr) = self.list.get_mut(di) {
                                dr.last_damage = ddmg;
                                // The owner's grenade: usize::MAX = the player's throw [M +0x2B4 = the
                                // thrower; fix-actors].
                                if owner == usize::MAX { dr.hit_by_player = true; }
                                dr.health -= ddmg * dr.damage_taken;
                                if dr.health <= 0.0 { dr.health = 0.0; dr.set_state(state::DEATH_ANIM); }
                                else { dr.send(d, &mut self.rng, frame, msg::BULLET); }
                            }
                        }
                        // The explosion visual: `Explode_Create` 0x17cbd0 Script_Loads the effect script
                        // (0x6000052 for every disc caller of the bullet path [M 0x12cd98]) at the blast,
                        // aligned to it, scale = radius·0.06666667 [M 0x17cbd0:84-106]. The port plays it as a
                        // one-shot script player; its entity streams draw through the scene's
                        // script-entity attachments. Missing from the level's lib → the player no-ops.
                        // The blast sound itself rides the script's own Sound(0x3E) request [D].
                        let mut bm = [[0.0f32; 4]; 4];
                        bm[0][0] = 1.0; bm[1][1] = 1.0; bm[2][2] = 1.0; bm[3] = [pos[0], pos[1], pos[2], 1.0];
                        self.scripts.spawn_once_scaled(usize::MAX, 0x0600_0052, bm, radius * 0.0666_6667);
                        // `Camera_Shake(pos, radius)` only from radius 0.5 [M 0x17cc28].
                        if radius >= 0.5 { self.shake = self.shake.max(radius); }
                        self.explosions.push((pos, radius, damage, owner, weapon_type));
                        break;
                    }
                }
            }
        }
        self.grenades.retain(|g| g.state < 2);
        // The HandleExplosives broadcast for the bounced explosives [M 0x1435f0 + 0x138b10]: classification by
        // the grenade's weapon record (the game classifies by the thrower's weapon id — the port's events carry
        // the record id; mapping documented in the report): frag 0x34 → radius 12.0, flee 4.0, deadline 2 s,
        // return Hide 0x87 [M 0x138b10:67-87 + 0x1431c0:101-119]; stun 0x35 → radius 8.0 [M 0x138b10:163-167],
        // flee 10.0, deadline 10 s, return to the caller state [M 0x1431c0:165-176]; smoke 0x36 → kind 1, no
        // flee [M 0x138b10:143-159 + 0x1435f0:24-29 — only kind-2 entries trigger]; satchel 0x3B-0x40 → radius
        // 12.0, flee 14.0 [M 0x138b10:113-141 + 0x1431c0:122-152]. Per drone: fear prop 0x3A/0x3B [M 0x1435f0:30-34],
        // within the radius, and the object visible [M 0x1435f0:37-44 ObjectVisibility ≥ 0.7 + CanSeeObject].
        let queued = std::mem::take(&mut self.explosive_queue);
        for (p, wt) in queued {
            let cls = match wt {
                0x34 => Some((12.0f32, 4.0f32, 120u32, 0x87u8)),
                0x35 => Some((8.0, 10.0, 600, 0)),
                0x3b..=0x40 => Some((12.0, 14.0, 600, 0x87)),
                _ => None,
            };
            let Some((radius, run, ticks, ret)) = cls else { continue };
            let Drones { list, rng, .. } = self;
            for dr in list.iter_mut() {
                if !dr.present || dr.corpse || !dr.enabled { continue; }
                let (dx, dy, dz) = (dr.pos[0] - p[0], dr.pos[1] - p[1], dr.pos[2] - p[2]);
                if dx * dx + dy * dy + dz * dz > radius * radius { continue; }
                if !dr.prop(d, 0x3a) && !dr.prop(d, 0x3b) { continue; }
                if !d.world.line_of_sight([dr.pos[0], dr.pos[1] + 1.0, dr.pos[2]], p) { continue; }
                dr.explosive_msg = Some((p, run, ret, frame + ticks));
                dr.send(d, rng, frame, msg::EXPLOSIVE);
            }
        }
        self.health.flash = (self.health.flash - 1.0).max(0.0);
        // The flashbang whiteout decays by its own BL amount per tick (FRAME_RATE · f · 10 = f/6 [M
        // Player_SetFlashBang 0x196040's first arg]); the exact fade is the view's Camera_SetFade.
        self.health.flashbang = (self.health.flashbang - self.health.flashbang / 6.0).max(0.0);
        self.shake = (self.shake * 0.9 - 0.05).max(0.0);
        // The SimpleScripts' playheads (`SS_Update` 0x1a9948 — channel on → the last key, off → key 0).
        self.sss.tick(&self.channels);
        // `Mission_Update` moved to the top of the tick (fix-objective-banner): the game's Game_Run
        // order — see the block before `fx.tick` above.
        // fix-progress: the tally folds — the per-drone `PlrStat_Log*` outboxes, `Trigger_Calc`'s
        // Bond-moment scan and the health snapshot (docs/fields/player-stats.md's writers table).
        // (Restored here by fix-objective-banner: the fold lived between the mission update and the
        // music update and was lost in the Game_Run-order rebase; it only reads the outboxes and
        // the mission status, so the end of the tick is still its place.)
        {
            let ms = self.mission.status();
            let (mut dispatched, mut disabled, mut surrendered, mut hit) = (0u32, 0u32, 0u32, 0u32);
            for dr in self.list.iter_mut() {
                // `PlrStat_LogEnemyDispatched` 0x1996a8 (OnInitDeath 0x145bd0:57-59),
                // `LogEnemyDisabled` 0x199730 (the KO enter 0x172ac8), `LogEnemySurrender`
                // 0x1997b8 (the Surrender_Anim ENTER 0x1626a0), `LogShotHitEnemy` 0x199840
                // (NDrone2_HitDamage 0x145558:57-60) — outboxed by the drones, folded here.
                if dr.dispatched_logged { dr.dispatched_logged = false; dispatched += 1; }
                if dr.disabled_logged { dr.disabled_logged = false; disabled += 1; }
                if dr.surrender_logged { dr.surrender_logged = false; surrendered += 1; }
                hit += dr.hit_log; dr.hit_log = 0;
            }
            for _ in 0..dispatched { self.progress.log_enemy_dispatched(ms); }
            for _ in 0..disabled { self.progress.log_enemy_disabled(ms); }
            for _ in 0..surrendered { self.progress.log_enemy_surrender(ms); }
            for _ in 0..hit { self.progress.log_shot_hit_enemy(ms); }
            // `Trigger_Calc` 0x1b1900:12-17: channels 0x65..0x6E on → `PlrStat_DoneBondMoment(0, ch)`
            // (the BondMoments gate makes repeats inert).
            for id in 0x65u32..=0x6E { if self.channels.on(id) { self.progress.done_bond_moment(ms, id); } }
            // `PlrStat_LogHealth` 0x199620 ← `Player_SetHealth` 0x1960b8:14: the snapshot GetScore's
            // row 6 reads; the port writes it once per tick (the game's last write wins too).
            self.progress.log_health(ms, self.health.health);
        }
        // `UpdateMusicalEvents` 0x114e70 — once per tick, after the drones (their script requests and the
        // mission's pulses are this tick's EventList) [M]. The audio layer feeds `iop_section`.
        self.prev_event2 = self.music.events[2];
        self.music.update(self.iop_section);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::CollisionWorld;

    /// A drone with only the collision-relevant fields set (pos, velocity, bounded, free_root, corpse), for the
    /// gravity / down-ray tests. The hip height is the default 1.0 (`Drone::hip`).
    fn drone_at(pos: V3) -> Drone {
        let dr = Drone {
            skin: 0, anim_set: 0, weapon: 0, pos, yaw: 0.0, desired_yaw: 0.0, aim_yaw: 0.0, alert: 0.0, alert_floor: 0.0,
            props: [0; 3], alert_props: None, sight_range: 0.0, half_fov: 0.0, engage_range: 0.0, combat_range: 0.0,
            accuracy: 0, aggression: 0, reaction: 0, los: false, seen: false, ever_seen: false, visibility: 0.0,
            seen_ticks: 0, unseen_ticks: 0, vision_on: false, dist: 0.0, yaw_to_opp: 0.0,
            state: 0, prev_state: 0, next_state: None, entered: 0, timer_c: None, timer_d: None,
            explosive_pos: pos, explosive_run_dist: 0.0,
            anim: Animator { danim: 0, script: None, time: 0, state: 0, class: 0, call: None, reached: false, call_from: 0, blend_wait: 0, ftime: 0.0, speed: 1.0, variant: 0, once: false, held: false, follow_on: 0, outgoing: Vec::new(), started: false, early_end: false, len_cap: None },
            firing: false, burst_done: false, burst: 0, next_shot: 0, last_shot: 0, clip: 0, clip_max: 0,
            grenade_weapon: 0x35, grenade_count: 0, pending_grenade: false, shots: Vec::new(),
            surrender_logged: false, hit_log: 0,
            log: Vec::new(), knocked_out: false, disabled_logged: false, dispatched_logged: false, start_script: 0, root_prev: None, aimed_at: false, aimed_until: 0, bunched: false, wobble: 0, lost_sight: false,
            damage_mul: 1.0, alert_bits: 0, light: 1.0, attack_frame: 0, alerted_by_drone: false, watching: false,
            route: None, last_wait: None, idle_timeout: None, delayed: None, sm_id: 0, stand_until: 0,
            croute: None, nav_path: None, move_quad: 0, yaw_offset: 0.0, no_route_odd: false, target: PlayerTarget::default(), links_used: Vec::new(),
            switch_channel: 0, channel_on: false, placement: 0, spawn_seq: 0, spawn_frame: 0, group: 0, present: true, started: false, drawn: true, enabled: true, index: 0, exiting_to: 0, self_msg: None,
            cover: None, cover_side: 0, cover_type: 0, cover_leave: 0, cover_point: pos, goal: None, cover_claimed: false, has_cover: false, cover_tries: 0,
            cover_rt: Vec::new(), others: Vec::new(), peers: Vec::new(), return_state: 0, second_channel: 0, second_tag: 0,
            loudness: 0.0, sound_share: 0.0, heard_frame: 0, heard_peak: 0.0, sound_pos: pos, noise_level: 0, investigating: false, turned: false, resume_state: 0, home: pos,
            health: 10.0, max_health: 10.0, damage_taken: 1.0, damageable: true, kind: 0,
            last_damage: 0.0, silent_kill: false, big_hit: false, flinch_at: None, hit: None, impact_resume: 0, hits_taken: 0,
            global_flinch: None, bond_aiming: false, cull: false, bond_armed: true, opp_facing: 0.0, opp_target: [0.0; 3], sighted: false, own_sight: false, first_attack_done: false,
            corpse: false, death_pending: false, hit_by_player: false, fail_reason: None, armed_peer_dist: f32::MAX, wait_channel: 0, wait_channel_on: false, route_blocked: false, death_channel: 0, special_death: 0, special_spot: pos, special_yaw: 0.0,
            mission_wait_channel: 0,
            stun_until: 0,
            stationary: false, alpha: 255.0, raise: None,
            velocity: [0.0; 3], in_air: false, on_surface: true,
            captain: false, snap: 0.0, event_20: false, event_21: false, holstered: false, anim_sounds: Vec::new(),
            voice_type: 0, female: false, attack_talked: false, voice: None, voice_finished: true, talks: Vec::new(), patrol_cooldown: 0, footstep: false, anim_stops: Vec::new(), drops: Vec::new(),
            free_root: false, bounded: true, dtype: 0, mode_normal: 0, mode_alert: 0, mode: 0, alt_mode: 0, team: 1, alt_state: 0, scared_radius: 20.0, screamed: false, scare_shouted: false, pending_shout: None, alert_block: 0.0,
            door_open_at: 0,
            hit_door: None, door_return: 0, door_node: None, door_attempt: false, pending_door_msg: false, hit_door_open: true,
            // fix-nav-remainder: the kick-node outbox + the ScriptPlayer binding (fields on
            // their own lines — the shared-constructor collisions, merge lesson 4).
            kick_sp: None, kick_node: None, ally_wait_node: None, kick_attempt: false, no_bunching: 0,
            // fix-room-sound: the foley one-shot outbox + the pain-grunt handle (own lines — merge lesson 4).
            foley_reqs: Vec::new(), pain_grunt: None,
            // fix-anim-events: the EventFunc outboxes (own lines — merge lesson 4).
            weapon_swap: 0, pending_melee: None, pending_weapon_swap: false, recoil_weapon: false,
            head_yaw: 0.0, head_pitch: 0.0, tracking: false, track_recheck: 0, aim_pitch: 0.0,
            // fix-cover-remainder: the scary-object flee family (own line — merge lesson 4).
            scared: false, flee_return: 0, flee_deadline: None, explosive_msg: None,
        };
        dr
    }

    /// A floor quad at y = 0 (two triangles, wound with the stored plane normal UP — the disc's
    /// convention: 7,246 floors up / 0 down; `Intersect_RayGeom` 0x10a8c8 skips dot(n,dir) >= 0).
    fn floor_world() -> CollisionWorld {
        CollisionWorld::from_triangles(vec![
            [[-5.0, 0.0, -5.0], [5.0, 0.0, 5.0], [5.0, 0.0, -5.0]],
            [[-5.0, 0.0, -5.0], [-5.0, 0.0, 5.0], [5.0, 0.0, 5.0]],
        ])
    }

    #[test]
    fn drone_stands_on_the_floor() {
        // A drone whose feet (pos.y − hip 1.0) are 0.1 above the floor at y=0 is on the surface: the down-ray hits
        // within 0.2 below the feet, velocity zeroes and pos snaps to floor + hip = 1.0.
        let w = floor_world();
        let mut dr = drone_at([0.0, 1.1, 0.0]);
        dr.collision_handler(&w);
        assert!((dr.pos[1] - 1.0).abs() < 1e-5, "snaps to floor+hip: {}", dr.pos[1]);
        assert!(dr.on_surface && !dr.in_air);
        assert!(dr.velocity[1].abs() < 1e-9);
    }

    #[test]
    fn drone_falls_under_gravity_off_a_ledge() {
        // No floor: the down-ray misses, gravity accumulates and the drone falls. After N ticks the velocity is
        // −9.8/60 · N and pos.y drops by velocity · 1/60 each tick (the WORKLOG 0041 gap — a guard dying at a ledge).
        let w = CollisionWorld::empty();
        let mut dr = drone_at([0.0, 5.0, 0.0]);
        for _ in 0..30 { dr.collision_handler(&w); }
        assert!(dr.in_air && !dr.on_surface, "still in the air");
        // velocity after 30 ticks = −9.8/60 · 30 = −4.9; pos dropped by sum of v·dt ≈ −2.45.
        assert!((dr.velocity[1] - (-9.8 / 60.0 * 30.0)).abs() < 1e-5, "v: {}", dr.velocity[1]);
        assert!(dr.pos[1] < 5.0, "fell: {}", dr.pos[1]);
    }

    #[test]
    fn free_root_bypasses_gravity() {
        // SpecialDeath's free root motion: the handler returns at once; velocity stays zero, pos unchanged.
        let w = CollisionWorld::empty();
        let mut dr = drone_at([0.0, 5.0, 0.0]);
        dr.free_root = true;
        dr.collision_handler(&w);
        assert!((dr.pos[1] - 5.0).abs() < 1e-9 && dr.velocity[1].abs() < 1e-9);
    }

    #[test]
    fn drone_just_below_the_floor_snap_threshold_is_in_air() {
        // The on-surface test: hit within 0.2 below the feet. A drone 0.3 above the floor (feet at −0.3 rel. to a
        // floor at hip−0.6 ray end) — construct a floor at y = 0.4 so the feet (pos.y − 1.0 = −0.6) are 1.0 above it:
        // on surface. Then move the floor down to y = −0.5: feet at 0.5, floor at −0.5 → gap 1.0 > 0.2 → in air.
        let w = CollisionWorld::from_triangles(vec![
            [[-5.0, -0.5, -5.0], [5.0, -0.5, 5.0], [5.0, -0.5, -5.0]],
            [[-5.0, -0.5, -5.0], [-5.0, -0.5, 5.0], [5.0, -0.5, 5.0]],
        ]);
        let mut dr = drone_at([0.0, 0.5, 0.0]); // feet at −0.5, floor at −0.5 → exactly on it
        dr.collision_handler(&w);
        assert!(dr.on_surface, "on the floor");
        // Sister: floor 1.0 below the feet → in air (the 0.2 threshold fails).
        let w2 = CollisionWorld::from_triangles(vec![
            [[-5.0, -1.5, -5.0], [5.0, -1.5, -5.0], [5.0, -1.5, 5.0]],
            [[-5.0, -1.5, -5.0], [5.0, -1.5, 5.0], [-5.0, -1.5, 5.0]],
        ]);
        let mut dr2 = drone_at([0.0, 0.5, 0.0]); // feet at −0.5, floor at −1.5 → gap 1.0 > 0.2
        dr2.collision_handler(&w2);
        assert!(dr2.in_air && !dr2.on_surface, "in the air");
    }
}

#[cfg(test)]
mod foley_tests {
    use super::*;

    /// `NDrone2_PlaySFX`'s gate + `Drone_SfxInfo` stamp (`foley_entry`, 0x141848): the 30-frame post-load gate,
    /// the entry's `repeatDelay` enforced from the table's `lastFrame` (stamped **before** the play — the rate
    /// limit holds even when the play itself fails [M 0x141848 disasm 0x141a1c]), and a zero-sfxId entry is
    /// silent (entry 0x23 [L ee_snapshot.bin: sfxId 0]).
    #[test]
    fn foley_gate_repeat_delay_and_stamp_before_play() {
        let lines = vec![
            nf_formats::sfx::VoiceLine { sfx: 1477, facial: 0, repeat: 0 },     // the door line (idx 0x27)
            nf_formats::sfx::VoiceLine { sfx: 0, facial: 0, repeat: 180 },      // the disabled surrender line
            nf_formats::sfx::VoiceLine { sfx: 924, facial: 0, repeat: 300 },    // a cooldown line
        ];
        let mut last: Vec<Option<u32>> = Vec::new();
        // Inside the first 30 frames nothing plays and nothing is stamped.
        assert_eq!(Drones::foley_entry(Some(&lines), &mut last, 0, 29), None);
        assert!(last.iter().all(|t| t.is_none()), "no entry stamped: {last:?}");
        // Frame 30: the door line plays and stamps frame 30 before the play.
        assert_eq!(Drones::foley_entry(Some(&lines), &mut last, 0, 30), Some(lines[0]));
        assert_eq!(last[0], Some(30));
        // A zero-sfxId entry (0x23) returns at the entry's first gate, before any stamp [M 0x141848: 0x1418ac].
        assert_eq!(Drones::foley_entry(Some(&lines), &mut last, 1, 40), None);
        assert_eq!(last[1], None);
        // The repeat delay gates a re-fire until lastFrame + repeat.
        let l2 = nf_formats::sfx::VoiceLine { sfx: 924, facial: 0, repeat: 300 };
        assert_eq!(Drones::foley_entry(Some(&lines), &mut last, 2, 100), Some(l2));
        assert_eq!(Drones::foley_entry(Some(&lines), &mut last, 2, 399), None, "inside lastFrame + 300");
        assert_eq!(Drones::foley_entry(Some(&lines), &mut last, 2, 400), Some(l2), "the gate opens on the tick");
        // Sister: no table → nothing (a level without a voice table stays silent).
        let mut none: Vec<Option<u32>> = Vec::new();
        assert_eq!(Drones::foley_entry(None, &mut none, 0, 100), None);
    }

    /// `BOT_handlePain`'s pools (fix-room-sound): the verdict/gender pick, the one `Rand_Rand(n)`
    /// draw before the play, and the line ids through the gate — pinned against the live
    /// `Drone_SfxInfo` entries [L the 0x2812d0 table: 0x14d→950, 0x152→944, 0x154→949, 0x157→945].
    #[test]
    fn bot_pain_pools_pick_by_verdict_and_gender_and_draw_once() {
        // The pick: death pool (no playing gate) 0x154/0x157 width 3, pain pool 0x14d width 5
        // (male) / 0x152 width 2 (female) [M disasm 0x1260c8-0x126168].
        assert_eq!(Drones::pain_pool(false, false), (0x14d, 5), "male pain");
        assert_eq!(Drones::pain_pool(true, false), (0x152, 2), "female pain");
        assert_eq!(Drones::pain_pool(false, true), (0x154, 3), "male death");
        assert_eq!(Drones::pain_pool(true, true), (0x157, 3), "female death");
        // The real pool entries as `VoiceLine`s at their indices (repeatDelay 0 on all of them,
        // so the shared gate rate-limits nothing; sfx values from the live table [L]).
        let mut lines: Vec<nf_formats::sfx::VoiceLine> = vec![nf_formats::sfx::VoiceLine { sfx: 0, facial: 0, repeat: 0 }; 0x15a];
        for (i, id) in [(0x14dusize, 950u32), (0x14e, 948), (0x14f, 943), (0x150, 596), (0x151, 138), (0x152, 944), (0x153, 465), (0x154, 949), (0x155, 947), (0x156, 139), (0x157, 945), (0x158, 581), (0x159, 466)] {
            lines[i] = nf_formats::sfx::VoiceLine { sfx: id, facial: 0, repeat: 0 };
        }
        // A deterministic draw stream: the id played is base + the one draw, gated like any
        // `NDrone2_PlaySFX` (the 30-frame post-load applies here too).
        let mut rng = Rng::new(0x1234_5678);
        let mut last: Vec<Option<u32>> = Vec::new();
        for frame in [30u32, 31, 32] {
            let (base, n) = Drones::pain_pool(false, false);
            let draw = rng.rand(n) as usize;
            let l = Drones::foley_entry(Some(&lines), &mut last, base + draw, frame).expect("the pool entry plays");
            assert!(l.sfx >= 943 && l.sfx <= 950 || l.sfx == 596 || l.sfx == 138, "a male pain-pool sfx, got {}", l.sfx);
            assert_eq!(last[base + draw], Some(frame), "the stamp is the played entry's");
        }
        // Sister: the death pool's ids differ from the pain pool's — 0x154..0x156, never 0x14d..0x151.
        let (base, n) = Drones::pain_pool(false, true);
        let draw = rng.rand(n) as usize;
        let l = Drones::foley_entry(Some(&lines), &mut last, base + draw, 40).expect("the death entry plays");
        assert!(l.sfx == 949 || l.sfx == 947 || l.sfx == 139, "a male death-pool sfx, got {}", l.sfx);
    }
}
