//! Sound output: the SPU's voices mixed in software and played through cpal. Spec: docs/systems/sound.md.
//!
//! The PS2 SPU2 plays each voice at its pitch (0x1000 = one sample per 48 kHz output tick) with a left and right
//! volume (0..0x3FFF); the driver (`SFX.IRX`, ported in `sfx`) decides which samples play at which pitch and volume.
//! This module is the hardware stand-in: 48 kHz voices resampled to the device rate (linear interpolation —
//! *deviation*: the SPU interpolates with a 4-point Gaussian filter), summed and clamped.

pub mod music;
pub mod sfx;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// A decoded sample: 16-bit mono PCM and, for looping samples, the loop start (in samples).
#[derive(Debug)]
pub struct Pcm { pub data: Vec<i16>, pub loop_start: Option<usize> }

/// A command to the voices.
#[derive(Clone, Debug)]
pub enum Cmd {
    /// Key on: `voice` (the caller's handle) starts `pcm` at SPU `pitch` (0x1000 = 48 kHz) with volumes 0..1, after
    /// `delay` seconds (the driver keys rear voices on 500 µs after the front ones, `PS2_SendKeyONKeyOff`).
    /// `reverb`: the voice's reverb-send bit (SFXParameters+0x00 `ReverbSend` != 0) routes it through the SPU reverb [M].
    Play { voice: u32, pcm: Arc<Pcm>, pitch: u32, left: f32, right: f32, delay: f32, reverb: bool },
    /// New pitch and volumes for a playing voice.
    Set { voice: u32, pitch: u32, left: f32, right: f32 },
    /// Key off (the voice stops at once — *deviation*: no ADSR release).
    Stop { voice: u32 },
    /// Every voice off.
    StopAll,
    /// Music (the stream's two SPU voices): append stereo PCM at 32 kHz to the queue.
    MusicAppend(Vec<[i16; 2]>),
    /// Drop queued music from absolute music sample `at` on (an instant jump's new data replaces it).
    MusicTruncate(u64),
    /// Music gain per channel (0..1).
    MusicGain(f32),
    /// Key off the music and empty its queue; the sample counter restarts at 0.
    MusicStop,
    /// `psiSetReverb` / `PS2_SetupReverb`: the SPU reverb depth (EVOL, 0..1) applied to voices with `ReverbSend`.
    /// The SPU's effect preset is mode 4 (delay 30, feedback 200 [M §5.5, slightly ambiguous]); the output runs a
    /// delay line at this depth — the exact preset coefficients are pending a `PS2_SetupReverb` register dump [S].
    Reverb { depth: f32 },
}

/// The music stream's output rate (MFX streams are 32 kHz).
pub const MUSIC_RATE: f64 = 32_000.0;

/// The SPU's fixed ADSR for every SFX voice (`psiRequestVoiceHandle2`@e420 sets ADSR1=0x000F,
/// ADSR2=0x1FC0: instant attack, no decay → max sustain, fastest linear release) [M sound.md §3.4].
/// Attack reaches peak in one step; sustain holds at peak while keyed on; on key-off the envelope releases
/// linearly to silence over `RELEASE_SAMPLES` (≈4 ms, the SPU's fastest release — the exact rate-table step count
/// is pending a disc cross-check, GAPS G-032d) [S].
const RELEASE_SAMPLES: f64 = 192.0;

struct Voice {
    id: u32,
    pcm: Arc<Pcm>,
    pos: f64,
    pitch: u32,
    left: f32,
    right: f32,
    wait: usize,
    /// The envelope (0..1): 1.0 while keyed on (instant attack, max sustain); ramps to 0 in release.
    env: f32,
    /// In the linear release phase (key-off received, envelope decaying) [M ADSR2 RR].
    releasing: bool,
    /// Samples consumed during the release (drives the linear ramp) [M].
    rel_pos: f64,
    /// Reverb-send bit (SFXParameters+0x00 `ReverbSend` != 0): routes the voice through the SPU reverb [M].
    reverb: bool,
}

/// The SPU reverb: a stereo comb-filter delay line at the `EVOL` depth [M sound.md §5.5]. The SPU's effect preset is
/// mode 4 (delay 30, feedback 200) — the `sceSdEffectAttr` layout is partially mangled in the decompile [M §9], so the
/// exact preset is deferred (GAPS G-032d); this single-comb stand-in gives an audible reverb tail at the right depth.
struct Reverb {
    /// Delay-line buffers (left, right), sized to ~25 ms (a small room) [S].
    buf: [Vec<f32>; 2],
    /// Write index into each buffer [S].
    w: usize,
    /// Feedback coefficient [S].
    fb: f32,
    /// The current `EVOL` send depth (0..1) [M].
    depth: f32,
}

