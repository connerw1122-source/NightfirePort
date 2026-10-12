//! The non-guard actors: allies (team 3), civilians/VIPs (teams 3/2), the protected-actor mission-fail
//! rules, and the ally follow/lead state machine the escort missions use.
//!
//! Sources: ACTION.ELF (the addresses inline). Port map: `docs/systems/actors.md`. The states live as
//! handlers called from `drone.rs`'s state dispatch; this file holds the rules and the new state logic so
//! the shared file keeps only small arms.

use crate::triggers::Channels;

/// The fail labels the game's `DroneFunc_SetMissionFailReason` writes [M 0x13e560].
pub const FAIL_CIVILIANS: u32 = 0x0400_0034; // "Killed Civilians"
pub const FAIL_COVER: u32 = 0x0400_0033; // "Cover Lost"
pub const FAIL_ALARM: u32 = 0x0400_0032; // "Alarm Raised"

/// What the mission layer must do this tick (the fail reason's verdict).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FailReason {
    /// `Mission_SetFailLabel` [M 0x13e560] — none when the reason's case does not set one.
    pub label: u32,
    /// The switch channel to raise: 0x60 (civilians), 99 (mission fail), or the alarm switch's channel.
    pub channel: u8,
}

/// `DroneFunc_SetMissionFailReason` (0x13e560) [M]:
/// - reasons 1/3/6/9 → FailLabel 0x4000034 + **ch 0x60**;
/// - reasons 2/4/5/7/8/10/11/12 → FailLabel 0x4000033, then the default arm: **ch 99**;
/// - any other reason → **ch 99** with no label change;
/// - reason 0x10 → the nearest alarm switch's channel written (the port: **ch 0x61** — the game looks the
///   channel up on the nearest `Switch_*` object, `Switch_GetNearest`/`Switch_GetSwitchChannel`; every
///   alarm switch on the disc writes 0x61 [S], docs/systems/triggers.md), with 0x4000032 when it is 0x61.
pub fn mission_fail_reason(reason: u32) -> FailReason {
    match reason {
        1 | 3 | 6 | 9 => FailReason { label: FAIL_CIVILIANS, channel: 0x60 },
        2 | 4 | 5 | 7 | 8 | 10 | 11 | 12 => FailReason { label: FAIL_COVER, channel: 99 },
        0x10 => FailReason { label: FAIL_ALARM, channel: 0x61 },
        _ => FailReason { label: 0, channel: 99 },
    }
}

/// Apply a fail reason to the channels + the mission layer's FailLabel. `set_label` mirrors
/// `Mission_SetFailLabel` (0 = no change) [M 0x13e560].
pub fn apply_fail_reason(reason: u32, ch: &mut Channels, frame: u32, fail_label: &mut u32) -> FailReason {
    let r = mission_fail_reason(reason);
    if r.label != 0 { *fail_label = r.label; }
    if r.channel != 0 { ch.set(r.channel as u32, 1, frame); }
    r
}

/// `DroneFunc_OnInitDeath`'s mission-fail half (0x145bd0) [M]: the killed drone's verdict.
/// `team` = Drone+0x44, `prop53`/`prop54` = behaviour properties 0x53/0x54 (the protected-actor flags),
/// `killed_by_player` = the killer (+0x2B4) was the player's bullet/explosion.
pub fn on_init_death(team: u8, prop53: bool, prop54: bool, killed_by_player: bool) -> Option<FailReason> {
    if team == 1 {
        // team 1: prop 0x54 → reason 2, else prop 0x53 → reason 4 — both "Cover Lost" + ch 99
        // [M 0x145bd0: the LAB_00145cc8/00145ce4 arms].
        if prop54 { return Some(mission_fail_reason(2)); }
        if prop53 { return Some(mission_fail_reason(4)); }
        None
    } else {
        // team 2/3 (civilians, VIPs, allies): prop 0x54 → a drone killer reasons 1, a player killer
        // reasons 3; else prop 0x53 → reason 3 — "Killed Civilians" + ch 0x60 [M the LAB_00145c3c/
        // 00145c58 arms]. bVar7 = the killer is class 3 (a drone) [M].
        if prop54 {
            return Some(mission_fail_reason(if killed_by_player { 3 } else { 1 }));
        }
        if prop53 { return Some(mission_fail_reason(3)); }
        None
    }
}

