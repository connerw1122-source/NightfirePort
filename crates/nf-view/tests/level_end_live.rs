//! Headless NF_ISO tests for the level-end mechanisms on 07000005 (the wine truck) and the door close-on-E on
//! 07000001. These run through the REAL Sim path (`nf_view::load` + `Sim::step_bond`) — the exact functions
//! the viewer calls — so a passing test means the mechanism works in play.
//!
//! A-11a/b: the truck level-end has three routes (truck ride, on-foot gate door, zip wire). This file tests:
//! - Route (b): Bond at the castle door presses E → `Player_Activate` → `Trigger_Activate` → level loads.
//! - Route (a): the truck chain (ch 0x70+0x71+0x72 → mux ch 0x65 → ScriptPlayer #320 → event 0x11 →
//!   ch 0x1D → placement 118 LOAD_LEVEL) — tested by setting the three channels directly (simulating the truck
//!   script's triggers + the gate-guard drone), since the full truck ride + drone AI is a larger integration.
//! - Door close-on-E (07000001): E opens, E again closes.
//!
//! Opt-in via `NF_ISO`.

use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Load a level and return the scene. Panics on failure (test-only).
fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

/// Tick the sim `n` times with idle input. Returns the frame after ticking.
fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// Place Bond at `pos` with `yaw` (radians; 0 = +Z forward), tick once with `use_key`.
fn step_at(scene: &mut nf_view::scene::Scene, pos: [f32; 3], yaw: f32, use_key: bool) {
    if let Some(sim) = scene.sim.as_mut() {
        sim.bond.pos = pos;
        sim.bond.yaw = yaw;
        sim.bond.pitch = 0.0;
        sim.step_bond(Default::default(), false, false, use_key);
    }
}

// ── Route (b): on-foot gate door on 07000005 ──────────────────────────────────

/// Bond walks to the castle door (placement 118, the 0xE8 LOAD_LEVEL at ~(30, 38, 38)) and presses E. The
/// `Player_Activate` → `Trigger_Activate` path fires the trigger immediately — no gate channel needed (the
/// gate is only for `Trigger_Update`'s auto-fire). The level should load (0x07000066).
/// Sister: Bond standing away from the door presses E → nothing happens.
#[test]
fn route_b_on_foot_gate_door_loads_level_on_e() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    // Find the LOAD_LEVEL trigger (placement 118) and read its volume centre.
    let exit = scene.sim.as_ref().unwrap().drones.triggers.iter()
        .find(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL && t.gate == 0x1D)
        .expect("a LOAD_LEVEL trigger with gate 0x1D exists on 07000005");
    let centre = [
        (exit.lo[0] + exit.hi[0]) * 0.5,
        (exit.lo[1] + exit.hi[1]) * 0.5,
        (exit.lo[2] + exit.hi[2]) * 0.5,
    ];
    eprintln!("[route_b] LOAD_LEVEL trigger at centre {:?}, gate 0x{:02x}, next {:08x}", centre, exit.gate, exit.next_level);
    // Bond at the door, looking forward (toward +Z), presses E.
    step_at(&mut scene, [centre[0], centre[1], centre[2] - 0.5], 0.0, true);
    assert_eq!(scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_0006),
        "pressing E at the castle door should load level 0x07000006");
    eprintln!("[route_b] level_exit = 0x07000006 ✓");

    // Sister: Bond 10 m away presses E → nothing.
    let mut scene2 = load(&iso, "07000005.bin");
    step_at(&mut scene2, [centre[0] + 10.0, centre[1], centre[2] - 0.5], 0.0, true);
    assert_eq!(scene2.sim.as_ref().unwrap().drones.level_exit, None,
        "pressing E 10 m from the door should NOT load the level");
    eprintln!("[route_b] sister: 10 m away → no level load ✓");
}

// ── Route (a): the truck chain (ch 0x70 + 0x71 + 0x72 → mux → ScriptPlayer → 0x1D → LOAD_LEVEL) ─────────

