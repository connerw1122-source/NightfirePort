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
    Anim { skin: u32, script: u32, frame: f32, weapon: u8, src: (u32, u8) },
    /// A cut-scene camera (`CAM_INFO`, `Script_CameraStart`): its world matrix (rows: right, up, forward, position)
    /// and vertical field of view in radians (key scale × `+0x114` = 4/3, `Camera_CalcViewAngles`).
    Camera { matrix: Mat4, fov: f32, src: (u32, u8) },
}

/// A request the script made this tick for the caller to carry out.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// `Sound_Play` (2-D) or `Sound_Play3D` at object `at` (an index into `Scripts::objects`), from stream `src`
    /// (script hash, stream): `Script_Interp` case 5 moves it with that object every frame.
    Sound { id: u32, at: Option<usize>, src: (u32, u8) },
    /// The stream's sound ends (`Script_SoundEnd` / `Script_KillStream` → `Sound_Stop`).
    SoundStop { src: (u32, u8) },
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
}

/// What the runtime needs from the game: animation lengths and an animated object's discrete object 0 (the world
/// matrix of the skin's first rigid part's bone).
pub trait World {
    fn anim_length(&self, script: u32) -> Option<u16>;
    fn discrete0(&self, skin: u32, script: u32, frame: f32) -> Option<Mat4>;
    /// The running level (fix-finale-blockers: the 0700004a lift stand-in keys on it).
    fn level(&self) -> u32 { 0 }
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
    pub fn ffwd(&mut self, objs: &mut Objects, out: &mut Vec<Request>) {
        if self.fade_flags & 0x10 == 0 || self.fade_flags & 1 != 0 {
            // No camera (or a skip already armed): the game only clears play bit 4 (0x1dac78:86-88).
            self.play &= !0x10;
            return;
        }
        if self.flags & 0x20 != 0 {
            // Event 0x10 (no cameras): kill the camera stream(s) and return — no fast-forward [M :32-47].
            for i in 0..self.streams.len() { if self.streams[i].kind == 4 { self.kill(i, objs); } }
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
    }

    /// `Script_Update` (0x1da998): every stream's `Script_Run`, then the clocks advance one tick. The game's
    /// do-while [M 0x1da998:19-100] loops while the skip fade is past its length (`fade_clock > fade_len`) and
    /// the script is still playing — that IS the skip's fast-forward: the remaining script (streams, commands,
    /// events, end) runs to completion inside this one call, then the final block fades back in [M :101-110].
    pub fn update(&mut self, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>) {
        // Flush Event 0xB's deferred weapon writes before any early-return: even if the script has since stopped
        // or is waiting on a delay, the Anim object's weapon byte must reflect the event the script already fired.
        if !self.pending_weapon.is_empty() {
            for (o, wp) in self.pending_weapon.drain(..) {
                if let Some(Some(Obj::Anim { weapon, .. })) = objs.list.get_mut(o) { *weapon = wp; }
            }
        }
        if self.play & 1 == 0 { return; }
        // `Script_Update` 0x1da998:17-18,112-114: the streams run while the delay is 0; only a
        // POSITIVE delay decrements and skips the tick — a negative one (event 3's `p0 − 1` with
        // p0 = 0, the lift script's 0x06000571 t1611 [D]) runs like 0 in the game. The port
        // blocked on it forever, so the lift never finished (fix-finale-blockers).
        if self.delay > 0 { self.delay -= 1; return; }
        if !self.playing() { return; }
        // The game's loop guard is only `Script_IsPlaying`; a looping script (play & 8) would spin forever, and
        // the data never skips one — bound the iterations (a guard, not a behaviour change: a non-looping script
        // finishes within `length` iterations because op 6 fires at clock >= length [M Script_Run :43-49]).
        let max_passes = 2 * self.nis.length as usize + 256;
        for _ in 0..max_passes {
            for i in 0..self.streams.len() { self.run(i, objs, ch, w, frame, out); }
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
                let _ = no_fade; // the SetFade call is view-side; the branch only gates it in the game
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
        // Final block [M :101-110]: once the fade is past its length and the script stopped, fade back in
        // (`Camera_SetFade(−9, 0, 0xff, 1)`, view-side), clear the fade clock, play bit 4 and the skip latch.
        if self.fade_clock > self.fade_len as f32 && !self.playing() {
            self.fade_clock = 0.0;
            self.play &= !0x10;
            self.fade_flags &= !1;
        }
    }

    /// `Script_Run` (0x1d8548).
    fn run(&mut self, i: usize, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>) {
        let len = self.nis.length as f32;
        let nis = self.nis.clone();
        let cmds = &nis.streams[i].commands;
        // A looping stream whose end sits before the script length would spin; the data never does it.
        for _ in 0..64 {
            if self.streams[i].pc.is_none() { return; }
            if self.streams[i].clock <= len { self.interp(i, objs, w); }
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
                        self.entity_start(i, hash, has_parent != 0, parent as i8, spline != 0, obj_type, params, objs, w);
                    }
                    9 => {
                        // `Script_EntityEnd` (0x1dafe8).
                        let s = &self.streams[i];
                        if s.obj.is_some() && (s.clock < len || self.play & 8 == 0 || s.tag != 1) { self.kill(i, objs); }
                        next = pc + 2;
                    }
                    0x0a => {
                        let (Some(script), Some(skin), Some(start)) = (rd(0, 4), rd(4, 4), rd(8, 4)) else { self.streams[i].pc = None; return };
                        self.anim_start(i, script, skin, start, objs, w);
                        next = pc + 14;
                    }
                    0x0c => { self.kill(i, objs); next = pc + 2; }
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
                        let s = &mut self.streams[i];
                        s.tag = stream as u16;
                        s.kind = 5;
                        let at = self.streams.get(stream).filter(|o| matches!(o.kind, 1 | 2)).and_then(|o| o.obj);
                        out.push(Request::Sound { id, at: if b[0] == 1 { None } else { at }, src: (self.nis.hashcode, i as u8) });
                        next = pc + 9;
                    }
                    0x17 => {
                        let s = &mut self.streams[i];
                        if s.kind == 5 { s.kind = 0; out.push(Request::SoundStop { src: (self.nis.hashcode, i as u8) }); }
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
                            self.interp(i, objs, w);
                        }
                    }
                    0x0f => {
                        // `Script_CameraEnd`.
                        if self.streams[i].clock < len || self.play & 8 == 0 { self.kill(i, objs); }
                        self.fade_flags &= !0x10; // the camera-active flag clears [M 0x1db348:21]
                        next = pc + 2;
                    }
                    0x13 | 0x14 | 0x15 | 0x18 | 0x1a | 0x1b | 0x1d | 0x1e => {
                        // The game has handlers for every one of these (`Script_Run` 0x1d8548:80-118 dispatches
                        // 0x13 FadeStart, 0x14/0x15 sprite start/end, 0x18/0x1A light start/end, 0x1B/0x1D
                        // nested-subscript start/end, 0x1E text start) and each handler consumes its arguments
                        // from the pc; `Script_Run` then adds the opcode's byte and the common +1 (line 120).
                        // Argument BYTES, counted from the handlers' BIN_Get calls [M: FadeStart 001db3d0 1×
                        // GetFloat = 4; SpriteStart 001d9300 5×GetWord + GetDWord = 14; SpriteEnd/LightEnd/
                        // SubScriptEnd 001db3b0/001db298/001db148 no reads = 0; LightStart 001db1b0 3×
                        // GetByte = 3; SubScriptStart 001db050 GetDWord + 2×GetByte = 6; TextStart 001db2b8
                        // GetDWord + GetWord = 6] — the atlas's "N reads" column mixed call counts with bytes
                        // and was corrected. The port does not model sprites, lights, subtitles, nested
                        // scripts or the fade (view/audio-side objects), so the command is consumed and the
                        // stream CONTINUES — the old placeholder halted the stream from the opcode onward
                        // (port-check §5).
                        let arg = match op { 0x13 => 4, 0x14 => 14, 0x15 | 0x1a | 0x1d => 0, 0x18 => 3, 0x1b => 6, 0x1e => 6, _ => 0 };
                        if cmds.len() < pc + 2 + arg { self.streams[i].pc = None; return; }
                        out.push(Request::Event { kind: 0x80 | op, params: Vec::new() });
                        next = pc + 2 + arg;
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
    fn entity_start(&mut self, i: usize, hash: u32, has_parent: bool, parent: i8, spline: bool, obj_type: u8, params: Vec<u32>, objs: &mut Objects, w: &dyn World) {
        let s = &mut self.streams[i];
        // An object of another type is deleted; one of the same type takes the new glist.
        if let Some(o) = s.obj { if s.obj_type != obj_type { objs.list[o] = None; s.obj = None; } }
        if s.obj.is_none() { s.tag = s.clock as i16 as u16 + 1; }
        let e = Obj::Entity { hash, kind: obj_type, params, matrix: IDENT, alpha: 1.0, src: (self.nis.hashcode, i as u8) };
        match s.obj {
            Some(o) => objs.list[o] = Some(e),
            None => { objs.list.push(Some(e)); s.obj = Some(objs.list.len() - 1); }
        }
        s.parent = if has_parent { parent } else { -1 };
        s.kind = 1;
        s.flags = if spline { 2 } else { 0 };
        s.obj_type = obj_type;
        self.interp(i, objs, w);
    }

    /// `Script_AnimStart` (0x1d8f40).
    fn anim_start(&mut self, i: usize, script: u32, skin: u32, start: u32, objs: &mut Objects, w: &dyn World) {
        if self.streams[i].obj.is_some() { self.kill(i, objs); }
        let len = w.anim_length(script).unwrap_or(1).max(1) as f32;
        let f = start as f32;
        let frame = if f < 1.0 || len < f { if f < 1.0 { 1.0 } else { len } } else { f };
        objs.list.push(Some(Obj::Anim { skin, script, frame, weapon: 0, src: (self.nis.hashcode, i as u8) }));
        let s = &mut self.streams[i];
        s.obj = Some(objs.list.len() - 1);
        s.kind = 2;
        s.start_clock = s.clock;
        s.start_frame = frame;
    }

    /// `Script_KillStream` (0x1d9860): an entity or animated object is deleted unless the script keeps its objects
    /// (flag 0x10: only detached); a sound stops.
    fn kill(&mut self, i: usize, objs: &mut Objects) {
        let keep = self.flags & 0x10 != 0;
        let s = &mut self.streams[i];
        match s.kind {
            1 | 2 => {
                let Some(o) = s.obj else { if s.kind == 1 { s.kind = 0; } return };
                if !keep { objs.list[o] = None; }
            }
            // The camera pops back to the game's.
            4 => if let Some(o) = s.obj { objs.list[o] = None; self.pending.push(Request::Camera { script: self.nis.hashcode, start: false }); },
            _ => {}
        }
        s.kind = 0;
        s.obj = None;
    }

    /// `Script_Free`'s stream pass (`Script_Free` 0x1da880).
    pub fn free(&mut self, objs: &mut Objects, out: &mut Vec<Request>) {
        for i in 0..self.streams.len() { self.kill(i, objs); }
        out.append(&mut self.pending);
        // Flag 0x80 → `Drone_EnableAll(1, hash)` [M Script_Free 0x1da880:41-43]: the cut-scene's guards come back
        // when the script that froze them (event 5 → `NPCGlobals = 0`) is freed and asked for it (event 7).
        if self.flags & 0x80 != 0 { out.push(Request::DroneEnable { on: true }); }
        if self.flags & 0x100 != 0 { out.push(Request::PlayerEnable { at: self.player_at, mode: self.player_mode }); }
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
            _ => out.push(Request::Event { kind, params: p }),
        }
    }

    /// `Script_Interp` (0x1d9a08) for entity and animation streams.
    fn interp(&mut self, i: usize, objs: &mut Objects, w: &dyn World) {
        let s = &self.streams[i];
        let Some(o) = s.obj else { return };
        match s.kind {
            1 => {
                let Some((mut m, scale, alpha)) = self.keys(i) else { return };
                if scale != 1.0 { for r in m.iter_mut().take(3) { for v in r.iter_mut().take(3) { *v *= scale; } } }
                // Each parent stream's matrix, then the script's.
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
                // *suggested*: the decomp loses that argument) and the field of view.
                // The skip's freeze: `Script_Interp` returns early while `+0xAB5 & 0x20` [M 0x1d9a08:205].
                if self.fade_flags & 0x20 != 0 { return; }
                let Some((m, scale, _)) = self.keys(i) else { return };
                let m = mul(&m, &self.matrix);
                if let Some(Some(Obj::Camera { matrix, fov, .. })) = objs.list.get_mut(o) { *matrix = m; *fov = scale * (4.0 / 3.0); }
            }
            2 => {
                // The skip's freeze: `Script_Interp` case 2 returns while `+0xAB5 & 0x20` [M 0x1d9a08:146].
                if self.fade_flags & 0x20 != 0 { return; }
                let f = s.clock - s.start_clock + s.start_frame;
                if let Some(Some(Obj::Anim { script, frame, .. })) = objs.list.get_mut(o) {
                    let len = w.anim_length(*script).unwrap_or(1).max(1) as f32;
                    *frame = f.floor().clamp(1.0, len);
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
        if self.scale != 1.0 { for v in m[3].iter_mut().take(3) { *v *= self.scale; } }
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
        })
    }

    fn other(&self) -> Option<u32> { Some(self.scripts[self.cur ^ 1]).filter(|&h| h != 0) }

    /// `SP_CreateScriptPlayer@0x1db9a8:81-86`'s load beat: mode A ∉ {0, 1} loads the script immediately and
    /// `SP_LoadScript@0x1dbc08:41-55`'s mode dispatch leaves it **primed in state 1 after having run once**
    /// (mode 64's arm is play+update+stop; mode 8 plays looping and leaves state 6) [M]. The game does this
    /// during the level load; the port runs it once at the start of the first `Scripts::tick` [L ee_start_05.bin:
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
            _ => { s.start(false); s.update(objs, ch, w, frame, out); s.stop(); self.state = 1; }
        }
        s.update(objs, ch, w, frame, out);
    }

