//! fix-drone4 probe: the 07000007 guests' walk path, live through `nf_view::load` + `Sim::step_bond`.
//! Checks: start states, route assignment (AssignAIPath 4), the level's path kinds, walk-anim
//! availability (`can_do` 0x37/38/39), and whether any guest actually translates.
//! Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn probe_party_guest_walk() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();

    // The level's AI paths: kind + node count.
    let data = sim.data();
    eprintln!("--- 07000007 nav: {} paths ---", data.nav.paths.len());
    for (pi, p) in data.nav.paths.iter().enumerate() {
        eprintln!("[path {pi:02}] kind {:#x} nodes {} first ({:.1},{:.1},{:.1})",
            p.kind, p.nodes.len(), p.nodes[0].pos[0], p.nodes[0].pos[1], p.nodes[0].pos[2]);
    }
    drop(data);

    let guests: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.dtype == 10)
        .map(|(i, _)| i).collect();
    eprintln!("--- {} dtype-10 guests ---", guests.len());
    for &g in &guests {
        let dr = &sim.drones.list[g];
        eprintln!("[g{g:02}] skin {:09x} state {:#x} mode {} team {} female {} aset {} route {} pos ({:.1},{:.1},{:.1}) enabled {} drawn {} started {}",
            dr.skin, dr.state, dr.mode, dr.team, dr.female as u8, dr.anim_set,
            dr.route.is_some() as u8, dr.pos[0], dr.pos[1], dr.pos[2], dr.enabled, dr.drawn, dr.started);
    }

    // Nearest kind-4 path/node for the first 5 guests (the port's AssignAIPath deviation: all paths,
    // height ±2.0, 2D < 0.3).
    let data = sim.data();
    for &g in guests.iter().take(5) {
        let dr = &sim.drones.list[g];
        let mut best: Option<(f32, usize, usize)> = None;
        for (pi, p) in data.nav.paths.iter().enumerate().filter(|(_, p)| p.kind & 4 != 0) {
            for (ni, n) in p.nodes.iter().enumerate() {
                if (n.pos[1] - dr.feet()[1]).abs() > 2.0 { continue }
                let dx = dr.pos[0] - n.pos[0]; let dz = dr.pos[2] - n.pos[2];
                let dist = (dx * dx + dz * dz).sqrt();
                if dist < best.map_or(0.3, |b: (f32, usize, usize)| b.0) { best = Some((dist, pi, ni)); }
            }
        }
        eprintln!("[g{g:02}] nearest kind-4 node within 0.3: {:?}", best);
        let mut any: Option<(f32, usize, usize, u32)> = None;
        for (pi, p) in data.nav.paths.iter().enumerate() {
            for (ni, n) in p.nodes.iter().enumerate() {
                let dx = dr.pos[0] - n.pos[0]; let dz = dr.pos[2] - n.pos[2];
                let dist = (dx * dx + dz * dz).sqrt();
                if dist < any.map_or(f32::MAX, |b: (f32, usize, usize, u32)| b.0) { any = Some((dist, pi, ni, p.kind)); }
            }
        }
        eprintln!("[g{g:02}] nearest node of any kind: {:?}", any);
        eprintln!("[g{g:02}] can_do walk 0x37 {} 0x38 {} 0x39 {} stand 0x24 {}",
            dr.can_do(&data, 0x37) as u8, dr.can_do(&data, 0x38) as u8,
            dr.can_do(&data, 0x39) as u8, dr.can_do(&data, 0x24) as u8);
    }
    drop(data);

    // All kind-8 paths' nodes:
    for (pi, p) in sim.data().nav.paths.iter().enumerate().filter(|(_, p)| p.kind & 8 != 0) {
        let ns: Vec<String> = p.nodes.iter().map(|n| format!("({:.1},{:.1})", n.pos[0], n.pos[2])).collect();
        eprintln!("[path {pi}] {}", ns.join(" "));
    }
    for &g in guests.iter() {
        let dr = &sim.drones.list[g];
        let mut best: Option<(f32, usize, usize)> = None;
        for (pi, p) in sim.data().nav.paths.iter().enumerate().filter(|(_, p)| p.kind & 8 != 0) {
            for (ni, n) in p.nodes.iter().enumerate() {
                if (n.pos[1] - dr.feet()[1]).abs() > 2.0 { continue }
                let dx = dr.pos[0] - n.pos[0]; let dz = dr.pos[2] - n.pos[2];
                let dist = (dx * dx + dz * dz).sqrt();
                if dist < best.map_or(0.3, |b: (f32, usize, usize)| b.0) { best = Some((dist, pi, ni)); }
            }
        }
        if best.is_some() { eprintln!("[g{g:02}] kind-8 within 0.3: {:?}", best); }
    }

    // Tick 300 idle, then measure movement (trace the first guest with a route).
    let start: Vec<[f32; 3]> = guests.iter().map(|&g| sim.drones.list[g].pos).collect();
    for t in 0..300 {
        sim.step_bond(Default::default(), false, false, false);
        if t % 20 == 0 {
            let gi = sim.drones.list.iter().position(|dr| dr.present && dr.route.is_some());
            if let Some(gi) = gi {
                let dr = &sim.drones.list[gi];
                let code = dr.route.as_ref().map(|r| (r.index, r.creep.step, r.creep.steps, r.target));
                eprintln!("[t{t}] g{gi} state {:#x} anim.state {:#x} anim.call {:?} route {:?} pos ({:.2},{:.2})",
                    dr.state, dr.anim.state, dr.anim.call, code, dr.pos[0], dr.pos[2]);
            }
        }
    }
    eprintln!("--- after 300 idle ticks ---");
    let mut moved_any = false;
    for (k, &g) in guests.iter().enumerate() {
        let dr = &sim.drones.list[g];
        let d0 = start[k];
        let moved = ((dr.pos[0] - d0[0]).powi(2) + (dr.pos[2] - d0[2]).powi(2)).sqrt();
        if moved > 0.2 { moved_any = true; }
        eprintln!("[g{g:02}] state {:#x} route {} moved {:.2} anim {:#x} pos ({:.1},{:.1},{:.1})",
            dr.state, dr.route.is_some() as u8, moved, dr.anim.state, dr.pos[0], dr.pos[1], dr.pos[2]);
    }
    assert!(moved_any, "at least one guest walks within 300 ticks (user: 'they don't move or walk at all')");
}

#[test]
fn probe_guest_props_and_channels() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    let data = sim.data();
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if !dr.present || dr.dtype != 10 { continue }
        eprintln!("[g{i:02}] skin {:09x} prop1d {} ch {} started {} state {:#x} route8 {:?}",
            dr.skin, dr.prop(&data, 0x1d) as u8, dr.switch_channel, dr.started, dr.state,
            dr.nav_path.map(|p| p.map(|q| q as i64)).unwrap_or(Some(-1)));
    }
}
