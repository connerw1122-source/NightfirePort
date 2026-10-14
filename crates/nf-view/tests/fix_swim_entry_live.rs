//! fix-swim-entry: the swim entry rides the game's stateful player cel relink — `control_handle_cel_change`
//! 0x133288 as the movement handler's moved pass runs it [M 0x1336f8:147-236]: the move segment's portal
//! cross (`Cel_ObjectLeftCel` 0x1e0d58) wins over the bbox keep (`build_point_in_bounding_box` 0x1c9500),
//! and the fresh-link fallback is `build_LinkToRoom` 0x1c9680 → `build_FindCel` 0x1c8cb0 AT THE WORLD-SPHERE
//! CENTRE (`obj+0x80`, with the raycast tie-break). `Player_InWater` 0x195e50 reads that cel's `+0x90`.
//! Opt-in via `NF_ISO`.

use nf_view::scene::Sim;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Sim {
    let mut scene = nf_view::load(iso, "07000002.bin", false).expect("load 07000002");
    scene.sim.take().expect("sim")
}

/// The garden pond west of the bunker: water 3.7675, x −80.3..−54.1 (cel 17, graph cell 184 [D probe]).
fn garden_pond(sim: &Sim) -> (usize, usize, nf_view::scene::Cel) {
    let (room, cel) = sim.cels.iter().enumerate()
        .find(|(_, c)| c.water > 3.76 && c.water < 3.775 && c.max[0] < -54.0)
        .map(|(i, c)| (i, c.clone())).expect("the garden water cel");
    let cell = sim.room_cell[room].expect("the garden water cel has a graph cell");
    (room, cell, cel)
}

/// Walking off the bank enters the swim on the game's relink: at the entry tick Bond's runtime cel IS
/// the water cel (the surface portal 91↔184 spans the pond top at y 3.765 [D probe_swim_entry]; the
/// crossing move segment relinks it the tick before the substate flips), and the game's InWater
/// inequality holds [M 0x195e50: `obj+0x34 − 0.3 < cel+0x90`].
#[test]
fn the_bank_walk_enters_the_swim_on_the_games_relink() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    let (_, water_cell, c) = garden_pond(&sim);
    let cz = 0.5 * (c.min[2] + c.max[2]);
    let bx = c.min[0] - 2.0;
    let floor = sim.world.floor_below([bx, 8.0, cz], 12.0).expect("the west bank floor");
    assert!(floor > c.water, "the start spot is not a bank above the water");
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [bx, floor + 1.1, cz], 0.0);
    sim.bond.yaw = std::f32::consts::FRAC_PI_2; // face +x, into the pond
    assert_eq!(sim.bond.cel, None, "the spawn relink has not run yet");
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    let mut entry: Option<(usize, f32)> = None;
    for t in 0..2400 {
        sim.step_bond(fwd, false, false, false);
        if sim.bond.substate == nf_game::player::SS_SWIM { entry = Some((t, sim.bond.pos[1])); break; }
    }
    let (t, y) = entry.expect("Bond never entered the water from the bank — he walks on it");
    // The entry's cel state is the water cel — the relink the InWater read, not a position lookup.
    assert_eq!(sim.bond.cel, Some(water_cell),
        "at the swim entry the runtime cel is not the water cel (pos.y {y})");
    // The game's inequality [M 0x195e50].
    assert!(y - 0.3 < c.water, "at the swim entry pos.y − 0.3 = {} is not under the surface {}", y - 0.3, c.water);
    eprintln!("entered the swim at tick {t}, y {y:.3} (surface {}), cel {:?}", c.water, sim.bond.cel);
    // Sister: the swim holds — the stateful cel keeps feeding the surface.
    for _ in 0..180 { sim.step_bond(fwd, false, false, false); }
    assert_eq!(sim.bond.substate, nf_game::player::SS_SWIM, "the swim substate did not hold");
    assert_eq!(sim.bond.cel, Some(water_cell), "the cel left the water cel while swimming mid-pond");
}

