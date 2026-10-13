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
pub mod hud;
pub mod scene;
pub mod replay;
pub mod panel;

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

#[derive(Clone, Copy)]
struct Camera { pos: Vec3, yaw: f32, pitch: f32, fov: f32 }

/// Render interpolation (G-043): logic runs at a fixed 60 Hz; frames between two ticks draw `t` of the way from
/// the previous tick's camera and palette to the latest one (one tick behind, the standard fixed-step scheme).
/// Jumps larger than `SNAP` (cut-scene cuts, teleports, hide/show) snap instead of blending.
const SNAP: f32 = 4.0;
fn lerp_cam(a: &Camera, b: &Camera, t: f32) -> Camera {
    if a.pos.distance(b.pos) > SNAP || (a.fov - b.fov).abs() > 0.2 { return *b; }
    let mut dy = b.yaw - a.yaw;
    while dy > std::f32::consts::PI { dy -= std::f32::consts::TAU; }
    while dy < -std::f32::consts::PI { dy += std::f32::consts::TAU; }
    Camera { pos: a.pos.lerp(b.pos, t), yaw: a.yaw + dy * t, pitch: a.pitch + (b.pitch - a.pitch) * t, fov: a.fov + (b.fov - a.fov) * t }
}
fn lerp_palette(a: &[nf_formats::anim::Mat4], b: &[nf_formats::anim::Mat4], t: f32) -> Vec<nf_formats::anim::Mat4> {
    if a.len() != b.len() { return b.to_vec(); }
    a.iter().zip(b).map(|(x, y)| {
        let d = (0..3).map(|k| (x[3][k] - y[3][k]).powi(2)).sum::<f32>().sqrt();
        if d > SNAP { return *y; }
        let mut m = *y;
        for r in 0..4 { for c in 0..4 { m[r][c] = x[r][c] + (y[r][c] - x[r][c]) * t; } }
        m
    }).collect()
}

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
    load_impl(iso, archive, all, None, false)
}

/// `--dev`: the same load with the dev view-model merge (folded 190-fix-dev2). Normal play never
/// passes `dev = true`, so no extra archive is opened without the flag.
pub fn load_dev(iso: &str, archive: &str, all: bool) -> Result<scene::Scene> {
    load_impl(iso, archive, all, None, true)
}

/// The sim catch-up cap (`NF_MAXTICKS`, default 4) — see the frame loop.
static MAX_TICKS: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

/// `load` with the level-transition carry-over (fix-level2). `Some(save)` = a warm transition:
/// `Player_Start` (0x190fd0) picks the type-0x24 start whose gate channel the previous level's exit
/// wrote, and `Player_InitWeapon` (0x1a7ef8) runs the per-level loadout switch over `Player_RamLoad`
/// (0x1957a8). `None` = a cold load (the game's boot path; `Player_RamLoad`'s whole body is skipped).
pub fn load_with_carry(iso: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>) -> Result<scene::Scene> {
    load_impl(iso, archive, all, save, false)
}

/// `--dev` level transition: the carry-over load with the dev view-model merge.
pub fn load_with_carry_dev(iso: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>) -> Result<scene::Scene> {
    load_impl(iso, archive, all, save, true)
}

