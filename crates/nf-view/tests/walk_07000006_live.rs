//! map-mayhew: the 07000006 walkthrough (docs/levels/07000006.md). The castle interior has NO
//! LOAD_LEVEL placement [D]: the single exit is Touch #746 (out ch 0x83) → ScriptPlayer #810 → the
//! dressing cut-scene 0x06000071 → its closing event 0x12 loads 07000007 [M Script_EventHandler
//! 0x1d94c0 case 0x12] and its event 0xD [2,1] completes the mission's second objective
//! ("Find a way inside to the party", complete ch 0x02 [L ee_start_05.bin record 1]).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::{Input, RamSave};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn carry(on: &[u32]) -> RamSave {
    let mut ch = nf_game::triggers::Channels::default();
    for &c in on { ch.set(c, 1, 0); }
    // A fresh Bond with only the carried gates: the save Trigger_Activate would make [M 0x1b2318:97-116].
    RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch)
}

#[test]
fn the_castle_interior_reaches_the_loadpoint_and_dresses_into_the_party() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The truck route's arrival: the carried gate ch 0x1D selects the truck-ride start #496 [D;
    // M Player_Start 0x190fd0], and the truck park's ch 0x70 (+ its reveal 0x45) rides the save —
    // the mission's first objective is complete, the second ("Find a way inside to the party",
    // reveal ch 0x70 [L]) is visible.
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&carry(&[0x1d, 0x45, 0x70]))).expect("load 07000006");
    {
        let sim = scene.sim.as_mut().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 57.18).abs() < 2.0 && (p[2] - 22.61).abs() < 2.0,
            "the 0x1D gate selects the truck-ride spawn #496, got {p:?}");
        // Wait out the level-start NIS.
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        assert!(!sim.disabled, "Bond is free after the intro");
        // The arrival plates raise ch 185 (the driver greeting) — walk a little at the spawn.
        let mut input = Input::default();
        input.forward = 1.0;
        for _ in 0..60 { sim.step_bond(input, false, false, false); }
        eprintln!("[arrival] ch185 {}", sim.drones.channels.on(185) as u8);
    }
    // The exit: the LoadPoint trigger #746 (out ch 0x83) — its volume from the parsed trigger.
    let lp = scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.out == 0x83)
        .expect("the LoadPoint trigger (out 0x83) exists").clone();
    let c = lp.centre();
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [c[0], c[1] + 1.0, c[2]];
        // Stand in it (the real path — the player walks into the LoadPoint and the cut-scene takes
        // over; event 4 disables Bond while it plays).
        for t in 1..=2000u32 {
            sim.step_bond(Input::default(), false, false, false);
            if t % 400 == 0 { eprintln!("[dress t{t}] ch83 {} ch2 {} exit {:?}", sim.drones.channels.on(0x83) as u8, sim.drones.channels.on(2) as u8, sim.drones.level_exit); }
            if sim.drones.level_exit.is_some() { break; }
        }
    }
    // The monitor runs earlier in the tick than the scripts: give it the tick after the exit.
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x83), "the LoadPoint fires the dressing script's watch channel");
    // The dressing cut-scene's closing event completes the mission objective [L record 1]...
    assert_eq!(sim.drones.mission.states[nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0006).expect("level in the mission table")][0], 4, "'Find a way inside to the party' completes on ch 0x02 [L]");
    // ...and its event 0x12 loads the party level [M 0x1d94c0 case 0x12].
    assert_eq!(sim.drones.level_exit, Some(0x0700_0007), "the dressing script ends into 07000007");
}

/// Sister: away from the LoadPoint the dressing scene never starts — no ch 0x83, no objective, no
/// exit (the level cannot be skipped past its only door).
#[test]
fn skipping_the_loadpoint_never_ends_the_castle() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&carry(&[0x1d]))).expect("load 07000006");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        // Stand at the spawn, far from the LoadPoint volume.
        for _ in 0..1200 { sim.step_bond(Input::default(), false, false, false); }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x83), "the LoadPoint stays cold away from its volume");
    assert_ne!(sim.drones.mission.states[nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0006).expect("level in the mission table")][0], 4, "the objective is not complete");
    assert_eq!(sim.drones.level_exit, None, "the castle does not end away from the LoadPoint");
}
