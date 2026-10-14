//! fix-0c-vent2 scratch probe: map the vent region geometry (port collision world) and drive
//! Bond's real input path from the duct top / inner ledge to the display case floor. NOT a test —
//! diagnostics with eprintln; run with --nocapture.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

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

fn burn_display_hinges(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    for k in 0..sim.drones.locks.len() {
        if (14..=17).contains(&sim.drones.locks[k].out) {
            sim.drones.locks[k].burn(&mut sim.drones.channels, 1);
        }
    }
}

/// All downward hits in a column (walk the ray down).
fn floors(world: &nf_game::world::CollisionWorld, x: f32, z: f32, top: f32, bottom: f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut y = top;
    for _ in 0..40 {
        let Some((t, hit)) = world.segment_hit([x, y, z], [x, bottom, z]) else { break };
        let hy = y + (bottom - y) * t;
        out.push(hy);
        y = hy - 0.06;
        if y < bottom { break; }
    }
    out
}

#[test]
fn probe_floors_map() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 15.8, -107.5], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    // z-slices through the vent region: print every floor per (x, z) column.
    for z in [-107.5f32, -109.0, -110.2, -111.0, -112.0, -112.6, -113.4, -114.2, -115.0, -115.8, -116.6] {
        print!("z {:7.1}: ", z);
        for x in [3.5f32, 4.5, 5.4, 6.2, 7.0, 8.0] {
            let fl = floors(&sim.world, x, z, 20.0, 5.5);
            let s: Vec<String> = fl.iter().map(|y| format!("{:.2}", y)).collect();
            print!(" x{:4.1}[{}] ", x, s.join(","));
        }
        println!();
    }
    // The vent wall plane: rays along z at the opening heights, x through the opening.
    println!("-- vent plane z -112.25 rays (x, y): hit? --");
    for y in [14.9f32, 15.3, 15.7, 16.1, 16.5, 16.9, 17.3, 17.6] {
        for x in [4.8f32, 5.4, 5.8, 6.4, 6.9] {
            let hit = sim.world.segment_hit([x, y, -111.4], [x, y, -113.4]);
            print!("({:.1},{:.1}){} ", x, y, if hit.is_some() { "X" } else { "." });
        }
        println!();
    }
    // Vertical shaft above the display case: is there an opening between ledge/case levels?
    println!("-- columns under the ledge (x, z) floors --");
    for z in [-112.8f32, -113.2, -113.6, -114.0, -114.4, -114.8] {
        for x in [4.6f32, 5.0, 5.4, 5.8, 6.2] {
            let fl = floors(&sim.world, x, z, 14.4, 6.0);
            if !fl.is_empty() {
                let s: Vec<String> = fl.iter().map(|y| format!("{:.2}", y)).collect();
                print!("({:.1},{:.1})[{}] ", x, z, s.join(","));
            }
        }
    }
    println!();
}

