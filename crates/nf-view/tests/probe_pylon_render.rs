//! fix-0c-pylon probe 2: render ONLY the pylon's materials (and the A/B shadow decal) in-process
//! with the exact shot() path, read the frame back, and report what rasterises. Also flips each
//! Draw field on the pylon's materials to find the state that kills the raster. Opt-in NF_ISO.

use nf_view::gpu;
use nf_view::scene::{Layer, Scene, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

#[test]
fn probe_render_pylon_only() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc: Scene = match nf_view::load(&iso, "0700000c.bin", false) { Ok(s) => s, Err(e) => { eprintln!("0700000c: {e}"); return } };
    // The bond camera at the user's spot (pitch 0 here so the whole pylon is in frame).
    if let Some(sim) = sc.sim.as_mut() {
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [14.22, 16.28, -139.54], 0.986);
        for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    }
    let cam = glam::Vec3::new(14.22, 16.96, -139.54);
    let (yaw, pitch) = (0.986f32, 0.0f32);
    let (w, h) = (1280u32, 720u32);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (device, queue) = pollster::block_on(async {
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.expect("adapter");
        adapter.request_device(&Default::default()).await.expect("device")
    });    let format = wgpu::TextureFormat::Rgba8Unorm;

    // The pylon materials and the shadow decal, by node (indices into sc.materials).
    let pylon: Vec<usize> = sc.materials.iter().enumerate().filter(|(_, m)| m.layer == Layer::World && [116u32, 117, 118, 119].contains(&m.node)).map(|(i, _)| i).collect();
    let shadow: Vec<usize> = sc.materials.iter().enumerate().filter(|(_, m)| m.layer == Layer::World && m.node == 74).map(|(i, _)| i).collect();
    eprintln!("pylon materials {pylon:?} shadow {shadow:?}");
    let all: Vec<nf_view::scene::Material> = sc.materials.clone();

    let render = |label: &str, sc: &mut Scene, all: &[nf_view::scene::Material], keep: &[usize], flip: Option<&dyn Fn(&mut nf_view::scene::Draw)>, out: &str| {
        let kept: Vec<nf_view::scene::Material> = keep.iter().map(|&i| all[i].clone()).collect();
        sc.materials = kept;
        if let Some(f) = flip { for m in &mut sc.materials { f(&mut m.draw); } }
        eprintln!("[{label}] {} materials kept", sc.materials.len());
        let mut r = gpu::Renderer::new(&device, &queue, format, sc, false, false);
        let (vp, _) = {
            let cp = pitch.cos(); let fw = glam::Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
            let s = fw.cross(glam::Vec3::Y).normalize(); let u = s.cross(fw);
            let f = 1.0 / (std::f32::consts::FRAC_PI_3 * 0.5).tan();
            let proj = glam::Mat4::from_cols_array_2d(&[
                [f / (w as f32 / h as f32), 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0],
                [0.0, 0.0, 4000.0 / (0.05 - 4000.0), -1.0], [0.0, 0.0, 0.05 * 4000.0 / (0.05 - 4000.0), 0.0]]);
            let view = glam::Mat4::from_cols_array_2d(&[
                [s.x, u.x, -fw.x, 0.0], [s.y, u.y, -fw.y, 0.0], [s.z, u.z, -fw.z, 0.0],
                [-s.dot(cam), -u.dot(cam), fw.dot(cam), 1.0]]);
            ((proj * view).to_cols_array_2d(), [s.to_array(), u.to_array()])
        };
        let (right, up) = { let cp = pitch.cos(); let fw = glam::Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
            let s = fw.cross(glam::Vec3::Y).normalize(); let u = s.cross(fw); (s.to_array(), u.to_array()) };
        let vis = r.cel_visibility(cam.to_array(), vp, sc.sim.as_ref().map(|s| &s.world));
        let mut opts = ViewOpts::default();
        opts.right = right; opts.up = up;
        r.update_skin(&queue, sc, None, 30.0, opts, vis.as_deref());
        r.lights = sc.frame_lights();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2, format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let depth = gpu::depth_view(&device, w, h);
        let mut enc = r.render(&device, &queue, &view, &depth, vp, vp, cam.to_array(), 30, &[], &gpu::HudFrame::default(), None);
        let row = (4 * w).next_multiple_of(256);
        let buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (row * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) } },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
        queue.submit([enc.finish()]);
        buf.map_async(wgpu::MapMode::Read, .., |r| r.expect("map"));
        device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let data = buf.get_mapped_range(..).expect("map range");
        let mut rgba = Vec::with_capacity((4 * w * h) as usize);
        for y in 0..h as usize { rgba.extend_from_slice(&data[y * row as usize..][..4 * w as usize]); }
        drop(data);
        let mut lit = 0usize;
        for px in rgba.chunks_exact(4) { if !(px[0] == 30 && px[1] == 36 && px[2] == 48) { lit += 1; } }
        eprintln!("[{label}] lit pixels {lit}/{}", w * h);
        std::fs::write(out, nf_formats::png::encode_rgba(w, h, &rgba)).ok();
        eprintln!("[{label}] wrote {out}");
    };

    render("shadow-only", &mut sc, &all, &shadow, None, "/home/z/my-project/renders/iso_shadow.png");
    render("pylon-only", &mut sc, &all, &pylon, None, "/home/z/my-project/renders/iso_pylon.png");
    // Subset bisect: which pass's other draws kill the pylon?
    let list0: Vec<usize> = all.iter().enumerate().filter(|(_, m)| m.layer == Layer::World && m.draw.pass == 1).map(|(i, _)| i).collect();
    let pass3o: Vec<usize> = all.iter().enumerate().filter(|(i, m)| m.layer == Layer::World && m.draw.pass == 3 && !pylon.contains(i) && !shadow.contains(i)).map(|(i, _)| i).collect();
    let others: Vec<usize> = all.iter().enumerate().filter(|(_, m)| m.layer != Layer::World || (m.draw.pass != 1 && m.draw.pass != 3)).map(|(i, _)| i).collect();
    eprintln!("list0 {} pass3-other {} other-layer/pass {}", list0.len(), pass3o.len(), others.len());
    render("pylon+list0", &mut sc, &all, &pylon.iter().chain(list0.iter()).copied().collect::<Vec<_>>(), None, "/home/z/my-project/renders/iso_pylon_list0.png");
    render("pylon+pass3", &mut sc, &all, &pylon.iter().chain(pass3o.iter()).copied().collect::<Vec<_>>(), None, "/home/z/my-project/renders/iso_pylon_pass3.png");
    render("pylon+others", &mut sc, &all, &pylon.iter().chain(others.iter()).copied().collect::<Vec<_>>(), None, "/home/z/my-project/renders/iso_pylon_others.png");
    render("full", &mut sc, &all, &(0..all.len()).collect::<Vec<_>>(), None, "/home/z/my-project/renders/iso_full.png");
    // Flip each field to isolate the killer state.
    render("pylon-zwrite", &mut sc, &all, &pylon, Some(&|d: &mut nf_view::scene::Draw| d.zwrite = true), "/home/z/my-project/renders/iso_pylon_zwrite.png");
    render("pylon-atest0", &mut sc, &all, &pylon, Some(&|d: &mut nf_view::scene::Draw| d.atest = 0), "/home/z/my-project/renders/iso_pylon_atest0.png");
    render("pylon-fog1", &mut sc, &all, &pylon, Some(&|d: &mut nf_view::scene::Draw| d.fog = 1), "/home/z/my-project/renders/iso_pylon_fog1.png");
    render("pylon-pass1", &mut sc, &all, &pylon, Some(&|d: &mut nf_view::scene::Draw| d.pass = 1), "/home/z/my-project/renders/iso_pylon_pass1.png");
    render("both", &mut sc, &all, &shadow.iter().chain(pylon.iter()).copied().collect::<Vec<_>>(), None, "/home/z/my-project/renders/iso_both.png");
}
