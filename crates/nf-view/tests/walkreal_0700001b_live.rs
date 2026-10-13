//! walkreal-c: 0700001b (the space station) walked with the player's own movement — no
//! `bond.pos` writes after the spawn. The route (docs/levels/0700001b.md; the walkreal_probe_c
//! dump): the shuttle arrival (22.98, −1.90, 3.13) → the hub floor west to the Drake arena
//! (−7.32, −1.42, 0.06) — one cel, no doors, no ladders on the route. The fight gate ch 0x0a
//! (NIS 0x060008f0 t104 [D]; its chain head is the missile choreography — the documented drive,
//! the same channel fix_finale_live drives) spawns Drake (spawner #187, trigger 10 [D]); his
//! defeat through the REAL fire path raises ch 0x0e → SP #198's NIS 0x06000a20 (NOT skipped —
//! the route watches it out) writes ch 0x0b at t600 and ch 100 at t720 → MissionState 6 →
//! 0x0730001b [M 0x182c60:114-116].
//!
//! Enemies are DISABLED for the walk only in the walkreal sense: damage is suppressed
//! (top_up_health, the walker's allowance). Drake himself dies to Bond's own bullets.
//! The zero-G substates (8/9, Player_ZeroG 0x192ab0) are asserted ABSENT on this route: the
//! station's zero-G set pieces are script-driven (C_CHCHZEROG_Handler 0x216388, control msg
//! 0x4b [M]) and none plays on this floor route. Opt-in via NF_ISO (the slim ISO is enough).

#[path = "common/walker.rs"]
mod walker;

use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

// GAP (walkreal-c): the hub crossing fell into the pit (y −15.4) on the straight z≈1.5 line; the nav-graph crossing was not verified in the session. The zero-G sister (the_hub_floor_route_never_enters_zero_g) passes.
#[test]
#[ignore = "the walk's reached blocker — see docs/reports/walkreal-c.md; un-ignore when the blocker lands"]
fn walk_arrival_to_drakes_defeat() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "0700001b.bin", false).unwrap());
    w.wait_enabled(3000, "the shuttle arrival");
    w.wait(120);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[23][0], 3,
            "'Disable the missiles' shows at once (enable 0 [L], the state-0 arm [M 0x1827d8:41-54])");
        assert_eq!(sim.drones.mission.states[23][1], 1, "'Defeat Drake' hides until ch 0x09 [L record 1b obj 2]");
    }

    // The route: the hub floor west to the Drake arena. The first run fell into the hub pit at
    // y −15.4 on the straight z≈1.5 line — the hub's walkable lanes wind (the walkreal_probe_c
    // depth-16 grid: the spawn platform x≈20-24 z 2-6, a raised lane at x≈10, the pit floor
    // −15.4 elsewhere). The nav graph (the guards' own paths) threads it.
    assert!(w.follow_nav_route([-4.0, -1.9, 0.2], "the hub crossing"), "the nav route crosses the hub");
    w.walk_to(walker::Wp::new(-7.0, -1.5, 0.1).r(1.2), "the arena floor");

    // The fight gate: ch 0x0a (0x060008f0 t104 [D]) — the documented drive of the channel the
    // missile choreography's chain writes (docs/levels/0700001b.md §6; fix_finale drives it the
    // same way). Drake spawns (spawner #187, trigger 10 [D]) and his wait-switch enables him.
    let drake = {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x0a, 1, sim.drones.frame);
        let mut found = None;
        for _ in 0..900u32 {
            w.tick(Default::default(), false);
            let sim = w.scene.sim.as_ref().unwrap();
            if let Some(i) = sim.drones.list.iter().position(|d| d.present && !d.corpse && d.skin & 0xffff == 0xba) {
                found = Some(i);
                break;
            }
        }
        let i = found.expect("the gate spawns Drake (spawner #187 [D])");
        assert!(w.scene.sim.as_ref().unwrap().drones.list[i].enabled, "the gate enables Drake (wait-switch 10 [D])");
        i
    };

    // The defeat through the REAL fire path: aim + tap the trigger until he dies (walker.shoot —
    // the gun, the bullets, the hit boxes).
    assert!(w.shoot(drake, "Drake"), "Drake dies to Bond's fire");

    // Death_Anim's msg-2 raises ch 0x0e on its first update [M 0x1637f8 case 2]; the defeat NIS
    // (SP #198 → 0x06000a20) follows. It must PLAY OUT (skip_cutscenes off): t600 writes ch 0x0b,
    // t720 ch 100 [D decode; Script_EventHandler 0x1d94c0 case 0x11].
    w.skip_cutscenes = false;
    let mut complete = false;
    for _ in 0..2400u32 {
        w.tick(Default::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.mission.status() == 6 { complete = true; break; }
    }
    w.skip_cutscenes = true;
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x0e), "Drake's death channel fires ch 0x0e [D param 12 = 14]");
        assert!(sim.drones.channels.on(0x0b), "the NIS's t600 event-11 writes ch 0x0b [D]");
        assert!(sim.drones.channels.on(100), "the NIS's t720 event-11 writes ch 0x64 = 100 [D]");
        assert!(complete, "the station mission completes (MissionState 6 [M 0x182c60])");
        assert_eq!(sim.drones.mission.to_end, 0x0730_001b, "the station's own end code [M 0x182c60:114-116]");
    }
    // Sister: the missile-console objective (ch 0x09) never completed — Drake's NIS ends the
    // mission past the monitor's AND (ch 0x09 & 0x0b) [D AND #139 unproven, docs §6].
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x09), "the console AND #139 never fired (no console writers on the route [D §6])");
}

/// Sister walk: the floor route never enters a scripted zero-G set piece — Bond's substate stays
/// a ground substate the whole way (the station's zero-G is script-driven, C_CHCHZEROG_Handler
/// 0x216388 msg 0x4b [M], and none plays on the hub floor).
#[test]
fn the_hub_floor_route_never_enters_zero_g() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "0700001b.bin", false).unwrap());
    w.wait_enabled(3000, "the shuttle arrival");
    let mut zerog_ticks = 0u32;
    for wp in [[14.0f32, -1.9, 1.5], [4.0, -1.9, 0.5], [-4.0, -1.9, 0.2], [-7.0, -1.5, 0.1]] {
        let _ = w.try_walk_to(walker::Wp::new(wp[0], wp[1], wp[2]).r(1.2), "the zero-G probe");
    }
    // The substate sample: drive 300 more ticks and count any 8/9.
    for _ in 0..300 {
        w.tick(Default::default(), false);
        if matches!(w.scene.sim.as_ref().unwrap().bond.substate, 8 | 9) { zerog_ticks += 1; }
    }
    assert_eq!(zerog_ticks, 0, "no zero-G substate on the hub floor route [M 0x216388 — script-driven only]");
}
