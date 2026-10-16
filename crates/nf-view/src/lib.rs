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
//! Any other key skips a cut-scene while one is playing (the game's own skip: `SP_Update` state 2's
//! `Input_Action(-1, 0x19)` → `Script_FFwd` [M 0x1dbda8:129-141]; a death cut-scene can't be skipped).
//! The guard AI (nf-game) runs at 60 Hz with the camera as Bond; `--frame N` runs N ticks before a `--shot`, pressing
//! the trigger on the `--pull` ticks.
//! (nf-view-shot.png), E = use (doors), Esc = release mouse, Esc again = quit.

pub mod gpu;
#[cfg(feature = "gamepad")]
pub mod gamepad;
pub mod dev;
pub mod devwin;
pub mod hud;
pub mod credits;
pub mod menu;
pub mod menu_music;
pub mod movie;
pub mod mpnet;
pub mod scene;
pub mod replay;
pub mod panel;
pub mod weapon_assets;
pub mod woman;
pub mod loading;
pub mod async_load;
/// The weapon matrix (task 100d): every on-foot level × every weapon, measured headless.
pub mod matrix;

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

/// The viewer at one tick (pos = the eye, yaw/pitch the look angles, fov half-angle radians). Pub for
/// the headless interpolation tests (fix-interp-ghosting) to build tick cameras the way the loop does.
#[derive(Clone, Copy)]
pub struct Camera { pub pos: Vec3, pub yaw: f32, pub pitch: f32, pub fov: f32 }

/// Render interpolation (G-043): logic runs at a fixed 60 Hz; frames between two ticks draw `t` of the way from
/// the previous tick's camera and palette to the latest one (one tick behind, the standard fixed-step scheme).
/// Jumps larger than `SNAP` (cut-scene cuts, teleports, hide/show) snap instead of blending.
const SNAP: f32 = 4.0;
fn lerp_cam(a: &Camera, b: &Camera, t: f32) -> Camera {
    if a.pos.distance(b.pos) > SNAP || (a.fov - b.fov).abs() > 0.2 { return *b; }
    Camera { pos: a.pos.lerp(b.pos, t), yaw: lerp_yaw(a.yaw, b.yaw, t), pitch: a.pitch + (b.pitch - a.pitch) * t, fov: a.fov + (b.fov - a.fov) * t }
}

/// The yaw half of `lerp_cam`: the short way round the ±π wrap (07000002's corridor turns must not
/// sweep the long way). Extracted with the tick scheduler so the fps-determinism test pins it.
pub fn lerp_yaw(a: f32, b: f32, t: f32) -> f32 {
    let mut dy = b - a;
    while dy > std::f32::consts::PI { dy -= std::f32::consts::TAU; }
    while dy < -std::f32::consts::PI { dy += std::f32::consts::TAU; }
    a + dy * t
}

/// The render loop's tick scheduler (`RedrawRequested`, the `anim_time += dt * 60` block), extracted
/// verbatim so the headless fps-determinism test drives the same arithmetic the winit loop runs:
/// a render frame advances the 60 Hz clock by `dt * 60` and runs the whole ticks that appeared,
/// capped at `max_ticks` (fix-perf-level's drop cap — capped ticks are dropped, not deferred).
/// Logic stays fixed-60 Hz at any frame rate: faster frames just run 0 ticks more often.
pub fn ticks_for_frame(anim_time: &mut f32, dt: f32, max_ticks: u32) -> u32 {
    let before = *anim_time as u32;
    *anim_time += dt * 60.0;
    (*anim_time as u32).saturating_sub(before).min(max_ticks)
}
/// The palette half of the G-043 interpolation: the draw loop blends the previous tick's skin
/// palette into the latest one while `lerp_cam` blends the camera by angle. fix-interp-ghosting:
/// world matrices blend as ROTATION (slerp of the rotation part [M `Quat_Slerp_Acc` 0x1e4188] — a
/// component-wise lerp of a rotation is not a rotation, it shears and shrinks mid-way, the
/// playtest's arm/gun smear) + component lerp of the translation. Matrices whose 3×3 is not a
/// rotation (the collapse/FAR hides, the camera-facing billboards) keep the component lerp, where
/// slerp has no defined meaning. Pub for the headless interpolation tests (fix-interp-ghosting) to
/// drive the exact path the `RedrawRequested` handler runs.
pub fn lerp_palette(a: &[nf_formats::anim::Mat4], b: &[nf_formats::anim::Mat4], t: f32) -> Vec<nf_formats::anim::Mat4> {
    if a.len() != b.len() { return b.to_vec(); }
    a.iter().zip(b).map(|(x, y)| {
        let d = (0..3).map(|k| (x[3][k] - y[3][k]).powi(2)).sum::<f32>().sqrt();
        if d > SNAP { return *y; }
        blend_pose(x, y, t)
    }).collect()
}

/// The rotation blend of one palette entry: slerp of the rotation part ([M `Quat_Slerp_Acc`
/// 0x1e4188], the game's own skin-pose blend) + lerp of the translation; entries whose 3×3 is not
/// orthonormal (collapse/FAR hides, scaled billboards) fall back to the component lerp.
fn blend_pose(x: &nf_formats::anim::Mat4, y: &nf_formats::anim::Mat4, t: f32) -> nf_formats::anim::Mat4 {
    match (rot_quat(x), rot_quat(y)) {
        (Some(qx), Some(qy)) => {
            let qt = nf_game::anim::slerp(qx, qy, t);
            let mut m = quat_rows(qt);
            for k in 0..4 { m[3][k] = x[3][k] + (y[3][k] - x[3][k]) * t; }
            m
        }
        _ => {
            let mut m = *y;
            for r in 0..4 { for c in 0..4 { m[r][c] = x[r][c] + (y[r][c] - x[r][c]) * t; } }
            m
        }
    }
}

/// The quaternion of a palette entry's 3×3 when it is a rotation (rows orthonormal within 1e-3 —
/// collapse/FAR hides and the scaled billboard coronas fail the gate): Shepperd's method on the
/// port's row-vector matrices (stored rows = the images of the local axes, i.e. the transposed
/// column-convention form the textbook formulas assume).
fn rot_quat(m: &nf_formats::anim::Mat4) -> Option<[f32; 4]> {
    for i in 0..3 {
        let n = (m[i][0] * m[i][0] + m[i][1] * m[i][1] + m[i][2] * m[i][2]).sqrt();
        if (n - 1.0).abs() > 1e-3 { return None; }
    }
    for i in 0..3 { for j in (i + 1)..3 {
        let d = m[i][0] * m[j][0] + m[i][1] * m[j][1] + m[i][2] * m[j][2];
        if d.abs() > 1e-3 { return None; }
    } }
    // Proper rotations only: a reflection (det −1) has no quaternion — Shepperd on one would
    // silently return an unrelated rotation. Scaled billboards with scale 1 can be improper.
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            + m[0][1] * (m[1][2] * m[2][0] - m[1][0] * m[2][2])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det < 0.5 { return None; }
    let tr = m[0][0] + m[1][1] + m[2][2];
    let (x, y, z, w);
    if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        w = 0.25 * s;
        x = (m[1][2] - m[2][1]) / s; y = (m[2][0] - m[0][2]) / s; z = (m[0][1] - m[1][0]) / s;
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        w = (m[1][2] - m[2][1]) / s; x = 0.25 * s;
        y = (m[0][1] + m[1][0]) / s; z = (m[0][2] + m[2][0]) / s;
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        w = (m[2][0] - m[0][2]) / s; x = (m[0][1] + m[1][0]) / s; y = 0.25 * s;
        z = (m[1][2] + m[2][1]) / s;
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        w = (m[0][1] - m[1][0]) / s; x = (m[0][2] + m[2][0]) / s; y = (m[1][2] + m[2][1]) / s;
        z = 0.25 * s;
    }
    Some([x, y, z, w])
}

