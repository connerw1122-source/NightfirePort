//! fix-swim-vent probe 1: the [D] winding of the 07000002 garden pond floor vs the bank floor.
//! The game's ray tests hit only triangles whose STORED plane normal has `n·dir < 0`
//! (`Intersect_RayGeom` 0x10a8c8:150-168) and the stored normal = +cross(v1−v0, v2−v0)
//! (docs/formats/collision.md whole-disc check), so a floor you LAND ON from above must wind
//! +y; the pond floor, which the game's Bond falls through to swim, should wind −y.
//!
//! Run: NF_ISO=<iso> cargo run --offline --release -p nf-view --example probe_vent_swim

use nf_game::world::V3;

fn cross_y(t: &[[f32; 3]; 3]) -> f32 {
    // The Y component of cross(v1−v0, v2−v0): e1z·e2x − e1x·e2z.
    let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
    let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
    e1[2] * e2[0] - e1[0] * e2[2]
}

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_ref().expect("sim");

    println!("== all water cels ==");
    for (i, c) in scene.cels.iter().enumerate() {
        if c.water > -400.0 {
            println!("cel {i}: min {:?} max {:?} surface {:.4}", c.min, c.max, c.water);
        }
    }

    // Every water cel: the column under its centre (with windings) and the bank ring.
    for (i, c) in scene.cels.iter().enumerate() {
        if c.water <= -400.0 { continue; }
        let (cx, cz) = (0.5 * (c.min[0] + c.max[0]), 0.5 * (c.min[2] + c.max[2]));
        println!("\n== cel {i} surface {:.4} centre ({cx:.2},{cz:.2}) ==", c.water);
        let col = sim.world.tris_near([cx - 0.4, c.water - 20.0, cz - 0.4], [cx + 0.4, c.water + 4.0, cz + 0.4]);
        let mut rows: Vec<(f32, f32)> = col.iter().map(|t| {
            ((t[0][1] + t[1][1] + t[2][1]) / 3.0, cross_y(t))
        }).collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut shown = 0;
        for (y, cy) in rows {
            if shown >= 10 { break; }
            let horiz = cy.abs() > 1.0; // a real floor piece, not a sliver or a wall
            println!("  y {y:8.3}  {} ({:+.3})", if !horiz { "wall/sliver" } else if cy > 0.0 { "UP  " } else { "DOWN" }, cy);
            shown += 1;
        }
        for (dx, dz, label) in [(2.0, 0.0, "E+2"), (-2.0, 0.0, "W-2"), (0.0, 2.0, "N+2"), (0.0, -2.0, "S-2")] {
            let x = if dx > 0.0 { c.max[0] + dx } else if dx < 0.0 { c.min[0] + dx } else { cx };
            let z = if dz > 0.0 { c.max[2] + dz } else if dz < 0.0 { c.min[2] + dz } else { cz };
            let f = sim.world.floor_below([x, 9.0, z], 14.0);
            println!("  bank {label} ({x:8.2},{z:8.2}): floor {:?}", f);
        }
    }

    // The garden pond (cel 17 per fix-ladder2): x −80.3..−54.1, z 20.1..44.0, surface 3.7675.
    let c = &scene.cels.iter().enumerate().find(|(_, c)| c.water > 3.7 && c.water < 3.8 && c.max[0] < -54.0)
        .expect("the garden water cel").1.clone();
    let (cx, cz) = (0.5 * (c.min[0] + c.max[0]), 0.5 * (c.min[2] + c.max[2]));
    println!("garden water cel: min {:?} max {:?} surface {}", c.min, c.max, c.water);

    // Every static triangle within the pond's xz box between 3.5 and 4.0 (the pond floor band).
    let lo = [c.min[0], 3.4, c.min[2]];
    let hi = [c.max[0], 4.1, c.max[2]];
    let tris = sim.world.tris_near(lo, hi);
    println!("triangles in the pond floor band: {}", tris.len());
    let (mut up, mut down, mut flat) = (0usize, 0usize, 0usize);
    let mut ys: Vec<f32> = Vec::new();
    for t in &tris {
        let cy = cross_y(t);
        let y = (t[0][1] + t[1][1] + t[2][1]) / 3.0;
        ys.push(y);
        if cy.abs() < 1e-6 { flat += 1; } else if cy > 0.0 { up += 1; } else { down += 1; }
    }
    ys.sort_by(|a, b| a.total_cmp(b));
    println!("winding +y (up-facing): {up}   winding −y (down-facing): {down}   degenerate: {flat}");
    println!("triangle y range: {:?} .. {:?}", ys.first(), ys.last());
    // Split only the FLOOR-PLANE triangles (centred within ±0.07 of the plane) by winding.
    let plane: Vec<&[[f32; 3]; 3]> = tris.iter().filter(|t| {
        let y = (t[0][1] + t[1][1] + t[2][1]) / 3.0;
        (y - (c.water - 0.003)).abs() < 0.07
    }).collect();
    let (pup, pdown) = plane.iter().fold((0, 0), |(u, d), t| if cross_y(t) > 0.0 { (u + 1, d) } else { (u, d + 1) });
    println!("floor-plane triangles (y within ±0.07 of {:.3}): {} → +y {} / −y {}", c.water - 0.003, plane.len(), pup, pdown);
    let floor_tris = ys.iter().filter(|y| **y > 3.7 && **y < 3.83).count();
    println!("triangles centred in 3.70..3.83 (the pond floor plane): {floor_tris}");

    // The bank floor for contrast: a column 2 units outside the pond's west edge, 3..6.
    let bx = c.min[0] - 2.0;
    let bank = sim.world.tris_near([bx - 1.0, 3.0, cz - 3.0], [bx + 1.0, 6.0, cz + 3.0]);
    let (bup, bdown) = bank.iter().fold((0, 0), |(u, d), t| if cross_y(t) > 0.0 { (u + 1, d) } else { (u, d + 1) });
    println!("bank column x≈{bx:.1}: {bup} up-facing, {bdown} down-facing (of {})", bank.len());

    // Where are the up-winding floor-plane triangles? Centroids of each group.
    for (name, want_up) in [("up-winding floor-plane", true), ("down-winding floor-plane", false)] {
        let sel: Vec<[f32; 3]> = plane.iter().filter(|t| (cross_y(t) > 0.0) == want_up)
            .map(|t| [(t[0][0] + t[1][0] + t[2][0]) / 3.0, (t[0][1] + t[1][1] + t[2][1]) / 3.0, (t[0][2] + t[1][2] + t[2][2]) / 3.0])
            .collect();
        let n = sel.len();
        let mx = if n > 0 { sel.iter().map(|p| p[0]).sum::<f32>() / n as f32 } else { 0.0 };
        let mz = if n > 0 { sel.iter().map(|p| p[2]).sum::<f32>() / n as f32 } else { 0.0 };
        let (minx, maxx) = sel.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
        let (minz, maxz) = sel.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[2]), b.max(p[2])));
        println!("{name}: {n} tris, centroid ({mx:.1},{mz:.1}), x {minx:.1}..{maxx:.1}, z {minz:.1}..{maxz:.1}");
    }

    // A walk-in line: from the west bank edge toward the pond centre — is every floor-plane
    // triangle along it down-winding (the game's Bond falls in everywhere along the line)?
    println!("walk-in line (z = {cz:.2}):");
    for f in [0.02, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.98] {
        let x = c.max[0] + (cx - c.max[0]) * f; // from the east edge inward
        let near = tris.iter().filter(|t| {
            let y = (t[0][1] + t[1][1] + t[2][1]) / 3.0;
            (y - (c.water - 0.003)).abs() < 0.07 && t.iter().any(|v| (v[0] - x).abs() < 1.0 && (v[2] - cz).abs() < 1.0)
        }).collect::<Vec<_>>();
        let up = near.iter().filter(|t| cross_y(t) > 0.0).count();
        println!("  x {x:8.2}: {} floor-plane tris touching, {up} up-winding", near.len());
    }

    // The floor the port currently finds under the pond centre, and what a front-face-only ray gets.
    let from = [cx, 6.0, cz];
    let hit = sim.world.segment_surface(from, [cx, -10.0, cz], true);
    println!("port segment_surface down the pond centre: {:?}", hit.map(|(t, n, m)| (6.0 + (-16.0) * t, n, m)));

    // The full column under the pond centre with windings: every static triangle whose xz is within
    // 0.4 of (cx, cz), sorted by y, with its winding sign.
    let col = sim.world.tris_near([cx - 0.4, -30.0, cz - 0.4], [cx + 0.4, 6.0, cz + 0.4]);
    let mut rows: Vec<(f32, f32, u8)> = col.iter().map(|t| {
        let y = (t[0][1] + t[1][1] + t[2][1]) / 3.0;
        (y, cross_y(t), 0)
    }).collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("column under the pond centre ({} tris, top→bottom):", rows.len());
    for (y, cy, _) in rows.iter().take(24) {
        println!("  y {y:8.3}  winding {} ({:+.4})", if *cy > 0.0 { "UP  " } else if *cy < 0.0 { "DOWN" } else { "FLAT" }, cy);
    }

    // Bank candidates around the pond: probe a ring of columns 2..8 units outside the bbox for the
    // highest up-facing floor within 3..8, i.e. where the user would walk from.
    println!("bank ring (floor_below one-sided, from y=8):");
    for (dx, dz, label) in [(1.0, 0.0, "E+1"), (2.0, 0.0, "E+2"), (4.0, 0.0, "E+4"), (-1.0, 0.0, "W-1"), (-2.0, 0.0, "W-2"), (0.0, 2.0, "N+2"), (0.0, -2.0, "S-2")] {
        let x = if dx > 0.0 { c.max[0] + dx } else if dx < 0.0 { c.min[0] + dx } else { cx };
        let z = if dz > 0.0 { c.max[2] + dz } else if dz < 0.0 { c.min[2] + dz } else { cz };
        let f = sim.world.floor_below([x, 8.0, z], 12.0);
        println!("  {label} ({x:8.2},{z:8.2}): floor {:?}", f);
    }
    materials_main();
}

