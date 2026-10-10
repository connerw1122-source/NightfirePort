//! Headless tests for every F2 dev-menu action (B-10).
//!
//! The user reported that "Spawn guard" crashed in play. Every menu action must run without panicking
//! (the result message is shown on screen; a panic kills the window). Each test loads the scene, applies
//! the action through the real `dev` API (the same path the viewer's `apply_menu_action` uses), ticks the
//! sim a few frames, and asserts no panic + a sane result. Opt-in via `NF_ISO`.
//!
//! These tests also serve as regression guards: if a future change breaks a menu action, the test fails
//! instead of the user's window crashing.

use nf_view::dev;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A loaded scene + dev state, the same path the viewer uses. Bond is placed at the level start.
fn loaded() -> Option<(nf_view::scene::Scene, dev::Dev)> {
    let iso = iso()?;
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let dev = dev::Dev { scenario: None, ..Default::default() };
    Some((scene, dev))
}

/// Tick the sim `n` times with idle input (no panic is the assertion).
fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n {
            // `step_bond` is the real per-tick path the viewer uses (input → Sim → output).
            sim.step_bond(Default::default(), false, false, false);
        }
    }
}

#[test]
fn menu_spawn_normal_guard_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.spawn = dev::SpawnPick::Normal;
    // Spawn at a known-open spot (Bond's start on 07000005 is open ground).
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.spawn_guard(&mut scene, pos, 0.0);
    assert!(gi.is_some(), "spawn_guard returned None — no drone created");
    // Tick 60 frames; the new drone's first tick (set_state, anim, vision) must not panic.
    tick(&mut scene, 60);
    let gi = gi.unwrap();
    let dr = &scene.sim.as_ref().unwrap().drones.list[gi];
    assert!(dr.present, "spawned guard not present after 60 ticks");
    eprintln!("[menu_test] spawn Normal: guard #{gi} state 0x{:x} health {}", dr.state, dr.health);
}

#[test]
fn menu_spawn_sniper_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.spawn = dev::SpawnPick::Sniper;
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.spawn_guard(&mut scene, pos, 0.0);
    assert!(gi.is_some(), "spawn_guard (sniper) returned None");
    tick(&mut scene, 60);
    eprintln!("[menu_test] spawn Sniper: ok");
}

#[test]
fn menu_spawn_ambush_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.spawn = dev::SpawnPick::Ambush;
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.spawn_guard(&mut scene, pos, 0.0);
    assert!(gi.is_some(), "spawn_guard (ambush) returned None");
    tick(&mut scene, 60);
    eprintln!("[menu_test] spawn Ambush: ok");
}

#[test]
fn menu_force_grenade_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    // Need a guard near Bond to force. Spawn one first, then force.
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    dev.spawn = dev::SpawnPick::Normal;
    let _ = dev.spawn_guard(&mut scene, pos, 0.0);
    let bond = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.force_grenade(&mut scene, bond);
    assert!(gi.is_some(), "force_grenade found no guard near Bond");
    tick(&mut scene, 60);
    eprintln!("[menu_test] force grenade: ok");
}

#[test]
fn menu_impulse_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    dev.spawn = dev::SpawnPick::Normal;
    let _ = dev.spawn_guard(&mut scene, pos, 0.0);
    let bond = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.impulse_guard(&mut scene, bond);
    assert!(gi.is_some(), "impulse_guard found no guard near Bond");
    tick(&mut scene, 30);
    eprintln!("[menu_test] impulse: ok");
}

#[test]
fn menu_special_death_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    dev.spawn = dev::SpawnPick::Normal;
    let _ = dev.spawn_guard(&mut scene, pos, 0.0);
    let bond = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let gi = dev.special_death(&mut scene, bond);
    assert!(gi.is_some(), "special_death found no guard near Bond");
    tick(&mut scene, 60);
    eprintln!("[menu_test] special_death: ok");
}

#[test]
fn menu_toggle_god_and_ammo_do_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.god_mode = true;
    dev.infinite_ammo = true;
    dev.after_tick(&mut scene);
    dev.god_mode = false;
    dev.infinite_ammo = false;
    dev.after_tick(&mut scene);
    tick(&mut scene, 10);
    eprintln!("[menu_test] toggle god/ammo: ok");
}

#[test]
fn menu_teleport_save_restore_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    let p = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    dev.saved_spot = Some(p);
    dev.teleport(&mut scene, p, 0.0);
    tick(&mut scene, 10);
    eprintln!("[menu_test] teleport save/restore: ok");
}

#[test]
fn menu_spawn_then_animate_does_not_panic() {
    // Reproduces the viewer's "Spawn guard crashed" report: the crash happens in the render/pose path after a
    // spawn (the headless tick-only test doesn't exercise it). This calls `scene.animate` — the same pose loop
    // the renderer's `shot()` uses — after spawning, to catch any indexing panic on the new drone.
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.spawn = dev::SpawnPick::Normal;
    let pos = scene.sim.as_ref().map(|s| s.bond.pos).unwrap_or([0.0; 3]);
    let _ = dev.spawn_guard(&mut scene, pos, 0.0);
    tick(&mut scene, 5);
    // The pose loop the renderer runs each frame. `choice = None` is the normal AI-driven path.
    let opts = nf_view::scene::ViewOpts { reinforcements: false, firing: false, right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0] };
    let _poses = scene.animate(None, 1.0, opts);
    eprintln!("[menu_test] spawn + animate: ok (no panic in pose loop)");
}

#[test]
fn menu_timescale_does_not_panic() {
    let Some((mut scene, mut dev)) = loaded() else { eprintln!("NF_ISO not set — skipping"); return };
    dev.time_scale = 0.25;
    let n = dev.tick_count(4);
    assert_eq!(n, 1, "0.25× of 4 ticks should be 1");
    dev.time_scale = 0.0;
    let n = dev.tick_count(4);
    assert_eq!(n, 0, "paused should be 0 ticks");
    dev.step = true;
    let n = dev.tick_count(0);
    assert_eq!(n, 1, "step one tick when paused");
    tick(&mut scene, n);
    eprintln!("[menu_test] time scale: ok");
}
