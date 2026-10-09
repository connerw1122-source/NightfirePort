//! `nf-view`: level viewer for 007: Nightfire (PS2), reading the user's own disc image.
//!
//! ```text
//! nf-view [<iso>] <level> [--all] [--seq K] [--frame F] [--fire] [--reinforcements] [--shot out.png [--cam x y z yaw pitch] [--pull T1,T2…]]
//! ```
//! `<level>` is a FILES.BIN name (`07000005`, `.bin` optional); without `<iso>` the image path comes from `NF_ISO`.
//! `--all` also draws dynamic entities at their placement transforms (not only 0x8000 cels). `--shot` renders one
//! 1280×720 frame headless to a PNG and exits (used to check output without a window).
//!
//! Controls: click to capture the mouse; mouse = look (raw deltas → yaw/pitch, no acceleration), WASD = move,
//! Space / Ctrl = up / down, Shift = fast, [ / ] = browse sequences ([ from the first returns to the game's start
//! animations), P = pause, F = firing demo for drones without AI, R = show held-back drones (spawner reinforcements, drones waiting
//! for a switch), K = turn all switch channels on, F12 = screenshot.
//! The guard AI (nf-game) runs at 60 Hz with the camera as Bond; `--frame N` runs N ticks before a `--shot`, pressing
//! the trigger on the `--pull` ticks.
//! (nf-view-shot.png), Esc = release mouse, Esc again = quit.

mod gpu;
mod scene;

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3};
use nf_formats::{disc, iso9660::IsoImage, level::Level};
use std::collections::HashSet;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

/// Radians per mouse count. Mouse deltas go straight into yaw/pitch (docs/systems/input.md §8).
const MOUSE_SENSITIVITY: f32 = 0.0025;

struct Camera { pos: Vec3, yaw: f32, pitch: f32 }

impl Camera {
    fn dir(&self) -> Vec3 { Vec3::new(self.pitch.cos() * self.yaw.sin(), self.pitch.sin(), self.pitch.cos() * self.yaw.cos()) }
    /// Right-handed view, wgpu clip depth 0..1, 60° vertical field of view (column-major for the shader).
    /// (full view-projection, the same with the eye at the origin for sky objects that follow the viewer).
    fn view_proj(&self, aspect: f32) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
        let (near, far, f) = (0.05f32, 4000.0f32, 1.0 / (30f32.to_radians()).tan());
        let proj = Mat4::from_cols_array_2d(&[[f / aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, far / (near - far), -1.0], [0.0, 0.0, near * far / (near - far), 0.0]]);
        let fw = self.dir().normalize();
        let s = fw.cross(Vec3::Y).normalize();
        let u = s.cross(fw);
        let view = |e: Vec3| Mat4::from_cols_array_2d(&[[s.x, u.x, -fw.x, 0.0], [s.y, u.y, -fw.y, 0.0], [s.z, u.z, -fw.z, 0.0], [-s.dot(e), -u.dot(e), fw.dot(e), 1.0]]);
        ((proj * view(self.pos)).to_cols_array_2d(), (proj * view(Vec3::ZERO)).to_cols_array_2d())
    }
}

fn start_camera(scene: &scene::Scene) -> Camera {
    match scene.start {
        Some((p, yaw)) => Camera { pos: Vec3::new(p[0], p[1] + 1.6, p[2]), yaw, pitch: 0.0 },
        None => {
            let (lo, hi) = scene.bounds;
            Camera { pos: Vec3::new((lo[0] + hi[0]) / 2.0, hi[1], lo[2]), yaw: 0.0, pitch: -0.4 }
        }
    }
}

