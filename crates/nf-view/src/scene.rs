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

/// How a material is drawn (docs/systems/render.md): the pass (0 sky before the world, fogged; 1 world list 0;
/// 2 characters; 3 world list 1 — glist flag 0x1, alpha list, faded; 4 sky after the fog pass; 5 effects), the GS
/// blend from ALPHA_1 (0 normal (Cs − Cd)·As + Cd, 1 additive Cs·As + Cd, 2 subtractive Cd − Cs·As), depth write
/// (ZBUF_1 ZMSK clear), the alpha test from TEST_1 (0 none, 1 As ≥ 1, 2 As > 0x7F) and the fog (0 none, 1 depth fog,
/// 2 alpha fade with distance).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Draw { pub pass: u8, pub blend: u8, pub zwrite: bool, pub atest: u8, pub fog: u8 }

impl Draw {
    /// From a leaf's GS registers (ALPHA_1, TEST_1, ZBUF_1 as last written).
    fn from_gs(pass: u8, fog: u8, alpha: u64, test: u64, zbuf: u64) -> Draw {
        let blend = match alpha & 0xff { 0x48 => 1, 0x42 => 2, _ => 0 };
        let atest = if test & 1 == 0 { 0 } else {
            let (atst, aref) = ((test >> 1) & 7, (test >> 4) & 0xff);
            match atst { 5 if aref <= 1 => 1, 6 | 5 => 2, 0 => 0, _ => 1 }
        };
        Draw { pass, blend, zwrite: zbuf >> 32 & 1 == 0, atest, fog }
    }
}

pub struct Material {
    pub layer: Layer,
    pub draw: Draw,
    /// The object's world centre (draw-order sort).
    pub centre: [f32; 3],
    /// `(map, texture)`; `None` = untextured.
    pub key: Option<(usize, u32)>,
    pub first_index: u32,
    pub index_count: u32,
}

pub struct TextureImage {
    pub key: (usize, u32),
    pub width: u32,
    pub height: u32,
    /// Frame 0.
    pub rgba: Vec<u8>,
    /// Animated textures: frames 1.. and the ticks each frame shows (`CacheTexture` 0x11cd58: frame =
    /// (`GameState+0x38` / rate) % frames, `rate` in ticks (`ticks_per_frame`); the counter gains `VIDEO_FRAME_RATE / FRAME_RATE_INT` = 60 / 60 per game
    /// tick [M, L]).
    pub more: Vec<Vec<u8>>,
    pub rate: u32,
}

/// `psiCreateMapTextures` (0x11d200): the header's rate byte is frames per second; 1–59 become `60 / rate` ticks per
/// frame, anything else 1 [M].
pub fn ticks_per_frame(rate: u8) -> u32 { if (1..60).contains(&rate) { 60 / rate as u32 } else { 1 } }

impl TextureImage {
    /// Map `mi`'s texture `t`, every frame.
    pub fn from_texture(mi: usize, t: &nf_formats::texture::Texture) -> TextureImage {
        let pal = t.palette_rgba8();
        let rgba = |f: usize| t.frame(f).iter().flat_map(|&i| pal[i as usize]).collect::<Vec<u8>>();
        TextureImage { key: (mi, t.index as u32), width: t.width as u32, height: t.height as u32, rgba: rgba(0),
            more: (1..t.frames.max(1) as usize).map(rgba).collect(), rate: ticks_per_frame(t.rate) }
    }
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
    /// A script's animated object (script hash, stream): drawn while the simulation has it, posed with its root in world
    /// space (docs/systems/script-player.md).
    pub script: Option<(u32, u8)>,
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
    pub surfaces: Option<nf_formats::sfx::SurfaceSounds>,
    /// The HUD's sprite tables and font (docs/systems/hud.md).
    pub hud: Option<nf_formats::hud::HudTables>,
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
    /// The camera is Bond's eye (`step_bond`); with the free camera (`step`) the first-person gun is not drawn.
    pub first_person: bool,
    /// The tick of Bond's last shot (the first-person flash).
    pub last_shot: Option<u32>,
    pub view_time: f32,
    pub view_speed: f32,
    pub bob: [f32; 2],
    /// Aim mode (action 0x13, a pad's L1; not with the mouse, D-002) this tick.
    pub aiming: bool,
    pub auto_aim: bool,
    /// `Player_Disable` by a script (cut-scene): no input, no first-person view, until `Player_Enable`.
    pub disabled: bool,
    /// Surface sounds (`EffectInfo`) and Bond's footstep distance (`BL+0x924`).
    pub surfaces: Option<nf_formats::sfx::SurfaceSounds>,
    pub step_dist: f32,
}

impl Sim {
    /// One game tick with Bond at `camera` (eye): his object position on the collision floor below, aimed at chest
    /// height (stand-in for the game's player object and `Drone_View_Bones`).
    /// `trigger`: the fire button (action 9) held; `aiming`: aim mode (L1).
    pub fn step(&mut self, camera: [f32; 3], look: [f32; 3], trigger: bool, aiming: bool) {
        let floor = self.world.floor_below(camera, 4.0).unwrap_or(camera[1] - 1.6);
        self.first_person = false;
        self.run([camera[0], floor + 0.4, camera[2]], camera, look, look, trigger, aiming, false);
    }

