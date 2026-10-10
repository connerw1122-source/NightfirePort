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
//! animations), P = pause, F = firing demo for drones without AI, R = reload, F3 = show held-back drones (spawner reinforcements, drones waiting
//! for a switch), K = turn all switch channels on, F12 = screenshot.
//! The guard AI (nf-game) runs at 60 Hz with the camera as Bond; `--frame N` runs N ticks before a `--shot`, pressing
//! the trigger on the `--pull` ticks.
//! (nf-view-shot.png), E = use (doors), Esc = release mouse, Esc again = quit.

mod gpu;
mod dev;
mod hud;
mod scene;

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3};
use nf_formats::{disc, iso9660::IsoImage, level::Level};
use std::collections::HashSet;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

/// Radians per mouse count. Mouse deltas go straight into yaw/pitch (docs/systems/input.md §8).
const MOUSE_SENSITIVITY: f32 = 0.0025;

/// The game's vertical field of view in play (60°, docs/systems/player.md).
const GAME_FOV: f32 = std::f32::consts::FRAC_PI_3;

struct Camera { pos: Vec3, yaw: f32, pitch: f32, fov: f32 }

impl Camera {
    fn dir(&self) -> Vec3 { Vec3::new(self.pitch.cos() * self.yaw.sin(), self.pitch.sin(), self.pitch.cos() * self.yaw.cos()) }
    /// Right-handed view, wgpu clip depth 0..1, 60° vertical field of view (column-major for the shader).
    /// (full view-projection, the same with the eye at the origin for sky objects that follow the viewer).
    fn view_proj(&self, aspect: f32) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
        let (near, far, f) = (0.05f32, 4000.0f32, 1.0 / (self.fov * 0.5).tan());
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
        Some((p, yaw)) => Camera { pos: Vec3::new(p[0], p[1] + 1.6, p[2]), yaw, pitch: 0.0, fov: GAME_FOV },
        None => {
            let (lo, hi) = scene.bounds;
            Camera { pos: Vec3::new((lo[0] + hi[0]) / 2.0, hi[1], lo[2]), yaw: 0.0, pitch: -0.4, fov: GAME_FOV }
        }
    }
}

