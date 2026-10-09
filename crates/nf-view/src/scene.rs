//! CPU side of the viewer: a level archive turned into vertex/index buffers grouped by layer and material, plus the
//! per-vertex skinning data that animates characters each frame.
//!
//! Material = (map, texture index) — a batch's texture index refers to the texture list of the map its object comes
//! from (docs/formats/ps2gfx.md). Vertex colour stays on the PS2 scale (0x80 = 1.0); the shader rescales it.
//! Layers follow the game's frame: the sky pass first (`View_DrawSky`, objects centred on the viewer, then fixed sky
//! objects), then the world; characters are drawn like the world but their vertices sit at the end of the buffer and
//! are rewritten every frame (docs/formats/animation.md).

use nf_formats::anim::{self, Mat4, Script, Sequence};
use nf_formats::character::{Skeleton, Skin};
use nf_formats::drone::{self, DroneAnimData};
use nf_formats::level::{Instance, Level};
use nf_formats::placement::transform;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub rgba: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Sky objects with glist flag bit 2: drawn around the viewer (vertices in object space, rotation only).
    SkyCamera,
    /// Sky objects at their placement translation, drawn in the sky pass (no depth).
    SkyFixed,
    World,
    /// Skinned character meshes; vertices rewritten each frame.
    Character,
    /// Additive effects posed with the characters (muzzle flashes); after Character so the rewritten vertex tail
    /// stays contiguous.
    Effect,
}

pub struct Material {
    pub layer: Layer,
    /// `(map, texture)`; `None` = untextured.
    pub key: Option<(usize, u32)>,
    pub first_index: u32,
    pub index_count: u32,
}

pub struct TextureImage {
    pub key: (usize, u32),
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// One drawn character mesh: its placement matrix, skin, skeleton and slot → bone list, the animation the game starts
/// it with, and the floor straight below it.
pub struct Character {
    pub matrix: Mat4,
    pub skin: Skin,
    pub skeleton: Skeleton,
    pub slot_bones: Vec<u8>,
    pub start: Option<StartAnim>,
    pub role: drone::SpawnRole,
    /// Highest World triangle below the placement (stand-in for the game's collision floor, docs/systems/drone-animation.md).
    pub floor: Option<f32>,
}

/// The script a drone starts with (`NDrone2_DefaultInit` → `DroneAnim_SetAnimD`, or placement parameter 1) and the
/// sequences its opcode-0 commands play: `(first script frame, last script frame, index into the skeleton's list)`.
pub struct StartAnim {
    pub script: Script,
    pub sequences: Vec<(u16, u16, usize)>,
}

/// What the drone setup needs besides the level: ACTION.ELF's tables, the level's scripts, the level id, and
/// `weapon_data` (built by running its static initialiser).
pub struct DroneContext {
    pub tables: DroneAnimData,
    pub scripts: HashMap<u32, Script>,
    pub level_id: u32,
    pub weapons: Option<nf_formats::weapon::WeaponData>,
    pub props: Option<nf_formats::drone::PropertyTables>,
    /// The level's tuning (`ReadTuningVars` applied), hit boxes and sound hearing data.
    pub tuning: nf_formats::tuning::Tuning,
    pub hitboxes: Option<nf_formats::hitbox::CollisionBones>,
    pub sfx: Option<nf_formats::sfx::SfxTable>,
}

/// The running game simulation (nf-game): drones plus the data they consult, and which character mesh shows which
/// drone. Stepped at the game's fixed 60 Hz by the app; `animate` reads it.
pub struct Sim {
    pub drones: nf_game::drone::Drones,
    pub tables: DroneAnimData,
    pub props: nf_formats::drone::PropertyTables,
    pub scripts: HashMap<u32, Script>,
    pub weapons: nf_formats::weapon::WeaponData,
    pub world: nf_game::world::CollisionWorld,
    pub level: u32,
    /// Per character: its drone.
    pub char_drone: Vec<Option<usize>>,
    /// Sequence hashcode → (skeleton, index in `Scene::sequences`).
    pub seq_index: HashMap<u32, (u8, usize)>,
    /// The same sequences by hashcode (root motion in the AI).
    pub seqs: HashMap<u32, Sequence>,
    pub skins: HashMap<u32, Skin>,
    pub skeletons: HashMap<u8, Skeleton>,
    pub nav: nf_game::nav::Network,
    pub covers: Vec<nf_game::cover::CoverNode>,
    pub tuning: nf_formats::tuning::Tuning,
    pub hitboxes: nf_formats::hitbox::CollisionBones,
    pub sfx: Option<nf_formats::sfx::SfxTable>,
    pub volumes: Vec<nf_game::aivolume::AiVolume>,
    pub models: HashMap<u32, nf_game::pickup::Model>,
    /// Bond's gun (07000005's start: P2K, 7 + 41 rounds) and the auto-aim option (`PlayerSetting+1` = 1 in the game
    /// [L]; off by default with a mouse, D-001).
    pub gun: nf_game::gun::Gun,
    /// Bond on foot (docs/systems/player.md).
    pub bond: nf_game::player::Bond,
    /// Aim mode's cursor (`BL+0x118 / +0x11C`) and its smoothed copy (`+0x120 / +0x124`), −1…1 across half the screen.
    pub cursor: [f32; 2],
    pub cross: [f32; 2],
    /// The first-person gun's script (`Player_SetWeaponAnimObj`), its time in frames and speed, and the bob phases
    /// (`BL+0x8DC` from the forward speed, `BL+0x8E8` idle time).
    pub view_script: u32,
    pub view_time: f32,
    pub view_speed: f32,
    pub bob: [f32; 2],
    /// Aim mode (action 0x13, a pad's L1; not with the mouse, D-002) this tick.
    pub aiming: bool,
    pub auto_aim: bool,
}

impl Sim {
    /// One game tick with Bond at `camera` (eye): his object position on the collision floor below, aimed at chest
    /// height (stand-in for the game's player object and `Drone_View_Bones`).
    /// `trigger`: the fire button (action 9) held; `aiming`: aim mode (L1).
    pub fn step(&mut self, camera: [f32; 3], look: [f32; 3], trigger: bool, aiming: bool) {
        let floor = self.world.floor_below(camera, 4.0).unwrap_or(camera[1] - 1.6);
        self.run([camera[0], floor + 0.4, camera[2]], camera, look, look, trigger, aiming);
    }