    /// One game tick with Bond walking (`Player_Update`): his input, then the world with him as the player. In aim
    /// mode the mouse turn and look (`input.turn` radians, `input.look` units of π/2) first move the cursor within the
    /// aim box (±0.296 across, ±0.4 up) and turn the view only past its edge (pad aim mode; the mouse has none, D-002); the shot goes through the smoothed
    /// cursor (`Player_GetAimingPoint`).
    pub fn step_bond(&mut self, mut input: nf_game::player::Input, trigger: bool, reload: bool) {
        let (trigger, reload) = if self.disabled { input = Default::default(); (false, false) } else { (trigger, reload) };
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
        // Riding (`Player_FeetOnPoint` → `Control_InheritVelocity`): standing on a moving object (the truck), Bond moves
        // and turns with it — the feet ray reaches cc + 0.6 below him (cc 1.0347 [L]).
        let p = self.bond.pos;
        if let Some(g) = self.world.ground_motion(p, [p[0], p[1] - 1.6347, p[2]]) {
            let (sn, cs) = g.turn.sin_cos();
            let (x, z) = (p[0] + g.delta[0] - g.pivot[0], p[2] + g.delta[2] - g.pivot[2]);
            self.bond.pos = [g.pivot[0] + x * cs + z * sn, p[1] + g.delta[1], g.pivot[2] - x * sn + z * cs];
            self.bond.yaw += g.turn;
        }
        let before = self.bond.pos;
        self.bond.tick(&self.world, &input);
        // Footsteps (`AnimObjectUpdate`, player): `BL+0x924` gathers the distance moved; past the stride (1.65,
        // crouched 0.4125) it wraps (fmod) and a step sounds: crouched 0x286 at Bond, else `Effect_Player` → the floor
        // surface's footstep (`EffectInfo+0x16`) at the floor point; AI alertness 0 (drones do not hear it).
        // *Suggested*: the distance is Bond's horizontal movement per tick (`anim+0xD8`).
        let moved = ((self.bond.pos[0] - before[0]).powi(2) + (self.bond.pos[2] - before[2]).powi(2)).sqrt();
        if self.bond.grounded && !self.disabled {
            self.step_dist += moved;
            let stride = if self.bond.crouched { 0.4125 } else { 1.65 };
            if self.step_dist > stride {
                self.step_dist %= stride;
                let p = self.bond.pos;
                if self.bond.crouched { self.drones.audio.play3d(0x286, p, None); }
                else if let (Some(s), Some((t, _, m))) = (self.surfaces.as_ref(), self.world.segment_surface(p, [p[0], p[1] - 1.6347, p[2]])) {
                    if let Some(id) = nf_formats::sfx::SurfaceSounds::get(&s.footstep, m) { self.drones.audio.play3d(id, [p[0], p[1] - 1.6347 * t, p[2]], None); }
                }
            }
        }
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
        self.first_person = !self.disabled;
        self.run(self.bond.pos, head, look, aim, trigger, aiming, reload);
        // Script hand-offs (docs/systems/script-player.md): event 4 stops Bond; `Script_Free` after event 6 puts him
        // back at the script's matrix (on the floor below it), facing its +Z.
        for e in std::mem::take(&mut self.drones.player_events) {
            match e {
                nf_game::nis::Request::PlayerDisable => self.disabled = true,
                nf_game::nis::Request::PlayerEnable { at, .. } => {
                    if let Some(m) = at {
                        self.bond = nf_game::player::Bond::stand_at(&self.world, [m[3][0], m[3][1], m[3][2]], m[2][0].atan2(m[2][2]));
                    }
                    self.disabled = false;
                }
                _ => {}
            }
        }
        self.first_person = !self.disabled;
    }

    /// The cut-scene camera, when a script runs one: position, yaw, pitch (from the matrix's +Z) and vertical field of
    /// view.
    pub fn script_camera(&self) -> Option<([f32; 3], f32, f32, f32)> {
        let (m, fov) = self.drones.scripts.camera()?;
        let f = m[2];
        let l = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt().max(1e-6);
        Some(([m[3][0], m[3][1], m[3][2]], f[0].atan2(f[2]), (f[1] / l).clamp(-1.0, 1.0).asin(), fov))
    }

