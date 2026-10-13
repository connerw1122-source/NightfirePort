//! The IOP sound driver (`SFX.IRX`) for sound effects: `SFXStart3D`, `SFXSetup`, `StartSample`, `SFXUpdate`,
//! `PS2_SFXCalculate3D`, `ES_CombineVolumes`, `SFXRemove`, `SFXSetPositionAndVelocity`, `SFXSetupListener`.
//! Spec: docs/systems/sound.md "IOP driver". All arithmetic is the driver's 32-bit integer arithmetic.
//!
//! Positions arrive as the EE packs them: world units × 1000, truncated to integers (`SFXStart3D` /
//! `SFXSetupListener` in ACTION.ELF). The SPU stand-in is [`crate::Output`]: a 3-D sound uses a front voice (left,
//! right) and a rear voice whose right volume is phase-inverted (`0x7FFF − s`).

use crate::{Cmd, Output, Pcm};
use nf_formats::soundbank::{Bank, Effect, SubTrack};
use std::collections::HashMap;
use std::sync::Arc;

/// `PS2_SquareRoot` (0x10250): 10 Newton steps from 1 with truncating 64-bit division — not converged for large
/// inputs (√1e6 → 1297), reproduced exactly.
pub fn ps2_sqrt(n: u32) -> u32 {
    if n == 0 { return 0; }
    let (mut x, nn) = (1i64, n as i32 as i64);
    for _ in 0..10 { x -= (x * x - nn) / (2 * x); }
    x as u32
}

/// The driver's random generator (`SFXrnd` 0x2efc): seeds 12341234 / 13123123.
#[derive(Clone, Debug)]
pub struct Rnd { s1: i32, s2: i32 }

impl Default for Rnd { fn default() -> Rnd { Rnd { s1: 12_341_234, s2: 13_123_123 } } }

impl Rnd {
    pub fn rnd(&mut self, n: i32) -> i32 {
        if n < 1 { return 0; }
        self.s1 = self.s1.wrapping_add(self.s2);
        self.s2 = self.s2.wrapping_add(self.s1).wrapping_add(self.s1 >> 31);
        (((self.s1 as u32) >> 16) % n as u32) as i32
    }
    /// `SFXsfrnd`: −1024..1024.
    pub fn sfrnd(&mut self) -> i32 { self.rnd(0x801) - 0x400 }
}

type P3 = [i32; 3];

/// One hardware voice pair (`ES_VoiceHandleData` → SPU voices): front, and rear for 3-D sounds.
#[derive(Clone, Debug)]
struct Voice { front: u32, rear: Option<u32>, looping: bool, playing: bool, pitch: u32, stream: bool }

#[derive(Clone, Debug)]
struct Item {
    id: u32,
    counter: i32,
    vh: [Option<usize>; 10],
    base_volume: [i32; 10],
    pitch: [i32; 10],
    pan: [i32; 10],
    cue_pos: usize,
    csw: [i32; 50],
    cued: Vec<usize>,
    fx: Arc<Effect>,
    slot: usize,
    updated: i32,
    real_pos: P3,
    pos: P3,
    handle: u32,
    inner: i32,
    outer: i32,
    delete: bool,
    fade_kill: bool,
    fade_kill_vol: i32,
    streamed: bool,
    /// `Paused` (SFXItem+0x290): a `PauseInNis` sound while an NIS plays — pitch 0, skipped by `SFXUpdate` [M §8].
    paused: bool,
    /// `KeyOnFrameDelay` (SFXItem+0x2B4): the precache countdown; `HasSampleEnded` reports false while ≥ 1 [M §3.2].
    key_on_delay: i32,
}

/// Bank slot data: the parsed bank, its decoded samples (lazily) and the raw ADPCM.
struct Slot { bank: Bank, sbf: Vec<u8>, pcm: HashMap<usize, Arc<Pcm>> }

/// The driver state (`SFX.IRX` globals).
pub struct Driver {
    slots: HashMap<usize, Slot>,
    /// Active list, head (index 0) = newest (`NewSFX` pushes at the head). At most 40.
    items: Vec<Item>,
    voices: Vec<Option<Voice>>,
    rnd: Rnd,
    started: i32,
    pub mode: i32,
    pub master: i32,
    pub music_master: i32,
    mute_sfx: i32,
    /// `SFXIndoorsVolume` (current), moves ±4/frame toward `indoors_target` (`SFXUpdateEnvironment`@e60) [M].
    indoors: i32,
    /// `SFXNewIndoorsVolume` (target): 50 indoors, 100 outdoors [M `SFXSetEnvironment`].
    indoors_target: i32,
    /// `SFXReverb` (current, 0..100), moves +16/−4 per frame toward `reverb_target` [M].
    reverb: i32,
    /// `SFXNewReverb` (target, 0..100) [M].
    reverb_target: i32,
    /// `SFXUnderWater` (0/1) [M].
    underwater: bool,
    /// `FadeVolume` (0..10000), the SFX class fade [M `SFXFadeUpdate`@f8c].
    fade: i32,
    /// `FadeMusicVolume` (0..10000), copied from `FadeVolume` when `fade_music` is set [M].
    fade_music_volume: i32,
    /// `FadeStatus`: 0 inactive, 1 fade-down, 2 fade-up [M].
    fade_status: i32,
    /// `FadeRate` (init 30) — the dead `10000/(status*rate)` term is overwritten; kept for fidelity [M].
    fade_rate: i32,
    /// `FadeMusic` (init 1): copy `FadeVolume`→`FadeMusicVolume` while fading [M].
    fade_music: i32,
    /// `FadeStartTimer` (ms at fade start) [M].
    fade_start: i32,
    /// `DuckerCurrentVolume` (0..100) [M]. Music ducks to this; SFX get 1/3 of the duck.
    ducker_current: i32,
    /// `DuckerVolume` (target) [M `ES_StartDucker`@83c8].
    ducker_volume: i32,
    /// `DuckerDuration` (ms) [M].
    ducker_duration: i32,
    /// `DuckerStartTime` (ms) [M].
    ducker_start: i32,
    /// `SFXTimerValue`*100 → ms; advances ~17 ms per SFXUpdate (the port's fixed 60 Hz; the IOP's TimerThread ticks
    /// every 100 000 µs, so the original quantises to 100 ms — the audible 2 s fade is preserved) [M + S quantisation].
    timer_ms: i32,
    /// `NisQuitten`: when set, `ES_StopDucker` runs (Duration=0 → fast recovery) [M sound.md §6 step 2].
    nis_quitten: bool,
    /// `SFXKeyOnFrameDelay` (gp −0x7f18): the pending precache countdown, read and cleared by the next start [M].
    key_on_frame_delay: i32,
    /// `NisPlaying` (gp 0x339b0): a cut-scene is playing — `PauseInNis` sounds pause [M §8, ES_HandleNisPauseSfx].
    nis_playing: bool,
    listener_pos: P3,
    listener_dir: P3,
    listener_norm: P3,
    /// The {id, handle} list the EE reads back after each batch (`SFXActiveData`).
    pub active: Vec<(u32, u32)>,
    /// Streamed effects: `STREAMS.LUT` and `STREAMS.BIN`, decoded entries.
    streams: Option<(Vec<[u32; 4]>, Arc<Vec<u8>>)>,
    stream_pcm: HashMap<usize, Arc<Pcm>>,
}