/// The palette-entry matrix of a quaternion: stored rows = the images of the local axes (the
/// row-vector convention's transposed form; round-trips `rot_quat` exactly).
fn quat_rows(q: [f32; 4]) -> nf_formats::anim::Mat4 {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + z * w), 2.0 * (x * z - y * w), 0.0],
        [2.0 * (x * y - z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + x * w), 0.0],
        [2.0 * (x * z + y * w), 2.0 * (y * z - x * w), 1.0 - 2.0 * (x * x + y * y), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// The tick camera as a view → world matrix (C⁻¹ = R·T(eye)): rows (left, up, fwd) + the eye —
/// the exact basis the first-person gun is posed from (`Player_PositionGun` 0x1a3b40 builds the
/// gun's axes as gl = up×gf = the viewer LEFT, gu = gf×gl = up, gf = the turned forward — det +1,
/// a proper rotation, and at yaw 0 it is the identity). So the gun's world matrix decomposes as
/// local pose · C⁻¹ and the camera factor cancels in camera space. (cam_axes' `right = f×Y` rows
/// would be det −1 — a reflection — which no quaternion represents.)
fn cam_view_to_world(cam: &Camera) -> nf_formats::anim::Mat4 {
    let f = cam.dir();
    let l = -cam.right(); // the viewer's left = −(f×Y) [M 0x1a3b40's gl], from the yaw (no ±90° degeneracy)
    let u = f.cross(l); // the game's gu = gf×gl [M 0x1a3b40] (cam_axes' up = right×f, same vector)
    [[l.x, l.y, l.z, 0.0], [u.x, u.y, u.z, 0.0], [f.x, f.y, f.z, 0.0], [cam.pos.x, cam.pos.y, cam.pos.z, 1.0]]
}

/// world → view: C = T(−eye)·Rᵀ (v_view = (v_world − eye)·Rᵀ, rows of the result = right/up/fwd
/// dots). `cam_view_to_world` is its exact inverse for orthonormal rows.
fn cam_world_to_view(cam: &Camera) -> nf_formats::anim::Mat4 {
    let mut m = cam_view_to_world(cam);
    // transpose the rotation, negate the translation: T(−p)·Rᵀ
    let (p, r) = ([m[3][0], m[3][1], m[3][2]], rot_transpose3(&m));
    let mut c = r;
    let dot = |row: usize| -(p[0] * c[0][row] + p[1] * c[1][row] + p[2] * c[2][row]);
    c[3] = [dot(0), dot(1), dot(2), 1.0];
    c
}

/// Transpose the 3×3 block, zeroing the translation row.
fn rot_transpose3(m: &nf_formats::anim::Mat4) -> nf_formats::anim::Mat4 {
    let mut r = *m;
    for i in 0..3 { for j in 0..3 { r[i][j] = m[j][i]; } }
    r[3] = [0.0; 4];
    r
}

/// The palette entries of the ACTIVE first-person view model (the object matrix, its bones and the
/// attachment slots hung on the view model character — the rigid gun parts, the muzzle flash).
fn view_model_entries(scene: &scene::Scene, l: &scene::SkinLayout) -> Option<Vec<usize>> {
    let s = scene.sim.as_ref()?;
    // fix-sentinel-camera-play: the ride draws no view model (the game's substate 10 cleared the
    // anim set [M `Player_ChangeSubState(10)`; L PCSX2 "no view model"]) — frame_state hides the
    // arms' palette; the reattach skips too.
    if s.missile_cam.is_some() { return None; }
    // fix-wallhug-prompt: the creep's third-person camera hides the view model too [M
    // `Player_PositionCamera` 0x18f5d8 case 0xb: `View_SetDrawInNoViews(BLData+0x7e8)`; the entry
    // holsters the weapon, `Player_WeaponNone` 0x196758:114].
    if s.bond.substate == nf_game::player::SS_CREEP { return None; }
    let ci = *scene.view_models.get(&s.gun.weapon)?;
    let base = *l.char_base.get(ci)? as usize;
    let nb = scene.characters.get(ci)?.skeleton.bones as usize;
    let mut v: Vec<usize> = (base..=base + nb).collect();
    let att = l.attach_base as usize;
    for (k, a) in scene.attachments.iter().enumerate() {
        if a.character as usize == ci { v.push(att + k); }
    }
    Some(v)
}

/// fix-interp-ghosting: re-attach the first-person view model's palette entries (the hands and gun,
/// posed in world space from each tick's camera [M `Player_PositionGun` 0x1a3b40]) to the
/// interpolated draw camera, blending them in CAMERA space: for each entry in `vm_entries`, convert
/// both ticks' world matrices camera-relative with that tick's camera (`cam_prev` / `cam_cur`),
/// blend the relative poses at `alpha`, and re-attach the result to `draw_cam` — the same scheme
/// `lerp_cam` uses for the view itself, so mid-turn the gun cannot swim against the view. Entries
/// whose relative pose jumps more than `SNAP` snap to the newer tick (draw/holster switches), like
/// `lerp_palette`'s world-space snap. `out` is the blended palette (from `lerp_palette`), `prev` /
/// `cur` the two ticks' palettes. Pub for the headless interpolation tests to drive the exact path
/// the `RedrawRequested` handler runs.
pub fn reattach_view_model(out: &mut [nf_formats::anim::Mat4], prev: &[nf_formats::anim::Mat4], cur: &[nf_formats::anim::Mat4], alpha: f32,
                           cam_prev: &Camera, cam_cur: &Camera, draw_cam: &Camera, vm_entries: &[usize]) {
    let (vp, vc, vd) = (cam_world_to_view(cam_prev), cam_world_to_view(cam_cur), cam_view_to_world(draw_cam));
    for &k in vm_entries {
        let (Some(mp), Some(mc)) = (prev.get(k), cur.get(k)) else { continue };
        let Some(op) = out.get_mut(k) else { continue };
        let rel_p = nf_formats::anim::mul(mp, &vp);
        let rel_c = nf_formats::anim::mul(mc, &vc);
        // The camera-space snap mirrors `lerp_palette`'s world-space one: a relative-pose jump
        // (draw/holster switches, scoped hide) snaps to the newer tick instead of sweeping.
        let d = (0..3).map(|k2| (rel_p[3][k2] - rel_c[3][k2]).powi(2)).sum::<f32>().sqrt();
        if d > SNAP { *op = *mc; continue; }
        // Non-rotation entries (the collapse/FAR hides) keep the world blend `lerp_palette` wrote.
        if rot_quat(&rel_p).is_none() || rot_quat(&rel_c).is_none() { continue; }
        *op = nf_formats::anim::mul(&blend_pose(&rel_p, &rel_c, alpha), &vd);
    }
}

impl Camera {
    /// The screen-right axis from the yaw alone — `dir() × Y` normalised for |pitch| < 90°, and still defined at
    /// ±90° (the game's clamp limit), where the cross product degenerates and the view flipped upside down.
    fn right(&self) -> Vec3 { Vec3::new(-self.yaw.cos(), 0.0, self.yaw.sin()) }
    fn dir(&self) -> Vec3 { Vec3::new(self.pitch.cos() * self.yaw.sin(), self.pitch.sin(), self.pitch.cos() * self.yaw.cos()) }
    /// Right-handed view, wgpu clip depth 0..1, 60° vertical field of view (column-major for the shader).
    /// (full view-projection, the same with the eye at the origin for sky objects that follow the viewer).
    fn view_proj(&self, aspect: f32) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
        let (near, far, f) = (0.05f32, 4000.0f32, 1.0 / (self.fov * 0.5).tan());
        let proj = Mat4::from_cols_array_2d(&[[f / aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, far / (near - far), -1.0], [0.0, 0.0, near * far / (near - far), 0.0]]);
        let fw = self.dir().normalize();
        // The right axis from the yaw alone (the game builds its view from yaw/pitch angles, not a
        // look-at): `fw × Y` degenerates at ±90° pitch (the game's own clamp limit, `Player_ViewClamping`
        // 0x1a92e8 ±1.0) and flipped the screen upside down there (the user, 1015-0046). For |pitch| < 90°
        // this is the same vector `fw × Y` normalised.
        let s = self.right();
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
/// music and tables are consumed (taken out) — the same path `main` uses at startup [M]. Public so the live
/// tests drive the exact runtime driver (fix-weapon-sounds).
pub fn build_sound(scene: &mut scene::Scene) -> Option<nf_audio::sfx::Sound> {
    let sound = scene.sound_bank.take().map(|(slot, bank, sbf)| {
        let mut driver = nf_audio::sfx::Driver::new();
        driver.load_bank(slot, bank, sbf);
        // The spare banks: every other SBINFO slot (fix-weapon-sounds). The level's own stays primary, so
        // ids it carries resolve exactly as the one-bank driver did [M `Sound_Ready` 0x1dceb8].
        for (slot, bank, sbf) in std::mem::take(&mut scene.sound_spares) { driver.load_spare_bank(slot, bank, sbf); }
        if let Some((lut, bin)) = scene.streams.take() { driver.load_streams(lut, std::sync::Arc::new(bin)); }
        let output = nf_audio::Output::open();
        if output.is_none() { eprintln!("audio: no output device; running silent"); }
        let music_files = scene.music_files.take().map(|(t, smf, ssd)| (t, smf, std::sync::Arc::new(ssd)));
        nf_audio::sfx::Sound { driver, output, music_files, music: None, music_pending: Vec::new() }
    });
    if let (Some(sim), Some((mm, vars))) = (scene.sim.as_mut(), scene.music_tables.take()) {
        if sim.drones.music.table.is_empty() {
            wire_music(sim, mm, vars);
        }
    }
    sound
}

/// `Sound_Ready`'s music wiring [M docs/systems/music.md]: the MapMusic/MapDroneData tables into the
/// sim's music logic + the attack gate, then `Music_Event(0, level)` / `Music_Event(1, 1)` — the start
/// pulse the first `UpdateMusicalEvents` consumes. fix-party-mission: the tests' `load` path now wires
/// the music too (the cut-scene music jumps were test-invisible before); `build_sound` reuses this.
fn wire_music(sim: &mut scene::Sim, mm: Vec<nf_formats::music::MapMusic>, vars: Vec<nf_formats::music::MusicVars>) {
    sim.drones.attack_gate.vars = vars.iter().find(|v| v.level == sim.level).copied()
        .or(Some(nf_formats::music::MusicVars { level: sim.level, count: 2, floor: 0, window: 8, distance: 25.0 }));
    sim.drones.music = nf_game::music::Music::new(mm);
    sim.drones.music.event(0, sim.level);
    sim.drones.music.event(1, 1);
}

pub fn load(iso: &str, archive: &str, all: bool) -> Result<scene::Scene> {
    load_inner(iso, archive, all, false)
}

/// mp-core: the `--mp` load — the same parse, plus the match core (the placement registrations,
/// `MP_Init`/`MP_Start` and the human spawn at the policy's point) inside `load_inner`.
pub fn load_mp(iso: &str, archive: &str, all: bool) -> Result<scene::Scene> {
    load_inner(iso, archive, all, true)
}

fn load_inner(iso: &str, archive: &str, all: bool, mp: bool) -> Result<scene::Scene> {
    load_impl(iso, archive, all, None, false, &[], mp, None, 1)
}

/// net-match-host: the ONLINE match load — the MP core from the negotiated settings (the host's
/// choices + seed, equal on every machine) instead of the `--mp` default. The level build draws
/// from the settings' seed (see `Scene::build`'s Rng note).
pub fn load_mp_carry(iso: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>, settings: nf_game::mp::MpSettings) -> Result<scene::Scene> {
    load_impl(iso, archive, all, save, false, &[], true, Some(settings), 1)
}

/// `--dev`: the same load with the dev view-model merge (folded 190-fix-dev2). Normal play never
/// passes `dev = true`, so no extra archive is opened without the flag.
pub fn load_dev(iso: &str, archive: &str, all: bool) -> Result<scene::Scene> {
    load_impl(iso, archive, all, None, true, &[], false, None, 1)
}

/// `load` pre-merged for the given weapon ids (`--give` at startup, the dev menu's lazy give):
/// each id's first-person skin (+0xDC), meshes, skeleton, `07F` sequences and view scripts merge
/// from their owning archive before the scene builds, so the weapon draws exactly as it does on a
/// level that carries it (`weapon_assets::merge_weapon_assets`).
pub fn load_weapons(iso: &str, archive: &str, all: bool, wids: &[u8]) -> Result<scene::Scene> {
    load_impl(iso, archive, all, None, false, wids, false, None, 1)
}

/// The sim catch-up cap (`NF_MAXTICKS`, default 4) — see the frame loop.
static MAX_TICKS: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

/// net-consolidate: the state-hash cadence moved into the session's own
/// `hash_every` (nf_net `session_cfg`, mpnet.rs) — the app submits every tick
/// and the session sends + piggybacks on that schedule.
#[allow(dead_code)]
const NET_HASH_EVERY: u32 = 15;

/// `load` with the level-transition carry-over (fix-level2). `Some(save)` = a warm transition:
/// `Player_Start` (0x190fd0) picks the type-0x24 start whose gate channel the previous level's exit
/// wrote, and `Player_InitWeapon` (0x1a7ef8) runs the per-level loadout switch over `Player_RamLoad`
/// (0x1957a8). `None` = a cold load (the game's boot path; `Player_RamLoad`'s whole body is skipped).
pub fn load_with_carry(iso: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>) -> Result<scene::Scene> {
    load_impl(iso, archive, all, save, false, &[], false, None, 1)
}

/// The carry-over load pre-merged for the given weapon ids (the dev menu's lazy give: the same
/// warm reload the level transition runs, plus the given weapons' assets).
pub fn load_with_carry_weapons(iso: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>, wids: &[u8]) -> Result<scene::Scene> {
    load_impl(iso, archive, all, save, false, wids, false, None, 1)
}

/// `--dev` level transition: the carry-over load with the dev view-model merge.
pub fn load_with_carry_dev(iso: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>) -> Result<scene::Scene> {
    load_impl(iso, archive, all, save, true, &[], false, None, 1)
}

/// difficulty-effects: the front-end's launch path — the mission loads with the difficulty the
/// player picked on the wheel (GameState+0x28 = 0x2a3790; every create-time gate reads it
/// [M C_SBNFDFCTY 0x216ca8:15, 0x1c7e78:25 boot default 2]). The plain `load*` wrappers keep 1
/// (the old direct-load replay parity).
pub fn load_with_carry_difficulty(iso: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>, difficulty: u32) -> Result<scene::Scene> {
    load_impl(iso, archive, all, save, false, &[], false, None, difficulty)
}

/// difficulty-effects: the warm reload with a weapon-asset set at the selected difficulty.
pub fn load_with_carry_weapons_difficulty(iso: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>, wids: &[u8], difficulty: u32) -> Result<scene::Scene> {
    load_impl(iso, archive, all, save, false, wids, false, None, difficulty)
}

/// The fail/death menu's Restart serve — the reload loop's `level_exit_restart` branch
/// (`fix-restart-health`). `P_ENDMISSION_Handler` 0x216618:19-21 = `ResetMap_LevelToLoad(current,
/// 1, 1)` + `Player_RamLoad(Bond, 1)`; `Player_RamLoad` 0x1957a8 floors the health at the
/// difficulty's `ContinueHealthBoost*` — 50.0 on all three [L ACTION.ELF 0x30cc34-3c] — and the
/// disasm writes the max BACK INTO the buffer (`swc1 $f12, 0x5fc` at 0x1958e4) before
/// `Player_SetHealth` (jal 0x1960b8 at 0x1958e8, $f12 = the floored value, $a0 = BLData). The
/// boost arg 1 is the port's `RamSave::continue_boost` on this serve (fix-death-retry-boost),
/// with the difficulty global's port mirror `GameState+0x40` = `Progress::difficulty`
/// [M 0x1957a8:23]. The reload's `Player_InitWeapon` then re-runs the level's entry arm over
/// that buffer and re-saves it [M 0x1a7ef8:325]. So the serve is the ENTRY checkpoint — the
/// buffer as of this level's load — with the boost floor: after a COLD load the buffer holds the
/// fresh load's health 100 (`Player_Init` 0x1910f8:128-138 sets it; the `Player_RamLoad` body is
/// gated on `IsWarmReset` [M 0x1957a8:9], so nothing overwrites it; the unconditional re-save
/// captures it) — the user's "if I die ... I should be at full health". `entry = None` = no load
/// has completed yet (the game's BSS zeros — unreachable in play, since every load ends in the
/// 0x1a7ef8:325 re-save): the shapes are zeroed at the live sizes and the boost keeps the health
/// at the floor (max(0, 50.0) = 50.0 [M 0x1957a8:21-34]).
pub fn fail_restart_save(entry: Option<&nf_game::player::RamSave>, sim: &scene::Sim) -> nf_game::player::RamSave {
    let mut s = entry.cloned().unwrap_or_else(|| {
        let mut z = nf_game::player::RamSave::capture(
            &sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels);
        z.health = 0.0; z.armour = 0.0;
        for v in z.carried.iter_mut() { *v = 0; }
        for v in z.clip.iter_mut() { *v = 0; }
        for o in z.owned.iter_mut() { *o = false; }
        for k in z.keys.iter_mut() { *k = false; }
        for c in z.channels.iter_mut() { *c = 0; }
        for t in z.ch_time.iter_mut() { *t = 0; }
        z
    });
    // The handler's `Player_RamLoad(Bond, 1)` [M 0x216618:21 → 0x1957a8:21-34]: the difficulty's
    // `ContinueHealthBoost*` max, written into the serve (the game writes it into the resident
    // buffer at 0x1958e4) — `continue_boost` is the game-logic port, not a viewer constant.
    s.continue_boost(sim.drones.progress.difficulty);
    // fix-objective-messages: the mission record states ride the game's `.data` and the restart
    // resets only the player (`P_ENDMISSION_Handler` 0x216618:19-21 — no mission reinit), so the
    // LIVE mission states ride the serve (the reloaded level's fresh `Mission` must not re-announce
    // the earlier parts' objectives).
    // The payload must be the shape the game's `Mission_Init(level, 1)` LEAVES the `.data` in when
    // the reload runs it: previous parts keep their states, current + future parts reset (enable →
    // 1, else 0) [M 0x182270:68-129]. The port applies the carried snapshot AFTER its
    // `Mission_Init` latch (drone.rs), so the dying run's dirt (the monitor's 2/3 on the fired
    // condition channels, the fail states) must be reset HERE — the game's RamLoad does not touch
    // the states, Mission_Init's reset is the last writer.
    let mut ms = sim.drones.mission.snapshot();
    {
        let mut off = 0usize;
        for l in nf_game::mission::LEVELS.iter() {
            let n = l.objectives.len();
            if l.mission == sim.drones.mission.base_map && l.part >= sim.drones.mission.order {
                for (j, o) in l.objectives.iter().enumerate() {
                    ms.states[off + j] = if o.enable != 0 { 1 } else { 0 }; // [M 0x182270:98-105]
                }
            }
            off += n;
        }
    }
    s = s.with_mission_snapshot(ms);
    s
}

/// The entry checkpoint a later fail-Restart restores: the buffer as of level ENTRY. The game
/// re-saves it with the POST-load state on EVERY load — `Player_InitWeapon`'s unconditional
/// `Player_RamSave` [M 0x1a7ef8:325] — cold loads included: `Player_Init`'s health 100 / armour 0
/// [M 0x1910f8:128-138] stand (the `Player_RamLoad` body is skipped when `IsWarmReset == 0` [M
/// 0x1957a8:9]) and the per-level arm's loadout/armour are what a restart gets back. On a warm
/// load the restored state is captured unchanged (health = the carried exit health [M 0x1957a8
/// SetHealth call], the arm's re-equips included).
pub fn entry_checkpoint_save(sim: &scene::Sim) -> nf_game::player::RamSave {
    nf_game::player::RamSave::capture(
        &sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
        .with_mission(&sim.drones.mission)
}

fn load_impl(iso_path: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>, dev: bool, give: &[u8], mp: bool, mp_settings: Option<nf_game::mp::MpSettings>, difficulty: u32) -> Result<scene::Scene> {
    use nf_formats::{anim::Sequence, character, levelbin::LevelBin};
    use std::collections::HashMap;
    let iso = IsoImage::open(iso_path)?;
    let (entries, bin) = disc::files_bin_entries(&iso)?;
    let read = |name: &str| -> Result<Option<Vec<u8>>> {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)) else { return Ok(None) };
        Ok(Some(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?))
    };
    let data = read(archive)?.with_context(|| format!("no {archive} in FILES.BIN"))?;
    let mut level = Level::parse(&data)?;
    // ── The session object table (`models-weapons-items`) ───────────────────────────────────────────
    // `hashtable_delete_hashtype` is called from the loadable-pack path ONLY (`LoadableLoad`
    // 0x1d69e8 / `LoadableReload` 0x1d6ab0 — the cut-scene packs) in all of ACTION.ELF [M decompile
    // grep]: the map-object hashtable accumulates every archive loaded in the session, so a level's
    // weapon-model meshes resolve even when its own archive doesn't hold them. Measured [D]: Tower-2's
    // guards (07000011-13) carry weapons 0x10/0x1c/0x1d/0x1a, whose muzzle-flash meshes 0x02000261 /
    // 0x0200044a and the SMG's held mesh 0x0200025c are on NO archive from 07000011 on — the game
    // resolves them through the session table filled by the earlier story archives. The story order is
    // the case order of `ResetMap_LevelCode2Img` (0x1d5258): the level codes ascend in mission order
    // [docs/systems/levels.md §1], so the session before a level is exactly the archives with smaller
    // codes. The port merges every weapon-model hash the level archive misses from the latest earlier
    // story archive that holds it. Appending maps never shifts an existing (map, object) index, and
    // `by_hash` is first-wins, so every local resolution is unchanged.
    let lb = LevelBin::parse(&data)?;
    let mut skeletons: HashMap<u8, character::Skeleton> = lb.entries.iter().filter(|e| e.kind == 6)
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
    merge_weapon_models(&iso, &read, &archive[..8], &mut level)?;

    // The dev view models (folded 190-fix-dev2): each level archive ships only its own weapons'
    // first-person skins (+0xDC), so the dev panel's "no 1st-person model here" covers most weapons.
    // With `--dev` the missing FP skins (and their textures, skeletons, sequences and the weapons'
    // anim scripts) merge from their holding archives — the same session-table mechanism the
    // weapon-model merge mirrors — and `Scene::build` then gives every weapon a view model. Dev
    // mode only: normal play loads no extra archive for skins.
    // The level's own kind-5 anim scripts, parsed once and shared with the DroneContext below.
    let mut level_scripts: HashMap<u32, nf_formats::anim::Script> = lb.entries.iter().filter(|e| e.kind == 5)
        .filter_map(|e| nf_formats::anim::Script::parse(lb.data(&data, e)).ok()).map(|x| (x.hashcode, x)).collect();
    // The weapon-asset merge (fix-weapon-assets): `--dev` pulls every missing fp skin as before
    // (folded 190-fix-dev2); a normal load pulls exactly the `--give` / dev-menu weapons it is
    // asked for. The gameplay-skeleton guard keeps the appended 07F sequences off every skeleton
    // a drone or character placement animates (the per-skeleton lists are order-sensitive gameplay
    // data — the loadable-pack sentinel rule); the fp hands/gun skeletons never carry those.
    let gameplay_skeletons: std::collections::HashSet<u8> = if dev || !give.is_empty() {
        let mut gs = std::collections::HashSet::new();
        if let Some(m) = level.main_map() {
            for p in level.maps[m].placements.iter() {
                let Some(&(_, skin)) = p.params.iter().find(|(i, _)| *i == 0) else { continue };
                if skin >> 24 != 5 { continue; }
                if let Some(sk) = level.skins.get(&skin) { gs.insert(sk.skeleton); }
            }
        }
        gs
    } else { Default::default() };
    let weapon_wanted = if dev { weapon_assets::Wanted::All }
        else if !give.is_empty() { weapon_assets::Wanted::Ids(give.to_vec()) }
        else { weapon_assets::Wanted::None };
    let mut weapons_table: Option<nf_formats::weapon::WeaponData> = None;
    if weapon_wanted != weapon_assets::Wanted::None {
        // weapon_data once for the whole load (the EE initialiser run the merge needs is the same
        // one the DroneContext consumes below; `--dev` used to run it twice).
        let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF on the disc")?)?;
        weapons_table = match nf_formats::weapon::WeaponData::from_action_elf(&elf) {
            Ok(mut w) => { w.apply_player_init_override(u32::from_str_radix(&archive[..8], 16).unwrap_or(0)); Some(w) }
            Err(e) => { eprintln!("weapon_data: {e} — drones drawn unarmed"); None }
        };
        if let Some(wd) = weapons_table.as_ref() {
            let index = weapon_assets::DiscIndex::for_iso(&iso, iso_path);
            let t = std::time::Instant::now();
            let have_scripts: std::collections::HashSet<u32> = level_scripts.keys().copied().collect();
            let st = weapon_assets::merge_weapon_assets(&iso, &read, &mut level, &mut skeletons, &mut sequences,
                &mut level_scripts, &have_scripts, &gameplay_skeletons, wd, &weapon_wanted, &index, dev,
                &casing_meshes(&elf))?;
            eprintln!("weapon assets: merged {} archive(s), {} skin(s), {} map(s), {} sequence(s), {} script(s) in {:.2}s",
                st.archives, st.skins, st.maps, st.sequences, st.scripts, t.elapsed().as_secs_f32());
            // weapon-model-coverage: the honest final tally — the session mesh merge's per-stage
            // "still missing" count is before this merge ran. The residue is the set on NO archive
            // of the disc (the shipped game cannot resolve it either, e.g. the two casing meshes
            // 0x02000446/0x02000443 [D census]).
            let still: Vec<u32> = weapon_assets::meshes_for_wanted(&level, wd, &weapon_wanted, &casing_meshes(&elf))
                .into_iter().filter(|h| !level.by_hash.contains_key(h)).collect();
            if !still.is_empty() {
                eprintln!("weapon models: {} hash(es) unresolved after the merge, on no archive of the disc: {}",
                    still.len(), still.iter().map(|h| format!("{h:08x}")).collect::<Vec<_>>().join(" "));
            }
        }
    }
    // The weapon_data the DroneContext consumes: the hoisted build when the merge ran, else built
    // here (a normal load without --give never opens another archive).
    let weapons_table = match weapons_table {
        Some(w) => Some(w),
        None => match nf_formats::weapon::WeaponData::from_action_elf(&{
            iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF on the disc")?)?
        }) {
            Ok(mut w) => { w.apply_player_init_override(u32::from_str_radix(&archive[..8], 16).unwrap_or(0)); Some(w) }
            Err(e) => { eprintln!("weapon_data: {e} — drones drawn unarmed"); None }
        },
    };


    // G-021 (models-characters): drone placements whose skin (placement param 0) is not in their
    // level's archive — 33 of them on 2 levels [D scan]. TWO mechanisms, read off the decompile:
    //
    // 1. `AnimObjectNew` (0x1be910 disasm 0x1be958-0x1be9b0) SUBSTITUTES 0x05000090 before the
    //    hashtable get: on GameState+0x12 == 0x0700000c → 0x05000087 (0700000c's own skin), on
    //    0x07000014 → 0x05000093 (07000014's own). The port mirrors it at load: the placement hash
    //    0x05000090 resolves to the substitute skin's data — exactly the object the game builds.
    // 2. 07000014's 0x050000b1 (11 placements) has NO substitution (the disassembly covers only
    //    0x05000090) — the game's skin table accumulates every archive loaded in the session
    //    (`AnimLoadFile` 0x1bdf40 registers per map; nothing frees between levels — 0x1d8248 is
    //    `Script_Load`, the old mislabel), and 0x050000b1
    //    lives in 07000015/16's archives [D]. `CommonAnimFile` (the LoaderLoad type-4 name source,
    //    the variable at 0x30d860 [M 0x1d4a00 case 0: `param_2 = CommonAnimFile` — the only
    //    reference in ACTION.ELF, no writer]) is 0 at every level-start snapshot [L
    //    ee_start_05.bin / ee_start_07000001.bin / ee_start_0700000a.bin @0x30d860], so the common
    //    file is not the source here. The port merges the holding archive: its maps (the skin's
    //    meshes are glist objects of that archive), object hashcodes (first wins), skins and
    //    skeletons — and its 07F sequences ONLY for skeletons the level has no sequences for (a NEW
    //    bucket; the per-skeleton lists are order-sensitive gameplay data — the WORKLOG's warning).
    {
        let mm = level.main_map();
        // The substitution [M 0x1be910]: 0x05000090 → the level's own variant, before anything
        // else looks the hash up. The compared word is `GameState+0xC` @0x2a3774 (the current
        // level id; the "+0x12" in older notes was the decompiler's decimal offset 12).
        match u32::from_str_radix(&archive[..8], 16).unwrap_or(0) {
            0x0700_000c | 0x0700_0014 => {
                let level_id = u32::from_str_radix(&archive[..8], 16).unwrap_or(0);
                let sub = if level_id == 0x0700_000c { 0x0500_0087u32 } else { 0x0500_0093 };
                if let Some(mut sk) = level.skins.get(&sub).cloned() {
                    // The port keys its skin tables by the placement's hash (the sim's `dr.skin`,
                    // the draw caches, the 0x800 ambient tables): the substituted entry keeps the
                    // placement's identity while carrying the variant's data.
                    sk.hashcode = 0x0500_0090;
                    level.skins.entry(0x0500_0090).or_insert(sk);
                }
            }
            _ => {}
        }
        let mut wanted: Vec<u32> = Vec::new();
        if let Some(m) = mm {
            for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x0f) {
                if let Some(&v) = p.params.iter().find(|(k, _)| *k == 0).map(|(_, v)| v) {
                    if v != 0 && !level.skins.contains_key(&v) && !wanted.contains(&v) { wanted.push(v); }
                }
            }
        }
        // The cut-scene actors: every ScriptPlayer placement's script's `Anim` skins.
        if let Some(m) = mm {
            for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0xd9) {
                let Some(&v) = p.params.iter().find(|(k, _)| *k == 0).map(|(_, v)| v) else { continue };
                let Some(nis) = level.nis.get(&v) else { continue };
                for st in nis.streams.iter() {
                    for cmd in st.decode() {
                        if let nf_formats::nis::Cmd::Anim { skin, .. } = cmd {
                            if skin != 0 && !level.skins.contains_key(&skin) && !wanted.contains(&skin) { wanted.push(skin); }
                        }
                    }
                }
            }
        if !wanted.is_empty() {
            static SKIN_ARCHIVES: std::sync::OnceLock<HashMap<u32, String>> = std::sync::OnceLock::new();
            let index = SKIN_ARCHIVES.get_or_init(|| {
                let mut map = HashMap::new();
                for e in entries.iter().filter(|e| e.name.len() == 12 && e.name.ends_with(".bin")) {
                    let Ok(d) = iso.read_bytes(bin.lba as u64 * 2048 + e.offset as u64, e.size as u64) else { continue };
                    let Ok(l) = LevelBin::parse(&d) else { continue };
                    for pe in l.entries.iter().filter(|pe| pe.kind == 3) {
                        let raw = l.data(&d, pe);
                        if raw.len() < 4 { continue }
                        let h = u32::from_le_bytes(raw[0..4].try_into().unwrap());
                        if (h & 0xff00_0000) == 0x0500_0000 { map.entry(h).or_insert_with(|| e.name.clone()); }
                    }
                }
                map
            });
            let mut merged: Vec<String> = Vec::new();
            for want in wanted.iter() {
                let Some(archive_name) = index.get(want) else { eprintln!("g-021: skin {want:08x} is on no archive of the disc"); continue };
                if merged.contains(archive_name) { continue; }
                merged.push(archive_name.clone());
                let Some(fd) = read(archive_name)? else { continue };
                let mut foreign = Level::parse(&fd)?;
                let flb = LevelBin::parse(&fd)?;
                let map_base = level.maps.len();
                // `LevelMap` is not `Clone`: move the maps out of the parsed foreign level.
                level.maps.append(&mut foreign.maps);
                let foreign_by_hash = std::mem::take(&mut foreign.by_hash);
                for (h, (m, o)) in foreign_by_hash.iter() {
                    level.by_hash.entry(*h).or_insert((map_base + m, *o));
                }
                for (&h, sk) in foreign.skins.iter() {
                    level.skins.entry(h).or_insert_with(|| sk.clone());
                }
                for e in flb.entries.iter().filter(|e| e.kind == 6) {
                    let sk = character::parse_skeleton(flb.data(&fd, e))?;
                    if skeletons.contains_key(&(sk.id as u8)) { continue; }
                    if let Some(anim) = read(&format!("{}F{}", &archive_name[..2], &archive_name[3..]))? {
                        let alb = LevelBin::parse(&anim)?;
                        for pe in alb.entries.iter().filter(|pe| pe.kind == 4 && pe.length >= 0x40) {
                            let s = Sequence::parse(alb.data(&anim, pe))?;
                            if s.skeleton == sk.id as u8 { sequences.entry(s.skeleton | 0x80).or_default().push(s); }
                        }
                    }
                    skeletons.insert(sk.id as u8, sk);
                }
                eprintln!("g-021: merged {} for skins {:?} ({} maps, {} skins)", archive_name,
                    wanted.iter().filter(|w| level.skins.contains_key(w)).map(|w| format!("{w:08x}")).collect::<Vec<_>>(),
                    foreign.maps.len(), foreign.skins.len());
            }
        }
    }
    }
    // The sleeve substitution (fix-missing-drone-skins): `AnimProcessSkinData` 0x1bd500:159-166
    // reroutes every 0x10-flagged MAIN entry through `AnimSleeveGetEntity(0)` 0x1c6590 — the first
    // present entity of the fixed SleeveEnts list @0x2c6ed8 — before the celglist lookup; none
    // present → celglist 0 (nothing drawn). The port applies it once here, after the session
    // merges above, so every downstream reader (character_instances' mesh walk, the scene build,
    // the sim) sees exactly the hashes the game's runtime skin records carry.
    {
        let present = |h: u32| level.by_hash.contains_key(&h);
        for sk in level.skins.values_mut() { sk.substitute_sleeves(present); }
    }
    // The loadable cut-scene packs (fix-cutscenes): `SP_LoadScript` (0x1dbc08) checks `isLoadable(hash)`
    // (0x1d69a0 — the table filled by `LoaderProcess` 0x1d4880 case 0xC from the level archive's type-0xC
    // entries, one record per entry: u32 script hashcode at +0, u8 flag at +4) and, when it hits, streams the
    // pack file named with the CURRENT LEVEL's id and the (flag+7) nibble at bits 20-23 (`LoadableLoad`
    // 0x1d69e8: `GameState+0xC & 0xff0fffff | ((flag+7) << 20)`; `LoaderLoad` 0x1d4a00 case 0 sprintf's that
    // into "%08x.bin"; flag 1 → nibble 8, flag 2 → nibble 9, flag 3 → nibble A). Measured on the disc
    // [D]: 07000007's records (0x06000072, 2) and (0x06000073, 1) → 07900007.bin / 07800007.bin, which hold
    // exactly those type-7 scripts plus their kind-4 sequences; 07000008's (0x06000077, 1) / (0x06000078, 2)
    // → 07800008.bin / 07900008.bin likewise. These scripts exist nowhere else — the party's Drake fly-through
    // (0x06000072), the party's hand-back cut-scene (0x06000073) and the gondola level's intro (0x06000077)
    // are pack-only. Without the packs the port's ScriptPlayers hit the missing-script arm of `Player::load`
    // (state 5, deleted) on their first trigger and the cut-scene never plays (the user's playtest). The
    // pack's kind-7 entries go into `level.nis` (the same lib `Scripts::new` reads) and kind-4 into
    // `sequences`, mirroring `parsemap_parsemap` (LoaderProcess case 2 → the map parser walks the entries:
    // kind 7 → `Script_Load`, kinds 3/4/5 → `AnimLoadFile`).
    let level_id = u32::from_str_radix(&archive[..8], 16).unwrap_or(0);
    for e in lb.entries.iter().filter(|e| e.kind == 0xC) {
        let r = lb.data(&data, e);
        if r.len() < 5 { continue; }
        let (hash, flag) = (u32::from_le_bytes([r[0], r[1], r[2], r[3]]), r[4]);
        if flag == 0 || flag > 3 { continue; }
        let pack_name = format!("{:08x}.bin", (level_id & 0xff0f_ffff) | ((flag as u32 + 7) << 20));
        let Some(pack) = read(&pack_name)? else { eprintln!("loadable pack {pack_name} not on the disc (script {hash:08x})"); continue };
        let plb = LevelBin::parse(&pack)?;
        for pe in plb.entries.iter() {
            match pe.kind {
                7 => { let h = pe.hash(); match nf_formats::nis::Nis::parse(h, plb.data(&pack, pe)) { Ok(n) => { level.nis.insert(h, n); }, Err(err) => eprintln!("pack {pack_name} script {h:08x}: {err}") } }
                // Pack sequences go under `0x80 | skeleton` — a key no skin's skeleton ever uses. The game
                // DELETES the animation hashtable before streaming a pack (`LoadableLoad` 0x1d69e8's
                // `hashtable_delete_hashtype(0x4000000)`, then `AnimLoadFile` for the pack's kind-4 entries):
                // the cut-scene's sequences REPLACE the cache while it plays and never join the level's
                // per-skeleton sequence list. In the port that list is order-sensitive gameplay data —
                // `frame_of`'s `k % len` idle cycle and the skeleton-filtered `seq_index` drone lookups —
                // and appending to it changed guards' animations and broke live tests (07000001's
                // `dropped_guns_dim_to_the_cel_ambient` "the guard died"). The sentinel key still reaches
                // the by-hash map `Scene::seqs` (built by flattening every bucket), which is the only thing
                // a cut-scene's `Script_AnimStart` resolution reads (`script_frame`'s hash → `seqs.get`).
                4 if pe.length >= 0x40 => { let s = Sequence::parse(plb.data(&pack, pe))?; sequences.entry(s.skeleton | 0x80).or_default().push(s); }
                _ => {}
            }
        }
        eprintln!("loadable pack {pack_name}: {} scripts, {} sequences (for {hash:08x})", plb.entries.iter().filter(|e| e.kind == 7).count(), plb.entries.iter().filter(|e| e.kind == 4).count());
    }
    // Drone start animations: ACTION.ELF's tables (read through its symbols) and the level's scripts (type 5).
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF on the disc")?)?;
    let drones = scene::DroneContext {
        tables: nf_formats::drone::DroneAnimData::from_action_elf(&elf)?,
        // The level's own kind-5 scripts plus the weapon-asset merge's foreign view scripts (the
        // merged FP skins' idle/fire/reload) — the merge inserts missing hashes only, so a hash
        // the level has stayed local.
        scripts: level_scripts,
        level_id: u32::from_str_radix(&archive[..8], 16).unwrap_or(0),
        // weapon_data is built at boot by its static initialiser; run it over the user's ELF (docs/formats/weapon-data.md).
        props: nf_formats::drone::PropertyTables::from_action_elf(&elf).ok(),
        // Built once above (the merge consumes the same table); `Player_Init` (0x1910f8)'s override
        // of weapon_data[1]+0xDC (the fists skin) is already applied.
        weapons: weapons_table,
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
        // net-match-host: the online match builds the level from the host's seed (equal on every
        // machine — the session's settings blob carries it); 0 keeps the pre-net constant.
        mp_seed: mp_settings.as_ref().map_or(0, |st| st.seed),
    };
    let mut scene = scene::Scene::build(&level, !all, &skeletons, sequences, Some(&drones), difficulty);
    // The pause/fail menus' burning woman [M `psiDecompressWoman` 0x1202a8]: the archive's kind-16 "woman" run-length
    // mask filled from the fire texture 0x030000da, drawn into 0x030000db's texels through 0x030000da's CLUT (the
    // disc's 0x030000db is a placeholder). The HUD texture starts on the first decoded frame.
    {
        let tex = |h: u32| level.maps.iter().flat_map(|m| m.textures.iter()).find(|t| t.hashcode == h);
        if let (Some(e), Some(fire)) = (lb.entries.iter().find(|e| e.kind == 16), tex(0x0300_00da)) {
            let mut w = woman::Woman::new(lb.data(&data, e).to_vec(), fire.frame(0).to_vec(), fire.palette_rgba8());
            w.tick();
            if let Some((_, t)) = scene.hud_textures.iter_mut().find(|(h, _)| *h == 0x0300_00db) {
                if t.width as usize == woman::W && t.height as usize == woman::H { t.rgba = w.rgba(); }
            }
            scene.woman = Some(w);
        }
    }
    // The HUD's language strings (`fix-hud`): `Txt_LoadLanguage` 0x1caad0 loads `FNames[SelLang]` — the
    // NTSC-U default is `USATxt.dat` [D FILES.BIN; FNames 0x244680]. Parsed per its on-disc layout.
    if let Some(s) = scene.sim.as_mut() {
        match read("USATxt.dat") {
            Ok(Some(bytes)) => match nf_formats::hud::Lang::parse(&bytes) {
                Ok(lang) => s.lang = Some(lang),
                Err(e) => eprintln!("USATxt.dat: {e} — HUD labels/messages blank"),
            },
            Ok(None) => eprintln!("no USATxt.dat in FILES.BIN — HUD labels/messages blank"),
            Err(e) => eprintln!("USATxt.dat: {e}"),
        }
    }
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
    // fix-weapon-sounds: the spare banks — every other SBINFO slot with files on the disc, so a weapon
    // given on a level whose own bank lacks a sound still plays it from a bank that holds it. Only ids
    // the level's own bank lacks ever resolve to a spare (the driver's primary-first lookup), so every
    // level-carried sound is bit-identical to the one-bank behaviour [M `Sound_Ready` 0x1dceb8; S the
    // extension — the real game loads one bank per level].
    scene.sound_spares = (|| -> Vec<(usize, nf_formats::soundbank::Bank, Vec<u8>)> {
        let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
        let Some(sbi) = get("PS2/SBINFO.SBI") else { return Vec::new() };
        let Some(slots) = nf_formats::soundbank::bank_slots(&sbi).ok() else { return Vec::new() };
        let own = scene.sound_bank.as_ref().map(|(s, _, _)| *s);
        let mut spares = Vec::new();
        for (slot, &hash) in slots.iter().enumerate() {
            if hash == u32::MAX || Some(slot) == own { continue; }
            let f = |ext: &str| get(&nf_formats::soundbank::bank_path("ENGLISH", slot, ext));
            let (Some(shf), Some(sfx), Some(sbf)) = (f("SHF"), f("SFX"), f("SBF")) else { continue };
            match nf_formats::soundbank::Bank::parse(&shf, &sfx) {
                Ok(bank) => spares.push((slot, bank, sbf)),
                Err(e) => eprintln!("sound bank slot {slot}: {e}"),
            }
        }
        spares
    })();
    // Streamed effects (guards' speech): STREAMS.LUT / STREAMS.BIN.
    scene.streams = (|| {
        let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
        Some((nf_formats::soundbank::stream_lut(&get("PS2/ENGLISH/STREAMS/STREAMS.LUT")?).ok()?, get("PS2/ENGLISH/STREAMS/STREAMS.BIN")?))
    })();
    // Music (`Sound_Ready` → `Music_Event(0, level)`; docs/systems/music.md): the MapMusic / MapDroneData tables, and
    // the level's track through MFXINFO.MXI to PS2/MUSIC/MFX_<n>.{SMF,SSD}.
    scene.music_tables = nf_formats::music::tables(&elf).map_err(|e| eprintln!("music tables: {e}")).ok();
    // The music logic rides every load (the game's `Sound_Ready` runs before the first frame whether or
    // not an output device exists); the audio driver takes the tables later in `build_sound`.
    if let (Some(sim), Some((mm, vars))) = (scene.sim.as_mut(), scene.music_tables.as_ref()) {
        if sim.drones.music.table.is_empty() {
            wire_music(sim, mm.clone(), vars.clone());
        }
    }
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
    // ── Player_Start (0x190fd0) + Player_InitWeapon (0x1a7ef8) ─────────────────────────────────────
    // The game registers every type-0x24 placement as a player start in placement order (param 0 = the
    // gate switch channel, `Player_AddNewStartPos` 0x195c00), picks the first whose gate is 0 or on with
    // the channels as the previous level's exit left them (they survive the warm reset,
    // `Init_SwitchChannels` runs only when `IsWarmReset == 0` [M 0x1d5080]), then runs the per-level
    // loadout switch. The port used to stand Bond at the first "Player1" placement with the [L] 07000005
    // loadout at EVERY level (playtest F1/F2/F4).
    if let Some(sim) = scene.sim.as_mut() {
        // The mission globals ride the RamSave (the game keeps them in `.data` across the load; the
        // port restores them at the `Mission_Init` latch — map-tower2).
        sim.drones.mission_carry = save.and_then(|s| s.mission.clone());
        let starts: Vec<nf_game::player::LevelStart> = level_starts(&level);
        if !starts.is_empty() {
            // The gate scan reads the carried channels on a warm transition (the save mirrors the live
            // globals — `Player_RamSave` snapshots them at the exit [M 0x195670:13-15]).
            let scan: nf_game::triggers::Channels = match save {
                Some(s) => nf_game::triggers::Channels { v: s.channels.clone(), time: s.ch_time.clone() },
                None => nf_game::triggers::Channels::default(),
            };
            if let Some(i) = nf_game::player::pick_start(&starts, &scan) {
                let s = starts[i];
                if let Some((old, _)) = scene.start { eprintln!("start: default {:?} → start {} gate {:#x} at {:?}", old, i, s.gate, s.pos); }
                // `Player_Init(0, pos, rot, params)` creates the player object at the entry; the port
                // stands him on the floor below (the level start convention, `Bond::stand_at`).
                sim.bond = nf_game::player::Bond::stand_at(&sim.world, s.pos, s.yaw);
                // `Player_Init` 0x1910f8:152-172 [M]: the default WldGravity write (0, −9.8, 0),
                // then p3 == 2 → `Vec_Zero(&WldGravity)` (the spacewalk starts; disasm 0x19151c-
                // 0x191550 the default, 0x191618 the zero). fix-world-gravity.
                sim.bond.world_gravity = if s.p3 == 2 { [0.0; 3] } else { [0.0, -9.8, 0.0] };
                // p3 == 3 → `Player_StandAtNewPosition(.., 4)` (the crouch substate) and
                // `BLData+0x90C = 0.45` (0x3ee66666, the full crouch eye drop) [M Player_Init 0x1910f8
                // :172-176]. 07000014 starts this way inside its crawl vent [L: PCSX2 substate 4,
                // BL+0x90C 0.45 at the start]; standing there put the eye above the duct roof.
                if s.p3 == 3 {
                    sim.bond.crouched = true;
                    sim.bond.cc = nf_game::player::CROUCH_ROOT_HEIGHT;
                    sim.bond.crouch_drop = 0.45;
                }
                scene.start = Some((s.pos, s.yaw));
            }
        } else if let Some((pos, yaw)) = scene.start {
            // No type-0x24 starts parsed: keep the "Player1" stand-in.
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, pos, yaw);
        }
        // mp-core: the MP arenas have no type-0x24 starts — `--mp` runs the match core instead:
        // the 0x25/0x26 registrations (`parsemap_create_dynamic_objects` 0x1d04f0 case 0x25/0x26 —
        // the team and the MP object type are the placement's param 0, the block+0x2c slot [M
        // 0x1d1b68]), `MP_Init` 0x183438 / `MP_Start` 0x183a50, then the human spawn at the policy's
        // point (the `Player_Init` 0x1910f8 port — the yaw is the placement rotation[1], the same
        // convention `level_starts` pins).
        if mp {
            // net-match-host: the online path hands the negotiated settings here (the host's
            // choices + seed); local `--mp` keeps the deathmatch default.
            let settings = mp_settings.unwrap_or_else(|| nf_game::mp::MpSettings::deathmatch(level_id));
            let mut m = nf_game::mp::MpMatch::init(settings);
            let map = level.main_map().and_then(|mi| level.maps.get(mi));
            let mut spawn_count = 0usize;
            if let Some(map) = map {
                let param = |p: &nf_formats::placement::Placement, k: u32| -> u32 {
                    p.params.iter().find(|(i, _)| *i == k).map_or(0, |&(_, v)| v)
                };
                for p in &map.placements {
                    match p.flags & 0xff {
                        0x25 => {
                            let team = param(p, 0) as u16;
                            let before = m.spawns.count;
                            m.register_spawn_point(p.translation, p.rotation, team, &sim.world);
                            if m.spawns.count > before { spawn_count += 1; }
                        }
                        0x26 => {
                            let ty = param(p, 0);
                            let cfg: [u32; 7] = std::array::from_fn(|k| param(p, k as u32));
                            m.register_mp_object(p.translation, p.rotation, ty, cfg, p.hashcode);
                        }
                        _ => {}
                    }
                }
            }
            m.start(&mut sim.drones.rng);
            // Every human's pick: `MP_Start` calls `MP_GetSpawnPoint(team, NULL)` BEFORE
            // `Player_Init` creates the obj [M 0x183a50:120-123] — no slots exist yet, so every used
            // point is a 9999-distance candidate (policy-dependent pick, empty slots). net-match-host:
            // ALL human slots spawn here in slot order with the SAME draws on every machine, so the
            // online match's bodies agree; slot 0's bond takes `sim.bond` (the pre-net behaviour) and
            // the rest park in `Sim::mp_bonds` until each machine swaps its own slot in
            // (`Sim::swap_local_slot`).
            let humans = (m.settings.human_count as usize).clamp(1, 4);
            let mut first_spawn = 0usize;
            for slot in 0..humans {
                let sp = m.get_spawn_point(m.settings.players[slot].team as u16, slot, &mut sim.drones.rng) as usize;
                let sp_ent = m.spawns.points[sp];
                let b = nf_game::player::Bond::stand_at(&sim.world, [sp_ent.pos[0], sp_ent.pos[1], sp_ent.pos[2]], sp_ent.facing[1]);
                if slot == 0 {
                    first_spawn = sp;
                    sim.bond = b;
                    m.set_slot(0, sim.bond.pos);
                } else {
                    m.set_slot(slot, b.pos);
                    sim.mp_bonds[slot] = Some(b);
                }
            }
            let sp = first_spawn;
            sim.mp = Some(m);
            eprintln!("mp: arena {level_id:08x} — {spawn_count} spawn points registered, {humans} human(s), Bond at spawn {sp} ({:.2}, {:.2}, {:.2}) yaw {:.3}",
                sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2], sim.bond.yaw);
        }
        // `Player_CheckWeaponsLoaded`'s model check (0x1a97c8): the hashtable holds the level's loaded
        // skins; the port's equivalent is the level skin table [M — the fp model hash is a skin hash].
        let loaded: Vec<bool> = (0..0x72u32)
            .map(|w| sim.skins.contains_key(&sim.weapons.first_person(w as usize))).collect();
        let loaded = |w: u8| loaded.get(w as usize).copied().unwrap_or(false);
        let req = nf_game::player::init_level_loadout(level_id, save, &mut sim.drones.health,
            &mut sim.drones.inventory, &mut sim.drones.channels, &sim.weapons, loaded);
        // The switch request is the game's `M_CONTROL+0x63`; the draw machine holsters and draws it
        // (`Player_WeaponSelect` 0x1a2cc0). 0x47 (none) draws nothing.
        sim.gun.switch_request = req;
        eprintln!("loadout: level {level_id:08x} switch request {req} ({} owned weapons, health {:.0}, armour {:.0})",
            sim.drones.inventory.owned.iter().filter(|&&o| o).count(), sim.drones.health.health, sim.drones.health.armour);
    }
    Ok(scene)
}