// (appended by the fix-swim-vent session) The material probe: for every water cel, the first
// one-sided hit straight down from just above the surface, on a grid, with the raw material byte
// (values ≥ 64 carry the game's 0x40 "no ground / no push" flag [M Collide_Filter 0x1ed888 +
// Collide_Intersect 0x1ebaa8:305-307]).
#[allow(dead_code)]
fn materials_main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_ref().expect("sim");
    for (i, c) in scene.cels.iter().enumerate() {
        if c.water <= -400.0 { continue; }
        let s = c.water;
        println!("== cel {i} surface {s:.4} ==");
        let nx = 6; let nz = 6;
        for iz in 0..nz {
            let z = c.min[2] + (c.max[2] - c.min[2]) * (iz as f32 + 0.5) / nz as f32;
            let mut row = String::new();
            for ix in 0..nx {
                let x = c.min[0] + (c.max[0] - c.min[0]) * (ix as f32 + 0.5) / nx as f32;
                match sim.world.segment_surface([x, s + 3.0, z], [x, s - 25.0, z], false) {
                    Some((t, n, m)) => {
                        let y = s + 3.0 + (s - 25.0 - s - 3.0) * t;
                        row.push_str(&format!("[y{y:7.2} m{m:3} ny{:+.2}] ", n[1]));
                    }
                    None => row.push_str("[none]            ".to_string().as_str()),
                }
            }
            println!("  z {z:8.2}: {row}");
        }
    }
}
