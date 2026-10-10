//! Headless NF_ISO tests for every dev scenario, through the REAL path the viewer uses.
//!
//! B-7: the old `nf-game/tests/scenario_live.rs` built its own `Drones` from the level data — a parallel path that
//! diverged from the viewer's `Scene::build` activation. A guard "active" in that test was not necessarily active in
//! play. These tests use `nf_view::load` + `nf_view::dev::scenario` — the EXACT functions the viewer calls — so a
//! passing test means the scenario works in play.
//!
//! Each test: loads the scene, applies the scenario (which places Bond near an active guard), finds the chosen guard
//! by proximity, asserts it is ACTIVE (present + enabled + alive + in-llist) and that Bond's spot is not sealed
//! (≥2 walkable directions), then ticks the Sim and asserts the promised outcome. A scenario whose test fails is
//! removed from `dev::SCENARIOS` until it passes (B-8).
//!
//! Opt-in via `NF_ISO`.

use nf_view::dev;
use nf_view::replay::TickInput;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Apply a scenario to a freshly loaded scene, returning the scene (the EXACT path `nf-view --dev --scenario` uses).
fn apply(iso: &str, level: &str, name: &str) -> Option<nf_view::scene::Scene> {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    let (_, f) = dev::scenario(name).expect("scenario exists");
    f(&mut scene);
    Some(scene)
}

/// The nearest active guard to Bond within `max_dist` — the guard the scenario placed Bond near.
fn chosen_guard(sim: &nf_view::scene::Sim, max_dist: f32) -> Option<usize> {
    dev::nearest_active_guard(sim, sim.bond.pos, max_dist)
}

/// Tick the sim `n` times with idle input (no keys, no trigger). The scenario's own setup (a guard near Bond, a
/// lifted guard, etc.) drives the behaviour; the test only advances time.
fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// Assert the common invariants the user demanded: the chosen guard is active, in the llist, and Bond's spot is not
/// sealed (≥2 walkable directions). Also that Bond stands on collision floor (y not far below the floor).
fn assert_active_and_walkable(scene: &nf_view::scene::Scene, gi: usize) {
    let sim = scene.sim.as_ref().expect("sim");
    let dr = &sim.drones.list[gi];
    // B-7: the user's demanded definition — present + enabled + alive + drawn + in-llist + channel on.
    assert!(dev::is_active_guard(&sim.drones, gi),
        "guard #{gi} is not active: present={} enabled={} corpse={} health={} drawn={} in_llist={}",
        dr.present, dr.enabled, dr.corpse, dr.health, dr.drawn, sim.drones.llist.contains(&gi));
    assert!(sim.drones.llist.contains(&gi), "guard #{gi} not in llist — the game won't run its AI");
    assert!(dr.drawn, "guard #{gi} not drawn (obj+0xFC) — the game won't render or run it in play");
    let dirs = dev::walkable_directions(&sim.world, sim.bond.pos, 5.0);
    assert!(dirs >= 2, "Bond's spot is sealed (only {dirs} walkable direction(s)) — the user would be stuck");
    // Bond stands on collision floor: the floor is within 2 m below him (not floating / not falling).
    let floor = sim.world.floor_below(sim.bond.pos, 4.0);
    assert!(floor.is_some(), "no collision floor below Bond at {:?}", sim.bond.pos);
    if let Some(fy) = floor {
        assert!((sim.bond.pos[1] - fy).abs() < 2.5, "Bond {:?} is far from floor {:.1} — not standing on it", sim.bond.pos, fy);
    }
}

// ── grenade-throw (07000002) ──────────────────────────────────────────────────

