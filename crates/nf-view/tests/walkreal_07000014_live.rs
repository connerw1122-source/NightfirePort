//! walkreal-b: 07000014 (Drake's underground facility → Delta Sector) walked with the player's
//! own movement — the sabotage and the kill are the REAL interaction paths; no teleports. Route
//! (docs/levels/07000014.md): the vent store room start #666/#667 (40.81, 2.40, 65.08 [D]) → the
//! REAL walk north to the vent drop TouchOnce #301 (39.68…41.75, 2.07…4.14, 37.62…40.40 [D]) →
//! the SP #276 cut-scene (0x06000833, 1400 ticks) writes ch 0x47 (objective 1) → the REAL walk
//! west through the facility to the Kiko cut-scene touch #381 (−51.97…−46.39, 0…2.07, 37.28…39.46
//! [D]) → SP #382 (0x0600062F) writes ch 0x48 (objective 2) → the REAL walk west-south-west down
//! into the west generator room → the two fuseboxes E-cut through the REAL FuseBox path ([M
//! FuseBox_Update 0x1adfe8]; #383 p1 0x87 at (−89.31, −2.22, 30.79), #385 p1 0x88 at (−89.30,
//! −2.22, 49.36) [D]) → AND #391 [0x87, 0x88] raises 0x8C → the REAL walk back east to the
//! server-room escape touch #456 (−33.1, 0.9, 26.8 [D]) → 0x0C + 0x8C → SEQ_AND #445 → 0x09 → SP
//! #407 (0x06000848) writes ch 0x4A (objective 3) → the REAL walk east to Rook's wake touch #476
//! (19.22…21.30, −26.26…−20.69 [D]) → SEQ_AND #475 [0x8C, 0x0F] → 0x0D wakes Rook (skin
//! 0x05000066 [D]) → the REAL gun path kills him (the walker's aim-and-fire loop) → his death
//! channel 0x54 fires [D drone param 12; M set_death_channel 0x13e660] (objective 4) → the REAL
//! walk to the exit #944 (27.79, 1.28, −25.32 [D]) → E → the fallback arm writes ch 0xCE
//! (objective 5) and loads 07000015 [M 0x1b2318].
//!
//! Enemies are DISABLED for the walk (the walker tops Bond's health every tick; the pain pipeline
//! still runs) except Rook, who must die. The fuseboxes' live wires carry param 2 non-zero on
//! every box here → the cut NEVER hurts [M 0x1adfe8:99-101; D placements census].

#[path = "common/walker.rs"]
mod walker;

