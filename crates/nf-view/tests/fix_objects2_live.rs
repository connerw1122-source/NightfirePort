//! fix-objects2: the 07000008 gondola level's objects, through the real input path
//! (`nf_view::load` + `Sim::step_bond`, the viewer's mapping).
//!
//! 1. The invisible wall after the opening cut-scene: the "BlockingPoly_PlayerMissile" objects
//!    (glist flags 0x1000) are a bullet cage over the map — a lid at y 52.8, a floor at y −40, and
//!    two walls over the station walkway (z −100.1 × x 135.3–144 × y 46.2–52.8, and x 144). The
//!    game's `Collide_Pick` (0x1eb320 disasm 0x1eb480-0x1eb49c) skips 0x1000-flagged collision
//!    nodes for the actor queries (HITTEST+0x98 bit 0x20); the port's static world took them
//!    unconditionally, so Bond hit an invisible wall walking down the station stairs. The cage
//!    keeps its bullet/sight duty (`segment`).
//!
//! 2. The safe and the gondola: the safe bolts (kind 0x2E locks, out chs 134/135) burn → mux #278
//!    → ch 136 → the SimpleScript safe door opens and SP #286 plays; the briefcase pickup's switch
//!    channel 0x4A completes "Retrieve guidance chip from Safe Room" [M Pickup_Handler
//!    0x18e650:330-333]; Bond on the cable-car floor (Touch 0xEB #236, mask 8 = the player) writes
//!    ch 4; mux #235 (0x4A AND 4) → ch 1 → SP #295 (mask 1, channel 1, mode 64) plays the ride
//!    script 0x0600004C: OBJECT_Carriage animates (112.0, 32.4, −126.3) → (112.0, 18.0, −179.4)
//!    with its parts and the in-car pickups; tick 1410 writes ch 83 → SP #296 plays 0x06000078 →
//!    its event writes ch 5; the helicopter (kind 0x2F, gate ch 5 [M Copter_Create 0x17d340
//!    0x17d4ac: record+0x70 = param 6]) flies its script and writes ch 0x4B on arrival [M
//!    Copter_Update 0x17dbb8 cases 4/5/6: switch_channels[record+0x72] = 1, record+0x72 = param 7]
//!    → "Escape with Zoe in gondola" completes → mux #276 → ch 100 → the mission ends the level
//!    into 0x0730008 [M Mission_Update 0x182c60:85-116].
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Steps the sim until `pred` holds or `max` ticks pass; returns whether it held.
fn wait(sim: &mut nf_view::scene::Sim, max: u32, pred: impl Fn(&nf_view::scene::Sim) -> bool) -> bool {
    for _ in 0..max {
        sim.step_bond(Input::default(), false, false, false);
        if pred(sim) { return true; }
    }
    pred(sim)
}

/// Walks Bond toward `target` (xz), pressing E when wedged (closed doors), waiting out cut-scenes.
/// Returns false when genuinely stuck (150 consecutive immobile ticks, no cut-scene).
fn walk(sim: &mut nf_view::scene::Sim, target: [f32; 3], max: u32, log: &dyn Fn(&nf_view::scene::Sim)) -> bool {
    let mut stuck = 0u32;
    for _ in 0..max {
        let b = &sim.bond;
        let (dx, dz) = (target[0] - b.pos[0], target[2] - b.pos[2]);
        if dx * dx + dz * dz < 3.2 { return true; } // ~1.8 reach: the descents are steep, the bot slides past tight radii
        let want = dx.atan2(dz);
        let mut input = Input { forward: 1.0, ..Default::default() };
        let mut diff = want - b.yaw;
        while diff > std::f32::consts::PI { diff -= std::f32::consts::TAU; }
        while diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
        input.turn = diff.clamp(-0.08, 0.08);
        let before = b.pos;
        let press_e = stuck > 40 && stuck < 100;
        sim.step_bond(input, false, false, press_e);
        let moved = ((sim.bond.pos[0] - before[0]).powi(2) + (sim.bond.pos[2] - before[2]).powi(2)).sqrt();
        if moved < 0.0025 {
            if sim.disabled { stuck = 0; continue; } // a cut-scene holds him — not a wall
            stuck += 1;
            if stuck == 41 { log(sim); }
            if stuck > 150 { return false; }
        } else { stuck = 0; }
    }
    false
}

