//! The mission-actor drone states the finale levels need (fix-finale-blockers).
//!
//! Everything here is read from the decompile (tags [M] with the function and address) or the EE
//! snapshot (tags [L]); the stand-ins are tagged [S] with the evidence that decided them.
//!
//! ## The KikoMission family (07000016's chaser states)
//!
//! `NDrone2_StateFuncs` (EE 0x29b300, 4-byte fn pointers): 0x18 → 0x172870 StandBlind, 0x19 →
//! 0x15df28 KikoMission, 0x1a → 0x15e0d8 KikoMissionRun, 0x89 → 0x1736f8 RunToAlarm [L].
//!
//! - **KikoMission 0x19** [M 0x15df28]: ENTER — vision on, `+0x4f8 |= 0x2000`, anim 0x34. UPDATE —
//!   on 0x07000016: `UpdateMissionRoute`; != 3 stay, == 3 → `behaviour_util_setProperty(0x1d,…,0)`
//!   → SetState(0x18). Off-level: `EnemyLookForOpponent` → seeing Bond → SetState(0x1a). Pain (6) /
//!   death (8) → `DroneFunc_HandleImpact(…,0)`; explosive-class (7/9/0x17/0x18/0x19) → (…,1);
//!   sound-class msgs (0x11–0x16, 0x1e, 0x1f) ignored on 0x07000016, else `EnemyAlerts` → 0x1a.
//! - **KikoMissionRun 0x1a** [M 0x15e0d8]: ENTER — vision on, anim 0x1e. UPDATE — route == 3 →
//!   SetState(0x89). Same impact arms.
//! - **StandBlind 0x18** [M 0x172870]: ENTER — anim 0x24 only.
//! - **RunToAlarm 0x89** [M 0x1736f8]: ENTER — vision on, alert 1, `FindAlarmPoint`: found →
//!   `+0x16=1, +0x5a2=0x8a, +0x5a0=0x56`, SetState(99 GoToGoalPosition); none → SetState(4 Idle).
//!   UPDATE — SeenAndAttacking.
//!
//! Entry: `NDrone2_DSTATE_CivilianMission` (0x15d860) ENTER `if (drone+0xd8 == 0x13) SetState(0x19)`
//! [M] — voice 0x13 = skin 0x0500005e (`NDrone2_DefaultInit` 0x14b300 sets +0xD8 from the skin [M];
//! the port's `voice_type`). No placement on the disc's finale levels reaches CivilianMission
//! (07000016's drones are dtypes 2/8/0xf/12 [D]) — the port keeps the transition for fidelity and
//! documents it as unreached.
//!
//! ## The chaser's defeat (ch 0x90)
//!
//! The objective "Prevent Kiko from launching into space" completes on ch 0x90; AND #271 [ch 0x90]
//! → ch 100 [D] → `Mission_Update` → 0x7300017 [M 0x182c60:111-112]. No writer of ch 0x90 exists
//! in the level's data: its 13 triggers (the only muxes are #264/#271/#286/#342 [D]), all 300+
//! scripts incl. the 07800016/07900016/07A00016 packs (event 0xD and the conditional 0x11 swept —
//! none write 144 [D]), the SP outs (#262→129, #263→130, #272→185, #347→80, #348→142 [D]) and no
//! drone's death-channel param 12 [D]. The code side: `DroneFunc_SetDeathChannel` 0x13e660 writes
//! `switch_channels[+0x13c]` (param 12; #261's is 0 [D]) and the full `switch_channels` writer
//! enumeration of the decompile (103 files) holds no 07000016 path to 144 [M].
//!
//! **The port's stand-in [S]**: on 0x07000016 the chaser — the skin-0x0500001e dtype-0xf drone at
//! the silo edge that is NOT in the death-ch 23..26 pit squad (#294–297 [D]; #261 is the extra one,
//! param 12 = 0 [D]) — carries death channel 0x90 at init. Killing her raises ch 0x90 exactly like
//! the game's defeat; every other writer path stays untouched.
//!
//! ## The ninja chain (0xa6/0xa7 …) — 07000004's assassin and 07000016's chaser
//!
//! dtype 0xf → `GetDTYPENEW` 0x14c938 case 0xf (mode 0) → 0xd; `DroneTypeSettings[13]` = {0xa6
//! NinjaStand, 0xa7, control 0x14b0f8} [L 0x29aee8+13·12]; alert ≥ 1 remaps 0xa6/0xa7 → 0xa7 [M
//! 0x14cbd8:128-155]. `NinjaStand 0xa6` (0x16b990) ENTER: ranges 3.0/5.0, the glowing-eyes datum,
//! then SetState(0xa7) unless the BLIND_DRONES cheat [M]. **NinjaAttack 0xa7** (0x16bb50) ENTER:
//! alert = 1.0, `+0x4F8 |= 0x80000000` (bounded); by distance `+0x1a0`: >12 → 0xa8, >6 → 0xa9,
//! >2 → 0xaa; < `+0xEC` (the melee band, 2.0 for mode 0xd [M 0x14b300:751]) → 10% sideflip (0xaf
//! via CanMoveToRelativePos ±3), else backflip test (0xae), else Rand(2): ≤2 → 0xac, else 0xb2.
//! UPDATE: SeenAndAttacking + SetAngleToObj + the same re-route; the close band backflips first,
//! then rolls the 10% sideflip [M]. 0xa8 LongRange (0x16bfc0): ENTER anim 0x67 + `DroneWeap_Fire(1)`;
//! UPDATE dist ≤ 12 → 0xa7, MoveToObject(2.0) arrived → 0xb2, moving → anim 0x67 (+0x2C8/rolls
//! evasion chain), no-route → 0xb3. 0xa9 MidRange (0x16c248): ENTER alert 1.0, TIMER_C at +60,
//! Fire(1); UPDATE 6<≤12 MoveToObject: moving → anim 0x67, arrived → 0xb2, no-route → 0xb3, else
//! 0xa7; TIMER_C seen & Rand(3) → 0xb2, then the somersault/sideflip alternatives. 0xaa ShortRange
//! (0x16c650): ENTER anim 0x6b, TIMER_C at +120+FRand(120), Fire(1), 1/3 → 0xab; EXIT +0x3B = 0;
//! UPDATE the same re-route with the +0x2C8 Rand(2) evasion chain, fire or 0xac ≤ 2; TIMER_C
//! Rand(2) → 0xab else the evasion chain. Death/pain → alive ? PainTalk : 0x42/0x44 [M].
//!
//! **The melee chain, ported as of this task** — the state-id ↔ function table is `NDrone2_StateFuncs`
//! (EE 0x29b300, 4-byte fn pointers) [L ee_snapshot: 0xab → 0x16cb70, 0xac → 0x16d100, 0xad →
//! 0x16d298, 0xae → 0x16d430, **0xaf → 0x173e08 Sideflip (the ±3 router)**, **0xb0 → 0x16cdd0
//! (anim 0x68)**, **0xb1 → 0x16cf68 (anim 0x69)**, 0xb2 → 0x16d5b8, **0xb3 → 0x16d878 NoRoute**]
//! — note `uncited-audit.md` gap ③ lists 0xaf/0xb0/0xb1/0xb3 shifted by one state; the table wins.
//! - **NinjaGetCloseToPlayer 0xab** (0x16cb70): ENTER anim 0x6b, TIMER_C at +180, Fire(1).
//!   UPDATE: ≤2 → 0xac; else MoveToObject(2.0): direct(2) with a clear 5.0 forward hop and
//!   5<dist<7 → 0xad somersault, moving → anim 0x67 + the fire gate, 10 → stay, else → 0xa7.
//!   TIMER_C → 0xaa.
//! - **NinjaSword 0xac** (0x16d100): ENTER ≤2 → anim 0x6d slash (done → 0xa7; the decompiler
//!   dropped CallAnim's blend 0 — the sibling states all pass next 0xa7/0xac and a stuck sword
//!   state would never exit), else → 0xa7. UPDATE: seen + angle. The slash's damage rides the
//!   anim's melee event (`DroneWeap_CloseCombatImpact`, already ported).
//! - **NinjaSomersault 0xad** (0x16d298): ENTER invalidate route, anim 0x6a (done → 0xac), Fire(1).
//! - **NinjaBackflip 0xae** (0x16d430): anim 0x66, variant Rand(3)!=0, done → 0xa7, Fire(1).
//! - **NinjaSideflip 0xaf** (0x173e08): ENTER-only router — ±3 clear → Rand(2) between 0xb0/0xb1,
//!   only +3 → 0xb0, only −3 → 0xb1, neither → 0xa7.
//! - **NinjaSideflipLeft 0xb0** (0x16cdd0): anim 0x68, variant Rand(3)==0, done → 0xa7, Fire(1);
//!   EXIT +0x3B = 0. **NinjaSideflipRight 0xb1** (0x16cf68): anim 0x69, variant Rand(3)!=0.
//! - **NinjaStandFire 0xb2** (0x16d5b8): ENTER anim 0x6b, TIMER_C at +180, Fire(1); UPDATE the
//!   somersault/sideflip alternatives after the first held tick, else re-fire; TIMER_C → 0xa7.
//! - **NinjaNoRoute 0xb3** (0x16d878): ENTER alert 1.0, TIMER_C at +30, anim 0x6b, Fire(1); UPDATE
//!   re-fire + angle + `+0x3B = (+0x228 >> 2) & 1`; TIMER_C MoveToObject(2.0): a route again (≤3)
//!   → 0xa7, else re-arm at +30. The star throws ARE `DroneWeap_Fire(1)` — the ninja's weapon.
//! The fight's CHANNEL effects — the assassin's death ch 0x54 on 07000004 [D], the chaser's 0x90
//! stand-in — are the mission's business and stay death-path driven.
//! `Drone+0x2C8` (the +0x2A-gated attack-hold counter, `NDrone2_PreDroneControl` 0x148db0:29-34)
//! is ported as `attack_hold`, incremented while the opponent is seen [S — the +0x2A writer is
//! untraced].
//!
//! ## The unported start states (fix-levels-sweep's table)
//!
//! - **0x85 EnemyRunToPoint** (0x168910) [M]: ENTER — vision on, alert 1; return state = 0x56, on
//!   levels 0x07000011/12/13/41 with the player not yet seen → 4 (alert < 0.66) / 5 (< 1.0);
//!   `FindRunToPoint` (0x153168): prop 0x56 → 0x89, else the closest AI_Point (the 0xe9
//!   placements, type by class byte +0x44: 1→0, 2→9, 3→2, else none); none/no-route →
//!   SetState(return) — **the mode-7 drones with no AI point provably stand** [M 0x168910:32-36].
//!   With a point → resume state saved, SetState(0x63 GoToGoalPosition) (dtype 7 rides the 0x63
//!   UPDATE arm's `if (dtype != 7) SetState(+0x5a2)` park [M 0x165640:63]). UPDATE —
//!   SeenAndAttacking only.
//! - **0xb4 DeleteMe** (0x173f00) [M]: ENTER — `NDrone2_Enable(0)` + `obj+0xfe |= 1`: a soft
//!   removal (invisible, skipped by triggers/hits; 07000007's k16=5 party placement [D]).
//!
//! The band re-dispatch arms yield to a pending transition (`next_state.is_none()`): the game's
//! damage messages run inside the state router (0x175778) so a same-tick death applies before the
//! next UPDATE, while the port's explosion damage sits outside `send` — without the guard the
//! router clobbered a pending death and left a zero-health ninja standing (fix-finale-blockers).
//! Death_Anim raises the death channel on its first UPDATE while `+0xCD8` is set, not the anim's
//! end (0x1637f8 case 2) — Drake's death danim never completes and still raises.

