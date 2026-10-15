//! fix-door-portal-links (GAPS G-035): doors open and close their portals (map block 0x21).
//!
//! `build_LinkDoors2Portals` (0x1c8ac8) links every portal-flagged door (`Door_Link2Portal`
//! 0x136788 = `obj+0xFD >> 3 & 1`, set by `Door_Create` when the door's placement param 4 != 0)
//! to the portal quad it sits in — plane within 0.25 of the door's gfx sphere centre
//! (`ABS(dot) < 0.25` @0x1c8c34) and the centre inside the quad (`vecutil_point_on_poly`
//! 0x1ec940). `Vision_Recurse` (0x1e82e0) then ends every portal whose linked door reads
//! `Door_IsClosed` (0x1367b0: kind 1 always; group → `DoorGroupsStates[g] == 2`; else
//! `obj+0xF4 == 2`) — a shut door's room is not walked into, an open one's is.
//!
//! Symptom (measured before the fix, `zz_probe` scratch run): from cell 7 the pre-fix walk
//! reached cell 40 through portal 3 while door 0 (placement 13, the 07000001 SlidingDoor)
//! stood CLOSED — "PRE-FIX SYMPTOM (G-035): cell 40 behind the SHUT door (placement 13, state 2)
//! is in the visible-cel set from cell 7". The level's portal data is block 0x21 [M
//! `parsemap_block_portal_data` 0x1d15f8; the parse `PortalQuad::parse_block` matches: 56-byte
//! records — 4 corners (3 floats), u16 celA, u16 celB, u16 flags at +0x34].
//!
//! Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near, far, fov) = (0.05f32, 4000.0f32, 1.0471976f32); // 60°, the render path's GAME_FOV
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
    ]);
    let cp = pitch.cos();
    let fw = Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
    let s = fw.cross(Vec3::Y).normalize();
    let u = s.cross(fw);
    let view = Mat4::from_cols_array_2d(&[
        [s.x, u.x, -fw.x, 0.0],
        [s.y, u.y, -fw.y, 0.0],
        [s.z, u.z, -fw.z, 0.0],
        [-s.dot(Vec3::from(pos)), -u.dot(Vec3::from(pos)), fw.dot(Vec3::from(pos)), 1.0],
    ]);
    (proj * view).to_cols_array_2d()
}

fn depth0_game(vp: [[f32; 4]; 4], eye: [f32; 3]) -> [[f32; 4]; 5] {
    let planes = nf_view::gpu::extract_frustum(vp);
    let fw = [planes[4][0], planes[4][1], planes[4][2]];
    let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
    [planes[0], planes[1], planes[2], planes[3], [-fw[0], -fw[1], -fw[2], fe + 200.0]]
}

fn quad_centre(c: &[[f32; 3]; 4]) -> [f32; 3] {
    [(c[0][0] + c[1][0] + c[2][0] + c[3][0]) * 0.25,
     (c[0][1] + c[1][1] + c[2][1] + c[3][1]) * 0.25,
     (c[0][2] + c[1][2] + c[2][2] + c[3][2]) * 0.25]
}

/// An eye one doorway back from `p` on one of its two sides: returns (eye, yaw, the resolved
/// cell) for the first standoff that lands the eye in an ENDPOINT cel of the edge — the fixture
/// the walk needs (the eye's cel is `Vision_Portal_Recurse`'s start). The look direction is
/// straight at the quad so the portal fills the frustum (yaw = atan2(dx, dz) — `view_proj`'s
/// forward is `(sin yaw, ·, cos yaw)`).
fn eye_on_edge(graph: &scene::CelGraph, p: &scene::PortalEdge) -> Option<([f32; 3], f32, usize)> {
    let ctr = quad_centre(&p.corners);
    for sign in [1.0f32, -1.0] {
        let eye = [ctr[0] + p.n_fwd[0] * 2.0 * sign, ctr[1] + p.n_fwd[1] * 2.0 * sign, ctr[2] + p.n_fwd[2] * 2.0 * sign];
        let mut st = scene::ViewerCel::default();
        let Some(cell) = scene::eye_cell(graph, &mut st, eye, None) else { continue };
        if cell != p.cel_a && cell != p.cel_b { continue; }
        let dir = [ctr[0] - eye[0], ctr[1] - eye[1], ctr[2] - eye[2]];
        return Some((eye, dir[0].atan2(dir[2]), cell));
    }
    None
}

