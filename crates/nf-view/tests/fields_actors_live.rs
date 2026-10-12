//! 044y-fields-actors: the state-machine fixups the field map drove, driven through the real paths
//! (`nf_view::load` + `Drone::send`, the port's `Drone_SM_RouteMsgDCV` 0x171df8). Opt-in via NF_ISO.
//! Run: `NF_ISO=… cargo test -p nf-view --test fields_actors_live -- --nocapture --test-threads=1`
//!
//! Facts (each cited in the asserts; the map: docs/fields/drone-statemachine.md + docs/fields/drone.md):
//! - Every state change clears BOTH message timers C and D and resets the hit-effect multiplier
//!   `Drone+0x14C` to 1.0 [M-dis Drone_SM_RouteMsgDCV 0x17207c-88] — the port cleared only timer C.
//! - `AllyLeadMissionWait` (0x15efc8 case 1) latches its wait channel from `Drone+0x12C` (init 0 by
//!   `Drone_SM_InitObject` 0x175838, no writer in the decompile → the game always waits on 0x99), not
//!   from the second switch channel `+0x135`.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// A live 07000005 guard driven through two real `send` transitions: entering the sniper-fire state
/// arms timers C and D ([M 0x16a610 case 1: `timer_c = now+60`, `timer_d = now+360`]), and the next
/// state change must clear BOTH [M-dis 0x17207c-88]. Sister assertion: timer C is cleared (true
/// before the fix too — it proves the assertion mechanism sees a cleared timer).
#[test]
fn a_state_change_clears_both_message_timers() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..50 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let s = scene.sim.as_mut().unwrap();
    // Split borrows: the Data borrows the sim's tables; the drones keep the mutable side (safe —
    // disjoint fields of `s`, the same construction `Sim::data()` does).
    let drones = &mut s.drones;
    let d = nf_game::drone::Data {
        tables: &s.tables, props: &s.props, scripts: &s.scripts, weapons: &s.weapons,
        world: &s.world, level: s.level, sequences: &s.seqs, skins: &s.skins,
        skeletons: &s.skeletons, nav: &s.nav, covers: &s.covers, tuning: &s.tuning,
        hitboxes: &s.hitboxes, volumes: &s.volumes, sfx: s.sfx.as_ref(),
        models: &s.models, surfaces: s.surfaces.as_ref(), voices: s.voices.as_deref(),
    };
    let i = drones.list.iter().position(|x| x.present && !x.corpse).expect("a live guard exists");
    let dr = &mut drones.list[i];
    // Enter UnderCoverSniperFire through the real dispatch: SetState = the pending slot, delivered by
    // send() (msg 0 = the game's no-op message) — its ENTER arms timer C (+1 s) and timer D (+6 s).
    dr.next_state = Some(nf_game::drone::state::UNDER_COVER_SNIPER_FIRE);
    dr.send(&d, &mut drones.rng, drones.frame, 0);
    let dr = &mut drones.list[i];
    assert_eq!(dr.state, nf_game::drone::state::UNDER_COVER_SNIPER_FIRE, "the forced entry happened");
    assert!(dr.timer_c.is_some() && dr.timer_d.is_some(), "the sniper-fire enter armed both timers [M 0x16a610]");
    // The next state change: both timers clear [M-dis 0x17207c-88].
    dr.next_state = Some(nf_game::drone::state::IDLE);
    dr.send(&d, &mut drones.rng, drones.frame, 0);
    let dr = &drones.list[i];
    assert!(dr.timer_c.is_none(), "timer C clears on every state change [M-dis 0x17207c-88]");
    assert!(dr.timer_d.is_none(), "timer D clears with it [M-dis 0x17207c-88]");
    assert!((dr.damage_taken - 1.0).abs() < 1e-6, "the +0x14C hit-effect multiplier resets on every change [M-dis 0x17207c-88]");

    // AllyLeadMissionWait latches the wait channel from Drone+0x12C (0 → 0x99) [M 0x15efc8:17-20],
    // never from the second switch channel +0x135 (a different field — the old port source).
    let dr = &mut drones.list[i];
    dr.mission_wait_channel = 0x2f;
    dr.next_state = Some(nf_game::drone::state::LEAD_MISSION_WAIT);
    dr.send(&d, &mut drones.rng, drones.frame, 0);
    let dr = &drones.list[i];
    assert_eq!(dr.wait_channel, 0x2f, "the ENTER latches the +0x12C word [M 0x15efc8 case 1]");
    let dr = &mut drones.list[i];
    dr.mission_wait_channel = 0;
    dr.next_state = Some(nf_game::drone::state::LEAD_MISSION_WAIT);
    dr.send(&d, &mut drones.rng, drones.frame, 0);
    let dr = &drones.list[i];
    assert_eq!(dr.wait_channel, 0x99, "0 → the game's 0x99 default [M 0x15efc8 case 1]");
}
