//! fix-aims20-lights-render probe: CPU-project the aims light quads through the palette + the shot
//! camera — where do the 96 indices' triangles land on screen? Run: cargo run --release --example probe_aims_project
use nf_formats::anim;

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let mut scene = nf_view::load_dev(&iso, "07000005.bin", true).unwrap();
    {
        let sim = scene.sim.as_mut().unwrap();
        let wd = sim.weapons.clone();
        let rounds = wd.clip_size(0x1a).max(1);
        sim.drones.inventory.equip_weapon(&wd, 0x1a, rounds, true);
        sim.drones.inventory.equip_ammo(&wd, 0x1a, rounds * 3);
        sim.gun.switch_request = 0x1a;
    }
    let input = nf_game::player::Input { strafe: 1.0, ..Default::default() };
    for _ in 0..60 { scene.sim.as_mut().unwrap().step_bond(input, false, false, false); }
    let opts = nf_view::scene::ViewOpts::default();
    // The shot camera: Bond's eye, like the --shot walk path.
    let s = scene.sim.as_ref().unwrap();
    let h = s.bond.head();
    // Camera::view_proj is private to the crate — replicate it (fov = GAME_FOV = PI/3).
    let (eye, yaw, pitch) = (glam::Vec3::new(h[0], h[1], h[2]), s.bond.yaw, s.bond.pitch * std::f32::consts::FRAC_PI_2);
    let (near, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
    let (aspect, f) = (1280.0f32 / 720.0, 1.0 / (fov * 0.5).tan());
    let proj = glam::Mat4::from_cols_array_2d(&[[f / aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, far / (near - far), -1.0], [0.0, 0.0, near * far / (near - far), 0.0]]);
    let cy = yaw.cos(); let sy = yaw.sin(); let cp = pitch.cos(); let sp = pitch.sin();
    let fw = glam::Vec3::new(-sy * cp, sp, cy * cp);
    let rgt = fw.cross(glam::Vec3::Y).normalize();
    let up = rgt.cross(fw);
    let view = glam::Mat4::from_cols_array_2d(&[[rgt.x, up.x, -fw.x, 0.0], [rgt.y, up.y, -fw.y, 0.0], [rgt.z, up.z, -fw.z, 0.0], [-rgt.dot(eye), -up.dot(eye), fw.dot(eye), 1.0]]);
    let vpm = (proj * view).to_cols_array_2d();
    let vp = vpm;
    let fs = scene.frame_state(None, 60.0, opts);
    // The aims attachments and their world matrices.
    let aims: Vec<(usize, u8, [[f32; 4]; 4])> = scene.attachments.iter().enumerate()
        .filter(|(_, a)| a.aims_light.is_some())
        .map(|(i, a)| (i, a.aims_light.unwrap(), fs.attach[i])).collect();
    // Per-bone table: where does each posed bone sit relative to the eye?
    let vm_char = scene.attachments.iter().find(|a| a.aims_light == Some(8)).map(|a| a.character as usize).unwrap_or(44);
    println!("  aims view-model character = {vm_char}");
    if let Some((_, world, m)) = &fs.posed[vm_char] {
        let ci = vm_char;
        let nb = scene.characters[ci].skeleton.bones;
        for b in 0..nb as usize {
            let bm = anim::mul(&world.get(b).copied().unwrap_or(*m), m);
            let d = [bm[3][0] - eye.x, bm[3][1] - eye.y, bm[3][2] - eye.z];
            let fwd = d[0] * fw.x + d[1] * fw.y + d[2] * fw.z;
            let rgt_d = d[0] * rgt.x + d[1] * rgt.y + d[2] * rgt.z;
            let up_d = d[0] * up.x + d[1] * up.y + d[2] * up.z;
            println!("  bone{b}: fwd={fwd:+.3} right={rgt_d:+.3} up={up_d:+.3} dist={:.2}", (d[0]*d[0]+d[1]*d[1]+d[2]*d[2]).sqrt());
        }
    }
    // The aims material's index range in the static buffers.
    let mats: Vec<_> = scene.materials.iter().enumerate()
        .filter(|(_, m)| m.layer == nf_view::scene::Layer::Effect && m.key == Some((10, 12)) && m.draw.blend == 1 && m.index_count == 96)
        .collect();
    for (mi, m) in &mats {
        println!("material[{mi}] first={} count={}", m.first_index, m.index_count);
        for t in (m.first_index as usize..(m.first_index + m.index_count) as usize).step_by(3) {
            if t == m.first_index as usize {
                for idx in &scene.indices[t..t + 3] {
                    let v = &scene.vertices[*idx as usize];
                    let cv = &scene.char_verts[(*idx as usize) - scene.char_vertex_start];
                    let ai = cv.attachment.unwrap() as usize;
                    let pm = fs.attach[ai];
                    println!("  raw idx={} attachment={} aims={:?} pm_det={:.3} pm_t=({:.2},{:.2},{:.2}) eye=({:.2},{:.2},{:.2}) fw=({:.2},{:.2},{:.2})",
                        idx, ai, scene.attachments[ai].aims_light,
                        pm[0][0] * (pm[1][1] * pm[2][2] - pm[1][2] * pm[2][1]) - pm[0][1] * (pm[1][0] * pm[2][2] - pm[1][2] * pm[2][0]) + pm[0][2] * (pm[1][0] * pm[2][1] - pm[1][1] * pm[2][0]),
                        pm[3][0], pm[3][1], pm[3][2], eye.x, eye.y, eye.z, fw.x, fw.y, fw.z);
                }
            }
            let mut pts = vec![];
            for idx in &scene.indices[t..t + 3] {
                let v = &scene.vertices[*idx as usize];
                // The GPU path: palette slot = attach_base + ai — recompute via the vert's CharVertex.
                let cv = &scene.char_verts[(*idx as usize) - scene.char_vertex_start];
                let Some(ai) = cv.attachment else { println!("  NOT an attachment vert!"); continue };
                let pm = fs.attach[ai as usize];
                let p = [
                    pm[0][0] * v.pos[0] + pm[1][0] * v.pos[1] + pm[2][0] * v.pos[2] + pm[3][0],
                    pm[0][1] * v.pos[0] + pm[1][1] * v.pos[1] + pm[2][1] * v.pos[2] + pm[3][1],
                    pm[0][2] * v.pos[0] + pm[1][2] * v.pos[1] + pm[2][2] * v.pos[2] + pm[3][2],
                    1.0,
                ];
                let clip = [
                    vp[0][0] * p[0] + vp[1][0] * p[1] + vp[2][0] * p[2] + vp[3][0],
                    vp[0][1] * p[0] + vp[1][1] * p[1] + vp[2][1] * p[2] + vp[3][1],
                    vp[0][2] * p[0] + vp[1][2] * p[1] + vp[2][2] * p[2] + vp[3][2],
                    vp[0][3] * p[0] + vp[1][3] * p[1] + vp[2][3] * p[2] + vp[3][3],
                ];
                let (sx, sy) = if clip[3] > 0.0 { ((clip[0] / clip[3] * 0.5 + 0.5) * 1280.0, (0.5 - clip[1] / clip[3] * 0.5) * 720.0) } else { (-1.0, -1.0) };
                pts.push((sx, sy, clip[3], p));
            }
            let onscreen = pts.iter().any(|(x, y, w, _)| *w > 0.0 && *x >= 0.0 && *x < 1280.0 && *y >= 0.0 && *y < 720.0);
            println!("  tri@{t} onscreen={onscreen} pts={:?}", pts.iter().map(|(x, y, w, _)| (*x as i32, *y as i32, *w as i32)).collect::<Vec<_>>());
        }
    }
}