/// The links on real level data: the three 07000001 SlidingDoors (placements 13/14/69, all
/// placement param 4 = 1 [D docs/atlas/door.md]) each take the portal quad their sphere sits on;
/// the two gallery_secretdoor leaves (224/225 — their doorway has no portal quad: the nearest
/// record's centre is ~4 units away) match nothing, as the game's own straddle+containment leaves
/// them unlinked.
#[test]
fn doors_link_to_their_portal_quads_on_07000001() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    let g = &scene.cel_graph;
    let links: Vec<(usize, usize, usize, u32)> = g.portal_doors.iter().enumerate()
        .filter_map(|(pi, d)| d.map(|di| (pi, g.portals[pi].cel_a, g.portals[pi].cel_b, di)))
        .collect();
    eprintln!("[links] {:?}", links.iter().map(|(pi, a, b, di)| format!("portal {pi} ({a}<->{b}) <- door {di}")).collect::<Vec<_>>());
    assert_eq!(links.len(), 3, "the three straddle-flagged SlidingDoors link, the secretdoor pair does not");
    // The door indices follow create order (placements 13/14/69 → doors 0/1/2, the secretdoors 3/4).
    let mut ends: Vec<(usize, usize)> = links.iter().map(|(_, a, b, _)| (*a, *b)).collect();
    ends.sort();
    assert_eq!(ends, vec![(7, 20), (7, 40), (11, 76)], "the linked edges are the measured doorway pairs");
    let doors: Vec<u32> = links.iter().map(|(_, _, _, d)| *d).collect();
    assert_eq!(doors, vec![1, 0, 2], "doors 0/1/2 (placements 13/14/69) hold the three links");
    for (di, d) in scene.sim.as_ref().unwrap().drones.doors.list.iter().enumerate() {
        if di <= 2 { assert!(d.straddle, "door {di} carries placement param 4"); }
        assert_eq!(d.state, nf_game::door::CLOSED, "door {di} starts closed at create [M Door_Create 0x136100]");
    }
}