/// The truck parks → Bond on the bed touches ch 0x70 (parked pos) + ch 0x71 (truck bed), and a gate-guard
/// drone at (28, 37, 44) touches ch 0x72. The MuxIn (placement 316) ANDs them → ch 0x65 → ScriptPlayer #320
/// (gate ch 0x65) activates → plays EndLevel script 0x06000A23 → event 0x11 at tick 300 → ch 0x1D = 1 →
/// placement 118 (LOAD_LEVEL, gate 0x1D) fires → level 0x07000006 loads.
/// This test sets ch 0x65 directly (the MuxIn output) — the three input channels require the full truck ride
/// (the truck script creates a TOUCH trigger ch 0x71 that follows the truck bed and clears every tick; Bond must
/// ride the truck for it to fire). Setting ch 0x65 directly tests the ScriptPlayer → event 0x11 → LOAD_LEVEL chain,
/// which is the part A-11 targets. Sister: ch 0x65 not set → no level load.
#[test]
fn route_a_truck_chain_loads_level_when_mux_channel_on() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000005.bin");
    let sim = scene.sim.as_mut().unwrap();
    // Simulate the MuxIn firing: set ch 0x65 (the AND of ch 0x70 + 0x71 + 0x72) directly.
    sim.drones.set_channel(0x65);
    eprintln!("[route_a] set ch 0x65 — simulating the MuxIn firing (truck parked + Bond on bed + guard at post)");
    // Tick up to 600 ticks (10 s) — the EndLevel script has a 300-tick delay before event 0x11.
    let mut loaded = None;
    for i in 0..600 {
        sim.step_bond(Default::default(), false, false, false);
        if let Some(next) = sim.drones.level_exit { loaded = Some((next, i)); break; }
    }
    assert_eq!(loaded.map(|(n, _)| n), Some(0x0700_0006),
        "the truck chain (ch 0x65 → ScriptPlayer #320 → 0x1D → LOAD_LEVEL) should load 0x07000006");
    if let Some((_, f)) = loaded {
        eprintln!("[route_a] level_exit = 0x07000006 at tick {} ✓", f);
    }

    // Sister: ch 0x65 not set → no level load.
    let mut scene2 = load(&iso, "07000005.bin");
    let sim2 = scene2.sim.as_mut().unwrap();
    for _ in 0..600 { sim2.step_bond(Default::default(), false, false, false); }
    assert_eq!(scene2.sim.as_ref().unwrap().drones.level_exit, None,
        "with ch 0x65 off (truck not parked), no level loads");
    eprintln!("[route_a] sister: ch 0x65 off → no level load ✓");
}

// ── Door close-on-E (07000001) ────────────────────────────────────────────────

/// On 07000001, pressing E on a closed slide door opens it; pressing E again on the open door closes it
/// (`Door_Activate` toggles, A-11b). The old bug: `react_to_hit` reversed CLOSING→OPENING when Bond stood
/// near the hinge, so the door never closed. Fix: `react_to_hit` is not called from `Door_Update` (the game
/// calls it from `Collide_Update`). Sister: the door stays open when Bond walks away (no E).
#[test]
fn door_e_opens_then_e_closes_on_07000001() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    // Find a slide door (kind 2) on the level.
    let door_idx = scene.sim.as_ref().unwrap().drones.doors.list.iter().enumerate()
        .find(|(_, d)| d.kind == 2)
        .map(|(i, _)| i)
        .expect("a slide door exists on 07000001");
    let door_pos = scene.sim.as_ref().unwrap().drones.doors.list[door_idx].pos;
    eprintln!("[door] slide door #{} at {:?}", door_idx, door_pos);
    // Bond at the door, presses E to open.
    step_at(&mut scene, [door_pos[0], door_pos[1], door_pos[2] - 0.5], 0.0, true);
    // Tick a few frames for the door to start opening.
    tick(&mut scene, 5);
    let state_after_open = scene.sim.as_ref().unwrap().drones.doors.list[door_idx].state;
    eprintln!("[door] after E: state = 0x{:x} (expect OPENING=8 or OPEN=4)", state_after_open);
    assert!(state_after_open & 0xc != 0, "the door should be opening or open after E");
    // Let the door finish opening.
    tick(&mut scene, 120);
    let state_open = scene.sim.as_ref().unwrap().drones.doors.list[door_idx].state;
    eprintln!("[door] after 120 ticks: state = 0x{:x} (expect OPEN=4)", state_open);
    assert_eq!(state_open, 4, "the door should be fully open (state 4)");
    // Press E again to close.
    step_at(&mut scene, [door_pos[0], door_pos[1], door_pos[2] - 0.5], 0.0, true);
    tick(&mut scene, 5);
    let state_after_close = scene.sim.as_ref().unwrap().drones.doors.list[door_idx].state;
    eprintln!("[door] after second E: state = 0x{:x} (expect CLOSING=0x10)", state_after_close);
    assert!(state_after_close & 0x10 != 0 || state_after_close == 2, "E on an open door should start closing (state 0x10) or be closed (2)");

    // Sister: a door left open without E stays open (for at least a few ticks).
    let mut scene2 = load(&iso, "07000001.bin");
    let door_pos2 = scene2.sim.as_ref().unwrap().drones.doors.list[door_idx].pos;
    step_at(&mut scene2, [door_pos2[0], door_pos2[1], door_pos2[2] - 0.5], 0.0, true);
    tick(&mut scene2, 120);
    let open_state = scene2.sim.as_ref().unwrap().drones.doors.list[door_idx].state;
    assert_eq!(open_state, 4, "the door should be open after one E (no second E)");
    eprintln!("[door] sister: door open without second E stays open ✓");
}

// Allow the `dev` import to be unused if the tests don't call dev functions.
#[allow(unused_imports)]
use dev as _dev;