    fn run(&mut self, pos: [f32; 3], camera: [f32; 3], look: [f32; 3], aim: [f32; 3], trigger: bool, aiming: bool, reload: bool) {
        // The drones aim at and look for Bond's chest (`Drone_View_Bones`; one point here, 0.45 under the eye).
        let player = nf_game::drone::Player { pos, target: [camera[0], camera[1] - 0.45, camera[2]], eye: camera, aim: look };
        let d = nf_game::drone::Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.level, sequences: &self.seqs, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: self.surfaces.as_ref() };
        // `Player_WeaponFiring` → `Player_WeaponInitBullet` from the eye along the view (*deviation*: no auto-aim and
        // no aim-point ray yet); the first-person fire script's sound 0x403 feeds the drones' hearing.
        // The gun's clip and spare rounds live in Bond's inventory (BLData `+0x1B8`, `+0x170`), which pickups fill.
        let (wi, t) = (self.gun.weapon as usize, self.weapons.ammo_type(self.gun.weapon as usize));
        if let (Some(&c), Some(&n)) = (self.drones.inventory.clip.get(wi), self.drones.inventory.carried.get(t)) { self.gun.clip = c.max(0) as u32; self.gun.spare = n.max(0) as u32; }
        if let Some(w) = self.weapons.records.get(self.gun.weapon as usize) {
            let scripts = &self.scripts;
            let params = nf_game::gun::GunParams::from_record(w, 0, |h| scripts.get(&h).map(|s| s.length as u32));
            // Auto-aim (on by default in the game's options) only outside aim mode; in aim mode the offset eases back.
            // Auto-aim off (or aim mode): no target; the offsets still halve every tick (`Player_AutoAim` 0x1a7c98), so
            // the crosshair returns to the centre instead of staying where the last lock left it.
            let dir = if self.auto_aim && !aiming { self.drones.auto_aim(&d, camera, look, self.gun.weapon) } else {
                self.drones.aim_target = None;
                self.drones.aim_offset = self.drones.aim_offset.map(|v| v * 0.5);
                aim
            };
            let was_reloading = self.gun.reloading > 0;
            let rd0 = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
            // The gun's animation is busy while a fire script plays (gun state 9; idle = 0).
            let len_of = |h: u32| self.scripts.get(&h).map_or(0.0, |s| s.length as f32);
            let busy = (self.view_script == rd0(0xb0) || self.view_script == rd0(0xb4)) && self.view_time < len_of(self.view_script) - 1.0;
            let fired = self.gun.update_with(trigger, reload, busy, params);
            let (prev_script, prev_time) = (self.view_script, self.view_time);
            // The gun's scripts (`weapon+0xA0` idle, `+0xA4` reload at 1.35 from frame 6 with flag 0x1000, `+0xB0` fire,
            // `+0xB4` fire on the last round with flag 0x1000); after a script ends the last frame holds (flag 0x1000:
            // the idle is not re-added) — docs/systems/player.md.
            let rd = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
            let flags = rd(0x68);
            if fired {
                self.last_shot = Some(self.drones.frame);
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
                // The reload done (gun state back to 0): the idle script.
                if self.view_script == rd(0xa4) && self.gun.reloading == 0 { (self.view_script, self.view_time, self.view_speed) = (rd(0xa0), 0.0, 1.0); }
            }
            // The gun scripts' sound commands (`AnimProcessScriptCmds` op 1: `Sound_Play3D(cmd[2], …)` when the
            // script's frame crosses cmd[1]): the shot (0x403 at frame 1 of the P2K's fire scripts) and the reload's
            // three sounds (0x40F, 0x40E, 0x411 at frames 0x2E, 0x59, 0x74). Frame = script time + 1.
            if self.view_script != 0 {
                let from = if self.view_script != prev_script || (fired && self.view_time == 0.0) { f32::NEG_INFINITY } else { prev_time + 1.0 };
                let to = self.view_time + 1.0;
                if let Some(sc) = self.scripts.get(&self.view_script) {
                    for c in sc.commands.iter().filter(|c| c.op == 1 && c.args.len() >= 2) {
                        let f = c.args[0] as f32;
                        if from < f && f <= to {
                            self.drones.audio.play3d(c.args[1] as u32, camera, None);
                            if std::env::var_os("NF_SOUNDLOG").is_some() { eprintln!("tick {}: gun script {:08x} frame {f}: sound {:#x}", self.drones.frame, self.view_script, c.args[1]); }
                        }
                    }
                }
            }
            if fired {
                let sound = self.sfx.as_ref().map(|t| t.get(0x403)).unwrap_or(nf_formats::sfx::SfxHearing { id: 0x403, inner: 0.0, outer: 0.0, weight: 0.0, looped: false, cull: false });
                self.drones.fire_player(&d, camera, dir, self.gun.weapon, sound);
            }
        }
        if let Some(c) = self.drones.inventory.clip.get_mut(wi) { *c = self.gun.clip as i16; }
        if let Some(n) = self.drones.inventory.carried.get_mut(t) { *n = self.gun.spare as i16; }
        self.aiming = aiming;
        self.drones.bond_aiming = aiming;
        self.drones.bond_weapon = self.gun.weapon;
        self.drones.tick(&d, &player);
    }

    /// The crosshair offset (`BL+0x120 / +0x124`: −1…1 across half the screen) from the auto-aim offset angles: tan of the
    /// angle over the half-screen tangents of the game's projection [L] (0.76966 across, 0.57724 up); a larger yaw
    /// turns left (+X is screen-left in the object frame), so x flips.
    pub fn crosshair(&self) -> (f32, f32) {
        if self.aiming { return (self.cross[0], self.cross[1]); }
        (-self.drones.aim_offset[0].tan() / 0.76966, self.drones.aim_offset[1].tan() / 0.57724)
    }

    /// Characters whose drone is present but whose animation cannot be shown (drawn in the bind pose): (character,
    /// drone, script, its first sequence, in `seq_index`, that sequence's skeleton vs the mesh's).
    pub fn unposed(&self, characters: &[Character]) -> Vec<String> {
        let mut out = Vec::new();
        for (ci, c) in characters.iter().enumerate() {
            let Some(di) = self.char_drone.get(ci).copied().flatten() else { continue };
            let dr = &self.drones.list[di];
            if !dr.present || self.frame_of(di, c.skin.skeleton).is_some() { continue; }
            let q = dr.anim.script.and_then(|h| self.scripts.get(&h)).and_then(|s| s.sequences().next().map(|x| x.2));
            out.push(format!("char {ci} drone {di} (placement {}) state {:#x} script {:x?} seq {:x?} indexed {:?} mesh skeleton {}", dr.placement, dr.state, dr.anim.script, q,
                q.and_then(|q| self.seq_index.get(&q)), c.skin.skeleton));
        }
        out
    }

    /// The falling drops as camera-facing quads (`DrawDrops` → `psiDrawParticleList(0.2, …, 0x808080, 0x03000045)`:
    /// GS sprites, colour (63, 63, 63, 127) × texture, additive, depth-tested without writing). The 0.2 is taken as the
    /// full width (*suggested*, docs/systems/render.md).
    pub fn snow_quads(&self, right: [f32; 3], up: [f32; 3]) -> Vec<Vertex> {
        let Some(env) = self.drones.env.as_ref() else { return Vec::new() };
        let h = 0.1;
        let (r, u) = (right.map(|v| v * h), up.map(|v| v * h));
        let mut out = Vec::with_capacity(env.drops.len() * 6);
        for (p, &alive) in env.drops.iter().zip(&env.alive) {
            if !alive { continue; }
            let c = |sr: f32, su: f32, uv: [f32; 2]| Vertex { pos: [p[0] + r[0] * sr + u[0] * su, p[1] + r[1] * sr + u[1] * su, p[2] + r[2] * sr + u[2] * su], uv, rgba: [63, 63, 63, 127] };
            let (a, b, cc, d) = (c(-1.0, -1.0, [0.0, 1.0]), c(1.0, -1.0, [1.0, 1.0]), c(1.0, 1.0, [1.0, 0.0]), c(-1.0, 1.0, [0.0, 0.0]));
            out.extend_from_slice(&[a, b, cc, a, cc, d]);
        }
        out
    }

    /// The live animated script object from `src` (script hash, stream) on `skin`: its animation script and frame.
    pub fn script_anim(&self, src: (u32, u8), skin: u32) -> Option<(u32, f32)> {
        self.drones.scripts.objects.list.iter().flatten().rev().find_map(|o| match o {
            nf_game::nis::Obj::Anim { skin: k, script, frame, src: o } if *o == src && *k == skin => Some((*script, *frame)),
            _ => None,
        })
    }

    /// The world matrix of the live script entity `hash` from `src`.
    pub fn script_entity(&self, src: (u32, u8), hash: u32) -> Option<Mat4> {
        self.drones.scripts.objects.list.iter().flatten().rev().find_map(|o| match o {
            nf_game::nis::Obj::Entity { hash: h, matrix, src: o, .. } if *o == src && *h == hash => Some(*matrix),
            _ => None,
        })
    }

    /// The game data the simulation consults, borrowed from this Sim.
    pub fn data(&self) -> nf_game::drone::Data<'_> {
        nf_game::drone::Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.level, sequences: &self.seqs, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models , surfaces: self.surfaces.as_ref() }
    }

    /// (sequence index, frame) the first-person gun's script shows.
    pub fn view_frame(&self, skeleton: u8) -> Option<(usize, u32)> {
        let script = self.scripts.get(&self.view_script)?;
        let t = (1.0 + self.view_time) as u16;
        let (a, _, q) = script.sequences().find(|(a, b, _)| *a <= t && t <= *b).or_else(|| script.sequences().last())?;
        let &(_, i) = self.seq_index.get(&q).filter(|(sk, _)| *sk == skeleton)?;
        Some((i, (t.saturating_sub(a) + 1) as u32))
    }

    /// The first-person gun's matrix (`Player_SetWeaponAnimObj` + `Player_PositionGun` 0x1a3b40): the gun turns toward
    /// the crosshair — pitch `cy·(π/6)`, yaw aspect·`cx·(π/6)` (G+0x50 / +0x54, zoom 1) — about the view-space point
    /// (0, −0.2, 0.5): position = eye + (offset `+0xE0` + bob + (0, 0.2, −0.5))·R_gun + (0, −0.2, 0.5)·R_view. The
    /// rotation order (`RotZYX`) and the yaw sign are *suggested*: the gun points along the turned forward.
    pub fn gun_matrix(&self) -> [[f32; 4]; 4] {
        let w = &self.weapons.records[self.gun.weapon as usize];
        let f = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
        let (p, t) = (self.bob[0], self.bob[1]);
        let l = [f(0xe0) + 0.01 * p.sin() + 0.001 * t.sin(), f(0xe4) + 0.01 * p.cos().abs() + 0.001 * (1.7 * t).cos(), f(0xe8) + 0.02 * (0.84329 * p).sin()];
        let (y, pt) = (self.bond.yaw, self.bond.pitch * std::f32::consts::FRAC_PI_2);
        let left = [y.cos(), 0.0, -y.sin()];
        let up = [-pt.sin() * y.sin(), pt.cos(), -pt.sin() * y.cos()];
        let fwd = self.bond.look();
        // The gun's axes: forward turned by the crosshair angles (right = −left), then left / up rebuilt.
        let (cx, cy) = self.crosshair();
        let (gy, gp) = (4.0 / 3.0 * cx * std::f32::consts::FRAC_PI_6, cy * std::f32::consts::FRAC_PI_6);
        let gf = [0, 1, 2].map(|k| fwd[k] * gp.cos() * gy.cos() - left[k] * gp.cos() * gy.sin() + up[k] * gp.sin());
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let norm = |v: [f32; 3]| { let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9); v.map(|x| x / n) };
        let gl = norm(cross(up, gf));
        let gu = cross(gf, gl);
        let h = self.bond.head();
        let o = [l[0], l[1] + 0.2, l[2] - 0.5];
        let pos = [0, 1, 2].map(|k| h[k] + o[0] * gl[k] + o[1] * gu[k] + o[2] * gf[k] - 0.2 * up[k] + 0.5 * fwd[k]);
        [[gl[0], gl[1], gl[2], 0.0], [gu[0], gu[1], gu[2], 0.0], [gf[0], gf[1], gf[2], 0.0], [pos[0], pos[1], pos[2], 1.0]]
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
    /// A script entity (script hash, stream, entity hash): drawn at the simulation object's matrix while it exists.
    pub script: Option<(u32, u8, u32)>,
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
    /// The snow flake texture (`DrawDrops`: 0x03000045).
    pub snow_texture: Option<TextureImage>,
    /// The level's music (track, SMF, SSD) and the music tables (set by the loader).
    pub music_files: Option<(i32, nf_formats::music::Smf, Vec<u8>)>,
    pub music_tables: Option<(Vec<nf_formats::music::MapMusic>, Vec<nf_formats::music::MusicVars>)>,
    /// The level's sound bank (`Sound_Ready` → `SFXLoadSoundBank`): slot, bank, ADPCM (set by the loader).
    pub sound_bank: Option<(usize, nf_formats::soundbank::Bank, Vec<u8>)>,
    /// The level's fog (`psiPreGameRun` 0x11e2e8: colour, `Ps2Far1`; `None` = `FogOn` 0).
    pub fog: Option<([u8; 3], f32)>,
    /// The HUD's textures by hash (docs/systems/hud.md), from any map of the archive, and its tables from ACTION.ELF.
    pub hud_textures: Vec<(u32, TextureImage)>,
    pub hud: Option<nf_formats::hud::HudTables>,
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

