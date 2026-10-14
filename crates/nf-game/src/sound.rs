//! The EE sound layer (`Sound_*`, ACTION.ELF 0x1dcc68–0x1de3a0): the `DYNAMICSOUNDS` pool, deferred starts and the
//! per-frame update that drives the IOP driver (`SFX.IRX`, ported in nf-audio). Spec: docs/systems/sound.md "EE".
//!
//! `Sound_Play` / `Sound_Play3D` only take a node (state 0); the next `Sound_UpdateSounds` sends the start (state 1),
//! then every frame the position of a playing 3-D sound; a one-shot dies when the driver no longer reports it playing
//! (state 2 → remove → free), a looped one only by `Sound_Stop`. Per-sound volume and pitch are stored by the game but
//! never reach the driver (it forces volume 100; pitch is never read) [M].

use crate::world::V3;
use nf_formats::sfx::SfxTable;

/// A handle: the node index + 1 (0 = none). *Deviation*: the game's handle is the node pointer.
pub type Handle = u32;

/// What the driver is asked to do (the batch commands `SFXStart`, `SFXStart3D`, `SFXSetPositionAndVelocity`,
/// `SFXRemove`, `SFXSetupListener`).
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    /// `SFXStart(id, handle)` (2-D) or `SFXStart3D(id, pos, 0, handle, volume, inner, outer)`; radii only for
    /// "custom" sounds (map ambients), else the bank's.
    Start { handle: Handle, id: u32, pos: Option<V3>, radii: Option<(f32, f32)> },
    Position { handle: Handle, pos: V3 },
    Remove { handle: Handle },
    /// The listener: eye position and the camera's row 0 (`Norm`, the screen-left axis), up and forward rows.
    Listener { pos: V3, norm: V3, up: V3, dir: V3 },
}

#[derive(Clone, Debug)]
struct Node {
    id: u32,
    pos: V3,
    is3d: bool,
    custom: Option<(f32, f32)>,
    state: u8,
    age: u32,
    /// Ticks since the start was sent (the driver's grace period before it can report the end).
    since_start: u32,
}

/// The pool (`DynamicSoundList`, 256 nodes).
#[derive(Clone, Debug, Default)]
pub struct Sounds {
    nodes: Vec<Option<Node>>,
    /// Commands for the driver since the last `take_commands`.
    pub out: Vec<Cmd>,
    /// The listener position of the last update (`GlobalListenerPos`).
    pub listener: V3,
    /// The head-under-water state of the previous tick (`Player_MonitorAir` 0x191a70:101-124's edge —
    /// the dive loop 0x27 starts on under-entry and stops with the surface gasp 0x60c on under-exit).
    /// Bookkeeping lives here so the scene-side hook needs no new Sim state.
    pub swim_prev_under: bool,
}

pub const POOL: usize = 256;

/// `SFXIsSFXPlaying` reports a just-started sound as playing for 5 VBlanks (the request is still in flight) [M].
pub const START_GRACE: u32 = 5;

/// Map ambient sounds (`HandleMapSoundAllocation` 0x1dd118, run by `Sound_UpdateListeners` before `Sound_UpdateSounds`):
/// a sound starts when the listener is inside its radius R (`Sound_Play3D(id, pos, 100, R, inner, …, custom 1)`) and
/// stops when the squared distance exceeds 1.2 · R².
#[derive(Clone, Debug, Default)]
pub struct MapSounds { pub list: Vec<nf_formats::level::MapSound>, active: Vec<Option<Handle>> }

impl MapSounds {
    pub fn new(list: Vec<nf_formats::level::MapSound>) -> MapSounds { let n = list.len(); MapSounds { list, active: vec![None; n] } }

    pub fn update(&mut self, sounds: &mut Sounds, listener: V3) {
        for (k, m) in self.list.iter().enumerate() {
            let d2 = (0..3).map(|i| (m.pos[i] - listener[i]).powi(2)).sum::<f32>();
            match self.active[k] {
                None => if d2 < m.radius * m.radius {
                    let custom = (m.inner > 0.0 || m.radius > 0.0).then_some((m.inner, m.radius));
                    self.active[k] = Some(sounds.play3d(m.id, m.pos, custom));
                },
                Some(h) => if m.radius * m.radius * 1.2 < d2 { sounds.stop(h); self.active[k] = None; },
            }
        }
    }
}

