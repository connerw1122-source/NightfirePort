//! fix-0c-pylon probe: the pylon is in the drawn set (102e's pin) but not on screen. Dump the
//! pylon materials' final vertices, texture alpha and projection verdicts — against the pylon's
//! baked SHADOW decal (node 74), which DOES draw, as the in-level A/B pair. Opt-in NF_ISO.

use nf_view::scene::{Layer, Scene};

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

const PYLON_NODES: [u32; 4] = [116, 117, 118, 119];
const SHADOW_NODE: u32 = 74;

#[test]
fn probe_pylon_draw_data() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc: Scene = match nf_view::load(&iso, "0700000c.bin", false) { Ok(s) => s, Err(e) => { eprintln!("0700000c: {e}"); return } };
    // The real --bond path with the USER's pitch (−0.269): the render camera's pitch matters for
    // the projection verdicts (the 102e probe used the stand pitch 0).
    let sim = sc.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [14.22, 16.28, -139.54], 0.986);
    for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    let eye = sim.bond.head();
    let (yaw, pitch) = (sim.bond.yaw, -0.269f32);
    let vp = view_proj(eye, yaw, pitch, 16.0 / 9.0);
    eprintln!("eye {eye:?} yaw {yaw} pitch {pitch}");
    for (mi, m) in sc.materials.iter().enumerate() {
        if m.layer != Layer::World { continue; }
        let pylon = PYLON_NODES.contains(&m.node);
        let shadow = m.node == SHADOW_NODE;
        if !(pylon || shadow) { continue; }
        eprintln!("\n[m{mi}] node {} {} centre ({:.2},{:.2},{:.2}) r {:.2} pass {} blend {} zwrite {} atest {} fog {} key {:?} first {} count {}",
            m.node, if pylon { "PYLON" } else { "shadow" }, m.centre[0], m.centre[1], m.centre[2], m.radius,
            m.draw.pass, m.draw.blend, m.draw.zwrite, m.draw.atest, m.draw.fog, m.key, m.first_index, m.index_count);
        let idx = &sc.indices[m.first_index as usize..(m.first_index + m.index_count) as usize];
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        let (mut amin, mut amax, mut a0) = (255u8, 0u8, 0usize);
        let (mut umin, mut umax, mut uvneg) = ([f32::MAX; 2], [f32::MIN; 2], 0usize);
        for &i in idx {
            let v = &sc.vertices[i as usize];
            for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); }
            amin = amin.min(v.rgba[3]); amax = amax.max(v.rgba[3]); if v.rgba[3] == 0 { a0 += 1; }
            for k in 0..2 { umin[k] = umin[k].min(v.uv[k]); umax[k] = umax[k].max(v.uv[k]); }
            if v.uv.iter().any(|&c| !c.is_finite() || c < -8192.0 || c > 8192.0) { uvneg += 1; }
        }
        eprintln!("  idx {} verts(bbox) lo ({:.2},{:.2},{:.2}) hi ({:.2},{:.2},{:.2})", idx.len(), lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
        eprintln!("  vert alpha min {amin} max {amax} zeros {a0} | uv [{umin:?}..{umax:?}] wild {uvneg}");
        // Project the bbox corners: NDC x/y in [-1,1] = on screen; z/w in [0,1] = depth in range.
        let (mut nx, mut ny) = (0usize, 0usize);
        let (mut zmin, mut zmax) = (f32::MAX, f32::MIN);
        let mut ndc = ([f32::MAX; 3], [f32::MIN; 3]);
        for &cx in &[lo[0], hi[0]] { for &cy in &[lo[1], hi[1]] { for &cz in &[lo[2], hi[2]] {
            let p = [cx, cy, cz, 1.0];
            // vp is to_cols_array_2d() output: vp[i] is COLUMN i — the shader's u.vp * vec4(p,1)
            // computes clip[r] = Σ_c vp[c][r]·p[c] (wgpu mat4x4 * vector, column-major storage).
            let clip: [f32; 4] = (0..4).map(|r| (0..4).map(|c| vp[c][r] * p[c]).sum()).collect::<Vec<_>>().try_into().unwrap();
            let (ndc_x, ndc_y, ndc_z) = (clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]);
            zmin = zmin.min(ndc_z); zmax = zmax.max(ndc_z);
            ndc.0 = [ndc.0[0].min(ndc_x), ndc.0[1].min(ndc_y), ndc.0[2].min(ndc_z)];
            ndc.1 = [ndc.1[0].max(ndc_x), ndc.1[1].max(ndc_y), ndc.1[2].max(ndc_z)];
            if (-1.0..=1.0).contains(&ndc_x) { nx += 1; }
            if (-1.0..=1.0).contains(&ndc_y) { ny += 1; }
        }}}
        eprintln!("  proj: {nx}/8 corners x-in, {ny}/8 y-in, z {zmin:.3}..{zmax:.3} ndc x [{:.2}..{:.2}] y [{:.2}..{:.2}]", ndc.0[0], ndc.1[0], ndc.0[1], ndc.1[1]);
        if let Some((map, t)) = m.key {
            match sc.textures.iter().position(|x| x.key == (map, t)) {
                Some(ti) => {
                    let tx = &sc.textures[ti];
                    let n = tx.width as usize * tx.height as usize;
                    let (mut tmax, mut top, mut tz) = (0u8, 0usize, 0usize);
                    for px in tx.rgba.chunks_exact(4) { let a = px[3]; tmax = tmax.max(a); if a >= 128 { top += 1; } if a == 0 { tz += 1; } }
                    eprintln!("  tex {map}:{t} (scene #{ti}) {}x{} alpha max {tmax} opaque>=128 {top}/{n} zero {tz}", tx.width, tx.height);
                }
                None => eprintln!("  tex {map}:{t} MISSING from scene textures"),
            }
        } else { eprintln!("  key None (untextured)"); }
    }
}
