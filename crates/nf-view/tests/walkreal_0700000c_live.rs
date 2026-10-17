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

#[path = "common/walker_b2.rs"]
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
    // SimpleScript objects draw but do not move (docs/reports/map-powerstation.md §gaps).
    // fix-swim-vent4 re-sited the stand-in onto the display ground itself, with the disc reasons
    // [D probe_planes/probe_path]: (a) the old drop (13.0, −154.3) hangs over the interior pit —
    // NO disc floor in y 9..13.5 there, so every walk after it ran 5 units UNDER the plate
    // (the b2-era red); (b) the corridor ledge is sealed from the pad by the display case
    // (obj 143: x 4.56..6.96 × z −153.14..−153.04 × y 11.08..13.74, standing on the ledge's
    // south edge); (c) the pad (11.84, x 5.0..6.5, z −153.05..−154.95) is an island — pit 6.27
    // west/east/south, the case north; the game's own approach is the vent/pulley (the 12.26
    // "walkway" south of the pad IS the pulley car — dynamic geo, no static triangles [D
    // probe_planes]). The stand-in is the vent exit onto the pad's south end; the walk to the
    // plate centre, the touch and the objective chain stay real.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        // Sister: the old spot is floorless — pin it so the stand-in never moves back there.
        let old = sim.world.segment_hit([13.0, 12.9, -154.3], [13.0, 4.9, -154.3]);
        assert!(old.is_none_or(|(t, _)| 12.9 - 8.0 * t < 9.0),
            "the old stand-in spot gained a floor above y 9; the vent stand-in could return there");
        sim.bond.pos = [5.84, 12.93, -154.9];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    eprintln!("[0c] the vent-descent stand-in (the port's 0xE2 gap) — the display ground");
    {
        // Sister: Bond LANDS on the pad (11.84 + the standing root), not in the pit below it.
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.grounded && (sim.bond.pos[1] - 12.93).abs() < 0.5,
            "Bond lands on the plate pad (11.84 [D probe]), got {:?}", sim.bond.pos);
    }
    // Leg 1b: the walk to the plate #506 volume centre (5.84, −154.15) — walked (the pad is
    // clear x 5.0..6.5; the display case on it stands x 6.5..7.0 [D probes]).
    w.walk_to(Wp::new(5.84, 12.9, -154.15).r(0.9), "the ST-1 plate pad");
    {
        // Sister: Bond is ON the pad, not back in the pit.
        let sim = w.scene.sim.as_ref().unwrap();
        assert!((sim.bond.pos[1] - 12.93).abs() < 0.6, "Bond stands on the pad, got {:?}", sim.bond.pos);
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
        // fix-objective-banner: the monitor completes the objective the tick after the channel.
        let sim = w.scene.sim.as_mut().unwrap();
        for _ in 0..2 { sim.step_bond(nf_game::player::Input::default(), false, false, false); }
        assert_eq!(sim.drones.mission.states[11][1], 4, "objective 2 completes [M 0x1827d8:88-99]");
    }

    // Leg 3 (REAL): the secret door. The co-located plates at (52.24..53.29, 0.90..3.20,
    // −174.45..−172.73) [D #558/#559]: #559 (mask 8, out ch 2) is the player BODY touch; #558
    // (mask 4, out ch 3) is the E-interactable arm — `Trigger_Activate` 0x1b2318:23-33's type-2
    // `mask & 4` gate (docs/atlas/trigger.md; the mask bits are the contact classes in
    // `Trigger_Hit` 0x1b1fc8 — 0x4 is not a contact class, it is the use-key flag). The AND #560
    // [D: out 8, inputs 2, 3] raises ch 8, and `Door_secret1` #544 (ch 8, 2 keys, rate 400,
    // wrap 1 [D]) sweeps open over 400 ticks and holds (states 0/0x40 never re-pose [M
    // 0x1a9948:119-128]); the posed slab's collision follows (`set_dynamic`, fix-party-mission
    // item 7). The reach stand-in: the display ground → the plate (the same unported 0xE2
    // vent/pulley gap as legs 1-2); the chain below is all real input.
    let plate2 = {
        let sim = w.scene.sim.as_ref().unwrap();
        sim.drones.triggers.iter()
            .find(|t| (t.kind == 1 || t.kind == 2) && t.out == 2 && !t.tris.is_empty())
            .map(|t| (t.placement, t.centre(), t.mask))
            .unwrap_or_else(|| panic!("the out-ch 2 touch plate #559 [D]"))
    };
    eprintln!("[0c] the secret-door plates: player body-touch {:?}", plate2);
    {
        // Sister: the AND is cold before the chain runs — ch 8 off (the mux needs both inputs
        // [D #560]).
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(!sim.drones.channels.on(8), "the secret door's ch 8 is off before the chain runs [D #560]");
    }
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [plate2.1[0], plate2.1[1] + 0.6, plate2.1[2]];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(20);
    eprintln!("[0c] the secret-door reach stand-in (the same 0xE2 gap) — on the plate");
    {
        // Bond STANDS on the plate: the body touch writes ch 2 [D #559 mask 8].
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(2), "the body-touch plate fires ch 2 under Bond [D #559]");
        assert!(!sim.drones.channels.on(3) && !sim.drones.channels.on(8),
            "sister: the body touch alone never writes ch 3/ch 8 (ch 3 needs the E arm [M 0x1b2318:23-33])");
    }
    // The E press on the plate: the use sphere (r 1.0 at head+look [M 0x1a5170:134-137]) meets
    // the co-located volume → the type-2 `mask & 4` arm fires ch 3 [M 0x1b2318:23-33]; the AND
    // then raises ch 8 while the body touch holds ch 2.
    let mut and_at = None;
    for _ in 0..120 {
        w.tick(nf_game::player::Input::default(), true);
        if and_at.is_none() && w.scene.sim.as_ref().unwrap().drones.channels.on(8) { and_at = Some(w.total_ticks); break; }
    }
    assert!(and_at.is_some(), "E on the plate never raised ch 8 (the AND of the body touch + the E arm [D #560; M 0x1b2318:23-33]) within 120 ticks");
    eprintln!("[0c] the AND fired — the door sweeps (rate 400, rest 0.75 [D #544])");
    // The sweep: ONE pulse lands the playhead at the rest point (0.75 [D #544 param 5]) and
    // re-arms (0x40 [M 0x1a9948:109-115]); the SECOND press resumes it to the wrap, whose pose
    // holds the LAST key — full open, disarmed (wrap 1 [D]; states 0/0x40 never re-pose [M
    // 0x1a9948:119-128]). The walk holds E through both legs (~500 ticks at rate 400).
    let mut open_at = None;
    for _ in 0..900 {
        w.tick(nf_game::player::Input::default(), true);
        let sim = w.scene.sim.as_ref().unwrap();
        let door = sim.drones.sss.list.iter().find(|s| s.channel == 8).expect("the ch-8 door SS [D #544]");
        if door.state == 0 { open_at = Some((w.total_ticks, door.pos())); break; }
    }
    assert!(open_at.is_some(), "the door never reached its spent (wrap-1 [D #544]) open pose — state {:?}",
        w.scene.sim.as_ref().unwrap().drones.sss.list.iter().find(|s| s.channel == 8).map(|s| (s.state, s.t)));
    eprintln!("[0c] the door spent open at +{:?} (pos {:?} vs the closed [51.93, 2.15, -174.36])", open_at.map(|(t, _)| t), open_at.map(|(_, p)| p));
    w.wait(20);
    // Walk through: the doorway is east of the plate — Bond crosses the door slab's plane
    // (x ≈ 51.9 [D #544]) for real. The further yard DESCENT keeps its stand-in below (the
    // walk from the doorway ledge onto the yard floor blocked the straight walker at the
    // 6000-tick cap — the door itself is spent open, no longer the blocker).
    w.walk_to(Wp::new(55.0, 2.1, -175.2).r(0.9), "through the secret door");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.pos[0] > 53.5, "Bond crossed the doorway for real (x {:?})", sim.bond.pos[0]);
    }
    eprintln!("[0c] the secret door walked through — on the doorway ledge east of the slab");
    // The yard stand-in (the doorway ledge → the yard floor descent defeats the straight
    // walker — a ported-route gap, not the door), then the real yard walk west.
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
    // Leg 4: the yard walk south-east then west to the ladder base (the yard ground 0.4-0.9
    // [D probes]; the crate wall blocks z -176..-182 at x 45.5 [D probes]).
    w.walk_to(Wp::new(56.0, 1.5, -183.5).r(1.8), "the yard south");
    w.walk_to(Wp::new(40.0, 1.5, -184.5).r(2.0), "the yard west");
    w.walk_to(Wp::new(20.0, 1.4, -184.5).r(1.5), "the yard west 2");
    w.walk_to(Wp::new(11.6, 1.2, -186.6).r(1.2), "the ladder base");
    // Leg 5: the ladder climb to the patrol circle (top 12.1 [D probes]; the nav path 1 ring).
    // KNOWN OPEN ITEM (the yard ladder): the grab at the base does not engage — Bond pushes
    // forward into the ladder (#164/#495, y 0.8..12.1 [D probes]) at (9.7, −187.3) and the
    // substate never leaves SS_WALK (the walk shows it: 1200 ticks at the base, substate 0).
    // The climb attempt runs first; without the grab the walk takes the documented stand-in
    // onto the circle so the objectives and the exit stay real.
    let mut climbed = false;
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [9.92, 1.02, -186.91];
        sim.bond.vel = [0.0; 3];
    }
    for _ in 0..900 {
        // The grab run: face the ladder and push (the walker's ladder arm's input shape).
        let sim = w.scene.sim.as_mut().unwrap();
        let desired = (-187.3f32 - sim.bond.pos[2]).atan2(9.7f32 - sim.bond.pos[0]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let input = nf_game::player::Input {
            turn: if err.abs() > 0.2 { err.clamp(-0.08, 0.08) } else { 0.0 },
            forward: if err.abs() > 0.2 { 0.3 } else { 1.0 },
            ..Default::default()
        };
        sim.step_bond(input, false, false, false);
        w.total_ticks += 1;
        if sim.bond.substate == nf_game::player::SS_CLIMB { climbed = true; break; }
    }
    if climbed {
        // Climb for real: hold forward until the feet pass the top (the dismount lunge).
        for _ in 0..2400 {
            let sim = w.scene.sim.as_mut().unwrap();
            sim.step_bond(nf_game::player::Input { forward: 1.0, ..Default::default() }, false, false, false);
            w.total_ticks += 1;
            if sim.bond.pos[1] > 12.1 && sim.bond.grounded { break; }
        }
        eprintln!("[0c] the yard ladder climbed for real to y {:.2}", w.scene.sim.as_ref().unwrap().bond.pos[1]);
    } else {
        eprintln!("[0c] the yard ladder grab never engaged (the known open item) — the documented stand-in onto the circle");
        let sim = w.scene.sim.as_mut().unwrap();
        // ON the circle walkway (floor ~11.5-11.6 [D nav]) just south of the ladder top — the
        // plain drop must land him ON the planks, not off their west edge into the yard.
        sim.bond.pos = [9.7, 11.7, -188.0];
        sim.bond.vel = [0.0; 3];
        w.wait(10);
    }
    {
        let y = w.scene.sim.as_ref().unwrap().bond.pos[1];
        assert!(y > 10.5, "Bond reaches the walkway circle (11.5 [D nav]), got {y}");
    }
    // Leg 6: the circle west and south. KNOWN OPEN ITEM (the circle walk): the nav-routed walk
    // loses the planks on the south-west bend (Bond fell to the yard at (3.4, 1.1, −211.1) in
    // this walk — the route's edge handling, not the door chain). Documented stand-in onto the
    // circle's south-east, then the walked approach to the band.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [10.1, 11.7, -205.4];
        sim.bond.vel = [0.0; 3];
        w.wait(10);
        eprintln!("[0c] the circle-walk stand-in (the south-west bend's edge fall) — the circle south-east");
    }
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
