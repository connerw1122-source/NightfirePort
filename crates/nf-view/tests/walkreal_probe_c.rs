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
    // 07000015: the spine gap at z 28-44 (the upper deck y 41.5).
    floor_grid("07000015", 24.0, 70.0, 24.0, 44.0, 41.5, 2.0, 8.0);
    // 0700004a: the top lobby (the spawn y −7.7) — the boarding wedge and the walk-off east.
    floor_grid("0700004a", -8.0, 12.0, -10.0, 6.0, -7.5, 1.0, 8.0);
    // 0700004a: the bottom lobby (the ride's end y −157) — the fall to −2761 crossed it.
    floor_grid("0700004a", -8.0, 12.0, -10.0, 6.0, -156.5, 1.0, 8.0);
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

#[test]
fn probe_4a_spawn_support() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700004a.bin", false).expect("load");
    for t in 0..300 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if t % 30 == 0 {
            let sim = scene.sim.as_ref().unwrap();
            eprintln!("t{t}: pos=({:.2},{:.2},{:.2}) grounded={} substate={}", sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2], sim.bond.grounded, sim.bond.substate);
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    eprintln!("final: y {:.2} (start −7.71)", sim.bond.pos[1]);
}

#[test]
fn probe_4a_ride_scrape() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700004a.bin", false).expect("load");
    for t in 0..2400 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        scene.sim.as_mut().unwrap().drones.health.health = 100.0;
        if (1350..=2400).contains(&t) && t % 20 == 0 {
            let sim = scene.sim.as_ref().unwrap();
            let p = sim.bond.pos;
            let doors: Vec<String> = sim.drones.doors.list.iter()
                .filter(|d| (110..=111).contains(&d.placement))
                .map(|d| format!("[{}]({:.1},{:.1},{:.1}) st{:#x} p01{:.2}", d.placement, d.pos[0], d.pos[1], d.pos[2], d.state, d.pos01))
                .collect();
            let plat = sim.drones.scripts.objects.list.iter().enumerate()
                .filter_map(|(i,o)| o.as_ref().map(|o| (i, match o { nf_game::nis::Obj::Entity { matrix, .. } => Some(matrix[3][1]), _ => None })))
                .collect::<Vec<_>>();
            if t == 1660 {
                let b = &sim.bond;
                let (a, c) = b.capsule_seg();
                let (push, walkable) = sim.world.capsule_push(a, c, 0.4);
                eprintln!("FROZEN probe: capsule a=({:.2},{:.2},{:.2}) c=({:.2},{:.2},{:.2}) push=({:.3},{:.3},{:.3}) walkable={}",
                    a[0], a[1], a[2], c[0], c[1], c[2], push[0], push[1], push[2], walkable);
                eprintln!("floor_below 3: {:?}  floor_below 0.5: {:?}", sim.world.floor_below([p[0], p[1], p[2]], 3.0), sim.world.floor_below([p[0], p[1], p[2]], 0.5));
                eprintln!("segment down 3: {:?}", sim.world.segment([p[0], p[1], p[2]], [p[0], p[1] - 3.0, p[2]]));
                eprintln!("segment up 3: {:?}", sim.world.segment([p[0], p[1] + 0.5, p[2]], [p[0], p[1] + 3.0, p[2]]));
            }
            eprintln!("t{t}: bond=({:.2},{:.2},{:.2}) grounded={} plat={:?} doors {}", p[0], p[1], p[2], sim.bond.grounded, plat, doors.join(" "));
        }
        if sim_y(scene.sim.as_ref().unwrap()) < -200.0 { eprintln!("fell past −200 at t{t}"); break; }
    }
}

fn sim_y(sim: &nf_view::scene::Sim) -> f32 { sim.bond.pos[1] }

#[test]
fn probe_16_door7_fields() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000016.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    for (i, d) in sim.drones.doors.list.iter().enumerate() {
        eprintln!("door[{i}] plc={} kind={} flags={:#x} approach={} locked_flag={} straddle={} ch_lock={:#x} state={:#x} sph=({:.2},{:.2},{:.2}) r{:.2}",
            d.placement, d.kind, d.flags, d.approach, d.locked_flag, d.straddle, d.ch_lock, d.state,
            d.sph.0[0], d.sph.0[1], d.sph.0[2], d.sph.1);
    }
}