/// Builds the sound driver + output from the scene's bank/music, and wires the music logic. The scene's bank,
/// music and tables are consumed (taken out) — the same path `main` uses at startup [M].
fn build_sound(scene: &mut scene::Scene) -> Option<nf_audio::sfx::Sound> {
    let sound = scene.sound_bank.take().map(|(slot, bank, sbf)| {
        let mut driver = nf_audio::sfx::Driver::new();
        driver.load_bank(slot, bank, sbf);
        if let Some((lut, bin)) = scene.streams.take() { driver.load_streams(lut, std::sync::Arc::new(bin)); }
        let output = nf_audio::Output::open();
        if output.is_none() { eprintln!("audio: no output device; running silent"); }
        let music_files = scene.music_files.take().map(|(t, smf, ssd)| (t, smf, std::sync::Arc::new(ssd)));
        nf_audio::sfx::Sound { driver, output, music_files, music: None, music_pending: Vec::new() }
    });
    if let (Some(sim), Some((mm, vars))) = (scene.sim.as_mut(), scene.music_tables.take()) {
        sim.drones.attack_gate.vars = vars.iter().find(|v| v.level == sim.level).copied()
            .or(Some(nf_formats::music::MusicVars { level: sim.level, count: 2, floor: 0, window: 8, distance: 25.0 }));
        sim.drones.music = nf_game::music::Music::new(mm);
        sim.drones.music.event(0, sim.level);
        sim.drones.music.event(1, 1);
    }
    sound
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
            Ok(mut w) => {
                // `Player_Init` (0x1910f8): overrides weapon_data[1]+0xDC (the fists skin) with a level-specific
                // value from PunchSkin.204 [M]. Without this, the port looks up the static value (0x0500003d)
                // instead of the level-specific one (0x050000ab on 07000005), and the fists view model is missing.
                w.apply_player_init_override(u32::from_str_radix(&archive[..8], 16).unwrap_or(0));
                Some(w)
            }
            Err(e) => { eprintln!("weapon_data: {e} — drones drawn unarmed"); None }
        },
        tuning: {
            let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap_or_default();
            nf_game::tuning::read_tuning_vars(&mut t, u32::from_str_radix(&archive[..8], 16).unwrap_or(0));
            t
        },
        hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).ok(),
        sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).ok(),
        surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).map_err(|e| eprintln!("EffectInfo: {e}")).ok(),
        voices: nf_formats::sfx::voice_lines(&elf).map_err(|e| eprintln!("Drone_SfxInfo: {e}")).ok(),
        hud: nf_formats::hud::HudTables::from_action_elf(&elf).map_err(|e| eprintln!("HUD tables: {e}")).ok(),
    };
    let scene = scene::Scene::build(&level, !all, &skeletons, sequences, Some(&drones));
    if std::env::var("NF_DRAWS").is_ok() {
        let mut c: std::collections::BTreeMap<String, (usize, u32)> = Default::default();
        for m in &scene.materials { let e = c.entry(format!("{:?} {:?}", m.layer, m.draw)).or_default(); e.0 += 1; e.1 += m.index_count / 3; }
        for (k, (n, t)) in c { eprintln!("{k}: {n} materials, {t} triangles"); }
        let mut names: std::collections::BTreeMap<String, usize> = Default::default();
        for inst in level.instances(!all).0 { let o = &level.maps[inst.map].objects[inst.object]; if o.flags & 1 != 0 { *names.entry(format!("{} flags {:#x}", o.name.trim(), o.flags)).or_default() += 1; } }
        for (k, n) in names { eprintln!("list 1: {k} x{n}"); }
        let mut fl: std::collections::BTreeMap<String, usize> = Default::default();
        for inst in level.instances(!all).0 { let o = &level.maps[inst.map].objects[inst.object]; if o.flags & 0xc != 0 { *fl.entry(format!("{} flags {:#x}", o.name.trim(), o.flags)).or_default() += 1; } }
        for (k, n) in fl { eprintln!("flag 4/8: {k} x{n}"); }
        for t in level.maps.iter().flat_map(|m| &m.textures).filter(|t| t.frames > 1) { eprintln!("animated texture {:#010x}: {}x{} {} frames, rate {}", t.hashcode, t.width, t.height, t.frames, t.rate); }
        let mut va: std::collections::BTreeMap<u8, usize> = Default::default();
        let mut low: std::collections::BTreeMap<String, usize> = Default::default();
        let mut pr: std::collections::BTreeMap<(u16, bool), usize> = Default::default();
        for inst in level.instances(!all).0 {
            let o = &level.maps[inst.map].objects[inst.object];
            if o.flags & 1 != 0 { continue; }
            for l in &o.gfx.leaves { for b in &l.batches { *pr.entry((b.prim, b.verts.iter().any(|v| v.rgba[3] < 0x7c))).or_default() += 1; for v in &b.verts { *va.entry(v.rgba[3]).or_default() += 1; if v.rgba[3] < 0x7c { *low.entry(o.name.trim().to_string()).or_default() += 1; } } } }
        }
        eprintln!("list-0 batch prims (prim, has low alpha): {pr:?}");
        if let (Some(w), Some(sim)) = (drones.weapons.as_ref(), scene.sim.as_ref()) {
            let r = &w.records[2];
            for o in [0xa0usize, 0xa4, 0xa8, 0xac, 0xb0, 0xb4, 0xb8, 0xbc] {
                let h = u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
                let Some(sc) = sim.scripts.get(&h) else { continue };
                let ops: Vec<String> = sc.commands.iter().filter(|c| c.op == 1 || c.op == 2).map(|c| format!("op{} {:x?}", c.op, c.args)).collect();
                eprintln!("P2K +{o:#x} script {h:08x} len {}: {}", sc.length, ops.join(", "));
            }
        }
        if let Ok(name) = std::env::var("NF_LEAVES") {
            for inst in level.instances(!all).0 {
                let o = &level.maps[inst.map].objects[inst.object];
                if o.name.trim() != name { continue; }
                for (li, l) in o.gfx.leaves.iter().enumerate() {
                    for b in &l.batches {
                        let (lo, hi) = b.verts.iter().fold((255u8, 0u8), |(lo, hi), v| (lo.min(v.rgba[3]), hi.max(v.rgba[3])));
                        let p0 = b.verts.first().map(|v| v.pos);
                        eprintln!("leaf {li} gs {:x?} tex {:?} verts {} alpha {lo}..{hi} rgb0 {:?} p0 {p0:?}", l.gs, b.texture, b.verts.len(), b.verts.first().map(|v| v.rgba));
                    }
                }
                break;
            }
        }
        for (k, n) in low.iter().take(40) { eprintln!("low alpha: {k} x{n}"); }
        let anim = level.maps.iter().flat_map(|m| &m.textures).filter(|t| t.frames > 1).count();
        eprintln!("animated textures: {anim}");
    }
    // The level's sound bank (`Sound_Ready`): its hash's slot in SBINFO.SBI, then SB_<slot>.{SHF,SFX,SBF}.
    let mut scene = scene;
    let level_id = u32::from_str_radix(&archive[..8], 16).unwrap_or(0);
    scene.sound_bank = (|| -> Option<(usize, nf_formats::soundbank::Bank, Vec<u8>)> {
        let hash = nf_game::sound::level_bank(level_id)?;
        let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
        let slot = nf_formats::soundbank::bank_slots(&get("PS2/SBINFO.SBI")?).ok()?.iter().position(|&h| h == hash)?;
        let f = |ext: &str| get(&nf_formats::soundbank::bank_path("ENGLISH", slot, ext));
        let bank = nf_formats::soundbank::Bank::parse(&f("SHF")?, &f("SFX")?).map_err(|e| eprintln!("sound bank: {e}")).ok()?;
        Some((slot, bank, f("SBF")?))
    })();
    // Streamed effects (guards' speech): STREAMS.LUT / STREAMS.BIN.
    scene.streams = (|| {
        let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
        Some((nf_formats::soundbank::stream_lut(&get("PS2/ENGLISH/STREAMS/STREAMS.LUT")?).ok()?, get("PS2/ENGLISH/STREAMS/STREAMS.BIN")?))
    })();
    // Music (`Sound_Ready` → `Music_Event(0, level)`; docs/systems/music.md): the MapMusic / MapDroneData tables, and
    // the level's track through MFXINFO.MXI to PS2/MUSIC/MFX_<n>.{SMF,SSD}.
    scene.music_tables = nf_formats::music::tables(&elf).map_err(|e| eprintln!("music tables: {e}")).ok();
    scene.music_files = (|| {
        let track = scene.music_tables.as_ref()?.0.iter().find(|m| m.level == level_id && m.track != 0)?.track;
        let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
        let n = nf_formats::music::track_file(&get("PS2/MFXINFO.MXI")?, track).ok()??;
        let smf = nf_formats::music::Smf::parse(&get(&nf_formats::music::music_path(n, "SMF"))?).ok()?;
        Some((track, smf, get(&nf_formats::music::music_path(n, "SSD"))?))
    })();
    if let Some((t, smf, ssd)) = &scene.music_files { eprintln!("music: track {t}, {} sections, {} markers, {:.1} s", smf.starts.len(), smf.markers.len(), ssd.len() as f64 * 7.0 / 8.0 / 32000.0); }
    match &scene.sound_bank { Some((slot, b, _)) => eprintln!("sound bank slot {slot}: {} samples, {} effects", b.samples.len(), b.effects.len()), None => eprintln!("no sound bank") }
    eprintln!("{archive} (+ {anim_name}): {}", scene.stats);
    Ok(scene)
}

/// The Character-layer vertices posed after `time` 60 Hz ticks: start animations (`None`) or sequence `choice`.
fn posed_vertices(scene: &scene::Scene, choice: Option<usize>, time: f32, opts: scene::ViewOpts) -> Vec<scene::Vertex> {
    scene.animate(choice, time, opts).into_iter().zip(&scene.vertices[scene.char_vertex_start..])
        .map(|(pos, v)| scene::Vertex { pos, ..*v }).collect()
}

/// The snow quads facing the camera.
fn snow_for(scene: &scene::Scene, cam: &Camera) -> Vec<scene::Vertex> {
    let Some(sim) = scene.sim.as_ref() else { return Vec::new() };
    let f = cam.dir();
    let right = f.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(f).normalize_or_zero();
    sim.snow_quads(right.to_array(), up.to_array())
}

