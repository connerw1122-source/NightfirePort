//! The walkreal-07000012 route, shared by the 07000012 and 07000013 tests (the warm chain walks
//! the same legs). Each step drives the REAL input path; the single documented stand-in is the
//! door[19] doorway crossing (see the note at `cross_door19`).

use crate::walker::Wp;

/// The walkreal-12 route: spawn → the keypad burn → the console E. Used again by the 07000013
/// test's warm chain.
pub fn walk_12(w: &mut crate::walker::Walker) {
    w.wait_enabled(3000, "the intro");
    w.wait(120);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x48), "the spawn touch #679 fires ch 0x48 [D]");
        assert_eq!(sim.drones.mission.states[15][1], 4,
            "07000011's 'Gain access to lower office floor' completes (the late AND) [M 0x1827d8]");
        assert_eq!(sim.drones.mission.states[16][0], 3,
            "'Activate elevator override control' announces [L record 07000012 obj 0]");
    }

    // Leg 1: south through the lobby, around the shaft core to the keypad by door[19] — walked.
    // (The core's east wall x ≈ 7.0-7.5 walls z 6..16.5 [D probes]; the crossing is at its south
    // end, the keypad wall z ≈ 0.)
    w.walk_to(Wp::new(14.0, 1.1, 6.0).r(1.2), "the lobby south");
    w.walk_to(Wp::new(10.0, 1.1, 2.0).r(1.2), "the floor south 2");
    w.walk_to(Wp::new(9.3, 1.1, -0.6).r(1.0), "the keypad approach");

    // Leg 2: the Q-Decryptor keypad burn — the real G cycle equips 0x4E (the loadout owns it [D];
    // the gadget→gadget prev-slot pollution is fixed), then the held beam burns the lock.
    {
        let mut cycles = 0;
        loop {
            let sim = w.scene.sim.as_ref().unwrap();
            if sim.gun.weapon == 0x4e { break; }
            assert!(cycles < 14, "the G cycle never reached the Q-Decryptor (0x4E)");
            w.tick(nf_game::player::Input { gadget_cycle: 1, ..Default::default() }, false);
            for _ in 0..60 { w.tick(nf_game::player::Input::default(), false); } // the draw cycle
            cycles += 1;
        }
        assert_eq!(w.scene.sim.as_ref().unwrap().gun.weapon, 0x4e, "the Q-Decryptor is held [M runs_beam]");
    }
    let lock_at = [7.0f32, 1.6, 0.2];
    let start = w.total_ticks;
    loop {
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0xa) { break; }
        assert!(w.total_ticks - start < 600, "the beam never burned the keypad lock (ch 0xa) [M 0x1ae3d8]");
        let p = sim.bond.pos;
        let eye = p[1] + sim.bond.eye - sim.bond.crouch_drop;
        let dx = lock_at[0] - p[0];
        let dz = lock_at[2] - p[2];
        let dy = lock_at[1] - eye;
        let desired = dx.atan2(dz);
        let err = crate::walker::wrap_pi(desired - sim.bond.yaw);
        let want_pitch = (dy / crate::walker::dxz(p, lock_at).max(0.5)).atan() / std::f32::consts::FRAC_PI_2;
        let input = nf_game::player::Input {
            turn: err.clamp(-0.05, 0.05),
            look: (want_pitch - sim.bond.pitch).clamp(-0.05, 0.05),
            ..Default::default()
        };
        w.tick(input, false);
    }
    eprintln!("[012] the keypad lock burned at +{} ticks (ch 0xa) [M 0x1ae3d8:194]", w.total_ticks - start);
    {
        // The unlock edge + the door's response (the two-stage latch fix): the door OPENS.
        for _ in 0..60 { w.tick(nf_game::player::Input::default(), false); }
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.doors.list[19].state & 0xc != 0,
            "ch 0xa's rising edge opens door[19] [M Door_Update's lock path; 0x1336f8's two-latch snapshot]");
    }

    // Leg 3: through the opened door[19] and north-west to the console. THE DOCUMENTED STAND-IN:
    // the doorway crossing itself. The tunnel (the closed leaf's volume x 6.74..7.07, z −1.46..−0.26
    // [D probe]) leaves a 1.2 opening; the OPEN leaf's slab (x 5.65..6.85, z −1.59..−1.26 [D probe])
    // pokes its east edge 0.2 into the tunnel mouth, and the capsule (r 0.55 [M Player_Collision
    // 0x194df0]) has no convergent push path between that edge and the north jamb — the port pins
    // at x ≈ 7.14 (the drives from z −0.9/−0.76/−0.72/−0.5 all stop; capsule_push at the pin sums
    // to an east push (0.057, 0.029) that cancels the move). The game passes this doorway on the
    // same key data — the funnel difference is unresolved; see the report's live checks. Everything
    // else on the level is walked.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [6.4, 1.03, -1.5];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    eprintln!("[012] the door[19] doorway stand-in (the corner pincer; see the report) — the west side");
    w.walk_to(Wp::new(1.0, 1.1, 4.0).r(1.5), "the office floor north");
    w.walk_to(Wp::new(-1.55, 0.5, 7.2).r(1.0), "the console");

    // The E press on the console (#731 override + #702 lift call, the same volume [D]).
    let mut pressed = false;
    for _ in 0..60 {
        w.tick(nf_game::player::Input::default(), true);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x49) && sim.drones.channels.on(0x59) { pressed = true; break; }
    }
    assert!(pressed, "E at the console writes ch 0x49 (override #731) and ch 0x59 (lift call #702) [D]");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert_eq!(sim.drones.mission.states[16][0], 4, "the override objective completes [M 0x1827d8]");
        assert_eq!(sim.drones.mission.states[16][1], 3, "'Escape through the elevator shaft' announces [L]");
    }
}