/// Fine maps: the mid-floor hole, the case floor, the wall plane at the side ledges.
#[test]
fn probe_fine_maps() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 15.8, -107.5], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    // 1. First floor below y 13.5 (through the mid floor level): holes show 11.27 missing.
    println!("-- first floor below 13.5 (rows z, cols x) --");
    let xs = [3.0f32, 3.75, 4.5, 5.25, 6.0, 6.75, 7.5, 8.25];
    for z in [-112.4f32, -113.0, -113.6, -114.2, -114.8, -115.4, -116.0, -116.6, -117.2] {
        print!("z {:6.1}: ", z);
        for x in xs {
            match sim.world.segment_hit([x, 13.5, z], [x, 6.0, z]) {
                Some((t, _)) => print!(" {:5.2}", 13.5 + (6.0 - 13.5) * t),
                None => print!("  ----"),
            }
        }
        println!();
    }
    // 2. First floor below 10.8 (under the mid floor): the case floor map.
    println!("-- first floor below 10.8 (rows z, cols x) --");
    for z in [-112.4f32, -113.0, -113.6, -114.2, -114.8, -115.4, -116.0, -116.6, -117.2] {
        print!("z {:6.1}: ", z);
        for x in xs {
            match sim.world.segment_hit([x, 10.8, z], [x, 5.8, z]) {
                Some((t, _)) => print!(" {:5.2}", 10.8 + (5.8 - 10.8) * t),
                None => print!("  ----"),
            }
        }
        println!();
    }
    // 3. The wall plane at the side ledges: rays along z at x 3.0..4.6 and 7.4..9.0, y 14.2..16.4.
    println!("-- rays along z -110.8 -> -115.8 (X = wall hit), rows y, cols x --");
    for y in [14.2f32, 14.8, 15.2, 15.6, 16.0, 16.4] {
        print!("y {:4.1}: ", y);
        for x in [3.0f32, 3.5, 4.0, 4.4, 4.6, 7.4, 7.6, 8.0, 8.5, 9.0] {
            let hit = sim.world.segment_hit([x, y, -110.8], [x, y, -115.8]);
            print!(" {}{}", x, if hit.is_some() { "X" } else { "." });
        }
        println!();
    }
    // 4. The walkway -> side ledge riser: rays along z at y 14.5 and 15.0, x 3.0..4.4.
    println!("-- the walkway end face: rays along z -109.0 -> -110.6 --");
    for y in [14.2f32, 14.6, 15.0] {
        print!("y {:4.1}: ", y);
        for x in [3.0f32, 3.5, 4.0, 4.4, 5.4, 6.5, 7.6, 8.5] {
            let hit = sim.world.segment_hit([x, y, -109.0], [x, y, -110.6]);
            print!(" {}{}", x, if hit.is_some() { "X" } else { "." });
        }
        println!();
    }
    // 5. x-rays across the display area at y 7.6 and 8.6 (the case walls?) at z -113.
    println!("-- rays along x -0.5 -> 11.5 at z -113 / -115.8 --");
    for (y, z) in [(7.6f32, -113.0f32), (8.6, -113.0), (7.6, -115.8), (8.6, -115.8), (9.6, -113.0)] {
        print!("y {:4.1} z {:6.1}: ", y, z);
        for yy in [y, y + 0.4] {
            let hit = sim.world.segment_hit([0.5, yy, z], [10.5, yy, z]);
            print!(" {:.1}{}", yy, if hit.is_some() { "X" } else { "." });
        }
        println!();
    }
}

/// The balcony full map: top surface, holes, ceilings above (crouch gaps), edges.
#[test]
fn probe_balcony_map() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 15.8, -107.5], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    println!("-- balcony: first floor below 15.4 (rows z, cols x); '*' = hole to below 13 --");
    let xs = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0];
    for z in [-112.6f32, -113.5, -114.5, -115.5, -116.5, -117.5, -118.5, -119.5, -120.5, -121.5] {
        print!("z {:6.1}: ", z);
        for x in xs {
            match sim.world.segment_hit([x, 15.4, z], [x, 5.8, z]) {
                Some((t, _)) => {
                    let y = 15.4 + (5.8 - 15.4) * t;
                    if y < 13.0 { print!("  *.**") } else { print!(" {:5.2}", y) }
                }
                None => print!("  ----"),
            }
        }
        println!();
    }
    println!("-- ceilings: first floor ABOVE 15.6 going up to 26 (low overpass = crouch gap) --");
    for z in [-112.6f32, -114.0, -115.5, -117.0, -118.5, -120.0, -121.0] {
        print!("z {:6.1}: ", z);
        for x in xs {
            match sim.world.segment_hit([x, 15.6, z], [x, 26.0, z]) {
                Some((t, _)) => print!(" {:5.2}", 15.6 + (26.0 - 15.6) * t),
                None => print!("  ----"),
            }
        }
        println!();
    }
}