fn load(iso: &str, archive: &str, all: bool) -> Result<scene::Scene> {
    use nf_formats::{anim::Sequence, character, levelbin::LevelBin};
    use std::collections::HashMap;
    let iso = IsoImage::open(iso)?;
    let (entries, bin) = disc::files_bin_entries(&iso)?;
    let read = |name: &str| -> Result<Option<Vec<u8>>> {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)) else { return Ok(None) };
        Ok(Some(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?))
    };
    let data = read(archive)?.with_context(|| format!("no {archive} in FILES.BIN"))?;
    let level = Level::parse(&data)?;
    let lb = LevelBin::parse(&data)?;
    let skeletons: HashMap<u8, character::Skeleton> = lb.entries.iter().filter(|e| e.kind == 6)
        .map(|e| character::parse_skeleton(lb.data(&data, e)).map(|s| (s.id as u8, s)))
        .collect::<std::result::Result<_, _>>()?;
    // The level's animations: LoaderLoad rewrites the hashcode's 0x00F00000 nibble to 0xF for loader types 0/3,
    // so 070000NN.bin's sequences are in 07F000NN.bin (docs/formats/level-bin.md).
    let anim_name = format!("{}F{}", &archive[..2], &archive[3..]);
    let mut sequences: HashMap<u8, Vec<Sequence>> = HashMap::new();
    if let Some(anim) = read(&anim_name)? {
        let alb = LevelBin::parse(&anim)?;
        for e in alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40) {
            let s = Sequence::parse(alb.data(&anim, e))?;
            sequences.entry(s.skeleton).or_default().push(s);
        }
    }
    // Drone start animations: ACTION.ELF's tables (read through its symbols) and the level's scripts (type 5).
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF on the disc")?)?;
    let drones = scene::DroneContext {
        tables: nf_formats::drone::DroneAnimData::from_action_elf(&elf)?,
        scripts: lb.entries.iter().filter(|e| e.kind == 5)
            .filter_map(|e| nf_formats::anim::Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect(),
        level_id: u32::from_str_radix(&archive[..8], 16).unwrap_or(0),
        // weapon_data is built at boot by its static initialiser; run it over the user's ELF (docs/formats/weapon-data.md).
        props: nf_formats::drone::PropertyTables::from_action_elf(&elf).ok(),
        weapons: match nf_formats::weapon::WeaponData::from_action_elf(&elf) {
            Ok(w) => Some(w),
            Err(e) => { eprintln!("weapon_data: {e} — drones drawn unarmed"); None }
        },
        tuning: {
            let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap_or_default();
            nf_game::tuning::read_tuning_vars(&mut t, u32::from_str_radix(&archive[..8], 16).unwrap_or(0));
            t
        },
        hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).ok(),
        sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).ok(),
    };
    let scene = scene::Scene::build(&level, !all, &skeletons, sequences, Some(&drones));
    eprintln!("{archive} (+ {anim_name}): {}", scene.stats);
    Ok(scene)
}

/// The Character-layer vertices posed after `time` 60 Hz ticks: start animations (`None`) or sequence `choice`.
fn posed_vertices(scene: &scene::Scene, choice: Option<usize>, time: f32, opts: scene::ViewOpts) -> Vec<scene::Vertex> {
    scene.animate(choice, time, opts).into_iter().zip(&scene.vertices[scene.char_vertex_start..])
        .map(|(pos, v)| scene::Vertex { pos, ..*v }).collect()
}

async fn device(instance: &wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: surface, ..Default::default() }).await?;
    let (device, queue) = adapter.request_device(&Default::default()).await?;
    Ok((adapter, device, queue))
}

