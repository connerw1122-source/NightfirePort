//! hatch-crouch-0c scratch probe: the balcony's south-wall wedge (z ≈ −120.8..−121.5) and the
//! descent route off the balcony. Diagnostics with eprintln; run with --nocapture. NOT a test.

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

/// All downward hits in a column (walk the ray down).
fn floors(world: &nf_game::world::CollisionWorld, x: f32, z: f32, top: f32, bottom: f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut y = top;
    for _ in 0..60 {
        let Some((t, hit)) = world.segment_hit([x, y, z], [x, bottom, z]) else { break };
        let hy = y + (bottom - y) * t;
        out.push(hy);
        y = hy - 0.06;
        if y < bottom { break; }
    }
    out
}

/// The wedge profile: every floor per column around the balcony's south edge.
#[test]
fn probe_wedge_profile() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_ref().unwrap();
    for z in [-119.0f32, -119.5, -120.0, -120.3, -120.6, -120.9, -121.1, -121.3, -121.5, -121.7, -122.0, -122.5, -123.0] {
        print!("z {:7.2}: ", z);
        for x in [4.0f32, 5.0, 5.8, 7.0, 8.5, 10.5, 12.5] {
            let fl = floors(&sim.world, x, z, 16.8, 5.0);
            let s: Vec<String> = fl.iter().map(|y| format!("{:.2}", y)).collect();
            print!(" x{:4.1}[{}] ", x, s.join(","));
        }
        println!();
    }
}

/// Rays crossing the south-wall planes at crouch heights: where is the passage, if any.
#[test]
fn probe_wedge_rays() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_ref().unwrap();
    for z in [-120.7f32, -121.0, -121.3, -121.6, -122.0] {
        println!("-- rays crossing z {:.2} (x, y): hit? --", z);
        for y in [14.65f32, 14.8, 14.95, 15.1, 15.25, 15.4, 15.6, 15.9, 16.2] {
            print!(" y{:5.2}: ", y);
            for x in [4.0f32, 5.0, 5.8, 7.0, 8.5, 10.0] {
                let hit = sim.world.segment_hit([x, y, z + 0.5], [x, y, z - 0.5]);
                print!("({:.1}){} ", x, if hit.is_some() { "X" } else { "." });
            }
            println!();
        }
    }
}

/// The oscillation repro: crouch-walk south along the balcony into the wedge, log everything.
#[test]
fn probe_wedge_walk() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    assert!(sim.bond.crouched, "crouched");
    for tick in 1..=500u32 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        if tick % 10 == 0 {
            let p = sim.bond.pos;
            eprintln!("tick {:3}: pos ({:.3}, {:.3}, {:.3}) grounded {} cc {:.2} vel_y {:.3}",
                tick, p[0], p[1], p[2], sim.bond.grounded, sim.bond.cc, sim.bond.vel[1]);
        }
        let p = sim.bond.pos;
        if p[2] < -123.0 || p[1] < 13.0 { eprintln!("left the wedge at tick {tick}"); break; }
    }
}

/// The east-descent route: floors from the balcony's south-east down to the well ground.
#[test]
fn probe_east_descent() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_ref().unwrap();
    for z in [-116.5f32, -117.5, -118.5, -119.5, -120.5, -121.5, -122.5, -123.5, -125.0, -127.0] {
        print!("z {:7.2}: ", z);
        for x in [8.0f32, 9.5, 10.5, 11.5, 12.5, 14.0] {
            let fl = floors(&sim.world, x, z, 15.0, 5.5);
            let s: Vec<String> = fl.iter().map(|y| format!("{:.2}", y)).collect();
            print!(" x{:4.1}[{}] ", x, s.join(","));
        }
        println!();
    }
}

