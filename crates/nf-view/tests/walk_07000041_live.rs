//! map-finale: the 07000041 walkthrough (docs/levels/07000041.md). The scripted interlude wrapper:
//! the load, the player-touch plate (#100, out ch 0x6e [D]) that unlocks the scripted-drone plate
//! (#99, lock 0x6e), and the five tether wires the fix-wire crossing covers. The wrapper's own end
//! is driven by the driving-side flow (out of scope — see the doc). Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_wrapper_starts_its_chain_and_the_tethers_are_ridable() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000041.bin", false).expect("load 07000041");
    let sim = scene.sim.as_mut().unwrap();
    let p = sim.bond.pos;
    assert!(p[0] > 25.0 && p[0] < 29.0 && p[2] > -32.0 && p[2] < -28.0,
        "the wrapper's start places Bond at the airlock (#1 [D]), got {p:?}");
    // No placed drones: every character is a script object [D survey].
    assert!(sim.drones.list.is_empty(), "no placed drones on the wrapper [D]");
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    // The player-touch plate #100 (14.51-16.71, 0..0.75, -34.32..-32.12 [D]): walk into it.
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [15.5, 1.4, -30.0], std::f32::consts::PI);
    let mut input = Input::default();
    input.forward = 1.0;
    for _ in 1..=200 { sim.step_bond(input, false, false, false); if sim.drones.channels.on(0x6e) { break; } }
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.channels.on(0x6e), "the player plate #100 fires ch 0x6e [D]");
    // The drone half (#99, mask 0x4, lock 0x6e [D]) needs the scripted drone — not ported (the doc's
    // §5): ch 0x01 stays off and the wrapper does not end.
    assert!(!sim.drones.channels.on(0x01), "the scripted-drone plate #99 never fires without the script entity [D]");
    assert_eq!(sim.drones.level_exit, None, "the wrapper has no level-exit trigger [D]");
    // The five tether wires parsed for the fix-wire crossing [D placements #26/#36/#102/#103/#104].
    assert_eq!(sim.wires.len(), 5, "the tether cables parse as 0x3E wires [D]");
}