/// The mountain-top station → the intro plate (the cut-scene plays) → through the shutters (E)
/// → down the stairs across the missile-cage wall line (z −100.1, x 135.3–144). Before the fix the
/// cage wall (z −100.1) stopped Bond at z ≈ −99.55 with nothing drawn there.
#[test]
fn bond_walks_down_the_station_stairs_past_the_former_invisible_wall() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    // The opening cut-scene fires from the intro plate #237 (out 0x49) just south of the spawn;
    // walk THROUGH it (the reach radius stops short of its volume) — the budget covers the
    // 3880-tick cut-scene the plate starts.
    assert!(walk(sim, [139.2, 49.5, -84.0], 6000, &|s| eprintln!("[plate] blocked at {:?}", s.bond.pos)),
        "Bond crosses the intro plate from the spawn");
    assert!(wait(sim, 4500, |s| !s.disabled), "the station intro (SP #294, 3880 ticks) releases Bond");
    assert!(sim.drones.channels.on(0x49), "the intro plate fired ch 0x49 [D #237]");
    // Down the balcony to the shutters (138.3–140.7, 49.5, −96.8, free swing doors, group 6).
    assert!(walk(sim, [139.5, 49.5, -95.0], 900, &|s| eprintln!("[shutters] blocked at {:?}", s.bond.pos)),
        "Bond reaches the shutters");
    // Through them, then the natural line a player walks — straight toward the walkway's east end
    // (the Hand_Over_Hand grab at (133.4, 48.3, −101.2) [D #105]) and down the walkway stairs
    // (the TouchOnces out 0x5a/0x5b/0x5c at z ≈ −101 [D #99-#101]). That line crosses the missile
    // cage's x 135.3 wall (z −100.1..−116, y 46.2..52.8 [D PM2 triangles]) at (135.3, −100.55):
    // pre-fix Bond wedged in the corner the two cage walls form there and the walk timed out.
    // Through the shutter doorway (x 138.3–140.7 [D #49-#52]) and down the face of the station wall.
    assert!(walk(sim, [139.3, 48.9, -97.8], 1200, &|s| eprintln!("[doorway] blocked at {:?}", s.bond.pos)),
        "through the shutters (blocked at {:?})", sim.bond.pos);
    assert!(walk(sim, [131.0, 48.0, -101.3], 2000, &|s| eprintln!("[route] blocked at {:?}", s.bond.pos)),
        "the walkway's east end reachable (blocked at {:?})", sim.bond.pos);
    assert!(sim.bond.pos[2] < -100.2 && sim.bond.pos[0] < 134.5, "Bond is past the cage wall, at {:?}", sim.bond.pos);
    // Down the walkway stairs (the descent 48.9 → 46.1 along z ≈ −101); the bot may overshoot onto
    // the firing-step ledge below — the assertion is only that he got DOWN here.
    let _ = walk(sim, [119.0, 45.8, -101.2], 1500, &|s| eprintln!("[stairs] at {:?}", s.bond.pos));
    assert!(sim.bond.pos[0] < 132.0 && sim.bond.pos[1] < 48.5, "Bond descended the stairs, at {:?}", sim.bond.pos);
}

/// Sister: the cage keeps its missile duty — a shot upward from the walkway hits the y 52.8 lid,
/// and the cage wall still shows up for `segment` (bullets, sight) but never for the capsule.
#[test]
fn the_missile_cage_still_stops_bullets_but_never_the_capsule() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_ref().unwrap();
    // The lid over the station: BlockingPoly_PlayerMissile3's top face at y 52.8 [D triangles].
    let hit = sim.world.segment([139.0, 50.0, -97.0], [139.0, 56.0, -97.0]);
    assert!(hit.is_some(), "a bullet upward from the walkway hits the cage lid");
    let y = 50.0 + (56.0 - 50.0) * hit.unwrap();
    assert!((y - 52.8).abs() < 0.1, "the lid is at y 52.8 [D], got {y}");
    // The cage wall at x 135.3 (z −100.1..−116, y 46.2..52.8, PM2's t0/t1 [D]) is ONE-SIDED like
    // every static triangle: its stored plane normal is (−1, 0, 0) — raw s16 (−16384, 0, 0),
    // facing WEST into the station side (probe_planes on the disc; `Intersect_RayGeom` 0x10a8c8
    // skips a triangle when dot(plane_normal, ray_dir) >= 0). A bullet/LOS ray running WEST
    // (walkway → station) hits its BACK face and passes; the same wall stops the EAST-bound ray.
    let wall = sim.world.segment([134.5, 48.6, -101.0], [136.0, 48.6, -101.0]);
    assert!(wall.is_some(), "the cage wall stops an east-bound bullet/LOS segment crossing x 135.3");
    let x = 134.5 + 1.5 * wall.unwrap();
    assert!((x - 135.33).abs() < 0.05, "the stop is at the wall plane x 135.33, got {x}");
    assert!(sim.world.segment([136.0, 48.6, -101.0], [134.5, 48.6, -101.0]).is_none(),
        "sister: the west-bound ray is a back-face hit — the game's one-sided test lets it pass");
    // … and invisible to the actor capsule standing against it (the walk height band): pre-fix the
    // wall pushed the capsule +x (east, away from its west face); the floor's push is +y only.
    let (push, _) = sim.world.capsule_push([135.5, 48.6, -101.0], [135.5, 48.0, -101.0], 0.55);
    assert!(push[0] < 0.05, "the cage wall does not push the capsule (+x): {push:?}");
}