/// Where does the open hatch's collision sit? Horizontal rays through the vent zone with the
/// hatch open, at the panel heights; print every hit z. Same with the hatch shut for contrast.
#[test]
fn probe_panel_collision() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    // Shut: load, no burn.
    let scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    {
        let sim = scene.sim.as_ref().unwrap();
        println!("== hatch SHUT: rays z -110.5 .. -115.5 ==");
        for y in [13.6f32, 14.2, 14.76, 15.3, 15.9, 16.5, 17.2, 17.6] {
            print!(" y{:5.2}: ", y);
            for x in [5.4f32, 5.8, 6.4] {
                let hits = z_hits(&sim.world, x, y, -110.5, -115.5);
                let s: Vec<String> = hits.iter().map(|z| format!("{:.2}", z)).collect();
                print!(" x{:.1}[{}] ", x, s.join(","));
            }
            println!();
        }
    }
    drop(scene);
    // Open: burn + wait out the sweep.
    let mut scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    {
        let sim = scene.sim.as_mut().unwrap();
        for k in 0..sim.drones.locks.len() {
            if (14..=17).contains(&sim.drones.locks[k].out) {
                sim.drones.locks[k].burn(&mut sim.drones.channels, 1);
            }
        }
    }
    for _ in 0..200 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    println!("== hatch OPEN: rays z -110.5 .. -115.5 ==");
    for y in [13.6f32, 14.2, 14.76, 15.3, 15.9, 16.5, 17.2, 17.6] {
        print!(" y{:5.2}: ", y);
        for x in [5.4f32, 5.8, 6.4] {
            let hits = z_hits(&sim.world, x, y, -110.5, -115.5);
            let s: Vec<String> = hits.iter().map(|z| format!("{:.2}", z)).collect();
            print!(" x{:.1}[{}] ", x, s.join(","));
        }
        println!();
    }
}

/// All hits along a horizontal +z..-z ray.
fn z_hits(world: &nf_game::world::CollisionWorld, x: f32, y: f32, z0: f32, z1: f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut z = z0;
    for _ in 0..40 {
        let Some((t, _)) = world.segment_hit([x, y, z], [x, y, z1]) else { break };
        let hz = z + (z1 - z) * t;
        out.push(hz);
        z = hz + 0.05 * (z1 - z).signum();
        if (z1 - z).signum() != (z1 - z0).signum() { break; }
    }
    out
}

/// The sister: STANDING walk south along the balcony into the wedge.
#[test]
fn probe_standing_south() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_mut().unwrap();
    for tick in 1..=400u32 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        if tick % 20 == 0 {
            let p = sim.bond.pos;
            eprintln!("tick {:3}: pos ({:.3}, {:.3}, {:.3}) grounded {} cc {:.2} crouched {}",
                tick, p[0], p[1], p[2], sim.bond.grounded, sim.bond.cc, sim.bond.crouched);
        }
        let p = sim.bond.pos;
        if p[2] < -123.0 || p[1] < 13.0 { eprintln!("moved past at tick {tick}"); break; }
    }
}

/// South of the landing: where does the 16.49 slab lead?
#[test]
fn probe_south_landing() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_ref().unwrap();
    for z in [-122.5f32, -123.5, -124.5, -125.5, -126.5, -128.0, -130.0] {
        print!("z {:7.2}: ", z);
        for x in [3.0f32, 4.5, 6.0, 7.5, 9.0] {
            let fl = floors(&sim.world, x, z, 20.0, 5.0);
            let s: Vec<String> = fl.iter().map(|y| format!("{:.2}", y)).collect();
            print!(" x{:4.1}[{}] ", x, s.join(","));
        }
        println!();
    }
}

/// Fine z scan of the last riser (z -121.3 .. -122.1).
#[test]
fn probe_last_riser() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let scene = load_at([5.8, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_ref().unwrap();
    let mut z = -121.30f32;
    while z >= -122.10 {
        let fl = floors(&sim.world, 5.8, z, 17.5, 14.0);
        let s: Vec<String> = fl.iter().map(|y| format!("{:.2}", y)).collect();
        println!("z {:7.2}: x5.8 [{}]", z, s.join(","));
        z -= 0.05;
    }
}

/// Crouch-walk south along the balcony's EAST side (x 11, past the wedge).
#[test]
fn probe_east_walk() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set"); return };
    let mut scene = load_at([11.0, 15.7, -116.0], std::f32::consts::PI);
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Input { crouch: true, ..Default::default() }, false, false, false);
    for tick in 1..=400u32 {
        sim.step_bond(Input { forward: 1.0, ..Default::default() }, false, false, false);
        if tick % 40 == 0 {
            let p = sim.bond.pos;
            eprintln!("tick {:3}: pos ({:.3}, {:.3}, {:.3}) grounded {}", tick, p[0], p[1], p[2], sim.bond.grounded);
        }
        let p = sim.bond.pos;
        if p[2] < -126.0 { eprintln!("reached the far walkway at tick {tick}"); break; }
    }
}
