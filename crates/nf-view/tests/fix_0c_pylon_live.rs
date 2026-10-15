//! fix-0c-pylon: the 0c pylon was in the drawn set but not on screen. The port drew the post-fog
//! sky (pass 4) AFTER the alpha list (pass 3); the game draws sky group 0 between list 0 and list 1
//! (Game_Draw 0x1e6f50 → psiPostSolid 0x121630 → View_DrawSky v,0 0x1e5ad0 → View_DrawObjList v,1).
//! The sky pass is pinned to the far plane with a LessEqual test, so it repainted every pixel where
//! only no-depth-write alpha-list draws had landed — the pylon's lattice (zwrite off, against
//! sky/sea) was erased; its baked shadow (zwrite on) survived. These tests render the FULL scene
//! through the exact shot() path and check the frame's pixels. Opt-in NF_ISO.

use nf_view::gpu;
use nf_view::scene::{Scene, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Render the full scene from `cam` and return the RGBA frame (row-padded rows collapsed).
fn render_full(scene: &Scene, cam: glam::Vec3, yaw: f32, pitch: f32, fov: f32, ticks: u32) -> (u32, u32, Vec<u8>) {
    let (w, h) = (1280u32, 720u32);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (device, queue) = pollster::block_on(async {
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await.expect("adapter");
        adapter.request_device(&Default::default()).await.expect("device")
    });
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut r = gpu::Renderer::new(&device, &queue, format, scene, false, false);
    let (vp, _) = {
        let cp = pitch.cos(); let fw = glam::Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
        let s = fw.cross(glam::Vec3::Y).normalize(); let u = s.cross(fw);
        let f = 1.0 / (fov * 0.5).tan();
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
    let vis = r.cel_visibility(cam.to_array(), vp, scene.sim.as_ref().map(|s| &s.world));
    let mut opts = ViewOpts::default();
    opts.right = right; opts.up = up;
    r.update_skin(&queue, scene, None, ticks as f32, opts, vis.as_deref());
    r.lights = scene.frame_lights();
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
        dimension: wgpu::TextureDimension::D2, format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let depth = gpu::depth_view(&device, w, h);
    let mut enc = r.render(&device, &queue, &view, &depth, vp, vp, cam.to_array(), ticks, &[], &gpu::HudFrame::default(), None, None);
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
    (w, h, rgba)
}

/// Dark pixels (the dark lattice / baked shadow against a bright fog-sky background).
fn dark_in(w: u32, h: u32, rgba: &[u8], x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
    let mut n = 0u32;
    for y in y0..y1.min(h) { for x in x0..x1.min(w) {
        let p = &rgba[((y * w + x) * 4) as usize..][..4];
        if p[0] < 128 && p[1] < 128 && p[2] < 128 { n += 1; }
    }}
    n
}

/// Bright pixels (the fog-sky background must stay bright: the fix must not paint the sky dark).
fn bright_in(w: u32, h: u32, rgba: &[u8], x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
    let mut n = 0u32;
    for y in y0..y1.min(h) { for x in x0..x1.min(w) {
        let p = &rgba[((y * w + x) * 4) as usize..][..4];
        if p[0] >= 128 && p[1] >= 128 && p[2] >= 128 { n += 1; }
    }}
    n
}

/// The user's playtest spot (`--bond 14.22,16.28,-139.54,0.986,-0.269`), pitch 0 so the whole
/// tower sits in frame. The pylon (placements 160-163, x 44-60, z -115..-106) projects to the
/// tower rect below; its 12 materials ride the alpha list (pass 3, zwrite off).
#[test]
fn the_0c_pylon_is_on_screen_at_the_user_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut sc: Scene = match nf_view::load(&iso, "0700000c.bin", false) { Ok(s) => s, Err(e) => { panic!("0700000c: {e}") } };
    if let Some(sim) = sc.sim.as_mut() {
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [14.22, 16.28, -139.54], 0.986);
        for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    }
    let (w, h, rgba) = render_full(&sc, glam::Vec3::new(14.22, 16.96, -139.54), 0.986, 0.0, std::f32::consts::FRAC_PI_3, 30);
    // The tower's screen band at this camera: the mast above the pier roofline (world bbox
    // 44.3..60.0 × 0..36.4 × −115..−106 projects with the lattice body in x 590..730, y 90..400 —
    // pre-fix that band is pure bright sky/sea, post-fix the lattice struts cross it).
    let lattice = dark_in(w, h, &rgba, 590, 90, 730, 400);
    eprintln!("lattice dark pixels in the tower band: {lattice}");
    assert!(lattice > 3000, "the 0c pylon's lattice is not on screen at the user spot (dark pixels in the tower band: {lattice})");
    // Sister: the sky beside the tower stays bright (the fix must not repaint the backdrop).
    let sky = bright_in(w, h, &rgba, 900, 100, 1150, 400);
    eprintln!("sky bright pixels beside the tower: {sky}");
    assert!(sky > 20000, "the sky beside the pylon went dark (bright pixels: {sky})");
    // Sister: the baked shadow decal (node 74, world x 34.8..54.9, z −143.5..−125.6) still draws —
    // it must survive the re-order (it did pre-fix, because it writes depth).
    let shadow = dark_in(w, h, &rgba, 240, 470, 620, 660);
    eprintln!("shadow dark pixels: {shadow}");
    assert!(shadow > 500, "the pylon's baked shadow vanished from the ground (dark pixels: {shadow})");
}

/// The close-up spot (14 units from the tower, looking straight at it): pre-fix the whole screen
/// area right of the pier building was empty sky/sea where the lattice should fill the view.
#[test]
fn the_0c_pylon_fills_the_view_from_the_closeup_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let sc: Scene = match nf_view::load(&iso, "0700000c.bin", false) { Ok(s) => s, Err(e) => { panic!("0700000c: {e}") } };
    let (w, h, rgba) = render_full(&sc, glam::Vec3::new(34.0, 18.0, -124.0), 0.707, 0.05, std::f32::consts::FRAC_PI_3, 30);
    // The tower fills the centre-right of this frame (the pier building is at the left edge).
    let lattice = dark_in(w, h, &rgba, 560, 60, 1000, 660);
    eprintln!("lattice dark pixels from the close-up: {lattice}");
    assert!(lattice > 2000, "the 0c pylon is not on screen from the close-up spot (dark pixels: {lattice})");
}