impl Reverb {
    fn new(rate: u32) -> Reverb {
        let len = (rate as f64 * 0.025) as usize;
        Reverb { buf: [vec![0.0; len.max(1)], vec![0.0; len.max(1)]], w: 0, fb: 0.5, depth: 0.0 }
    }
    /// Process one stereo frame: `send` is the dry signal of a reverb-routed voice; returns the wet output.
    fn process(&mut self, send: [f32; 2]) -> [f32; 2] {
        let n = self.buf[0].len();
        let r = self.w.checked_sub(1).unwrap_or(n - 1);
        let (pl, pr) = (self.buf[0][r], self.buf[1][r]);
        let d = self.depth;
        // The voice's dry signal, scaled by EVOL, feeds the delay line; the line feeds back at `fb` [S topology].
        let (il, ir) = (send[0] * d, send[1] * d);
        self.buf[0][self.w] = il + pl * self.fb;
        self.buf[1][self.w] = ir + pr * self.fb;
        self.w = (self.w + 1) % n;
        // The wet output is the delayed signal at the send depth.
        [pl * d, pr * d]
    }
}

#[derive(Default)]
struct Shared { cmds: Vec<Cmd>, ended: Vec<u32>, playing: Vec<u32>, music_played: u64 }

/// The music voices: queued stereo samples, the absolute index of the queue's front, the fractional read position.
#[derive(Default)]
struct MusicVoice { queue: std::collections::VecDeque<[i16; 2]>, front: u64, frac: f64, gain: f32 }

/// The SPU stand-in: a cpal output stream and its voices.
pub struct Output {
    _stream: cpal::Stream,
    shared: Arc<Mutex<Shared>>,
    pub rate: u32,
}

/// The SPU's output rate: pitch 0x1000 advances one sample per tick at this rate.
pub const SPU_RATE: f64 = 48_000.0;