/// `Streams[1..2]`: at most two streamed effects at once (slot 0 is the music) [M `SFXInitialiseStream`].
pub const STREAM_SLOTS: usize = 2;
/// A streamed effect's base pitch: 22 050 Hz (0x759) [M `StartSample`].
pub const STREAM_PITCH: i32 = 0x759;

pub const MAX_ITEMS: usize = 40;
pub const HW_VOICES: usize = 48;
/// `ES_VoiceHandleData`: 100 ES handles.
pub const ES_HANDLES: usize = 100;

/// The port's fixed logic rate (59.94/60 Hz, docs/PLAN.md "Timing"); the IOP's `TimerThread` ticks every 100 000 µs,
/// so the original quantises `psiSFXGetTimer()` to 100 ms. We advance ~17 ms/frame — the 2 s fade/duck durations hold
/// (10000/5 = 2000 ms) [M sound.md §6; S 100 ms quantisation].
pub const FRAME_MS: i32 = 17;

/// `IsUnderWaterSFX`: the ids not halved under water (`BonbUnderWaterSFX`).
const UNDERWATER_EXEMPT: [u32; 23] = [335, 331, 332, 333, 334, 330, 279, 278, 277, 253, 1303, 137, 1492, 1491, 136, 514, 10, 40, 33, 310, 311, 39, 9];

impl Default for Driver { fn default() -> Driver { Driver::new() } }

impl Driver {
    pub fn new() -> Driver {
        Driver { slots: HashMap::new(), items: Vec::new(), voices: vec![None; HW_VOICES], rnd: Rnd::default(), started: 0, mode: 1, master: 100,
            music_master: 70, mute_sfx: 1, indoors: 100, indoors_target: 100, reverb: 0, reverb_target: 0, underwater: false,
            fade: 10000, fade_music_volume: 10000, fade_status: 0, fade_rate: 30, fade_music: 1, fade_start: 0,
            ducker_current: 100, ducker_volume: 100, ducker_duration: 1, ducker_start: 0, timer_ms: 0, nis_quitten: false, nis_playing: false,
            key_on_frame_delay: 0,
            listener_pos: [0; 3], listener_dir: [0, 1000, 0], listener_norm: [-1000, 0, 0], active: Vec::new(), streams: None, stream_pcm: HashMap::new() }
    }

    /// `STREAMS.LUT` / `STREAMS.BIN` (`SFXInitialiseAudioStreamSystem`).
    pub fn load_streams(&mut self, lut: Vec<[u32; 4]>, bin: Arc<Vec<u8>>) { self.streams = Some((lut, bin)); }

    fn stream(&mut self, k: usize) -> Option<Arc<Pcm>> {
        if let Some(p) = self.stream_pcm.get(&k) { return Some(p.clone()); }
        let (lut, bin) = self.streams.as_ref()?;
        let data = nf_formats::soundbank::stream_data(bin, *lut.get(k)?).ok()?;
        let p = Arc::new(Pcm { data: nf_formats::vag::decode(data), loop_start: None });
        self.stream_pcm.insert(k, p.clone());
        Some(p)
    }

    /// `SFXLoadSoundBank` into `slot` (the index of its hash in `SBINFO.SBI`).
    pub fn load_bank(&mut self, slot: usize, bank: Bank, sbf: Vec<u8>) { self.slots.insert(slot, Slot { bank, sbf, pcm: HashMap::new() }); }

    /// `SFXSetupListener` (one listener): eye position and the camera's forward and left axes, all × 1000 as the EE
    /// packs them. `Norm` is the camera matrix's row 0, which points to the screen left.
    pub fn set_listener(&mut self, pos: [f32; 3], dir: [f32; 3], left: [f32; 3]) {
        let k = |v: [f32; 3]| v.map(|x| (x * 1000.0) as i32);
        (self.listener_pos, self.listener_dir, self.listener_norm) = (k(pos), k(dir), k(left));
    }

    fn pcm(&mut self, slot: usize, k: usize) -> Option<Arc<Pcm>> {
        let s = self.slots.get_mut(&slot)?;
        if let Some(p) = s.pcm.get(&k) { return Some(p.clone()); }
        let (data, loop_start) = s.bank.decode(&s.sbf, k).ok()?;
        let p = Arc::new(Pcm { data, loop_start });
        s.pcm.insert(k, p.clone());
        Some(p)
    }