    /// One game tick with Bond walking (`Player_Update`): his input, then the world with him as the player. In aim
    /// mode the mouse turn and look (`input.turn` radians, `input.look` units of π/2) first move the cursor within the
    /// aim box (±0.296 across, ±0.4 up) and turn the view only past its edge (pad aim mode; the mouse has none, D-002); the shot goes through the smoothed
    /// cursor (`Player_GetAimingPoint`).
    pub fn step_bond(&mut self, mut input: nf_game::player::Input, trigger: bool) {
        let aiming = input.aiming;
        if aiming {
            let want = [self.cursor[0] - input.turn / 0.76966, self.cursor[1] + input.look * std::f32::consts::FRAC_PI_2 / 0.57724];
            let clamped = [want[0].clamp(-0.296, 0.296), want[1].clamp(-0.4, 0.4)];
            input.turn = -(want[0] - clamped[0]) * 0.76966;
            input.look = (want[1] - clamped[1]) * 0.57724 / std::f32::consts::FRAC_PI_2;
            self.cursor = clamped;
            for k in 0..2 { self.cross[k] += (self.cursor[k] - self.cross[k]) * 0.5; }
        } else {
            // `Player_AutoAim` halves the offsets each tick outside aim mode.
            self.cursor = [self.cursor[0] * 0.5, self.cursor[1] * 0.5];
            self.cross = [self.cross[0] * 0.5, self.cross[1] * 0.5];
        }
        self.bond.tick(&self.world, &input);
        // The bob phases: `BL+0x8DC` += the forward speed (`BL+0x28`), `BL+0x8E8` += (π/2) / 60.
        self.bob[0] += self.bond.local[1];
        self.bob[1] += std::f32::consts::FRAC_PI_2 / 60.0;
        if let Some(dmg) = self.bond.fall_damage { self.drones.health.health = (self.drones.health.health - dmg).max(0.0); }
        let (head, look) = (self.bond.head(), self.bond.look());
        // The shot's direction through the crosshair: forward + 0.76966 cx · right + 0.57724 cy · up.
        let (yaw, p) = (self.bond.yaw, self.bond.pitch * std::f32::consts::FRAC_PI_2);
        let right = [-yaw.cos(), 0.0, yaw.sin()];
        let up = [-p.sin() * yaw.sin(), p.cos(), -p.sin() * yaw.cos()];
        let (tx, ty) = (0.76966 * self.cross[0], 0.57724 * self.cross[1]);
        let d = [0, 1, 2].map(|k| look[k] + tx * right[k] + ty * up[k]);
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let aim = d.map(|v| v / l);
        self.run(self.bond.pos, head, look, aim, trigger, aiming);
    }

    fn run(&mut self, pos: [f32; 3], camera: [f32; 3], look: [f32; 3], aim: [f32; 3], trigger: bool, aiming: bool) {
        // The drones aim at and look for Bond's chest (`Drone_View_Bones`; one point here, 0.45 under the eye).
        let player = nf_game::drone::Player { pos, target: [camera[0], camera[1] - 0.45, camera[2]], eye: camera, aim: look };
        let d = nf_game::drone::Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.level, sequences: &self.seqs, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models };
        // `Player_WeaponFiring` → `Player_WeaponInitBullet` from the eye along the view (*deviation*: no auto-aim and
        // no aim-point ray yet); the first-person fire script's sound 0x403 feeds the drones' hearing.
        // The gun's clip and spare rounds live in Bond's inventory (BLData `+0x1B8`, `+0x170`), which pickups fill.
        let (wi, t) = (self.gun.weapon as usize, self.weapons.ammo_type(self.gun.weapon as usize));
        if let (Some(&c), Some(&n)) = (self.drones.inventory.clip.get(wi), self.drones.inventory.carried.get(t)) { self.gun.clip = c.max(0) as u32; self.gun.spare = n.max(0) as u32; }
        if let Some(w) = self.weapons.records.get(self.gun.weapon as usize) {
            let scripts = &self.scripts;
            let params = nf_game::gun::GunParams::from_record(w, 0, |h| scripts.get(&h).map(|s| s.length as u32));
            // Auto-aim (on by default in the game's options) only outside aim mode; in aim mode the offset eases back.
            let dir = if self.auto_aim && !aiming { self.drones.auto_aim(&d, camera, look, self.gun.weapon) } else { self.drones.aim_target = None; aim };
            let was_reloading = self.gun.reloading > 0;
            let fired = self.gun.update(trigger, params);
            // The gun's scripts (`weapon+0xA0` idle, `+0xA4` reload at 1.35 from frame 6 with flag 0x1000, `+0xB0` fire,
            // `+0xB4` fire on the last round with flag 0x1000); after a script ends the last frame holds (flag 0x1000:
            // the idle is not re-added) — docs/systems/player.md.
            let rd = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
            let flags = rd(0x68);
            if fired {
                self.view_script = if flags & 0x1000 != 0 && self.gun.clip < 1 && rd(0xb4) != 0 { rd(0xb4) } else { rd(0xb0) };
                (self.view_time, self.view_speed) = (0.0, 1.0);
            } else if self.gun.reloading > 0 && !was_reloading {
                self.view_script = rd(0xa4);
                (self.view_time, self.view_speed) = (if flags & 0x1000 != 0 { 6.0 } else { 0.0 }, if rd(0xa8) == 0 && flags & 0x200 == 0 { 1.35 } else { 1.0 });
            } else if self.view_script == 0 {
                (self.view_script, self.view_time, self.view_speed) = (rd(0xa0), 0.0, 1.0);
            } else {
                let len = self.scripts.get(&self.view_script).map_or(0.0, |s| s.length as f32);
                self.view_time = (self.view_time + self.view_speed).min((len - 1.0).max(0.0));
                if self.view_script == rd(0xa0) && self.view_time >= len - 1.0 { self.view_time = 0.0; }
            }
            if fired {
                let sound = self.sfx.as_ref().map(|t| t.get(0x403)).unwrap_or(nf_formats::sfx::SfxHearing { id: 0x403, inner: 0.0, outer: 0.0, weight: 0.0, looped: false });
                self.drones.fire_player(&d, camera, dir, self.gun.weapon, sound);
            }
        }
        if let Some(c) = self.drones.inventory.clip.get_mut(wi) { *c = self.gun.clip as i16; }
        if let Some(n) = self.drones.inventory.carried.get_mut(t) { *n = self.gun.spare as i16; }
        self.aiming = aiming;
        self.drones.bond_aiming = aiming;
        self.drones.tick(&d, &player);
    }

