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

    // Poke Bond onto the pad centre and step: does the 0x1b touch fire at all?
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [5.84, 12.9, -154.15];
    sim.bond.yaw = 0.0;
    for t in 0..40 {
        sim.step_bond(Default::default(), false, false, false);
        if t % 10 == 0 {
            println!("poke t {t}: bond {:?} cel {:?} ch0x1b {}", sim.bond.pos, sim.bond.cel, sim.drones.channels.on(0x1b));
        }
    }
    println!("poke end: ch0x1b {} bond {:?}", sim.drones.channels.on(0x1b), sim.bond.pos);
    // And at the test's stop spot (0.03 outside the z band).
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [6.8, 12.9, -153.15];
    for t in 0..40 {
        sim.step_bond(Default::default(), false, false, false);
    }
    println!("edge spot: ch0x1b {} bond {:?}", sim.drones.channels.on(0x1b), sim.bond.pos);

    // Floor map of the corridor→pad seam (flat down-rays from y 12.9, depth 8).
    let sim = scene.sim.as_ref().unwrap();
    for z in [-152.8f32, -153.0, -153.2, -153.4, -153.6, -153.9, -154.2] {
        let mut row = format!("z {z:7.2}: ");
        for x in [4.5f32, 5.0, 5.5, 6.0, 6.5, 7.0] {
            let h = sim.world.segment_hit([x, 12.9, z], [x, 4.9, z]);
            row.push_str(&match h { Some((t, _)) => format!("x{x:.1}={:.2}  ", 12.9 - 8.0 * t), None => format!("x{x:.1}=—     ") });
        }
        println!("{row}");
    }

    // Fine seam map: z −153.02..−153.22, x 5.4..7.3 (flat down-ray from y 12.5).
    let sim = scene.sim.as_ref().unwrap();
    for z in [-153.02f32, -153.08, -153.14, -153.2] {
        let mut row = format!("z {z:7.2}: ");
        for x in [5.4f32, 5.8, 6.0, 6.2, 6.5, 6.8, 7.0, 7.2] {
            let h = sim.world.segment_hit([x, 12.5, z], [x, 4.5, z]);
            row.push_str(&match h { Some((t, _)) => format!("x{x:.1}={:.2} ", 12.5 - 8.0 * t), None => format!("x{x:.1}=—    ") });
        }
        println!("{row}");
    }

    // The seam triangles' materials (the water drop drops mat 0xE from the player's capsule).
    let sim = scene.sim.as_ref().unwrap();
    for (x, z) in [(6.8f32, -153.1f32), (6.0, -153.1), (5.8, -153.3), (5.84, -154.15), (6.8, -153.4)] {
        let h = sim.world.segment_surface([x, 12.5, z], [x, 4.5, z], false);
        println!("mat ({x:.2},{z:.2}): {}", h.map_or("—".into(), |(_, _, m)| format!("{m:02x}")));
    }

}