    /// `SP_Update` (0x1dbda8). Trigger flags: only the channel bit (1) and activation (4); bullets and touching
    /// objects (2, 8, 0x10, `SP_Hit`) are not modelled yet. `skip` is the game's `Input_Action(-1, 0x19, 1)` —
    /// any player pressed the skip action this tick (`Scripts::skip_request`, set by the viewer's key handler).
    pub fn update(&mut self, lib: &HashMap<u32, Arc<Nis>>, objs: &mut Objects, ch: &mut Channels, w: &dyn World, frame: u32, out: &mut Vec<Request>, skip: bool) {
        if self.deleted || (self.gate != 0 && !ch.on(self.gate as u32)) { return; }
        if let Some(s) = self.info[self.cur].as_mut() { s.update(objs, ch, w, frame, out); }
        let mut flags = 0u16;
        if self.countdown == 0 && self.out != 0 && ch.on(self.out as u32) { flags |= 1; }
        let mut value = self.out != 0 && ch.on(self.out as u32);
        if self.countdown != 0 { self.countdown -= 1; }
        if self.activated && self.countdown == 0 { flags |= 4; }
        self.activated = false;
        let trig = flags & self.mask != 0;
        match self.state {
            0 if trig => {
                self.load(lib, objs, ch, w, frame, out);
                if let Some(s) = self.info[self.cur].as_mut() { s.start(false); }
                self.state = 2;
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
                // The skip [M 0x1dbda8:129-141]: `Input_Action(-1, 0x19, 1)` — any player pressing the skip
                // action — fast-forwards the current script via `Script_FFwd(cur, 1)`, unless the script is one
                // of the three death NISs (`Script_IsDeathNIS` reads slot A's hash, always +0x1C).
                if skip {
                    if !Script::is_death_nis(self.scripts[0]) {
                        if let Some(s) = self.info[self.cur].as_mut() { s.ffwd(objs, out); }
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
                for s in self.info.iter_mut().flatten() { s.free(objs, out); }
                self.info = [None, None];
                self.deleted = true;
            }
            6 if trig => {
                if let Some(s) = self.info[self.cur].as_mut() { s.delay = 0; }
                if self.other().is_some() {
                    if let Some(mut s) = self.info[0].take() { s.free(objs, out); }
                    self.cur ^= 1;
                    self.load(lib, objs, ch, w, frame, out);
                    self.state = 2;
                }
            }
            7 => self.load(lib, objs, ch, w, frame, out),
            _ => {}
        }
        if self.out != 0 && trig { ch.set(self.out as u32, value as u8, frame); }
    }
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
    /// The scripts (`lib`) and the players placed on the main map.
    pub fn new(lib: HashMap<u32, Arc<Nis>>, placements: &[Placement]) -> Scripts {
        let players = placements.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xd9).filter_map(|(i, p)| Player::from_placement(i, p)).collect();
        Scripts { lib, players, objects: Objects::default(), requests: Vec::new(), primed: false, skip_request: false }
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

    pub fn tick(&mut self, ch: &mut Channels, w: &dyn World, frame: u32) {
        self.requests.clear();
        let skip = std::mem::take(&mut self.skip_request);
        if !self.primed {
            self.primed = true;
            let mut out = std::mem::take(&mut self.requests);
            for p in &mut self.players { p.prime(&self.lib, &mut self.objects, ch, w, frame, &mut out); }
            self.requests = out;
        }
        for p in &mut self.players { p.update(&self.lib, &mut self.objects, ch, w, frame, &mut self.requests, skip); }
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
            script.update(&mut objs, &mut ch, &world, frame, &mut out);
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
        script.update(&mut objs, &mut ch, &TestWorld, 1, &mut out);
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
        s2.update(&mut objs, &mut ch, &TestWorld, 1, &mut out2);
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
        let mut p = Player { placement: 0, scripts: [0x0600_0421, 0], modes: [1, 0], stay: false, mask: 0, out: 0, gate: 0, scale: 1.0, matrix: IDENT, cur: 0, state: 0, activated: false, countdown: 0, info: [None, None], deleted: false };
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
        for frame in 0..10 { p.update(&lib, &mut objs, &mut ch, &world, frame, &mut out, true); }
        {
            let s = p.info[0].as_ref().unwrap();
            assert!(s.fade_flags & 1 != 0, "the skip armed (the game's skip latch +0xAB5 bit 0)");
            assert_eq!(s.fade_len, 45, "fade_len = min(100-10, 45) [M Script_FFwd 0x1dac78:48-51]");
            assert!(s.fade_clock > 0.0, "the fade clock runs");
        }
        // Keep mashing: the fade runs up to 45 ticks, then the catch-up loop runs the remaining frames —
        // done well inside 130 ticks.
        for frame in 10..140 { p.update(&lib, &mut objs, &mut ch, &world, frame, &mut out, true); }
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
        let mut p2 = Player { placement: 0, scripts: [0x0600_0952, 0], modes: [1, 0], stay: false, mask: 0, out: 0, gate: 0, scale: 1.0, matrix: IDENT, cur: 0, state: 0, activated: false, countdown: 0, info: [None, None], deleted: false };
        let mut objs2 = Objects::default();
        p2.load(&lib, &mut objs2, &mut ch, &world, 0, &mut out);
        p2.state = 2;
        if let Some(s) = p2.info[0].as_mut() { s.start(false); }
        for frame in 0..70 { p2.update(&lib, &mut objs2, &mut ch, &world, frame, &mut out, true); }
        let s2 = p2.info[0].as_ref().unwrap();
        assert_eq!(s2.fade_flags & 1, 0, "a death NIS never arms the skip");
        assert!(s2.play & 2 == 0 || s2.clock < 100.0, "the death NIS was not fast-forwarded (clock {} play {:#x})", s2.clock, s2.play);
    }
}
