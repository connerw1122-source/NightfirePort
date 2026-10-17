//! Scratch probe (not committed): find a standing spot in 07000008 with line of sight to a
//! Glass_Controlroom pane (the breakable's bounds beyond the first world surface — i.e. the pane
//! plane is crossed before any world stop).

fn main() {
    let Some(iso_path) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    let mut scene = nf_view::load(&iso_path, "07000008.bin", false).expect("load 08");
    let sim = scene.sim.as_mut().unwrap();
    let panes: Vec<([f32; 3], [f32; 3], usize)> = sim.drones.breaks.list.iter()
        .map(|b| (b.lo, b.hi, b.placement)).collect();
    println!("panes: {panes:?}");
    for (lo, hi, plc) in &panes {
        let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
        // Candidate stand spots: offset from the pane centre along the pane's normal(s), a few
        // distances, dropped onto the floor by stand_at.
        for off in [2.0f32, 3.5, 5.0, 7.0] {
            for axis in 0..3 {
                if (hi[axis] - lo[axis]).abs() > 0.5 { continue } // the pane's plane axis
                let mut sign = 1.0;
                for _ in 0..2 {
                    let mut p = c; p[axis] += sign * off;
                    let mut bond = nf_game::player::Bond::stand_at(&sim.world, p, 0.0);
                    bond.yaw = 0.0; bond.pitch = 0.0;
                    let eye = [bond.pos[0], bond.pos[1] + bond.eye - bond.crouch_drop, bond.pos[2]];
                    let d = [c[0] - eye[0], c[1] - eye[1], c[2] - eye[2]];
                    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    if len < 0.5 { sign = -sign; continue }
                    let end = [eye[0] + d[0] / len * (len + 2.0), eye[1] + d[1] / len * (len + 2.0), eye[2] + d[2] / len * (len + 2.0)];
                    let t_world = sim.data().world.segment(eye, end);
                    let t_pane = len; // the ray aims at the pane centre
                    let clear = t_world.map_or(true, |t| t > t_pane - 0.3);
                    let inside = |q: [f32; 3], lo: [f32; 3], hi: [f32; 3]| (0..3).all(|k| q[k] >= lo[k] - 0.3 && q[k] <= hi[k] + 0.3);
                    // The crossing point of the ray with the pane plane:
                    let ax = if (hi[0] - lo[0]).abs() <= 0.5 { 0 } else if (hi[1] - lo[1]).abs() <= 0.5 { 1 } else { 2 };
                    let t_cross = (c[ax] - eye[ax]) / (end[ax] - eye[ax]);
                    let q: [f32; 3] = [eye[0] + (end[0] - eye[0]) * t_cross, eye[1] + (end[1] - eye[1]) * t_cross, eye[2] + (end[2] - eye[2]) * t_cross];
                    if clear && inside(q, *lo, *hi) {
                        println!("PLC {plc} stand {bond:?} eye {eye:?} axis{ax} off{off} s{sign:.0} t_world {t_world:?} t_pane {t_pane:.2} q {q:?}");
                    }
                    sign = -sign;
                }
            }
        }
    }
}