/// The level's player starts: the main map's type-0x24 placements in placement order, with the gate
/// (parameter 0) [M `parsemap_create_dynamic_objects` 0x1d04f0 case 0x24 → `Player_AddNewStartPos`
/// 0x195c00 → `Player_Start` 0x190fd0]. Yaw = the placement's rotation[1], the same convention the
/// The story order (`ResetMap_LevelCode2Img` 0x1d5258's case order — docs/systems/levels.md §1: the
/// level codes ascend in mission order), the session the game accumulates before each level.
const STORY_ORDER: [u32; 20] = [0x0700_0001, 0x0700_0002, 0x0700_0003, 0x0700_0004, 0x0700_0005, 0x0700_0006,
    0x0700_0007, 0x0700_0008, 0x0700_0009, 0x0700_000a, 0x0700_000b, 0x0700_000c, 0x0700_000d, 0x0700_0011,
    0x0700_0012, 0x0700_0013, 0x0700_0014, 0x0700_0015, 0x0700_0016, 0x0700_001b];

/// The view-model sleeve meshes (`Scene::build`'s SLEEVES — `AnimSleeveGetEntity(0)`'s per-level set).
const SLEEVES: [u32; 8] = [0x0200_09d0, 0x0200_0900, 0x0200_01f7, 0x0200_0929, 0x0200_09e6, 0x0200_09ff, 0x0200_0a14, 0x0200_0a13];

/// The weapon-model hashes the level archive misses and the session can supply (`merge_weapon_models`).
/// fix-weapon-fx-levels adds two FX sets to the list:
/// - the first-person muzzle-flash mesh `+0x48` (`Player_MuzzleFlash` 0x1a4298 gates on `+0x48 != 0`,
///   `Draw_MuzzleFlash` 0x1a3fe0 attaches it to the view model [M]; `Scene::build` looks it up in
///   `by_hash`, scene.rs) — the disc census [D]: every record's `+0x48` resolves in every story
///   archive EXCEPT 0x02000ac7 (records 0x32/0x33/0x6f), which lives only in 07000014/15/16/1b;
/// - the shell-casing meshes (`ammo_data` entry +4, `Bullet_init_casing` 0x12fde0 attaches
///   `ammo_data[weapon+0x90]+4` [M]; `Casings::eject` reads the parsed table) — ammo 9's default
///   shell 0x02000548 is absent from 07000005-0d/14-16/1b [D census], so the shotgun's casings
///   simulated there were never drawn.
/// The game never hits either gap (a level only offers its own weapons — 0x32/0x33 ride
/// 07000014-16/1b, the shotgun rides 07000011-13/MP, whose archives hold their meshes); the dev
/// give path did, the same way 100a's skins did.
fn wanted_weapon_models(level: &Level, wd: &nf_formats::weapon::WeaponData, casing_meshes: &[u32]) -> Vec<u32> {
    let mut needed: Vec<u32> = Vec::new();
    let mut want = |h: u32, needed: &mut Vec<u32>| {
        if h != 0 && h != u32::MAX && !level.by_hash.contains_key(&h) && !needed.contains(&h) { needed.push(h); }
    };
    for r in &wd.records {
        // fix-silencer-visual: the view-entity mesh (+0x7C, `AnimDatumSetEntity(vm, 0, weapon+0x7C,
        // 0xff)` per tick on the 0x800 records — `Player_WeaponFiring` 0x1a5460:96-110) rides the
        // give/merge path too, or a foreign-level give of a silenced twin misses the tube.
        for off in [0x80usize, 0x4c, 0x48, 0x5c, 0x7c] {
            want(u32::from_le_bytes(r[off..off + 4].try_into().unwrap()), &mut needed);
        }
    }
    for h in SLEEVES { want(h, &mut needed); }
    for &h in casing_meshes { want(h, &mut needed); }
    needed
}

/// The shell-casing meshes (`AmmoData.casing_mesh`, ACTION.ELF `ammo_data`+4 per 0xc-byte record;
/// `Bullet_init_casing` 0x12fde0's mesh argument [M]). Empty when the ELF lacks the symbol.
fn casing_meshes(elf: &[u8]) -> Vec<u32> {
    nf_formats::sfx::AmmoData::from_action_elf(elf).map(|a| a.casing_mesh).unwrap_or_default()
}

/// Moves the foreign maps that hold any of `hashes` into `level` (appended; `by_hash` first-wins, so a
/// hash the level resolves locally keeps its (map, object)). Returns how many maps moved.
fn take_maps_holding(level: &mut Level, foreign: Level, hashes: &[u32]) -> usize {
    let mut take: std::collections::HashSet<usize> = Default::default();
    for (mi, m) in foreign.maps.iter().enumerate() {
        if m.objects.iter().any(|o| hashes.contains(&o.hashcode)) { take.insert(mi); }
    }
    if take.is_empty() { return 0; }
    let base = level.maps.len();
    let mut moved = 0;
    for (mi, m) in foreign.maps.into_iter().enumerate() {
        if !take.contains(&mi) { continue; }
        let idx = base + moved;
        for (oi, o) in m.objects.iter().enumerate() {
            level.by_hash.entry(o.hashcode).or_insert((idx, oi));
        }
        level.maps.push(m);
        moved += 1;
    }
    moved
}

/// The session object table (`models-weapons-items`, see the caller's comment): every weapon-model mesh
/// (`weapon_data` +0x80 held guns, +0x4C muzzle flashes, +0x5C projectile bodies, and the view-model
/// sleeves) the level archive misses is merged from the latest earlier story archive that holds it.
pub fn merge_weapon_models<R: std::io::Read + std::io::Seek>(iso: &IsoImage<R>,
    read: &impl Fn(&str) -> Result<Option<Vec<u8>>>, prefix: &str, level: &mut Level) -> Result<()> {
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF on the disc")?)?;
    let Ok(wd) = nf_formats::weapon::WeaponData::from_action_elf(&elf) else { return Ok(()) };
    let level_id = u32::from_str_radix(prefix, 16).unwrap_or(0);
    let mut needed = wanted_weapon_models(level, &wd, &casing_meshes(&elf));
    if needed.is_empty() { return Ok(()); }
    for code in STORY_ORDER.iter().rev().filter(|&&c| c < level_id) {
        if needed.is_empty() { break; }
        let name = format!("{code:08x}.bin");
        let fd = match read(&name) { Ok(Some(d)) => d, _ => continue };
        let Ok(foreign) = Level::parse(&fd) else { continue };
        let moved = take_maps_holding(level, foreign, &needed);
        if moved > 0 {
            needed.retain(|h| !level.by_hash.contains_key(h));
            eprintln!("session mesh merge: {moved} map(s) from {name} ({} weapon-model hashes still missing)", needed.len());
        }
    }
    Ok(())
}

/// 0x195c00 → `Player_Start` 0x190fd0]. Yaw = the placement's rotation[1], the same convention the
/// "Player1" stand-in uses (scene.rs).
fn level_starts(level: &Level) -> Vec<nf_game::player::LevelStart> {
    let Some(m) = level.main_map() else { return Vec::new() };
    level.maps[m].placements.iter()
        .filter(|p| p.flags & 0xff == 0x24)
        .map(|p| nf_game::player::LevelStart {
            pos: p.translation,
            yaw: p.rotation[1],
            gate: p.params.iter().find(|(i, _)| *i == 0).map_or(0, |&(_, v)| v as u16),
            // The zero-g selector (level_tag+0x38 = placement param 3) [M Player_Init 0x1910f8].
            p3: p.params.iter().find(|(i, _)| *i == 3).map_or(0, |&(_, v)| v),
        })
        .collect()
}

/// The level archive for a hashcode, `LoaderLoad` (0x1d4a00) style [M]: `sprintf("%s%8.8x.bin", …,
/// code & 0xff0fffff | …)` — the loader type's nibble (0x00F00000) is masked OFF for the level itself
/// (loader type 1; type 0/3 force 0xF00000 for the anim archives). `ResetMap_LevelToLoad` (0x1d3800)
/// stores e.g. 0x07100006 for a `LOAD_LEVEL` to 0x07000006 (param_3 == 0 remap), and the level load
/// then masks it back to 07000006.bin.
pub fn level_archive_name(code: u32) -> String { format!("{:08x}.bin", code & 0xff0f_ffff) }

/// `NF_FPSLOG` (the port's own measuring aid): accumulates whole-frame times and prints, once a
/// second, the frame count, the average and the worst frame in ms.
fn fps_log(frame: std::time::Duration) {
    use std::sync::Mutex;
    static ACC: Mutex<Option<(std::time::Instant, u32, f64, f64)>> = Mutex::new(None);
    let Ok(mut acc) = ACC.lock() else { return };
    let ms = frame.as_secs_f64() * 1000.0;
    let a = acc.get_or_insert((std::time::Instant::now(), 0, 0.0, 0.0));
    a.1 += 1; a.2 += ms; a.3 = a.3.max(ms);
    if a.0.elapsed().as_secs_f64() >= 1.0 {
        eprintln!("[fps] {} frames, avg {:.2} ms, worst {:.2} ms", a.1, a.2 / a.1 as f64, a.3);
        *acc = None;
    }
}

/// Reads one FILES.BIN entry by name (fix-menus: the front end's USATxt.dat when the map has no Sim).
fn read_files_bin(iso_path: &str, name: &str) -> Option<Vec<u8>> {
    let iso = IsoImage::open(iso_path).ok()?;
    let (entries, bin) = disc::files_bin_entries(&iso).ok()?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
    iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()
}

/// frontend-music: the credits music's files — `SFXStartMusic(0x27, 0)` [M P_CREDITS 0x2152b0:94]
/// plays MFX track 0x27 through the same MFXINFO.MXI → MFX_<n>.{SMF,SSD} path the level music
/// uses [M docs/systems/music.md; the front end has no MapMusic entry — the loader runs from the
/// command, not from `Sound_Ready`'s table].
fn menu_music_files(iso_path: &str, track: i32) -> Option<(i32, nf_formats::music::Smf, std::sync::Arc<Vec<u8>>)> {
    let iso = IsoImage::open(iso_path).ok()?;
    let get = |p: &str| iso.find(p).and_then(|e| iso.read_file(e).ok());
    let n = nf_formats::music::track_file(&get("PS2/MFXINFO.MXI")?, track).ok()??;
    let smf = nf_formats::music::Smf::parse(&get(&nf_formats::music::music_path(n, "SMF"))?).ok()?;
    Some((track, smf, std::sync::Arc::new(get(&nf_formats::music::music_path(n, "SSD"))?)))
}

/// The keyboard stand-ins for the game's cut-scene skip action (`Input_Action(-1, 0x19, 1)`, read by
/// `SP_Update` state 2 [M 0x1dbda8:129-141]). Action 0x19 is bound to raw pad mask 0x840 — R1 (0x800)
/// or D-pad DOWN (0x040), pad 0 only [M psiInput_MapInputs 0x10cd80:415-417] — so a movement key
/// (W/A/S/D, Space, the mouse) NEVER skips a scene; the user's W press skipped the 07000007 Kimiko
/// scene (2026-10-12 playtest). The port maps that action to ArrowDown (D-pad down) and R (R1). The
/// viewer's dedicated keys (Esc, F1/F2/F11/F12) keep their own functions; `--skip-cutscenes` is the
/// dev override and bypasses this.
pub fn cutscene_skip_key(k: winit::keyboard::KeyCode) -> bool {
    matches!(k, winit::keyboard::KeyCode::ArrowDown | winit::keyboard::KeyCode::KeyR)
}

/// The dev menu's type-to-filter lane: a physical key → the ASCII char it types into the filter
/// (lowercase; the HUD font has only ASCII glyphs anyway, `dev::fit`).
fn dev_filter_char(k: winit::keyboard::KeyCode) -> Option<char> {
    use winit::keyboard::KeyCode as K;
    Some(match k {
        K::KeyA => 'a', K::KeyB => 'b', K::KeyC => 'c', K::KeyD => 'd', K::KeyE => 'e', K::KeyF => 'f',
        K::KeyG => 'g', K::KeyH => 'h', K::KeyI => 'i', K::KeyJ => 'j', K::KeyK => 'k', K::KeyL => 'l',
        K::KeyM => 'm', K::KeyN => 'n', K::KeyO => 'o', K::KeyP => 'p', K::KeyQ => 'q', K::KeyR => 'r',
        K::KeyS => 's', K::KeyT => 't', K::KeyU => 'u', K::KeyV => 'v', K::KeyW => 'w', K::KeyX => 'x',
        K::KeyY => 'y', K::KeyZ => 'z',
        K::Digit1 => '1', K::Digit2 => '2', K::Digit3 => '3', K::Digit4 => '4', K::Digit5 => '5',
        K::Digit6 => '6', K::Digit7 => '7', K::Digit8 => '8', K::Digit9 => '9', K::Digit0 => '0',
        K::Space => ' ', K::Minus => '-', K::Period => '.', K::Comma => ',',
        _ => return None,
    })
}

/// The Character-layer vertices posed after `time` 60 Hz ticks: start animations (`None`) or sequence `choice`.
/// (`Scene::posed_vertices` also applies the per-character ambient multiplier — the 0x800 TWEAK dim.)
fn posed_vertices(scene: &scene::Scene, choice: Option<usize>, time: f32, opts: scene::ViewOpts) -> Vec<scene::Vertex> {
    scene.posed_vertices(choice, time, opts)
}

/// The snow quads facing the camera, plus the emitter particles (fix-world-fx: the same billboard
/// pass — `Emitter_Draw` 0x17b708 path A draws both through `psiDrawParticleList`).
fn snow_for(scene: &scene::Scene, cam: &Camera) -> Vec<scene::Vertex> {
    let Some(sim) = scene.sim.as_ref() else { return Vec::new() };
    let f = cam.dir();
    let right = cam.right();
    let up = right.cross(f).normalize_or_zero();
    let mut out = sim.snow_quads(right.to_array(), up.to_array());
    out.extend(sim.particle_quads(right.to_array(), up.to_array()));
    out
}