/// One frame of sound (`Sound_UpdateListeners` → `Sound_UpdateSounds` → the batch → `SFXUpdate`): the listener is
/// Bond's eye and view (glb_viewer[0]; the cut-scene camera's listener call is dropped by the EE batcher [M]).
fn sound_tick(s: &mut nf_audio::sfx::Sound, sim: &mut scene::Sim) {
    let (pos, f) = (sim.bond.head(), sim.bond.look());
    // Row 0 of the view matrix: up × forward, the screen-left axis (snapshot: dir (−0.839, 0, −0.545) → (−0.545, 0, 0.839)).
    let norm = { let l = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6); [f[2] / l, 0.0, -f[0] / l] };
    sim.drones.map_sounds.update(&mut sim.drones.audio, pos);
    // `UpdateMusicalEvents` after the batch: its commands go out with the next frame's batch.
    let section = s.music.as_ref().map_or(0, |p| p.now_playing);
    sim.drones.music.update(section);
    let mut fade_down = None;
    for c in std::mem::take(&mut sim.drones.music.out) {
        match c {
            nf_game::music::Cmd::Start { track, section } => s.music_pending.push((0, track, section)),
            nf_game::music::Cmd::Jump { section, instant } => s.music_pending.push((1, section, instant as i32)),
            nf_game::music::Cmd::FadeDown { music } => fade_down = Some(music),
        }
    }
    let driver = &s.driver;
    sim.drones.audio.update(sim.sfx.as_ref(), (pos, norm, [0.0, 1.0, 0.0], f), &|h| driver.is_playing(h));
    let cmds = sim.drones.audio.take_commands();
    s.batch(|drv, out| {
        // `Sound_UpdateListeners`→`SFXSetEnvironment(room env, underwater)` [M sound.md §0]. The cel/room system that
        // supplies `room env` is not yet ported (G-016); default to no reverb / no underwater until it is.
        drv.set_environment(0, 0);
        for c in cmds {
            match c {
                nf_game::sound::Cmd::Listener { pos, norm, dir, .. } => { drv.set_listener(pos, dir, norm); }
                nf_game::sound::Cmd::Start { handle, id, pos, radii } => { drv.start(out, id, pos, handle, radii.map(|(i, o)| (i as i32, o as i32))); }
                nf_game::sound::Cmd::Position { handle, pos } => drv.set_position(handle, None, pos),
                nf_game::sound::Cmd::Remove { handle } => drv.remove(handle, None),
            }
        }
        // `SFXFadeDown(fadeMusic)` (the music end-sting / level-transition fade) [M sound.md §5.5].
        if let Some(music) = fade_down { drv.fade_down(music); }
    });
}

async fn device(instance: &wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: surface, ..Default::default() }).await?;
    let (device, queue) = adapter.request_device(&Default::default()).await?;
    Ok((adapter, device, queue))
}

/// The camera's screen axes for the corona billboards (same basis as the snow quads).
fn cam_axes(cam: &Camera) -> ([f32; 3], [f32; 3]) {
    let f = cam.dir();
    let right = f.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(f).normalize_or_zero();
    (right.to_array(), up.to_array())
}

/// Renders one frame offscreen and writes it as a PNG.
fn shot(scene: &scene::Scene, cam: &Camera, out: &str, choice: Option<usize>, time: f32, mut opts: scene::ViewOpts, notex: bool, nofog: bool) -> Result<()> {
    let (w, h) = (1280u32, 720u32);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (_adapter, device, queue) = pollster::block_on(device(&instance, None))?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let (opts_right, opts_up) = cam_axes(cam);
    opts.right = opts_right;
    opts.up = opts_up;
    let r = gpu::Renderer::new(&device, &queue, format, scene, notex, nofog);
    r.update_vertices(&queue, scene.char_vertex_start, &posed_vertices(scene, choice, time, opts));
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
        dimension: wgpu::TextureDimension::D2, format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let depth = gpu::depth_view(&device, w, h);
    let (vp, vp_rot) = cam.view_proj(w as f32 / h as f32);
    let snow = snow_for(scene, cam);
    let mut enc = r.render(&device, &queue, &view, &depth, vp, vp_rot, cam.pos.to_array(), time as u32, &snow, &hud::overlay(scene, w as f32 / h as f32));
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
    instance: wgpu::Instance,
    /// Minimised (0×0): nothing is configured or drawn until the window has a size again.
    minimized: bool,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
    renderer: gpu::Renderer,
}

struct App {
    /// The sound driver and output (none without a device or bank).
    sound: Option<nf_audio::sfx::Sound>,
    scene: scene::Scene,
    cam: Camera,
    gfx: Option<Gfx>,
    keys: HashSet<KeyCode>,
    grabbed: bool,
    /// Fire button held (left mouse while grabbed) and aim mode (a pad's L1; never set by the mouse, D-002).
    trigger: bool,
    aiming: bool,
    last: std::time::Instant,
    /// Animation: `None` = the game's start animations; `Some(k)` = the k-th sequence per skeleton (`[` / `]`);
    /// time in 60 Hz ticks; paused (`P`).
    choice: Option<usize>,
    anim_time: f32,
    paused: bool,
    /// F3 = show held-back drones (reinforcements, drones waiting for a switch), F = make armed drones fire.
    opts: scene::ViewOpts,
    /// N: the free camera instead of Bond. Keys pressed since the last game tick (jump, crouch), and the mouse turn
    /// (radians) and look (units of π/2) not yet given to a tick.
    noclip: bool,
    just: HashSet<KeyCode>,
    turn: f32,
    look: f32,
    /// Pending weapon/gadget cycle: ±1 for next/prev (mouse wheel = weapons, G = gadgets), or `Some(id)` for a
    /// direct selection (number keys). Consumed by the next game tick.
    weapon_cycle: i8,
    gadget_cycle: i8,
    weapon_switch_to: Option<u8>,
    /// Right mouse held: scope zoom (`Player_Zoom`).
    zoom: bool,
    /// Dev mode (WORKLOG 0083): `None` when `--dev` is off.
    dev: Option<Box<dev::Dev>>,
    /// Debug render overrides (Bug 1): `--no-tex` ignores textures (isolate vertex colour), `--no-fog` skips fog.
    notex: bool,
    nofog: bool,
    /// The ISO path (for level transitions: `LOAD_LEVEL` → reload the next archive).
    iso: String,
    /// A pending level transition: the next level's hashcode, set by `LOAD_LEVEL`; consumed by the main loop.
    pending_level: Option<u32>,
    /// The `--all` flag (draw dynamic entities at their placements, not only 0x8000 cels) — reused on level reload.
    all: bool,
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

