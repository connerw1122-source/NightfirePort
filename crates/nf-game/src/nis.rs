//! Script players and level scripts (`SP_*` 0x1db9a8–0x1dcc30, `Script_*` 0x1d8248–0x1db6e0). Spec:
//! docs/systems/script-player.md, format docs/formats/nis-script.md.
//!
//! A script drives objects from its streams: entities (glist objects moved along keys, parented to other streams),
//! animated objects (a skin playing an animation script with its root in world space: the wine truck), sounds,
//! events that set switch channels. Objects a script made outlive it when it set "keep objects" (event 0xC).

use crate::triggers::Channels;
use nf_formats::anim::{mul, quat_trans_to_mat, Mat4};
use nf_formats::nis::{Key, Nis};
use nf_formats::placement::Placement;
use std::collections::HashMap;
use std::sync::Arc;

/// An object a script created (`Script_CreateEntity` / `Script_AnimStart`).
#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    /// A glist entity: hash, `Script_CreateEntity` object type, its parameters, world matrix (scaled), key alpha.
    Entity { hash: u32, kind: u8, params: Vec<u32>, matrix: Mat4, alpha: f32, src: (u32, u8) },
    /// An animated skin: the animation script and its integer frame (from 1); the object sits at the origin and the
    /// sequence's root carries it (docs/systems/script-player.md). `weapon` is the byte Event 0xB writes on
    /// `sAnimObject+0x62` (`Script_EventHandler` 0x1d94c0:152-158) — the held weapon's `weapon_data` index, applied
    /// by `AnimObjectDraw` (0x1c4490:256-265) as `weapon_data[w]+0x80` mesh on datum `weapon_data[w]+0x50`.
    /// `ticked`: false until the stream's first `Script_Interp` runs — the game's fresh anim script has no previous
    /// time, so the start frame's own commands fire on that first interp (`AnimScriptTick`'s old time is 0).
    Anim { skin: u32, script: u32, frame: f32, weapon: u8, src: (u32, u8), ticked: bool },
    /// A cut-scene camera (`CAM_INFO`, `Script_CameraStart`): its world matrix (rows: right, up, forward, position)
    /// and vertical field of view in radians (key scale × `+0x114` = 4/3, `Camera_CalcViewAngles`).
    Camera { matrix: Mat4, fov: f32, src: (u32, u8) },
    /// A script sprite (`Script_SpriteStart` 0x1d9300 → `Sprite_CreateLink(4)`): the sprite's hash
    /// (`hashtable_set_sprite`) and its screen rect in the HUD's 512×448 space — the game scales the
    /// 640×480 script coords by 0.8 (x/w) and 0.9333 (y/h) [M 0x1d9300:31-36] — plus the flag bits it
    /// set (script bit 2 → +0x22 |= 0x40, bit 4 → |= 0x80, always |= 0x100) and draw layer +0x38 = 2.
    Sprite { hash: u32, rect: [f32; 4], flags: u16, src: (u32, u8) },
    /// A script light (`Script_LightStart` 0x1db1b0 → `Light_Create` 0x1cc3d8 at the origin vector
    /// 0x2d6200 [L ACTION.ELF: (0,0,0)]): the kind byte b2 (the call's `b2 == 1` argument) and the
    /// stream's parent pair (b0/b1 → +0x25). Deleted by `Script_KillStream` case 6 unless the script
    /// keeps its objects (flag 0x10) [M 0x1d9860:104-113].
    Light { kind: u8, src: (u32, u8) },
}

/// A request the script made this tick for the caller to carry out.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// `Sound_Play` (2-D) or `Sound_Play3D` at object `at` (an index into `Scripts::objects`), from stream `src`
    /// (script hash, stream): `Script_Interp` case 5 moves it with that object every frame.
    Sound { id: u32, at: Option<usize>, src: (u32, u8) },
    /// An animation script's op-1 sound (`AnimProcessScriptCmds` 0x1c1b38 case 1 → `Sound_Play3D` at the anim
    /// object, `Script_Interp` case 2's `AnimObjectUpdate` firing it): the cut-scene characters' lip-synced
    /// voice lines and one-shot effects are baked into their animation scripts this way. `at` is the owning
    /// `Obj::Anim`'s object index (the sound's position, read each drain).
    AnimSound { id: u32, at: Option<usize> },
    /// The stream's sound ends (`Script_SoundEnd` / `Script_KillStream` → `Sound_Stop`).
    SoundStop { src: (u32, u8) },
    /// `Text_FlushAllSubtitles()` (0x1b15c8) from `Script_FFwd`'s skip arm [M 0x1dac78:28-29, the
    /// `ScriptCam != 0` branch]: the shown dialogue line and the pending script texts die at the skip.
    TextFlush,
    /// An event the port does not carry out yet (type, parameters).
    Event { kind: u8, params: Vec<u32> },
    /// Event 4: `Player_Disable` — the player stops (the script runs a cut-scene).
    PlayerDisable,
    /// `Script_Free` after event 6: `Player_Enable` at this matrix (`None`: where the player is), with `+0xAB6`.
    PlayerEnable { at: Option<Mat4>, mode: u8 },
    /// Event 0x12: `ResetMap_LevelToLoad(level, 1, 0)` — the script-driven level transition
    /// [M Script_EventHandler 0x1d94c0:189-199]. Note: no NIS script on 07000005 fires kind 0x12 — the truck script
    /// (0x06000A22) fires only 0xC (keep objects, tick 5890) and 0xD (channel 0x73 = 1, tick 5412). The actual level
    /// transition on 07000005 is the placement-0xE8 trigger at the castle door (Bond walks to it after the truck parks).
    LoadLevel { level: u32 },
    /// A camera stream started (`Script_CameraStart` → `Music_Event(0xB, script)`) or ended (`Script_KillStream` →
    /// `Music_Event(0xC, script)`).
    Camera { script: u32, start: bool },
    /// Event 5 / `Script_Free`'s 0x80 flag: `Drone_EnableAll(on, hash)` — the game's own drone-freeze gate
    /// (`NPCGlobals = on`). Event 5 disables (mid-cut-scene); `Script_Free` re-enables when the script set flag 0x80
    /// (event 7) [M Script_EventHandler 0x1d94c0 case 5, Script_Free 0x1da880:41-43, Drone_EnableAll 0x1383e8].
    /// The two hash-specific enable extras (0x600001b → msg 10 + Music_Event(0xf), 0x600002e → msg 0xe walk) are
    /// not ported — neither script is live on the ported levels.
    DroneEnable { on: bool },
    /// Event 0xF: the SP create-callback hand-off (`Script_EventHandler` 0x1d94c0:172-175 case 0xF
    /// calls `SPINFO+0x84` with the CURRENT stream's entity and the event's param 0) — the owning
    /// ScriptPlayer's placement (`player`, the fusebox's), the object index in `Scripts::objects`
    /// and the group (param 0; `FuseBox_Register` 0x1b0970 guards group < 2).
    WireRegister { player: usize, obj: usize, group: u16 },
    /// Op 0x13: `Script_FadeStart` 0x1db3d0 → `Camera_SetFade(-f, 4, 0xff, 0)` (0x1bbdd8) — a black
    /// overlay (colour 0x000000ff: RGB 0, alpha target 0xff) whose alpha ramps |1/f| per tick on the
    /// viewer; f < 0 starts at alpha 1 and fades in, f > 0 starts at 0 and fades out. Suppressed
    /// while `SCRIPTINFO+0xAB5` bit 0 (the skip latch) is set [M 0x1db3d0:10-12].
    /// `viewer`: `Camera_SetFade`'s last argument — 0 the cut-scene (script camera) viewer, 1 the gameplay viewer.
    Fade { ticks: f32, viewer: u8 },
    /// Op 0x1e: `Script_TextStart` 0x1db2b8 → `Text_AddMsg(-1, 0, 4, Txt_BindLabel(label, 0), 0,
    /// time)` — a HUD message from the text bank shown for `time` frames [M 0x1db2b8:9-12]. The raw
    /// label travels (the Lang table lives above nf-game); nf-view binds it like the pickup messages.
    Text { label: u32, time: u16 },
}

#[derive(Clone, Debug, Default)]
struct Stream {
    start_clock: f32,
    start_frame: f32,
    clock: f32,
    wait: f32,
    /// Index into `Scripts::objects`.
    obj: Option<usize>,
    /// Command offset; `None` = the stream stopped (a command the port does not read yet).
    pc: Option<usize>,
    first_key: u16,
    last_key: u16,
    tag: u16,
    flags: u16,
    kind: u8,
    parent: i8,
    /// `Script_CreateEntity`'s object type of the current entity (reuse check).
    obj_type: u8,
    /// Op 0x1b (`Script_SubScriptStart` 0x1db050): the running sub-script's index in the owning
    /// `Player`'s `subs` (`Script_Load`'s SCRIPTINFO at stream+0x10); `None` = none yet.
    sub: Option<usize>,
    /// `Script_SoundStart`'s live handle (`stream+0x10`), as the sound id the EE keeps at Sound+0x34
    /// (`Sound_IsLooping` reads the SFX table through it [M 0x1de260]); `None` = no live sound — the
    /// game's handle 0, from a refused start, a 0xffff id or after a kill [M 0x1d9860:96-127].
    sound_id: Option<u32>,
    /// `stream+0x22`: the frame the line started (`Script_SoundStart` writes stream clock + 1) [M 0x1d8820].
    sound_start_frame: i16,
}

/// A running script (`SCRIPTINFO`).
#[derive(Clone, Debug)]
pub struct Script {
    pub nis: Arc<Nis>,
    /// `+0x00`: the player's rotation and position.
    pub matrix: Mat4,
    pub scale: f32,
    pub delay: i32,
    pub clock: f32,
    /// `+0xAA8`: 0x10 keep objects, 0x20, 0x40 camera, 0x80 re-enable drones on free (`Drone_EnableAll(1, hash)`,
    /// set by event 7), 0x100 player return.
    pub flags: u16,
    /// `+0x98` ScriptCam (event 0x13): a script camera is active.
    pub script_cam: bool,
    /// `+0xAB2`: 1 playing, 2 done, 8 looping.
    pub play: u8,
    /// `+0xA4` fade clock: the skip's fade-out progress (`Script_FFwd` sets it to `FRAME_RATE_MUL`; the game's
    /// `Camera_SetFade` call itself is view-side and not modelled).
    pub fade_clock: f32,
    /// `+0xAB4` fade length: `min(length − clock, 45)` frames from `Script_FFwd` (0x1dac78:48-51).
    pub fade_len: u8,
    /// `+0xAB5`: bit 0 skip latch (a skip ran), bit 4 camera active (event 0xD set / `Script_CameraEnd`
    /// cleared), bit 5 fade past its length — anims and the camera freeze while it is set (`Script_Interp`
    /// 0x1d9a08:146,205).
    pub fade_flags: u8,
    /// Event 6: where the player comes back (`+0x40`) and `+0xAB6`.
    pub player_at: Option<Mat4>,
    /// Requests raised where no output list is at hand (`Script_KillStream`), passed on by the next update / free.
    pending: Vec<Request>,
    /// Event 0xB writes the weapon byte on the previous stream's Anim object, but `event()` borrows `objs`
    /// immutably (`&Objects`). The write is deferred to the next `update()` pass: `(obj index, weapon)`.
    pending_weapon: Vec<(usize, u8)>,
    pub player_mode: u8,
    streams: Vec<Stream>,
    /// The owning `Player`'s placement index (for `NF_SCRIPTLOG`); 0 until `Player::load` sets it.
    pub player: usize,
}

/// The port's object store for scripts: indices stay valid; `None` = deleted.
#[derive(Clone, Debug, Default)]
pub struct Objects {
    pub list: Vec<Option<Obj>>,
    /// `Script_CreateEntity` 0x1d8968's obj_type 0xF0 arm: `Pickup_Create` (0x18dda0) with the
    /// entity params (kind/id/rounds/switch from the first four, sound the fifth [M disasm
    /// 0x1d8bcc-0x1d8c20: t0=lhu 0x349974 kind, t1=lhu 0x349978 id, t2=lw 0x34997c rounds,
    /// t3=lw 0x349980 switch, 0(sp)=lw 0x349984 sound, 8(sp)=1 rest, lifetime 0, label 0]) at the
    /// stream's current key position (the 0x2d6200 scratch VECTOR; rest=1, so it rests there). The
    /// game emits this only on the CREATE path (an existing same-class stream object is re-pointed
    /// without a new pickup [M 0x1d8a68-0x1d8a80]). The sim drains it after the scripts tick.
    pub pending_pickups: Vec<PendingPickup>,
    /// `Script_CreateEntity` 0x1d8968's obj_type 0xF5 arm: `Drone_AIVolume_Create` (0x1376c0) —
    /// created at the scratch origin with the identity scales and keyed from the script from the
    /// same tick on [M 0x1d8a58 + Script_EntityStart's `Script_Interp` tail]; the caller drains it
    /// into the level's volume list (the game's `LList_Add` 0x2706c8 registration [M 0x1376c0:119]).
    /// 07000005's truck script 0x06000A22 stream 2 (glist 0x0200040C, params [0]) is the disc's
    /// only script-created volume [D the 289 census].
    pub pending_volumes: Vec<PendingVolume>,
}

/// One queued `Drone_AIVolume_Create` (0x1376c0) from a script's Entity command obj_type 0xF5 —
/// see `Objects::pending_volumes`.
#[derive(Clone, Debug)]
pub struct PendingVolume {
    pub script: u32,
    pub stream: u8,
    /// The Entity command's glist hashcode — the half-extents come from its map-object bounds
    /// (|max − min| / 2 [M 0x1376c0:84-92: `glist+0x20` min, `+0x30` max, ×0.5 × the scale triple]).
    pub hash: u32,
    /// The stream's posed matrix at the creation tick (the game creates at the scratch origin
    /// 0x2d6200 [L (0,0,0)] with the identity scale triple {1,1,1} [M 0x1d8a58], and the object
    /// matrix is keyed from the script from the same tick on).
    pub matrix: Mat4,
    /// The Entity command's parameters: [0] × 0.01 is the visibility factor (+0x20, `level+0x2c`),
    /// [1] the kind selector (+0x24, `level+0x30`: 0 → 0, 1 → 2, 2 → 1, 4 → 3 [M 0x1376c0:100-116]).
    pub params: Vec<u32>,
}