/// Dump SSs near the pulley placements: channels, rates, hashes (positions via the world).
#[test]
fn probe_pulleys() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let scene = nf_view::load(&iso().unwrap(), "0700000c.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    for ss in sim.drones.sss.list.iter() {
        if (330..=390).contains(&ss.placement) {
            eprintln!("SS placement {} ch {} rate {} t {:.2} keys? hash {:x}", ss.placement, ss.channel, ss.rate, ss.t, ss.hash);
        }
    }
}

/// The balcony edges: west/east/south — what's below and how far the floors extend.
#[test]
fn probe_edges() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 15.8, -107.5], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let fl = |x: f32, z: f32, top: f32, bot: f32| -> Vec<f32> {
        let mut out = Vec::new();
        let mut y = top;
        for _ in 0..30 {
            let Some((t, _)) = sim.world.segment_hit([x, y, z], [x, bot, z]) else { break };
            let hy = y + (bot - y) * t;
            out.push(hy);
            y = hy - 0.06;
            if y < bot { break; }
        }
        out
    };
    let show = |v: &[f32]| v.iter().map(|y| format!("{:.2}", y)).collect::<Vec<_>>().join(",");
    println!("-- west edge scan (x -2..2), floors from 15.4 --");
    for z in [-113.5f32, -115.5, -117.5, -119.5] {
        print!("z {:6.1}: ", z);
        for x in [-2.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0] {
            print!(" x{:5.1}[{}] ", x, show(&fl(x, z, 15.4, 5.8)));
        }
        println!();
    }
    println!("-- east edge scan (x 10..14) --");
    for z in [-113.5f32, -115.5, -117.5, -119.5, -121.5] {
        print!("z {:6.1}: ", z);
        for x in [10.0f32, 11.0, 11.5, 12.0, 12.5, 13.0, 14.0] {
            print!(" x{:5.1}[{}] ", x, show(&fl(x, z, 15.4, 5.8)));
        }
        println!();
    }
    println!("-- south: the wall face at z -121..-123, floors from 15.4 --");
    for x in [3.0f32, 5.0, 7.0, 9.0, 11.0] {
        print!("x {:4.1}: ", x);
        for z in [-120.9f32, -121.3, -121.7, -122.1, -122.5, -123.0] {
            print!(" z{:7.1}[{}] ", z, show(&fl(x, z, 15.4, 5.8)));
        }
        println!();
    }
    println!("-- the mid floor extent at 11.27: x -2..14 at z -113.5 / -115.5 --");
    for z in [-113.5f32, -115.5] {
        print!("z {:6.1}: ", z);
        for x in [-2.0f32, -1.0, 0.0, 1.0, 2.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0] {
            let f = fl(x, z, 12.6, 9.5);
            print!(" x{:5.1}[{}] ", x, show(&f));
        }
        println!();
    }
    println!("-- the ground 6.27 under the mid floor: is the space open from the south? rays along z at y 7.0, x 5.4 --");
    for (z0, z1) in [(-117.0f32, -112.6f32), (-121.0, -117.0)] {
        let hit = sim.world.segment_hit([5.4, 7.0, z0], [5.4, 7.0, z1]);
        println!("  ray z {:6.1} -> {:6.1} at y 7.0: {}", z0, z1, if hit.is_some() { "BLOCKED" } else { "open" });
    }
    for x in [3.0f32, 7.0, 9.0] {
        let hit = sim.world.segment_hit([x, 7.0, -117.0], [x, 7.0, -112.8]);
        println!("  ray x {:4.1} z -117 -> -112.8 at y 7.0: {}", x, if hit.is_some() { "BLOCKED" } else { "open" });
    }
}

