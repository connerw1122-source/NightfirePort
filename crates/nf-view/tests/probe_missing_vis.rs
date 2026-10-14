//! fix-missing-objects probe 4: the per-frame visibility verdict for the named objects at the
//! user's spots — the exact draw-list input (`scene::eye_cell` + `portal_visible_cels_game_from_cell`,
//! gpu.rs's `cel_visibility` composition), plus each named object's node/chain owner. Opt-in NF_ISO.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near_p, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near_p - far), -1.0],
        [0.0, 0.0, near_p * far / (near_p - far), 0.0],
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

struct Spot {
    archive: &'static str,
    label: &'static str,
    eye_off: [f32; 3],
    yaw: f32,
    /// (placement index, name) pairs to inspect on this level
    wanted: &'static [(usize, &'static str)],
}

const SPOTS: &[Spot] = &[
    Spot { archive: "07000006.bin", label: "fountain", eye_off: [21.12, 39.04 + 0.7, -0.48], yaw: 8.683,
        wanted: &[(352, "OBJECT_Fountain"), (351, "Object_Pondwater"), (61, "OBJECT_LampPost1"), (372, "BlockingPoly_Fountainarea")] },
    Spot { archive: "07000006.bin", label: "guard room", eye_off: [33.50, 41.13 + 0.7, -17.90], yaw: 9.158,
        wanted: &[(80, "OBJECT_GuardroomTable"), (81, "OBJECT_Locker"), (82, "OBJECT_WallClock"), (84, "Object_bed"), (85, "Object_chair"), (89, "Object_Mapboard"), (83, "Object_box")] },
    Spot { archive: "07000006.bin", label: "lockers", eye_off: [31.94, 41.13 + 0.7, -17.36], yaw: 10.756,
        wanted: &[(81, "OBJECT_Locker"), (747, "OBJECT_FuseBox(kind 0x31)")] },
    Spot { archive: "0700000c.bin", label: "pylon", eye_off: [14.22, 16.28 + 0.7, -139.54], yaw: 0.986,
        wanted: &[(160, "pylon_back"), (161, "pylon"), (162, "pylon_left"), (163, "pylon_right"), (113, "pylonshadow"), (97, "turbine-crates2")] },
];

#[test]
fn probe_visibility_verdicts() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for spot in SPOTS {
        let sc = match nf_view::load(&iso, spot.archive, false) { Ok(s) => s, Err(e) => { eprintln!("{}: {e}", spot.archive); continue } };
        let g = &sc.cel_graph;
        let world = sc.sim.as_ref().map(|s| &s.world);
        let eye = spot.eye_off;
        let vp = view_proj(eye, spot.yaw, 0.0, 16.0 / 9.0);
        let planes = nf_view::gpu::extract_frustum(vp);
        let sides = [planes[0], planes[1], planes[2], planes[3]];
        let depth0 = [planes[0], planes[1], planes[2], planes[3], {
            let fw = [planes[4][0], planes[4][1], planes[4][2]];
            let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
            [-fw[0], -fw[1], -fw[2], fe + 200.0]
        }];
        let mut state = scene::ViewerCel::default();
        let cell = scene::eye_cell(g, &mut state, eye, world);
        eprintln!("\n=== {} {} eye {eye:?} yaw {} → eye cell {cell:?} ({} cells) ===", spot.archive, spot.label, spot.yaw, g.cells.len());
        let Some(cell) = cell else { continue };
        let game = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides).expect("game mask");
        eprintln!("  game mask: {}/{} cells visible", game.iter().filter(|v| **v).count(), g.cells.len());
        for (pi, name) in spot.wanted {
            let Some(node) = g.own_cells_main.get(*pi).copied().flatten() else {
                eprintln!("  #{pi:<4} {name:<26} — NO own cell (non-cel placement)");
                continue;
            };
            let n = &g.cells[node as usize];
            let owner = g.chains.iter().enumerate().find(|(_, ch)| ch.contains(&node)).map(|(ri, _)| ri);
            let owner_visited = owner.is_some_and(|ri| game[ri]);
            let in_view = scene::vision_in_view(n.sphere.0, n.sphere.1, &sides);
            let node_vis = game[node as usize];
            eprintln!("  #{pi:<4} {name:<26} node {node:<4} class {:#06x} sphere ({:.1},{:.1},{:.1}) r {:.1} owner {owner:?} owner_visited {owner_visited} in_view {in_view} → node_vis {node_vis}",
                n.flags & 0xffff, n.sphere.0[0], n.sphere.0[1], n.sphere.0[2], n.sphere.1);
        }
        // The drawn set check: materials keyed to these nodes (centre within 8 of the node sphere centre).
        for (pi, name) in spot.wanted {
            let Some(node) = g.own_cells_main.get(*pi).copied().flatten() else { continue };
            let n = &g.cells[node as usize];
            let mut mats = 0; let mut kept = 0;
            for m in sc.materials.iter().filter(|m| m.layer == scene::Layer::World) {
                if m.node != node { continue; }
                mats += 1;
                let vis = game[m.node as usize] && sides.iter().all(|p| p[0] * m.centre[0] + p[1] * m.centre[1] + p[2] * m.centre[2] + p[3] + m.radius >= 0.0);
                if vis { kept += 1; }
            }
            eprintln!("  #{pi:<4} {name:<26} materials at node {node}: {mats}, visible {kept}");
        }
    }
}