/// The sound-trigger objects (`SoundTrigger_Create` 0x1b2628, placement kind 0xF9 `oSoundTrigger`;
/// `SoundTrigger_Update` 0x1b2790): level-placed emitters that play one sound while a switch channel
/// is up. The placement's params read sequentially, param k at record +0x2c+4k — the layout the
/// searchlight's channels pin [D 07000006 placements 2/4/6: on/off/alarm = params 1/2/3]. So:
/// p0 = the ON channel, p1 = the one-shot flag, p2 = the sound id (0/0xffff → no object), p3 = the
/// 2-D flag (else `Sound_Play3D` at the placement), p4 = the OFF channel, p5 = the
/// `GlobalVoiceHandle` claim; the keep-when-ON-clears byte is the record's +0x48 low byte — the
/// placement's param count [D 07000006's four triggers all carry 8 params → disabled].
/// 07000006 placement 702 is the courtyard alarm siren: ON ch 55 (the searchlights' alarm write
/// [M Searchlight_Update 0x1cb900:142-144]), id 329 (0x149: LOOPED, inner 400/outer 500 [D
/// SfxTable]), 2-D, OFF ch 177 — the klaxon the user never heard (playtest 1013-1825). The other
/// three (ch 47 → 219, ch 57 → 241, ch 103 → 331, all one-shot non-looped) are the same object type.
#[derive(Clone, Debug)]
pub struct SoundTrigger {
    pub on: u16,
    pub off: u16,
    pub one_shot: u16,
    pub id: u16,
    pub is2d: u16,
    pub global: bool,
    pub keep_on: u8,
    pub pos: V3,
    handle: Handle,
}

/// The level's sound triggers + the single `GlobalVoiceHandle` slot (0 = free): a global trigger
/// starts only while the slot is free and holds it until it stops or finishes; a global trigger
/// that finds it busy kills itself [M 0x1b2790:19-28; 0x1b2628:14 resets the slot per create].
#[derive(Clone, Debug, Default)]
pub struct SoundTriggers { pub list: Vec<SoundTrigger>, global_voice: Handle }

impl SoundTriggers {
    /// `SoundTrigger_Create` for every 0xF9 placement of the main map, in placement order [M
    /// parsemap_create_dynamic_objects 0x1d04f0 case 0xf9, from ResetMap_Load 0x1d6360 whose
    /// create gate is 1; the 0xa000-flagged placements never reach the dispatcher (its caller
    /// 0x1d1b68 skips them)]. The difficulty gate is the record's +0x44 (param 6): skip when it
    /// is below the difficulty, or 5/6/7 without difficulty 1/2/3 [M 0x1b2628:24-40].
    pub fn create(placements: &[nf_formats::placement::Placement], difficulty: u32) -> SoundTriggers {
        let mut st = SoundTriggers { list: Vec::new(), global_voice: 0 };
        for p in placements {
            if p.flags & 0xff != 0xf9 || p.flags & 0xa000 != 0 { continue; }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
            let (on, one_shot, id, is2d, off, global) =
                (param(0), param(1), param(2), param(3), param(4), param(5));
            if id == 0 || id == 0xffff || on == 0 { continue; }
            let gate = p.extra[8];
            if gate != 0 && gate < difficulty || matches!(gate, 5 | 6 | 7) && difficulty != gate - 4 { continue; }
            st.list.push(SoundTrigger {
                on: on as u16, off: off as u16, one_shot: one_shot as u16, id: id as u16,
                is2d: is2d as u16, global: global & 0xff != 0, keep_on: p.params.len() as u8,
                pos: p.translation, handle: 0,
            });
        }
        st
    }

    /// `SoundTrigger_Update`, once per frame. Killed objects (`obj+0xfe` bit 0 →
    /// `control_delete_object` [M 0x1336f8:72-89]) leave the list.
    pub fn tick(&mut self, audio: &mut Sounds, channels: &crate::triggers::Channels) {
        self.list.retain_mut(|t| {
            if t.handle == 0 {
                // Not playing: the OFF channel risen → the object kills itself [M 0x1b2790:15,
                // 44→83-84]; the ON channel low → idle [M 0x1b2790:16-17].
                if channels.on(t.off as u32) { return false; }
                if !channels.on(t.on as u32) { return true; }
                // A global trigger needs the free voice slot, else it dies [M 0x1b2790:19-28].
                if t.global && self.global_voice != 0 { return false; }
                t.handle = if t.is2d != 0 { audio.play(t.id as u32) } else { audio.play3d(t.id as u32, t.pos, None) };
                if t.global && t.handle != 0 { self.global_voice = t.handle; }
                return true;
            }
            if audio.finished(t.handle) {
                // Finished on its own: the voice slot frees; a one-shot dies [M 0x1b2790:71-80].
                t.handle = 0;
                if t.global { self.global_voice = 0; }
                return t.one_shot != 1;
            }
            // Still playing: stop on the OFF channel (only when it is a real channel), or when the
            // ON channel clears and the keep byte is 0 [M 0x1b2790:49-59].
            let stop = channels.on(t.off as u32) || (t.keep_on == 0 && !channels.on(t.on as u32));
            if !stop { return true; }
            audio.stop(t.handle);
            t.handle = 0;
            if t.global { self.global_voice = 0; return false; }
            t.one_shot != 1
        });
    }
}

