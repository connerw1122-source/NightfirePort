//! map-castle walkthrough: 07000003 "Alpine Escape" (Castle C) — the exit route from the level's own
//! data. Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000003_live -- --nocapture`
//!
//! The chain [D placements + scripts; M decompile addresses cited inline]:
//! 1. Bond spawns at the 0x24 start (gate 0) [D placement probe].
//! 2. The exit = placement #451 "Load Point" (0xE8 LOAD_LEVEL, next 0x07000004, **key gate ch 0x52**, no
//!    gate channel) at (18.11, 6.65, 16.05). Same `Trigger_Activate` 0x1b2318 key-gate refusal as
//!    07000002's [M 0x1b2318:76-90].
//! 3. The mission's three objectives (the table 0x2a3f18) watch ch 0x4E / 0x51 / 0x53; the script
//!    0x06000563 writes ch 0x96 (the searchlight alarm / monitor channel [D script decode]). The writers
//!    of the key-gate ch 0x52 are NOT on the disc (scanned: scripts, triggers, drone death channels,
//!    pickups, doors, locks — see docs/levels/07000003.md); the test drives it directly and asserts the
//!    real refusal + load behaviour both ways. The level carries 1 ladder at (34.2, 7.6, 18.2) and 5
//!    water cels (the garden moat) [D levels.md survey] — the ported ladder/water systems cover them.

use nf_formats::iso9660::IsoImage;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// E on the Load Point: refused without the key channel, loads 07000004 with it. Sister: no E, no exit.
#[test]
fn the_load_point_refuses_without_the_key_channel_and_loads_07000004_with_it() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000003.bin", false).expect("load 07000003");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0003);
    let exit = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL)
        .expect("the Load Point (0xE8) exists");
    assert_eq!(exit.next_level, 0x0700_0004);
    assert_eq!(exit.key_gate, 0x52);
    let centre = [(exit.lo[0] + exit.hi[0]) * 0.5, (exit.lo[1] + exit.hi[1]) * 0.5, (exit.lo[2] + exit.hi[2]) * 0.5];
    sim.bond.pos = centre;
    for _ in 0..6 { sim.step_bond(Default::default(), false, false, true); }
    assert_eq!(sim.drones.level_exit, None, "the key gate must refuse the exit while ch 0x52 is off");
    sim.drones.set_channel(0x52);
    for _ in 0..6 { sim.step_bond(Default::default(), false, false, true); }
    assert_eq!(sim.drones.level_exit, Some(0x0700_0004), "the exit must load 07000004 once the key gate is on");
    let next = nf_view::level_archive_name(0x0700_0004).to_string();
    let scene2 = nf_view::load(&iso_v, &next, false).expect("07000004 must load after the exit");
    assert_eq!(scene2.sim.as_ref().expect("sim").level, 0x0700_0004);
}

/// Sister: with the gate channel on but no E press the level never ends.
#[test]
fn the_load_point_needs_the_use_press() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000003.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let exit = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL).unwrap();
    sim.bond.pos = [(exit.lo[0] + exit.hi[0]) * 0.5, (exit.lo[1] + exit.hi[1]) * 0.5, (exit.lo[2] + exit.hi[2]) * 0.5];
    sim.drones.set_channel(0x52);
    for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.level_exit, None, "no E press, no exit");
}
