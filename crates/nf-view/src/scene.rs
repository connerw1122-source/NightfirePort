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

/// Per-weapon op-1 sound id remap (docs/systems/sound.md §7 line 313): the first-person gun's animation script
/// emits a sound id that the game replaces based on the player's weapon (`+0xDC → +0x62`). The P2K (weapon 2) has no
/// remap — 0x403 plays as-is. Other weapons' fire sounds are remapped so they do not all sound like the P2K [M].
fn remap_op1_sound(id: u32, weapon: u8) -> u32 {
    match (id, weapon) {
        (1, 7) | (1, 9) => 2,
        (0x403, 3) | (0x403, 5) => 2,
        (0xfa, 0x14) => 0x1fb,
        (0x106, 0x1e..=0x23) => 0x284,
        (0x1e1, 0x10) => 0x602,
        (4, 0x16) | (4, 0x17) => 0x604,
        _ => id,
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub rgba: [u8; 4],
    /// The vertex normal (snorm, xyz + 0): world space for static World geometry, object/bone space for
    /// everything the palette poses (the shader rotates those). Used by the dynamic lights (fix-lighting).
    pub nrm: [i8; 4],
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
    /// The object's world-space bounding-sphere radius (the glist radius scaled by the instance matrix). Used by the
    /// CPU-side frustum cull (A-11e): a draw is skipped when its sphere is fully outside the view frustum.
    pub radius: f32,
    /// The room cel this static draw sits in — `build_FindCel` (0x1c8cb0) over the placement's world
    /// centre, the same lookup `build_LinkToRoom` (0x1c9680) uses to link the game's objects to their
    /// cel. `None` = no room cel contains it (outdoor/uncel'd geometry — never cel-culled).
    /// `View_AddCels` (0x1e6ba0) frustum-tests each cel's bbox (`Vision_InView`); fix-render3 skips a
    /// World draw whose cel fails that test.
    pub cel: Option<u16>,
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
    pub voices: Option<Vec<nf_formats::sfx::VoiceLine>>,
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
    /// Per character: its script stream when the character is a script object (`Scene::Character::script`).
    pub char_script: Vec<Option<(u32, u8)>>,
    /// Per character: its skin hashcode (for `characters_script_pos`'s entity lookup).
    pub char_script_skin: Vec<Option<u32>>,
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
    /// Bond fired during the latest `step_bond` (the muzzle light's one tick; `last_shot` is stamped before the
    /// drone tick advances the frame, so it cannot say this alone).
    pub shot_tick: bool,
    pub view_time: f32,
    pub view_speed: f32,
    pub bob: [f32; 2],
    /// Aim mode (action 0x13, a pad's L1; not with the mouse, D-002) this tick.
    pub aiming: bool,
    /// The crosshair state (BLData+0x133): 2 normal, 3 = the red "no" mark on a non-enemy target.
    /// Set by the `Check_Target` block in `run` every 6th frame, read by the HUD [M 0x1a7810, 0x19e6d0].
    pub cross_state: u8,
    pub auto_aim: bool,
    /// `BLData+0x8D0`: the current zoom factor (1.0 = no zoom, up to `weapon_data+0x88`). The FOV is divided
    /// by this. `Player_Zoom` (0x1a1a70) interpolates it toward `zoom_target` (`+0x8D4`).
    pub zoom: f32,
    /// `BLData+0x8D4`: the target zoom factor. Right mouse (mouse-first) zooms in; releasing returns to 1.0.
    pub zoom_target: f32,
    /// `Player_Disable` by a script (cut-scene): no input, no first-person view, until `Player_Enable`.
    pub disabled: bool,
    /// Surface sounds (`EffectInfo`) and Bond's footstep distance (`BL+0x924`).
    pub surfaces: Option<nf_formats::sfx::SurfaceSounds>,
    pub voices: Option<Vec<nf_formats::sfx::VoiceLine>>,
    pub step_dist: f32,
    /// The level's zip wires (placement type 0x3E, `Wire_Create` 0x1981d8). Bond grabs one by walking
    /// into its world sphere (fix-wire: the real grab + crossing live in `nf_game::player`).
    pub wires: Vec<nf_game::wire::Wire>,
    /// Whether Bond is currently on a wire (substate 6/0xF) — the music event 0xD source.
    pub creeping: bool,
    /// The wire anims' root tracks, resolved from the level archive's kind-5 scripts + the anim
    /// archive's sequences (fix-wire).
    pub wire_anims: nf_game::wire::WireAnims,
    /// The wire switch channel currently held on (0 = none) — set 1 while Bond rides, 0 on drop
    /// [M Player_Wire 0x1976bc/0x1978c4].
    pub wire_channel: u16,
    /// The level's ladders (placement type 0x22, `Ladder_Create` 0x1966c0, class 0x2D) — fix-ladder.
    pub ladders: Vec<nf_game::ladder::Ladder>,
    /// The set of weapon ids that have a view model built in `Scene::view_models` (D-003: the cycle skips
    /// weapons not in this set, because their skin is not in the level archive).
    pub view_model_ids: std::collections::HashSet<u8>,
    /// The level's room cels (`Cel`; `build_FindCel` 0x1c8cb0 walks them, `Lights_CalcAmbientLight` 0x1cb178
    /// reads their ambient) — the source of the 0x800-object ambient ramp.
    pub cels: Vec<Cel>,
    /// Per character (scene index): `obj+0x100..0x102`, the ambient-light bytes of `Lights_CalcAmbientLight`
    /// (0x1cb178), initialised to the 0xff,ff,ff sentinel of `control_init_object` (0x134630) and stepped by
    /// `ramp_char_ambient` each tick.
    pub char_ambient: Vec<[u8; 3]>,
    /// The scene character indices of the first-person view models (`Scene::view_models` values) — they follow
    /// Bond's cel instead of a drone's.
    pub view_model_chars: Vec<usize>,
    /// fix-render3: the `ambient_at` call counter (a test guard for the posed-vertices hoist).
    pub ambient_lookups: std::cell::Cell<usize>,
    /// The language string table (`USATxt.dat`, the NTSC-U default `FNames[SelLang]`) — the source of the
    /// HUD's label text (`Txt_BindLabel` 0x1cad60). Loaded in `nf_view::load`.
    pub lang: Option<nf_formats::hud::Lang>,
    /// The HUD's per-frame state (`fix-hud`): the weapon-name timer (`BL+0x95E`), the health pane's
    /// crouch-icon timer, the pickup message bar (pane 21) and how many `drones.taken` events it has
    /// converted. Stepped per tick in `hud_tick` (the game's HUD update runs per 60 Hz frame).
    pub hud: HudState,
}

/// A status-pane message (`Text_AddMsg` 0x1b0c30's `TXT_MSG` node): the bound text, its remaining ticks,
/// and — for the objective banner — the status label id shown above it (0x2000051 "NEW OBJECTIVE"
/// / 0x2000053 "OBJECTIVE COMPLETE"; `HUD_UpdateStatusPane` 0x19c310:127-144 rebinds it per display
/// by comparing the message text with "OBJECTIVE COMPLETE" — the port carries the id from the
/// event instead) and whether the game's Fail-prefixed format ("%s: %s", 0x30cbb8) rendered it.
#[derive(Clone, Debug)]
pub struct Msg { pub text: String, pub time: f32, pub status: u32, pub failed: bool }

impl Msg {
    fn new(text: String, time: f32) -> Msg { Msg { text, time, status: 0, failed: false } }
}

/// The HUD state the port keeps in Sim (the game keeps it in BLData / the TXTMSG list / pane structs).
#[derive(Clone, Debug, Default)]
pub struct HudState {
    /// The (viewer 0, type 6) pickup message queue, head first (`Text_AddMsg` 0x1b0c30's list head 0x31997c).
    pub msgs: std::collections::VecDeque<Msg>,
    /// How many `drones.taken` events `hud_tick` has converted into messages.
    pub taken_seen: usize,
    /// `BL+0x95E`: 60 ticks after a weapon whose record's +2 word changed; the ammo pane shows the
    /// fire-mode label bright while it runs, then the weapon name (`HUD_UpdateAmmoPane` 0x19a7f0).
    pub weapon_label_timer: u8,
    /// `obj+0x63`: the weapon id the HUD last saw (the timer's change test) [M 0x19a7f0:206-213].
    pub hud_weapon: u8,
    /// The health pane's `CrouchIconTimer` (60 after Bond stands; fades sprite 19 over the last 30)
    /// [M `HUD_UpdateHealthPane` 0x19b9c8:305-328].
    pub crouch_icon_timer: i32,
    /// The pickup bar's close animation (its remaining width; `HUD_UpdateStatusPane` 0x19c310 state 4
    /// shrinks 16 px per tick — pane+0xC = 0x10 for the pickup pane).
    pub bar_close: f32,
    /// The type-2 objective notice queue (pane 3, `MsgObjectiveStatusPane` 0x319710) and the type-3
    /// mission banner queue (pane 2, `MsgMissionStatusPane` 0x2bd538) — the game keeps one TXTMSG
    /// list per type; `Mission_MonitorObjectives` posts type 2 and `Mission_Update` type 3
    /// [M 0x1827d8/0x182c60, docs/systems/mission.md].
    pub objective_msgs: std::collections::VecDeque<Msg>,
    pub mission_msgs: std::collections::VecDeque<Msg>,
    /// The mission banner's second line: the `FailLabel` global (0x30cba8) — rebound per display
    /// [M 0x19c310:171-173].
    pub fail_label: u32,
    /// The banners' close animations (16 px per tick, the panes' slide field).
    pub objective_close: f32,
    pub mission_close: f32,
}

impl HudState {
    /// The bar's open width when a message is up (256) — the pane template's width [D hud.md].
    const BAR_W: f32 = 256.0;
    /// The objective banner's open width (480, `ObjectiveStatusSprInfo[2].w` [L 0x319608+0x2c·2]) and
    /// the mission banner's open height (103, `MissionStatusSprInfo[2].h` [L 0x319580+0x2c·2]).
    const OBJECTIVE_W: f32 = 480.0;
    const MISSION_H: f32 = 103.0;
}

impl Sim {
    /// A script character's live position: the root-bone world translation of its animation. The
    /// script Anim object sits at the origin and the sequence's root carries it (nis.rs; the game's
    /// `AnimSeqTick` 0x1c28d0 takes the root motion as the object's movement) — so the root
    /// translation is the object position `Lights_CalcAmbientLight` (0x1cb178) reads. fix-render2:
    /// the old version looked the skin up through `script_entity`, which matches only `Obj::Entity`
    /// — script Anim characters (the cut-scene actors, the truck) never resolved and kept the white
    /// 0xff sentinel, which is the user's "every cut-scene still draws characters at the old,
    /// over-bright level" and "the 07000005 truck is too bright".
    pub fn characters_script_pos(&self, ci: usize) -> Option<[f32; 3]> {
        let src = self.char_script.get(ci).copied().flatten()?;
        let skin_hash = (*self.char_script_skin.get(ci)?)?;
        let (script, frame) = self.script_anim(src, skin_hash)?;
        let (q, f) = nf_game::anim::script_frame(self.scripts.get(&script)?, frame)?;
        let seq = self.seqs.get(&q)?;
        let skin = self.skins.get(&skin_hash)?;
        let skeleton = self.skeletons.get(&skin.skeleton)?;
        if !anim::fits(seq, skeleton) { return None; }
        let world = anim::world_pose(&anim::local_pose(seq, skeleton, skin, f as u32), skin);
        Some([world[0][3][0], world[0][3][1], world[0][3][2]])
    }
}

    /// The ambient triple for a 0x800 object at `p` with the zone channel byte `off` (the game's
    /// `switch_channels[cel+0xa2]` value: the read shifts `base` by that many bytes).
pub fn cel_ambient(cels: &[Cel], p: [f32; 3], off: u8) -> Option<[u8; 3]> {
        let c = &cels[find_cel(cels, p)?];
        Some(c.ambient[(off as usize).min(1)])
    }

/// `build_FindCel` (0x1c8cb0): the bbox walk (see `Sim::cel_at`).
pub fn find_cel(cels: &[Cel], p: [f32; 3]) -> Option<usize> {
        let mut best: Option<usize> = None;
        let mut best_vol = f32::INFINITY;
        let mut matches = 0;
        for (i, c) in cels.iter().enumerate() {
            let inside = c.min[0] <= p[0] && p[0] <= c.max[0] && c.min[1] <= p[1] && p[1] <= c.max[1] && c.min[2] <= p[2] && p[2] <= c.max[2];
            if !inside { continue; }
            matches += 1;
            let vol = (c.max[0] - c.min[0]) * (c.max[1] - c.min[1]) * (c.max[2] - c.min[2]);
            if vol < best_vol { best_vol = vol; best = Some(i); }
            if matches >= 0x40 { break; }
        }
        best
}

/// The `(tint × ambient) >> 8` bytes of `View_SetupRenderModes_Tweak` (0x1e5888) as floats — the VU1
/// `TAmbient` term (`psiSetTweakARGB` 0x124770: each byte / 255). White tint (the port's default) leaves the
/// bytes equal to the ambient bytes.
pub fn mul_bytes(a: [u8; 3]) -> [f32; 3] {
    let tint = [0xffu8; 3];
    [0, 1, 2].map(|k| ((tint[k] as u32 * a[k] as u32) >> 8) as f32 / 255.0)
}

/// The level's room cels: the main map's cel placements whose low 16 flag bits are 0xc023/0xc047
/// (`parseentity_fixup_entity` 0x1d00c0 fills the runtime cel's ambient/zone from the placement's params
/// for exactly these two flag words). See `Cel`. The room half of `parse_cel_graph` — the two MUST
/// agree; `Scene::cels` and the portal walk's `room_to_cell` are built in one pass.
pub fn parse_room_cels(level: &Level) -> Vec<Cel> {
    // The room half of parse_cel_graph (fix-perf-level): one pass builds Scene::cels and the portal
    // graph so the two cannot diverge.
    parse_cel_graph(level).0
}

/// A parsed room cel — the ambient-light source for 0x800 objects (`docs/atlas/cel.md`, `gs-leaf.md`).
/// Built from the main map's cel placements: flags & 0x8000 create a cel, and `parseentity_fixup_entity`
/// (0x1d00c0) fills the runtime cel from the placement's params when the low 16 flag bits are 0xc023/0xc047
/// (the two "room cel" flag words, both carrying the 0x40000 room-cel bit `build_FindCel` walks):
/// ambient RGB = params 0/1/2 low bytes (all zero → 0x3c each), the +1-byte-shifted second triple =
/// params 6/7/8 (0xc047 only), the lighting zone = param 5 (0xc047 only). The bbox is the object's glist
/// min/max copied raw into `cel+0x40..0x60` (`parsemap_block_map_data_static` 0x1d1998 + the
/// `parsemap_block_entity_params` 0x1d1508 record map) — the same box `build_FindCel` (0x1c8cb0) tests.
#[derive(Clone)]
pub struct Cel {
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// `base+0x9a/0x9c/0x9e` for base = cel + `switch_channels[zone]` with offset 0 (triple 0) and 1 (triple 1).
    pub ambient: [[u8; 3]; 2],
    /// `cel+0xa2`: the switch-channel index whose byte value shifts the ambient read by that many bytes
    /// (`base = cel + switch_channels[cel+0xa2]`, `Lights_CalcAmbientLight` 0x1cb178).
    pub zone: u8,
    /// `cel+0x90`: the water surface [M fix-ladder]. `build_alloc_cel` 0x1c93f8 sets it to −500.0 (no
    /// water); `parseentity_fixup_entity` 0x1d00c0 sets `cel+0x90 = cel+0x54` for flag word 0xC023 —
    /// `cel+0x54` is the glist bbox max y (`amemcpy(cel+0x40, glist+0x20, 0x20)` 0x1d1998 copies the bbox
    /// to cel+0x40..0x5F) — and to −500.0 for 0xC047. `Player_InWater` 0x195e50 and the swim substate
    /// clamp read it, and `Player_Collision` skips gravity below it.
    pub water: f32,
}

/// The runtime cel graph for the portal visibility walk (fix-perf-level, G-016's portal half).
///
/// `cells` = the level parse's pCelList record array stand-in [M cel.md]: one node per main-map
/// placement that creates a cel (`flags & 0x8000`), in placement order, with the object's glist bbox
/// (the box `build_FindCel` 0x1c8cb0 tests). `portals` = map block 0x21 with each endpoint remapped
/// into `cells` (the game matches `runtime_cel+0x38 == pCelList + idx*0x50` [M 0x1d15f8]; a portal
/// whose endpoint placement has no resolvable object is dropped). `room_to_cell` maps the port's room
/// cel index (a `parse_room_cels` / `Scene::cels` index) to its `cells` index.
#[derive(Clone)]
pub struct CelGraph {
    pub cells: Vec<CelNode>,
    pub portals: Vec<PortalEdge>,
    pub room_to_cell: Vec<Option<usize>>,
    /// Portal records dropped because an endpoint cel placement did not resolve.
    pub dropped_portals: usize,
}

#[derive(Clone, Copy)]
pub struct CelNode { pub min: [f32; 3], pub max: [f32; 3] }

/// One block-0x21 portal as two half-edges: `corners` in file order, the FORWARD plane built
/// `Plane_PlaneEq(plane, ref = corner 0, corner 1, corner 2)` [M 0x1d15f8] — its normal points into
/// celA, so the runtime node attached to celB (dest celA) carries it [M 0x1c8928] and the node
/// attached to celA carries its reverse. Side flags per record [M 0x1d15f8]: the celB node walks
/// while `(flags & 2) == 0`, the celA node while `(flags & 1) == 0`.
#[derive(Clone, Copy)]
pub struct PortalEdge {
    pub corners: [[f32; 3]; 4],
    pub n_fwd: [f32; 3],
    pub d_fwd: f32,
    pub cel_a: usize,
    pub cel_b: usize,
    pub flags: u16,
}

/// `Plane_PlaneEq` (0x1df8d0) [M]: n = normalize(cross(v1 − r, v2 − v1)), d = −dot(n, r); the zero
/// vector when the points are collinear (the game stores the zero plane and the walk's dot test then
/// never passes — the same effect here). Value form: `dot(n, p) + d`, zero on the plane.
pub fn plane_eq(r: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> [f32; 4] {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let e = sub(v2, v1);
    let cr = cross(sub(v1, r), e);
    let len = (cr[0] * cr[0] + cr[1] * cr[1] + cr[2] * cr[2]).sqrt();
    if len == 0.0 { return [0.0; 4]; }
    let n = [cr[0] / len, cr[1] / len, cr[2] / len];
    [n[0], n[1], n[2], -(n[0] * r[0] + n[1] * r[1] + n[2] * r[2])]
}

/// The level's cel graph, built alongside `parse_room_cels` (which is this function's room half —
/// the two MUST agree; `Scene::cels` is `.0`).
/// The level's cel graph, built alongside `parse_room_cels` (which is this function's room half —
/// the two MUST agree; `Scene::cels` is `.0`).
///
/// `cells` spans the WHOLE parse in archive order — the game's pCelList record array is parse-wide
/// (parsemap_block_map_data_static appends one 0x50-byte record per cel placement as each map is
/// parsed, sub-maps first, main map last), and the main map's portal block indexes THAT array.
/// *Measured* [D]: 07000002's main-map portal records point past the main map's own cel placements
/// (22 of 37 endpoints) — they address sub-map cels too.
/// The level's cel graph, built alongside `parse_room_cels` (which is this function's room half —
/// the two MUST agree; `Scene::cels` is `.0`).
///
/// The pCelList record index: `parsemap_block_map_data_static` (0x1d1998) addresses the record as
/// `pCelList + placement.index * 0x50` for by-index placements (the by-hash path goes through the
/// entity hashtable, untraced — those records are unreachable here and the portals naming them are
/// counted in `dropped_portals`). Records the port cannot resolve are the *sparse* remainder of the
/// game's array; only the named ones exist as nodes.
pub fn parse_cel_graph(level: &Level) -> (Vec<Cel>, CelGraph) {
    let main = level.main_map();
    let mut cells: Vec<CelNode> = Vec::new();
    let mut rooms: Vec<Cel> = Vec::new();
    let mut room_to_cell: Vec<Option<usize>> = Vec::new();
    let mut cell_of_record: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (mi, map) in level.maps.iter().enumerate() {
        for p in &map.placements {
            if !p.creates_cel() { continue; }
            let flags16 = (p.flags & 0xffff) as u16;
            let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v as u8).unwrap_or(0);
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < map.objects.len() { Some((mi, p.index as usize)) } else { None };
            let is_main = Some(mi) == main;
            let idx = cells.len();
            match obj {
                Some((om, oi)) => {
                    let o = &level.maps[om].objects[oi];
                    cells.push(CelNode { min: o.min, max: o.max });
                    if is_main && matches!(flags16, 0xc023 | 0xc047) {
                        let mut a0 = [param(0), param(1), param(2)];
                        if a0 == [0, 0, 0] { a0 = [0x3c; 3]; }
                        let a1 = if flags16 == 0xc047 {
                            let b = [param(6), param(7), param(8)];
                            if b == [0, 0, 0] { a0 } else { b }
                        } else { a0 };
                        rooms.push(Cel { min: o.min, max: o.max, ambient: [a0, a1], zone: if flags16 == 0xc047 { param(5) } else { 0 },
                            water: if flags16 == 0xc023 { o.max[1] } else { -500.0 } });
                        room_to_cell.push(Some(idx));
                    }
                }
                None => { cells.push(CelNode { min: [0.0; 3], max: [0.0; 3] }); }
            }
            // The record address is `pCelList + index*0x50` [M 0x1d1998] — by-index placements only.
            if !p.by_hash() { cell_of_record.insert(p.index as usize, idx); }
        }
    }
    let mut portals = Vec::new();
    let mut dropped = 0usize;
    for map in &level.maps {
        for q in &map.portals {
            let (Some(a), Some(b)) = (cell_of_record.get(&(q.cel_a as usize)).copied(), cell_of_record.get(&(q.cel_b as usize)).copied()) else {
                dropped += 1;
                continue;
            };
            // Forward plane: Plane_PlaneEq(plane, ref = c0, v1 = c1, v2 = c2) [M 0x1d15f8 — the
            // parse calls it over the corner array in order, then over the reversed copy for the
            // B side].
            let pl = plane_eq(q.corners[0], q.corners[1], q.corners[2]);
            portals.push(PortalEdge { corners: q.corners, n_fwd: [pl[0], pl[1], pl[2]], d_fwd: pl[3], cel_a: a, cel_b: b, flags: q.flags });
        }
    }
    (rooms, CelGraph { cells, portals, room_to_cell, dropped_portals: dropped })
}

/// One `ClipPortalWithPlane` (0x1e7fa0) pass: Sutherland–Hodgman keeping `value >= 0`, with the game's
/// compaction (a vertex is kept only when its successor is farther than √1e-7). `None` = every vertex
/// inside (the game's return 0 — the polygon passes through unchanged), `Some(v)` = the clipped
/// polygon (possibly empty = fully outside).
fn clip_positive(poly: &[[f32; 3]], plane: [f32; 4]) -> Option<Vec<[f32; 3]>> {
    let val = |p: &[f32; 3]| plane[0] * p[0] + plane[1] * p[1] + plane[2] * p[2] + plane[3];
    if !poly.iter().any(|p| val(p) < 0.0) { return None; }
    let n = poly.len();
    let mut out = Vec::with_capacity(n + 2);
    for i in 0..n {
        let (cur, nxt) = (poly[i], poly[(i + 1) % n]);
        let (cin, nin) = (val(&cur) >= 0.0, val(&nxt) >= 0.0);
        let denom = plane[0] * (cur[0] - nxt[0]) + plane[1] * (cur[1] - nxt[1]) + plane[2] * (cur[2] - nxt[2]);
        match (cin, nin) {
            (true, true) => out.push(nxt),
            (true, false) => {
                if denom != 0.0 {
                    let t = -val(&nxt) / denom;
                    out.push([nxt[0] + t * (cur[0] - nxt[0]), nxt[1] + t * (cur[1] - nxt[1]), nxt[2] + t * (cur[2] - nxt[2])]);
                }
            }
            (false, true) => {
                if denom != 0.0 {
                    let t = -val(&nxt) / denom;
                    out.push([nxt[0] + t * (cur[0] - nxt[0]), nxt[1] + t * (cur[1] - nxt[1]), nxt[2] + t * (cur[2] - nxt[2])]);
                }
                out.push(nxt);
            }
            (false, false) => {}
        }
    }
    // The game's compaction: keep vertex i only when its (wrapped) successor differs by more than
    // √1e-7 (Vec_SqDist3D over the out buffer, 0x1e82e0's copy-back loop).
    let n = out.len();
    let mut comp = Vec::with_capacity(n);
    for i in 0..n {
        let nx = if i + 1 != n { i + 1 } else { 0 };
        let d = [out[i][0] - out[nx][0], out[i][1] - out[nx][1], out[i][2] - out[nx][2]];
        if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > 1e-7 { comp.push(out[i]); }
    }
    Some(comp)
}

/// The game's portal visibility walk (`Vision_Portal_Recurse` 0x1e86e8 → `Vision_Recurse` 0x1e82e0),
/// producing the per-`cells`-index visibility the renderer gates static World draws with (the
/// `View_AddCels` GfxList set: only portal-visited cels are added).
///
/// `depth0`: the 5-plane view pyramid — `Vision_Calculate_View_Fustrum` (0x1e7df8) builds the eye's
/// four side planes plus the near plane at depth 0; the port passes the frame frustum's first five
/// Gribb-Hartmann planes (the same five faces; convention inside ≥ 0, the clip's keep side).
/// Returns None when the walk cannot run (no graph, the eye's room cel unmapped) — the caller keeps
/// its previous cull. Deviation: the game skips a portal whose linked door `Door_IsClosed` (0x1367b0)
/// [M 0x1e82e0:51]; the port has no per-portal door link yet, so every portal is treated open —
/// conservative (it can only keep cels visible, never hide them).
pub fn portal_visible_cels(graph: &CelGraph, eye_room: usize, eye: [f32; 3], depth0: &[[f32; 4]; 5]) -> Option<Vec<bool>> {
    let start = graph.room_to_cell.get(eye_room).copied().flatten()?;
    let mut visible = vec![false; graph.cells.len()];
    let mut visited = vec![false; graph.cells.len()];
    let mut planes: Vec<Vec<[f32; 4]>> = vec![Vec::new(); 48];
    planes[0] = depth0.to_vec();
    let mut budget = 0usize;
    // `Vision_Recurse` [M 0x1e82e0]: the recursion budget (VisionRecurseCount > 100) and the depth cap
    // (45) are the game's own guards; VisCelCount caps at 0x40 cels.
    fn recurse(
        graph: &CelGraph, eye: [f32; 3], cell: usize, depth: usize,
        visible: &mut Vec<bool>, visited: &mut Vec<bool>, planes: &mut Vec<Vec<[f32; 4]>>, budget: &mut usize,
    ) {
        *budget += 1;
        if *budget > 100 || depth > 45 { return; }
        let n0 = planes[depth].len();
        for p in &graph.portals {
            let (side_open, dest, fwd) = if cell == p.cel_a {
                ((p.flags & 1) == 0, p.cel_b, false)
            } else if cell == p.cel_b {
                ((p.flags & 2) == 0, p.cel_a, true)
            } else { continue };
            if !side_open { continue; }
            // The side's plane: the node attached to celB carries the forward plane, the celA node
            // its reverse [M 0x1c8928 / 0x1d15f8]. `Vision_Recurse` processes a portal only when the
            // eye is on its negative side [M 0x1e82e0:50].
            let (n, d) = if fwd { (p.n_fwd, p.d_fwd) } else { ([-p.n_fwd[0], -p.n_fwd[1], -p.n_fwd[2]], -p.d_fwd) };
            if n[0] * eye[0] + n[1] * eye[1] + n[2] * eye[2] + d >= 0.0 { continue; }
            // The quad as the runtime node stores it: forward corners on the celB node, reversed
            // (c3,c2,c1,c0) on the celA node [M 0x1c8928's amemcpy sources].
            let mut poly: Vec<[f32; 3]> = if fwd { p.corners.to_vec() } else { vec![p.corners[3], p.corners[2], p.corners[1], p.corners[0]] };
            for k in 0..n0 {
                match clip_positive(&poly, planes[depth][k]) {
                    None => {}
                    Some(clipped) => { poly = clipped; }
                }
                if poly.is_empty() { break; }
            }
            if poly.is_empty() { continue; }
            if !visited[dest] { visited[dest] = true; visible[dest] = true; }
            if poly.len() >= 3 {
                // The next depth's planes: one per clipped-polygon edge, ref = the eye (ViewPoints[0]
                // [M 0x1e82e0:146]), plus the portal's own plane last [M 0x1e82e0:151-155].
                let m = poly.len();
                let mut next: Vec<[f32; 4]> = (0..m).map(|i| plane_eq(eye, poly[i], poly[(i + 1) % m])).collect();
                next.push([n[0], n[1], n[2], d]);
                planes[depth + 1] = next;
                recurse(graph, eye, dest, depth + 1, visible, visited, planes, budget);
                if *budget > 100 { return; }
            }
        }
    }
    visited[start] = true;
    visible[start] = true;
    recurse(graph, eye, start, 0, &mut visible, &mut visited, &mut planes, &mut budget);
    Some(visible)
}

/// `Lights_CalcAmbientLight` (0x1cb178): the object's ambient RGB steps toward its cel's ambient through
/// `ClrStep(…, 0.1)` (0x1e8a48: `(target−cur)·rate`, at least ±1 byte per step, disassembled), snapping
/// once on the first cel link when still at the 0xff,ff,ff init sentinel (`control_link_object_to_cel`
/// 0x1346b8). Ported on the Sim's per-character state in `Sim::ramp_char_ambient`.
fn clr_step(cur: u8, target: u8) -> u8 {
    let d = (target as f32 - cur as f32) * 0.1;
    let d = if (target as f32 - cur as f32) == 0.0 { return cur }
        else if d.abs() < 1.0 { if d < 0.0 { -1.0 } else { 1.0 } } else { d };
    (cur as f32 + d).clamp(0.0, 255.0) as u8
}

impl Sim {
    /// `build_FindCel` (0x1c8cb0) over the parsed room cels: inclusive bbox test, one match wins, no match
    /// → None. More than one match: the game raycasts each matched cel's OWN object list — up from the point
    /// (256 max), then down limited by the up hit — and keeps the cel with the nearest hit; no hit → the
    /// first match (0x1c8cb0 disassembly). nf-view has no per-cel object lists, so the smallest-volume cel
    /// wins (*deviation*). fix-render2 re-measured this against every [L] snapshot drone position on
    /// 07000001 (ee_start_07000001.bin: 15 drones in 6 cels, several inside two-cel overlaps) and on
    /// 07000005: the smallest-volume pick reproduces the game's ambient at every one of them, while a
    /// shared-geometry raycast mis-attributes vertically-adjacent geometry (the courtyard arch belongs to
    /// the room cel's list, not the big overlapping box's). Fix-render2's first-guard report ("the first
    /// guard looks too dark") could not be reproduced at any [L] position with this tie-break.
    pub fn cel_at(&self, p: [f32; 3]) -> Option<usize> { find_cel(&self.cels, p) }
    /// The ambient triple a 0x800 object at `p` ramps toward: `base = cel + switch_channels[cel+0xa2]`,
    /// read at `base+0x9a/0x9c/0x9e` (`Lights_CalcAmbientLight` 0x1cb178). The port's live switch-channel
    /// table stands in for the game's; values above 1 clamp to 1 (only the 0/1 shift has defined bytes).
    pub fn ambient_at(&self, p: [f32; 3]) -> Option<[u8; 3]> {
        // fix-render3: a test-visible counter for the posed-vertices hoist (the per-vertex version
        // ran this through every pickup/attachment vertex; the hoisted one runs it once per key).
        self.ambient_lookups.set(self.ambient_lookups.get() + 1);
        let c = &self.cels[self.cel_at(p)?];
        let off = *self.drones.channels.v.get(c.zone as usize).unwrap_or(&0);
        Some(c.ambient[(off as usize).min(1)])
    }

    /// One `Lights_CalcAmbientLight` pass over the characters: the view models follow Bond's cel, drone-backed
    /// characters their drone's cel, script characters their entity matrix; characters with none keep the value
    /// snapped at build. The 0x10000 doubled targets (night vision, `glb_blokes+0x962`) are not ported — the
    /// goggles are not in the port yet, so the flag can never set.
    pub fn ramp_char_ambient(&mut self) {
        // Positions first (the borrow of `self` ends before the mutable ambient walk).
        let positions: Vec<Option<[f32; 3]>> = (0..self.char_ambient.len()).map(|ci| {
            if self.view_model_chars.contains(&ci) {
                Some(self.bond.sphere_centre())
            } else if let Some(di) = self.char_drone[ci] {
                let d = &self.drones.list[di];
                Some([d.pos[0], d.pos[1], d.pos[2]])
            } else {
                // Script characters: the live entity matrix, when the simulation is showing one.
                self.characters_script_pos(ci)
            }
        }).collect();
        let targets: Vec<Option<[u8; 3]>> = positions.iter().map(|p| p.and_then(|p| self.ambient_at(p))).collect();
        for (ci, amb) in self.char_ambient.iter_mut().enumerate() {
            let Some(target) = targets[ci] else { continue };
            if *amb == [0xff, 0xff, 0xff] {
                *amb = target; // snap on the first link (`control_link_object_to_cel` 0x1346b8)
            } else {
                for k in 0..3 { amb[k] = clr_step(amb[k], target[k]); }
            }
        }
    }

    /// The multiplier bytes of `View_SetupRenderModes_Tweak` (0x1e5888) for a 0x800 object:
    /// `(tint × ambient) >> 8` per channel, the VU1 `TAmbient` term as floats (`psiSetTweakARGB`
    /// 0x124770 → TAmbient = byte/255). The port's tint is the white default (`Script_SetColour` and the
    /// laser-dot object's 0x7f red are not ported), so the bytes equal the ambient bytes.
    pub fn char_multiplier(&self, ci: usize) -> [f32; 3] {
        mul_bytes(self.char_ambient.get(ci).copied().unwrap_or([0xff; 3]))
    }
}

impl Sim {
    /// One game tick with Bond at `camera` (eye): his object position on the collision floor below, aimed at chest
    /// height (stand-in for the game's player object and `Drone_View_Bones`).
    /// `trigger`: the fire button (action 9) held; `aiming`: aim mode (L1).
    pub fn step(&mut self, camera: [f32; 3], look: [f32; 3], trigger: bool, aiming: bool) {
        let floor = self.world.floor_below(camera, 4.0).unwrap_or(camera[1] - 1.6);
        self.first_person = false;
        let ground = [camera[0], floor + 0.4, camera[2]];
        self.run(ground, ground, camera, look, look, trigger, aiming, false, false);
    }

    /// One game tick with Bond walking (`Player_Update`): his input, then the world with him as the player. In aim
    /// mode the mouse turn and look (`input.turn` radians, `input.look` units of π/2) first move the cursor within the
    /// aim box (±0.296 across, ±0.4 up) and turn the view only past its edge (pad aim mode; the mouse has none, D-002); the shot goes through the smoothed
    /// cursor (`Player_GetAimingPoint`).
    pub fn step_bond(&mut self, mut input: nf_game::player::Input, trigger: bool, reload: bool, use_key: bool) {
        nf_game::prof::tick();
        let _prof = nf_game::prof::scope("step_bond (total)");
        self.shot_tick = false;
        let (trigger, reload) = if self.disabled { input = Default::default(); (false, false) } else { (trigger, reload) };
        let aiming = input.aiming;
        // Weapon switching (`Player_WeaponChange` 0x1a6f80): only when idle (not firing/reloading/holstering). The
        // switch request (`anim+0x63`) is set here; the holster/draw cycle runs in `run` (`Player_SetWeaponAnimObj`).
        // The weapon/gadget split: mouse wheel / Q cycles weapons (mode=0, 1..0x47); G cycles gadgets (mode=1,
        // 0x4A..0x5E); number keys select directly [M disassembly 0x1a707c].
        // The toggle-back: if cycling weapons from a gadget, jump to the last weapon; if cycling gadgets from a
        // weapon, jump to the last gadget [M 0x1a6f80 lines 66-121].
        // If the cycle finds nothing, `Player_WeaponNone` (0x1a9858) → hands (id 0x47), then `Player_GetBestWeapon`
        // (0x1a9680) scans the BestWeapon table for the first owned+has-ammo weapon.
        // `Player_WeaponChange` (0x1a6f80:33-47) refuses EVERY switch while the view-anim state is 9 (a fire
        // script playing) [M the state switch maps 9 → case 8: return] — the port approximates state 9 with its
        // fire-script gate (fix-player2: this is the half of the grenade bug that wedged the weapon after a
        // failed throw: the game plays the 26-frame throw script and blocks switching until it ends).
        // The refused set is {1, 3, 5, 6, 7, 8, 9, 0xb, 0xd, 0xe, 0xf, 0x10} — the reload states 5/6/7 are in
        // it: a reload always completes before a switch can start [M 0x1a6f80:33-47; fix-player3].
        let fire_busy = self.weapons.records.get(self.gun.weapon as usize).is_some_and(|w| {
            let rd = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
            let len = |h: u32| self.scripts.get(&h).map_or(0.0, |s| s.length as f32);
            let (fs, fl) = (rd(0xb0), rd(0xb4));
            (self.view_script == fs || (fl != 0 && self.view_script == fl)) && self.view_time < len(self.view_script) - 1.0
        });
        // The per-tick clip regen [M Player_Update 0x193600 disasm 0x193eb8-0x19405c]: while the view-anim
        // state is not 9 (the port's `fire_busy` gate), six clip slots refill +FRAME_RATE_MUL toward their
        // weapon's `weapon_data[w].clip_size` (+0x92): clip[0x4A] (+0x530) and clip[0x4E] (+0x560) and
        // clip[0x33] (+0x41C) on `frame & 3 == 0`; clip[0x4C] (+0x548) and clip[0x4F] (+0x56C) on
        // `frame & 1 == 0`; clip[0x45] (+0x4F4, the grenade belt) on `frame % 6 == 0`. No ownership gate.
        if !self.disabled && !fire_busy {
            let frame = self.drones.frame;
            let gates: [(usize, bool); 6] = [
                (0x4A, frame & 3 == 0), (0x4E, frame & 3 == 0), (0x33, frame & 3 == 0),
                (0x4C, frame & 1 == 0), (0x4F, frame & 1 == 0), (0x45, frame % 6 == 0),
            ];
            for (w, on) in gates {
                if !on { continue; }
                let cap = self.weapons.records.get(w).map_or(0, |rec| u16::from_le_bytes([rec[0x92], rec[0x93]]) as i16);
                if let Some(c) = self.drones.inventory.clip.get_mut(w) {
                    if *c < cap { *c += 1; }
                    if w == self.gun.weapon as usize { self.gun.clip = (*c).max(0) as u32; }
                }
            }
        }
        if self.gun.switch_state == 0 && self.gun.reload_state == 0 && !fire_busy {
            let owned = self.drones.inventory.owned.clone();
            let carried = self.drones.inventory.carried.clone();
            let clip = self.drones.inventory.clip.clone();
            let weapons = &self.weapons;
            let ammo_type_of = |w: u8| weapons.ammo_type(w as usize);
            let flags_of = |w: u8| weapons.flags(w as usize);
            let has_ammo = |w: u8| nf_game::gun::Gun::has_ammo(w, &carried, &clip, ammo_type_of, flags_of);
            let selectable = |w: u8| weapons.selectable(w);
            // D-003 REMOVED: the root cause was the missing `Player_Init` override of weapon_data[1]+0xDC
            // (the fists skin). With the override applied in main.rs, the fists skin (0x050000ab) is found
            // in the level archive, the view model is built, and the cycle can land on fists — matching the
            // game. The `has_view_model` filter is no longer needed.
            let target = if let Some(id) = input.switch_to {
                if id != self.gun.weapon && owned.get(id as usize).copied().unwrap_or(false)
                    && weapons.first_person(id as usize) != 0 { Some(id) } else { None }
            } else if input.weapon_cycle != 0 {
                self.gun.toggle_back(nf_game::gun::CycleMode::Weapons, &owned, has_ammo, selectable)
                    .or_else(|| self.gun.cycle(input.weapon_cycle, nf_game::gun::CycleMode::Weapons, &owned, has_ammo, selectable))
            } else if input.gadget_cycle != 0 {
                self.gun.toggle_back(nf_game::gun::CycleMode::Gadgets, &owned, has_ammo, selectable)
                    .or_else(|| self.gun.cycle(input.gadget_cycle, nf_game::gun::CycleMode::Gadgets, &owned, has_ammo, selectable))
            } else { None };
            if let Some(id) = target {
                // Update prev_weapon/prev_gadget [M 0x1a6f80 lines 134-141]: if current is a weapon, save to
                // prev_weapon; if current is a gadget, save to prev_gadget. Special: 0x55 ('U') saves as 0x54 ('T').
                let cur = self.gun.weapon;
                if cur >= 1 && cur <= 0x47 { self.gun.prev_weapon = cur; }
                if cur >= 0x4a && cur <= 0x5e { self.gun.prev_gadget = if cur == 0x55 { 0x54 } else { cur }; }
                self.gun.switch_request = id;
                if std::env::var_os("NF_SOUNDLOG").is_some() {
                    eprintln!("[weapon-switch] tick {}: requesting switch from {:#x} to {:#x}", self.drones.frame, self.gun.weapon, id);
                }
            } else if input.weapon_cycle != 0 || input.gadget_cycle != 0 {
                // The cycle found nothing: go to hands, then try GetBestWeapon [M Player_WeaponNone + Player_GetBestWeapon].
                const BEST_WEAPON: [u16; 18] = [0x42, 0x32, 0x1a, 0x18, 0x1c, 0x16, 0x15, 0x08, 0x10, 0x0e, 0x0d, 0x0c, 0x12, 0x06, 0x04, 0x02, 0x0a, 0x01];
                self.gun.go_to_hands();
                let base_of = |w: u8| self.weapons.base(w as usize) as u8;
                if let Some(id) = self.gun.get_best_weapon(&BEST_WEAPON, base_of, &owned, has_ammo) {
                    let cur = self.gun.weapon;
                    if cur >= 1 && cur <= 0x47 { self.gun.prev_weapon = cur; }
                    if cur >= 0x4a && cur <= 0x5e { self.gun.prev_gadget = if cur == 0x55 { 0x54 } else { cur }; }
                    self.gun.switch_request = id;
                    if std::env::var_os("NF_SOUNDLOG").is_some() {
                        eprintln!("[weapon-switch] tick {}: hands → GetBestWeapon → {:#x}", self.drones.frame, id);
                    }
                }
            }
        }
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
        // Riding (`Player_FeetOnPoint` → `Control_InheritVelocity` 0x133e88): standing on a moving object (the truck),
        // Bond moves and turns with it — the feet ray reaches cc + 0.6 below him (cc 1.0347 [L]). In the game this runs
        // inside `Player_CollisionHandler` (0x1949a8), which early-returns when `BLData+0x94a == 0` (disabled by a
        // script cut-scene). So riding does NOT run while Bond is disabled — the cut-scene's `Script_Free` →
        // `Player_Enable` places Bond directly. The A-5 change (running riding unconditionally) caused Bond to bounce
        // against the truck during the cut-scene. Fix: gate on `!self.disabled` [M 0x1949a8:24].
        if !self.disabled {
            let p = self.bond.pos;
            if let Some(g) = self.world.ground_motion(p, [p[0], p[1] - 1.6347, p[2]]) {
                let (sn, cs) = g.turn.sin_cos();
                let (x, z) = (p[0] + g.delta[0] - g.pivot[0], p[2] + g.delta[2] - g.pivot[2]);
                self.bond.pos = [g.pivot[0] + x * cs + z * sn, p[1] + g.delta[1], g.pivot[2] - x * sn + z * cs];
                self.bond.yaw += g.turn;
            }
        }
        let before = self.bond.pos;
        // `Player_Update` 0x193600 with the ladder/water context (fix-ladder): the level's ladders
        // (BLData+0x884) and the water surface of the cel Bond is in (cel+0x90; −500 = none, and a cel
        // whose top is far below Bond is no water either). The sink control is unused in play (the
        // viewer's input has no bind; the crouch key sinks in the Sim path below).
        if !self.disabled {
            let water = self.cel_at(self.bond.pos).map(|i| self.cels[i].water).filter(|w| *w > -400.0);
            let sink = input.crouch;
            let ctx = nf_game::player::LadderCtx { ladders: &self.ladders, wires: &self.wires, wire_anims: Some(&self.wire_anims), water, sink };
            // The wire grab scan [M Player_DealWithObjHit 0x194458: the collision pass's class-0x2C hit
            // dispatches to Player_CollWire 0x197130; Player_HandleJump 0x1916b0 adds the jump-grab (the
            // nearest class-0x2C scan, 0x1916b0:118-147)]. The game's hit test for a dynamic object is
            // its world sphere vs the player's swept segment (Collide_PickObj 0x1eae80's version-0x200
            // branch); the port scans the level's wires with the capsule segment while Bond walks. The
            // use key runs the same scan [S — the PC action key; the game's jump/action button drives
            // HandleJump's jump-grab, whose icon context (+0x95F == 3) is not ported]. Health guard [M
            // CollWire refuses at +0x894 <= 0].
            if self.bond.substate == nf_game::player::SS_WALK && self.bond.regrab == 0 && self.drones.health.health > 0.0 {
                let (ca, cb) = self.bond.capsule_seg();
                if let Some(wi) = self.wires.iter().position(|w| w.sphere_hits(ca, cb)) {
                    self.bond.grab_wire(wi, &ctx);
                }
            }
            let was_on_wire = matches!(self.bond.substate, nf_game::player::SS_WIRE | nf_game::player::SS_ZIPLINE);
            let was_climbing = self.bond.substate == nf_game::player::SS_CLIMB;
            self.bond.tick_ctx(&self.world, &ctx, &input);
            // `Player_WeaponNone` 0x1a9858 runs in `Player_CollLadder` on the grab [M 0x196118]: the
            // current weapon id is SAVED to `anim+0x64` (the port's `saved_weapon`), the class becomes
            // 'G' (0x47) immediately (no holster cycle — `anim+0xF4 = 0`), the aim bit clears.
            if !was_climbing && self.bond.substate == nf_game::player::SS_CLIMB && self.gun.weapon != 0x47 {
                self.gun.go_to_hands();
            }
            // `Player_WeaponNone` 0x1a9858 also runs in `Player_CollWire` on the grab [M 0x1974e4].
            if !was_on_wire && matches!(self.bond.substate, nf_game::player::SS_WIRE | nf_game::player::SS_ZIPLINE) && self.gun.weapon != 0x47 {
                self.gun.go_to_hands();
            }
            // The dismount carry [M Player_Climb 0x196310:99-102]: `if (AnimObj+0x62 == 'G')
            // AnimObj[99] = AnimObj[100]` — the selected id becomes the saved weapon, and
            // `Player_SetWeaponAnimObj`'s idle path re-runs `Player_WeaponSelect` [M 0x1a2e38:197-201]:
            // the saved weapon is re-equipped on dismount (fix-ladder2: Bond stayed on hands).
            if was_climbing && self.bond.substate == nf_game::player::SS_WALK && self.gun.weapon == 0x47
                && self.gun.saved_weapon != 0x47 && self.gun.saved_weapon != self.gun.weapon {
                self.gun.switch_request = self.gun.saved_weapon;
            }
            // The wire drops write `AnimObj[99] = AnimObj[100]` [M Player_Wire 0x1978bc, Player_Zipline
            // 0x19808c] — the same re-equip carry the climb dismount uses.
            if was_on_wire && self.bond.substate == nf_game::player::SS_WALK && self.gun.weapon == 0x47
                && self.gun.saved_weapon != 0x47 && self.gun.saved_weapon != self.gun.weapon {
                self.gun.switch_request = self.gun.saved_weapon;
            }
            // `Player_Wire`'s per-tick effects outside Bond [M 0x1976b4-0x1976d4]: `Music_Event(0xD, 1)`
            // every tick on the wire (the music jumps to the stealth section) and the wire's switch
            // channel set 1 while Bond is on it, 0 on drop.
            let on_wire = matches!(self.bond.substate, nf_game::player::SS_WIRE | nf_game::player::SS_ZIPLINE);
            self.creeping = on_wire;
            if on_wire {
                self.drones.music.event(0xd, 1);
                if let Some(ch) = self.bond.wire.and_then(|wi| self.wires.get(wi)).map(|w| w.channel).filter(|c| *c != 0) {
                    self.drones.channels.set(ch as u32, 1, self.drones.frame);
                    self.wire_channel = ch;
                }
            } else if self.wire_channel != 0 {
                self.drones.channels.set(self.wire_channel as u32, 0, self.drones.frame);
                self.wire_channel = 0;
            }
            // `Player_MonitorAir` 0x191a70's drown damage: `Player_HandlePain(0.5, type 7)` [M 0x191a70:136-143]
            // — the FULL pipeline (difficulty factor, no armour for type 7 [M 0x1902c8:84-104], the pain
            // flash +0x968 and the both-sides bits +0x967 = 0xC), not a bare subtraction.
            if let Some(dmg) = self.bond.drown_damage {
                self.drones.health.pain_generic(dmg, nf_game::drone::DIFFICULTY, self.level, 7);
            }
        }
        // Footsteps (`AnimObjectUpdate` 0x1befd8:163-198 [M], player): `BL+0x924` gathers the distance
        // moved (`+= |anim+0xD8|`); past the stride it wraps (fmod) and a step sounds. The stride is
        // 1.65 — EXCEPT while the CLIMB substate (1) runs, where it is 0.4125 and the step is the fixed
        // `Sound_Play3D(0x286)` at Bond (the rung sound); every other step is `Effect_Create(…, foot|0x42)`
        // → the floor surface's footstep (`EffectInfo+0x16`) at the floor point, AI alertness 0
        // (`Sound_SetAlertness(0)` [M]). fix-ladder2: the port had the 0.4125/0x286 pair on CROUCH — the
        // user's "rapid footsteps on metal" while crouch-walking. The game's crouch steps run the normal
        // 1.65 surface path; the crouch anim's smaller per-tick displacement slows them naturally.
        // *Suggested*: the distance is Bond's movement per tick (horizontal; 3D while climbing, where the
        // game's `anim+0xD8` follows the climbing anim up the rungs).
        let (dx, dy, dz) = (self.bond.pos[0] - before[0], self.bond.pos[1] - before[1], self.bond.pos[2] - before[2]);
        let moved_h = (dx * dx + dz * dz).sqrt();
        let climbing = self.bond.substate == nf_game::player::SS_CLIMB;
        let moved = if climbing { (moved_h * moved_h + dy * dy).sqrt() } else { moved_h };
        if self.bond.grounded && !self.disabled {
            self.step_dist += moved;
            let stride = if climbing { 0.4125 } else { 1.65 };
            if self.step_dist > stride {
                self.step_dist %= stride;
                let p = self.bond.pos;
                if climbing { self.drones.audio.play3d(0x286, p, None); }
                else if let (Some(s), Some((t, _, m))) = (self.surfaces.as_ref(), self.world.segment_surface(p, [p[0], p[1] - 1.6347, p[2]])) {
                    if let Some(id) = nf_formats::sfx::SurfaceSounds::get(&s.footstep, m) { self.drones.audio.play3d(id, [p[0], p[1] - 1.6347 * t, p[2]], None); }
                }
            }
        }
        // The bob phases: `BL+0x8DC` += the forward speed (`BL+0x28`), `BL+0x8E8` += (π/2) / 60.
        self.bob[0] += self.bond.local[1];
        self.bob[1] += std::f32::consts::FRAC_PI_2 / 60.0;
        // The landing damage: `Player_CollisionHandler` 0x1949a8:117-126 calls `Player_HandlePain(timer - 60,
        // type 6)` — the full pipeline: the difficulty factor, NO armour (types 1..7 bypass it [M
        // 0x1902c8:84-104]), the pain flash +0x968 and the vertical direction bits +0x967 = 2.
        if let Some(dmg) = self.bond.fall_damage { self.drones.health.pain_generic(dmg, nf_game::drone::DIFFICULTY, self.level, 6); }
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
        // The pickup take test's LOS origin: the player's world-sphere centre (`obj+0x80`), not his position
        // [M 0x18e148:212-214] — `Bond::sphere_centre` ports `Control_BuildWorldSph`'s player branch.
        self.run(self.bond.pos, self.bond.sphere_centre(), head, look, aim, trigger, aiming, reload, use_key);
        // `Player_Zoom` (0x1a1a70): the scope zoom. Right mouse (mouse-first) zooms in; releasing returns to 1.0.
        // The zoom interpolates toward the target at the game's rate (1.025× per tick zooming in, 1.15× out).
        // The FOV is divided by the zoom factor. Scoped weapons only (`weapon_data+0x68 & 0x40`).
        let scoped = self.weapons.records.get(self.gun.weapon as usize).is_some_and(|w| u32::from_le_bytes(w[0x68..0x6c].try_into().unwrap()) & 0x40 != 0);
        let max_zoom = self.weapons.zoom(self.gun.weapon as usize);
        if scoped {
            if input.zoom { self.zoom_target = max_zoom; } else { self.zoom_target = 1.0; }
        } else {
            self.zoom_target = 1.0;
        }
        // Interpolate zoom toward target [M 0x1a1a70 lines 22-39].
        // `Player_Zoom` (0x1a1a70): zoom in at 1.025×/tick, zoom out at 1.15×/tick [M lines 22-39].
        // The game's BLData+0x8D4 is the inverse (decreases as zoom increases); the port's `zoom` is the FOV
        // divisor (increases as zoom increases), so the interpolation factor is the complement.
        let rate = if self.zoom < self.zoom_target { 1.0 + 0.025 } else { 1.0 + 0.15 };
        self.zoom = self.zoom + (self.zoom_target - self.zoom) * (1.0 - 1.0 / rate);
        self.zoom = self.zoom.clamp(1.0, max_zoom.max(1.0));
        // Wire / creep: the music event moved up with the substate effects (fix-wire) — the substate is
        // now the only source (the old proximity hack is gone).
        let _ = &self.wires;
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

    fn run(&mut self, pos: [f32; 3], sph: [f32; 3], camera: [f32; 3], look: [f32; 3], aim: [f32; 3], trigger: bool, aiming: bool, reload: bool, use_key: bool) {
        // The drones aim at and look for Bond's chest (`Drone_View_Bones`; one point here, 0.45 under the eye).
        let hanging = matches!(self.bond.substate, nf_game::player::SS_WIRE | nf_game::player::SS_ZIPLINE);
        let player = nf_game::drone::Player { pos, target: [camera[0], camera[1] - 0.45, camera[2]], eye: camera, aim: look, sph, hanging };
        let d = nf_game::drone::Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.level, sequences: &self.seqs, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: self.surfaces.as_ref(), voices: self.voices.as_deref() };
        // `Player_WeaponFiring` → `Player_WeaponInitBullet` from the eye along the view (*deviation*: no auto-aim and
        // no aim-point ray yet); the first-person fire script's sound 0x403 feeds the drones' hearing.
        // The gun's clip and spare rounds live in Bond's inventory (BLData `+0x1B8`, `+0x170`), which pickups fill.
        let (wi, t) = (self.gun.weapon as usize, self.weapons.ammo_type(self.gun.weapon as usize));
        if let (Some(&c), Some(&n)) = (self.drones.inventory.clip.get(wi), self.drones.inventory.carried.get(t)) { self.gun.clip = c.max(0) as u32; self.gun.spare = n.max(0) as u32; }

        // Weapon switch state machine (`Player_SetWeaponAnimObj` 0x1a2e38 + `Player_WeaponSelect` 0x1a2cc0):
        // the holster/draw cycle. The holster script is `weapon+0xC4` (played at 1.25x), the draw script is
        // `weapon+0xC0` (1.25x), the variant-swap script is `weapon+0xC8` (1.3x for same-base swaps) [M disassembly].
        // State 0 (idle): a switch request starts the holster. State 1 (holstering): the holster script plays; when
        // done, swap the weapon byte and start the draw. State 3 (drawing): the draw script plays; when done, back to idle.
        // State 8 (variant swap): the variant-swap script plays; when done, back to idle.
        // While switching, the trigger and reload are blocked (the game's state 1/3 checks `AnimScriptIsStopped`).
        let log_switch = std::env::var_os("NF_SOUNDLOG").is_some();
        let mut switching = self.gun.switch_state != 0;
        // State 0 → 1/3/8: a switch request starts the holster (`Player_WeaponSelect` 0x1a2cc0).
        // The reload states block it here too (the game's refused view-state set {1,3,5,6,7,8,9,0xb,
        // 0xd..0x10} covers 5/6/7): a pending request simply waits for the reload to finish [M 0x1a6f80:33-47].
        if self.gun.switch_state == 0 && self.gun.reload_state == 0 && self.gun.switch_request != self.gun.weapon {
            let cur_id = self.gun.weapon;
            let req_id = self.gun.switch_request;
            let cur_base = self.weapons.base(cur_id as usize);
            let req_base = self.weapons.base(req_id as usize);
            // `Player_WeaponSelect` variant-swap path: same base weapon AND same first-person model → play +0xC8
            // at 1.3x, state 8 (a fast swap without holster/draw) [M 0x1a2cc0 lines 13-29].
            if cur_base == req_base && self.weapons.first_person(cur_id as usize) == self.weapons.first_person(req_id as usize) {
                let variant = self.weapons.variant_swap_script(req_id as usize);
                if variant != 0 {
                    if log_switch { eprintln!("[weapon-switch] tick {}: state 0→8 variant-swap {:08x} (weapon {:#x}→{:#x}, same base {:#x})", self.drones.frame, variant, cur_id, req_id, cur_base); }
                    self.view_script = variant;
                    (self.view_time, self.view_speed) = (0.0, 1.3);
                    self.gun.switch_state = 8;
                    switching = true;
                    // Swap the weapon byte now (the variant-swap doesn't holster).
                    if let Some(c) = self.drones.inventory.clip.get_mut(wi) { *c = self.gun.clip as i16; }
                    if let Some(n) = self.drones.inventory.carried.get_mut(t) { *n = self.gun.spare as i16; }
                    self.gun.prev_weapon = self.gun.weapon;
                    self.gun.weapon = self.gun.switch_request;
                    let (nwi, nt) = (self.gun.weapon as usize, self.weapons.ammo_type(self.gun.weapon as usize));
                    if let (Some(&c), Some(&n)) = (self.drones.inventory.clip.get(nwi), self.drones.inventory.carried.get(nt)) { self.gun.clip = c.max(0) as u32; self.gun.spare = n.max(0) as u32; }
                } else {
                    // Same base but no variant script: instant swap to idle.
                    if log_switch { eprintln!("[weapon-switch] tick {}: instant variant swap (no script) {:#x}→{:#x}", self.drones.frame, cur_id, req_id); }
                    if let Some(c) = self.drones.inventory.clip.get_mut(wi) { *c = self.gun.clip as i16; }
                    if let Some(n) = self.drones.inventory.carried.get_mut(t) { *n = self.gun.spare as i16; }
                    self.gun.prev_weapon = self.gun.weapon;
                    self.gun.weapon = self.gun.switch_request;
                    let (nwi, nt) = (self.gun.weapon as usize, self.weapons.ammo_type(self.gun.weapon as usize));
                    if let (Some(&c), Some(&n)) = (self.drones.inventory.clip.get(nwi), self.drones.inventory.carried.get(nt)) { self.gun.clip = c.max(0) as u32; self.gun.spare = n.max(0) as u32; }
                    self.view_script = self.weapons.idle_script(nwi);
                    (self.view_time, self.view_speed) = (0.0, 1.0);
                }
            } else {
                // Different base: holster → draw. `Player_WeaponSelect` checks +0xC4 (holster) [M 0x1a2cc0 line 33].
                let holster = self.weapons.holster_script(cur_id as usize);
                if holster != 0 {
                    if log_switch { eprintln!("[weapon-switch] tick {}: state 0→1 holster {:08x} (weapon {:#x})", self.drones.frame, holster, cur_id); }
                    self.view_script = holster;
                    (self.view_time, self.view_speed) = (0.0, 1.25);
                    self.gun.switch_state = 1;
                    switching = true;
                    // Sound 0x280 (weapon) or 0x283 (gadget, id > 0x4A) [M 0x1a2e38 case 1].
                    let s = if cur_id >= 0x4a && cur_id <= 0x5e { 0x280 } else { 0x283 };  // gadgets 0x280, weapons 0x283 [M disasm 0x1a3704]
                    self.drones.audio.play(s);
                } else {
                    // No holster script: swap instantly, go to draw (state 3) [M 0x1a2cc0 line 34-35].
                    if log_switch { eprintln!("[weapon-switch] tick {}: instant swap (no holster) {:#x}→{:#x}", self.drones.frame, cur_id, req_id); }
                    if let Some(c) = self.drones.inventory.clip.get_mut(wi) { *c = self.gun.clip as i16; }
                    if let Some(n) = self.drones.inventory.carried.get_mut(t) { *n = self.gun.spare as i16; }
                    self.gun.prev_weapon = self.gun.weapon;
                    self.gun.weapon = self.gun.switch_request;
                    let (nwi, nt) = (self.gun.weapon as usize, self.weapons.ammo_type(self.gun.weapon as usize));
                    if let (Some(&c), Some(&n)) = (self.drones.inventory.clip.get(nwi), self.drones.inventory.carried.get(nt)) { self.gun.clip = c.max(0) as u32; self.gun.spare = n.max(0) as u32; }
                    let draw = self.weapons.draw_script(nwi);
                    if draw != 0 {
                        if log_switch { eprintln!("[weapon-switch] tick {}: state 0→3 draw {:08x} (weapon {:#x})", self.drones.frame, draw, req_id); }
                        self.view_script = draw;
                        (self.view_time, self.view_speed) = (0.0, 1.25);
                        self.gun.switch_state = 3;
                        switching = true;
                        let s = if self.gun.weapon >= 0x4a && self.gun.weapon <= 0x5e { 0x281 } else { 0x282 };  // gadgets 0x281, weapons 0x282 [M disasm 0x1a385c]
                        self.drones.audio.play(s);
                    } else {
                        self.view_script = self.weapons.idle_script(nwi);
                        (self.view_time, self.view_speed) = (0.0, 1.0);
                    }
                }
            }
        }
        if switching {
            let cur = self.gun.weapon as usize;
            let len_of = |h: u32| self.scripts.get(&h).map_or(0.0, |s| s.length as f32);
            match self.gun.switch_state {
                1 => {
                    // Holstering: wait for the holster script (+0xC4) to finish [M case 1].
                    let holster = self.weapons.holster_script(cur);
                    if holster == 0 || self.view_time >= len_of(holster).max(1.0) - 1.0 {
                        // Holster done: write the current weapon's clip/spare back, swap, read the new weapon's.
                        if let Some(c) = self.drones.inventory.clip.get_mut(wi) { *c = self.gun.clip as i16; }
                        if let Some(n) = self.drones.inventory.carried.get_mut(t) { *n = self.gun.spare as i16; }
                        self.gun.prev_weapon = self.gun.weapon;
                        self.gun.weapon = self.gun.switch_request;
                        let (nwi, nt) = (self.gun.weapon as usize, self.weapons.ammo_type(self.gun.weapon as usize));
                        if let (Some(&c), Some(&n)) = (self.drones.inventory.clip.get(nwi), self.drones.inventory.carried.get(nt)) { self.gun.clip = c.max(0) as u32; self.gun.spare = n.max(0) as u32; }
                        // Start the draw script (+0xC0) at 1.25x [M case 2 → case 3].
                        let draw = self.weapons.draw_script(nwi);
                        if draw != 0 {
                            if log_switch { eprintln!("[weapon-switch] tick {}: state 1→3 draw {:08x} (weapon {:#x})", self.drones.frame, draw, self.gun.weapon); }
                            self.view_script = draw;
                            (self.view_time, self.view_speed) = (0.0, 1.25);
                            self.gun.switch_state = 3;
                        } else {
                            if log_switch { eprintln!("[weapon-switch] tick {}: state 1→0 idle (no draw script) {:#x}", self.drones.frame, self.gun.weapon); }
                            self.view_script = self.weapons.idle_script(nwi);
                            (self.view_time, self.view_speed) = (0.0, 1.0);
                            self.gun.switch_state = 0;
                        }
                        // Sound 0x281 (weapon) or 0x282 (gadget, id > 0x4A) [M 0x1a2e38 case 3].
                        let s = if self.gun.weapon >= 0x4a && self.gun.weapon <= 0x5e { 0x281 } else { 0x282 };  // gadgets 0x281, weapons 0x282 [M disasm 0x1a385c]
                        self.drones.audio.play(s);
                    }
                }
                8 => {
                    // Variant swap: wait for +0xC8 to finish [M case 8].
                    let variant = self.weapons.variant_swap_script(cur);
                    if variant == 0 || self.view_time >= len_of(variant).max(1.0) - 1.0 {
                        if log_switch { eprintln!("[weapon-switch] tick {}: state 8→0 idle (variant done) {:#x}", self.drones.frame, self.gun.weapon); }
                        self.view_script = self.weapons.idle_script(cur);
                        (self.view_time, self.view_speed) = (0.0, 1.0);
                        self.gun.switch_state = 0;
                    }
                }
                3 => {
                    // Drawing: wait for the draw script (+0xC0) to finish [M case 3].
                    let draw = self.weapons.draw_script(self.gun.weapon as usize);
                    if draw == 0 || self.view_time >= len_of(draw).max(1.0) - 1.0 {
                        // Draw done: back to idle [M case 4]. The game zeroes the refire timer here
                        // (`*(float*)(BLData+0x92C) = 0`, case 4 0x1a2e38:316) — without this, a grenade's
                        // 900-tick refire interval poisons every weapon after switching (fix-player2).
                        self.gun.timer = 0.0;
                        if log_switch { eprintln!("[weapon-switch] tick {}: state 3→0 idle (draw done) {:#x}", self.drones.frame, self.gun.weapon); }
                        self.view_script = self.weapons.idle_script(self.gun.weapon as usize);
                        (self.view_time, self.view_speed) = (0.0, 1.0);
                        self.gun.switch_state = 0;
                    }
                }
                _ => {}
            }
            // Advance the view script while switching.
            let len = len_of(self.view_script);
            self.view_time = (self.view_time + self.view_speed).min((len - 1.0).max(0.0));
        }

        let can_fire = !switching;
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
            let was_reloading = self.gun.reload_state;
            let rd0 = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
            // The gun's animation is busy while a fire script plays (gun state 9; idle = 0).
            let len_of = |h: u32| self.scripts.get(&h).map_or(0.0, |s| s.length as f32);
            let busy = (self.view_script == rd0(0xb0) || self.view_script == rd0(0xb4)) && self.view_time < len_of(self.view_script) - 1.0;
            // Block the trigger and reload while the holster/draw cycle plays.
            // The hands class 'G' (0x47) cannot fire: its record carries no fire scripts (+0xB0/+0xB4 = 0
            // [D probe_ladder2]; also no idle +0xA0) and the round spawn runs from the state-9 fire-script
            // release path — `Player_SetFiringAnim` 0x1a3c70 queues the script, case 9 0x1a2e38:416-420
            // spawns on its release frame; the press-tick spawn for +0x44 == 0 sits behind that call
            // [M 0x1a5460:681-690]. Fists (0x01) keeps its fire script 0x060006de [D] — punching is
            // unaffected (fix-ladder2: the hands state fired once).
            let has_fire_script = rd0(0xb0) != 0 || rd0(0xb4) != 0;
            // The game's general fire branch has NO fire-script gate — `Player_RoundToFire`
            // (0x1a18e0:9-25) rounds to fire unconditionally for ammo_type 0 and checks only the
            // ammo pool otherwise [M]. The gate here stands in for the hands class 'G' (0x47: no
            // fire scripts and no idle anim — fix-ladder2); every other script-less weapon fires
            // and spawns on the press tick (+0x44 == 0 [M 0x1a5460:688-693]) — the camera gadget
            // 0x54 (idle 0x060005A6, no fire scripts [D probe]) photographs through this path.
            let hands = self.gun.weapon == 0x47;
            let (trigger, reload) = if can_fire && (has_fire_script || !hands) { (trigger, reload) } else { (false, reload) };
            let fired = self.gun.update_with(trigger, reload, busy, params);
            // Advance the reload state machine [M Player_SetWeaponAnimObj cases 5/6/7].
            let new_reload_state = self.gun.reload_tick(trigger, params);
            let (prev_script, prev_time) = (self.view_script, self.view_time);
            let rd = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
            let flags = rd(0x68);
            let fire_anim_len = rd(0x44);
            if fired {
                // `Player_SetFiringAnim` (0x1a3c70), via `Gun::firing_anim_script`: the last-round pick
                // (flag 0x1000 / base 0x11), the fists' Rand(100) left/right alternation (flag 0x80000)
                // and the 0x40000 toggle. The game only queues when the fp model and the script exist
                // (0x1a3c70:51-54); with no script the view stays as it was.
                let script = self.gun.firing_anim_script(flags, u16::from_le_bytes([w[2], w[3]]), self.gun.clip, rd(0xb0), rd(0xb4), &mut self.drones.rng);
                if script != 0 {
                    self.view_script = script;
                    (self.view_time, self.view_speed) = (0.0, 1.0);
                }
            } else if self.gun.reload_state != 0 && was_reloading == 0 {
                // Reload just started [M Player_WeaponFiring's shared tail 0x1a6250-0x1a631c]: state 6
                // queues +0xA4 at 1.35× (`AnimScriptAddSpeed`, disasm 0x1a62d0), starting at frame 6 for
                // flag-0x1000 weapons (AnimListBuild +0x90 = 6.0; the port's script frames are time+1, so
                // view_time 5.0). State 5 (a +0xA8 start script) plays at 1.0 [M 0x1a6260-0x1a626c].
                if self.gun.reload_state == 5 {
                    self.view_script = rd(0xa8);
                    (self.view_time, self.view_speed) = (0.0, 1.0);
                } else {
                    self.view_script = rd(0xa4);
                    (self.view_time, self.view_speed) = (if flags & 0x1000 != 0 { 5.0 } else { 0.0 }, 1.35);
                }
            } else if self.gun.reload_loop_restarted {
                // The shell loop continued: restart +0xA4 from frame 0 (matching `AnimScriptAdd(+0xA4)` in
                // the game's case 6) [M 0x1a2e38].
                self.view_script = rd(0xa4);
                (self.view_time, self.view_speed) = (0.0, 1.0);
            } else if self.gun.reload_state != was_reloading {
                // Reload state changed: update the view script.
                match self.gun.reload_state {
                    5 => { self.view_script = rd(0xa8); (self.view_time, self.view_speed) = (0.0, 1.0); }
                    6 => { self.view_script = rd(0xa4); (self.view_time, self.view_speed) = (0.0, 1.0); }
                    7 => { self.view_script = rd(0xac); (self.view_time, self.view_speed) = (0.0, 1.0); }
                    0 => { self.view_script = rd(0xa0); (self.view_time, self.view_speed) = (0.0, 1.0); }
                    _ => {}
                }
            } else if self.view_script == 0 {
                (self.view_script, self.view_time, self.view_speed) = (rd(0xa0), 0.0, 1.0);
            } else {
                let len = self.scripts.get(&self.view_script).map_or(0.0, |s| s.length as f32);
                self.view_time = (self.view_time + self.view_speed).min((len - 1.0).max(0.0));
                // Gate the idle-loop reset on switch_state == 0 — fists has draw_script == idle_script
                // (both 0x6000485), so without this gate the reset fires during state 3 (drawing), zeroing
                // view_time before the state-3→0 check can succeed, leaving switch_state stuck at 3 forever.
                if self.gun.switch_state == 0 && self.view_script == rd(0xa0) && self.view_time >= len - 1.0 { self.view_time = 0.0; }
                // The fire script ended: state 9 → 0, the idle script re-queued (unless flags68 & 0x1000)
                // [M Player_SetWeaponAnimObj case 9 0x1a2e38:449-456 via AnimScriptIsStopped].
                let fire_last = rd(0xb4);
                if self.gun.fire_state && (self.view_script == rd(0xb0) || (fire_last != 0 && self.view_script == fire_last))
                    && self.view_time >= len - 1.0 {
                    if flags & 0x1000 == 0 { self.view_script = rd(0xa0); (self.view_time, self.view_speed) = (0.0, 1.0); }
                    self.gun.end_fire_state();
                }
            }
            // The release frame: the round spawns when the fire script's frame reaches `fire_anim_len` (+0x44)
            // and the +0x95C latch is clear [M Player_SetWeaponAnimObj case 9 0x1a2e38:416-420]; weapons with
            // `fire_anim_len == 0` spawn on the press tick (latched there) [M 0x1a5460:688-693]. Before this,
            // the port spawned every round on the press tick — a grenade pull fired a hitscan bullet instead
            // of winding up and throwing, and the 900-tick refire interval stayed armed afterwards.
            let release = if fire_anim_len == 0 { fired } else { self.gun.release_frame_reached(self.view_time, fire_anim_len) };
            if release && !fired { self.gun.latch = true; }
            // The gun scripts' sound commands (`AnimProcessScriptCmds` op 1: `Sound_Play3D(cmd[2], …)` when the
            // script's frame crosses cmd[1]): the shot (0x403 at frame 1 of the P2K's fire scripts) and the reload's
            // three sounds (0x40F, 0x40E, 0x411 at frames 0x2E, 0x59, 0x74). Frame = script time + 1.
            // Per-weapon op-1 id remap (docs/systems/sound.md §7): the script's id is replaced based on the weapon.
            if self.view_script != 0 {
                let from = if self.view_script != prev_script || (fired && self.view_time == 0.0) { f32::NEG_INFINITY } else { prev_time + 1.0 };
                let to = self.view_time + 1.0;
                let wid = self.gun.weapon;
                if let Some(sc) = self.scripts.get(&self.view_script) {
                    for c in sc.commands.iter().filter(|c| c.op == 1 && c.args.len() >= 2) {
                        let f = c.args[0] as f32;
                        if from < f && f <= to {
                            let id = remap_op1_sound(c.args[1] as u32, wid);
                            self.drones.audio.play3d(id, camera, None);
                            if std::env::var_os("NF_SOUNDLOG").is_some() { eprintln!("tick {}: gun script {:08x} frame {f}: sound {:#x} (remapped from {:#x} for weapon {:#x})", self.drones.frame, self.view_script, id, c.args[1], wid); }
                        }
                    }
                }
            }
            if release {
                // Gravity weapons (`+0x6C & 0x8`) are grenades: `Bullet_init` with the weapon record
                // [M 0x12c5a0] — the throw origin the game uses is the midpoint of the head and the view
                // model's bone 0 (`flags68 & 0x10000` arm, `Player_WeaponInitBullet` 0x1a4bf0:230); the port
                // has no view bones and spawns from the eye (*deviation*). Direction: the aim ray with
                // `Bullet_init`'s three spread draws, in the game's RNG order [M 0x12c5a0:166-184]. The spawn
                // itself is silent (`Bullet_init`'s param_7 = 0xffff); the throw's own sound is the fire
                // script's op-1 command (played above).
                let flags6c = u32::from_le_bytes(w[0x6c..0x70].try_into().unwrap());
                if flags6c & 0x8 != 0 {
                    let f32_at = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
                    let sdir = nf_game::bullet::spread(dir, f32_at(0x24), flags, &mut self.drones.rng);
                    let speed = f32_at(0x20);
                    // The fuse is the bullet's BU+0xFC: 210 ticks, 300 when weapon +0x70 & 0x10000 —
                    // hardcoded in `Bullet_init` (0x12c5a0:103, :196), NOT weapon_data+0x1C (0x35's +0x1C
                    // is 5000.0 [L ee_start_07000001.bin, audit fix-player2 claim 8]). The port's
                    // `Grenade.range` doubles as the tick fuse.
                    let fuse = if u32::from_le_bytes(w[0x70..0x74].try_into().unwrap()) & 0x10000 != 0 { 300.0 } else { 210.0 };
                    // The blast damage: `Explode_Create`'s damage arg is the hardcoded 100
                    // [M disasm 0x12cd98/0x12ce18: `addiu $t2, $zero, 0x64` → Explode_Propagate's amount via
                    // EXPLODEINFO+0x28 (Explode_CollisionHandler 0x17cde0:13-15); radius = $f12 =
                    // weapon_data+0x08]. +0x0C is the damage multiplier (weapon-data.md), not the amount.
                    self.drones.grenades.push(nf_game::grenade::Grenade {
                        pos: camera,
                        velocity: [sdir[0] * speed, sdir[1] * speed, sdir[2] * speed],
                        speed, travelled: 0.0, range: fuse, damage: 100.0, radius: f32_at(0x08),
                        weapon_type: u16::from_le_bytes([w[2], w[3]]), owner: usize::MAX, state: 0, bounces: 0,
                    });
                } else {
                    self.last_shot = Some(self.drones.frame);
                    self.shot_tick = true;
                    // The hearing sound: the P2K's 0x403 by default, or the remapped op-1 id for other weapons.
                    // The drones' hearing list uses the sfx table's entry for this id.
                    let hearing_id = match self.gun.weapon {
                        7 | 9 => 2,
                        3 | 5 => 2,
                        0x14 => 0x1fb,
                        0x1e..=0x23 => 0x284,
                        0x10 => 0x602,
                        0x16 | 0x17 => 0x604,
                        _ => 0x403,
                    };
                    let sound = self.sfx.as_ref().map(|t| t.get(hearing_id)).unwrap_or(nf_formats::sfx::SfxHearing { id: hearing_id, inner: 0.0, outer: 0.0, weight: 0.0, looped: false, cull: false });
                    self.drones.fire_player(&d, camera, dir, self.gun.weapon, sound);
                }
            }
        }
        // Write the gun's clip/spare back to the inventory. Recompute wi/t from the CURRENT weapon (not the
        // stale ones from line 410) — the switch state machine may have swapped the weapon mid-tick, and the
        // swap paths already wrote the OLD weapon's values back. Using stale wi/t here corrupts the old
        // weapon's ammo with the new weapon's values [bug fix C-10].
        let (cur_wi, cur_t) = (self.gun.weapon as usize, self.weapons.ammo_type(self.gun.weapon as usize));
        if let Some(c) = self.drones.inventory.clip.get_mut(cur_wi) { *c = self.gun.clip as i16; }
        if let Some(n) = self.drones.inventory.carried.get_mut(cur_t) { *n = self.gun.spare as i16; }
        self.aiming = aiming;
        self.drones.bond_aiming = aiming;
        self.drones.bond_weapon = self.gun.weapon;
        // The use key (`Player_Activate` 0x1a5170): the doors its sphere — one unit ahead of the head — meets
        // open, swing or thud locked (docs/systems/world-objects.md "Doors"); the player object updates before
        // the control list's doors, so they see the activation this same tick.
        if use_key { self.drones.player_use_doors(&d, camera, look, self.drones.frame); }
        // `Check_Target` (0x1a7810, every 6th frame — `GameState+0x52 % 6`): Bond's 50-unit view ray (mask
        // 0xC, world only); the first drone under it decides the crosshair state (BLData+0x133): 2 normal,
        // 3 = a non-enemy target — in SP the test is the target's `BLData+0x44 != 1` [M 0x1a7810:109-139,
        // 181-182]. `HUD_UpdateCrossHair` (0x19e6d0) renders state 3 as the red "no" mark (HUDCrossCoords
        // state 3 [L 0x2bec48]). The game also marks the target (+0x2A = 1, +0x2C4 = the clock) for the
        // civilian AI — the port's aimed_at marking in `Drones::tick` covers that arm.
        if self.drones.frame % 6 == 0 {
            let far = [camera[0] + look[0] * 50.0, camera[1] + look[1] * 50.0, camera[2] + look[2] * 50.0];
            let wall = self.world.segment(camera, far).unwrap_or(1.0);
            let hit = self.drones.list.iter().enumerate()
                .filter(|(_, dr)| dr.present && dr.enabled && dr.drawn && !dr.corpse)
                .filter_map(|(i, dr)| {
                    let vol = nf_game::bullet::PlayerVolume { feet: [dr.pos[0], dr.pos[1] - 1.0, dr.pos[2]], height: 1.8, radius: 0.4 };
                    nf_game::bullet::segment_cylinder(camera, far, &vol).filter(|&t| t < wall).map(|t| (i, t))
                }).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i);
            self.cross_state = match hit { Some(i) if self.drones.list[i].team != 1 => 3, _ => 2 };
        }
        // The object handler (`control_movement_object_handler`) runs whenever `GS_IsPaused(-1) == 0` — ScriptCam
        // plays NO role [M Game_Run 0x1c9790:30-35]: `ScriptCam` only picks `Text_Update2Line` vs `Text_Update`
        // (line 17) and the paused branch's `Light_Update`. The port therefore does NOT gate the drone tick on
        // `Sim::disabled`. A cut-scene freezes the guards only when its own script fires event 5
        // (`Drone_EnableAll(0)` → `NPCGlobals = 0` [M 0x1d94c0 case 5, 0x1383e8]), undone by `Script_Free`'s
        // 0x80 flag (event 7) [M 0x1da880:41-43] — ported as `Drones::npc_enabled` (core-loop §7 items 1-2).
        // (The B-10 symptom was the opposite gating bug: freezing the whole tick stopped Trigger_Calc +
        // SP_Update, so PlayerEnable never arrived.)
        self.drones.tick(&d, &player);
        // The HUD's per-frame state: the weapon-name timer, the crouch-icon timer, the pickup messages
        // (`Text_UpdateMsg` 0x1b11d8 runs from the HUD/status-pane update at frame rate).
        self.hud_tick();
        // `Lights_CalcAmbientLight` (0x1cb178) runs per moving object from `control_movement_object_handler`
        // (0x1336f8) — the last step of the tick, before the draw reads obj+0x100..0x102.
        self.ramp_char_ambient();
    }

    /// The HUD's per-tick state (`fix-hud`). Runs from the tick path (the game's HUD update is per
    /// 60 Hz frame): `HUD_UpdateAmmoPane` 0x19a7f0's BL+0x95E weapon timer, `HUD_UpdateHealthPane`
    /// 0x19b9c8's crouch-icon timer, and the pickup message queue (`Pickup_Handler` 0x18e650 →
    /// `Text_AddMsg` 0x1b0c30; `Text_UpdateMsg` 0x1b11d8 decrements the head's time by FRAME_RATE_MUL
    /// = 1 and deletes it at ≤ 0).
    fn hud_tick(&mut self) {
        // The weapon-name timer: set to 60 when the HUD's weapon id differs from the last seen one and
        // the two records' +2 words differ [M 0x19a7f0:206-213]; counts down every frame.
        let w = self.gun.weapon;
        if self.hud.hud_weapon != w && w != 0 {
            let changed = match (self.weapons.records.get(self.hud.hud_weapon as usize), self.weapons.records.get(w as usize)) {
                (Some(a), Some(b)) => a[2..4] != b[2..4],
                _ => false,
            };
            if changed { self.hud.weapon_label_timer = 60; }
        }
        self.hud.hud_weapon = w;
        self.hud.weapon_label_timer = self.hud.weapon_label_timer.saturating_sub(1);
        // The crouch icon: the timer holds 60 while crouched, then fades after standing
        // [M 0x19b9c8:305-328].
        if self.bond.crouched { self.hud.crouch_icon_timer = 60; }
        else if self.hud.crouch_icon_timer > 0 { self.hud.crouch_icon_timer -= 1; }
        // New pickup events → messages (`Pickup_Handler` 0x18e650's Text_AddMsg calls, type 6, 180 ticks
        // in SP; sounds 0xF5/0x141/0x14C are the take sounds the pickup path already plays).
        let lang = self.lang.take();
        let mut new_msgs: std::collections::VecDeque<Msg> = Default::default();
        for (_, t) in self.drones.taken.iter().skip(self.hud.taken_seen) {
            let Some(lang) = lang.as_ref() else { break };
            let text = match t {
                nf_game::pickup::Taken::Weapon { weapon } => {
                    let name = lang.bind(self.weapons.name_label(*weapon as usize)).unwrap_or("");
                    lang.format(0x200_000D, &[name.to_string()])
                }
                nf_game::pickup::Taken::Ammo { weapon, rounds } => {
                    let at = self.weapons.ammo_type(*weapon as usize);
                    let name = lang.bind(self.weapons.ammo_name_label(at)).unwrap_or("");
                    lang.format(0x200_000E, &[rounds.to_string(), name.to_string()])
                }
                nf_game::pickup::Taken::Key { .. } => lang.bind(0x200_0010).unwrap_or("").to_string(),
                nf_game::pickup::Taken::Armour => lang.bind(0x200_000F).unwrap_or("").to_string(),
                nf_game::pickup::Taken::Bonus => lang.bind(0x200_004A).unwrap_or("").to_string(),
                // Case 4 [M 0x18e650]: the pickup's own label (PICKUPINFO+0x34); the port's pickups
                // do not carry it, and the only kind-4 pickup on the disc data so far labels 0.
                nf_game::pickup::Taken::Label => String::new(),
            };
            if !text.is_empty() { new_msgs.push_back(Msg::new(text, 180.0)); }
        }
        self.hud.taken_seen = self.drones.taken.len();
        for m in new_msgs { self.hud.msgs.push_back(m); }
        self.lang = lang;
        // `Text_UpdateMsg` 0x1b11d8: time −= 1 per frame, delete at ≤ 0. The bar closes (state 4) when
        // the queue empties [M `HUD_UpdateStatusPane` 0x19c310].
        for m in &mut self.hud.msgs { m.time -= 1.0; }
        self.hud.msgs.retain(|m| m.time > 0.0);
        if self.hud.msgs.is_empty() {
            if self.hud.bar_close > 0.0 { self.hud.bar_close = (self.hud.bar_close - 16.0).max(0.0); }
        } else {
            self.hud.bar_close = HudState::BAR_W;
        }
        // The mission layer's events (`Mission_Update` 0x182c60 / `Mission_MonitorObjectives`
        // 0x1827d8, docs/systems/mission.md): type-2 objective notices, the FailLabel, the type-3
        // mission banner and the level-end / restart decisions. The banner SFX play when the pane
        // opens in the game [M 0x19c310:150-163, 220-234] — the port plays them as the event lands.
        let mut events = std::mem::take(&mut self.drones.mission_events);
        if !events.is_empty() {
            let cam = self.bond.head();
            for ev in events.drain(..) {
                match ev {
                    nf_game::mission::Event::Objective { label, is_new, ticks, failed } => {
                        // Sounds: a new objective plays 1532 then 469; "OBJECTIVE COMPLETE" 468
                        // [M 0x19c310:150-163, 220-234].
                        let complete = label == 0x0200_0053;
                        if complete { self.drones.audio.play3d(468, cam, None); }
                        else if is_new { self.drones.audio.play3d(1532, cam, None); self.drones.audio.play3d(469, cam, None); }
                        let mut text = match self.lang.as_ref().and_then(|l| l.bind(label)) {
                            Some(t) => t.to_string(),
                            None => continue,
                        };
                        if failed {
                            // The game's "%s: %s" (0x30cbb8) with 0x170 "Fail" [M 0x1827d8:72-74].
                            let fail = self.lang.as_ref().and_then(|l| l.bind(0x170)).unwrap_or("Fail").to_string();
                            text = format!("{fail}: {text}");
                        }
                        let status = if complete { 0x0200_0053 } else { 0x0200_0051 };
                        self.hud.objective_msgs.push_back(Msg { text, time: ticks as f32, status, failed });
                    }
                    nf_game::mission::Event::FailLabel(l) => {
                        self.hud.fail_label = l;
                    }
                    nf_game::mission::Event::MissionComplete { next: _ } => {
                        // The type-3 "Mission Complete" (0x2000006), 240 ticks [M 0x182c60:85-122].
                        // The level end itself arrives as `Event::LevelEnd` after the banner (case 4).
                        if let Some(t) = self.lang.as_ref().and_then(|l| l.bind(0x0200_0006)) {
                            self.hud.mission_msgs.push_back(Msg::new(t.to_string(), 240.0));
                        }
                    }
                    nf_game::mission::Event::MissionFailed => {
                        // The type-3 "Mission Fail" (0x2000007), 300 ticks [M 0x182c60:45-57].
                        if let Some(t) = self.lang.as_ref().and_then(|l| l.bind(0x0200_0007)) {
                            self.hud.mission_msgs.push_back(Msg::new(t.to_string(), 300.0));
                        }
                    }
                    nf_game::mission::Event::PlayerDead => {} // the red fade is render-side; the restart arrives below
                    nf_game::mission::Event::LevelEnd { next } => {
                        // `ResetMap_LevelToLoad(LevelToEndTo)` [M 0x182c60:73]. The 0x0730xxxx codes
                        // are DRIVING.ELF levels the port cannot load — logged and skipped.
                        if next & 0xffff_0000 == 0x0700_0000 {
                            self.drones.level_exit = Some(next);
                        } else {
                            eprintln!("[mission] complete → LevelToEndTo {next:08x} (driving level, not ported)");
                        }
                    }
                    nf_game::mission::Event::Restart => {
                        // The game opens the 0x40000042 menu (retry/quit) [M 0x182c60:78]; the port
                        // restarts the level.
                        self.drones.level_exit = Some(self.level);
                    }
                }
            }
        }
        for m in &mut self.hud.objective_msgs { m.time -= 1.0; }
        self.hud.objective_msgs.retain(|m| m.time > 0.0);
        if self.hud.objective_msgs.is_empty() {
            if self.hud.objective_close > 0.0 { self.hud.objective_close = (self.hud.objective_close - 16.0).max(0.0); }
        } else {
            self.hud.objective_close = HudState::OBJECTIVE_W;
        }
        for m in &mut self.hud.mission_msgs { m.time -= 1.0; }
        self.hud.mission_msgs.retain(|m| m.time > 0.0);
        if self.hud.mission_msgs.is_empty() {
            if self.hud.mission_close > 0.0 { self.hud.mission_close = (self.hud.mission_close - 32.0).max(0.0); }
        } else {
            self.hud.mission_close = HudState::MISSION_H;
        }
    }

    /// The crosshair offset (`BL+0x120 / +0x124`: −1…1 across half the screen) from the auto-aim offset angles: tan of the
    /// angle over the half-screen tangents of the game's projection [L] (0.76966 across, 0.57724 up); a larger yaw
    /// turns left (+X is screen-left in the object frame), so x flips.
    pub fn crosshair(&self) -> (f32, f32) {
        if self.aiming { return (self.cross[0], self.cross[1]); }
        (-self.drones.aim_offset[0].tan() / 0.76966, self.drones.aim_offset[1].tan() / 0.57724)
    }

    /// The drawn instance of a level-placed pickup (`View_DrawObjectGlist` 0x1e6780 draws `obj+0xD8` at
    /// `obj+0x90`): its matrix while it lies there, `None` once taken or hidden. fix-player2.
    pub fn placed_pickup_pose(&self, k: usize) -> Option<nf_formats::anim::Mat4> {
        self.drones.pickups.get(k).filter(|p| p.from.is_none() && !p.gone && !p.hidden).map(|p| p.m)
    }

    /// The crosshair state (`HUD_UpdateCrossHair` 0x19e6d0 reads BLData+0x133): 2 normal, 3 = the red
    /// "no" mark on a non-enemy target (`Check_Target` 0x1a7810:181-182). fix-player2.
    pub fn crosshair_state(&self) -> u8 { self.cross_state }

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
            let c = |sr: f32, su: f32, uv: [f32; 2]| Vertex { pos: [p[0] + r[0] * sr + u[0] * su, p[1] + r[1] * sr + u[1] * su, p[2] + r[2] * sr + u[2] * su], uv, rgba: [63, 63, 63, 127], nrm: [0; 4] };
            let (a, b, cc, d) = (c(-1.0, -1.0, [0.0, 1.0]), c(1.0, -1.0, [1.0, 1.0]), c(1.0, 1.0, [1.0, 0.0]), c(-1.0, 1.0, [0.0, 0.0]));
            out.extend_from_slice(&[a, b, cc, a, cc, d]);
        }
        out
    }

    /// The live animated script object from `src` (script hash, stream) on `skin`: its animation script and frame.
    pub fn script_anim(&self, src: (u32, u8), skin: u32) -> Option<(u32, f32)> {
        self.drones.scripts.objects.list.iter().flatten().rev().find_map(|o| match o {
            nf_game::nis::Obj::Anim { skin: k, script, frame, src: o, .. } if *o == src && *k == skin => Some((*script, *frame)),
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
        nf_game::drone::Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.level, sequences: &self.seqs, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models , surfaces: self.surfaces.as_ref(), voices: self.voices.as_deref() }
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

/// `FlashMod` (Check_Target 0x1a7810:95-97, the pickup/armour glow driver): a 1 Hz sine shimmer in
/// [0.5, 1.0] — `((sin(2π·t + π/2) + 1)·0.5 + 1)·0.5` with t = the game time in seconds
/// (GameState+0x76 = psiGetTimeIn100ths, written each main-loop pass 0x1c98c8:84; the phase is
/// time100 · 0.01 s). Brightest at phase 0 (level start + whole seconds). The port feeds it its tick
/// count (time100 = frame · 100/60, so the phase is frame/60) — render-side only, no sim state.
pub fn flash_mod(frame: u32) -> f32 {
    let t = frame as f32 / 60.0;
    let s = (t * std::f32::consts::TAU + std::f32::consts::FRAC_PI_2).sin();
    ((s + 1.0) * 0.5 + 1.0) * 0.5
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
    /// The camera's screen axes this frame (corona billboards; set by the caller from its camera).
    pub right: [f32; 3],
    pub up: [f32; 3],
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
    /// A searchlight part's vertex (index into `Sim::drones.searchlights.parts`), posed by the simulation
    /// each frame (`Searchlight_Update`): the mount/head/beam/spot matrices, coronas as camera billboards.
    pub searchlight: Option<u16>,
    /// A door's vertex (index into `Sim::drones.doors.list`), posed by the simulation each frame
    /// (`Door_Interp`'s matrix at `obj+0x90`).
    pub door: Option<u16>,
    /// A level-placed pickup's vertex (index into `Sim::drones.pickups`), posed by the simulation each
    /// frame at the pickup's matrix (`View_DrawObjectGlist` 0x1e6780 draws `obj+0xD8` at `obj+0x90`);
    /// collapsed when taken/hidden. fix-player2 (the invisible armour).
    pub pickup: Option<u16>,
}

/// Per-vertex GPU skinning data (`Scene::skin_layout`); world vertices carry `a = u32::MAX` (static).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinAttr { pub a: u32, pub b: u32, pub w: f32, pub col: u32 }

/// The palette / colour-table layout of `Scene::skin_layout`.
pub struct SkinLayout { pub char_base: Vec<u32>, pub attach_base: u32, pub sl_base: u32, pub door_base: u32, pub pick_base: u32, pub n_mats: u32, pub col_att: u32, pub col_pick: u32, pub col_one: u32 }

/// One frame's pose state (`Scene::frame_state`).
pub struct FrameState {
    pub posed: Vec<Option<(Vec<Mat4>, Vec<Mat4>, Mat4)>>,
    pub attach: Vec<Mat4>,
    pub hidden: Vec<bool>,
}

/// Dev-panel NPC pool size per skin.
pub const NPC_POOL: usize = 3;

/// One hidden pool NPC (`Scene::npc_pool`): its skin, the placement parameters it was copied from, and the
/// characters (meshes) that draw it.
#[derive(Clone)]
pub struct NpcSlot { pub key: [u32; 3], pub skin: u32, pub params: Vec<(u32, u32)>, pub chars: Vec<usize> }

pub struct Scene {
    /// Hidden NPC copies the dev panel can bring to life (`dev::spawn_npc`).
    pub npc_pool: Vec<NpcSlot>,
    /// The level's room cels (`parse_room_cels`), kept for the static-draw cel cull
    /// (`View_AddCels` 0x1e6ba0 frustum-tests each cel's bbox; `cel_sphere`/`cel_index_at`).
    pub cels: Vec<Cel>,
    /// The runtime cel graph + portals (map block 0x21) — `portal_visible_cels`' input (fix-perf-level,
    /// G-016's portal half). `.room_to_cell` maps a `cels` index into `cells`.
    pub cel_graph: CelGraph,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub materials: Vec<Material>,
    pub textures: Vec<TextureImage>,
    /// The first-person gun (`Player_SetWeaponAnim`: `AnimObjectNew(G, weapon+0xDC)`): one character per owned
    /// weapon id, built at scene build time. The active one is selected each frame in `animate` from
    /// `sim.gun.weapon`.
    pub view_models: HashMap<u8, usize>,
    /// The snow flake texture (`DrawDrops`: 0x03000045).
    pub snow_texture: Option<TextureImage>,
    /// Streamed effects: `STREAMS.LUT` entries and `STREAMS.BIN` (set by the loader).
    pub streams: Option<(Vec<[u32; 4]>, Vec<u8>)>,
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
    /// How many ladder (0x22) placements got a World-layer instance (fix-ladder2's draw sweep).
    /// The game draws each ladder exactly once through the dynamic-object path [M View_DrawObjList
    /// 0x1e69a0 → View_DrawObjectGlist 0x1e6780]; 0 means the ladders are invisible.
    pub ladders_drawn: usize,
    /// Per level object hashcode: whether its glist flags carry 0x800 (the character-lighting bit
    /// `Control_SetGList` 0x134960 merges into obj+0xF0). fix-render2: decides the ambient dim for
    /// the drawn script entities (the truck's nine objects carry 0x8880 [L ee_start_05.bin
    /// 0x0137C8A0…, script-player.md]); pickups are gated by their kind instead (Pickup_Create
    /// 0x18dda0 forces 0x840, clearing it for kinds 3 and 6).
    pub models_0x800: HashMap<u32, bool>,
    /// Bug 3 diagnostic: objects with collision, for the "name what I'm touching" overlay (`C` key). Each entry is
    /// the object's name and its world-space AABB (lo, hi), built from the instance matrix + the object's bounds.
    pub colliders: Vec<(String, [f32; 3], [f32; 3])>,
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
/// and draw state. The value keeps the object's world centre and scaled radius for the per-frame sort + frustum cull.
type Groups = BTreeMap<(Layer, [u32; 3], u32, Option<(usize, u32)>, Draw), (Vec<Vertex>, Vec<u32>, Vec<CharVertex>, [f32; 3], f32, Option<u16>)>;

fn add_instance(level: &Level, inst: &Instance, layer: Layer, character: Option<(u32, Option<u32>)>, groups: &mut Groups, bounds: Option<&mut ([f32; 3], [f32; 3])>, searchlight: Option<u16>, door: Option<u16>, pickup: Option<u16>, mult: [f32; 3], cels: &[Cel]) {
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
    // keep each object apart. The world-space bounding-sphere radius (glist radius × the instance matrix's max axis
    // scale) feeds the CPU-side frustum cull (A-11e) — `transform` uses row-vector convention, so the columns of the
    // upper-left 3×3 are the basis vectors and their lengths are the per-axis scales. `object_matrix` is a pure
    // rotation+translation (column lengths = 1), so most world placements keep `obj.radius` unchanged.
    let centre = transform(&inst.matrix, obj.centre);
    let col_len = |c: usize| (inst.matrix[0][c] * inst.matrix[0][c] + inst.matrix[1][c] * inst.matrix[1][c] + inst.matrix[2][c] * inst.matrix[2][c]).sqrt();
    let radius = obj.radius * col_len(0).max(col_len(1)).max(col_len(2));
    let ikey = if matches!(layer, Layer::World) { centre.map(f32::to_bits) } else { [0; 3] };
    // GS state: each leaf's `DIRECT` writes, inherited by the leaves that send none. The GS applies each DIRECT
    // write immediately at its position in the packet, so a write between batch 2 and batch 3 affects batch 3
    // onward — we apply the writes per-batch, not per-leaf (the old per-leaf approach made the sniper's gun body
    // transparent because the leaf's final state was additive for the trailing reflection batches).
    let (mut alpha, mut test, mut zbuf) = (0x44u64, 0x5001bu64, 0x3100_0000u64);
    for (li, leaf) in obj.gfx.leaves.iter().enumerate() {
    let mut gs_idx = 0usize;
    for (bi, b) in leaf.batches.iter().enumerate() {
        // Apply DIRECT writes that take effect before/at this batch.
        while gs_idx < leaf.gs.len() && leaf.gs[gs_idx].2 <= bi {
            let (r, v, _) = leaf.gs[gs_idx];
            match r { 0x42 => alpha = v, 0x47 => test = v, 0x4e => zbuf = v, _ => {} }
            gs_idx += 1;
        }
        let draw = Draw::from_gs(pass, fog, alpha, test, zbuf);
        let lkey = if matches!(layer, Layer::World) { (li as u32) << 16 | bi as u32 } else { 0 };
        let key = b.texture.filter(|&t| (t as usize) < level.maps[inst.map].textures.len()).map(|t| (inst.map, t));
        // fix-render3: the room cel for the static World layer (build_LinkToRoom 0x1c9680 →
        // build_FindCel 0x1c8cb0 on the object's world centre; View_AddCels 0x1e6ba0 frustum-tests
        // each cel's bbox per viewer). Everything posed per frame keeps None.
        let cel = if layer == Layer::World { find_cel(cels, centre).map(|i| i as u16) } else { None };
        let (vs, is, cv, c, r, gcel) = groups.entry((layer, ikey, lkey, key, draw)).or_default();
        *gcel = cel;
        *c = centre;
        *r = radius;
        let base = vs.len() as u32;
        for v in &b.verts {
            let p = transform(&m, v.pos);
            if let Some((lo, hi)) = bounds.as_deref_mut() {
                if p.iter().all(|c| c.is_finite()) {
                    for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); }
                }
            }
            let uv = if v.uv.iter().all(|c| c.is_finite()) { v.uv } else { [0.0, 0.0] };
            // The per-instance colour multiplier: the game's VU1 `TAmbient` term (`psiSetTweakARGB` 0x124770,
            // `View_SetupRenderModes_Tweak` 0x1e5888) multiplies the vertex colour of 0x800 objects by
            // tint × obj+0x100..0x102 >> 8 / 255. Applied here on the vertex bytes (the GS modulate is the
            // same multiply); alpha byte obj+0x106 is passed as-is (white tint → untouched).
            let rgba = if mult == [1.0; 3] { v.rgba } else {
                let rgb = [0, 1, 2].map(|k| ((v.rgba[k] as f32 * mult[k]).round() as i32).clamp(0, 255) as u8);
                [rgb[0], rgb[1], rgb[2], v.rgba[3]]
            };
            // The normal: posed vertices keep the mesh's own (the shader rotates it with the palette); static
            // World vertices get it in world space through the instance matrix here.
            let posed = character.is_some() || searchlight.is_some() || door.is_some() || pickup.is_some();
            let n0 = v.normal.map(|c| c as f32);
            let n = if posed { n0 } else { [0, 1, 2].map(|k| n0[0] * m[0][k] + n0[1] * m[1][k] + n0[2] * m[2][k]) };
            let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            let nrm = if nl > 1e-6 { [(n[0] / nl * 127.0) as i8, (n[1] / nl * 127.0) as i8, (n[2] / nl * 127.0) as i8, 0] } else { [0; 4] };
            vs.push(Vertex { pos: p, uv, rgba, nrm });
            if let Some((c, attachment)) = character { cv.push(CharVertex { character: c, pos: v.pos, weight: v.weight, slots: v.slots, attachment, searchlight: None, door: None, pickup: None }); }
            else if let Some(sl) = searchlight { cv.push(CharVertex { character: 0, pos: v.pos, weight: v.weight, slots: v.slots, attachment: None, searchlight: Some(sl), door: None, pickup: None }); }
            else if let Some(dr) = door { cv.push(CharVertex { character: 0, pos: v.pos, weight: v.weight, slots: v.slots, attachment: None, searchlight: None, door: Some(dr), pickup: None }); }
            else if let Some(pk) = pickup { cv.push(CharVertex { character: 0, pos: v.pos, weight: v.weight, slots: v.slots, attachment: None, searchlight: None, door: None, pickup: Some(pk) }); }
        }
        for t in b.triangles() { is.extend(t.map(|k| base + k as u32)); }
    }
    // Apply trailing DIRECT writes (after the leaf's last batch) so the state carries to the next leaf.
    while gs_idx < leaf.gs.len() {
        let (r, v, _) = leaf.gs[gs_idx];
        match r { 0x42 => alpha = v, 0x47 => test = v, 0x4e => zbuf = v, _ => {} }
        gs_idx += 1;
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
        let (mut instances, missing) = level.scene_instances(cels_only);
        // Sky objects are drawn by the sky pass only (with --all they would also appear as dynamic entities).
        // Ladders (0x22) are dynamic objects too (`parsemap_create_dynamic_objects` 0x1d04f0 case 0x22):
        // their placements never carry the 0x8000 CREATE_CEL flag [M 0x1d1998 vs 0x1d1b68 — the static and
        // dynamic paths are mutually exclusive per placement], so they must not ride the instance loop;
        // the ladder sweep below draws them exactly once (fix-ladder2: the ladders were invisible because
        // `instances(cels_only=true)` skips every placement without 0x8000).
        instances.retain(|i| !matches!(i.placement.flags & 0xff, 0x2a | 0x22));
        let (mut char_instances, chars_missing) = level.character_instances();
        // Dev NPC pool (the external dev panel's "spawn NPC"): for the first drone placement of each skin, NPC_POOL
        // hidden copies of all its meshes, far below the world. They are ordinary characters with their held weapon,
        // so a spawned drone is drawn exactly like a placed one once `Sim::char_drone` points them at it.
        let pool_sentinel = |t: [f32; 3]| t[1] <= -999_000.0;
        {
            let mut first: Vec<(u32, [u32; 3])> = Vec::new();
            for inst in &char_instances {
                if inst.placement.flags & 0xff != 0x0f { continue; }
                let skin = inst.placement.params.iter().find(|(i, _)| *i == 0).map_or(0, |&(_, v)| v);
                if skin == 0 || !level.skins.contains_key(&skin) || first.iter().any(|(s, _)| *s == skin) { continue; }
                first.push((skin, inst.placement.translation.map(f32::to_bits)));
            }
            let mut extra = Vec::new();
            for (slot, (_, t)) in first.iter().enumerate() {
                for k in 0..NPC_POOL {
                    let tr = [-1.0e6 - (slot * NPC_POOL + k) as f32, -1.0e6, -1.0e6];
                    for inst in char_instances.iter().filter(|i| i.placement.translation.map(f32::to_bits) == *t) {
                        let mut placement = inst.placement.clone();
                        placement.translation = tr;
                        let mut matrix = inst.matrix;
                        matrix[3] = [tr[0], tr[1], tr[2], 1.0];
                        extra.push(Instance { map: inst.map, object: inst.object, matrix, placement });
                    }
                }
            }
            char_instances.extend(extra);
        }
        let mut npc_pool: Vec<NpcSlot> = Vec::new();
        let sky = level.sky_instances();
        let mut groups: Groups = BTreeMap::new();
        let mut bounds = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        // Level-placed pickups (`Pickup_Create` 0x18dda0 via `parsemap_create_dynamic_objects` type 0xF0),
        // built BEFORE the draw groups so the draw loop can instance them (fix-player2: the placed armour
        // had no drawn instance at all — `View_DrawObjectGlist` 0x1e6780 draws `obj+0xD8` at `obj+0x90`).
        let placed_weapons = drones.and_then(|d| d.weapons.clone()).unwrap_or_else(|| nf_formats::weapon::WeaponData { records: Vec::new(), ammo_cap: Vec::new(), ammo_raw: Vec::new() });
            let hoist_models = nf_game::pickup::models(level);
            let mm = level.main_map().unwrap_or(0);
            let placed_pickups: Vec<nf_game::pickup::Pickup> = level.maps[mm].placements.iter()
                .filter(|p| p.flags & 0xff == 0xf0)
                .filter_map(|p| {
                    // `parsemap_block_map_data_dynamic` collects the placement's params into a stack buffer;
                    // `parsemap_create_dynamic_objects` case 0xf0 reads from that buffer. The params are
                    // (index, value) pairs and parameter k lives at `level_tag+0x2C+4k` — looked up by
                    // INDEX, not by position [M 0x1d04f0:457-462; pickup-port-check §3]. An absent index
                    // reads 0.
                    let param = |idx: u32| -> u32 { p.params.iter().find(|(i, _)| *i == idx).map(|(_, v)| *v).unwrap_or(0) };
                    let type_ = param(0) as u16;
                    let id = param(1) as u16;
                    let rounds = param(2) as i16;
                    let switch_channel = param(3) as u16;
                    let sound = param(4) as u16;
                    let respawn = param(5) & 0xFFFF;
                    let lifetime = 0u8; // always 0 for level-placed pickups (rest=1 in the game)
                    // The model: the placement's object (the celglist passed to Pickup_Create, [M]
                    // 0x1d04f0:459-462). fix-objects: keep the RESOLVED object — the game draws the
                    // glist pointer, not a hashcode re-lookup, and an object with hashcode 0xFFFFFFFF
                    // (07000005's `oOBJECT_AmmoPickUp_9mm` [D]) has no by-hash entry, so the old
                    // `model`-hash fallback left those pickups undrawn. The bounds are the object's
                    // own (the glist's), not the first hashcode match's.
                    let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                        else if (p.index as usize) < level.maps[mm].objects.len() { Some((mm, p.index as usize)) } else { None };
                    let (model, bounds, obj) = target.and_then(|(om, oi)| {
                        let obj_ = &level.maps[om].objects[oi];
                        hoist_models.get(&obj_.hashcode).map(|&m| (obj_.hashcode,
                            nf_game::pickup::Model { centre: obj_.centre, radius: obj_.radius }, (om, oi)))
                    })?;
                    let mut p2 = nf_game::pickup::Pickup::create_placed(p.translation, p.rotation, type_, id, rounds, switch_channel, sound, respawn, lifetime, model, bounds, &placed_weapons)?;
                    p2.obj = Some(obj);
                    Some(p2)
                })
                .collect();

        // The spawn comes from the placement records even when entities are not drawn (Player1 is not a 0x8000 cel).
        let start = level.instances(false).0.iter()
            .find(|i| level.maps[i.map].objects[i.object].name.trim() == "Player1")
            .map(|i| (i.placement.translation, i.placement.rotation[1]));
        let mut hidden = 0;
        let mut ladders_drawn = 0usize; // the sweep below counts the drawn ladder instances (fix-ladder2)
        // The level's room cels (see `Cel`): parsed before the draw so the static 0x800 world objects can snap
        // to their cel's ambient exactly as the game's link does at level start (`build_LinkToRoom` 0x1c9680 →
        // the snap in `control_link_object_to_cel` 0x1346b8; channels are all 0 here, so triple 1 reads).
        let (room_cels, cel_graph) = parse_cel_graph(level);
        let sim_cels = room_cels.clone();
        // Bug 3 diagnostic (`NF_COLL=1`): log objects that have collision but are not drawn — the "invisible walls".
        // An object is an invisible wall if it has collision triangles but: it's skipped by the cel pass (flag 0x10),
        // it's a door/searchlight/corona that the static pass skips (drawn elsewhere — but if those paths also skip
        // it, it's invisible), or its gfx leaves are all empty (no visible mesh). The user can then check the named
        // object against PCSX2 to see if the game hides, destroys or never creates it.
        let coll_debug = std::env::var_os("NF_COLL").is_some();
        let mut invisible = 0;
        for inst in &instances {
            let obj = &level.maps[inst.map].objects[inst.object];
            let has_coll = obj.collision.is_some();
            let has_mesh = obj.gfx.leaves.iter().any(|l| !l.batches.is_empty());
            let skipped = inst.placement.creates_cel() && obj.skipped_by_cel_pass();
            if skipped { hidden += 1; }
            // An invisible wall: collision but no visible mesh, or skipped by the cel pass while not a door/searchlight/corona.
            if has_coll && coll_debug && (!has_mesh || (skipped && !matches!(inst.placement.flags & 0xff, 0xdb | 0xde | 0xfc))) {
                let n_tris = obj.collision.as_ref().map(|c| c.triangles().len()).unwrap_or(0);
                eprintln!("coll: invisible wall \"{}\" (map {} obj {}): {} tris, flags {:#x}, has_mesh={has_mesh}, skipped={skipped} at {:?}",
                    obj.name.trim(), inst.map, inst.object, n_tris, obj.flags, inst.placement.translation);
                invisible += 1;
            }
            if skipped { continue; }
            // Searchlight parts are dynamic objects (`Searchlight_Update` moves them every tick): drawn from
            // the simulation below, not from their placements.
            if inst.placement.flags & 0xff == 0xde { continue; }
            // Doors likewise (`Door_Update`/`Door_Interp` move them): posed from the simulation below.
            if inst.placement.flags & 0xff == 0xdb { continue; }
            // Level coronas are dynamic objects too (`Corona_Create`, class 1 with no update function): added
            // below from their placements so the alpha-list pass draws them (docs/systems/world-objects.md).
            if inst.placement.flags & 0xff == 0xfc { continue; }
            // Debug: `NF_HIDE=name,name` leaves out world objects whose name contains one of them.
            if let Ok(h) = std::env::var("NF_HIDE") { if h.split(',').any(|n| !n.is_empty() && obj.name.contains(n)) { continue; } }
            // The static 0x800 world objects (pickups and props flagged `celglist+0x44 & 0x800`): the game dims
            // them by tint × obj+0x100..0x102 (`View_SetupRenderModes_Tweak` 0x1e5888, the 0x800 branch),
            // snapped to their cel's ambient at the level-start link. World-cel geometry goes through
            // `View_DrawCelGlist` (0x1e6538) which forces the white tweak — multiplier 1.0 [M gs-leaf.md].
            let mult = if !inst.placement.creates_cel() && obj.flags & 0x800 != 0 {
                let p = transform(&inst.matrix, obj.centre);
                mul_bytes(cel_ambient(&room_cels, p, 0).unwrap_or([0xff; 3]))
            } else { [1.0; 3] };
            add_instance(level, inst, Layer::World, None, &mut groups, Some(&mut bounds), None, None, None, mult, &room_cels);
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
                add_instance(level, inst, Layer::World, None, &mut groups, Some(&mut bounds), None, None, None, if obj.flags & 0x800 != 0 {
                    // A non-skinned character object drawn through the World layer: the same 0x800 dim.
                    mul_bytes(cel_ambient(&room_cels, transform(&inst.matrix, obj.centre), 0).unwrap_or([0xff; 3]))
                } else { [1.0; 3] }, &room_cels);
                continue;
            }
            add_instance(level, inst, Layer::Character, Some((characters.len() as u32, None)), &mut groups, Some(&mut bounds), None, None, None, [1.0; 3], &room_cels);
            let start = drones.and_then(|d| {
                let param = |k: u32| inst.placement.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
                let hash = drone::start_script(&d.tables, param, d.level_id).script()?;
                let script = d.scripts.get(&hash)?.clone();
                let sequences = script.sequences()
                    .filter_map(|(a, b, q)| by_hash.get(&q).filter(|(sk, _)| *sk == skin.skeleton).map(|&(_, i)| (a, b, i))).collect();
                Some(StartAnim { script, sequences })
            });
            let pooled = pool_sentinel(inst.placement.translation);
            let role = if pooled { drone::SpawnRole::Reinforcement { channel: 0 } } else {
                roles.get(&inst.placement.translation.map(f32::to_bits)).copied().unwrap_or(drone::SpawnRole::Placed)
            };
            if pooled {
                let ci = characters.len();
                match npc_pool.iter_mut().find(|n| n.key == inst.placement.translation.map(f32::to_bits)) {
                    Some(n) => n.chars.push(ci),
                    None => npc_pool.push(NpcSlot { key: inst.placement.translation.map(f32::to_bits), skin: skin_hash, params: inst.placement.params.clone(), chars: vec![ci] }),
                }
            }
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
            add_instance(level, &winst, Layer::Character, Some((character, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
            let local = anim::quat_trans_to_mat(dt.q, dt.t);
            attachments.push(Attachment { character, bone: dt.bone as usize, local, flash: None, tracer: None, dropped: false, script: None });
            // The same gun as a pickup once the drone drops it.
            add_instance(level, &winst, Layer::Character, Some((character, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
            attachments.push(Attachment { character, bone: 0, local, flash: None, tracer: None, dropped: true, script: None });
            // Its muzzle flash: `DroneWeap_FireWeapon` asks `AnimGetBoneWorldTrans(obj, datum id, 1, …)`, which looks up
            // the datum whose id is id + 1 — the muzzle (07000005: 8 → 9 at x −0.77, 16 → 17 at x −1.18).
            if let Some((fmesh, rgb)) = wd.muzzle_flash(weapon as u8) {
                let muzzle = skin.datums.iter().find(|d| d.id == datum_id + 1 && d.bone >= 0);
                if let (Some(&(fmap, fobject)), Some(md)) = (level.by_hash.get(&fmesh), muzzle) {
                    let finst = Instance { map: fmap, object: fobject, ..winst };
                    add_instance(level, &finst, Layer::Effect, Some((character, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                    attachments.push(Attachment { character, bone: md.bone as usize, local: anim::quat_trans_to_mat(md.q, md.t), flash: Some(rgb), tracer: None, dropped: false, script: None });
                }
            }
        }
        // The first-person gun models: one character per owned weapon with a first-person skin (`weapon_data+0xDC`),
        // drawn with a sleeve mesh and rigid parts on their bones. The active one is selected each frame
        // in `animate` from `sim.gun.weapon`. The P2K (weapon 2) is always first; others follow the owned list [M].
        let mut view_models: HashMap<u8, usize> = HashMap::new();
        if let Some(wd) = drones.and_then(|d| d.weapons.as_ref()) {
            const SLEEVES: [u32; 8] = [0x0200_09d0, 0x0200_0900, 0x0200_01f7, 0x0200_0929, 0x0200_09e6, 0x0200_09ff, 0x0200_0a14, 0x0200_0a13];
            // Build a view model for each weapon with a first-person skin present in the level. The P2K (weapon 2)
            // is always first; others follow the weapon_data order.
            for wid in 1..=0x72u8 {
                let fp = wd.first_person(wid as usize);
                if fp == 0 { continue; }
                let Some(skin) = level.skins.get(&fp) else { continue };
                let Some(&sleeve) = SLEEVES.iter().find(|h| level.by_hash.contains_key(h)) else { continue };
                let Some(skeleton) = skeletons.get(&skin.skeleton) else { continue };
                let ci = characters.len() as u32;
                let ident = anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]);
                let placement = level.maps[level.main_map().unwrap_or(0)].placements.first().cloned().unwrap_or_default();
                let (map, object) = level.by_hash[&sleeve];
                let inst = Instance { map, object, matrix: ident, placement: placement.clone() };
                add_instance(level, &inst, Layer::Character, Some((ci, None)), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                for &(bone, part) in &skin.extra {
                    let Some(&(pmap, pobject)) = part.and_then(|h| level.by_hash.get(&h)) else { continue };
                    let pinst = Instance { map: pmap, object: pobject, matrix: ident, placement: placement.clone() };
                    add_instance(level, &pinst, Layer::Character, Some((ci, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                    attachments.push(Attachment { character: ci, bone: bone as usize, local: ident, flash: None, tracer: None, dropped: false, script: None });
                }
                // The muzzle flash (`Player_MuzzleFlash`): mesh `weapon+0x48` on datum 0, colour (+0x56, +0x55, +0x54).
                let r = &wd.records[wid as usize];
                let flash = u32::from_le_bytes(r[0x48..0x4c].try_into().unwrap());
                if let (Some(&(fmap, fobject)), Some(dt)) = (level.by_hash.get(&flash), skin.datums.iter().find(|d| d.id == 0 && d.bone >= 0)) {
                    let finst = Instance { map: fmap, object: fobject, matrix: ident, placement: placement.clone() };
                    add_instance(level, &finst, Layer::Effect, Some((ci, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                    attachments.push(Attachment { character: ci, bone: dt.bone as usize, local: anim::quat_trans_to_mat(dt.q, dt.t), flash: Some([r[0x56], r[0x55], r[0x54]]), tracer: None, dropped: false, script: None });
                }
                characters.push(Character { matrix: ident, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones: level.maps[map].objects[object].gfx.slot_bones.clone(), start: None, role: drone::SpawnRole::Placed, floor: None, script: None });
                view_models.insert(wid, ci as usize);
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
                                    add_instance(level, &inst, Layer::Character, Some((ci, None)), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                                    slot_bones = level.maps[map].objects[object].gfx.slot_bones.clone();
                                }
                                for &(bone, part) in &skin.extra {
                                    let Some(&(map, object)) = part.and_then(|ph| level.by_hash.get(&ph)) else { continue };
                                    let pinst = Instance { map, object, matrix: ident, placement: placement.clone() };
                                    add_instance(level, &pinst, Layer::Character, Some((ci, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                                    attachments.push(Attachment { character: ci, bone: bone as usize, local: ident, flash: None, tracer: None, dropped: false, script: None });
                                }
                                characters.push(Character { matrix: ident, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones, start: None, role: drone::SpawnRole::Placed, floor: None, script: Some((h, si as u8)) });
                                script_objects += 1;
                                // The held weapon (`AnimObjectDraw` 0x1c4490:256-265): Event 0xB on the next stream
                                // sets the weapon byte on this Anim (`sAnimObject+0x62`), then `AnimDatumGetWeaponInfo`
                                // reads `weapon_data[w]+0x80` (3rd-person mesh) on datum `weapon_data[w]+0x50`. The
                                // scene's attachments are fixed at build time, so we scan stream si+1's commands for
                                // the static `Cmd::Event { kind: 0xB, params: [w, ..] }` (the live `Obj::Anim.weapon`
                                // is 0 until the script ticks). Same path as in-play drones (scene.rs:1099-1127).
                                let weapon = nis.streams.get(si + 1).and_then(|next| {
                                    next.decode().into_iter().find_map(|c| match c {
                                        nf_formats::nis::Cmd::Event { kind: 0xB, params } => params.first().copied().map(|w| w as u8),
                                        _ => None,
                                    })
                                });
                                if let (Some(w), Some(wd)) = (weapon, drones.and_then(|d| d.weapons.as_ref())) {
                                    if let Some((mesh, datum_id)) = (w < 0x73).then(|| wd.third_person(w)).flatten() {
                                        let datum_id = if datum_id == 0xff { 0 } else { datum_id as u32 };
                                        if let Some(dt) = skin.datums.iter().find(|d| d.id == datum_id && d.bone >= 0) {
                                            if let Some(&(wmap, wobject)) = level.by_hash.get(&mesh) {
                                                let winst = Instance { map: wmap, object: wobject, matrix: ident, placement: placement.clone() };
                                                add_instance(level, &winst, Layer::Character, Some((ci, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                                                let local = anim::quat_trans_to_mat(dt.q, dt.t);
                                                attachments.push(Attachment { character: ci, bone: dt.bone as usize, local, flash: None, tracer: None, dropped: false, script: None });
                                            } else { weapons_missing += 1; }
                                        } else { weapons_missing += 1; }
                                    }
                                }
                            }
                            // Triggers, AI volumes and emitters draw nothing (`Script_CreateEntity`).
                            nf_formats::nis::Cmd::Entity { hash, obj_type, .. } if !matches!(obj_type, 0xea | 0xeb | 0xf5 | 0xf2) => {
                                if !seen.insert((h, si, hash)) { continue; }
                                let Some(&(map, object)) = level.by_hash.get(&hash) else { continue };
                                let inst = Instance { map, object, matrix: ident, placement: placement.clone() };
                                add_instance(level, &inst, Layer::Character, Some((0, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
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
                add_instance(level, &tinst, Layer::Effect, Some((0, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                attachments.push(Attachment { character: 0, bone: 0, local: tinst.matrix, flash: None, tracer: Some(slot), dropped: false, script: None });
            }
        }
        for (inst, follows_camera) in &sky {
            add_instance(level, inst, if *follows_camera { Layer::SkyCamera } else { Layer::SkyFixed }, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels);
        }
        // Level coronas (`Corona_Create` 0x18da00, placement type 0xFC): class-1 objects with NO update
        // function — static glow quads at their placement transforms, in every view mode. The glist's +0x44
        // flags word (bit 0x1: `View_AddObjects` draw list 1, mirrored sort key) is the object's own flags
        // here, so `Layer::World` puts them in the alpha list with the leaves' own blend state [L 13/13].
        if let Some(m) = level.main_map() {
            for pl in nf_game::corona::placements(&level.maps[m].placements) {
                let p = &level.maps[m].placements[pl];
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                if let Some((map, object)) = target {
                    // `NF_HIDE` applies here too (the 0xFC skip above leaves these out of the loop's check).
                    if let Ok(h) = std::env::var("NF_HIDE") { if h.split(',').any(|n| !n.is_empty() && level.maps[map].objects[object].name.contains(n)) { continue; } }
                    let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                    add_instance(level, &inst, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                }
            }
        }
        // Wire visuals (`Wire_Create` 0x1981d8, placement type 0x3E): the wire's visual mesh is the placement's
        // object (the celglist), like coronas. The wire placement is a dynamic entity (no 0x8000 CREATE_CEL),
        // so it's filtered from the main instance loop; re-add it here [M 0x1981d8].
        if let Some(m) = level.main_map() {
            for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x3e) {
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                if let Some((map, object)) = target {
                    let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                    add_instance(level, &inst, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                }
            }
        }
        // Ladder visuals (`Ladder_Create` 0x1966c0, placement type 0x22, class 0x2D): the drawn mesh is the
        // placement's resolved object — `Ladder_Create` stores it at `obj+0xD8` (direct store 0x19671c, no
        // `Control_SetGList`), and the generic dynamic-object draw `View_DrawObjList` 0x1e69a0 →
        // `View_DrawObjectGlist` 0x1e6780 renders `obj+0xD8` at `obj+0x90` — the placement transform
        // `control_create_object` 0x1344e0 built (`View_RotTransMatrix` 0x1e5168). Class 0x2D has no update
        // function (docs/atlas/object.md control_funcs), so `obj+0x90` never changes: a static World-layer
        // mesh at the placement transform is the game's draw exactly. No ambient dim: `Ladder_Create` never
        // calls `Control_SetGList`, so `obj+0xF0 & 0x800` is clear and `View_SetupRenderModes_Tweak`
        // 0x1e5888 takes the white-tweak branch [M]. A ladder whose sphere straddles cels is drawn by
        // `View_AddForcedObjects` 0x1e5fb8 instead of the per-cel path — still exactly once [M
        // Controls_StraddleTest 0x134778].
        if let Some(m) = level.main_map() {
            for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x22) {
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                if let Some((map, object)) = target {
                    let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                    add_instance(level, &inst, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels);
                    ladders_drawn += 1;
                }
            }
        }
        // Searchlight parts (`Searchlight_Create`'s creation order = `part_placements`): each part's object as
        // Character-layer vertices posed by the simulation every tick — the mount and head rotating, the beam
        // cone along the aim, the spot on its ray hit, the corona a camera billboard (docs/systems/world-objects.md).
        if let Some(m) = level.main_map() {
            for (pi, pl) in nf_game::searchlight::part_placements(&level.maps[m].placements).into_iter().enumerate() {
                let p = &level.maps[m].placements[pl];
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                if let Some((map, object)) = target {
                    let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                    add_instance(level, &inst, Layer::Character, None, &mut groups, None, Some(pi as u16), None, None, [1.0; 3], &room_cels);
                }
            }
            // Doors (`Door_Create`'s creation order = `door_placements`): each door's object as Character-layer
            // vertices posed by the simulation every tick (`Door_Interp`'s matrix at `obj+0x90`); the mesh is
            // opaque, so the pass ordering is depth-correct against the world (docs/systems/world-objects.md).
            for (di, pl) in nf_game::door::door_placements(&level.maps[m].placements).into_iter().enumerate() {
                let p = &level.maps[m].placements[pl];
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                let Some((map, object)) = target else { continue };
                // `NF_HIDE` leaves the door out of the draw (the A/B check; the simulation still runs it).
                if let Ok(h) = std::env::var("NF_HIDE") { if h.split(',').any(|n| !n.is_empty() && level.maps[map].objects[object].name.contains(n)) { continue; } }
                let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                add_instance(level, &inst, Layer::Character, None, &mut groups, None, None, Some(di as u16), None, [1.0; 3], &room_cels);
            }
            // SimpleScripts (`SS_Create` 0x1a9c58 / `SS_Update` 0x1a9948, map-tower2): the animated 0xE2
            // placements (key count non-zero) ride the door slot space — their vertices pose from
            // `door_base + n_doors + k`, the SS list's matrices, every tick. At rest each sits at key 0
            // (= its placement transform [D]); the switch channel swaps it to the other key (the
            // 07000012 lift screens, shutters and lift doors).
            let n_doors = nf_game::door::door_placements(&level.maps[m].placements).len();
            // The same construction the sim's `Drones::attach_level` runs, so the slot order matches
            // the matrix upload's `sss.list` order (deterministic: placement order, same filter).
            let ss_list = nf_game::ss::SsList::create(&level.maps[m].placements, &level.path_blocks());
            for (si, ss) in ss_list.list.iter().enumerate() {
                let p = &level.maps[m].placements[ss.placement];
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                let Some((map, object)) = target else { continue };
                let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                add_instance(level, &inst, Layer::Character, None, &mut groups, None, None, Some((n_doors + si) as u16), None, [1.0; 3], &room_cels);
            }
            // Level-placed pickups (`Pickup_Create` 0x18dda0): `View_DrawObjectGlist` 0x1e6780 draws
            // `obj+0xD8` at `obj+0x90` — the same object carries the glist the renderer draws [M], so the
            // port draws them the way it draws doors: Character-layer vertices posed by the simulation every
            // tick at the pickup's matrix, collapsed once taken or hidden. Before fix-player2 only the
            // drone-dropped guns were drawn — a placed armour (kind 3, placed-only per the atlas) had no
            // instance at all and was invisible while fully takeable.
            for (pi, p) in placed_pickups.iter().enumerate() {
                if p.gone || p.hidden { continue; }
                // fix-objects: draw the RESOLVED object (`Pickup.obj` — the game draws obj+0xD8, the
                // celglist handed to `Pickup_Create`, not a hashcode re-lookup). The by-hash fallback
                // stays for pickups built without an object (tests) but cannot show an object whose
                // own hashcode is 0xFFFFFFFF (07000005's `oOBJECT_AmmoPickUp_9mm` [D]).
                let target = p.obj.or_else(|| level.by_hash.get(&p.model).copied());
                let Some((om, oi)) = target else { continue };
                let inst = Instance { map: om, object: oi, matrix: p.m, placement: level.maps[om].placements.first().cloned().unwrap_or_default() };
                add_instance(level, &inst, Layer::Character, None, &mut groups, None, None, None, Some(pi as u16), [1.0; 3], &room_cels);
            }
            // LOAD_LEVEL triggers (0xE8): the placement carries a visible mesh (e.g. 07000005's `Door_Gate8` —
            // the castle gate door — and the route-c zip-wire exit `Locked_Door_Top_I`). In the real game
            // `Trigger_LoadLevelCreate` (0x1b2950) → `Trigger_Create` (0x1b1720) calls `control_create_object`
            // which attaches the glist to `obj+0xF0`, so the trigger entity IS drawn. The port's `instances()`
            // skips these placements (no 0x8000 CREATE_CEL bit), so without this loop the castle gate door is
            // invisible in play — the user sees "nothing to interact with" at the gate. Draw them here as
            // static World-layer meshes at their placement transform (they don't animate) [M 0x1b2950, 0x1b1720].
            for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0xe8) {
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                let Some((map, object)) = target else { continue };
                if let Ok(h) = std::env::var("NF_HIDE") { if h.split(',').any(|n| !n.is_empty() && level.maps[map].objects[object].name.contains(n)) { continue; } }
                let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                add_instance(level, &inst, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels);
            }
        }
        let (mut vertices, mut indices, mut materials, mut char_verts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut char_vertex_start = None;
        for ((layer, _, _, key, draw), (vs, is, cv, centre, radius, cel)) in groups {
            if is.is_empty() { continue; }
            if matches!(layer, Layer::Character | Layer::Effect) && char_vertex_start.is_none() { char_vertex_start = Some(vertices.len()); }
            let base = vertices.len() as u32;
            materials.push(Material { layer, draw, centre, radius, cel, key, first_index: indices.len() as u32, index_count: is.len() as u32 });
            indices.extend(is.iter().map(|i| i + base));
            vertices.extend(vs);
            char_verts.extend(cv);
        }
        let keys: BTreeSet<(usize, u32)> = materials.iter().filter_map(|m| m.key).collect();
        // Bug 1 diagnostic: how many world materials have no texture (render as the white fallback). A high count on a
        // non-07000005 level would explain "ghostly white": texture × white vertex × GAIN → white.
        let untextured = materials.iter().filter(|m| m.key.is_none()).count();
        if untextured > 0 { eprintln!("render: {untextured} of {} materials have no texture (white fallback)", materials.len()); }
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
        let mut sim = drones.and_then(|d| Some((d, d.props.clone()?, d.weapons.clone()?))).and_then(|(d, props, weapons)| {
            let m = level.main_map()?;
            // Breakables (type 0x20) are drawn but not solid yet: their collision rules (glass that sight and bullets
            // pass or break) come with the Break_Create port.
            let world = nf_game::world::CollisionWorld::build(level, &level.instances(true).0);
            let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap_or(&Default::default()), &world, d.level_id);
            let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
            let hitboxes = d.hitboxes.clone()?;
            let volumes = nf_game::aivolume::build(level, d.level_id);
            let models = nf_game::pickup::models(level);
            let data = nf_game::drone::Data { tables: &d.tables, props: &props, scripts: &d.scripts, weapons: &weapons, world: &world, level: d.level_id, sequences: &seqs, skins: &level.skins, skeletons, nav: &nav, covers: &covers, tuning: &d.tuning, hitboxes: &hitboxes, volumes: &volumes, sfx: d.sfx.as_ref(), models: &models, surfaces: d.surfaces.as_ref(), voices: d.voices.as_deref() };
            let mut rng = nf_game::drone::Rng::new(0x1234_5678);
            // Every drone placement; spawner templates stay absent until a spawner brings them in.
            let list = nf_game::drone::level_drones(&data, level, &mut rng);
            let by_pos: HashMap<[u32; 3], usize> = list.iter().enumerate()
                .map(|(i, dr)| (level.maps[m].placements[dr.placement].translation.map(f32::to_bits), i)).collect();
            let char_drone = characters.iter().map(|c| by_pos.get(&[c.matrix[3][0], c.matrix[3][1], c.matrix[3][2]].map(f32::to_bits)).copied()).collect();
            // Per-character ambient state (`Lights_CalcAmbientLight` 0x1cb178): the 0xff,ff,ff sentinel of
            // `control_init_object` (0x134630) until the first cel link snaps it.
            let char_ambient = vec![[0xffu8; 3]; characters.len()];
            let char_script = characters.iter().map(|c| c.script).collect();
            let char_script_skin = characters.iter().map(|c| Some(c.skin.hashcode)).collect();
            let seq_index = by_hash.clone();
            // Bond at the "Player1" placement, on the floor below (`Player_StandAtNewPosition`: floor + 1.1).
            let (p0, yaw0) = start.unwrap_or(([0.0; 3], 0.0));
            let bond = nf_game::player::Bond::stand_at(&world, p0, yaw0);
            let mut drones = nf_game::drone::Drones::new(list, rng);
            drones.attach_level(level);
            // Ladders (`Ladder_Create` 0x1966c0, placement type 0x22, class 0x2D): fix-ladder. The world
            // sphere comes from the placement's object glist bounds, like the game's
            // `Control_BuildWorldSph` 0x133070.
            let ladders = nf_game::ladder::all(level);
            // Wires (`Wire_Create` 0x1981d8, placement type 0x3E): the level's zip wires, resolved
            // against their map objects like the ladders (fix-wire: the world sphere replaces the
            // hardcoded 4.0 length; the endpoints derive at grab time per the atlas).
            let wires = nf_game::wire::all(level);
            // The wire anims' root tracks: the level archive's kind-5 scripts (the player body anims)
            // resolved to their sequences [D 07000005: 0x060005e5 101 frames → 0x04000536, the move pair
            // 0x060000b4/b5 → 0x04000034/35 travelling 0.7637/0.7106 per cycle].
            let mut wire_anims = nf_game::wire::WireAnims::default();
            for h in [0x0600_05e5u32, 0x0600_05ec, 0x0600_05ed, 0x0600_05ee, 0x0600_05ea, 0x0600_00b4, 0x0600_00b5, 0x0600_0095, 0x0600_0096, 0x0600_0094] {
                if let Some(sc) = d.scripts.get(&h) {
                    for (_a, _b, q) in sc.sequences() {
                        if let Some(s) = seqs.get(&q) { wire_anims.tracks.insert(h, nf_game::wire::RootTrack::of(s)); break; }
                    }
                }
            }
            // Level-placed pickups (`Pickup_Create` 0x18dda0 via `parsemap_create_dynamic_objects` type 0xF0):
            // the placement's PARAMS array gives the type, id, rounds, switch, sound, respawn and label [M 0x1d04f0
            // line 459-462: reads from the param buffer at +0x2c..+0x38, NOT the placement's extra words].
            // The model comes from the placement's object (the celglist), not from weapon_data.
            let pickup_models = nf_game::pickup::models(level);
            let placed_pickups: Vec<nf_game::pickup::Pickup> = level.maps[m].placements.iter()
                .filter(|p| p.flags & 0xff == 0xf0)
                .filter_map(|p| {
                    // `parsemap_block_map_data_dynamic` collects the placement's params into a stack buffer;
                    // `parsemap_create_dynamic_objects` case 0xf0 reads from that buffer. The params are
                    // (index, value) pairs and parameter k lives at `level_tag+0x2C+4k` — looked up by
                    // INDEX, not by position [M 0x1d04f0:457-462; pickup-port-check §3]. An absent index
                    // reads 0.
                    let param = |idx: u32| -> u32 { p.params.iter().find(|(i, _)| *i == idx).map(|(_, v)| *v).unwrap_or(0) };
                    let type_ = param(0) as u16;
                    let id = param(1) as u16;
                    let rounds = param(2) as i16;
                    let switch_channel = param(3) as u16;
                    let sound = param(4) as u16;
                    let respawn = param(5) & 0xFFFF;
                    let lifetime = 0u8; // always 0 for level-placed pickups (rest=1 in the game)
                    // The model: the placement's object (the celglist passed to Pickup_Create, [M]
                    // 0x1d04f0:459-462). fix-objects: keep the RESOLVED object — the game draws the
                    // glist pointer, not a hashcode re-lookup, and an object with hashcode 0xFFFFFFFF
                    // (07000005's `oOBJECT_AmmoPickUp_9mm` [D]) has no by-hash entry, so the old
                    // `model`-hash fallback left those pickups undrawn. The bounds are the object's
                    // own (the glist's), not the first hashcode match's.
                    let m_map = m;
                    let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                        else if (p.index as usize) < level.maps[m_map].objects.len() { Some((m_map, p.index as usize)) } else { None };
                    let (model, bounds, obj) = target.and_then(|(om, oi)| {
                        let obj_ = &level.maps[om].objects[oi];
                        pickup_models.get(&obj_.hashcode).map(|&m| (obj_.hashcode,
                            nf_game::pickup::Model { centre: obj_.centre, radius: obj_.radius }, (om, oi)))
                    })?;
                    let mut p2 = nf_game::pickup::Pickup::create_placed(p.translation, p.rotation, type_, id, rounds, switch_channel, sound, respawn, lifetime, model, bounds, &weapons)?;
                    p2.obj = Some(obj);
                    Some(p2)
                })
                .collect();
            drones.pickups.extend(placed_pickups);
            Some(Sim { drones, tables: d.tables.clone(), props, scripts: d.scripts.clone(), weapons, world, level: d.level_id, char_drone, char_script, char_script_skin, seq_index, seqs, skins: level.skins.clone(), skeletons: skeletons.clone(), nav, covers, tuning: d.tuning.clone(), hitboxes, sfx: d.sfx.clone(), volumes, models, gun: nf_game::gun::Gun::new(2, 7, 41), bond, cursor: [0.0; 2], cross: [0.0; 2], view_script: 0, first_person: false, last_shot: None, shot_tick: false, view_time: 0.0, view_speed: 1.0, bob: [0.0; 2], aiming: false, auto_aim: false, zoom: 1.0, zoom_target: 1.0, disabled: false, surfaces: d.surfaces.clone(), voices: d.voices.clone(), step_dist: 0.0, wires, creeping: false, wire_anims, wire_channel: 0, ladders, view_model_ids: std::collections::HashSet::new(), cross_state: 2, cels: sim_cels, char_ambient, view_model_chars: Vec::new(), ambient_lookups: std::cell::Cell::new(0), lang: None, hud: Default::default() })
        });
        let fog = drones.and_then(|d| fog_for(d.level_id));
        let snow_texture = level.maps.iter().enumerate().find_map(|(mi, m)| m.textures.iter().find(|t| t.hashcode == 0x0300_0045).map(|t| {
            TextureImage::from_texture(mi, t)
        }));
        // Bug 3: build the collider name index for the "name what I'm touching" debug overlay (`C` key).
        let colliders: Vec<(String, [f32; 3], [f32; 3])> = instances.iter()
            .filter(|i| level.maps[i.map].objects[i.object].collision.is_some())
            .map(|i| {
                let obj = &level.maps[i.map].objects[i.object];
                let lo = transform(&i.matrix, obj.min);
                let hi = transform(&i.matrix, obj.max);
                (obj.name.trim().to_string(), [lo[0].min(hi[0]), lo[1].min(hi[1]), lo[2].min(hi[2])], [lo[0].max(hi[0]), lo[1].max(hi[1]), lo[2].max(hi[2])])
            }).collect();
        // Populate the Sim's view_model_ids from the view_models map (D-003).
        if let Some(sim) = sim.as_mut() {
            sim.view_model_ids = view_models.keys().copied().collect();
            // The view-model characters follow Bond's cel for their ambient ramp.
            sim.view_model_chars = view_models.values().copied().collect();
        }
        let models_0x800: HashMap<u32, bool> = level.maps.iter()
            .flat_map(|m| m.objects.iter().map(|o| (o.hashcode, o.flags & 0x800 != 0))).collect();
        Scene { cels: room_cels, cel_graph, streams: None, music_files: None, music_tables: None, sound_bank: None, snow_texture, vertices, indices, materials, textures, hud_textures, hud, view_models, fog, bounds, start, stats, characters, char_vertex_start, char_verts, sequences, attachments, sim, colliders, models_0x800, npc_pool, ladders_drawn }
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
    /// The per-frame matrices shared by the CPU pose (`animate`) and the GPU palette (`skin_palette`): each
    /// character's skin matrices, bone world matrices and object matrix (None = unposed), every attachment's
    /// matrix, and which characters are hidden (collapsed to their object origin).
    pub fn frame_state(&self, choice: Option<usize>, ticks: f32, opts: ViewOpts) -> FrameState {
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
            let view = self.sim.as_ref().and_then(|s| self.view_models.get(&s.gun.weapon).filter(|&&v| v == ci)).and(self.sim.as_ref());
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
                // `Player_PositionCamera` (0x18f5d8:100-106): hide the view model when the aim flag is set
                // AND the weapon is scoped (`+0x68 & 0x40`). `zoom_target > 1.0` ⇔ scoped && zoom held.
                let scoped_zoom = s.zoom_target > 1.0;
                return Some((mats, world, if s.first_person && !scoped_zoom { g } else { far }));
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
                // fix-objects: `!p.gone` — a TAKEN drop stays in the list as a tombstone (drone.rs
                // `update_pickups` keeps the Vec indices stable for the draw slots) and must not be
                // drawn; the game deleted the object (`Pickup_Handler` 0x18e650:338-348).
                let p = drone_of(a.character as usize).and_then(|(s, di)| s.drones.pickups.iter().find(|p| p.from == Some(di) && !p.hidden && !p.gone));
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
            if self.sim.as_ref().and_then(|s| self.view_models.get(&s.gun.weapon)) == Some(&(a.character as usize)) {
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
        let hidden: Vec<bool> = self.characters.iter().enumerate().map(|(ci, c)| hidden(c, ci)).collect();
        FrameState { posed, attach, hidden }
    }

    pub fn animate(&self, choice: Option<usize>, ticks: f32, opts: ViewOpts) -> Vec<[f32; 3]> {
        let FrameState { posed, attach, hidden } = self.frame_state(choice, ticks, opts);
        let hidden = |_: &Character, ci: usize| hidden[ci];
        self.char_verts.iter().map(|v| {
            // Searchlight parts: the simulation's matrices (`Searchlight_Update`), coronas as camera billboards
            // with their scale (the alpha byte is not wired to the shader — G-033); hidden parts collapse below
            // the world like the game's `View_SetDrawInNoViews`.
            if let Some(sl) = v.searchlight {
                const FAR: Mat4 = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
                let Some(s) = self.sim.as_ref() else { return [0.0, -10000.0, 0.0] };
                let Some(p) = s.drones.searchlights.parts.get(sl as usize) else { return [0.0, -10000.0, 0.0] };
                if !p.drawn { return transform(&FAR, v.pos); }
                if p.kind == 4 {
                    let m: Mat4 = [
                        [opts.right[0] * p.scale, opts.right[1] * p.scale, opts.right[2] * p.scale, 0.0],
                        [opts.up[0] * p.scale, opts.up[1] * p.scale, opts.up[2] * p.scale, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [p.pos[0], p.pos[1], p.pos[2], 1.0],
                    ];
                    return transform(&m, v.pos);
                }
                return transform(&p.matrix, v.pos);
            }
            // Level-placed pickups: the pickup's own matrix (`obj+0x90`), collapsed once taken or hidden
            // (`View_DrawObjectGlist` 0x1e6780 draws `obj+0xD8` at `obj+0x90`; fix-player2).
            if let Some(pk) = v.pickup {
                let Some(s) = self.sim.as_ref() else { return [0.0, -10000.0, 0.0]; };
                let Some(p) = s.drones.pickups.get(pk as usize).filter(|p| !p.gone && !p.hidden) else { return [0.0, -10000.0, 0.0]; };
                return transform(&p.m, v.pos);
            }
            // Doors: the simulation's pose (`Door_Interp`'s matrix at `obj+0x90`, docs/systems/world-objects.md).
            // Without the drone context (the sequence browser) there is no door state: hidden below the world,
            // like the searchlight parts.
            if let Some(dr) = v.door {
                let Some(s) = self.sim.as_ref() else { return [0.0, -10000.0, 0.0]; };
                // The door slot space also holds the SimpleScripts (map-tower2): indices past the doors
                // list index `drones.sss.list`.
                if let Some(d) = s.drones.doors.list.get(dr as usize) { return transform(&d.matrix, v.pos); }
                let Some(q) = s.drones.sss.list.get(dr as usize - s.drones.doors.list.len()) else { return [0.0, -10000.0, 0.0]; };
                return transform(&q.matrix, v.pos);
            }
            if let Some(a) = v.attachment { return transform(&attach[a as usize], v.pos); }
            let c = &self.characters[v.character as usize];
            if hidden(c, v.character as usize) { return [c.matrix[3][0], c.matrix[3][1], c.matrix[3][2]]; }
            match &posed[v.character as usize] {
                Some((mats, _, m)) => transform(m, anim::skin_vertex(v.pos, v.weight, v.slots, &c.slot_bones, mats)),
                None => transform(&c.matrix, v.pos),
            }
        }).collect()
    }

    /// The Character-layer vertex tail posed and colour-multiplied: `animate` gives the positions, and the rgba
    /// gets the per-object `TAmbient` multiplier of `View_SetupRenderModes_Tweak` (0x1e5888) — the dim every
    /// `obj+0xF0 & 0x800` object carries in the game. fix-render2: characters and view models keep the
    /// per-character ramp; placed pickups, dropped guns and 0x800-glist script entities (the truck) dim by the
    /// cel ambient at their live matrix; searchlight/door parts, the additive attachments (muzzle flash,
    /// tracers) and non-0x800 entities stay at 1.0 (doors carry no 0x800 glist flag on 07000005 [D — flags 0x0],
    /// so the game draws them untinted).
    /// The cel's frustum-test sphere (the bbox centre + half-diagonal): the shape `Vision_InView`
    /// guards in `View_AddCels` (0x1e6ba0) — the game tests each cel's bbox per viewer.
    pub fn cel_sphere(&self, i: usize) -> Option<([f32; 3], f32)> {
        let c = self.cels.get(i)?;
        let centre = [0, 1, 2].map(|k| (c.min[k] + c.max[k]) * 0.5);
        let r = [0, 1, 2].map(|k| (c.max[k] - c.min[k]) * 0.5);
        Some((centre, (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt()))
    }
    /// `build_FindCel` (0x1c8cb0) over the Scene's room cels — the same lookup the ambient code and
    /// the World-draw attribution use.
    pub fn cel_index_at(&self, p: [f32; 3]) -> Option<usize> { find_cel(&self.cels, p) }

    /// The GPU skinning layout (G-043): each Character-layer vertex's two palette matrices, blend weight and
    /// colour slot, plus the palette and colour-table sizes. The palette holds, in order: per character its
    /// object matrix then one matrix per skeleton bone (skin · object), then the attachments, searchlight
    /// parts, doors and placed pickups. The colour table: characters, attachments, pickups, then 1.0. The
    /// shader computes `w·(P[a]·p) + (1−w)·(P[b]·p)`, which equals the CPU path's `transform(m, skin_vertex(..))`
    /// because the blend weights sum to 1.
    pub fn skin_layout(&self) -> (SkinLayout, Vec<SkinAttr>) {
        let mut char_base = Vec::with_capacity(self.characters.len());
        let mut n = 0u32;
        for c in &self.characters { char_base.push(n); n += 1 + c.skeleton.bones as u32; }
        let max_of = |f: &dyn Fn(&CharVertex) -> Option<u32>| self.char_verts.iter().filter_map(|v| f(v)).max().map_or(0, |m| m + 1);
        let n_sl = max_of(&|v| v.searchlight.map(u32::from));
        let n_door = max_of(&|v| v.door.map(u32::from));
        let n_pick = max_of(&|v| v.pickup.map(u32::from));
        let n_att = self.attachments.len() as u32;
        let attach_base = n;
        let sl_base = attach_base + n_att;
        let door_base = sl_base + n_sl;
        let pick_base = door_base + n_door;
        let n_mats = pick_base + n_pick;
        let nc = self.characters.len() as u32;
        let (col_att, col_pick) = (nc, nc + n_att);
        let col_one = col_pick + n_pick;
        let attrs = self.char_verts.iter().map(|v| {
            let one = |m: u32, col: u32| SkinAttr { a: m, b: m, w: 1.0, col };
            if let Some(sl) = v.searchlight { return one(sl_base + sl as u32, col_one); }
            if let Some(pk) = v.pickup { return one(pick_base + pk as u32, col_pick + pk as u32); }
            if let Some(dr) = v.door { return one(door_base + dr as u32, col_one); }
            if let Some(a) = v.attachment { return one(attach_base + a, col_att + a); }
            let c = &self.characters[v.character as usize];
            let base = char_base[v.character as usize];
            let bone = |s: u16| c.slot_bones.get(s as usize).filter(|&&b| (b as u32) < c.skeleton.bones as u32).map(|&b| base + 1 + b as u32);
            match (bone(v.slots[0]), bone(v.slots[1])) {
                (Some(a), Some(b)) => SkinAttr { a, b, w: v.weight, col: v.character },
                (Some(a), None) => one(a, v.character),
                _ => one(base, v.character),
            }
        }).collect();
        (SkinLayout { char_base, attach_base, sl_base, door_base, pick_base, n_mats, col_att, col_pick, col_one }, attrs)
    }

    /// The muzzle-flash light of Bond's shot this tick, packed for the shader (position, 1/r², colour × intensity,
    /// active). `Player_MuzzleFlash` (0x1a4298, disassembly 0x1a4370-0x1a43a8) creates it on every shot:
    /// `Light_Create(radius = weapon+0x58, intensity 2.0, the view model's datum 0, rgb = +0x56/+0x55/+0x54,
    /// channel 0, life 0, kind 1, …)`. Life 0 → +0x20 = 1, so `Light_Update` deletes it on the next tick: one
    /// tick of light per shot. Not ported here: `Light_Create`'s `Rand_Rand(0)` draw and the `Gas_Init` smoke
    /// (game-RNG draws on every shot; fix-lighting). Render-side only: the simulation is unchanged.
    pub fn dyn_light(&self, time: f32, opts: ViewOpts) -> [f32; 8] {
        let Some(s) = self.sim.as_ref() else { return [0.0; 8] };
        if !s.shot_tick { return [0.0; 8]; }
        let w = s.gun.weapon as usize;
        let Some(rec) = s.weapons.records.get(w) else { return [0.0; 8] };
        let radius = f32::from_le_bytes([rec[0x58], rec[0x59], rec[0x5a], rec[0x5b]]);
        if !(radius > 0.0) { return [0.0; 8]; }
        let Some(&vm) = self.view_models.get(&s.gun.weapon) else { return [0.0; 8] };
        let fs = self.frame_state(None, time, opts);
        let Some(k) = self.attachments.iter().position(|a| a.character as usize == vm && a.flash.is_some()) else { return [0.0; 8] };
        let at = fs.attach[k][3];
        let i = 2.0 / 255.0;
        [at[0], at[1], at[2], 1.0 / (radius * radius), rec[0x56] as f32 * i, rec[0x55] as f32 * i, rec[0x54] as f32 * i, 1.0]
    }

    /// One frame's palette and colour table for `skin_layout` (the GPU replacement for `posed_vertices`).
    pub fn skin_palette(&self, l: &SkinLayout, choice: Option<usize>, time: f32, opts: ViewOpts) -> (Vec<Mat4>, Vec<[f32; 4]>) {
        const FAR: Mat4 = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
        let fs = self.frame_state(choice, time, opts);
        let collapse = |at: &Mat4| -> Mat4 { [[0.0; 4], [0.0; 4], [0.0; 4], [at[3][0], at[3][1], at[3][2], 1.0]] };
        let mut p = vec![FAR; l.n_mats as usize];
        for (ci, c) in self.characters.iter().enumerate() {
            let base = l.char_base[ci] as usize;
            let nb = c.skeleton.bones as usize;
            if fs.hidden[ci] { for k in 0..=nb { p[base + k] = collapse(&c.matrix); } continue; }
            match &fs.posed[ci] {
                Some((mats, _, m)) => {
                    p[base] = *m;
                    for b in 0..nb { p[base + 1 + b] = mats.get(b).map_or(*m, |s| anim::mul(s, m)); }
                }
                None => for k in 0..=nb { p[base + k] = c.matrix; },
            }
        }
        for (a, m) in fs.attach.iter().enumerate() { p[l.attach_base as usize + a] = *m; }
        if let Some(s) = self.sim.as_ref() {
            for k in 0..(l.door_base - l.sl_base) as usize {
                p[l.sl_base as usize + k] = match s.drones.searchlights.parts.get(k) {
                    Some(sp) if !sp.drawn => FAR,
                    Some(sp) if sp.kind == 4 => [
                        [opts.right[0] * sp.scale, opts.right[1] * sp.scale, opts.right[2] * sp.scale, 0.0],
                        [opts.up[0] * sp.scale, opts.up[1] * sp.scale, opts.up[2] * sp.scale, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [sp.pos[0], sp.pos[1], sp.pos[2], 1.0],
                    ],
                    Some(sp) => sp.matrix,
                    None => FAR,
                };
            }
            for k in 0..(l.pick_base - l.door_base) as usize {
                // The door region also holds the SimpleScripts' matrices (map-tower2): the ss vertices
                // index `n_doors + k`, and `pick_base − door_base` = max door-or-ss index + 1.
                p[l.door_base as usize + k] = match s.drones.doors.list.get(k) {
                    Some(d) => d.matrix,
                    None => s.drones.sss.list.get(k - s.drones.doors.list.len()).map_or(FAR, |q| q.matrix),
                };
            }
            for k in 0..(l.n_mats - l.pick_base) as usize {
                p[l.pick_base as usize + k] = s.drones.pickups.get(k).filter(|q| !q.gone && !q.hidden).map_or(FAR, |q| q.m);
            }
        }
        let (mults, pickup_mult, attach_mult) = self.colour_mults();
        let mut cols = vec![[1.0f32; 4]; l.col_one as usize + 1];
        let has = !mults.is_empty();
        let rgb = |m: [f32; 3]| [m[0], m[1], m[2], 1.0];
        if has {
            for (ci, m) in mults.iter().enumerate() { cols[ci] = rgb(*m); }
            for (a, m) in &attach_mult { if let Some(c) = cols.get_mut((l.col_att + a) as usize) { *c = rgb(*m); } }
            for (pk, m) in &pickup_mult { if let Some(c) = cols.get_mut((l.col_pick + *pk as u32) as usize) { *c = rgb(*m); } }
        }
        // Attachments without an entry fall back to their owner's multiplier, as `posed_vertices` does.
        if has {
            for (a, at) in self.attachments.iter().enumerate() {
                if !attach_mult.contains_key(&(a as u32)) {
                    cols[(l.col_att as usize) + a] = rgb(mults.get(at.character as usize).copied().unwrap_or([1.0; 3]));
                }
            }
        }
        (p, cols)
    }

    /// The per-object colour multipliers of one frame (`View_SetupRenderModes_Tweak`'s TAmbient): per character,
    /// per placed pickup and per attachment. Shared by the CPU path (`posed_vertices`) and the GPU palette.
    pub fn colour_mults(&self) -> (Vec<[f32; 3]>, std::collections::HashMap<u16, [f32; 3]>, std::collections::HashMap<u32, [f32; 3]>) {
        let mults: Vec<[f32; 3]> = self.sim.as_ref()
            .map(|s| (0..self.characters.len()).map(|ci| s.char_multiplier(ci)).collect())
            .unwrap_or_default();
        // fix-render3 (the 07000006 frame jitter): fix-render2 computed the pickup / dropped-gun /
        // script-entity ambient multipliers INSIDE the per-vertex closure — every vertex of a dimmed
        // character re-ran `find_cel` (a linear cel scan) and, for drops, a linear `pickups` scan.
        // The multiplier depends only on the pickup / attachment key, so it is computed once per
        // distinct key per frame and the vertex loop does a hash hit.
        let one = || [1.0; 3];
        let amb_at = |s: &Sim, m4: &Mat4| {
            s.ambient_at([m4[3][0], m4[3][1], m4[3][2]]).map(mul_bytes).unwrap_or_else(one)
        };
        let tintable = |p: &nf_game::pickup::Pickup| p.kind != nf_game::pickup::kind::ARMOUR && p.kind != nf_game::pickup::kind::BOND_BONUS;
        let mut pickup_mult: std::collections::HashMap<u16, [f32; 3]> = std::collections::HashMap::new();
        let mut attach_mult: std::collections::HashMap<u32, [f32; 3]> = std::collections::HashMap::new();
        if let Some(s) = self.sim.as_ref() {
            for cv in &self.char_verts {
                if let Some(pk) = cv.pickup {
                    pickup_mult.entry(pk).or_insert_with(|| {
                        match s.drones.pickups.get(pk as usize) {
                            Some(p) if tintable(p) => amb_at(s, &p.m),
                            // Armour glow (fix-player3): kind 3 carries the 0x80000 "close-up flash" flag
                            // (`Pickup_Create` 0x18dda0:145-151) — `View_SetupRenderModes_Tweak`
                            // (0x1e5888:22-44) multiplies its tint by `FlashMod` while the viewer's camera
                            // (viewer+0xC0) is within 4.0 units of the pickup's world-sphere centre
                            // (obj+0x80, squared distance < 16.0) and the options byte DAT_002a38d4 is set
                            // (1 at level start [L]). The port's camera is Bond's eye (first person, as in
                            // play); no corona, no additive pass — the pulse rides the same vertex-colour
                            // multiply as the ambient dim.
                            Some(p) if p.kind == nf_game::pickup::kind::ARMOUR => {
                                let c = p.centre();
                                let e = s.bond.head();
                                let d2 = (c[0] - e[0]).powi(2) + (c[1] - e[1]).powi(2) + (c[2] - e[2]).powi(2);
                                if d2 < 16.0 { [flash_mod(s.drones.frame); 3] } else { one() }
                            }
                            _ => one(),
                        }
                    });
                }
                if let Some(ai) = cv.attachment {
                    attach_mult.entry(ai).or_insert_with(|| {
                        let Some(a) = self.attachments.get(ai as usize) else { return one() };
                        if a.flash.is_some() || a.tracer.is_some() { return one(); }
                        if a.dropped {
                            // The dropped gun is a pickup object (`DroneWeap_DropWeapon` → `Pickup_Create`
                            // 0x18dda0, the same forced 0x840): the ambient dim at its drop matrix.
                            // fix-objects: `!p.gone` — taken drops stay as tombstones (see the pose step).
                            return match s.char_drone.get(a.character as usize).copied().flatten()
                                .and_then(|di| s.drones.pickups.iter().find(|p| p.from == Some(di) && !p.hidden && !p.gone)) {
                                Some(p) if tintable(p) => amb_at(s, &p.m),
                                _ => one(),
                            };
                        }
                        if let Some((h, si, hash)) = a.script {
                            // A script entity is its own object (the truck: nine 0x8880 objects [L]):
                            // 0x800-glist entities dim by the ambient at their live matrix; the others
                            // (coronas, triggers) draw untinted — not character 0's ambient.
                            return match s.script_entity((h, si), hash).map(|m4| amb_at(s, &m4)) {
                                Some(m) if self.models_0x800.get(&hash).copied().unwrap_or(false) => m,
                                _ => one(),
                            };
                        }
                        mults.get(a.character as usize).copied().unwrap_or_else(one)
                    });
                }
            }
        }
        (mults, pickup_mult, attach_mult)
    }

    pub fn posed_vertices(&self, choice: Option<usize>, time: f32, opts: ViewOpts) -> Vec<Vertex> {
        let (mults, pickup_mult, attach_mult) = self.colour_mults();
        let one = || [1.0f32; 3];
        self.animate(choice, time, opts).into_iter()
            .zip(self.vertices[self.char_vertex_start..].iter().zip(self.char_verts.iter()))
            .map(|(pos, (v, cv))| {
                let m = if mults.is_empty() { one() }
                    else if cv.searchlight.is_some() || cv.door.is_some() { one() }
                    else if let Some(pk) = cv.pickup {
                        // The pickup is its own object (`View_DrawObjectGlist` 0x1e6780 draws obj+0xD8);
                        // `Pickup_Create` (0x18dda0) forces obj+0xF0 = glist_flags & ~0x2000 | 0x840 — every
                        // pickup is character-lit — except kind 3 (armour) and kind 6 (007 bonus). fix-render3:
                        // the multiplier is precomputed per distinct pickup above (was per vertex).
                        pickup_mult.get(&pk).copied().unwrap_or_else(one)
                    }
                    else if let Some(ai) = cv.attachment {
                        // fix-render3: precomputed per distinct attachment above (flash/tracer → 1.0,
                        // dropped → its pickup's dim, script entity → the 0x800 dim, else the owner's).
                        attach_mult.get(&ai).copied().unwrap_or_else(one)
                    }
                    else { mults.get(cv.character as usize).copied().unwrap_or_else(one) };
                let rgba = if m == [1.0; 3] { v.rgba } else {
                    let rgb = [0, 1, 2].map(|k| ((v.rgba[k] as f32 * m[k]).round() as i32).clamp(0, 255) as u8);
                    [rgb[0], rgb[1], rgb[2], v.rgba[3]]
                };
                Vertex { pos, uv: v.uv, rgba, nrm: v.nrm }
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