/// `psiPreGameRun` (0x11e2e8, read): per level the fog colour and `Ps2Far1`; `FogOn` only on these levels.
pub fn fog_for(level: u32) -> Option<([u8; 3], f32)> {
    Some(match level {
        0x0700_0002 => ([0x24, 0x2e, 0x4c], 9_987_520.0),
        0x0700_0004 => ([0x11, 0x15, 0x21], 9_379_776.0),
        0x0700_0005 => ([0x3a, 0x4d, 0x68], 19_118_528.0),
        0x0700_0006 => ([0x2e, 0x3c, 0x52], 8_059_232.0),
        0x0700_0008 => ([0x3b, 0x50, 0x69], 13_191_350.0),
        0x0700_000c => ([0xd1, 0xd5, 0xd6], 20_475_840.0),
        0x0700_000d => ([0x92, 0x85, 0x69], 19_707_840.0),
        _ => return None,
    })
}

/// Grouped by layer, then (world layers) the object — the game sorts cels per object (`View_AddCels`) — then texture
/// and draw state. The value keeps the object's world centre for the per-frame sort.
type Groups = BTreeMap<(Layer, [u32; 3], u32, Option<(usize, u32)>, Draw), (Vec<Vertex>, Vec<u32>, Vec<CharVertex>, [f32; 3])>;