/// One queued `Pickup_Create` from a script's Entity command (see `Objects::pending_pickups`).
#[derive(Clone, Debug)]
pub struct PendingPickup {
    pub script: u32,
    /// The Entity command's object hashcode (the glist `Pickup_Create` receives, 0x1d8bcc).
    pub hash: u32,
    pub stream: u8,
    pub kind: u16,
    pub id: u16,
    pub rounds: i16,
    pub switch_ch: u16,
    pub sound: u32,
    /// The stream's posed matrix at the creation tick (the game's scratch VECTOR 0x2d6200).
    pub matrix: Mat4,
}

/// What the runtime needs from the game: animation lengths and an animated object's discrete object 0 (the world
/// matrix of the skin's first rigid part's bone).
pub trait World {
    fn anim_length(&self, script: u32) -> Option<u16>;
    fn discrete0(&self, skin: u32, script: u32, frame: f32) -> Option<Mat4>;
    /// An animation script's op-1 sound ids with frame in `(from, to]` — the interval `AnimProcessScriptCmds`
    /// (0x1c1b38 case 1) fires, in command order. Empty default: only the runtime's `Data` holds the scripts.
    fn anim_ops1(&self, script: u32, from: i32, to: i32) -> Vec<u32> { let _ = (script, from, to); Vec::new() }
    /// The running level (fix-finale-blockers: the 0700004a lift stand-in keys on it).
    fn level(&self) -> u32 { 0 }
    /// fix-script-objects: a glist's LOCAL bounding sphere (the map object's centre and radius —
    /// `Control_BuildWorldSph` 0x133070 transforms the centre by the object matrix into obj+0x80,
    /// obj+0x8C carries the radius). `None` = the hash resolves to nothing above nf-game.
    fn glist_sphere(&self, hash: u32) -> Option<([f32; 3], f32)> { let _ = hash; None }
    /// `Sound_IsLooping` (0x1de260): the SFX table's loop bit for the sound id (`SFXOutputData
    /// [id*0x18+0x14]`) — `Script_SoundEnd` (0x1daf60) stops only a looping sound at its marker.
    /// Default true (stop): no-table callers behave as before.
    fn sfx_looped(&self, id: u32) -> bool { let _ = id; true }
}

const IDENT: Mat4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

impl Script {
    /// `Script_Load` (0x1d8248): `matrix` = `View_RotTransMatrix(rotation, position)` of the player.
    pub fn load(nis: Arc<Nis>, matrix: Mat4, scale: f32) -> Script {
        let streams = nis.streams.iter().map(|s| Stream { first_key: s.first_key, last_key: s.last_key, pc: Some(0), parent: -1, ..Default::default() }).collect();
        let flags = if nis.camera { 0x40 } else { 0 };
        Script { nis, matrix, scale, delay: 0, clock: 0.0, flags, script_cam: false, play: 0, fade_clock: 0.0, fade_len: 0, fade_flags: 0, streams, player_at: None, player_mode: 0, pending: Vec::new(), pending_weapon: Vec::new(), player: 0 }
    }

    /// `Script_Play` (0x1db588).
    pub fn start(&mut self, looping: bool) { self.play = if looping { self.play | 9 } else { (self.play & !8) | 1 }; }
    /// `Script_Stop`.
    pub fn stop(&mut self) { self.play &= !1; }
    /// `Script_Set2Start` (0x1daf00).
    pub fn rewind(&mut self) {
        for s in &mut self.streams { s.clock = 0.0; s.pc = Some(0); }
        self.play &= !2;
    }
    /// `Script_IsPlaying` (0x1db630).
    pub fn playing(&self) -> bool { self.play & 1 != 0 && (self.play & 8 != 0 || self.play & 2 == 0) }

    /// `Script_IsDeathNIS` (0x1dae60): the three death cut-scenes can NOT be skipped [M 0x1dae60:6-17].
    pub fn is_death_nis(hash: u32) -> bool { matches!(hash, 0x0600_0952 | 0x0600_098a | 0x0600_0991) }

    /// `Script_FFwd(si, 1)` (0x1dac78) — the game's cut-scene skip, called from `SP_Update` state 2 while the
    /// player presses the skip action (`Input_Action(-1, 0x19, 1)` [M 0x1dbda8:129-141]).
    ///
    /// Only a script with its camera running (event 0xD set `+0xAB5` bit 4; `Script_CameraEnd` clears it) can
    /// skip [M 0x1dac78:19-21]. On the FIRST press [M :23-84]: a no-cameras script (event 0x10) just kills its
    /// camera stream; otherwise the fade is armed — `+0xAB4 = min(length − clock, 45)`, `play |= 0x10`,
    /// `+0xA4 = FRAME_RATE_MUL`, `Music_Event(5, hash)`, `+0xAB5 |= 1`, `Camera_SetFade(+0xAB4, 4, 0xff, 1)`
    /// (view-side; not modelled) and every kind-2 stream's datum gets `+0xC8 |= 0x40000` (the snap flag; the
    /// port's Anim objects carry a frame number, no datum). Later presses only clear `play` bit 4 [M :86-88].
    /// The end-of-script effects still apply: the fade's catch-up fast-forwards the script through every
    /// remaining command in `Script_Update` (below), so events/channels/`LoadLevel` all run.
    pub fn ffwd(&mut self, subs: &mut Vec<Option<Script>>, objs: &mut Objects, out: &mut Vec<Request>) {
        if self.fade_flags & 0x10 == 0 || self.fade_flags & 1 != 0 {
            // No camera (or a skip already armed): the game only clears play bit 4 (0x1dac78:86-88).
            self.play &= !0x10;
            return;
        }
        if self.flags & 0x20 != 0 {
            // Event 0x10 (no cameras): kill the camera stream(s) and return — no fast-forward [M :32-47].
            for i in 0..self.streams.len() { if self.streams[i].kind == 4 { self.kill(i, subs, objs); } }
            return;
        }
        let remaining = self.nis.length as f32 - self.clock;
        self.fade_len = remaining.max(0.0).min(45.0) as u8;
        self.play |= 0x10;
        self.fade_clock = 1.0; // FRAME_RATE_MUL
        self.fade_flags |= 1;
        // Music_Event(5, hash) [M 0x1dac78:55]: the skip's music pulse (EventList[5] = hash; the port's music
        // runtime has no consumer for the skip pulse, so it rides the uncarried Event request).
        out.push(Request::Event { kind: 0x40, params: vec![self.nis.hashcode] });
        // The ScriptCam arm's other half [M 0x1dac78:26-30]: `Input_ClearAllActions(-1)` (the port's skip is
        // edge-latched, nothing to clear) and `Text_FlushAllSubtitles()` — the shown dialogue line dies with
        // the skip (the user's playtest 1014-1725: the port kept it). Reaching here means the camera flag is
        // set (the top gate), which is the game's `ScriptCam != 0`.
        out.push(Request::TextFlush);
    }

    /// `Script_Update` (0x1da998): every stream's `Script_Run`, then the clocks advance one tick. The game's
    /// do-while [M 0x1da998:19-100] loops while the skip fade is past its length (`fade_clock > fade_len`) and
    /// the script is still playing — that IS the skip's fast-forward: the remaining script (streams, commands,
    /// events, end) runs to completion inside this one call, then the final block fades back in [M :101-110].
    pub fn update(&mut self, subs: &mut Vec<Option<Script>>, lib: &HashMap<u32, Arc<Nis>>, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>) {
        // Flush Event 0xB's deferred weapon writes before any early-return: even if the script has since stopped
        // or is waiting on a delay, the Anim object's weapon byte must reflect the event the script already fired.
        if !self.pending_weapon.is_empty() {
            for (o, wp) in self.pending_weapon.drain(..) {
                if let Some(Some(Obj::Anim { weapon, .. })) = objs.list.get_mut(o) { *weapon = wp; }
            }
        }
        if self.play & 1 == 0 { return; }
        // `Script_Update`'s delay gate [M disasm 0x1da9c4-0x1da9dc: `lw $v0,156($s0)` (SCRIPTINFO+0x9c);
        // `beql $v0,$zero,0x1da9e0` → delay == 0 RUNS the streams; `blez $v0,0x1dac68` → delay < 0
        // branches straight to the epilogue — a HOLD, no store, no decrement; the fall-through stores
        // `$v0-1` — only a POSITIVE delay counts down and skips the tick]. A negative delay (event 3's
        // `p0 − 1` with p0 = 0, the 0700001b panels' 0x06000832 stream 11 [D]) HOLDS the script until
        // `SP_UnPause` 0x1dca58 zeroes it (the panels: SP_Update's damage path; the 0700004a lift
        // 0x06000571: the state-2 mask&1/out-channel rule). The fix-finale-blockers version ran a
        // negative delay like 0 — that made every solar panel shatter at t5 (GAPS G-047) and was the
        // wrong reading of 0x1da9c4: the lift had never blocked on the delay itself but waits on its
        // channel (see the state-2 unpause below).
        if self.delay != 0 {
            if self.delay > 0 { self.delay -= 1; }
            return;
        }
        if !self.playing() { return; }
        // The game's loop guard is only `Script_IsPlaying`; a looping script (play & 8) would spin forever, and
        // the data never skips one — bound the iterations (a guard, not a behaviour change: a non-looping script
        // finishes within `length` iterations because op 6 fires at clock >= length [M Script_Run :43-49]).
        let max_passes = 2 * self.nis.length as usize + 256;
        for pass in 0..max_passes {
            for i in 0..self.streams.len() { self.run(i, subs, lib, objs, ch, w, frame, out); }
            if pass > 0 {
                // The catch-up's subtitle adds never display as a continuing dialogue in the game [S]:
                // `Script_FFwd` flushed the subtitles at the skip [M 0x1dac78:28-29] and the game draws a
                // single active subtitle slot (`Text_Update` 0x1b1510) — while the port's queued HUD lines
                // would replay the whole remaining dialogue after the skip (the user's playtest 1014-1725:
                // "the dialogue still kept going until it finished"). Drop the Text requests the catch-up
                // passes push; the flush request already cleared what was on screen.
                out.retain(|r| !matches!(r, Request::Text { .. }));
            }
            out.append(&mut self.pending);
            self.clock += 1.0;
            for s in &mut self.streams { s.clock += 1.0; }
            // The near-end fade of a camera script [M :44-79]: from `length − 20` on, `Camera_SetFade(40, 4,
            // 0xff, 0)` (view-side; not modelled) unless the hash is in the game's no-fade set; a death NIS
            // breaks the loop there (a death cut-scene never fast-forwards past its fall). The game also gates
            // this on a settings byte (DAT_002d88c0+0x235 != 1) the port has no equivalent for.
            if self.script_cam && self.flags & 0x40 != 0 && self.clock >= self.nis.length as f32 - 20.0 {
                let h = self.nis.hashcode;
                if Script::is_death_nis(h) { break; }
                let no_fade = h == 0x0600_04fa || h == 0x0600_08c8 || h == if h < 0x0600_04fb { 0x0600_0080 } else { 0x0600_0994 };
                // The game's own fade-out in the last 20 ticks of a camera script [M :53-71]:
                // `Camera_SetFade(40, 4, 0xff, 0)` — 40 ticks to black — unless the hash is in the
                // no-fade set. It re-arms every tick; the drain's in-progress guard debounces.
                if !no_fade { out.push(Request::Fade { ticks: 40.0, viewer: 0 }); }
            }
            // The fade bookkeeping [M :84-98]: the skip's fade advances one frame per tick; past its length it
            // sets the freeze flag (bit 5).
            if self.fade_clock != 0.0 {
                if self.fade_clock <= self.fade_len as f32 { self.fade_clock += 1.0; } else { self.fade_flags |= 0x20; }
            }
            if self.fade_clock <= self.fade_len as f32 { break; } // the normal single-pass tick
            if !self.playing() { break; }                          // fade past its length + script finished
            // else: loop again — the game's fast-forward.
        }
        // Final block [M :101-110]: once the fade is past its length and the script stopped, fade back in on
        // the GAMEPLAY viewer — `Camera_SetFade(0xc1b00000, 0, 0xff, 1)` [L 0xc1b00000 = -22.0: 22 ticks from
        // black], clear the fade clock, play bit 4 and the skip latch.
        if self.fade_clock > self.fade_len as f32 && !self.playing() {
            out.push(Request::Fade { ticks: -22.0, viewer: 1 });
            self.fade_clock = 0.0;
            self.play &= !0x10;
            self.fade_flags &= !1;
        }
    }