    /// A click on the dev menu (F2 open): hit-test the mouse against the menu items and apply the action (WORKLOG 0081).
    fn dev_menu_click(&mut self) {
        let aspect = self.gfx.as_ref().map_or(1.0, |g| g.config.width as f32 / g.config.height as f32);
        let (mouse, spawn_pick) = match self.dev.as_deref_mut() {
            None => return,
            Some(d) => (d.mouse, d.spawn),
        };
        let Some(idx) = dev::menu_hit(self.dev.as_deref().unwrap(), mouse.0, mouse.1, aspect) else { return };
        let Some(action) = dev::menu_action(self.dev.as_deref().unwrap(), idx) else { return };
        match action {
            dev::MenuAction::Close => { if let Some(d) = self.dev.as_deref_mut() { d.menu = false; } }
            dev::MenuAction::Spawn(pick) => {
                // Spawn at the crosshair (the camera's aim point 5 m ahead, or the floor hit).
                let eye = [self.cam.pos.x, self.cam.pos.y, self.cam.pos.z];
                let dir = self.cam.dir().to_array();
                let hit = self.scene.sim.as_ref().and_then(|s| s.world.segment(eye, [eye[0] + dir[0] * 30.0, eye[1] + dir[1] * 30.0, eye[2] + dir[2] * 30.0]));
                let pos = hit.map_or([eye[0] + dir[0] * 5.0, eye[1], eye[2] + dir[2] * 5.0], |t| [eye[0] + (dir[0] * 30.0) * t, eye[1] + (dir[1] * 30.0) * t, eye[2] + (dir[2] * 30.0) * t]);
                if let Some(d) = self.dev.as_deref_mut() { d.spawn = pick; }
                if let Some(d) = self.dev.as_deref() { d.spawn_guard(&mut self.scene, pos, self.cam.yaw); }
            }
            dev::MenuAction::ForceGrenade => { let bond = self.scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos); if let Some(d) = self.dev.as_deref_mut() { d.force_grenade(&mut self.scene, bond); } }
            dev::MenuAction::Impulse => { let bond = self.scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos); if let Some(d) = self.dev.as_deref_mut() { d.impulse_guard(&mut self.scene, bond); } }
            dev::MenuAction::SpecialDeath => { let bond = self.scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos); if let Some(d) = self.dev.as_deref_mut() { d.special_death(&mut self.scene, bond); } }
            dev::MenuAction::ToggleGod => { if let Some(d) = self.dev.as_deref_mut() { d.god_mode = !d.god_mode; } }
            dev::MenuAction::ToggleAmmo => { if let Some(d) = self.dev.as_deref_mut() { d.infinite_ammo = !d.infinite_ammo; } }
            dev::MenuAction::SaveSpot => { let p = self.scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos); if let Some(d) = self.dev.as_deref_mut() { d.saved_spot = Some(p); } }
            dev::MenuAction::RestoreSpot => {
                let spot = self.dev.as_deref().and_then(|d| d.saved_spot);
                if let Some(p) = spot { if let Some(d) = self.dev.as_deref() { d.teleport(&mut self.scene, p, self.cam.yaw); } }
            }
            dev::MenuAction::TimeScale(s) => { if let Some(d) = self.dev.as_deref_mut() { d.time_scale = s; } }
        }
        let _ = spawn_pick;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() { return; }
        let window = Arc::new(el.create_window(Window::default_attributes().with_title("nf-view")).expect("create window"));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone()).expect("create surface");
        let (adapter, device, queue) = pollster::block_on(device(&instance, Some(&surface))).expect("GPU device");
        // A GPU validation error is reported, not fatal (wgpu's default handler panics).
        device.on_uncaptured_error(Arc::new(|e| eprintln!("GPU error: {e}")));
        let size = window.inner_size();
        let config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).expect("surface config");
        surface.configure(&device, &config);
        let depth = gpu::depth_view(&device, config.width, config.height);
        let renderer = gpu::Renderer::new(&device, &queue, config.format, &self.scene, self.notex, self.nofog);
        self.gfx = Some(Gfx { window, instance, minimized: size.width == 0 || size.height == 0, surface, device, queue, config, depth, renderer });
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: winit::event::DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if self.grabbed {
                if self.noclip || self.scene.sim.is_none() {
                    self.cam.yaw -= dx as f32 * MOUSE_SENSITIVITY;
                    self.cam.pitch = (self.cam.pitch - dy as f32 * MOUSE_SENSITIVITY).clamp(-1.55, 1.55);
                } else {
                    // Mouse look (PLAN.md): raw deltas straight into Bond's yaw / pitch at the next tick.
                    // When scoped, the sensitivity is divided by the zoom factor (the scope zooms the view,
                    // so the same mouse delta should turn less) [M Player_Zoom: the zoom affects the view].
                    let zoom = self.scene.sim.as_ref().map_or(1.0, |s| s.zoom);
                    self.turn -= dx as f32 * MOUSE_SENSITIVITY / zoom;
                    self.look -= dy as f32 * MOUSE_SENSITIVITY / zoom / std::f32::consts::FRAC_PI_2;
                }
            }
        }
        // Mouse wheel: weapon switching (next/prev). Up = next (+1), down = prev (−1).
        if let DeviceEvent::MouseWheel { delta: MouseScrollDelta::LineDelta(_x, y) } = event {
            if self.grabbed && !self.noclip {
                if y > 0.0 { self.weapon_cycle = 1; }
                else if y < 0.0 { self.weapon_cycle = -1; }
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => if let Some(g) = &mut self.gfx {
                // Minimised windows report 0×0; a surface cannot be configured at that size.
                g.minimized = s.width == 0 || s.height == 0;
                if !g.minimized {
                    g.config.width = s.width;
                    g.config.height = s.height;
                    g.surface.configure(&g.device, &g.config);
                    g.depth = gpu::depth_view(&g.device, g.config.width, g.config.height);
                }
                g.window.request_redraw();
            },
            // Focus lost (Alt-Tab, a click elsewhere): let the mouse go and drop held keys and the trigger.
            WindowEvent::Focused(false) => {
                self.grab(false);
                self.keys.clear();
                self.just.clear();
                self.trigger = false;
            }
            // Left button: grab the mouse, then fire (action 9) while held. With the dev menu open (F2), a left click
            // selects a menu item instead (WORKLOG 0081).
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                if self.dev.as_deref().is_some_and(|d| d.menu) {
                    if state == ElementState::Pressed { self.dev_menu_click(); }
                } else if !self.grabbed { if state == ElementState::Pressed { self.grab(true) } } else { self.trigger = state == ElementState::Pressed }
            }
            // Right button: scope zoom (mouse-first, like the game's aim button). Held = zoomed in.
            WindowEvent::MouseInput { state, button: MouseButton::Right, .. } => {
                if self.grabbed { self.zoom = state == ElementState::Pressed; }
            }
            // Cursor moves: track the NDC position for the dev menu's click hit-test (when the menu is open).
            WindowEvent::CursorMoved { position, .. } => if let (Some(g), Some(d)) = (&self.gfx, self.dev.as_deref_mut()) {
                if d.menu {
                    let (w, h) = (g.config.width as f32, g.config.height as f32);
                    d.mouse = ((position.x as f32 / w) * 2.0 - 1.0, 1.0 - (position.y as f32 / h) * 2.0);
                }
            },
            // No manual-aim mode with the mouse (PLAN.md D-002): the mouse aims directly; `aiming` stays for a pad.
            WindowEvent::KeyboardInput { event, .. } => if let PhysicalKey::Code(k) = event.physical_key {
                match (k, event.state) {
                    (KeyCode::Escape, ElementState::Pressed) => if self.grabbed { self.grab(false) } else { el.exit() },
                    (KeyCode::F12, ElementState::Pressed) => if let Err(e) = shot(&self.scene, &self.cam, "nf-view-shot.png", self.choice, self.anim_time, self.opts, self.notex, self.nofog) { eprintln!("screenshot: {e}") },
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
                    // Dev mode keys (WORKLOG 0081): F1 = AI overlay, F2 = dev menu, F11 = step one tick (when paused).
                    (KeyCode::F1, ElementState::Pressed) => if let Some(d) = self.dev.as_deref_mut() { d.overlay = !d.overlay; },
                    (KeyCode::F2, ElementState::Pressed) => if let Some(d) = self.dev.as_deref_mut() { d.menu = !d.menu; if d.menu { self.grab(false); } },
                    (KeyCode::F11, ElementState::Pressed) => if let Some(d) = self.dev.as_deref_mut() { if d.time_scale <= 0.0 { d.step = true; } },
                    (KeyCode::F3, ElementState::Pressed) => self.opts.reinforcements = !self.opts.reinforcements,
                    (KeyCode::KeyF, ElementState::Pressed) => self.opts.firing = !self.opts.firing,
                    (KeyCode::KeyN, ElementState::Pressed) => {
                        self.noclip = !self.noclip;
                        // Back on foot: Bond where the free camera is.
                        if !self.noclip { if let Some(sim) = self.scene.sim.as_mut() {
                            sim.bond = nf_game::player::Bond::stand_at(&sim.world, [self.cam.pos.x, self.cam.pos.y - 0.7, self.cam.pos.z], self.cam.yaw);
                        } }
                    }
                    (KeyCode::KeyT, ElementState::Pressed) => if let Some(sim) = self.scene.sim.as_mut() { sim.auto_aim = !sim.auto_aim },
                    // Weapon switching: number keys 1–9 select directly, Q = prev, E (when not use) = next.
                    // The mapping: 1 = P2K (weapon 2), 2 = SG5 Commando (0x16), 3 = sniper (0x24), etc. The exact
                    // slot mapping follows the game's weapon order (selectable weapons with `weapon_data+0x04 == 1`).
                    (KeyCode::Digit1, ElementState::Pressed) => if self.grabbed { self.weapon_switch_to = Some(2) },
                    (KeyCode::Digit2, ElementState::Pressed) => if self.grabbed { self.weapon_switch_to = Some(0x16) },
                    (KeyCode::Digit3, ElementState::Pressed) => if self.grabbed { self.weapon_switch_to = Some(0x24) },
                    // Gadgets (4-7) are disabled until at least one is ported (docs/port-map/gadgets.md).
                    // (KeyCode::Digit4, ElementState::Pressed) => if self.grabbed { self.weapon_switch_to = Some(0x4a) },
                    // (KeyCode::Digit5, ElementState::Pressed) => if self.grabbed { self.weapon_switch_to = Some(0x4e) },
                    // (KeyCode::Digit6, ElementState::Pressed) => if self.grabbed { self.weapon_switch_to = Some(0x50) },
                    // (KeyCode::Digit7, ElementState::Pressed) => if self.grabbed { self.weapon_switch_to = Some(0x54) },
                    (KeyCode::KeyQ, ElementState::Pressed) => if self.grabbed && !self.noclip { self.weapon_cycle = -1 },
                    // Gadgets (G) are disabled until at least one is ported (docs/port-map/gadgets.md).
                    // (KeyCode::KeyG, ElementState::Pressed) => if self.grabbed && !self.noclip { self.gadget_cycle = 1 },
                    // Turn every switch channel on: drones waiting for one (WaitSwitch) start (triggers are not ported).
                    (KeyCode::KeyK, ElementState::Pressed) => if let Some(sim) = self.scene.sim.as_mut() { for c in 1..=255 { sim.drones.set_channel(c) } },
                    // Bug 3: log the name of the object Bond is touching (its AABB contains his feet).
                    (KeyCode::KeyC, ElementState::Pressed) => {
                        let p = if let Some(sim) = self.scene.sim.as_ref() { sim.bond.pos } else { [self.cam.pos.x, self.cam.pos.y, self.cam.pos.z] };
                        let mut hits: Vec<&str> = self.scene.colliders.iter()
                            .filter(|(_, lo, hi)| p[0] >= lo[0] - 0.5 && p[0] <= hi[0] + 0.5 && p[2] >= lo[2] - 0.5 && p[2] <= hi[2] + 0.5 && p[1] >= lo[1] - 2.0 && p[1] <= hi[1] + 0.5)
                            .map(|(n, _, _)| n.as_str()).collect();
                        hits.sort(); hits.dedup();
                        eprintln!("coll: at {:?} touching: {}", p, if hits.is_empty() { "(nothing in range)".to_string() } else { hits.join(", ") });
                    }
                    (_, ElementState::Pressed) => { if !event.repeat { self.just.insert(k); } self.keys.insert(k); }
                    (_, ElementState::Released) => { self.keys.remove(&k); }
                }
            },
            WindowEvent::RedrawRequested => {
                // Bug 5: level transition. A `LOAD_LEVEL` (0xE8) trigger set `pending_level`; reload the scene
                // with the next archive (`ResetMap_LevelToLoad` 0x1d3800). The renderer is rebuilt on the next
                // `Gfx` re-init (the old surface is dropped with the old scene).
                if let Some(next) = self.pending_level.take() {
                    let name = format!("{next:08x}.bin");
                    eprintln!("level: loading {name}");
                    match load(&self.iso, &name, self.all) {
                        Ok(mut scene) => {
                            self.sound = build_sound(&mut scene);
                            if let Some(start) = scene.start { self.cam = Camera { pos: Vec3::new(start.0[0], start.0[1] + 1.6, start.0[2]), yaw: start.1, pitch: 0.0, fov: GAME_FOV }; }
                            self.scene = scene;
                            self.anim_time = 0.0;
                            // Force a Gfx re-init (the old GPU resources are dropped with the old scene).
                            self.gfx = None;
                            eprintln!("level: loaded {name}");
                        }
                        Err(e) => eprintln!("level: failed to load {name}: {e}"),
                    }
                }
                let dt = self.last.elapsed().as_secs_f32().min(0.1);
                self.last = std::time::Instant::now();
                let speed = if self.keys.contains(&KeyCode::ShiftLeft) { 40.0 } else { 8.0 } * dt;
                let fwd = self.cam.dir();
                let right = fwd.cross(Vec3::Y).normalize_or_zero();
                let mut mv = Vec3::ZERO;
                for (k, d) in [(KeyCode::KeyW, fwd), (KeyCode::KeyS, -fwd), (KeyCode::KeyD, right), (KeyCode::KeyA, -right), (KeyCode::Space, Vec3::Y), (KeyCode::ControlLeft, -Vec3::Y)] {
                    if self.keys.contains(&k) { mv += d; }
                }
                if self.noclip || self.scene.sim.is_none() { self.cam.pos += mv * speed; }
                // One animation frame per 60 Hz game update (FRAME_RATE_MUL = 1), docs/formats/animation.md.
                if !self.paused {
                    // Game logic at its fixed 60 Hz, decoupled from the render rate: one AI tick per whole tick of time.
                    // Dev mode's time scale changes the number of ticks run (WORKLOG 0081), not the tick rate.
                    let before = self.anim_time as u32;
                    self.anim_time += dt * 60.0;
                    let elapsed = (self.anim_time as u32).saturating_sub(before).min(8);
                    let n_ticks = self.dev.as_deref_mut().map_or(elapsed, |d| d.tick_count(elapsed));
                    let eye = [self.cam.pos.x, self.cam.pos.y, self.cam.pos.z];
                    let look = self.cam.dir().to_array();
                    if let Some(sim) = self.scene.sim.as_mut() {
                        for _ in 0..n_ticks {
                            if self.noclip { sim.step(eye, look, self.trigger, self.aiming); continue; }
                            // Keys as full-deflection actions: W / S forward (action 2), A / D strafe (action 1, + = left).
                            let held = |k: KeyCode| if self.keys.contains(&k) { 1.0 } else { 0.0 };
                            let input = nf_game::player::Input {
                                forward: held(KeyCode::KeyW) - held(KeyCode::KeyS),
                                strafe: held(KeyCode::KeyA) - held(KeyCode::KeyD),
                                turn: std::mem::take(&mut self.turn),
                                look: std::mem::take(&mut self.look),
                                jump: self.just.remove(&KeyCode::Space),
                                crouch: self.just.remove(&KeyCode::KeyC) | self.just.remove(&KeyCode::ControlLeft),
                                aiming: self.aiming,
                                weapon_cycle: std::mem::take(&mut self.weapon_cycle),
                                gadget_cycle: std::mem::take(&mut self.gadget_cycle),
                                switch_to: self.weapon_switch_to.take(),
                                zoom: self.zoom,
                            };
                            sim.step_bond(input, self.trigger, self.just.remove(&KeyCode::KeyR), self.just.remove(&KeyCode::KeyE));
                            if let Some(s) = self.sound.as_mut() { sound_tick(s, sim); }
                        }
                        // Dev mode after-tick: god mode / infinite ammo (the game's own state, set the way the game sets it).
                        // (Moved below the sim borrow — see the end of this block.)
                        // Bug 5: level transition. When a `LOAD_LEVEL` (0xE8) trigger fires, the sim sets
                        // `drones.level_exit` to the next level's hashcode. Reload the scene with that archive.
                        if let Some(next) = sim.drones.level_exit.take() {
                            eprintln!("level: exit trigger fired → loading {next:08x}");
                            self.pending_level = Some(next);
                        }
                        if !self.noclip {
                            let h = sim.bond.head();
                            self.cam = Camera { pos: Vec3::new(h[0], h[1], h[2]), yaw: sim.bond.yaw, pitch: sim.bond.pitch * std::f32::consts::FRAC_PI_2, fov: GAME_FOV / sim.zoom };
                            // A script's cut-scene camera takes over the view (`Camera_Enable(4)`).
                            if let Some((p, yaw, pitch, fov)) = sim.script_camera() { self.cam = Camera { pos: Vec3::new(p[0], p[1], p[2]), yaw, pitch, fov }; }
                        }
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
                    // Dev mode after-tick: god mode / infinite ammo (the game's own state, set the way the game sets it).
                    if let Some(d) = self.dev.as_deref_mut() { d.after_tick(&mut self.scene); }
                }
                let Some(g) = &mut self.gfx else { return };
                if g.minimized {
                    // Keep the game ticking at a modest pace without drawing.
                    std::thread::sleep(std::time::Duration::from_millis(16));
                    g.window.request_redraw();
                    return;
                }
                let (r, u) = cam_axes(&self.cam);
                self.opts.right = r;
                self.opts.up = u;
                g.renderer.update_vertices(&g.queue, self.scene.char_vertex_start, &posed_vertices(&self.scene, self.choice, self.anim_time, self.opts));
                let frame = match g.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                    wgpu::CurrentSurfaceTexture::Outdated => { g.surface.configure(&g.device, &g.config); g.window.request_redraw(); return; }
                    wgpu::CurrentSurfaceTexture::Lost => {
                        // Recreate the surface (`Lost` cannot be fixed by configuring the old one).
                        match g.instance.create_surface(g.window.clone()) {
                            Ok(sf) => { g.surface = sf; g.surface.configure(&g.device, &g.config); }
                            Err(e) => eprintln!("surface lost and not recreated: {e}"),
                        }
                        g.window.request_redraw();
                        return;
                    }
                    // Timeout, occluded, a validation error: try again next frame.
                    _ => { g.window.request_redraw(); return; }
                };
                let view = frame.texture.create_view(&Default::default());
                let aspect = g.config.width as f32 / g.config.height as f32;
                let (vp, vp_rot) = self.cam.view_proj(aspect);
                let snow = snow_for(&self.scene, &self.cam);
                let mut hud = hud::overlay(&self.scene, aspect);
                if let Some(d) = self.dev.as_deref() { if d.overlay || d.menu { hud.append(&dev::hud(d, &self.scene, aspect, vp)); } }
                let enc = g.renderer.render(&g.device, &g.queue, &view, &g.depth, vp, vp_rot, self.cam.pos.to_array(), self.anim_time as u32, &snow, &hud);
                g.queue.submit([enc.finish()]);
                g.queue.present(frame);
                g.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    // A panic also goes to nf-view-crash.log (message, place, backtrace): a crash in a window without a console
    // leaves a trace.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = format!("{info}\n{}\n", std::backtrace::Backtrace::force_capture());
        let _ = std::fs::write("nf-view-crash.log", &text);
        default_hook(info);
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    const USAGE: &str = "usage: nf-view [<iso>] <level> [--all] [--seq K] [--frame F] [--fire] [--reinforcements] [--dev --scenario <name>] [--no-fog] [--no-tex] [--shot out.png [--cam x y z yaw pitch]]\n\
        <level> is a FILES.BIN name such as 07000005 (\".bin\" optional); without <iso> the image path comes from NF_ISO\n\
        --dev --scenario <name>: place Bond for a feature test (weapon-switch, sg5-pickup, wire-music, autoaim)\n\
        --no-fog: skip the fog screen pass (isolate vertex colour/texture). --no-tex: ignore texture samples (solid white, isolate lighting/fog).";
    // Positional arguments are the ones before the first flag.
    let pos: Vec<&String> = args.iter().take_while(|a| !a.starts_with("--")).collect();
    let (iso, level) = match pos.as_slice() {
        [iso, level] => (iso.to_string(), level.to_string()),
        [level] => (std::env::var("NF_ISO").map_err(|_| anyhow::anyhow!("NF_ISO is not set and no <iso> was given\n{USAGE}"))?, level.to_string()),
        _ => bail!("{USAGE}"),
    };
    let level = if level.to_ascii_lowercase().ends_with(".bin") { level } else { format!("{level}.bin") };
    let all = args.iter().any(|a| a == "--all");
    let dev = args.iter().any(|a| a == "--dev");
    let scenario = args.iter().position(|a| a == "--scenario").and_then(|i| args.get(i + 1)).cloned();
    let scene = load(&iso, &level, all)?;
    let mut cam = start_camera(&scene);
    if let Some(i) = args.iter().position(|a| a == "--cam") {
        let v: Vec<f32> = args[i + 1..i + 6].iter().map(|s| s.parse()).collect::<Result<_, _>>()?;
        cam = Camera { pos: Vec3::new(v[0], v[1], v[2]), yaw: v[3], pitch: v[4], fov: GAME_FOV };
    }
    let num = |flag: &str| -> Result<Option<f32>> { match args.iter().position(|a| a == flag) { Some(i) => Ok(Some(args.get(i + 1).context("missing value")?.parse()?)), None => Ok(None) } };
    let choice = num("--seq")?.map(|k| k as usize);
    let time = num("--frame")?.unwrap_or(0.0);
    let notex = args.iter().any(|a| a == "--no-tex");
    let nofog = args.iter().any(|a| a == "--no-fog");
    if dev {
        eprintln!("dev mode ON — F1: AI overlay  F2: dev menu  F11: step one tick (when paused)");
        if let Some(ref n) = scenario {
            match dev::scenario(n) {
                Some((label, _)) => eprintln!("scenario: {n} — {label}"),
                None => { eprintln!("unknown scenario: {n}\nscenarios: {}", dev::SCENARIOS.join(", ")); }
            }
        }
    }
    let opts = scene::ViewOpts { reinforcements: args.iter().any(|a| a == "--reinforcements"), firing: args.iter().any(|a| a == "--fire"), right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0] };
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
            // Without `--cam`, Bond stands at the start and the camera is his eye (the first-person gun shows).
            let walk = !args.iter().any(|a| a == "--cam");
            // `--channels`: every switch channel on first (as K).
            if args.iter().any(|a| a == "--channels") { for c in 1..=255 { sim.drones.set_channel(c) } }
            // `--bond x,y,z`: Bond starts exactly there (in the air he falls), instead of the level start.
            if let Some(i) = args.iter().position(|a| a == "--bond") {
                let v: Vec<f32> = args.get(i + 1).context("--bond needs x,y,z")?.split(',').map(|t| t.parse()).collect::<Result<_, _>>()?;
                sim.bond = nf_game::player::Bond::new([v[0], v[1], v[2]], sim.bond.yaw);
            }
            for t in 1..=time as u32 {
                if walk { sim.step_bond(Default::default(), pulls.contains(&t), false, false); } else { sim.step([cam.pos.x, cam.pos.y, cam.pos.z], cam.dir().to_array(), pulls.contains(&t), false); }
            }
            if walk { let h = sim.bond.head(); cam = Camera { pos: Vec3::new(h[0], h[1], h[2]), yaw: sim.bond.yaw, pitch: sim.bond.pitch * std::f32::consts::FRAC_PI_2, fov: GAME_FOV }; }
            if walk { if let Some((p, yaw, pitch, fov)) = sim.script_camera() { cam = Camera { pos: Vec3::new(p[0], p[1], p[2]), yaw, pitch, fov }; } }
            eprintln!("Bond at {:?}{}, health {}", sim.bond.pos, if sim.disabled { " (disabled by a script)" } else { "" }, sim.drones.health.health);
            for u in sim.unposed(&scene.characters) { eprintln!("unposed: {u}"); }
            for p in &sim.drones.pickups { eprintln!("pickup: weapon {:#x}, {} rounds at {:?}", p.weapon, p.rounds, &p.m[3][..3]); }
            if let Some(dr) = sim.drones.list.iter().find(|d| !d.shots.is_empty() || d.state != 4) {
                eprintln!("AI: a drone in state {:#x}, {} shots, clip {}", dr.state, dr.shots.len(), dr.clip);
            }
        }
        return shot(&scene, &cam, args.get(i + 1).context("--shot needs a file name")?, choice, time, opts, notex, nofog);
    }
    let event_loop = EventLoop::new()?;
    // Sound: the level's bank in the driver, the default output device (silent without one).
    let mut scene = scene;
    let sound = build_sound(&mut scene);
    // `--dev --scenario <name>`: place Bond and set up the world for a specific feature test.
    // The HEAD's `apply_scenario` (weapon-switch / sg5-pickup / wire-music / autoaim) + this agent's `dev::scenario`
    // (grenade-throw / sniper-cover / ambush / guard-falls-off-ledge / searchlight / door / bullet-flyby / friendly).
    let mut app = App { sound, scene, cam, gfx: None, keys: HashSet::new(), grabbed: false, trigger: false, aiming: false, last: std::time::Instant::now(), choice, anim_time: time, paused: false, opts, noclip: false, just: HashSet::new(), turn: 0.0, look: 0.0, weapon_cycle: 0, gadget_cycle: 0, weapon_switch_to: None, zoom: false, dev: dev.then(|| Box::new(dev::Dev::default())), notex, nofog, iso, pending_level: None, all };
    if dev {
        if let Some(name) = &scenario {
            if let Some(sim) = app.scene.sim.as_mut() { apply_scenario(sim, name); }
            if let (Some(d), Some(n)) = (app.dev.as_deref_mut(), scenario.as_ref()) {
                if let Some((_, f)) = dev::scenario(n) { f(&mut app.scene); d.scenario = Some(leak_str(n)); }
            }
        }
    }
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// Leak a `String` to a `&'static str` (the scenario name lives for the program's life).
fn leak_str(s: &str) -> &'static str { Box::leak(s.to_string().into_boxed_str()) }

