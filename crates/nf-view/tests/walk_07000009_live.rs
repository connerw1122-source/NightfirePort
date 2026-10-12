//! Headless NF_ISO walkthrough for 07000009 ("Night of the Agent", Tower A) — the real Sim path
//! (`nf_view::load` + `Sim::step_bond`), the same functions the viewer calls, so a pass means the
//! level works in play. map-tower: the tower missions must be completable start to end.
//!
//! Level map (docs/levels/07000009.md): Bond spawns in the Phoenix Building car park
//! (start #68, (3.59, −377.08, 36.76), gate 0 [D placements 0x24]); the exit is the main lift
//! (LOAD_LEVEL #63 next 0x0700000a, gate ch 0x32 [D]) with TOUCH_ONCE #67 (out 0x32) inside the
//! cab; objective 2 (0x4000017 "Get to the lobby and activate the main elevator system")
//! completes on ch 0x46 = AND(ch 0x39) [D trigger #274]; objective 1 (0x4000016, inverted on the
//! alarm channel 0x61 = OR(0x7C, 0x7D) [D trigger #278]) fails the mission when the alarm raises.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, "07000009.bin", false).expect("load 07000009")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

fn step_at(scene: &mut nf_view::scene::Scene, pos: [f32; 3], yaw: f32, use_key: bool) {
    if let Some(sim) = scene.sim.as_mut() {
        sim.bond.pos = pos;
        sim.bond.yaw = yaw;
        sim.bond.pitch = 0.0;
        sim.step_bond(Default::default(), false, false, use_key);
    }
}

/// Smoke: the level loads, the player start is the car-park placement, and 600 ticks of the real
/// Sim run without panic (drones spawn, triggers and scripts step, nothing asserts).
#[test]
fn load_and_smoke() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.level, 0x0700_0009);
    eprintln!("[smoke] start {:?}", sim.bond.pos);
    assert!((sim.bond.pos[0] - 3.59).abs() < 1.0 && (sim.bond.pos[1] + 377.0).abs() < 1.5,
        "Bond should stand at the car-park start #68, not {:?}", sim.bond.pos);
    assert_eq!(sim.drones.list.len(), 6, "6 drone placements on this level [D survey]");
    tick(&mut scene, 600);
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.level_exit.is_none(), "an idle 600-tick run must not end the level");
    assert!(!sim.drones.channels.on(0x61) && !sim.drones.channels.on(0x7c) && !sim.drones.channels.on(0x7d),
        "no alarm without a cause");
    // Objective 1 (the alarm gate) starts visible-complete: its channel (0x61) is off, so the
    // live AND for the mission must not already fire the complete (obj2's channel 0x46 is off).
    assert!(!sim.drones.channels.on(0x46) && !sim.drones.channels.on(0x32));
}

/// The alarm switch path: pressing an "alarmswitchplayer" (0x29 Switch, ch 0x7C/0x7D) or otherwise
/// raising ch 0x7C/0x7D must OR into ch 0x61 (trigger #278) and fail the mission (objective 1,
/// inverted) — Mission_Update's fail end (0x07000048) [M 0x182c60].
#[test]
fn alarm_fails_the_mission() {
    let Some(iso) = iso() else { return };
    let mut scene = load(&iso);
    let sim = scene.sim.as_mut().unwrap();
    // The game's switch (Switch_Activate 0x1b02a8) writes the channel; the OR mux #278 does the rest.
    sim.drones.set_channel(0x7c);
    // The fail goes through Mission_Update's banner (MissionFailed, 300 ticks [M 0x182c60:45-57])
    // before the LevelEnd to 0x07000048 lands.
    for _ in 0..340 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x61), "OR mux #278: ch 0x7C must raise the alarm ch 0x61");
    // The monitor: objective 1 inverted on 0x61 → ch 99 fail → Mission_Update → 0x07000048.
    assert_eq!(sim.drones.level_exit, Some(0x0700_0048), "the alarm must fail the mission [M 0x1827d8:128-141]");
    // Sister: ch 0x32 alone (in the lift) must NOT fire while the alarm is on — the exit's cancel
    // channels 0x60..0x63 [M 0x1b2318:64-75].
    let mut scene2 = load(&iso);
    let sim2 = scene2.sim.as_mut().unwrap();
    sim2.drones.set_channel(0x7d);
    for _ in 0..340 { sim2.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim2.drones.level_exit, Some(0x0700_0048));
    sim2.drones.set_channel(0x32);
    for _ in 0..5 { sim2.step_bond(Default::default(), false, false, false); }
    // The exit already fired with the fail — the level_exit stays the FIRST request.
    assert_eq!(sim2.drones.level_exit, Some(0x0700_0048), "the fail end wins");
}

/// The elevator chain end-to-end (once the activation writer is ported): ch 0x39 → AND #274 →
/// ch 0x46 (objective 2 "activate the main elevator system") → then Bond in the lift → TOUCH_ONCE
/// #67 → ch 0x32 → LOAD_LEVEL #63 → 0x0700000a.
#[test]
fn elevator_and_lift_exit_reach_0700000a() {
    let Some(iso) = iso() else { return };
    let mut scene = load(&iso);
    let exit = scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL)
        .expect("the Load Point exit exists").clone();
    assert_eq!(exit.next_level, 0x0700_000a, "the exit's destination [D param 0]");
    assert_eq!(exit.gate, 0x32, "the exit's auto-fire gate is ch 0x32 [D param 2]");
    let sim = scene.sim.as_mut().unwrap();
    // The elevator-activation channel (writer ported separately — docs/levels/07000009.md).
    sim.drones.set_channel(0x39);
    for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.channels.on(0x46), "AND mux #274 must complete objective 2's channel");
    drop(sim);
    // Bond steps into the lift (TOUCH_ONCE #67 → ch 0x32 → the gate fires).
    {
        let centre = [(exit.lo[0] + exit.hi[0]) * 0.5, exit.lo[1] + 1.0, (exit.lo[2] + exit.hi[2]) * 0.5];
        step_at(&mut scene, centre, 0.0, false);
    }
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.level_exit, Some(0x0700_000a), "in the lift → the level ends to 0700000a");
    // Sister: standing outside the lift must not end the level.
    let mut scene2 = load(&iso);
    let sim2 = scene2.sim.as_mut().unwrap();
    sim2.drones.set_channel(0x39);
    for _ in 0..5 { sim2.step_bond(Default::default(), false, false, false); }
    drop(sim2);
    {
        let centre = [(exit.lo[0] + exit.hi[0]) * 0.5, exit.lo[1] + 1.0, (exit.lo[2] + exit.hi[2]) * 0.5 + 6.0];
        step_at(&mut scene2, centre, 0.0, false);
    }
    let sim2 = scene2.sim.as_mut().unwrap();
    for _ in 0..60 { sim2.step_bond(Default::default(), false, false, false); }
    assert_ne!(sim2.drones.level_exit, Some(0x0700_000a), "outside the lift the level must not end");
}