    /// `Script_Run` (0x1d8548).
    #[allow(clippy::too_many_arguments)]
    fn run(&mut self, i: usize, subs: &mut Vec<Option<Script>>, lib: &HashMap<u32, Arc<Nis>>, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>) {
        let len = self.nis.length as f32;
        let nis = self.nis.clone();
        let cmds = &nis.streams[i].commands;
        // A looping stream whose end sits before the script length would spin; the data never does it.
        for _ in 0..64 {
            if self.streams[i].pc.is_none() { return; }
            if self.streams[i].clock <= len { self.interp(i, subs, lib, objs, ch, w, frame, out); }
            loop {
                let s = &self.streams[i];
                let Some(pc) = s.pc else { return };
                if s.clock < s.wait { return; }
                let Some(&op) = cmds.get(pc) else { self.streams[i].pc = None; return };
                let rd = |o: usize, n: usize| -> Option<u32> { cmds.get(pc + 1 + o..pc + 1 + o + n).map(|b| b.iter().rev().fold(0u32, |a, &x| a << 8 | x as u32)) };
                let mut next = pc + 1;
                match op {
                    5 => { let Some(t) = rd(0, 2) else { self.streams[i].pc = None; return }; self.streams[i].wait = t as f32; next = pc + 4; }
                    6 => { if len <= self.streams[i].clock { self.play |= 2; } next = pc + 4; }
                    4 => {
                        // `Script_StreamEnd` (0x1db420).
                        let Some(t) = rd(0, 2) else { self.streams[i].pc = None; return };
                        let s = &mut self.streams[i];
                        s.wait = t as f32;
                        // Held on the end command (the game rewinds onto it; a looping script would re-run it at the
                        // same clock forever when clock = time < length, which the data never does).
                        if s.clock <= t as f32 && s.clock < len { return; }
                        if self.play & 8 != 0 {
                            s.wait = 0.0;
                            s.pc = Some(0);
                            self.clock = nis.loop_start as f32;
                            s.clock = nis.loop_start as f32 - 1.0;
                            self.play |= 2;
                            break;
                        }
                        self.play = (self.play & !1) | 2;
                        return;
                    }
                    7 => {
                        let (Some(hash), Some(head)) = (rd(0, 4), cmds.get(pc + 5..pc + 10)) else { self.streams[i].pc = None; return };
                        let (has_parent, parent, spline, obj_type, n) = (head[0], head[1], head[2], head[3], head[4] as usize);
                        let params: Vec<u32> = (0..n).filter_map(|k| rd(9 + 4 * k, 4)).collect();
                        next = pc + 1 + 9 + 4 * n + 1;
                        self.entity_start(i, subs, lib, hash, has_parent != 0, parent as i8, spline != 0, obj_type, params, objs, ch, w, frame, out);
                    }
                    9 => {
                        // `Script_EntityEnd` (0x1dafe8).
                        let s = &self.streams[i];
                        if s.obj.is_some() && (s.clock < len || self.play & 8 == 0 || s.tag != 1) { self.kill(i, subs, objs); }
                        next = pc + 2;
                    }
                    0x0a => {
                        let (Some(script), Some(skin), Some(start)) = (rd(0, 4), rd(4, 4), rd(8, 4)) else { self.streams[i].pc = None; return };
                        self.anim_start(i, subs, script, skin, start, objs, w);
                        next = pc + 14;
                    }
                    0x0c => { self.kill(i, subs, objs); next = pc + 2; }
                    0x12 => {
                        let (Some(&n), Some(&kind)) = (cmds.get(pc + 1), cmds.get(pc + 2)) else { self.streams[i].pc = None; return };
                        let p: Vec<u32> = (0..n as usize).filter_map(|k| rd(2 + 4 * k, 4)).collect();
                        next = pc + 3 + 4 * n as usize + 1;
                        self.event(i, kind, p, objs, ch, frame, out);
                    }
                    0x16 => {
                        // `Script_SoundStart` (0x1d8820): 2-D when asked or when the object stream has no object.
                        let (Some(id), Some(b)) = (rd(0, 4), cmds.get(pc + 5..pc + 8)) else { self.streams[i].pc = None; return };
                        let stream = if b[1] == 0 { i.saturating_sub(1) } else { b[2] as usize };
                        let at = self.streams.get(stream).filter(|o| matches!(o.kind, 1 | 2)).and_then(|o| o.obj);
                        let s = &mut self.streams[i];
                        s.tag = stream as u16;
                        // The tag (+0x20) and the start frame (+0x22 = stream clock + 1) are written
                        // before the start gate [M 0x1d8820:20-28].
                        s.sound_start_frame = (s.clock as i32 + 1) as i16;
                        // The start gate [M 0x1d8820]: `stream+0x10 == 0 && script+0xAB5 & 0x20 == 0`.
                        // A stream with a live handle never re-fires, and once the skip's fade has run
                        // past its length (+0xAB5 bit 5 — the catch-up state `Script_Update` sets) no
                        // new line starts: the skipped script's remaining lines (and their SoundEnd
                        // arms) never come to be. The kind still becomes 5 even when the id is the
                        // 0xffff "none" (handle 0) [M :36-52].
                        if s.sound_id.is_none() && self.fade_flags & 0x20 == 0 {
                            s.kind = 5;
                            s.sound_id = (id & 0xfffff != 0xffff).then_some(id);
                            out.push(Request::Sound { id, at: if b[0] == 1 { None } else { at }, src: (self.nis.hashcode, i as u8) });
                        }
                        next = pc + 9;
                    }
                    0x17 => {
                        // `Script_SoundEnd` (0x1daf60): the marker stops only a live LOOPING sound
                        // under the game's condition — `Sound_IsLooping` reads the SFX table's loop
                        // bit for the handle's id [M 0x1de260]; a one-shot line keeps playing to its
                        // own sample end. A stream with no live handle returns from `Script_KillStream`
                        // case 5 untouched (kind and restartability stay) [M 0x1d9860:96-99]. The
                        // kill/free path stops unconditionally (kill() case 5).
                        let s = &mut self.streams[i];
                        if let Some(sid) = s.sound_id {
                            if w.sfx_looped(sid) && (s.clock < self.nis.length as f32 || self.play & 8 == 0 || s.sound_start_frame != 1) {
                                s.sound_id = None;
                                s.kind = 0;
                                out.push(Request::SoundStop { src: (self.nis.hashcode, i as u8) });
                            }
                        }
                        next = pc + 2;
                    }
                    0x0d => {
                        // `Script_CameraStart` (0x1d9140): u8 spline, u16 first key, u16 last key; the camera uses keys
                        // first..last − 1. Skipped while flag 0x20 (no cameras). The camera-active flag (`+0xAB5 |=
                        // 0x10` [M 0x1d9140]) arms `Script_FFwd`'s skip.
                        let (Some(&sp), Some(a), Some(b)) = (cmds.get(pc + 1), rd(1, 2), rd(3, 2)) else { self.streams[i].pc = None; return };
                        next = pc + 7;
                        if self.flags & 0x20 == 0 {
                            let s = &mut self.streams[i];
                            if s.obj.is_none() {
                                objs.list.push(Some(Obj::Camera { matrix: IDENT, fov: 1.0, src: (self.nis.hashcode, i as u8) }));
                                s.obj = Some(objs.list.len() - 1);
                            }
                            s.kind = 4;
                            out.push(Request::Camera { script: self.nis.hashcode, start: true });
                            s.first_key = a as u16;
                            s.last_key = (b as u16).wrapping_sub(1);
                            s.flags = if sp != 0 { 2 } else { 0 };
                            self.fade_flags |= 0x10;
                            self.interp(i, subs, lib, objs, ch, w, frame, out);
                        }
                    }
                    0x0f => {
                        // `Script_CameraEnd`.
                        if self.streams[i].clock < len || self.play & 8 == 0 { self.kill(i, subs, objs); }
                        self.fade_flags &= !0x10; // the camera-active flag clears [M 0x1db348:21]
                        next = pc + 2;
                    }
                    0x13 => {
                        // `Script_FadeStart` 0x1db3d0: one f32, then `Camera_SetFade(-f, 4, 0xff, 0)`
                        // (0x1bbdd8 — the game's viewer overlay: rate 1/|f| per tick, f < 0 starts at
                        // alpha 1 and fades in, f > 0 fades out) unless the skip latch (+0xAB5 bit 0)
                        // is set [M 0x1db3d0:10-12].
                        let Some(t) = rd(0, 4) else { self.streams[i].pc = None; return };
                        if self.fade_flags & 1 == 0 { out.push(Request::Fade { ticks: -f32::from_bits(t), viewer: 0 }); }
                        next = pc + 6;
                    }
                    0x14 => {
                        // `Script_SpriteStart` 0x1d9300: 5×u16 (x, y, w, h, flags) + u32 hash. The
                        // sprite object is created once per stream (`if (+0x10 == 0)
                        // Sprite_CreateLink(4)`) and updated in place; the 640×480 coords scale to
                        // the HUD's 512×448 by 0.8 (x/w) / 0.9333 (y/h), layer +0x38 = 2, and
                        // sprite+0x22 |= 0x100 always, | 0x40 when script flag 2, | 0x80 when 4
                        // [M 0x1d9300:23-46].
                        let (Some(x), Some(y), Some(sw), Some(sh), Some(fl), Some(hash)) =
                            (rd(0, 2), rd(2, 2), rd(4, 2), rd(6, 2), rd(8, 2), rd(10, 4)) else { self.streams[i].pc = None; return };
                        let rect = [x as f32 * 0.8, y as f32 * 0.933_333_4, sw as f32 * 0.8, sh as f32 * 0.933_333_4];
                        let flags = fl as u16 | 0x100 | if fl & 2 != 0 { 0x40 } else { 0 } | if fl & 4 != 0 { 0x80 } else { 0 };
                        let o = match self.streams[i].obj {
                            Some(o) => o,
                            None => {
                                objs.list.push(Some(Obj::Sprite { hash, rect, flags, src: (self.nis.hashcode, i as u8) }));
                                let o = objs.list.len() - 1;
                                self.streams[i].obj = Some(o);
                                o
                            }
                        };
                        if let Some(Some(Obj::Sprite { hash: hh, rect: r, flags: ff, .. })) = objs.list.get_mut(o) {
                            *hh = hash; *r = rect; *ff = flags;
                        }
                        self.streams[i].kind = 3;
                        next = pc + 16;
                    }
                    0x15 => {
                        // `Script_SpriteEnd` 0x1db3b0 → `Script_KillStream` case 3: `Sprite_Delete`.
                        self.kill(i, subs, objs);
                        next = pc + 2;
                    }
                    0x18 => {
                        // `Script_LightStart` 0x1db1b0: 3 bytes — b0/b1 the parent pair (b0 == 0 →
                        // +0x25 = −1, else the parent stream), b2 the light kind (`Light_Create`
                        // 0x1cc3d8's `b2 == 1` argument; position = the static origin vector 0x2d6200
                        // [L ACTION.ELF: (0,0,0)]; kind 6).
                        let (Some(&b0), Some(&b1), Some(&b2)) = (cmds.get(pc + 1), cmds.get(pc + 2), cmds.get(pc + 3)) else { self.streams[i].pc = None; return };
                        self.streams[i].parent = if b0 == 0 { -1 } else { b1 as i8 };
                        if self.streams[i].obj.is_none() {
                            objs.list.push(Some(Obj::Light { kind: b2, src: (self.nis.hashcode, i as u8) }));
                            self.streams[i].obj = Some(objs.list.len() - 1);
                        }
                        self.streams[i].kind = 6;
                        next = pc + 5;
                    }
                    0x1a => {
                        // `Script_LightEnd` 0x1db298 → `Script_KillStream` case 6: `Light_Delete`
                        // unless the script keeps its objects (flag 0x10).
                        self.kill(i, subs, objs);
                        next = pc + 2;
                    }
                    0x1b => {
                        // `Script_SubScriptStart` 0x1db050: u32 hash + u8 + u8. On a fresh stream
                        // (`+0x10 == 0`): +0x20 = clock + 1, the parent pair (b0 == 0 → −1),
                        // `Script_Load(hash)` (the origin matrix — the stream's keys position the sub,
                        // `Script_Interp` case 7), kind 7 and `Script_Play(sub, 1)` = looping.
                        let (Some(hash), Some(&b0), Some(&b1)) = (rd(0, 4), cmds.get(pc + 5), cmds.get(pc + 6)) else { self.streams[i].pc = None; return };
                        if self.streams[i].obj.is_none() && self.streams[i].sub.is_none() {
                            self.streams[i].tag = (self.streams[i].clock as i16).wrapping_add(1) as u16;
                        }
                        self.streams[i].parent = if b0 == 0 { -1 } else { b1 as i8 };
                        if self.streams[i].sub.is_none() {
                            let Some(n) = lib.get(&hash) else { next = pc + 8; self.streams[i].pc = Some(next); return };
                            let mut sub = Script::load(n.clone(), self.matrix, self.scale);
                            sub.start(true); // `Script_Play(sub, 1)` — playing + looping [M 0x1db050:36]
                            subs.push(Some(sub));
                            self.streams[i].sub = Some(subs.len() - 1);
                        }
                        self.streams[i].kind = 7;
                        next = pc + 8;
                        // The handler's trailing `Script_Interp(param_1, param_2)` [M 0x1db050:38]:
                        // the sub's matrix is keyed and the sub ticks its first update this same tick.
                        self.interp(i, subs, lib, objs, ch, w, frame, out);
                    }
                    0x1d => {
                        // `Script_SubScriptEnd` 0x1db148: kill unless the parent is looping, past its
                        // length, with the sub started on tick 1 (+0x20 == 1) — the loop-boundary
                        // persistence condition, the same shape as `Script_CameraEnd`.
                        if self.streams[i].sub.is_some() {
                            if self.streams[i].clock < len || self.play & 8 == 0 || self.streams[i].tag != 1 { self.kill(i, subs, objs); }
                        }
                        next = pc + 2;
                    }
                    0x1e => {
                        // `Script_TextStart` 0x1db2b8: u32 label + u16 time → `Text_AddMsg(-1, 0, 4,
                        // Txt_BindLabel(label, 0), 0, time)` — the raw label travels to nf-view (the
                        // Lang table lives above nf-game), the pickup-message pattern.
                        let (Some(label), Some(time)) = (rd(0, 4), rd(4, 2)) else { self.streams[i].pc = None; return };
                        out.push(Request::Text { label, time: time as u16 });
                        next = pc + 8;
                    }
                    // `Script_Run`'s default: a one-byte command.
                    _ => {}
                }
                self.streams[i].pc = Some(next);
            }
            // Looping restart (or a held end while looping): interp and commands again at the new clock.
        }
    }

    /// `Script_EntityStart` (0x1d8d78) and `Script_CreateEntity` (0x1d8968).
    #[allow(clippy::too_many_arguments)]
    fn entity_start(&mut self, i: usize, subs: &mut Vec<Option<Script>>, lib: &HashMap<u32, Arc<Nis>>, hash: u32, has_parent: bool, parent: i8, spline: bool, obj_type: u8, params: Vec<u32>, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>) {
        let s = &mut self.streams[i];
        // An object of another type is deleted; one of the same type takes the new glist.
        if let Some(o) = s.obj { if s.obj_type != obj_type { objs.list[o] = None; s.obj = None; } }
        if s.obj.is_none() { s.tag = s.clock as i16 as u16 + 1; }
        let fresh = s.obj.is_none();
        let e = Obj::Entity { hash, kind: obj_type, params: params.clone(), matrix: IDENT, alpha: 1.0, src: (self.nis.hashcode, i as u8) };
        match s.obj {
            Some(o) => objs.list[o] = Some(e),
            None => { objs.list.push(Some(e)); s.obj = Some(objs.list.len() - 1); }
        }
        s.parent = if has_parent { parent } else { -1 };
        s.kind = 1;
        s.flags = if spline { 2 } else { 0 };
        s.obj_type = obj_type;
        let mut none = Vec::new();
        self.interp(i, subs, lib, objs, ch, w, frame, &mut none);
        // `Script_CreateEntity` 0x1d8968's 0xF0 arm (see `Objects::pending_pickups`): a NEW entity of
        // the pickup class creates the takeable pickup at the stream's posed position [M disasm
        // 0x1d8bcc-0x1d8c20]; the reuse path only re-points the gfx. 07000008's SafePickUps script
        // 0x060009E2 spawns the safe's briefcase (kind 4, switch 0x4A) and rocket launcher this way
        // when the safe opens [D the level bin script; the parked 0xF0 placements sit 10 units east
        // inside the east wall].
        if obj_type == 0xF0 && fresh {
            let pos = match self.streams[i].obj { Some(p) => p, None => 0 };
            if let Some(Some(Obj::Entity { matrix, .. })) = objs.list.get(pos) {
                let p = |k: u32| params.get(k as usize).copied().unwrap_or(0);
                objs.pending_pickups.push(PendingPickup {
                    script: self.nis.hashcode, stream: i as u8, hash,
                    kind: p(0) as u16, id: p(1) as u16, rounds: p(2) as u16 as i16,
                    switch_ch: p(3) as u16, sound: p(4), matrix: *matrix,
                });
            }
        }
        // `Script_CreateEntity` 0x1d8968's 0xF5 arm (→ class 0x3E): `Drone_AIVolume_Create` 0x1376c0.
        // The create path only (a same-class stream object is re-pointed at the gfx without a new
        // volume [M 0x1d8a68-0x1d8a80]); the caller resolves the glist's bounds and registers the
        // volume with the level's list. The disc's only instance: 07000005's truck script
        // 0x06000A22 stream 2 (glist 0x0200040C, params [0]) [D the 289 census].
        if obj_type == 0xF5 && fresh {
            let pos = match self.streams[i].obj { Some(p) => p, None => 0 };
            if let Some(Some(Obj::Entity { matrix, .. })) = objs.list.get(pos) {
                objs.pending_volumes.push(PendingVolume {
                    script: self.nis.hashcode, stream: i as u8, hash, matrix: *matrix, params: params.clone(),
                });
            }
        }
    }

