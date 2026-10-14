//! fix-0c-hatch: the display vent's two playtest symptoms (results 1013-1758, 0700000c):
//! 1. "the hatch actually opened very slowly" — the SS playhead rate. The game's `SS_Create`
//!    0x1a9c58:51 computes the per-tick step ONCE as `(key count − 1) / rate` (ext+0x04), and
//!    `SS_Update` 0x1a9948:103-108 advances `t += step * FRAME_RATE_MUL` (`GS_SetRefreshRate`
//!    0x17d2b8:15: MUL = 60/FRAME_RATE = 1.0 at the game's 60 Hz) — so the rate word is the whole
//!    sweep's duration in ticks: `ventmesh-front` #138 (6 keys, rate 50 [D]) opens in 50 ticks.
//!    The port moved the playhead 1/rate per tick — 250 ticks, five times slow.
//! 2. "he still can't get through because he can't crouch through" — after the hatch slides down
//!    (its top edge lands at y 15.86, the wall opening tops at y 17.50 [D #138 box]) the crouched
//!    capsule (top = feet + 0.65 + radius 0.55 [M Player_Collision 0x194df0]) fits the 1.64 gap;
//!    the real input path must carry Bond through to the display ground (y ~ 7.3 at (5.4, −113)).
//! There is no creepwall on 0700000c [D kind-census: no 0x3F row] — the game's way through is the
//! crouch, not a crawl volume.
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Load 0700000c, ride out the load, and stand Bond on the ST-1 walkway before the display vent.
fn load_at(pos: [f32; 3], yaw: f32) -> nf_view::scene::Scene {
    let iso = iso().expect("NF_ISO");
    let mut scene = nf_view::load(&iso, "0700000c.bin", false).expect("load 0700000c");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    for _ in 0..100 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, pos, yaw);
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    scene
}

/// Burn the four display-vent hinges (locks with out chs 14-17 [D #139-142 param 5]) — the laser
/// contact itself is fix-level-0c/fix-gadgets' (`burning_the_four_hinges_opens_the_display_vent`
/// drives the real beam); these tests exercise the hatch side.
fn burn_display_hinges(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    for k in 0..sim.drones.locks.len() {
        if (14..=17).contains(&sim.drones.locks[k].out) {
            sim.drones.locks[k].burn(&mut sim.drones.channels, 1);
        }
    }
}

/// Symptom 1: the hatch sweep takes the game's `rate` ticks (50 for #138), not `rate × (keys−1)`.
#[test]
fn the_hatch_sweep_takes_the_game_rate_ticks() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([6.0, 13.8, -110.5], 0.0);
    burn_display_hinges(&mut scene);
    let sim = scene.sim.as_mut().unwrap();
    let ss138 = sim.drones.sss.of_placement(138).expect("ventmesh-front SS exists [D #138]");
    let ss179 = sim.drones.sss.of_placement(179).expect("VentBlockingPoly SS exists [D #179]");
    sim.step_bond(Input::default(), false, false, false);
    assert!(sim.drones.channels.on(18), "the AND mux #505 raised ch 18 [D #505]");
    // Count the ticks from the mux's rise to each SS's last key.
    let (mut t138_done, mut t179_done) = (None, None);
    for tick in 1..=600u32 {
        sim.step_bond(Input::default(), false, false, false);
        let (t138, t179) = (sim.drones.sss.list[ss138].t, sim.drones.sss.list[ss179].t);
        if t138_done.is_none() && t138 >= 5.0 { t138_done = Some(tick); }
        if t179_done.is_none() && t179 >= 1.0 { t179_done = Some(tick); }
        if t138_done.is_some() && t179_done.is_some() { break; }
    }
    let t138_done = t138_done.expect("the hatch playhead reaches its last key");
    let t179_done = t179_done.expect("the blocker playhead reaches its last key");
    // The game: ch 18 rises on the first tick after the burn (the mux), then SS_Update 0x1a9948
    // adds (5/50) per tick — the hatch's sweep is its rate word, 50 ticks. Give the mux tick and
    // one tick of slack. The old port model (1/rate per tick) took 250 — five times slow.
    assert!(t138_done <= 55, "the hatch (#138, 6 keys, rate 50 [D]) must sweep in ~50 ticks [M 0x1a9c58:51 + 0x1a9948:107], took {t138_done}");
    // Sister: the blocker (#179, 2 keys, rate 10 [D]) — the last==1 special case advances 1/rate
    // per tick [M 0x1a9948:103-105], so its sweep is unchanged by the rate fix: ~10 ticks.
    assert!(t179_done <= 13, "the blocker (#179, rate 10 [D]) must sweep in ~10 ticks, took {t179_done}");
    // Sister: the blocker STAYS up (the channel holds, the playhead does not return).
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    assert!((sim.drones.sss.list[ss179].t - 1.0).abs() < 0.01, "the blocker stays risen while ch 18 holds");
    assert!((sim.drones.sss.list[ss138].t - 5.0).abs() < 0.01, "the hatch stays open while ch 18 holds");
}

