//! The 07000002 bathhouse floor (generic placement 270) rides the bathhouse room's `+0x18` chain,
//! not the big room's. [L 2026-10-17, fresh-boot PCSX2 snapshot at Bond (−43.11, 6.46, 10.37)]:
//! the floor's generic cel 0x17086f0 (sphere (−38.19, 5.34, 8.00) r 5.27) hangs on room 0x17087b0
//! (sphere (−38.15, 7.24, 7.97) r 6.11), whose chain is exactly three nodes (the floor and two
//! small generics at (−39.07, 5.40, 9.96) r 0.38 / (−39.10, 5.39, 11.27) r 0.34). The owner pick
//! (`build_link_objects_to_rooms` 0x1c9150, `Collide_SphereIntersect` scoped to each candidate
//! room's own collision [M]) measured other rooms' geometry while the scope buckets were keyed by
//! push ordinal; the floor then hung on the big room and vanished through the doorway (user F12
//! 1017, 005-07000002). Opt-in via NF_ISO.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn near(a: [f32; 3], b: [f32; 3], t: f32) -> bool { (0..3).all(|k| (a[k] - b[k]).abs() < t) }

#[test]
fn the_bathhouse_floor_rides_the_bathhouse_room_chain() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let g = &sc.cel_graph;
    let room = g.cells.iter().position(|c| c.flags & 0x40000 != 0 && near(c.sphere.0, [-38.15, 7.24, 7.97], 0.1) && (c.sphere.1 - 6.11).abs() < 0.1)
        .expect("the bathhouse room (sphere (−38.15, 7.24, 7.97) r 6.11) exists");
    let floor = g.cells.iter().position(|c| near(c.sphere.0, [-38.19, 5.34, 8.00], 0.05) && (c.sphere.1 - 5.27).abs() < 0.05)
        .expect("the floor generic exists");
    assert!(g.chains[room].contains(&(floor as u32)), "the floor hangs on the bathhouse room's chain [L]");
    assert_eq!(g.chains[room].len(), 3, "the live chain is exactly three nodes [L]");
    // Sister: no other room carries it.
    let owners = g.chains.iter().filter(|ch| ch.contains(&(floor as u32))).count();
    assert_eq!(owners, 1, "exactly one owner");
}