    /// `Script_AnimStart` (0x1d8f40).
    fn anim_start(&mut self, i: usize, subs: &mut Vec<Option<Script>>, script: u32, skin: u32, start: u32, objs: &mut Objects, w: &dyn World) {
        if self.streams[i].obj.is_some() { self.kill(i, subs, objs); }
        let len = w.anim_length(script).unwrap_or(1).max(1) as f32;
        let f = start as f32;
        let frame = if f < 1.0 || len < f { if f < 1.0 { 1.0 } else { len } } else { f };
        objs.list.push(Some(Obj::Anim { skin, script, frame, weapon: 0, src: (self.nis.hashcode, i as u8), ticked: false }));
        let s = &mut self.streams[i];
        s.obj = Some(objs.list.len() - 1);
        s.kind = 2;
        s.start_clock = s.clock;
        s.start_frame = frame;
    }

    /// `Script_KillStream` (0x1d9860): an entity, animated object, sprite or light is deleted unless the script
    /// keeps its objects (flag 0x10: only detached — the light's case 6 keeps the object and just clears the
    /// kind [M 0x1d9860:104-113]); a camera pops back (Music_Event(0xC)); a sub-script is freed.
    fn kill(&mut self, i: usize, subs: &mut Vec<Option<Script>>, objs: &mut Objects) {
        let keep = self.flags & 0x10 != 0;
        let s = &mut self.streams[i];
        match s.kind {
            1 | 2 => {
                let Some(o) = s.obj else { if s.kind == 1 { s.kind = 0; } return };
                if !keep { objs.list[o] = None; }
            }
            // The camera pops back to the game's.
            4 => if let Some(o) = s.obj { objs.list[o] = None; self.pending.push(Request::Camera { script: self.nis.hashcode, start: false }); },
            // Case 3: `Sprite_Delete` [M 0x1d9860:76-82].
            3 => if let Some(o) = s.obj { objs.list[o] = None; },
            // Case 6: `Light_Delete` unless the script keeps its objects (the kind clears either way)
            // [M 0x1d9860:104-113].
            6 => if !keep { if let Some(o) = s.obj { objs.list[o] = None; } },
            // Case 7: `Script_Free` on the sub SCRIPTINFO [M 0x1d9860:115-123] — its requests (a
            // PlayerEnable from a returning sub, say) join the parent's pending.
            7 => {
                if let Some(k) = s.sub.take() {
                    if let Some(slot) = subs.get_mut(k) {
                        if let Some(mut sub) = slot.take() {
                            sub.free(subs, objs, &mut self.pending);
                        }
                    }
                }
            }
            // Case 5: `Sound_Stop(stream+0x10)` [M 0x1d9860:96-99] — the NIS voice/sound stream dies with
            // the script (the skip's catch-up frees the script mid-line; the user's playtest 1014-1725:
            // "the voice line itself" kept going to its end because the port never stopped it).
            5 => self.pending.push(Request::SoundStop { src: (self.nis.hashcode, i as u8) }),
            _ => {}
        }
        s.kind = 0;
        s.obj = None;
        s.sound_id = None;
    }

    /// `Script_Free`'s stream pass (`Script_Free` 0x1da880).
    pub fn free(&mut self, subs: &mut Vec<Option<Script>>, objs: &mut Objects, out: &mut Vec<Request>) {
        for i in 0..self.streams.len() { self.kill(i, subs, objs); }
        out.append(&mut self.pending);
        // Flag 0x80 → `Drone_EnableAll(1, hash)` [M Script_Free 0x1da880:41-43]: the cut-scene's guards come back
        // when the script that froze them (event 5 → `NPCGlobals = 0`) is freed and asked for it (event 7).
        if self.flags & 0x80 != 0 { out.push(Request::DroneEnable { on: true }); }
        // fix-final-blockers2 diagnostic: the enable matrix `Script_Free` passes to `Player_Enable(glb_players,
        // SCRIPTINFO+0x40, +0xab6)` [M 0x1da880:46-49]. SCRIPTINFO+0x40 was written ONCE at event-6 time
        // (`Mat_Copy(iVar5+0x90 | glb_players+0x90 → +0x40)` [M 0x1d94c0:125-134]) — the game does NOT re-read
        // the entity at free time, so the snapshot's SOURCE is the whole story.
        if self.flags & 0x100 != 0 {
            if std::env::var("NF_SCRIPTLOG").is_ok_and(|v| v == "1") {
                if let Some(m) = &self.player_at {
                    eprintln!("[scriptlog] free hash {:08x} enable at ({:.2},{:.2},{:.2}) mode {}", self.nis.hashcode, m[3][0], m[3][1], m[3][2], self.player_mode);
                } else {
                    eprintln!("[scriptlog] free hash {:08x} enable at None (the game's case-6 fallback re-reads the PLAYER's live matrix at EVENT time) mode {}", self.nis.hashcode, self.player_mode);
                }
            }
            out.push(Request::PlayerEnable { at: self.player_at, mode: self.player_mode });
        }
    }

    /// fix-final-blockers2 probe accessor: every stream's (kind, bound-object index, first/last key) —
    /// the probe pairs the obj index with `Scripts::objects.list` matrices. Read-only, no behaviour.
    pub fn stream_bindings(&self) -> Vec<(u8, Option<usize>, u16, u16)> {
        self.streams.iter().map(|s| (s.kind, s.obj, s.first_key, s.last_key)).collect()
    }

    /// `Script_EventHandler` (0x1d94c0). Events 3/4/5/6/7/8/9/0xB/0xC/0xD/0xE/0x10/0x11/0x12/0x13 measured there;
    /// event 5 (`Drone_EnableAll(0, hash)`) and event 7 (flag 0x80) are the drone-freeze pair.
    #[allow(clippy::too_many_arguments)]
    fn event(&mut self, i: usize, kind: u8, p: Vec<u32>, objs: &Objects, ch: &mut Channels, frame: u32, out: &mut Vec<Request>) {
        // NF_SCRIPTLOG=1 (or NF_LEVELLOG=1): every script event (op 0x12) fired — tick (script clock), game frame,
        // owning ScriptPlayer's placement index, script hashcode, stream, event kind, params. Lets the user see
        // e.g. that 07000005's EndLevel script (0x06000A23) fires event 0x11 → ch 0x1D, or that the truck script
        // (0x06000A22) fires only events 0xC and 0xD (not 0x12) [M Script_EventHandler 0x1d94c0].
        if std::env::var("NF_SCRIPTLOG").is_ok_and(|v| v == "1") || std::env::var("NF_LEVELLOG").is_ok_and(|v| v == "1") {
            eprintln!("[scriptlog] tick {} frame {} player #{} hash {:08x} stream {} kind 0x{:02x} params {:x?}", self.clock as u32, frame, self.player, self.nis.hashcode, i, kind, p);
        }
        let g = |k: usize| p.get(k).copied().unwrap_or(0);
        match kind {
            3 => {
                // `SCRIPTINFO+0x9C = p0 − 1` (the delay), then every stream's object gets
                // `Vec_Copy2(obj+0xC0 → obj+0x30, obj+0x40)` — the position snapped to its matrix before the
                // pause [M 0x1d94c0:98-117, port-check §6]. The port's streams hold no separate position:
                // `Obj::Entity.matrix` IS the animated matrix, so the sync's invariant (position == matrix
                // while paused) holds by construction.
                self.delay = g(0) as i32 - 1
            }
            4 => out.push(Request::PlayerDisable),
            // Where the player comes back: the previous stream's entity (else where he is), applied by `Script_Free`.
            6 => {
                self.flags |= 0x100;
                let prev = i.checked_sub(1).map(|j| &self.streams[j]).filter(|s| matches!(s.kind, 1 | 2)).and_then(|s| s.obj);
                self.player_at = prev.and_then(|o| match objs.list.get(o) { Some(Some(Obj::Entity { matrix, .. })) => Some(*matrix), _ => None });
                self.player_mode = g(0) as u8;
                // fix-final-blockers2 diagnostic: which stream/object the snapshot came from (the game's iVar5
                // = record(i−1)+0x10 gated on record(i−1)+0x24 kind ∈ {1,2}, fallback `glb_players+0x90`
                // [M 0x1d94c0:73-84,125-134]) — and what else was live at the same instant, so a probe can
                // compare against e.g. the lift platform's SP live matrix.
                if std::env::var("NF_SCRIPTLOG").is_ok_and(|v| v == "1") {
                    let pk = i.checked_sub(1).map(|j| self.streams[j].kind);
                    match (prev, self.player_at) {
                        (Some(o), Some(m)) => eprintln!("[scriptlog] event6 hash {:08x} stream {i} prev obj {o} (kind {:?}) snapshot ({:.2},{:.2},{:.2})", self.nis.hashcode, pk, m[3][0], m[3][1], m[3][2]),
                        _ => eprintln!("[scriptlog] event6 hash {:08x} stream {i} prev kind {:?} → NO entity (the game falls back to the player's own matrix at event time)", self.nis.hashcode, pk),
                    }
                }
            }
            // Event 5: `Drone_EnableAll(0, +0x98 hash)` → `NPCGlobals = 0` — the game's ONLY scripted drone freeze
            // [M Script_EventHandler 0x1d94c0 case 5, Drone_EnableAll 0x1383e8]. Every other cut-scene keeps the
            // guards live (Drone_Control is gated on `NPCGlobals != 0` [0x139268:39-42], not on ScriptCam).
            5 => out.push(Request::DroneEnable { on: false }),
            7 => self.flags |= 0x80,
            // Event 0xB: `Script_EventHandler` case 0xB [M 0x1d94c0:152-158] writes `sAnimObject+0x62 = param[0]` (the
            // weapon byte) on the PREVIOUS stream's Anim object — the same "previous stream" pattern as Event 6.
            // `event()` borrows `objs: &Objects` immutably, so the write is deferred to the next `update()` pass.
            0xb => {
                let w = g(0) as u8;
                let prev = i.checked_sub(1).map(|j| &self.streams[j]).filter(|s| matches!(s.kind, 1 | 2)).and_then(|s| s.obj);
                if let Some(o) = prev { self.pending_weapon.push((o, w)); }
            }
            0xc => self.flags |= 0x10,
            0xd => if g(0) != 0 { ch.set(g(0), g(1) as u8, frame); },
            0xe => self.streams[i].flags |= 1,
            0x10 => self.flags |= 0x20,
            0x11 => {
                let (c, v) = if ch.on(g(0)) { (g(1), g(3)) } else { (g(2), g(3)) };
                if c != 0 { ch.set(c, v as u8, frame); }
            }
            // Event 0x12: `ResetMap_LevelToLoad(level, 1, 0)` — the script-driven level transition (e.g. the truck
            // parking at the end of 07000005) [M Script_EventHandler 0x1d94c0:189-199]. `g(0)` is the next level's
            // hashcode; `Player_RamSave` is skipped (no memory card in the port).
            0x12 => out.push(Request::LoadLevel { level: g(0) }),
            0x13 => self.script_cam = true,
            // Event 0xF (`Script_EventHandler` 0x1d94c0:172-175): the create-callback hand-off —
            // the object of stream i−1 (the handler's record base is +0x80, one 0x28 record below
            // `Script_Run`'s +0xa8 — the same previous-stream pattern as events 6 and 0xB [M
            // 0x1d94c0:84-89]; the kind must be 1/2 = entity/anim), group = param 0, through the
            // SP's callback (`FuseBox_Register` 0x1b0970 for the fuseboxes). The fusebox scripts'
            // wires are the streams' ANIM objects [L dump: 07000014's 0x06000A1B — stream 1's anim
            // registered by stream 2's event (group 0), stream 5's by stream 6 (group 1)].
            0xf => {
                let prev = i.checked_sub(1).map(|j| &self.streams[j]).filter(|s| matches!(s.kind, 1 | 2)).and_then(|s| s.obj);
                if let Some(obj) = prev {
                    out.push(Request::WireRegister { player: self.player, obj, group: g(0) as u16 });
                }
            }
            // Event 9 (cam2mode) is unhandled here: the game routes it via `Player_Cam2Mode`
            // 0x195fc8 → SetCamMode(2) = `Player_PositionCamera` 0x18f5d8 case 2 →
            // `camera_track_XZ_pos_behind` 0x1bc260 + `camera_track_Y_pos` 0x1bc1f0 (the
            // cut-scene behind-Bond orbit) [M Script_EventHandler 0x1d94c0 case 9] — gap.
            _ => out.push(Request::Event { kind, params: p }),
        }
    }

