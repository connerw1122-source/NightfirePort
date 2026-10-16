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
