//! hatch-crouch-0c: the 0700000c display-vent traversal end to end (playtests 1012-1723,
//! 1013-1758, 1014-1725: "he still can't get through because he can't crouch through" — Bond
//! blocked and soft-locked under the hatch). The route: from the duct/hatch top the open hatch's
//! hole drops the crouched capsule onto the inner ledge/balcony (14.61–14.81), and the balcony
//! runs SOUTH past the vent well to the far walkway (14.61 at z −126.5+); the drop leg needs the
//! crouch (the hole's clear band over the open panel), the south leg is the balcony floor.
//! The blocker this task fixes: the port grounded Bond only off the floor ray / a walkable
//! (ny > 0.7) push, while the game grounds on ANY capsule pick contact
//! (`Player_FeetOnPoint` 0x194188:176-185 sets `obj+0x60 |= 8` when `HITTEST+0x9c != 0` OR the
//! floor ray lands within `cc + 0.1` [M 0x194188:106]; the history shift at `Player_CollisionHandler`
//! 0x1949a8:78-82). Riding the balcony's south pier ramp (14.61 → 16.49 over z −121..−122) the
//! push lifts the capsule past the ray tolerance (crouch rest 0.646 vs the 0.55+0.1 gate) and
//! Bond bounced ungrounded at the base forever — the playtest's stuck-at-the-hatch symptom.
//! The crouched capsule (top = feet + 1.20 = cc 0.55 + 0.65 [M Player_Collision 0x194df0 case 3:
//! radius 0x3f0ccccd, A = 0.1 − (cc − 0.55), B = 0.45 − cc]) is the ported model; the standing
//! slim arm (case 0/6: radius 0x3e8ccccd = 0.275, A = pos + 0.55, B = pos + 0.275 − cc) gives the
//! same [0, feet+1.86] envelope, so the standing block at the pier is geometric, not a stance bug.
//!
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Load 0700000c, ride out the load, and stand Bond at `pos` facing `yaw`.
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

/// Burn the four display-vent hinges (locks with out chs 14–17 [D #139-142 param 5]); the AND mux
/// #505 raises ch 18 and `ventmesh-front` #138 sweeps down in its rate-50 ticks [M 0x1a9c58:51].
fn burn_display_hinges(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    for k in 0..sim.drones.locks.len() {
        if (14..=17).contains(&sim.drones.locks[k].out) {
            sim.drones.locks[k].burn(&mut sim.drones.channels, 1);
        }
    }
}

/// The task's end-to-end: from the hatch top, open the hatch, drop through the hole crouched, and
/// land grounded on the balcony behind the wall (the far room's ledge).
#[test]
fn from_the_hatch_top_the_open_hole_drops_the_crouch_onto_the_balcony() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // Bond on the duct ramp (top 17.17–17.50 [D probes]) facing the wall (−z).
    let mut scene = load_at([5.8, 18.6, -110.8], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..300 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        let ss138 = sim.drones.sss.of_placement(138).expect("ventmesh-front SS exists [D #138]");
        // The game's observable is the POSE: after the sweep's wrap (`(int)last < (int)t` → t=0,
        // state 0/0x40 [M SS_Update 0x1a9948:119-128]) no state re-poses the matrix, so the hatch
        // holds its last key's pose (open) while the playhead rests at 0 — grates-02's state-driven
        // SS_Update model (the pre-grates channel-seek `t == 5.0` observable is stale on master).
        let last138 = (sim.drones.sss.list[ss138].keys.len() - 1) as u16;
        let open138 = nf_game::door::interp(&sim.drones.sss.list[ss138].keys, 0, last138, last138 as f32, sim.drones.sss.list[ss138].spline).unwrap();
        assert_eq!(sim.drones.sss.list[ss138].matrix, open138, "the hatch is fully open before the drop (the held pose is the last key's)");
        let p = sim.bond.pos;
        assert!(p[1] > 16.5, "Bond stands on the duct ramp; at y {:.2}", p[1]);
    }
    // The crouch (a toggle — one tick, then release), then hold forward off the duct edge.
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    assert!(sim.bond.crouched, "the crouch key crouched Bond");
    let mut landed = None;
    for tick in 1..=1200u32 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        let p = sim.bond.pos;
        // Through the wall plane (z −112.25) and down onto the balcony (top 14.61–14.81): the
        // crouched root settles at floor + 0.646 (cc 0.55 + the capsule's −0.10 bottom).
        if p[2] < -112.6 && p[1] < 15.5 && sim.bond.grounded { landed = Some((tick, p)); break; }
    }
    let (tick, p) = landed.expect("the crouch drops through the open hole onto the balcony");
    assert!(sim.bond.grounded, "landed grounded");
    assert!(p[1] > 14.5 && p[1] < 15.5, "root over the balcony floor 14.61-14.81, at y {:.2}", p[1]);
    assert!(p[0] > 4.5 && p[0] < 7.0, "landed under the vent opening (x 4.54..7.02 [D #138]), at x {:.2}", p[0]);
    eprintln!("landed on the balcony at tick {tick}: ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]);
}

/// The south leg: crouch-walking the balcony south past the vent well reaches the far walkway
/// (floor 14.61 continuing at z −126.5+ [D probes]). Crouched, as the playtester travelled.
#[test]
fn crouch_walking_the_balcony_south_reaches_the_far_walkway() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // On the balcony's east half (x 11) facing −z; the pier ramp sits at x ≤ 9.
    let mut scene = load_at([11.0, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    assert!(sim.bond.crouched, "the crouch key crouched Bond");
    let mut arrived = None;
    for tick in 1..=400u32 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        let p = sim.bond.pos;
        if p[2] < -125.0 { arrived = Some((tick, p)); break; }
    }
    let (tick, p) = arrived.expect("the crouch-walk crosses the well's south edge (z −121.5) to the far walkway");
    assert!(p[1] > 15.0 && p[1] < 15.5, "still on the 14.61 floor (crouch root ~15.26), at y {:.2}", p[1]);
    eprintln!("reached the far walkway at tick {tick}: ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]);
}

/// The fix's regression: riding the balcony's south pier ramp (14.61 → 16.49 over z −121..−122,
/// a ~58° face [D probes]) the crouched capsule STAYS GROUNDED (the pick-contact arm of the
/// game's grounded rule) and rides up; before the fix it bounced ungrounded (y 15.31..15.62,
/// never past z −120.85) — the playtest's "can't crouch through" soft-lock.
#[test]
fn the_south_pier_ramp_ride_stays_grounded_and_climbs() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    let mut rode = None;
    for tick in 1..=400u32 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        let p = sim.bond.pos;
        if p[2] < -121.3 && p[1] > 16.3 && sim.bond.grounded { rode = Some((tick, p)); break; }
    }
    let (tick, p) = rode.expect("the crouch rides the ramp face grounded past z −121.3 and y 16.3 \
        (the pre-fix bounce never left z −120.85 / y 15.62)");
    assert!(sim.bond.grounded, "grounded on the ramp contact");
    eprintln!("riding the ramp at tick {tick}: ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]);
}

/// The sister: STANDING is blocked at the south pier (the slim standing capsule gets no lift off
/// the ~58° face and the 0.25 lip) — the walk south never crosses z −121.5, while the crouched
/// east route above crosses to z −125.
#[test]
fn standing_is_blocked_at_the_south_pier() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_mut().unwrap();
    let mut deepest = -116.0f32;
    for tick in 1..=400u32 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        deepest = deepest.min(sim.bond.pos[2]);
    }
    assert!(deepest > -121.5, "standing stays blocked at the pier base; reached z {:.2}", deepest);
}