/// `Sound_Ready` (0x1dceb8): the bank hash a level loads (`SFXLoadSoundBank`).
pub fn level_bank(level: u32) -> Option<u32> {
    Some(match level {
        0x0700_0001 => 1, 0x0700_0002 => 2, 0x0700_0003 => 3, 0x0700_0004 => 8, 0x0700_0005 => 5, 0x0700_0006 => 4,
        0x0700_0007 => 6, 0x0700_0008 => 7, 0x0700_0009 => 0x17, 0x0700_000a => 0x18, 0x0700_000b => 0x19,
        0x0700_000c => 0x1a, 0x0700_000d => 0x37, 0x0700_0011 => 0x1e, 0x0700_0012 => 0x1f, 0x0700_0013 => 0x20,
        0x0700_0014 => 0x27, 0x0700_0015 => 0x28, 0x0700_0016 => 0x2b, 0x0700_001a | 0x0700_001b => 0x25,
        0x0700_0021..=0x0700_0029 | 0x0700_004b | 0x0700_004c => 0x16, 0x0700_0041 => 0xb, 0x0700_0048 => 0xc,
        0x0700_004a => 0x21, _ => return None,
    })
}

impl Sounds {
    fn alloc(&mut self, n: Node) -> Handle {
        if self.nodes.len() < POOL { self.nodes.push(Some(n)); return self.nodes.len() as Handle; }
        match self.nodes.iter().position(|x| x.is_none()) {
            Some(i) => { self.nodes[i] = Some(n); i as Handle + 1 }
            None => 0,
        }
    }

    /// `Sound_Play(id, …)`: a 2-D sound. 0xFFFF (low 20 bits) is "none".
    pub fn play(&mut self, id: u32) -> Handle {
        if id & 0xfffff == 0xffff { return 0; }
        self.alloc(Node { id, pos: [0.0; 3], is3d: false, custom: None, state: 0, age: 0, since_start: 0 })
    }

    /// `Sound_Play3D(id, pos, …, custom)`: `radii` = Some((inner, outer)) only for callers passing custom = 1 (map
    /// ambients); everything else uses the bank's radii.
    pub fn play3d(&mut self, id: u32, pos: V3, radii: Option<(f32, f32)>) -> Handle {
        if id & 0xfffff == 0xffff { return 0; }
        self.alloc(Node { id, pos, is3d: true, custom: radii, state: 0, age: 0, since_start: 0 })
    }

    /// A LOOPED id's re-trigger: one voice per id. The fire scripts' op-1 loop starts replay every held
    /// refire tick (`AnimScriptAdd` 0x1c6d40 DELETES the playing script and creates a fresh one [M] —
    /// the Laser/Stunner's +0x68 & 0x400 family refires at interval 1 [L weapon_data +0x40 = 1], so
    /// the op-1 fires 60×/s); the game's re-trigger keeps the single loop audible instead of piling
    /// attack transients. fix-laser-beam: without this the port allocated a node per tick (the pool
    /// churned, the loop never held steady). [S: the PS2 bank's per-id behaviour; the port matches the
    /// audible result.]
    pub fn play_loop(&mut self, id: u32, pos: V3) -> Handle {
        if id & 0xfffff == 0xffff { return 0; }
        if let Some(i) = self.nodes.iter().position(|n| matches!(n, Some(x) if x.id == id && x.state == 0)) {
            return i as Handle + 1;
        }
        self.play3d(id, pos, None)
    }

    /// `Sound_Stop`: removed at the next update.
    pub fn stop(&mut self, h: Handle) { if let Some(Some(n)) = self.node(h) { n.state = 2; } }

    /// `Sound_StopAllWithId`.
    pub fn stop_all_with_id(&mut self, id: u32) { for n in self.nodes.iter_mut().flatten() { if n.id == id { n.state = 2; } } }