/// One frame of sound (`Sound_UpdateListeners` → `Sound_UpdateSounds` → the batch → `SFXUpdate`): the listener is
/// Bond's eye and view (glb_viewer[0]; the cut-scene camera's listener call is dropped by the EE batcher [M]).
/// `pub` so the headless tests drive the exact function the winit loop runs (the fix-gpu-skinning precedent).
pub fn sound_tick(s: &mut nf_audio::sfx::Sound, sim: &mut scene::Sim) {
    let (pos, f) = (sim.bond.head(), sim.bond.look());
    // Row 0 of the view matrix: up × forward, the screen-left axis (snapshot: dir (−0.839, 0, −0.545) → (−0.545, 0, 0.839)).
    let norm = { let l = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6); [f[2] / l, 0.0, -f[0] / l] };
    sim.drones.map_sounds.update(&mut sim.drones.audio, pos);
    // `UpdateMusicalEvents` moved into the sim's tick (it runs every frame with or without an output
    // device [M 0x114e70]); the IOP's now-playing section feeds it through `Drones::iop_section`.
    sim.drones.iop_section = s.music.as_ref().map_or(0, |p| p.now_playing);
    let mut fade_down = None;
    for c in std::mem::take(&mut sim.drones.music.out) {
        match c {
            nf_game::music::Cmd::Start { track, section } => s.music_pending.push((0, track, section)),
            nf_game::music::Cmd::Jump { section, instant } => s.music_pending.push((1, section, instant as i32)),
            nf_game::music::Cmd::FadeDown { music } => fade_down = Some(music),
        }
    }
    // The IOP's op-0x19 pair each frame (`SFXUpdate` emits it when it changed): `NisPlaying` pauses the
    // `PauseInNis` sounds (`ES_HandleNisPauseSfx`), `NisQuitten` stops the ducker [M sound.md §6 step 3, §8].
    let (nis_playing, nis_quitten) = sim.drones.music.nis_state();
    s.driver.set_nis_playing(nis_playing);
    s.driver.set_nis_quitten(nis_quitten);
    let driver = &s.driver;
    sim.drones.audio.update(sim.sfx.as_ref(), (pos, norm, [0.0, 1.0, 0.0], f), &|h| driver.is_playing(h));
    let cmds = sim.drones.audio.take_commands();
    // `Sound_UpdateListeners` 0x1dd278 (fix-room-sound): every frame, from the active viewer's cel —
    // `SFXSetEnvironment(cel s16 +0x98, underwater = (cel+0x3c & 0x10000000) || env >= 20)` [M disasm
    // 0x1dd4b0-0x1dd4fc]; a viewer with no cel sends nothing and the IOP keeps its previous targets.
    // The port's single viewer is Bond's eye (as below); the glb_viewer[4] script-camera override is
    // not modelled — the script-camera listener call is dropped by the EE batcher.
    let env_send = sim.cel_at(pos).map(|i| {
        let c = &sim.cels[i];
        (c.env as i32, (c.underwater || c.env >= 20) as i32)
    });
    s.batch(|drv, out| {
        // `Sound_UpdateListeners`→`SFXSetEnvironment(room env, underwater)` [M sound.md §0, 0x1dd278].
        if let Some((reverb, underwater)) = env_send { drv.set_environment(reverb, underwater); }
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

/// The screen-space HUD overlay. The game hides the whole sprite pass while a script camera runs:
/// `Script_CameraStart` (0x1d9140) → `Camera_Enable` (0x1b9920) clears `viewer+0x2a` (the sprite-draw flag)
/// for every gameplay viewer, and `Game_Draw` (0x1e6f50) skips `Sprite_BuildList`/`View_DrawSprites` when it
/// is clear — health iris, ammo, crosshair, radar, messages and the damage glow (`psiFadeView`, in the same
/// gated block) all disappear together. Snapshot-verified: the 07000001 opening cut-scene has viewer[0]/[5]
/// +0x2a = 0 while `ScriptCam`@0x30d100 is non-zero [L ee_start_07000001.bin]. The port's condition is the
/// one it already drives the cut-scene camera from: a live script camera object.
pub fn overlay_for(scene: &scene::Scene, aspect: f32) -> gpu::HudFrame {
    let cutscene = scene.sim.as_ref().is_some_and(|s| s.script_camera().is_some());
    if cutscene { gpu::HudFrame::default() } else { hud::overlay(scene, aspect) }
}

/// The cut-scene frame (fix-cutscene-letterbox): while a script camera runs the port shows the
/// game's own picture — the projection narrowed to the console's 4:3 aspect and the render
/// confined to the band that fits the window (pillar-boxed black on a wider window); the game's
/// letterbox bars ride INSIDE the band (`hud::script_overlay`, `gpu::render`'s viewport). The
/// game rendered every cut-scene at the console's 4:3 frame — the port's widescreen projection
/// showed more at the sides than the game's frame (the main agent's PCSX2 compare, frame 400:
/// Dominique waiting in the left doorway visible in the port, outside the game's frame).
/// Gameplay keeps the window's full widescreen. Returns (projection aspect, the band [x, y, w, h]
/// in target pixels).
pub fn cutscene_frame(win_w: f32, win_h: f32) -> (f32, [f32; 4]) {
    const GAME: f32 = 4.0 / 3.0;
    let (bw, bh) = if win_w / win_h > GAME { (win_h * GAME, win_h) } else { (win_w, win_w / GAME) };
    (GAME, [(win_w - bw) * 0.5, (win_h - bh) * 0.5, bw, bh])
}

/// The HUD frame the headless `--shot` render draws (fix-cutscene-letterbox): the gameplay
/// overlay (`overlay_for` — empty while a script camera runs) PLUS the script player's overlay —
/// the fade, the letterbox bars and the op-0x14 sprites (`Scene::script_overlay_hud`). The shot
/// path used to draw `overlay_for` alone, so the Kimiko-scene compare frames showed no letterbox
/// bars; the window path always appended the script overlay (App redraw) and now both do.
pub fn shot_hud(scene: &scene::Scene, aspect: f32, menu: Option<&gpu::HudFrame>) -> gpu::HudFrame {
    let mut f = menu.map_or_else(|| overlay_for(scene, aspect), |m| m.clone());
    f.append(&scene.script_overlay_hud(aspect));
    f
}

/// The camera's screen axes for the corona billboards (same basis as the snow quads).
fn cam_axes(cam: &Camera) -> ([f32; 3], [f32; 3]) {
    let f = cam.dir();
    let right = cam.right();
    let up = right.cross(f).normalize_or_zero();
    (right.to_array(), up.to_array())
}

/// fix-menus2: the front-end overlay for `--shot`. On the front-end boot the menu overlay replaces the
/// gameplay HUD (the task: the shot path must draw the menus). `--page <start|main|codename|
/// difficulty|missions|dossier>` jumps to a page for the per-page renders, `--cursor N` picks the row,
/// `--unlock-all` is the `menu_unlock_everything` dev global [M 0x30d2a7, 0x20b8d8:18]. The page is
/// drawn with the game's font (ACTION.ELF's FONTSerpLight) and the USATxt.dat strings.
/// fix-menu-movies2: returns `(hud, movie)` — with the `movies` feature and an image that has movie
/// data, the page's backdrop is its own looping FMV instead of the dark plate, decoded as a still
/// (closed-GOP chunks in order until the frame is covered — fast; `--movie-frame N` picks the
/// frame) and drawn on the ONE
/// movie plane (gpu.rs), exactly like the live loop. `--intro [N]` renders the P_INTRO movie the
/// same way, fullscreen with no overlay (the intro page has no controls [M 0x209e88]).
fn front_end_menu(args: &[String], scene: &scene::Scene, front_end: bool, iso_path: &str) -> (Option<gpu::HudFrame>, Option<(u32, u32, Vec<u8>)>) {
    let page_arg = args.iter().position(|a| a == "--page");
    if !front_end && page_arg.is_none() { return (None, None); }
    let Some(tables) = scene.hud.as_ref() else { eprintln!("[front end] no HUD tables (the game font) — the menu overlay needs ACTION.ELF"); return (None, None); };
    let mut fe = menu::FrontEnd::new();
    if args.iter().any(|a| a == "--unlock-all") { fe.unlock_all = true; }
    if let Some(i) = args.iter().position(|a| a == "--cursor") {
        fe.cursor = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(0);
    }
    if let Some(i) = page_arg {
        let name = args.get(i + 1).map(String::as_str).unwrap_or("start");
        fe.page = match name {
            "start" | "title" => menu::Page::Start,
            "main" => menu::Page::Main,
            "codename" => menu::Page::Codename,
            "difficulty" => menu::Page::Difficulty,
            "missions" | "single" => menu::Page::MissionSelect,
            "dossier" => menu::Page::Dossier,
            // fix-menus-real (additive hook): the dossier's Records table renders headless too.
            "records" => menu::Page::Records,
            // dossier-pages-data (additive hooks): the Dossier wheel's rows 1-3 render headless too.
            "rewards" => menu::Page::Rewards,
            "gadgets" => menu::Page::Gadgets,
            "weapons" => menu::Page::Weapons,
            "cnselect" => menu::Page::CnSelect,
            "cnmenu" => { fe.cn_name = "Default".into(); menu::Page::CnMenu }
            "advanced" => menu::Page::CnAdvanced,
            "mpoptions" => menu::Page::CnMp,
            "avoptions" => menu::Page::CnAv,
            // codename-entry: the name-entry grid renders headless too (cn_open seeds the proposal).
            "cnname" => { fe.cn_open(menu::PAGE_CN_SELECT); menu::Page::CnName }
            "cncheat" => { fe.cn_open(menu::PAGE_CN_MENU); menu::Page::CnName }
            "mpscenario" => menu::Page::MpScenario,
            "mpmap" => menu::Page::MpMap,
            "mpjoin" => menu::Page::MpJoin,
            "mpscenopts" => menu::Page::MpOptions,
            "mpbots" => menu::Page::MpBots,
            "mpbotchoose" => menu::Page::MpBotChoose,
            "mprules" => menu::Page::MpRules,
            "mpplayermods" => menu::Page::MpPlayerMods,
            "mpenviromods" => menu::Page::MpEnviroMods,
            "mpbotsetup" => menu::Page::MpBotSetup,
            // fix-pause-menu: the in-game pages render headless too (the pause page over the
            // paused frame, the results per outcome). `--cursor N` picks the row, `--pause-tab N`
            // picks the pause tab, `--confirm` arms the pause confirm.
            "pause" => menu::Page::Pause,
            "results" | "failed" => { fe.outcome = menu::Outcome::Failed; menu::Page::Results }
            "complete" => { fe.outcome = menu::Outcome::Complete; menu::Page::Results }
            other => { eprintln!("unknown --page '{other}' (start|main|codename|difficulty|missions|dossier|pause|results|complete)"); return (None, None); }
        };
    }
    // The MP wheels keep their own row counters: `--cursor N` picks the row there too.
    if fe.page == menu::Page::MpScenario { fe.mp_scenario_row = fe.cursor; }
    if fe.page == menu::Page::MpMap { fe.mp_map_row = fe.cursor; }
    if fe.page == menu::Page::MpOptions { fe.mp_options_row = fe.cursor; }
    if fe.page == menu::Page::MpBots { fe.mp_bots_row = fe.cursor; }
    if fe.page == menu::Page::MpBotChoose { fe.mp_bot_char = fe.cursor; }
    // Join Game: `--cursor` is agent 1's panel state (0 press to join, 1 choose codename, 5 joined).
    if fe.page == menu::Page::MpJoin { fe.mp_join[0].state = fe.cursor as u32; fe.mp_join[0].joined = fe.cursor > 0; fe.cursor = 0; }
    if let Some(i) = args.iter().position(|a| a == "--pause-tab") {
        fe.pause_tab = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(0);
    }
    if args.iter().any(|a| a == "--confirm") {
        fe.confirming_quit = fe.cursor == 2;
        fe.confirming_restart = fe.cursor == 1;
        fe.confirming_default_profile = fe.page == menu::Page::Codename;
    }
    // The pause page is a MISSION page: give the flow a mission level so open_pause's guard matches
    // (the game opens it over a running mission [M Player_Update 0x193600:73]).
    if fe.page == menu::Page::Pause && fe.flow.current_level == 0 {
        fe.flow.current_level = 0x0700_0005;
        fe.mission = 1;
        fe.open_pause();
    }
    if fe.page == menu::Page::Pause {
        if let Some(sim) = scene.sim.as_ref() { fe.pause_objectives = pause_objective_rows(sim); }
        // objective-hints: pre-open the ✕ hint box for the render (the row the cursor stands on) —
        // the same state `FrontEnd::action`'s Confirm sets on the OBJECTIVES tab [M 0x204408
        // 0x205b98-0x205c34: control 1's activation reads control 8's row and opens the ok-only
        // box; 0xffffffff → 0x31d]. After the fill above (the rows must exist).
        if args.iter().any(|a| a == "--pause-hint") && fe.pause_tab == 1 && !fe.pause_objectives.is_empty() {
            let sel = fe.pause_objectives_sel.min(fe.pause_objectives.len() - 1);
            let hint = fe.pause_objectives[sel].hint;
            fe.pause_hint = Some(if hint == 0xffff_ffff { 0x31d } else { hint });
        }
    }
    // Tick through the boot load and let the page's timers run (the flash cycle etc.).
    for _ in 0..3 { fe.flow.main_tick(); let _ = fe.flow.load_tick(); }
    fe.press_start_on = true;
    let lang = read_files_bin(&scene_iso_name(), "USATxt.dat").and_then(|b| nf_formats::hud::Lang::parse(&b).ok())
        .or_else(|| scene.sim.as_ref().and_then(|s| s.lang.clone()));
    let intro = args.iter().any(|a| a == "--intro");
    #[allow(unused_mut)] // the movies feature assigns it
    let mut movie = None;
    #[cfg(feature = "movies")]
    {
        // --movie-frame defaults past the first GOP: frame 0 of the aux FMVs is the fade-in from
        // black (fix-menus-real's live check; the movie reports pin decodes on frame 12), so a bare
        // --page main --shot must not show it.
        let frame_n: usize = args.iter().position(|a| a == "--movie-frame").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(12);
        let want = if intro {
            Some(menu::INTRO_MOVIES[args.iter().position(|a| a == "--intro").and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0).min(menu::INTRO_MOVIES.len() - 1)])
        } else {
            let aux = menu::aux_movie_for(fe.page_id());
            (aux != 0).then_some(aux)
        };
        if let Some(code) = want {
            // The image the movies are on: NF_MOVIE_ISO over the ISO argument (the live loop's
            // App::movie_iso_path precedence) — not the bare NF_ISO env, which may point at the
            // slim on-foot image whose PSS files are 0-byte stubs [L probe_iso_movies]. (Both
            // sides of the fix-menus-real2 merge landed this precedence; movie_iso_name keeps the
            // ISO argument as the no-NF_MOVIE_ISO fallback, which the env-only form dropped.)
            match movie::movie_still(&movie_iso_name(iso_path), code, frame_n) {
                Ok(img) => movie = Some((img.width, img.height, img.rgba)),
                Err(e) => eprintln!("[front end] movie {code:08X}: {e} — the dark plate stands in"),
            }
        }
    }
    if intro {
        // The intro FMV owns the screen — no controls, no plate [M 0x209e88]; the movie plane is
        // the whole picture. Without movie data the headless plate stands in.
        let hud = if movie.is_some() { None }
            else { Some(menu::menu_overlay(&fe, &tables.font, &tables.font_header, lang.as_ref(), &scene.hud_textures, 1280.0 / 720.0)) };
        return (hud, movie);
    }
    fe.backdrop_live = movie.is_some();
    (Some(menu::menu_overlay(&fe, &tables.font, &tables.font_header, lang.as_ref(), &scene.hud_textures, 1280.0 / 720.0)), movie)
}

/// The ISO path for `read_files_bin` (the NF_ISO env or the default sandbox path).
fn scene_iso_name() -> String {
    std::env::var("NF_ISO").unwrap_or_else(|_| "Nightfire-onfoot.iso".to_string())
}

/// The image the headless movie stills read: NF_MOVIE_ISO (a full retail image) over the ISO
/// argument the binary was given — the live loop's `App::movie_iso_path` precedence. The old
/// still path read the bare `NF_ISO` env instead of the ISO argument, so
/// `nf-view <retail.iso> 07000048 --page main` reported the slim image's 0-byte PSS stubs and
/// never saw the retail bytes [fix-menu-backdrop].
pub fn movie_iso_name(iso_path: &str) -> String {
    std::env::var("NF_MOVIE_ISO").unwrap_or_else(|_| iso_path.to_string())
}

/// Renders one frame offscreen and writes it as a PNG. `menu` = a front-end overlay frame (fix-menus2:
/// `--shot` on the front-end boot draws the menu pages too, so the main agent can compare with PCSX2).
fn shot(scene: &scene::Scene, cam: &Camera, out: &str, choice: Option<usize>, time: f32, mut opts: scene::ViewOpts, notex: bool, nofog: bool, menu: Option<&gpu::HudFrame>, movie: Option<&(u32, u32, Vec<u8>)>) -> Result<()> {
    let (w, h) = (1280u32, 720u32);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (_adapter, device, queue) = pollster::block_on(device(&instance, None))?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // fix-sentinel-camera-play: the headless renders show the ride too — a `--frame N` shot taken
    // during the guided ride draws the rocket's forward view [M `Player_PositionCamera` 0x18f5d8
    // case 0xc], not Bond's.
    let cam = &scene.sim.as_ref().and_then(|s| s.view_override()).map(|(p, d)| Camera {
        pos: Vec3::new(p[0], p[1], p[2]), yaw: d[0].atan2(d[2]), pitch: d[1].clamp(-1.0, 1.0).asin(), fov: GAME_FOV,
    }).unwrap_or(*cam);
    let (opts_right, opts_up) = cam_axes(cam);
    opts.right = opts_right;
    opts.up = opts_up;
    let mut r = gpu::Renderer::new(&device, &queue, format, scene, notex, nofog);
    // fix-cutscene-letterbox: a live script camera frames the shot at the game's own 4:3 aspect
    // and pillar-boxes the render into the band; gameplay keeps the window's widescreen.
    let cut = scene.sim.as_ref().is_some_and(|s| s.script_camera().is_some());
    let (proj_aspect, band) = if cut { cutscene_frame(w as f32, h as f32) } else { (w as f32 / h as f32, [0.0; 4]) };
    let (vp_shot, _) = cam.view_proj(proj_aspect);
    // The doors' `Door_IsClosed` snapshot for the walk's portal gate (fix-door-portal-links).
    let door_closed = scene.sim.as_ref().map(|s| s.drones.doors.closed_flags());
    let vis_shot = r.cel_visibility(cam.pos.to_array(), vp_shot, scene.sim.as_ref().map(|s| &s.world), door_closed.as_deref());
    r.update_skin(&queue, scene, choice, time, opts, vis_shot.as_deref());
    r.lights = scene.frame_lights();
    r.screen_aspect = w as f32 / h as f32;
    // `NF_BENCH=N`: time the per-frame CPU side N times (posing + vertex build, and the upload) for profiling.
    if let Some(n) = std::env::var("NF_BENCH").ok().and_then(|v| v.parse::<u32>().ok()) {
        let t0 = std::time::Instant::now();
        for _ in 0..n { let _ = scene.animate(choice, time, opts); }
        let t1 = std::time::Instant::now();
        let mut v = Vec::new();
        for _ in 0..n { v = posed_vertices(scene, choice, time, opts); }
        let t2 = std::time::Instant::now();
        let _ = &v;
        queue.submit([]); let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let t3 = std::time::Instant::now();
        let ms = |a: std::time::Instant, b: std::time::Instant| (b - a).as_secs_f64() * 1000.0 / n as f64;
        eprintln!("[bench] {} char vertices: animate {:.2} ms, posed_vertices (incl. animate) {:.2} ms, upload {:.2} ms", v.len(), ms(t0, t1), ms(t1, t2), ms(t2, t3));
        let t4 = std::time::Instant::now();
        for _ in 0..n { r.update_skin(&queue, scene, choice, time, opts, vis_shot.as_deref()); }
        queue.submit([]); let _ = device.poll(wgpu::PollType::wait_indefinitely());
        eprintln!("[bench] GPU path: palette build + upload {:.2} ms", ms(t4, std::time::Instant::now()));
    }
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
        dimension: wgpu::TextureDimension::D2, format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let depth = gpu::depth_view(&device, w, h);
    let (vp, vp_rot) = cam.view_proj(proj_aspect);
    let snow = snow_for(scene, cam);
    let hud_owned = shot_hud(scene, proj_aspect, menu);
    let hud = &hud_owned;
    let movie_arg = movie.map(|(w, h, rgba)| (*w, *h, rgba.as_slice()));
    let mut enc = r.render(&device, &queue, &view, &depth, vp, vp_rot, cam.pos.to_array(), time as u32, &snow, hud, movie_arg, if cut { Some(band) } else { None });
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
    /// The adapter (the dev window configures its own surface against it).
    adapter: wgpu::Adapter,
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
    /// microcam-wheel-zoom: pending wheel zoom step (±1) — the wheel while the aim (right) button
    /// is held on a weapon that can zoom. Consumed by the next game tick, like `weapon_cycle`.
    zoom_cycle: i8,
    weapon_switch_to: Option<u8>,
    /// The external dev panel (http://127.0.0.1:7007, `--dev`) and the level its catalog describes.
    panel: Option<panel::Handle>,
    panel_level: u32,
    panel_frames: u32,
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
    /// difficulty-effects: the selected difficulty, the port's `GameState+0x28` (0x2a3790) —
    /// boot default 2 [M 0x1c7e78:25], the difficulty page's confirm writes it [M 0x216ca8:15],
    /// and every load/adopt reads it.
    selected_difficulty: u32,
    /// fix-weapon-assets: a dev-menu/panel give of a weapon whose assets the level archive lacks —
    /// the tick loop answers it with a warm same-level reload (`load_with_carry_weapons`) that
    /// merges the weapon's assets, then re-places Bond and re-issues the switch request.
    pending_weapon_assets: Option<u8>,
    /// fix-restart-target: the ENTRY checkpoint — the RamSave buffer 0x2b9440 the game keeps
    /// across level changes. The fail/death menu's Restart re-serves it (`P_ENDMISSION_Handler`
    /// 0x216618:21's `Player_RamLoad(Bond, 1)`) instead of capturing the dying state — the entry
    /// checkpoint's channels predate the fail, so the reload cannot re-fail (the 07000007 loop).
    /// fix-restart-health: re-captured with the POST-load state after EVERY load
    /// (`entry_checkpoint_save` — the game's `Player_InitWeapon` re-save [M 0x1a7ef8:325]; cold
    /// loads included, whose capture holds `Player_Init`'s health 100). The old model kept the
    /// save the level was loaded with — `None` after a cold load — and the restart served the
    /// zeroed buffer: health 50.0, the playtest's "half health".
    entry_save: Option<nf_game::player::RamSave>,
    /// The `--all` flag (draw dynamic entities at their placements, not only 0x8000 cels) — reused on level reload.
    all: bool,
    /// Input recorder (B-5): `None` unless `--record <file>` was given. Each tick's input is pushed right before
    /// `step_bond`; `finish()` writes the file on exit. Costs nothing when `None`.
    recorder: Option<replay::Recorder>,
    /// Input replay (B-5): `None` unless `--replay <file>` was given. When set, the tick loop reads the next input
    /// from `replay_ticks[idx]` instead of the keyboard/mouse, so the recorded session plays back bit-for-bit.
    /// `replay_idx == replay_ticks.len()` ⇒ the recording ended (the sim pauses).
    replay: Option<replay::Replay>,
    replay_idx: usize,
    /// The party weapon lock's rising-edge memory (`switch_channels_prev[0x48]`, `Player_Weapon`
    /// 0x1a24c8:247): the previous tick's channel state. Reset on every level load — the game's `_prev`
    /// is restored from the RamSave, whose value for the lock channel is 0 in every pre-party level.
    prev_lock_ch: bool,
    /// Render interpolation state: the cameras and palettes of the last two ticks, and whether a tick ran this frame.
    ticked: bool,
    /// This frame's 60 Hz tick count (the burning woman advances once per tick while a paused page shows her).
    frame_ticks: u32,
    /// The paused menus' 60 Hz accumulator (seconds) for the animations that run while the game is paused.
    menu_clock: f32,
    /// The sim reported the mission's own end this frame (see `Drones::mission_end`).
    pending_mission_end: bool,
    /// Dev `--set-channel <ch>`: a switch channel raised 2 s into the level (channel 100 = mission
    /// success — what scratchpad complete.py writes in PCSX2), for end-of-mission checks.
    force_channel: Option<u8>,
    cam_prev: Option<Camera>,
    cam_cur: Option<Camera>,
    pal_prev: Option<(Vec<nf_formats::anim::Mat4>, Vec<[f32; 4]>)>,
    pal_cur: Option<(Vec<nf_formats::anim::Mat4>, Vec<[f32; 4]>)>,
    /// `--fps N`: a frame-rate cap (vsync off); None = vsync.
    fps_cap: Option<f32>,
    frame_start: std::time::Instant,
    /// The front end (fix-menus): Some while a menu page is up (title / main / mission select /
    /// difficulty / options / pause / results). None = playing a mission directly.
    fe: Option<menu::FrontEnd>,
    /// net-consolidate: the ONE session layer (nf-net) while an online match runs (`--net-host` /
    /// `--net-join`, or adopted from the lobby pages' `fe.mp_session` when their match starts).
    /// None = local play, no session, no gate.
    net: Option<mpnet::SessionProxy>,
    /// The lobby's session (the same core `fe.mp_session` wraps): created when the online pages
    /// open, adopted into `net` when the match starts.
    net_lobby: Option<mpnet::SessionProxy>,
    /// This machine's slot in the online match (the host is 0; a client's arrives in WELCOME).
    net_local_slot: usize,
    /// The next sim tick of the online match (the lockstep counter; render interpolation is
    /// unchanged — `ticked` still marks the frames that advanced it).
    net_tick: u32,
    /// The online match's negotiated settings (the host's choices + seed): set when the session
    /// reports `MatchStarted`, consumed by the next level load (`load_mp_carry`). The local menu
    /// confirm path rides the same slot (its `mp_match_settings` was set but never read before —
    /// the menu's own choices now reach the match core through it too).
    pending_mp_settings: Option<nf_game::mp::MpSettings>,
    /// The `--net-host` demo arm: the settings waiting for the first client to join (the lobby
    /// pages are task 052's; the flag path starts the match as soon as someone connects).
    net_armed: Option<nf_game::mp::MpSettings>,
    /// The current level's hashcode (the archive name), for the pause page's restart / the results.
    level_code: u32,
    /// frontend-music: the front-end page the music transitions last saw (the manager's msg-0x44
    /// page change re-runs the new page's 0x4c arm [M 0x21c858:345-346]); None before the first.
    fe_music_page: Option<menu::Page>,
    /// frontend-music: the host's SFXStart handle counter (the game's RPC passes the EE node
    /// handle; the port's driver dedupes/tracks by it).
    menu_handle: u32,
    /// frontend-music (menu-audio-state): the game's music statics, which are .bss bytes that
    /// outlive every menu manager [M 0x1dceb8's `DAT_002e1b11`; L `MusicMasterVolume`
    /// 0x30c958]. Stashed from the front end on every manager drop (`stash_music_statics`) and
    /// restored into every fresh one (`inherit_music_statics`).
    music_bank_flip: bool,
    music_loaded_bank: u32,
    music_master: i32,
    /// The game's strings (USATxt.dat) for the front end when the loaded map has no Sim to hold them.
    lang: Option<nf_formats::hud::Lang>,
    /// The gamepad backend (fix-input), behind the `gamepad` cargo feature. `None` without a pad
    /// device; with one, its Input merges into the keyboard's each tick (the game's own mapping,
    /// curves and ramps — docs/systems/input.md §8).
    #[cfg(feature = "gamepad")]
    pad: Option<gamepad::Backend>,
    /// fix-menu-movies2: the ONE playing movie, the port's `psiStartBackgroundMovie` [M 0x1081a8].
    /// Every movie rides it — the GameFlow state-6 briefing/outro (fix-movie-playback), the
    /// P_INTRO pair and the page's looping backdrop (fix-menu-movies); the host's `movie_tick`
    /// decides which one the game would be playing and `present()` feeds the ONE plane in
    /// gpu.rs. Cleared by the skip, the end of a non-looping stream, or a code change.
    movie: Option<movie::Player>,
    /// `--dev`: the dev menu in its own native window beside the game (devwin.rs; the user: "outside of the game, but
    /// not in a browser"). None without `--dev`, or when the window couldn't open (the F2 overlay is the fallback).
    devwin: Option<devwin::DevWindow>,
    /// The sound hold's mirror (fix-pause-audio): true while a menu page is up or the viewer pause
    /// holds the sim. The diff in `RedrawRequested` applies it to `sound` on the edge — the game's
    /// pause page runs `SFXPause`+`SFXPauseAllStreams` on open and the unpause pair on close
    /// [M 0x21b908:121-124, 0x21c2b0:11-22].
    audio_paused: bool,
    /// The loading screens' pictures (07000500.bin, crate::loading), read on the first level change.
    loading_art: Option<std::collections::HashMap<u32, (u32, u32, Vec<u8>)>>,
    /// The pointer in the menu's 640×480 space (PC mouse support).
    menu_pointer: Option<(f32, f32)>,
    /// async-level-load: the level load running on a worker thread — `Some` from the frame the
    /// transition spawns it until the scene is swapped in. While it runs, the window thread
    /// redraws the loading screen with the dot animation each frame (`poll_async_load`); the
    /// game does the same on one thread: the load blocks the main thread while `VBlankInt`
    /// 0x124088 steps the dots every 11 vblanks.
    async_load: Option<LevelLoad>,
}

/// A running level load plus what its swap needs (async-level-load).
struct LevelLoad {
    job: async_load::Job<Result<scene::Scene, String>>,
    /// The level code the load serves (`pending_level`'s value at spawn): the loading screen's
    /// picture and the failure log read it.
    next: u32,
    /// drive-ptp: the DRIVING return (`Boot_LoadPTPData` 0x1c8560), taken from the OLD sim at
    /// spawn and applied to the NEW one at the swap (see `swap_in_scene`).
    ptp_return: Option<nf_game::ptp::DriveReturn>,
    /// The finished build, parked while the loading screen's minimum time runs (`LOADING_HOLD`).
    ready: Option<Option<Result<scene::Scene, String>>>,
}

/// The loading screen's minimum time on screen. The PS2's disc load kept it up ~11.5 s on
/// 07000001 → 07000002 (PS2LSActive 2 → 0, measured in PCSX2 2026-10-15) — long enough to read
/// the objective and the hint while the dots sweep (183.5 ms a step, the port's 11-vblank cadence
/// exactly). The port builds a level in 1-2 s, so the screen flashed by (playtest 1015-2142); the
/// user chose to hold it a few seconds [S — a PC-port choice, not the game's timing]. Pub for the
/// flow tests (pause-results-flows): the headless suite pins the 5 s = 300-tick window.
pub const LOADING_HOLD: f64 = 5.0;

/// microcam-wheel-zoom: where a wheel notch goes. While the aim (right) button is held on a weapon
/// that can zoom — the game's own scope flag +0x68 & 0x40 with a magnification above 1 [M
/// weapon_data 0x2bf150; the sim's zoom block gates on the same two] — the wheel steps the zoom
/// [S the 2026-10-15 playtest]. Otherwise it keeps switching weapons (the pre-feature behaviour).
pub fn wheel_steps_zoom(zoom_held: bool, weapon_flags: u32, weapon_zoom: f32) -> bool {
    zoom_held && weapon_flags & 0x40 != 0 && weapon_zoom > 1.0
}

impl App {
    /// One menu action from the keyboard or the mouse (PC mouse support, 2026-10-15): the movie skip, the pause page's
    /// Back = resume, the page's own handling, then the level request / close the page asked for.
    fn menu_input(&mut self, a: menu::Action) {
        if self.fe.is_none() { return; }
        use menu::Action as MA;
        let fe = self.fe.as_mut().unwrap();
        // State 6: only the skip action matters [M GameFlow_Main 0x1c98c8 case 6
        // reads Input_Action(-1, 0x19, 1) and nothing else — the menu pages are not
        // up during a movie]. The port's Confirm keys (Enter/Space) stand in for
        // the pad's X/Start [S docs/systems/input.md §3].
        if fe.flow.get_state() == menu::ST_MOVIE {
            // A movie can't be skipped in its first second on screen: the press that picked the
            // mission (or a quick double-tap) used to skip its briefing before a frame showed.
            let shown = self.movie.as_ref().map_or(f32::MAX, |p| p.shown_for());
            if a == MA::Confirm && shown >= 1.0 { fe.flow.skip_movie(); }
            return;
        }
        if fe.page == menu::Page::Pause && a == MA::Back {
            self.fe = None; // resume [S: the game's resume is its own row; Back resumes too]
            if self.scene.sim.is_some() { self.grab(true); }
            return;
        }
        fe.action(a);
        self.menu_after();
    }

    /// After a menu action (key, click or hover): the page's sounds, the level it requested and its close.
    fn menu_after(&mut self) {
        let Some(fe) = self.fe.as_mut() else { return };
        // difficulty-effects: the difficulty page's confirm writes GameState+0x28 at pick time
        // [M C_SBNFDFCTY 0x216ca8:15] — the game keeps it across loads; the port stores it on
        // the App and every following load adopts it (Scene::build's create-time gates).
        if let Some(d) = fe.requested_difficulty.take() {
            self.selected_difficulty = d;
        }
        // menu-sfx: the front end's sound events play straight through the driver's 2-D start —
        // `Menu_PlaySound`'s `Sound_Play(100.0f, id, 0, 0)` tail [M 0x1f3278] batched like the EE's
        // `SFXStart`. The loaded bank(s) decide audibility exactly as the game's lookup does (a
        // miss returns without starting): the front-end map boots bank 0xC, a paused mission keeps
        // its own bank, which carries the pause blips [D SB_4/SB_5; docs/systems/menu-sfx.md §2-3].
        let menu_sfx: Vec<_> = fe.drain_sfx().collect();
        let menu_rm: Vec<u32> = fe.drain_sfx_removes().collect();
        if !menu_sfx.is_empty() || !menu_rm.is_empty() {
            if let Some(snd) = self.sound.as_mut() {
                snd.batch(|drv, out| {
                    // The 0x4c order: the removes land before the same frame's starts [M
                    // 0x20a408:17-22] — the restart replaces the loop instead of stacking it.
                    for id in &menu_rm { drv.remove(0, Some(*id)); }
                    for ev in &menu_sfx { drv.start(out, ev.id, None, 0, None); }
                });
            }
        }
        let Some(fe) = self.fe.as_mut() else { return };
        // net-consolidate: the online pages need their session. The proxy is
        // created lazily on page entry (the listen/dial rows replace its
        // unbound placeholder core); `net_lobby` holds the same Rc so the app
        // can adopt it when the match starts.
        if matches!(fe.page, menu::Page::MpHost | menu::Page::MpNetJoin) && self.net_lobby.is_none() {
            match mpnet::SessionProxy::host(0) {
                Ok(p) => {
                    fe.mp_session = Some(Box::new(p.clone()));
                    self.net_lobby = Some(p);
                }
                Err(_) => {}
            }
        }
        let closed = fe.closed;
        if let Some(code) = fe.requested_level.take() {
            // The menu picked a level: through the same reload path a LOAD_LEVEL uses. A cold pick
            // (the pause page's RESTART [M 0x204408:139]) loads fresh, not with the current state.
            if !fe.flow.warm_reset { if let Some(sim) = self.scene.sim.as_mut() { sim.drones.level_exit_cold = true; } }
            // death-loop-07: the results page's REPLAY row carries the `Player_RamLoad(Bond, 1)`
            // half [M P_ENDMISSION_Handler 0x216618:19-21] — mark the serve so the reload comes
            // off the ENTRY checkpoint (the restart arm), not the dying sim's live capture: the
            // carried fail channel re-failed the mission the tick it came back — the 07000007
            // replay loop (playtest 1012-1723). Consumed with the request, one pick only.
            let replay_pick = std::mem::take(&mut fe.requested_restart);
            if replay_pick { if let Some(sim) = self.scene.sim.as_mut() { sim.drones.level_exit_restart = true; } }
            self.pending_level = Some(code);
            // net-match-host: the confirm page's settings ride with the level (`mp_confirm_start`
            // builds them [M mp-menu-flow]; before this they were set but never read — the match
            // core now gets the menu's own scenario/map/limits/players).
            if let Some(st) = fe.mp_match_settings.take() { self.pending_mp_settings = Some(st); }
            // net-consolidate: an online match starting (the host's confirm page armed the
            // session, or the client took the tunnel's match payload) — adopt the lobby's
            // session as the lockstep gate. `has_match` is exact: match_start is Some only
            // after the arm / the tunnel payload, and a cancelled lobby's core is closed.
            let adopt = self.net_lobby.as_ref().map_or(false, |p| p.has_match() && !p.core_is_closed());
            if adopt && self.net.is_none() {
                if let Some(p) = self.net_lobby.take() {
                    self.net_local_slot = p.0.borrow().local_slot();
                    self.net_tick = 0;
                    self.net = Some(p);
                    eprintln!("[net] adopted the lobby's session for the match");
                }
            }
        }
        // The pause page's Resume row (cursor 0) closes the menu [M C_GCPAUSE_Handler
        // 0x204408 — the game's resume is its own row]: the host consumes the flag and
        // leaves the menu (the sound hold lifts through the diff below).
        if closed {
            self.close_front_end();
        }
    }

    /// The front end closes (a briefing resolved to a level, the pause's pick, the resume):
    /// the menu-music pair dies with it. The game's exit path plays the menus fade and lets the
    /// level's own `SFXLoadSoundBank` reset the items [M C_SBNFMAP 0x20b8d8 case 0x4b —
    /// SFXFadeDown(1), no explicit pair remove]; the port's banks swap without that reset, so
    /// the close removes the pair explicitly through the driver (all instances of the id — the
    /// IOP's KillSFX semantics [M IRX 0x1f88 → KillSFX 0x1ea4]). Without it the looped items
    /// played on through gameplay and the pause (the playtest's "music continues after I leave
    /// the menu").
    fn close_front_end(&mut self) {
        let rm: Vec<u32> = self.fe.as_mut().map(|fe| fe.drain_sfx_removes().collect()).unwrap_or_default();
        if let Some(snd) = self.sound.as_mut() {
            snd.batch(|drv, _out| {
                for id in &rm { drv.remove(0, Some(*id)); }
                for id in [menu::SFX_MENU_OPEN_MOVE, menu::SFX_MENU_OPEN_SELECT] { drv.remove(0, Some(id)); }
            });
        }
        self.stash_music_statics();
        self.fe = None;
        if self.scene.sim.is_some() { self.grab(true); }
    }

    /// frontend-music (menu-audio-state): the music statics are .bss bytes that OUTLIVE every
    /// menu manager — `DAT_002e1b11` (the bank alternation) persists across front-end loads and
    /// rides the PTP data across boots [M 0x1dceb8:94-101, Boot_LoadPTPData 0x1c8560:61], and
    /// `MusicMasterVolume` @ 0x30c958 outlives the managers the same way [L ACTION.ELF .data].
    /// The port keeps them per-`FrontEnd`, so every drop of a manager stashes them here and
    /// every create of one (the gameplay pause's `get_or_insert_with`, `demo_results`) restores
    /// them — without it a fresh pause front end re-played the boot's jingle on the
    /// quit-to-front-end load instead of the alternation's other one.
    fn stash_music_statics(&mut self) {
        if let Some(fe) = self.fe.as_ref() {
            self.music_bank_flip = fe.music.bank_flip;
            self.music_loaded_bank = fe.music.loaded_bank;
            self.music_master = fe.music.music_master;
        }
    }

    /// The create side of [`App::stash_music_statics`]: a fresh front end inherits the statics.
    fn inherit_music_statics(fe: &mut menu::FrontEnd, bank_flip: bool, loaded_bank: u32, master: i32) {
        fe.music.bank_flip = bank_flip;
        fe.music.loaded_bank = loaded_bank;
        fe.music.music_master = master;
    }

    // ── net-match-host: the online session's app surface ────────────────────────────────────────
    /// Host: listen on `port` (slot 0 is this machine's agent). The lobby pages are task 052's;
    /// the flag path (`--net-host`) arms the demo match for the first join.
    pub fn net_host(&mut self, port: u16) -> Result<(), String> {
        self.net = Some(mpnet::SessionProxy::host(port)?);
        self.net_lobby = None;
        self.net_local_slot = 0;
        self.net_tick = 0;
        Ok(())
    }

    /// Client: dial `addr` ("host:port"); the slot arrives in the session's WELCOME.
    pub fn net_join(&mut self, addr: &str) -> Result<(), String> {
        let s = mpnet::SessionProxy::connect(addr)?;
        s.0.borrow_mut().set_codename("Bond");
        self.net = Some(s);
        self.net_lobby = None;
        self.net_local_slot = 0; // corrected when the WELCOME lands
        self.net_tick = 0;
        Ok(())
    }

    /// A desync / a lost host / a refused join: the game's message box (the front end's option
    /// box with the port-only string) and the lobby page underneath it; the session is closed.
    fn net_fail(&mut self, msg: &str) {
        eprintln!("[net] {msg}");
        if let Some(se) = self.net.as_mut() { se.0.borrow_mut().close(); }
        self.net = None;
        self.net_lobby = None;
        self.net_armed = None;
        let fe = self.fe.get_or_insert_with(menu::FrontEnd::new);
        fe.net_notice = Some(msg.to_string());
        fe.page = menu::Page::MpJoin;
        fe.cursor = 0;
        fe.closed = false;
        fe.mp_match_settings = None;
        for a in fe.mp_join.iter_mut() { a.joined = false; a.ready = false; }
        if self.scene.sim.is_some() { self.grab(false); }
    }

    /// The menu's 640×480 space for a window pixel: the menus draw as a centred 4:3 picture (Anchor::Centre).
    fn menu_space(&self, px: f64, py: f64) -> Option<(f32, f32)> {
        let g = self.gfx.as_ref()?;
        let (w, h) = (g.config.width as f32, g.config.height.max(1) as f32);
        let (bw, bh) = if w / h >= 4.0 / 3.0 { (h * 4.0 / 3.0, h) } else { (w, w * 3.0 / 4.0) };
        let (bx, by) = ((w - bw) / 2.0, (h - bh) / 2.0);
        Some(((px as f32 - bx) / bw * 640.0, (py as f32 - by) / bh * 480.0))
    }

    /// Draws the level's loading screen and presents it [M ResetMap_Load 0x1d6360 state 3 →
    /// ResetMap_GenLoadScreen 0x1d5de0, skipped for the front end 0x07000048]. async-level-load:
    /// called once with 0 dots before the worker spawns, then once per frame by
    /// `poll_async_load` with `loading::dots_lit` over the load's elapsed vblanks — the game's
    /// `VBlankInt` 0x124088 animates the same six dots while the main thread is inside the load.
    fn draw_loading_screen(&mut self, level: u32, dots: u32) {
        if level & 0xff00_ffff == 0x0700_0048 { return; }
        if self.loading_art.is_none() {
            self.loading_art = Some(read_files_bin(&self.iso, loading::ARCHIVE).map(|b| loading::art(&b)).unwrap_or_default());
        }
        let Some(g) = self.gfx.as_mut() else { return };
        if g.minimized { return; }
        let level = if level & 0xff00_ffff == 0x0700_0018 { 0x0700_001b } else { level };
        let picture = self.loading_art.as_ref().and_then(|a| a.get(&loading::image_for(level)));
        let lang = self.scene.sim.as_ref().and_then(|s| s.lang.as_ref()).or(self.lang.as_ref());
        let aspect = g.config.width as f32 / g.config.height.max(1) as f32;
        let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) | 1;
        let hint = loading::hint_for(level, || { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; seed >> 8 });
        let hud = match self.scene.hud.as_ref() {
            Some(t) => menu::loading_overlay(&t.font, lang, &self.scene.hud_textures, aspect, level, hint, dots),
            None => gpu::HudFrame::default(),
        };
        let frame = match g.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => return,
        };
        let view = frame.texture.create_view(&Default::default());
        g.renderer.screen_aspect = aspect;
        // A zero view-projection collapses every world vertex (w = 0), so only the picture and the overlay draw.
        let id = [[0.0f32; 4]; 4];
        let movie = picture.map(|(w, h, rgba)| (*w, *h, rgba.as_slice()));
        let enc = g.renderer.render(&g.device, &g.queue, &view, &g.depth, id, id, [0.0; 3], 0, &[], &hud, movie, None);
        g.queue.submit([enc.finish()]);
        g.queue.present(frame);
    }

    /// The level transition's first half (async-level-load): capture the carry-over exactly as
    /// the synchronous path did, draw the screen's first frame and hand the build to a worker.
    fn start_async_load(&mut self, next: u32) {
        // `Trigger_Activate`'s exit order: the success/fallback channel write (already applied in
        // the sim), then `Player_RamSave` [M 0x1b2318:97-116] — capture the old sim now, before
        // the scene swap drops it (fix-level2 F2). The old sim stays in `self.scene` until the
        // swap, so the worker's `save` snapshot and the loading screen's font/picture reads see
        // the same state the synchronous path saw.
        // fix-restart-target: the fail/death menu's Restart (`P_ENDMISSION_Handler`
        // 0x216618:19-21, menu 0x40000042's row 0) is a WARM reload of the CURRENT level on
        // the ENTRY checkpoint: `ResetMap_LevelToLoad(GameState._12_4_, 1, 1)` +
        // `Player_RamLoad(Bond, 1)`. The game does NOT capture the dying state — it re-serves
        // the RamSave buffer as of level entry (re-saved with the post-load state by
        // `Player_InitWeapon` [M 0x1a7ef8:325]), whose ch 99/100 predate the fail, so the
        // reload cannot re-fail (the 07000007 replay loop). The boost arg 1 floors the health
        // at the difficulty's `ContinueHealthBoost*` — 50.0 on all three [L ELF 0x30cc34-3c].
        // The PAUSE menu's Restart stays the COLD arm to BaseMap (`C_GCPAUSE_Handler`
        // 0x204408:139 — `level_exit_cold`, save = None).
        let cold = self.scene.sim.as_ref().is_some_and(|sim| sim.drones.level_exit_cold);
        let restart = self.scene.sim.as_ref().is_some_and(|sim| sim.drones.level_exit_restart);
        let save = if cold { None } else if restart {
            // The ENTRY checkpoint (+ the health floor) — `fail_restart_save` carries the
            // evidence (`P_ENDMISSION_Handler` 0x216618:19-21 + `Player_RamLoad`'s floored
            // buffer). fix-restart-health: the entry save is the POST-load capture the
            // load arm makes (`Player_InitWeapon`'s unconditional `Player_RamSave`
            // [M 0x1a7ef8:325]) — after a cold load it holds health 100, the user's "I
            // should be at full health" (the old zero-buffer fallback served 50.0).
            Some(fail_restart_save(self.entry_save.as_ref(), self.scene.sim.as_ref().unwrap()))
        } else {
            self.scene.sim.as_ref().map(|sim| nf_game::player::RamSave::capture(
                &sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
                .with_mission(&sim.drones.mission))
        };
        let name = level_archive_name(next);
        eprintln!("level: loading {name} ({})", if save.is_some() { "warm" } else { "cold" });
        // drive-ptp: the DRIVING return (`Boot_LoadPTPData` 0x1c8560) rides the reload —
        // taken from the OLD sim (the handover hook stashed it, scene.rs) and applied to
        // the NEW one at the swap: the mission status (`Mission_SetStatus` 6/3/2 from PTP
        // +0x978) and the 12 `PlrStats` driving slots (`PlrStats_SetDrivingData`
        // 0x1c8560's call). The resume level itself is the return's 0x07000048 [M tail].
        let ptp_return = self.scene.sim.as_ref().and_then(|s| s.drones.driving_return);
        self.draw_loading_screen(next, 0);
        let iso = self.iso.clone();
        let all = self.all;
        let dev = self.dev.is_some();
        // net-match-host: an online match's handover loads the MP core from the negotiated
        // settings (the same `load_mp_carry` on every machine — the seed rides inside). The local
        // menu confirm's settings ride here too now (they were set but never read before).
        // difficulty-effects: otherwise the front-end pick rides every front-end load [M the game's
        // level load reads GameState+0x28; the direct-load wrappers keep the replay 1].
        let mp_settings = self.pending_mp_settings.take();
        let difficulty = self.selected_difficulty;
        let job = async_load::Job::spawn(move || {
            if let Some(st) = mp_settings {
                load_mp_carry(&iso, &name, all, save.as_ref(), st)
            } else if dev { load_with_carry_dev(&iso, &name, all, save.as_ref()) }
            else { load_with_carry_difficulty(&iso, &name, all, save.as_ref(), difficulty) }
                .map_err(|e| e.to_string())
        });
        self.async_load = Some(LevelLoad { job, next, ptp_return, ready: None });
    }

    /// One frame of the running load (async-level-load): `false` = the worker still runs — the
    /// loading screen was redrawn with the dots advanced (`loading::dots_lit` over the elapsed
    /// 60 Hz vblanks, the window's redraws standing in for `VBlankInt` 0x124088's), the caller
    /// ends the frame and asks for the next; `true` = nothing running or the result arrived and
    /// the swap is done, the caller falls through to normal play.
    fn poll_async_load(&mut self) -> bool {
        let Some(load) = self.async_load.as_mut() else { return true; };
        if load.ready.is_none() { load.ready = load.job.poll(); }
        // A successful build into a level with a loading screen waits out LOADING_HOLD (the
        // front-end map draws no loading screen [M ResetMap_GenLoadScreen skipped for 0x07000048];
        // a failure goes at once).
        let hold = load.next != menu::FRONTEND_LEVEL
            && load.job.elapsed_ticks() < (LOADING_HOLD * 60.0) as u32
            && !matches!(load.ready, Some(Some(Err(_))) | Some(None));
        if load.ready.is_none() || hold {
            let (level, dots) = (load.next, loading::dots_lit(load.job.elapsed_ticks()));
            self.draw_loading_screen(level, dots);
            if let Some(g) = &self.gfx { g.window.request_redraw(); }
            return false;
        }
        // The job is over whatever the result: take it and let the frame finish normally.
        let mut load = self.async_load.take().unwrap();
        let res = load.ready.take().unwrap();
        match res {
            Some(Ok(scene)) => {
                let name = level_archive_name(load.next);
                self.swap_in_scene(scene, load.ptp_return);
                // A real level loaded from the front end: the menu manager is gone [M the MP
                // confirm's / the SP pick's MenuManager_Delete — "the host closing the overlay once
                // the load leaves the front-end map"]. The MP start loads its arena directly (no
                // briefing hand-over), and the front end stayed open over it — the arena never
                // showed, only the menu's backdrop (playtest 1015-2231 "cant get into a match").
                if load.next != menu::FRONTEND_LEVEL
                    && self.fe.as_ref().is_some_and(|fe| fe.flow.get_state() != menu::ST_MOVIE)
                {
                    self.stash_music_statics();
                    self.fe = None;
                    if self.scene.sim.is_some() { self.grab(true); }
                }
                // frontend-music: `Sound_Ready`'s front-end case [M 0x1dceb8 case 0x7000048] — the
                // load alternates bank 0xc (SBI slot 0 = SB_0, jingle 0x1d7) and 0x38 (slot 1 =
                // SB_1, jingle 0x470) through DAT_002e1b11, then `MenuManager_Create` resets
                // `menu_movie` [M 0x21b908:127] so the first page enter restarts the pair. The
                // port keeps every bank resident, so the flip re-points the driver's primary slot.
                if load.next == menu::FRONTEND_LEVEL {
                    if let Some(fe) = self.fe.as_mut() {
                        let hash = fe.music.front_end_sound_ready();
                        fe.music.jingles_up = false;
                        if let Some(s) = self.sound.as_mut() { s.driver.set_primary(if hash == 0x38 { 1 } else { 0 }); }
                    }
                    self.fe_music_page = None; // the manager re-create re-runs the page's 0x4c [M 0x21b908:127]
                    // The load's own MenuManager_Create re-opens the queued page [M 0x1d6360:160-163]:
                    // the quit arms' 0x4000001c lands on the mission list, not the placeholder page
                    // they left up during the load (pause-results-flows).
                    if let Some(fe) = self.fe.as_mut() { fe.reopen_queued_page(); }
                }
                eprintln!("level: loaded {name} (loading screen {:.1} s)", load.job.elapsed_ticks() as f64 / 60.0);
            }
            Some(Err(e)) => eprintln!("level: failed to load {}: {e}", level_archive_name(load.next)),
            None => eprintln!("level: load of {} panicked (worker thread died)", level_archive_name(load.next)),
        }
        // The lock channel's edge memory starts clean on the new level (the game's `_prev`
        // comes back from the RamSave; 0x48 has no writer before the party). The sync path
        // ran this on both the Ok and Err arms — every terminal branch here does too.
        self.prev_lock_ch = false;
        true
    }

    /// The load's main-thread half (async-level-load): the sound stack, the camera, the PTP
    /// return, the renderer rebuild — GPU upload, this thread owns the device — and the entry
    /// checkpoint, in the exact order the synchronous path ran.
    fn swap_in_scene(&mut self, mut scene: scene::Scene, ptp_return: Option<nf_game::ptp::DriveReturn>) {
        // The previous scene's skin palette and its interpolation cameras belong to the OLD skin
        // layout: uploading them into the new renderer overran its palette buffer every frame
        // until a tick rebuilt them — and a level that doesn't tick at once (an MP arena with the
        // menu still up) never did (the 787 GPU validation errors, playtest 1015-2231).
        self.pal_cur = None;
        self.pal_prev = None;
        self.cam_prev = None;
        self.cam_cur = None;
        self.sound = build_sound(&mut scene);
        // The fresh sound stack is unpaused: reset the mirror so the per-frame diff
        // re-applies the current hold (a menu can still be up over the reload).
        self.audio_paused = false;
        if let Some(start) = scene.start { self.cam = Camera { pos: Vec3::new(start.0[0], start.0[1] + 1.6, start.0[2]), yaw: start.1, pitch: 0.0, fov: GAME_FOV }; }
        self.scene = scene;
        self.anim_time = 0.0;
        // drive-ptp: `Boot_LoadPTPData` 0x1c8560's mirror runs on the returned
        // session — the mission status + the 12 slots, after the hub's own
        // `Mission_Init` (the game's `Mission_Init` skips 0x7000048 entirely
        // [M 0x182270:20-22], so the Boot-set status stands).
        if let Some(ret) = ptp_return {
            if let Some(sim) = self.scene.sim.as_mut() {
                ret.apply(&mut sim.drones.mission, &mut sim.drones.driving_stats);
                eprintln!("[ptp] return applied: MissionState {} (status byte {}), 12 slots carried", sim.drones.mission.status(), ret.status_byte);
            }
        }
        // mission-unlock-flow: the game's sp_level+0x10 flags are ONE global that survives every
        // map load; the port splits the store across the front end (FrontEnd::unlocked) and the
        // per-level sim (Progress::nightfire_status), so each fresh sim is seeded from the live
        // front end here — the results page's award (results_enter → set_level_bonus → the
        // mirror) then accumulates on the session's bits instead of restarting from the boot 0x3
        // [D ACTION.ELF .data 0x1d91e0]. Without a front end (headless loads, boot) the fresh
        // boot value stands. [S the port's glue for the game's single store.]
        if let Some(fe) = self.fe.as_ref() {
            if let Some(sim) = self.scene.sim.as_mut() {
                sim.drones.progress.nightfire_status = fe.packed_unlocks();
            }
        }
        // The renderer holds the old scene's GPU resources (textures, buffers): rebuild it
        // IN PLACE — the window and surface must survive the level change (the game's window
        // does not close). Dropping `Gfx` here closed the window and nothing recreated it
        // (winit's `resumed` only runs at startup), so the process sat on a headless event
        // loop with the audio still playing (the user's report).
        if let Some(g) = &mut self.gfx {
            g.renderer = gpu::Renderer::new(&g.device, &g.queue, g.config.format, &self.scene, self.notex, self.nofog);
        }
        // fix-restart-health: the loaded level's checkpoint is what a later
        // fail-Restart restores — the game re-saves the buffer with the POST-load
        // state on EVERY load (`Player_InitWeapon`'s unconditional `Player_RamSave`
        // [M 0x1a7ef8:325]), cold loads included (the `Player_RamLoad` body is
        // gated on `IsWarmReset` [M 0x1957a8:9], so `Player_Init`'s health 100 /
        // armour 0 [M 0x1910f8:128-138] stand and the arm's loadout/armour are
        // captured). The old model kept the save the level was LOADED with —
        // `None` after a cold load — and the restart then served the zeroed
        // buffer: health 50.0, the playtest's "half health".
        self.entry_save = Some(entry_checkpoint_save(self.scene.sim.as_ref().unwrap()));
    }

    /// F8/F9: show the mission-complete / mission-failed results page for the loaded mission [S demo
    /// hook — the game reaches it from `Mission_Update` 0x182c60:74's event; the port's mission-event
    /// → results glue is listed in docs/reports/fix-menus.md as remaining host work].
    fn demo_results(&mut self, complete: bool) {
        if self.level_code == 0 || self.level_code == menu::FRONTEND_LEVEL { return; }
        let mut fe = menu::FrontEnd::new();
        // frontend-music: `DAT_002e1b11` and `MusicMasterVolume` are statics, not per-manager —
        // the demo fe inherits the bank-flip state (the next front-end load must flip like the
        // game's), the resident bank and the music volume [M 0x1dceb8:94-101, 0x30c958]. The
        // stash outlives the dropped manager (menu-audio-state): a mission runs with no front
        // end at all, so the statics come from the last stash when the results open.
        if let Some(old) = self.fe.take() {
            Self::inherit_music_statics(&mut fe, old.music.bank_flip, old.music.loaded_bank, old.music.music_master);
        } else {
            Self::inherit_music_statics(&mut fe, self.music_bank_flip, self.music_loaded_bank, self.music_master);
        }
        // The demo hook stands in for Mission_Update 0x182c60, which fires in state 2 — the flow
        // must be in a state reset_map_level_to_load accepts (its gate [M 0x1d3800]) for the outro
        // request to wrap into the movie state.
        fe.flow.stack = vec![menu::ST_RUNNER];
        fe.flow.current_level = self.level_code;
        fe.mission_ended(if complete { menu::Outcome::Complete } else { menu::Outcome::Failed }, menu::Stats::default());
        // The completed page's enter runs against the live progress layer [M P_NFRESULTS 0x208e78
        // :77 GetScore(0) + :169 Menu_SetLevelBonus] — the fields the overlay draws
        // (front-end-chain). GameState+0x72 is BaseMap [M 0x182c60:68-70's clamp].
        if complete {
            // The front end's language table isn't loaded while a mission runs — fall back to the
            // level's own (the medal name drew as a raw "<0x2c7>" on the live end sequence).
            let lang = self.lang.clone().or_else(|| self.scene.sim.as_ref().and_then(|s| s.lang.clone()));
            if let Some(sim) = self.scene.sim.as_mut() {
                let b = sim.drones.mission.base_map;
                let base = if b == u32::MAX || b == 0 { self.level_code } else { b };
                let diff = sim.drones.progress.difficulty;
                fe.results_enter(&mut sim.drones.progress, base, 6, diff, lang.as_ref());
            }
        }
        self.fe = Some(fe);
        self.grab(false);
    }

    /// frontend-music: the front end's audio, one call per redraw beside `movie_tick`. Runs the
    /// page-change transitions (the manager's msg-0x44 re-runs the new page's 0x4c arm [M
    /// 0x21c858:345-346]), applies the queued [`menu_music::MusicCmd`]s to the sound driver, and
    /// pumps the IOP stand-in while the menus are up (the sim path's `sound_tick` only runs in
    /// the tick gate, which the front end skips). Silent under NF_SILENT: without an output
    /// device `build_sound` returns None and the commands just bookkeep on the state machine.
    fn menu_music_tick(&mut self) {
        // frontend-music (menu-audio-state): the transitions + the drain live in the extracted
        // `fe_music_step` (menu_music.rs) so the headless flows drive the SAME glue the window
        // runs. This wrapper only applies the commands to the sound driver.
        let cmds = if self.fe.is_some() {
            let mut cmds = Vec::new();
            if let Some(fe) = self.fe.as_mut() {
                cmds = menu_music::fe_music_step(fe, &mut self.fe_music_page);
            }
            cmds
        } else { Vec::new() };
        if !cmds.is_empty() {
            // `StartJingle` only sounds on the front end: with a mission's bank as the primary the
            // game's own `SFXStart(0x1d7/0x470)` finds no effect (the mission banks lack both ids —
            // [D the bank probe: e.g. SB_12 carries 0x1d8-0x1da but neither jingle]), so the
            // results page over a mission stays silent exactly like the game.
            let front_end_loaded = self.level_code == 0 || self.level_code == menu::FRONTEND_LEVEL;
            if let Some(s) = self.sound.as_mut() {
                for c in cmds {
                    match c {
                        menu_music::MusicCmd::StopJingles => { s.driver.remove(0, Some(0x1d7)); s.driver.remove(0, Some(0x470)); }
                        menu_music::MusicCmd::StartJingle(id) => {
                            if front_end_loaded {
                                self.menu_handle += 1;
                                let h = self.menu_handle;
                                s.batch(|drv, out| { drv.start(out, id, None, h, None); });
                            }
                        }
                        menu_music::MusicCmd::StartMusic(track, section) => {
                            match menu_music_files(&self.iso, track) {
                                Some((t, smf, ssd)) => { s.music_files = Some((t, smf, ssd)); s.music_pending.push((0, track, section)); }
                                None => eprintln!("menu music: track {track:#x} files not on the disc — no credits music"),
                            }
                        }
                        menu_music::MusicCmd::StopMusic => { s.music = None; }
                        menu_music::MusicCmd::MusicVolume(v) => { s.driver.music_master = v; }
                        menu_music::MusicCmd::FadeDownMusic => { s.driver.fade_down(true); }
                    }
                }
            }
        }
        // The pump: one driver update + output send per frame while the menus are up and the
        // audio is not held (the pause page's hold mirrors `MenuManager_Create(pause_sound=1)`).
        if !self.audio_paused {
            if let Some(s) = self.sound.as_mut() { s.batch(|_, _| {}); }
        }
    }

    /// The ONE movie glue, one call per redraw. Every movie the game plays routes through the one
    /// `self.movie` player, by the game's own start sites: the P_INTRO pair while the run is up
    /// [M `P_INTRO_Handler` 0x209e88], the GameFlow state-6 briefing/outro [M 0x1c98c8 case 6 +
    /// `psiStartBackgroundMovie` 0x1081a8], and the page's LOOPING aux backdrop on every page
    /// change [M `Manager_SendMessage` 0x21c858:338 — stop when the new page's aux is 0, restart
    /// when it differs from the playing one]. Plus the `psiMovieFinished` poll [M 0x1082e8] and
    /// the phase-2 resolve routing the host's level load [M ResetMap_Load 0x1d6360 LoadState 2].
    fn movie_tick(&mut self) {
        let movie_iso = self.movie_iso_path();
        let Some(fe) = self.fe.as_mut() else {
            // No front-end page is up: no page wants the looping backdrop — stop it. The game's
            // page-change arm stops the aux when the new page's aux is 0 [M 0x21c858:334-338];
            // the front end's close is that arm with no page at all (menu-audio-leaks finding 1:
            // the backdrop — the menu music — played on through gameplay and the pause). A
            // non-looping movie (a briefing's tail) keeps playing to its end.
            if self.movie.as_ref().is_some_and(|p| p.is_looping()) {
                if let Some(p) = self.movie.take() { let c = p.code(); p.stop(); eprintln!("movie: stopped {c:08x} (no front-end page)"); }
            }
            return;
        };
        // A pending end_to request from mission_ended rides the same path a menu pick does.
        if let Some(code) = fe.requested_level.take() {
            // A cold menu reload (the pause page's RESTART [M 0x204408:139 ResetMap_LevelToLoad(.., 0, 1)])
            // must not carry the current state over: the sim's cold flag makes the switch load fresh.
            if !fe.flow.warm_reset { if let Some(sim) = self.scene.sim.as_mut() { sim.drones.level_exit_cold = true; } }
            self.pending_level = Some(code);
            // net-match-host: the settings ride with the level (same as menu_after).
            if let Some(st) = fe.mp_match_settings.take() { self.pending_mp_settings = Some(st); }
        }
        let intro_up = fe.intro_playing && fe.flow.get_state() == menu::ST_MOVIE;
        if intro_up {
            // The intro owns the screen; the game stops any playing movie before starting the
            // next [M psiStartBackgroundMovie 0x001081a8 restarts the single BGFMV slot].
            let code = fe.intro_movie_code();
            let wrong = match &self.movie { Some(p) => p.code() != code || p.is_looping(), None => true };
            if wrong {
                if let Some(p) = self.movie.take() { p.stop(); }
                match movie::Player::start(&movie_iso, code) {
                    Ok(p) => {
                        self.movie = Some(p); eprintln!("movie: playing {code:08x} (intro)");
                        // frontend-music: the intro's 0x4c-equivalent removes the pair before the
                        // movie (menu_movie = 0) [M P_INTRO 0x209e88:42-44]; the movie's own audio
                        // replaces it.
                        fe.music.movie_started();
                    }
                    Err(e) => eprintln!("movie: {code:08x} not started: {e:?} — the headless countdown carries the intro"),
                }
            }
            // psiMovieFinished [M 0x1082e8 polled at 0x209e88]: the pair advances; after the
            // second movie the host shortens the headless countdown so the state pops at the real
            // end (the slim ISO's zeroed PSS files [L] leave the countdown alone to carry it).
            if self.movie.as_ref().is_some_and(|p| p.code() == code && p.finished()) {
                // frontend-music: each finished intro movie restarts the pair while it is down
                // [M 0x209e88:23-28 — the psiMovieFinished arm].
                fe.music.movie_finished();
                fe.intro_index += 1;
                if fe.intro_index >= menu::INTRO_MOVIES.len() { fe.flow.movie_frames = fe.flow.movie_frames.min(1); }
            }
        } else if fe.attract_playing {
            // attract-mode: P_ATTRACT's 0x4c starts the attract movie on the ONE BGFMV slot
            // [M psiStartBackgroundMovie 0x1081a8 — it replaces the playing backdrop; the page
            // aux loop resumes on the pop — 0x21c858:338]. The host polls psiMovieFinished
            // [M 0x21c3d8:37] and runs the 0x5f pop through `attract_movie_finished` when it
            // fires; a declined start reports finished at once [M `BGFMVPlaying == 0` — the
            // game's un-started movie — so the headless attract ends immediately].
            let code = fe.attract_movie;
            let wrong = match &self.movie { Some(p) => p.code() != code || p.is_looping(), None => true };
            if wrong {
                if let Some(p) = self.movie.take() { p.stop(); }
                match movie::Player::start(&movie_iso, code) {
                    Ok(p) => {
                        self.movie = Some(p); eprintln!("movie: playing {code:08x} (attract)");
                        // frontend-music: P_ATTRACT's 0x4c removes the menu-music pair before the
                        // movie (menu_movie = 0) [M 0x20a1f0:20-22] — the trailer's own soundtrack
                        // replaces it. Missing here, the menu music kept playing over the trailer
                        // (the user's "the cutscene that plays if you don't touch anything should
                        // have its own audio").
                        fe.music.movie_started();
                    }
                    Err(e) => {
                        // A declined start = the game's un-started movie: BGFMVPlaying == 0, so
                        // psiMovieFinished is true from tick one and the page closes at once
                        // [M 0x1082e8 + the 0x50 finish path 0x20a1f0:39-65].
                        eprintln!("movie: {code:08x} not started: {e:?} — the attract closes at once");
                        fe.attract_movie_finished();
                    }
                }
            }
            if self.movie.as_ref().is_some_and(|p| p.code() == code && p.finished()) {
                // The 0x50 finish arm restarts the pair while it is down [M 0x20a1f0:34-39].
                fe.music.movie_finished();
                fe.attract_movie_finished();
            }
        } else if fe.flow.get_state() == menu::ST_MOVIE {
            // psiStartBackgroundMovie replaces whatever the one BGFMV slot holds [M 0x1081a8] —
            // a looping page backdrop standing in the slot must yield to the state-6 movie
            // (menu-audio-leaks: the playtest's menu music under the EA logo).
            if self.movie.as_ref().is_some_and(|p| p.is_looping()) {
                if let Some(p) = self.movie.take() { let c = p.code(); p.stop(); eprintln!("movie: stopped {c:08x} (state 6 replaces the backdrop)"); }
            }
            match &mut self.movie {
                Some(p) => {
                    // BGFMVPlaying follows the player: 1 while it decodes, 0 at the end of file —
                    // state 6 pops through the psiMovieFinished arm on the next main_tick
                    // [M 0x1082e8 + 0x1c98c8 case 6].
                    fe.flow.movie_playing = !p.finished();
                }
                None => {
                    let code = fe.flow.movie_code;
                    if code != 0 {
                        match movie::Player::start(&movie_iso, code) {
                            Ok(p) => {
                                eprintln!("movie: playing {code:08x} (state 6)");
                                fe.flow.movie_playing = true; // BGFMVPlaying = 1 [M 0x1081a8]
                                self.movie = Some(p);
                            }
                            Err(e) => eprintln!("movie: {code:08x} not started: {e:?} — the headless countdown stands in"),
                        }
                    }
                }
            }
        } else if fe.flow.movie_pending {
            // A movie-wrapped load is on its way: the looping page backdrop yields the single
            // player slot to it [M psiStartBackgroundMovie replaces the playing movie].
            if let Some(p) = self.movie.as_ref() {
                if p.is_looping() {
                    let code = p.code();
                    if let Some(p) = self.movie.take() { p.stop(); }
                    eprintln!("movie: stopped {code:08x} (a briefing is pending)");
                }
            }
            if self.movie.is_none() && fe.flow.get_state() == menu::ST_MAPLOAD {
                // Early start: the decoder works through the fade and the rest of the menu so
                // state 6 opens on the first frame instead of a black screen (the game's IPU
                // starts streaming instantly [M 0x1081a8]).
                let code = fe.flow.level_to_load;
                if matches!((code >> 20) & 0xff, 0x71 | 0x73) {
                    match movie::Player::start(&movie_iso, code) {
                        Ok(p) => { self.movie = Some(p); eprintln!("movie: decoding {code:08x} ahead of state 6"); }
                        Err(e) => eprintln!("movie: {code:08x} not started: {e:?} — the headless countdown stands in"),
                    }
                }
            }
        } else {
            // The page's aux loop [M 0x21c858:338]: stop when the page has no aux, (re)start when
            // the playing code differs from the page's.
            let aux = menu::aux_movie_for(fe.page_id());
            // A skipped attract (the Enter skip ends it inside the FrontEnd, not through the
            // finished poll above) leaves the trailer's player here: its 0x50 arm restarts the
            // menu-music pair the same way the natural end does [M 0x20a1f0:34-39].
            if self.movie.as_ref().is_some_and(|p| !p.is_looping() && matches!(p.code(), menu::ATTRACT_MOVIE_A | menu::ATTRACT_MOVIE_B)) {
                fe.music.movie_finished();
            }
            let wrong = match &self.movie {
                Some(p) => aux == 0 || p.code() != aux || !p.is_looping(),
                None => aux != 0,
            };
            if wrong {
                if let Some(p) = self.movie.take() {
                    let code = p.code();
                    p.stop();
                    eprintln!("movie: stopped {code:08x}");
                }
                if aux != 0 {
                    match movie::Player::start_looping(&movie_iso, aux) {
                        Ok(p) => { self.movie = Some(p); eprintln!("movie: looping {aux:08x} (page backdrop)"); }
                        Err(e) => eprintln!("movie: {aux:08x} not started: {e:?} — the dark plate stands in"),
                    }
                }
            }
        }
        // The phase-2 resolve: the movie code became the level to load [M 0x1d6360 LoadState 2].
        if let Some(resolved) = fe.flow.phase2_resolved.take() {
            self.pending_level = Some(resolved);
            if resolved != menu::FRONTEND_LEVEL {
                // The briefing's return: the real level loads and the front end closes — the game
                // leaves the menu system and Game_Draw takes over.
                self.close_front_end();
            }
            // The outro's return: the front end stays and the results page reopens on the map
            // (phase 2 set flow.page = 0x40000036 [M]; fe.page follows below in the hud build).
        }
    }

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

    // -------------------------------------------------------------------------
    // fix-menu-movies2: the front end's background movies ride the ONE player/decoder path
    // (movie::Player over MovieStream's closed-GOP worker) and the ONE full-screen plane in
    // gpu.rs — the game plays them all through psiStartBackgroundMovie 0x001081a8 /
    // psiStopBackgroundMovie 0x00108328 (the page switch plays/stops the page's aux loop
    // [M Manager_SendMessage 0x21c858:338]; P_INTRO runs the intro pair [M 0x209e88]).
    // -------------------------------------------------------------------------

    /// The image the movies load from: NF_MOVIE_ISO (a full retail image) over the running one —
    /// the same precedence movie.rs's tests use [L the on-foot ISO's /MOVIES/30_FPS files are
    /// zero-length stubs].
    fn movie_iso_path(&self) -> String { std::env::var("NF_MOVIE_ISO").unwrap_or_else(|_| self.iso.clone()) }

    /// A click on the dev menu (F2 open): hit-test the mouse against the menu rows and apply the action (WORKLOG 0081,
    /// pages in fix-ingame-dev-menu). The hit-test shares the draw fn's layout (`dev::layout_rows`).
    fn dev_menu_click(&mut self) {
        let aspect = self.gfx.as_ref().map_or(1.0, |g| g.config.width as f32 / g.config.height as f32);
        let mouse = match self.dev.as_deref_mut() {
            None => return,
            Some(d) => d.mouse,
        };
        let Some(idx) = dev::menu_hit(self.dev.as_deref().unwrap(), Some(&self.scene), mouse.0, mouse.1, aspect) else { return };
        // B-9: a click also moves the keyboard selection to the clicked row.
        if let Some(d) = self.dev.as_deref_mut() { d.menu_sel = idx; }
        let Some(action) = dev::menu_action(self.dev.as_deref().unwrap(), Some(&self.scene), idx) else { return };
        self.apply_menu_action(action);
    }

    /// B-9: apply a dev-menu action and set the on-screen result message. Shared by mouse-click and keyboard
    /// (Enter) so both produce the same feedback. The result stays ~3 s (180 ticks).
    /// Apply the dev panel's queued commands and keep its catalog current (every 30 frames, and on level change).
    /// A numbered screenshot plus its spot: `nf-shots/NNN-<level>.png` and `.txt` holding the `--bond x,y,z,yaw,pitch` and
    /// level to re-render the view headless (F12 and the panel's "Screenshot + log spot").
    fn spot_shot(&mut self) -> String {
        let dir = std::path::Path::new("nf-shots");
        if let Err(e) = std::fs::create_dir_all(dir) { return format!("screenshot: {e}"); }
        let level = self.scene.sim.as_ref().map_or(0, |s| s.level);
        let n = (1..10000).find(|n| !dir.join(format!("{n:03}-{level:08x}.png")).exists()).unwrap_or(0);
        let png = dir.join(format!("{n:03}-{level:08x}.png"));
        let spot = self.scene.sim.as_ref().map(|s| (s.bond.pos, s.bond.yaw, s.bond.pitch)).map(|(p, y, pt)| format!("--bond {:.2},{:.2},{:.2},{:.3},{:.3}", p[0], p[1], p[2], y, pt))
            .unwrap_or_else(|| { let p = self.cam.pos; format!("camera {:.2},{:.2},{:.2}", p.x, p.y, p.z) });
        let line = format!("{level:08x} {spot} frame {}", self.anim_time as u32);
        let _ = std::fs::write(png.with_extension("txt"), format!("{line}\n"));
        let fe_menu = self.fe.as_ref().and_then(|fe| self.scene.hud.as_ref().map(|t| {
            let lang = self.lang.as_ref().or_else(|| self.scene.sim.as_ref().and_then(|s| s.lang.as_ref()));
            menu::menu_overlay(fe, &t.font, &t.font_header, lang, &self.scene.hud_textures, 1280.0 / 720.0)
        }));
        match shot(&self.scene, &self.cam, &png.to_string_lossy(), self.choice, self.anim_time, self.opts, self.notex, self.nofog, fe_menu.as_ref(), None) {
            Ok(()) => format!("saved {} ({line})", png.display()),
            Err(e) => format!("screenshot failed: {e} ({line})"),
        }
    }

    /// fix-weapon-assets: true when the weapon's first-person assets are absent (its `+0xDC` skin
    /// is in no merged archive) — the view model would not build and the view scripts would not
    /// resolve, exactly the user's symptom on any weapon the level doesn't carry.
    fn weapon_needs_assets(&self, w: u8) -> bool {
        self.scene.sim.as_ref().is_some_and(|sim| {
            let fp = sim.weapons.first_person(w as usize);
            fp != 0 && !sim.skins.contains_key(&fp)
        })
    }

    fn poll_panel(&mut self) {
        let Some(h) = self.panel.clone() else { return };
        let queued: Vec<String> = h.lock().map(|mut g| std::mem::take(&mut g.queue)).unwrap_or_default();
        let mut changed = !queued.is_empty();
        for q in queued {
            let q = panel::query(&q);
            let what = q.iter().find(|(k, _)| k == "do").map(|(_, v)| v.clone()).unwrap_or_default();
            let status = match what.as_str() {
                "god" => match self.dev.as_deref_mut() { Some(d) => { d.god_mode = !d.god_mode; format!("god mode {}", if d.god_mode { "on" } else { "off" }) } None => "no dev mode".into() },
                "shot" => self.spot_shot(),
                "ammo" => match self.dev.as_deref_mut() { Some(d) => { d.infinite_ammo = !d.infinite_ammo; format!("infinite ammo {}", if d.infinite_ammo { "on" } else { "off" }) } None => "no dev mode".into() },
                _ => {
                    let (status, switch) = panel::apply(&mut self.scene, &q);
                    if let Some(w) = switch {
                        self.weapon_switch_to = Some(w);
                        // fix-weapon-assets: the level archive lacks the weapon — answer the give
                        // with the warm asset reload (the tick loop's pending_weapon_assets arm).
                        if self.weapon_needs_assets(w) { self.pending_weapon_assets = Some(w); }
                    }
                    status
                }
            };
            eprintln!("[panel] {status}");
            if let Ok(mut g) = h.lock() { g.status = status; }
        }
        let level = self.scene.sim.as_ref().map_or(0, |s| s.level);
        self.panel_frames += 1;
        if changed || level != self.panel_level || self.panel_frames % 30 == 0 {
            changed = true;
            self.panel_level = level;
        }
        if changed { let c = panel::catalog(&self.scene); if let Ok(mut g) = h.lock() { g.catalog = c; } }
    }

    /// B-9: apply a dev-menu action and set the on-screen result message. Shared by mouse-click and keyboard
    /// (Enter) so both produce the same feedback. The result stays ~3 s (180 ticks).
    ///
    /// fix-ingame-dev-menu: the whole action match moved into `dev::apply_menu_action_to_scene` — the REAL path,
    /// shared with the headless menu-flow tests (`tests/dev_menu_flow.rs`). This shell only applies what needs the
    /// windowed App: the weapon switch request, the pending level, the viewer cel reset, the noclip state and the
    /// screenshot. Every give/spawn/clear command still routes through `panel::apply` (the shared command layer).
    fn apply_menu_action(&mut self, action: dev::MenuAction) {
        let eye = [self.cam.pos.x, self.cam.pos.y, self.cam.pos.z];
        let (out, msg) = match self.dev.as_deref_mut() {
            None => return,
            Some(d) => {
                let out = dev::apply_menu_action_to_scene(d, &mut self.scene, action, eye, self.cam.yaw, self.noclip);
                let msg = out.status.clone();
                (out, msg)
            }
        };
        if let Some(w) = out.switch_to {
            self.weapon_switch_to = Some(w);
            // fix-weapon-assets: same as poll_panel — a give the level archive can't dress yet.
            if self.weapon_needs_assets(w) { self.pending_weapon_assets = Some(w); }
        }
        if let Some(id) = out.pending_level { self.pending_level = Some(id); }
        if let Some(on) = out.noclip_to { self.set_noclip(on); }
        if out.shot { let m = self.spot_shot(); eprintln!("[spot] {m}"); }
        if out.viewer_cel_reset {
            // The game's own placement clears the viewer's cel [M 0x1b9920] — the next
            // `Camera_CheckLocation` re-finds it at Bond's position.
            if let Some(g) = self.gfx.as_mut() { g.renderer.viewer_cel = crate::scene::ViewerCel::default(); }
        }
        // "spawn at crosshair" (`Dev::spawn_guard`) adds a Character; the renderer's skin layout was
        // built for the old count and `skin_palette` indexed past it (a panic on the next frame — the
        // main agent's playtest-check dry run). Rebuild the renderer like the weapon-asset reload does.
        if let Some(g) = self.gfx.as_mut() {
            if g.renderer.skin_layout.char_base.len() != self.scene.characters.len() {
                g.renderer = gpu::Renderer::new(&g.device, &g.queue, g.config.format, &self.scene, self.notex, self.nofog);
            }
        }
        if let Some(d) = self.dev.as_deref_mut() { d.result_msg = Some(msg); d.result_timer = 180; }
    }

    /// One dev-window event (devwin.rs): navigation/filter in the window, actions through the shared path.
    fn dev_window_event(&mut self, event: WindowEvent) {
        let Some(dpi) = self.devwin.as_ref().map(|w| w.window.scale_factor() as f32) else { return };
        match &event {
            WindowEvent::Resized(s) => {
                if let (Some(w), Some(g)) = (self.devwin.as_mut(), self.gfx.as_ref()) { w.resize(&g.device, s.width, s.height); }
                return;
            }
            WindowEvent::ScaleFactorChanged { .. } => { if let Some(w) = self.devwin.as_ref() { w.window.request_redraw(); } return; }
            WindowEvent::RedrawRequested => {
                if let (Some(w), Some(g), Some(d)) = (self.devwin.as_mut(), self.gfx.as_ref(), self.dev.as_deref()) { w.render(&g.device, &g.queue, &self.scene, d); }
                return;
            }
            _ => {}
        }
        let (Some(w), Some(d)) = (self.devwin.as_mut(), self.dev.as_deref_mut()) else { return };
        let size = w.window.inner_size();
        let rows = dev::menu_items(d, Some(&self.scene));
        let req = devwin::handle(&event, d, &mut w.ui, &rows, (size.width as f32, size.height as f32), dpi);
        match req {
            devwin::Request::None => {}
            devwin::Request::Redraw => w.window.request_redraw(),
            devwin::Request::Hide => w.window.set_visible(false),
            devwin::Request::Apply(a) => {
                w.window.request_redraw();
                self.apply_menu_action(a);
                if let Some(m) = self.dev.as_deref().and_then(|d| d.result_msg.clone()) { eprintln!("[dev] {m}"); }
            }
        }
    }

    /// The N key's noclip toggle, shared with the Toggles page row (fix-ingame-dev-menu).
    fn set_noclip(&mut self, on: bool) {
        self.noclip = on;
        // Back on foot: Bond where the free camera is.
        if !self.noclip {
            if let Some(sim) = self.scene.sim.as_mut() {
                sim.bond = nf_game::player::Bond::stand_at(&sim.world, [self.cam.pos.x, self.cam.pos.y - 0.7, self.cam.pos.z], self.cam.yaw);
            }
            // The game's own placement clears the viewer's cel [M 0x1b9920] — the
            // next frame re-finds at Bond's position.
            if let Some(g) = self.gfx.as_mut() { g.renderer.viewer_cel = crate::scene::ViewerCel::default(); }
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
        // A GPU validation error is reported, not fatal (wgpu's default handler panics).
        device.on_uncaptured_error(Arc::new(|e| eprintln!("GPU error: {e}")));
        let size = window.inner_size();
        let mut config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).expect("surface config");
        // The GS writes its colours straight to the framebuffer (no gamma step), and the shaders output those values as
        // they are — like the headless `--shot` path's Rgba8Unorm. An sRGB surface would encode them again and wash the
        // picture out, so use the plain (non-sRGB) variant of the surface's format.
        config.format = config.format.remove_srgb_suffix();
        // Vsync at the display rate by default (a 120/144 Hz display gets 120/144 fps); `--fps N` turns it off and caps.
        config.present_mode = if self.fps_cap.is_some() { wgpu::PresentMode::AutoNoVsync } else { wgpu::PresentMode::AutoVsync };
        surface.configure(&device, &config);
        let depth = gpu::depth_view(&device, config.width, config.height);
        let renderer = gpu::Renderer::new(&device, &queue, config.format, &self.scene, self.notex, self.nofog);
        // `--dev`: open the dev window beside the game (devwin.rs).
        if self.dev.is_some() && self.devwin.is_none() {
            self.devwin = devwin::DevWindow::new(el, &window, &instance, &adapter, &device, &queue, &self.scene);
            if self.devwin.is_some() {
                // The window's tabs are the index: open on Weapons.
                if let Some(d) = self.dev.as_deref_mut() { if d.page == 0 { d.page = devwin::TAB_PAGES[0]; } }
                eprintln!("dev window open beside the game (F2 raises it)");
            }
        }
        self.gfx = Some(Gfx { window, instance, minimized: size.width == 0 || size.height == 0, surface, device, queue, config, depth, renderer, adapter });
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
        // Mouse wheel: with the dev menu open it moves the menu's selection; otherwise weapon
        // switching (next/prev). Up = previous row / next weapon (−1 direction, matching the old
        // weapon-cycle mapping: wheel up = next).
        if let DeviceEvent::MouseWheel { delta: MouseScrollDelta::LineDelta(_x, y) } = event {
            if self.fe.is_some() && !self.dev.as_deref().is_some_and(|d| d.menu) {
                if let Some(a) = menu::wheel_scroll_action(y) { self.menu_input(a); }   // up = previous row
            } else if self.dev.as_deref().is_some_and(|d| d.menu) {
                if let Some(d) = self.dev.as_deref_mut() {
                    let n = dev::menu_items(d, Some(&self.scene)).len();
                    if n > 0 {
                        if y > 0.0 { d.menu_sel = d.menu_sel.saturating_sub(1); } else { d.menu_sel = (d.menu_sel + 1).min(n - 1); }
                    }
                }
            } else if self.grabbed && !self.noclip {
                // microcam-wheel-zoom: while the right button (aim) is held and the held weapon can
                // zoom, the wheel steps the zoom instead of switching [S the 2026-10-15 playtest:
                // "have the scroll wheel do the zoom while you're right-clicking to aim in. And if
                // you're not right-clicking, of course, weapon switch"]. Zoomable = the game's own
                // scope flag +0x68 & 0x40 with a magnification above 1 [M weapon_data; the sim's
                // zoom block gates on the same two]. Otherwise the wheel keeps switching weapons.
                let zoomable = self.scene.sim.as_ref().is_some_and(|s| {
                    wheel_steps_zoom(self.zoom, s.weapons.flags(s.gun.weapon as usize), s.weapons.zoom(s.gun.weapon as usize))
                });
                if zoomable {
                    if y > 0.0 { self.zoom_cycle = 1; }
                    else if y < 0.0 { self.zoom_cycle = -1; }
                } else if y > 0.0 { self.weapon_cycle = 1; }
                else if y < 0.0 { self.weapon_cycle = -1; }
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // The dev window's events never reach the game (devwin.rs): its clicks/keys drive the dev menu only, and the
        // game keeps running.
        if self.devwin.as_ref().is_some_and(|w| w.id() == id) {
            self.dev_window_event(event);
            return;
        }
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
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if self.fe.is_some() && self.async_load.is_none() && !self.dev.as_deref().is_some_and(|d| d.menu) => {
                if state == ElementState::Pressed {
                    if let (Some((x, y)), Some(fe)) = (self.menu_pointer, self.fe.as_mut()) {
                        if fe.flow.get_state() == menu::ST_MOVIE { self.menu_input(menu::Action::Confirm); }
                        else if fe.mouse_click(x, y) { self.menu_after(); }
                    }
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } if self.fe.is_some() && self.async_load.is_none() && !self.dev.as_deref().is_some_and(|d| d.menu) => {
                self.menu_input(menu::MOUSE_RIGHT_ACTION);   // menu-mouse-parity: right press = △ = Back
            }
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
            WindowEvent::CursorMoved { position, .. } if self.fe.is_some() && self.async_load.is_none() && !self.dev.as_deref().is_some_and(|d| d.menu) => {
                self.menu_pointer = self.menu_space(position.x, position.y);
                if let (Some((x, y)), Some(fe)) = (self.menu_pointer, self.fe.as_mut()) {
                    if fe.mouse_move(x, y) { self.menu_after(); }   // the hover's focus-move sound
                }
            }
            WindowEvent::CursorMoved { position, .. } => if let (Some(g), Some(d)) = (&self.gfx, self.dev.as_deref_mut()) {
                if d.menu {
                    let (w, h) = (g.config.width as f32, g.config.height as f32);
                    d.mouse = ((position.x as f32 / w) * 2.0 - 1.0, 1.0 - (position.y as f32 / h) * 2.0);
                }
            },
            // No manual-aim mode with the mouse (PLAN.md D-002): the mouse aims directly; `aiming` stays for a pad.
            WindowEvent::KeyboardInput { event, .. } => if let PhysicalKey::Code(k) = event.physical_key {
                // The skip is the game's `Input_Action(-1, 0x19, 1)` read in `SP_Update` state 2 →
                // `Script_FFwd(cur, 1)` unless the script is a death NIS [M SP_Update 0x1dbda8:129-141,
                // Script_FFwd 0x1dac78]. Action 0x19 is bound to raw pad mask 0x840 — R1 (0x800) or
                // D-pad DOWN (0x040), pad 0 only [M psiInput_MapInputs 0x10cd80:415-417] — so MOVEMENT
                // NEVER SKIPS (the user's W press skipped the 07000007 Kimiko scene, 2026-10-12
                // playtest). The port's keyboard stand-ins for that action: ArrowDown (the D-pad-down
                // key) and R (R1). The viewer's dedicated keys keep their own functions and never skip;
                // `--skip-cutscenes` (lib.rs 1556) still skips every tick as a dev override.
                if event.state == ElementState::Pressed && !event.repeat
                    && cutscene_skip_key(k)
                    && self.scene.sim.as_ref().is_some_and(|s| s.drones.scripts.cutscene_playing()) {
                    if let Some(sim) = self.scene.sim.as_mut() { sim.drones.scripts.skip_request = true; }
                }
                // fix-menus: while a menu page is up, the keys drive the menu and never the game.
                // F12 stays the viewer's screenshot — fix-pause-menu: it previously fell through to
                // the menu branch (key_action(F12) is None → early return), so the user could not
                // capture the pause menu (the task's report request).
                if event.state == ElementState::Pressed && k == KeyCode::F12 {
                    let m = self.spot_shot();
                    eprintln!("[spot] {m}");
                    return;
                }
                // No menu input while a level load runs: the loading screen owns the frame and the
                // game's main loop runs no menu manager inside ResetMap_Load [M 0x1d6360 state 3].
                // Keys pressed there used to reach the still-open front end — a second Enter on
                // the MP confirm page started a second load of the same arena (playtest 1015-2231).
                if event.state == ElementState::Pressed && self.fe.is_some() && self.async_load.is_some() { return; }
                if event.state == ElementState::Pressed && self.fe.is_some() {
                    // net-online-lobby: the join address page takes the raw keys FIRST — digits,
                    // dots and the colon type, Backspace deletes, Ctrl+C/Ctrl+V work the page's
                    // own clipboard — and every other press is swallowed, so the letter rows
                    // (W/A/S/D are the menu's own keys) never fire while an address is typed
                    // [S docs/systems/net-online-lobby.md §2.2]. Enter/Escape still confirm and
                    // back out below for the non-typing keys.
                    let ctrl = self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight);
                    // The host page's port row types digits too (Backspace deletes); its other keys
                    // (arrows, Enter, Esc) still drive the page below.
                    if let Some(fe) = self.fe.as_mut().filter(|fe| fe.page == menu::Page::MpHost && fe.cursor == 0 && !fe.mp_hosting) {
                        let typed = match k {
                            KeyCode::Backspace => fe.mp_net_backspace(),
                            _ => menu::net_char_key(k).is_some_and(|c| fe.mp_net_type(c)),
                        };
                        if typed { return; }
                    }
                    let typing = self.fe.as_ref().is_some_and(|fe| fe.page == menu::Page::MpNetJoin);
                    if typing {
                        let fe = self.fe.as_mut().unwrap();
                        match k {
                            KeyCode::Backspace => { fe.mp_net_backspace(); }
                            KeyCode::KeyV if ctrl => { let clip = fe.mp_net_clip.clone(); fe.mp_net_paste(&clip); }
                            KeyCode::KeyC if ctrl => { fe.mp_net_copy(); }
                            KeyCode::Enter | KeyCode::Space => { if fe.action(menu::Action::Confirm) { self.menu_after(); } }
                            KeyCode::Escape => { if fe.action(menu::Action::Back) { self.menu_after(); } }
                            _ => if let Some(c) = menu::net_char_key(k) { fe.mp_net_type(c); }
                        }
                        return;
                    }
                    // Windows key auto-repeat never confirms, backs out or skips: the game's actions
                    // are edge-triggered [M Input_Action(.., 1)], and a held Enter's repeats skipped
                    // the briefing the moment it started (playtest 1015-2231 "no intro"). Held
                    // arrows still scroll.
                    if let Some(a) = menu::key_action(k) {
                        if event.repeat && matches!(a, menu::Action::Confirm | menu::Action::Back) { return; }
                        self.menu_input(a);
                    }
                    return;
                }
                // fix-ingame-dev-menu: while the dev menu is open, the keys drive the menu and never the game
                // (the same rule the front-end pages follow). The sim is paused under the open menu
                // (`paused_tick_count`), so nothing can race the menu's actions. F1/F11/F12 keep their viewer
                // functions; Esc/F2 close and re-grab the mouse.
                if self.dev.as_deref().is_some_and(|d| d.menu) {
                    if event.state != ElementState::Pressed { return; }
                    match k {
                        KeyCode::F1 => if let Some(d) = self.dev.as_deref_mut() { d.overlay = !d.overlay; },
                        KeyCode::F2 => {
                            if let Some(d) = self.dev.as_deref_mut() { d.menu = false; }
                            if self.scene.sim.is_some() { self.grab(true); }
                        }
                        KeyCode::F11 => if let Some(d) = self.dev.as_deref_mut() { d.step = true; },
                        KeyCode::F12 => { let m = self.spot_shot(); eprintln!("[spot] {m}"); }
                        KeyCode::Escape => {
                            if let Some(d) = self.dev.as_deref_mut() { d.menu = false; }
                            if self.scene.sim.is_some() { self.grab(true); }
                        }
                        KeyCode::PageUp => if let Some(d) = self.dev.as_deref_mut() { dev::menu_key(d, dev::MenuKey::PageUp); },
                        KeyCode::PageDown => if let Some(d) = self.dev.as_deref_mut() { dev::menu_key(d, dev::MenuKey::PageDown); },
                        KeyCode::ArrowUp => if let Some(d) = self.dev.as_deref_mut() { dev::menu_key(d, dev::MenuKey::Up); },
                        KeyCode::ArrowDown => if let Some(d) = self.dev.as_deref_mut() { dev::menu_key(d, dev::MenuKey::Down); },
                        KeyCode::Backspace => if let Some(d) = self.dev.as_deref_mut() { dev::menu_key(d, dev::MenuKey::Backspace); },
                        KeyCode::Enter => {
                            // Compute the action while holding the dev borrow, drop it, then apply.
                            let action = self.dev.as_deref_mut().and_then(|d| dev::menu_activate(d, Some(&self.scene)));
                            if let Some(a) = action { self.apply_menu_action(a); }
                        }
                        _ => { if let Some(c) = dev_filter_char(k) { if let Some(d) = self.dev.as_deref_mut() { dev::menu_type(d, c); } } }
                    }
                    return;
                }
                match (k, event.state) {
                    (KeyCode::Escape, ElementState::Pressed) => {
                        if self.grabbed {
                            self.grab(false);
                            // Esc in a mission opens the pause page [S: the game pauses on Start].
                            if self.scene.sim.is_some() && self.level_code != 0 && self.level_code != menu::FRONTEND_LEVEL {
                                let base = self.scene.sim.as_ref().map(|s| s.drones.mission.base_map).filter(|&b| b != 0 && b != u32::MAX).unwrap_or(self.level_code);
                                let fe = self.fe.get_or_insert_with(menu::FrontEnd::new);
                                // frontend-music: the fresh manager still meets the game's
                                // statics (menu-audio-state — the stash from the front-end close;
                                // the quit-to-front-end load must flip to the OTHER bank).
                                Self::inherit_music_statics(fe, self.music_bank_flip, self.music_loaded_bank, self.music_master);
                                fe.flow.current_level = self.level_code;
                                // RESTART reloads the running mission's base map [M 0x204408:138-141
                                // Mission_BaseMapHCode()]: a level started straight from the command line never
                                // went through the mission list, so pick the row from the mission itself (row 0
                                // is the driving Paris Prelude 0x09000001, which the old default reloaded).
                                if let Some(i) = menu::CAMPAIGN.iter().position(|c| c.3 == base) { fe.mission = i; }
                                // The GameState rides the runner state while a mission plays [M
                                // GameFlow_Main 0x1c98c8 case 2 — the game's stack sits on 2 under
                                // Mission_Update], so the pause arms' ResetMap_LevelToLoad accepts
                                // (its state gate 0x1d3800:11-17 wants {2,7,8,0xd,0xe}).
                                fe.flow.stack = vec![menu::ST_RUNNER];
                                fe.open_pause();
                            }
                        } else { el.exit() }
                    }
                    (KeyCode::F12, ElementState::Pressed) => { let m = self.spot_shot(); eprintln!("[spot] {m}"); }
                    // fix-menus demo hooks: F8 = mission complete, F9 = mission failed (the results page).
                    (KeyCode::F8, ElementState::Pressed) => self.demo_results(true),
                    (KeyCode::F9, ElementState::Pressed) => self.demo_results(false),
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
                    (KeyCode::F2, ElementState::Pressed) => if self.devwin.is_some() { self.grab(false); if let Some(w) = self.devwin.as_ref() { w.show(); } } else if let Some(d) = self.dev.as_deref_mut() { d.menu = !d.menu; if d.menu { self.grab(false); } else if self.scene.sim.is_some() { self.grab(true); } },
                    (KeyCode::F11, ElementState::Pressed) => if let Some(d) = self.dev.as_deref_mut() { if d.time_scale <= 0.0 { d.step = true; } },
                    (KeyCode::F3, ElementState::Pressed) => self.opts.reinforcements = !self.opts.reinforcements,
                    (KeyCode::KeyF, ElementState::Pressed) => self.opts.firing = !self.opts.firing,
                    (KeyCode::KeyN, ElementState::Pressed) => { let on = !self.noclip; self.set_noclip(on); }
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
                    // Gadgets (G): the cycle is live — the weapon_data's +0x04 selectable flag is 1 for
                    // the gadget ids [D ACTION.ELF weapon_data] and the switch/equip path is tested
                    // (gadget_select_live). The gadgets' fire-side substates land per gadget.
                    (KeyCode::KeyG, ElementState::Pressed) => if self.grabbed && !self.noclip { self.gadget_cycle = 1 },
                    // The fire-mode button is the game's ○ (action 12); the keyboard binding is B (the user's choice),
                    // which the catch-all below already lands in `just` (read in the Input build:
                    // `mode_switch`) [M Player_Weapon 0x1a24c8: `Input_Action(slot, 0xc, 4)`].
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
                // fix-movie-playback: the movie glue runs before the load path so a phase-2
                // resolve lands in the same frame the state 6 popped.
                self.movie_tick();
                // frontend-music: the menus' own audio (the jingle pair, the credits track), the
                // page-change transitions and the per-frame pump.
                self.menu_music_tick();
                // fix-weapon-assets: a dev-menu/panel give of a weapon the level archive lacks.
                // The game never gives cross-level weapons, so there is no game path to mirror —
                // the port answers the give with a warm reload of the SAME level with the weapon's
                // assets merged (`LoaderLoad` of the same archive + the merge), re-places Bond
                // where he stood and re-issues the switch. One hitch per newly-given weapon.
                if let Some(w) = self.pending_weapon_assets.take() {
                    if self.weapon_needs_assets(w) {
                        let (pos, yaw) = self.scene.sim.as_ref().map(|s| (s.bond.pos, s.bond.yaw)).unwrap_or(([0.0; 3], 0.0));
                        let save = self.scene.sim.as_ref().map(|sim| nf_game::player::RamSave::capture(
                            &sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
                            .with_mission(&sim.drones.mission));
                        self.entry_save = save.clone();
                        let name = level_archive_name(self.scene.sim.as_ref().map_or(0, |s| s.level));
                        eprintln!("weapon assets: {w:#04x} is not on this level — warm reload with its assets");
                        match load_with_carry_weapons_difficulty(&self.iso, &name, self.all, save.as_ref(), &[w], self.selected_difficulty) {
                            Ok(mut scene) => {
                                self.sound = build_sound(&mut scene);
                                if let Some(sim) = scene.sim.as_mut() {
                                    sim.bond = nf_game::player::Bond::stand_at(&sim.world, pos, yaw);
                                    sim.gun.switch_request = w;
                                }
                                self.scene = scene;
                                if let Some(g) = &mut self.gfx {
                                    g.renderer = gpu::Renderer::new(&g.device, &g.queue, g.config.format, &self.scene, self.notex, self.nofog);
                                }
                                self.weapon_switch_to = Some(w);
                            }
                            Err(e) => eprintln!("weapon assets: reload failed: {e}"),
                        }
                    }
                }
                // Bug 5: level transition. A `LOAD_LEVEL` (0xE8) trigger set `pending_level`; reload the scene
                // with the next archive (`ResetMap_LevelToLoad` 0x1d3800 → `LoaderLoad` 0x1d4a00).
                if let Some(next) = self.pending_level.take() {
                    // A movie-wrapped load plays its movie FIRST: the game loads the empty backdrop
                    // 0x7000500, the briefing/outro runs over it, and only phase 3 loads the real
                    // level [M 0x1d6360 phases 0/3]. Hold the request while the flow is heading to
                    // state 6; the phase-2 resolve re-issues it (movie_tick).
                    if self.fe.as_ref().is_some_and(|fe| fe.flow.movie_pending) {
                        self.pending_level = Some(next);
                    } else {
                        // async-level-load: the build runs on a worker; the frames below redraw
                        // the loading screen with the dots until the scene is ready.
                        self.start_async_load(next);
                    }
                }
                // async-level-load: while a worker builds the next scene, this thread is the
                // game's main thread inside `ResetMap_Load` 0x1d6360 state 3 — nothing else of
                // the frame runs (the game's vblank interrupt alone animates the dots,
                // `VBlankInt` 0x124088). Each frame redraws the screen with `dots_lit` advanced
                // and ends; the frame the result arrives, `poll_async_load` swaps the scene in
                // (the renderer's GPU rebuild stays on this thread) and the frame falls through
                // to normal play.
                if self.async_load.is_some() && !self.poll_async_load() { return; }
                let dt = self.last.elapsed().as_secs_f32().min(0.1);
                self.last = std::time::Instant::now();
                // The Toggles page's fps display (wall-clock; a dev readout, never in the Sim).
                if let Some(d) = self.dev.as_deref_mut() {
                    if dt > 0.0 { d.fps = if d.fps <= 0.0 { 1.0 / dt } else { d.fps * 0.9 + (1.0 / dt) * 0.1 }; }
                }
                let speed = if self.keys.contains(&KeyCode::ShiftLeft) { 40.0 } else { 8.0 } * dt;
                let fwd = self.cam.dir();
                let right = self.cam.right();
                let mut mv = Vec3::ZERO;
                for (k, d) in [(KeyCode::KeyW, fwd), (KeyCode::KeyS, -fwd), (KeyCode::KeyD, right), (KeyCode::KeyA, -right), (KeyCode::Space, Vec3::Y), (KeyCode::ControlLeft, -Vec3::Y)] {
                    if self.keys.contains(&k) { mv += d; }
                }
                if self.noclip || self.scene.sim.is_none() { self.cam.pos += mv * speed; }
                // One animation frame per 60 Hz game update (FRAME_RATE_MUL = 1), docs/formats/animation.md.
                self.ticked = false;
                self.poll_panel();
                // fix-pause-audio: the pause page holds the sound. `MenuManager_Create` with
                // `pause_sound` runs `SFXPause` (batch op 9) + `SFXPauseAllStreams` (op 0x16) and
                // `MenuManager_Delete` runs the unpause pair [M 0x21b908:121-124, 0x21c2b0:11-22];
                // GameState+0x50 is read by NO sound code — the pause rides these ops. The port's
                // hook is the pause page (the in-game pause manager's create) or the viewer's P
                // pause — NOT the front end's own pages: its manager passes pause_sound = 0 and
                // the jingle pair keeps playing over every page [M frontend-music].
                // `set_paused` reaches the output at once because `sound_tick` (and the
                // batch) only run inside the tick gate below. The fail/results page opens with
                // `pause_sound` = 0 in the game (sound keeps playing [M Mission_Update 0x182c60:78])
                // — the port's demo results hook rides the same gate [S, remaining host glue].
                // net-consolidate: pump the ONE session once per frame (receive, timeouts,
                // handshake, the armed GO). The events drive the same handover a menu confirm
                // drives; the lobby pages drive the same core through their `MpSession` proxy.
                if let Some(session) = self.net.as_mut() {
                    session.0.borrow_mut().poll();
                }
                // Drain first, then react: the borrow must be gone before the
                // handlers touch `self` again (net_fail replaces the session).
                let net_events = self.net.as_mut()
                    .map(|session| session.0.borrow_mut().take_app_events())
                    .unwrap_or_default();
                for ev in net_events {
                    match ev {
                            mpnet::CoreEvent::MatchStarted(st) => {
                                // Every machine takes the same handover: the level loads from the
                                // settings (the seed rides inside), the menu closes. The lobby
                                // pages also see this through `match_started()` (mp_net_poll),
                                // which mirrors it into the same fields — idempotent.
                                eprintln!("[net] match starting: arena {:08x}, {} humans, seed {:#x}",
                                    st.level_code, st.human_count, st.seed);
                                self.net_local_slot = self.net.as_ref().map_or(0, |s| s.0.borrow().local_slot());
                                self.pending_level = Some(st.level_code);
                                self.pending_mp_settings = Some(*st);
                                if let Some(fe) = self.fe.as_mut() { fe.closed = true; }
                            }
                            mpnet::CoreEvent::AgentJoined { slot } => {
                                eprintln!("[net] agent {slot} joined");
                                // The flag path's auto-start: the first join arms the match.
                                if let Some(st) = self.net_armed.take() {
                                    let n = 1 + slot as u32;
                                    let mut armed = st;
                                    armed.human_count = n;
                                    armed.player_count = n as u16;
                                    armed.total_players = n;
                                    if let Some(s) = self.net.as_mut() { s.0.borrow_mut().start_match(&armed); }
                                    self.pending_level = Some(armed.level_code);
                                    self.pending_mp_settings = Some(armed);
                                }
                            }
                            mpnet::CoreEvent::Refused => {
                                self.net_fail("Refused: a different build or the match is full");
                            }
                            mpnet::CoreEvent::Desync { tick } => {
                                self.net_fail(&format!("State desync at tick {tick}\nReturning to the lobby"));
                            }
                            mpnet::CoreEvent::ClientLeft { slot, graceful } => {
                                // DropPolicy::Neutral: the dropped agent rides NEUTRAL inputs and
                                // the match runs on — the game's own disconnected-pad shape
                                // (psiInput_PollDevices 0x10ca78 writes buttons 0 / analog 0x7f
                                // for a dead pad), not the old terminal PeerLost.
                                eprintln!("[net] player {} {} — the match runs on with NEUTRAL inputs",
                                    slot + 1, if graceful { "left" } else { "timed out" });
                            }
                            mpnet::CoreEvent::HostTimeout => {
                                self.net_fail("The host went away\nReturning to the lobby");
                            }
                    }
                }
                let menu_hold = self.paused || self.fe.as_ref().is_some_and(|fe| fe.page == menu::Page::Pause);
                if menu_hold != self.audio_paused {
                    self.audio_paused = menu_hold;
                    if let Some(s) = self.sound.as_mut() { s.set_paused(menu_hold); }
                }
                if !self.paused && self.fe.is_none() {
                    // Game logic at its fixed 60 Hz, decoupled from the render rate: one AI tick per whole tick of time.
                    // Dev mode's time scale changes the number of ticks run (WORKLOG 0081), not the tick rate.
                    let before = self.anim_time as u32;
                    // fix-perf-level: the catch-up cap. A slow render frame must not burst a spiral of
                    // ticks — on 07000006 a tick costs ~4.1 ms, so the old 8-tick cap let the sim
                    // burst 33 ms behind one frame (the jitter fix-render3 measured). The game itself
                    // has no catch-up: under load it slows down. Capped ticks are DROPPED, not
                    // deferred — logic stays fixed-60 Hz and deterministic (the tick count depends
                    // only on frame times, never on their content). `NF_MAXTICKS` retunes it.
                    let max_ticks = *MAX_TICKS.get_or_init(|| std::env::var("NF_MAXTICKS").ok()
                        .and_then(|v| v.parse::<u32>().ok()).filter(|v| *v > 0).unwrap_or(4));
                    // G-043: the loop's tick scheduling, extracted verbatim into `ticks_for_frame`
                    // (the fps-determinism test drives the same arithmetic headless).
                    let elapsed = ticks_for_frame(&mut self.anim_time, dt, max_ticks);
                    self.frame_ticks = elapsed;
                    // fix-ingame-dev-menu: the open menu pauses the sim — 0 ticks unless F11's
                    // one-shot step was pressed (`paused_tick_count`).
                    let n_ticks = match self.dev.as_deref_mut() {
                        Some(d) if d.menu => d.paused_tick_count(),
                        Some(d) => d.tick_count(elapsed),
                        None => elapsed,
                    };
                    self.ticked = n_ticks > 0;
                    let eye = [self.cam.pos.x, self.cam.pos.y, self.cam.pos.z];
                    let look = self.cam.dir().to_array();
                    if let Some(sim) = self.scene.sim.as_mut() {
                        for _ in 0..n_ticks {
                            if self.noclip { sim.step(eye, look, self.trigger, self.aiming); continue; }
                            // net-consolidate: the lockstep gate — the sim advances tick t only
                            // when every live agent's input for t has arrived. A stalled tick
                            // renders the last ticked frame (`ticked` false keeps the
                            // interpolation honest), and NO capture runs on it, so key edges
                            // survive the stall. Never guess: the session's missing input is a
                            // wait, not a default.
                            if let Some(session) = self.net.as_mut() {
                                if session.0.borrow_mut().ready_inputs(self.net_tick).is_none() { self.ticked = false; break; }
                            }
                            // B-5 record/replay: when `--replay` is set, the input comes from the recording, not the
                            // keyboard. The recorded tick drives `step_bond` exactly as it did when recorded; the user
                            // can only watch (Esc to quit). When the recording ends, the sim pauses.
                            let (mut input, trigger, reload, use_key) = if let Some(rp) = &self.replay {
                                if self.replay_idx < rp.ticks.len() {
                                    let t = rp.ticks[self.replay_idx];
                                    self.replay_idx += 1;
                                    t.to_parts()
                                } else {
                                    self.paused = true; // recording exhausted — hold the last frame.
                                    break;
                                }
                            } else {
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
                                    zoom_cycle: std::mem::take(&mut self.zoom_cycle),
                                    switch_to: self.weapon_switch_to.take(),
                                    fire: self.trigger,
                                    zoom: self.zoom,
                                    scope: self.zoom,
                                    // The fire-mode button (pad ○; keyboard M) [M Player_Weapon 0x1a24c8].
                                    mode_switch: self.just.remove(&KeyCode::KeyB),
                                };
                                (input, self.trigger, self.just.remove(&KeyCode::KeyR), self.just.remove(&KeyCode::KeyE))
                            };
                            // weapon-matrix: the dev menu's "Test weapon" run (dev.rs) merges its
                            // inputs into this tick — the same lanes the keyboard uses
                            // (`switch_to`, the trigger, the R edge), never during a replay
                            // (the recording drives the sim alone).
                            let (tw_switch, tw_fire, tw_reload) = if self.replay.is_none() {
                                self.dev.as_deref_mut().map_or((None, false, false), |d| d.test_inputs())
                            } else { (None, false, false) };
                            input.switch_to = input.switch_to.or(tw_switch);
                            let trigger = trigger || tw_fire;
                            let reload = reload || tw_reload;
                            // The gamepad (fix-input, `gamepad` feature): the pad's path through the
                            // game's own mapping/curves/ramps merges into the keyboard's Input; no pad,
                            // no change. Mouse look is untouched (its deltas are in input.turn/look; the
                            // pad's contributions add on top).
                            #[cfg(feature = "gamepad")]
                            if let Some(pad) = self.pad.as_mut() {
                                pad.merge(&mut input, sim.zoom, sim.bond.underwater, false);
                            }
                            // The party weapon lock (`Player_Weapon` 0x1a24c8:242-275): on 07000007 with
                            // ch 0x48 off the cycle actions cannot leave the fists (fix-level2 F4).
                            nf_game::player::weapon_lock_input(sim.level, sim.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL), &mut input);
                            // net-consolidate: the finalized input goes to the session (this slot's
                            // entry in the per-tick exchange; the trigger/reload/use_key lanes ride
                            // the 24-byte wire input now — 053's 20-byte packing dropped them).
                            if let Some(session) = self.net.as_mut() {
                                let ni = mpnet::to_net_input(&input, trigger, reload, use_key);
                                session.0.borrow_mut().local_input(self.net_tick, ni);
                            }
                            // B-5 record: capture the exact input passed to step_bond, before the call.
                            if let Some(r) = &mut self.recorder { r.push(replay::TickInput::from_parts(&input, trigger, reload, use_key)); }
                            sim.step_bond(input, trigger, reload, use_key);
                            // weapon-matrix: the "Test weapon" run watches the tick's result (the
                            // phase machine in dev.rs; `sim` and `self.dev` are disjoint fields).
                            if let Some(d) = self.dev.as_deref_mut() { d.test_observe(&*sim); }
                            // net-consolidate: the remote agents' movement tick with the session's
                            // inputs (the same `Bond::tick_ctx` core on every machine), the MP
                            // slots' live positions, and the state hash on the session's own
                            // cadence (hash_every 15; the previous hash piggybacks every packet).
                            if self.net.is_some() {
                                let session = self.net.as_mut().unwrap();
                                let slot = self.net_local_slot;
                                if let Some(net_inputs) = session.0.borrow_mut().ready_inputs(self.net_tick) {
                                    let inputs = net_inputs.map(|i| mpnet::to_player_input(&i));
                                    sim.step_net_remotes(&inputs, slot);
                                    if let Some(m) = sim.mp.as_mut() {
                                        for sl in 0..4 {
                                            if sl != slot { if let Some(b) = &sim.mp_bonds[sl] { m.set_slot(sl, b.pos); } }
                                        }
                                        m.set_slot(slot, sim.bond.pos);
                                    }
                                    let h = sim.net_state_hash(slot);
                                    session.0.borrow_mut().submit_hash(self.net_tick, h);
                                }
                                self.net_tick += 1;
                            }
                            // The lock's rising edge auto-draws the handgun (`Upgrade_Weapon(6)` = 2 [L])
                            // [M 0x1a24c8:247-250]. `prev` = last tick's channel (the game's
                            // `switch_channels_prev` semantics).
                            let on = sim.drones.channels.on(nf_game::player::WEAPON_LOCK_CHANNEL);
                            if nf_game::player::weapon_lock_auto_draw(sim.level, on, self.prev_lock_ch) {
                                eprintln!("[weapon-lock] ch 0x48 rising edge: switch request → 2 (P2K)");
                                sim.gun.switch_request = 2;
                            }
                            self.prev_lock_ch = on;
                            if let Some(s) = self.sound.as_mut() { sound_tick(s, sim); }
                        }
                        // Dev mode after-tick: god mode / infinite ammo (the game's own state, set the way the game sets it).
                        // (Moved below the sim borrow — see the end of this block.)
                        // Bug 5: level transition. When a `LOAD_LEVEL` (0xE8) trigger fires, the sim sets
                        // `drones.level_exit` to the next level's hashcode. Reload the scene with that archive.
                        // Script event 0x12 requests ride the same field (07000006's cut-scene → 07000007;
                        // 0700004a's pack cut-scene → the 0x0720004a movie code) — resolve the movie/wrapper
                        // codes the way the game's load pipeline does [M 0x1d3800 + 0x1d6360 phase 2 +
                        // 0x1d3560; nf_game::triggers::resolve_exit_code].
                        if let Some(end_to) = sim.drones.mission_end.take() {
                            eprintln!("level: mission over (LevelToEndTo {end_to:08x}) — the outro + results");
                            self.pending_mission_end = true;
                        }
                        if let Some(next) = sim.drones.level_exit.take() {
                            let resolved = nf_game::triggers::resolve_exit_code(next);
                            if resolved != next { eprintln!("level: exit {next:08x} resolves to {resolved:08x} [M 0x1d3560]"); }
                            eprintln!("level: exit trigger fired → loading {resolved:08x}");
                            self.pending_level = Some(resolved);
                        }
                        if !self.noclip {
                            let h = sim.bond.head();
                            self.cam = Camera { pos: Vec3::new(h[0], h[1], h[2]), yaw: sim.bond.yaw, pitch: sim.bond.pitch * std::f32::consts::FRAC_PI_2, fov: GAME_FOV / sim.zoom };
                            // A script's cut-scene camera takes over the view (`Camera_Enable(4)`).
                            if let Some((p, yaw, pitch, fov)) = sim.script_camera() { self.cam = Camera { pos: Vec3::new(p[0], p[1], p[2]), yaw, pitch, fov }; }
                            // fix-sentinel-camera-play [M `Player_PositionCamera` 0x18f5d8 case 0xc]: while the
                            // guided camera rides the missile, the view IS the missile's (`Sim::view_override`
                            // = its pos/dir) — the user's "doesn't go into first person": the window never
                            // consulted the override (the sim side rode; the matrix's gcam saw it). The HUD
                            // ammo pane stays; the view model and the crosshair do not draw (frame_state's
                            // ride hide + hud.rs's gate).
                            if let Some((p, d)) = sim.view_override() {
                                self.cam = Camera { pos: Vec3::new(p[0], p[1], p[2]), yaw: d[0].atan2(d[2]), pitch: d[1].clamp(-1.0, 1.0).asin(), fov: GAME_FOV };
                            }
                            // fix-wallhug-prompt [M `Player_PositionCamera` 0x18f5d8 case 0xb → `camera_tracking`
                            // 0x1ba5e0]: the wall shimmy's camera is the game's third-person tracking camera
                            // (the user's "when I entered a prompt it was in first person. should be 3rd person").
                            if let Some((p, yaw, pitch, fov)) = sim.creep_camera() {
                                self.cam = Camera { pos: Vec3::new(p[0], p[1], p[2]), yaw, pitch, fov: GAME_FOV / fov };
                            }
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
                                    nf_game::pickup::Taken::Key { id } => format!("  picked up key {id}"), // 0x2000010 [M 0x18e650:285-289]
                                    nf_game::pickup::Taken::Armour => "  picked up armour".to_string(),   // 0x200000F [M 0x18e650:290-294]
                                    nf_game::pickup::Taken::Bonus => "  picked up a 007 bonus".to_string(), // 0x200004A [M 0x18e650:315-321]
                                    nf_game::pickup::Taken::Label { .. } => "  picked up the item".to_string(), // case 4 [M 0x18e650]; the real message binds the label (scene.rs)
                                    nf_game::pickup::Taken::Message => "  picked up a pickup".to_string(), // 0x2000013 [M 0x18e650 case 5]
                                });
                                g.window.set_title(&format!("nf-view — Bond health {:.0}{}  P2K {} / {}{hit}{msg}", h.health, if h.health <= 0.0 { " (dead)" } else { "" }, sim.gun.clip, sim.gun.spare));
                            }
                        }
                    }
                    // Dev mode after-tick: god mode / infinite ammo (the game's own state, set the way the game sets it).
                    if let Some(d) = self.dev.as_deref_mut() { d.after_tick(&mut self.scene); }
                    // The mission's end sequence (`Mission_Update` case 4 → `ResetMap_LevelToLoad`
                    // of the outro or the front end): the same front-end path the F8 hook drives.
                    if let Some(ch) = self.force_channel {
                        if let Some(sim) = self.scene.sim.as_mut() {
                            if sim.drones.frame >= 120 { sim.drones.set_channel(ch); self.force_channel = None; eprintln!("[dev] channel {ch} raised"); }
                        }
                    }
                    if std::mem::take(&mut self.pending_mission_end) { self.demo_results(true); }
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
                let interp0 = !self.noclip && !self.paused && self.scene.sim.is_some() && std::env::var_os("NF_NOINTERP").is_none()
                    && self.dev.as_deref().map_or(true, |d| d.interp);
                let alpha0 = if interp0 { self.anim_time.fract() } else { 1.0 };
                let draw_cam = match (self.cam_prev, self.cam_cur) { (Some(a), Some(b)) if interp0 => lerp_cam(&a, &b, alpha0), _ => self.cam };
                let aspect0 = g.config.width as f32 / g.config.height as f32;
                // fix-cutscene-letterbox: a live script camera frames the view at the game's own
                // 4:3 aspect and pillar-boxes the render into the band (the game's letterbox bars
                // ride inside it). Gameplay keeps the window's widescreen. Same condition the HUD
                // gate (`overlay_for`) drives the cut-scene HUD blank from.
                let cutscene = self.scene.sim.as_ref().is_some_and(|s| s.script_camera().is_some());
                let (proj_aspect, band) = if cutscene { cutscene_frame(g.config.width as f32, g.config.height as f32) } else { (aspect0, [0.0; 4]) };
                let (vp0, _) = draw_cam.view_proj(proj_aspect);
                // One visibility walk per frame, shared by the palette's object cull and render's
                // World cull (fix-portal): the eye's cel from `Camera_CheckLocation` 0x1bae90's
                // persistent state, the walk from it [M 0x1e86e8].
                // The Toggles page's portal-culling row: off passes no world, so the walk returns
                // everything (the whole level draws — the isolation view).
                let portal = self.dev.as_deref().map_or(true, |d| d.portal_cull);
                let door_closed = if portal { self.scene.sim.as_ref().map(|s| s.drones.doors.closed_flags()) } else { None };
                let vis = g.renderer.cel_visibility(draw_cam.pos.to_array(), vp0, if portal { self.scene.sim.as_ref().map(|s| &s.world) } else { None }, door_closed.as_deref());
                // G-043: build the palette only when a tick ran, then draw between the last two ticks.
                if self.ticked || self.pal_cur.is_none() {
                    let new = self.scene.skin_palette(&g.renderer.skin_layout, self.choice, self.anim_time, self.opts, vis.as_deref());
                    self.pal_prev = self.pal_cur.take();
                    self.pal_cur = Some(new);
                    self.cam_prev = self.cam_cur;
                    self.cam_cur = Some(self.cam);
                }
                let interp = interp0;
                let alpha = alpha0;
                if let Some((pal, cols)) = &self.pal_cur {
                    match &self.pal_prev {
                        Some((pp, _)) if interp => {
                            let mut blended = lerp_palette(pp, pal, alpha);
                            // fix-interp-ghosting: the view model (posed in world space from each
                            // tick's camera [M Player_PositionGun 0x1a3b40]) re-attaches to the
                            // interpolated draw camera — the same camera-space scheme lerp_cam
                            // uses for the view — so mid-turn the gun cannot swim against the
                            // view. With a live script camera the tick cams are not Bond's: the
                            // gun keeps the plain world blend there.
                            if self.scene.sim.as_ref().map_or(true, |s| s.script_camera().is_none()) {
                                if let (Some(entries), Some(cp), Some(cc)) =
                                    (view_model_entries(&self.scene, &g.renderer.skin_layout), self.cam_prev, self.cam_cur) {
                                    reattach_view_model(&mut blended, pp, pal, alpha, &cp, &cc, &draw_cam, &entries);
                                }
                            }
                            g.renderer.upload_skin(&g.queue, &blended, cols);
                        }
                        _ => g.renderer.upload_skin(&g.queue, pal, cols),
                    }
                }
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
                let aspect = proj_aspect;
                let (_, vp_rot) = draw_cam.view_proj(aspect);
                let vp = vp0;
                let snow = snow_for(&self.scene, &draw_cam);
                // fix-menus: a menu page up → the menu overlay replaces the HUD. fix-menu-movies2:
                // the ONE movie plane (gpu.rs) serves every movie — the state-6 briefing/outro
                // (fix-movie-playback), the P_INTRO pair and the page's looping backdrop
                // (fix-menu-movies) — and draws BEFORE the HUD runs, so the menu pages' controls
                // paint over their backdrop exactly like the game's page background
                // [M PS2UploadBGMImage 0x108740 + movies.md §Presentation].
                // The borrow rules: read the state first, then pull the frame — `movie_frame`
                // holds a borrow into `self.movie` until render() consumes it.
                let state6 = self.fe.as_ref().is_some_and(|fe| fe.flow.get_state() == menu::ST_MOVIE);
                let intro_up = self.fe.as_ref().is_some_and(|fe| fe.intro_playing) && state6;
                // attract-mode: the attract movie owns the screen like the intro does — the
                // attract page has no controls [L the menu-def's 0x40000035 record], the movie
                // plane IS the page [M P_ATTRACT_Handler 0x20a1f0's psiStartBackgroundMovie].
                let attract_up = self.fe.as_ref().is_some_and(|fe| fe.attract_playing);
                let flow_up = state6 && !intro_up && self.movie.is_some();
                // The looping page player is the backdrop under the overlay; the early-started
                // briefing player (non-looping, held through ST_MAPLOAD) is not.
                let aux_up = self.movie.as_ref().is_some_and(|p| p.is_looping())
                    && !self.fe.as_ref().is_some_and(|fe| fe.flow.movie_pending);
                let movie_up = intro_up || flow_up || aux_up || attract_up;
                let movie_frame = if movie_up { self.movie.as_mut().and_then(|p| p.present()) } else { None };
                let mut hud = if let Some(fe) = self.fe.as_mut() {
                    let lang = self.lang.as_ref().or_else(|| self.scene.sim.as_ref().and_then(|s| s.lang.as_ref()));
                    fe.tick();
                    fe.backdrop_live = aux_up && movie_frame.is_some();
                    if flow_up || (intro_up && movie_frame.is_some()) || (attract_up && movie_frame.is_some()) {
                        // The movie plane replaces the overlay: the state-6 briefing blits over the
                        // empty backdrop and the fog pass is suppressed while a movie plays [M
                        // DecodeBGFMVFrame 0x108550 + PS2UploadBGMImage 0x108740, psiDrawFogPass
                        // 0x10b960]; the intro FMV owns the screen the same way [M 0x209e88]. The
                        // quad itself is drawn inside render(), before the (empty) HUD runs.
                        gpu::HudFrame::default()
                    } else {
                        // fix-pause-menu2: the OBJECTIVES tab's rows come from the sim's mission
                        // states (Mission_ObjectiveState's data [M 0x204408:251-271]; state 4 =
                        // the done marker), refreshed while the pause page is up.
                        // The burning woman (crate::woman): `Game_Run` 0x1c9790:36-41 runs `psiDecompressWoman` once per
                        // frame in the paused branch — the pause page and the fail menu both pause the game.
                        let shows_woman = fe.page == menu::Page::Pause || (fe.page == menu::Page::Results && fe.outcome == menu::Outcome::Failed);
                        if shows_woman {
                            if let Some(w) = self.scene.woman.as_mut() {
                                let mut changed = false;
                                // The paused frames' own 60 Hz clock: `frame_ticks` is the GAME's tick count and stays frozen
                                // at its last value while a menu is up, so ticking by it ran the figure once per render frame
                                // × that stale count (several times too fast on a fast display — the user's playtest).
                                self.menu_clock += dt;
                                let n = (self.menu_clock * 60.0) as u32;
                                self.menu_clock -= n as f32 / 60.0;
                                for _ in 0..n.min(8) { changed |= w.tick(); }
                                if changed { g.renderer.update_woman(&g.queue, &w.rgba()); }
                            }
                        }
                        if fe.page == menu::Page::Pause {
                            if let Some(sim) = self.scene.sim.as_ref() { fe.pause_objectives = pause_objective_rows(sim); }
                        }
                        let mut f = match &self.scene.hud {
                            Some(tables) => menu::menu_overlay(fe, &tables.font, &tables.font_header, lang, &self.scene.hud_textures, aspect),
                            None => overlay_for(&self.scene, aspect),
                        };
                        // fix-pause-menu2: the in-game HUD stays visible under the pause page —
                        // the game's pause menu draws OVER the paused frame and its HUD (radar
                        // ring, ammo pane) [L the main agent's PCSX2 captures 2026-10-13; the
                        // port replaced the HUD with the overlay]. The HUD's quads go FIRST so
                        // the menu draws over them (frame order = draw order).
                        if fe.page == menu::Page::Pause && !matches!(fe.flow.get_state(), menu::ST_MOVIE) {
                            // The menu's runs shift by the HUD's vertex count (the HUD's vertices go first) and the HUD
                            // keeps its own runs — the old merge shifted by the MENU's count and dropped the HUD's runs,
                            // so the live pause page drew from the wrong vertices (the plate, the figure and most rows
                            // missing, the cursor a square) and the radar/ammo vanished while paused.
                            let hud_frame = overlay_for(&self.scene, aspect);
                            let base = hud_frame.v.len() as u32;
                            let menu_runs = std::mem::take(&mut f.runs);
                            let mut v = hud_frame.v;
                            v.extend(f.v.drain(..));
                            f.runs = hud_frame.runs;
                            for (tex, blend, start, count) in menu_runs {
                                f.runs.push((tex, blend, start + base, count));
                            }
                            f.v = v;
                        }
                        f
                    }
                } else { overlay_for(&self.scene, aspect) };
                if let Some(d) = self.dev.as_deref() { if d.overlay || d.menu { hud.append(&dev::hud(d, &self.scene, aspect, vp)); } }
                hud.append(&self.scene.script_overlay_hud(aspect));
                g.renderer.lights = self.scene.frame_lights();
                g.renderer.screen_aspect = g.config.width as f32 / g.config.height.max(1) as f32;
                let enc = g.renderer.render(&g.device, &g.queue, &view, &g.depth, vp, vp_rot, draw_cam.pos.to_array(), self.anim_time as u32, &snow, &hud, movie_frame, if cutscene { Some(band) } else { None });
                g.queue.submit([enc.finish()]);
                g.queue.present(frame);
                if let Some(w) = self.devwin.as_mut() { w.tick(); }
                if let Some(fps) = self.fps_cap.filter(|f| *f > 0.0) {
                    let target = std::time::Duration::from_secs_f32(1.0 / fps);
                    let el = self.frame_start.elapsed();
                    if el < target { std::thread::sleep(target - el); }
                }
                // `NF_FPSLOG=1`: one stderr line per second — frames, average and worst frame time.
                if std::env::var_os("NF_FPSLOG").is_some() { fps_log(self.frame_start.elapsed()); }
                self.frame_start = std::time::Instant::now();
                g.window.request_redraw();
            }
            _ => {}
        }
    }
}

