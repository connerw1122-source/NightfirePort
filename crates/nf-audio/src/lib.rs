//! Sound output: the SPU's voices mixed in software and played through cpal. Spec: docs/systems/sound.md.
//!
//! The PS2 SPU2 plays each voice at its pitch (0x1000 = one sample per 48 kHz output tick) with a left and right
//! volume (0..0x3FFF); the driver (`SFX.IRX`, ported in `sfx`) decides which samples play at which pitch and volume.
//! This module is the hardware stand-in: 48 kHz voices resampled to the device rate through the SPU's 4-point
//! Gaussian interpolation (G-032d: the nocash PSX-SPX coefficient table), summed and clamped.

pub mod music;
pub mod reverb;
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
    /// `psiSetReverb` / `PS2_SetupReverb`: the SPU reverb depth (EVOL, 0..1) applied to voices with
    /// `ReverbSend`. The SPU's effect preset is mode 4 STUDIO_3 — the exact 32-register set lives in
    /// [`crate::reverb`] (ps2sdk libsd `effect.c`'s Sony preset tables, cross-checked against PCSX2's
    /// SPU2 `V_Reverb`/`DoReverb`), and the wet output runs the SPU2 reverb equations at that preset.
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

/// The SPU reverb: the SPU2 effect area running the game's mode-4 (STUDIO_3) preset through the
/// reverb equations ([`reverb::ReverbCore`]; PCSX2 `V_Core::DoReverb`). The per-voice wet sends
/// (VMIXEL/VMIXER) enter at full scale; the EVOL depth scales the wet return into the mix [M
/// sound.md §5.5; reverb.rs module comment].
struct Reverb {
    /// The reverb delay RAM and equations, at the game's registers.
    core: reverb::ReverbCore,
    /// The current `EVOL` send depth (0..1), scaling the wet return [M].
    depth: f32,
    /// Frame counter: one core tick every two frames — the reverb runs at half the SPU rate
    /// (nocash psx-spx; PCSX2 downsamples by 2).
    frame: usize,
    /// The wet output held between ticks (zero-order hold) [S].
    hold: [f32; 2],
}

impl Reverb {
    fn new(rate: u32) -> Reverb {
        let _ = rate;
        Reverb {
            core: reverb::ReverbCore::new(reverb::ReverbRegs::from_mode(reverb::GAME_EFFECT_MODE), reverb::game_area(0)),
            depth: 0.0,
            frame: 0,
            hold: [0.0; 2],
        }
    }
    /// Process one stereo frame: `send` is the dry signal of a reverb-routed voice; returns the wet output.
    fn process(&mut self, send: [f32; 2]) -> [f32; 2] {
        self.frame += 1;
        if self.frame % 2 == 1 {
            let to_i16 = |x: f32| (x * 32_767.0).clamp(-32_768.0, 32_767.0) as i16;
            let wet = self.core.tick([to_i16(send[0]), to_i16(send[1])]);
            self.hold = [wet[0] as f32 / 32_768.0, wet[1] as f32 / 32_768.0];
        }
        [self.hold[0] * self.depth, self.hold[1] * self.depth]
    }
}

#[derive(Default)]
struct Shared {
    cmds: Vec<Cmd>,
    ended: Vec<u32>,
    playing: Vec<u32>,
    music_played: u64,
    /// The stream hold (`SFXPauseAllStreams` op 0x16 — `MenuManager_Create`/`Delete` with
    /// `pause_sound`): the music playhead freezes while set [M music.md §3.7 `GamePausedMe`].
    paused: bool,
}

/// The mix state the cpal callback owns — the SPU voices, the music queue and the reverb — extracted
/// verbatim from `Output::open`'s closure (fix-pause-audio) so the pause hold is testable headless.
struct Mixer { voices: Vec<Voice>, music: MusicVoice, reverb: Reverb, rate: u32, channels: usize, music_step: f64, step_scale: f64 }