/// The game's FRESH cel link is `build_FindCel` AT THE WORLD-SPHERE CENTRE — `build_LinkToRoom` 0x1c9680
/// passes `obj+0x80..0x8c` (the centre `Control_BuildWorldSph` 0x133070 built), and the centre rides
/// `M_CONTROL+0x80 = (−0.1347, −0.1267, 0.2402)` local [L ee_start_05.bin], i.e. 0.1267 BELOW the
/// position. The water cel's bbox top is `cel+0x90` itself (3.7675 [D]), so there is a band where the
/// POSITION is above the water cel's box but the CENTRE is inside it: BOTH the land air cel and the
/// water cel are candidates, and the game's tie-break hands it to the LAND cel — its own scope holds
/// the slope right below the centre while the water cel's mat-14 surface plane is up-facing (an up-ray
/// cannot hit it one-sided [M 0x10a8c8]) and its down-scope has no plane under the slope column. The
/// pre-fix stateless smallest-bbox lookup read the POSITION and snapped to the smaller water cel —
/// Bond spawned SWIMMING in the wade band where the game wades (walks) on the submerged slope.
#[test]
fn the_spawn_cel_pick_reads_the_sphere_centre_not_the_position() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sim = load(&iso);
    let (_, water_cell, c) = garden_pond(&sim);
    let cz = 0.5 * (c.min[2] + c.max[2]);
    // The band: the position above the water cel's bbox top, the sphere centre (−0.1267) inside it.
    let y = c.water + 0.05;
    assert!(y > c.water, "the position should sit above the water cel's bbox top for this test");
    assert!(y - 0.1267 < c.water, "the sphere centre should sit inside the water cel's bbox");
    sim.bond = nf_game::player::Bond::new([-75.4, y, cz], 0.0);
    // The first tick's relink (cur = None → FindCel at the centre [M 0x1c9680]): the tie-break keeps
    // the LAND cel (the game wades here — no swim until a portal cross or a deeper centre).
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    let linked = sim.bond.cel.expect("the spawn relink left Bond unlinked");
    assert_ne!(Some(linked), Some(water_cell),
        "the fresh link at the sphere centre snapped to the water cel (pos.y {:.3}, centre {:.3}) — the pre-fix position lookup",
        sim.bond.pos[1], sim.bond.sphere_centre()[1]);
    // And no swim follows [M 0x195e50 needs a water cel's +0x90].
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK,
        "spawning in the wade band entered the swim — the game's tie-break wades here");
    // Deeper spawn (the centre below the land cel's box): the water cel links alone and the swim
    // follows [M 0x195e50]. The level-start NIS grabs control within a couple of ticks — skip it the
    // game's way first (the viewer's own skip field), then place and tick.
    let deep = c.water - 0.3;
    for _ in 0..4000 {
        if !sim.disabled { break; }
        sim.drones.scripts.skip_request = true;
        sim.step_bond(nf_game::player::Input::default(), false, false, false);
    }
    assert!(!sim.disabled, "the intro NIS never released Bond");
    sim.bond = nf_game::player::Bond::new([-75.4, deep, cz], 0.0);
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    assert_eq!(sim.bond.cel, Some(water_cell), "the deep spawn did not link the water cel");
    assert_eq!(sim.bond.substate, nf_game::player::SS_SWIM,
        "spawning deep in the pond did not enter the swim (pos.y {:.3})", sim.bond.pos[1]);
    // Sister: dry land spawns still link a land cel — the sphere-centre read does not leak water onto
    // the bank. The west bank 2 units outside the pond's x range.
    let bank = sim.world.floor_below([c.min[0] - 2.0, 8.0, cz], 12.0).expect("the bank floor");
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [c.min[0] - 2.0, bank + 1.1, cz], 0.0);
    sim.step_bond(nf_game::player::Input::default(), false, false, false);
    assert_ne!(sim.bond.cel, Some(water_cell), "the dry bank spawn linked the water cel");
    assert_eq!(sim.bond.substate, nf_game::player::SS_WALK, "the dry bank spawn entered the swim");
}
