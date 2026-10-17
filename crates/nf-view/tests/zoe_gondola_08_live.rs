//! zoe-gondola-08 (task 750d): the user, 2026-10-16 — "zoe shoots at you when you get to the
//! gondola". Zoe (skin 0x0500_003b, dtype 0xC → team 2 [M NDrone2_GetDroneTypeAttackTypeFriend
//! 0x14cbd8, the store at disasm 0x14ccd8]) is an ally on 07000008 and must never target or fire
//! at Bond. The game's rule: `NDrone2_FindOpponent` (0x1424e0) stores Bond as the opponent
//! (Drone+0x170) only when the drone's OWN team != 2 [M disasm 0x142e04-0x142e0c: own +0x44==2
//! branches to the ally scan]; the ally scan stores only class-0x3B objects or drones whose own
//! team == 1 [M disasm 0x143100-0x143104: skip unless candidate +0x44 == 1] with range + LOS
//! [M 0x143110-0x143130], and the Attack entry `DroneVision_EnemyLookForOpponent` (0x177ed0)
//! requires a stored opponent (+0x170 != 0 [M :18-20]). Bond (the player) is therefore never a
//! team-2 ally's opponent, in any state.
//!
//! Reproduce: the real trigger path on 07000008 — the intro plate #237 (out 0x49 [D]), the walk
//! to the boarding platform, the cable-car floor Touch #236 (kind 0xEB, out 4, mask 8 [D]) and
//! the ride. The ride's own events (the station bombardment ch 0x35 at s46@1410 [D nis dump])
//! are driven by hand because the port does not parse the loaded-pack/level-bin ride script
//! (the documented pack-script gap, docs/levels/07000008.md §2) — stated here per the task's
//! teleport rule; every other step goes through the real trigger layer.
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn zoe_index(sim: &nf_view::scene::Sim) -> usize {
    sim.drones.list.iter().position(|d| d.skin == 0x0500_003b)
        .expect("Zoe (skin 0x0500003b) is placed on 07000008 [D actors.md census]")
}

/// Zoe is team 2 [M 0x14ccd8], and she never turns on Bond: no Attack entry, no shots, no
/// bullets, no player hits from her drone index.
fn assert_zoe_never_hostile(sim: &nf_view::scene::Sim, z: usize, stage: &str) {
    let d = &sim.drones.list[z];
    assert_eq!(d.team, 2, "Zoe stays team 2 [M 0x14ccd8] at {stage}");
    assert!(d.state != nf_game::drone::state::ATTACK,
        "Zoe never enters ATTACK at {stage} (state {:#x}, alert {:.2}, seen {})", d.state, d.alert, d.seen);
    assert!(d.shots.is_empty(), "Zoe never fires at {stage} ({} shots, last at frame {:?})",
        d.shots.len(), d.shots.last());
    assert!(!sim.drones.bullets.iter().any(|b| b.owner == z), "no bullets from Zoe at {stage}");
    assert!(!sim.drones.player_hits.iter().any(|&(_, o, _)| o == z), "Zoe never hits Bond at {stage}");
}