/// Renders one frame offscreen and writes it as a PNG.
fn shot(scene: &scene::Scene, cam: &Camera, out: &str, choice: Option<usize>, time: f32, opts: scene::ViewOpts) -> Result<()> {
    let (w, h) = (1280u32, 720u32);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (_adapter, device, queue) = pollster::block_on(device(&instance, None))?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let r = gpu::Renderer::new(&device, &queue, format, scene);
    r.update_vertices(&queue, scene.char_vertex_start, &posed_vertices(scene, choice, time, opts));
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
        dimension: wgpu::TextureDimension::D2, format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let depth = gpu::depth_view(&device, w, h);
    let (vp, vp_rot) = cam.view_proj(w as f32 / h as f32);
    let mut enc = r.render(&device, &queue, &view, &depth, vp, vp_rot);
    let row = (4 * w).next_multiple_of(256);
    let buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (row * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
    queue.submit([enc.finish()]);
    buf.map_async(wgpu::MapMode::Read, .., |r| r.expect("map screenshot buffer"));
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let data = buf.get_mapped_range(..)?;
    let mut rgba = Vec::with_capacity((4 * w * h) as usize);
    for y in 0..h as usize { rgba.extend_from_slice(&data[y * row as usize..][..4 * w as usize]); }
    std::fs::write(out, nf_formats::png::encode_rgba(w, h, &rgba))?;
    eprintln!("wrote {out}");
    Ok(())
}

struct Gfx {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
    renderer: gpu::Renderer,
}

struct App {
    scene: scene::Scene,
    cam: Camera,
    gfx: Option<Gfx>,
    keys: HashSet<KeyCode>,
    grabbed: bool,
    /// Fire button held (left mouse while grabbed) and aim mode (right mouse held).
    trigger: bool,
    aiming: bool,
    last: std::time::Instant,
    /// Animation: `None` = the game's start animations; `Some(k)` = the k-th sequence per skeleton (`[` / `]`);
    /// time in 60 Hz ticks; paused (`P`).
    choice: Option<usize>,
    anim_time: f32,
    paused: bool,
    /// R = show held-back drones (reinforcements, drones waiting for a switch), F = make armed drones fire.
    opts: scene::ViewOpts,
}

impl App {
    fn grab(&mut self, on: bool) {
        let Some(g) = &self.gfx else { return };
        if on {
            let ok = g.window.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| g.window.set_cursor_grab(CursorGrabMode::Confined)).is_ok();
            g.window.set_cursor_visible(!ok);
            self.grabbed = ok;
        } else {
            let _ = g.window.set_cursor_grab(CursorGrabMode::None);
            g.window.set_cursor_visible(true);
            self.grabbed = false;
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() { return; }
        let window = Arc::new(el.create_window(Window::default_attributes().with_title("nf-view")).expect("create window"));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone()).expect("create surface");
        let (adapter, device, queue) = pollster::block_on(device(&instance, Some(&surface))).expect("GPU device");
        let size = window.inner_size();
        let config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).expect("surface config");
        surface.configure(&device, &config);
        let depth = gpu::depth_view(&device, config.width, config.height);
        let renderer = gpu::Renderer::new(&device, &queue, config.format, &self.scene);
        self.gfx = Some(Gfx { window, surface, device, queue, config, depth, renderer });
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: winit::event::DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if self.grabbed {
                self.cam.yaw -= dx as f32 * MOUSE_SENSITIVITY;
                self.cam.pitch = (self.cam.pitch - dy as f32 * MOUSE_SENSITIVITY).clamp(-1.55, 1.55);
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => if let Some(g) = &mut self.gfx {
                g.config.width = s.width.max(1);
                g.config.height = s.height.max(1);
                g.surface.configure(&g.device, &g.config);
                g.depth = gpu::depth_view(&g.device, g.config.width, g.config.height);
            },
            // Left button: grab the mouse, then fire (action 9) while held; right button held: aim mode (L1).
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                if !self.grabbed { if state == ElementState::Pressed { self.grab(true) } } else { self.trigger = state == ElementState::Pressed }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Right, .. } => self.aiming = state == ElementState::Pressed,
            WindowEvent::KeyboardInput { event, .. } => if let PhysicalKey::Code(k) = event.physical_key {
                match (k, event.state) {
                    (KeyCode::Escape, ElementState::Pressed) => if self.grabbed { self.grab(false) } else { el.exit() },
                    (KeyCode::F12, ElementState::Pressed) => if let Err(e) = shot(&self.scene, &self.cam, "nf-view-shot.png", self.choice, self.anim_time, self.opts) { eprintln!("screenshot: {e}") },
                    (KeyCode::BracketRight | KeyCode::BracketLeft, ElementState::Pressed) => {
                        self.choice = match (k == KeyCode::BracketRight, self.choice) {
                            (true, None) => Some(0),
                            (true, Some(n)) => Some(n + 1),
                            (false, Some(0) | None) => None,
                            (false, Some(n)) => Some(n - 1),
                        };
                        self.anim_time = 0.0;
                        if let Some(g) = &self.gfx { g.window.set_title(&format!("nf-view — {}", self.scene.sequence_label(self.choice))); }
                    }
                    (KeyCode::KeyP, ElementState::Pressed) => self.paused = !self.paused,
                    (KeyCode::KeyR, ElementState::Pressed) => self.opts.reinforcements = !self.opts.reinforcements,
                    (KeyCode::KeyF, ElementState::Pressed) => self.opts.firing = !self.opts.firing,
                    // Turn every switch channel on: drones waiting for one (WaitSwitch) start (triggers are not ported).
                    (KeyCode::KeyK, ElementState::Pressed) => if let Some(sim) = self.scene.sim.as_mut() { for c in 1..=255 { sim.drones.set_channel(c) } },
                    (_, ElementState::Pressed) => { self.keys.insert(k); }
                    (_, ElementState::Released) => { self.keys.remove(&k); }
                }
            },
            WindowEvent::RedrawRequested => {
                let dt = self.last.elapsed().as_secs_f32().min(0.1);
                self.last = std::time::Instant::now();
                let speed = if self.keys.contains(&KeyCode::ShiftLeft) { 40.0 } else { 8.0 } * dt;
                let fwd = self.cam.dir();
                let right = fwd.cross(Vec3::Y).normalize_or_zero();
                let mut mv = Vec3::ZERO;
                for (k, d) in [(KeyCode::KeyW, fwd), (KeyCode::KeyS, -fwd), (KeyCode::KeyD, right), (KeyCode::KeyA, -right), (KeyCode::Space, Vec3::Y), (KeyCode::ControlLeft, -Vec3::Y)] {
                    if self.keys.contains(&k) { mv += d; }
                }
                self.cam.pos += mv * speed;
                // One animation frame per 60 Hz game update (FRAME_RATE_MUL = 1), docs/formats/animation.md.
                if !self.paused {
                    // Game logic at its fixed 60 Hz, decoupled from the render rate: one AI tick per whole tick of time.
                    let before = self.anim_time as u32;
                    self.anim_time += dt * 60.0;
                    let eye = [self.cam.pos.x, self.cam.pos.y, self.cam.pos.z];
                    let look = self.cam.dir().to_array();
                    if let Some(sim) = self.scene.sim.as_mut() {
                        for _ in before..(self.anim_time as u32).min(before + 8) { sim.step(eye, look, self.trigger, self.aiming); }
                        // HUD stand-in: Bond's health (`BL+0x894`) and the hit flash (`BL+0x968`) in the title.
                        if before / 30 != self.anim_time as u32 / 30 {
                            if let Some(g) = &self.gfx {
                                let h = &sim.drones.health;
                                let hit = if h.flash > 0.0 { "  ** HIT **" } else { "" };
                                // The last pickup message (`Text_AddMsg`, shown 180 ticks).
                                let msg = sim.drones.taken.last().filter(|(f, _)| sim.drones.frame < f + 180).map_or(String::new(), |(_, t)| match t {
                                    nf_game::pickup::Taken::Weapon { weapon } => format!("  picked up weapon {weapon:#x}"),
                                    nf_game::pickup::Taken::Ammo { weapon, rounds } => format!("  picked up {rounds} rounds (weapon {weapon:#x})"),
                                });
                                g.window.set_title(&format!("nf-view — Bond health {:.0}{}  P2K {} / {}{hit}{msg}", h.health, if h.health <= 0.0 { " (dead)" } else { "" }, sim.gun.clip, sim.gun.spare));
                            }
                        }
                    }
                }
                let Some(g) = &mut self.gfx else { return };
                g.renderer.update_vertices(&g.queue, self.scene.char_vertex_start, &posed_vertices(&self.scene, self.choice, self.anim_time, self.opts));
                let frame = match g.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                    _ => { g.surface.configure(&g.device, &g.config); g.window.request_redraw(); return; }
                };
                let view = frame.texture.create_view(&Default::default());
                let aspect = g.config.width as f32 / g.config.height as f32;
                let (vp, vp_rot) = self.cam.view_proj(aspect);
                let enc = g.renderer.render(&g.device, &g.queue, &view, &g.depth, vp, vp_rot);
                g.queue.submit([enc.finish()]);
                g.queue.present(frame);
                g.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    const USAGE: &str = "usage: nf-view [<iso>] <level> [--all] [--seq K] [--frame F] [--fire] [--reinforcements] [--shot out.png [--cam x y z yaw pitch]]\n\
        <level> is a FILES.BIN name such as 07000005 (\".bin\" optional); without <iso> the image path comes from NF_ISO";
    // Positional arguments are the ones before the first flag.
    let pos: Vec<&String> = args.iter().take_while(|a| !a.starts_with("--")).collect();
    let (iso, level) = match pos.as_slice() {
        [iso, level] => (iso.to_string(), level.to_string()),
        [level] => (std::env::var("NF_ISO").map_err(|_| anyhow::anyhow!("NF_ISO is not set and no <iso> was given\n{USAGE}"))?, level.to_string()),
        _ => bail!("{USAGE}"),
    };
    let level = if level.to_ascii_lowercase().ends_with(".bin") { level } else { format!("{level}.bin") };
    let all = args.iter().any(|a| a == "--all");
    let scene = load(&iso, &level, all)?;
    let mut cam = start_camera(&scene);
    if let Some(i) = args.iter().position(|a| a == "--cam") {
        let v: Vec<f32> = args[i + 1..i + 6].iter().map(|s| s.parse()).collect::<Result<_, _>>()?;
        cam = Camera { pos: Vec3::new(v[0], v[1], v[2]), yaw: v[3], pitch: v[4] };
    }
    let num = |flag: &str| -> Result<Option<f32>> { match args.iter().position(|a| a == flag) { Some(i) => Ok(Some(args.get(i + 1).context("missing value")?.parse()?)), None => Ok(None) } };
    let choice = num("--seq")?.map(|k| k as usize);
    let time = num("--frame")?.unwrap_or(0.0);
    let opts = scene::ViewOpts { reinforcements: args.iter().any(|a| a == "--reinforcements"), firing: args.iter().any(|a| a == "--fire") };
    eprintln!("animation: {}", scene.sequence_label(choice));
    if let Some(i) = args.iter().position(|a| a == "--shot") {
        // Headless: run the game for `--frame` ticks with Bond at the camera, then render.
        let mut scene = scene;
        if let Some(sim) = scene.sim.as_mut() {
            // `--pull 100,130`: the trigger is pressed on those ticks (one tick each).
            let pulls: Vec<u32> = match args.iter().position(|a| a == "--pull") {
                Some(i) => args.get(i + 1).context("--pull needs ticks")?.split(',').map(|t| t.parse()).collect::<Result<_, _>>()?,
                None => Vec::new(),
            };
            for t in 1..=time as u32 { sim.step([cam.pos.x, cam.pos.y, cam.pos.z], cam.dir().to_array(), pulls.contains(&t), false); }
            for p in &sim.drones.pickups { eprintln!("pickup: weapon {:#x}, {} rounds at {:?}", p.weapon, p.rounds, &p.m[3][..3]); }
            if let Some(dr) = sim.drones.list.iter().find(|d| !d.shots.is_empty() || d.state != 4) {
                eprintln!("AI: a drone in state {:#x}, {} shots, clip {}", dr.state, dr.shots.len(), dr.clip);
            }
        }
        return shot(&scene, &cam, args.get(i + 1).context("--shot needs a file name")?, choice, time, opts);
    }
    let event_loop = EventLoop::new()?;
    let mut app = App { scene, cam, gfx: None, keys: HashSet::new(), grabbed: false, trigger: false, aiming: false, last: std::time::Instant::now(), choice, anim_time: time, paused: false, opts };
    event_loop.run_app(&mut app)?;
    Ok(())
}
