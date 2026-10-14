//! Throwaway probe (fix-swim-entry). READ-ONLY: loads 07000002 through the viewer path and walks the
//! garden-pond bank exactly like `fix_ladder2_live::swim_repro_walks_on_the_water_from_the_bank`, while
//! comparing three cel picks per tick:
//!   (a) the port's current water lookup: smallest-bbox `find_cel(cels, pos)` over the room cels;
//!   (b) the game's stateful relink (`control_handle_cel_change` 0x133288): the move segment's portal
//!       cross (`Cel_ObjectLeftCel` 0x1e0d58) → the bbox keep (`build_point_in_bounding_box` 0x1c9500)
//!       → `build_LinkToRoom` 0x1c9680 → `build_FindCel` 0x1c8cb0 at the world-sphere centre (the
//!       raycast tie-break, `find_cell_raycast` over the 0x40000 room cells);
//!   (c) the same FindCel at POS (what the port's icon path uses today).
//! Prints the water cels, the portals touching them, and the per-tick verdicts + the entry ticks.
//!
//! Run: NF_ISO=<iso> cargo run --offline --release -p nf-view --example probe_swim_entry

use nf_view::scene::{find_cel, find_cell_raycast, Cel};

fn main() { column_census();
    let iso = std::env::var("NF_ISO").unwrap_or_default();
    if iso.is_empty() { eprintln!("NF_ISO not set"); std::process::exit(2); }
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let mut sim = scene.sim.take().expect("sim");

    // ---- the water room cels ----
    println!("== room cels with water ==");
    let mut water_rooms: Vec<(usize, f32)> = Vec::new();
    for (i, c) in sim.cels.iter().enumerate() {
        if c.water > -400.0 { water_rooms.push((i, c.water)); }
    }
    let cell_room: Vec<Option<usize>> = {
        let mut v = vec![None; sim.cel_graph.cells.len()];
        for (r, cell) in sim.room_cell.iter().enumerate() { if let Some(c) = cell { v[*c] = Some(r); } }
        v
    };
    for &(i, w) in &water_rooms {
        let c = &sim.cels[i];
        let cell = sim.room_cell[i];
        println!("room cel {i}: water {w:.4} min {:?} max {:?} → graph cell {:?} flags {:?}",
            c.min, c.max, cell, cell.map(|k| sim.cel_graph.cells[k].flags));
    }

    // ---- the garden pond cel (the repro test's pick) ----
    let (wi, _) = water_rooms.iter().find(|&&(i, w)| w > 3.76 && w < 3.775 && sim.cels[i].max[0] < -54.0).copied()
        .expect("the garden water cel");
    let wc = sim.room_cell[wi].expect("the garden water cel has a graph cell");
    let c: &Cel = &sim.cels[wi];
    let (cx, cz) = (0.5 * (c.min[0] + c.max[0]), 0.5 * (c.min[2] + c.max[2]));
    println!("\n== portals touching the garden water cell {wc} ==");
    for (pi, p) in sim.cel_graph.portals.iter().enumerate() {
        if p.cel_a != wc && p.cel_b != wc { continue; }
        let other = if p.cel_a == wc { p.cel_b } else { p.cel_a };
        let ys: Vec<f32> = p.corners.iter().map(|k| k[1]).collect();
        let xs = [p.corners[0][0], p.corners[2][0]];
        let zs = [p.corners[0][2], p.corners[2][2]];
        let oflags = sim.cel_graph.cells.get(other).map(|n| n.flags);
        let oroom = sim.cel_graph.cells.get(other).and_then(|_| cell_room[other]);
        println!("portal {pi}: {wc} ↔ {other} (other flags {oflags:?}, other room {oroom:?}) corner y {ys:?} x {xs:?} z {zs:?}");
    }
    println!("\n== the water cell's own bbox vs the room cel bbox ==");
    let cn = &sim.cel_graph.cells[wc];
    println!("graph cell {wc}: min {:?} max {:?} (room cel: min {:?} max {:?})", cn.min, cn.max, c.min, c.max);

    // ---- the walk ----
    let bx = c.min[0] - 2.0;
    let floor = sim.world.floor_below([bx, 8.0, cz], 12.0).expect("the west bank floor");
    println!("\n== walk: start ({bx:.2}, {floor:.2}, {cz:.2}) floor {floor:.4}, yaw π/2, forward ==",);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [bx, floor + 1.1, cz], 0.0);
    sim.bond.yaw = std::f32::consts::FRAC_PI_2;
    let mut prev = sim.bond.pos;
    let mut cur: Option<usize> = None;
    let mut game_entry: Option<(usize, [f32; 3])> = None;
    let mut port_entry: Option<(usize, [f32; 3])> = None;
    let fwd = nf_game::player::Input { forward: 1.0, ..Default::default() };
    for t in 0..2400 {
        sim.step_bond(fwd, false, false, false);
        let pos = sim.bond.pos;
        let centre = sim.bond.sphere_centre();

        // (a) the port's pick
        let port_room = find_cel(&sim.cels, pos);
        let port_water = port_room.map(|r| sim.cels[r].water).filter(|w| *w > -400.0);

        // (b) the game's relink
        let before_cell = cur;
        let (newcur, reason) = relink(cur, prev, pos, centre, &sim);
        cur = newcur;
        let event = if cur != before_cell { format!("RELINK {before_cell:?}->{cur:?} via {reason}") } else { reason.to_string() };
        let cur_water = cur.and_then(|k| cell_room[k]).map(|r| sim.cels[r].water).filter(|w| *w > -400.0);

        let port_in = port_water.map_or(false, |w| pos[1] - 0.3 < w);
        let game_in = cur_water.map_or(false, |w| pos[1] - 0.3 < w);
        if port_in && port_entry.is_none() { port_entry = Some((t, pos)); }
        if game_in && game_entry.is_none() { game_entry = Some((t, pos)); }

        // log the interesting ticks
        let sub = sim.bond.substate;
        if (1590..=1630).contains(&t) || t % 10 == 0 || (!event.is_empty() && event != "keep") || (port_in != game_in) {
            println!("t {t:4} sub {sub:2} pos [{:8.3} {:7.3} {:8.3}] centre.y {:7.3} | port room {:?} w {:?} in {port_in} | game cell {:?} w {:?} in {game_in} {}",
                pos[0], pos[1], pos[2], centre[1], port_room, port_water, cur, cur_water, event);
        }
        if sim.bond.substate == nf_game::player::SS_SWIM {
            println!(">>> the port's substate 3 at t {t} pos.y {:.3}", pos[1]);
            break;
        }
        prev = pos;
    }
    println!("\nport entry (rule a): {:?}\ngame entry (rule b): {:?}\nsubstate entry at the last logged tick",
        port_entry.map(|(t, p)| (t, p[1])), game_entry.map(|(t, p)| (t, p[1])));
}

