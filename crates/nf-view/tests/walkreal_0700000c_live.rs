//! walkreal-b: 0700000c (the power station, part 1) walked with the player's own movement — no
//! teleports except the documented stand-ins (the unported 0xE2 SimpleScript family; see the
//! blockers below). Route (docs/levels/0700000c.md): the ST-1 walkway start #1 (8.98, 15.07,
//! −149.65 [D]) → the raised south-east corner (x 14-17 [D probes]) → BLOCKER: the descent into
//! ST-1's interior (the vent/pulley route — the south edge is walled for x ≤ 14 at y 14.2 and
//! the raised corner boxes in [D probes]) → the documented stand-in onto the lower walkway →
//! the ST-1 plate #506 (4.67–7.00, 12.73–13.67, −155.13..−153.18 [D]) through the display case's
//! south side (the case blocks x ~6.5-7.0, z −153.5..−154.9 [D probes]) → ch 0x1b (objective 1)
//! → the display stand-in (the same vent gap; the walkthrough's spot 5.4, 7.3, −113 [D
//! map-powerstation]) → the REAL camera photograph (0x54 [M 0x1a7ef8 case 0x700000c]) into the
//! CamSubject volumes #414/#454 (ch 0x1c [D]) → ch 0x1c (objective 2) → the secret-door stand-in
//! (Door_secret1 is a 0xE2 SS [D §6]) east of the doorway onto the yard → the REAL yard walk
//! west (the crate wall blocks z −176..−182 at x 45.5; the passage z −184..−186 [D probes]) →
//! the ladder (9.7, −187.3, 0.8..12.1 [D probe]) → the patrol circle west and south (the nav
//! path 1 ring [D]) → BLOCKER: the circle is a closed loop; the gap floors (x 14-38, z −202..−209)
//! and the band's own gap (x 59-70, z −209.2..−210.4) and the checkerboard east piece with a
//! crate at x ~84 [D probes] — the `obj_stor*_pulley` winches (#351/#364/#377, 0xE2) cross it in
//! the game → the documented stand-in onto the band east piece → the REAL band walk to the gate
//! plane → E → level_exit 0x0700000d ([M Player_Activate 0x1a5170 case 0x1d → Trigger_Activate
//! 0x1b2318:111], the key gate ch 0x1c on).

#[path = "common/walker.rs"]
mod walker;