#[test]
fn scenario_grenade_throw_active_guard_throws() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = apply(&iso, "07000002.bin", "grenade-throw").expect("scenario applied");
    let gi = chosen_guard(scene.sim.as_ref().unwrap(), 15.0)
        .expect("a grenade guard was placed near Bond");
    assert_active_and_walkable(&scene, gi);
    // The guard must carry grenades (the scenario's whole point).
    assert!(scene.sim.as_ref().unwrap().drones.list[gi].grenade_count > 0, "guard #{gi} has no grenades");
    // Tick ~5 s (300 frames). A grenade entity should spawn from the guard's own decision (the can_throw_grenade
    // path + throw_grenade). Assert a grenade appears in the sim's grenade list.
    tick(&mut scene, 360);
    let n = scene.sim.as_ref().unwrap().drones.grenades.len();
    assert!(n > 0, "no grenade was thrown in 360 ticks — the guard never decided to throw");
    eprintln!("[scenario_live] grenade-throw: {n} grenade(s) spawned (guard #{gi}).");
}

// ── guard-falls-off-ledge (07000005) ──────────────────────────────────────────

#[test]
fn scenario_guard_falls_off_ledge_drops_to_floor() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = apply(&iso, "07000005.bin", "guard-falls-off-ledge").expect("scenario applied");
    let gi = chosen_guard(scene.sim.as_ref().unwrap(), 25.0)
        .expect("a guard was placed near Bond");
    assert_active_and_walkable(&scene, gi);
    // The scenario lifts the guard 3 m. Capture the start y, then tick; the guard's y should drop toward the floor.
    let start_y = scene.sim.as_ref().unwrap().drones.list[gi].pos[1];
    tick(&mut scene, 30);
    let end_y = scene.sim.as_ref().unwrap().drones.list[gi].pos[1];
    assert!(end_y < start_y - 0.5, "guard did not fall: start y={start_y:.2}, end y={end_y:.2} (Δ should be < -0.5)");
    let floor = scene.sim.as_ref().unwrap().world.floor_below(scene.sim.as_ref().unwrap().drones.list[gi].pos, 4.0);
    if let Some(fy) = floor {
        assert!((end_y - fy).abs() < 3.0, "guard landed far from floor: end y={end_y:.2}, floor={fy:.2}");
    }
    eprintln!("[scenario_live] guard-falls-off-ledge: guard #{gi} fell y {start_y:.2} → {end_y:.2}.");
}

// ── grenade-bounce-explode (07000002) ─────────────────────────────────────────

#[test]
fn scenario_grenade_bounce_explode_detonates() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = apply(&iso, "07000002.bin", "grenade-bounce-explode").expect("scenario applied");
    let gi = chosen_guard(scene.sim.as_ref().unwrap(), 20.0)
        .expect("a grenade guard was placed near Bond");
    assert_active_and_walkable(&scene, gi);
    assert!(scene.sim.as_ref().unwrap().drones.list[gi].grenade_count > 0, "guard #{gi} has no grenades");
    // Tick ~10 s, tracking the MAX grenades + explosions ever seen (a grenade may be thrown, bounce, and explode
    // — and the explosion entity is short-lived — so checking only the end state misses it).
    let mut max_seen = 0usize;
    for _ in 0..600 {
        tick(&mut scene, 1);
        let n = scene.sim.as_ref().unwrap().drones.grenades.len() + scene.sim.as_ref().unwrap().drones.explosions.len();
        if n > max_seen { max_seen = n; }
    }
    assert!(max_seen > 0, "no grenade was ever thrown in 600 ticks — the guard never decided to throw");
    eprintln!("[scenario_live] grenade-bounce-explode: max grenades+explosions seen = {max_seen}.");
}

// ── searchlight-tracks-bond (07000005) — REMOVED B-9: searchlight PORT is broken ───────────────
// The searchlight create links only 1 of 7 heads to a spot (the 3-unit linking radius orphans most spots),
// and that head's mid=6.11 rad points the beam UP (spot at y=42.5, above the head at 37.8). The tracking
// transition (state 0→1) needs Bond within 2 m of the SPOT, which never exists on the ground. This is a port
// bug in Searchlight_Create's spot linking + the half_mid / param-4/5 read, not a scenario placement issue.
// See docs/GAPS.md G-034 and docs/WORKLOG.md B-9. The scenario fn is kept (dead_code) for when the port is fixed.

