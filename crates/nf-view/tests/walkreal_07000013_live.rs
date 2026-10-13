//! walkreal-b: 07000013 (HT Tower2 C, the tower lobby → the front entrance) walked with the
//! player's own movement. The warm chain (mission 0x07000011's parts): 07000012 WALKED (the
//! keypad burn, the console E, the shaft entrance — the shared `tower2` route, with its one
//! documented doorway stand-in) → its exit loads 0700004A, whose start #43 (4.26, −7.71, −4.76
//! [D]) sits INSIDE TouchOnce #277 (out 0x4D, y −9.87…−7.65 [D]) — Bond drops onto the touch at
//! the spawn (the game's own placement; no input writes) and "Escape through the elevator shaft"
//! completes — the carried RamSave arrives at 07000013 ([M 0x182270 keeps previous parts]) → the
//! spawn touches fire 0x28 and 0x4A ("Gain access to lobby" completes — the late AND) → the intro
//! ScriptPlayer #65 (0x060006D7 [D]) releases Bond at tick ≈351 → the REAL walk south through the
//! lobby to the front entrance → TouchOnce #156 (−8.61…−4.15, −368.81…−366.74, 65.09…67.16 [D])
//! writes ch 0x50 (the objective completes) → the success touches #73/#164/#165 write ch 0x64
//! (=100) and with every part's channel on (0x48, 0x49, 0x4D, 0x4A, 0x50) the monitor raises
//! MISSION SUCCESS [M 0x1827d8:119-123] → "Mission Complete" → LevelToEndTo 0x07300013 (the
//! driving interlude; the port logs and skips driving codes [M 0x182c60]) → E at the level's own
//! exit #72 (−12.79, −367.97, 68.25) requests 0x07300013 with its gate 0x4E off (the E arm falls
//! through [M 0x1b2318 case 10]; dead data — ch 0x4E has no writer on this disc [D]).
//!
//! The fake lift (SP #161, trigger ch 0x23) has no writer on the disc [D §4] — dead data; the
//! route out is the front entrance, not the lift.

#[path = "common/walker.rs"]
mod walker;
#[path = "common/tower2.rs"]
mod tower2;

use walker::{Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
#[ignore = "the front entrance's mouth is glazed shut for the capsule: every walked lane (x −6.2 and the slid west x −8.09) pins at z 64.47 against a south-facing wall at z ≈ 65.0 — the security gate, whose raise has no writer the port runs (the objective's cut-scene/channel is unported); cold segment rays at y −368.4 pass under the sill, so only the capsule-height band is blocked. Needs the game's raise path (gdbwatch in PCSX2 which channel/SP lifts the gate when 'Raise security gate and escape' arms) — walkreal-b's addendum documents the same regression; owner: the cut-scene/collision owner (fix-cutscenes2 candidate)"]
fn walk_the_warm_chain_to_the_front_entrance_and_out() {
    let iso = iso();
    // ---- 07000012 walked (the shared route) ----
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000012.bin", false, None).unwrap());
    tower2::walk_12(&mut w);
    tower2::walk_12_exit(&mut w);
    let save = {
        let sim = w.scene.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request,
            &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission)
    };

    // ---- 0700004A: the spawn drop onto the 0x4D touch (the game's own placement) ----
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700004a.bin", false, Some(&save)).unwrap());
    w.wait_enabled(3000, "the 4A intro");
    let mut landed = false;
    for _ in 0..600 {
        w.tick(nf_game::player::Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x4d) { landed = true; break; }
    }
    assert!(landed, "4A: the start #43 drops Bond into touch #277 → ch 0x4D [D]");
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.mission.states[16][1], 4,
        "12's 'Escape through the elevator shaft' completes (the late AND) [M 0x1827d8]");
    let save = {
        let sim = w.scene.sim.as_ref().unwrap();
        nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request,
            &sim.drones.inventory, &sim.drones.channels).with_mission(&sim.drones.mission)
    };

    // ---- 07000013 walked ----
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000013.bin", false, Some(&save)).unwrap());
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x48) && sim.drones.channels.on(0x49) && sim.drones.channels.on(0x4d),
            "the previous parts' channels carry through the warm load [M 0x182270]");
    }
    w.wait_enabled(3000, "the 13 intro");
    w.wait(120);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x4a), "the spawn touch #153 fires ch 0x4A [D]");
        assert_eq!(sim.drones.mission.states[17][0], 4, "4A's 'Gain access to lobby' completes (the late AND) [M]");
        assert_eq!(sim.drones.mission.states[18][0], 3, "'Raise security gate…' announces (enable 0x4A) [L]");
    }

    // The REAL walk south through the lobby to the front entrance (~55 units; the desks, the
    // glass and the pillars thread between the nav nodes [D §4]; the direct steer wedges on the
    // central desk block at z ≈ 25 [D probe] — the route swings west of it).
    w.walk_to(Wp::new(5.0, -368.4, 26.5).r(1.5), "13: the lobby east lane");
    w.walk_to(Wp::new(3.0, -368.4, 45.0).r(1.5), "13: the lobby mid");
    w.walk_to(Wp::new(-6.0, -368.2, 60.0).r(1.5), "13: the lobby south");
    w.walk_to(Wp::new(-6.2, -368.0, 66.3).r(0.6), "13: the front entrance");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x50), "the entrance touch #156 writes ch 0x50 [D]");
        assert_eq!(sim.drones.mission.states[18][0], 4, "the entrance objective completes [M 0x1827d8]");
    }
    // The success touches #73/#164/#165 (ch 0x64) + the AND over all parts → MISSION SUCCESS
    // [M 0x1827d8:119-123]. The three success plates sit across the entrance mouth [D] — walk
    // the band while waiting for the latch.
    let mut success = false;
    for band in [[-5.0f32, 66.0], [-7.6, 65.8], [-6.4, 66.4], [-6.4, 65.6]] {
        for _ in 0..400 {
            w.tick(nf_game::player::Input::default(), false);
            if w.scene.sim.as_ref().unwrap().drones.mission.state == 6 { success = true; break; }
        }
        if success { break; }
        let sim = w.scene.sim.as_ref().unwrap();
        eprintln!("[013] no latch at {:?}: ch64 {} ch50 {} ch4a {} ch4d {} ch49 {} ch48 {}",
            band, sim.drones.channels.on(0x64), sim.drones.channels.on(0x50), sim.drones.channels.on(0x4d),
            sim.drones.channels.on(0x4a), sim.drones.channels.on(0x49), sim.drones.channels.on(0x48));
        let _ = w.try_walk_to(Wp::new(band[0], -368.0, band[1]).r(0.8), "13: the success band");
    }
    assert!(success, "the AND raises mission success at the entrance [M 0x1827d8]");
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x64), "the success touches wrote ch 0x64 [D]");

    // LevelToEndTo 0x07300013 (the driving interlude) is the mission end's own hand-off [M
    // 0x182c60 case 6] — latched above. The level's own exit #72 (−12.79, −367.97, 68.25; the
    // E-load fallback) sits beyond the entrance's west wall (the walked probe wedges at x ≈ −9.1
    // [D]) and is DEAD DATA anyway (gate ch 0x4E has no writer on the disc [D §3]) — the walkreal
    // route ends at the mission latch; the E-arm itself is covered by walk_07000013_live.
    eprintln!("[013] walked out in {} ticks; stuck {}; falls {}", w.total_ticks, w.report.stuck_events.len(), w.report.falls.len());
}