fn load_impl(iso_path: &str, archive: &str, all: bool, save: Option<&nf_game::player::RamSave>, dev: bool) -> Result<scene::Scene> {
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
    let mut level_scripts: HashMap<u32, nf_formats::anim::Script> = HashMap::new();
    if dev {
        merge_dev_view_models(&iso, &read, &archive[..8], &mut level, &mut skeletons, &mut sequences, &mut level_scripts)?;
    }


    // G-021 (models-characters): drone placements whose skin (placement param 0) is not in their
    // level's archive — 33 of them on 2 levels [D scan]. TWO mechanisms, read off the decompile:
    //
    // 1. `AnimObjectNew` (0x1be910 disasm 0x1be958-0x1be9b0) SUBSTITUTES 0x05000090 before the
    //    hashtable get: on GameState+0x12 == 0x0700000c → 0x05000087 (0700000c's own skin), on
    //    0x07000014 → 0x05000093 (07000014's own). The port mirrors it at load: the placement hash
    //    0x05000090 resolves to the substitute skin's data — exactly the object the game builds.
    // 2. 07000014's 0x050000b1 (11 placements) has NO substitution (the disassembly covers only
    //    0x05000090) — the game's skin table accumulates every archive loaded in the session
    //    (`AnimLoadFile` 0x1d8248 registers per map; nothing frees between levels), and 0x050000b1
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
        }
        let mm = level.main_map();
        // The substitution [M 0x1be910]: 0x05000090 → the level's own variant, before anything
        // else looks the hash up. `GameState+0x12` is the current level id.
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
        scripts: {
            let mut sc: HashMap<u32, nf_formats::anim::Script> = lb.entries.iter().filter(|e| e.kind == 5)
                .filter_map(|e| nf_formats::anim::Script::parse(lb.data(&data, e)).ok()).map(|x| (x.hashcode, x)).collect();
            // The dev view-model merge's foreign anim scripts (the merged FP skins' idle/fire/reload)
            // join the level's own; a hash the level has stays local.
            for (h, x) in level_scripts { sc.entry(h).or_insert(x); }
            sc
        },
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
    let mut scene = scene::Scene::build(&level, !all, &skeletons, sequences, Some(&drones));
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
                scene.start = Some((s.pos, s.yaw));
            }
        } else if let Some((pos, yaw)) = scene.start {
            // No type-0x24 starts parsed: keep the "Player1" stand-in.
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, pos, yaw);
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
fn wanted_weapon_models(level: &Level, wd: &nf_formats::weapon::WeaponData) -> Vec<u32> {
    let mut needed: Vec<u32> = Vec::new();
    let mut want = |h: u32, needed: &mut Vec<u32>| {
        if h != 0 && h != u32::MAX && !level.by_hash.contains_key(&h) && !needed.contains(&h) { needed.push(h); }
    };
    for r in &wd.records {
        for off in [0x80usize, 0x4c, 0x5c] {
            want(u32::from_le_bytes(r[off..off + 4].try_into().unwrap()), &mut needed);
        }
    }
    for h in SLEEVES { want(h, &mut needed); }
    needed
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
fn merge_weapon_models<R: std::io::Read + std::io::Seek>(iso: &IsoImage<R>,
    read: &impl Fn(&str) -> Result<Option<Vec<u8>>>, prefix: &str, level: &mut Level) -> Result<()> {
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF on the disc")?)?;
    let Ok(wd) = nf_formats::weapon::WeaponData::from_action_elf(&elf) else { return Ok(()) };
    let level_id = u32::from_str_radix(prefix, 16).unwrap_or(0);
    let mut needed = wanted_weapon_models(level, &wd);
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

/// The dev view models (folded 190-fix-dev2): index every archive's type-3 skins (skin hash → archive),
/// then merge every selectable weapon's first-person skin (+0xDC) that the level archive lacks — its
/// meshes (the skin's `extra` rigid parts and the level's sleeve), its skeleton, its 07F sequences and
/// the weapons' kind-5 anim scripts. `Scene::build` then gives every weapon a view model. Dev only.
fn merge_dev_view_models<R: std::io::Read + std::io::Seek>(iso: &IsoImage<R>,
    read: &impl Fn(&str) -> Result<Option<Vec<u8>>>, _prefix: &str, level: &mut Level,
    skeletons: &mut std::collections::HashMap<u8, nf_formats::character::Skeleton>,
    sequences: &mut std::collections::HashMap<u8, Vec<nf_formats::anim::Sequence>>,
    scripts: &mut std::collections::HashMap<u32, nf_formats::anim::Script>) -> Result<()> {
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF on the disc")?)?;
    let Ok(wd) = nf_formats::weapon::WeaponData::from_action_elf(&elf) else { return Ok(()) };
    // skin hash → archive (the first archive on the disc that holds it).
    static SKIN_ARCHIVES: std::sync::OnceLock<std::collections::HashMap<u32, String>> = std::sync::OnceLock::new();
    let index = SKIN_ARCHIVES.get_or_init(|| {
        let mut map = std::collections::HashMap::new();
        let Ok((entries, bin)) = disc::files_bin_entries(iso) else { return map };
        for e in entries.iter().filter(|e| e.name.len() == 12 && e.name.ends_with(".bin")) {
            let Ok(d) = iso.read_bytes(bin.lba as u64 * 2048 + e.offset as u64, e.size as u64) else { continue };
            let Ok(l) = nf_formats::levelbin::LevelBin::parse(&d) else { continue };
            for pe in l.entries.iter().filter(|pe| pe.kind == 3) {
                let raw = l.data(&d, pe);
                if raw.len() < 4 { continue; }
                let h = u32::from_le_bytes(raw[0..4].try_into().unwrap());
                if (h & 0xff00_0000) == 0x0500_0000 { map.entry(h).or_insert_with(|| e.name.clone()); }
            }
        }
        map
    });
    // The weapons' FP skins the level lacks + the four fists skins (`Player_Init` overrides record 1
    // per level; the merged set covers whichever the level asks for).
    const PUNCH_SKINS: [u32; 4] = [0x0500_00ab, 0x0500_0099, 0x0500_003d, 0x0500_00a1];
    let mut wanted: Vec<u32> = Vec::new();
    for w in 1..0x73usize {
        let fp = wd.first_person(w);
        if fp != 0 && !level.skins.contains_key(&fp) && !wanted.contains(&fp) { wanted.push(fp); }
    }
    for &p in &PUNCH_SKINS { if !level.skins.contains_key(&p) && !wanted.contains(&p) { wanted.push(p); } }
    let mut merged: Vec<String> = Vec::new();
    for skin in wanted.iter() {
        let Some(archive_name) = index.get(skin) else {
            eprintln!("dev view models: skin {skin:08x} is on no archive of the disc"); continue;
        };
        if merged.contains(archive_name) { continue; }
        merged.push(archive_name.clone());
        let fd = match read(archive_name) { Ok(Some(d)) => d, _ => continue };
        let Ok(mut foreign) = Level::parse(&fd) else { continue };
        // The skins the level lacks join the session table (the game's skin table accumulates the
        // same way [M AnimLoadFile 0x1d8248]); first-wins keeps the level's own.
        let mut new_skins = 0usize;
        for (h, sk) in foreign.skins.clone() {
            if level.skins.contains_key(&h) { continue; }
            level.skins.insert(h, sk);
            new_skins += 1;
        }
        // The foreign maps that hold the merged skins' meshes and the sleeves.
        let mut hashes = wanted_weapon_models(level, &wd);
        for (h, sk) in foreign.skins.iter() {
            if !level.skins.contains_key(h) { continue; }
            for mh in sk.main.iter().flatten() { hashes.push(*mh); }
            for (_, ph) in sk.extra.iter() { if let Some(x) = ph { hashes.push(*x); } }
        }
        // The foreign skeletons the level lacks; the merged skins' skeletons mark the sequence
        // buckets the view models will look up.
        let flb = nf_formats::levelbin::LevelBin::parse(&fd)?;
        let mut fp_skeletons: std::collections::HashSet<u8> = Default::default();
        for e in flb.entries.iter().filter(|e| e.kind == 6) {
            let sk = nf_formats::character::parse_skeleton(flb.data(&fd, e))?;
            if skeletons.contains_key(&(sk.id as u8)) { continue; }
            skeletons.insert(sk.id as u8, sk);
        }
        for h in wanted.iter() { if let Some(sk) = level.skins.get(h) { fp_skeletons.insert(sk.skeleton); } }
        // The foreign 07F sequences: a NEW skeleton gets its own bucket (no gameplay data touched);
        // a skeleton the view models use appends — dev only (the port's per-skeleton lists are
        // order-sensitive gameplay data for DRONES, which the FP hands skeleton never carries).
        let anim_name = format!("{}F{}", &archive_name[..2], &archive_name[3..]);
        if let Some(anim) = read(&anim_name)? {
            let alb = nf_formats::levelbin::LevelBin::parse(&anim)?;
            let mut appended = 0usize;
            for pe in alb.entries.iter().filter(|pe| pe.kind == 4 && pe.length >= 0x40) {
                let s = nf_formats::anim::Sequence::parse(alb.data(&anim, pe))?;
                if fp_skeletons.contains(&s.skeleton) { sequences.entry(s.skeleton).or_default().push(s); appended += 1; }
            }
            if appended > 0 { eprintln!("dev view models: {appended} sequences from {anim_name}"); }
        }
        // The foreign kind-5 anim scripts (the weapons' idle/fire/reload hashes) — missing hashes only.
        let mut new_scripts = 0usize;
        for e in flb.entries.iter().filter(|e| e.kind == 5) {
            if let Ok(sc) = nf_formats::anim::Script::parse(flb.data(&fd, e)) { if scripts.insert(sc.hashcode, sc).is_none() { new_scripts += 1; } }
        }
        let moved = take_maps_holding(level, foreign, &hashes);
        eprintln!("dev view models: merged {archive_name} for skin {skin:08x} ({new_skins} skins, {new_scripts} scripts, {moved} maps)");
    }
    Ok(())
}

/// The level's player starts: the main map's type-0x24 placements in placement order, with the gate
/// (parameter 0) [M `parsemap_create_dynamic_objects` 0x1d04f0 case 0x24 → `Player_AddNewStartPos`
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
        })
        .collect()
}