/// The safe chain: the two safe bolts (kind 0x2E locks) burn → chs 134/135 → mux #278 → ch 136 →
/// the `Object_SafeDoor` SimpleScript opens and SP #286 (SafePickUps) plays; the briefcase pickup
/// (switch ch 0x4A) completes "Retrieve guidance chip from Safe Room" [L mission record 3].
#[test]
fn the_safe_bolts_open_the_safe_and_the_briefcase_completes_the_boarding_objective() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
    // The safe bolts: two 0x2E locks at (162.2, 43.9/42.7, −117.1) with out channels 134/135 [D #207/#208].
    assert_eq!(sim.drones.locks.len(), 2, "the level's two safe-bolt locks exist");
    let outs: Vec<u16> = sim.drones.locks.iter().map(|l| l.out).collect();
    assert!(outs.contains(&134) && outs.contains(&135), "the bolts write chs 134/135 [D], got {outs:?}");
    // The laser watch burns them (the gadget hook — see lock.rs; the watch itself is not ported).
    let frame = sim.drones.frame;
    for k in 0..sim.drones.locks.len() { sim.drones.locks[k].burn(&mut sim.drones.channels, frame); }
    for _ in 0..4 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(136), "mux #278 ORs the burns into ch 136 [D #278]");
    // The safe door (Object_SafeDoor #205, a 0xE2 SimpleScript on ch 136, rate 60 [D extra7 0x3c0008])
    // opens: its playhead advances while the channel is on [M SS_Update 0x1a9948].
    let ss_t = sim.drones.sss.list.iter().find(|s| s.channel == 136).map(|s| s.t);
    assert!(ss_t.is_some_and(|t| t > 0.0), "the safe-door SimpleScript's playhead advanced: {ss_t:?}");
    // The objective reveals on ch 0x49 (the intro plate [L record 3 enable]) — in play Bond walks
    // it first; here the plate's channel directly, then the monitor sees the completion.
    sim.drones.channels.set(0x49, 1, sim.drones.frame);
    sim.step_bond(Input::default(), false, false, false);
    // The briefcase (oBriefcase_closed, switch ch 0x4A [D #290]) — Bond touches it, it is taken.
    sim.bond.pos = [172.7, 44.6, -116.2];
    assert!(wait(sim, 120, |s| s.drones.channels.on(0x4a)), "taking the briefcase writes ch 0x4A [M Pickup_Handler 0x18e650:330-333]");
    let i8 = nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0008).unwrap();
    assert_eq!(sim.drones.mission.states[i8][0], 4, "the boarding objective completes [L record 3]");
    // Sister: the safe chain alone never completes the mission (the ride is still ahead).
    assert!(!sim.drones.channels.on(100), "the mission stays open without the ride");
}

