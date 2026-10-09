//! Sound output: the SPU's voices mixed in software and played through cpal. Spec: docs/systems/sound.md.
//!
//! The PS2 SPU2 plays each voice at its pitch (0x1000 = one sample per 48 kHz output tick) with a left and right
//! volume (0..0x3FFF); the driver (`SFX.IRX`, ported in `sfx`) decides which samples play at which pitch and volume.
//! This module is the hardware stand-in: 48 kHz voices resampled to the device rate (linear interpolation —
//! *deviation*: the SPU interpolates with a 4-point Gaussian filter), summed and clamped.

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
    Play { voice: u32, pcm: Arc<Pcm>, pitch: u32, left: f32, right: f32, delay: f32 },
    /// New pitch and volumes for a playing voice.
    Set { voice: u32, pitch: u32, left: f32, right: f32 },
    /// Key off (the voice stops at once — *deviation*: no ADSR release).
    Stop { voice: u32 },
    /// Every voice off.
    StopAll,
}

struct Voice { id: u32, pcm: Arc<Pcm>, pos: f64, pitch: u32, left: f32, right: f32, wait: usize }

#[derive(Default)]
struct Shared { cmds: Vec<Cmd>, ended: Vec<u32>, playing: Vec<u32> }

/// The SPU stand-in: a cpal output stream and its voices.
pub struct Output {
    _stream: cpal::Stream,
    shared: Arc<Mutex<Shared>>,
    pub rate: u32,
}

/// The SPU's output rate: pitch 0x1000 advances one sample per tick at this rate.
pub const SPU_RATE: f64 = 48_000.0;

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
        let sh = shared.clone();
        let step_scale = SPU_RATE / rate as f64 / 4096.0;
        let mut mix = move |out: &mut [f32]| {
            {
                let mut s = sh.lock().unwrap();
                for c in s.cmds.drain(..) {
                    match c {
                        Cmd::Play { voice, pcm, pitch, left, right, delay } => {
                            voices.retain(|v| v.id != voice);
                            voices.push(Voice { id: voice, pcm, pos: 0.0, pitch, left, right, wait: (delay * rate as f32).round() as usize });
                        }
                        Cmd::Set { voice, pitch, left, right } => if let Some(v) = voices.iter_mut().find(|v| v.id == voice) { (v.pitch, v.left, v.right) = (pitch, left, right); },
                        Cmd::Stop { voice } => voices.retain(|v| v.id != voice),
                        Cmd::StopAll => voices.clear(),
                    }
                }
            }
            out.iter_mut().for_each(|x| *x = 0.0);
            let mut ended = Vec::new();
            for v in &mut voices {
                let n = v.pcm.data.len();
                let step = v.pitch as f64 * step_scale;
                for frame in out.chunks_mut(channels) {
                    if v.wait > 0 { v.wait -= 1; continue; }
                    let i = v.pos as usize;
                    if i >= n {
                        match v.pcm.loop_start { Some(l) if l < n => v.pos = l as f64 + (v.pos - n as f64), _ => { ended.push(v.id); break; } }
                    }
                    let i = v.pos as usize;
                    let f = (v.pos - i as f64) as f32;
                    let a = v.pcm.data[i] as f32;
                    let b = v.pcm.data.get(i + 1).copied().unwrap_or(v.pcm.data[i]) as f32;
                    let s = (a + (b - a) * f) / 32768.0;
                    match channels { 1 => frame[0] += s * (v.left + v.right) * 0.5, _ => { frame[0] += s * v.left; frame[1] += s * v.right; } }
                    v.pos += step;
                }
            }
            voices.retain(|v| !ended.contains(&v.id));
            for x in out.iter_mut() { *x = x.clamp(-1.0, 1.0); }
            let mut s = sh.lock().unwrap();
            s.ended.extend(ended);
            s.playing = voices.iter().map(|v| v.id).collect();
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

    /// Voices playing at the last mix.
    pub fn playing(&self) -> Vec<u32> { self.shared.lock().unwrap().playing.clone() }
}