    /// `Script_Interp` (0x1d9a08) for entity, animation and sub-script streams.
    #[allow(clippy::too_many_arguments)]
    fn interp(&mut self, i: usize, subs: &mut Vec<Option<Script>>, lib: &HashMap<u32, Arc<Nis>>, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>) {
        match self.streams[i].kind {
            1 => {
                let s = &self.streams[i];
                let Some(o) = s.obj else { return };
                let Some((mut m, scale, alpha)) = self.keys(i) else { return };
                if scale != 1.0 { for r in m.iter_mut().take(3) { for v in r.iter_mut().take(3) { *v *= scale; } } }
                // Each parent stream's matrix, then the script's. (fix-final-blockers2 tried "the base only
                // at a parent-chain end" per the case-1 tail-Concat structure [M 0x1d9a08:106-133] — it
                // REVERTED: 07000014's fusebox wire and 07000008's safe entities key LOCAL offsets and are
                // render-verified with the base applied, so the game composes the base for unparented
                // streams too (the decompile loses the step). See the report: #289's marker stays the
                // open question (base-composed −107.66 = void on its column; raw −54.13 = the freeze pose;
                // the game's mode-1 SphereIntersect rescue is the live check).)
                let mut p = s.parent;
                let mut n = 0;
                while p >= 0 && (p as usize) < self.streams.len() && n < self.streams.len() {
                    if let Some(pm) = self.stream_matrix(p as usize, objs, w) { m = mul(&m, &pm); }
                    p = self.streams[p as usize].parent;
                    n += 1;
                }
                m = mul(&m, &self.matrix);
                if let Some(Some(Obj::Entity { matrix, alpha: a, .. })) = objs.list.get_mut(o) { *matrix = m; *a = alpha; }
            }
            4 => {
                // `Script_Interp` case 4 → `Script_UpdateCamera`: the keys' matrix (the script matrix applied,
                // *suggested*: the decomp loses that argument; fix-final-blockers2 measured the no-base
                // reading [M 0x1d9a08:190-200 + 0x1db518] and reverted it with the entity change — the
                // render-verified fusebox/safe entities need the base, so the camera keeps it).
                // The skip's freeze: `Script_Interp` returns early while `+0xAB5 & 0x20` [M 0x1d9a08:205].
                if self.fade_flags & 0x20 != 0 { return; }
                let Some(o) = self.streams[i].obj else { return };
                let Some((m, scale, _)) = self.keys(i) else { return };
                let m = mul(&m, &self.matrix);
                if let Some(Some(Obj::Camera { matrix, fov, .. })) = objs.list.get_mut(o) { *matrix = m; *fov = scale * (4.0 / 3.0); }
            }
            2 => {
                // The skip's freeze: `Script_Interp` case 2 returns while `+0xAB5 & 0x20` [M 0x1d9a08:146].
                if self.fade_flags & 0x20 != 0 { return; }
                let s = &self.streams[i];
                let f = s.clock - s.start_clock + s.start_frame;
                let o = s.obj;
                if let Some(Some(Obj::Anim { script, frame, ticked, .. })) = o.and_then(|o| objs.list.get_mut(o)) {
                    let len = w.anim_length(*script).unwrap_or(1).max(1) as f32;
                    // `Script_Interp` case 2 (0x1d9a08) writes the stream's frame into the anim script's time
                    // (+0x90) and zeroes its advance (+0x98), then runs `AnimObjectUpdate` → `AnimScriptTick` →
                    // `AnimProcessScriptCmds(old, new)` [M 0x1d9a08 case 2 + 0x1c2630:31-37]: the op-1 sounds fire
                    // on (old, new] — old is 0 on the first tick (a fresh script has no previous time), the
                    // previous frame afterwards. The game's per-command fired bit only suppresses re-fires while
                    // the window still covers the time, which the interval test already does.
                    let from = if *ticked { *frame as i32 } else { 0 };
                    *frame = f.floor().clamp(1.0, len);
                    *ticked = true;
                    for id in w.anim_ops1(*script, from, *frame as i32) {
                        out.push(Request::AnimSound { id, at: o });
                    }
                }
            }
            7 => {
                // `Script_Interp` case 7 (0x1d9a08): the sub-script's matrix comes from the stream's
                // keys (the sub's own scale applied, like case 1's), each parent stream's matrix, then
                // the script's (the tail `Concat(param_1, ·)` [M 0x1d9a08, the render-verified base
                // composition]) — and the sub is TICKED here: `Script_Update(sub)` is called from the
                // case-7 block (the jal at 0x1d9e4c [M]). The skip's freeze applies (case 1's shape).
                if self.fade_flags & 0x20 != 0 { return; }
                let Some((mut m, scale, _)) = self.keys(i) else { return };
                if scale != 1.0 { for r in m.iter_mut().take(3) { for v in r.iter_mut().take(3) { *v *= scale; } } }
                let mut p = self.streams[i].parent;
                let mut n = 0;
                while p >= 0 && (p as usize) < self.streams.len() && n < self.streams.len() {
                    if let Some(pm) = self.stream_matrix(p as usize, objs, w) { m = mul(&m, &pm); }
                    p = self.streams[p as usize].parent;
                    n += 1;
                }
                m = mul(&m, &self.matrix);
                if let Some(k) = self.streams[i].sub {
                    let mut sub = match subs.get_mut(k) { Some(slot) => slot.take(), _ => None };
                    if let Some(mut sub) = sub {
                        sub.matrix = m;
                        sub.update(subs, lib, objs, ch, w, frame, out);
                        subs[k] = Some(sub);
                    }
                }
            }
            _ => {}
        }
    }

    /// A parent stream's matrix (`Script_GetInterp` without the script matrix): its keys, or for an animation its
    /// discrete object 0.
    fn stream_matrix(&self, i: usize, objs: &Objects, w: &dyn World) -> Option<Mat4> {
        let s = &self.streams[i];
        match s.kind {
            2 => match objs.list.get(s.obj?)? { Some(Obj::Anim { skin, script, frame, .. }) => w.discrete0(*skin, *script, *frame), _ => None },
            _ => self.keys(i).map(|(m, _, _)| m),
        }
    }

    /// `Script_GetInterp` (0x1da158) over the stream's keys at its clock: matrix, scale, `+0x24` value.
    fn keys(&self, i: usize) -> Option<(Mat4, f32, f32)> {
        let s = &self.streams[i];
        let (a, b) = (s.first_key as usize, s.last_key as usize);
        if b < a || b >= self.nis.keys.len() { return None; }
        let k = &self.nis.keys[a..=b];
        let n = k.len() - 1;
        let c = s.clock.floor();
        // The last key with time ≤ ⌊clock⌋ + 0.001 (−1 before the first).
        let idx = k.iter().take_while(|x| x.time <= c + 0.001).count() as isize - 1;
        let lerp = |x: f32, y: f32, t: f32| x * (1.0 - t) + y * t;
        let (pos, q, scale, f24) = if s.flags & 2 == 0 {
            let (i0, i1, t) = if idx < 0 { (0, 0, 0.0) } else if (idx as usize) < n {
                let i0 = idx as usize;
                (i0, i0 + 1, (c - k[i0].time) / (k[i0 + 1].time - k[i0].time))
            } else { (n, n, 0.0) };
            let p = [0, 1, 2].map(|j| lerp(k[i0].pos[j], k[i1].pos[j], t));
            (p, crate::anim::slerp(k[i0].quat, k[i1].quat, t), lerp(k[i0].scale, k[i1].scale, t), lerp(k[i0].f24(), k[i1].f24(), t))
        } else {
            // `GetSplineWeights`: Catmull-Rom over i−1..i+2 (clamped); the rotation slerps i → i+1 by t (*suggested*:
            // the decomp loses the slerp's weight argument, U-040).
            let (ix, wt, t) = if idx < 0 || idx as usize >= n {
                let j = idx.clamp(0, n as isize) as usize;
                ([j; 4], [0.0, 1.0, 0.0, 0.0], 0.0)
            } else {
                let j = idx as usize;
                let t = (c - k[j].time) / (k[j + 1].time - k[j].time);
                let (t2, t3) = (t * t, t * t * t);
                ([j.saturating_sub(1), j, (j + 1).min(n), (j + 2).min(n)],
                 [t2 - (t + t3) * 0.5, 1.0 - t2 * 2.5 + t3 * 1.5, t * 0.5 + t2 + t2 - t3 * 1.5, (t3 - t2) * 0.5], t)
            };
            let sum = |f: &dyn Fn(&Key) -> f32| (0..4).map(|m| f(&k[ix[m]]) * wt[m]).sum::<f32>();
            let p = [sum(&|x| x.pos[0]), sum(&|x| x.pos[1]), sum(&|x| x.pos[2])];
            (p, crate::anim::slerp(k[ix[1]].quat, k[ix[2]].quat, t), sum(&|x| x.scale), sum(&|x| x.f24()))
        };
        let mut m = quat_trans_to_mat(q, pos);
        // fix-grenade-throw: the game's SCRIPTINFO+0x94 scale multiplies only the SCALE output
        // (`*param_7 = key_scale * *(float *)(SCRIPTINFO + 0x94)` [M Script_GetInterp 0x1da158:117])
        // and the entity matrix's 3×3 (`Mat_Scale3f` 0x1e35f0 copies the translation unscaled) — the
        // key TRANSLATION is never scaled. The port scaled m[3] too, which flung the blast script's
        // sub-effects (radius 8 → ×8 offsets) once the blast scale became the radius.
        Some((m, scale * self.scale, f24))
    }
}

/// A ScriptPlayer (placement type 0xD9, `SP_CreateScriptPlayer` 0x1db9a8).
#[derive(Clone, Debug)]
pub struct Player {
    pub placement: usize,
    pub scripts: [u32; 2],
    pub modes: [u16; 2],
    pub stay: bool,
    pub mask: u16,
    pub out: u16,
    pub gate: u16,
    pub scale: f32,
    pub matrix: Mat4,
    pub cur: usize,
    pub state: u16,
    pub activated: bool,
    pub countdown: u8,
    pub info: [Option<Script>; 2],
    pub deleted: bool,
    /// The SP object's LIVE matrix — `Script_Interp` 0x1d9a08:106-133 keys the SP object's `obj+0x90`
    /// from stream 0 every frame (parent chain, then the script base), so the object's world position
    /// follows its script (the elevator cab, the keyboard): `SP_Activate` 0x1dc338's use test reads
    /// THAT sphere, not the placement transform. Starts at the placement matrix.
    pub live_matrix: Mat4,
    /// Op 0x1b's running sub-scripts (`Script_Load`'s SCRIPTINFOs, `Script_Play(sub, 1)`) — the
    /// owning player's, indexed by the parent stream's `sub` handle; `None` = freed
    /// (`Script_KillStream` case 7 [M 0x1d9860:115-123]).
    pub subs: Vec<Option<Script>>,
    /// SP+0x14: the hit points — placement param 7 int→float [M SP_CreateScriptPlayer 0x1db9a8:
    /// `puVar2[5] = (float)(int)*(uint *)(placement + 0x48)`]. A HELD script on a player with hit
    /// points unpauses when the accumulated bullet damage reaches them (SP_Update's damage tail —
    /// GAPS G-047: the 0700001b solar panels carry 20).
    pub hit_points: f32,
    /// SP+0x18: the accumulated hit damage. `SP_Update` adds `SP_GetHitDamage` while held; on the
    /// threshold `SP_UnPause` fires and the threshold is SUBTRACTED, keeping the remainder
    /// [M 0x1dc2f8-0x1dc318].
    pub damage: f32,
    /// The tick's bullet damage gathered on this player's current stream objects by the port's
    /// bullet pass (the game's Collide pass fills the objects' +0xD0 HITDATA chains and
    /// `SP_GetHitDamage` 0x1dc5f8 sums them). Cleared at the bullet pass's head (the game's
    /// Collide_Update clears every hit list each frame), consumed by `update_player`.
    pub tick_damage: f32,
}

impl Player {
    pub fn from_placement(index: usize, p: &Placement) -> Option<Player> {
        let g = |k: u32| p.params.iter().find(|x| x.0 == k).map_or(0, |x| x.1);
        if g(0) == 0 && g(1) == 0 { return None; }
        let matrix = nf_formats::placement::object_matrix(p.rotation, p.translation);
        let (modes, mask) = ([g(2) as u16, g(3) as u16], g(5) as u16);
        let state = if modes[0] == 1 { 7 } else if mask == 0 { 6 } else { 0 };
        Some(Player {
            placement: index, scripts: [g(0), g(1)], modes, stay: g(4) as u8 != 0, mask, out: g(6) as u16, gate: g(9) as u16,
            scale: if g(10) == 0 { 1.0 } else { g(10) as f32 * 0.01 }, matrix, cur: 0, state, activated: false, countdown: 0,
            info: [None, None], deleted: false,
            live_matrix: matrix,
            subs: Vec::new(),
            hit_points: g(7) as i32 as f32,
            damage: 0.0,
            tick_damage: 0.0,
        })
    }

    fn other(&self) -> Option<u32> { Some(self.scripts[self.cur ^ 1]).filter(|&h| h != 0) }

    /// The single-player start checks `SP_Update` (0x1dbda8) runs in state 0 when a trigger lands
    /// [M 0x1dbda8:80-106, disasm 0x1dbf50-0x1dbf70: `lw $v1, glb_blokes; lwc1 $f0, 0x894($v1)` — the
    /// decompiler's `*(float*)(glb_blokes+0x894)` is player 0's BLData+0x894 = Bond's health]:
    /// `bVar5 = health > 0`; any set mission status must be 1 (`Mission_Status` 0x1830d0 — 2/3/6 are
    /// the fail/dead/complete states, Mission_Update 0x182c60); and for exactly one script — the
    /// 07000005 ledge cut-scene 0x06000994 (SP #298, the plate #301 → ch 0x43) — a live combat-music
    /// event (`Get_Music_Event(2) != 0` = `EventList[2]`, posted by `DroneFunc_CheckAlarmRaised`
    /// 0x1453a8 and `NDrone2_SeenAndAttacking` 0x145150) refuses the scene. Refused: state 5 — the
    /// player deletes itself next tick (obj+0xFE |= 1), so the level's other ch-0x43 users run on:
    /// the real play's swap with no cut-scene and no `Drone_EnableAll(0)` freeze (trace
    /// tests/traces/05-gate-swap 174328→174329, the courtyard sniper cycling SNIPER_AIM/FIRE since
    /// 173533). The state-1/3/6 retriggers skip these checks [M 0x1dbda8 cases 1/3/6], as does the
    /// create-time load beat [M 0x1db9a8]. `MPSettings+0x180 == 0` (single player) is the port's
    /// only case.
    pub fn start_allowed(&self, gate: SpStart) -> bool {
        gate.health_ok
            && (gate.mission == 0 || gate.mission == 1)
            && !(self.scripts[self.cur] == 0x0600_0994 && gate.music2 != 0)
    }

