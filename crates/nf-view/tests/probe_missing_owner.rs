//! fix-missing-objects probe 5: why `link_owner_room` returns None for the orphan generics —
//! every room candidate's gates (class, radius, bbox containment, scoped closest point). Opt-in NF_ISO.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn probe_owner_gates() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = match nf_view::load(&iso, "07000006.bin", false) { Ok(s) => s, Err(e) => { eprintln!("07000006: {e}"); return } };
    let g = &sc.cel_graph;
    let world = sc.sim.as_ref().map(|s| &s.world);
    let Some(world) = world else { eprintln!("no world"); return };
    // (node, name, centre, radius) — the orphans from probe 4
    let orphans: &[(u32, &str, [f32; 3], f32)] = &[
        (334, "OBJECT_Fountain", [27.5, 41.0, -7.0], 6.2),
        (333, "Object_Pondwater", [27.5, 38.6, -7.0], 4.6),
        (68, "OBJECT_GuardroomTable", [31.7, 40.5, -24.4], 1.4),
        (72, "Object_bed", [36.9, 40.4, -23.5], 2.4),
    ];
    let rooms: Vec<usize> = (0..g.cells.len()).filter(|&i| matches!(g.cells[i].flags & 0xffff, 0xc023 | 0xc047)).collect();
    eprintln!("06: {} rooms of {} cells", rooms.len(), g.cells.len());
    for (node, name, centre, radius) in orphans {
        eprintln!("\n--- node {node} \"{name}\" centre {centre:?} r {radius} ---");
        for &ri in &rooms {
            let r = &g.cells[ri];
            let radius_ok = *radius <= r.sphere.1;
            let bbox_ok = (0..3).all(|k| r.min[k] <= centre[k] && centre[k] <= r.max[k]);
            if !radius_ok && !bbox_ok { continue; }
            let d = world.closest_point_scoped(ri, *centre);
            eprintln!("  room {ri:<4} r {:.1} min {:?} max {:?} radius_ok {radius_ok} bbox_ok {bbox_ok} closest {d:?}",
                r.sphere.1, r.min.map(|v| (v * 10.0).round() / 10.0), r.max.map(|v| (v * 10.0).round() / 10.0));
        }
    }
    // Also: where does the guard-room furniture's centre fall relative to EVERY room bbox?
    eprintln!("\n--- rooms whose bbox contains (31.7, 40.5, -24.4) ---");
    for &ri in &rooms {
        let r = &g.cells[ri];
        if (0..3).all(|k| r.min[k] <= [31.7f32, 40.5, -24.4][k] && [31.7f32, 40.5, -24.4][k] <= r.max[k]) {
            eprintln!("  room {ri} r {:.1}", r.sphere.1);
        }
    }
}