/// Drive Bond: duct top -> hole -> balcony -> find the way down to the display ground.
#[test]
fn probe_drive_to_display_floor() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 18.6, -110.8], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_mut().unwrap();
    // Leg 2-3: crouch-walk off the duct through the hole onto the balcony.
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    for _ in 0..240 {
        let inp = if sim.bond.crouched { Input { forward: 1.0, ..Default::default() } } else { Input { forward: 1.0, crouch: true, ..Default::default() } };
        sim.step_bond(inp, false, false, false);
    }
    let p = sim.bond.pos;
    eprintln!("after the drop: ({:.2}, {:.2}, {:.2}) crouch {}", p[0], p[1], p[2], sim.bond.crouched);
    assert!(p[2] < -111.5, "crossed the vent plane");
    // Uncrouch and walk south along the balcony to its south wall, logging.
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, false); }
    for tick in 1..=400 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        if tick % 50 == 0 {
            let p = sim.bond.pos;
            eprintln!("south tick {tick}: ({:.2}, {:.2}, {:.2}) gnd {}", p[0], p[1], p[2], sim.bond.grounded);
        }
    }
    let p = sim.bond.pos;
    eprintln!("at the south end: ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]);
    // Now strafe east (+x relative to facing -z => strafe -1? Input.strafe: +1 = left?) — try both.
    for s in [-1.0f32, 1.0] {
        let sim = scene.sim.as_mut().unwrap();
        let start = sim.bond.pos;
        for _ in 0..150 {
            sim.step_bond(Input { strafe: s, ..Default::default() }, false, false, false);
        }
        let p = sim.bond.pos;
        eprintln!("strafe {:+.0}: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2}) gnd {}", s, start[0], start[1], start[2], p[0], p[1], p[2], sim.bond.grounded);
    }
}

/// Drive Bond from the walkway spot: forward into the face, log. Then jump attempts.
#[test]
fn probe_walk_from_walkway() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 15.8, -107.5], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_mut().unwrap();
    let input = Input { forward: 1.0, ..Default::default() };
    for tick in 1..=300 {
        sim.step_bond(input, false, false, false);
        if tick % 30 == 0 {
            let p = sim.bond.pos;
            eprintln!("fwd tick {tick}: ({:.2}, {:.2}, {:.2}) grounded {} jump {}", p[0], p[1], p[2], sim.bond.grounded, sim.bond.jump);
        }
    }
    // Jump at the face: walk + jump repeatedly.
    for round in 0..6 {
        let sim = scene.sim.as_mut().unwrap();
        let inp = Input { forward: 1.0, jump: true, ..Default::default() };
        for _ in 0..20 { sim.step_bond(inp, false, false, false); }
        let p = sim.bond.pos;
        eprintln!("jump round {}: ({:.2}, {:.2}, {:.2})", round, p[0], p[1], p[2]);
    }
}

/// The known-good legs: duct top -> through the hole -> inner ledge -> keep walking. Log the path
/// and find the stuck point.
#[test]
fn probe_from_duct_top_down() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 18.6, -110.8], std::f32::consts::PI);
    burn_display_hinges(&mut scene);
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    for tick in 1..=1200 {
        let inp = if sim.bond.crouched { Input { forward: 1.0, ..Default::default() } } else { Input { forward: 1.0, crouch: true, ..Default::default() } };
        sim.step_bond(inp, false, false, false);
        if tick % 30 == 0 {
            let p = sim.bond.pos;
            eprintln!("tick {tick}: ({:.2}, {:.2}, {:.2}) crouch {} gnd {} jump {}", p[0], p[1], p[2], sim.bond.crouched, sim.bond.grounded, sim.bond.jump);
        }
    }
    let p = sim.bond.pos;
    eprintln!("END: ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]);
    // Where could he go from here? Floors and walls around the stuck point.
    let sim = scene.sim.as_ref().unwrap();
    let (x, y, z) = (p[0], p[1], p[2]);
    eprintln!("floors below: {:?}", floors(&sim.world, x, z, y + 0.5, y - 9.0));
    for (dx, dz, lbl) in [(0.0, -1.0, "-z"), (0.0, 1.0, "+z"), (-1.0, 0.0, "-x"), (1.0, 0.0, "+x")] {
        for dy in [0.3f32, 0.8, 1.2] {
            let hit = sim.world.segment_hit([x, y + dy, z], [x + dx * 2.0, y + dy, z + dz * 2.0]);
            if let Some((t, _)) = hit { eprintln!("wall {} dy {:.1}: hit at t {:.2}", lbl, dy, t); }
        }
    }
}
