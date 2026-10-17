//! map-castle walkthrough: 07000002 "The Exchange" (Castle B) — the exit route from the level's own
//! data. Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000002_live -- --nocapture`
//!
//! The chain [D placements + scripts; M decompile addresses cited inline]:
//! 1. Bond spawns at start #0 (−27.17, 1.64, 29.80, gate 0) [D placement probe].
//! 2. The intro cut-scene SP #400 (script 0x06000084 from pack 07800002, mode 1) plays at level load;
//!    its stream 7 posts event 0xD ch 0x47 = 1 [D script decode] — the arc's stage-0 completion channel
//!    (the castle-A objective watches ch 0x47 [M the mission table 0x2a3eb8; Mission_MonitorObjectives
//!    0x1827d8], so the story continues from level A's objective).
//! 3. The exit = placement #206 "Load Point 2" (0xE8 LOAD_LEVEL, next 0x07000003, **key gate ch 0x4C**,
//!    no gate channel) at (−10.05, 6.75, −23.18). The E-key path is `Trigger_Activate` 0x1b2318: the key
//!    gate (param 1) must be ON or the activation is refused with the `Txt_BindLabel(0x2000021)` message
//!    [M 0x1b2318:76-90]; then `ResetMap_LevelToLoad` + `Player_RamSave` [M 0x1b2318:111-121].
//! 4. The mission's three objectives (the table 0x2a3ed0) watch ch 0x48 / 0x4B / 0x4D; the AND trigger
//!    #303 (MultiplexIn out 0x4B = AND(0x49, 0x4A)) builds one of them from touchplate chains. The
//!    writer of ch 0x4C (the key gate) IS on the disc — AND #480 (out 0x4C = AND(0x48, 0x4B); the
//!    correction and the census are in docs/levels/07000002.md) — but 0x48's other half (the 0x4B
//!    chain's 0x49/0x4A inputs) belongs to the un-ported mission/dialogue layer, so the test drives
//!    the gate channel directly and asserts the real refusal + load behaviour both ways.

use nf_formats::iso9660::IsoImage;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The level's exit: E on the Load Point with the key-gate channel off is REFUSED (the "you need
/// something" message path [M 0x1b2318:80-88]); with the channel on it loads 07000003. Sister: without
/// the E press the level never ends.
#[test]
fn the_load_point_refuses_without_the_key_channel_and_loads_07000003_with_it() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0002);
    let exit = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL)
        .expect("the Load Point 2 (0xE8) exists");
    assert_eq!(exit.next_level, 0x0700_0003);
    assert_eq!(exit.key_gate, 0x4c);
    let centre = [(exit.lo[0] + exit.hi[0]) * 0.5, (exit.lo[1] + exit.hi[1]) * 0.5, (exit.lo[2] + exit.hi[2]) * 0.5];
    // Bond at the Load Point, presses E: refused while ch 0x4C is off.
    sim.bond.pos = centre;
    for _ in 0..6 { sim.step_bond(Default::default(), false, false, true); }
    assert_eq!(sim.drones.level_exit, None, "the key gate must refuse the exit while ch 0x4C is off");
    assert!(!sim.drones.channels.on(0x4c));
    // With the gate channel on (the un-ported lock/dialogue interaction's write), the same E loads the level.
    sim.drones.set_channel(0x4c);
    for _ in 0..6 { sim.step_bond(Default::default(), false, false, true); }
    assert_eq!(sim.drones.level_exit, Some(0x0700_0003), "the exit must load 07000003 once the key gate is on");
    // The reload path (the viewer's own next-level step).
    let next = nf_view::level_archive_name(0x0700_0003).to_string();
    let scene2 = nf_view::load(&iso_v, &next, false).expect("07000003 must load after the exit");
    assert_eq!(scene2.sim.as_ref().expect("sim").level, 0x0700_0003);
}

/// Sister: standing at the Load Point with the gate channel on but NOT pressing E never ends the level.
#[test]
fn the_load_point_needs_the_use_press() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000002.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let exit = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL).unwrap();
    sim.bond.pos = [(exit.lo[0] + exit.hi[0]) * 0.5, (exit.lo[1] + exit.hi[1]) * 0.5, (exit.lo[2] + exit.hi[2]) * 0.5];
    sim.drones.set_channel(0x4c);
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.level_exit, None, "no E press, no exit");
}