impl Mixer {
    fn mix(&mut self, s: &mut Shared, out: &mut [f32]) {
        for c in s.cmds.drain(..) {
            match c {
                Cmd::Play { voice, pcm, pitch, left, right, delay, reverb } => {
                    self.voices.retain(|v| v.id != voice);
                    self.voices.push(Voice { id: voice, pcm, pos: 0.0, pitch, left, right, wait: (delay * self.rate as f32).round() as usize,
                        env: 1.0, releasing: false, rel_pos: 0.0, reverb });
                }
                Cmd::Set { voice, pitch, left, right } => if let Some(v) = self.voices.iter_mut().find(|v| v.id == voice) { (v.pitch, v.left, v.right) = (pitch, left, right); },
                // Key-off: begin the ADSR release (the voice stays until `env` reaches 0) [M ADSR2 RR].
                Cmd::Stop { voice } => if let Some(v) = self.voices.iter_mut().find(|v| v.id == voice) { v.releasing = true; },
                Cmd::StopAll => { for v in &mut self.voices { v.releasing = true; } }
                Cmd::MusicAppend(pcm) => self.music.queue.extend(pcm),
                Cmd::MusicTruncate(at) => { let keep = at.saturating_sub(self.music.front) as usize; self.music.queue.truncate(keep); }
                Cmd::MusicGain(g) => self.music.gain = g,
                Cmd::MusicStop => self.music = MusicVoice { gain: self.music.gain, ..Default::default() },
                Cmd::Reverb { depth } => self.reverb.depth = depth,
            }
        }
        out.iter_mut().for_each(|x| *x = 0.0);
        let channels = self.channels;
        let frames = out.len() / channels;
        let mut ended = Vec::new();
        // The per-frame dry signal of every reverb-routed voice (fed to the reverb delay line after the mix).
        let mut rev_buf: Vec<[f32; 2]> = vec![[0.0; 2]; frames];
        for v in &mut self.voices {
            let n = v.pcm.data.len();
            let step = v.pitch as f64 * self.step_scale;
            let mut fi = 0usize;
            while fi < frames {
                if v.wait > 0 { v.wait -= 1; fi += 1; continue; }
                let i = v.pos as usize;
                if i >= n {
                    if let Some(l) = v.pcm.loop_start { if l < n { v.pos = l as f64 + (v.pos - n as f64); continue; } }
                    if !v.releasing { ended.push(v.id); break; }
                }
                if v.releasing && v.env <= 0.0 { ended.push(v.id); break; }
                // The SPU's 4-point Gaussian interpolation (G-032d): i = the pitch counter's bit 4–11 [M nocash].
                let smp = resample_gauss(&v.pcm.data, v.pos) as f32 / 32768.0;
                let g = smp * v.env;
                let (li, ri) = (fi * channels, fi * channels + 1);
                match channels { 1 => out[li] += g * (v.left + v.right) * 0.5, _ => { out[li] += g * v.left; out[ri] += g * v.right; } }
                if v.reverb { rev_buf[fi][0] += g * v.left; rev_buf[fi][1] += g * v.right; }
                // Advance the envelope: linear release over RELEASE_SAMPLES [M ADSR2 RR].
                if v.releasing { v.rel_pos += 1.0; v.env = ((1.0 - v.rel_pos / RELEASE_SAMPLES).max(0.0)) as f32; }
                v.pos += step;
                fi += 1;
            }
        }
        self.voices.retain(|v| !ended.contains(&v.id));
        // The SPU reverb: each reverb-routed voice's dry signal feeds the effect area at full send;
        // the wet output returns scaled by EVOL (reverb.rs has the register-exact preset) [M §5.5].
        if self.reverb.depth > 0.0 && channels > 1 {
            for fi in 0..frames {
                let wet = self.reverb.process(rev_buf[fi]);
                let (li, ri) = (fi * channels, fi * channels + 1);
                out[li] += wet[0];
                out[ri] += wet[1];
            }
        }
        // Music: linear resampling 32 kHz → device rate; silence when the queue runs dry. The stream
        // hold (op 0x16) freezes the drain — the playhead waits and the resume continues exactly
        // (`GamePausedMe`; `SFXUpdateStreams` 0xa1a0 won't auto-resume while it is set) [M music.md §3.7].
        if !s.paused {
            for frame in out.chunks_mut(channels) {
                let (Some(a), b) = (self.music.queue.front().copied(), self.music.queue.get(1).copied()) else { break };
                let b = b.unwrap_or(a);
                let f = self.music.frac as f32;
                let smp = [0, 1].map(|c| (a[c] as f32 + (b[c] as f32 - a[c] as f32) * f) / 32768.0 * self.music.gain);
                match channels { 1 => frame[0] += (smp[0] + smp[1]) * 0.5, _ => { frame[0] += smp[0]; frame[1] += smp[1]; } }
                self.music.frac += self.music_step;
                while self.music.frac >= 1.0 && !self.music.queue.is_empty() { self.music.frac -= 1.0; self.music.queue.pop_front(); self.music.front += 1; }
            }
        }
        for x in out.iter_mut() { *x = x.clamp(-1.0, 1.0); }
        s.ended.extend(ended);
        s.playing = self.voices.iter().map(|v| v.id).collect();
        s.music_played = self.music.front;
    }
}

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

