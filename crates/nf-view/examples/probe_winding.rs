//! Throwaway probe (fix-swim-vent2). READ-ONLY: answers three questions against the real disc
//! geometry through the same `CollisionWorld` the sim walks on:
//! 1. Do the real floors' winding normals point up? (grid of downward `segment_surface` rays;
//!    the returned normal is the winding cross, normalized — world.rs segment_surface.)
//! 2. The 07000008 missile cage: which faces stop which rays (the failing bullet test casts
//!    x 136 → 134.5 at y 48.6, z −101; the wall is at x 135.3)?
//! 3. 07000001 Mayhew's stall spot (−33.0, 4.54, 11.8): what does the capsule feel there?
//!
//! Run: cargo run --offline --release -p nf-view --example probe_winding -- <iso> <level>

use nf_view::scene::Cel;

fn main() {
    let mut args = std::env::args().skip(1);
    let iso = args.next().unwrap_or_else(|| std::env::var("NF_ISO").unwrap_or_default());
    let level = args.next().unwrap_or_else(|| "07000008.bin".to_string());
    if iso.is_empty() {
        eprintln!("usage: probe_winding <iso> [level]");
        std::process::exit(2);
    }
    eprintln!("loading {iso} {level} …");
    let scene = nf_view::load(&iso, &level, false).expect("load");
    let cels: &Vec<Cel> = &scene.cels;
    let sim = scene.sim.as_ref().expect("sim");

    // 1. Floor winding: down-rays across the cel bboxes' union, from 3 above to 30 below.
    let (mut up, mut down, mut horiz, mut none) = (0u32, 0u32, 0u32, 0u32);
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for c in cels {
        for k in 0..3 {
            lo[k] = lo[k].min(c.min[k]);
            hi[k] = hi[k].max(c.max[k]);
        }
    }
    let step = 4.0;
    let mut z = lo[2];
    while z <= hi[2] {
        let mut x = lo[0];
        while x <= hi[0] {
            let from = [x, hi[1] + 3.0, z];
            if let Some((_, n, m)) = sim.world.segment_surface(from, [x, lo[1] - 30.0, z], false) {
                if n[1] > 0.3 { up += 1; }
                else if n[1] < -0.3 { down += 1; }
                else { horiz += 1; }
                let _ = m;
            } else {
                none += 1;
            }
            x += step;
        }
        z += step;
    }
    println!("floors: n.y>0 (up) {up}, n.y<0 (DOWN) {down}, |n.y|<=0.3 {horiz}, no-hit {none}");

    if level.starts_with("07000008") {
        // 2. The cage: the failing test's exact rays, plus the reverses.
        let rays: [(&str, [f32; 3], [f32; 3]); 6] = [
            ("wall fwd (test) ", [136.0, 48.6, -101.0], [134.5, 48.6, -101.0]),
            ("wall rev        ", [134.5, 48.6, -101.0], [136.0, 48.6, -101.0]),
            ("wall fwd z-113  ", [136.0, 48.6, -113.0], [134.5, 48.6, -113.0]),
            ("wall rev z-113  ", [134.5, 48.6, -113.0], [136.0, 48.6, -113.0]),
            ("lid up (test)   ", [139.0, 50.0, -97.0], [139.0, 56.0, -97.0]),
            ("lid down        ", [139.0, 56.0, -97.0], [139.0, 50.0, -97.0]),
        ];
        for (name, a, b) in rays {
            match sim.world.segment_surface(a, b, false) {
                Some((t, n, m)) => {
                    let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
                    println!("{name}: HIT t {t:.4} at [{:.2} {:.2} {:.2}] n [{:.3} {:.3} {:.3}] mat {m:#x}", p[0], p[1], p[2], n[0], n[1], n[2]);
                }
                None => println!("{name}: MISS"),
            }
        }
    }

    if level.starts_with("07000001") {
        // 3. Mayhew's stall: capsule pushes at and around the stall spot, toward the plate
        // (the plate centre comes from the test log; use the stall spot the report gives).
        let spot = [-33.0, 4.54, 11.8];
        let dirs: [(&str, f32, f32); 4] = [("+x", 1.0, 0.0), ("-x", -1.0, 0.0), ("+z", 0.0, 1.0), ("-z", 0.0, -1.0)];
        for (name, dx, dz) in dirs {
            for (r, dy) in [(0.55, 0.0), (0.55, -0.5), (0.25, 0.0)] {
                let (push, walked) = sim.world.capsule_push([spot[0], spot[1] + dy, spot[2]], [spot[0] + dx * 0.5, spot[1] + dy, spot[2] + dz * 0.5], r);
                println!("capsule at spot dir {name} r {r} dy {dy}: push [{:.3} {:.3} {:.3}] walkable {walked}", push[0], push[1], push[2]);
            }
        }
        // Also: the floor under the spot and under the plate.
        for (name, p) in [("stall spot", spot), ("near plate", [-36.6, 2.3, 11.8])] {
            match sim.world.segment_surface([p[0], p[1] + 1.0, p[2]], [p[0], p[1] - 6.0, p[2]], false) {
                Some((t, n, m)) => {
                    let y = p[1] + 1.0 + (-7.0) * t;
                    println!("{name}: floor y {y:.3} n.y {:.3} mat {m:#x}", n[1]);
                }
                None => println!("{name}: no floor within 6"),
            }
        }
    }
}
