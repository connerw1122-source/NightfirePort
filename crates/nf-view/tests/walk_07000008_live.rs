//! map-mayhew: the 07000008 walkthrough (docs/levels/07000008.md). The gondola: the intro plate
//! TouchOnce #237 raises ch 0x49 (completing 07000007's second objective through the carried
//! mission records), the boarding (ch 0x4A) and the ride (ch 0x4B) complete the last two, and the
//! mission monitor's ch 100 ends the level into the DRIVING hand-off 0x7300008
//! [M Mission_Update 0x182c60:85-116]. The level has NO LOAD_LEVEL placement [D].
//!
//! The F5 sister: the mask-8 plunge triggers (the falling-to-death cut-scene) can never fire from
//! the use key [M Trigger_Activate 0x1b2318:38-40].
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_gondola_mission_completes_and_hands_off_to_the_driving_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    {
        let sim = scene.sim.as_mut().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 139.5).abs() < 2.0 && (p[2] + 76.4).abs() < 2.0,
            "the gondola spawns at the mountain-top station #0 [D], got {p:?}");
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        assert!(!sim.disabled, "Bond is free after the intro");
        // The carried latches from the previous levels (in play they ride the RamSaved channels):
        // ch 0x45 (05's reveal), 0x70 (the truck park), 0x02 (the dressing scene), 0x48 (the hall).
        // Their writers live on those levels [D; docs/levels/*.md] — the warm transition carries
        // them [M Player_RamSave 0x195670 saves the switch channels].
        for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    }
    // Walk to the intro plate #237 (139.2, 49.5, −81.2) — the real trigger: ch 0x49.
    let touch = |s: &nf_view::scene::Sim, out: u32| {
        s.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && u32::from(t.out) == out)
            .map(|t| t.centre()).expect("the TouchOnce placement exists")
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        let c = touch(sim, 0x49);
        sim.bond.pos = [c[0], c[1], c[2]];
        // The plate also fires the station-intro script (SP #294, 3880 ticks) — walk in and let it play.
        for t in 1..=5000u32 {
            sim.step_bond(Input::default(), false, false, false);
            if t % 500 == 0 { eprintln!("[intro t{t}] disabled {} ch49 {} obj2 {}", sim.disabled, sim.drones.channels.on(0x49) as u8, sim.drones.mission[2].objectives[1].state); }
            if sim.drones.channels.on(0x49) && !sim.disabled { break; }
        }
        // The monitor runs earlier in the tick than the trigger calc: settle it (the intro script
        // SP #294 may hold Bond disabled a while — step through it).
        for _ in 0..4000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x49), "the intro plate fires ch 0x49 [D #237]");
        assert_eq!(sim.drones.mission[2].objectives[1].state, 4,
            "07000007's second objective completes on THIS level's plate [L record 2, complete ch 0x49]");
        assert_eq!(sim.drones.mission[3].objectives[0].state, 3, "the boarding objective reveals (ch 0x49) [L record 3]");
    }
    // The boarding (ch 0x4A) and the ride (ch 0x4B): their writers are the loadable-pack scripts
    // (SP #295 "ScriptPlayer_CableCar" 0x0600004C behind mux #235 = AND(0x4A, ch 4) [D levels.md
    // §5.3]) — the pack scripts are not parsed yet (docs/levels/07000008.md §2, the reported gap).
    // Drive the two channels the way the ride script's events would.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x4a, 1, sim.drones.frame);
        sim.step_bond(Input::default(), false, false, false);
        sim.drones.channels.set(0x4b, 1, sim.drones.frame);
        sim.step_bond(Input::default(), false, false, false);
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.mission[3].objectives[0].state, 4, "the boarding completes the fourth objective");
    assert_eq!(sim.drones.mission[3].objectives[1].state, 4, "the ride completes the fifth");
    // All six Mayhew objectives' channels hold → ch 100 → the mission ends the level [M 0x182c60:85-116].
    assert!(sim.drones.channels.on(100), "the mission-complete channel fires [M 0x1827d8:155-160]");
    assert_eq!(sim.drones.level_exit, Some(0x0730_0008), "the gondola hands off to the DRIVING level [M 0x182c60:100]");
}

/// Sister: without the boarding and the ride the mission is incomplete — the level cannot end
/// (there is no LOAD_LEVEL to fall back to [D]).
#[test]
fn skipping_the_ride_leaves_the_mission_open() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
        // The intro plate fires, but Bond never boards.
        let c = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && t.out == 0x49)
            .map(|t| t.centre()).expect("the intro plate exists");
        sim.bond.pos = [c[0], c[1], c[2]];
        for _ in 0..200 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0x49) { break; } }
        for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x49), "the intro plate fired");
    assert!(!sim.drones.channels.on(100), "the mission is incomplete without the boarding and the ride");
    assert_eq!(sim.drones.level_exit, None, "the gondola level cannot end without the ride");
}

/// The F5 sister: pressing E at the gondola doors can never fire the mask-8 plunge (the falling
/// cut-scene) [M Trigger_Activate 0x1b2318:38-40] — the door E-path is safe.
#[test]
fn the_use_key_never_fires_the_falling_cut_scene() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        // The cliff-side doors (#117/#118 near the station) and the plunge volumes: E everywhere.
        let door = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL);
        let _ = door; // 07000008 has none [D] — the doors are the 0xDB placements; E-press at Bond's spot.
        sim.bond.pos = [136.3, 50.0, -75.9]; // door #117's xz [D placements dump]
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, true); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x7c), "the plunge trigger (out 0x7C, mask 8) never fires from E [M 0x1b2318:38-40]");
    assert!(!sim.drones.channels.on(0x62), "no death channel — the falling cut-scene did not play");
    assert_eq!(sim.drones.level_exit, None);
}
