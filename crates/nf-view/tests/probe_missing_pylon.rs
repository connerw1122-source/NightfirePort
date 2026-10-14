//! fix-missing-objects probe 6: the pylon spot through the REAL camera path — Bond stood at the
//! user's `--bond` spot (`Bond::stand_at` → `head()`), the sim ticked, then the exact draw-list
//! input. Also dumps the pylon materials' draw states. Opt-in NF_ISO.

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

const PYLON_NODES: [u32; 5] = [116, 117, 118, 119, 74];

#[test]
fn probe_pylon_real_camera() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc = match nf_view::load(&iso, "0700000c.bin", false) { Ok(s) => s, Err(e) => { eprintln!("0700000c: {e}"); return } };
    // The real --bond path: stand Bond at the spot (the floor below decides y), tick, camera = head.
    let sim = sc.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [14.22, 16.28, -139.54], 0.986);
    for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    let eye = sim.bond.head();
    let yaw = sim.bond.yaw;
    let pitch = sim.bond.pitch * std::f32::consts::FRAC_PI_2;
    eprintln!("bond pos {:?} → eye {eye:?} yaw {yaw} pitch {pitch}", sim.bond.pos);
    let g = &sc.cel_graph;
    let world = &sim.world;
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    let planes = nf_view::gpu::extract_frustum(vp);
    let sides = [planes[0], planes[1], planes[2], planes[3]];
    let depth0 = [planes[0], planes[1], planes[2], planes[3], {
        let fw = [planes[4][0], planes[4][1], planes[4][2]];
        let fe = fw[0] * eye[0] + fw[1] * eye[1] + fw[2] * eye[2];
        [-fw[0], -fw[1], -fw[2], fe + 200.0]
    }];
    let mut state = scene::ViewerCel::default();
    let Some(cell) = scene::eye_cell(g, &mut state, eye, Some(world)) else { eprintln!("no eye cell"); return };
    eprintln!("eye cell {cell}");
    let Some(game) = scene::portal_visible_cels_game_from_cell(g, cell, eye, &depth0, &sides) else { eprintln!("no walk"); return };
    eprintln!("game mask {}/{} visible", game.iter().filter(|v| **v).count(), g.cells.len());
    for n in PYLON_NODES {
        let node = &g.cells[n as usize];
        let owner = g.chains.iter().enumerate().find(|(_, ch)| ch.contains(&n)).map(|(ri, _)| ri);
        eprintln!("  node {n:<4} owner {owner:?} owner_visited {} in_view {} → vis {}",
            owner.is_some_and(|ri| game[ri]), scene::vision_in_view(node.sphere.0, node.sphere.1, &sides), game[n as usize]);
    }
    // The pylon materials' draw states + the frustum verdict on the DRAW spheres.
    for (mi, m) in sc.materials.iter().enumerate() {
        if m.layer != scene::Layer::World || !PYLON_NODES.contains(&m.node) { continue; }
        let outside = {
            // Gribb-Hartmann sphere test against the six planes.
            planes.iter().any(|p| p[0] * m.centre[0] + p[1] * m.centre[1] + p[2] * m.centre[2] + p[3] < -m.radius)
        };
        eprintln!("  [m{mi}] node {} centre ({:.1},{:.1},{:.1}) r {:.1} pass {} blend {} zwrite {} atest {} fog {} outside {outside}",
            m.node, m.centre[0], m.centre[1], m.centre[2], m.radius, m.draw.pass, m.draw.blend, m.draw.zwrite, m.draw.atest, m.draw.fog);
    }
}