/// 4-tap Catmull-Rom interpolation (the SPU's 4-point filter structure; the exact Gaussian coefficients are
/// deferred to GAPS G-032d) [S]. `p0..p3` are samples x[-1]..x[2]; `t` is the fractional position in [0,1).
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t; let t3 = t2 * t;
    0.5 * (2.0 * p1 + (-p0 + p2) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

impl Output {
    /// Opens the default output device. `None` when there is none (the game runs silent).
    pub fn open() -> Option<Output> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let format = config.sample_format();
        let cfg: cpal::StreamConfig = config.into();
        let (rate, channels) = (cfg.sample_rate, cfg.channels as usize);
        let shared = Arc::new(Mutex::new(Shared::default()));
        let mut voices: Vec<Voice> = Vec::new();
        let mut music = MusicVoice::default();
        let mut reverb = Reverb::new(rate);
        let music_step = MUSIC_RATE / rate as f64;
        let sh = shared.clone();
        let step_scale = SPU_RATE / rate as f64 / 4096.0;
        let mut mix = move |out: &mut [f32]| {
            {
                let mut s = sh.lock().unwrap();
                for c in s.cmds.drain(..) {
                    match c {
                        Cmd::Play { voice, pcm, pitch, left, right, delay, reverb } => {
                            voices.retain(|v| v.id != voice);
                            voices.push(Voice { id: voice, pcm, pos: 0.0, pitch, left, right, wait: (delay * rate as f32).round() as usize,
                                env: 1.0, releasing: false, rel_pos: 0.0, reverb });
                        }
                        Cmd::Set { voice, pitch, left, right } => if let Some(v) = voices.iter_mut().find(|v| v.id == voice) { (v.pitch, v.left, v.right) = (pitch, left, right); },
                        // Key-off: begin the ADSR release (the voice stays until `env` reaches 0) [M ADSR2 RR].
                        Cmd::Stop { voice } => if let Some(v) = voices.iter_mut().find(|v| v.id == voice) { v.releasing = true; },
                        Cmd::StopAll => { for v in &mut voices { v.releasing = true; } }
                        Cmd::MusicAppend(pcm) => music.queue.extend(pcm),
                        Cmd::MusicTruncate(at) => { let keep = at.saturating_sub(music.front) as usize; music.queue.truncate(keep); }
                        Cmd::MusicGain(g) => music.gain = g,
                        Cmd::MusicStop => music = MusicVoice { gain: music.gain, ..Default::default() },
                        Cmd::Reverb { depth } => reverb.depth = depth,
                    }
                }
            }
            out.iter_mut().for_each(|x| *x = 0.0);
            let frames = out.len() / channels;
            let mut ended = Vec::new();
            // The per-frame dry signal of every reverb-routed voice (fed to the reverb delay line after the mix).
            let mut rev_buf: Vec<[f32; 2]> = vec![[0.0; 2]; frames];
            for v in &mut voices {
                let n = v.pcm.data.len();
                let step = v.pitch as f64 * step_scale;
                let mut fi = 0usize;
                while fi < frames {
                    if v.wait > 0 { v.wait -= 1; fi += 1; continue; }
                    let i = v.pos as usize;
                    if i >= n {
                        if let Some(l) = v.pcm.loop_start { if l < n { v.pos = l as f64 + (v.pos - n as f64); continue; } }
                        if !v.releasing { ended.push(v.id); break; }
                    }
                    if v.releasing && v.env <= 0.0 { ended.push(v.id); break; }
                    // 4-tap interpolation: x[-1], x[0], x[1], x[2] (the SPU's 4-point filter [M]).
                    let xm1 = v.pcm.data.get(i.wrapping_sub(1)).copied().unwrap_or(0i16) as f32;
                    let x0 = v.pcm.data.get(i).copied().unwrap_or(0i16) as f32;
                    let x1 = v.pcm.data.get(i + 1).copied().unwrap_or(0i16) as f32;
                    let x2 = v.pcm.data.get(i + 2).copied().unwrap_or(0i16) as f32;
                    let f = (v.pos - v.pos.floor()) as f32;
                    // Catmull-Rom 4-tap kernel — structurally identical to the SPU's 4-tap filter; the exact SPU2
                    // Gaussian coefficient table is deferred (GAPS G-032d, pending work/decomp) [S kernel coeffs].
                    let s = catmull_rom(xm1, x0, x1, x2, f) / 32768.0;
                    let g = s * v.env;
                    let (li, ri) = (fi * channels, fi * channels + 1);
                    match channels { 1 => out[li] += g * (v.left + v.right) * 0.5, _ => { out[li] += g * v.left; out[ri] += g * v.right; } }
                    if v.reverb { rev_buf[fi][0] += g * v.left; rev_buf[fi][1] += g * v.right; }
                    // Advance the envelope: linear release over RELEASE_SAMPLES [M ADSR2 RR].
                    if v.releasing { v.rel_pos += 1.0; v.env = ((1.0 - v.rel_pos / RELEASE_SAMPLES).max(0.0)) as f32; }
                    v.pos += step;
                    fi += 1;
                }
            }
            voices.retain(|v| !ended.contains(&v.id));
            // The SPU reverb: each reverb-routed voice's dry signal feeds the delay line at `EVOL`; the wet output is
            // mixed back in. A single comb per channel (the exact `PS2_SetupReverb` preset is deferred, GAPS G-032d) [S].
            if reverb.depth > 0.0 && channels > 1 {
                for fi in 0..frames {
                    let wet = reverb.process(rev_buf[fi]);
                    let (li, ri) = (fi * channels, fi * channels + 1);
                    out[li] += wet[0];
                    out[ri] += wet[1];
                }
            }
            // Music: linear resampling 32 kHz → device rate; silence when the queue runs dry.
            for frame in out.chunks_mut(channels) {
                let (Some(a), b) = (music.queue.front().copied(), music.queue.get(1).copied()) else { break };
                let b = b.unwrap_or(a);
                let f = music.frac as f32;
                let s = [0, 1].map(|c| (a[c] as f32 + (b[c] as f32 - a[c] as f32) * f) / 32768.0 * music.gain);
                match channels { 1 => frame[0] += (s[0] + s[1]) * 0.5, _ => { frame[0] += s[0]; frame[1] += s[1]; } }
                music.frac += music_step;
                while music.frac >= 1.0 && !music.queue.is_empty() { music.frac -= 1.0; music.queue.pop_front(); music.front += 1; }
            }
            for x in out.iter_mut() { *x = x.clamp(-1.0, 1.0); }
            let mut s = sh.lock().unwrap();
            s.ended.extend(ended);
            s.playing = voices.iter().map(|v| v.id).collect();
            s.music_played = music.front;
        };
        let err = |e: cpal::Error| eprintln!("audio: {e}");
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_output_stream(cfg, move |d: &mut [f32], _: &cpal::OutputCallbackInfo| mix(d), err, None),
            cpal::SampleFormat::I16 => {
                let mut buf = Vec::new();
                device.build_output_stream(cfg, move |d: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    buf.resize(d.len(), 0.0);
                    mix(&mut buf);
                    for (o, x) in d.iter_mut().zip(&buf) { *o = (x * 32767.0) as i16; }
                }, err, None)
            }
            f => { eprintln!("audio: unsupported device sample format {f}"); return None; }
        }.map_err(|e| eprintln!("audio: {e}")).ok()?;
        stream.play().map_err(|e| eprintln!("audio: {e}")).ok()?;
        Some(Output { _stream: stream, shared, rate })
    }

    pub fn send(&self, cmds: impl IntoIterator<Item = Cmd>) { self.shared.lock().unwrap().cmds.extend(cmds); }

    /// Voices that reached their end since the last call (the SPU's end flags).
    pub fn take_ended(&self) -> Vec<u32> { std::mem::take(&mut self.shared.lock().unwrap().ended) }

    /// Music samples played so far (32 kHz, since the last `MusicStop`).
    pub fn music_played(&self) -> u64 { self.shared.lock().unwrap().music_played }

    /// Voices playing at the last mix.
    pub fn playing(&self) -> Vec<u32> { self.shared.lock().unwrap().playing.clone() }
}