    /// fix-laser-beam test hook: the ids of the ACTIVE (state 0 = playing) voices, pool order.
    pub fn active_ids(&self) -> Vec<u32> { self.nodes.iter().flatten().filter(|n| n.state == 0).map(|n| n.id).collect() }

    /// `Sound_SetPosition`.
    pub fn set_position(&mut self, h: Handle, pos: V3) { if let Some(Some(n)) = self.node(h) { n.pos = pos; } }

    /// `Sound_IsFinnished`: no node, or stop requested / ended.
    pub fn finished(&self, h: Handle) -> bool {
        match h.checked_sub(1).and_then(|i| self.nodes.get(i as usize)) { Some(Some(n)) => n.state == 2, _ => true }
    }

    fn node(&mut self, h: Handle) -> Option<&mut Option<Node>> { h.checked_sub(1).and_then(|i| self.nodes.get_mut(i as usize)) }

    /// `Sound_UpdateListeners` + `Sound_UpdateSounds` for one frame. `looped(id)`: `SFXOutputData+0x14`;
    /// `playing(handle)`: the driver still has the sound. One-shots marked with `SFXOutputData+0x15` are not started
    /// when the listener is beyond 1.1 × their outer radius (single player) [M `Sound_ReqestPlaySfx`].
    pub fn update(&mut self, table: Option<&SfxTable>, listener: (V3, V3, V3, V3), playing: &dyn Fn(Handle) -> bool) {
        let (pos, norm, up, dir) = listener;
        self.listener = pos;
        self.out.push(Cmd::Listener { pos, norm, up, dir });
        let entry = |id: u32| table.and_then(|t| t.entries.get(id as usize).copied().filter(|_| id < 0x60d));
        for i in 0..self.nodes.len() {
            let h = i as Handle + 1;
            let Some(n) = self.nodes[i].as_mut() else { continue };
            n.age += 1;
            match n.state {
                0 => {
                    n.state = 1;
                    n.since_start = 0;
                    let e = entry(n.id);
                    if n.is3d {
                        let d = ((n.pos[0] - pos[0]).powi(2) + (n.pos[1] - pos[1]).powi(2) + (n.pos[2] - pos[2]).powi(2)).sqrt();
                        let culled = e.is_some_and(|e| !e.looped && e.cull && d > 1.1 * e.outer);
                        if !culled { self.out.push(Cmd::Start { handle: h, id: n.id & 0xfffff, pos: Some(n.pos), radii: n.custom }); }
                    } else {
                        self.out.push(Cmd::Start { handle: h, id: n.id & 0xfffff, pos: None, radii: None });
                    }
                }
                1 => {
                    n.since_start += 1;
                    let looped = entry(n.id).is_some_and(|e| e.looped);
                    if !looped && n.since_start > START_GRACE && !playing(h) { n.state = 2; }
                }
                _ => {
                    self.out.push(Cmd::Remove { handle: h });
                    self.nodes[i] = None;
                    continue;
                }
            }
            let n = self.nodes[i].as_ref().unwrap();
            if n.is3d && n.state == 1 { self.out.push(Cmd::Position { handle: h, pos: n.pos }); }
        }
    }

    /// The driver commands since the last call.
    pub fn take_commands(&mut self) -> Vec<Cmd> { std::mem::take(&mut self.out) }

    /// Live nodes (tests, debugging).
    pub fn live(&self) -> usize { self.nodes.iter().flatten().count() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deferred_start_grace_and_removal() {
        let mut s = Sounds::default();
        let l = ([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        let h = s.play3d(0x403, [1.0, 0.0, 0.0], None);
        assert!(s.take_commands().is_empty(), "nothing until the update");
        s.update(None, l, &|_| false);
        let c = s.take_commands();
        assert!(c.contains(&Cmd::Start { handle: h, id: 0x403, pos: Some([1.0, 0.0, 0.0]), radii: None }));
        // Within the grace period a sound the driver does not report yet stays.
        for _ in 0..START_GRACE { s.update(None, l, &|_| false); }
        assert!(!s.finished(h));
        s.update(None, l, &|_| false);
        assert!(s.finished(h));
        s.update(None, l, &|_| false);
        assert!(s.take_commands().contains(&Cmd::Remove { handle: h }));
        assert_eq!(s.live(), 0);
        // Sister: a sound the driver keeps playing stays alive; 0xFFFF is no sound.
        let h2 = s.play(0x282);
        for _ in 0..20 { s.update(None, l, &|x| x == h2); }
        assert!(!s.finished(h2));
        assert_eq!(s.play(0xffff), 0);
    }
}
