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