pub const KIKO_MISSION: u8 = 0x19;
pub const KIKO_MISSION_RUN: u8 = 0x1a;
pub const STAND_BLIND: u8 = 0x18;
pub const RUN_TO_ALARM: u8 = 0x89;
pub const NINJA_ATTACK: u8 = 0xa7;
pub const NINJA_LONG_RANGE: u8 = 0xa8;
pub const NINJA_MID_RANGE: u8 = 0xa9;
pub const NINJA_SHORT_RANGE: u8 = 0xaa;
pub const NINJA_GET_CLOSE: u8 = 0xab;
pub const NINJA_SWORD: u8 = 0xac;
pub const NINJA_SOMERSAULT: u8 = 0xad;
pub const NINJA_BACKFLIP: u8 = 0xae;
pub const NINJA_SIDEFLIP: u8 = 0xaf;
pub const NINJA_SIDEFLIP_LEFT: u8 = 0xb0;
pub const NINJA_SIDEFLIP_RIGHT: u8 = 0xb1;
pub const NINJA_STAND_FIRE: u8 = 0xb2;
pub const NINJA_NO_ROUTE: u8 = 0xb3;
pub const ENEMY_RUN_TO_POINT: u8 = 0x85;
pub const DELETE_ME: u8 = 0xb4;