    /// The crosshair offset (`BL+0x120 / +0x124`: −1…1 across half the screen) from the auto-aim offset angles: tan of the
    /// angle over the half-screen tangents of the game's projection [L] (0.76966 across, 0.57724 up); a larger yaw
    /// turns left (+X is screen-left in the object frame), so x flips.
    pub fn crosshair(&self) -> (f32, f32) {
        if self.aiming { return (self.cross[0], self.cross[1]); }
        (-self.drones.aim_offset[0].tan() / 0.76966, self.drones.aim_offset[1].tan() / 0.57724)
    }

    /// (sequence index, frame) the first-person gun's script shows.
    pub fn view_frame(&self, skeleton: u8) -> Option<(usize, u32)> {
        let script = self.scripts.get(&self.view_script)?;
        let t = (1.0 + self.view_time) as u16;
        let (a, _, q) = script.sequences().find(|(a, b, _)| *a <= t && t <= *b).or_else(|| script.sequences().last())?;
        let &(_, i) = self.seq_index.get(&q).filter(|(sk, _)| *sk == skeleton)?;
        Some((i, (t.saturating_sub(a) + 1) as u32))
    }

    /// The first-person gun's matrix (`Player_SetWeaponAnimObj` + `Player_PositionGun`): the view axes (left, up,
    /// forward with pitch), at the eye + the weapon offset `+0xE0` (P2K (0.0283, 0.0755, −0.9026)) + the bob. With the
    /// crosshair centred the pivot terms cancel. *Deviation*: the turn toward an off-centre crosshair is left out.
    pub fn gun_matrix(&self) -> [[f32; 4]; 4] {
        let w = &self.weapons.records[self.gun.weapon as usize];
        let f = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let (p, t) = (self.bob[0], self.bob[1]);
        let l = [f(0xe0) + 0.01 * p.sin() + 0.001 * t.sin(), f(0xe4) + 0.01 * p.cos().abs() + 0.001 * (1.7 * t).cos(), f(0xe8) + 0.02 * (0.84329 * p).sin()];
        let (y, pt) = (self.bond.yaw, self.bond.pitch * std::f32::consts::FRAC_PI_2);
        let left = [y.cos(), 0.0, -y.sin()];
        let up = [-pt.sin() * y.sin(), pt.cos(), -pt.sin() * y.cos()];
        let fwd = self.bond.look();
        let h = self.bond.head();
        let pos = [0, 1, 2].map(|k| h[k] + l[0] * left[k] + l[1] * up[k] + l[2] * fwd[k]);
        [[left[0], left[1], left[2], 0.0], [up[0], up[1], up[2], 0.0], [fwd[0], fwd[1], fwd[2], 0.0], [pos[0], pos[1], pos[2], 1.0]]
    }

    /// (sequence index, frame) the drone's playing script shows.
    fn frame_of(&self, drone: usize, skeleton: u8) -> Option<(usize, u32)> {
        let dr = &self.drones.list[drone];
        let script = self.scripts.get(&dr.anim.script?)?;
        let t = (1 + dr.anim.time) as u16;
        let (a, _, q) = script.sequences().find(|(a, b, _)| *a <= t && t <= *b).or_else(|| script.sequences().next())?;
        let &(sk, i) = self.seq_index.get(&q).filter(|(sk, _)| *sk == skeleton)?;
        let _ = sk;
        Some((i, (t.saturating_sub(a) + 1) as u32))
    }
}

/// A rigid mesh hung on a character's datum (`AnimObjectDraw`): world = `QuatTransToMat(q, t)` · bone world.
pub struct Attachment {
    pub character: u32,
    pub bone: usize,
    pub local: Mat4,
    /// `Some(rgb)`: the muzzle flash of `DroneWeap_FireWeapon` (mesh `weapon_data+0x4C`, light colour +0x56/+0x55/+0x54),
    /// drawn only on firing frames; `None`: the weapon itself.
    pub flash: Option<[u8; 3]>,
    /// `Some(slot)`: a bullet tracer (`GUN_BULLET_TRAIL` 0x02000123) showing bullet `slot` of the simulation.
    pub tracer: Option<usize>,
    /// The character's gun once dropped (`DroneWeap_DropWeapon`): drawn at the pickup's matrix while it lies there.
    pub dropped: bool,
}

/// Viewer switches that change what the game state would be.
#[derive(Clone, Copy, Default)]
pub struct ViewOpts {
    /// Also draw drones a spawner holds back at level start (docs/systems/drone-animation.md "Spawners").
    pub reinforcements: bool,
    /// Make every armed drone fire (muzzle flashes); the game's AI decides this, nf-view has none yet.
    pub firing: bool,
}

/// Bind-pose data of one character vertex (same order as the Character-layer vertices).
#[derive(Clone, Copy)]
pub struct CharVertex {
    pub character: u32,
    pub pos: [f32; 3],
    pub weight: f32,
    pub slots: [u16; 2],
    /// Rigid vertex of an attached mesh (index into `Scene::attachments`) instead of a skinned one.
    pub attachment: Option<u32>,
}

pub struct Scene {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub materials: Vec<Material>,
    pub textures: Vec<TextureImage>,
    /// The first-person gun (`Player_SetWeaponAnim`: `AnimObjectNew(G, weapon+0xDC)`): its character index.
    pub view_model: Option<usize>,
    /// The HUD sheet (texture 0x03000026: the crosshairs, `HUD_UpdateCrossHair`), from any map of the archive.
    pub hud_texture: Option<TextureImage>,
    pub bounds: ([f32; 3], [f32; 3]),
    /// Camera start: the first placement whose object is named "Player1" (translation, yaw), if any.
    pub start: Option<([f32; 3], f32)>,
    pub stats: String,
    pub characters: Vec<Character>,
    /// Index of the first Character-layer vertex; `char_verts` describe that tail of `vertices`.
    pub char_vertex_start: usize,
    pub char_verts: Vec<CharVertex>,
    /// Sequences of the level's animation archive (`07F000NN.bin`), by skeleton id.
    pub sequences: HashMap<u8, Vec<Sequence>>,
    /// Weapons and props hung on character datums; their vertices are Character-layer vertices with `attachment`.
    pub attachments: Vec<Attachment>,
    /// The guard AI, when the drone context was available.
    pub sim: Option<Sim>,
}

type Groups = BTreeMap<(Layer, Option<(usize, u32)>), (Vec<Vertex>, Vec<u32>, Vec<CharVertex>)>;