    /// `SFXStart3D(id, pos, vel, handle, volume, inner, outer)`; `pos = None` is `SFXStart` (2-D, position zero).
    /// Radii −1 = the effect's. Returns 0 ok, 1 not found / setup failed, 2 list full.
    pub fn start(&mut self, out: &mut Vec<Cmd>, id: u32, pos: Option<[f32; 3]>, handle: u32, radii: Option<(i32, i32)>) -> u32 {
        // `kd = SFXKeyOnFrameDelay; SFXKeyOnFrameDelay = 0` — read and cleared on every start [M §3.2 step 1].
        let kd = std::mem::replace(&mut self.key_on_frame_delay, 0);
        let mut slots: Vec<usize> = self.slots.keys().copied().collect();
        slots.sort();
        let Some((slot, fx)) = slots.into_iter().find_map(|s| self.slots[&s].bank.effect(id).and_then(|b| Effect::parse(b).ok()).map(|e| (s, e))) else { return 1 };
        // Precache (§3.2 step 3): only 1-sample streams can be precached. A live countdown item with the same id
        // is taken over by a real start (kd == 0; the port's zero-latency stand-in keyed it on at once, so the
        // audible voice just keeps playing) and a second precache of the same id is refused [M].
        if let Some(k) = self.items.iter().position(|it| it.id == id && it.key_on_delay > 0 && !it.delete) {
            if kd != 0 { return 1; }
            self.items[k].key_on_delay = 0;
            return 0;
        }
        if kd != 0 && !(fx.count == 1 && fx.subs[0].file < 0) { return 1; }
        if self.items.len() >= MAX_ITEMS { return 2; }
        let p = pos.map_or([0; 3], |v| v.map(|x| (x * 1000.0) as i32));
        let (inner, outer) = radii.unwrap_or((-1, -1));
        let item = Item {
            id, counter: 0, vh: [None; 10], base_volume: [0; 10], pitch: [0; 10], pan: [0; 10], cue_pos: 0, csw: [0; 50], cued: Vec::new(),
            inner: if inner == -1 { fx.inner } else { inner }, outer: if outer == -1 { fx.outer } else { outer }, fx: Arc::new(fx), slot,
            updated: 0, real_pos: p, pos: p, handle, delete: false, fade_kill: false, fade_kill_vol: 100, streamed: false,
            paused: false, key_on_delay: kd,
        };
        self.items.insert(0, item);
        if self.setup(out, 0) { 0 } else { 1 }
    }

    /// `ES_GetDist`: world units (`√(min Σ(Δ/100)²) / 10`).
    fn dist(&self, it: &Item) -> i32 {
        let s: i64 = (0..3).map(|k| { let d = (self.listener_pos[k] - it.real_pos[k]) / 100; d as i64 * d as i64 }).sum();
        ps2_sqrt(s as u32) as i32 / 10
    }

    /// Other items' live ES handles (`CountAllVoicesActive`).
    fn count_voices(&self, except: usize) -> i32 {
        self.items.iter().enumerate().filter(|(i, _)| *i != except).map(|(_, it)| it.vh.iter().flatten().count() as i32).sum()
    }

    /// `SFXSetup` (0x4220): rejection, budget stealing, the per-id limit, then the sample choice.
    fn setup(&mut self, out: &mut Vec<Cmd>, i: usize) -> bool {
        let fx = self.items[i].fx.clone();
        self.items[i].vh = [None; 10];
        self.started = self.started.min(2000);
        self.items[i].delete = false;
        self.items[i].streamed = false;
        let budget = 15 - self.started / 400;
        let n = self.count_voices(i);
        if fx.tracking == 2 && fx.priority < 100 {
            let it = &self.items[i];
            let d = self.dist(it);
            let mut reject = d > it.outer;
            if !reject {
                let r = (it.outer - it.inner).max(1);
                let v = (100 - ((d - it.inner) * 100) / r).clamp(0, 100);
                reject = v * fx.priority < (n * budget * 2000) / (budget * budget);
            }
            if reject { self.items[i].delete = true; return false; }
        }
        if n > budget {
            let n2 = self.count_voices(i);
            for _ in 0..(n2 - budget) {
                let mut best: Option<(i32, usize)> = None;
                for (k, it) in self.items.iter().enumerate() {
                    if it.delete || it.streamed || it.fx.tracking == 3 { continue; }
                    if best.is_none_or(|b| it.fx.priority < b.0) { best = Some((it.fx.priority, k)); }
                }
                if let Some((_, k)) = best { self.items[k].delete = true; }
            }
            if fx.max_reject != 0 { self.items[i].delete = true; return false; }
            if self.items[i].delete { return false; }
        }
        if fx.max_voices != 0 {
            let id = self.items[i].id;
            let same: Vec<usize> = (0..self.items.len()).filter(|&k| k != i && self.items[k].id == id).collect();
            if same.len() as i32 >= fx.max_voices {
                if fx.max_reject != 0 { self.items[i].delete = true; return false; }
                let mut oldest: Option<(i32, usize)> = None;
                for &k in &same { if oldest.is_none_or(|o| self.items[k].counter > o.0) { oldest = Some((self.items[k].counter, k)); } }
                if let Some((_, k)) = oldest { self.items[k].delete = true; }
            }
        }
        self.started += 100;
        self.items[i].cue_pos = 0;
        if fx.ducker != 0 && (fx.tracking == 0 || fx.tracking == 3) { self.start_ducker(100 - fx.ducker as i32, fx.ducker_length); }
        if fx.multi_sample == 0 {
            let k = self.rnd.rnd(fx.count as i32) as usize;
            if !self.start_sample(out, i, fx.subs[k], 0) { self.items[i].delete = true; return false; }
            self.items[i].csw[0] = self.cue_timer(&fx);
        } else if fx.polyphonic == 0 {
            let n = fx.count;
            self.items[i].cue_pos = n;
            self.items[i].csw[0] = 0;
            let cued: Vec<usize> = if fx.shuffled == 0 { (0..n).map(|k| n - 1 - k).collect() } else {
                let (mut used, mut rem, mut v) = ([false; 50], n as i32, Vec::new());
                for _ in 0..n {
                    let mut r = self.rnd.rnd(rem);
                    let mut idx = 0usize;
                    loop { if r > 0 { idx += 1; } while used[idx] { idx += 1; } let go = r > 0; r -= 1; if !go { break; } }
                    used[idx] = true;
                    rem -= 1;
                    v.push(idx);
                }
                v
            };
            self.items[i].cued = cued;
            self.items[i].cue_pos -= 1;
            let k = self.items[i].cued[self.items[i].cue_pos];
            self.start_sample(out, i, fx.subs[k], 0);
            self.items[i].csw[0] = self.cue_timer(&fx);
        } else {
            for k in 0..fx.count.min(50) {
                self.items[i].csw[k] = self.cue_timer(&fx);
            }
            self.items[i].cued = (0..fx.count).collect();
        }
        true
    }

    fn cue_timer(&mut self, fx: &Effect) -> i32 { fx.min_delay + self.rnd.rnd(fx.max_delay - fx.min_delay) }

