//! walkreal-b2 probe: why does the 0700000a lobby walk pin at the lift-cab mouth (z ≈ 12.4)?
//! Dumps the SS leaves near the shaft mouth, the segment hits along the stuck line, and every
//! channel that turns on over the first ticks. Diagnostic — run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn probe_0a_lift_mouth() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700000a.bin", false).expect("load 0700000a");
    eprintln!("=== 0a: the Ss list (placement, channel, matrix[3], rate, t, keys) ===");
    {
        let sim = scene.sim.as_ref().unwrap();
        for s in &sim.drones.sss.list {
            eprintln!("ss placement={} ch={:#04x} at=({:.2},{:.2},{:.2}) rate={} t={:.2} keys={}",
                s.placement, s.channel, s.matrix[3][0], s.matrix[3][1], s.matrix[3][2], s.rate, s.t, s.keys.len());
        }
    }
    // Drive the real input path for 600 ticks, sampling.
    for tick in 0..600 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false);
        if tick % 60 == 0 {
            let sim = scene.sim.as_ref().unwrap();
            let from = sim.bond.pos;
            let to = [-3.9f32, from[1], 15.5];
            eprintln!("--- tick {tick}: bond=({:.2},{:.2},{:.2}) grounded={} ss states:",
                from[0], from[1], from[2], sim.bond.grounded);
            for s in &sim.drones.sss.list {
                if s.matrix[3][2] > 10.0 && s.matrix[3][2] < 17.0 {
                    eprintln!("    ss placement={} ch={:#04x} at=({:.2},{:.2},{:.2}) t={:.2}",
                        s.placement, s.channel, s.matrix[3][0], s.matrix[3][1], s.matrix[3][2], s.t);
                }
            }
            match sim.world.segment_hit(from, to) {
                Some((t, surf)) => eprintln!("    seg→(−3.9,·,15.5): t={t:.3} surf={surf:?} hit=({:.2},{:.2},{:.2})",
                    from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t, from[2] + (to[2] - from[2]) * t),
                None => eprintln!("    seg→(−3.9,·,15.5): clear"),
            }
            // The ray at knee and head height too (is the blocker a full wall or a low sill?).
            let knee = [from[0], from[1] + 0.3, from[2]];
            let head = [from[0], from[1] + 1.4, from[2]];
            eprintln!("    knee seg: {:?}  head seg: {:?}",
                sim.world.segment_hit(knee, [to[0], knee[1] + 0.3, to[2]]).map(|(t, _)| t),
                sim.world.segment_hit(head, [to[0], head[1] + 1.4, to[2]]).map(|(t, _)| t));
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    let _ = walker::fail_dump_pub(&scene, [-3.9, 0.0, 15.5], "the lobby south");
}