fn add_instance(level: &Level, inst: &Instance, layer: Layer, character: Option<(u32, Option<u32>)>, groups: &mut Groups, bounds: Option<&mut ([f32; 3], [f32; 3])>) {
    let obj = &level.maps[inst.map].objects[inst.object];
    let mut m = inst.matrix;
    if layer == Layer::SkyCamera { m[3] = [0.0, 0.0, 0.0, 1.0]; }
    let mut bounds = bounds;
    for b in obj.gfx.leaves.iter().flat_map(|l| &l.batches) {
        let key = b.texture.filter(|&t| (t as usize) < level.maps[inst.map].textures.len()).map(|t| (inst.map, t));
        let (vs, is, cv) = groups.entry((layer, key)).or_default();
        let base = vs.len() as u32;
        for v in &b.verts {
            let p = transform(&m, v.pos);
            if let Some((lo, hi)) = bounds.as_deref_mut() {
                if p.iter().all(|c| c.is_finite()) {
                    for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); }
                }
            }
            let uv = if v.uv.iter().all(|c| c.is_finite()) { v.uv } else { [0.0, 0.0] };
            vs.push(Vertex { pos: p, uv, rgba: v.rgba });
            if let Some((c, attachment)) = character { cv.push(CharVertex { character: c, pos: v.pos, weight: v.weight, slots: v.slots, attachment }); }
        }
        for t in b.triangles() { is.extend(t.map(|k| base + k as u32)); }
    }
}

/// Height of the highest World triangle straight below `p` (vertical ray from 0.5 above it), within 3 units.
/// In 07000005 the drone placements sit 1.0845 above this floor (10 of 16; the rest stand on steps or slopes).
fn floor_below(vertices: &[Vertex], indices: &[u32], materials: &[Material], p: [f32; 3]) -> Option<f32> {
    let [px, py, pz] = p;
    let mut best: Option<f32> = None;
    for m in materials.iter().filter(|m| m.layer == Layer::World) {
        for t in indices[m.first_index as usize..(m.first_index + m.index_count) as usize].chunks(3) {
            let [a, b, d] = [0, 1, 2].map(|k| vertices[t[k] as usize].pos);
            if a[1].min(b[1]).min(d[1]) > py + 0.5 || a[1].max(b[1]).max(d[1]) < py - 3.0 { continue; }
            // Barycentric test in the xz plane, then the plane height at (px, pz).
            let det = (b[2] - d[2]) * (a[0] - d[0]) + (d[0] - b[0]) * (a[2] - d[2]);
            if det.abs() < 1e-9 { continue; }
            let l1 = ((b[2] - d[2]) * (px - d[0]) + (d[0] - b[0]) * (pz - d[2])) / det;
            let l2 = ((d[2] - a[2]) * (px - d[0]) + (a[0] - d[0]) * (pz - d[2])) / det;
            let l3 = 1.0 - l1 - l2;
            if l1 < 0.0 || l2 < 0.0 || l3 < 0.0 { continue; }
            let y = l1 * a[1] + l2 * b[1] + l3 * d[1];
            if y <= py + 0.5 && y >= py - 3.0 && best.is_none_or(|bb| y > bb) { best = Some(y); }
        }
    }
    best
}

/// `DroneWeap_FireWeapon` (0x178f68): the flash object sits at the weapon datum's world position; its rotation is
/// the spherical angles of the datum's −x axis (`Vec_Cartesian_2_Spherical`: atan2(y, |xz|), atan2(x, z), length)
/// plus `Rand_FRand(2π)` on x, `Rand_FRandHalf(0.2) + π/2` on y and `Rand_FRandHalf(0.2)` on z; matrix as for
/// placements (`RotMatrix`). `seed` stands in for the game's random state.
fn muzzle_flash_matrix(datum: &Mat4, seed: u32) -> Mat4 {
    let mut s = seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let mut rand = || { s ^= s << 13; s ^= s >> 17; s ^= s << 5; (s >> 8) as f32 / (1u32 << 24) as f32 };
    let d = [-datum[0][0], -datum[0][1], -datum[0][2]];
    let horiz = (d[0] * d[0] + d[2] * d[2]).sqrt();
    let sph = [d[1].atan2(horiz), d[0].atan2(d[2]), (horiz * horiz + d[1] * d[1]).sqrt()];
    let rot = [
        sph[0] + rand() * std::f32::consts::TAU,
        sph[1] + (rand() - 0.5) * 0.2 + std::f32::consts::FRAC_PI_2,
        sph[2] + (rand() - 0.5) * 0.2,
    ];
    nf_formats::placement::object_matrix(rot, [datum[3][0], datum[3][1], datum[3][2]])
}

/// Bullet tracer matrix: local +Z along the bullet (`Mat_Align2Dir`), scaled by its speed (one tick of travel).
fn tracer_matrix(b: &nf_game::bullet::Bullet) -> Mat4 {
    let d = b.dir;
    let yaw = d[0].atan2(d[2]);
    let pitch = -d[1].atan2((d[0] * d[0] + d[2] * d[2]).sqrt());
    let mut m = nf_formats::placement::object_matrix([pitch, yaw, 0.0], b.pos);
    for k in 0..3 { m[2][k] *= b.speed; }
    m
}

/// Tracer slots (bullets drawn at once).
const TRACER_SLOTS: usize = 32;

/// The drone's height state after `DroneAnim_PreTransCallback` (0x13b490): `Drone+0xA0` = hip height − 0.02, at least
/// 0.2 (it starts at 1.0 and keeps its value while the hip height is 0).
fn drone_height(hip: f32) -> f32 {
    if hip == 0.0 { 1.0 } else { (hip - 0.02).max(0.2) }
}