/// The SPU's 4-point Gaussian interpolation coefficients (G-032d): 512 16-bit entries from the nocash PSX-SPX
/// documentation, "SPU ADPCM Sample Decoding" → "4-Point Gaussian Interpolation" (problemkaputt.de/psx-spx.htm;
/// the PS2 SPU2 uses the same table and formula — nocash documents the SPU2 pitch counter's bit 4–11 as the
/// 8-bit interpolation index). Cross-check: the 512 entries are identical to duckstation's
/// `gauss` table (src/core/spu.cpp, `GenerateInterpolationCoefficients`) — independent reference;
/// PCSX2's SPU2 gaussian is the same table (recorded in docs/reports/spu-gaussian-reverb.md). The
/// table is slightly bugged on hardware: each four taps
/// (gauss[i], gauss[0xFF−i], gauss[0x100+i], gauss[0x1FF−i]) should sum to 0x8000 but sum to 0x7F7F..0x7F81 —
/// the `gauss_quads_sum_near_0x8000` test pins that property [M nocash psx-spx].
pub const GAUSS: [i16; 512] = [
    (-0x0001), (-0x0001), (-0x0001), (-0x0001), (-0x0001), (-0x0001), (-0x0001), (-0x0001),
    (-0x0001), (-0x0001), (-0x0001), (-0x0001), (-0x0001), (-0x0001), (-0x0001), (-0x0001),
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0001,
    0x0001, 0x0001, 0x0001, 0x0002, 0x0002, 0x0002, 0x0003, 0x0003,
    0x0003, 0x0004, 0x0004, 0x0005, 0x0005, 0x0006, 0x0007, 0x0007,
    0x0008, 0x0009, 0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x000E,
    0x000F, 0x0010, 0x0011, 0x0012, 0x0013, 0x0015, 0x0016, 0x0018,
    0x0019, 0x001B, 0x001C, 0x001E, 0x0020, 0x0021, 0x0023, 0x0025,
    0x0027, 0x0029, 0x002C, 0x002E, 0x0030, 0x0033, 0x0035, 0x0038,
    0x003A, 0x003D, 0x0040, 0x0043, 0x0046, 0x0049, 0x004D, 0x0050,
    0x0054, 0x0057, 0x005B, 0x005F, 0x0063, 0x0067, 0x006B, 0x006F,
    0x0074, 0x0078, 0x007D, 0x0082, 0x0087, 0x008C, 0x0091, 0x0096,
    0x009C, 0x00A1, 0x00A7, 0x00AD, 0x00B3, 0x00BA, 0x00C0, 0x00C7,
    0x00CD, 0x00D4, 0x00DB, 0x00E3, 0x00EA, 0x00F2, 0x00FA, 0x0101,
    0x010A, 0x0112, 0x011B, 0x0123, 0x012C, 0x0135, 0x013F, 0x0148,
    0x0152, 0x015C, 0x0166, 0x0171, 0x017B, 0x0186, 0x0191, 0x019C,
    0x01A8, 0x01B4, 0x01C0, 0x01CC, 0x01D9, 0x01E5, 0x01F2, 0x0200,
    0x020D, 0x021B, 0x0229, 0x0237, 0x0246, 0x0255, 0x0264, 0x0273,
    0x0283, 0x0293, 0x02A3, 0x02B4, 0x02C4, 0x02D6, 0x02E7, 0x02F9,
    0x030B, 0x031D, 0x0330, 0x0343, 0x0356, 0x036A, 0x037E, 0x0392,
    0x03A7, 0x03BC, 0x03D1, 0x03E7, 0x03FC, 0x0413, 0x042A, 0x0441,
    0x0458, 0x0470, 0x0488, 0x04A0, 0x04B9, 0x04D2, 0x04EC, 0x0506,
    0x0520, 0x053B, 0x0556, 0x0572, 0x058E, 0x05AA, 0x05C7, 0x05E4,
    0x0601, 0x061F, 0x063E, 0x065C, 0x067C, 0x069B, 0x06BB, 0x06DC,
    0x06FD, 0x071E, 0x0740, 0x0762, 0x0784, 0x07A7, 0x07CB, 0x07EF,
    0x0813, 0x0838, 0x085D, 0x0883, 0x08A9, 0x08D0, 0x08F7, 0x091E,
    0x0946, 0x096F, 0x0998, 0x09C1, 0x09EB, 0x0A16, 0x0A40, 0x0A6C,
    0x0A98, 0x0AC4, 0x0AF1, 0x0B1E, 0x0B4C, 0x0B7A, 0x0BA9, 0x0BD8,
    0x0C07, 0x0C38, 0x0C68, 0x0C99, 0x0CCB, 0x0CFD, 0x0D30, 0x0D63,
    0x0D97, 0x0DCB, 0x0E00, 0x0E35, 0x0E6B, 0x0EA1, 0x0ED7, 0x0F0F,
    0x0F46, 0x0F7F, 0x0FB7, 0x0FF1, 0x102A, 0x1065, 0x109F, 0x10DB,
    0x1116, 0x1153, 0x118F, 0x11CD, 0x120B, 0x1249, 0x1288, 0x12C7,
    0x1307, 0x1347, 0x1388, 0x13C9, 0x140B, 0x144D, 0x1490, 0x14D4,
    0x1517, 0x155C, 0x15A0, 0x15E6, 0x162C, 0x1672, 0x16B9, 0x1700,
    0x1747, 0x1790, 0x17D8, 0x1821, 0x186B, 0x18B5, 0x1900, 0x194B,
    0x1996, 0x19E2, 0x1A2E, 0x1A7B, 0x1AC8, 0x1B16, 0x1B64, 0x1BB3,
    0x1C02, 0x1C51, 0x1CA1, 0x1CF1, 0x1D42, 0x1D93, 0x1DE5, 0x1E37,
    0x1E89, 0x1EDC, 0x1F2F, 0x1F82, 0x1FD6, 0x202A, 0x207F, 0x20D4,
    0x2129, 0x217F, 0x21D5, 0x222C, 0x2282, 0x22DA, 0x2331, 0x2389,
    0x23E1, 0x2439, 0x2492, 0x24EB, 0x2545, 0x259E, 0x25F8, 0x2653,
    0x26AD, 0x2708, 0x2763, 0x27BE, 0x281A, 0x2876, 0x28D2, 0x292E,
    0x298B, 0x29E7, 0x2A44, 0x2AA1, 0x2AFF, 0x2B5C, 0x2BBA, 0x2C18,
    0x2C76, 0x2CD4, 0x2D33, 0x2D91, 0x2DF0, 0x2E4F, 0x2EAE, 0x2F0D,
    0x2F6C, 0x2FCC, 0x302B, 0x308B, 0x30EA, 0x314A, 0x31AA, 0x3209,
    0x3269, 0x32C9, 0x3329, 0x3389, 0x33E9, 0x3449, 0x34A9, 0x3509,
    0x3569, 0x35C9, 0x3629, 0x3689, 0x36E8, 0x3748, 0x37A8, 0x3807,
    0x3867, 0x38C6, 0x3926, 0x3985, 0x39E4, 0x3A43, 0x3AA2, 0x3B00,
    0x3B5F, 0x3BBD, 0x3C1B, 0x3C79, 0x3CD7, 0x3D35, 0x3D92, 0x3DEF,
    0x3E4C, 0x3EA9, 0x3F05, 0x3F62, 0x3FBD, 0x4019, 0x4074, 0x40D0,
    0x412A, 0x4185, 0x41DF, 0x4239, 0x4292, 0x42EB, 0x4344, 0x439C,
    0x43F4, 0x444C, 0x44A3, 0x44FA, 0x4550, 0x45A6, 0x45FC, 0x4651,
    0x46A6, 0x46FA, 0x474E, 0x47A1, 0x47F4, 0x4846, 0x4898, 0x48E9,
    0x493A, 0x498A, 0x49D9, 0x4A29, 0x4A77, 0x4AC5, 0x4B13, 0x4B5F,
    0x4BAC, 0x4BF7, 0x4C42, 0x4C8D, 0x4CD7, 0x4D20, 0x4D68, 0x4DB0,
    0x4DF7, 0x4E3E, 0x4E84, 0x4EC9, 0x4F0E, 0x4F52, 0x4F95, 0x4FD7,
    0x5019, 0x505A, 0x509A, 0x50DA, 0x5118, 0x5156, 0x5194, 0x51D0,
    0x520C, 0x5247, 0x5281, 0x52BA, 0x52F3, 0x532A, 0x5361, 0x5397,
    0x53CC, 0x5401, 0x5434, 0x5467, 0x5499, 0x54CA, 0x54FA, 0x5529,
    0x5558, 0x5585, 0x55B2, 0x55DE, 0x5609, 0x5632, 0x565B, 0x5684,
    0x56AB, 0x56D1, 0x56F6, 0x571B, 0x573E, 0x5761, 0x5782, 0x57A3,
    0x57C3, 0x57E2, 0x57FF, 0x581C, 0x5838, 0x5853, 0x586D, 0x5886,
    0x589E, 0x58B5, 0x58CB, 0x58E0, 0x58F4, 0x5907, 0x5919, 0x592A,
    0x593A, 0x5949, 0x5958, 0x5965, 0x5971, 0x597C, 0x5986, 0x598F,
    0x5997, 0x599E, 0x59A4, 0x59A9, 0x59AD, 0x59B0, 0x59B2, 0x59B3,
];

