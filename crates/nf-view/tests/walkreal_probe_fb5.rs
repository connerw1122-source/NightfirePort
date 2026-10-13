//! fix-final-blockers probe 6: the 0700004a ride's t1550..1680 window — Bond's pose, the floor
//! ray, the capsule push, and WHICH dynamic mover `ground_motion` reports (id 0 = the platform
//! entity, 0x100_0000_0000|k = door k). Diagnostic only — run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

use nf_game::player::Input;
use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn ride_and_dump_the_window() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load(&iso, "0700004a.bin", false).unwrap());
    for k in 0..84 {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        let target = if k < 42 { [3.2, 0.0, -2.2] } else { [3.2, 0.0, -3.8] };
        let desired = (target[0] - p[0]).atan2(target[2] - p[2]);
        let err = walker::wrap_pi(desired - sim.bond.yaw);
        w.tick(nf_game::player::Input { turn: err.clamp(-0.08, 0.08), forward: 1.0, ..Default::default() }, false);
    }
    let y_board = w.scene.sim.as_ref().unwrap().bond.pos[1];
    eprintln!("boarded at y {y_board}");
    for k in 0..5400u32 {
        w.tick(Default::default(), false);
        let t = k + 84;
        if t < 1550 || t > 1690 { continue; }
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        let feet_ray = sim.world.segment_hit([p[0], p[1], p[2]], [p[0], p[1] - 1.6347, p[2]]);
        let gm = sim.world.ground_motion(p, [p[0], p[1] - 1.6347, p[2]]);
        let gm_s = gm.map(|g| format!("delta ({:.3},{:.3},{:.3}) pivot ({:.2},{:.2},{:.2}) turn {:.4}",
            g.delta[0], g.delta[1], g.delta[2], g.pivot[0], g.pivot[1], g.pivot[2], g.turn));
        // Which doors are within 4 of the platform y (their states)?
        let mut near = String::new();
        for (i, d) in sim.drones.doors.list.iter().enumerate() {
            if (d.pos[1] - p[1]).abs() < 5.0 && (d.pos[2].abs() < 0.5) {
                near.push_str(&format!(" d{i}@y{:.1}s{:#x}", d.pos[1], d.state));
            }
        }
        eprintln!("t{t} bond ({:.2},{:.2},{:.2}) grounded {} feet_ray {feet_ray:?} gm {gm_s:?}{near}",
            p[0], p[1], p[2], sim.bond.grounded);
        if t == 1560 || t == 1610 || t == 1620 { sim.drones.doors.log_doors(None); }
        if t == 1615 {
            // The world bbox of every nearby dynamic part: the platform (script entity 0's parts)
            // and the doors 46..57, transformed from their shapes by their live matrices.
            for (i, d) in sim.drones.doors.list.iter().enumerate() {
                if !(46..=57).contains(&i) { continue; }
                if let Some(shape) = sim.world.shape(d.glist) {
                    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                    for tri in shape { for v in tri {
                        let w = [
                            d.matrix[0][0] * v[0] + d.matrix[1][0] * v[1] + d.matrix[2][0] * v[2] + d.matrix[3][0],
                            d.matrix[0][1] * v[0] + d.matrix[1][1] * v[1] + d.matrix[2][1] * v[2] + d.matrix[3][1],
                            d.matrix[0][2] * v[0] + d.matrix[1][2] * v[1] + d.matrix[2][2] * v[2] + d.matrix[3][2],
                        ];
                        for k in 0..3 { lo[k] = lo[k].min(w[k]); hi[k] = hi[k].max(w[k]); }
                    } }
                    eprintln!("door[{i}] y {:.1} state {:#x} world bbox lo {lo:.2?} hi {hi:.2?}", d.pos[1], d.state);
                }
            }
            // The platform entity's part bbox: read back the dynamic set via the world's public
            // per-object query is not exposed; instead the feet-ray and a deep ray locate it.
            let deep = sim.world.segment_hit([p[0], p[1], p[2]], [p[0], p[1] - 6.0, p[2]]);
            eprintln!("deep ray down 6.0 from bond: {deep:?}");
            let (a, b) = sim.bond.capsule_seg();
            let (push, walkable) = sim.world.capsule_push(a, b, 0.55);
            eprintln!("bond capsule a {a:.2?} b {b:.2?} push {push:.2?} walkable {walkable}");
        }
        if p[1] < y_board - 150.0 { eprintln!("FELL by t{t}"); break; }
    }
}
