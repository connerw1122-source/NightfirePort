//! walkreal-c probe: dump the route-relevant data (start, triggers, doors, pickups, ladders)
//! for the four finale levels (07000015, 07000016, 0700001b, 0700004a). Diagnostic only —
//! run with --nocapture.

#[path = "common/walker.rs"]
mod walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// A floor map at walk height: '#' = floor within `depth` below the plane, '.' = lower, ' ' = void.
fn floor_grid(code: &str, x0: f32, x1: f32, z0: f32, z1: f32, y: f32, step: f32, depth: f32) {
    let iso = iso();
    let scene = nf_view::load(&iso, &format!("{code}.bin"), false).expect(code);
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("=== {code} floor grid at y {y} depth {depth} (rows z {z0}..{z1}, cols x {x0}..{x1}, step {step}) ===");
    let mut z = z1;
    while z >= z0 {
        let mut row = format!("z {:6.1}: ", z);
        let mut x = x0;
        while x <= x1 {
            let f = sim.world.floor_below([x, y, z], depth);
            match f {
                Some(fy) => {
                    let d = y - fy;
                    if d < 0.0 { row.push('^'); }          // floor above the plane
                    else if d < 2.5 { row.push('#'); }     // walkable
                    else { row.push('.'); }                // a hole
                }
                None => row.push(' '),                     // void below
            }
            x += step;
        }
        eprintln!("{row}");
        z -= step;
    }
}

#[test]
fn dump_floor_grids() {
    // 0700001b: the hub — the walk fell into a pit at y −15.4 west of the spawn.
    floor_grid("0700001b", -12.0, 26.0, -20.0, 8.0, -1.8, 2.0, 16.0);
    // 07000015: the spawn gantry and the z≈0 crossing holes (Bond fell at x 67.45).
    floor_grid("07000015", 60.0, 96.0, -14.0, 8.0, 41.5, 2.0, 8.0);
    // 07000016: the pit at z −88..−100 (the ladder at x −0.4) — the side ledges.
    floor_grid("07000016", -10.0, 14.0, -106.0, -82.0, 1.0, 2.0, 8.0);
}

#[test]
fn dump_levels() {
    let iso = iso();
    for code in ["07000015", "07000016", "0700001b", "0700004a"] {
        let scene = match nf_view::load(&iso, &format!("{code}.bin"), false) {
            Ok(s) => s,
            Err(e) => { eprintln!("=== {code}: LOAD FAIL {e}"); continue; }
        };
        let sim = scene.sim.as_ref().unwrap();
        eprintln!("=== {code} ===");
        let b = &sim.bond;
        eprintln!("start: bond=({:.2},{:.2},{:.2}) yaw={:.3} disabled={}", b.pos[0], b.pos[1], b.pos[2], b.yaw, sim.disabled);
        eprintln!("cels: {}", sim.cels.len());
        for t in &sim.drones.triggers {
            if t.tris.is_empty() && t.kind != walker::trigger_kind::LOAD_LEVEL && t.kind != walker::trigger_kind::AND
                && t.kind != walker::trigger_kind::OUT && t.kind != walker::trigger_kind::OR { continue; }
            let c = t.centre();
            let list = t.list.iter().map(|x| format!("{x:#x}")).collect::<Vec<_>>().join(",");
            eprintln!("trig kind={:2} out={:#04x} gate={} key={} fb={} next={:#x} mask={:#04x} lock={} at=({:.1},{:.1},{:.1}) list=[{}]",
                t.kind, t.out, t.gate, t.key_gate, t.fallback, t.next_level, t.mask, t.lock, c[0], c[1], c[2], list);
        }
        for (i, d) in sim.drones.doors.list.iter().enumerate() {
            eprintln!("door[{i}] at=({:.1},{:.1},{:.1}) kind={} ch_open={:#x} ch_lock={:#x} lock_flag={} approach={} state={:#x}",
                d.pos[0], d.pos[1], d.pos[2], d.kind, d.ch_open, d.ch_lock, d.locked_flag, d.approach, d.state);
        }
        for (c, top, bot) in walker::ladders(sim) {
            eprintln!("ladder at=({:.1},{:.1},{:.1}) top={top:.1} bottom={bot:.1}", c[0], c[1], c[2]);
        }
        for (i, d) in sim.drones.list.iter().enumerate() {
            if !d.present { continue; }
            eprintln!("drone[{i}] skin={:#x} dtype={:#x} pos=({:.1},{:.1},{:.1}) mode={} state={:#x} enabled={} switch_ch={:#x} death_ch={} corpse={} health={:.0}",
                d.skin, d.dtype, d.pos[0], d.pos[1], d.pos[2], d.mode, d.state, d.enabled, d.switch_channel, d.death_channel, d.corpse, d.health);
        }
    }
}