    /// `StartSample` (0x3524): a voice (pair), the randomised pitch / volume / pan, then key on.
    fn start_sample(&mut self, out: &mut Vec<Cmd>, i: usize, sub: SubTrack, ch: usize) -> bool {
        let fx = self.items[i].fx.clone();
        let is3d = fx.tracking != 0;
        let slot = self.items[i].slot;
        // A streamed sound (FileRef < 0): `STREAMS.LUT` entry −1 − FileRef, 22 050 Hz, one of two stream slots.
        let streamed = sub.file < 0;
        let (base, looped, pcm) = if streamed {
            self.items[i].streamed = true;
            let busy = self.items.iter().filter(|it| it.vh.iter().flatten().any(|&v| self.voices[v].as_ref().is_some_and(|x| x.stream))).count();
            if busy >= STREAM_SLOTS { return false; }
            let Some(p) = self.stream((-1 - sub.file) as usize) else { return false };
            (STREAM_PITCH, false, Some(p))
        } else {
            let Some(hdr) = self.slots.get(&slot).and_then(|s| s.bank.samples.get(sub.file as usize).copied()) else { return false };
            (hdr.pitch as i32, hdr.looped & 1 != 0, None)
        };
        // ES handles (100) and hardware voices (48; a 3-D sound needs two).
        let es_used: usize = self.items.iter().map(|it| it.vh.iter().flatten().count()).sum();
        if es_used >= ES_HANDLES { return false; }
        let free: Vec<usize> = (0..HW_VOICES).filter(|&v| self.voices[v].is_none()).collect();
        if free.len() < if is3d { 2 } else { 1 } { return false; }
        let Some(pcm) = (match pcm { Some(p) => Some(p), None => self.pcm(slot, sub.file as usize) }) else { return false };
        let (r1, r2, r3) = (self.rnd.sfrnd(), self.rnd.sfrnd(), self.rnd.sfrnd());
        let vol = (sub.volume + ((r2 * sub.volume_rnd) >> 10)).clamp(0, 100);
        let pitch = base + (base * (sub.pitch + ((r1 * sub.pitch_rnd) >> 10))) / 0x3000;
        let pan = (sub.pan + ((r3 * sub.pan_rnd) >> 10)).clamp(-100, 100);
        let it = &mut self.items[i];
        (it.base_volume[ch], it.pitch[ch], it.pan[ch]) = (vol, pitch, pan);
        let (front, rear) = (free[0] as u32, is3d.then(|| free[1] as u32));
        let v = Voice { front, rear, looping: looped, playing: true, pitch: pitch.max(0) as u32, stream: streamed };
        self.voices[front as usize] = Some(v);
        it.vh[ch] = Some(front as usize);
        it.pos = it.real_pos;
        self.get_my_pos(i);
        let (l, r, b) = self.levels(i, ch);
        // `ReverbSend` (SFXParameters+0x00): non-zero at key-on sets the voice's reverb-send bit (VMIXEL/VMIXER) [M].
        let reverb = fx.reverb_send != 0;
        out.push(Cmd::Play { voice: front, pcm: pcm.clone(), pitch: pitch.max(0) as u32, left: l, right: r, delay: 0.0, reverb });
        if let Some(rv) = rear {
            self.voices[rv as usize] = Some(Voice { front: rv, rear: None, looping: looped, playing: true, pitch: pitch.max(0) as u32, stream: streamed });
            // Mode ≠ 0: the rear voices are keyed on after `DelayThread(500)` (µs); mode 0 keys them with the front.
            out.push(Cmd::Play { voice: rv, pcm, pitch: pitch.max(0) as u32, left: b, right: if self.mode == 0 { b } else { -b }, delay: if self.mode == 0 { 0.0 } else { 0.0005 }, reverb });
        }
        true
    }

    /// `ES_GetMyPos`: the effective position (one listener).
    fn get_my_pos(&mut self, i: usize) {
        let (lp, ld) = (self.listener_pos, self.listener_dir);
        let it = &mut self.items[i];
        match it.fx.tracking {
            3 => { it.inner = 2; it.outer = 3; it.pos = [lp[0] + ld[0], lp[1] + ld[1], lp[2] + ld[2]]; }
            1 => {
                let s: i64 = (0..3).map(|k| { let d = (lp[k] - it.real_pos[k]) / 100; d as i64 * d as i64 }).sum();
                let d = ps2_sqrt(s as u32) as i32 * 100;
                it.pos = [0, 1, 2].map(|k| lp[k] + (d * ld[k]) / 1000);
            }
            _ => it.pos = it.real_pos,
        }
    }

    /// `ES_CombineVolumes` (0x1300) for channel `ch`.
    fn combine(&self, it: &Item, ch: usize) -> i32 {
        let fx = &it.fx;
        let mut v = it.base_volume[ch];
        if it.id == 0x1d7 || it.id == 0x470 { v = (v * self.music_master) / 100; } else {
            v = (((v * self.master * self.mute_sfx) / 100) * 100) / 100;
            if fx.outdoors != 0 { v = (v * self.indoors) / 100; }
            if self.underwater && !UNDERWATER_EXEMPT.contains(&it.id) { v /= 2; }
        }
        v = (((((v * self.fade) / 10000) * it.fade_kill_vol) / 100) * fx.master_volume) / 100;
        if fx.ducker == 0 { v = (v * (100 - (100 - self.ducker_current) / 3)) / 100; }
        (v * v) / 100
    }