/// The ride: Bond on the cable-car floor (Touch 0xEB #236, mask 8 = the player) with the briefcase
/// taken → mux #235 (0x4A AND 4) → ch 1 → SP #295 plays the ride script: OBJECT_Carriage descends
/// (112.0, 32.4, −126.3) → (112.0, 18.0, −179.4) [D script keys]; tick 1410 writes ch 83 → SP #296
/// plays 0x06000078 → ch 5 → the helicopter (gate ch 5 [M Copter_Create 0x17d340]) flies and writes
/// ch 0x4B [M Copter_Update 0x17dbb8 cases 4/5/6] → mux #276 → ch 100 → the mission ends the level
/// into 0x0730008.
#[test]
fn the_gondola_ride_runs_and_the_helicopter_completes_the_mission() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
    // The carried latches from the previous levels (in play they ride the RamSaved channels:
    // 0x45 = 05's reveal, 0x70 = the truck park, 0x02 = the dressing scene, 0x48 = the hall) —
    // the mission completes only with ALL six Mayhew objectives' channels on [L records 0-3].
    for ch in [0x45u32, 0x70, 0x02, 0x48] { sim.drones.channels.set(ch, 1, sim.drones.frame); }
    // The safe chain (the briefcase taken — the ride requires ch 0x4A [D mux #235]); the intro
    // plate's ch 0x49 reveals the objective first [L record 3 enable].
    sim.drones.channels.set(0x49, 1, sim.drones.frame);
    sim.step_bond(Input::default(), false, false, false);
    let frame = sim.drones.frame;
    for k in 0..sim.drones.locks.len() { sim.drones.locks[k].burn(&mut sim.drones.channels, frame); }
    sim.bond.pos = [172.7, 44.6, -116.2];
    assert!(wait(sim, 120, |s| s.drones.channels.on(0x4a)), "the briefcase taken");
    // Bond steps onto the cable-car floor (SWITCH_CABLECARFLOOR, Touch 0xEB out 4, mask 8 [D #236]).
    sim.bond.pos = [112.0, 28.0, -127.4];
    assert!(wait(sim, 120, |s| s.drones.channels.on(4)), "the cable-car floor writes ch 4");
    assert!(wait(sim, 60, |s| s.drones.channels.on(1)), "mux #235 (0x4A AND 4) fires ch 1 [D #235]");
    // The ride: the carriage (script entity 0x0200038a) descends for 1420 ticks [D script keys].
    let carriage = |s: &nf_view::scene::Sim| s.drones.scripts.objects.list.iter().flatten()
        .find_map(|o| match o { nf_game::nis::Obj::Entity { hash: 0x0200_038a, matrix, .. } => Some(matrix[3][1]), _ => None });
    let y0 = carriage(sim).expect("OBJECT_Carriage exists once the ride starts");
    // The carriage's hull sweeps its boarding column: step north of the path for the ride (the
    // riding inherit on script-entity parts is an open item — see the report; standing on the
    // carriage's line, the hull pushes Bond off into the valley plunge volumes and the death NIS
    // fires).
    sim.bond.pos = [114.5, 28.0, -121.0];
    assert!(wait(sim, 2500, |s| carriage(s).is_none_or(|y| y < y0 - 5.0)),
        "the carriage descends (start y {y0})");
    // The ride runs to tick 1410 of 1420 [D s46], then the cinematic (342 ticks) plays out to its
    // own ch-5 event — the budget covers both.
    assert!(wait(sim, 2600, |s| s.drones.channels.on(5)), "the ride's end cinematic (0x06000078) writes ch 5 [D script s10]");
    // The helicopter: gate ch 5 → the flight (0x06000050, 299 ticks) → ch 0x4B [M Copter_Update].
    assert!(wait(sim, 900, |s| s.drones.channels.on(0x4b)), "the copter's arrival writes ch 0x4B");
    assert!(wait(sim, 200, |s| s.drones.mission.status() == 6), "mux #276 → ch 100 → the mission completes");
    let i8 = nf_game::mission::LEVELS.iter().position(|l| l.level == 0x0700_0008).unwrap();
    assert_eq!(sim.drones.mission.states[i8][1], 4, "the ride objective completes [L record 3]");
}

/// Sister: boarding without the briefcase never starts the ride — mux #235 needs BOTH channels
/// [D #235], and with no ride there is no cinematic, no ch 5, no copter, no ch 0x4B.
#[test]
fn boarding_without_the_briefcase_never_starts_the_ride() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000008.bin", false).expect("load 07000008");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
    sim.bond.pos = [112.0, 28.0, -127.4];
    assert!(wait(sim, 120, |s| s.drones.channels.on(4)), "the cable-car floor writes ch 4 even alone");
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    assert!(!sim.drones.channels.on(1), "mux #235 stays off without ch 0x4A [D #235]");
    assert!(!sim.drones.channels.on(0x4b) && !sim.drones.channels.on(100),
        "no ride, no copter, no mission end");
}