fn add_instance(level: &Level, inst: &Instance, layer: Layer, character: Option<(u32, Option<u32>)>, groups: &mut Groups, bounds: Option<&mut ([f32; 3], [f32; 3])>) {
    let obj = &level.maps[inst.map].objects[inst.object];
    let mut m = inst.matrix;
    if layer == Layer::SkyCamera { m[3] = [0.0, 0.0, 0.0, 1.0]; }
    let mut bounds = bounds;
    // The pass and fog by layer: sky slots with placement parameter 4 = 1 go before the world (fogged), others after
    // the fog pass; world objects with glist flag 0x1 are in the alpha list (list 1).
    let param4 = inst.placement.params.iter().find(|(i, _)| *i == 4).map(|&(_, v)| v).unwrap_or(0);
    let (pass, fog) = match layer {
        Layer::SkyCamera | Layer::SkyFixed => if param4 == 1 { (0, 1) } else { (4, 0) },
        Layer::World => if obj.flags & 1 != 0 { (3, 2) } else { (1, 1) },
        Layer::Character => (2, 1),
        Layer::Effect => (5, 0),
    };
    // The object's world centre (glist sphere centre through the instance matrix) for the draw-order sort; world layers
    // keep each object apart.
    let centre = transform(&inst.matrix, obj.centre);
    let ikey = if matches!(layer, Layer::World) { centre.map(f32::to_bits) } else { [0; 3] };
    // GS state: each leaf's `DIRECT` writes, inherited by the leaves that send none.
    let (mut alpha, mut test, mut zbuf) = (0x44u64, 0x5001bu64, 0x3100_0000u64);
    for (li, leaf) in obj.gfx.leaves.iter().enumerate() {
    // World objects draw their leaves and batches in order: a leaf repeats its triangles with blend layers (another
    // texture weighted by vertex alpha, 0 where it does not show) after the opaque base they cover.
    for &(r, v) in &leaf.gs { match r { 0x42 => alpha = v, 0x47 => test = v, 0x4e => zbuf = v, _ => {} } }
    let draw = Draw::from_gs(pass, fog, alpha, test, zbuf);
    for (bi, b) in leaf.batches.iter().enumerate() {
        let lkey = if matches!(layer, Layer::World) { (li as u32) << 16 | bi as u32 } else { 0 };
        let key = b.texture.filter(|&t| (t as usize) < level.maps[inst.map].textures.len()).map(|t| (inst.map, t));
        let (vs, is, cv, c) = groups.entry((layer, ikey, lkey, key, draw)).or_default();
        *c = centre;
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
/// `Draw_MuzzleFlash` for Bond: local +Z along the muzzle datum's +Z, a random roll, scale 0.9 + rand(0.2).
fn player_flash_matrix(datum: &Mat4, seed: u32) -> Mat4 {
    let mut s = seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let mut rand = || { s ^= s << 13; s ^= s >> 17; s ^= s << 5; (s >> 8) as f32 / (1u32 << 24) as f32 };
    let n = |v: [f32; 3]| { let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9); v.map(|x| x / l) };
    let z = n([datum[2][0], datum[2][1], datum[2][2]]);
    let (x0, y0) = (n([datum[0][0], datum[0][1], datum[0][2]]), n([datum[1][0], datum[1][1], datum[1][2]]));
    let (a, sc) = (rand() * std::f32::consts::TAU, 0.9 + rand() * 0.2);
    let (sa, ca) = (a.sin(), a.cos());
    let x = [0, 1, 2].map(|k| (x0[k] * ca + y0[k] * sa) * sc);
    let y = [0, 1, 2].map(|k| (y0[k] * ca - x0[k] * sa) * sc);
    [[x[0], x[1], x[2], 0.0], [y[0], y[1], y[2], 0.0], [z[0] * sc, z[1] * sc, z[2] * sc, 0.0], [datum[3][0], datum[3][1], datum[3][2], 1.0]]
}

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
            // Debug: `NF_HIDE=name,name` leaves out world objects whose name contains one of them.
            if let Ok(h) = std::env::var("NF_HIDE") { if h.split(',').any(|n| !n.is_empty() && obj.name.contains(n)) { continue; } }
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
            characters.push(Character { matrix: inst.matrix, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones: obj.gfx.slot_bones.clone(), start, role, floor: None, script: None });
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
            attachments.push(Attachment { character, bone: dt.bone as usize, local, flash: None, tracer: None, dropped: false, script: None });
            // The same gun as a pickup once the drone drops it.
            add_instance(level, &winst, Layer::Character, Some((character, Some(attachments.len() as u32))), &mut groups, None);
            attachments.push(Attachment { character, bone: 0, local, flash: None, tracer: None, dropped: true, script: None });
            // Its muzzle flash: `DroneWeap_FireWeapon` asks `AnimGetBoneWorldTrans(obj, datum id, 1, …)`, which looks up
            // the datum whose id is id + 1 — the muzzle (07000005: 8 → 9 at x −0.77, 16 → 17 at x −1.18).
            if let Some((fmesh, rgb)) = wd.muzzle_flash(weapon as u8) {
                let muzzle = skin.datums.iter().find(|d| d.id == datum_id + 1 && d.bone >= 0);
                if let (Some(&(fmap, fobject)), Some(md)) = (level.by_hash.get(&fmesh), muzzle) {
                    let finst = Instance { map: fmap, object: fobject, ..winst };
                    add_instance(level, &finst, Layer::Effect, Some((character, Some(attachments.len() as u32))), &mut groups, None);
                    attachments.push(Attachment { character, bone: md.bone as usize, local: anim::quat_trans_to_mat(md.q, md.t), flash: Some(rgb), tracer: None, dropped: false, script: None });
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
                        attachments.push(Attachment { character: ci, bone: bone as usize, local: ident, flash: None, tracer: None, dropped: false, script: None });
                    }
                    // The muzzle flash (`Player_MuzzleFlash`): mesh `weapon+0x48` on datum 0, colour (+0x56, +0x55, +0x54).
                    let r = &wd.records[2];
                    let flash = u32::from_le_bytes(r[0x48..0x4c].try_into().unwrap());
                    if let (Some(&(fmap, fobject)), Some(dt)) = (level.by_hash.get(&flash), skin.datums.iter().find(|d| d.id == 0 && d.bone >= 0)) {
                        let finst = Instance { map: fmap, object: fobject, matrix: ident, placement: placement.clone() };
                        add_instance(level, &finst, Layer::Effect, Some((ci, Some(attachments.len() as u32))), &mut groups, None);
                        attachments.push(Attachment { character: ci, bone: dt.bone as usize, local: anim::quat_trans_to_mat(dt.q, dt.t), flash: Some([r[0x56], r[0x55], r[0x54]]), tracer: None, dropped: false, script: None });
                    }
                    characters.push(Character { matrix: ident, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones: level.maps[map].objects[object].gfx.slot_bones.clone(), start: None, role: drone::SpawnRole::Placed, floor: None, script: None });
                    view_model = Some(ci as usize);
                }
            }
        }
        // Script objects (docs/systems/script-player.md): every animation and drawn entity the main map's script players
        // can make; each shows only while the simulation has the object.
        let mut script_objects = 0;
        if let Some(m) = level.main_map() {
            let ident = anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]);
            let placement = level.maps[m].placements.first().cloned().unwrap_or_default();
            let mut seen = BTreeSet::new();
            let players: Vec<nf_game::nis::Player> = level.maps[m].placements.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xd9)
                .filter_map(|(i, p)| nf_game::nis::Player::from_placement(i, p)).collect();
            for h in players.iter().flat_map(|p| p.scripts).filter(|&h| h != 0) {
                let Some(nis) = level.nis.get(&h) else { continue };
                for (si, st) in nis.streams.iter().enumerate() {
                    for cmd in st.decode() {
                        match cmd {
                            nf_formats::nis::Cmd::Anim { skin: sh, .. } => {
                                if !seen.insert((h, si, sh)) { continue; }
                                let (Some(skin), Some(skeleton)) = (level.skins.get(&sh), level.skins.get(&sh).and_then(|k| skeletons.get(&k.skeleton))) else { continue };
                                let ci = characters.len() as u32;
                                let mut slot_bones = Vec::new();
                                for &mh in skin.main.iter().flatten() {
                                    let Some(&(map, object)) = level.by_hash.get(&mh) else { continue };
                                    let inst = Instance { map, object, matrix: ident, placement: placement.clone() };
                                    add_instance(level, &inst, Layer::Character, Some((ci, None)), &mut groups, None);
                                    slot_bones = level.maps[map].objects[object].gfx.slot_bones.clone();
                                }
                                for &(bone, part) in &skin.extra {
                                    let Some(&(map, object)) = part.and_then(|ph| level.by_hash.get(&ph)) else { continue };
                                    let pinst = Instance { map, object, matrix: ident, placement: placement.clone() };
                                    add_instance(level, &pinst, Layer::Character, Some((ci, Some(attachments.len() as u32))), &mut groups, None);
                                    attachments.push(Attachment { character: ci, bone: bone as usize, local: ident, flash: None, tracer: None, dropped: false, script: None });
                                }
                                characters.push(Character { matrix: ident, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones, start: None, role: drone::SpawnRole::Placed, floor: None, script: Some((h, si as u8)) });
                                script_objects += 1;
                            }
                            // Triggers, AI volumes and emitters draw nothing (`Script_CreateEntity`).
                            nf_formats::nis::Cmd::Entity { hash, obj_type, .. } if !matches!(obj_type, 0xea | 0xeb | 0xf5 | 0xf2) => {
                                if !seen.insert((h, si, hash)) { continue; }
                                let Some(&(map, object)) = level.by_hash.get(&hash) else { continue };
                                let inst = Instance { map, object, matrix: ident, placement: placement.clone() };
                                add_instance(level, &inst, Layer::Character, Some((0, Some(attachments.len() as u32))), &mut groups, None);
                                attachments.push(Attachment { character: 0, bone: 0, local: ident, flash: None, tracer: None, dropped: false, script: Some((h, si as u8, hash)) });
                                script_objects += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        // Bullet tracers: drone bullets draw `GUN_BULLET_TRAIL` (Bullet_update state 0), stretched along their
        // direction by the speed (Control_GetObjMatrix scale 1, 1, speed); a pool of slots, hidden when unused.
        if let (Some(_), Some(&(tmap, tobj))) = (drones, level.by_hash.get(&0x0200_0123)) {
            for slot in 0..TRACER_SLOTS {
                let tinst = Instance { map: tmap, object: tobj, matrix: anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]), placement: level.maps[level.main_map().unwrap_or(0)].placements.first().cloned().unwrap_or_default() };
                add_instance(level, &tinst, Layer::Effect, Some((0, Some(attachments.len() as u32))), &mut groups, None);
                attachments.push(Attachment { character: 0, bone: 0, local: tinst.matrix, flash: None, tracer: Some(slot), dropped: false, script: None });
            }
        }
        for (inst, follows_camera) in &sky {
            add_instance(level, inst, if *follows_camera { Layer::SkyCamera } else { Layer::SkyFixed }, None, &mut groups, None);
        }
        let (mut vertices, mut indices, mut materials, mut char_verts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut char_vertex_start = None;
        for ((layer, _, _, key, draw), (vs, is, cv, centre)) in groups {
            if is.is_empty() { continue; }
            if matches!(layer, Layer::Character | Layer::Effect) && char_vertex_start.is_none() { char_vertex_start = Some(vertices.len()); }
            let base = vertices.len() as u32;
            materials.push(Material { layer, draw, centre, key, first_index: indices.len() as u32, index_count: is.len() as u32 });
            indices.extend(is.iter().map(|i| i + base));
            vertices.extend(vs);
            char_verts.extend(cv);
        }
        let keys: BTreeSet<(usize, u32)> = materials.iter().filter_map(|m| m.key).collect();
        let textures = keys.into_iter().map(|key| {
            TextureImage::from_texture(key.0, &level.maps[key.0].textures[key.1 as usize])
        }).collect();
        let hud_textures = crate::hud::TEXTURES.iter().filter_map(|&h| level.maps.iter().enumerate().find_map(|(mi, m)| m.textures.iter().find(|t| t.hashcode == h).map(|t| {
            (h, TextureImage::from_texture(mi, t))
        }))).collect();
        let hud = drones.and_then(|d| d.hud.clone());
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
        let held_back = characters.iter().filter(|c| matches!(c.role, drone::SpawnRole::Reinforcement { .. })).count();
        let stats = format!("{stats}; {started}/{} characters with their start animation; {} weapons held ({weapons_missing} without mesh or datum), {flashes} with a muzzle flash; {held_back} character meshes held back by spawners (R shows them); {script_objects} script objects", characters.len(), attachments.iter().filter(|a| a.script.is_none() && a.flash.is_none() && a.tracer.is_none()).count());
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
            let data = nf_game::drone::Data { tables: &d.tables, props: &props, scripts: &d.scripts, weapons: &weapons, world: &world, level: d.level_id, sequences: &seqs, skins: &level.skins, skeletons, nav: &nav, covers: &covers, tuning: &d.tuning, hitboxes: &hitboxes, volumes: &volumes, sfx: d.sfx.as_ref(), models: &models, surfaces: d.surfaces.as_ref() };
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
            Some(Sim { drones, tables: d.tables.clone(), props, scripts: d.scripts.clone(), weapons, world, level: d.level_id, char_drone, seq_index, seqs, skins: level.skins.clone(), skeletons: skeletons.clone(), nav, covers, tuning: d.tuning.clone(), hitboxes, sfx: d.sfx.clone(), volumes, models, gun: nf_game::gun::Gun::new(2, 7, 41), bond, cursor: [0.0; 2], cross: [0.0; 2], view_script: 0, first_person: false, last_shot: None, view_time: 0.0, view_speed: 1.0, bob: [0.0; 2], aiming: false, auto_aim: false, disabled: false, surfaces: d.surfaces.clone(), step_dist: 0.0 })
        });
        let fog = drones.and_then(|d| fog_for(d.level_id));
        let snow_texture = level.maps.iter().enumerate().find_map(|(mi, m)| m.textures.iter().find(|t| t.hashcode == 0x0300_0045).map(|t| {
            TextureImage::from_texture(mi, t)
        }));
        Scene { music_files: None, music_tables: None, sound_bank: None, snow_texture, vertices, indices, materials, textures, hud_textures, hud, view_model, fog, bounds, start, stats, characters, char_vertex_start, char_verts, sequences, attachments, sim }
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
            // Drones: the simulation's pose — blended scripts, fractional frames, upper-body aim — the same one the hit
            // boxes use (`Drone::object_pose`).
            if let Some((s, di)) = drone_of(ci) {
                let dr = &s.drones.list[di];
                let (world, hip) = dr.object_pose(&s.data())?;
                let m = nf_formats::placement::object_matrix([0.0, dr.yaw, 0.0], [dr.pos[0], dr.draw_y(hip), dr.pos[2]]);
                return Some((anim::skin_matrices(&world, &c.skin), world, m));
            }
            // Script animations: the object's frame, the root kept (the object sits at the origin).
            if let Some(src) = c.script {
                let s = self.sim.as_ref()?;
                let (script, frame) = s.script_anim(src, c.skin.hashcode)?;
                let (q, f) = nf_game::anim::script_frame(s.scripts.get(&script)?, frame)?;
                let seq = s.seqs.get(&q).filter(|q| anim::fits(q, &c.skeleton))?;
                let world = anim::world_pose(&anim::local_pose(seq, &c.skeleton, &c.skin, f as u32), &c.skin);
                return Some((anim::skin_matrices(&world, &c.skin), world, c.matrix));
            }
            let view = (Some(ci) == self.view_model).then_some(()).and(self.sim.as_ref());
            let (i, frame) = match (view, drone_of(ci)) {
                (Some(s), _) => s.view_frame(c.skin.skeleton)?,
                (None, Some((s, di))) => s.frame_of(di, c.skin.skeleton)?,
                (None, None) => self.frame_of(c, choice, ticks)?,
            };
            let seq = &self.sequences[&c.skin.skeleton][i];
            let (mats, world, hip) = cache.entry((c.skin.hashcode, i, frame)).or_insert_with(|| {
                let mut local = anim::local_pose(seq, &c.skeleton, &c.skin, frame);
                let hip = local[0].0[1];
                local[0].0 = [0.0; 3];
                let world = anim::world_pose(&local, &c.skin);
                (anim::skin_matrices(&world, &c.skin), world, hip)
            }).clone();
            if let Some(s) = view {
                let g = s.gun_matrix();
                let far = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
                return Some((mats, world, if s.first_person { g } else { far }));
            }
            let mut m = c.matrix;
            let a0 = drone_height(hip);
            m[3][1] = match c.floor { Some(f) => f + a0, None => c.matrix[3][1] + a0 - 1.0 };
            Some((mats, world, m))
        }).collect();
        // Held back: spawner reinforcements and drones disabled while they wait for a switch channel (object flags
        // 0x30, `NDrone2_Enable(0)`); R shows them.
        let hidden = |c: &Character, ci: usize| match c.script {
            Some(src) => self.sim.as_ref().and_then(|s| s.script_anim(src, c.skin.hashcode)).is_none(),
            None => !opts.reinforcements && match drone_of(ci) {
                Some((s, di)) => !s.drones.list[di].present || !s.drones.list[di].enabled,
                None => matches!(c.role, drone::SpawnRole::Reinforcement { .. }),
            },
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
            if let Some((h, si, hash)) = a.script {
                return self.sim.as_ref().and_then(|s| s.script_entity((h, si), hash)).unwrap_or(far);
            }
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
            // Unarmed (dropped, or surrendered: `ChangeWeapon(1)`): no gun. *Deviation*: a drawn sidearm (weapon 6) still
            // shows the original weapon's mesh.
            if drone_of(a.character as usize).is_some_and(|(s, di)| s.drones.list[di].weapon <= 1) { return collapse(&datum); }
            if a.flash.is_none() { return datum; }
            // Bond's flash: 3 ticks after his shot (`BL+0x938` = 3), along the datum's +Z with a random roll and scale.
            if Some(a.character as usize) == self.view_model {
                return match self.sim.as_ref().and_then(|s| s.last_shot.filter(|&f| s.drones.frame.saturating_sub(f) < 3).map(|f| (s, f))) {
                    Some((s, f)) => player_flash_matrix(&datum, s.drones.frame ^ f.wrapping_mul(0x9e37_79b9)),
                    None => collapse(&datum),
                };
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_rate_is_ticks_per_frame() {
        // The rates on 07000005: 12 fps → 5 ticks, 17 → 3, 30 → 2, 0 → every tick.
        assert_eq!([12, 17, 30, 0, 60, 255].map(ticks_per_frame), [5, 3, 2, 1, 1, 1]);
        // Sister: the old reading (the byte as ticks) disagrees.
        assert_ne!(ticks_per_frame(12), 12);
    }
}