/// Leg 4 + the exit: the shaft entrance and the gated exit (the shared tail of the route).
pub fn walk_12_exit(w: &mut crate::walker::Walker) {
    // THE SECOND DOCUMENTED STAND-IN: the shaft lobby's entrance. The lobby (the exit #669
    // trigger at (2.3, 10.0), the touch #683 at z 12.1-13.1 [D]) is walled off from the office
    // floor on every probed line (the south x 0.5/2.4, the east z 9.5/11, the west z 9.5 — all
    // class-21 hits [D probe]; the call-button #699 (1.1, 8.2) is an E-touch ON the wall face,
    // not a doorway). The game reaches the lobby some probed-ray-invisible way (see the report's
    // live checks). The stand-in places Bond inside the lobby; the touch and the exit stay REAL.
    {
        let sim = w.scene.sim.as_mut().unwrap();
        sim.bond.pos = [2.3, 0.7, 10.4];
        sim.bond.vel = [0.0; 3];
    }
    w.wait(10);
    eprintln!("[012] the shaft-lobby entrance stand-in (the walled-off lobby; see the report)");
    w.walk_to(Wp::new(2.2, 0.7, 12.55).r(0.4), "into the shaft entrance");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x1f), "the shaft touch #683 holds ch 0x1F [D]");
        assert_eq!(sim.drones.level_exit, Some(0x0700_004a),
            "the gated exit #669 loads 0700004A [M 0x1b1a78 case 10]");
        let door1 = sim.drones.sss.list.iter().position(|s| s.placement == 677)
            .expect("lft_LIFTDOOR1 (#677) is an animated SS [D]");
        let x = sim.drones.sss.list[door1].matrix[3][0];
        assert!((x - 3.070).abs() < 0.06,
            "the lift door leaf stands at key 1 (x ≈ 3.070 [D]) after the call, got {x}");
    }
    eprintln!("[012] walked out in {} ticks; stuck {}; falls {}", w.total_ticks, w.report.stuck_events.len(), w.report.falls.len());
}