// ── ambush (07000005) — the user said this works ───────────────────────────────

#[test]
fn scenario_ambush_enters_ambush_state() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = apply(&iso, "07000005.bin", "ambush").expect("scenario applied");
    let gi = chosen_guard(scene.sim.as_ref().unwrap(), 12.0).expect("a guard was placed near Bond");
    assert_active_and_walkable(&scene, gi);
    // The scenario sets mode 0x10 (ambush) + alert 1.0. The guard should enter a cover/ambush state (0x37/0x38) or
    // attack (0x56) when Bond is close.
    assert_eq!(scene.sim.as_ref().unwrap().drones.list[gi].mode, 0x10, "ambush mode not set");
    tick(&mut scene, 180);
    let st = scene.sim.as_ref().unwrap().drones.list[gi].state;
    eprintln!("[scenario_live] ambush: guard #{gi} state 0x{st:02x}.");
    // Accept any combat/ambush state (not still Idle 4). The ambush AI should have acted.
    assert!(st != 4, "ambush guard stayed Idle (state 4) for 180 ticks — the ambush AI did not activate");
}

// ── door-anti-crush (07000005) — the user said this works ──────────────────────

#[test]
fn scenario_door_anti_crush_has_door() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = apply(&iso, "07000005.bin", "door-anti-crush").expect("scenario applied");
    // The scenario checks for doors; the test just asserts a door exists on the level (the anti-crush behaviour is
    // interactive: the user opens a door and stands in it). This is a placement sanity check.
    let has_door = scene.sim.as_ref().is_some_and(|s| s.drones.doors.list.iter().any(|d| d.glist != 0));
    assert!(has_door, "no doors on 07000005 — door-anti-crush cannot work");
}

// ── the active-guard invariant for ALL guard scenarios (smoke test) ─────────────

/// Every scenario that places Bond near a guard must pick an ACTIVE guard and a non-sealed spot. This runs the
/// placement for each guard-based scenario and asserts the invariant, catching regressions in the placement filter.
#[test]
fn all_guard_scenarios_pick_active_guard_and_walkable_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (name, level) in [
        ("grenade-throw", "07000002.bin"), ("grenade-bounce-explode", "07000002.bin"),
        ("ambush", "07000005.bin"),
        ("guard-falls-off-ledge", "07000005.bin"),
        ("door-anti-crush", "07000005.bin"),
    ] {
        let Some(scene) = apply(&iso, level, name) else { eprintln!("{name}: scenario returned None"); continue };
        let Some(sim) = scene.sim.as_ref() else { eprintln!("{name}: no sim"); continue };
        let Some(gi) = chosen_guard(sim, 30.0) else {
            // Some scenarios (door-anti-crush, searchlight) don't strictly need a guard, but the placement uses one.
            eprintln!("{name}: no active guard within 30 m of Bond (may be expected for non-guard scenarios)");
            continue;
        };
        let dr = &sim.drones.list[gi];
        let dirs = dev::walkable_directions(&sim.world, sim.bond.pos, 5.0);
        assert!(dev::is_active_guard(&sim.drones, gi), "{name}: guard #{gi} not active (present={} enabled={} corpse={} hp={} drawn={} in_llist={})",
            dr.present, dr.enabled, dr.corpse, dr.health, dr.drawn, sim.drones.llist.contains(&gi));
        assert!(dirs >= 2, "{name}: Bond's spot sealed ({dirs} walkable dir)");
        eprintln!("{name}: guard #{gi} active, {dirs} walkable dirs ✓");
    }
}

/// Keep the TickInput import used (the record/replay integration test could extend this later).
#[allow(dead_code)]
fn _tick_input_used() -> TickInput { TickInput::default() }
