//! Throwaway probe (fix-ladder follow-up, swim-entry investigation). READ-ONLY: loads a level through
//! the same path the viewer uses (`nf_view::load`) and prints, for every room cel and every water cel:
//! the bbox, the water surface, the collision floor under the cel (centre + a small grid, from the same
//! `CollisionWorld` the sim walks on), and the entry verdicts for both the game's formula
//! (`Player_InWater` 0x195e50: `pos.y − 0.3 < cel+0x90` with pos = floor + 1.1) and the port's cel
//! lookup (`Sim::cel_at(bond.pos)`). Also probes the 07000002 garden bank at the bunker-exit ladder
//! (−26.9, 2.3, 42.2).
//!
//! Run: cargo run --offline --release -p nf-view --example probe_swim -- <iso> <level>

use nf_view::scene::Cel;

fn main() {
    let mut args = std::env::args().skip(1);
    let iso = args.next().unwrap_or_else(|| std::env::var("NF_ISO").unwrap_or_default());
    let level = args.next().unwrap_or_else(|| "07000002.bin".to_string());
    if iso.is_empty() {
        eprintln!("usage: probe_swim <iso> [level]  (or NF_ISO=<iso>)");
        std::process::exit(2);
    }
    eprintln!("loading {iso} {level} …");
    let scene = match nf_view::load(&iso, &level, false) {
        Ok(s) => s,
        // The FILES.BIN entries carry the `.bin` suffix; try both spellings.
        Err(_) if !level.ends_with(".bin") => nf_view::load(&iso, &format!("{level}.bin"), false).expect("load"),
        Err(e) => return Err::<(), _>(e).expect("load"),
    };
    let cels: &Vec<Cel> = &scene.cels;
    let sim = scene.sim.as_ref().expect("sim");
    println!("== {level}: {} room cels ==", cels.len());
    for (i, c) in cels.iter().enumerate() {
        let kind = if c.water > -400.0 { "WATER" } else { "     " };
        println!(
            "cel {i:2} {kind} min [{:8.3} {:8.3} {:8.3}] max [{:8.3} {:8.3} {:8.3}] surface {:7.3}",
            c.min[0], c.min[1], c.min[2], c.max[0], c.max[1], c.max[2], c.water
        );
    }

    // Every candidate of build_FindCel's bbox walk at a point (what `find_cel` reduces to one).
    let candidates = |p: [f32; 3]| -> Vec<usize> {
        (0..cels.len())
            .filter(|&i| {
                let c = &cels[i];
                (0..3).all(|k| c.min[k] <= p[k] && p[k] <= c.max[k])
            })
            .collect()
    };
    let describe = |p: [f32; 3]| -> String {
        let cand = candidates(p);
        let pick = nf_view::scene::find_cel(cels, p);
        format!(
            "candidates {:?} → port pick {:?} (water {:?})",
            cand,
            pick,
            pick.map(|i| cels[i].water)
        )
    };

    // Straight down from above the surface, well past any plausible pool bottom.
    let floor_at = |x: f32, z: f32, surface: f32| -> Option<(f32, f32, u8)> {
        let from = [x, surface + 3.0, z];
        sim.world
            .segment_surface(from, [x, surface - 30.0, z], false)
            .map(|(t, n, m)| (from[1] + (surface - 30.0 - from[1]) * t, n[1], m))
    };

    println!("\n== water cels: floor vs surface ==");
    for (i, c) in cels.iter().enumerate() {
        if c.water <= -400.0 {
            continue;
        }
        let surface = c.water;
        let cx = (c.min[0] + c.max[0]) * 0.5;
        let cz = (c.min[2] + c.max[2]) * 0.5;
        println!(
            "-- cel {i}: surface {surface:.3}  bbox x {:7.2}..{:7.2}  z {:7.2}..{:7.2}  (pool spans {:.1} x {:.1})",
            c.min[0], c.max[0], c.min[2], c.max[2],
            c.max[0] - c.min[0], c.max[2] - c.min[2]
        );
        // All collision surfaces down the centre column (is there a basin BELOW the surface plane?).
        let mut y_from = surface + 3.0;
        let mut hits = String::new();
        for _ in 0..8 {
            let Some((t, n, m)) = sim
                .world
                .segment_surface([cx, y_from, cz], [cx, surface - 30.0, cz], false)
            else {
                if hits.is_empty() {
                    hits.push_str(" none");
                }
                break;
            };
            let y = y_from + (surface - 30.0 - y_from) * t;
            hits.push_str(&format!(" [y={y:.3} mat={m} ny={:.2}]", n[1]));
            if y <= surface - 30.0 {
                break;
            }
            y_from = y - 0.05; // step just below this plane and keep casting
        }
        println!("   centre column({cx:.2},{cz:.2}) downward:{hits}");
        let samples = [
            (cx, cz),
            (c.min[0] + 0.5, c.min[2] + 0.5),
            (c.max[0] - 0.5, c.min[2] + 0.5),
            (c.min[0] + 0.5, c.max[2] - 0.5),
            (c.max[0] - 0.5, c.max[2] - 0.5),
        ];
        for (x, z) in samples {
            match floor_at(x, z, surface) {
                Some((y, ny, m)) => {
                    let depth = surface - y;
                    // Standing on that floor: the game's object origin sits 1.1 above it
                    // (`Player_StandAtNewPosition`, the port's Bond::stand_at floor + 1.1).
                    let pos_y = y + 1.1;
                    let game_test = pos_y - 0.3 < surface; // [M 0x195e50]
                    let sphere_y = pos_y - 0.1267; // Bond::sphere_centre LOCAL y [L ee_start_05]
                    let sphere_in_bbox = sphere_y >= c.min[1] && sphere_y <= c.max[1]
                        && x >= c.min[0] && x <= c.max[0] && z >= c.min[2] && z <= c.max[2];
                    let port_lookup = nf_view::scene::find_cel(cels, [x, pos_y, z])
                        .map(|k| cels[k].water)
                        .filter(|w| *w > -400.0);
                    let port_entry = port_lookup.map_or(false, |w| pos_y - 0.3 < w);
                    println!(
                        "   floor({:8.2},{:8.2}) = {:8.3}  depth {:6.3}  ny {:5.2} mat {:3} | game: test {} + sphere-in-cel {} = {} | port: lookup {:?} → {}",
                        x, z, y, depth, ny, m, game_test, sphere_in_bbox, game_test && sphere_in_bbox, port_lookup, port_entry
                    );
                }
                None => println!("   floor({x:8.2},{z:8.2}) = — no collision within 30 below the surface"),
            }
        }
        // Where does the port's lookup point at the standing height land? (overlap structure)
        let stand = match floor_at(cx, cz, surface) {
            Some((y, _, _)) => [cx, y + 1.1, cz],
            None => [cx, surface - 1.1, cz],
        };
        println!("   standing at centre {:?}: {}", &stand[..], describe(stand));
    }

    println!("\n== garden bank / bunker exit (−26.9, 2.3, 42.2) ==");
    let bank_y0 = 6.0f32;
    match sim.world.segment_surface([-26.9, bank_y0, 42.2], [-26.9, bank_y0 - 30.0, 42.2], false) {
        Some((t, _, m)) => {
            let y = bank_y0 + (bank_y0 - 30.0 - bank_y0) * t;
            println!("bank floor y = {y:.3} (mat {m}); standing y = {:.3}", y + 1.1);
            println!("  cel at standing point: {}", describe([-26.9, y + 1.1, 42.2]));
            println!("  5 units −z: {}", describe([-26.9, y + 1.1, 37.0]));
            println!("  5 units +z: {}", describe([-26.9, y + 1.1, 47.2]));
        }
        None => println!("bank: no floor within 30"),
    }

    println!("\n== entry band per water cel (how deep must the pool be?) ==");
    for (i, c) in cels.iter().enumerate() {
        if c.water <= -400.0 {
            continue;
        }
        let s = c.water;
        // Port: the lookup point pos = floor+1.1 must be inside the water bbox (max y = surface)
        // AND pos.y − 0.3 < surface → binding: depth = surface − floor ≥ 1.1.
        // Game: the LINKED cel comes from the sphere centre at floor+0.973 (must be inside the bbox)
        // AND pos.y − 0.3 < surface → binding: depth ≥ 0.973.
        println!(
            "cel {i}: surface {s:.3} | port enters when depth ≥ 1.100 (floor < {:.3}); game enters when depth ≥ 0.973 (floor < {:.3})",
            s - 1.1,
            s - 0.973
        );
    }
}