    /// `PS2_SFXCalculate3D` (0x1060c) + `PS2_SFXSetup3D` / `Setup2D`: the voice volumes (front left, front right,
    /// rear), each `GSV(level) / 0x3FFF` as a fraction.
    fn levels(&self, i: usize, ch: usize) -> (f32, f32, f32) {
        let it = &self.items[i];
        let vol = self.combine(it, ch);
        let gsv = |v: i32| (((v * 0x3fff) / 100).clamp(0, 0x3fff)) as f32 / 0x3fff as f32;
        match it.fx.tracking {
            0 => {
                let pan = it.pan[ch];
                let (mut l, mut r) = (((-((pan - 100) / 2)) * vol / 100).clamp(0, 100), ((((pan + 100) / 2) * vol) / 100).clamp(0, 100));
                if self.mode == 0 { l = (l + r) / 2; r = l; }
                (gsv(l), gsv(r), 0.0)
            }
            3 => (gsv(vol / 2), gsv(vol / 2), gsv((vol * 30) / 100)),
            _ => {
                let lp = self.listener_pos;
                let dv = [0, 1, 2].map(|k| it.pos[k] - lp[k]);
                let mut u = dv.map(|x| x / 100);
                let s: i64 = u.iter().map(|&x| x as i64 * x as i64).sum();
                let mut d = ps2_sqrt(s as u32) as i32;
                d = (d - it.inner * 10).max(0);
                let rng = ((it.outer - it.inner) * 10).max(10);
                let a = ((1000 - (d * 1000) / rng).max(0) / 10).min(100);
                let att = (a * a) / 100;
                let m = ps2_sqrt(s as u32) as i32;
                if m != 0 { u = u.map(|x| (x * 1000) / m); }
                let dot = |a: P3, b: P3| (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / 1000;
                let n = self.listener_norm;
                // `PS2_DotProduct` = Σ / 1000: −1000..1000 for these length-1000 vectors.
                let dl = dot(n, u);
                let dr = dot(n.map(|x| -x), u);
                let db = dot(self.listener_dir.map(|x| -x), u);
                let rear = ((db + 1000) * att) / 2000;
                let front = 100 - rear;
                let mut l = ((((dl + 1000) * att) / 2000) * front) / 100;
                let mut r = ((((dr + 1000) * att) / 2000) * front) / 100;
                let mut b = (rear * 75) / 100;
                if self.mode == 0 { let m = (l + r + b) / 3; (l, r, b) = (m, m, m); }
                (gsv((l.clamp(0, 100) * vol) / 100), gsv((r.clamp(0, 100) * vol) / 100), gsv((b.clamp(0, 100) * vol) / 100))
            }
        }
    }

    /// `SFXSetPositionAndVelocity(pos, vel, handle, id)`: handle 0 / id −1 match everything.
    pub fn set_position(&mut self, handle: u32, id: Option<u32>, pos: [f32; 3]) {
        let p = pos.map(|x| (x * 1000.0) as i32);
        for it in &mut self.items { if (handle == 0 || it.handle == handle) && id.is_none_or(|i| it.id == i) { it.real_pos = p; } }
    }

    /// `SFXRemove(handle, id)`: marked, killed in the next update's kill pass.
    pub fn remove(&mut self, handle: u32, id: Option<u32>) {
        for it in &mut self.items { if (handle == 0 || it.handle == handle) && id.is_none_or(|i| it.id == i) { it.delete = true; } }
    }

    fn ended(&self, v: Option<usize>) -> bool {
        match v { None => true, Some(v) => self.voices[v].as_ref().is_none_or(|x| !x.playing && !x.looping) }
    }

    /// `HasSampleEnded` (0x6ac) for the item's first channel: 0 (not ended) while `KeyOnFrameDelay` ≥ 1 [M §3.2].
    fn sample_ended(&self, i: usize) -> bool { self.items[i].key_on_delay <= 0 && self.ended(self.items[i].vh[0]) }

    fn unlock(&mut self, i: usize, ch: usize, out: &mut Vec<Cmd>) {
        if let Some(v) = self.items[i].vh[ch].take() {
            if let Some(x) = self.voices[v].take() { if let Some(r) = x.rear { self.voices[r as usize] = None; let _ = out; } }
        }
    }

    /// `SFXPreCache(id, frames)` (RPC cmd 0x26, IOP 0x39f4): `SFXKeyOnFrameDelay = frames + 5` then `SFXStart(id, 0)`.
    /// Only streams can be precached: the start finds an existing same-id item with a countdown to take over, else
    /// marks the new one `DeleteMe` [M sound.md §3.2 step 3].
    pub fn pre_cache(&mut self, out: &mut Vec<Cmd>, id: u32, frames: i32) {
        self.key_on_frame_delay = frames + 5;
        self.start(out, id, None, 0, None);
    }

    /// `SFXSetEnvironment(reverb, indoors)` (RPC cmd 0x1D; called by `Sound_UpdateListeners`@1dce10 [M]).
    /// `reverb == -0x6768` (-26472) → underwater on, target 0; else target = clamp(reverb, 0, 100).
    /// `indoors != 0` → `SFXNewIndoorsVolume = 50`, else 100.
    pub fn set_environment(&mut self, reverb: i32, indoors: i32) {
        if reverb == -0x6768 { self.underwater = true; self.reverb_target = 0; }
        else { self.underwater = false; self.reverb_target = reverb.clamp(0, 100); }
        self.indoors_target = if indoors != 0 { 50 } else { 100 };
    }

    /// `SFXFadeDown(fadeMusic)` (RPC cmd 6): start a fade-down. `music` copies `FadeVolume`→`FadeMusicVolume` [M].
    pub fn fade_down(&mut self, music: bool) {
        self.fade_status = 1;
        self.fade_start = self.timer_ms;
        self.fade_music = music as i32;
    }

    /// `SFXFadeUp(fadeMusic)` (RPC cmd 7): fade-up always starts from 0 [M].
    pub fn fade_up(&mut self, music: bool) {
        self.fade = 0;
        self.fade_status = 2;
        self.fade_start = self.timer_ms;
        self.fade_music = music as i32;
    }

    /// `ES_StartDucker(target, len)` (0x83c8): StartTime = now, Duration = len, DuckerVolume = target [M].
    fn start_ducker(&mut self, target: i32, len: i32) {
        self.ducker_start = self.timer_ms;
        self.ducker_duration = len;
        self.ducker_volume = target;
    }

    /// `ES_StopDucker` (on `NisQuitten`): Duration = 0, so `passed` is true and the duck recovers fast [M].
    fn stop_ducker(&mut self) { self.ducker_duration = 0; self.ducker_start = self.timer_ms; }

    /// Sets the `NisQuitten` flag (a cut-scene is quitting); `ES_StopDucker` runs next `SFXUpdate` [M].
    pub fn set_nis_quitten(&mut self, on: bool) { self.nis_quitten = on; }

    /// Sets `NisPlaying` (RPC op 0x19, emitted by `SFXUpdate` when it changed): `PauseInNis` sounds pause while
    /// it is set [M sound.md §8, `ES_HandleNisPauseSfx`].
    pub fn set_nis_playing(&mut self, on: bool) { self.nis_playing = on; }

    /// `SFXUpdateEnvironment` (0xe60): `SFXReverb` moves toward `reverb_target` by at most +16 (rising) / −4 (falling)
    /// per frame; `SFXIndoorsVolume` moves ±4. Returns the new reverb depth 0..1 (EVOL = r·80/100·0x3FFF) [M].
    fn update_environment(&mut self) -> f32 {
        if self.reverb < self.reverb_target { self.reverb = (self.reverb + 16).min(self.reverb_target); }
        else if self.reverb > self.reverb_target { self.reverb = (self.reverb - 4).max(self.reverb_target); }
        if self.indoors < self.indoors_target { self.indoors = (self.indoors + 4).min(self.indoors_target); }
        else if self.indoors > self.indoors_target { self.indoors = (self.indoors - 4).max(self.indoors_target); }
        // `psiSetReverb`: EVOL = (((r*80)/100)*0x3FFF)/100 → 0..0x3FFF; the 80 % ceiling is the game's max send depth [M].
        let evol = ((self.reverb * 80) / 100 * 0x3fff) / 100;
        evol as f32 / 0x3fff as f32
    }

    /// `ES_UpdateDucker` (0x8438): once per `SFXUpdate` [M].
    fn update_ducker(&mut self) {
        let now = self.timer_ms;
        // `passed = (now-StartTime >= -0x40000000) && (now-StartTime > Duration)`: the first term guards against
        // wrap (a huge negative means StartTime is in the future); with i32 ms this is always satisfied post-start.
        let elapsed = now.wrapping_sub(self.ducker_start);
        let passed = elapsed >= -0x4000_0000 && elapsed > self.ducker_duration;
        let s = (self.ducker_volume - self.ducker_current) * 1024;
        if s != 0 {
            // `s = (passed ? s/15 : s/6) >> 10`, then if 0 take the sign (so a tiny step still creeps) [M].
            let div = if passed { 15 } else { 6 };
            let mut step = (s / div) >> 10;
            if step == 0 { step = if s > 0 { 1 } else { -1 }; }
            self.ducker_current += step;
        }
        if passed { self.ducker_volume = 100; }
    }

    /// `SFXFadeUpdate` (0xf8c): when `FadeStatus != 0`, `FadeVolume = 10000 - 5*(t-t0)` (down) or `5*(t-t0)` (up),
    /// clamped 0..10000; at the limit `FadeStatus = 0` (a 2 s fade). `FadeMusic` copies into `FadeMusicVolume` [M].
    fn fade_update(&mut self) {
        if self.fade_status == 0 { return; }
        let t = self.timer_ms - self.fade_start;
        let v = if self.fade_status == 1 { 10000 - 5 * t } else { 5 * t };
        if v <= 0 { self.fade = 0; self.fade_status = 0; }
        else if v >= 10000 { self.fade = 10000; self.fade_status = 0; }
        else { self.fade = v; }
        if self.fade_music != 0 { self.fade_music_volume = self.fade; }
    }

    /// `SFXUpdate` (0x4ee4): once per game frame. `ended`: hardware voices that reached their end flag.
    /// Returns the reverb depth 0..1 (`EVOL`) the output should apply this frame [M §6 step 2].
    pub fn update(&mut self, out: &mut Vec<Cmd>, ended: &[u32]) -> f32 {
        for &e in ended { if let Some(Some(v)) = self.voices.get_mut(e as usize) { v.playing = false; } }
        if self.started > 0 { self.started = self.started * 9 / 10; }
        // §6 step 2: `SFXUpdateEnvironment; ES_UpdateDucker; SFXFadeUpdate; if NisQuitten: ES_StopDucker` [M].
        // The timer advances ~17 ms/frame (the port's fixed 60 Hz; the IOP quantises to 100 ms — the 2 s fade holds).
        self.timer_ms = self.timer_ms.wrapping_add(FRAME_MS);
        let reverb_depth = self.update_environment();
        self.update_ducker();
        self.fade_update();
        if self.nis_quitten { self.stop_ducker(); }
        for it in &mut self.items { it.updated = 0; }
        let mut i = 0;
        while i < self.items.len() {
            let fx = self.items[i].fx.clone();
            {
                let it = &mut self.items[i];
                // `ES_HandleNisPauseSfx` (§8): a `PauseInNis` sound pauses while an NIS plays — the live voices get
                // pitch 0 and the item is skipped (no counter, no 3-D update, no end test); unpausing restores the
                // pitch on the next full update (the port has no global `SFXPause`) [M].
                let want = self.nis_playing && fx.pause_in_nis != 0;
                if want && !it.paused {
                    for ch in 0..10 {
                        if let Some(v) = it.vh[ch] { if let Some(vx) = self.voices[v].as_ref() { out.push(Cmd::Set { voice: vx.front, pitch: 0, left: 0.0, right: 0.0 }); if let Some(r) = vx.rear { out.push(Cmd::Set { voice: r, pitch: 0, left: 0.0, right: 0.0 }); } } }
                    }
                }
                it.paused = want;
                // The precache countdown (`KeyOnFrameDelay`): counts down once per frame [M §3.2]. (*Deviation*: the
                // game deletes the silent pre-setup at 0 — the port keyed the voice on at once, so expiry just
                // closes the precache window.)
                if it.key_on_delay > 0 { it.key_on_delay -= 1; }
                if it.paused { i += 1; continue; }
                if it.fade_kill { it.fade_kill_vol -= 5; if it.fade_kill_vol < 0 { it.fade_kill_vol = 0; it.delete = true; } }
                it.updated += 1;
                it.counter += 1;
            }
            let mut rerun = false;
            if fx.multi_sample == 0 {
                if self.sample_ended(i) { self.unlock(i, 0, out); rerun = self.ending(out, i); }
            } else if fx.polyphonic == 0 {
                if self.sample_ended(i) {
                    self.unlock(i, 0, out);
                    if self.items[i].cue_pos == 0 { rerun = self.ending(out, i); } else {
                        let c = self.items[i].csw[0];
                        self.items[i].csw[0] -= 1;
                        if c < 1 {
                            self.items[i].cue_pos -= 1;
                            let k = self.items[i].cued[self.items[i].cue_pos];
                            self.start_sample(out, i, fx.subs[k], 0);
                            self.items[i].csw[0] = self.cue_timer(&fx);
                        }
                    }
                }
            } else {
                let (mut started, mut busy) = (false, false);
                for k in 0..fx.count.min(10) {
                    let c = self.items[i].csw[k];
                    if c == 0 { let s = fx.subs[self.items[i].cued[k]]; self.start_sample(out, i, s, k); self.items[i].csw[k] = -1; started = true; }
                    else if c < 1 { if self.ended(self.items[i].vh[k]) { self.unlock(i, k, out); } else { busy = true; } }
                    else { self.items[i].csw[k] -= 1; busy = true; }
                }
                if !started && !busy { rerun = self.ending(out, i); }
            }
            // Recompute every live channel's volumes (3-D position, attenuation and globals).
            self.get_my_pos(i);
            for ch in 0..10 {
                let Some(v) = self.items[i].vh[ch] else { continue };
                let (l, r, b) = self.levels(i, ch);
                let Some(vx) = self.voices[v].clone() else { continue };
                out.push(Cmd::Set { voice: vx.front, pitch: vx.pitch, left: l, right: r });
                if let Some(rv) = vx.rear { out.push(Cmd::Set { voice: rv, pitch: vx.pitch, left: b, right: if self.mode == 0 { b } else { -b } }); }
            }
            if !rerun { i += 1; }
        }
        // Kill pass: key off and free.
        let mut k = 0;
        while k < self.items.len() {
            if self.items[k].delete {
                for ch in 0..10 {
                    if let Some(v) = self.items[k].vh[ch].take() {
                        if let Some(x) = self.voices[v].take() {
                            out.push(Cmd::Stop { voice: x.front });
                            if let Some(r) = x.rear { out.push(Cmd::Stop { voice: r }); self.voices[r as usize] = None; }
                        }
                    }
                }
                self.items.remove(k);
            } else { k += 1; }
        }
        self.active = self.items.iter().map(|it| (it.id, it.handle)).collect();
        reverb_depth
    }

    /// `SFXUpdateEnding` (0x4e20): delete, or for `Loop` re-trigger after the cue delay. True = process again now.
    fn ending(&mut self, out: &mut Vec<Cmd>, i: usize) -> bool {
        let fx = self.items[i].fx.clone();
        if fx.looped == 0 { self.items[i].delete = true; return false; }
        let c = self.items[i].csw[0];
        self.items[i].csw[0] -= 1;
        c < 1 && self.setup(out, i) && self.items[i].updated < 2
    }

    /// `SFXIsSFXPlaying(handle, −1)`: the handle is in the active data.
    pub fn is_playing(&self, handle: u32) -> bool { self.active.iter().any(|a| a.1 == handle) }

    /// Live items (tests).
    pub fn items(&self) -> usize { self.items.len() }
}

/// The driver plus the sound output, fed by the EE layer's commands once per game tick.
pub struct Sound {
    pub driver: Driver,
    pub output: Option<Output>,
    /// The level's music: track number, marker file and SSD; the stream player once started; the EE's music
    /// commands of the last frame (sent with this frame's batch).
    pub music_files: Option<(i32, nf_formats::music::Smf, std::sync::Arc<Vec<u8>>)>,
    pub music: Option<crate::music::Player>,
    pub music_pending: Vec<(i32, i32, i32)>,
}

impl Sound {
    /// Runs one batch: the EE commands, then `SFXUpdate`, and sends the voice changes to the output.
    pub fn batch(&mut self, f: impl FnOnce(&mut Driver, &mut Vec<Cmd>)) {
        let mut out = Vec::new();
        f(&mut self.driver, &mut out);
        let ended = self.output.as_ref().map(|o| o.take_ended()).unwrap_or_default();
        let reverb_depth = self.driver.update(&mut out, &ended);
        // `PS2_SetupReverb`'s EVOL depth (the reverb send level), written to the output [M sound.md §5.5].
        out.push(Cmd::Reverb { depth: reverb_depth });
        // The music commands queued last frame (op 0x11 start / 0x13 jump), then `SFXUpdateStreams`.
        for (kind, a, b) in std::mem::take(&mut self.music_pending) {
            match kind {
                0 => if let Some((track, smf, ssd)) = &self.music_files { if *track == a { self.music = Some(crate::music::Player::start(smf.clone(), ssd.clone(), b, &mut out)); } },
                _ => if let Some(p) = self.music.as_mut() { p.request(a, b != 0); },
            }
        }
        if let Some(p) = self.music.as_mut() {
            let played = self.output.as_ref().map_or(0, |o| o.music_played());
            p.update(played, &mut out);
            // `ES_MusicSetVolume`: music ducks to `DuckerCurrentVolume` and fades with `FadeMusicVolume` [M §5.5].
            out.push(Cmd::MusicGain(p.gain(self.driver.music_master, self.driver.ducker_current, self.driver.fade_music_volume)));
        }
        if let Some(o) = &self.output { o.send(out); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_and_rng_match_the_driver() {
        // `PS2_SquareRoot`: overshoots by one for small squares, unconverged for large inputs (sqrtsim).
        assert_eq!([0, 2, 3, 5, 8, 15, 24, 100, 10000, 250_000, 1_000_000, 4_000_000].map(ps2_sqrt), [0, 1, 2, 3, 3, 4, 5, 11, 101, 518, 1297, 4243]);
        let mut r = Rnd::default();
        let a = (r.rnd(100), r.rnd(100), r.rnd(0));
        let mut r2 = Rnd::default();
        assert_eq!(a, (r2.rnd(100), r2.rnd(100), 0));
        // Sister: an exact square root would give 1000 for 1e6.
        assert_ne!(ps2_sqrt(1_000_000), 1000);
    }

    /// A bank with one sample (two ADPCM frames, end flag) and effect 7: full 3-D, inner 2, outer 30, priority 50, volume 100.
    fn one_sound_bank() -> (Bank, Vec<u8>) {
        let mut shf = 1u32.to_le_bytes().to_vec();
        for w in [0u32, 0, 32, 0x1000, 0, 1, 4, 0, 0] { shf.extend(w.to_le_bytes()); }
        let mut sbf = vec![0u8; 32];
        sbf[2..16].fill(0x77);
        sbf[17] = 1; // end flag on the second frame
        let mut sfx = 1u32.to_le_bytes().to_vec();
        sfx.extend(7u32.to_le_bytes());
        sfx.extend(12u32.to_le_bytes());
        let mut blk = vec![0u8; 0x44 + 28 * 2];
        for (o, v) in [(4usize, 2i32), (8, 2), (0xc, 30), (0x14, 50), (0x28, 100), (0x40, 1), (0x44 + 12, 100)] { blk[o..o + 4].copy_from_slice(&v.to_le_bytes()); }
        sfx.extend(blk);
        (Bank::parse(&shf, &sfx).unwrap(), sbf)
    }

    fn front_levels(pos: [f32; 3]) -> (f32, f32) {
        let mut d = Driver::new();
        let (b, sbf) = one_sound_bank();
        d.load_bank(4, b, sbf);
        // Facing +z; the screen-left axis is +x (up × forward).
        d.set_listener([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
        let mut out = Vec::new();
        assert_eq!(d.start(&mut out, 7, Some(pos), 1, None), 0);
        match out.first() { Some(Cmd::Play { left, right, .. }) => (*left, *right), o => panic!("{o:?}") }
    }

    #[test]
    fn a_sound_on_the_left_is_louder_on_the_left_and_fades_with_distance() {
        let (l, r) = front_levels([5.0, 0.0, 0.0]);
        assert!(l > r && r >= 0.0, "{l} {r}");
        let (l2, _) = front_levels([20.0, 0.0, 0.0]);
        assert!(l2 < l, "{l2} {l}");
        // Sister: beyond the outer radius the sound is rejected (no voice).
        let mut d = Driver::new();
        let (b, sbf) = one_sound_bank();
        d.load_bank(4, b, sbf);
        d.set_listener([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
        let mut out = Vec::new();
        assert_eq!(d.start(&mut out, 7, Some([40.0, 0.0, 0.0]), 1, None), 1);
        assert!(out.is_empty());
    }

    /// `SFXFadeUpdate` (0xf8c): a fade-down reaches 0 after ~2 s (≈120 frames at 60 Hz) and clears `FadeStatus`;
    /// the music volume tracks it when `fade_music` is set. Sister: with `fade_music` off, music stays full.
    #[test]
    fn fade_down_reaches_zero_in_two_seconds_and_tracks_music() {
        let mut d = Driver::new();
        d.fade_down(true);
        assert_eq!(d.fade_status, 1);
        // 100 frames (< 2 s): still fading, not yet 0.
        for _ in 0..100 { d.update(&mut Vec::new(), &[]); }
        assert!(d.fade > 0 && d.fade < 10000, "mid-fade: {}", d.fade);
        assert_eq!(d.fade_music_volume, d.fade, "music tracks the SFX fade");
        // Past 2 s: clamped to 0, status cleared.
        for _ in 0..40 { d.update(&mut Vec::new(), &[]); }
        assert_eq!(d.fade, 0);
        assert_eq!(d.fade_status, 0);
        // Sister: fade-up starts from 0 and reaches 10000.
        d.fade_up(false);
        assert_eq!(d.fade, 0, "fade-up starts from 0");
        for _ in 0..200 { d.update(&mut Vec::new(), &[]); }
        assert_eq!(d.fade, 10000);
        assert_eq!(d.fade_music_volume, 0, "music not touched when fade_music is off");
    }

    /// `ES_StartDucker` / `ES_UpdateDucker` (0x83c8/0x8438): `DuckerCurrentVolume` creeps toward the target while
    /// within the duration (rate s/6), then fast (s/15) once passed; at 100 it stops. Sister: a sound with no ducker
    /// flag never starts one.
    #[test]
    fn ducker_creeps_toward_target_then_recovers() {
        let mut d = Driver::new();
        // target = 100 - ducker(80) = 20; duration 0 ms → passed immediately → fast recovery toward 100, but the
        // target is 20 first, then 100 once passed. Use a positive duration so the slow phase runs.
        d.start_ducker(20, 1000);
        assert_eq!(d.ducker_volume, 20);
        // Within the 1000 ms duration: slow phase (s/6) toward 20 — ducker_current drops below 100.
        for _ in 0..5 { d.update(&mut Vec::new(), &[]); }
        assert!(d.ducker_current < 100, "ducked: {}", d.ducker_current);
        assert!(d.ducker_current > 20, "not yet at target: {}", d.ducker_current);
        // Past the duration: DuckerVolume becomes 100 and it recovers back up.
        for _ in 0..200 { d.update(&mut Vec::new(), &[]); }
        assert_eq!(d.ducker_volume, 100);
        assert!(d.ducker_current > 20, "recovering: {}", d.ducker_current);
        // Sister: without a ducker, ducker_current stays at 100 (full music volume).
        let mut d2 = Driver::new();
        for _ in 0..50 { d2.update(&mut Vec::new(), &[]); }
        assert_eq!(d2.ducker_current, 100);
    }

    /// `SFXSetEnvironment` (0x1D) + `SFXUpdateEnvironment` (0xe60): the underwater flag, the reverb target clamp,
    /// the ±16/−4 envelope, and the indoors volume. Sister: reverb falls slower than it rises.
    #[test]
    fn environment_envelope_and_underwater() {
        let mut d = Driver::new();
        // Underwater: reverb == -0x6768 → underwater on, target 0.
        d.set_environment(-0x6768, 1);
        assert!(d.underwater);
        assert_eq!(d.reverb_target, 0);
        assert_eq!(d.indoors_target, 50, "indoors → 50");
        // A normal reverb target of 90.
        d.set_environment(90, 0);
        assert!(!d.underwater);
        assert_eq!(d.reverb_target, 90);
        assert_eq!(d.indoors_target, 100, "outdoors → 100");
        // Rising at +16/frame from 0: after 3 frames, 48 (not yet 90).
        for _ in 0..3 { d.update(&mut Vec::new(), &[]); }
        assert_eq!(d.reverb, 48, "rising +16: {}", d.reverb);
        // Sister: falling is at −4/frame (slower). Drop the target to 0 and check it falls by 4 in one frame.
        d.set_environment(0, 0);
        let before = d.reverb;
        d.update(&mut Vec::new(), &[]);
        assert_eq!(d.reverb, before - 4, "falling −4");
        // Out-of-range reverb is clamped.
        d.set_environment(500, 0);
        assert_eq!(d.reverb_target, 100);
    }
}