pub fn run() -> Result<()> {
    // A panic also goes to nf-view-crash.log (message, place, backtrace): a crash in a window without a console
    // leaves a trace.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = format!("{info}\n{}\n", std::backtrace::Backtrace::force_capture());
        let _ = std::fs::write("nf-view-crash.log", &text);
        default_hook(info);
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    const USAGE: &str = "usage: nf-view [<iso>] <level> [--all] [--seq K] [--frame F] [--fire] [--reinforcements] [--dev [--panel]] [--scenario <name>] [--record <file>] [--replay <file>] [--no-fog] [--no-tex] [--shot out.png [--cam x y z yaw pitch] [--dev-page <name>]]\n\
        <level> is a FILES.BIN name such as 07000005 (\".bin\" optional); without <iso> the image path comes from NF_ISO\n\
        --dev --scenario <name>: place Bond for a feature test (weapon-switch, sg5-pickup, wire-music, autoaim)\n\
        --record <file>: record every logic tick's input to <file> (B-5). --replay <file>: replay a recording.\n\
        --no-fog: skip the fog screen pass (isolate vertex colour/texture). --no-tex: ignore texture samples (solid white, isolate lighting/fog).";
    // --movie <name/code> --frame N --shot out.png: headless PSS movie test (fix-movies) — demux, decode one
    // frame of the disc's MPEG-2 video, dump it as a PNG (no window, no GPU). The game plays these through
    // psiStartBackgroundMovie 0x001081a8 (cdrom0:\movies\30_FPS\%8.8x.pss;1). Needs `--features movies`.
    #[cfg(feature = "movies")]
    if let Some(m) = args.iter().position(|a| a == "--movie") {
        let name = args.get(m + 1).context("--movie needs a movie name or hashcode (e.g. 07100001)")?;
        let frame: usize = args.iter().position(|a| a == "--frame").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0);
        let out = args.iter().position(|a| a == "--shot").and_then(|i| args.get(i + 1)).map(std::path::PathBuf::from).context("--movie is headless only: pass --shot <out.png>")?;
        let iso_path = args.first().cloned().or_else(|| std::env::var("NF_ISO").ok()).context("no <iso> given and NF_ISO is not set")?;
        let iso = nf_formats::iso9660::IsoImage::open(&iso_path)?;
        movie::movie_frame_shot(&iso, name, frame, &out)?;
        return Ok(());
    }
    // Positional arguments are the ones before the first flag.
    let pos: Vec<&String> = args.iter().take_while(|a| !a.starts_with("--")).collect();
    // No <level> at all: the front end (fix-menus) — the game's own boot loads the front-end map
    // 0x7000048 and opens the P_START page on it [M bootup 0x1c7e78 + ResetMap_Load 0x1d6360].
    // A lone positional that names an existing file is the image, not a level (`nf-view <iso>` boots the
    // front end too).
    let lone_iso = matches!(pos.as_slice(), [p] if std::path::Path::new(p.as_str()).is_file());
    let (iso, level) = match pos.as_slice() {
        [iso, level] => (iso.to_string(), level.to_string()),
        [iso] if lone_iso => (iso.to_string(), "07000048".to_string()),
        [level] => (std::env::var("NF_ISO").map_err(|_| anyhow::anyhow!("NF_ISO is not set and no <iso> was given\n{USAGE}"))?, level.to_string()),
        [] => (std::env::var("NF_ISO").map_err(|_| anyhow::anyhow!("NF_ISO is not set and no <iso> was given\n{USAGE}"))?, "07000048".to_string()),
        _ => bail!("{USAGE}"),
    };
    let front_end = pos.is_empty() || lone_iso;
    let level = if level.to_ascii_lowercase().ends_with(".bin") { level } else { format!("{level}.bin") };
    let all = args.iter().any(|a| a == "--all");
    let dev = args.iter().any(|a| a == "--dev");
    // `--fps N`: cap the frame rate at N with vsync off (0 = uncapped). Without it: vsync at the display rate.
    let fps_cap: Option<f32> = args.iter().position(|a| a == "--fps").and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<f32>().ok());
    let mut scenario = args.iter().position(|a| a == "--scenario").and_then(|i| args.get(i + 1)).cloned();
    let record_path = args.iter().position(|a| a == "--record").and_then(|i| args.get(i + 1)).cloned();
    let replay_path = args.iter().position(|a| a == "--replay").and_then(|i| args.get(i + 1)).cloned();
    // --replay <file>: the level and scenario come from the recording's header, not the command line.
    let replay_recording = match &replay_path { Some(p) => Some(replay::Replay::load(p)?), None => None };
    let (level, level_for_record): (String, String) = match &replay_recording {
        Some(r) => {
            let lvl = r.header.level.clone();
            if let Some(s) = &r.header.scenario { scenario = Some(s.clone()); }
            eprintln!("[replay] {} ({} ticks, git {}); level {}, scenario {}",
                replay_path.as_deref().unwrap_or("?"), r.tick_count(), r.header.git_hash, lvl, r.header.scenario.as_deref().unwrap_or("(none)"));
            (lvl.clone(), lvl)
        }
        None => (level.clone(), level.clone()),
    };
    // fix-weapon-assets: the `--give` ids must reach the load — each given weapon's assets merge
    // from its owning archive BEFORE the scene builds, so the weapon draws with its skin,
    // sequences, muzzle flash and sounds on any level (`nf_view::load_weapons`).
    let give_ids: Vec<u8> = args.iter().position(|a| a == "--give").and_then(|i| args.get(i + 1))
        .map(|list| list.split(',').filter_map(|t| {
            let t = t.trim();
            let id = t.strip_prefix("0x").and_then(|h| usize::from_str_radix(h, 16).ok()).or_else(|| t.parse().ok());
            id.filter(|&i| i < 0x72).map(|i| i as u8)
        }).collect()).unwrap_or_default();
    let scene = if dev { load_dev(&iso, &level, all)? }
        else if !give_ids.is_empty() { load_weapons(&iso, &level, all, &give_ids)? }
        else if args.iter().any(|a| a == "--mp") { load_mp(&iso, &level, all)? }
        else { load(&iso, &level, all)? };
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
    // B-6: `--frame N --shot out.png` now applies `--dev --scenario`, so the headless render shows the scenario's
    // placement (Bond near a guard, etc.) instead of the level start. Apply the scenario to a mutable scene before
    // the tick loop. (The event-loop path applies it below.)
    let apply_scenario = |sc: &mut scene::Scene, name: &str| {
        if let Some((_, f)) = dev::scenario(name) { f(sc); }
        else { eprintln!("[dev] unknown scenario '{name}'. Available: {}", dev::SCENARIOS.join(", ")); }
    };
    if let Some(i) = args.iter().position(|a| a == "--shot") {
        // Headless: run the game for `--frame` ticks with Bond at the camera, then render. (B-6: apply the scenario
        // first, so Bond is placed for the feature test and the rendered frame shows it.)
        let mut scene = scene;
        if let Some(n) = &scenario { apply_scenario(&mut scene, n); }
        // The burning woman for the paused-page renders: advance her by the shot's tick count, as the paused frames
        // would (crate::woman), so `--page pause|failed --frame N` shows frame N/2 of her loop.
        if args.iter().position(|a| a == "--page").and_then(|i| args.get(i + 1)).is_some_and(|p| p == "pause" || p == "failed" || p == "results") {
            if let Some(w) = scene.woman.as_mut() {
                for _ in 0..time as u32 { w.tick(); }
                let rgba = w.rgba();
                if let Some((_, t)) = scene.hud_textures.iter_mut().find(|(h, _)| *h == 0x0300_00db) { if t.rgba.len() == rgba.len() { t.rgba = rgba; } }
            }
        }
        if let Some(sim) = scene.sim.as_mut() {
            // `--pull 100,130`: the trigger is pressed on those ticks (one tick each).
            let pulls: Vec<u32> = match args.iter().position(|a| a == "--pull") {
                Some(i) => args.get(i + 1).context("--pull needs ticks")?.split(',').map(|t| t.parse()).collect::<Result<_, _>>()?,
                None => Vec::new(),
            };
            // Without `--cam`, Bond stands at the start and the camera is his eye (the first-person gun shows).
            let walk = !args.iter().any(|a| a == "--cam");
            // `--zoom-hold`: hold aim + the zoom key every tick, so the headless --shot renders can show the
            // zoom overlays (the overlay draws only while the key is held; `Player_Zoom` 0x1a1a70 raises the
            // zoom on the held input — the same path the fix-zoom-overlays tests drive).
            let zoom_hold = args.iter().any(|a| a == "--zoom-hold");
            // fix-aims20-lights-render: hold a strafe stick deflection every tick — the analog half of the
            // idle-counter reset (DAT_002a3a1c [M 0x17ee88:80-84]), so the aims lights' gates evaluate on a
            // moving Bond exactly as the main agent's strafing render does, instead of the dead-still all-off
            // state 0.75 s in.
            let strafe_hold = args.iter().any(|a| a == "--strafe-hold");
            let shot_input = nf_game::player::Input { aiming: zoom_hold, zoom: zoom_hold, strafe: if strafe_hold { 1.0 } else { 0.0 }, ..Default::default() };
            // `--channels`: every switch channel on first (as K).
            if args.iter().any(|a| a == "--channels") { for c in 1..=255 { sim.drones.set_channel(c) } }
            // `--hold-npcs` (dev): guards freeze where they stand but stay shootable — aim render checks at them.
            if args.iter().any(|a| a == "--hold-npcs") { sim.drones.hold_npcs = true; }
            // `--bond x,y,z`: Bond starts exactly there (in the air he falls), instead of the level start.
            if let Some(i) = args.iter().position(|a| a == "--bond") {
                let v: Vec<f32> = args.get(i + 1).context("--bond needs x,y,z")?.split(',').map(|t| t.parse()).collect::<Result<_, _>>()?;
                sim.bond = nf_game::player::Bond::new([v[0], v[1], v[2]], v.get(3).copied().unwrap_or(sim.bond.yaw)); if let Some(&pt) = v.get(4) { sim.bond.pitch = pt; }
            }
            // `--draw <id>`: equip that weapon (the `--give` path's equip) and switch to it before the ticks, so a
            // headless render can show a fire script frame by frame (`--draw 0x34 --pull 60 --frame 70`).
            let mut draw: Option<u8> = None;
            if let Some(i) = args.iter().position(|a| a == "--draw") {
                let t = args.get(i + 1).context("--draw needs a weapon id")?;
                let id = match t.strip_prefix("0x") { Some(h) => usize::from_str_radix(h, 16)?, None => t.parse()? };
                let wd = sim.weapons.clone();
                let rounds = wd.clip_size(id).max(1);
                if sim.drones.inventory.equip_weapon(&wd, id, rounds, true) { sim.drones.inventory.equip_ammo(&wd, id, rounds * 3); }
                draw = Some(id as u8);
            }
            // `--skip-cutscenes`: hold the game's own skip while a script camera runs (`Script_FFwd` 0x1dac78), for image pairs.
            let skip_cuts = args.iter().any(|a| a == "--skip-cutscenes");
            let modes: Vec<u32> = match args.iter().position(|a| a == "--mode") {
                Some(i) => args.get(i + 1).context("--mode needs ticks")?.split(',').map(|t| t.parse()).collect::<Result<_, _>>()?,
                None => Vec::new(),
            };
            if let Some(w) = draw { sim.step_bond(nf_game::player::Input { switch_to: Some(w), ..shot_input }, false, false, false); }
            // fix-aims20-lights-render: the headless `--shot` path honours `--give` too — the same equip +
            // switch_request the window path applies below (the shot path returned before that block, so
            // `nf-view <iso> <level> --dev --give 0x1a --shot …` rendered the level's starting gun and the
            // main agent could never get the AIMS-20's view model into a headless render). Runs BEFORE the
            // ticks, so the draw anim completes and the aims lights' gates evaluate on the held 0x1a/0x1b.
            if let Some(list) = args.iter().position(|a| a == "--give").and_then(|i| args.get(i + 1)) {
                let wd = sim.weapons.clone();
                let mut first = None;
                for t in list.split(',') {
                    let t = t.trim();
                    let id = if let Some(h) = t.strip_prefix("0x") { usize::from_str_radix(h, 16).ok() } else { t.parse().ok() };
                    let Some(id) = id.filter(|&i| i < 0x72) else { eprintln!("--give: bad id {t:?}"); continue };
                    let rounds = wd.clip_size(id).max(1);
                    if sim.drones.inventory.equip_weapon(&wd, id, rounds, true) {
                        sim.drones.inventory.equip_ammo(&wd, id, (rounds * 3).max(rounds));
                        first.get_or_insert(id as u8);
                        eprintln!("--give: equipped {id:#04x}");
                    } else if sim.drones.inventory.owned.get(id).copied().unwrap_or(false) {
                        first.get_or_insert(id as u8);
                        eprintln!("--give: {id:#04x} already owned — drawing it");
                    } else {
                        eprintln!("--give: {id:#04x} refused (no first-person model)");
                    }
                }
                if let Some(id) = first { sim.gun.switch_request = id; }
            }
            for t in 1..=time as u32 {
                if skip_cuts && sim.disabled { sim.drones.scripts.skip_request = true; }
                // `--mode 70,90`: the fire-mode press (the B key) on those ticks.
                let mode_now = modes.contains(&t);
                if walk { sim.step_bond(nf_game::player::Input { mode_switch: mode_now, ..shot_input }, pulls.contains(&t), false, false); } else { sim.step([cam.pos.x, cam.pos.y, cam.pos.z], cam.dir().to_array(), pulls.contains(&t), false); }
            }
            // With `--skip-cutscenes`, a skipped cut-scene's hand-back moves Bond: put him back on the `--bond` spot (and
            // let one tick settle) so the image pair matches the real game's placement.
            if skip_cuts {
                if let Some(i) = args.iter().position(|a| a == "--bond") {
                    let v: Vec<f32> = args.get(i + 1).map(|s| s.split(',').filter_map(|t| t.parse().ok()).collect()).unwrap_or_default();
                    if v.len() >= 3 { sim.bond = nf_game::player::Bond::new([v[0], v[1], v[2]], v.get(3).copied().unwrap_or(sim.bond.yaw)); if let Some(&pt) = v.get(4) { sim.bond.pitch = pt; } sim.step_bond(shot_input, false, false, false); }
                }
            }
            if walk { let h = sim.bond.head(); cam = Camera { pos: Vec3::new(h[0], h[1], h[2]), yaw: sim.bond.yaw, pitch: sim.bond.pitch * std::f32::consts::FRAC_PI_2, fov: GAME_FOV / sim.zoom }; }
            if walk { if let Some((p, yaw, pitch, fov)) = sim.script_camera() { cam = Camera { pos: Vec3::new(p[0], p[1], p[2]), yaw, pitch, fov }; } }
            eprintln!("Bond at {:?}{}, health {}", sim.bond.pos, if sim.disabled { " (disabled by a script)" } else { "" }, sim.drones.health.health);
            for u in sim.unposed(&scene.characters) { eprintln!("unposed: {u}"); }
            for p in &sim.drones.pickups { eprintln!("pickup: weapon {:#x}, {} rounds at {:?}", p.weapon, p.rounds, &p.m[3][..3]); }
            if let Some(dr) = sim.drones.list.iter().find(|d| !d.shots.is_empty() || d.state != 4) {
                eprintln!("AI: a drone in state {:#x}, {} shots, clip {}", dr.state, dr.shots.len(), dr.clip);
            }
        }
        // fix-armour-ring: `--armour <n>` writes BLData+0x8b0 after the ticks (the pane's armour — the
        // same write the dev panel's armour row and cheats.py's god make every frame), so the health
        // pane's armour state renders headless.
        if let Some(i) = args.iter().position(|a| a == "--armour") {
            let n: f32 = args.get(i + 1).context("--armour needs a value (0..=50)")?.parse()?;
            if let Some(sim) = scene.sim.as_mut() { sim.drones.health.armour = n; }
        }
        let (mut menu_frame, mut movie_frame) = front_end_menu(&args, &scene, front_end, &iso);
        // `--loading`: the level's loading screen (crate::loading) — the picture on the movie plane, the text and the
        // dots (`--frame` vblanks into the animation) over it.
        if args.iter().any(|a| a == "--loading") {
            let level = u32::from_str_radix(level.trim_end_matches(".bin"), 16).unwrap_or(0);
            if let Some(art) = read_files_bin(&iso, loading::ARCHIVE).map(|b| loading::art(&b)) {
                movie_frame = art.get(&loading::image_for(level)).cloned();
            }
            let lang = scene.sim.as_ref().and_then(|s| s.lang.clone());
            let mut seed = 0x1234_5678u32;
            let hint = loading::hint_for(level, || { seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345); seed >> 16 });
            if let Some(t) = scene.hud.as_ref() {
                menu_frame = Some(menu::loading_overlay(&t.font, lang.as_ref(), &scene.hud_textures, 1280.0 / 720.0, level, hint, loading::dots_lit(time as u32)));
            }
        }
        // fix-ingame-dev-menu: `--dev-page <name>` draws the F2 menu's page (menu/weapons/gadgets/
        // grenades/ammo/npcs/teleport/level/toggles/screenshot) over the game HUD, so the main agent
        // can render and check each page headless.
        if let Some(pg) = args.iter().position(|a| a == "--dev-page") {
            let name = args.get(pg + 1).context("--dev-page needs a page name")?;
            let page = dev::page_index(name).with_context(|| format!("--dev-page: unknown page {name:?} (one of: {})", dev::PAGES.join(", ")))?;
            let d = dev::Dev { menu: true, page, ..Default::default() };
            let f = dev::hud(&d, &scene, 1280.0 / 720.0, [[0.0; 4]; 4]);
            menu_frame.get_or_insert_with(|| overlay_for(&scene, 1280.0 / 720.0)).append(&f);
        }
        return shot(&scene, &cam, args.get(i + 1).context("--shot needs a file name")?, choice, time, opts, notex, nofog, menu_frame.as_ref(), movie_frame.as_ref());
    }
    let event_loop = EventLoop::new()?;
    // Sound: the level's bank in the driver, the default output device (silent without one).
    let mut scene = scene;
    let sound = build_sound(&mut scene);
    // `--dev --scenario <name>`: place Bond and set up the world for a specific feature test. (B-3a/b: the two
    // scenario systems are merged into `dev::scenario` / `dev::SCENARIOS` in `dev.rs`; `apply_scenario` is gone.)
    #[cfg(feature = "gamepad")]
    let pad_backend = gamepad::Backend::new(&iso);
    // `--hold-npcs` (dev): the window run holds every guard in place, still shootable.
    if dev && args.iter().any(|a| a == "--hold-npcs") { if let Some(sim) = scene.sim.as_mut() { sim.drones.hold_npcs = true; } }
    let force_channel = if dev { args.iter().position(|a| a == "--set-channel").and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<u8>().ok()) } else { None };
    let mut app = App { sound, scene, cam, gfx: None, keys: HashSet::new(), grabbed: false, trigger: false, aiming: false, last: std::time::Instant::now(), choice, anim_time: time, paused: false, opts, noclip: false, just: HashSet::new(), turn: 0.0, look: 0.0, weapon_cycle: 0, gadget_cycle: 0, weapon_switch_to: None, zoom: false, dev: dev.then(|| Box::new(dev::Dev { infinite_ammo: true, ..Default::default() })), notex, nofog, iso, pending_level: None, pending_weapon_assets: None, entry_save: None, all, recorder: None, replay: None, replay_idx: 0, prev_lock_ch: false, ticked: false, frame_ticks: 0, menu_clock: 0.0, selected_difficulty: 2, zoom_cycle: 0, pending_mission_end: false, force_channel, cam_prev: None, cam_cur: None, pal_prev: None, pal_cur: None, fps_cap, frame_start: std::time::Instant::now(), panel: None, panel_level: 0, panel_frames: 0, fe: None, level_code: 0, fe_music_page: None, menu_handle: 0, lang: None, #[cfg(feature = "gamepad")] pad: pad_backend,
        movie: None, devwin: None,
        audio_paused: false, loading_art: None, menu_pointer: None,
        net: None, net_lobby: None, net_local_slot: 0, net_tick: 0, pending_mp_settings: None, net_armed: None,
        async_load: None,
        music_bank_flip: false,
        music_loaded_bank: 0xc,
        music_master: 70, };
    if dev {
        // fix-ingame-dev-menu: the browser panel is no longer part of `--dev` — the in-window F2
        // menu carries ALL of its functions now (through the same `panel::apply` command layer).
        // `--panel` puts the HTTP page back for anyone who wants a second screen.
        app.panel = if args.iter().any(|a| a == "--panel") { panel::start() } else { None };
        match &app.panel {
            Some(_) => eprintln!("dev panel: open http://127.0.0.1:{} in a browser (beside the game window)", panel::PORT),
            None if args.iter().any(|a| a == "--panel") => eprintln!("dev panel: port {} busy (another nf-view?) — panel not started", panel::PORT),
            None => {}
        }
        if args.iter().any(|a| a == "--arena") {
            let (status, _) = panel::apply(&mut app.scene, &[("do".to_string(), "arena".to_string())]);
            eprintln!("[arena] {status}");
        }
        if let (Some(d), Some(n)) = (app.dev.as_deref_mut(), scenario.as_ref()) {
            if let Some((_, f)) = dev::scenario(n) {
                f(&mut app.scene);
                d.scenario = Some(leak_str(n));
            } else {
                eprintln!("[dev] unknown scenario '{n}'. Available: {}", dev::SCENARIOS.join(", "));
            }
        }
    }
    // fix-menus: the front end — the level code, the game's strings, and the boot page.
    app.level_code = u32::from_str_radix(level.trim_end_matches(".bin"), 16).unwrap_or(0);
    if front_end {
        app.fe = Some(menu::FrontEnd::new());
        // frontend-music: the boot front end's own `Sound_Ready` case 0x7000048 consumes the
        // flip [M 0x1dceb8:94-101 — DAT_002e1b11 (.bss) = 0 → bank 0xc resident, DAT → 1;
        // P_START's once arm pins it right after, 0x20a408:31]. The port consumes it here so
        // the first mission-return load flips to 0x38 like the game's, and re-points the
        // driver's primary at the boot bank's slot.
        if let Some(fe) = app.fe.as_mut() {
            let hash = fe.music.front_end_sound_ready();
            if let Some(s) = app.sound.as_mut() { s.driver.set_primary(if hash == 0x38 { 1 } else { 0 }); }
        }
        // The front end needs the game's strings even when the front-end map carries no Sim
        // (`Txt_LoadLanguage` 0x1caad0 loads `FNames[SelLang]` = USATxt.dat [M lib.rs load_impl]).
        match read_files_bin(&app.iso, "USATxt.dat") {
            Some(bytes) => match nf_formats::hud::Lang::parse(&bytes) {
                Ok(lang) => app.lang = Some(lang),
                Err(e) => eprintln!("USATxt.dat: {e} — menu labels show their label ids"),
            },
            None => eprintln!("no USATxt.dat in FILES.BIN — menu labels show their label ids"),
        }
        eprintln!("[front end] title screen (P_START 0x40000034); Enter/click = start, arrows = move, Esc = back");
    }
    // `--bond x,y,z` in the window too (it only applied to the headless `--frame` path): Bond starts exactly there.
    if let Some(i) = args.iter().position(|a| a == "--bond") {
        if let (Some(v), Some(sim)) = (args.get(i + 1), app.scene.sim.as_mut()) {
            let v: Vec<f32> = v.split(',').filter_map(|t| t.parse().ok()).collect();
            if v.len() >= 3 { sim.bond = nf_game::player::Bond::new([v[0], v[1], v[2]], v.get(3).copied().unwrap_or(sim.bond.yaw)); if let Some(&pt) = v.get(4) { sim.bond.pitch = pt; } }
        }
    }
    // net-match-host: `--net-host [port] [arena]` listens for clients and starts the match (the
    // demo settings: an Arena deathmatch with a fresh per-match seed) when the first one joins;
    // `--net-join host:port` dials it. The lobby pages are task 052's — these flags are the
    // runnable path until they land. The seed comes from the wall clock OUTSIDE the Sim: it is
    // matchmaking data (sent to every machine in the START packet), never sim state.
    if let Some(i) = args.iter().position(|a| a == "--net-host") {
        let port = args.get(i + 1).and_then(|v| v.parse::<u16>().ok()).unwrap_or(27420);
        let arena = args.get(i + 2).and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()).unwrap_or(0x0700_0023);
        let mut st = nf_game::mp::MpSettings::deathmatch(arena);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1);
        st.seed = nanos | 1;
        app.net_armed = Some(st);
        if let Err(e) = app.net_host(port) { eprintln!("[net] host failed: {e}"); }
        else { eprintln!("[net] hosting on udp/{port}, arena {arena:08x} — waiting for a client"); }
    }
    if let Some(i) = args.iter().position(|a| a == "--net-join") {
        let addr = args.get(i + 1).cloned().unwrap_or_else(|| "127.0.0.1:27420".to_string());
        if let Err(e) = app.net_join(&addr) { eprintln!("[net] join failed: {e}"); }
        else { eprintln!("[net] dialing {addr}"); }
    }
    // `--give id[,id...]` (hex like 0x50 or decimal): equip those weapons/gadgets at the start, the way the dev panel's
    // weapon button does (`Player_EquipWeapon` 0x1a8a58 + three clips of ammo), and draw the first one. For playtest
    // checks that need a gadget the level hasn't handed out yet.
    if let Some(i) = args.iter().position(|a| a == "--give") {
        if let (Some(list), Some(sim)) = (args.get(i + 1), app.scene.sim.as_mut()) {
            let wd = sim.weapons.clone();
            let mut first = None;
            for t in list.split(',') {
                let t = t.trim();
                let id = if let Some(h) = t.strip_prefix("0x") { usize::from_str_radix(h, 16).ok() } else { t.parse().ok() };
                let Some(id) = id.filter(|&i| i < 0x72) else { eprintln!("--give: bad id {t:?}"); continue };
                let rounds = wd.clip_size(id).max(1);
                if sim.drones.inventory.equip_weapon(&wd, id, rounds, true) {
                    sim.drones.inventory.equip_ammo(&wd, id, (rounds * 3).max(rounds));
                    // The panel give's twin fill (fix-aims20-modes-lights, panel.rs): a twin-linked record
                    // (+0x05, the AIMS-20 0x1a ↔ its grenade 0x1b) needs rounds in the twin's own slot or
                    // the mode press is refused (`Player_WeaponHasAmmo` 0x1a16c0) — the playtest check's
                    // `--give 0x1a` left B dead and the grenade light dark.
                    let twin = wd.records.get(id).map(|r| r[5] as i8).unwrap_or(0);
                    if twin != 0 {
                        let t = (id as i32 + twin as i32) as usize;
                        if let Some(c) = sim.drones.inventory.clip.get_mut(t) {
                            if *c == 0 { *c = wd.clip_size(t).max(1); }
                        }
                    }
                    first.get_or_insert(id as u8);
                    eprintln!("--give: equipped {id:#04x}");
                } else if sim.drones.inventory.owned.get(id).copied().unwrap_or(false) {
                    // Already owned and the game's own equip refuses it (the ammo-type cap — the Q-Claw
                    // 0x50 on 07000005 is owned at the start [L Inventory::level_start] with ammo type 0's
                    // cap 0 [L +0x90/+0x92], so `Player_EquipWeapon` 0x1a8a58's owned branch declines it
                    // [M]). The give's contract is to DRAW the thing: the switch request is the game's own
                    // `Player_WeaponSelect` path for an owned weapon. Without this the playtest's
                    // `--give 0x50` printed "refused" and Bond stayed on the P2K — firing gave the real
                    // bullet impact instead of the grapple.
                    first.get_or_insert(id as u8);
                    eprintln!("--give: {id:#04x} already owned — drawing it");
                } else {
                    eprintln!("--give: {id:#04x} refused (no first-person model)");
                }
            }
            if let Some(id) = first { sim.gun.switch_request = id; }
        }
    }
    // fix-restart-health: the level opened from the command line is a load too — the game re-saves
    // the RamSave buffer with the post-load state on EVERY load [M `Player_InitWeapon` 0x1a7ef8:325],
    // so a fail-Restart serves health 100. Only the in-play level change set `entry_save`; the
    // first level kept None and its Restart served the zero buffer's 50 floor (the main agent's
    // dry run on 07000005). Taken after `--give` so the restart keeps the given gear.
    app.entry_save = app.scene.sim.as_ref().map(entry_checkpoint_save);
    // B-5: start the recorder AFTER the scenario is applied (so the header captures the post-scenario starting state:
    // Bond near a guard, the RNG seed, etc.). On replay, the same starting state is reproduced.
    if let Some(path) = record_path {
        app.recorder = Some(replay::Recorder::start(&app.scene, &path, &level_for_record, scenario.as_deref(), dev)?);
        eprintln!("[replay] recording to {path} (Esc-Esc to stop & write)");
    }
    // B-5: hand the parsed recording to the App; the tick loop reads from it instead of the keyboard.
    if let Some(r) = replay_recording { app.replay = Some(r); app.replay_idx = 0; app.grab(true); }
    event_loop.run_app(&mut app)?;
    // B-5: flush the recording on a clean exit (Esc-Esc). A panic does NOT flush (the file would be partial).
    if let Some(r) = app.recorder.take() { let _ = r.finish(); }
    Ok(())
}