/// `--dev --scenario <name>`: places Bond and sets up the world for a feature test. Never changes game logic
/// or the order of RNG calls — only Bond's position, the inventory, or a spawned pickup. Each scenario prints
/// one line saying what to check. See `docs/tools/dev-mode.md`.
fn apply_scenario(sim: &mut scene::Sim, name: &str) {
    let wi = sim.weapons.records.len();
    match name {
        "weapon-switch" => {
            // Bond at the level start with the full owned arsenal: P2K (2), SG5 (0x16), sniper (0x24), gadgets.
            // The SG5 and sniper are given owned + ammo so the weapon cycle finds them.
            if wi > 0x16 { sim.drones.inventory.owned[0x16] = true; sim.drones.inventory.clip[0x16] = 30; sim.drones.inventory.carried[5] = 60; }
            if wi > 0x24 { sim.drones.inventory.owned[0x24] = true; sim.drones.inventory.clip[0x24] = 5; sim.drones.inventory.carried[7] = 25; }
            eprintln!("[dev] weapon-switch: scroll the mouse wheel (or 1-3) to cycle WEAPONS (P2K/SG5/sniper). Gadgets are disabled (not ported yet). Watch the holster/draw animation and listen for the sounds. Set NF_SOUNDLOG=1 to see each animation id.");
        }
        "sg5-pickup" => {
            // A dropped SG5 at Bond's feet. Walk forward to pick it up.
            if wi > 0x16 {
                let p = sim.bond.pos;
                let m: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [p[0] + 1.0, p[1], p[2] + 1.5, 1.0]];
                if let Some(mut pickup) = nf_game::pickup::Pickup::create_simple(&m, 0x16, 12, nf_game::pickup::kind::WEAPON, &sim.weapons, &sim.models) {
                    pickup.state = 1; // already on the floor
                    sim.drones.pickups.push(pickup);
                }
            }
            eprintln!("[dev] sg5-pickup: a dropped SG5 Commando is at Bond's feet — walk forward to pick it up, then scroll to switch to it");
        }
        "wire-music" => {
            // Bond 5 m from the zip wire. Walk in and out — the music section changes (event 0xD).
            if let Some(w) = sim.wires.first() {
                let c = w.closest_point(sim.bond.pos);
                // Place Bond 5 m from the wire's closest point, on the floor.
                let dir = [sim.bond.pos[0] - c[0], 0.0, sim.bond.pos[2] - c[2]];
                let l = (dir[0] * dir[0] + dir[2] * dir[2]).sqrt().max(1e-6);
                let p = [c[0] + 5.0 * dir[0] / l, c[1], c[2] + 5.0 * dir[2] / l];
                sim.bond = nf_game::player::Bond::stand_at(&sim.world, p, sim.bond.yaw);
            }
            eprintln!("[dev] wire-music: Bond is 5 m from the zip wire — walk toward it; when close the music changes to the stealth section, walk away and it returns");
        }
        "autoaim" => {
            // A guard in view. T toggles auto-aim — the crosshair snaps to the guard.
            sim.auto_aim = false; // off by default (D-001)
            // Find the nearest alive guard and face Bond toward it.
            if let Some((i, dr)) = sim.drones.list.iter().enumerate().filter(|(_, d)| d.present && d.health > 0.0).min_by_key(|(_, d)| {
                let dx = d.pos[0] - sim.bond.pos[0]; let dz = d.pos[2] - sim.bond.pos[2];
                (dx * dx + dz * dz) as u64
            }) {
                let dx = dr.pos[0] - sim.bond.pos[0]; let dz = dr.pos[2] - sim.bond.pos[2];
                sim.bond.yaw = dx.atan2(dz);
                eprintln!("[dev] autoaim: guard #{i} is in view — press T to toggle auto-aim (the crosshair snaps to the guard when on)");
            } else {
                eprintln!("[dev] autoaim: no guard found — press T to toggle auto-aim");
            }
        }
        "sniper-scope" => {
            // Give Bond the sniper (0x24) and select it. Hold right mouse to zoom.
            if wi > 0x24 {
                sim.drones.inventory.owned[0x24] = true;
                sim.drones.inventory.clip[0x24] = 5;
                sim.drones.inventory.carried[7] = 25;
                sim.gun.weapon = 0x24;
                sim.gun.switch_request = 0x24;
                sim.view_script = sim.weapons.idle_script(0x24);
                sim.view_time = 0.0;
                sim.view_speed = 1.0;
            }
            eprintln!("[dev] sniper-scope: Bond has the sniper rifle. Hold RIGHT MOUSE to zoom — the FOV narrows and the sensitivity drops. Release to return to normal.");
        }
        _ => {
            eprintln!("[dev] unknown scenario '{name}'. Available: weapon-switch, sg5-pickup, wire-music, autoaim, sniper-scope. See docs/tools/dev-mode.md.");
        }
    }
}
