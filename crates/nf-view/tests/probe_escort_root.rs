//! probe 3: the frozen lead spot — LOS probes + the run script's root translation per frame
use nf_view::scene::Scene;

#[test]
fn probe_root_block() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let mut sc: Scene = match nf_view::load(&iso, "07000001.bin", false) { Ok(s) => s, Err(e) => panic!("{e}") };
    let sim = sc.sim.as_mut().unwrap();
    // The frozen spot + the creep target from probe 2.
    let p = [3.48f32, 6.34, 7.28];
    let q = [4.8f32, 5.9, 5.9];
    let step_dir = [q[0] - p[0], q[2] - p[2]];
    let l = step_dir[0].hypot(step_dir[1]);
    let to = [p[0] + step_dir[0] / l * 1.5, p[1], p[2] + step_dir[1] / l * 1.5];
    for y in [5.34f32, 5.74, 6.04, 6.34, 6.84, 7.34] {
        eprintln!("[root] LOS y {y:.2}: ({:.2},{y:.2},{:.2}) -> ({:.2},{y:.2},{:.2}): {:?}",
            p[0], p[2], to[0], to[2], sim.world.line_of_sight([p[0], y, p[2]], [to[0], y, to[2]]));
    }
    // The floor below the spot and the target.
    eprintln!("[root] floor at p: {:?}, at q: {:?}", sim.world.floor_below(p, 3.0), sim.world.floor_below(q, 3.0));
    // The run script's root per frame (danim 220 script 0x06002056).
    // A probe must not fail the suite: skip cleanly when its input is absent (the slim ISO's
    // 07000001 does not carry script 0x06002056 — probe_escort_root.rs:22 panicked here before).
    let Some(script) = sim.scripts.get(&0x0600_2056) else {
        eprintln!("[root] script 0x06002056 absent from this scene — skipping the root probe");
        return;
    };
    eprintln!("[root] script 0x06002056 length {}", script.length);
    let mut prev: Option<[f32;3]> = None;
    for f in 1..=40u32 {
        if let Some((seqh, frame)) = nf_game::anim::script_frame(script, f as f32) {
            if let Some(seq) = sim.seqs.get(&seqh) {
                if seq.channels.len() >= 3 {
                    let r = nf_game::anim::root_translation_at(seq, frame);
                    let d = prev.map(|p| ((r[0]-p[0]).powi(2) + (r[2]-p[2]).powi(2)).sqrt()).unwrap_or(0.0);
                    eprintln!("[root] f{f}: seq 0x{seqh:08x} frame {frame} root ({:.3},{:.3},{:.3}) step2d {:.4}", r[0], r[1], r[2], d);
                    if f % 8 == 0 { prev = None; } else { prev = Some(r); }
                } else { eprintln!("[root] f{f}: seq 0x{seqh:08x} channels {}", seq.channels.len()); }
            }
        }
    }
}
