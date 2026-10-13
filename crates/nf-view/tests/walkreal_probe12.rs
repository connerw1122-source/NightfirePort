//! walkreal-b probe: 07000012/13/14 route facts — the doors, the nav coverage along the route,
//! and a stepped walk toward the 12 console to locate the wedge. Diagnostic only (--nocapture).

#[path = "common/walker.rs"]
mod walker;

use walker::{Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn probe_12_13_14() {
    let iso = iso();
    for code in ["07000012", "07000013", "07000014"] {
        let scene = match nf_view::load(&iso, &format!("{code}.bin"), false) {
            Ok(s) => s,
            Err(e) => { eprintln!("=== {code}: LOAD FAIL {e}"); continue; }
        };
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("=== {code} === bond {:?}", sim.bond.pos);
        for (i, d) in sim.drones.doors.list.iter().enumerate() {
            eprintln!("door[{i}] at=({:.1},{:.1},{:.1}) kind={} ch_open={:#x} ch_lock={:#x} state={:#x}",
                d.pos[0], d.pos[1], d.pos[2], d.kind, d.ch_open, d.ch_lock, d.state);
        }
        // Nav node coverage: the nearest nodes to the route ends (a coarse sanity read).
        drop(sim);
        drop(scene);
    }

    // 12: the stepped walk toward the console — where does the direct steer wedge?
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000012.bin", false, None).unwrap());
    w.wait_enabled(3000, "intro");
    w.wait(120);
    {
        // The nav paths and their node extents, plus wall segments near the wedge.
        let sim = w.scene.sim.as_ref().unwrap();
        for (pi, p) in sim.nav.paths.iter().enumerate() {
            let (mut x0, mut x1, mut z0, mut z1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for n in &p.nodes {
                x0 = x0.min(n.pos[0]); x1 = x1.max(n.pos[0]);
                z0 = z0.min(n.pos[2]); z1 = z1.max(n.pos[2]);
            }
            eprintln!("nav path {pi}: {} nodes x {x0:.1}..{x1:.1} z {z0:.1}..{z1:.1} y {:.1}..{:.1}",
                p.nodes.len(),
                p.nodes.iter().map(|n| n.pos[1]).fold(f32::MAX, f32::min),
                p.nodes.iter().map(|n| n.pos[1]).fold(f32::MIN, f32::max));
        }
        eprintln!("walls near the wedge (x 3..12, z 8..14):");
        for wl in &sim.nav.walls {
            if wl.a[0] > 2.0 && wl.a[0] < 13.0 && wl.a[2] > 7.0 && wl.a[2] < 15.0
                && wl.b[0] > 2.0 && wl.b[0] < 13.0 && wl.b[2] > 7.0 && wl.b[2] < 15.0 {
                eprintln!("  wall ({:.1},{:.1})-({:.1},{:.1}) h {:.1}/{:.1} flags {:#x}", wl.a[0], wl.a[2], wl.b[0], wl.b[2], wl.ha, wl.hb, wl.flags);
            }
        }
    }
    let routes: &[(&str, Vec<(f32, f32, f32)>)] = &[
        ("south-door19", vec![(14.0, 0.4, 6.0), (10.0, 0.4, 2.0), (9.0, 0.4, -1.5), (5.0, 0.4, -1.5), (1.0, 0.4, 4.0), (-1.55, 0.5, 7.2)]),
    ];
    for (name, wps) in routes {
        let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000012.bin", false, None).unwrap());
        w.wait_enabled(3000, "intro");
        w.wait(120);
        let mut ok = true;
        for (k, p) in wps.iter().enumerate() {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                w.walk_to(Wp::new(p.0, p.1, p.2).r(1.0), &format!("probe12 {name} leg {k}"))
            }));
            let sim = w.scene.sim.as_ref().unwrap();
            eprintln!("probe12 {name} leg {k} {:?} -> bond {:?}", p, sim.bond.pos);
            if r.is_err() { ok = false; break; }
        }
        eprintln!("probe12 route {name}: {}", if ok { "ARRIVED" } else { "WEDGED" });
    }
    // Cast a chest-high ray from the wedge toward the console and print what stops it.
    {
        let sim = w.scene.sim.as_ref().unwrap();
        for y in [0.6f32, 1.2, 1.8] {
            let from = [7.6, y, 11.5];
            let to = [-1.55, y, 7.2];
            match sim.world.segment_surface(from, to) {
                Some((t, n, class)) => {
                    let at = [from[0] + (to[0] - from[0]) * t, y, from[2] + (to[2] - from[2]) * t];
                    eprintln!("ray y {y}: hit t {t:.2} at ({:.1},{:.1}) normal {:?} class {class}", at[0], at[2], n);
                }
                None => eprintln!("ray y {y}: clear"),
            }
        }
    }
}
