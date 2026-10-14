//! fix-exact-math-swap3: the frame-170 NIS camera dump. Prints the live `Obj::Camera` matrix bits,
//! the (pos, yaw, pitch, fov) the renderer's `script_camera()` derives from it, and one actor
//! (drone 0) matrix bits, on 07000002 (intro cut-scene) and 07000007 (--skip-cutscenes).
//! Bit-exact reference for the master-vs-branch element diff. Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn bits(m: &[[f32; 4]; 4]) -> String {
    m.iter().map(|r| r.iter().map(|v| format!("{:08x}", v.to_bits())).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join(" | ")
}

fn dump_camera(tag: &str, sim: &nf_view::scene::Sim) {
    let (m, fov) = match sim.drones.scripts.camera() {
        Some(x) => x,
        None => { eprintln!("[{tag}] NO CAMERA OBJECT at tick {}", sim.drones.frame); return }
    };
    // The renderer's exact derivation (scene.rs script_camera).
    let f = m[2];
    let l = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt().max(1e-6);
    let yaw = f[0].atan2(f[2]);
    let pitch = (f[1] / l).clamp(-1.0, 1.0).asin();
    eprintln!("[{tag}] tick {} camera fov {}", sim.drones.frame, fov);
    eprintln!("[{tag}] camera matrix {}", bits(&m));
    eprintln!("[{tag}] derived pos ({:?}) yaw {yaw:.9} ({:08x}) pitch {pitch:.9} ({:08x})", m[3], yaw.to_bits(), pitch.to_bits());
}

fn dump_drone0(tag: &str, sim: &nf_view::scene::Sim) {
    let Some(dr) = sim.drones.list.iter().filter(|d| d.present).max_by_key(|d| d.skin) else { return };
    eprintln!("[{tag}] drone0 skin {:09x} pos ({:.4},{:.4},{:.4}) yaw {:.6} state {:#x} drawn {}",
        dr.skin, dr.pos[0], dr.pos[1], dr.pos[2], dr.yaw, dr.state, dr.drawn);
}

#[test]
fn dump_07000002_frame_170() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_mut().unwrap();
    // The intro cut-scene runs from level start; step to each sampled frame with idle inputs
    // (the viewer's --frame path steps `sim.step` without player input; the camera is script-driven).
    for t in 1..=200 {
        sim.step([0.0; 3], [0.0, 0.0, 1.0], false, false);
        if t == 140 || t == 170 || t == 200 {
            dump_camera(&format!("07000002 f{t}"), sim);
            dump_drone0(&format!("07000002 f{t}"), sim);
        }
    }
}

#[test]
fn dump_07000007_skip_frame_170() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    let sim = scene.sim.as_mut().unwrap();
    // --skip-cutscenes: the viewer holds the game's own skip while a script camera runs.
    for t in 1..=170 {
        if sim.disabled { sim.drones.scripts.skip_request = true; }
        sim.step([0.0; 3], [0.0, 0.0, 1.0], false, false);
        if t == 140 || t == 170 { dump_camera(&format!("07000007 f{t}"), sim); }
    }
    // The hall guard: the --bond spot looks at him from (42.59, 44.10, -71.71) yaw -1.228.
    dump_drone0("07000007 f170", sim);
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if dr.present && (dr.pos[0] - 42.59).abs() < 30.0 && (dr.pos[2] + 71.71).abs() < 30.0 {
            eprintln!("[07000007 f170] guard d{i:02} skin {:09x} pos ({:.4},{:.4},{:.4}) yaw {:.6} drawn {}",
                dr.skin, dr.pos[0], dr.pos[1], dr.pos[2], dr.yaw, dr.drawn);
        }
    }
}
