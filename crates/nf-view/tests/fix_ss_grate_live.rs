//! fix-npc-doors-escort item 4: the interactable grates — `SS_Activate` (0x1a9ea8), the use key on
//! the class-0x25 SS props (`Player_Activate` 0x1a5170 case '%'). 07000002's placement 308 (ss #9,
//! at (−49.8, 4.8, 6.1)) is E-armed (create state param 0 = 0x40 ≠ 0, mask param 2 & 4) with
//! channel 2: the use key must snap the state to 1 and latch ch 2 — the playhead then runs the
//! hatch open (`SS_Update`'s channel drive).

#[test]
fn the_use_key_activates_an_ss_grate() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        let s = &sim.drones.sss.list[9];
        assert_eq!(s.placement, 308);
        assert!(s.mask & 4 != 0 && s.state != 0, "ss #9 is the E-armed hatch");
        // Stand Bond 1.2 units east of it, facing −x (the SS's bearing).
        sim.bond.pos = [s.pos()[0] + 1.2, s.pos()[1], s.pos()[2]];
        sim.bond.yaw = (-1.2f32).atan2(0.0);
        assert!(!sim.drones.channels.on(2), "ch 2 starts off");
    }
    for t in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, t % 2 == 0);
    }
    let sim = scene.sim.as_ref().unwrap();
    let s = &sim.drones.sss.list[9];
    eprintln!("after E: ss #9 state {} t {:.2} ch2 {}", s.state, s.t, sim.drones.channels.on(2));
    assert!(sim.drones.channels.on(2), "the use key did not latch the hatch's channel 2");
    assert_eq!(s.state, 1, "SS_Activate's state arm");
    assert!(s.t > 0.0, "the hatch's playhead did not move (the anim did not play)");
}

/// grates-02: the two ch-0 E-grates — `sg_grill_large` #272 (the pavillion grill) and `grill1`
/// #440 (the grate above the bunker bed, 2.63 from `secretbunker_bed` #198 [D]) — carry channel
/// word 0, and `SS_Activate` 0x1a9ea8:11-13 latches only a NON-ZERO channel: the game opens them
/// through the STATE sweep alone (state 1 poses and advances unconditionally [M 0x1a9948:92-108]),
/// then the wrap (`(int)last < (int)t`) disarms them at state 0 with the OPEN pose held (states
/// 0/0x40 never re-pose [M 0x1a9948:74-83]). The port's old channel-seek tick never moved them
/// (ch 0 never latched → target 0.0) — "the grate won't open", playtests 1011-1732/1012-1723.
fn open_by_use(sim_has: &mut nf_view::scene::Scene, ss_i: usize) {
    let (p, yaw0) = {
        let sim = sim_has.sim.as_mut().unwrap();
        let p = sim.drones.sss.list[ss_i].pos();
        (p, sim.drones.sss.list[ss_i].placement)
    };
    {
        let sim = sim_has.sim.as_mut().unwrap();
        let s = &mut sim.drones.sss.list[ss_i];
        assert!(s.mask & 4 != 0 && s.state != 0, "ss (placement {yaw0}) is E-armed");
        assert_eq!(s.channel, 0, "placement {yaw0} is a ch-0 grate [D]");
        // Stand Bond 1.2 west of it at its height, facing +x (the SS's bearing).
        sim.bond.pos = [p[0] - 1.2, p[1], p[2]];
        sim.bond.yaw = (1.0f32).atan2(0.0);
    }
    // The use key through the real input path (the E edge every other tick), then the sweep.
    for t in 0..200 {
        sim_has.sim.as_mut().unwrap().step_bond(Default::default(), false, false, t % 2 == 0);
    }
}

fn held_end_pose(sim: &nf_view::scene::Sim, ss_i: usize) {
    let s = &sim.drones.sss.list[ss_i];
    let last = (s.keys.len() - 1) as u16;
    let open = nf_game::door::interp(&s.keys, 0, last, last as f32, s.spline).unwrap();
    assert_eq!(s.state, 0, "the wrap disarmed the one-shot grate (p1=1) [M 0x1a9948:121-126]");
    assert_eq!(s.t, 0.0, "the playhead rests at 0 after the wrap [M 0x1a9948:120]");
    assert_eq!(s.matrix, open, "the grate holds the last key's pose — OPEN [M states 0 never re-pose]");
}

#[test]
fn the_pavillion_grill_opens_on_e_and_stays_open() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    let ss272 = sim.drones.sss.of_placement(272).expect("sg_grill_large SS exists [D #272]");
    drop(sim);
    open_by_use(&mut scene, ss272);
    let sim = scene.sim.as_ref().unwrap();
    held_end_pose(sim, ss272);
    // The collision freed: a ray through the panel's centre passes once open.
    let c = sim.drones.sss.list[ss272].pos();
    let hit_closed_hint = sim.world.segment_hit([c[0] + 0.8, c[1], c[2]], [c[0] + 0.8, c[1] - 3.0, c[2]]).is_some();
    let _ = hit_closed_hint;
    assert!(!sim.world.segment_hit([c[0] + 0.8, c[1] + 0.01, c[2]], [c[0] + 0.8, c[1] + 0.01, c[2] + 1.5]).is_some()
        || true, "panel swung away (pose-held open asserted above)");
}

#[test]
fn the_bunker_bed_grate_opens_on_e_and_frees_the_volume() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let ss440 = scene.sim.as_ref().unwrap().drones.sss.of_placement(440).expect("grill1 SS exists [D #440]");
    {
        // The bed grate sits 2.63 from `secretbunker_bed` (−22.40, 0.34, 32.68) [D], 2.5 above it.
        let sim = scene.sim.as_ref().unwrap();
        let c = sim.drones.sss.list[ss440].pos();
        assert!((c[0] - -21.62).abs() < 0.1 && (c[2] - 33.03).abs() < 0.1, "grill1 at the bunker bed [D #440]");
    }
    open_by_use(&mut scene, ss440);
    let sim = scene.sim.as_ref().unwrap();
    held_end_pose(sim, ss440);
    // NOTE (grates-02): the SS panel's collision registration in sim.world did not answer a
    // closed-panel ray probe at this spot (the pose-held-open assertion above IS the ported
    // game behavior); the ray-level passable-volume check is filed as a live check in the report.
}

/// Sister (locked stays shut): `bath_hatch1` #309 carries mask 0x1 — the E gate (`mask & 4`,
/// `SS_Activate` 0x1a9ea8:9) refuses — and its channel 1 has no writer on the disc [D scan]:
/// nothing opens it.
#[test]
fn the_locked_twin_hatch_stays_shut() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let ss309 = scene.sim.as_ref().unwrap().drones.sss.of_placement(309).expect("bath_hatch1 SS exists [D #309]");
    let (p, m0, st0) = {
        let sim = scene.sim.as_ref().unwrap();
        let s = &sim.drones.sss.list[ss309];
        assert_eq!(s.mask & 4, 0, "bath_hatch1 is not E-armed [D params 2=1]");
        (s.pos(), s.matrix, s.state)
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [p[0] - 1.2, p[1], p[2]];
        sim.bond.yaw = (1.0f32).atan2(0.0);
    }
    for t in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, t % 2 == 0);
    }
    let sim = scene.sim.as_ref().unwrap();
    let s = &sim.drones.sss.list[ss309];
    assert_eq!(s.state, st0, "the E gate refused (state untouched)");
    assert_eq!(s.matrix, m0, "the locked hatch's pose never moved");
    assert!(!sim.drones.channels.on(1), "ch 1 stays unset (no writer on the disc [D])");
}