impl Scene {
    /// `skeletons`: every skeleton on the disc by id; `sequences`: the level's animation archive, grouped by skeleton.
    pub fn build(level: &Level, cels_only: bool, skeletons: &HashMap<u8, Skeleton>, sequences: HashMap<u8, Vec<Sequence>>, drones: Option<&DroneContext>) -> Scene {
        let by_hash: HashMap<u32, (u8, usize)> = sequences.iter()
            .flat_map(|(&sk, v)| v.iter().enumerate().map(move |(i, s)| (s.hashcode, (sk, i)))).collect();
        let (mut instances, missing) = level.instances(cels_only);
        // Sky objects are drawn by the sky pass only (with --all they would also appear as dynamic entities).
        instances.retain(|i| i.placement.flags & 0xff != 0x2a);
        let (char_instances, chars_missing) = level.character_instances();
        let sky = level.sky_instances();
        let mut groups: Groups = BTreeMap::new();
        let mut bounds = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        // The spawn comes from the placement records even when entities are not drawn (Player1 is not a 0x8000 cel).
        let start = level.instances(false).0.iter()
            .find(|i| level.maps[i.map].objects[i.object].name.trim() == "Player1")
            .map(|i| (i.placement.translation, i.placement.rotation[1]));
        let mut hidden = 0;
        for inst in &instances {
            let obj = &level.maps[inst.map].objects[inst.object];
            if inst.placement.creates_cel() && obj.skipped_by_cel_pass() { hidden += 1; continue; }
            add_instance(level, inst, Layer::World, None, &mut groups, Some(&mut bounds));
        }
        let mut characters = Vec::new();
        let (mut attachments, mut armed, mut weapons_missing) = (Vec::new(), std::collections::HashSet::new(), 0);
        // Spawner roles of the main map's drone placements, keyed by translation (docs/systems/drone-animation.md).
        let roles: HashMap<[u32; 3], drone::SpawnRole> = match level.main_map() {
            Some(m) => {
                let ps = &level.maps[m].placements;
                let list: Vec<(u32, Vec<(u32, u32)>)> = ps.iter().map(|p| (p.flags & 0xff, p.params.clone())).collect();
                let drones_in_order = ps.iter().filter(|p| p.flags & 0xff == 0x0f);
                drones_in_order.zip(drone::spawn_roles(&list)).map(|(p, r)| (p.translation.map(f32::to_bits), r)).collect()
            }
            None => HashMap::new(),
        };
        for inst in &char_instances {
            let obj = &level.maps[inst.map].objects[inst.object];
            let skin_hash = inst.placement.params.iter().find(|(i, _)| *i == 0).map(|&(_, v)| v).unwrap_or(0);
            let Some(skin) = level.skins.get(&skin_hash) else { continue };
            let Some(skeleton) = skeletons.get(&skin.skeleton) else { continue };
            if !obj.gfx.is_skinned() || skin.inverse_bind.is_empty() {
                add_instance(level, inst, Layer::World, None, &mut groups, Some(&mut bounds));
                continue;
            }
            add_instance(level, inst, Layer::Character, Some((characters.len() as u32, None)), &mut groups, Some(&mut bounds));
            let start = drones.and_then(|d| {
                let param = |k: u32| inst.placement.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
                let hash = drone::start_script(&d.tables, param, d.level_id).script()?;
                let script = d.scripts.get(&hash)?.clone();
                let sequences = script.sequences()
                    .filter_map(|(a, b, q)| by_hash.get(&q).filter(|(sk, _)| *sk == skin.skeleton).map(|&(_, i)| (a, b, i))).collect();
                Some(StartAnim { script, sequences })
            });
            let role = roles.get(&inst.placement.translation.map(f32::to_bits)).copied().unwrap_or(drone::SpawnRole::Placed);
            characters.push(Character { matrix: inst.matrix, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones: obj.gfx.slot_bones.clone(), start, role, floor: None });
            // The held weapon (`AnimObjectDraw`): weapon_data[w] +0x80 mesh on the datum whose id is +0x50 (0xFF → 0),
            // once per drone (a skin may have several main meshes).
            let key = (inst.placement.translation.map(f32::to_bits), skin_hash);
            if !armed.insert(key) { continue; }
            let Some(wd) = drones.and_then(|d| d.weapons.as_ref()) else { continue };
            let param9 = inst.placement.params.iter().find(|(i, _)| *i == 9).map(|&(_, v)| v).unwrap_or(0);
            let weapon = drone::weapon_id(skin_hash, param9);
            let Some((mesh, datum_id)) = (weapon < 0x73).then(|| wd.third_person(weapon as u8)).flatten() else { continue };
            let datum_id = if datum_id == 0xff { 0 } else { datum_id as u32 };
            let Some(dt) = skin.datums.iter().find(|d| d.id == datum_id && d.bone >= 0) else { weapons_missing += 1; continue };
            let Some(&(map, object)) = level.by_hash.get(&mesh) else { weapons_missing += 1; continue };
            let winst = Instance { map, object, matrix: anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]), placement: inst.placement.clone() };
            let character = characters.len() as u32 - 1;
            add_instance(level, &winst, Layer::Character, Some((character, Some(attachments.len() as u32))), &mut groups, None);
            let local = anim::quat_trans_to_mat(dt.q, dt.t);
            attachments.push(Attachment { character, bone: dt.bone as usize, local, flash: None, tracer: None, dropped: false });
            // The same gun as a pickup once the drone drops it.
            add_instance(level, &winst, Layer::Character, Some((character, Some(attachments.len() as u32))), &mut groups, None);
            attachments.push(Attachment { character, bone: 0, local, flash: None, tracer: None, dropped: true });
            // Its muzzle flash: `DroneWeap_FireWeapon` asks `AnimGetBoneWorldTrans(obj, datum id, 1, …)`, which looks up
            // the datum whose id is id + 1 — the muzzle (07000005: 8 → 9 at x −0.77, 16 → 17 at x −1.18).
            if let Some((fmesh, rgb)) = wd.muzzle_flash(weapon as u8) {
                let muzzle = skin.datums.iter().find(|d| d.id == datum_id + 1 && d.bone >= 0);
                if let (Some(&(fmap, fobject)), Some(md)) = (level.by_hash.get(&fmesh), muzzle) {
                    let finst = Instance { map: fmap, object: fobject, ..winst };
                    add_instance(level, &finst, Layer::Effect, Some((character, Some(attachments.len() as u32))), &mut groups, None);
                    attachments.push(Attachment { character, bone: md.bone as usize, local: anim::quat_trans_to_mat(md.q, md.t), flash: Some(rgb), tracer: None, dropped: false });
                }
            }
        }
        // The first-person P2K: skin `weapon_data[2]+0xDC` (0x05000088) drawn with its main mesh — a sleeve, replaced by
        // `AnimSleeveGetEntity(BL+0x965 = 0)`: the first loaded of `SleeveEnts` [L] — and its rigid parts on their bones.
        let mut view_model = None;
        if let Some(wd) = drones.and_then(|d| d.weapons.as_ref()) {
            let fp = wd.first_person(2);
            const SLEEVES: [u32; 8] = [0x0200_09d0, 0x0200_0900, 0x0200_01f7, 0x0200_0929, 0x0200_09e6, 0x0200_09ff, 0x0200_0a14, 0x0200_0a13];
            if let (Some(skin), Some(&sleeve)) = (level.skins.get(&fp), SLEEVES.iter().find(|h| level.by_hash.contains_key(h))) {
                if let Some(skeleton) = skeletons.get(&skin.skeleton) {
                    let ci = characters.len() as u32;
                    let ident = anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]);
                    let placement = level.maps[level.main_map().unwrap_or(0)].placements.first().cloned().unwrap_or_default();
                    let (map, object) = level.by_hash[&sleeve];
                    let inst = Instance { map, object, matrix: ident, placement: placement.clone() };
                    add_instance(level, &inst, Layer::Character, Some((ci, None)), &mut groups, None);
                    for &(bone, part) in &skin.extra {
                        let Some(&(map, object)) = part.and_then(|h| level.by_hash.get(&h)) else { continue };
                        let pinst = Instance { map, object, matrix: ident, placement: placement.clone() };
                        add_instance(level, &pinst, Layer::Character, Some((ci, Some(attachments.len() as u32))), &mut groups, None);
                        attachments.push(Attachment { character: ci, bone: bone as usize, local: ident, flash: None, tracer: None, dropped: false });
                    }
                    characters.push(Character { matrix: ident, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones: level.maps[map].objects[object].gfx.slot_bones.clone(), start: None, role: drone::SpawnRole::Placed, floor: None });
                    view_model = Some(ci as usize);
                }
            }
        }
        // Bullet tracers: drone bullets draw `GUN_BULLET_TRAIL` (Bullet_update state 0), stretched along their
        // direction by the speed (Control_GetObjMatrix scale 1, 1, speed); a pool of slots, hidden when unused.
        if let (Some(_), Some(&(tmap, tobj))) = (drones, level.by_hash.get(&0x0200_0123)) {
            for slot in 0..TRACER_SLOTS {
                let tinst = Instance { map: tmap, object: tobj, matrix: anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]), placement: level.maps[level.main_map().unwrap_or(0)].placements.first().cloned().unwrap_or_default() };
                add_instance(level, &tinst, Layer::Effect, Some((0, Some(attachments.len() as u32))), &mut groups, None);
                attachments.push(Attachment { character: 0, bone: 0, local: tinst.matrix, flash: None, tracer: Some(slot), dropped: false });
            }
        }
        for (inst, follows_camera) in &sky {
            add_instance(level, inst, if *follows_camera { Layer::SkyCamera } else { Layer::SkyFixed }, None, &mut groups, None);
        }
        let (mut vertices, mut indices, mut materials, mut char_verts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut char_vertex_start = None;
        for ((layer, key), (vs, is, cv)) in groups {
            if is.is_empty() { continue; }
            if matches!(layer, Layer::Character | Layer::Effect) && char_vertex_start.is_none() { char_vertex_start = Some(vertices.len()); }
            let base = vertices.len() as u32;
            materials.push(Material { layer, key, first_index: indices.len() as u32, index_count: is.len() as u32 });
            indices.extend(is.iter().map(|i| i + base));
            vertices.extend(vs);
            char_verts.extend(cv);
        }
        let keys: BTreeSet<(usize, u32)> = materials.iter().filter_map(|m| m.key).collect();
        let textures = keys.into_iter().map(|key| {
            let t = &level.maps[key.0].textures[key.1 as usize];
            let pal = t.palette_rgba8();
            TextureImage { key, width: t.width as u32, height: t.height as u32, rgba: t.frame(0).iter().flat_map(|&i| pal[i as usize]).collect() }
        }).collect();
        let hud_texture = level.maps.iter().enumerate().find_map(|(mi, m)| m.textures.iter().find(|t| t.hashcode == 0x0300_0026).map(|t| {
            let pal = t.palette_rgba8();
            TextureImage { key: (mi, t.index as u32), width: t.width as u32, height: t.height as u32, rgba: t.frame(0).iter().flat_map(|&i| pal[i as usize]).collect() }
        }));
        let n_seq: usize = sequences.values().map(Vec::len).sum();
        let stats = format!("{} world instances ({hidden} skipped by the cel pass: flag 0x10), {} animated character meshes, {chars_missing} drones with a skin from another archive, {} sky objects, {missing} unresolved, {} triangles, {} materials, {n_seq} sequences",
            instances.len(), characters.len(), sky.len(), indices.len() / 3, materials.len());
        let char_vertex_start = char_vertex_start.unwrap_or(vertices.len());
        for c in characters.iter_mut() {
            c.floor = floor_below(&vertices, &indices, &materials, [c.matrix[3][0], c.matrix[3][1], c.matrix[3][2]]);
        }
        if std::env::var_os("NF_FLOOR_PROBE").is_some() {
            for (n, c) in characters.iter().enumerate() {
                eprintln!("floor probe: character {n} skin {:08x} at {:?}: placement − floor = {:?}", c.skin.hashcode, &c.matrix[3][..3], c.floor.map(|f| c.matrix[3][1] - f));
            }
        }
        let started = characters.iter().filter(|c| c.start.as_ref().is_some_and(|s| !s.sequences.is_empty())).count();
        let flashes = attachments.iter().filter(|a| a.flash.is_some()).count();
        let tracers = attachments.iter().filter(|a| a.tracer.is_some()).count();
        let held_back = characters.iter().filter(|c| matches!(c.role, drone::SpawnRole::Reinforcement { .. })).count();
        let stats = format!("{stats}; {started}/{} characters with their start animation; {} weapons held ({weapons_missing} without mesh or datum), {flashes} with a muzzle flash; {held_back} character meshes held back by spawners (R shows them)", characters.len(), attachments.len() - flashes - tracers);
        // The guard AI: one drone per main-map drone placement present at level start; characters map to it by
        // placement translation.
        let seqs: HashMap<u32, Sequence> = sequences.values().flatten().map(|s| (s.hashcode, s.clone())).collect();
        let sim = drones.and_then(|d| Some((d, d.props.clone()?, d.weapons.clone()?))).and_then(|(d, props, weapons)| {
            let m = level.main_map()?;
            let world = nf_game::world::CollisionWorld::build(level, &level.instances(true).0);
            let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap_or(&Default::default()), &world, d.level_id);
            let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
            let hitboxes = d.hitboxes.clone()?;
            let volumes = nf_game::aivolume::build(level, d.level_id);
            let models = nf_game::pickup::models(level);
            let data = nf_game::drone::Data { tables: &d.tables, props: &props, scripts: &d.scripts, weapons: &weapons, world: &world, level: d.level_id, sequences: &seqs, skins: &level.skins, skeletons, nav: &nav, covers: &covers, tuning: &d.tuning, hitboxes: &hitboxes, volumes: &volumes, sfx: d.sfx.as_ref(), models: &models };
            let mut rng = nf_game::drone::Rng::new(0x1234_5678);
            // Every drone placement; spawner templates stay absent until a spawner brings them in.
            let list = nf_game::drone::level_drones(&data, level, &mut rng);
            let by_pos: HashMap<[u32; 3], usize> = list.iter().enumerate()
                .map(|(i, dr)| (level.maps[m].placements[dr.placement].translation.map(f32::to_bits), i)).collect();
            let char_drone = characters.iter().map(|c| by_pos.get(&[c.matrix[3][0], c.matrix[3][1], c.matrix[3][2]].map(f32::to_bits)).copied()).collect();
            let seq_index = by_hash.clone();
            // Bond at the "Player1" placement, on the floor below (`Player_StandAtNewPosition`: floor + 1.1).
            let (p0, yaw0) = start.unwrap_or(([0.0; 3], 0.0));
            let bond = nf_game::player::Bond::stand_at(&world, p0, yaw0);
            let mut drones = nf_game::drone::Drones::new(list, rng);
            drones.attach_level(level);
            Some(Sim { drones, tables: d.tables.clone(), props, scripts: d.scripts.clone(), weapons, world, level: d.level_id, char_drone, seq_index, seqs, skins: level.skins.clone(), skeletons: skeletons.clone(), nav, covers, tuning: d.tuning.clone(), hitboxes, sfx: d.sfx.clone(), volumes, models, gun: nf_game::gun::Gun::new(2, 7, 41), bond, cursor: [0.0; 2], cross: [0.0; 2], view_script: 0, view_time: 0.0, view_speed: 1.0, bob: [0.0; 2], aiming: false, auto_aim: false })
        });
        Scene { vertices, indices, materials, textures, hud_texture, view_model, bounds, start, stats, characters, char_vertex_start, char_verts, sequences, attachments, sim }
    }

    /// Sequence index and frame a character shows after `ticks` 60 Hz updates: its start script (`choice = None`) or the
    /// `choice`-th sequence of its skeleton (modulo the list), looped over frames 1 … frames − 1.
    fn frame_of(&self, c: &Character, choice: Option<usize>, ticks: f32) -> Option<(usize, u32)> {
        let seqs = self.sequences.get(&c.skin.skeleton).filter(|v| !v.is_empty())?;
        match choice {
            None => {
                let start = c.start.as_ref()?;
                // A finished non-looping start script is restarted, as the drone AI calls the same idle again
                // (`DroneAnim_SetStandIdleAnim`); AnimScriptEnd's wrap-around otherwise.
                let looped = Script { looping: true, ..start.script.clone() };
                let t = looped.time_at(ticks);
                let &(a, b, i) = start.sequences.iter().find(|(a, b, _)| (*a as f32) <= t && t <= *b as f32).or(start.sequences.first())?;
                let frame = ((t - a as f32) as u32 + 1).min((b - a) as u32 + 1).clamp(1, seqs[i].frames.max(1) as u32);
                Some((i, frame))
            }
            Some(k) => {
                let i = k % seqs.len();
                let span = (seqs[i].frames as f32 - 1.0).max(1.0);
                Some((i, 1 + (ticks % span) as u32))
            }
        }
    }

    /// Skins every character for `choice` at `ticks` and returns the new Character-layer vertex positions (world).
    ///
    /// Root handling as the game: `AnimSeqTick` (0x1c28d0) zeroes the root bone's translation after taking its
    /// per-frame change as root motion, so the root sits at the object position; the drone's height comes from
    /// `Drone+0xA0` (hip height − 0.02), whose collision sphere rests its lowest point `0xA0` below the object on the
    /// floor (`NDrone2_Collision` 0x140e90). nf-view has no collision or AI yet, so drones stay at their placement x/z
    /// (horizontal root motion is dropped) and stand on the World floor below them; without one found they keep the
    /// pre-collision height (placement + 0xA0 − 1.0).
    pub fn animate(&self, choice: Option<usize>, ticks: f32, opts: ViewOpts) -> Vec<[f32; 3]> {
        let mut cache: HashMap<(u32, usize, u32), (Vec<Mat4>, Vec<Mat4>, f32)> = HashMap::new();
        // With the AI running (and no sequence browsing), each guard shows its drone's animation, yaw and position.
        let drone_of = |ci: usize| choice.is_none().then(|| self.sim.as_ref().and_then(|s| s.char_drone[ci].map(|di| (s, di)))).flatten();
        let posed: Vec<Option<(Vec<Mat4>, Vec<Mat4>, Mat4)>> = self.characters.iter().enumerate().map(|(ci, c)| {
            let view = (Some(ci) == self.view_model).then_some(()).and(self.sim.as_ref());
            let (i, frame) = match (view, drone_of(ci)) {
                (Some(s), _) => s.view_frame(c.skin.skeleton)?,
                (None, Some((s, di))) => s.frame_of(di, c.skin.skeleton)?,
                (None, None) => self.frame_of(c, choice, ticks)?,
            };
            let seq = &self.sequences[&c.skin.skeleton][i];
            let (mut mats, mut world, hip) = cache.entry((c.skin.hashcode, i, frame)).or_insert_with(|| {
                let mut local = anim::local_pose(seq, &c.skeleton, &c.skin, frame);
                let hip = local[0].0[1];
                local[0].0 = [0.0; 3];
                let world = anim::world_pose(&local, &c.skin);
                (anim::skin_matrices(&world, &c.skin), world, hip)
            }).clone();
            // Upper-body aim (`AnimObjectAimAt`, bone 2): turn bone 2 and everything below it by the drone's aim yaw
            // about the model's up axis through bone 2 (*approximation*: the game rotates about the spine axis and
            // also pitches; G-026).
            if let Some((s, di)) = drone_of(ci) {
                let a = s.drones.list[di].aim_yaw;
                if a.abs() > 1e-4 && world.len() > 2 {
                    let pivot = world[2][3];
                    let (sn, cs) = (a.sin(), a.cos());
                    // Row-vector rotation about +Y by a: (x, z) → (x cos + z sin, −x sin + z cos).
                    let rot: Mat4 = [[cs, 0.0, -sn, 0.0], [0.0, 1.0, 0.0, 0.0], [sn, 0.0, cs, 0.0], [0.0, 0.0, 0.0, 1.0]];
                    let below = |b: usize| { let mut p = Some(b); while let Some(x) = p { if x == 2 { return true; } p = c.skin.parent(x); } false };
                    for b in 0..world.len() {
                        if !below(b) { continue; }
                        let mut w = world[b];
                        for k in 0..3 { w[3][k] -= pivot[k]; }
                        let mut r = anim::mul(&w, &rot);
                        for k in 0..3 { r[3][k] += pivot[k]; }
                        world[b] = r;
                    }
                    mats = anim::skin_matrices(&world, &c.skin);
                }
            }
            if let Some(s) = view { return Some((mats, world, s.gun_matrix())); }
            let mut m = match drone_of(ci) {
                Some((s, di)) => { let dr = &s.drones.list[di]; nf_formats::placement::object_matrix([0.0, dr.yaw, 0.0], [dr.pos[0], 0.0, dr.pos[2]]) }
                None => c.matrix,
            };
            let a0 = drone_height(hip);
            m[3][1] = match (drone_of(ci), c.floor) {
                // The simulation keeps the drone on the collision floor (pos.y = floor + 1.0 as Drone+0xA0 starts).
                (Some((s, di)), _) => s.drones.list[di].draw_y(hip),
                (None, Some(f)) => f + a0,
                (None, None) => c.matrix[3][1] + a0 - 1.0,
            };
            Some((mats, world, m))
        }).collect();
        // Held back: spawner reinforcements and drones disabled while they wait for a switch channel (object flags
        // 0x30, `NDrone2_Enable(0)`); R shows them.
        let hidden = |c: &Character, ci: usize| !opts.reinforcements && match drone_of(ci) {
            Some((s, di)) => !s.drones.list[di].present || !s.drones.list[di].enabled,
            None => matches!(c.role, drone::SpawnRole::Reinforcement { .. }),
        };
        // A matrix that collapses a mesh to one point: its triangles become degenerate and draw nothing.
        let collapse = |at: &Mat4| -> Mat4 { [[0.0; 4], [0.0; 4], [0.0; 4], [at[3][0], at[3][1], at[3][2], 1.0]] };
        // Attached meshes: datum local · bone world · object (bind world when the character has no animation).
        let attach: Vec<Mat4> = self.attachments.iter().enumerate().map(|(k, a)| {
            if let Some(slot) = a.tracer {
                let b = self.sim.as_ref().and_then(|s| s.drones.bullets.get(slot)).filter(|b| b.state == 1);
                return match b { Some(b) => tracer_matrix(b), None => [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]] };
            }
            let far = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
            if a.dropped {
                let p = drone_of(a.character as usize).and_then(|(s, di)| s.drones.pickups.iter().find(|p| p.from == Some(di)));
                return p.map_or(far, |p| p.m);
            }
            let c = &self.characters[a.character as usize];
            let datum = match &posed[a.character as usize] {
                Some((_, world, m)) => anim::mul(&anim::mul(&a.local, &world[a.bone]), m),
                None => {
                    let ib = c.skin.inverse_bind.get(a.bone).map(|b| b.0).unwrap_or([0.0; 3]);
                    let bind = anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], ib.map(|v| -v));
                    anim::mul(&anim::mul(&a.local, &bind), &c.matrix)
                }
            };
            if hidden(c, a.character as usize) { return collapse(&datum); }
            // Dropped: the drone holds nothing (`anim+0x62` = 0).
            if drone_of(a.character as usize).is_some_and(|(s, di)| s.drones.list[di].weapon == 0) { return collapse(&datum); }
            if a.flash.is_none() { return datum; }
            // AI-driven: the flash object lives 2 ticks after each shot (`DroneWeap_FireWeapon`).
            if let Some((s, di)) = drone_of(a.character as usize) {
                let now = s.drones.frame;
                return match s.drones.list[di].shots.iter().rev().find(|&&f| now.saturating_sub(f) < 2) {
                    Some(&f) => muzzle_flash_matrix(&datum, f ^ (k as u32).wrapping_mul(0x9e37_79b9)),
                    None => collapse(&datum),
                };
            }
            // Firing demo: a shot every 8 ticks, the flash object living 2 ticks (`Drone+0x30` life = 2), staggered
            // per drone. The rate is nf-view's, not the weapon's (G-025).
            let tick = ticks as u32 + 3 * a.character;
            if !opts.firing || tick % 8 >= 2 { return collapse(&datum); }
            muzzle_flash_matrix(&datum, (tick / 8) ^ (k as u32).wrapping_mul(0x9e37_79b9))
        }).collect();
        self.char_verts.iter().map(|v| {
            if let Some(a) = v.attachment { return transform(&attach[a as usize], v.pos); }
            let c = &self.characters[v.character as usize];
            if hidden(c, v.character as usize) { return [c.matrix[3][0], c.matrix[3][1], c.matrix[3][2]]; }
            match &posed[v.character as usize] {
                Some((mats, _, m)) => transform(m, anim::skin_vertex(v.pos, v.weight, v.slots, &c.slot_bones, mats)),
                None => transform(&c.matrix, v.pos),
            }
        }).collect()
    }

    /// Window-title text: the start scripts in use, or the chosen sequence per skeleton.
    pub fn sequence_label(&self, choice: Option<usize>) -> String {
        match choice {
            None => {
                let mut names: Vec<String> = self.characters.iter().filter_map(|c| c.start.as_ref())
                    .map(|s| format!("{:08x}", s.script.hashcode)).collect();
                names.sort();
                names.dedup();
                format!("game start animations (scripts {}); [ ] = browse sequences", names.join(", "))
            }
            Some(choice) => {
                let mut used: Vec<u8> = self.characters.iter().map(|c| c.skin.skeleton).collect();
                used.sort();
                used.dedup();
                used.iter().filter_map(|s| self.sequences.get(s).filter(|v| !v.is_empty()).map(|v| {
                    let q = &v[choice % v.len()];
                    format!("skel {s}: {:08x} ({} frames, {}/{})", q.hashcode, q.frames, choice % v.len() + 1, v.len())
                })).collect::<Vec<_>>().join("; ")
            }
        }
    }
}
