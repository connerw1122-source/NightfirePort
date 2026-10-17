//! G-052: 07000009's security room (security_room_new, placement 85 = graph cell 40) was not drawn
//! from inside it. Cause: `Geo::push_flags` dropped instances whose collision mesh has no
//! triangles, while `build_scoped`'s per-instance cel list counts every instance with collision —
//! so every later chunk owner shifted by one and the security room's floor/ceiling runs landed in
//! the bucket of graph cell 39 (placement 84, a room at x 18..25). `build_FindCel` 0x1c8cb0's
//! tie-break rays [M] then hit nothing in either candidate and the first one — the lobby, cell 30 —
//! won, and the portal walk from the lobby never adds the room. With the room's own geometry in
//! its own bucket, the down/up rays hit the room's floor/ceiling first and the eye cel is the room
//! [L 2026-10-17: PCSX2 at (−17.8, −368.4, 25.0) draws the room; the port with NF_NOCELCULL
//! matched it].
//!
//! Opt-in via NF_ISO.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn the_security_room_is_the_eye_cel_at_the_elevator_panel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000009.bin", false).expect("load 07000009");
    let sim = scene.sim.as_ref().unwrap();
    let g = &sim.cel_graph;
    let room = g.own_cells_main[85].expect("placement 85 creates the security-room cel") as usize;
    let lobby = g.own_cells_main[46].expect("placement 46 creates the lobby cel") as usize;
    for y in [-368.4f32, -367.6, -366.8, -366.0] {
        let eye = [-17.8f32, y, 25.0];
        let c = nf_view::scene::find_cell_raycast(g, eye, Some(&sim.world));
        assert_eq!(c, Some(room), "eye {eye:?}: the room the eye stands in, not the lobby (cell {lobby}) [M 0x1c8cb0]");
    }
    // The room's floor/ceiling runs sit in the room's own scope bucket (none in the far room 39).
    let owners = sim.world.debug_bucket_owners([-18.0, -380.0, 24.8], [-17.6, -350.0, 25.2]);
    assert!(owners.iter().any(|&(c, _)| c == room as i32), "the room bucket holds its floor/ceiling: {owners:?}");
    let far = g.own_cells_main[84].unwrap() as i32;
    assert!(!owners.iter().any(|&(c, _)| c == far), "nothing at the panel column belongs to the x 18..25 room: {owners:?}");
}

/// Sister: the lobby floor outside the security room still finds the lobby.
#[test]
fn the_lobby_floor_still_finds_the_lobby() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000009.bin", false).expect("load 07000009");
    let sim = scene.sim.as_ref().unwrap();
    let g = &sim.cel_graph;
    let lobby = g.own_cells_main[46].unwrap() as usize;
    let c = nf_view::scene::find_cell_raycast(g, [-3.9, -367.0, 14.9], Some(&sim.world));
    assert_eq!(c, Some(lobby), "the lift mouth stands in the lobby");
}
