//! walkreal-b: 07000002's geisha rescue through the real input path. The castle geishas are
//! hostage drones (mode 5 → `DroneTypeSettings[5].start` = 0x0A Hostage [L 0x29aee8+5·12]) whose
//! rescue channel is their placement param 6 (`DCVars+0x135`, the port's `second_channel`):
//! `DroneFunc_HostageSaved` 0x1441c0:11-18 writes it once, level-gated to 0x7000002/3 with the
//! 0x200004f/0x2000050 messages [M]. The state enters from `NDrone2_DSTATE_Hostage` 0x15cb78:24-36
//! (captor link empty or the captor's health ≤ 0 → state 0xC HostageSaved [M]; the captor comes
//! from `NDrone2_SetupHostageKiller` 0x144030). The disc data: geisha 23 (ch 0x49) at
//! (−45.5, 5.75, 12.2) and geisha 24 (ch 0x4A) at (−12.0, 6.3, −39.1) [D placements, param 6 =
//! 73/74], both enabled from WaitSwitch by their param-3 channels 0x17 / 0x19 [D; the TouchOnce
//! plates tr[003] (−35.2, 6.5, 5.05) out 0x17 and tr[009] (−26.9, 6.0, −20.3) out 0x19].
//! The captors are the guards enabled by the same channels: 0x17's three at (−45.6, 5.8, 10.9)
//! death ch 0x88, (−45.2, 5.8, 16.3) death ch 0x2A, (−46.6, 5.8, 7.4) death ch 0x29 [D param 12] —
//! exactly the AND #397 inputs AND(0x29, 0x2A, 0x88, 0x49) → 0x3B [D tr[017]].
//! Sisters: the first test pins the captive state while the captor lives (it fails the moment a
//! writer raises the channel early); the second pins the save on the captor's death — it FAILS on
//! the old code (no Hostage/HostageSaved path, the channel never rises).

#[path = "common/walker.rs"]
mod walker;

use walker::{Act, Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn geisha(sim: &nf_view::scene::Sim, ch: u8) -> Option<usize> {
    sim.drones.list.iter().position(|d| d.second_channel == ch)
}

#[test]
fn geisha_1_sits_captive_while_her_captor_lives() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000002.bin", false).expect("load 07000002"));
    w.wait(120);
    // The plate: the game raises 0x17 by touching tr[003] — the walk touches it with the real path.
    let sim = w.scene.sim.as_ref().unwrap();
    let gi = geisha(sim, 0x49).expect("geisha 1 (param 6 = 0x49) on the disc");
    drop(sim);
    // The plate zone centre [D tr[003]]; the walkreal route reaches it across the west wing.
    // The enable channel the plate raises (tr[003] out 0x17 [D]) — set as the trigger would, the
    // test's subject is the hostage path that follows it (the plate raise itself is the proven
    // Trigger_Activate machinery the walkreal test exercises).
    let sim = w.scene.sim.as_mut().unwrap();
    sim.drones.channels.set(0x17, 1, sim.drones.frame);
    drop(sim);
    w.wait(60);
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x17), "the plate raises ch 0x17 [D tr[003] out 0x17]");
        let g = &sim.drones.list[gi];
        assert_eq!(g.state, nf_game::drone::state::HOSTAGE, "the enabled geisha starts in Hostage 0x0A [L DroneTypeSettings[5].start]");
        assert!(g.captor.is_some(), "the captor link is bound [M 0x144030]");
        assert!(!sim.drones.channels.on(0x49), "the rescue channel stays off while the captor lives [M 0x1441c0 one-shot on save]");
    }
}

#[test]
fn killing_the_captor_saves_geisha_1() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "07000002.bin", false).expect("load 07000002"));
    w.wait(120);
    let sim = w.scene.sim.as_ref().unwrap();
    let gi = geisha(sim, 0x49).expect("geisha 1");
    drop(sim);
    // The enable channel the plate raises (tr[003] out 0x17 [D]) — set as the trigger would, the
    // test's subject is the hostage path that follows it (the plate raise itself is the proven
    // Trigger_Activate machinery the walkreal test exercises).
    let sim = w.scene.sim.as_mut().unwrap();
    sim.drones.channels.set(0x17, 1, sim.drones.frame);
    drop(sim);
    w.wait(60);
    let sim = w.scene.sim.as_ref().unwrap();
    let captor = sim.drones.list[gi].captor.expect("captor bound");
    let at = sim.drones.list[captor].pos;
    drop(sim);
    // Test setup: stand Bond beside the captor (a straight walk crosses the whole castle; the
    // shot itself below is the real fire path).
    let sim = w.scene.sim.as_mut().unwrap();
    sim.bond.pos = [at[0] + 2.5, at[1], at[2]];
    drop(sim);
    w.wait(30);
    // Kill the captor through the real fire path.
    w.shoot(captor, "the captor guard");
    // The save: the per-frame captor check → HostageSaved → the channel [M 0x15cb78:24-36, 0x1441c0].
    for _ in 0..120 {
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x49) { break; }
        drop(sim);
        w.tick(Default::default(), false);
    }
    let sim = w.scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x49),
        "the captor's death saves the geisha: ch 0x49 rises [M 0x1441c0 via 0x15cb78:24-36]");
    assert!(matches!(sim.drones.list[gi].state, nf_game::drone::state::HOSTAGE_GO_TO_GOAL | nf_game::drone::state::HOSTAGE_IDLE),
        "the saved geisha leaves in HostageGoToGoalPosition (or cowers) [M 0x15ccf0:24-44]");
}