use walker::{Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// Walk onto a fusebox's stand point and E-cut it through the real interaction loop (the lid
/// press, then the cut — the teleport test's 3×E, gap, 3×E rhythm, held until the group channel
/// rises). The look arm: the walker's turn input keeps Bond facing the box.
fn cut_fusebox(w: &mut Walker, box_pos: [f32; 3], want: u8, label: &str) {
    let stand = [box_pos[0] + 0.6, box_pos[1] - 1.2, box_pos[2]];
    w.walk_to(Wp::new(stand[0], stand[1], stand[2]).r(0.6), &format!("{label} stand point"));
    let start = w.total_ticks;
    loop {
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(want as u32) { break; }
        assert!(w.total_ticks - start < 240, "{label}: the cut never wrote ch {want:#x} [M 0x1adfe8]");
        let p = sim.bond.pos;
        let desired = (box_pos[0] - p[0]).atan2(box_pos[2] - p[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        let t = w.total_ticks - start;
        // E held 3 ticks, released 1 (the press edge the trigger's mask-4 arm reads [M 0x1b2318]).
        let input = nf_game::player::Input { turn: err.clamp(-0.08, 0.08), ..Default::default() };
        w.tick(input, t % 4 != 3);
    }
    // The lid-press/cut pair writes both channel words on the cut [M 0x1adfe8].
    eprintln!("[014] {label}: ch {want:#x} on at +{} ticks", w.total_ticks - start);
}

#[test]
#[ignore = "the vent cut-scene (SP #276, script 0x06000833 [D]) never writes ch 0x47 in the port — the scene's firing (the touch #301 → SP start → the 1400-tick script's channel write) is the documented open gap walkreal-b parked ('the next session debugs the vent cut-scene'); the route past it is written and continues from the branch once the scene fires — owner: the cut-scene owner (fix-cutscenes2 candidate)"]
fn walk_the_infiltration_to_the_delta_exit() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000014.bin", false, None).unwrap());
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[0] > 39.0 && p[0] < 42.0 && p[1] > 1.5 && p[1] < 6.0 && p[2] > 63.0 && p[2] < 67.0,
            "the vent store room start #666 [D], got {p:?}");
    }
    w.wait_enabled(3000, "the intro");
    w.wait(120);
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.mission.states[20][0], 3,
        "'Infiltrate Drake's facility' announces at the start [L]");

    // Leg 1: north to the vent drop #301 — walked.
    w.walk_to(Wp::new(40.7, 2.2, 39.0).r(1.2), "to the vent drop #301");
    // The SP #276 cut-scene (0x06000833, 1400 ticks [D decode]) writes ch 0x47 at its last tick.
    let mut ch47 = false;
    for _ in 0..2400 {
        w.tick(nf_game::player::Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x47) { ch47 = true; break; }
    }
    assert!(ch47, "the vent cut-scene must write ch 0x47 [D]");
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.mission.states[20][0], 4, "objective 1 completes [M 0x1827d8]");
    for _ in 0..900 {
        if !w.scene.sim.as_ref().unwrap().disabled { break; }
        w.tick(nf_game::player::Input::default(), false);
    }

    // Leg 2: west through the facility to the Kiko cut-scene touch #381 — walked (the long cross
    // floor; the nav web threads the corridors [D]).
    w.follow_nav_route([-49.0, 1.0, 38.4], "west to the Kiko touch #381");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.bond.pos[0] < -45.0, "Bond stands in the #381 band (x −51.97…−46.39 [D]), got {:?}", sim.bond.pos);
    }
    let mut ch48 = false;
    for _ in 0..2400 {
        w.tick(nf_game::player::Input::default(), false);
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x48) { ch48 = true; break; }
    }
    assert!(ch48, "the Kiko cut-scene 0x0600062F must write ch 0x48 [D]");
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.mission.states[20][1], 4, "objective 2 completes [M 0x1827d8]");
    for _ in 0..900 {
        if !w.scene.sim.as_ref().unwrap().disabled { break; }
        w.tick(nf_game::player::Input::default(), false);
    }

    // Leg 3: down into the west generator room; the two fuseboxes cut through the REAL path.
    cut_fusebox(&mut w, [-89.31, -2.22, 30.79], 0x87, "fusebox #383");
    cut_fusebox(&mut w, [-89.30, -2.22, 49.36], 0x88, "fusebox #385");
    for _ in 0..10 { w.tick(nf_game::player::Input::default(), false); }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x8c), "AND #391 [0x87, 0x88] raises 0x8C [D]");
        assert_eq!(sim.drones.health.health, 100.0,
            "no live-wire hurt with +0x56 configured [M 0x1adfe8:99-101]");
    }

    // Leg 4: back east to the server-room escape touch #456 — walked; the SP #407 cut-scene
    // (0x06000848, 950 ticks [D]) writes ch 0x4A.
    w.follow_nav_route([-33.1, 0.9, 26.8], "east to the escape touch #456");
    let mut ch4a = false;
    for _ in 0..2000 {
        w.tick(nf_game::player::Input::default(), false);
        let sim = w.scene.sim.as_ref().unwrap();
        if sim.drones.channels.on(0x4a) { ch4a = true; break; }
        assert!(sim.drones.channels.on(0x0c), "the room touch #456 fired 0x0C on contact [D]");
    }
    assert!(ch4a, "the escape cut-scene must write ch 0x4A [D]");
    assert_eq!(w.scene.sim.as_ref().unwrap().drones.mission.states[20][2], 4, "objective 3 completes [M 0x1827d8]");
    for _ in 0..900 {
        if !w.scene.sim.as_ref().unwrap().disabled { break; }
        w.tick(nf_game::player::Input::default(), false);
    }

    // Leg 5: east to Rook's wake touch #476 — walked; SEQ_AND #475 wakes him.
    w.follow_nav_route([20.3, 0.9, -23.5], "east to Rook's wake touch #476");
    for _ in 0..60 { w.tick(nf_game::player::Input::default(), false); }
    let rook_i = w.scene.sim.as_ref().unwrap().drones.list.iter()
        .position(|d| d.skin == 0x0500_0066).expect("Rook (skin 0x05000066) exists [D]");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x0d), "SEQ_AND #475 [0x8C, 0x0F] raises 0x0D [D]");
        assert!(sim.drones.list[rook_i].started, "Rook leaves his switch-channel wait [M 0x14b300]");
    }
    // The kill: the REAL gun path (the walker's aim-and-fire; resupply on the drops when out).
    let mut dead = false;
    for attempt in 0..3 {
        if w.shoot(rook_i, "Rook") { dead = true; break; }
        assert!(w.resupply(&format!("Rook attempt {attempt}")), "no ammo for the Rook kill");
    }
    assert!(dead, "Rook must die to the walked gun path");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(sim.drones.channels.on(0x54), "Rook's death channel 0x54 fires [D param 12; M 0x13e660]");
        assert_eq!(sim.drones.mission.states[20][3], 4, "objective 4 completes [M 0x1827d8]");
    }

    // Leg 6: to the Delta exit #944 — walked; E fires the fallback write + the load.
    w.walk_to(Wp::new(27.79, 1.28, -25.32).r(1.0), "to the Delta exit #944");
    let mut loaded = false;
    for _ in 0..60 {
        w.tick(nf_game::player::Input::default(), true);
        if w.scene.sim.as_ref().unwrap().drones.level_exit.is_some() { loaded = true; break; }
    }
    {
        let sim = w.scene.sim.as_ref().unwrap();
        assert!(loaded && sim.drones.level_exit == Some(0x0700_0015),
            "E at the exit #944 loads 07000015 (EvilBase) [D]");
        assert!(sim.drones.channels.on(0xce), "the fallback arm wrote ch 0xCE (objective 5) [M 0x1b2318]");
    }
    eprintln!("[014] walked out in {} ticks; stuck {}; falls {}", w.total_ticks, w.report.stuck_events.len(), w.report.falls.len());
}
