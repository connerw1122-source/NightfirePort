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

/// Test export of `remap_op1_sound` (the `map_analog_stick_for_test` pattern).
pub fn remap_op1_sound_for_test(id: u32, weapon: u8) -> u32 { remap_op1_sound(id, weapon) }

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
/// blend from ALPHA_1 (0 normal (Cs − Cd)·As + Cd, 1 additive Cs·As + Cd, 2 subtractive Cd − Cs·As, 3 keep-destination
/// — the FIX=0 mod batches, 4 additive by destination alpha Cs·Ad + Cd — the env-map overlays), depth write
/// (ZBUF_1 ZMSK clear), the alpha test from TEST_1 (0 none, 1 As ≥ 1, 2 As > 0x7F) and the fog (0 none, 1 depth fog,
/// 2 alpha fade with distance).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Draw { pub pass: u8, pub blend: u8, pub zwrite: bool, pub atest: u8, pub fog: u8 }

impl Draw {
    /// From a leaf's GS registers (ALPHA_1, TEST_1, ZBUF_1 as last written).
    ///
    /// ALPHA_1 fields (ps2tek GS register spec, *sourced*): Output = ((A − B)·C >> 7) + D with
    /// A/B/D ∈ {0: Cs, 1: Cd, 2: 0} and C ∈ {0: As, 1: Ad, 2: FIX} (FIX = bits 8-15 of the register).
    /// *Measured* over every DIRECT A+D write of all 27 on-foot levels: only ALPHA_1 (0x42), TEST_1
    /// (0x47) and ZBUF_1 (0x4e) ever occur, so CLAMP/filtering are global texture-upload state, not
    /// per-object. The disc carries exactly six A/B/C/D shapes:
    /// - 0x44 (0,1,0): (Cs−Cd)·As+Cd → blend 0 (normal) — 179,862 writes
    /// - 0x48 (0,2,0): (Cs−0)·As+Cd → blend 1 (additive) — 28,925 writes
    /// - 0x42 (2,0,0): (0−Cs)·As+Cd → blend 2 (subtractive) — 312 writes
    /// - 0x58 (0,2,1): (Cs−0)·Ad+Cd → blend 4 (additive gated by the framebuffer's alpha — the
    ///   "EnvModTest_*" env-map overlay of 07000041 adds only where the base batch wrote alpha) — 159 writes
    /// - 0x64 (0,1,2, FIX=0): (Cs−Cd)·0+Cd → blend 3 (keep-destination; the GS still depth-tests and
    ///   alpha-tests) — 159 writes
    /// Nothing in the load/draw path patches FIX (psiCreateEntityGfx 0x1171b0 rewrites only texture
    /// REF addresses; DrawThisBox 0x1230d8 only kicks the packet), so the disc values are final [M].
    pub fn from_gs(pass: u8, fog: u8, alpha: u64, test: u64, zbuf: u64) -> Draw {
        let lo = alpha & 0xff;
        let (a, b, c, fix) = (lo & 3, (lo >> 2) & 3, (lo >> 4) & 3, (alpha >> 8) & 0xff);
        let blend = match (a, b, c, fix) {
            (0, 1, 0, _) => 0, // (Cs−Cd)·As+Cd — normal
            (0, 2, 0, _) => 1, // (Cs−0)·As+Cd — additive
            (2, 0, 0, _) => 2, // (0−Cs)·As+Cd — subtractive
            (0, 2, 1, _) => 4, // (Cs−0)·Ad+Cd — additive by destination alpha (07000041 env maps)
            // C = FIX 0: the equation collapses to +D (0x64 → Cd). Paint nothing; the GS still runs
            // the depth test/write and the alpha test on these batches.
            (_, _, 2, 0) => 3,
            _ => 0, // no other shape occurs on the disc
        };
        // TEST_1: bit 0 ATE; bits 1-3 ATST (5=GEQUAL, 6=GREATER on the disc); bits 4-11 AREF.
        // GEQUAL 0 and ALWAYS pass everything — no test (the port used to reject a = 0 texels there,
        // which also skips their depth write, a behaviour the game only has for AFAIL ≠ 0).
        let atest = if test & 1 == 0 { 0 } else {
            let (atst, aref) = ((test >> 1) & 7, (test >> 4) & 0xff);
            match atst {
                0 | 1 => 0,
                5 => match aref { 0 => 0, 1 => 1, _ => 2 },
                6 => if aref >= 0x7f { 2 } else { 1 },
                _ => 1,
            }
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
    /// The list-0 sort key source. `View_AddCels` (0x1e6ba0, disassembled) pushes the −90000 + 10k
    /// keys on the **+0x20 chain** `Vision_Portal_Recurse` (0x1e86e8) threads per frame from
    /// `VisCelList` (0x34b3b0) — `QuickSort` (0x1d30a8) comparator[3] @0x1d3430 orders it by the
    /// ascending u16 portal-recursion depth at element+4 — and only the **room cels** (placement
    /// flag words 0xc023/0xc047) ride it [L ee_start_05.bin: 19 rooms + 113 inner cels]. The cel
    /// `+0x18` inner nodes — every non-room 0x8000 placement (0x8019/0x8018 marker classes,
    /// `parseentity_fixup_entity` 0x1d00c0) — get **±dist²** keys (list by flags bit 0x1/0x2000).
    /// So `chain` = the room's fallback ordinal (its rank among the room placements in parse
    /// order, used when the per-frame walk cannot run); `u32::MAX` = a distance-keyed entry (the
    /// inner-node statics: the baked decals/walkways/blockers, and the dynamic objects —
    /// `View_AddObjects` 0x1e62d8 pushes `{±dist², obj, 0}`).
    pub chain: u32,
    /// The draw's own runtime cel in the `CelGraph::cells` sense — the placement's pCelList record
    /// identity — for the room-class placements whose primary glist rides the −90000 + 10k chain;
    /// `u32::MAX` = a distance-keyed entry (never looked up in the per-frame chain). The renderer
    /// maps `node` → the cel's position in the depth-sorted `VisCelList` chain each frame
    /// (`gpu::render`); `chain` is the fallback ordinal when the walk cannot run.
    pub node: u32,
    /// `(map, texture)`; `None` = untextured.
    pub key: Option<(usize, u32)>,
    /// The draw's light-selection flags: the low 16 bits of the source object's glist flags word
    /// (`MapObject.flags` = `celglist+0x44`, which `Control_SetGList` 0x134960 copies to `obj+0xF0`;
    /// cels: the same word lands in `cel+0x94` — `lights.rs`' `closest` reads bit 0x80 for kinds 0/2
    /// and bit 0x100 to skip kind 1). fix-lighting2: the port used to pass 0x80 for every draw, which
    /// let 07000005's kind-0 ambient radiators crowd the muzzle light off the world draws — the disc
    /// data has almost no 0x80/0x100 on world cels, 0x8a0/0x28a0 on character/weapon skins, 0x100 on
    /// effect objects [D probe in lighting_live.rs].
    pub lflags: u16,
    pub first_index: u32,
    pub index_count: u32,
}

/// fix-menu-movies: `Clone` so the front end can keep its own texture list (the scene's entries
/// plus the background-movie frame slot) without re-parsing the archive.
#[derive(Clone)]
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
    /// fix-facial2: the object's facial morph TARGET array (the MORPH_DATA block 0x2B payload — the
    /// runtime gfx record's +0x14 base [M `parsemap_block_morph_data` 0x1d23e8]), f32[3] per
    /// (vertex morph index + channel). `SkinIt` 0x10bf30 adds Σ weight[m] × target[morph + m].
    pub morph_targets: Option<Vec<u8>>,
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
    /// The fire trigger's last-tick state — the press edge the watch pair 0x54/0x55's mode toggle ORs
    /// in (`Player_Weapon` 0x1a24c8's `Input_Action(slot, 9, 4)`) (fix-weapon-modes).
    pub prev_trigger: bool,
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
    /// The laser-family beam's endpoints this tick (the hand/head end, the hit end) —
    /// `gadget::laser_tick` while a laser weapon is out [M Player_LaserPointer 0x1a65d8]. The
    /// renderer draws the beam mesh (0x2000131/0x2000133) between them.
    pub laser_beam: Option<(nf_game::world::V3, nf_game::world::V3)>,
    /// The stunner's per-tick target test [M Player_WeaponInitBullet 0x1a4bf0:169-200 — BLData+0x95d]:
    /// `Some(true)` = the aim ray met a live guard within weapon_data+0x1c (the shot spawns and the
    /// taser beam draws), `Some(false)` = no target (no bullet, no beam), `None` = not holding fire.
    pub stunner_target: Option<bool>,
    /// The level's zip wires (placement type 0x3E, `Wire_Create` 0x1981d8). Bond grabs one by walking
    /// into its world sphere (fix-wire: the real grab + crossing live in `nf_game::player`).
    pub wires: Vec<nf_game::wire::Wire>,
    /// The level's creepwalls (class 0x2B, placement 0x3F) — the crawlable vents (fix-swim-vent).
    pub creepwalls: Vec<nf_game::creepwall::CreepWall>,
    /// The level's third icons (placement kind 0xFA, `ThirdIcon_Create` 0x197070). Each tick the Sim
    /// writes the enabled icon's context into `bond.icon_ctx` (`BLData+0x95F`) — fix-wire2.
    pub icons: Vec<nf_game::wire::Icon>,
    /// Each wire's owning cel (the smallest-volume `cels` bbox containing the wire's world-sphere
    /// centre — `build_FindCel` 0x1c8cb0's test). The game's pick pass is CEL-CULLED:
    /// `Collide_Pick` 0x1eb320 tests only the cels the query segment straddles
    /// (`Collide_StraddleCels`) and each cel's own object list, and every dynamic object is created
    /// with its enclosing cel's glist (`parsemap_create_dynamic_objects` 0x1d04f0 passes the
    /// celglist to `Wire_Create`) — a wire is pickable only in its own cel. The port assigns the cel
    /// spatially [S — the parse stream's placement→cel parentage is not recovered]; `None` (centre
    /// inside no bbox) keeps the unculled scan. Without the gate 07000002's courtyard wire (world
    /// sphere r 11.5 at (−63.1, 8.1, −34.5)) swallows Bond the moment he enters the secret bunker
    /// (a different cel), making the bunker's touch plates unreachable.
    pub wire_cels: Vec<Option<usize>>,
    /// The runtime cel graph (a build clone) — `find_cell_raycast`'s per-tick input for the third
    /// icons' arm test (fix-08-walls: the arm region is the game's straddle-cel set, `Collide_StraddleCels`
    /// 0x1eac98, resolved over this graph at load into `icon_cels`).
    pub cel_graph: CelGraph,
    /// Per third icon, the straddled `cells` indices — the game's obj+0xd0 chain occupancy set
    /// [M `Collide_Update` 0x1ea398; computed by `icon_straddle_cels` at load]. Empty = the icon
    /// never arms (its `build_FindCel` cel did not resolve — the game's unlinked icon gets no
    /// occupants either [M 0x1c9680:47]).
    pub icon_cels: Vec<Vec<usize>>,
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
    /// The room cel → pCelList cell map (`cel_graph.room_to_cell`), the actor queries' start-cell
    /// lookup (obj+0x20 — fix-safe08b).
    pub room_cell: Vec<Option<usize>>,
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
    /// The muzzle smoke puffs (`Gas_Init` 0x1aacc8; fix-lighting: the RNG draw rides the shared
    /// generator in the game's order, the puff itself is a [S] rise+fade model).
    pub smoke: Vec<nf_game::light::Smoke>,
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
/// `life` is the time the node was created with (`Text_AddMsg`'s scaled tick count); `opened`/
/// `second` track the pane's per-display SFX (`ObjectiveSFXTriggered1/2` 0x30ccb4/0x30ccb8 —
/// reset by case 2 each time the message becomes the displayed head [M 0x19c310:148-164]).
#[derive(Clone, Debug)]
pub struct Msg { pub text: String, pub time: f32, pub status: u32, pub failed: bool, pub life: f32, pub opened: bool, pub second: bool }

impl Msg {
    pub fn new(text: String, time: f32) -> Msg { Msg { text, time, status: 0, failed: false, life: time, opened: false, second: false } }
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

    /// fix-laser-beam: stop every LOOPED op-1 sound of `weapon`'s fire + release scripts (the
    /// Laser/Stunner's beam loops 0x12/0x3d [L 0x060004e9/0x0600048a op-1, +0x060004e8/0x06000488
    /// op-4.3]). Called on the weapon switch (the held state 0xb never reaches the release script's
    /// stop) and from the pause path — the game's `MenuManager_Create` runs `SFXPause` +
    /// `SFXPauseAllStreams` [M 0x21b908:120-123], which silences the loop while the menu is up.
    pub fn gadget_loop_ids(&self, weapon: u8) -> Vec<u32> {
        let scripts = [self.weapons.records.get(weapon as usize).map_or(0, |r| u32::from_le_bytes(r[0xb0..0xb4].try_into().unwrap())),
                       self.weapons.records.get(weapon as usize).map_or(0, |r| u32::from_le_bytes(r[0xb4..0xb8].try_into().unwrap()))];
        let mut ids = Vec::new();
        for h in scripts.into_iter().filter(|&h| h != 0) {
            if let Some(sc) = self.scripts.get(&h) {
                for c in &sc.commands {
                    if c.op == 1 && c.args.len() >= 2 {
                        let id = remap_op1_sound(c.args[1] as u32, weapon);
                        if self.sfx.as_ref().map(|t| t.get(id)).is_some_and(|e| e.looped) { ids.push(id); }
                    }
                }
            }
        }
        ids
    }
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

/// `vecutil_Dist2Tri` (the distance the game's `Intersect_Portal` 0x1ea7b8 version-0x101 path
/// thresholds against the straddle sphere's radius): the Euclidean distance from `p` to the
/// triangle (a, b, c) — the standard closest-point-on-triangle construction.
fn dist_point_tri(p: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let sub = |u: [f32; 3], v: [f32; 3]| [u[0] - v[0], u[1] - v[1], u[2] - v[2]];
    let dot = |u: [f32; 3], v: [f32; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 { return dot(ap, ap).sqrt(); }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 { return dot(bp, bp).sqrt(); }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 { let v = d1 / (d1 - d3); return dot(ap, [ab[0] * v, ab[1] * v, ab[2] * v]).sqrt(); }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 { return dot(cp, cp).sqrt(); }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 { let w = d2 / (d2 - d6); return dot(ap, [ac[0] * w, ac[1] * w, ac[2] * w]).sqrt(); }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bc = sub(c, b);
        return dot(bp, [bc[0] * w, bc[1] * w, bc[2] * w]).sqrt();
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let closest = [a[0] + ab[0] * v + ac[0] * w, a[1] + ab[1] * v + ac[1] * w, a[2] + ab[2] * v + ac[2] * w];
    dot(sub(p, closest), sub(p, closest)).sqrt()
}

/// The icon's straddle-cel set — the port of `Collide_StraddleCels` (0x1eac98) as
/// `Controls_StraddleTest` (0x134778) drives it for the third icons: starting at the object's own
/// cel (the room `build_FindCel` 0x1c8cb0 links at load, via `room_to_cell`), a BFS over the
/// cel graph's portals adds each neighbour whose portal quad the object's world sphere
/// intersects (`Intersect_Portal` 0x1ea7b8, HITTEST version 0x101: the portal AABB first, then
/// `vecutil_Dist2Tri` from the sphere centre to the portal's two triangles < the radius), up to
/// the game's 0x7e-cel cap. `Collide_Update` (0x1ea398) leaves a hitdata node on every object
/// occupying a straddled cel, and `ThirdIcon_Update` (0x196e80) writes the icon's context into
/// the player's +0x95F off that node — so this set IS the icon's arm region. A wall between the
/// icon and the player keeps them in different cels with no portal for the small sphere to walk
/// through (07000008's guardhouse wall: the icon's r-1.164 sphere crosses no portal), which the
/// old flat 12.0-unit arm radius ignored.
///
/// Returns the straddled `cells` indices (always containing the start cell when it resolves).
pub fn icon_straddle_cels(graph: &CelGraph, start_cell: usize, centre: [f32; 3], radius: f32) -> Vec<usize> {
    let mut seen = vec![false; graph.cells.len()];
    let mut out = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    seen[start_cell] = true;
    out.push(start_cell);
    queue.push_back(start_cell);
    while let Some(c) = queue.pop_front() {
        if out.len() >= 0x7e { break; }
        for p in &graph.portals {
            let other = if p.cel_a == c { Some(p.cel_b) } else if p.cel_b == c { Some(p.cel_a) } else { None };
            let Some(other) = other else { continue };
            if seen[other] { continue; }
            // Intersect_Portal 0x101, step 1: the portal quad's AABB against the sphere's AABB.
            let (mut mn, mut mx) = (p.corners[0], p.corners[0]);
            for corner in &p.corners[1..] {
                for k in 0..3 { mn[k] = mn[k].min(corner[k]); mx[k] = mx[k].max(corner[k]); }
            }
            if centre[0] + radius < mn[0] || centre[0] - radius > mx[0]
                || centre[1] + radius < mn[1] || centre[1] - radius > mx[1]
                || centre[2] + radius < mn[2] || centre[2] - radius > mx[2] { continue; }
            // Step 2: Dist2Tri from the sphere centre to the quad's two triangles < radius.
            let (a, b, c2, d) = (p.corners[0], p.corners[1], p.corners[2], p.corners[3]);
            let hit = dist_point_tri(centre, a, b, c2) < radius || dist_point_tri(centre, a, c2, d) < radius;
            if hit {
                seen[other] = true;
                out.push(other);
                queue.push_back(other);
            }
        }
    }
    out
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
    /// `cel+0x98` (s16): the room's reverb environment [M 0x1d00c0, fix-room-sound]. `parseentity_fixup_entity`
    /// writes 0x9898 (s16 −26472, the IOP's underwater sentinel) for flag word 0xC023 and placement param 9
    /// (`level_tag+0x50`, a u16 with no mask or clamp) for 0xC047; a cel without the fixup keeps the 0x98
    /// allocator fill in both bytes, which reads back as the same 0x9898 sentinel [L ee_start_05.bin: the
    /// untouched cel bytes at +0xa4..0xaf are 0x98]. `Sound_UpdateListeners` 0x1dd278 sends it to
    /// `SFXSetEnvironment` from the active viewer's cel every frame.
    pub env: i16,
    /// `cel+0x3c & 0x10000000` [M 0x1d00c0]: the underwater flag — placement param 3 (`level_tag+0x38`,
    /// a full u32) != 0, written for both room-cel classes. `Sound_UpdateListeners` ORs it with `env >= 20`
    /// (a signed s16 compare) into `SFXSetEnvironment`'s underwater argument.
    pub underwater: bool,
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
    /// `View_AddCels` (0x1e6ba0)'s `cel+0x18` chain per cell. The chain threads through the cel
    /// records' +0x18 field [M 0x1e6ba0: the head is the VisCelList cel's +0x18, each node's next
    /// is node+0x18]; only room-class cels own chains and only the generic (runtime class 0x8019/
    /// 0x8018) cels ride them as nodes. Built by `build_chains`: each generic hangs on the room
    /// `build_FindCel` (0x1c8cb0) picks for its world-sphere centre — the bbox candidates plus the
    /// collision up/down ray tie-break [L ee_start_05.bin + ee_snapshot.bin: 8 room owners, 113
    /// members, 0 members outside their owner's bbox, 65/65 multi-bbox owners matching the
    /// FindCel tie-break; L live 07000006: the eye cel's chain empty, the big room 0x0179fb30's
    /// chain = 40 generics all inside its bbox]. Node order within a chain is reverse parse
    /// order [L ground-truth doc: the 40-node dump descends by runtime address] — the builder
    /// prepends. fix-portal4 (the old per-map-block parse-order model is refuted by that data).
    pub chains: Vec<Vec<u32>>,
    /// The main map's placements' own `cells` index, by placement ordinal (`None` = the placement
    /// does not create a cel). A cel placement's glist draws when ITS cel is visible
    /// (`View_AddCels` 0x1e6ba0), not when the room its centre lands in is. fix-portal2.
    pub own_cells_main: Vec<Option<u32>>,
}

#[derive(Clone, Copy)]
pub struct CelNode {
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// The runtime cel's flags word (`cel+0x3c`), derived per class by `parseentity_fixup_entity`
    /// 0x1d00c0: the 0xc023/0xc047 room classes become **0x804c023/0x804c047** (bit 0x40000 = the
    /// room bit `build_FindCel` 0x1c8cb0 collects; the placement's raw word 0xc047 does NOT carry
    /// it), the generic classes 0x8019. fix-portal.
    pub flags: u32,
    /// The cel's world (centre, radius) sphere — `cel+0x80..0x8c`, the pCelList record's +0x10..0x1c
    /// local sphere transformed by the placement matrix (`parseentity_transform_bounding_box`
    /// 0x1d0fb0 [M]; the port derives it from the placement-transformed bbox corners). This is what
    /// `Vision_InView` (0x1e89e0) tests for the `+0x18` chain pass; `min`/`max` stay raw (the game's
    /// `build_FindCel` 0x1c8cb0 tests the raw glist bbox copied to cel+0x40..0x58). fix-portal2.
    pub sphere: ([f32; 3], f32),
}

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
/// The collision scope graph from the parsed cel graph (fix-safe08b): plain bboxes + the portals as
/// the two triangles `Intersect_Portal` (0x1ea7b8) tests, for `CollisionWorld::build_scoped`.
fn scope_graph(g: &CelGraph) -> nf_game::world::ScopeGraph {
    nf_game::world::ScopeGraph {
        cells: g.cells.iter().map(|c| (c.min, c.max)).collect(),
        portals: g.portals.iter().map(|p| nf_game::world::ScopePortal {
            tris: [[p.corners[0], p.corners[1], p.corners[2]], [p.corners[0], p.corners[2], p.corners[3]]],
            a: p.cel_a, b: p.cel_b,
        }).collect(),
    }
}

/// The runtime cel's `+0x3c` flags word for a placement whose low 16 flag bits are `flags16`
/// [M `parseentity_fixup_entity` 0x1d00c0: the 0xc023 → 0x804c023, 0xc047 → 0x804c047, the
/// 0xd8..0xf3/0x8017..0x8019 generics → 0x8019, 0x803d → 0x28019; the unlisted words leave the
/// `build_alloc_cel` 0x1c93f8 zero]. The port's reads need the room bit (0x40000) only.
fn cel_flags_word(flags16: u16) -> u32 {
    match flags16 {
        0xc023 | 0xc047 => 0x804c000 | flags16 as u32,
        0x803d => 0x28019,
        0xf | 0x20 | 0x4e | 0xd8..=0xf3 | 0x8017..=0x8019 => 0x8019,
        _ => 0,
    }
}

pub fn parse_cel_graph(level: &Level) -> (Vec<Cel>, CelGraph) {
    let main = level.main_map();
    let mut cells: Vec<CelNode> = Vec::new();
    let mut rooms: Vec<Cel> = Vec::new();
    let mut room_to_cell: Vec<Option<usize>> = Vec::new();
    let mut cell_of_record: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut chains: Vec<Vec<u32>> = Vec::new();
    let mut own_cells_main: Vec<Option<u32>> = Vec::new();
    for (mi, map) in level.maps.iter().enumerate() {
        for p in &map.placements {
            if !p.creates_cel() {
                if Some(mi) == main { own_cells_main.push(None); }
                continue;
            }
            let flags16 = (p.flags & 0xffff) as u16;
            let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v as u8).unwrap_or(0);
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < map.objects.len() { Some((mi, p.index as usize)) } else { None };
            let is_main = Some(mi) == main;
            let idx = cells.len();
            // The raw param values: the fixup reads param 3 (`level_tag+0x38`) as a full u32 and
            // param 9 (`level_tag+0x50`) as a u16 — neither through the low-byte view of `param`.
            let param32 = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
            let node_flags = cel_flags_word(flags16);
            // The world sphere: the object's raw bbox corners through the placement matrix, then the
            // AABB's centre / half-diagonal (`parseentity_transform_bounding_box` 0x1d0fb0).
            // (Merge resolution: this block landed on master with two corrupted lines — the `[m`/`[h`
            // byte pairs dropped from the corner-array and half-diagonal literals; restored here.)
            let sphere = |min: [f32; 3], max: [f32; 3]| {
                let m = nf_formats::placement::object_matrix(p.rotation, p.translation);
                let mut lo = [f32::MAX; 3];
                let mut hi = [f32::MIN; 3];
                for cx in [min[0], max[0]] { for cy in [min[1], max[1]] { for cz in [min[2], max[2]] {
                    let w = nf_formats::placement::transform(&m, [cx, cy, cz]);
                    for k in 0..3 { lo[k] = lo[k].min(w[k]); hi[k] = hi[k].max(w[k]); }
                } } }
                let c3 = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5, (lo[2] + hi[2]) * 0.5];
                let d = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
                (c3, (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() * 0.5)
            };
            match obj {
                Some((om, oi)) => {
                    let o = &level.maps[om].objects[oi];
                    let sp = sphere(o.min, o.max);
                    cells.push(CelNode { min: o.min, max: o.max, flags: node_flags, sphere: sp });
                    if is_main && matches!(flags16, 0xc023 | 0xc047) {
                        let mut a0 = [param(0), param(1), param(2)];
                        if a0 == [0, 0, 0] { a0 = [0x3c; 3]; }
                        let a1 = if flags16 == 0xc047 {
                            let b = [param(6), param(7), param(8)];
                            if b == [0, 0, 0] { a0 } else { b }
                        } else { a0 };
                        // `parseentity_fixup_entity` 0x1d00c0: +0x98 = 0x9898 for 0xc023 (the
                        // underwater sentinel), param 9 as a raw u16 for 0xc047; the 0x10000000
                        // flags bit (the underwater flag) = param 3 != 0, both classes.
                        let env = if flags16 == 0xc023 { -0x6768i16 } else { param32(9) as u16 as i16 };
                        rooms.push(Cel { min: o.min, max: o.max, ambient: [a0, a1], zone: if flags16 == 0xc047 { param(5) } else { 0 },
                            water: if flags16 == 0xc023 { o.max[1] } else { -500.0 },
                            env, underwater: param32(3) != 0 });
                        room_to_cell.push(Some(idx));
                    }
                }
                None => { cells.push(CelNode { min: [0.0; 3], max: [0.0; 3], flags: cel_flags_word(flags16), sphere: ([0.0; 3], 0.0) }); }
            }
            chains.push(Vec::new());
            // (The chains are built AFTER the parse by `build_chains` — the game's own builder
            // hangs each generic on the room `build_FindCel` 0x1c8cb0 picks for its centre, a
            // collision raycast tie-break that needs the whole cel set + the scoped world.)
            if is_main { own_cells_main.push(Some(idx as u32)); }
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
    (rooms, CelGraph { cells, portals, room_to_cell, dropped_portals: dropped, chains, own_cells_main })
}

/// The `+0x18` chain builder (fix-portal4, the owner rule fix-portal5). The game hangs each generic
/// cel (runtime class 0x8019/0x8018 — the placement flag words `cel_flags_word` maps there) on a
/// room's chain at LOAD time by `build_link_objects_to_rooms` (0x1c9150): the owner is picked among
/// the ROOM-class cels (short class 0xC047/0xC023) whose radius ≥ the generic's and whose world bbox
/// contains the generic's sphere centre, by `Collide_SphereIntersect` (0x1ec940) — a sphere at the
/// generic's centre with radius 4001 (containing), scoped to the candidate room, i.e. the candidate
/// whose collision sits CLOSEST to the centre; strict `<` keeps the first candidate in parse order.
/// The port's `find_cell_raycast` (the runtime `build_FindCel` 0x1c8cb0 two-ray rule) is the EYE-cel
/// picker, not the linker — as a stand-in it hung the 9 courtyard extras on the big room 21 [L live
/// ground truth: the game's 0x0179fb30 chain is exactly the 40 named courtyard generics] and the
/// wall generics on the wrong rooms. Evidence [L ee_start_05.bin + ee_snapshot.bin, both walked
/// +0x18-head/next]: 8 room owners / 113 generic members / 0 members outside the owner's world bbox
/// (+0x40 min vec4, +0x50 max vec4) / no orphan generic (every 0x8019-class cel rides some chain).
/// Live [L ground-truth doc]: the eye cel's chain is empty, the big room 0x0179fb30's chain is
/// exactly the 40 courtyard generics. `world` (the sim's scoped `CollisionWorld`) supplies the
/// closest-hit tie-break (and the radius/bbox gates stand alone); `None` keeps `find_cell_raycast`'s
/// bbox-face approximation (the world-less callers). Nodes are prepended so a chain reads in reverse
/// parse order — the live dump order (the game walks its +0x14 list in parse order, prepending).
pub fn build_chains(graph: &mut CelGraph, world: Option<&nf_game::world::CollisionWorld>) {
    let mut chains = std::mem::take(&mut graph.chains);
    for i in 0..graph.cells.len() {
        let f16 = graph.cells[i].flags & 0xffff;
        if !matches!(f16, 0x8018 | 0x8019) { continue; }
        let node = &graph.cells[i];
        // The port's unresolvable-object cells (zero bbox/sphere) stand for placements the parse
        // could not resolve — the game's records always carry real data, and `View_AddCels`
        // (0x1e6ba0) skips chain nodes whose glist pointer is null; skip them here too.
        if node.min == [0.0; 3] && node.max == [0.0; 3] && node.sphere.1 == 0.0 { continue; }
        let owner = match world {
            Some(w) => link_owner_room(graph, i, w),
            None => find_cell_raycast(graph, node.sphere.0, None),
        };
        if let Some(o) = owner {
            if o < chains.len() && o != i { chains[o].insert(0, i as u32); }
        }
    }
    graph.chains = chains;
}

/// `build_link_objects_to_rooms` (0x1c9150)'s owner pick for the generic cel `generic`: the
/// room-class candidate (short class 0xC047/0xC023, i.e. the port's flags word & 0xffff) with
/// `generic.radius <= room.radius` and the generic's sphere centre inside the room's world bbox,
/// ranked by the room's closest collision point to the centre (`Collide_SphereIntersect` 0x1ec940's
/// scoped closest hit with a containing sphere — `world.closest_point_scoped`); strict `<` keeps
/// the first candidate in parse order [M 0x1c9150 walk]. `None` = no candidate hits (the game
/// leaves the generic unchained).
fn link_owner_room(graph: &CelGraph, generic: usize, world: &nf_game::world::CollisionWorld) -> Option<usize> {
    let g = &graph.cells[generic];
    let mut best: Option<(usize, f32)> = None;
    for (ri, r) in graph.cells.iter().enumerate() {
        if !matches!(r.flags & 0xffff, 0xc023 | 0xc047) { continue; }
        if g.sphere.1 > r.sphere.1 { continue; }
        if !(0..3).all(|k| r.min[k] <= g.sphere.0[k] && g.sphere.0[k] <= r.max[k]) { continue; }
        let Some(d) = world.closest_point_scoped(ri, g.sphere.0) else { continue };
        if best.map_or(true, |(_, bd)| d < bd) { best = Some((ri, d)); }
    }
    best.map(|(i, _)| i)
}

/// `Vision_InView` (0x1e89e0) [M]: visible iff the node's (centre, radius) sphere meets each of the
/// four viewer+0x70 side planes. The port reads the game's test as the signed-distance form
/// `n·c + d + r >= 0`. The RAW decompile is `dot((cx,cy,cz,r),(n,d)) < r -> reject` (c.lt.s f0, f12
/// + bc1t -> return 0; the decompile's `< -param_1` is a Ghidra polarity artifact), i.e. the literal
/// `n·c + d·r >= r` — but that form is untestable-absurd with the game's OWN stored planes [L
/// ee_start_05.bin: |n| = 1, n·eye + d = 0, d = 51..117 — the fustrum builder 0x1e7df8 anchors every
/// side plane at the eye through the far-quad corners, so d = -n·eye scales with the eye's distance
/// from the world origin]: at the 05 start (|eye| = 153) the d·r term adds 130..300 of slack per
/// plane (the cull degenerates to draw-everything), and at the 07000007 clock-hall spot (the eye
/// looking away from the origin, d = -69.6 on the left plane) it subtracts 184 and rejects the
/// grandfather clock at r = 2.65 that the real game DRAWS [L ground truth]. The two readings agree
/// exactly when |d| << 1, and the signed-distance form is that common limit — the port ships it and
/// lists the open question (what the game's +0x70 quads hold mid-gameplay) under the report's live
/// checks. fix-portal6 (the fix-portal5 regression: every chain node of a visited room failed this
/// gate at the clock hall, dropping the clock + the wall shields the game shows).
/// The port derives (centre, radius) from the cel's glist bbox (the game's sphere comes from the
/// pCelList record's +0x10..0x1c) — documented as the approximation. fix-portal2.
pub fn vision_in_view(centre: [f32; 3], radius: f32, sides: &[[f32; 4]; 4]) -> bool {
    sides.iter().all(|p| p[0] * centre[0] + p[1] * centre[1] + p[2] * centre[2] + p[3] + radius >= 0.0)
}

/// A cel's (centre, radius) sphere from its bbox — the `Vision_InView` input.
fn cel_sphere(c: &CelNode) -> ([f32; 3], f32) { c.sphere }

/// The game's World-glist visibility: the VisCelList walk PLUS `View_AddCels` (0x1e6ba0)'s `+0x18`
/// chain pass — for every visited cel, each chain node draws when `Vision_InView` passes, skipping
/// 0x8018-class nodes while the viewer's cel is 0xc023-class [M 0x1e6ba0:47-56]. `sides` = the
/// frustum's four side planes (the viewer+0x70 set). Returns the per-`cells` mask; the strict
/// VisCelList mask (what `View_DrawObjList`'s object cull keys on — chain nodes never enter the
/// VisCelList, `Vision_AddCelToDraw` 0x1e8908 is the only inserter) stays
/// `portal_visible_cels_from_cell`. fix-portal2.
pub fn portal_visible_cels_game_from_cell(graph: &CelGraph, start: usize, eye: [f32; 3], depth0: &[[f32; 4]; 5], sides: &[[f32; 4]; 4]) -> Option<Vec<bool>> {
    let chain = portal_cel_chain_from_cell(graph, start, eye, depth0)?;
    let mut vis = vec![false; graph.cells.len()];
    for &c in &chain { vis[c] = true; }
    let start_class = graph.cells[start].flags & 0xffff;
    for &c in &chain {
        for &n in &graph.chains[c] {
            let n = n as usize;
            if vis[n] { continue; }
            let node = &graph.cells[n];
            if node.flags & 0xffff == 0x8018 && start_class == 0xc023 { continue; }
            let (c3, r) = cel_sphere(node);
            if vision_in_view(c3, r, sides) { vis[n] = true; }
        }
    }
    Some(vis)
}

/// The sweep's LOS ray (fix-portal-sweep): the segment from `eye` to `target` (the closest point
/// of a cel's bbox) against the STATIC collision world, stopped 0.5 units short of the target —
/// the closest bbox point sits on the cel's own collision surface and the collision meshes run up
/// to ~0.3 units in front of the render glist bboxes [D the 07000005 trench probe: node 40's face
/// at 0.27 units], so a full-length ray always "hits" the very cel it is trying to clear. An
/// occluder still blocks whenever its surface is farther than the margin from the target.
/// `d` = the eye→target distance (0 = the eye inside the bbox).
pub fn los_ray_clear(world: &nf_game::world::CollisionWorld, eye: [f32; 3], target: [f32; 3], d: f32) -> bool {
    const LOS_SHORT: f32 = 0.5;
    let dir = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
    let shrink = if d > LOS_SHORT { (d - LOS_SHORT) / d } else { 1.0 };
    let to = [eye[0] + dir[0] * shrink, eye[1] + dir[1] * shrink, eye[2] + dir[2] * shrink];
    world.segment_hit_static(eye, to).is_none()
}

/// The near-LOS rescue (fix-portal-sweep): the playtest found 07000005
/// `--bond 16.67,30.04,58.60,1.755,-0.001` (the snow trench between stone walls) culling the near
/// trench walls with the walk ON. The chain there is sound per the game's own rules — the trench
/// generics sit (per `build_link_objects_to_rooms` 0x1c9150's closest-hit owner pick, the metric
/// being `Intersect_PointGeom`'s point-to-geometry distance [M 0x1ec940→0x1ebaa8 type 0x101]) on
/// the chains of the overlapping rooms 15/24, and `View_AddCels` (0x1e6ba0) only walks the chains
/// of VisCelList rooms, so the game itself draws them only because its eye cel resolves to 15
/// there — `build_FindCel` (0x1c8cb0)'s per-candidate up/down ray tie-break, whose outcome depends
/// on the per-cel object lists the port only approximates (`world.closest_point_scoped`
/// attribution). Where that approximation disagrees, the walk drops geometry the player is
/// touching. This pass restores the game's own floor: a cel the port's walk drops still draws when
/// (1) `Vision_InView` (0x1e89e0, the signed-distance form fix-portal6 ships) keeps its sphere,
/// (2) the closest point of its bbox is within `max_dist` of the eye, and (3) the segment from the
/// eye to that point crosses no collision (the sweep's ray). Only the generic classes
/// `build_link_objects_to_rooms` links ride it (0x8017..0x8019 — the chain nodes; room cels and
/// their objects stay on the strict `VisCelList` mask, so intentionally hidden rooms — the
/// 07000007 ballroom-window building the game hides [L ground-truth doc] — stay hidden).
/// Returns the number of rescued cels.
pub fn near_los_rescue(
    graph: &CelGraph, mask: &mut [bool], eye: [f32; 3], sides: &[[f32; 4]; 4],
    world: &nf_game::world::CollisionWorld, max_dist: f32,
) -> usize {
    let mut rescued = 0usize;
    for (i, c) in graph.cells.iter().enumerate() {
        if mask[i] { continue; }
        if !matches!(c.flags & 0xffff, 0x8017 | 0x8018 | 0x8019) { continue; }
        // The port's unresolvable-object cells stand for placements the parse could not resolve
        // (build_chains skips them the same way); a non-finite or reversed bbox would panic the
        // clamp below and can never pass the game's own tests — skip both.
        if c.min == [0.0; 3] && c.max == [0.0; 3] && c.sphere.1 == 0.0 { continue; }
        if (0..3).any(|k| !c.min[k].is_finite() || !c.max[k].is_finite() || c.min[k] > c.max[k]) { continue; }
        if !vision_in_view(c.sphere.0, c.sphere.1, sides) { continue; }
        let mut tp = [0f32; 3];
        for k in 0..3 { tp[k] = eye[k].clamp(c.min[k], c.max[k]); }
        let d = ((tp[0] - eye[0]).powi(2) + (tp[1] - eye[1]).powi(2) + (tp[2] - eye[2]).powi(2)).sqrt();
        if d > max_dist { continue; }
        if !los_ray_clear(world, eye, tp, d) { continue; }
        mask[i] = true;
        rescued += 1;
    }
    rescued
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

/// The viewer's persistent cel state — `viewer+4` [M `Camera_CheckLocation` 0x1bae90]. The game
/// carries the eye's cel across frames; the port used to re-pick from scratch every frame, which is
/// the 07000006 collapse's enabling bug (the smallest-volume tie-break can flip between overlapping
/// room boxes mid-walk where the game keeps the cel it walked through).
#[derive(Clone, Copy, Default)]
pub struct ViewerCel {
    /// The current cell (`cells` index); `None` = the game's 0 (no cel yet).
    pub cell: Option<usize>,
    /// The position the cel was resolved at (`viewer+0x34`; `Cel_ObjectLeftCel`'s ray origin).
    pub pos: [f32; 3],
}

/// `Cel_ObjectLeftCel` (0x1e0d58) [M]: raycast the move `from -> to` against `cell`'s portal list —
/// the per-portal bbox pre-cull (+0x20/+0x24, the ±0.1 inset box `build_alloc_portal` 0x1c8928
/// fills) then two ray-triangle tests on the quad corners (+0xc/+0x10/+0x14 and +0x14/+0x18/+0xc —
/// corners (0,1,2) and (2,3,0)); a hit returns `portal+0x8` = the destination cel of the node the
/// walker's cel owns. The node attached to celB stores the forward corners (dest celA), the celA
/// node the reversed ones (dest celB) [M 0x1d15f8 / 0x1c8928] — either way the destination is the
/// OTHER endpoint, so the quad to test is the same record's.
fn portal_move_dest(graph: &CelGraph, cell: usize, from: [f32; 3], to: [f32; 3]) -> Option<usize> {
    let dir = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let hit = |tri: [[f32; 3]; 3]| -> bool {
        // Möller–Trumbore over the move segment [0, 1].
        let e1 = [tri[1][0] - tri[0][0], tri[1][1] - tri[0][1], tri[1][2] - tri[0][2]];
        let e2 = [tri[2][0] - tri[0][0], tri[2][1] - tri[0][1], tri[2][2] - tri[0][2]];
        let pv = [dir[1] * e2[2] - dir[2] * e2[1], dir[2] * e2[0] - dir[0] * e2[2], dir[0] * e2[1] - dir[1] * e2[0]];
        let det = e1[0] * pv[0] + e1[1] * pv[1] + e1[2] * pv[2];
        if det.abs() < 1e-12 { return false; }
        let inv = 1.0 / det;
        let tv = [from[0] - tri[0][0], from[1] - tri[0][1], from[2] - tri[0][2]];
        let u = (tv[0] * pv[0] + tv[1] * pv[1] + tv[2] * pv[2]) * inv;
        if !(-1e-9..=1.0 + 1e-9).contains(&u) { return false; }
        let qv = [tv[1] * e1[2] - tv[2] * e1[1], tv[2] * e1[0] - tv[0] * e1[2], tv[0] * e1[1] - tv[1] * e1[0]];
        let v = (dir[0] * qv[0] + dir[1] * qv[1] + dir[2] * qv[2]) * inv;
        if !(-1e-9..=1.0 + 1e-9).contains(&v) || u + v > 1.0 + 2e-9 { return false; }
        let t = (e2[0] * qv[0] + e2[1] * qv[1] + e2[2] * qv[2]) * inv;
        (0.0..=1.0).contains(&t)
    };
    for p in &graph.portals {
        let dest: usize = if p.cel_a == cell { p.cel_b } else if p.cel_b == cell { p.cel_a } else { continue };
        let c = &p.corners;
        if hit([c[0], c[1], c[2]]) || hit([c[2], c[3], c[0]]) { return Some(dest); }
    }
    None
}

/// `build_point_in_bounding_box` (0x1c9500) [M]: the inclusive bbox test over one cel.
fn point_in_cell(graph: &CelGraph, cell: usize, p: [f32; 3]) -> bool {
    graph.cells.get(cell).is_some_and(|c| (0..3).all(|k| c.min[k] <= p[k] && p[k] <= c.max[k]))
}

/// `build_FindCel` (0x1c8cb0) over the graph: collect the containing room cels (the 0x40000 flag
/// [M `*puVar1 & 0x40000`], cap 0x40). One candidate returns directly [M 0x1c8cb0: `if
/// CelListCount_154 != 1` guards the tie-break]. With several, the game's tie-break is a
/// COLLISION raycast per candidate [M 0x1c8cb0: `Collide_RayIntersect(param_1, up_end, cand, …)`
/// then the mirrored down ray — the down ray's length is the up ray's min(hit, len), len starts
/// 256 and shrinks to the best-so-far distance], the candidate with the nearest hit distance
/// wins, and with no hit at all the FIRST candidate stands (0x1c8cb0's tail:
/// `iVar6 = CelList_153; ... return iVar6`). `world` (the sim's scoped `CollisionWorld`) supplies
/// the per-candidate rays through `segment_hit_scoped(Some(cell), …)` — the port's stand-in for
/// `Collide_RayIntersect`'s per-cel object list; without a world the port's bbox-face
/// approximation stands (documented).
pub fn find_cell_raycast(graph: &CelGraph, eye: [f32; 3], world: Option<&nf_game::world::CollisionWorld>) -> Option<usize> {
    let mut cands: Vec<usize> = Vec::new();
    for (i, c) in graph.cells.iter().enumerate() {
        if c.flags & 0x40000 == 0 { continue; }
        if (0..3).all(|k| c.min[k] <= eye[k] && eye[k] <= c.max[k]) {
            cands.push(i);
            if cands.len() >= 0x40 { break; }
        }
    }
    if cands.len() <= 1 { return cands.first().copied(); }
    if let Some(world) = world {
        // [M 0x1c8cb0]: len = the best-so-far distance (256 first); up ray, then the down ray
        // capped at min(up hit, len); the nearest hit distance wins; no hits -> the first.
        let mut best: Option<(usize, f32)> = None;
        for &i in &cands {
            let len = best.map_or(256.0, |(_, d)| d);
            let up = world.segment_hit_scoped(Some(i), eye, [eye[0], eye[1] + len, eye[2]]).map(|(t, _)| t * len);
            let dlen = up.map_or(len, |ud| ud.min(len));
            let down = world.segment_hit_scoped(Some(i), eye, [eye[0], eye[1] - dlen, eye[2]]).map(|(t, _)| t * dlen);
            match (up, down) {
                (None, None) => continue,
                (a, b) => {
                    let d = a.iter().chain(b.iter()).copied().fold(f32::MAX, f32::min);
                    if best.map_or(true, |(_, bd)| d < bd) { best = Some((i, d)); }
                }
            }
        }
        return Some(best.map_or(cands[0], |(i, _)| i));
    }
    // The no-world approximation: the nearest bbox face (the pre-fix-portal3 behaviour).
    {
        let mut best: Option<(usize, f32)> = None;
        for &i in &cands {
            let c = &graph.cells[i];
            let up = c.max[1] - eye[1];
            let down = eye[1] - c.min[1];
            let d = up.min(down);
            if d < 256.0 && best.map_or(true, |(_, bd)| d < bd) { best = Some((i, d)); }
        }
        Some(best.map_or(cands[0], |(i, _)| i))
    }
}

/// The eye's cel for this frame — `Camera_CheckLocation` (0x1bae90) [M], over the persistent state:
/// 1. `Cel_ObjectLeftCel` (0x1e0d58): the move raycast against the current cel's portals relinks
///    directly to the crossed portal's destination;
/// 2. `build_point_in_bounding_box` (0x1c9500): still inside the current cel's box keeps it;
/// 3. `build_FindCel` (0x1c8cb0): a fresh search;
/// 4. a FindCel miss KEEPS the previous cel, and a first-ever miss takes the world's first cel
///    (`puVar1[1] = *glb_world`) [M 0x1bae90:34-38].
///
/// `world` (the sim's scoped `CollisionWorld`) feeds `build_FindCel`'s collision tie-break
/// [M 0x1c8cb0]; `None` keeps the bbox-face approximation.
pub fn eye_cell(graph: &CelGraph, state: &mut ViewerCel, eye: [f32; 3], world: Option<&nf_game::world::CollisionWorld>) -> Option<usize> {
    if let Some(c) = state.cell {
        if let Some(dest) = portal_move_dest(graph, c, state.pos, eye) {
            state.pos = eye;
            state.cell = Some(dest);
            return Some(dest);
        }
        if point_in_cell(graph, c, eye) {
            state.pos = eye;
            return Some(c);
        }
    }
    let found = find_cell_raycast(graph, eye, world);
    state.pos = eye;
    match found {
        Some(c) => { state.cell = Some(c); Some(c) }
        None => {
            let keep = state.cell.or(Some(0)).filter(|&c| c < graph.cells.len());
            state.cell = keep;
            keep
        }
    }
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
    portal_visible_cels_from_cell(graph, start, eye, depth0)
}

/// The walk from an already-resolved runtime cel (the `viewer+4` the game hands
/// `Vision_Portal_Recurse` 0x1e86e8 — `Camera_CheckLocation` 0x1bae90's state, not a per-frame
/// re-pick). fix-portal.
pub fn portal_visible_cels_from_cell(graph: &CelGraph, start_cell: usize, eye: [f32; 3], depth0: &[[f32; 4]; 5]) -> Option<Vec<bool>> {
    let chain = portal_cel_chain_from_cell(graph, start_cell, eye, depth0)?;
    let mut visible = vec![false; graph.cells.len()];
    for c in chain { visible[c] = true; }
    Some(visible)
}

/// The game's `VisCelList` (0x34b3b0) chain order — what `View_AddCels` (0x1e6ba0) walks when it
/// hands out the −90000 + 10·k list-0 keys. `Vision_Portal_Recurse` (0x1e86e8) inserts the viewer's
/// cel at depth 0, `Vision_Recurse` (0x1e82e0) inserts each portal-reached cel at **first reach**
/// with depth = parent+1 (`*(short *)(&DAT_0034b3b4 + uVar9 * 8) = (short)iVar11`), then
/// `QuickSort` (0x1d30a8, comparator id 3) orders the array and it is threaded through `cel+0x20`.
/// Returns the sorted `cells` indices in that order (the viewer's cel first), or None when the walk
/// cannot run (same guards as `portal_visible_cels`, whose visibility mask this also is).
pub fn portal_cel_chain(graph: &CelGraph, eye_room: usize, eye: [f32; 3], depth0: &[[f32; 4]; 5]) -> Option<Vec<usize>> {
    let start = graph.room_to_cell.get(eye_room).copied().flatten()?;
    portal_cel_chain_from_cell(graph, start, eye, depth0)
}

/// The chain from a resolved cell — see `portal_cel_chain`.
pub fn portal_cel_chain_from_cell(graph: &CelGraph, start: usize, eye: [f32; 3], depth0: &[[f32; 4]; 5]) -> Option<Vec<usize>> {
    let mut visited = vec![false; graph.cells.len()];
    let mut planes: Vec<Vec<[f32; 4]>> = vec![Vec::new(); 48];
    planes[0] = depth0.to_vec();
    let mut budget = 0usize;
    // The VisCelList insertion order: the viewer's cel first (depth 0 — `Vision_Portal_Recurse`
    // inserts it before `Vision_Recurse` runs), then each first-reach cel at depth = parent+1.
    // `Vision_Recurse` [M 0x1e82e0]: the recursion budget (VisionRecurseCount > 100) and the depth
    // cap (45) are the game's own guards; VisCelCount caps at 0x40 cels.
    let mut order: Vec<(usize, u16)> = vec![(start, 0)];
    fn recurse(
        graph: &CelGraph, eye: [f32; 3], cell: usize, depth: usize,
        visited: &mut Vec<bool>, planes: &mut Vec<Vec<[f32; 4]>>, budget: &mut usize, order: &mut Vec<(usize, u16)>,
    ) {
        *budget += 1;
        if *budget > 100 || depth > 45 { return; }
        let n0 = planes[depth].len();
        let dbg = std::env::var_os("NF_PORTAL_DEBUG").is_some();
        for p in &graph.portals {
            let (side_open, dest, fwd) = if cell == p.cel_a {
                ((p.flags & 1) == 0, p.cel_b, false)
            } else if cell == p.cel_b {
                ((p.flags & 2) == 0, p.cel_a, true)
            } else { continue };
            if !side_open {
                if std::env::var_os("NF_PORTAL_DEBUG").is_some() { eprintln!("[pwalk] d{depth} cell {cell} -> {dest}: side closed (flags {:x})", p.flags); }
                continue;
            }
            // The side's plane: the node attached to celB carries the forward plane, the celA node
            // its reverse [M 0x1c8928 / 0x1d15f8]. `Vision_Recurse` processes a portal only when the
            // eye is on its negative side [M 0x1e82e0:50].
            let (n, d) = if fwd { (p.n_fwd, p.d_fwd) } else { ([-p.n_fwd[0], -p.n_fwd[1], -p.n_fwd[2]], -p.d_fwd) };
            if n[0] * eye[0] + n[1] * eye[1] + n[2] * eye[2] + d >= 0.0 {
                if dbg { eprintln!("[pwalk] d{depth} cell {cell} -> {dest}: eye on the wrong side of the portal plane"); }
                continue;
            }
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
            if poly.is_empty() {
                if dbg { eprintln!("[pwalk] d{depth} cell {cell} -> {dest}: poly empty after clip (n0 {n0})"); }
                continue;
            }
            // The VisCelList insert cap: `if (VisCelCount < 0x40)` guards the dedup + insert
            // [M 0x1e82e0]; the recursion itself continues past the cap.
            if !visited[dest] && order.len() < 0x40 { visited[dest] = true; order.push((dest, (depth + 1) as u16)); }
            if dbg { eprintln!("[pwalk] d{depth} cell {cell} -> {dest}: poly {} (n0 {n0}, budget {budget}, {} new)", poly.len(), !visited[dest] as u8); }
            if poly.len() >= 3 {
                // The next depth's planes: one per clipped-polygon edge, ref = the eye (ViewPoints[0]
                // [M 0x1e82e0:146]), plus the portal's own plane last [M 0x1e82e0:151-155].
                let m = poly.len();
                let mut next: Vec<[f32; 4]> = (0..m).map(|i| plane_eq(eye, poly[i], poly[(i + 1) % m])).collect();
                next.push([n[0], n[1], n[2], d]);
                planes[depth + 1] = next;
                recurse(graph, eye, dest, depth + 1, visited, planes, budget, order);
                if *budget > 100 { return; }
            }
        }
    }
    visited[start] = true;
    recurse(graph, eye, start, 0, &mut visited, &mut planes, &mut budget, &mut order);
    quicksort_viscel(&mut order);
    Some(order.into_iter().map(|(c, _)| c).collect())
}

/// `QuickSort` (0x1d30a8) with comparator id 3 — jumptable 0x304fc0[3] → 0x1d3430, disassembled:
/// `lhu $a0, 4($a0); lhu $v1, 4($a1)` — the ascending **u16 portal depth** at element+4. Unstable:
/// Hoare partition, pivot = element `count>>1` swapped to the front, the smaller side pushed to the
/// stack (the decomp's byte-size test `(j − (lo+1)) < (hi − i)` at stride 8 is index-space
/// `(j − lo) <= (hi − i)`). The tie order feeds `View_AddCels`' −90000 + 10·k ordinals, so the port
/// sorts exactly like the game over the same insertion order.
pub fn quicksort_viscel(items: &mut [(usize, u16)]) {
    let n = items.len();
    if n < 2 { return; }
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let (mut lo, mut hi) = (0usize, n - 1);
    loop {
        // Pivot: element count>>1 swapped to the front (0x1d31a8's `Swap(((hi−lo)/stride+1)>>1·stride+base, base)`).
        let count = hi - lo + 1;
        items.swap(lo, lo + count / 2);
        let mut i = lo + 1;
        let mut j = hi + 1;
        loop {
            // `while (uVar3 <= uVar5 && Comp(uVar3, param_1) < 1)` — Comp <= 0 stays left.
            while i <= hi && items[i].1 <= items[lo].1 { i += 1; }
            j -= 1;
            // `while (param_1 < uVar2 && Comp(uVar2, param_1) > -1)` — Comp >= 0 stays right.
            while lo < j && items[j].1 >= items[lo].1 { j -= 1; }
            if j < i { break; }
            items.swap(i, j);
            i += 1;
        }
        items.swap(lo, j);
        if j - lo <= hi.saturating_sub(i) {
            // Left side smaller (or equal): push the right, loop on the left [LAB_001d32a8].
            if i < hi { stack.push((i, hi)); }
            if j > lo + 1 {
                hi = j - 1;
            } else {
                // Left is a single element: pop (LAB_001d32d0).
                match stack.pop() { Some((l, h)) => { lo = l; hi = h; } None => return }
            }
        } else {
            // Right side smaller: push the left, loop on the right.
            if lo + 1 < j { stack.push((lo, j - 1)); }
            lo = i;
            if lo >= hi {
                // The do-while's `i < hi` failed: pop.
                match stack.pop() { Some((l, h)) => { lo = l; hi = h; } None => return }
            }
        }
    }
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
        // `Game_Run` 0x1c9790:33 runs `Light_Update` before the object movement each tick (fix-lighting):
        // lifetimes (the muzzle light of the previous tick dies here), the script-channel gate and the
        // flicker state machine. The flicker's RNG re-seeds ride the shared generator before this tick's draws.
        {
            let frame = self.drones.frame + 1; // Light_Update sees the new tick's counter
            let channels = &self.drones.channels;
            let rng = &mut self.drones.rng;
            self.drones.lights.update(&|c| channels.on(c as u32), frame, rng);
        }
        self.shot_tick = false;
        // The smoke puffs (`gases_smoke` 0x1ab2d0 [S] simplified) tick with the sim.
        for s in self.smoke.iter_mut() { s.update(); }
        // The laser/stunner fire model [M Player_WeaponFiring 0x1a5460's held-fire branch →
        // Player_WeaponInitBullet 0x1a4bf0 → BLData+0x95b → Player_MuzzleFlash 0x1a4298 →
        // Draw_LaserBeam/Draw_TaserBeam]: every refire tick (interval +0x40 = 1 for both [L
        // weapon_data]) is a shot; the beam draws from the muzzle to the CROSSHAIR ray's re-cast hit
        // [M 0x1a4bf0:152-159 Player_GetAimingPoint, 0x1a4298:107-132]. The port runs the burn ray
        // and parks the beam endpoints here on the crosshair ray (head → aim), computed below with
        // the shot direction (the old bond.look() ray was the user's "the laser isn't accurate").
        // The STUNNER's beam shows only when the ray's target is a guard in range [M 0x1a4298:134
        // reads BLData+0x95d, set at 0x1a4bf0:169-186 — a class-2/3 hit within weapon_data+0x1c].
        self.laser_beam = None;
        self.stunner_target = None;
        let laser_w = self.gun.weapon;
        if !self.disabled && trigger && nf_game::gadget::runs_beam(laser_w) {
            let head = self.bond.head();
            // The crosshair ray (the same construction run() fires bullets down: look + the crosshair
            // offsets [M 0x1a4bf0:152-159's aim-point ray]; the port's cross/scoped offsets live in run).
            let look = self.bond.look();
            let (yaw, p) = (self.bond.yaw, self.bond.pitch * std::f32::consts::FRAC_PI_2);
            let right = [-yaw.cos(), 0.0, yaw.sin()];
            let up = [-p.sin() * yaw.sin(), p.cos(), -p.sin() * yaw.cos()];
            let (tx, ty) = (0.76966 * self.cross[0], 0.57724 * self.cross[1]);
            let d0 = [0, 1, 2].map(|k| look[k] + tx * right[k] + ty * up[k]);
            let l = (d0[0] * d0[0] + d0[1] * d0[1] + d0[2] * d0[2]).sqrt().max(1e-6);
            let dir = d0.map(|v| v / l);
            let frame = self.drones.frame + 1;
            // The stunner's target test [M 0x1a4bf0:169-200]: the ray must meet a live guard (class 2 —
            // Bullet_heatseekable 0x12f248's class table) within weapon_data+0x1c; no target → no shot
            // bullet (and Draw_TaserBeam's +0x95d gate keeps the beam off too).
            let mut target_hit = true;
            if matches!(laser_w, 0x4a | 0x4c) {
                target_hit = false;
                let range = self.weapons.records.get(laser_w as usize).map_or(5.0, |r| f32::from_le_bytes(r[0x1c..0x20].try_into().unwrap()));
                let end = [head[0] + dir[0] * range, head[1] + dir[1] * range, head[2] + dir[2] * range];
                for dr in self.drones.list.iter() {
                    if !dr.present || !dr.enabled || dr.corpse || dr.health <= 0.0 { continue; }
                    let c = [dr.pos[0], dr.pos[1], dr.pos[2]];
                    // The ray-vs-sphere contact (the port's `Collide_PickObj` 1.05 sphere stand-in).
                    let ab = [end[0] - head[0], end[1] - head[1], end[2] - head[2]];
                    let ab2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
                    let ap = [c[0] - head[0], c[1] - head[1], c[2] - head[2]];
                    let t = if ab2 > 1e-9 { (ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / ab2 } else { 0.0 };
                    let cl = [head[0] + ab[0] * t.clamp(0.0, 1.0), head[1] + ab[1] * t.clamp(0.0, 1.0), head[2] + ab[2] * t.clamp(0.0, 1.0)];
                    let d2 = (cl[0] - c[0]).powi(2) + (cl[1] - c[1]).powi(2) + (cl[2] - c[2]).powi(2);
                    if d2 <= 1.05 * 1.05 { target_hit = true; break; }
                }
            }
            self.stunner_target = Some(target_hit);
            if target_hit {
                let (a, b) = nf_game::gadget::laser_tick(head, dir, &mut self.drones.locks, &self.world, &mut self.drones.channels, frame);
                self.laser_beam = Some((a, b));
            }
        }
        self.smoke.retain(|s| !s.dead());
        let (trigger, reload) = if self.disabled { input = Default::default(); (false, false) } else { (trigger, reload) };
        // fix-death-restart: a dead Bond gets no input — `Player_Update` 0x193600 runs only the anim
        // and `Player_HandleDeath` for state 2/3 [M 0x193600:317-319] — and the 0x40000042 menu
        // pauses the player (`GS_PausePlayer(1,0)` [M 0x182c60:124]). The fire bit still reaches
        // the death's skip gate while dying (action 25 [M 0x190bb0:30]); the menu eats it.
        let menu_pause = self.drones.fail_menu_open();
        self.drones.bond_fire = trigger && !menu_pause;
        let (mut input, trigger, reload, use_key) = if self.drones.death.active || menu_pause {
            (Default::default(), false, false, false)
        } else { (input, trigger, reload, use_key) };
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
        // The fire-mode button (pad ○ = action 12; keyboard B) [M `Player_Weapon` 0x1a24c8:204-260]:
        // only with the view anim idle (the game's `anim+0xF4 == 0`: no switch, no reload, no fire
        // script) and not while aiming a scoped weapon (+0x68 & 0x40). Twin-linked records (weapon_data
        // +0x05) switch to their twin through the normal switch request; the rest cycle the mode byte
        // (the Inventory's `fire_mode`, `BL + w*0xC + 0x1BB`) up to the record's mode count (+0x2E).
        // For the watch pair 0x54/0x55 the fire press ORs in (the game reads action 9 mask 4 there) —
        // the port tracks the trigger's press edge in `prev_trigger`.
        if input.mode_switch {
            let watch = matches!(self.gun.weapon, 0x54 | 0x55);
            let fire_new = input.fire && !self.prev_trigger;
            let toggled = if watch { fire_new } else { false };
            if (input.mode_switch || toggled)
                && self.gun.switch_state == 0 && self.gun.reload_state == 0 && !self.gun.fire_state && !fire_busy
                && !(aiming && self.weapons.flags(self.gun.weapon as usize) & 0x40 != 0)
            {
                if let Some(w) = self.weapons.records.get(self.gun.weapon as usize) {
                    let twin = w[5] as i8;
                    let mode_count = w[0x2e];
                    let mode = self.drones.inventory.fire_mode.get(self.gun.weapon as usize).copied().unwrap_or(0);
                    let carried = self.drones.inventory.carried.clone();
                    let clip = self.drones.inventory.clip.clone();
                    let weapons = &self.weapons;
                    let has_ammo = |id: u8| nf_game::gun::Gun::has_ammo(id, &carried, &clip, |x| weapons.ammo_type(x as usize), |x| weapons.flags(x as usize));
                    let sw = nf_game::gun::fire_mode_switch(self.gun.weapon, twin, mode_count, mode, has_ammo);
                    if let Some(new) = sw.new_weapon {
                        // The twin switch writes `obj+0x63` (the switch request) [M 0x1a24c8:246-260].
                        let cur = self.gun.weapon;
                        if cur >= 1 && cur <= 0x47 { self.gun.prev_weapon = cur; }
                        if cur >= 0x4a && cur <= 0x5e { self.gun.prev_gadget = if cur == 0x55 { 0x54 } else { cur }; }
                        self.gun.switch_request = new;
                    }
                    if let Some(m) = sw.new_mode {
                        if let Some(fm) = self.drones.inventory.fire_mode.get_mut(self.gun.weapon as usize) { *fm = m; }
                    }
                }
            }
        }
        self.prev_trigger = input.fire;
        // The pickup auto-switch, LIVE (fix-auto-switch re-opens the fix-pickups-remainder gate)
        // [M Player_EquipWeapon 0x1a8a58's tail]: the take path writes ONLY the request byte
        // `anim+0x63 = id` (per-player flag DAT_002a38d2 ∧ Player_IsBetterWeapon 0x1a6eb0) — no
        // Player_WeaponChange gate, no prev-weapon memory (+0x930/+0x932 are the cycle path's
        // bookkeeping, written by Player_WeaponChange 0x1a6f80 only), no anim, no view state. The
        // draw machine picks it up on the next idle tick (Player_SetWeaponAnimObj 0x1a2e38 default
        // tail: `if (anim+0x63 != anim+0x62) { anim+0x64 = anim+0x62; Player_WeaponSelect
        // 0x1a2cc0 }`); a later weapon-cycle request overwrites it, as in the game.
        // The 07000001 escort-walk wedge that gated this off does not reproduce on master with the
        // drain live (walkreal_07000001 twice-green this session; the decompile shows why: the full
        // view-state census — Player_WeaponFiring 0x1a5460, Player_Weapon 0x1a24c8, Player_Move
        // 0x191dd0, Player_Update 0x193600 — gates only the fire pipeline on the switch states;
        // walking, walk speed, the capsule and the walk anim are NEVER gated on them, and the
        // holster/draw scripts touch only the first-person view model at BLData+0x7e8, never
        // Bond's body/position). The draw machine below is that default-tail behaviour.
        if let Some(id) = self.drones.weapon_switch_request.take() {
            if id != self.gun.weapon as u16 {
                self.gun.switch_request = id as u8;
            }
        }
        self.drones.weapon_switch_request = None;
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
            // fix-silencer-modes: the toggle-back gates on the remembered id's BASE (0x1a6f80:97-111)
            // and restores the remembered TWIN verbatim (line 113); the cycle walk carries the twin
            // step (0x1a6f80:179-198) — `cycle` takes &mut self for the marker clear.
            let base_of = |w: u8| weapons.base(w as usize) as u8;
            let target = if let Some(id) = input.switch_to {
                if id != self.gun.weapon && owned.get(id as usize).copied().unwrap_or(false)
                    && weapons.first_person(id as usize) != 0 { Some(id) } else { None }
            } else if input.weapon_cycle != 0 {
                let t = self.gun.toggle_back(nf_game::gun::CycleMode::Weapons, &owned, has_ammo, selectable, base_of);
                t.or_else(|| self.gun.cycle(input.weapon_cycle, nf_game::gun::CycleMode::Weapons, &owned, has_ammo, selectable))
            } else if input.gadget_cycle != 0 {
                let t = self.gun.toggle_back(nf_game::gun::CycleMode::Gadgets, &owned, has_ammo, selectable, base_of);
                t.or_else(|| self.gun.cycle(input.gadget_cycle, nf_game::gun::CycleMode::Gadgets, &owned, has_ammo, selectable))
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
            // The Q-Claw grapple's fire [M Player_SetGrapplePoint 0x195da0]: with the claw (0x50/0x51 —
            // the bank's "Grapple" records, `weapon_data+0x70 & 0x40` [L ee_start_05.bin]; the OLD gate
            // 0x4A/0x4C was the Stunner's — the hardcoded table's shift) out and the fire pressed on
            // foot, the aim ray's grapple point (placement 0x3A) latches and Bond enters the substate 2
            // (the reel). The reach is the claw bolt's own `weapon_data+0x1c` death cap [M Bullet_update
            // 0x12e7b0:158; L: 50.0 for 0x50, 100.0 for 0x51]. The enter tick swallows the fire — it runs
            // BEFORE the bond tick so a release tick's fire cannot re-latch in the same tick.
            let mut trigger = trigger;
            if trigger
                && self.bond.substate == nf_game::player::SS_WALK
                && self.bond.grapple.is_none()
                && nf_game::gadget::is_grapple(self.gun.weapon)
            {
                let head = self.bond.head();
                let dir = self.bond.look();
                if let Some(point) = nf_game::gadget::pick_grapple_point(&self.drones.grapple_points, head, dir, nf_game::gadget::grapple_range(&self.weapons, self.gun.weapon)) {
                    self.bond.grapple = Some(nf_game::gadget::GrappleState { point, phase: 1, pull: 0.0 });
                    self.bond.substate = nf_game::player::SS_GRAPPLE;
                    self.bond.vel = [0.0; 3];
                    trigger = false;
                }
            }
            let water = self.cel_at(self.bond.pos).map(|i| self.cels[i].water).filter(|w| *w > -400.0);
            let sink = input.crouch;
            let ctx = nf_game::player::LadderCtx { ladders: &self.ladders, wires: &self.wires, wire_anims: Some(&self.wire_anims), water, sink, creepwalls: &self.creepwalls };
            // The third-icon context [M ThirdIcon_Update 0x196e80 + Player_CollisionHandler 0x1949a8:22]:
            // the tick starts with +0x95F = 0xFF (none), then every ENABLED icon (its param-1 channel on,
            // or param 1 == 0 = always [M ThirdIcon_Create 0x197070:27-32]) whose straddle-cel set holds
            // the player's cel writes its type. The game's set is the icon's obj+0xd0 chain: the icon's
            // world sphere straddles cels through the portals it intersects [M Controls_StraddleTest
            // 0x134778 → Collide_StraddleCels 0x1eac98 → Intersect_Portal 0x1ea7b8 v0x101], every object
            // occupying a straddled cel gets a hitdata node [M Collide_Update 0x1ea398:160-233], and the
            // icon writes when a node's +0x58 is the class-3 player [M 0x196e80:36-52]. The port resolves
            // each icon's set once at load (`icon_cels` — `icon_straddle_cels` over `cel_graph` with the
            // icon's own world-sphere radius, e.g. 07000008's "Hand_Over_Hand" r 1.164 [D probe]) and
            // tests the player's FindCel cel [M build_FindCel 0x1c8cb0: room cels + the collision
            // tie-break, `find_cell_raycast`] against it. The old flat 12.0-unit radius armed through
            // walls — the 07000008 playtest: walking into the guardhouse wall and jumping threw Bond onto
            // the zipline across it (the icon sits 1.9 from the wall on the far side); the r-1.164 sphere
            // crosses no portal there, so the wall keeps the contexts apart exactly as the cel graph lays
            // them. **Deviation [S]**: the port's cel buckets are coarser than the game's cels — the
            //   fix-swim-vent5 probe put 07000005's gate door and the second vent's ctx-6 icons in one
            //   port cel — so pure cel occupancy would arm the gate door against the [L PCSX2 2026-10-13
            //   pin (0xFF there; enforced by fix_swim_vent5_live)]. The arm region is therefore the
            //   INTERSECTION: the straddle set AND the live-measured 12.0 window (the same measured
            //   middle the pre-fix radius used: the vent crawl spot must read 6 at 3.9, the gate door
            //   must not at 24.1). The intersection keeps the wall gate this task fixes (the 07000008
            //   east balcony reads ctx 3 pre-fix at 11.1 through the wall — its cel is not the icon's;
            //   the wall is the difference) and every measured pin. When the port's FindCel resolves
            //   cels exactly like the game's, the distance factor can drop.
            self.bond.icon_ctx = 0xff;
            let bond_cell = find_cell_raycast(&self.cel_graph, self.bond.pos, Some(&self.world));
            for (ii, icon) in self.icons.iter().enumerate() {
                if icon.on_channel != 0 && !self.drones.channels.on(u32::from(icon.on_channel)) { continue; }
                let Some(cells) = self.icon_cels.get(ii) else { continue };
                if cells.is_empty() { continue; }
                if !bond_cell.is_some_and(|bc| cells.contains(&bc)) { continue; }
                let near = (icon.centre[0] - self.bond.pos[0]).powi(2) + (icon.centre[1] - self.bond.pos[1]).powi(2) + (icon.centre[2] - self.bond.pos[2]).powi(2) < 144.0;
                if near { self.bond.icon_ctx = icon.ctx; }
            }
            // No walk-in or use-key wire grab (main agent at the fix-wire2 × walkreal-b2 merge): the game's
            // grab that is certain is the JUMP grab [M Player_HandleJump 0x1916b0:107-147 — action 7 is the
            // jump (the same press plays the refused-jump sound 0x1db on 07000007), with the icon context
            // +0x95F == 3], ported in `Bond::physics_tick`. E is action 0xE → `Player_Activate` 0x1a5170, which
            // has no class-0x2C arm. A walking-pick grab (`Player_DealWithObjHit` 0x194458:104 → CollWire)
            // exists only if the walking query version reaches `Collide_PickObj`'s world-sphere branch
            // (versions 0x200/0x201 [M 0x1eae80]) — unproven, and the unculled sphere let 07000002's courtyard
            // wire swallow Bond in the bunker; left out until measured. `wire_cels` stays for that.

            let was_on_wire = matches!(self.bond.substate, nf_game::player::SS_WIRE | nf_game::player::SS_ZIPLINE);
            let was_climbing = self.bond.substate == nf_game::player::SS_CLIMB;
            input.fire = trigger;
            // The queries' start cel (obj+0x20 — fix-safe08b): the movement pass's relink, read fresh
            // off the position the same way `build_LinkToRoom` 0x1c9680 re-links.
            self.bond.cel = self.cel_at(self.bond.pos).and_then(|room| self.room_cell.get(room).copied().flatten());
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
            // The crawl's per-tick effects [M Player_Creep 0x1969d0:85-88]: `Music_Event(0xD, 1)` and
            // `switch_channels[wall.ch] = 1` every tick while Bond crawls (fix-swim-vent).
            let on_creep = self.bond.substate == nf_game::player::SS_CREEP;
            self.creeping = on_wire || on_creep;
            if on_wire {
                self.drones.music.event(0xd, 1);
                if let Some(ch) = self.bond.wire.and_then(|wi| self.wires.get(wi)).map(|w| w.channel).filter(|c| *c != 0) {
                    self.drones.channels.set(ch as u32, 1, self.drones.frame);
                    self.wire_channel = ch;
                }
            } else if on_creep {
                self.drones.music.event(0xd, 1);
                if let Some(ch) = self.bond.creep.and_then(|wi| self.creepwalls.get(wi)).map(|w| w.ch).filter(|c| *c != 0) {
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
                self.drones.health.pain_generic(dmg, self.drones.difficulty, self.level, 7);
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
                else if let (Some(s), Some((t, _, m))) = (self.surfaces.as_ref(), self.world.segment_surface(p, [p[0], p[1] - 1.6347, p[2]], true)) {
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
        if let Some(dmg) = self.bond.fall_damage { self.drones.health.pain_generic(dmg, self.drones.difficulty, self.level, 6); }
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
        // The game's collision capsule off `pos` (= Bond's obj+0x30): standing (case 0/6) ends
        // pos+up·0.55 / pos+up·(0.275 − cc); crouched (case 3) pos+up·(0.65 − cc) / pos+up·(0.45 − cc)
        // [M Player_Collision 0x194df0]. Kept on `Player` for the future toucher work: the trigger
        // toucher still uses the historical stand-in — the game-accurate capsule broke 07000005's
        // truck chain (the class-5 object-contact stand-in, see drone.rs's toucher comment).
        let capsule = if self.bond.crouched {
            ([pos[0], pos[1] + 0.65 - self.bond.cc, pos[2]], [pos[0], pos[1] + 0.45 - self.bond.cc, pos[2]])
        } else {
            ([pos[0], pos[1] + 0.55, pos[2]], [pos[0], pos[1] + 0.275 - self.bond.cc, pos[2]])
        };
        let player = nf_game::drone::Player { pos, target: [camera[0], camera[1] - 0.45, camera[2]], eye: camera, aim: look, sph, hanging, capsule,
            cel: self.cel_at(self.bond.pos).and_then(|room| self.room_cell.get(room).copied().flatten()) };
        let d = nf_game::drone::Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.level, sequences: &self.seqs, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: self.surfaces.as_ref(), voices: self.voices.as_deref(), difficulty: self.drones.difficulty };
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
            // fix-silencer-modes: the twin-variant cycle marker [M Player_WeaponSelect 0x1a2cc0:15-16]
            // is written for EVERY same-base request, before the FP-model gate — (requested id − base
            // id), 0 when the base itself is equipped. `Player_WeaponChange`'s walk (0x1a6f80:180-195)
            // reads it to hand the equipped twin back on the next cycle to this family.
            if cur_base == req_base {
                let slot = cur_base as usize;
                if slot < self.gun.twin_skip.len() {
                    self.gun.twin_skip[slot] = req_id as i8 - self.weapons.base(req_id as usize) as i8;
                }
            }
            if cur_base == req_base && self.weapons.first_person(cur_id as usize) == self.weapons.first_person(req_id as usize) {
                let variant = self.weapons.variant_swap_script(req_id as usize);
                if variant != 0 {
                    if log_switch { eprintln!("[weapon-switch] tick {}: state 0→8 variant-swap {:08x} (weapon {:#x}→{:#x}, same base {:#x})", self.drones.frame, variant, cur_id, req_id, cur_base); }
                    self.view_script = variant;
                    (self.view_time, self.view_speed) = (0.0, 1.3);
                    self.gun.switch_state = 8;
                    switching = true;
                    // The variant swap does NOT touch the clip/carried or the gun's live magazine:
                    // [M 0x1a2cc0:18-29] writes only the weapon byte, the mode byte, the +0xC8 script
                    // at 1.3x and view state 8 — the twin is the same physical gun, the magazine
                    // continues. (The port used to round-trip gun.clip→clip[old] and clip[new]→
                    // gun.clip here; clip[0x03] is a never-equipped slot, so the P2K's 7 live rounds
                    // zeroed and the next tick auto-reloaded — whose +0xA4 script then overwrote the
                    // +0xC8 variant script and left switch_state stuck at 8 forever.)
                    if self.gun.weapon >= 1 && self.gun.weapon <= 0x47 { self.gun.prev_weapon = self.gun.weapon; }
                    if self.gun.weapon >= 0x4a && self.gun.weapon <= 0x5e { self.gun.prev_gadget = if self.gun.weapon == 0x55 { 0x54 } else { self.gun.weapon }; }
                    self.gun.weapon = self.gun.switch_request;
                } else {
                    // Same base but no variant script: instant swap to idle. The game's variant path
                    // touches no clip arrays either [M 0x1a2cc0:18-29] — same physical gun.
                    if log_switch { eprintln!("[weapon-switch] tick {}: instant variant swap (no script) {:#x}→{:#x}", self.drones.frame, cur_id, req_id); }
                    if self.gun.weapon >= 1 && self.gun.weapon <= 0x47 { self.gun.prev_weapon = self.gun.weapon; }
                    if self.gun.weapon >= 0x4a && self.gun.weapon <= 0x5e { self.gun.prev_gadget = if self.gun.weapon == 0x55 { 0x54 } else { self.gun.weapon }; }
                    self.gun.weapon = self.gun.switch_request;
                    let nwi = self.gun.weapon as usize;
                    self.view_script = self.weapons.idle_script(nwi);
                    (self.view_time, self.view_speed) = (0.0, 1.0);
                }
            } else {
                // Different base: holster → draw. `Player_WeaponSelect` checks +0xC4 (holster) [M 0x1a2cc0 line 33].
                // fix-laser-beam: a fire-script LOOP left playing (the Laser/Stunner's op-1 loop — the
                // game's held state 0xb does not reach its op-4.3 stop before the switch) dies here:
                // the fire + release scripts' looped op-1 ids all stop [S the game's own stop point is
                // unmeasured — the loop must not outlive the weapon; live check filed].
                for id in self.gadget_loop_ids(cur_id) { self.drones.audio.stop_all_with_id(id); }
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
                    // fix-laser-beam: the fire-script loop dies with the weapon (see the holster arm).
                    for id in self.gadget_loop_ids(cur_id) { self.drones.audio.stop_all_with_id(id); }
                    if let Some(c) = self.drones.inventory.clip.get_mut(wi) { *c = self.gun.clip as i16; }
                    if let Some(n) = self.drones.inventory.carried.get_mut(t) { *n = self.gun.spare as i16; }
                    if self.gun.weapon >= 1 && self.gun.weapon <= 0x47 { self.gun.prev_weapon = self.gun.weapon; }
                    if self.gun.weapon >= 0x4a && self.gun.weapon <= 0x5e { self.gun.prev_gadget = if self.gun.weapon == 0x55 { 0x54 } else { self.gun.weapon }; }
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
                        if self.gun.weapon >= 1 && self.gun.weapon <= 0x47 { self.gun.prev_weapon = self.gun.weapon; }
                        if self.gun.weapon >= 0x4a && self.gun.weapon <= 0x5e { self.gun.prev_gadget = if self.gun.weapon == 0x55 { 0x54 } else { self.gun.weapon }; }
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
            let params = nf_game::gun::GunParams::from_record(w, self.drones.inventory.fire_mode.get(self.gun.weapon as usize).copied().unwrap_or(0) as usize, |h| scripts.get(&h).map(|s| s.length as u32));
            // Auto-aim (on by default in the game's options) only outside aim mode; in aim mode the offset eases back.
            // Auto-aim off (or aim mode): no target; the offsets still halve every tick (`Player_AutoAim` 0x1a7c98), so
            // the crosshair returns to the centre instead of staying where the last lock left it.
            let dir = if self.auto_aim && !aiming { self.drones.auto_aim(&d, camera, look, self.gun.weapon) } else {
                self.drones.aim_target = None;
                self.drones.aim_offset = self.drones.aim_offset.map(|v| v * 0.5);
                aim
            };
            let was_reloading = self.gun.reload_state;
            // The pre-shot fire-script state (`anim+0xF4 == 9`): the game SKIPS `Player_SetFiringAnim`
            // — the running fire script is NOT restarted — when the view state is already 9 and
            // `weapon_data+0x68 & 0x100400` [M 0x1a5460:664-672: `if ((*(short *)(...+0xf4) == 9) &&
            // ((*(uint *)(weapon_data + iVar4 + 0x68) & 0x100400) != 0)) { uVar8 = weapon+0x6c; } else
            // { Player_SetFiringAnim(...) }`]. Without this gate a held gadget whose fire script has a
            // release frame (the Q-Laser's +0x44 = 20) restarts the script every refire tick, the frame
            // never reaches the release, and the shot only spawns after the button comes up — the
            // user's "when I let go, they actually release and actually do what they're supposed to do
            // briefly".
            let was_fire_state = self.gun.fire_state;
            let rd0 = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
            // The gun's animation is busy while a fire script plays (gun state 9; idle = 0), and the
            // reload branches additionally gate on the VIEW state being 0 [M 0x1a5460:409 — the R-key
            // branch's `if (0xf4 == 0)` — and :447-450, the auto-reload branch's `if (0xf4 != 0)
            // return`]: a reload can only START from the fully idle view, so the switch states 1/3/8
            // gate here too (`switching` is the port's 0xf4 stand-in for them). Without this the
            // variant-swap tick could start a reload whose +0xA4 script overwrote the +0xC8 variant
            // script, and the state-8 wait (which reads the variant's length) never completed.
            let len_of = |h: u32| self.scripts.get(&h).map_or(0.0, |s| s.length as f32);
            let busy = switching || (self.view_script == rd0(0xb0) || self.view_script == rd0(0xb4)) && self.view_time < len_of(self.view_script) - 1.0;
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
                // fix-gadgets2: while the fire script is ALREADY playing (state 9) and +0x68 carries
                // 0x100400, the game leaves it running — no restart [M 0x1a5460:664-672]. fix-laser-beam:
                // that hold applies only to the RELEASE-FRAME family (+0x44 != 0 — the spawn waits for
                // the script's release frame, so a restart would starve it). The Laser/Stunner (+0x68
                // & 0x400, +0x44 = 0 [L weapon_data]) spawn in the FIRE branch itself [M 0x1a5460:681-690]
                // and every refire tick re-queues the script through AnimScriptAdd — which DELETES the
                // playing script and starts fresh [M 0x1c6d40] — keeping the beam + loop alive while held.
                let script = self.gun.firing_anim_script(flags, u16::from_le_bytes([w[2], w[3]]), self.gun.clip, rd(0xb0), rd(0xb4), &mut self.drones.rng);
                if script != 0 && !(was_fire_state && flags & 0x100400 != 0 && fire_anim_len != 0) {
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
                // fix-laser-beam: the +0x68 & 0x400 family (the Laser 0x4E/0x4F, the Stunner 0x4A/0x4C
                // [L weapon_data]) runs the game's state-0xb transition instead [M 0x1a2e38 case 0xb
                // :499-507 — the fire script stopped → AnimScriptAdd(+0xB4), the RELEASE script whose
                // op-4.3 frame 1 STOPS the loop sound (0x12/0x3d [L 0x060004e8/0x06000488]), at 1.5×
                // [M 0x1a5460:677-679]. The port kept fire_state through the +0xB4 play so its own
                // finish lands in the idle arm below (the game's case 0xc → 0 does the same).
                let fire_last = rd(0xb4);
                if self.gun.fire_state && (self.view_script == rd(0xb0) || (fire_last != 0 && self.view_script == fire_last))
                    && self.view_time >= len - 1.0 {
                    if flags & 0x400 != 0 && fire_last != 0 && self.view_script == rd(0xb0) {
                        self.view_script = fire_last;
                        (self.view_time, self.view_speed) = (0.0, 1.5);
                    } else {
                        if flags & 0x1000 == 0 { self.view_script = rd(0xa0); (self.view_time, self.view_speed) = (0.0, 1.0); }
                        self.gun.end_fire_state();
                    }
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
                            // fix-laser-beam: a LOOPED op-1 re-trigger keeps the single voice (the held
                            // laser/stunner restarts its fire script every refire tick [M AnimScriptAdd
                            // 0x1c6d40; L weapon_data +0x40 = 1], re-firing the op-1 60×/s).
                            let looped = self.sfx.as_ref().map(|t| t.get(id)).is_some_and(|e| e.looped);
                            if looped { self.drones.audio.play_loop(id, camera); } else { self.drones.audio.play3d(id, camera, None); }
                            if std::env::var_os("NF_SOUNDLOG").is_some() { eprintln!("tick {}: gun script {:08x} frame {f}: sound {:#x} (remapped from {:#x} for weapon {:#x}, looped {looped})", self.drones.frame, self.view_script, id, c.args[1], wid); }
                        }
                    }
                    // fix-gadgets2, the user's "the sounds keep playing after you stop": `AnimProcessScriptCmds`
                    // op 4 sub-op 3 is `Sound_StopAllWithId(cmd[3])` [M 0x1c1b38:388] — the fire scripts stop
                    // their own loop sounds at a scripted frame. The port played the op-1 starts and dropped
                    // the op-4.3 stops, so every gadget loop (the watch-laser burn, the decryptor decode) leaked
                    // forever — worse while the old restart bug re-fired the script every tick. Same frame
                    // window as op 1; sub-op is args[1], the id args[2] (the port's parse: [frame, sub-op, id]).
                    for c in sc.commands.iter().filter(|c| c.op == 4 && c.args.len() >= 3 && c.args[1] == 3) {
                        let f = c.args[0] as f32;
                        if from < f && f <= to {
                            self.drones.audio.stop_all_with_id(u32::from(c.args[2]));
                            if std::env::var_os("NF_SOUNDLOG").is_some() { eprintln!("tick {}: gun script {:08x} frame {f}: stop sound {:#x}", self.drones.frame, self.view_script, c.args[2]); }
                        }
                    }
                }
            }
            if release {
                // The stunner spawns NO bullet without a target [M Player_WeaponInitBullet
                // 0x1a4bf0:187-200 — the 0x4A/0x4C ray test's miss arm returns before Bullet_init;
                // the port's ray test ran at the top of step_bond into `stunner_target`].
                let spawn_shot = !matches!(self.gun.weapon, 0x4a | 0x4c) || self.stunner_target == Some(true);
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
                    // The fuse is the bullet's BU+0xFC: 210 ticks, 300 when weapon +0x70 & 0x10000, the
                    // mine 0x37 → 420 (7777.0 for a player owner — trigger-detonated; the port has no
                    // detonator and keeps 420) — hardcoded in `Bullet_init` (0x12c5a0:103,196-223), NOT
                    // weapon_data+0x1C (0x34's +0x1C is 5000.0 [L ee_start_07000001.bin, audit
                    // fix-player2 claim 8]). The port's `Grenade.range` doubles as the tick fuse.
                    let wt = u16::from_le_bytes([w[2], w[3]]);
                    let fuse = if u32::from_le_bytes(w[0x70..0x74].try_into().unwrap()) & 0x10000 != 0 { 300.0 } else { 210.0 };
                    let fuse = if wt == 0x37 { 420.0 } else { fuse };
                    // The blast damage: `weapon_data+0x0C` [M 0x12cd98 disasm: `lwc1 $f14, 12($v0)` →
                    // Explode_Create's f14 → EXPLODEINFO+0x28 (Explode_CollisionHandler 0x17cde0:13-15);
                    // radius = $f12/$f13 = weapon_data+0x08]. The `addiu $t2, $zero, 0x64` (100) is the
                    // 10th argument, an u8 `Explode_Create` never reads (its prologue — 0x17cbd0 — reads
                    // only a0-a3, f12-f14, t3 and the stack u16). fix-explosions2: the disc labels name
                    // the records [L USATxt.dat] — the frag is 0x34 'Fragmentation Grenade' (radius 8,
                    // damage 100: the real kill through Explode_Propagate), 0x35 'Stun Grenade' (10/0,
                    // the flash branch), 0x36 'Smoke Grenade' (0/0, the occlude/re-arm branch). The old
                    // hardcode's sin was killing with EVERY type and on impact; the per-type records
                    // route each grenade to its own death branch [M 0x12dd60/0x12e104/0x12e1c0].
                    self.drones.grenades.push(nf_game::grenade::Grenade {
                        pos: camera,
                        velocity: [sdir[0] * speed, sdir[1] * speed, sdir[2] * speed],
                        speed, travelled: 0.0, range: fuse, damage: f32_at(0x0c), radius: f32_at(0x08),
                        // The FIRED weapon's id, not the record's +0x02 base: the MGL's 'Delay' twin
                        // 0x2b bases to 0x2a, and the death branches (the impact explode [M
                        // 0x12d068-0x12d0bc], the flash/smoke routing [M 0x12dd60]) read the BULLET'S own
                        // record. The grenade family bases to itself [L ee_start_05.bin +0x02 sweep], so
                        // this only re-points the twins.
                        weapon_type: self.gun.weapon as u16,
                        mesh: self.weapons.projectile_mesh(self.gun.weapon as usize),
                        owner: usize::MAX, state: 0, bounces: 0, smoke_popped: false,
                    });
                } else if spawn_shot {
                    self.last_shot = Some(self.drones.frame);
                    self.shot_tick = true;
                    // `Player_MuzzleFlash` 0x1a4298 runs on the shot tick: `Light_Create(radius =
                    // weapon_data+0x58, intensity 2.0, the muzzle (the view model's datum 0; the sim
                    // approximates it eye + aim·0.5 [S]), rgb = +0x56/+0x55/+0x54, channel 0, life 0 →
                    // +0x20 = 1 (one tick), kind 1)` — its `Rand_Rand(0)` draw sits between the accuracy
                    // draws and the bullet's spread draws, as in the game. The smoke: `Gas_Init`'s
                    // `Rand_FRand(2π)` draw, unless weapon_data+0x68 & 0x2000 (no smoke).
                    // The game's gate [M 0x1a42e8]: weapon_data+0xDC != 0 && weapon_data+0x48 != 0.
                    let light_gate = u32::from_le_bytes(w[0xdc..0xe0].try_into().unwrap()) != 0
                        && u32::from_le_bytes(w[0x48..0x4c].try_into().unwrap()) != 0;
                    if light_gate {
                        let radius = f32::from_le_bytes([w[0x58], w[0x59], w[0x5a], w[0x5b]]);
                        let pos = [camera[0] + dir[0] * 0.5, camera[1] + dir[1] * 0.5, camera[2] + dir[2] * 0.5];
                        self.drones.lights.create(radius, 2.0, pos, [w[0x56], w[0x55], w[0x54]], 0, 0, 1, 0, &mut self.drones.rng);
                        let flags68 = u32::from_le_bytes(w[0x68..0x6c].try_into().unwrap());
                        if flags68 & 0x2000 == 0 {
                            // `Gas_Init`'s `Rand_FRand(2π)` draw (the smoke puff's angle), in the game's order.
                            let _angle = self.drones.rng.frand(6.2831854820251465);
                            // The puff itself: models-misc's gas system (`Gas_Init(0.05, 0.1, …, 0x2000039, 7,
                            // 0x48403080, 1)` [M 0x1a4298]); Gas_Init's own draws stay un-consumed (G-033). Life [S] 8 ticks.
                            self.drones.gas.init(pos, 0.0, [0.0; 3], 0.05, 0.1, 7, [0x48, 0x40, 0x30, 0x80], 8.0);
                        }
                    }
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
        // The class byte the AI's `ObjectIsArmed` 0x178768 reads: Bond obj+0xdc+0x62. `Player_WeaponSelect`
        // 0x1a2cc0 copies the request (+0x63) into it at the switch's START (the `weapon_data[+0x64].+0xc4
        // == 0` arm — +0x64 is 0x47 from InitWeapon 0x1a7ef8:137-139), so mid-holster the AI already reads
        // the TARGET weapon: at the party Bond reads as the fists (unarmed, class 1) from the first tick,
        // not as the carried P2K for the ~100 switch ticks (playtest item 1: the entrance bouncers shot).
        self.drones.bond_weapon = if self.gun.switch_state != 0 && self.gun.switch_request != 0x47 {
            self.gun.switch_request
        } else {
            self.gun.weapon
        };
        // The use key (`Player_Activate` 0x1a5170): the doors its sphere — one unit ahead of the head — meets
        // open, swing or thud locked (docs/systems/world-objects.md "Doors"); the player object updates before
        // the control list's doors, so they see the activation this same tick.
        let mut crept = false;
        if use_key {
            // `Player_Activate` 0x1a5170:66-97's creepwall arm comes FIRST: with the third-icon
            // context 6 (BLData+0x95f, the game's ONLY gate [M 0x1a5170:66 `if (BL+0x95f == 6)`])
            // the use key scans every object for class '+' (0x2B) and calls
            // `Player_CollCreepWall` on the NEAREST within the squared distance 1000 (no capsule
            // test) [M] — returning before the head-ray pick ever runs. No proximity fallback: at
            // the 07000005 gate door the 0x1D LOAD_LEVEL trigger sits ~24 units from the second
            // vent's creepwall (squared 579 < 1000), and the game reads +0x95F = 0xFF there
            // [L PCSX2 2026-10-13] — the scan alone must never creep.
            if self.bond.substate == nf_game::player::SS_WALK {
                let mut cbest: Option<(f32, usize)> = None;
                for (i, w) in self.creepwalls.iter().enumerate() {
                    let d = (w.pos[0] - self.bond.pos[0]).powi(2) + (w.pos[1] - self.bond.pos[1]).powi(2) + (w.pos[2] - self.bond.pos[2]).powi(2);
                    if d >= 1000.0 { continue; }
                    if cbest.map_or(true, |(bd, _)| d < bd) { cbest = Some((d, i)); }
                }
                if self.bond.icon_ctx == 6 {
                    if let Some((_, i)) = cbest {
                        let cctx = nf_game::player::LadderCtx { creepwalls: &self.creepwalls, ..Default::default() };
                        self.bond.start_creep(i, &self.world, &cctx);
                        crept = self.bond.substate == nf_game::player::SS_CREEP;
                    }
                }
            }
        }
        if use_key && !crept {
            self.drones.player_use_doors(&d, camera, look, self.drones.frame);
            // `Player_Activate` 0x1a5170 case '%': the pick's class-0x25 SS objects activate
            // (`SS_Activate` 0x1a9ea8 — state + mask gate, then the channel latch). The game's
            // pick is a 1.0-radius sphere at head+look vs the SS's mesh; the port takes the
            // nearest E-armed SS under the head ray within 2.2 (the mesh extent of a hatch panel) (the doors' reach stand-in shape).
            let head = [camera[0], camera[1], camera[2]];
            let mut best: Option<(f32, usize)> = None;
            for (i, s) in self.drones.sss.list.iter().enumerate() {
                if s.state == 0 || s.mask & 4 == 0 { continue; }
                let p = s.pos();
                let to = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
                let proj = to[0] * look[0] + to[1] * look[1] + to[2] * look[2];
                if proj < 0.0 { continue; }
                let cl = [head[0] + look[0] * proj, head[1] + look[1] * proj, head[2] + look[2] * proj];
                let dd = ((p[0] - cl[0]).powi(2) + (p[1] - cl[1]).powi(2) + (p[2] - cl[2]).powi(2)).sqrt();
                if dd > 2.2 { continue; }
                if best.map_or(true, |(bd, _)| dd < bd) { best = Some((dd, i)); }
            }
            // The same creepwall arm at the second activation site: icon context 6 ONLY
            // [M Player_Activate 0x1a5170:66 `if (BL+0x95f == 6)`; L the 0xFF live pin at the
            // gate door] — the scan's own proximity never creeps.
            let mut crept = false;
            if self.bond.substate == nf_game::player::SS_WALK {
                let mut best: Option<(f32, usize)> = None;
                for (i, w) in self.creepwalls.iter().enumerate() {
                    let d = (w.pos[0] - self.bond.pos[0]).powi(2) + (w.pos[1] - self.bond.pos[1]).powi(2) + (w.pos[2] - self.bond.pos[2]).powi(2);
                    if d >= 1000.0 { continue; }
                    if best.map_or(true, |(bd, _)| d < bd) { best = Some((d, i)); }
                }
                if self.bond.icon_ctx == 6 {
                    if let Some((_, i)) = best {
                        let cctx = nf_game::player::LadderCtx { creepwalls: &self.creepwalls, ..Default::default() };
                        self.bond.start_creep(i, &self.world, &cctx);
                        crept = self.bond.substate == nf_game::player::SS_CREEP;
                    }
                }
            }
            if !crept {
            if let Some((_, i)) = best {
                let frame = self.drones.frame;
                let st = self.drones.sss.list[i].state;
                let mk = self.drones.sss.list[i].mask;
                let _ = (st, mk);
                let mut ch = std::mem::take(&mut self.drones.channels);
                let hit = self.drones.sss.list[i].activate(&mut ch, frame);
                self.drones.channels = ch;
                if hit { eprintln!("[ss] SS #{i} (placement {}) activated by the use key", self.drones.sss.list[i].placement); }
            }
            }
        }
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
        // `PlarStat_LogUpdateElapsedTime(0)` 0x1995a0 — `Player_Update` 0x193600:132 calls it once per
        // tick; `now` is the port's frame-derived 1/100 s clock (deterministic; fix-progress).
        self.drones.progress.log_update_elapsed(nf_game::progress::PlrMissionStats::now_100ths(self.drones.frame));
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
        // Script op-0x1e text commands (`Script_TextStart` 0x1db2b8 → `Text_AddMsg(-1, 0, 4,
        // Txt_BindLabel(label), 0, time)` [M]): the raw labels drain to HUD messages with the same
        // bank binding as the pickup texts (fix-script-commands).
        let lang = self.lang.take();
        let mut new_msgs: std::collections::VecDeque<Msg> = Default::default();
        if let Some(lang) = lang.as_ref() {
            for (label, time) in self.drones.script_msgs.drain(..) {
                let text = lang.bind(label).unwrap_or("").trim().to_string();
                if !text.is_empty() { new_msgs.push_back(Msg::new(text, time as f32)); }
            }
        }
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
                // Case 5 [M 0x18e650:229-233 + the shared tail]: message 0x2000013.
                nf_game::pickup::Taken::Message => lang.bind(0x200_0013).unwrap_or("").to_string(),
                // Case 4 [M 0x18e650]: the pickup's own label (PICKUPINFO+0x34); the port's pickups
                // do not carry it, and the only kind-4 pickup on the disc data so far labels 0.
                nf_game::pickup::Taken::Label => String::new(),
            };
            if !text.is_empty() { new_msgs.push_back(Msg::new(text, 180.0)); }
        }
        self.hud.taken_seen = self.drones.taken.len();
        for m in new_msgs { self.hud.msgs.push_back(m); }
        self.lang = lang;
        // The mission layer's events (`Mission_Update` 0x182c60 / `Mission_MonitorObjectives`
        // 0x1827d8, docs/systems/mission.md): type-2 objective notices, the FailLabel, the type-3
        // mission banner and the level-end / restart decisions. `Text_AddMsg` only QUEUES the
        // message — every sound and every drawn pixel belong to the pane update
        // (`HUD_UpdateStatusPane` 0x19c310 cases 2/3), which cannot open while a script camera
        // holds the viewer's +0x2a gate [M 0x1b11d8:16-18]. The port therefore drains the events
        // BEFORE the frozen check (a banner queued mid-cut-scene waits in the queue, its timer
        // untouched) and plays the SFX when the message first displays, below.
        // fix-objective-banner: the drain used to sit behind the frozen check and play the SFX as
        // the event landed, so on 07000007 the Kimiko trigger's ch 0x48 announced (and played
        // 1532+469) on the trigger tick — one tick before the scene's `Script_CameraStart` set the
        // freeze — which the user heard as "obj doesnt wait for kimiko cutscene and plays the
        // sound immediately".
        let mut events = std::mem::take(&mut self.drones.mission_events);
        for ev in events.drain(..) {
            match ev {
                nf_game::mission::Event::Objective { label, is_new: _, ticks, failed } => {
                    let complete = label == 0x0200_0053;
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
                    // (fix-objective-messages: `Text_AddMsg` 0x1b0c30 also wraps the stored string
                    // with `Font_WordWrapString` at TextMsgFormats[2]'s 480 [L 0x2bf108+0x14] and
                    // scales the life by the wrapped/orig length [M 0x1b0c30's +0x18 store]. The
                    // port wraps at draw time — the font lives in the scene's HudTables, which the
                    // Sim-side drain doesn't hold; the life scaling is not ported, see the report.)
                    self.hud.objective_msgs.push_back(Msg { text, time: ticks as f32, status, failed, life: ticks as f32, opened: false, second: false });
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
                nf_game::mission::Event::PlayerDead => {} // the red fade is render-side; the menu arrives below
                nf_game::mission::Event::LevelEnd { next } => {
                    // `ResetMap_LevelToLoad(LevelToEndTo)` [M 0x182c60:73]. The 0x0730xxxx codes
                    // are DRIVING.ELF levels the port cannot load — logged and skipped.
                    if next & 0xffff_0000 == 0x0700_0000 {
                        self.drones.level_exit = Some(next);
                    } else {
                        eprintln!("ission] complete → LevelToEndTo {next:08x} (driving level, not ported)");
                    }
                }
                nf_game::mission::Event::MenuOpen => {
                    // The 0x40000042 fail/death menu [M 0x182c60:78]. Until 180-fix-menus draws
                    // it, the port's auto-restart answers Restart immediately (the flag keeps
                    // the view free to answer instead via `fail_choose_restart`/`_quit`).
                    if self.drones.auto_restart { self.drones.fail_choose_restart(); }
                }
            }
        }
        // `Text_UpdateMsg` 0x1b11d8: time −= 1 per frame, delete at ≤ 0 — but the viewer's +0x2a HUD
        // flag returns early while a cut-scene camera runs [M 0x1b11d8:16-18], freezing every message
        // until the NIS ends (playtest item 5). The port: no countdown and no bar close while frozen.
        if self.drones.hud_frozen {
            return;
        }
        for m in &mut self.hud.msgs { m.time -= 1.0; }
        self.hud.msgs.retain(|m| m.time > 0.0);
        if self.hud.msgs.is_empty() {
            if self.hud.bar_close > 0.0 { self.hud.bar_close = (self.hud.bar_close - 16.0).max(0.0); }
        } else {
            self.hud.bar_close = HudState::BAR_W;
        }
        // `Text_UpdateMsg` ticks only the displayed head [M 0x1b11d8:48-73 — the out-param
        // `*param_3` is the head node; the queue behind waits]; when it expires the next message
        // becomes the head and the pane re-opens for it (case 0 → 1). The port used to tick every
        // queue member in parallel, which expired the banners behind the front one before they
        // ever showed.
        if let Some(m) = self.hud.objective_msgs.front_mut() {
            m.time -= 1.0;
            if m.time <= 0.0 { self.hud.objective_msgs.pop_front(); }
        }
        if self.hud.objective_msgs.is_empty() {
            if self.hud.objective_close > 0.0 { self.hud.objective_close = (self.hud.objective_close - 16.0).max(0.0); }
        } else {
            self.hud.objective_close = HudState::OBJECTIVE_W;
        }
        if let Some(m) = self.hud.mission_msgs.front_mut() {
            m.time -= 1.0;
            if m.time <= 0.0 { self.hud.mission_msgs.pop_front(); }
        }
        if self.hud.mission_msgs.is_empty() {
            if self.hud.mission_close > 0.0 { self.hud.mission_close = (self.hud.mission_close - 32.0).max(0.0); }
        } else {
            self.hud.mission_close = HudState::MISSION_H;
        }
        // The banner SFX, where the game plays them: the pane update. Case 2 schedules the pair per
        // display (new objective: PlayMe1st 1532, PlayMe2nd 469; "OBJECTIVE COMPLETE": PlayMe1st 468
        // and ObjectiveSFXTriggered2 preset so the 2nd never plays) [M 0x19c310:148-164]; case 3
        // fires PlayMe1st in the message's first 60 ticks — only while `ScriptCam == 0` (a cut-scene
        // holds it) [M 0x19c310:219-229] — and PlayMe2nd once the steady phase begins (remaining + 60
        // ≤ life) [M 0x19c310:232-235]. The port's pane cannot open while `hud_frozen` (the
        // viewer+0x2a gate above), so at this point the ScriptCam gate has already cleared.
        if let Some(m) = self.hud.objective_msgs.front_mut() {
            let cam = self.bond.head();
            if !m.opened {
                m.opened = true;
                let id = if m.status == 0x0200_0053 { 468 } else { 1532 };
                self.drones.audio.play3d(id, cam, None);
            } else if !m.second && m.status != 0x0200_0053 && m.time <= m.life - 60.0 {
                m.second = true;
                self.drones.audio.play3d(469, cam, None);
            }
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

    /// The emitter particles as camera-facing quads (`Emitter_Draw` 0x17b708 path A →
    /// `psiDrawParticleList` 0x122d50: a size-`def.size` rgba sprite per alive slot). The game sizes
    /// the sprite in screen space (`4·size / (PS2ParticleSize · PS2SnowSizeMult)`, 0x11e2e8:67) — the
    /// port takes the def size as the world-space half-extent [S], the way the snow quads do. The
    /// white rgb in the game's sprite comes from the bound def texture (the font-slot hash, def+0x04)
    /// modulated by the slot rgba [M 0x119910:19]; the port has no per-def texture, so the colour is
    /// the slot rgba itself — listed in the report's render follow-ups.
    pub fn particle_quads(&self, right: [f32; 3], up: [f32; 3]) -> Vec<Vertex> {
        let sprites = self.drones.fx.particle_sprites();
        let mut out = Vec::with_capacity(sprites.len() * 6);
        for s in sprites {
            let (r, u) = (right.map(|v| v * s.size), up.map(|v| v * s.size));
            let c = |sr: f32, su: f32, uv: [f32; 2]| Vertex {
                pos: [s.pos[0] + r[0] * sr + u[0] * su, s.pos[1] + r[1] * sr + u[1] * su, s.pos[2] + r[2] * sr + u[2] * su],
                uv, rgba: s.rgba, nrm: [0; 4],
            };
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
        nf_game::drone::Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.level, sequences: &self.seqs, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models , surfaces: self.surfaces.as_ref(), voices: self.voices.as_deref(), difficulty: self.drones.difficulty }
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
        // G-044 [M Player_WeaponRecoil 0x1a3de8:52-57]: both tilt terms divide by `glb_viewer+0x108`
        // (the zoom — `Player_PositionCamera` 0x18f5d8:129 feeds it BL+0x8D0, the port's `self.zoom`),
        // and the x term scales by `viewer+0x114` = 4/3 on a 4:3 screen [L ee_start_05 viewer
        // 0x1fc6c10+0x114 = 0x3faaaaab; `Camera_CalcViewAngles` 0x1baf70 writes 16/9 there in the
        // widescreen mode the port doesn't model]. A zoomed view used to tilt the gun as if unzoomed.
        let (gy, gp) = (
            4.0 / 3.0 * cx * std::f32::consts::FRAC_PI_6 / self.zoom,
            cy * std::f32::consts::FRAC_PI_6 / self.zoom,
        );
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
    /// `Some((slot, mesh))`: a projectile mesh slot (`Bullet_update` state 0 sets the weapon's `+0x5C` mesh on the
    /// bullet object, `View_SetDrawInAllViews`, disasm 0x12e8f0-0x12e900 [M]). The slot indexes the simulation's
    /// projectile lists (0..GRENADE_SLOTS grenades, then the drone and player bullets); the mesh selects which of
    /// the slot's per-mesh instances shows this frame.
    pub projectile: Option<(u8, u32)>,
    /// The character's gun once dropped (`DroneWeap_DropWeapon`): drawn at the pickup's matrix while it lies there.
    pub dropped: bool,
    /// A script entity (script hash, stream, entity hash): drawn at the simulation object's matrix while it exists.
    pub script: Option<(u32, u8, u32)>,
    /// `Some(slot)`: a gas/smoke puff (`Gas_Init` 0x1aacc8) showing `sim.drones.gas.puffs[slot]` —
    /// a billboarded `smoke_1_dust` quad scaled by the puff's scale, faded by its rgba.
    pub gas: Option<u16>,
    /// fix-impacts-render: an impact-effect draw slot — kind 0 a bullet decal (`Effect_Bullet`
    /// 0x1ce8a8's 200-node FIFO, `sim.drones.effects.decals[slot]`), kind 1 a shell casing
    /// (`Bullet_InitCasingLists` 0x12fd48's 32-object pool, `sim.drones.casings.list[slot]`),
    /// kind 2 a debris particle (`Debris_Create` 0x1aaa00, `sim.drones.effects.debris[slot]`).
    /// The u32 is the slot's mesh hash (one instance per distinct `EffectInfo`/`ammo_data` hash
    /// the level archive resolves, the `hashtable_set_sub_object_to_entity_gfx` mesh); the slot
    /// draws only while the live effect carries THAT mesh — free slots collapse far below the world.
    pub effect: Option<(u8, usize, u32)>,
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
    /// fix-facial2: the vertex's morph base index — the skinning word's upper half-word (the short at
    /// +0x1E, −1 = none), which `SkinIt` 0x10bf30 indexes the morph targets with:
    /// `target + (morph + channel) * 12`.
    pub morph: i16,
}

/// Per-vertex GPU skinning data (`Scene::skin_layout`); world vertices carry `a = u32::MAX` (static).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinAttr { pub a: u32, pub b: u32, pub w: f32, pub col: u32 }

/// The palette / colour-table layout of `Scene::skin_layout`.
pub struct SkinLayout { pub char_base: Vec<u32>, pub attach_base: u32, pub sl_base: u32, pub door_base: u32, pub pick_base: u32, pub n_mats: u32, pub col_att: u32, pub col_pick: u32, pub col_sl: u32, pub col_one: u32 }

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
    /// Spare banks (fix-weapon-sounds): the other SBINFO slots' `SB_<slot>.{SHF,SFX,SBF}`, loaded so a
    /// weapon given on a level whose own bank lacks its sounds still plays them from a bank that holds
    /// them (`load_spare_bank`). The level's own bank stays the primary lookup [M `Sound_Ready`].
    pub sound_spares: Vec<(usize, nf_formats::soundbank::Bank, Vec<u8>)>,
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
    /// Models-misc: how many static dynamic entities got a World-layer instance, per placement kind
    /// (`misc_static_sweep` in `build`). Zero for a kind the census says the level carries means the
    /// model is missing (see docs/models/misc.md for the per-kind creator evidence).
    pub misc_drawn: BTreeMap<u8, usize>,
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
type Groups = BTreeMap<(Layer, [u32; 3], u32, u32, Option<(usize, u32)>, Draw, u16), (Vec<Vertex>, Vec<u32>, Vec<CharVertex>, [f32; 3], f32, Option<u16>, u32)>;

/// fix-safe08: the pickups the level's scripts spawn through `Script_CreateEntity` 0x1d8968's
/// obj_type-0xF0 arm (`Pickup_Create` 0x18dda0 at the stream's posed position, rest = 1 [M disasm
/// 0x1d8bcc-0x1d8c20]). Each such (script, stream) gets a RESERVED pickup slot: built inert
/// (`gone`, so the draw loop and the take test skip it) and activated by the sim when the script
/// actually spawns it. The slot keeps the renderer's per-index draw (`placed_pickup_pose`) working
/// for a pickup that does not exist at load. 07000008's SafePickUps script 0x060009E2 spawns the
/// safe's briefcase (kind 4, switch 0x4A) and the rocket launcher (id 0x2C) at the safe's shelves
/// this way [D the level-bin script; the parked 0xF0 placements sit 10 units east, inside the east
/// wall]. Scans `level.nis` sorted by hashcode (deterministic); pack scripts (07x00008) are not in
/// the level archive and fall back to the runtime's no-slot push (takeable, by-hash mesh).
fn script_pickup_reservations(level: &Level, weapons: &nf_formats::weapon::WeaponData, models: &std::collections::HashMap<u32, nf_game::pickup::Model>) -> (Vec<nf_game::pickup::Pickup>, Vec<((u32, u8), usize)>) {
    let mut out = Vec::new();
    let mut slots = Vec::new();
    let mut hashes: Vec<u32> = level.nis.keys().copied().collect();
    hashes.sort_unstable();
    for hash in hashes {
        let nis = &level.nis[&hash];
        for (si, st) in nis.streams.iter().enumerate() {
            let cmds = st.decode();
            let Some(nf_formats::nis::Cmd::Entity { hash: eh, obj_type: 0xf0, params, .. }) = cmds.iter().find(|c| matches!(c, nf_formats::nis::Cmd::Entity { obj_type: 0xf0, .. })) else { continue };
            let p = |k: usize| params.get(k).copied().unwrap_or(0);
            let target = level.by_hash.get(eh).copied();
            let Some((om, oi)) = target else { continue };
            let obj_ = &level.maps[om].objects[oi];
            let Some(&m0) = models.get(&obj_.hashcode) else { continue };
            let key = nis.keys.get(st.first_key as usize).map(|k| k.pos).unwrap_or([0.0; 4]);
            let Some(mut pk) = nf_game::pickup::Pickup::create_placed(
                [key[0], key[1], key[2]], [0.0; 3], p(0) as u16, p(1) as u16, p(2) as u16 as i16,
                p(3) as u16, p(4) as u16, p(5) as u16, 0, obj_.hashcode,
                nf_game::pickup::Model { centre: obj_.centre, radius: obj_.radius }, weapons, models) else { continue };
            pk.gone = true; // reserved: inert until the script spawns it
            pk.obj = Some((om, oi));
            slots.push(((hash, si as u8), out.len()));
            out.push(pk);
        }
    }
    (out, slots)
}

/// fix-facial2: `SkinIt` 0x10bf30's morph term — `pos += Σ_m weight[m] × target(base + (morph + m) × 12)`,
/// the 12 bytes read as three f32 (the decompile's `auVar14._0_12_ = *(undefined1 (*) [12])lVar1` over the
/// base `parsemap_block_morph_data` 0x1d23e8 stores at the gfx record's +0x14). Out-of-range channels (a
/// target array shorter than the mesh's index space) contribute nothing, as the game's count-bounded loop.
fn morph_vertex(mut pos: [f32; 3], morph: i16, weights: &[f32], targets: &[u8]) -> [f32; 3] {
    let f32_at = |i: usize| -> Option<[f32; 3]> {
        let o = i * 12;
        if o + 12 > targets.len() { return None; }
        Some([
            f32::from_le_bytes(targets[o..o + 4].try_into().unwrap()),
            f32::from_le_bytes(targets[o + 4..o + 8].try_into().unwrap()),
            f32::from_le_bytes(targets[o + 8..o + 12].try_into().unwrap()),
        ])
    };
    for (m, w) in weights.iter().enumerate() {
        if let Some(d) = f32_at(morph as usize + m) {
            for k in 0..3 { pos[k] += w * d[k]; }
        }
    }
    pos
}

fn add_instance(level: &Level, inst: &Instance, layer: Layer, character: Option<(u32, Option<u32>)>, groups: &mut Groups, bounds: Option<&mut ([f32; 3], [f32; 3])>, searchlight: Option<u16>, door: Option<u16>, pickup: Option<u16>, mult: [f32; 3], cels: &[Cel], chain: u32, node: u32) {
    let obj = &level.maps[inst.map].objects[inst.object];
    let mut m = inst.matrix;
    if layer == Layer::SkyCamera { m[3] = [0.0, 0.0, 0.0, 1.0]; }
    let mut bounds = bounds;
    // The pass and fog by layer: sky slots with placement parameter 4 = 1 go before the world (fogged), others after
    // the fog pass. World: the static cel primaries (chain set) draw in list 0 regardless of glist flags —
    // `View_AddCels` (0x1e6ba0) pushes the primary into the list-0 ring without consulting flags bit 0x1 (only the
    // +0x18 chain records and `View_AddObjects`' objects pick their list by flags). Dynamic World objects
    // (chain == u32::MAX) keep the flag-0x1 alpha list (pass 3, back-to-front, `View_AddObjects` 0x1e62d8).
    let param4 = inst.placement.params.iter().find(|(i, _)| *i == 4).map(|&(_, v)| v).unwrap_or(0);
    let (pass, fog) = match layer {
        Layer::SkyCamera | Layer::SkyFixed => if param4 == 1 { (0, 1) } else { (4, 0) },
        Layer::World => if chain == u32::MAX && obj.flags & 1 != 0 { (3, 2) } else { (1, 1) },
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
    // The light-selection flags: the glist flags word's low 16 bits — `Control_SetGList` 0x134960
    // copies `celglist+0x44` to `obj+0xF0` and `View_DrawObjList` 0x1e69a0 / `View_DrawCelGlist`
    // 0x1e6538 / `View_SetupRenderModes` 0x1e78b0 (the anim-object path) pass its u16 to
    // `Lights_CalcClosestLights`. `MapObject.flags` is that word [D level.rs ENTITY_PARAMS +4].
    let lflags = (obj.flags & 0xffff) as u16;
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
        let (vs, is, cv, c, r, gcel, gnode) = groups.entry((layer, ikey, lkey, chain, key, draw, lflags)).or_default();
        *gcel = cel;
        *gnode = node;
        *c = centre;
        let base = vs.len() as u32;
        // fix-render-artifacts: the A-11e sphere cull must be SOUND. fix-render2 assumed the placement-matrix
        // glist sphere is exact, but the glist radius can undershoot the drawn geometry [D 0700000c.bin: the
        // floor decal at glist centre (28.54, 0.33, −182.49) carries r 0.6 while its drawn verts span a
        // 1.1×0.9×1.0 box — half-diagonal 0.87]. A too-small sphere drops draws whose geometry IS in the
        // frustum (the user's pale fog-coloured patches: the skipped quad uncovers the fog-coloured clear).
        // The game's own cull consults the bbox as well as the sphere (`View_AddObjects` 0x1e62d8 passes
        // obj+0x8c sphere AND obj+0x80 bbox to `Vision_InView` 0x1e89e0), so the group's real bound belongs
        // in the test: every batch expands its group's radius to cover its transformed verts.
        let mut batch_d2 = 0.0f32;
        for v in &b.verts {
            let p = transform(&m, v.pos);
            let d2 = (p[0] - centre[0]).powi(2) + (p[1] - centre[1]).powi(2) + (p[2] - centre[2]).powi(2);
            if d2.is_finite() && d2 > batch_d2 { batch_d2 = d2; }
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
            if let Some((c, attachment)) = character { cv.push(CharVertex { character: c, pos: v.pos, weight: v.weight, slots: v.slots, attachment, searchlight: None, door: None, pickup: None, morph: (v.w >> 16) as i16 }); }
            else if let Some(sl) = searchlight { cv.push(CharVertex { character: 0, pos: v.pos, weight: v.weight, slots: v.slots, attachment: None, searchlight: Some(sl), door: None, pickup: None, morph: -1 }); }
            else if let Some(dr) = door { cv.push(CharVertex { character: 0, pos: v.pos, weight: v.weight, slots: v.slots, attachment: None, searchlight: None, door: Some(dr), pickup: None, morph: -1 }); }
            else if let Some(pk) = pickup { cv.push(CharVertex { character: 0, pos: v.pos, weight: v.weight, slots: v.slots, attachment: None, searchlight: None, door: None, pickup: Some(pk), morph: -1 }); }
        }
        // The batch's bound joins the group's (max across the group's batches — the entry's radius persists).
        *r = radius.max(*r).max(batch_d2.sqrt());
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

/// fix-impacts-render: a shell casing's draw matrix. `Bullet_casing_update` 0x12faa0 adds the eject spin to the
/// object's euler every tick (`rot(+0x50..+0x5b) += spin` [M]); `Bullet_init_casing_ex` 0x12f678 starts the euler
/// at the spherical angles of the eject direction (`vecutil_cartesian_to_spherical_acc`, obj+0x58 = 0 [M]). The
/// port reconstructs the eject direction by undoing the tick gravity (`vel[1] += GRAVITY·REC²·age`) and takes the
/// tracer's +Z euler convention for it [S — the game eulers the weapon-local dir through the view-model basis;
/// the port's eject origin/basis is already a documented stand-in, docs/atlas/effect.md].
fn casing_matrix(c: &nf_game::casing::Casing) -> Mat4 {
    let g = 9.8 * (1.0 / 60.0) * (1.0 / 60.0);
    let v0 = [c.vel[0], c.vel[1] + g * c.age, c.vel[2]];
    let horiz = (v0[0] * v0[0] + v0[2] * v0[2]).sqrt();
    let pitch0 = -v0[1].atan2(horiz);
    let yaw0 = v0[0].atan2(v0[2]);
    let eul = [pitch0 + c.spin[0] * c.age, yaw0 + c.spin[1] * c.age, c.spin[2] * c.age];
    nf_formats::placement::object_matrix(eul, c.pos)
}

/// Tracer slots (bullets drawn at once).
const TRACER_SLOTS: usize = 32;
/// Projectile mesh slots (`models-weapons-items`): 0..GRENADE_SLOTS are the simulation's grenades
/// (`Sim::drones.grenades`), GRENADE_SLOTS..+TRACER_SLOTS the drone bullets (`Sim::drones.bullets`, the same
/// indices the tracer attachments use), and the last PLAYER_BULLET_SLOTS Bond's (`Sim::drones.player_bullets`).
/// Each (slot, mesh) pair has its own Effect attachment; the live projectile's `+0x5C` mesh picks the one that
/// shows (`Bullet_update` disasm 0x12e8f0 [M]).
const GRENADE_SLOTS: usize = 8;
const PLAYER_BULLET_SLOTS: usize = 8;
/// fix-impacts-render: the impact-effect draw pools. Decals: the game's 200-node FIFO
/// (`Effect_InitLists` 0x1cf268: `DList_Init(0x3496e0, 0x60, 200)` [M]). Casings: the 32-object
/// round-robin pool (`Bullet_InitCasingLists` 0x12fd48 [M]). Debris: unbounded in the game (a
/// class-0x40 object per particle, ≤ 6 per impact, ≤ ~54 ticks) — the port caps the draw slots.
const DECAL_SLOTS: usize = 200;
const DEBRIS_SLOTS: usize = 128;

/// The drone's height state after `DroneAnim_PreTransCallback` (0x13b490): `Drone+0xA0` = hip height − 0.02, at least
/// 0.2 (it starts at 1.0 and keeps its value while the hip height is 0).
fn drone_height(hip: f32) -> f32 {
    if hip == 0.0 { 1.0 } else { (hip - 0.02).max(0.2) }
}

impl Scene {
    /// The script player's overlay (fix-script-commands): the fade, the letterbox borders and the
    /// op-0x14 sprites — see `hud::script_overlay`. Appended to the HUD frame OUTSIDE the cut-scene
    /// gate (the game draws these over the blanked HUD).
    pub fn script_overlay_hud(&self, aspect: f32) -> crate::gpu::HudFrame {
        let Some(sim) = self.sim.as_ref() else { return Default::default() };
        let sprites: Vec<(u32, [f32; 4])> = sim.drones.scripts.objects.list.iter()
            .filter_map(|o| match o { Some(nf_game::nis::Obj::Sprite { hash, rect, .. }) => Some((*hash, *rect)), _ => None })
            .collect();
        crate::hud::script_overlay(sim.drones.view_fade, sim.drones.letterbox, &sprites, &self.hud_textures, aspect)
    }

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
        instances.retain(|i| !matches!(i.placement.flags & 0xff, 0x2a | 0x22 | 0xd4));
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
                    let mut p2 = nf_game::pickup::Pickup::create_placed(p.translation, p.rotation, type_, id, rounds, switch_channel, sound, respawn as u16, lifetime, model, bounds, &placed_weapons, &hoist_models)?;
                    p2.obj = Some(obj);
                    Some(p2)
                })
                .collect();
            // fix-safe08: the scripts' obj_type-0xF0 Entity spawns reserve slots after the placed
            // pickups, in the SAME order the sim build appends them (the draw slot k must equal the
            // sim's pickups[k]).
            let (reserved, _render_slots) = script_pickup_reservations(level, &placed_weapons, &hoist_models);
            let reserved_len = reserved.len();
            let mut placed_pickups = placed_pickups;
            placed_pickups.extend(reserved);

        // The spawn comes from the placement records even when entities are not drawn (Player1 is not a 0x8000 cel).
        let start = level.instances(false).0.iter()
            .find(|i| level.maps[i.map].objects[i.object].name.trim() == "Player1")
            .map(|i| (i.placement.translation, i.placement.rotation[1]));
        let mut hidden = 0;
        let mut ladders_drawn = 0usize; // the sweep below counts the drawn ladder instances (fix-ladder2)
        let mut misc_drawn: BTreeMap<u8, usize> = BTreeMap::new(); // models-misc static dynamic entities
        // The level's room cels (see `Cel`): parsed before the draw so the static 0x800 world objects can snap
        // to their cel's ambient exactly as the game's link does at level start (`build_LinkToRoom` 0x1c9680 →
        // the snap in `control_link_object_to_cel` 0x1346b8; channels are all 0 here, so triple 1 reads).
        let (room_cels, mut cel_graph) = parse_cel_graph(level);
        let sim_cels = room_cels.clone();
        // Bug 3 diagnostic (`NF_COLL=1`): log objects that have collision but are not drawn — the "invisible walls".
        // An object is an invisible wall if it has collision triangles but: it's skipped by the cel pass (flag 0x10),
        // it's a door/searchlight/corona that the static pass skips (drawn elsewhere — but if those paths also skip
        // it, it's invisible), or its gfx leaves are all empty (no visible mesh). The user can then check the named
        // object against PCSX2 to see if the game hides, destroys or never creates it.
        let coll_debug = std::env::var_os("NF_COLL").is_some();
        let mut invisible = 0;
        // The list-0 key (`Material::chain`/`Material::node`): `View_AddCels` (0x1e6ba0, disassembled)
        // gives the −90000 + 10·k keys to the +0x20 chain `Vision_Portal_Recurse` (0x1e86e8) threads
        // per frame from the depth-sorted `VisCelList` — and only the room cels (placement flag words
        // 0xc023/0xc047, `parseentity_fixup_entity` 0x1d00c0) ride that chain [L ee_start_05.bin: 19
        // rooms + 113 inner cels; the port's 05 creates_cel set is exactly 19 + 113]. Every non-room
        // 0x8000 placement (the 0x8019/0x8018 marker classes: the baked decals, walkways, blockers)
        // hangs on a room's +0x18 inner chain and draws with a ±dist² key. So: rooms keep a chain
        // ordinal (their parse rank among rooms — the fallback when the per-frame walk cannot run)
        // plus their `cells` node for the per-frame lookup; every other creates_cel placement is
        // distance-keyed (chain = node = u32::MAX, the same path as the dynamics).
        // fix-portal2: every creates_cel placement now also carries its OWN cell in `node` (the
        // placement's pCelList identity) — `View_AddCels` (0x1e6ba0) draws a cel placement's
        // glist when ITS cel is visible, not when the room its centre lands in is. The per-
        // instance walk below is aligned with `instances()` (the main map's placements in order,
        // resolved ones kept) via the same resolution test over `own_cells_main` (by placement
        // ordinal).
        // fix-portal5: `own_cells` must line up 1:1 with the instances that feed it below — the
        // positional `own_i` advances only for non-room instances that survived the retain above,
        // so the build applies the SAME filters (unresolved dropped; the retained-out classes
        // 0x2a/0x22/0xd4 dropped; the room classes dropped — they take the room branch), plus,
        // when `cels_only`, the trailing 0x20/0xe2 extension `scene_instances` appends. The old
        // build listed every resolved placement, so after the first room every non-room placement
        // read an EARLIER placement's cell (07000006: 379 of 765 non-room instances misaligned;
        // the wall-region generics keyed under hidden ROOM cells 13/20/22/38 while their true
        // cells hung on room 21's chain — the courtyard walls did not draw).
        let own_cells: Vec<u32> = match level.main_map() {
            Some(m) => {
                let mut v: Vec<u32> = Vec::new();
                let resolved = |p: &nf_formats::placement::Placement| if p.by_hash() { level.by_hash.contains_key(&p.hashcode) } else { (p.index as usize) < level.maps[m].objects.len() };
                for (pi, p) in level.maps[m].placements.iter().enumerate() {
                    if !resolved(p) { continue; }
                    if cels_only && !p.creates_cel() { continue; }
                    if matches!(p.flags & 0xff, 0x2a | 0x22 | 0xd4) { continue; }
                    if matches!(p.flags & 0xffff, 0xc023 | 0xc047) { continue; }
                    v.push(cel_graph.own_cells_main.get(pi).copied().flatten().unwrap_or(u32::MAX));
                }
                if cels_only {
                    // `scene_instances(true)`'s extension: the static dynamic entities (0x20/0xe2,
                    // non-cel placements) appended after the cel placements, in placement order.
                    for (pi, p) in level.maps[m].placements.iter().enumerate() {
                        if !resolved(p) || p.creates_cel() || !matches!(p.flags & 0xff, 0x20 | 0xe2) { continue; }
                        if p.flags & 0xff == 0xe2 && p.extra[7] & 0xffff != 0 { continue; }
                        v.push(u32::MAX);
                    }
                }
                v
            }
            None => Vec::new(),
        };
        let mut own_i = 0usize;
        let mut room_rank = 0u32;
        for inst in &instances {
            let obj = &level.maps[inst.map].objects[inst.object];
            // The room rank counts every resolved main-map room-class placement in parse order — the
            // same order `parse_cel_graph` pushes `rooms`/`room_to_cell` — before any skip so the
            // rank stays aligned when a room is cel-pass-skipped or NF_HIDE'd.
            let room_class = matches!(inst.placement.flags & 0xffff, 0xc023 | 0xc047);
            if room_class { room_rank += 1; }
            let (chain, node) = if room_class {
                let node = cel_graph.room_to_cell.get(room_rank as usize - 1).copied().flatten().unwrap_or(usize::MAX) as u32;
                (room_rank - 1, node)
            } else {
                // fix-portal2: the placement's own cell (u32::MAX when it creates no cel — the
                // dynamics stay distance-keyed).
                let own = own_cells.get(own_i).copied().unwrap_or(u32::MAX);
                own_i += 1;
                (u32::MAX, own)
            };
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
            add_instance(level, inst, Layer::World, None, &mut groups, Some(&mut bounds), None, None, None, mult, &room_cels, chain, node);
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
                } else { [1.0; 3] }, &room_cels, u32::MAX, u32::MAX);
                continue;
            }
            add_instance(level, inst, Layer::Character, Some((characters.len() as u32, None)), &mut groups, Some(&mut bounds), None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
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
            let morph_of_main = skin.main.iter().flatten()
                .filter_map(|mh| level.by_hash.get(mh).and_then(|&(m2, o2)| level.maps[m2].objects[o2].morph_targets.clone()))
                .next();
            characters.push(Character { matrix: inst.matrix, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones: obj.gfx.slot_bones.clone(), start, role, floor: None, script: None, morph_targets: obj.morph_targets.clone().or(morph_of_main) });
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
            add_instance(level, &winst, Layer::Character, Some((character, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
            let local = anim::quat_trans_to_mat(dt.q, dt.t);
            attachments.push(Attachment { character, bone: dt.bone as usize, local, flash: None, tracer: None, dropped: false, projectile: None, script: None, gas: None, effect: None });
            // The same gun as a pickup once the drone drops it.
            add_instance(level, &winst, Layer::Character, Some((character, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
            attachments.push(Attachment { character, bone: 0, local, flash: None, tracer: None, dropped: true, projectile: None, script: None, gas: None, effect: None });
            // Its muzzle flash: `DroneWeap_FireWeapon` asks `AnimGetBoneWorldTrans(obj, datum id, 1, …)`, which looks up
            // the datum whose id is id + 1 — the muzzle (07000005: 8 → 9 at x −0.77, 16 → 17 at x −1.18).
            if let Some((fmesh, rgb)) = wd.muzzle_flash(weapon as u8) {
                let muzzle = skin.datums.iter().find(|d| d.id == datum_id + 1 && d.bone >= 0);
                if let (Some(&(fmap, fobject)), Some(md)) = (level.by_hash.get(&fmesh), muzzle) {
                    let finst = Instance { map: fmap, object: fobject, ..winst };
                    add_instance(level, &finst, Layer::Effect, Some((character, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                    attachments.push(Attachment { character, bone: md.bone as usize, local: anim::quat_trans_to_mat(md.q, md.t), flash: Some(rgb), tracer: None, dropped: false, projectile: None, script: None, gas: None, effect: None });
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
                add_instance(level, &inst, Layer::Character, Some((ci, None)), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                for &(bone, part) in &skin.extra {
                    let Some(&(pmap, pobject)) = part.and_then(|h| level.by_hash.get(&h)) else { continue };
                    let pinst = Instance { map: pmap, object: pobject, matrix: ident, placement: placement.clone() };
                    add_instance(level, &pinst, Layer::Character, Some((ci, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                    attachments.push(Attachment { character: ci, bone: bone as usize, local: ident, flash: None, tracer: None, dropped: false, projectile: None, script: None, gas: None, effect: None });
                }
                // The muzzle flash (`Player_MuzzleFlash`): mesh `weapon+0x48` on datum 0, colour (+0x56, +0x55, +0x54).
                let r = &wd.records[wid as usize];
                let flash = u32::from_le_bytes(r[0x48..0x4c].try_into().unwrap());
                if let (Some(&(fmap, fobject)), Some(dt)) = (level.by_hash.get(&flash), skin.datums.iter().find(|d| d.id == 0 && d.bone >= 0)) {
                    let finst = Instance { map: fmap, object: fobject, matrix: ident, placement: placement.clone() };
                    add_instance(level, &finst, Layer::Effect, Some((ci, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                    attachments.push(Attachment { character: ci, bone: dt.bone as usize, local: anim::quat_trans_to_mat(dt.q, dt.t), flash: Some([r[0x56], r[0x55], r[0x54]]), tracer: None, dropped: false, projectile: None, script: None, gas: None, effect: None });
                }
                characters.push(Character { matrix: ident, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones: level.maps[map].objects[object].gfx.slot_bones.clone(), start: None, role: drone::SpawnRole::Placed, floor: None, script: None , morph_targets: None });
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
            // The explosion script (`Explode_Create` 0x17cbd0 Script_Loads 0x6000052 on every disc caller
            // [M 0x12cd98/0x12ce80/0x17ffa0/0x180498/0x1c350-calls]): the sim spawns it per blast
            // (`Drones`' grenade explode), so its entity streams need attachments exactly like a placed
            // player's. 0x6000027/0x600004F (Destroy_Smash's types) stay unported — the port's destroyables
            // don't explode yet.
            let hashes: Vec<u32> = players.iter().flat_map(|p| p.scripts).filter(|&h| h != 0)
                .chain(std::iter::once(0x0600_0052)).collect();
            for h in hashes {
                let Some(nis) = level.nis.get(&h) else { continue };
                for (si, st) in nis.streams.iter().enumerate() {
                    for cmd in st.decode() {
                        match cmd {
                            nf_formats::nis::Cmd::Anim { skin: sh, .. } => {
                                if !seen.insert((h, si, sh)) { continue; }
                                let (Some(skin), Some(skeleton)) = (level.skins.get(&sh), level.skins.get(&sh).and_then(|k| skeletons.get(&k.skeleton))) else { continue };
                                let ci = characters.len() as u32;
                                let mut slot_bones = Vec::new();
                                let mut morph_of_object = None;
                                for &mh in skin.main.iter().flatten() {
                                    let Some(&(map, object)) = level.by_hash.get(&mh) else { continue };
                                    let inst = Instance { map, object, matrix: ident, placement: placement.clone() };
                                    add_instance(level, &inst, Layer::Character, Some((ci, None)), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                                    slot_bones = level.maps[map].objects[object].gfx.slot_bones.clone();
                                    morph_of_object = level.maps[map].objects[object].morph_targets.clone();
                                }
                                for &(bone, part) in &skin.extra {
                                    let Some(&(map, object)) = part.and_then(|ph| level.by_hash.get(&ph)) else { continue };
                                    let pinst = Instance { map, object, matrix: ident, placement: placement.clone() };
                                    add_instance(level, &pinst, Layer::Character, Some((ci, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                                    attachments.push(Attachment { character: ci, bone: bone as usize, local: ident, flash: None, tracer: None, dropped: false, projectile: None, script: None, gas: None, effect: None });
                                }
                                characters.push(Character { matrix: ident, skin: skin.clone(), skeleton: skeleton.clone(), slot_bones, start: None, role: drone::SpawnRole::Placed, floor: None, script: Some((h, si as u8)), morph_targets: morph_of_object.clone() });
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
                                                add_instance(level, &winst, Layer::Character, Some((ci, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                                                let local = anim::quat_trans_to_mat(dt.q, dt.t);
                                                attachments.push(Attachment { character: ci, bone: dt.bone as usize, local, flash: None, tracer: None, dropped: false, projectile: None, script: None, gas: None, effect: None });
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
                                add_instance(level, &inst, Layer::Character, Some((0, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                                attachments.push(Attachment { character: 0, bone: 0, local: ident, flash: None, tracer: None, dropped: false, projectile: None, script: Some((h, si as u8, hash)), gas: None, effect: None });
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
        // Slot TRACER_SLOTS is the gadget beam (fix-laser-beam: the Laser/Stunner's per-shot beam draw).
        if let (Some(_), Some(&(tmap, tobj))) = (drones, level.by_hash.get(&0x0200_0123)) {
            for slot in 0..=TRACER_SLOTS {
                let tinst = Instance { map: tmap, object: tobj, matrix: anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]), placement: level.maps[level.main_map().unwrap_or(0)].placements.first().cloned().unwrap_or_default() };
                add_instance(level, &tinst, Layer::Effect, Some((0, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                attachments.push(Attachment { character: 0, bone: 0, local: tinst.matrix, flash: None, tracer: Some(slot), dropped: false, projectile: None, script: None, gas: None, effect: None });
            }
        }
        // Projectile meshes (`models-weapons-items`): a bullet whose weapon has a `+0x5C` mesh draws THAT mesh
        // instead of the trail — `Bullet_update` state 0 (disasm 0x12e8f0-0x12e900): `weapon+0x5C != 0` →
        // `hashtable_set_object_to_entity_gfx(obj, weapon+0x5C)` + `View_SetDrawInAllViews`; the mesh sits at the
        // bullet's matrix, aligned to the flight direction (`Bullet_init` 0x12c5a0:185
        // `Mat_Align2Dir(obj+0x90, dir, up (0,1,0) [L ee_start_05.bin @0x2d6210], fallback (1,0,0) @0x2d6220)`),
        // while the position lives in `obj+0x30` (the per-tick tail 0x12f1ac-0x12f1e4 moves `obj+0x30..0x38` only).
        // Grenades are gravity bullets: the stun (0x35 → 0x02000726), the smoke (0x36 → 0x0200071f), the mine
        // (0x37 → 0x02000a6e) and the rocket (0x3A → 0x0200074f) all show their body in flight [L ee_start_05.bin].
        // The port pools the slots like the tracers: GRENADE_SLOTS grenades, then TRACER_SLOTS drone bullets,
        // then PLAYER_BULLET_SLOTS of Bond's. One instance per (slot, mesh) for every distinct projectile mesh the
        // level archive resolves — the others hide (FAR) in `frame_state`.
        let proj_slots = GRENADE_SLOTS + TRACER_SLOTS + PLAYER_BULLET_SLOTS;
        let mut proj_meshes: Vec<u32> = Vec::new();
        if drones.is_some() {
            for w in 1..0x73usize {
                let m = placed_weapons.projectile_mesh(w);
                if m != 0 && !proj_meshes.contains(&m) && level.by_hash.contains_key(&m) { proj_meshes.push(m); }
            }
        }
        if let Some(wd) = drones.and_then(|d| d.weapons.as_ref()) {
            for w in 1..0x73usize {
                let m = wd.projectile_mesh(w);
                if m != 0 && !proj_meshes.contains(&m) && level.by_hash.contains_key(&m) { proj_meshes.push(m); }
            }
        }
        let pslot_placement = level.maps[level.main_map().unwrap_or(0)].placements.first().cloned().unwrap_or_default();
        for &mesh in &proj_meshes {
            let Some(&(pmap, pobject)) = level.by_hash.get(&mesh) else { continue };
            for slot in 0..proj_slots {
                let pinst = Instance { map: pmap, object: pobject, matrix: anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]), placement: pslot_placement.clone() };
                add_instance(level, &pinst, Layer::Effect, Some((0, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                attachments.push(Attachment { character: 0, bone: 0, local: pinst.matrix, flash: None, tracer: None, dropped: false, projectile: Some((slot as u8, mesh)), script: None, gas: None, effect: None });
            }
        }
        for (inst, follows_camera) in &sky {
            add_instance(level, inst, if *follows_camera { Layer::SkyCamera } else { Layer::SkyFixed }, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
        }
        // fix-impacts-render: the impact-effect draw pools — the game's own meshes for the bullet decals, the
        // shell casings and the debris. There is no dedicated draw function in ACTION.ELF: the decal nodes ride
        // the host obj/cel's sub-object list (`View_DrawObjectGlist` 0x1e6780 / `View_DrawCelGlist` 0x1e6538 draw
        // `node+0x10`'s glist at `node+0x20`), casings and debris are ordinary class-0x40 objects drawn through
        // `Control_GetObjMatrix` 0x134210's cached `obj+0x90`. All three render in the game's pass 0 — the port
        // instances them as Effect-layer pools (the leaf GS registers carry each mesh's own blend/z state; the
        // Effect layer only skips the static-World cull/sort). One pool per distinct hash the level archive
        // resolves: the decal mesh `EffectInfo+4` (`hashtable_set_sub_object_to_entity_gfx` 0x1ce9c8), the
        // casing mesh `ammo_data[type]+4` (`Bullet_init_casing` 0x12fe2c) and the debris mesh `EffectInfo+0x74`
        // (`Debris_Create` 0x1aab10) — the same hashes the sim stores per effect. Slot counts: the game's
        // 200-node decal FIFO (`DList_Init(0x3496e0, 0x60, 200)` 0x1cf268) and 32 casing objects
        // (`Bullet_InitCasingLists` 0x12fd48); debris are unbounded per-impact objects (≤ 6, ≤ ~54 ticks) —
        // the port caps the draw slots.
        if let Some(dc) = drones {
            if let Some(surf) = &dc.surfaces {
                let mut decal_meshes: Vec<u32> = surf.effects.iter().map(|r| r.decal).filter(|m| *m != 0).collect();
                let mut debris_meshes: Vec<u32> = surf.effects.iter().map(|r| r.debris_mesh).filter(|m| *m != 0).collect();
                let mut casing_meshes: Vec<u32> = surf.ammo.casing_mesh.iter().copied().filter(|m| *m != 0).collect();
                for v in [&mut decal_meshes, &mut debris_meshes, &mut casing_meshes] { v.sort(); v.dedup(); }
                let slot_placement = level.maps[level.main_map().unwrap_or(0)].placements.first().cloned().unwrap_or_default();
                let ident = anim::quat_trans_to_mat([0.0, 0.0, 0.0, 1.0], [0.0; 3]);
                for (meshes, slots, kind) in [
                    (&decal_meshes, DECAL_SLOTS, 0u8),
                    (&casing_meshes, nf_game::casing::POOL, 1u8),
                    (&debris_meshes, DEBRIS_SLOTS, 2u8),
                ] {
                    for &mesh in meshes {
                        let Some(&(m, o)) = level.by_hash.get(&mesh) else { continue };
                        for slot in 0..slots {
                            let inst = Instance { map: m, object: o, matrix: ident, placement: slot_placement.clone() };
                            add_instance(level, &inst, Layer::Effect, Some((0, Some(attachments.len() as u32))), &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                            attachments.push(Attachment { character: 0, bone: 0, local: inst.matrix, flash: None, tracer: None, dropped: false, projectile: None, script: None, gas: None, effect: Some((kind, slot, mesh)) });
                        }
                    }
                }
            }
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
                    add_instance(level, &inst, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
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
                    add_instance(level, &inst, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
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
                    add_instance(level, &inst, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                    ladders_drawn += 1;
                }
            }
        }
        // Models-misc: the static dynamic entities (docs/models/misc.md has the full per-kind evidence).
        // `View_DrawObjList` 0x1e69a0 → `View_DrawObjectGlist` 0x1e6780 draws `obj+0xD8` at `obj+0x90`;
        // `control_create_object` 0x1344e0:20-29 builds `obj+0x90` from the placement pos/rot, and none of
        // these creators' updates move the matrix (the lock's unlock spline, the rotor's obj+0x50 spin, the
        // rigid bodies' RB_EulerEvolve and the gun imp's turret child all start from here), so a static
        // World-layer instance at the placement transform is the game's draw at rest exactly. Per kind:
        //   0x2E Lock_Create 0x1ae1f8:55 (direct obj+0xD8 store) · 0x30 Monitor_Create 0x1b08b0:26
        //   (Control_SetGList) · 0x34 GunImp_Create 0x131f20:68-69 (parent; the fixed-hashcode turret
        //   child needs the ELF TurretGuns table, still open) · 0x35 PCQWorm_Create 0x1b07a8:12-13 ·
        //   0x3B DynamicObject_Create 0x18dbf8:19-31 (hidden until its
        //   switch channel when param 0 ≠ 0 — those stay skipped)
        //   0x71 rotor_init 0x18f4b8:24 · 0xD5 OneSided_Create 0x1e7ce8:14 (its per-frame backface self-cull
        //   is the port's triangle culling) · 0xD8 RB_Create 0x1d6bf0:52 (rigid bodies evolve obj+0x90 once
        //   disturbed; at rest = placement) · 0xE1 Destroy_Create 0x12c318:23 (death keeps the object drawn:
        //   Destroy_Smash 0x12bb60 sets class 1 and only swaps to the block-0x27 LOD variant when one exists)
        //   · 0xE3 Flicker_Create 0x1acb00:49 · 0xFA ThirdIcon_Create 0x197070:33 / 0xFD CamSubject_Create
        //   0x1bc5d8:17 / 0x4A Apocalypse_Create 0x18dcc0:20 — these attach whatever glist the placement
        //   carries; `psiDrawObjectMatrix` no-ops on an empty one, so only objects with gfx batches draw.
        //   0xDA Car_Create 0x1305b0:20-21 body, plus Car_InitBits 0x12fec8's two fixed-hashcode children:
        //   0x2000502 at the body's exact matrix (Mat_Copy 0x12fec8:33) and 0x2000505 at body translation
        //   + identity rotation [M 0012fec8:13-33]. The +1.5 lift and the 0x2000194 re-skin are the MPSettings
        //   variant (data+0x112), not the SP default.
        // Kinds that must NOT draw here (docs/models/misc.md): 0x29 switch and 0x31 fusebox (obj+0xD8 = 0 —
        // no mesh is ever attached [M 001adcc0:35, 001b0448:25]); 0x27 GT (obj+0xF0 |= 0x10 at create: hidden
        // until deployed [M 0017ffa0:78]); 0x32 rainbox (the mesh is never drawn — RainBox_Clip 0x18d0d0 only
        // reads the AABB [M 0018d0d0:20-35]); 0x38/0x39 space laser/missile (invisible controllers, visuals
        // via scripts); 0xF2 emitter (no glist ever [M 0017ae00:100-101]); 0xD4 cloud (sky pipeline, drawn by
        // sky_instances); 0xF3 tree (anim-object path, like characters).
        // fix-gadgets2, the user's "red and yellow boxes ... for the wall shimmy and one for the grapple":
        // 0x3A grapple points and 0x3F creep walls are the same shape of invisible: both creators build a
        // bare control object and attach NO gfx hash — `Grapple_Create` 0x198298 has zero
        // `hashtable_set_object_to_entity_gfx` calls (instance size 0, no placement params read; only +0xD8,
        // +0xF0, class 0x51, LinkToRoom, StraddleTest) and `CreepWall_Create` 0x196fc8 likewise sets class 0x2B
        // with no gfx attach — so `View_DrawObjList`'s empty-glist no-op never draws them. The marker meshes
        // the sweep used to draw are the placements' referenced objects, which the game discards.
        if cels_only {
        if let Some(m) = level.main_map() {
            const MISC_KINDS: [u8; 15] = [0x2e, 0x30, 0x34, 0x35, 0x3b, 0x71, 0xd5, 0xd8, 0xe1, 0xe3, 0xda, 0xfa, 0xfd, 0xfe, 0x4a];
            let nf_hide = std::env::var("NF_HIDE").ok();
            for p in level.maps[m].placements.iter().filter(|p| MISC_KINDS.contains(&((p.flags & 0xff) as u8))) {
                let kind = (p.flags & 0xff) as u8;
                // 0x3B hidden-until-channel (DynamicObject_Create 0x18dbf8:31, param 0 = the s16 channel).
                if kind == 0x3b && p.params.iter().find(|(i, _)| *i == 0).is_some_and(|(_, v)| *v != 0) { continue; }
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                let Some((map, object)) = target else { continue };
                let obj = &level.maps[map].objects[object];
                // Empty glists draw nothing (`View_DrawObjectGlist`'s psiDrawObjectMatrix no-ops), and a
                // mirror-only object (glist flags 0x400, View_AddObjects 0x1e62d8:38-47) has no mirror viewer here.
                if !obj.gfx.leaves.iter().any(|l| !l.batches.is_empty()) || obj.flags & 0x400 != 0 { continue; }
                if let Some(h) = &nf_hide { if h.split(',').any(|n| !n.is_empty() && obj.name.contains(n)) { continue; } }
                // The 0x800 dim, the same rule the main static loop applies (View_SetupRenderModes_Tweak 0x1e5888).
                let mult = if obj.flags & 0x800 != 0 {
                    let mx = nf_formats::placement::object_matrix(p.rotation, p.translation);
                    mul_bytes(cel_ambient(&room_cels, transform(&mx, obj.centre), 0).unwrap_or([0xff; 3]))
                } else { [1.0; 3] };
                let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                add_instance(level, &inst, Layer::World, None, &mut groups, Some(&mut bounds), None, None, None, mult, &room_cels, u32::MAX, u32::MAX);
                *misc_drawn.entry(kind).or_default() += 1;
                // The car's two fixed-hashcode children (Car_InitBits 0x12fec8), drawn only with a car body.
                if kind == 0xda {
                    let has_gfx = |(cm, co): (usize, usize)| level.maps[cm].objects[co].gfx.leaves.iter().any(|l| !l.batches.is_empty());
                    // Child 1 (data+0xE0, 0x2000502): a full Mat_Copy of the body's matrix [M 0x12fec8:33].
                    if let Some(c) = level.by_hash.get(&0x0200_0502).copied().filter(|&c| has_gfx(c)) {
                        add_instance(level, &Instance { map: c.0, object: c.1, matrix: inst.matrix, placement: p.clone() }, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                        *misc_drawn.entry(0xda).or_default() += 1;
                    }
                    // Child 2 (data+0xE4, 0x2000505): identity rotation + the body's translation [M 0x12fec8:13-33].
                    if let Some(c) = level.by_hash.get(&0x0200_0505).copied().filter(|&c| has_gfx(c)) {
                        let mut cm = inst.matrix;
                        cm[0] = [1.0, 0.0, 0.0, 0.0]; cm[1] = [0.0, 1.0, 0.0, 0.0]; cm[2] = [0.0, 0.0, 1.0, 0.0];
                        add_instance(level, &Instance { map: c.0, object: c.1, matrix: cm, placement: p.clone() }, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
                        *misc_drawn.entry(0xda).or_default() += 1;
                    }
                }
            }
        }
        } // cels_only: in --all the generic instance loop already draws these kinds (debug view only).
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
                    add_instance(level, &inst, Layer::Character, None, &mut groups, None, Some(pi as u16), None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
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
                add_instance(level, &inst, Layer::Character, None, &mut groups, None, None, Some(di as u16), None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
            }
            // SimpleScripts (`SS_Create` 0x1a9c58 / `SS_Update` 0x1a9948, map-tower2): the animated 0xE2
            // placements (key count non-zero) ride the door slot space — their vertices pose from
            // `door_base + n_doors + k`, the SS list's matrices, every tick. At rest each sits at key 0
            // (= its placement transform [D]); the switch channel swaps it to the other key (the
            // 07000012 lift screens, shutters and lift doors).
            let n_doors = nf_game::door::door_placements(&level.maps[m].placements).len();
            // The same construction the sim's `Drones::attach_level` runs, so the slot order matches
            // the matrix upload's `sss.list` order (deterministic: placement order, same filter).
            let ss_list = nf_game::ss::SsList::create(&level.maps[m].placements, &level.path_blocks(), |_p| 0);
            // (hash 0 here: the render pass does not need the collision hash — it reads the placement
            // object directly below; the sim's attach_level resolves the real one for set_dynamic.)
            for (si, ss) in ss_list.list.iter().enumerate() {
                let p = &level.maps[m].placements[ss.placement];
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
                let Some((map, object)) = target else { continue };
                let inst = Instance { map, object, matrix: nf_formats::placement::object_matrix(p.rotation, p.translation), placement: p.clone() };
                add_instance(level, &inst, Layer::Character, None, &mut groups, None, None, Some((n_doors + si) as u16), None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
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
                add_instance(level, &inst, Layer::Character, None, &mut groups, None, None, None, Some(pi as u16), [1.0; 3], &room_cels, u32::MAX, u32::MAX);
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
                add_instance(level, &inst, Layer::World, None, &mut groups, None, None, None, None, [1.0; 3], &room_cels, u32::MAX, u32::MAX);
            }
        }
        let (mut vertices, mut indices, mut materials, mut char_verts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut char_vertex_start = None;
        for ((layer, _, _, chain, key, draw, lflags), (vs, is, cv, centre, mut radius, cel, node)) in groups {
            if is.is_empty() { continue; }
            // fix-render-artifacts: the cull sphere must contain the group's DRAWN geometry — the glist
            // radius can undershoot it (0700000c's floor decal: glist r 0.6 vs a 0.87 half-diagonal box),
            // which dropped in-view statics at the frustum edge (the user's pale fog-coloured patches).
            // The game's cull consults the bbox next to the sphere (`View_AddObjects` 0x1e62d8 passes
            // obj+0x8c AND obj+0x80 to `Vision_InView` 0x1e89e0); this is that bbox term.
            {
                let mut d2max = radius * radius;
                for v in &vs {
                    let d2 = (v.pos[0] - centre[0]).powi(2) + (v.pos[1] - centre[1]).powi(2) + (v.pos[2] - centre[2]).powi(2);
                    if d2.is_finite() && d2 > d2max { d2max = d2; }
                }
                radius = d2max.sqrt();
            }
            if matches!(layer, Layer::Character | Layer::Effect) && char_vertex_start.is_none() { char_vertex_start = Some(vertices.len()); }
            let base = vertices.len() as u32;
            materials.push(Material { layer, draw, centre, radius, cel, chain, node, key, lflags, first_index: indices.len() as u32, index_count: is.len() as u32 });
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
        let hud_textures = crate::hud::TEXTURES.iter().chain(crate::menu::MENU_TEXTURES.iter()).filter_map(|&h| level.maps.iter().enumerate().find_map(|(mi, m)| m.textures.iter().find(|t| t.hashcode == h).map(|t| {
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
            // fix-safe08b: the cel scope is built (`CollisionWorld::build_scoped`, the straddle walk
            // of `Collide_StraddleCels` 0x1eac98 via `Intersect_Portal` 0x1ea7b8). The TAKE query uses
            // it (`segment_take_scoped`: flags 0x721's cel scoping + the 0xd/0xe surface skip +
            // `Collide_Pick`'s 0x1000 skip). The ACTOR CAPSULE and the floor ray stay FLAT for now:
            // with the scope, Bond falls through the GuardRoom floor at the fusebox — the shared
            // `Room_Stairwell_T_2` mesh maps to cell 59 by the port's smallest-volume rule, which the
            // capsule's straddle walk from cell 40 never keeps; the game's `build_LinkToRoom` 0x1c9680
            // links by `build_FindCel` 0x1c8cb0's RAYCAST tie-break among all containing cels, which
            // the port has not ported (the next session's item, with the live check).
            let world = nf_game::world::CollisionWorld::build_scoped(level, &level.instances(true).0, Some(scope_graph(&cel_graph)));
            // fix-portal4: the +0x18 chains hang by the game's build_FindCel (0x1c8cb0) owner pick
            // over each generic's sphere centre — needs the scoped world for the tie-break rays.
            build_chains(&mut cel_graph, Some(&world));
            let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap_or(&Default::default()), &world, d.level_id);
            let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
            let hitboxes = d.hitboxes.clone()?;
            let volumes = nf_game::aivolume::build(level, d.level_id);
            let models = nf_game::pickup::models(level);
            let data = nf_game::drone::Data { tables: &d.tables, props: &props, scripts: &d.scripts, weapons: &weapons, world: &world, level: d.level_id, sequences: &seqs, skins: &level.skins, skeletons, nav: &nav, covers: &covers, tuning: &d.tuning, hitboxes: &hitboxes, volumes: &volumes, sfx: d.sfx.as_ref(), models: &models, surfaces: d.surfaces.as_ref(), voices: d.voices.as_deref(), difficulty: 1 };
            let mut rng = nf_game::drone::Rng::new(0x1234_5678);
            // The drones' creates (and the interleaved type-0x4E light creates) draw from the shared
            // generator here; the map parse's ambient radiators went in first, inside `level_drones`.
            let mut lights = nf_game::light::LightList::default();
            let list = nf_game::drone::level_drones(&data, level, &mut rng, &mut lights);
            // Every drone placement; spawner templates stay absent until a spawner brings them in.
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
            drones.lights = lights;
            drones.attach_level(level);
            // Ladders (`Ladder_Create` 0x1966c0, placement type 0x22, class 0x2D): fix-ladder. The world
            // sphere comes from the placement's object glist bounds, like the game's
            // `Control_BuildWorldSph` 0x133070.
            let ladders = nf_game::ladder::all(level);
            // Wires (`Wire_Create` 0x1981d8, placement type 0x3E): the level's zip wires, resolved
            // against their map objects like the ladders (fix-wire: the world sphere replaces the
            // hardcoded 4.0 length; the endpoints derive at grab time per the atlas).
            let wires = nf_game::wire::all(level);
            // The creepwalls (`CreepWall_Create` 0x196fc8, placement type 0x3F): the crawlable vents.
            let creepwalls = nf_game::creepwall::all(level);
            // The wires' owning cels (see `wire_cels`): the game's pick pass tests a dynamic object
            // only in the cels its query straddles [M 0x1eb320], and the object lives in the cel
            // whose creation block carried it [M 0x1d04f0].
            let wire_cels = wires.iter().map(|w| find_cel(&sim_cels, w.centre)).collect::<Vec<_>>();
            // The third icons (placement kind 0xFA, `ThirdIcon_Create` 0x197070): the interaction
            // contexts written to `BLData+0x95F` each tick (fix-wire2 — the cable's type-3 icons gate
            // the jump-grab). fix-08-walls: each icon carries its object-resolved world sphere
            // (`Icon::centre`/`radius`, `Control_BuildWorldSph` 0x133070 via `build_LinkToRoom`
            // 0x1c9680:40) and its straddle-cel set — the arm region the game's obj+0xd0 chain
            // implements [M Collide_StraddleCels 0x1eac98 + Intersect_Portal 0x1ea7b8 v0x101 +
            // Collide_Update 0x1ea398:160-233], resolved once here over the final cel graph (the
            // icon's own cel via the faithful `build_FindCel` 0x1c8cb0 port, `find_cell_raycast`).
            let icons = nf_game::wire::icons(level);
            let icon_cels = icons.iter().map(|ic| match find_cell_raycast(&cel_graph, ic.centre, Some(&world)) {
                Some(c) => icon_straddle_cels(&cel_graph, c, ic.centre, ic.radius),
                None => Vec::new(),
            }).collect::<Vec<_>>();
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
                    let mut p2 = nf_game::pickup::Pickup::create_placed(p.translation, p.rotation, type_, id, rounds, switch_channel, sound, respawn as u16, lifetime, model, bounds, &weapons, &pickup_models)?;
                    p2.obj = Some(obj);
                    Some(p2)
                })
                .collect();
            let (reserved, sim_slots) = script_pickup_reservations(level, &weapons, &pickup_models);
            drones.pickups.extend(placed_pickups);
            drones.pickups.extend(reserved);
            drones.script_pickup_slots = sim_slots.into_iter().collect();
            // The direct-level load replays the user's live sessions: they ran at difficulty 1 (the
            // old pre-menu `DIFFICULTY` constant, and the guards above were created with it —
            // captain_setup reads it [M 0x14b300:504-535]). The game's boot default is 2 [M
            // 0x1c7e78:25] and the front-end boot keeps it until 180-fix-menus wires the page's
            // write (C_SBNFDFCTY 0x216ca8) into `set_difficulty`.
            drones.difficulty = 1;
            Some(Sim { drones, tables: d.tables.clone(), props, scripts: d.scripts.clone(), weapons, world, level: d.level_id, char_drone, char_script, char_script_skin, seq_index, seqs, skins: level.skins.clone(), skeletons: skeletons.clone(), nav, covers, tuning: d.tuning.clone(), hitboxes, sfx: d.sfx.clone(), volumes, models, gun: nf_game::gun::Gun::new(2, 7, 41), bond, cursor: [0.0; 2], cross: [0.0; 2], view_script: 0, first_person: false, last_shot: None, shot_tick: false, view_time: 0.0, view_speed: 1.0, bob: [0.0; 2], aiming: false, auto_aim: false, zoom: 1.0, zoom_target: 1.0, disabled: false, surfaces: d.surfaces.clone(), voices: d.voices.clone(), step_dist: 0.0, wires, creepwalls, creeping: false, wire_anims, wire_channel: 0, ladders, icons, view_model_ids: std::collections::HashSet::new(), cross_state: 2, cels: sim_cels, char_ambient, view_model_chars: Vec::new(), ambient_lookups: std::cell::Cell::new(0), lang: None, hud: Default::default(), smoke: Vec::new(), room_cell: cel_graph.room_to_cell.clone(), prev_trigger: false,
                wire_cels, cel_graph: cel_graph.clone(), icon_cels, laser_beam: None, stunner_target: None,
            })
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
        // fix-portal4: sim-less loads never built a world — the chains fall back to the
        // bbox-face FindCel approximation (documented deviation; every live load path builds sim).
        if sim.is_none() { build_chains(&mut cel_graph, None); }
        Scene { cels: room_cels, cel_graph, streams: None, music_files: None, music_tables: None, sound_bank: None, sound_spares: Vec::new(), snow_texture, vertices, indices, materials, textures, hud_textures, hud, view_models, fog, bounds, start, stats, characters, char_vertex_start, char_verts, sequences, attachments, sim, colliders, models_0x800, npc_pool, ladders_drawn, misc_drawn }
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
                // Every started opcode-0 window plays together (`AnimProcessScriptCmds` 0x1c1b38): the
                // 22-channel face track keeps the bones its channels reach, the 222-channel body track
                // fills the rest — the cut-scene lip-sync (fix-cutscenes2; the old single-window read
                // played the body only, so the mouths never moved).
                let local = nf_game::anim::script_pose_together(&s.scripts, &s.seqs, &c.skeleton, &c.skin, script, frame)?;
                let world = anim::world_pose(&local, &c.skin);
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
            // The gas puffs: a camera billboard with the puff's scale and spin (the game's draw path is
            // `View_DrawObjectGlist`'s obj+0xF0 & 8 billboard: viewer rotation · the object's angles, scaled
            // by obj+0xE8 [M 0x1e6780:43-65]); free slots collapse like the tracers.
            if let Some(gi) = a.gas {
                let far = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
                let Some(s) = self.sim.as_ref() else { return far };
                let Some(g) = s.drones.gas.puffs.get(gi as usize) else { return far };
                let (c, sn) = g.spin.sin_cos();
                let (rx, ry) = (opts.right, opts.up);
                let (r2, u2) = ([rx[0] * c - ry[0] * sn, rx[1] * c - ry[1] * sn, rx[2] * c - ry[2] * sn],
                                [rx[0] * sn + ry[0] * c, rx[1] * sn + ry[1] * c, rx[2] * sn + ry[2] * c]);
                let sc = g.scale;
                return [
                    [r2[0] * sc, r2[1] * sc, r2[2] * sc, 0.0],
                    [u2[0] * sc, u2[1] * sc, u2[2] * sc, 0.0],
                    [0.0, 0.0, sc, 0.0],
                    [g.pos[0], g.pos[1], g.pos[2], 1.0],
                ];
            }
            if let Some(slot) = a.tracer {
                // fix-laser-beam: slot TRACER_SLOTS is the gadget BEAM (Draw_LaserBeam / Draw_TaserBeam
                // via Player_MuzzleFlash 0x1a4298:107-137 — one stretched sprite from the muzzle to the
                // crosshair-ray hit). The mesh is the port's trail stand-in [S: the game's 0x2000131/
                // 0x20006c1 sprite objects live in the global gfx table, not the level archives; the
                // main agent's render check owns the look].
                if slot as usize == TRACER_SLOTS {
                    let far = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
                    let Some(s) = self.sim.as_ref() else { return far };
                    let Some((a, b)) = s.laser_beam else { return far };
                    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    if len < 1e-4 { return far; }
                    let dir = [d[0] / len, d[1] / len, d[2] / len];
                    let yaw = dir[0].atan2(dir[2]);
                    let pitch = -dir[1].atan2((dir[0] * dir[0] + dir[2] * dir[2]).sqrt());
                    let mut m = nf_formats::placement::object_matrix([pitch, yaw, 0.0], a);
                    for k in 0..3 { m[2][k] *= len; }
                    return m;
                }
                let b = self.sim.as_ref().and_then(|s| s.drones.bullets.get(slot)).filter(|b| b.state == 1);
                return match b { Some(b) => tracer_matrix(b), None => [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]] };
            }
            if let Some((slot, mesh)) = a.projectile {
                // A projectile-mesh slot: visible only while the slot's live projectile flies AND carries THIS
                // attachment's mesh (the game's `hashtable_set_object_to_entity_gfx(obj, weapon+0x5C)`). The drawn
                // matrix = `Mat_Align2Dir(dir, (0,1,0), (1,0,0))` at the projectile's position — `Bullet_init`
                // 0x12c5a0:185 aligns the bullet's matrix to its flight direction (the up/fallback vectors [L]
                // ee_start_05.bin @0x2d6210/0x2d6220); the per-tick tail moves `obj+0x30..0x38` only.
                let far = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
                let Some(s) = self.sim.as_ref() else { return far };
                let norm = |v: [f32; 3]| { let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt(); if l > 1e-9 { [v[0] / l, v[1] / l, v[2] / l] } else { [0.0, 1.0, 0.0] } };
                let (pos, dir, live_mesh): ([f32; 3], [f32; 3], u32) = if (slot as usize) < GRENADE_SLOTS {
                    match s.drones.grenades.get(slot as usize) {
                        Some(g) if g.state < 2 => (g.pos, norm(g.velocity), g.mesh),
                        _ => return far,
                    }
                } else if (slot as usize) < GRENADE_SLOTS + TRACER_SLOTS {
                    match s.drones.bullets.get(slot as usize - GRENADE_SLOTS) {
                        Some(b) if b.state == 1 => (b.pos, b.dir, s.weapons.projectile_mesh(b.weapon as usize)),
                        _ => return far,
                    }
                } else {
                    match s.drones.player_bullets.get(slot as usize - GRENADE_SLOTS - TRACER_SLOTS) {
                        Some(b) if !b.dead => (b.pos, b.dir, s.weapons.projectile_mesh(b.weapon as usize)),
                        _ => return far,
                    }
                };
                if live_mesh != mesh { return far; }
                let mut m = nf_game::searchlight::align2dir(dir, [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
                m[3] = [pos[0], pos[1], pos[2], 1.0];
                return m;
            }
            // fix-impacts-render: the impact-effect slots — decals, casings, debris at the game's own draw
            // transforms (docs/atlas/effect.md). A slot draws only while its live effect carries the slot's
            // mesh (the game sets the node/object's glist from the hash at create); free slots collapse far
            // below the world like the tracers.
            if let Some((kind, slot, mesh)) = a.effect {
                let far = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
                let Some(s) = self.sim.as_ref() else { return far };
                return match kind {
                    // The decal node: `Mat_Align2Dir(node+0x20, normal, (0,1,0) @0x2d6210, (1,0,0) @0x2d6220)`
                    // 0x1df2a8 with the translation `hit + normal·0.01` stored at node+0x50 [M 0x1ce9f0]; drawn
                    // by `View_DrawObjectGlist`/`View_DrawCelGlist`'s `psiDrawObjectMatrix(node+0x10, node+0x20)`
                    // (0x1e6780/0x1e6538). No scale — the quad size lives in the mesh [M 0x1e6780:58-65].
                    0 => match s.drones.effects.decals.get(slot) {
                        Some(d) if d.mesh == mesh => {
                            let mut m = nf_game::searchlight::align2dir(d.normal, [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
                            m[3] = [d.pos[0], d.pos[1], d.pos[2], 1.0];
                            m
                        }
                        _ => far,
                    },
                    // The casing object: `Control_GetObjMatrix` 0x134210's euler(+0x50)·pos(+0x30)
                    // (`View_RotTransMatrix`), scaled by obj+0xe8 (= 1 here — no scale write at eject);
                    // only the flying state draws (`Bullet_init_casing` 0x12fde0 parks the slot
                    // `View_SetDrawInNoViews` until `Bullet_init_casing_ex` 0x12f678 re-shows it).
                    1 => match s.drones.casings.list.get(slot) {
                        Some(c) if c.state == 0 && c.mesh == mesh => casing_matrix(c),
                        _ => far,
                    },
                    // The debris object: euler spin·age at pos, scaled by obj+0xe8 (0.4 + FRand(0.1), applied
                    // once by `Control_GetObjMatrix` 0x134210:25-33 — `View_DrawObjectGlist` copies the cached
                    // matrix unscaled for non-billboards [M 0x1e6780:47-49]). The tick count is 255 − alpha:
                    // `Debris_Create` stores obj+0x106 = 0xff and `Debris_Update` 0x1aac00 steps it down 1/tick.
                    2 => match s.drones.effects.debris.get(slot) {
                        Some(d) if d.mesh == mesh => {
                            let age = 255.0 - d.alpha as f32;
                            let eul = [d.spin[0] * age, d.spin[1] * age, d.spin[2] * age];
                            let mut m = nf_formats::placement::object_matrix(eul, d.pos);
                            for r in 0..3 { for k in 0..3 { m[r][k] *= d.scale; } }
                            m
                        }
                        _ => far,
                    },
                    _ => far,
                };
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

    /// fix-facial2: the morph weights of a morph-bearing script character right now — the face-track
    /// window (the started opcode-0 window whose sequence carries the skin's facial channel count,
    /// the same window `script_pose_together` poses bones from) sampled at the object's frame via
    /// `AnimFacialBlend`'s source arithmetic [M `facial_weights`, `AnimFacialBlend` 0x1be648].
    pub fn facial_weights_now(&self, c: &Character) -> Option<Vec<f32>> {
        let src = c.script?;
        let s = self.sim.as_ref()?;
        let count = c.skin.facial as usize;
        if count == 0 { return None; }
        let (script_h, frame) = s.script_anim(src, c.skin.hashcode)?;
        let script = s.scripts.get(&script_h)?;
        let win = script.sequences().filter(|(a, _, _)| (*a as f32) <= frame)
            .filter_map(|(a, _, q)| s.seqs.get(&q).filter(|sq| sq.channels.len() == count).map(|sq| (a, q)))
            .last()?;
        let seq = s.seqs.get(&win.1)?;
        nf_game::anim::facial_weights(seq, (frame - win.0 as f32 + 1.0).max(1.0), count)
    }

    pub fn animate(&self, choice: Option<usize>, ticks: f32, opts: ViewOpts) -> Vec<[f32; 3]> {
        let FrameState { posed, attach, hidden } = self.frame_state(choice, ticks, opts);
        let hidden = |_: &Character, ci: usize| hidden[ci];
        // fix-facial2: the morph weights per morph-bearing script character (the ledge dialogue's
        // face track), computed once per frame — `SkinIt` 0x10bf30 consumes them per vertex.
        let facial: Vec<Option<Vec<f32>>> = self.characters.iter().map(|c| {
            if c.morph_targets.is_some() { self.facial_weights_now(c) } else { None }
        }).collect();
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
            // fix-facial2: the morph term (object space, before the bone skinning — the game's
            // `SkinIt` 0x10bf30 adds Σ weight[m] × target(base + (morph + m) × 12) to the source
            // position while it streams through the scratchpad; the vertex's morph index is the
            // skinning word's upper half-word, −1 = none).
            let pos = match (facial.get(v.character as usize).and_then(|w| w.as_deref()), &c.morph_targets, v.morph) {
                (Some(w), Some(t), mo) if mo >= 0 => morph_vertex(v.pos, mo, w, t),
                _ => v.pos,
            };
            match &posed[v.character as usize] {
                Some((mats, _, m)) => transform(m, anim::skin_vertex(pos, v.weight, v.slots, &c.slot_bones, mats)),
                None => transform(&c.matrix, pos),
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

    /// fix-impacts-render: the attachment index that draws impact-effect slot `(kind, slot, mesh)` —
    /// kind 0 decal, 1 casing, 2 debris (`Scene::build`'s per-mesh pools; `frame_state` poses it from
    /// the sim each frame). Tests and the dev panel read the pools through this.
    pub fn effect_attachment(&self, kind: u8, slot: usize, mesh: u32) -> Option<u32> {
        self.attachments.iter().position(|a| a.effect == Some((kind, slot, mesh))).map(|i| i as u32)
    }

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
        // fix-searchlight-remainder: one colour slot per searchlight part, so the corona's alpha byte
        // (obj+0x106, `(dot−0.2)·4·255` low byte [M 0x1cb900:334-336]) reaches the shader — the game's
        // draw reads obj+0x106 as the quad's alpha. All other parts keep alpha 1.0; the slots sit
        // before the shared 1.0 entry (`col_one`).
        let col_sl = col_pick + n_pick;
        let col_one = col_sl + n_sl;
        let attrs = self.char_verts.iter().map(|v| {
            let one = |m: u32, col: u32| SkinAttr { a: m, b: m, w: 1.0, col };
            if let Some(sl) = v.searchlight { return one(sl_base + sl as u32, col_sl + sl as u32); }
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
        (SkinLayout { char_base, attach_base, sl_base, door_base, pick_base, n_mats, col_att, col_pick, col_sl, col_one }, attrs)
    }

    /// The frame's resolved lights for the renderer: the sim's `LightList` with the kind-2
    /// positions resolved against their searchlight parts. Per draw the renderer picks the ≤2
    /// closest within the draw's bounding sphere — the game's
    /// `Lights_CalcClosestLights` 0x1cb2e0 / `psiLight_SetLights` 0x10e588 per object/cel.
    pub fn frame_lights(&self) -> nf_game::light::ResolvedLights {
        let Some(s) = self.sim.as_ref() else { return Default::default() };
        s.drones.lights.resolve(&|i: u32| s.drones.searchlights.parts.get(i as usize).map(|p| p.pos).unwrap_or([0.0; 3]))
    }

    /// One frame's palette and colour table for `skin_layout` (the GPU replacement for `posed_vertices`).
    /// `vis`: the frame's room-level visibility mask (the portal walk's set, `Renderer::cel_visibility`)
    /// — the object half of the game's cull [M `View_AddObjects` 0x1e62d8 / `View_DrawObjList`
    /// 0x1e69a0: only the VISIBLE cels' object lists draw; each object's cel via `build_LinkToRoom`
    /// 0x1c9680 → `build_FindCel`]. A posed object whose room cel is not visible collapses its
    /// palette to FAR (off-picture — the port's established "not drawn" state, the same mechanism as
    /// `fs.hidden`); objects the cel pass cannot place draw (the game's unlinked objects draw from
    /// the forced list, `View_AddForcedObjects` 0x1e5fb8). `None` = the walk is off (`NF_NOPORTAL`).
    pub fn skin_palette(&self, l: &SkinLayout, choice: Option<usize>, time: f32, opts: ViewOpts, vis: Option<&[bool]>) -> (Vec<Mat4>, Vec<[f32; 4]>) {
        const FAR: Mat4 = [[0.0; 4], [0.0; 4], [0.0; 4], [0.0, -10000.0, 0.0, 1.0]];
        let fs = self.frame_state(choice, time, opts);
        let collapse = |at: &Mat4| -> Mat4 { [[0.0; 4], [0.0; 4], [0.0; 4], [at[3][0], at[3][1], at[3][2], 1.0]] };
        let vis_room = |pos: [f32; 3]| -> bool {
            vis.map_or(true, |v| self.cel_index_at(pos).map_or(true, |r| v.get(r).copied().unwrap_or(true)))
        };
        let mut p = vec![FAR; l.n_mats as usize];
        for (ci, c) in self.characters.iter().enumerate() {
            let base = l.char_base[ci] as usize;
            let nb = c.skeleton.bones as usize;
            if fs.hidden[ci] { for k in 0..=nb { p[base + k] = collapse(&c.matrix); } continue; }
            let cpos = fs.posed[ci].as_ref().map_or(c.matrix[3], |(_, _, m)| m[3]);
            if !vis_room([cpos[0], cpos[1], cpos[2]]) { for k in 0..=nb { p[base + k] = FAR; } continue; }
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
                let keep = match s.drones.searchlights.parts.get(k) {
                    Some(sp) => vis_room(sp.pos),
                    None => true,
                };
                if !keep { p[l.sl_base as usize + k] = FAR; continue; }
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
                let dpos = s.drones.doors.list.get(k).map(|d| d.matrix[3])
                    .or_else(|| s.drones.sss.list.get(k - s.drones.doors.list.len()).map(|q| q.matrix[3]));
                if dpos.is_some_and(|p3| !vis_room([p3[0], p3[1], p3[2]])) { p[l.door_base as usize + k] = FAR; continue; }
                p[l.door_base as usize + k] = match s.drones.doors.list.get(k) {
                    Some(d) => d.matrix,
                    None => s.drones.sss.list.get(k - s.drones.doors.list.len()).map_or(FAR, |q| q.matrix),
                };
            }
            for k in 0..(l.n_mats - l.pick_base) as usize {
                let pk = s.drones.pickups.get(k).filter(|q| !q.gone && !q.hidden);
                if pk.is_some_and(|q| !vis_room([q.m[3][0], q.m[3][1], q.m[3][2]])) { continue; } // stays FAR
                p[l.pick_base as usize + k] = pk.map_or(FAR, |q| q.m);
            }
        }
        let (mults, pickup_mult, attach_mult) = self.colour_mults();
        let mut cols = vec![[1.0f32; 4]; l.col_one as usize + 1];
        let has = !mults.is_empty();
        let rgb = |m: [f32; 3]| [m[0], m[1], m[2], 1.0];
        // fix-searchlight-remainder: the per-part alpha slot — kind-4 coronas carry obj+0x106/255 in
        // .a (the shader's ×255/128 rescale makes byte 0x2e → 0.36 exactly like the GS); the other
        // parts stay 1.0. The byte is the sim's (`Searchlight_Update` 0x1cb900:334-336).
        if let Some(s) = self.sim.as_ref() {
            for (k, sp) in s.drones.searchlights.parts.iter().enumerate() {
                if let Some(c) = cols.get_mut(l.col_sl as usize + k) {
                    c[3] = if sp.kind == 4 { sp.alpha as f32 / 255.0 } else { 1.0 };
                }
            }
        }
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
        // The gas puffs' live rgba (the object's obj+0x103..0x106 bytes the draw reads [M 0x1ab2d0 tail]):
        // read straight from the pool — the alpha byte fades the quad out.
        let gas_rgba: Vec<Option<[u8; 4]>> = self.attachments.iter().map(|a| {
            a.gas.and_then(|gi| self.sim.as_ref().and_then(|s| s.drones.gas.puffs.get(gi as usize))
                .map(|g| g.rgba.map(|c| c.round().clamp(0.0, 255.0) as u8)))
        }).collect();
        // The corpse fade (fix-ai-remainder; G-026): the game's `NDrone2_FadeOut` 0x172dd8 decays the
        // drone object's alpha byte obj+0x106 (0.6 per tick in Fade 0x48, 8.0 in FadeFast 0x49) and the
        // character draw multiplies the vertices' alpha by it (Dead 0x47's timer-0xC hands the corpse to
        // Fade, which disables it at 0 — the removal). Per scene character, from its drone.
        let char_alpha: Vec<f32> = (0..self.characters.len()).map(|ci| {
            match self.sim.as_ref().and_then(|s| s.char_drone.get(ci).copied().flatten()) {
                Some(di) => self.sim.as_ref().map_or(1.0, |s| (s.drones.list[di].alpha / 255.0).clamp(0.0, 1.0)),
                None => 1.0,
            }
        }).collect();
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
                let rgba = if let Some(ai) = cv.attachment.and_then(|ai| gas_rgba.get(ai as usize)).copied().flatten() {
                    ai
                } else if m == [1.0; 3] { v.rgba } else {
                    let rgb = [0, 1, 2].map(|k| ((v.rgba[k] as f32 * m[k]).round() as i32).clamp(0, 255) as u8);
                    [rgb[0], rgb[1], rgb[2], v.rgba[3]]
                };
                // The corpse fade rides the plain character vertices only (the pickups, the attachments and
                // the searchlight/door parts are their own objects with their own alpha).
                let ca = if cv.searchlight.is_none() && cv.door.is_none() && cv.pickup.is_none() && cv.attachment.is_none() {
                    char_alpha.get(cv.character as usize).copied().unwrap_or(1.0)
                } else { 1.0 };
                let a = ((rgba[3] as f32 * ca).round() as i32).clamp(0, 255) as u8;
                Vertex { pos, uv: v.uv, rgba: [rgba[0], rgba[1], rgba[2], a], nrm: v.nrm }
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