/// The ally states the port dispatches here (0x1E/0x1F/0x21/0x22/0x24–0x28 — the AllyLead sub-states
/// fix-mission left as stand-ins, and the AllyFollow chain). The transitions below cite each state's
/// decompile; the shared match arms call `ally_state`.
pub mod states {
    /// AllyLeadPlayerInWay 0x1E (`NDrone2_DSTATE_AllyLeadPlayerInWay` 0x15ea28) [M]: Bond is blocking
    /// the lead path. ENTER: alert status 2, anim 0x21 (alert idle). UPDATE: no armed drone within 10
    /// (0x41200000) and Bond NOT within 1.06 rad (0x3f860a92) of the drone's path bearing → back to
    /// Lead 0x1D; Bond in the way again → hold (the game nudges him aside through the path creep).
    pub const PLAYER_IN_WAY: u8 = 0x1e;
    /// AllyLeadHide 0x1F (0x15ec08) [M]: combat near the escort. ENTER: alert status 2, anim 0x19 (the
    /// cover cower), +0xCD8 = 0. UPDATE: once no armed drone within 14.0 (0x41600000) → latch +0xCD8 = 1
    /// and timer C in 2 ticks → the timer returns to Lead 0x1D.
    pub const LEAD_HIDE: u8 = 0x1f;
    /// AllyLeadMissionWait 0x21 (0x15efc8) [M]: the route's scripted wait. ENTER: the wait channel =
    /// obj+0x12C (the placement's param 6, the drone's second channel; 0 → 0x99), MoveTo the route node,
    /// anim 0x21. UPDATE: the wait channel on → Lead 0x1D.
    pub const LEAD_MISSION_WAIT: u8 = 0x21;
    /// AllyLeadBondCombat 0x22 (0x15f120) [M]: Bond is fighting near the escort. ENTER: alert status 2,
    /// anim 0x21. UPDATE: no armed drone within 10 → `ObjectMissionPathDistance(5.0)` to Bond; negative
    /// (no route) → stay; else → Lead 0x1D. With an armed drone within 10 → FindRunToPoint → the cover
    /// run (the port: stand-fast — the cover nodes are the fight's business, not the escort's).
    pub const LEAD_BOND_COMBAT: u8 = 0x22;
    /// AllyFollowInit 0x24 (0x15f3c8) [M]: ENTER: alert status 1, anim 0x21. UPDATE: Bond visible
    /// (+0x228 & 4) → AllyFollow 0x25.
    pub const FOLLOW_INIT: u8 = 0x24;
    /// AllyFollow 0x25 (0x15f4e0) [M]: follow Bond at the mission band. The port models the follow as
    /// the lead walk toward Bond when he is beyond the follow distance, the wait idle inside it.
    pub const FOLLOW: u8 = 0x25;
    /// AllyFollowWait 0x26 (0x15f6a0) [M]: Bond far → the follow resumes through the wait's exit.
    pub const FOLLOW_WAIT: u8 = 0x26;
    /// AllyFollowDone 0x27 (0x15f858) [M]: the follow route finished — stand at the goal.
    pub const FOLLOW_DONE: u8 = 0x27;
    /// AllyGoToGoalPosition 0x28 (0x15f938) [M]: walk to the saved goal position (+0x970), then the
    /// state saved at +0x5A6 (the port: Lead 0x1D — the escort's only user).
    pub const GO_TO_GOAL: u8 = 0x28;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch() -> Channels { Channels::default() }

    #[test]
    fn fail_reasons_match_the_switch() {
        // [M 0x13e560] the reason groups.
        let r = mission_fail_reason(1);
        assert_eq!((r.label, r.channel), (FAIL_CIVILIANS, 0x60));
        let r = mission_fail_reason(9);
        assert_eq!((r.label, r.channel), (FAIL_CIVILIANS, 0x60));
        let r = mission_fail_reason(4);
        assert_eq!((r.label, r.channel), (FAIL_COVER, 99));
        let r = mission_fail_reason(0x10);
        assert_eq!((r.label, r.channel), (FAIL_ALARM, 0x61));
        let r = mission_fail_reason(0xf); // Zoe's abandonment reason [M 0x14b058]
        assert_eq!((r.label, r.channel), (0, 99));
    }

    #[test]
    fn protected_actor_deaths_fail_like_the_game() {
        // [M 0x145bd0]: team 1 with the props → Cover Lost (2/4); team 2/3 → Killed Civilians (1/3).
        assert!(on_init_death(1, false, false, true).is_none(), "an ordinary guard's death never fails");
        let r = on_init_death(1, true, false, true).unwrap();
        assert_eq!((r.label, r.channel), (FAIL_COVER, 99));
        let r = on_init_death(1, false, true, false).unwrap();
        assert_eq!((r.label, r.channel), (FAIL_COVER, 99));
        let r = on_init_death(3, true, false, true).unwrap();
        assert_eq!((r.label, r.channel), (FAIL_CIVILIANS, 0x60));
        let r = on_init_death(2, false, true, false).unwrap(); // a drone killed the VIP
        assert_eq!((r.label, r.channel), (FAIL_CIVILIANS, 0x60));
        assert!(on_init_death(3, false, false, true).is_none(), "an unprotected ally's death does not fail");
    }

    #[test]
    fn apply_writes_the_channel_and_label() {
        let mut ch = ch();
        let mut label = 0u32;
        let r = apply_fail_reason(3, &mut ch, 7, &mut label);
        assert_eq!(r.channel, 0x60);
        assert!(ch.on(0x60));
        assert_eq!(label, FAIL_CIVILIANS);
        // The channel write stamped the time (the monitor's AND reads the live channel [M 0x1827d8]).
        assert_eq!(ch.time[0x60], 7);
    }
}