/// Every ninja AI state the fight can reach (the state-id ↔ function table [L 0x29b300+st·4]).
pub const NINJA_FIGHT_STATES: [u8; 13] = [
    NINJA_ATTACK, NINJA_LONG_RANGE, NINJA_MID_RANGE, NINJA_SHORT_RANGE, NINJA_GET_CLOSE, NINJA_SWORD,
    NINJA_SOMERSAULT, NINJA_BACKFLIP, NINJA_SIDEFLIP, NINJA_SIDEFLIP_LEFT, NINJA_SIDEFLIP_RIGHT,
    NINJA_STAND_FIRE, NINJA_NO_ROUTE,
];

/// 07000016's chaser: skin 0x0500001e, dtype 0xf, not in the death-ch 23..26 squad (#294–297 [D]).
/// Her init death channel becomes 0x90 so her defeat raises the objective's channel (the stand-in
/// documented above) [S].
pub fn chaser_death_channel(level: u32, skin: u32, dtype: u8, death_channel: u8) -> u8 {
    if level == 0x0700_0016 && death_channel == 0 && dtype == 0xf && skin & 0xffff == 0x1e { 0x90 } else { death_channel }
}

/// The state arms for this module live in `drone.rs`'s state `match` (the Drone methods they call
/// are private to that impl); `drone.rs` cites this module's doc for every address. The band router
/// stays here as the one piece of shared logic.
/// NinjaAttack's ENTER/UPDATE distance router for the outside bands [M 0x16bb50]: >12 → 0xa8,
/// >6 → 0xa9, >2 → 0xaa. The ≤2 close band never routes here — `Drone::ninja_attack_route`
/// runs the game's inline evasion / sword / stand-fire choice.
pub fn ninja_band(dist: f32) -> u8 {
    if dist > 12.0 {
        NINJA_LONG_RANGE
    } else if dist > 6.0 {
        NINJA_MID_RANGE
    } else if dist > 2.0 {
        NINJA_SHORT_RANGE
    } else {
        NINJA_STAND_FIRE
    }
}