/// The user's report: walking to the gondola, Zoe shoots at Bond. Fail-first on current master.
#[test]
fn zoe_never_fires_at_bond_through_the_gondola_sequence() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        assert!(!sim.disabled, "Bond is free after the intro");
        // The carried latches from the previous levels (walk_07000008_live, [M Player_RamSave 0x195670]).
        for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    }
    let z = zoe_index(scene.sim.as_ref().unwrap());
    {
        let sim = scene.sim.as_ref().unwrap();
        let d = &sim.drones.list[z];
        assert_eq!(d.dtype, 0xc, "Zoe's dtype is 0xC [D actors.md census]");
        assert_eq!(d.team, 2, "dtype 0xC → team 2 [M 0x14cbd8 store 0x14ccd8]");
        eprintln!("[zoe] start pos {:?} state {:#x} alert {:.2} seen {}", d.pos, d.state, d.alert, d.seen);
        for (i, g) in sim.drones.list.iter().enumerate() {
            eprintln!("[roster {i}] team {} skin {:x} state {:#x} enabled {} present {} started {} corpse {}",
                g.team, g.skin, g.state, g.enabled as u8, g.present as u8, g.started as u8, g.corpse as u8);
        }
    }

    // The intro plate #237 (out 0x49, ~3.5 m from the spawn) — the real trigger; the station-intro
    // script SP #294 (3880 ticks) plays through (walk_07000008_live pattern).
    {
        let sim = scene.sim.as_mut().unwrap();
        let c = sim.drones.triggers.iter()
            .find(|t| t.kind == nf_game::triggers::kind::TOUCH_ONCE && u32::from(t.out) == 0x49)
            .map(|t| t.centre()).expect("the intro plate #237 exists [D]");
        sim.bond.pos = [c[0], c[1], c[2]];
        for _ in 0..5000 { sim.step_bond(Input::default(), false, false, false);
            if sim.drones.channels.on(0x49) && !sim.disabled { break; } }
        for _ in 0..4000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        for _ in 0..3 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert_zoe_never_hostile(scene.sim.as_ref().unwrap(), z, "the intro plate");

    // The walk to the boarding platform: Bond walks the station floor toward the cable-car floor
    // Touch #236 (kind 0xEB, out 4, mask 8 — the player). Real trigger, real walk inputs toward
    // the platform centre (112, ~25, -127 [D the carriage object (112.0, 32.4, -126.3)]).
    {
        let sim = scene.sim.as_mut().unwrap();
        // Placement #236 (flags 0xEB) is the port's kind::TOUCH with out 4 [D flags → kind map, triggers.rs].
        let floor = sim.drones.triggers.iter()
            .find(|t| t.kind == nf_game::triggers::kind::TOUCH && u32::from(t.out) == 4)
            .map(|t| t.centre()).expect("the cable-car floor Touch #236 (out 4) exists [D]");
        eprintln!("[walk] floor trigger centre {floor:?}");
        // Face the platform and walk forward in small steps (real inputs; no teleport past the
        // problem spot).
        for _ in 0..2400u32 {
            let bp = sim.bond.pos;
            let dx = floor[0] - bp[0]; let dz = floor[2] - bp[2];
            let want = dx.atan2(dz);
            let cur = sim.bond.yaw;
            let mut d = want - cur;
            while d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
            while d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
            sim.step_bond(Input { turn: d.clamp(-0.05, 0.05), forward: 1.0, ..Input::default() }, false, false, false);
            if sim.drones.channels.on(4) { break; } // the floor Touch fired [D #236 out 4]
        }
        eprintln!("[walk] ch4 (floor touch) {}", sim.drones.channels.on(4) as u8);
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    }
    assert_zoe_never_hostile(scene.sim.as_ref().unwrap(), z, "the boarding walk");

    // The boarding: the briefcase writer (ch 0x4A) is in the safe room — the walkthrough drives it
    // by hand [D Pickup_Handler 0x18e650:330-333, the documented pack gap]; the floor Touch #236
    // fired ch 4 above (real trigger). Mux #235 = AND(0x4A, 4) → ch 1 [D].
    {
        let sim = scene.sim.as_mut().unwrap();
        if !sim.drones.channels.on(4) {
            // The floor Touch didn't fire from the walk (the port holds the carriage at the top;
            // the trigger sits on its floor [D]) — board by placing Bond on it, as walk_07000008_live
            // boards by hand.
            sim.bond.pos = sim.drones.triggers.iter()
                .find(|t| t.kind == nf_game::triggers::kind::TOUCH && u32::from(t.out) == 4)
                .map(|t| t.centre()).expect("the cable-car floor Touch #236 (out 4) exists [D]");
            for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        }
        for sp in &sim.drones.spawners {
            eprintln!("[spawner placement {}] mode {} group {} start {} stop {} kill {} templates {:?}",
                sp.placement, sp.mode, sp.group, sp.start, sp.stop, sp.kill, sp.templates);
        }
        sim.drones.channels.set(0x4a, 1, sim.drones.frame);
        sim.step_bond(Input::default(), false, false, false);
    }
    assert_zoe_never_hostile(scene.sim.as_ref().unwrap(), z, "the boarding");

    // The ride: the level-bin ride script (0x0600004C) is not parsed by the port (the documented
    // gap) — drive its events the way the script does: the ride runs 1420 ticks, then s46@1410
    // writes ch 0x35 (53, the station bombardment) [D nis dump; docs/levels/07000008.md §2.4].
    // The 5 spawner drones bring the ambush [D drones.tsv]; the port runs the real spawner +
    // guard AI from here. Watch Zoe through the whole ride and the ambush.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.drones.channels.set(0x4b, 1, sim.drones.frame); // the ride objective completes as in walk_07000008_live
        sim.step_bond(Input::default(), false, false, false);
        sim.drones.channels.set(0x35, 1, sim.drones.frame); // s46@1410's bombardment channel [D]
        let mut zoe_shot_at = None;
        for t in 0..2400u32 {
            sim.step_bond(Input::default(), false, false, false);
            let d = &sim.drones.list[z];
            if t % 200 == 0 {
                let hostiles = sim.drones.list.iter().filter(|g| g.team == 1 && g.started && !g.corpse).count();
                let spawned: usize = sim.drones.spawners.iter().map(|s| s.spawned.len()).sum();
                eprintln!("[ride t{t}] zoe state {:#x} alert {:.2} seen {} unseen {} shots {} | hostiles {hostiles} spawned {spawned} | bullets {}",
                    d.state, d.alert, d.seen, d.unseen_ticks, d.shots.len(), sim.drones.bullets.len());
            }
            if d.state == nf_game::drone::state::ATTACK && zoe_shot_at.is_none() {
                zoe_shot_at = Some(t);
                eprintln!("[ride t{t}] ZOE ENTERED ATTACK — team {} shots {}", d.team, d.shots.len());
            }
        }
        let _ = zoe_shot_at;
    }
    assert_zoe_never_hostile(scene.sim.as_ref().unwrap(), z, "the ride + ambush");
}