/// THE G-035 proof: with a portalled door SHUT, the room behind it is not in the visible-cel set
/// from the other side (strict `VisCelList` walk and the game's `+0x18` chain mask both); with
/// the door OPENED through the real use path (`Player_Activate` 0x1a5170's door arm →
/// `Door_Activate` → `Door_Interp` → state OPEN) the room is. Sisters: the block holds from BOTH
/// sides of the doorway (the game writes the door into both portal halves), and the level's other
/// two links block their rooms the same way.
#[test]
fn a_shut_portalled_door_hides_the_room_behind_it_on_07000001() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    // Eye fixed on the door-0 side of its portal (portal 3, cells 7 <-> 40 — the probe's
    // measured fixture): the eye cell resolves through the real `eye_cell`.
    let p3 = scene.cel_graph.portals[3].clone();
    let (eye, yaw, cell) = eye_on_edge(&scene.cel_graph, &p3).expect("the eye lands in an endpoint cel of portal 3");
    let dest = if cell == p3.cel_a { p3.cel_b } else { p3.cel_a };
    eprintln!("[door0] eye ({:.2},{:.2},{:.2}) cell {cell} dest {dest}", eye[0], eye[1], eye[2]);
    let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
    let depth0 = depth0_game(vp, eye);
    let closed = scene.sim.as_ref().unwrap().drones.doors.closed_flags();
    assert!(closed[0], "door 0 (placement 13) is CLOSED at create");
    // SHUT: the walk ends at the door — dest stays out of the visible set, and the game mask too.
    let vis = scene::portal_visible_cels_from_cell_doors(&scene.cel_graph, cell, eye, &depth0, Some(&closed)).expect("the walk runs");
    assert!(!vis[dest], "cell {dest} must not be visible from cell {cell} while the door is shut (G-035)");
    let sides = { let pl = nf_view::gpu::extract_frustum(vp); [pl[0], pl[1], pl[2], pl[3]] };
    let game = scene::portal_visible_cels_game_from_cell_doors(&scene.cel_graph, cell, eye, &depth0, &sides, Some(&closed)).expect("the game mask runs");
    assert!(!game[dest], "the game mask also keeps the shut door's room out");
    // The shut door itself still draws: it straddles both cels of its portal (`Collide_StraddleCels` 0x1eac98),
    // so the near cel's visibility keeps it (render, 07000001: with only its centre's cel counted the shoji
    // vanished and the sky showed through the doorway). The palette is the room mask the renderer feeds it.
    {
        let g = &scene.cel_graph;
        let room_vis: Vec<bool> = g.room_to_cell.iter().map(|m| m.map(|c2| vis[c2]).unwrap_or(true)).collect();
        let (layout, _) = scene.skin_layout();
        let (pal, _) = scene.skin_palette(&layout, None, 0.0, nf_view::scene::ViewOpts::default(), Some(&room_vis));
        let m = pal[layout.door_base as usize];
        assert!(m[3][1] > -9999.0, "door 0 is drawn while shut and its room is hidden (got {:?})", m[3]);
        // Sister: with no room of either side visible, the door is culled like any object.
        let none = vec![false; room_vis.len()];
        let (pal0, _) = scene.skin_palette(&layout, None, 0.0, nf_view::scene::ViewOpts::default(), Some(&none));
        assert!(pal0[layout.door_base as usize][3][1] <= -9999.0, "no visible side: the door culls");
    }
    // Sister: the block holds from the OTHER side too — both portal halves carry the door.
    let ctr = quad_centre(&p3.corners);
    let back = [ctr[0] - p3.n_fwd[0] * 2.0, ctr[1] - p3.n_fwd[1] * 2.0, ctr[2] - p3.n_fwd[2] * 2.0];
    let mut st = scene::ViewerCel::default();
    if let Some(cell_b) = scene::eye_cell(&scene.cel_graph, &mut st, back, None) {
        if cell_b == p3.cel_a || cell_b == p3.cel_b {
            let dest_b = if cell_b == p3.cel_a { p3.cel_b } else { p3.cel_a };
            let dir = [ctr[0] - back[0], ctr[1] - back[1], ctr[2] - back[2]];
            let vp_b = view_proj(back, dir[0].atan2(dir[2]), 0.0, 16.0 / 9.0);
            let d0_b = depth0_game(vp_b, back);
            let vis_b = scene::portal_visible_cels_from_cell_doors(&scene.cel_graph, cell_b, back, &d0_b, Some(&closed)).expect("the walk runs from the far side");
            assert!(!vis_b[dest_b], "the shut door blocks the walk from cell {cell_b} as well");
            eprintln!("[door0] far side: eye cell {cell_b} -> dest {dest_b} hidden while shut");
        }
    }
    // The level's other two links block the same way (doors 1 and 2 from their own sides).
    for (pi, di) in [(2usize, 1usize), (5, 2)] {
        let p = scene.cel_graph.portals[pi].clone();
        let Some((eye_i, yaw_i, cell_i)) = eye_on_edge(&scene.cel_graph, &p) else { panic!("the eye lands on portal {pi}") };
        let dest_i = if cell_i == p.cel_a { p.cel_b } else { p.cel_a };
        let vp_i = view_proj(eye_i, yaw_i, 0.0, 16.0 / 9.0);
        let d0_i = depth0_game(vp_i, eye_i);
        let vis_i = scene::portal_visible_cels_from_cell_doors(&scene.cel_graph, cell_i, eye_i, &d0_i, Some(&closed)).expect("the walk runs");
        assert!(!vis_i[dest_i], "door {di}'s shut portal hides cell {dest_i} from cell {cell_i}");
        eprintln!("[door{di}] eye cell {cell_i} -> dest {dest_i} hidden while shut");
    }
    // OPEN: the real use path — `Player_Activate`'s door arm (head one unit in front of the door,
    // look at it) → `Door_Activate` → `Door_Interp` to state OPEN.
    let (head, look) = {
        let d = &scene.sim.as_ref().unwrap().drones.doors.list[0];
        let l = [d.pos[0] - eye[0], 0.0, d.pos[2] - eye[2]];
        let len = (l[0] * l[0] + l[2] * l[2]).sqrt().max(1e-4);
        let look = [l[0] / len, 0.0, l[2] / len];
        ([eye[0], d.pos[1] + 1.1, eye[2]], look)
    };
    {
        let s = scene.sim.as_mut().unwrap();
        let (snd, hits) = s.drones.doors.player_use(head, look, &mut s.drones.channels, s.drones.frame);
        eprintln!("[door0] use hits {hits}, sounds {:?}", snd.iter().map(|s| s.0).collect::<Vec<_>>());
        assert!(hits >= 1, "the use sphere meets the door");
    }
    let mut opened = false;
    for _ in 0..240 {
        scene.sim.as_mut().unwrap().step_bond(nf_game::player::Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.doors.list[0].state == nf_game::door::OPEN { opened = true; break; }
    }
    let st0 = scene.sim.as_ref().unwrap().drones.doors.list[0].state;
    assert!(opened, "door 0 reached OPEN through the use path (state {st0})");
    // The same eye, the door open: the walk now crosses into dest.
    let closed = scene.sim.as_ref().unwrap().drones.doors.closed_flags();
    assert!(!closed[0], "door 0 no longer reads closed [M Door_IsClosed 0x1367b0]");
    let vis = scene::portal_visible_cels_from_cell_doors(&scene.cel_graph, cell, eye, &depth0, Some(&closed)).expect("the walk runs");
    assert!(vis[dest], "cell {dest} is visible from cell {cell} once the door has opened");
    let game = scene::portal_visible_cels_game_from_cell_doors(&scene.cel_graph, cell, eye, &depth0, &sides, Some(&closed)).expect("the game mask runs");
    assert!(game[dest], "the game mask shows the open door's room too");
    eprintln!("[door0] open: dest {dest} visible from cell {cell} — the portal opened with the door");
}