use walker::{Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn walk_start_to_the_st1_plate_and_the_photograph() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "0700000c.bin", false, None).unwrap());
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] - 8.98).abs() < 2.0 && (p[1] - 15.07).abs() < 2.0 && (p[2] + 149.65).abs() < 2.0,
            "the walkway start #1 [D], got {p:?}");
        assert!(sim.drones.inventory.owned.get(0x54).copied().unwrap_or(false),
            "the camera gadget (0x54) is in the loadout [M 0x1a7ef8 case 0x700000c]");
    }
    w.wait_enabled(3000, "the intro");
    w.wait(120);

    // Leg 1: east along the walkway to its raised south-east corner — walked.
    w.walk_to(Wp::new(13.0, 14.6, -150.5).r(1.3), "the walkway east");
    w.walk_to(Wp::new(14.6, 15.6, -152.6).r(1.2), "the raised corner");
    // BLOCKER (pinned, documented): the descent into ST-1's interior. The south edge is walled
    // for x ≤ 14 at y 14.2 and the raised corner box walls all four sides above y 15.2 [D
    // probes]; the game's way in/out is the vent/pulley route (`ventmesh-front` #138, `VentLatch`
    // #182 `Lock_Create` 0x1ae1f8, the `obj_stor*_pulley` 0xE2 winches) and the port's
    // SimpleScript objects draw but do not move (docs/reports/map-powerstation.md §gaps). The
    // documented stand-in: onto the lower walkway by the plate (the map task's render spot).
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [13.0, 12.2, -154.3];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    eprintln!("[0c] the vent-descent stand-in (the port's 0xE2 gap) — the lower walkway");
    // Leg 1b: the lower walkway to the ST-1 plate #506 — walked. The display case stands at
    // x ~6.5-7.0, z -153.5..-154.9 [D probes]; the walked approach crouch-walks the corridor
    // north of it (z -153.0..-152.8, open at all walking heights [D probes]) — the upper
    // walkway's underside gives head_bump there, so the real crouch input carries Bond through.
    w.walk_to(Wp::new(7.2, 12.2, -153.1).r(0.9), "north of the display case");
    {
        let start = w.total_ticks;
        let target = [6.8f32, 12.2, -153.15];
        loop {
            if w.scene.sim.as_ref().unwrap().drones.channels.on(0x1b) { break; }
            if w.total_ticks - start >= 900 {
                let p = w.scene.sim.as_ref().unwrap().bond.pos;
                panic!("[0c] the crouch corridor never reached the band; bond {p:?}");
            }
            let sim = w.scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            if walker::dxz(p, target) < 0.5 { break; }
            let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let input = nf_game::player::Input {
                crouch: true,
                turn: err.clamp(-0.07, 0.07),
                forward: 0.5,
                ..Default::default()
            };
            w.tick(input, false);
        }
        // Stand back up (crouch is the edge-triggered toggle — docs/systems/input.md §+0x12c).
        w.tick(nf_game::player::Input { crouch: true, ..Default::default() }, false);
        for _ in 0..60 {
            if !w.scene.sim.as_ref().unwrap().bond.crouched { break; }
            w.tick(nf_game::player::Input::default(), false);
        }
    }
    assert!(w.scene.sim.as_ref().unwrap().drones.channels.on(0x1b),
        "the ST-1 plate #506 → ch 0x1b (objective 1) [D]");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[11][0], 4, "objective 1 completes [M 0x1827d8:88-99]");
        assert_eq!(sim.drones.mission.states[11][1], 3, "ch 0x1b reveals objective 2 [L record 11 obj 1]");
    }

    // Leg 2: the photograph. BLOCKER (pinned, documented): the game reaches the display ground
    // through the same unported vent route — the stand-in places Bond at the walkthrough's
    // display position [D map-powerstation render spot 5.4,7.3,-113]. The shot itself is the
    // REAL bullet path into the CamSubject volume ([M CamSubject_Update 0x1bc668:14-24]).
    let subject = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.props.subjects.iter().find(|s| s.channel == 0x1c).expect("the jetpack subjects [D #414/#454]").clone()
    };
    let sc = [(subject.lo[0] + subject.hi[0]) / 2.0, (subject.lo[1] + subject.hi[1]) / 2.0, (subject.lo[2] + subject.hi[2]) / 2.0];
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [5.4, 7.35, -113.0];
        sim.bond.vel = [0.0; 3];
        sim.gun.switch_request = 0x54;
    }
    w.wait(40);
    eprintln!("[0c] the display stand-in (the same 0xE2 gap) — the camera out");
    assert_eq!(w.scene.sim.as_ref().unwrap().gun.weapon, 0x54, "the camera is held [M 0x1a7ef8]");
    let photographed = shoot_at(&mut w, sc, 30.0, 900, "the jetpack photograph",
        &|sim| sim.drones.channels.on(0x1c));
    assert!(photographed, "the photograph never wrote ch 0x1c [M 0x1bc668:14-24]");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[11][1], 4, "objective 2 completes [M 0x1827d8:88-99]");
    }

    // Leg 3: the yard stand-in (Door_secret1 is an unported 0xE2 SS [D; the doc's gap list]) —
    // east of the doorway (the closed 0xE2 door's slab holds collision in the port), then the
    // real yard walk west.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [56.5, 2.8, -176.5];
        sim.bond.vel = [0.0; 3];
    }
    // Land, then stand up (crouch is the edge-triggered toggle — docs/systems/input.md §+0x12c
    // mask 8; the fall lands him still crouched from the corridor walk).
    for _ in 0..200 {
        if w.scene.sim.as_ref().unwrap().bond.grounded { break; }
        w.tick(nf_game::player::Input::default(), false);
    }
    w.tick(nf_game::player::Input { crouch: true, ..Default::default() }, false);
    for _ in 0..60 {
        if !w.scene.sim.as_ref().unwrap().bond.crouched { break; }
        w.tick(nf_game::player::Input::default(), false);
    }
    eprintln!("[0c] the secret-door stand-in (the port's 0xE2 gap) — the yard below");
    // Leg 4: the yard walk south-east then west to the ladder base (the yard ground 0.4-0.9
    // [D probes]; the crate wall blocks z -176..-182 at x 45.5 [D probes]).
    w.walk_to(Wp::new(56.0, 1.5, -183.5).r(1.8), "the yard south");
    w.walk_to(Wp::new(40.0, 1.5, -184.5).r(2.0), "the yard west");
    w.walk_to(Wp::new(20.0, 1.4, -184.5).r(1.5), "the yard west 2");
    w.walk_to(Wp::new(11.6, 1.2, -186.6).r(1.2), "the ladder base");
    // Leg 5: the ladder climb to the patrol circle (top 12.1 [D probes]; the nav path 1 ring).
    w.walk_to(Wp { at: [9.7, 12.4, -187.3], act: walker::Act::Ladder { top_y: 12.1 }, arrive: 1.2 }, "the ladder climb");
    {
        let y = w.scene.sim.as_ref().unwrap().bond.pos[1];
        assert!(y > 10.5, "Bond climbs to the walkway circle (11.5 [D nav]), got {y}");
    }
    // Leg 6: the circle west and south — walked.
    w.follow_nav_route([2.8, 11.6, -208.4], "the walkway circle south");
    w.walk_to(Wp::new(10.1, 11.6, -205.4).r(1.5), "the circle south-east");
    // BLOCKER (pinned, documented): the circle is a closed patrol loop; the gap floors (x 14-38,
    // z -202..-209) and the band's own gap (x 59-70, z -209.2..-210.4) and the checkerboard east
    // piece (z -209.2+ floorless at x 78-82/90, a crate at x ~84 [D probes]). The game crosses
    // by the `obj_stor*_pulley` winches (0xE2). The stand-in: onto the band east piece.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [90.0, 11.6, -208.8];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    eprintln!("[0c] the pulley stand-in (the port's 0xE2 gap) — the gate band east piece");
    // Leg 7: the band east to the gate plane (floor 10.3 at x 86-96 [D probes]).
    w.walk_to(Wp::new(93.0, 11.6, -209.0).r(0.9), "the gate approach");
    w.walk_to(Wp::new(96.0, 11.6, -209.3).r(0.9), "the gate plane");
    // The E press: USE-activated LOAD_LEVEL [M 0x1a5170 case 0x1d → 0x1b2318:111].
    let mut loaded = false;
    for _ in 0..60 {
        w.tick(nf_game::player::Input::default(), true);
        if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { loaded = true; break; }
    }
    assert!(loaded, "the gate #556 loads 0700000d (key ch 0x1c on) [D; M 0x1b2318:111]");
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_000d), "the hand-off target [D]");
    eprintln!("[0c] walked out in {} ticks; stuck {}; falls {}", w.total_ticks, w.report.stuck_events.len(), w.report.falls.len());
}