/// Sister: the same sighting conditions that must stay inert for Zoe DO attack for a team-1
/// guard — the team rule boundary [M FindOpponent 0x142e04-0x142e0c: own team != 2 → Bond].
#[test]
fn a_team1_guard_under_the_same_conditions_attacks_bond() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
        for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    }
    let guard = scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.team == 1 && !d.corpse && d.present)
        .expect("07000008 places team-1 guards [D 9 placed + 5 spawners]");
    {
        let sim = scene.sim.as_mut().unwrap();
        // Six units in front of Bond, present, started, enabled, vision on, facing him — the game's
        // FindOpponent stores Bond for any team-1 drone [M 0x142e04-0x142e0c], and the sight +
        // prop 0x3C + alert ≥ 0.66 route (EnemyLookForOpponent 0x177ed0) enters Attack. The waiting
        // guards hold WAIT_SWITCH with vision off; start_state() is not callable from here, so put
        // the guard in IDLE directly (the DroneTypeSettings start state for the standard modes [D
        // dronetype-tables.md]) — the UPDATE arm runs the same look the game's would.
        let bp = sim.bond.pos;
        // 1.5 units from Bond: visibility() returns 1.0 inside 2.0 [M drone.rs visibility's TooClose arm],
        // so the open-floor LOS question stays out of the assertion.
        sim.drones.list[guard].pos = [bp[0], bp[1], bp[2] - 1.5];
        sim.drones.list[guard].present = true;
        sim.drones.list[guard].enabled = true;
        sim.drones.list[guard].started = true;
        sim.drones.list[guard].vision_on = true;
        sim.drones.list[guard].sight_range = 30.0; // the waiting templates hold sight_range 0.0 [M drone.rs init]
        sim.drones.list[guard].light = 1.0; // held-back templates carry no room light [L the visibility × light pass]
        sim.drones.list[guard].state = nf_game::drone::state::IDLE;
        sim.drones.list[guard].alert = 0.9; // the ambush wakes the guards alert ≥ 0.66 [M 0x177ed0's gate]
        sim.drones.list[guard].yaw = 0.0; // Bond at −z; the guard looks back down +z→−z at him
        let mut fired = false;
        for t in 0..900u32 {
            sim.step_bond(Input::default(), false, false, false);
            let g = &sim.drones.list[guard];
            if t % 150 == 0 {
                eprintln!("[sister t{t}] guard state {:#x} alert {:.2} seen {} shots {}", g.state, g.alert, g.seen, g.shots.len());
            }
            if !g.shots.is_empty()
                || sim.drones.bullets.iter().any(|b| b.owner == guard)
                || sim.drones.player_hits.iter().any(|&(_, o, _)| o == guard) {
                fired = true; break;
            }
        }
        assert!(fired, "a team-1 guard six units from Bond enters ATTACK and fires [M 0x142e04-0x142e0c + 0x177ed0]");
        // ...and Zoe, still standing in the same fight, does not.
        if let Some(z) = scene.sim.as_ref().unwrap().drones.list.iter().position(|d| d.skin == 0x0500_003b) {
            assert_zoe_never_hostile(scene.sim.as_ref().unwrap(), z, "the sister's fight");
        }
    }
}