/// The level archive for a hashcode, `LoaderLoad` (0x1d4a00) style [M]: `sprintf("%s%8.8x.bin", …,
/// code & 0xff0fffff | …)` — the loader type's nibble (0x00F00000) is masked OFF for the level itself
/// (loader type 1; type 0/3 force 0xF00000 for the anim archives). `ResetMap_LevelToLoad` (0x1d3800)
/// stores e.g. 0x07100006 for a `LOAD_LEVEL` to 0x07000006 (param_3 == 0 remap), and the level load
/// then masks it back to 07000006.bin.
pub fn level_archive_name(code: u32) -> String { format!("{:08x}.bin", code & 0xff0f_ffff) }

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
    let right = f.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(f).normalize_or_zero();
    let mut out = sim.snow_quads(right.to_array(), up.to_array());
    out.extend(sim.particle_quads(right.to_array(), up.to_array()));
    // fix-impacts: the bullet decals, debris and casings as flat-colour quads (the real decal/casing/debris
    // meshes are still owed, see docs/reports/fix-impacts.md).
    out.extend(sim.effect_quads(right.to_array(), up.to_array()));
    out
}

/// One frame of sound (`Sound_UpdateListeners` → `Sound_UpdateSounds` → the batch → `SFXUpdate`): the listener is
/// Bond's eye and view (glb_viewer[0]; the cut-scene camera's listener call is dropped by the EE batcher [M]).
fn sound_tick(s: &mut nf_audio::sfx::Sound, sim: &mut scene::Sim) {
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
    let mut r = gpu::Renderer::new(&device, &queue, format, scene, notex, nofog);
    r.update_skin(&queue, scene, choice, time, opts);
    r.lights = scene.frame_lights();
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
        for _ in 0..n { r.update_skin(&queue, scene, choice, time, opts); }
        queue.submit([]); let _ = device.poll(wgpu::PollType::wait_indefinitely());
        eprintln!("[bench] GPU path: palette build + upload {:.2} ms", ms(t4, std::time::Instant::now()));
    }
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
        dimension: wgpu::TextureDimension::D2, format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let depth = gpu::depth_view(&device, w, h);
    let (vp, vp_rot) = cam.view_proj(w as f32 / h as f32);
    let snow = snow_for(scene, cam);
    let mut enc = r.render(&device, &queue, &view, &depth, vp, vp_rot, cam.pos.to_array(), time as u32, &snow, &overlay_for(scene, w as f32 / h as f32));
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
    cam_prev: Option<Camera>,
    cam_cur: Option<Camera>,
    pal_prev: Option<(Vec<nf_formats::anim::Mat4>, Vec<[f32; 4]>)>,
    pal_cur: Option<(Vec<nf_formats::anim::Mat4>, Vec<[f32; 4]>)>,
    /// `--fps N`: a frame-rate cap (vsync off); None = vsync.
    fps_cap: Option<f32>,
    frame_start: std::time::Instant,
    /// The gamepad backend (fix-input), behind the `gamepad` cargo feature. `None` without a pad
    /// device; with one, its Input merges into the keyboard's each tick (the game's own mapping,
    /// curves and ramps — docs/systems/input.md §8).
    #[cfg(feature = "gamepad")]
    pad: Option<gamepad::Backend>,
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
        // B-9: a click also moves the keyboard selection to the clicked row.
        if let Some(d) = self.dev.as_deref_mut() { d.menu_sel = idx; }
        let Some(action) = dev::menu_action(self.dev.as_deref().unwrap(), idx) else { return };
        let _ = spawn_pick;
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
        match shot(&self.scene, &self.cam, &png.to_string_lossy(), self.choice, self.anim_time, self.opts, self.notex, self.nofog) {
            Ok(()) => format!("saved {} ({line})", png.display()),
            Err(e) => format!("screenshot failed: {e} ({line})"),
        }
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
                    if let Some(w) = switch { self.weapon_switch_to = Some(w); }
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

    fn apply_menu_action(&mut self, action: dev::MenuAction) {
        let result = match action {
            dev::MenuAction::Close => { if let Some(d) = self.dev.as_deref_mut() { d.menu = false; } "menu closed".to_string() }
            dev::MenuAction::Spawn(pick) => {
                let eye = [self.cam.pos.x, self.cam.pos.y, self.cam.pos.z];
                let dir = self.cam.dir().to_array();
                let hit = self.scene.sim.as_ref().and_then(|s| s.world.segment(eye, [eye[0] + dir[0] * 30.0, eye[1] + dir[1] * 30.0, eye[2] + dir[2] * 30.0]));
                let pos = hit.map_or([eye[0] + dir[0] * 5.0, eye[1], eye[2] + dir[2] * 5.0], |t| [eye[0] + (dir[0] * 30.0) * t, eye[1] + (dir[1] * 30.0) * t, eye[2] + (dir[2] * 30.0) * t]);
                if let Some(d) = self.dev.as_deref_mut() { d.spawn = pick; }
                let gi = self.dev.as_deref().and_then(|d| d.spawn_guard(&mut self.scene, pos, self.cam.yaw));
                let kind = match pick { dev::SpawnPick::Normal => "normal", dev::SpawnPick::Sniper => "sniper", dev::SpawnPick::Ambush => "ambush" };
                match gi { Some(gi) => format!("guard #{gi}: spawned ({kind}) at ({:.1},{:.1},{:.1})", pos[0], pos[1], pos[2]), None => format!("spawn failed ({kind}) — no free slot or no sim") }
            }
            dev::MenuAction::ForceGrenade => {
                let bond = self.scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos);
                let gi = self.dev.as_deref_mut().and_then(|d| { d.force_grenade(&mut self.scene, bond); d.forced.last().copied() });
                match gi { Some(gi) => format!("guard #{gi}: forced grenade throw — watch the lob"), None => "no guard within 30 m to force".to_string() }
            }
            dev::MenuAction::Impulse => {
                let bond = self.scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos);
                let gi = self.dev.as_deref_mut().and_then(|d| { d.impulse_guard(&mut self.scene, bond); d.forced.last().copied() });
                match gi { Some(gi) => format!("guard #{gi}: impulse applied — watch the hop"), None => "no guard within 30 m to impulse".to_string() }
            }
            dev::MenuAction::SpecialDeath => {
                let bond = self.scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos);
                let gi = self.dev.as_deref_mut().and_then(|d| { d.special_death(&mut self.scene, bond); d.forced.last().copied() });
                match gi { Some(gi) => format!("guard #{gi}: SpecialDeath triggered — watch the fall"), None => "no guard within 30 m".to_string() }
            }
            dev::MenuAction::ToggleGod => { let on = self.dev.as_deref_mut().map_or(false, |d| { d.god_mode = !d.god_mode; d.god_mode }); format!("god mode: {}", if on { "ON" } else { "OFF" }) }
            dev::MenuAction::ToggleAmmo => { let on = self.dev.as_deref_mut().map_or(false, |d| { d.infinite_ammo = !d.infinite_ammo; d.infinite_ammo }); format!("infinite ammo: {}", if on { "ON" } else { "OFF" }) }
            dev::MenuAction::ToggleNoReload => { let on = self.dev.as_deref_mut().map_or(false, |d| { d.no_reload = !d.no_reload; d.no_reload }); format!("no reload: {}", if on { "ON" } else { "OFF" }) }
            dev::MenuAction::SaveSpot => { let p = self.scene.sim.as_ref().map_or([0.0; 3], |s| s.bond.pos); if let Some(d) = self.dev.as_deref_mut() { d.saved_spot = Some(p); } format!("spot saved at ({:.1},{:.1},{:.1})", p[0], p[1], p[2]) }
            dev::MenuAction::RestoreSpot => {
                let spot = self.dev.as_deref().and_then(|d| d.saved_spot);
                match spot { Some(p) => { if let Some(d) = self.dev.as_deref() { d.teleport(&mut self.scene, p, self.cam.yaw); } format!("restored to ({:.1},{:.1},{:.1})", p[0], p[1], p[2]) }, None => "no saved spot — use 'save spot' first".to_string() }
            }
            dev::MenuAction::TimeScale(s) => { if let Some(d) = self.dev.as_deref_mut() { d.time_scale = s; } format!("time scale: {}", if s == 0.0 { "paused (F11=step)" } else if s == 1.0 { "1×" } else { "0.25×" }) }
            // The level-select jump (fix-dev T5): the same path a level exit takes — a warm load with the
            // carry-over (`pending_level` consumed at the top of RedrawRequested, lib.rs).
            dev::MenuAction::Level(id) => { self.pending_level = Some(id); format!("loading {} ..", level_archive_name(id)) }
        };
        if let Some(d) = self.dev.as_deref_mut() { d.result_msg = Some(result); d.result_timer = 180; }
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
                match (k, event.state) {
                    (KeyCode::Escape, ElementState::Pressed) => if self.grabbed { self.grab(false) } else { el.exit() },
                    (KeyCode::F12, ElementState::Pressed) => { let m = self.spot_shot(); eprintln!("[spot] {m}"); }
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
                    // B-9: keyboard nav for the dev menu. ↑/↓ move, Enter activates the selected row.
                    (KeyCode::ArrowUp, ElementState::Pressed) => if let Some(d) = self.dev.as_deref_mut() { if d.menu { dev::menu_key(d, dev::MenuKey::Up); } },
                    (KeyCode::ArrowDown, ElementState::Pressed) => if let Some(d) = self.dev.as_deref_mut() { if d.menu { dev::menu_key(d, dev::MenuKey::Down); } },
                    (KeyCode::Enter, ElementState::Pressed) => {
                        // B-9: keyboard nav. Compute the action while holding the dev borrow, drop it, then apply.
                        let action = self.dev.as_deref_mut().and_then(|d| {
                            if d.menu { dev::menu_key(d, dev::MenuKey::Activate) } else { None }
                        });
                        if let Some(a) = action { self.apply_menu_action(a); }
                    }
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
                    // Gadgets (G): the cycle is live — the weapon_data's +0x04 selectable flag is 1 for
                    // the gadget ids [D ACTION.ELF weapon_data] and the switch/equip path is tested
                    // (gadget_select_live). The gadgets' fire-side substates land per gadget.
                    (KeyCode::KeyG, ElementState::Pressed) => if self.grabbed && !self.noclip { self.gadget_cycle = 1 },
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
                // with the next archive (`ResetMap_LevelToLoad` 0x1d3800 → `LoaderLoad` 0x1d4a00).
                if let Some(next) = self.pending_level.take() {
                    // `Trigger_Activate`'s exit order: the success/fallback channel write (already applied in
                    // the sim), then `Player_RamSave` [M 0x1b2318:97-116] — capture the old sim now, before
                    // the scene swap drops it (fix-level2 F2).
                    // fix-death-restart: the fail/death restart is the COLD arm — the menu's
                    // `ResetMap_LevelToLoad(Mission_BaseMapHCode(), 0, 1)` sets IsWarmReset = 0
                    // [M 0x204408:139], which zeroes the switch channels (`Init_SwitchChannels`
                    // [M 0x1d5080:44-46]) and skips `Player_RamLoad` (its whole body is gated on the
                    // warm flag [M 0x1957a8]). Carrying the RamSave across a restart is what looped
                    // the 07000007 session: the saved ch 99 re-failed the mission every reload.
                    let cold = self.scene.sim.as_ref().is_some_and(|sim| sim.drones.level_exit_cold);
                    let save = if cold { None } else {
                        self.scene.sim.as_ref().map(|sim| nf_game::player::RamSave::capture(
                            &sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels)
                            .with_mission(&sim.drones.mission))
                    };
                    let name = level_archive_name(next);
                    eprintln!("level: loading {name} ({})", if save.is_some() { "warm" } else { "cold" });
                    let load = if self.dev.is_some() { load_with_carry_dev } else { load_with_carry };
                    match load(&self.iso, &name, self.all, save.as_ref()) {
                        Ok(mut scene) => {
                            self.sound = build_sound(&mut scene);
                            if let Some(start) = scene.start { self.cam = Camera { pos: Vec3::new(start.0[0], start.0[1] + 1.6, start.0[2]), yaw: start.1, pitch: 0.0, fov: GAME_FOV }; }
                            self.scene = scene;
                            self.anim_time = 0.0;
                            // The renderer holds the old scene's GPU resources (textures, buffers): rebuild it
                            // IN PLACE — the window and surface must survive the level change (the game's window
                            // does not close). Dropping `Gfx` here closed the window and nothing recreated it
                            // (winit's `resumed` only runs at startup), so the process sat on a headless event
                            // loop with the audio still playing (the user's report).
                            if let Some(g) = &mut self.gfx {
                                g.renderer = gpu::Renderer::new(&g.device, &g.queue, g.config.format, &self.scene, self.notex, self.nofog);
                            }
                            eprintln!("level: loaded {name}");
                        }
                        Err(e) => eprintln!("level: failed to load {name}: {e}"),
                    }
                    // The lock channel's edge memory starts clean on the new level (the game's `_prev`
                    // comes back from the RamSave; 0x48 has no writer before the party).
                    self.prev_lock_ch = false;
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
                self.ticked = false;
                self.poll_panel();
                if !self.paused {
                    // Game logic at its fixed 60 Hz, decoupled from the render rate: one AI tick per whole tick of time.
                    // Dev mode's time scale changes the number of ticks run (WORKLOG 0081), not the tick rate.
                    let before = self.anim_time as u32;
                    self.anim_time += dt * 60.0;
                    // fix-perf-level: the catch-up cap. A slow render frame must not burst a spiral of
                    // ticks — on 07000006 a tick costs ~4.1 ms, so the old 8-tick cap let the sim
                    // burst 33 ms behind one frame (the jitter fix-render3 measured). The game itself
                    // has no catch-up: under load it slows down. Capped ticks are DROPPED, not
                    // deferred — logic stays fixed-60 Hz and deterministic (the tick count depends
                    // only on frame times, never on their content). `NF_MAXTICKS` retunes it.
                    let max_ticks = *MAX_TICKS.get_or_init(|| std::env::var("NF_MAXTICKS").ok()
                        .and_then(|v| v.parse::<u32>().ok()).filter(|v| *v > 0).unwrap_or(4));
                    let elapsed = (self.anim_time as u32).saturating_sub(before).min(max_ticks);
                    let n_ticks = self.dev.as_deref_mut().map_or(elapsed, |d| d.tick_count(elapsed));
                    self.ticked = n_ticks > 0;
                    let eye = [self.cam.pos.x, self.cam.pos.y, self.cam.pos.z];
                    let look = self.cam.dir().to_array();
                    if let Some(sim) = self.scene.sim.as_mut() {
                        for _ in 0..n_ticks {
                            if self.noclip { sim.step(eye, look, self.trigger, self.aiming); continue; }
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
                                    switch_to: self.weapon_switch_to.take(),
                                    fire: self.trigger,
                                    zoom: self.zoom,
                                };
                                (input, self.trigger, self.just.remove(&KeyCode::KeyR), self.just.remove(&KeyCode::KeyE))
                            };
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
                            // B-5 record: capture the exact input passed to step_bond, before the call.
                            if let Some(r) = &mut self.recorder { r.push(replay::TickInput::from_parts(&input, trigger, reload, use_key)); }
                            sim.step_bond(input, trigger, reload, use_key);
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
                                    nf_game::pickup::Taken::Key { id } => format!("  picked up key {id}"), // 0x2000010 [M 0x18e650:285-289]
                                    nf_game::pickup::Taken::Armour => "  picked up armour".to_string(),   // 0x200000F [M 0x18e650:290-294]
                                    nf_game::pickup::Taken::Bonus => "  picked up a 007 bonus".to_string(), // 0x200004A [M 0x18e650:315-321]
                                    nf_game::pickup::Taken::Label => "  picked up the item".to_string(), // case 4, label 0 on 07000008 [M 0x18e650]
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
                // G-043: build the palette only when a tick ran, then draw between the last two ticks.
                if self.ticked || self.pal_cur.is_none() {
                    let new = self.scene.skin_palette(&g.renderer.skin_layout, self.choice, self.anim_time, self.opts);
                    self.pal_prev = self.pal_cur.take();
                    self.pal_cur = Some(new);
                    self.cam_prev = self.cam_cur;
                    self.cam_cur = Some(self.cam);
                }
                let interp = !self.noclip && !self.paused && self.scene.sim.is_some() && std::env::var_os("NF_NOINTERP").is_none();
                let alpha = if interp { self.anim_time.fract() } else { 1.0 };
                let draw_cam = match (self.cam_prev, self.cam_cur) { (Some(a), Some(b)) if interp => lerp_cam(&a, &b, alpha), _ => self.cam };
                if let Some((pal, cols)) = &self.pal_cur {
                    match &self.pal_prev {
                        Some((pp, _)) if interp => g.renderer.upload_skin(&g.queue, &lerp_palette(pp, pal, alpha), cols),
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
                let aspect = g.config.width as f32 / g.config.height as f32;
                let (vp, vp_rot) = draw_cam.view_proj(aspect);
                let snow = snow_for(&self.scene, &draw_cam);
                let mut hud = overlay_for(&self.scene, aspect);
                if let Some(d) = self.dev.as_deref() { if d.overlay || d.menu { hud.append(&dev::hud(d, &self.scene, aspect, vp)); } }
                g.renderer.lights = self.scene.frame_lights();
                let enc = g.renderer.render(&g.device, &g.queue, &view, &g.depth, vp, vp_rot, draw_cam.pos.to_array(), self.anim_time as u32, &snow, &hud);
                g.queue.submit([enc.finish()]);
                g.queue.present(frame);
                if let Some(fps) = self.fps_cap.filter(|f| *f > 0.0) {
                    let target = std::time::Duration::from_secs_f32(1.0 / fps);
                    let el = self.frame_start.elapsed();
                    if el < target { std::thread::sleep(target - el); }
                }
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
    const USAGE: &str = "usage: nf-view [<iso>] <level> [--all] [--seq K] [--frame F] [--fire] [--reinforcements] [--dev --scenario <name>] [--record <file>] [--replay <file>] [--no-fog] [--no-tex] [--shot out.png [--cam x y z yaw pitch]]\n\
        <level> is a FILES.BIN name such as 07000005 (\".bin\" optional); without <iso> the image path comes from NF_ISO\n\
        --dev --scenario <name>: place Bond for a feature test (weapon-switch, sg5-pickup, wire-music, autoaim)\n\
        --record <file>: record every logic tick's input to <file> (B-5). --replay <file>: replay a recording.\n\
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
    let scene = if dev { load_dev(&iso, &level, all)? } else { load(&iso, &level, all)? };
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
                sim.bond = nf_game::player::Bond::new([v[0], v[1], v[2]], v.get(3).copied().unwrap_or(sim.bond.yaw)); if let Some(&pt) = v.get(4) { sim.bond.pitch = pt; }
            }
            // `--skip-cutscenes`: hold the game's own skip while a script camera runs (`Script_FFwd` 0x1dac78), for image pairs.
            let skip_cuts = args.iter().any(|a| a == "--skip-cutscenes");
            for t in 1..=time as u32 {
                if skip_cuts && sim.disabled { sim.drones.scripts.skip_request = true; }
                if walk { sim.step_bond(Default::default(), pulls.contains(&t), false, false); } else { sim.step([cam.pos.x, cam.pos.y, cam.pos.z], cam.dir().to_array(), pulls.contains(&t), false); }
            }
            // With `--skip-cutscenes`, a skipped cut-scene's hand-back moves Bond: put him back on the `--bond` spot (and
            // let one tick settle) so the image pair matches the real game's placement.
            if skip_cuts {
                if let Some(i) = args.iter().position(|a| a == "--bond") {
                    let v: Vec<f32> = args.get(i + 1).map(|s| s.split(',').filter_map(|t| t.parse().ok()).collect()).unwrap_or_default();
                    if v.len() >= 3 { sim.bond = nf_game::player::Bond::new([v[0], v[1], v[2]], v.get(3).copied().unwrap_or(sim.bond.yaw)); if let Some(&pt) = v.get(4) { sim.bond.pitch = pt; } sim.step_bond(Default::default(), false, false, false); }
                }
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
    // `--dev --scenario <name>`: place Bond and set up the world for a specific feature test. (B-3a/b: the two
    // scenario systems are merged into `dev::scenario` / `dev::SCENARIOS` in `dev.rs`; `apply_scenario` is gone.)
    #[cfg(feature = "gamepad")]
    let pad_backend = gamepad::Backend::new(&iso);
    let mut app = App { sound, scene, cam, gfx: None, keys: HashSet::new(), grabbed: false, trigger: false, aiming: false, last: std::time::Instant::now(), choice, anim_time: time, paused: false, opts, noclip: false, just: HashSet::new(), turn: 0.0, look: 0.0, weapon_cycle: 0, gadget_cycle: 0, weapon_switch_to: None, zoom: false, dev: dev.then(|| Box::new(dev::Dev { infinite_ammo: true, ..Default::default() })), notex, nofog, iso, pending_level: None, all, recorder: None, replay: None, replay_idx: 0, prev_lock_ch: false, ticked: false, cam_prev: None, cam_cur: None, pal_prev: None, pal_cur: None, fps_cap, frame_start: std::time::Instant::now(), panel: None, panel_level: 0, panel_frames: 0, #[cfg(feature = "gamepad")] pad: pad_backend };
    if dev {
        app.panel = panel::start();
        match &app.panel {
            Some(_) => eprintln!("dev panel: open http://127.0.0.1:{} in a browser (beside the game window)", panel::PORT),
            None => eprintln!("dev panel: port {} busy (another nf-view?) — panel not started", panel::PORT),
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
    // `--bond x,y,z` in the window too (it only applied to the headless `--frame` path): Bond starts exactly there.
    if let Some(i) = args.iter().position(|a| a == "--bond") {
        if let (Some(v), Some(sim)) = (args.get(i + 1), app.scene.sim.as_mut()) {
            let v: Vec<f32> = v.split(',').filter_map(|t| t.parse().ok()).collect();
            if v.len() >= 3 { sim.bond = nf_game::player::Bond::new([v[0], v[1], v[2]], v.get(3).copied().unwrap_or(sim.bond.yaw)); if let Some(&pt) = v.get(4) { sim.bond.pitch = pt; } }
        }
    }
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