/// Fire the held weapon at a static point through the real trigger path (the walker's aim loop;
/// kept local to keep walker.rs additive).
fn shoot_at(w: &mut Walker, target: [f32; 3], range: f32, max_ticks: u32, _label: &str, done: &dyn Fn(&nf_view::scene::Sim) -> bool) -> bool {
    let start = w.total_ticks;
    let mut last_tap = 0u32;
    loop {
        if done(&w.scene.sim.as_ref().unwrap()) { return true; }
        if w.total_ticks - start >= max_ticks { return false; }
        {
            let sim = w.scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let eye = bp[1] + sim.bond.eye - sim.bond.crouch_drop;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - eye;
            if (dx * dx + dy * dy + dz * dz).sqrt() > range { return false; }
            let desired = dx.atan2(dz);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let want_pitch = (dy / walker::dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            let on_target = err.abs() < 0.008 && perr.abs() < 0.012 && w.total_ticks - last_tap > 15;
            let trigger = on_target;
            if trigger { last_tap = w.total_ticks; }
            let input = if trigger {
                nf_game::player::Input { aiming: true, ..Default::default() }
            } else {
                nf_game::player::Input {
                    turn: err.clamp(-0.08, 0.08),
                    look: perr.clamp(-0.08, 0.08),
                    aiming: true,
                    ..Default::default()
                }
            };
            w.scene.sim.as_mut().unwrap().step_bond(input, trigger, false, false);
        }
        w.total_ticks += 1;
        if w.top_up_health { w.scene.sim.as_mut().unwrap().drones.health.health = 100.0; }
    }
}