    /// `SP_CreateScriptPlayer@0x1db9a8:81-86`'s load beat: mode A ∉ {0, 1} loads the script immediately and
    /// `SP_LoadScript@0x1dbc08:41-55`'s mode dispatch leaves it **primed in state 1 after having run once**
    /// (mode 64's arm is play+update+stop; mode 8 plays looping and leaves state 6) [M]. The game does this
    /// during the level load; the port runs it once at the start of the first `Scripts::begin_tick` [L ee_start_05.bin:
    /// the 07000005 EndLevel SP (script 0x6000A23, mode 64) is primed in state 1 with its clock at 2.0 at level
    /// start, and the disabled looping pair (modes 8, mask 0) already plays].
    fn prime(&mut self, lib: &HashMap<u32, Arc<Nis>>, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>) {
        // Mask 0 (state 6) SPs also load at create in the game — the disabled looping pair on 07000005
        // (0x16ddb40, modes 8,8, mask 0) is loaded and playing behind its `+0x36` freeze [L]. The port has
        // no disabled byte (port-check §9), so priming those would set a frozen-in-the-game script running;
        // only mask ≠ 0 SPs are primed (the live EndLevel case: mask 1, mode 64, state 1, clock 2.0 [L]).
        if self.mask == 0 { return; }
        if self.state != 0 && self.state != 6 { return; }
        let m = self.modes[0];
        if m == 0 || m == 1 { return; }
        self.load(lib, objs, ch, w, frame, out);
    }

    /// `SP_LoadScript` (0x1dbc08).
    fn load(&mut self, lib: &HashMap<u32, Arc<Nis>>, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>) {
        let cur = self.cur;
        match self.info[cur].as_mut() {
            Some(s) => s.rewind(),
            None => {
                let Some(n) = lib.get(&self.scripts[cur]) else { self.state = 5; return };
                self.info[cur] = Some(Script::load(n.clone(), self.matrix, self.scale));
            }
        }
        let mode = self.modes[cur];
        let s = self.info[cur].as_mut().unwrap();
        s.player = self.placement; // for NF_SCRIPTLOG
        match mode {
            1 => { s.start(false); self.state = 2; }
            8 => { s.start(true); self.state = 6; }
            _ => { s.start(false); s.update(&mut self.subs, lib, objs, ch, w, frame, out); s.stop(); self.state = 1; }
        }
        s.update(&mut self.subs, lib, objs, ch, w, frame, out);
    }

    /// `SP_Update` (0x1dbda8). Trigger flags: only the channel bit (1) and activation (4); bullets and touching
    /// objects (2, 8, 0x10, `SP_Hit`) are not modelled yet. `skip` is the game's `Input_Action(-1, 0x19, 1)` —
    /// any player pressed the skip action this tick (`Scripts::skip_request`, set by the viewer's key handler).
    pub fn update(&mut self, lib: &HashMap<u32, Arc<Nis>>, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>, skip: bool, gate: SpStart) {
        if self.deleted || (self.gate != 0 && !ch.on(self.gate as u32)) { return; }
        if let Some(s) = self.info[self.cur].as_mut() {
            s.update(&mut self.subs, lib, objs, ch, w, frame, out);
            // `Script_Interp` 0x1d9a08:106-133 keys the SP object's `obj+0x90` from stream 0 every
            // frame: the keys' matrix (an anim stream: its discrete object 0), each parent stream's
            // matrix, then the script base. The SP's use sphere follows it (`SP_Activate` 0x1dc338
            // reads the live object).
            let s0 = &s.streams[0];
            let m0 = match s0.kind {
                2 => s.stream_matrix(0, objs, w),
                _ => s.keys(0).map(|(m, _, _)| m),
            };
            if let Some(mut m) = m0 {
                let mut par = s.streams[0].parent as i32;
                let mut n = 0;
                while par >= 0 && (par as usize) < s.streams.len() && n < s.streams.len() {
                    if let Some(pm) = s.stream_matrix(par as usize, objs, w) { m = mul(&m, &pm); }
                    par = s.streams[par as usize].parent as i32;
                    n += 1;
                }
                self.live_matrix = mul(&m, &s.matrix);
            }
        }
        let mut flags = 0u16;
        if self.countdown == 0 && self.out != 0 && ch.on(self.out as u32) { flags |= 1; }
        // `SP_Hit` 0x1dc470's bit 0x2 — a class-5 object (an explosion) in one of the current
        // script's stream objects' +0xD0 hit chains, while the countdown byte is 0 [M 0x1dbda8:66
        // gates the SP_Hit call on the countdown]. The game's chain carries the HITDATA the
        // explosion objects appended; the port's analog is the alive-blast list reaching the
        // stream object's own sphere (obj+0x80 centre = `Control_BuildWorldSph` of the glist's
        // local centre +0x10, obj+0x8C radius [M 0x133070:36-40]) — the same sphere shape
        // `Explode_Propagate` 0x17c518 sweeps with [M 0x17c620]. Mask 2 SPs are exactly the ones
        // gated on this bit (07000007's urn SP #312 [D levels.md §5.3]); the mask == 2 arming
        // `Collide_FilterBullets(obj+0xD0, 0x106)` 0x1dc524-0x1dc534 has no port-side chain to
        // arm (the port models no per-object bullet chains) — a listed deviation.
        if self.countdown == 0 && !gate.blasts.is_empty() {
            if let Some(s) = self.info[self.cur].as_ref() {
                'outer: for st in &s.streams {
                    if st.kind != 1 { continue; }
                    let Some(Some(Obj::Entity { hash, matrix, .. })) = st.obj.and_then(|o| objs.list.get(o)) else { continue };
                    let (centre, radius) = match w.glist_sphere(*hash) {
                        Some((c, r)) => ([
                            matrix[0][0] * c[0] + matrix[1][0] * c[1] + matrix[2][0] * c[2] + matrix[3][0],
                            matrix[0][1] * c[0] + matrix[1][1] * c[1] + matrix[2][1] * c[2] + matrix[3][1],
                            matrix[0][2] * c[0] + matrix[1][2] * c[1] + matrix[2][2] * c[2] + matrix[3][2]], r),
                        // No model above: the matrix translation, radius 0 (only a blast AT the
                        // object's origin touches) [S].
                        None => ([matrix[3][0], matrix[3][1], matrix[3][2]], 0.0),
                    };
                    for &(bp, br) in &gate.blasts {
                        let d = ((bp[0] - centre[0]).powi(2) + (bp[1] - centre[1]).powi(2) + (bp[2] - centre[2]).powi(2)).sqrt();
                        if d - radius <= br { flags |= 2; break 'outer; }
                    }
                }
            }
        }
        let mut value = self.out != 0 && ch.on(self.out as u32);
        if self.countdown != 0 { self.countdown -= 1; }
        if self.activated && self.countdown == 0 { flags |= 4; }
        self.activated = false;
        let trig = flags & self.mask != 0;
        match self.state {
            0 if trig => {
                if self.start_allowed(gate) {
                    self.load(lib, objs, ch, w, frame, out);
                    if let Some(s) = self.info[self.cur].as_mut() { s.start(false); }
                    self.state = 2;
                } else {
                    self.state = 5; // [M 0x1dbda8:106 → LAB_001dc0fc; the delete lands next tick]
                }
            }
            1 if trig => {
                self.state = 2;
                self.countdown = 5;
                let looping = self.modes[self.cur] == 8;
                if let Some(s) = self.info[self.cur].as_mut() { s.start(looping); }
                value = true;
            }
            2 => {
                // `SP_UnPause` while the channel is on (mask bit 1).
                if self.mask & 1 != 0 && self.out != 0 && ch.on(self.out as u32) {
                    self.activated = true;
                    if let Some(s) = self.info[self.cur].as_mut() { s.delay = 0; }
                }
                // script-negative-delay [S stand-in, docs/levels/0700004a.md §6]: the Tower-2 lift
                // (SP #42: mask 1, out 0x22 [D placement]) resumes ONLY while ch 0x22 is on — the
                // game's case-2 rule above — and its script HOLDS itself at t1611 (stream 4's
                // event 3 p0 = 0 → the −1 delay [D 0x06000571]). The disc carries no ch-0x22
                // writer the port's scans reach (the 11 scripts' 0xD/0x11 events, the placements,
                // the 13 TouchOnce plates — the same "no writer found" shape as ch 0x4a below),
                // so the port marks the play the doc's reading ("the ch-0x22 SP out marks the
                // play"): the channel rises when the hold engages, and the mask&1 rule then
                // releases the hold through `SP_UnPause` — the game's path, not a delay override.
                if w.level() == 0x0700_004a && self.scripts[self.cur] == 0x0600_0571
                    && self.info[self.cur].as_ref().is_some_and(|s| s.delay < 0) {
                    ch.set(0x22, 1, frame);
                }
                // The skip [M 0x1dbda8:129-141]: `Input_Action(-1, 0x19, 1)` — any player pressing the skip
                // action — fast-forwards the current script via `Script_FFwd(cur, 1)`, unless the script is one
                // of the three death NISs (`Script_IsDeathNIS` reads slot A's hash, always +0x1C).
                if skip {
                    if !Script::is_death_nis(self.scripts[0]) {
                        if let Some(s) = self.info[self.cur].as_mut() { s.ffwd(&mut self.subs, objs, out); }
                    }
                }
                if !self.info[self.cur].as_ref().is_some_and(Script::playing) {
                    // fix-finale-blockers [S, docs/levels/0700004a.md §6]: the Tower-2 lift — the
                    // 0x06000571 script's last key parks the platform entity 150 units down at the
                    // lobby. The game's own ch-0x4a writer ("the lift's lobby arrival") is not in
                    // the data the port reaches (no mux, trigger, script event or SP writes it —
                    // the scans), so the port completes the objective at the lift script's end.
                    if w.level() == 0x0700_004a && self.scripts[self.cur] == 0x0600_0571 {
                        ch.set(0x4a, 1, frame);
                    }
                    if self.stay { self.state = 5; } else {
                        if self.other().is_some() { self.cur ^= 1; self.load(lib, objs, ch, w, frame, out); }
                        self.state = 3;
                    }
                }
            }
            3 if trig => {
                self.state = 4;
                self.countdown = 5;
                let looping = self.modes[self.cur] == 8;
                if let Some(s) = self.info[self.cur].as_mut() { s.start(looping); }
                value = false;
            }
            4 => {
                if !self.info[self.cur].as_ref().is_some_and(Script::playing) {
                    if self.other().is_some() { self.cur ^= 1; self.load(lib, objs, ch, w, frame, out); }
                    self.state = 1;
                }
            }
            5 => {
                // `SP_Delete`: both scripts freed (their kept objects stay).
                for s in self.info.iter_mut().flatten() { s.free(&mut self.subs, objs, out); }
                self.info = [None, None];
                self.deleted = true;
            }
            6 if trig => {
                if let Some(s) = self.info[self.cur].as_mut() { s.delay = 0; }
                if self.other().is_some() {
                    if let Some(mut s) = self.info[0].take() { s.free(&mut self.subs, objs, out); }
                    self.cur ^= 1;
                    self.load(lib, objs, ch, w, frame, out);
                    self.state = 2;
                }
            }
            7 => self.load(lib, objs, ch, w, frame, out),
            _ => {}
        }
        if self.out != 0 && trig { ch.set(self.out as u32, value as u8, frame); }
        // `SP_Update`'s damage tail [M 0x1dc1c0-0x1dc330, after the mode switch and the out-channel
        // write]: a HELD current script (`SCRIPTINFO+0x9c != 0`) on a player with hit points
        // (`SP+0x14 != 0`) adds the tick's bullet damage on its stream objects (`SP_GetHitDamage`
        // 0x1dc5f8 — the port's bullet pass gathered it into `tick_damage`) into `SP+0x18`; when the
        // sum reaches the hit points, `SP_UnPause` 0x1dca58 fires — the one-tick `+0x2c` flag (the
        // port's `activated`, gathered as trigger bit 4 next tick) and `SCRIPTINFO+0x9c = 0` — and
        // the threshold is SUBTRACTED, keeping the remainder [M 0x1dc2f8-0x1dc318]. GAPS G-047: the
        // 0700001b solar panels (param 7 = 20) resume and their stream 1-10 debris plays.
        if let Some(s) = self.info[self.cur].as_ref() {
            if s.delay != 0 && self.hit_points != 0.0 {
                let dmg = std::mem::take(&mut self.tick_damage);
                if dmg != 0.0 {
                    self.damage += dmg;
                    // The game's scenery-shot stat (`PlrStat_LogShotHitScenery(0)` for scripts
                    // 0x60007e2..0x60007ea but 0x60007e3 [M 0x1dc290-0x1dc2c8]) has no port-side
                    // PlrStat to call [*deviation*].
                    if self.hit_points <= self.damage {
                        self.damage -= self.hit_points;
                        self.activated = true;
                        if let Some(s) = self.info[self.cur].as_mut() { s.delay = 0; }
                    }
                }
            }
        }
    }
}

/// The `SP_Update` state-0 start-check inputs [M 0x1dbda8:80-106]: Bond's health gate
/// (BLData+0x894 > 0 [D trace_record.py pokes health there]), the global `MissionState`
/// (0x30cb9c; only 0/1 let a script start), and the tick's view of `EventList[2]` — the
/// combat-music event (`Get_Music_Event` 0x1de350). The game's `UpdateMusicalEvents`
/// (0x114e70, from `SFXUpdate` 0x1156c0 in `Game_Run`'s pre/post passes 0x1c9790) clears the
/// EventList once per frame, so a post made later in the object walk survives into the next
/// walk — the port passes the LAST walk's event here (`Drones::prev_event2`).
#[derive(Clone, Debug, Default)]
pub struct SpStart {
    pub health_ok: bool,
    pub mission: u32,
    pub music2: u32,
    /// fix-script-objects (`SP_Hit` 0x1dc470): the alive blasts of the PREVIOUS tick's end —
    /// (centre, radius). The game's SP_Hit reads the +0xD0 hit chains the explosion objects
    /// filled; the port passes the `Drones::explosions` records instead (same tick order: the
    /// blast spawns at tick T's end, the SP pass reads it at T+1 [M 0x1dbda8:66 — SP_Hit runs
    /// while the countdown byte is 0]). Empty on most ticks.
    pub blasts: Vec<([f32; 3], f32)>,
}

/// The level's script players, their scripts and the objects they made.
#[derive(Clone, Debug, Default)]
pub struct Scripts {
    pub lib: HashMap<u32, Arc<Nis>>,
    pub players: Vec<Player>,
    pub objects: Objects,
    /// Requests of the last tick (sounds, events not carried out).
    pub requests: Vec<Request>,
    /// The create-time load beat (`SP_CreateScriptPlayer`'s immediate `SP_LoadScript` for mode A ∉ {0, 1})
    /// runs once at the first tick — the game does it during the level load [M 0x1db9a8:81-86].
    primed: bool,
    /// The game's `Input_Action(-1, 0x19, 1)` for the next tick — any player pressing the skip action. The
    /// viewer's key handler sets this while a cut-scene is playing (fix-dev T1); consumed every tick.
    pub skip_request: bool,
}