/// Leak a `String` to a `&'static str` (the scenario name lives for the program's life).
fn leak_str(s: &str) -> &'static str { Box::leak(s.to_string().into_boxed_str()) }

/// The pause OBJECTIVES tab's rows: the mission's visible objectives (label / hint / done) [M C_GCPAUSE_Handler
/// 0x204408:251-268 lists `Mission_NumVisObjectives` rows; 0x182540 counts the objectives whose state (record+0x14)
/// is > 1 or < 0; a row is done at state 4].
fn pause_objective_rows(sim: &scene::Sim) -> Vec<menu::PauseObjective> {
    // objective-hints [M C_GCPAUSE_Handler 0x204408:251-271]: the rows come from
    // Mission_NumVisObjectives 0x182540 + Mission_ObjectiveState 0x182630 — the WHOLE mission's
    // announced objectives in LEVELS order (prior parts included, future parts and unannounced
    // rows excluded), not just the current level's record. state 4 = the ~J done marker; the
    // hint stays 0xffffffff for hint-less records (the view's ✕ applies the 0x31d fallback).
    let m = &sim.drones.mission;
    (0..m.num_vis_objectives()).filter_map(|i| m.objective_state(i))
        .map(|o| menu::PauseObjective { text: o.text, hint: o.hint, done: o.state == 4 })
        .collect()
}