/// The game's per-tick relink (control_handle_cel_change 0x133288): portal cross → bbox keep → FindCel.
fn relink(cur: Option<usize>, prev: [f32; 3], pos: [f32; 3], centre: [f32; 3], sim: &nf_view::scene::Sim) -> (Option<usize>, &'static str) {
    if let Some(cell) = cur {
        // Cel_ObjectLeftCel 0x1e0d58: the move segment against the current cel's portals.
        let dir = [pos[0] - prev[0], pos[1] - prev[1], pos[2] - prev[2]];
        let hit = |tri: [[f32; 3]; 3]| -> bool {
            let e1 = [tri[1][0] - tri[0][0], tri[1][1] - tri[0][1], tri[1][2] - tri[0][2]];
            let e2 = [tri[2][0] - tri[0][0], tri[2][1] - tri[0][1], tri[2][2] - tri[0][2]];
            let pv = [dir[1] * e2[2] - dir[2] * e2[1], dir[2] * e2[0] - dir[0] * e2[2], dir[0] * e2[1] - dir[1] * e2[0]];
            let det = e1[0] * pv[0] + e1[1] * pv[1] + e1[2] * pv[2];
            if det.abs() < 1e-12 { return false; }
            let inv = 1.0 / det;
            let tv = [prev[0] - tri[0][0], prev[1] - tri[0][1], prev[2] - tri[0][2]];
            let u = (tv[0] * pv[0] + tv[1] * pv[1] + tv[2] * pv[2]) * inv;
            if !(-1e-9..=1.0 + 1e-9).contains(&u) { return false; }
            let qv = [tv[1] * e1[2] - tv[2] * e1[1], tv[2] * e1[0] - tv[0] * e1[2], tv[0] * e1[1] - tv[1] * e1[0]];
            let v = (dir[0] * qv[0] + dir[1] * qv[1] + dir[2] * qv[2]) * inv;
            if !(-1e-9..=1.0 + 1e-9).contains(&v) || u + v > 1.0 + 2e-9 { return false; }
            let t = (e2[0] * qv[0] + e2[1] * qv[1] + e2[2] * qv[2]) * inv;
            (0.0..=1.0).contains(&t)
        };
        for p in &sim.cel_graph.portals {
            let dest = if p.cel_a == cell { p.cel_b } else if p.cel_b == cell { p.cel_a } else { continue };
            let c = &p.corners;
            if hit([c[0], c[1], c[2]]) || hit([c[2], c[3], c[0]]) { return (Some(dest), "portal"); }
        }
        // build_point_in_bounding_box 0x1c9500: the current cel's bbox keeps.
        let n = &sim.cel_graph.cells[cell];
        if (0..3).all(|k| n.min[k] <= pos[k] && pos[k] <= n.max[k]) { return (Some(cell), "keep"); }
    }
    // build_LinkToRoom 0x1c9680 → build_FindCel 0x1c8cb0 at the world-sphere centre.
    (find_cell_raycast(&sim.cel_graph, centre, Some(&sim.world)), "findcel")
}