impl Scripts {
    /// fix-script-objects: the live entity matrix of `src` (script hash, stream) — the
    /// script-created AI volumes' per-tick refresh reads it (the game keys obj+0x90 from the
    /// stream every `Script_Interp` 0x1d9a08:106-133). Read-only.
    pub fn entity_matrix(&self, src: (u32, u8)) -> Option<Mat4> {
        self.objects.list.iter().flatten().rev().find_map(|o| match o {
            Obj::Entity { matrix, src: s, .. } if *s == src => Some(*matrix),
            _ => None,
        })
    }

    /// The scripts (`lib`) and the players placed on the main map.
    pub fn new(lib: HashMap<u32, Arc<Nis>>, placements: &[Placement]) -> Scripts {
        let mut players = placements.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xd9).filter_map(|(i, p)| Player::from_placement(i, p)).collect::<Vec<_>>();
        // fix-searchlight-alarm: 07000006's courtyard searchlights. The disc's NIS 0x06000066 stream 0
        // raises the heads' on-channels in a staged reveal — ch 64 (0x40) = 1 at wait 1, ch 63 (0x3F) at
        // 59, ch 61 (0x3D) at 116, ch 62 (0x3E) at 167 [D 07000006 kind-7 NIS 0x06000066; the heads'
        // on-channels are 64/63/61/62 on the four 0xDE kind-1 placements] — all four courtyard lights
        // sweep within ~2.8 s of the script's start. Nothing in the level's
        // placements, triggers or sub-scripts references the script (no param carries the hash; the raw
        // bytes occur nowhere in 07000006.bin outside compressed-data noise), so the game's activation
        // beat is not derivable from the disc — the port boots it once at level start, mode 1 (play
        // once), the way a hidden `SP_CreateScriptPlayer` one-shot runs (`spawn_once`'s precedent, the
        // copter script [M 0x1dbc08's mode dispatch]). Without it the port's courtyard heads stay state
        // 3 (dark) forever, the alarm channel 55 can never rise and the guards never react. The real
        // activation moment (which SP fires it in the game, and on which beat) is a live check in
        // docs/reports/fix-searchlight-alarm.md. Pinned to the level by its own data: a class-0x22 head
        // placement whose alarm channel is 55 AND the script in the lib.
        let light_boot = placements.iter().any(|p| p.flags & 0xff == 0xde
            && p.params.iter().any(|&(k, v)| k == 3 && v == 55))
            && lib.contains_key(&0x0600_0066);
        if light_boot {
            players.push(Player {
                placement: placements.len(), scripts: [0x0600_0066, 0], modes: [1, 0], stay: false, mask: 0, out: 0, gate: 0,
                scale: 1.0, matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                cur: 0, state: 7, activated: false, countdown: 0, info: [None, None], deleted: false,
                // Merge drift (fix-searchlight-alarm × walkreal-b2's field): the identity, like the matrix.
                live_matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                subs: Vec::new(),
                hit_points: 0.0,
                damage: 0.0,
                tick_damage: 0.0,
            });
        }
        Scripts { lib, players, objects: Objects::default(), requests: Vec::new(), primed: false, skip_request: false }
    }

    /// `SP_GetHitDamage` 0x1dc5f8's read set: the current script's stream objects of every player
    /// whose script is HELD (`SCRIPTINFO+0x9c != 0`) on a hit-point player [M SP_Update's damage
    /// tail 0x1dc1c0-0x1dc330 gates the accumulation on exactly that; `Script_GetObj` 0x1db688
    /// walks the records with kind 1/2]. Returns `(player index, world centre, radius)`; the
    /// centre is the entity matrix through the glist sphere's local centre, the radius the
    /// glist's — the game's `Control_BuildWorldSph` volume [M 0x133070:36-40, the same shape the
    /// SP_Hit blast test sweeps]; an anim object keeps its stream matrix's translation with
    /// radius 0 [S] (the port carries no anim volume).
    pub fn held_hit_targets(&self, w: &dyn World) -> Vec<(usize, [f32; 3], f32)> {
        let mut out = Vec::new();
        for (pi, p) in self.players.iter().enumerate() {
            if p.deleted || p.hit_points == 0.0 { continue; }
            let Some(s) = p.info[p.cur].as_ref() else { continue };
            if s.delay == 0 { continue; }
            for i in 0..s.streams.len() {
                if !matches!(s.streams[i].kind, 1 | 2) { continue; }
                let Some(o) = s.streams[i].obj else { continue };
                let Some(Some(obj)) = self.objects.list.get(o) else { continue };
                match obj {
                    Obj::Entity { hash, matrix, .. } => {
                        let (c, r) = w.glist_sphere(*hash).unwrap_or(([0.0; 3], 0.0));
                        let centre = [
                            matrix[0][0] * c[0] + matrix[1][0] * c[1] + matrix[2][0] * c[2] + matrix[3][0],
                            matrix[0][1] * c[0] + matrix[1][1] * c[1] + matrix[2][1] * c[2] + matrix[3][1],
                            matrix[0][2] * c[0] + matrix[1][2] * c[1] + matrix[2][2] * c[2] + matrix[3][2]];
                        out.push((pi, centre, r));
                    }
                    _ => {
                        if let Some(m) = s.stream_matrix(i, &self.objects, w) {
                            out.push((pi, [m[3][0], m[3][1], m[3][2]], 0.0));
                        }
                    }
                }
            }
        }
        out
    }

    /// One bullet contact's damage on a player's stream objects this tick (`Collide_Update`
    /// 0x1ea398 fills the object's +0xD0 HITDATA chain; `SP_GetHitDamage` sums it — the port
    /// gathers per contact in the bullet pass). All damage types count: `SP_Update` calls
    /// `SP_GetHitDamage` with filter 0 (`Collide_FilterBullets` drops nothing at mask 0
    /// [M 0x1ece30:24-31 — only a non-zero mask zeroes a hit's damage]) and
    /// `Collide_GetDamageNObjects` sums every record [M 0x1ee220].
    pub fn add_hit_damage(&mut self, player: usize, dmg: f32) {
        if let Some(p) = self.players.get_mut(player) { p.tick_damage += dmg; }
    }

    /// A synthetic one-shot player (fix-objects2): mode 1 (plays once from spawn), no gate, no
    /// trigger — the game's hidden `SP_Create` inside `Copter_Create` 0x17d340 plays the copter's
    /// flight script the same way once its gate channel opens [M 0x1dbc08's mode dispatch].
    pub fn spawn_once(&mut self, placement: usize, script: u32, matrix: Mat4) {
        self.spawn_once_scaled(placement, script, matrix, 1.0);
    }

    /// `spawn_once` with the script scale: `Explode_Create` (0x17cbd0:106) plays the explosion script
    /// with scale = radius·0.06666667 — the port's `Player.scale` multiplies the key translations and
    /// key scales [M nis.rs keys/interp], the same object field the game's script player carries.
    pub fn spawn_once_scaled(&mut self, placement: usize, script: u32, matrix: Mat4, scale: f32) {
        self.players.push(Player {
            placement, scripts: [script, 0], modes: [1, 0], stay: false, mask: 0, out: 0, gate: 0,
            scale, matrix, cur: 0, state: 7, activated: false, countdown: 0, info: [None, None], deleted: false,
            live_matrix: matrix,
            subs: Vec::new(),
            hit_points: 0.0,
            damage: 0.0,
            tick_damage: 0.0,
        });
    }

    /// Any script player playing a non-looping script right now — the game's skip window (`SP_Update` state 2
    /// with `Script_IsPlaying`). The viewer's key handler asks this before mapping a key press to the skip.
    pub fn cutscene_playing(&self) -> bool {
        self.players.iter().any(|p| p.state == 2 && p.info[p.cur].as_ref().is_some_and(Script::playing))
    }

    /// The live cut-scene camera (the newest): world matrix and vertical field of view.
    pub fn camera(&self) -> Option<(Mat4, f32)> {
        self.objects.list.iter().rev().flatten().find_map(|o| match o { Obj::Camera { matrix, fov, .. } => Some((*matrix, *fov)), _ => None })
    }

    /// The camera object's source stream's key inputs — fix-exact-math-swap3: lets a test recompute the
    /// camera matrix from the NIS keys through its own copy of the game's arithmetic
    /// (`Script_GetInterp` 0x1da158's inputs: the stream clock, the key range, the spline flag).
    pub fn camera_stream_inputs(&self) -> Option<(u32, usize, f32, usize, usize, u16)> {
        let (hash, i) = self.objects.list.iter().rev().flatten().find_map(|o| match o {
            Obj::Camera { src, .. } => Some(*src),
            _ => None,
        })?;
        let s = self.players.iter().find_map(|p| p.info.iter().flatten().find(|s| s.nis.hashcode == hash))?;
        let st = s.streams.get(i as usize)?;
        Some((hash, i as usize, st.clock, st.first_key as usize, st.last_key as usize, st.flags))
    }

    /// The tick head: the requests reset, the game's `Input_Action(-1, 0x19, 1)` skip taken, and the
    /// create-time load beat (`prime`) on the first tick. The players themselves update from the
    /// control walk (`update_player`), in the game's list order — NOT all together here.
    pub fn begin_tick(&mut self, ch: &mut Channels, w: &dyn World, frame: u32) -> bool {
        self.requests.clear();
        let skip = std::mem::take(&mut self.skip_request);
        if !self.primed {
            self.primed = true;
            let mut out = std::mem::take(&mut self.requests);
            for p in &mut self.players { p.prime(&self.lib, &mut self.objects, ch, w, frame, &mut out); }
            self.requests = out;
        }
        skip
    }

    /// One player's `SP_Update`, called from the control walk in the game's list order. The game
    /// runs every object's control func in ONE walk — `control_funcs[obj+0xFF]`, newest first
    /// (`control_add_object_to_list` prepends [M 0x134698]) — so `SP_Update` is interleaved with
    /// the triggers, drones and spawners by placement [M 0x1336f8:54-68]. A script triggered by a
    /// channel an earlier object wrote starts THIS tick and its first commands run inside the same
    /// walk (`SP_LoadScript`'s trailing `Script_Update` [M 0x1dbc08:56]); the objects after it see
    /// event 5's `Drone_EnableAll(0)` before their own control funcs run. (The port used to update
    /// every player together BEFORE the walk — a script triggered by a placement trigger saw the
    /// channel one tick late and the spawners acted a tick before the freeze: the 07000005 playtest
    /// had the gate guards vanish on the touch tick instead of after the wall-hug scene.)
    pub fn update_player(&mut self, i: usize, ch: &mut Channels, w: &dyn World, frame: u32, skip: bool, gate: SpStart) {
        if i >= self.players.len() { return; }
        self.players[i].update(&self.lib, &mut self.objects, ch, w, frame, &mut self.requests, skip, gate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_formats::nis::{Nis, Stream as NisStream};

    /// A no-op `World` for tests: animation length 100, no discrete object 0.
    struct TestWorld;
    impl World for TestWorld {
        fn anim_length(&self, _script: u32) -> Option<u16> { Some(100) }
        fn discrete0(&self, _skin: u32, _script: u32, _frame: f32) -> Option<Mat4> { None }
    }

    /// Event 0xB (`Script_EventHandler` 0x1d94c0:152-158) writes the weapon byte on the PREVIOUS stream's Anim
    /// object — the same "previous stream" pattern as Event 6. Since `event()` borrows `objs` immutably, the
    /// write is deferred to the next `update()` pass via `pending_weapon`.
    #[test]
    fn event_0xb_sets_weapon_on_previous_stream_anim() {
        // Stream 0: Cmd::Anim { script: 0x06000001, skin: 0x0500003A, start: 1 } (op 0x0a, 14 bytes).
        let anim_cmd = {
            let mut v = vec![0x0a];
            v.extend(0x0600_0001u32.to_le_bytes()); // script
            v.extend(0x0500_003au32.to_le_bytes()); // skin
            v.extend(1u32.to_le_bytes());            // start
            v.push(0);                               // padding (command is 14 bytes)
            v
        };
        // Stream 1: Cmd::Event { kind: 0xB, params: [6] } (op 0x12, 8 bytes: op + n + kind + param[0] + pad).
        let event_cmd = {
            let mut v = vec![0x12, 0x01, 0x0b];
            v.extend(6u32.to_le_bytes()); // param[0] = weapon byte 6 (Handgun)
            v.push(0);                     // padding (command is 8 bytes)
            v
        };
        let nis = Arc::new(Nis {
            hashcode: 0x0600_0086,
            camera: false,
            loop_start: 0,
            length: 100,
            streams: vec![
                NisStream { first_key: 0, last_key: 0, commands: anim_cmd },
                NisStream { first_key: 0, last_key: 0, commands: event_cmd },
            ],
            keys: Vec::new(),
        });
        let mut script = Script::load(nis, IDENT, 1.0);
        script.start(false);
        let mut objs = Objects::default();
        let mut ch = Channels::default();
        let mut out = Vec::new();
        let world = TestWorld;
        // Tick 1: stream 0 creates the Anim (weapon=0); stream 1 fires Event 0xB → pending_weapon.
        // Tick 2: update() flushes pending_weapon → Anim.weapon = 6.
        for frame in 0..3 {
            let mut subs = Vec::new(); script.update(&mut subs, &Default::default(), &mut objs, &mut ch, &world, frame, &mut out);
        }
        let weapon = objs.list.iter().flatten().find_map(|o| match o { Obj::Anim { weapon, .. } => Some(*weapon), _ => None });
        assert_eq!(weapon, Some(6), "Event 0xB should set the Anim's weapon byte to 6 (Handgun)");
    }

    /// The eight unhandled opcodes (0x13 fade, 0x14/0x15 sprite, 0x18/0x1A light, 0x1B/0x1D subscript,
    /// 0x1E text) consume their arguments and the stream CONTINUES — the game has handlers for all of them
    /// (`Script_Run` 0x1d8548:80-118) [M; port-check §5]. Before the fix the stream halted (`pc = None`) and
    /// everything after the opcode never ran.
    #[test]
    fn sprite_fade_light_text_opcodes_do_not_halt_the_stream() {
        // Op 0x13 (fade, 4 arg bytes: a float) + op 0x14 (sprite, 14 arg bytes) + op 0x1E (text, 6 arg
        // bytes), then a real event 0xD (channel 0x33 = 1) and a stream end. The old code froze on 0x13.
        // Every command is op + args + 1 trailing byte (`Script_Run` 0x1d8548:120 adds 1 after the handler).
        let mut cmds = vec![0x13, 1, 2, 3, 4, 0];
        cmds.extend([0x14]);
        cmds.extend(0..14);                      // op + 14 args
        cmds.push(0);                            // sprite pad
        cmds.extend([0x1e, 7, 8, 9, 10, 11, 12, 0]); // op + 6 args + pad
        cmds.extend([0x12, 0x02, 0x0d]);         // event: n=2, kind 0xD (channel set)
        cmds.extend(0x33u32.to_le_bytes());      // p0 = channel
        cmds.extend(1u32.to_le_bytes());         // p1 = value
        cmds.push(0);                            // event pad
        cmds.extend([0x04, 0xff, 0xff]);         // stream end, held past the length
        let nis = Arc::new(Nis {
            hashcode: 0x0600_0099,
            camera: false,
            loop_start: 0,
            length: 100,
            streams: vec![NisStream { first_key: 0, last_key: 0, commands: cmds }],
            keys: Vec::new(),
        });
        let mut script = Script::load(nis, IDENT, 1.0);
        script.start(false);
        let mut objs = Objects::default();
        let mut ch = Channels::default();
        let mut out = Vec::new();
        let mut subs = Vec::new(); script.update(&mut subs, &Default::default(), &mut objs, &mut ch, &TestWorld, 1, &mut out);
        assert!(ch.on(0x33), "the event after the fade/sprite/text ops must have fired (the stream must not halt)");
        // Sister: a command truncated inside its args halts the stream (no read past the end).
        let bad = Arc::new(Nis {
            hashcode: 0x0600_009a,
            camera: false,
            loop_start: 0,
            length: 100,
            streams: vec![NisStream { first_key: 0, last_key: 0, commands: vec![0x14, 1, 2] }],
            keys: Vec::new(),
        });
        let mut s2 = Script::load(bad, IDENT, 1.0);
        s2.start(false);
        let mut out2 = Vec::new();
        let mut subs2 = Vec::new(); s2.update(&mut subs2, &Default::default(), &mut objs, &mut ch, &TestWorld, 1, &mut out2);
        assert_eq!(s2.streams[0].pc, None, "sister: a command truncated past the blob halts that stream (no read past the end)");
    }

    /// The cut-scene skip (`Script_FFwd` 0x1dac78 + `Script_Update` 0x1da998's catch-up loop, via `SP_Update`
    /// state 2's `Input_Action(-1, 0x19)` [M 0x1dbda8:129-141]). A camera script (op 0xD arms `+0xAB5` bit 4)
    /// with a skip pressed every tick: the fade runs ≤ 45 ticks, then the remaining frames fast-forward in ONE
    /// `Script_Update`, the script reaches its done bit, and the commands after the skip point (the event) still
    /// ran. A death NIS (0x6000952/0x600098a/0x6000991) refuses the skip in `SP_Update`.
    #[test]
    fn skip_fast_forwards_a_camera_script_but_not_a_death_nis() {
        // One stream: camera start (op 0xD: spline, first, last — 7 bytes total), a long wait, then an event
        // (op 0x12: n=0, kind=4 PlayerDisable) near the end and the end command. length 100.
        let mut cmds = vec![0x0d, 0, 0, 0, 63, 0, 0]; // camera, keys 0..63
        cmds.extend([0x05, 0, 100, 0]);              // wait until 100 (the length) — the game holds the pc there
        cmds.extend([0x12, 0x00, 0x04]);             // event 4 (PlayerDisable), no params
        cmds.push(0);                                 // event pad
        cmds.extend([0x06, 0x64, 0x00, 0x00]);       // done: length (100) <= clock → play |= 2 [M Script_Run :43-49]
        cmds.extend([0x04, 0xff, 0xff]);             // stream end (held)
        let mk = |hash: u32| Arc::new(Nis {
            hashcode: hash, camera: true, loop_start: 0, length: 100,
            streams: vec![NisStream { first_key: 0, last_key: 0, commands: cmds.clone() }],
            keys: Vec::new(),
        });
        let nis = mk(0x0600_0421);
        let mut p = Player { placement: 0, scripts: [0x0600_0421, 0], modes: [1, 0], stay: false, mask: 0, out: 0, gate: 0, scale: 1.0, matrix: IDENT, cur: 0, state: 0, activated: false, countdown: 0, info: [None, None], deleted: false,
            live_matrix: IDENT,
            subs: Vec::new(),
            hit_points: 0.0,
            damage: 0.0,
            tick_damage: 0.0,
        };
        let mut lib = HashMap::new();
        lib.insert(0x0600_0421u32, nis);
        let mut objs = Objects::default();
        let mut ch = Channels::default();
        let world = TestWorld;
        let mut out = Vec::new();
        // State 0 with mask 0... the game needs a trigger; start it the way SP_Update state 0 does.
        p.load(&lib, &mut objs, &mut ch, &world, 0, &mut out);
        p.state = 2;
        if let Some(s) = p.info[0].as_mut() { s.start(false); }
        // Tick to clock 10 with the skip held (the game: the key pressed every tick).
        for frame in 0..10 { p.update(&lib, &mut objs, &mut ch, &world, frame, &mut out, true, crate::nis::SpStart::default()); }
        {
            let s = p.info[0].as_ref().unwrap();
            assert!(s.fade_flags & 1 != 0, "the skip armed (the game's skip latch +0xAB5 bit 0)");
            assert_eq!(s.fade_len, 45, "fade_len = min(100-10, 45) [M Script_FFwd 0x1dac78:48-51]");
            assert!(s.fade_clock > 0.0, "the fade clock runs");
        }
        // Keep mashing: the fade runs up to 45 ticks, then the catch-up loop runs the remaining frames —
        // done well inside 130 ticks.
        for frame in 10..140 { p.update(&lib, &mut objs, &mut ch, &world, frame, &mut out, true, crate::nis::SpStart::default()); }
        {
            let s = p.info[0].as_ref().unwrap();
            assert_eq!(s.play & 2, 2, "the skipped script reached its done bit within ~50 ticks of mashing");
            assert!(s.clock >= 100.0, "the script fast-forwarded to its end (clock {}), not cut short", s.clock);
        }
        // The end-of-script effects: the event after the skip point ran (kind 4 → the PlayerDisable request).
        assert!(out.iter().any(|r| matches!(r, Request::PlayerDisable)), "the event past the skip point still fired (end effects apply)");
        assert_eq!(p.state, 3, "SP_Update state 2 saw the finished script and moved on");

        // Sister: the same script as a death NIS (hash 0x6000952) NEVER skips — `SP_Update` refuses `Script_FFwd`
        // [M 0x1dbda8:134-136] and `Script_Update`'s near-end break keeps it from fast-forwarding.
        lib.insert(0x0600_0952u32, mk(0x0600_0952));
        let mut p2 = Player { placement: 0, scripts: [0x0600_0952, 0], modes: [1, 0], stay: false, mask: 0, out: 0, gate: 0, scale: 1.0, matrix: IDENT, cur: 0, state: 0, activated: false, countdown: 0, info: [None, None], deleted: false,
            live_matrix: IDENT,
            subs: Vec::new(),
            hit_points: 0.0,
            damage: 0.0,
            tick_damage: 0.0,
        };
        let mut objs2 = Objects::default();
        p2.load(&lib, &mut objs2, &mut ch, &world, 0, &mut out);
        p2.state = 2;
        if let Some(s) = p2.info[0].as_mut() { s.start(false); }
        for frame in 0..70 { p2.update(&lib, &mut objs2, &mut ch, &world, frame, &mut out, true, crate::nis::SpStart::default()); }
        let s2 = p2.info[0].as_ref().unwrap();
        assert_eq!(s2.fade_flags & 1, 0, "a death NIS never arms the skip");
        assert!(s2.play & 2 == 0 || s2.clock < 100.0, "the death NIS was not fast-forwarded (clock {} play {:#x})", s2.clock, s2.play);
    }

    /// The held-script rule (GAPS G-047, task 732): a NEGATIVE delay HOLDS — the streams stop,
    /// the clock freezes, the delay does not count down — and a held entity sits at its FIRST
    /// KEY's position [M Script_Update 0x1da9c4-0x1da9dc: `blez` → the epilogue, no store, no
    /// decrement; Script_EntityStart 0x1d8d78 tails with `Script_Interp`, so the object is
    /// placed at creation — the game's held panels never stack at an identity matrix]. Sister:
    /// a positive delay still counts down one per tick and releases the script.
    #[test]
    fn negative_delay_holds_and_the_held_entity_takes_its_first_key() {
        // Stream 0: op 7 creates glist 0x2000A42 at t0 (10-byte head: op, hash, has_parent,
        // parent, spline, type, n... the port reads head[0..5] at pc+5 — head = cmds[pc+5..pc+10]).
        let mut cmds = vec![7u8];
        cmds.extend(0x0200_0A42u32.to_le_bytes());
        cmds.extend([0u8, 0, 0, 2, 0]); // no parent, no spline, type 2, no params
        cmds.push(0);
        cmds.extend([0x04, 0xff, 0xff]); // stream end (held on it)
        // Stream 1: event 3 with p0 = 0 → SCRIPTINFO+0x9c = −1 [M Script_EventHandler 0x1d94c0:100].
        let mut ev = vec![0x12u8, 1, 3];
        ev.extend(0u32.to_le_bytes());
        ev.push(0);
        // One key at t5, position (7, 8, 9), identity rotation [x,y,z,w].
        let key = nf_formats::nis::Key { pos: [7.0, 8.0, 9.0, 1.0], quat: [0.0, 0.0, 0.0, 1.0], scale: 1.0, w24: [0; 4], w28: 0.0, time: 5.0 };
        let mk = |hash: u32| Arc::new(Nis {
            hashcode: hash, camera: false, loop_start: 0, length: 100,
            streams: vec![NisStream { first_key: 0, last_key: 0, commands: cmds.clone() }, NisStream { first_key: 0, last_key: 0, commands: ev.clone() }],
            keys: vec![key],
        });
        let mut lib = HashMap::new();
        lib.insert(0x0600_0832u32, mk(0x0600_0832));
        let mut objs = Objects::default();
        let mut ch = Channels::default();
        let world = TestWorld;
        let mut out = Vec::new();
        let mut p = Player { placement: 0, scripts: [0x0600_0832, 0], modes: [1, 0], stay: false, mask: 0, out: 0, gate: 0, scale: 1.0, matrix: IDENT, cur: 0, state: 7, activated: false, countdown: 0, info: [None, None], deleted: false,
            live_matrix: IDENT, subs: Vec::new(), hit_points: 0.0, damage: 0.0, tick_damage: 0.0 };
        p.load(&lib, &mut objs, &mut ch, &world, 0, &mut out);
        {
            let s = p.info[0].as_ref().unwrap();
            assert_eq!(s.delay, -1, "event 3 with p0 = 0 sets the delay to −1 [M 0x1d94c0:100]");
        }
        // Ten ticks held: the clock must not move and the delay must not count down.
        for frame in 0..10 { p.update(&lib, &mut objs, &mut ch, &world, frame, &mut out, false, crate::nis::SpStart::default()); }
        {
            let s = p.info[0].as_ref().unwrap();
            assert_eq!(s.delay, -1, "a NEGATIVE delay holds without decrementing [M 0x1da9d0 blez → the epilogue]");
            assert_eq!(s.clock, 1.0, "the held script's clock froze at the load beat (1.0), not {} — the streams do not run", s.clock);
        }
        // The held entity sits at its first key's position — created ONCE with Script_Interp's
        // placement [M Script_EntityStart 0x1d8d78 tail], never the identity matrix.
        let idx = objs.list.iter().position(|o| matches!(o.as_ref(), Some(Obj::Entity { hash: 0x0200_0A42, .. }))).expect("the held script's entity exists");
        let Some(Some(Obj::Entity { matrix, .. })) = objs.list.get(idx) else { unreachable!() };
        let t = (matrix[3][0], matrix[3][1], matrix[3][2]);
        assert!((t.0 - 7.0).abs() < 0.01 && (t.1 - 8.0).abs() < 0.01 && (t.2 - 9.0).abs() < 0.01,
            "the held entity takes its FIRST KEY's position (7, 8, 9), got {t:?} — not the identity/origin");

        // Sister: a positive delay counts down. p0 = 5 → delay 4 → 3 → 2 → 1 → the streams run.
        let mut ev5 = vec![0x12u8, 1, 3];
        ev5.extend(5u32.to_le_bytes());
        ev5.push(0);
        lib.insert(0x0600_0833u32, Arc::new(Nis {
            hashcode: 0x0600_0833, camera: false, loop_start: 0, length: 100,
            streams: vec![NisStream { first_key: 0, last_key: 0, commands: cmds.clone() }, NisStream { first_key: 0, last_key: 0, commands: ev5.clone() }],
            keys: vec![nf_formats::nis::Key { pos: [1.0, 2.0, 3.0, 1.0], quat: [0.0, 0.0, 0.0, 1.0], scale: 1.0, w24: [0; 4], w28: 0.0, time: 0.0 }],
        }));
        let mut objs2 = Objects::default();
        let mut p2 = Player { placement: 0, scripts: [0x0600_0833, 0], modes: [1, 0], stay: false, mask: 0, out: 0, gate: 0, scale: 1.0, matrix: IDENT, cur: 0, state: 7, activated: false, countdown: 0, info: [None, None], deleted: false,
            live_matrix: IDENT, subs: Vec::new(), hit_points: 0.0, damage: 0.0, tick_damage: 0.0 };
        p2.load(&lib, &mut objs2, &mut ch, &world, 0, &mut out);
        {
            let s = p2.info[0].as_ref().unwrap();
            assert_eq!(s.delay, 4, "event 3 with p0 = 5 sets the delay to p0 − 1 = 4 [M 0x1d94c0:100]");
            assert_eq!(s.clock, 1.0, "the load beat ran once");
        }
        // The stream object's matrix at t0 IS placed (the first key, time 0) — then the delay
        // counts down without the streams running.
        for frame in 0..4 { p2.update(&lib, &mut objs2, &mut ch, &world, frame, &mut out, false, crate::nis::SpStart::default()); }
        {
            let s = p2.info[0].as_ref().unwrap();
            assert_eq!(s.delay, 0, "the positive delay counted down 4 → 0 over four ticks [M 0x1da9d8 sw $v0,156($s0)]");
            assert_eq!(s.clock, 1.0, "the skipped ticks never advanced the clock");
        }
        p2.update(&lib, &mut objs2, &mut ch, &world, 5, &mut out, false, crate::nis::SpStart::default());
        {
            let s = p2.info[0].as_ref().unwrap();
            assert!(s.clock > 1.0, "a zero delay runs the streams again (clock {})", s.clock);
        }
    }
}