/// Symptom 1's sister: the warehouse ventmesh #180 (6 keys, rate 50 [D]) — the same 5×-slow model
/// through its own channel (the VentLatch #182's burn → ch 84).
#[test]
fn the_warehouse_ventmesh_sweep_takes_the_game_rate_ticks() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([6.0, 13.8, -110.5], 0.0);
    let sim = scene.sim.as_mut().unwrap();
    let k = sim.drones.locks.iter().position(|l| l.out == 84).expect("the VentLatch exists [D #182]");
    sim.drones.locks[k].burn(&mut sim.drones.channels, 1);
    let ss180 = sim.drones.sss.of_placement(180).expect("ventmesh SS [D #180]");
    let mut done = None;
    for tick in 1..=600u32 {
        sim.step_bond(Input::default(), false, false, false);
        if sim.drones.sss.list[ss180].t >= 5.0 { done = Some(tick); break; }
    }
    let done = done.expect("the warehouse ventmesh reaches its last key");
    assert!(done <= 55, "the warehouse ventmesh (#180, 6 keys, rate 50 [D]) must sweep in ~50 ticks [M 0x1a9c58:51], took {done}");
}

/// Symptom 2: with all four hinges burnt and the hatch open, the real input path — crouch, walk
/// forward through the opening — carries Bond through the vent to the display ground below
/// (y ~ 7.3 at (5.4, −113)). The gap (hatch top edge 15.86 .. opening ceiling 17.50 [D #138 box])
/// takes the crouched capsule (top = feet + 1.20 [M 0x194df0]) with room to spare.
#[test]
fn bond_crouches_through_the_open_vent_to_the_display_ground() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Bond on the walkway before the vent, facing −z (yaw π: forward = (sin, ·, cos) = (0, ·, −1)).
    let mut scene = load_at([5.8, 15.8, -108.5], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    // Let the open finish even at the OLD (slow) rate, so this test isolates the traversal.
    for _ in 0..300 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        let ss138 = sim.drones.sss.of_placement(138).expect("ventmesh-front SS exists [D #138]");
        assert!((sim.drones.sss.list[ss138].t - 5.0).abs() < 0.01, "the hatch is fully open before the walk");
        let from = [5.78, 16.3, -111.7];
        assert!(sim.world.segment_hit(from, [5.78, 16.3, -113.0]).is_none(), "the opening passes a ray");
    }
    // The real input path: the crouch key (a toggle — one tick, then release; the port's uncrouch
    // needs the key re-pressed [M Player_HandleJump]), then hold forward into the opening.
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    assert!(sim.bond.crouched, "the crouch key crouched Bond");
    for tick in 1..=1200u32 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        if tick % 120 == 0 {
            let p = sim.bond.pos;
            eprintln!("tick {tick}: pos ({:.2}, {:.2}, {:.2}) crouched {} substate {}", p[0], p[1], p[2], sim.bond.crouched, sim.bond.substate);
        }
        let p = sim.bond.pos;
        if p[1] < 9.5 && p[2] < -111.5 { break; }
    }
    let p = sim.bond.pos;
    assert!(p[2] < -111.5, "Bond crosses the vent plane (z −112.25); stalled at z {:.2}", p[2]);
    assert!(p[1] < 9.5, "Bond drops to the display ground (root ~8.4 over floor 7.3 at (5.4, −113)); ended at y {:.2}", p[1]);
    assert!((p[0] - 5.4).abs() < 3.0, "Bond lands under the vent (x ~5.4), at x {:.2}", p[0]);
}
