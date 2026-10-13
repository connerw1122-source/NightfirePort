//! Throwaway probe (fix-swim-vent3). READ-ONLY: the 0700000c ST-1 plate #506 touch (out 0x1b).
//! Casts the grounding ray at the plate centre through the FLAT segment_hit and the SCOPED
//! segment_hit_scoped (Bond's runtime cel) — the cel scoping is the remaining suspect for the
//! pad the walk never stands on.
//!
//! Run: cargo run --offline --release -p nf-view --example probe_scoped -- <iso> [level]

fn main() {
    let mut args = std::env::args().skip(1);
    let iso = args.next().unwrap_or_else(|| std::env::var("NF_ISO").unwrap_or_default());
    let level = args.next().unwrap_or_else(|| "0700000c.bin".into());
    eprintln!("loading {iso} {level} …");
    let mut scene = nf_view::load(&iso, &level, false).expect("load");
    // A few idle ticks so the Sim relinks Bond's cel (obj+0x20) and the touch volumes exist.
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let touch = sim.drones.triggers.iter().find(|t| t.placement == 506).expect("the plate 506 trigger");
    let cx = (touch.lo[0] + touch.hi[0]) * 0.5;
    let cy = (touch.lo[1] + touch.hi[1]) * 0.5;
    let cz = (touch.lo[2] + touch.hi[2]) * 0.5;
    println!("trigger out 0x1b lo {:?} hi {:?} centre ({cx:.2},{cy:.2},{cz:.2}) placement {}", touch.lo, touch.hi, touch.placement);
    println!("bond pos {:?} cel {:?}", sim.bond.pos, sim.bond.cel);
    for (dx, dz) in [(0.0f32, 0.0f32), (0.5, 0.0), (-0.5, 0.0), (0.0, 0.5), (0.0, -0.5)] {
        let p = [cx + dx, cy + 1.0, cz + dz];
        let flat = sim.world.segment_hit(p, [p[0], p[1] - 8.0, p[2]]);
        let scoped = sim.world.segment_hit_scoped(sim.bond.cel, p, [p[0], p[1] - 8.0, p[2]]);
        println!("({:.2},{:.2}) flat {} scoped {}", p[0], p[2],
            flat.map_or("—".into(), |(t, n)| format!("y {:.2} ny {:.2}", p[1] - 8.0 * t, n[1])),
            scoped.map_or("—".into(), |(t, n)| format!("y {:.2} ny {:.2}", p[1] - 8.0 * t, n[1])));
    }
}