// ---- appended: the static column census along the walk line (z = cz) ----
#[allow(dead_code)]
fn column_census() {
    let iso = std::env::var("NF_ISO").unwrap_or_default();
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load");
    let sim = scene.sim.take().expect("sim");
    let wi = (0..sim.cels.len()).find(|&i| { let c = &sim.cels[i]; c.water > 3.76 && c.water < 3.775 && c.max[0] < -54.0 }).expect("pond");
    let c = &sim.cels[wi];
    let cz = 0.5 * (c.min[2] + c.max[2]);
    println!("== column census at z {cz:.2}: water cel {} bbox {:?}..{:?} ==", wi, c.min, c.max);
    for x in [-82.0f32, -81.0, -80.5, -80.0, -79.5, -79.0, -78.5, -78.0, -77.0, -76.0, -75.0] {
        // every surface down the column (banks + beds)
        let mut hits = String::new();
        let mut y_from = 8.0f32;
        for _ in 0..10 {
            let Some((t, n, m)) = sim.world.segment_surface([x, y_from, cz], [x, -3.0, cz], false) else { break };
            let y = y_from + (-3.0 - y_from) * t;
            hits.push_str(&format!(" [y={y:.3} mat={m} ny={:.2}]", n[1]));
            if y <= -3.0 { break; }
            y_from = y - 0.05;
        }
        // the cel sets at two standing heights
        let desc = |p: [f32; 3]| -> String {
            let cands: Vec<usize> = (0..sim.cel_graph.cells.len())
                .filter(|&i| sim.cel_graph.cells[i].flags & 0x40000 != 0)
                .filter(|&i| (0..3).all(|k| sim.cel_graph.cells[i].min[k] <= p[k] && p[k] <= sim.cel_graph.cells[i].max[k]))
                .collect();
            let vols: Vec<(usize, f32)> = cands.iter().map(|&i| { let n = &sim.cel_graph.cells[i]; (i, (n.max[0]-n.min[0])*(n.max[1]-n.min[1])*(n.max[2]-n.min[2])) }).collect();
            format!("cands {vols:?}")
        };
        let p1 = [x, 4.3, cz];
        let p2 = [x, 3.9, cz];
        let p3 = [x, 3.75, cz];
        let p4 = [x, 3.6, cz];
        println!("x {x:7.1}:{hits}");
        println!("        y4.3 {}", desc(p1));
        println!("        y3.9 {}", desc(p2));
        println!("        y3.75 {}", desc(p3));
        println!("        y3.6 {}", desc(p4));
    }
    // cel 91's bbox (the bank/air cel the walk started in) and every cell with a portal to 184
    println!("\n== cells around the pond ==");
    for (i, n) in sim.cel_graph.cells.iter().enumerate() {
        if n.flags & 0x40000 == 0 { continue; }
        // rooms overlapping the pond bbox in x/z
        let overlap = n.min[0] < c.max[0] && n.max[0] > c.min[0] && n.min[2] < c.max[2] && n.max[2] > c.min[2];
        if overlap { println!("cell {i} (room {:?}) min {:?} max {:?} vol {:.1}",
            sim.room_cell.iter().position(|&r| r == Some(i)), n.min, n.max,
            (n.max[0]-n.min[0])*(n.max[1]-n.min[1])*(n.max[2]-n.min[2])); }
    }
}