/// One SPU interpolation step at the fractional position `pos` (the pitch counter / 4096): the 4 most recent
/// samples through the Gaussian table [M nocash psx-spx "4-Point Gaussian Interpolation"]:
/// `out = (gauss[0xFF−i]·oldest + gauss[0x1FF−i]·older + gauss[0x100+i]·old + gauss[i]·new) SAR 15` with
/// `i` = the pitch counter's bit 4–11 (= floor(frac·256)), each product shifted separately. `new`..`oldest`
/// are x[n]..x[n−3] (n = floor(pos)); taps before the sample start (and past the end, which the voice loop
/// treats as end/loop) read 0 — the SPU's ring buffer starts zeroed, so a voice's first samples ramp in [S].
pub fn resample_gauss(data: &[i16], pos: f64) -> i16 {
    let n = pos as i64;
    let i = (((pos - n as f64) * 256.0) as i64).clamp(0, 255) as usize;
    let tap = |k: i64| if k < 0 { 0i16 } else { data.get(k as usize).copied().unwrap_or(0) };
    let (new, old, older, oldest) = (tap(n), tap(n - 1), tap(n - 2), tap(n - 3));
    let g = |k: usize| GAUSS[k] as i32;
    let out = (g(0xFF - i) * oldest as i32 >> 15)
        + (g(0x1FF - i) * older as i32 >> 15)
        + (g(0x100 + i) * old as i32 >> 15)
        + (g(i) * new as i32 >> 15);
    out.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

/// `NF_SILENT` is set (any value): open no audio device. The test runner (tools/test/suite.py) sets it.
pub fn silent() -> bool { std::env::var_os("NF_SILENT").is_some() }

impl Output {
    /// Opens the default output device. `None` when there is none (the game runs silent), or when
    /// `NF_SILENT` is set: the test runner sets it so no test ever plays through the user's speakers
    /// (the weapon-sound tests once fired bursts of shots at full volume through them).
    pub fn open() -> Option<Output> {
        if silent() { return None; }
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let format = config.sample_format();
        let cfg: cpal::StreamConfig = config.into();
        let (rate, channels) = (cfg.sample_rate, cfg.channels as usize);
        let shared = Arc::new(Mutex::new(Shared::default()));
        let sh = shared.clone();
        let mut mixer = Mixer { voices: Vec::new(), music: MusicVoice::default(), reverb: Reverb::new(rate), rate, channels,
            music_step: MUSIC_RATE / rate as f64, step_scale: SPU_RATE / rate as f64 / 4096.0 };
        let mut mix = move |out: &mut [f32]| {
            let mut s = sh.lock().unwrap();
            mixer.mix(&mut s, out);
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

    /// The stream hold (`SFXPauseAllStreams` op 0x16 / unpause op 0x17): while set, the music
    /// playhead freezes — the queue's front stops draining, so the resume continues the stream
    /// exactly where it held [M music.md §3.7 `GamePausedMe`]. Voices are NOT held here — a voice
    /// keyed on while the pause is up (the menu's own sounds) still sounds [M §3.2].
    pub fn set_paused(&self, on: bool) { self.shared.lock().unwrap().paused = on; }

    /// The hold state (live checks).
    pub fn paused(&self) -> bool { self.shared.lock().unwrap().paused }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The nocash table's own cross-check: each four taps (gauss[i], gauss[0xFF−i], gauss[0x100+i],
    /// gauss[0x1FF−i]) should sum to 0x8000 but hardware sums to 0x7F7F..0x7F81 [M nocash psx-spx].
    /// A corrupted or mirrored table violates it.
    #[test]
    fn gauss_quads_sum_near_0x8000() {
        for i in 0..256 {
            let s = GAUSS[i] as i32 + GAUSS[0xFF - i] as i32 + GAUSS[0x100 + i] as i32 + GAUSS[0x1FF - i] as i32;
            assert!((0x7F7F..=0x7F81).contains(&s), "quad {i}: {s:#x}");
        }
    }

    /// G-032d sample-exact: one VAG (decoded PCM [1000, −2000, 3000, −4000, 5000, −6000, 7000]) through the
    /// Gaussian resampler. Expected values computed from the nocash formula
    /// `out = Σ gauss[tap_index]·x SAR 15` (an independent implementation of the same doc formula), with
    /// i = floor(frac·256) and the taps new..oldest = x[n]..x[n−3]. At pitch 0x1000 the voice emits one output
    /// per source sample: pos 1.0, 2.0, … (the first pos-0 output is the zeroed-buffer ramp-in, ≈ 0).
    #[test]
    fn one_vag_resampled_sample_exact() {
        let x = [1000i16, -2000, 3000, -4000, 5000, -6000, 7000];
        // Pitch 0x1000 (48 kHz → one source sample per output): pos 1.0..7.0.
        let got: Vec<i16> = (1..=7).map(|k| resample_gauss(&x, k as f64)).collect();
        assert_eq!(got, [148, 401, -811, 1212, -1621, 2023, -2432]);
        // Sister 1: pitch 0x2000 skips every other source sample (pos 2, 4, 6).
        let got2: Vec<i16> = [2.0, 4.0, 6.0].map(|p| resample_gauss(&x, p)).to_vec();
        assert_eq!(got2, [401, 1212, 2023]);
        // Sister 2: the fractional index i (the counter's bit 4–11) moves the output: pos 1.25 / 1.5 / 1.75
        // (i = 64 / 128 / 192) interpolate between the pos-1.0 and pos-2.0 answers.
        assert_eq!([1.25, 1.5, 1.75].map(|p| resample_gauss(&x, p)), [299, 460, 530]);
        // Sister 3: the ramp-in from the zeroed buffer — the first output (pos 0, i = 0) is ≈ 0, and linear
        // interpolation would start at x[0] exactly, so the kernel is not a disguised linear one.
        assert_eq!(resample_gauss(&x, 0.0), -1);
        assert_ne!(resample_gauss(&x, 1.5), (x[1] + x[2]) / 2, "not linear midpoint");
    }

    /// The pause at the output (`SFXPauseAllStreams` op 0x16 / `SFXUnPauseAllStreams` op 0x17 —
    /// `MenuManager_Create`/`Delete` with `pause_sound` [M 0x21b908:121-124, 0x21c2b0:11-22]): the
    /// music queue's playhead (`front`) freezes and resumes exactly; a voice pitched to 0 by the
    /// driver's pause edge (`psiSamplePause` 0xf46c: saved pitch, pitch 0) holds its sample position
    /// and resumes when the recompute restores the pitch [M sound.md §8]. A voice NOT pitched to 0
    /// keeps advancing while the pause is up — the menu's own sounds key on under the pause [M §3.2].
    #[test]
    fn the_pause_holds_the_music_playhead_and_the_pitch0_voice_resumes() {
        let mut s = Shared::default();
        let mut m = Mixer { voices: Vec::new(), music: MusicVoice::default(), reverb: Reverb::new(48_000), rate: 48_000, channels: 2, music_step: MUSIC_RATE / 48_000.0, step_scale: SPU_RATE / 48_000.0 / 4096.0 };
        // A looping voice at pitch 0x1000 (one source sample per output) — the ambient loop.
        s.cmds.push(Cmd::Play { voice: 1, pcm: std::sync::Arc::new(Pcm { data: (0..1000).map(|i| (i * 7) as i16).collect(), loop_start: Some(0) }), pitch: 0x1000, left: 1.0, right: 1.0, delay: 0.0, reverb: false });
        // Music: half a second queued (the port's buffered stream).
        s.cmds.push(Cmd::MusicAppend(vec![[100i16, -100]; 16_000]));
        let mut buf = vec![0.0f32; 480]; // 240 stereo frames per mix
        m.mix(&mut s, &mut buf);
        let (p1, m1) = (m.voices[0].pos, s.music_played);
        m.mix(&mut s, &mut buf);
        let (p2, m2) = (m.voices[0].pos, s.music_played);
        assert!(p2 > p1, "unpaused the voice advances (sister): {p1} {p2}");
        assert!(m2 > m1, "unpaused the music playhead advances (sister): {m1} {m2}");
        // The pause: the driver's pitch-0 edge + the stream hold reach the output.
        s.cmds.push(Cmd::Set { voice: 1, pitch: 0, left: 0.0, right: 0.0 });
        s.paused = true;
        m.mix(&mut s, &mut buf);
        let (p3, m3) = (m.voices[0].pos, s.music_played);
        m.mix(&mut s, &mut buf);
        let (p4, m4) = (m.voices[0].pos, s.music_played);
        assert_eq!(p3, p4, "paused the pitch-0 voice's sample position holds");
        assert_eq!(m3, m4, "paused the music playhead holds");
        // The unpause: the driver's recompute restores the pitch; both resume.
        s.cmds.push(Cmd::Set { voice: 1, pitch: 0x1000, left: 1.0, right: 1.0 });
        s.paused = false;
        m.mix(&mut s, &mut buf);
        let (p5, m5) = (m.voices[0].pos, s.music_played);
        m.mix(&mut s, &mut buf);
        assert!(m.voices[0].pos > p5, "unpaused the voice resumes");
        assert!(s.music_played > m5, "unpaused the music resumes");
        // Sister: a voice started while the pause is up (the menu click) still advances.
        s.paused = true;
        s.cmds.push(Cmd::Play { voice: 2, pcm: std::sync::Arc::new(Pcm { data: (0..1000).map(|i| (i * 11) as i16).collect(), loop_start: Some(0) }), pitch: 0x1000, left: 1.0, right: 1.0, delay: 0.0, reverb: false });
        m.mix(&mut s, &mut buf);
        let q1 = m.voices.iter().find(|v| v.id == 2).unwrap().pos;
        m.mix(&mut s, &mut buf);
        let q2 = m.voices.iter().find(|v| v.id == 2).unwrap().pos;
        assert!(q2 > q1, "a while-paused start still advances (the menu click): {q1} {q2}");
    }
}
