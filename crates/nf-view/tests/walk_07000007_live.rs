//! map-mayhew: the 07000007 walkthrough (docs/levels/07000007.md). The party: the spawn plate
//! TouchOnce #411 raises ch 0x47 (objective 1 reveals), the hall walk-in #412 raises ch 0x48 (the
//! weapon lock releases [M Player_Weapon 0x1a24c8:242-275] and objective 1 completes), the roof
//! exit LOAD_LEVEL #418 loads 07000008 unconditionally [D placements dump].
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_party_starts_locked_reveals_unlocks_at_the_hall_and_exits_to_the_gondola() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    {
        let sim = scene.sim.as_mut().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 33.4).abs() < 2.0 && (p[2] + 70.6).abs() < 2.0,
            "the party spawns at start #0 [D], got {p:?}");
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        assert!(!sim.disabled, "Bond is free after the intro");
        // The spawn plate #411 (33.6, 43.7, −70.6) is under Bond — ch 0x47 fires on the first ticks —
        // and the fists switch request lands within ~100 ticks [M Player_InitWeapon case 0x7000007].
        for _ in 0..150 { sim.step_bond(Input::default(), false, false, false); if sim.drones.channels.on(0x47) && sim.gun.weapon == 1 { break; } }
        assert!(sim.drones.channels.on(0x47), "the spawn plate reveals the party's first objective [D #411]");
        assert_eq!(sim.drones.mission[2].objectives[0].state, 3, "objective 1 is visible (revealed by ch 0x47 [L])");
        // The weapon lock holds before the hall: Bond holds the fists (weapon 1 [M level_carry_live's
        // party test; Player_InitWeapon 0x1a7ef8 case 0x7000007's switch request]).
        assert_eq!(sim.gun.weapon, 1, "the party starts on fists (the disguise lock, F4)");
    }
    // Walk the hall to #412 (94.3, 43.6, −45.9) — the weapon-lock release + objective 1.
    let touch = |s: &nf_view::scene::Sim, out: u32| {
        s.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && u32::from(t.out) == out)
            .map(|t| t.centre()).expect("the TouchOnce placement exists")
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        let c = touch(sim, 0x48);
        sim.bond.pos = [c[0], c[1], c[2]];
        // The viewer's input mapping (nf-view lib.rs 989/997): the lock swallows the cycle while
        // ch 0x48 is off, and its rising edge auto-draws the P2K.
        let mut prev = sim.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL);
        for _ in 0..120 {
            let mut input = Input::default();
            nf_game::player::weapon_lock_input(sim.level, prev, &mut input);
            sim.step_bond(input, false, false, false);
            let on = sim.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL);
            if nf_game::player::weapon_lock_auto_draw(sim.level, on, prev) { sim.gun.switch_request = 2; }
            prev = on;
            if on { break; }
        }
        // The mission monitor runs earlier in the tick than the trigger calc: settle it. The draw
        // request completes over the gun's switch ticks.
        for _ in 0..40 { sim.step_bond(Input::default(), false, false, false); }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x48), "the hall walk-in fires the lock release [D #412]");
        assert_eq!(sim.drones.mission[2].objectives[0].state, 4, "objective 1 completes on ch 0x48 [L record 2]");
        assert_eq!(sim.drones.mission[2].objectives[1].state, 3, "objective 2 reveals (ch 0x48) [L]");
        // The lock released: the weapon auto-draws on the rising edge [M 0x1a24c8:252-275].
        assert_eq!(sim.gun.weapon, 2, "the P2K auto-draws when the lock releases (Upgrade_Weapon(6) = 2 [M 0x1a24c8:252-275])");
    }
    // The exit: LOAD_LEVEL #418 (135.9, 50.1, −76.5) — gate 0, fires on E.
    let exit = scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL)
        .expect("the gondola LOAD_LEVEL (#418) exists").clone();
    {
        let sim = scene.sim.as_mut().unwrap();
        let c = exit.centre();
        sim.bond.pos = [c[0], c[1], c[2] - 0.5];
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, true); if sim.drones.level_exit.is_some() { break; } }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.level_exit, Some(0x0700_0008), "E at the roof exit loads the gondola level [D #418]");
}

/// Sister: skipping the hall walk-in leaves the lock on and the objective incomplete (the exit
/// itself is ungated — the mission carries the gap to 07000008, exactly as the records chain).
#[test]
fn skipping_the_hall_keeps_the_lock_and_the_objective() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
        // Walk straight to the exit, never through #412.
        let exit = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL)
            .expect("the LOAD_LEVEL exists").clone();
        let c = exit.centre();
        sim.bond.pos = [c[0], c[1], c[2] - 0.5];
        for _ in 0..30 { sim.step_bond(Input::default(), false, false, true); if sim.drones.level_exit.is_some() { break; } }
    }
    let sim = scene.sim.as_ref().unwrap();
    assert!(!sim.drones.channels.on(0x48), "the lock release never fires away from #412");
    assert_eq!(sim.gun.weapon, 1, "Bond stays on fists (the disguise lock, F4)");
    assert_ne!(sim.drones.mission[2].objectives[0].state, 4, "objective 1 is not complete without the hall");
    assert_eq!(sim.drones.level_exit, Some(0x0700_0008), "the ungated exit still loads the gondola level");
}
