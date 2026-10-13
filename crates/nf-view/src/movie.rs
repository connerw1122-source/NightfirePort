//! PSS movie (FMV) support for nf-view: load a movie from the disc image, decode video frames (behind the
//! cargo feature `movies`, i.e. `nf-formats::mpeg2`), decode the PCM audio, and dump frames headlessly
//! (`nf-view --movie 07100001 --frame N --shot out.png` — no GPU needed; the main agent checks the pixels).
//!
//! **Game references** (ACTION.ELF): the player is `psiStartBackgroundMovie__FUiScUi` (0x001081a8) — filename
//! `sprintf("%s%s%8.8x.pss%s", HostName "cdrom0:\movies\", DirName "30_FPS\", code, EndTag ";1")`, gated by the
//! `UsingMovie` whitelist (0x00107eb8); frames land in a 512×448 RGBA32 background (`DecodeBGFMVFrame`
//! 0x00108550, `AllocateMovieMemory` 0x00107d70) at 30 fps (main 0x00109080 skips to even vblanks); skip =
//! action 0x19 (X/Start, newly pressed) in `GameFlow_Main` 0x001c98c8 case 6 / `MenuManager_Update` 0x0021c3d8.

use anyhow::{bail, Context, Result};
use nf_formats::iso9660::IsoImage;
use nf_formats::pss::{demux, Audio, Demuxed};

/// A movie loaded from the disc: the demuxed program stream plus its sequence-header geometry.
pub struct Movie {
    /// Bare file name, e.g. `07100001.PSS`.
    pub name: String,
    /// The movie hashcode (the `%8.8x` filename stem), e.g. 0x07100001.
    pub code: u32,
    pub demuxed: Demuxed,
    pub width: u32,
    pub height: u32,
    /// MPEG frame-rate code from the sequence header (5 = 30 Hz on every movie).
    pub frame_rate_code: u32,
}

impl Movie {
    /// Finds and loads `/MOVIES/30_FPS/<name-or-code>.PSS` (the game builds `cdrom0:\movies\30_FPS\%8.8x.pss;1`
    /// from the hashcode [M psiStartBackgroundMovie 0x001081a8]; the `;1`, the case and the extension are
    /// optional here, so `07100001`, `07100001.PSS` and `0x7100001` all work).
    pub fn load(iso: &IsoImage, name_or_code: &str) -> Result<Movie> {
        let stem = name_or_code.trim_start_matches("0x").trim_end_matches(".PSS").trim_end_matches(".pss");
        let (name, code) = match u32::from_str_radix(stem, 16) {
            Ok(code) if stem.len() == 8 => (format!("{code:08X}.PSS"), code),
            _ => {
                let code = u32::from_str_radix(&stem[..stem.len().min(8)], 16).unwrap_or(0);
                (stem.to_ascii_uppercase() + ".PSS", code)
            }
        };
        let path = format!("/MOVIES/30_FPS/{name}");
        let entry = iso.find(&path).with_context(|| format!("{path} is not in the image tree"))?;
        if entry.size == 0 {
            bail!("{path} is a stub on this image (the slim on-foot ISO has no movie data); point NF_MOVIE_ISO (or NF_ISO) at an image with movies");
        }
        let data = iso.read_file(entry)?;
        let demuxed = demux(&data)?;
        // Sequence header geometry (§6.2.2.1): 12+12 bit sizes, then aspect/frc.
        let seq = demuxed
            .video
            .windows(4)
            .position(|w| w == [0, 0, 1, 0xb3])
            .context("no MPEG sequence header in the video stream")?;
        let b = &demuxed.video[seq + 4..seq + 8];
        let width = ((b[0] as u32) << 4 | (b[1] >> 4) as u32) & 0xfff;
        let height = (((b[1] & 0xf) as u32) << 8 | b[2] as u32) & 0xfff;
        let frame_rate_code = (b[3] & 0xf) as u32;
        Ok(Movie { name, code, demuxed, width, height, frame_rate_code })
    }

    /// The audio stream (Nightfire: one channel, 16-bit LE PCM 48 kHz stereo — kind 1).
    pub fn audio(&self) -> Result<Audio<'_>> {
        let bytes = self.demuxed.audio.values().next().context("movie has no audio stream")?;
        Audio::parse(bytes).map_err(|e| anyhow::Error::msg(e.to_string()))
    }

    /// Decodes frame `n` (display order; B-frames are reordered by the decoder). Slow: the whole ES is decoded;
    /// headless tests and `--shot` only need one or two frames.
    #[cfg(feature = "movies")]
    pub fn decode_frame(&self, n: usize) -> Result<nf_formats::mpeg2::VideoFrame> {
        let frames = nf_formats::mpeg2::decode_video(&self.demuxed.video)?;
        frames.into_iter().nth(n).context(format!("no frame {n} (the movie has {} pictures)", nf_formats::mpeg2::picture_counts(&self.demuxed.video).0))
    }

    /// Decodes frame `n` and writes it as an RGBA PNG (4:2:0 → RGB8, BT.601). Returns the PNG bytes.
    #[cfg(feature = "movies")]
    pub fn shot(&self, n: usize) -> Result<Vec<u8>> {
        let f = self.decode_frame(n)?;
        let rgb = nf_formats::mpeg2::yuv420_to_rgb8(&f);
        Ok(nf_formats::png::encode_rgba(f.width as u32, f.height as u32, &rgb))
    }
}

/// `nf-view --movie <name> --frame N --shot out.png`: demux + decode + PNG dump, headless.
#[cfg(feature = "movies")]
pub fn movie_frame_shot(iso: &IsoImage, movie: &str, frame: usize, out: &std::path::Path) -> Result<()> {
    let m = Movie::load(iso, movie)?;
    let (pics, i, p, b) = nf_formats::mpeg2::picture_counts(&m.demuxed.video);
    eprintln!(
        "{}: {} packs, {} video pkts, {} audio pkts, video {}x{} frc {}, {} pictures (I{i} P{p} B{b}), audio kind {} {} Hz x {} ch, {:.2} s",
        m.name, m.demuxed.packs, m.demuxed.video_packets, m.demuxed.audio_packets, m.width, m.height, m.frame_rate_code, pics,
        m.audio()?.header.kind, m.audio()?.header.rate, m.audio()?.header.channels, m.audio()?.seconds()
    );
    let png = m.shot(frame)?;
    std::fs::write(out, &png)?;
    eprintln!("frame {frame} -> {} ({} bytes)", out.display(), png.len());
    Ok(())
}

/// The `UsingMovie` whitelist [M 0x00107eb8]: the 23 hashcodes `psiStartBackgroundMovie` accepts —
/// `psiStartBackgroundMovie` returns without opening anything for any other code, and
/// `strFileOpen`'s failure path hangs, so the gate is what keeps state 6 short for codes with no
/// file. Briefings 0x7100001/05/09/0c/11/14/18; outros 0x7300004/08/0b/0d/13/17/1b; the specials
/// 0x7330048/0x7350048/0x7360048 (not on the disc) /0x7370048/0x7380048/0x7390048/0x73a0048/0x73b0048/0x73f0048.
pub const USING_MOVIE: [u32; 23] = [
    0x0710_0001, 0x0710_0005, 0x0710_0009, 0x0710_000c, 0x0710_0011, 0x0710_0014, 0x0710_0018,
    0x0730_0004, 0x0730_0008, 0x0730_000b, 0x0730_000d, 0x0730_0013, 0x0730_0017, 0x0730_001b,
    0x0733_0048, 0x0735_0048, 0x0736_0048, 0x0737_0048, 0x0738_0048, 0x0739_0048, 0x073a_0048,
    0x073b_0048, 0x073f_0048,
];

/// Why `Player::start` refused. `NotWhitelisted` mirrors the game exactly (the movie never starts,
/// `BGFMVPlaying` stays 0 and state 6 exits on frame 1 [M 0x107eb8 + 0x1082e8]); `NoMovieData`
/// covers images without the movie data (the game would hang in `strFileOpen` 0x108cc0:107 — the
/// port reports and falls back to the headless countdown instead).
#[derive(Clone, Debug)]
pub enum StartError {
    NotWhitelisted,
    NoMovieData(String),
}

/// One decoded frame (RGBA8) or the end of stream.
enum Decoded { Frame(u32, u32, Vec<u8>), Eof }

/// The movie PCM output: a cpal stream draining a ring the decoder thread feeds per frame
/// (the game's AudioDec IOP ring: `audioDecSendToIOP` per decoded frame 0x107100, SPU2 at 48 kHz
/// stereo [M 0x107360]; the port's mix runs at the device's own rate with a nearest-neighbour
/// resample). Volume 1.0 [S — the game applies `MovieVolume` = `SFXGetVolume()` at the start site;
/// the sfx master volume is not plumbed into the player].
#[cfg(feature = "movies")]
struct PcmOut {
    _stream: cpal::Stream,
    #[allow(dead_code)] // held so the ring outlives the stream callback
    ring: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<i16>>>,
}

/// A playing movie, the port's `psiStartBackgroundMovie` (0x001081a8). The decoder runs on a worker
/// thread with a bounded channel (2 frames in flight — the game's decode/present loop keeps exactly
/// one decoded frame buffered between IPU and blit), the PCM goes through cpal, and `present()`
/// hands out one frame per second call (the game decodes one MPEG frame per 2 vblanks, 30 fps,
/// main 0x00109080's `PS2FramesToSkip = (VBlankCount & ~1) - GameCount`).
///
/// Without the cargo feature `movies` the player declines real playback (`start` still runs the
/// whitelist + file checks, then reports done immediately) so the state machine is testable
/// everywhere.
pub struct Player {
    code: u32,
    rx: Option<std::sync::mpsc::Receiver<Decoded>>,
    last: Option<(u32, u32, Vec<u8>)>,
    tick: u32,
    ended: bool,
    #[cfg(feature = "movies")]
    _pcm: Option<PcmOut>,
}

impl Player {
    /// `psiStartBackgroundMovie__FUiScUi` 0x001081a8 for one of the mission-flow codes: gate on the
    /// `UsingMovie` whitelist, open `cdrom0:\movies\30_FPS\%8.8x.pss;1`, start decoding + audio.
    /// (The game's `loop` and `volume` args are 0 / `SFXGetVolume()` at every mission-flow site
    /// [M]; the port plays once at fixed volume.)
    pub fn start(iso_path: &str, code: u32) -> Result<Player, StartError> {
        if !USING_MOVIE.contains(&code) { return Err(StartError::NotWhitelisted); }
        let iso = nf_formats::iso9660::IsoImage::open(iso_path).map_err(|e| StartError::NoMovieData(e.to_string()))?;
        let m = Movie::load(&iso, &format!("{code:08X}")).map_err(|e| StartError::NoMovieData(e.to_string()))?;
        #[cfg(not(feature = "movies"))]
        { let _ = &m; Ok(Player { code, rx: None, last: None, tick: 0, ended: true }) }
        #[cfg(feature = "movies")]
        {
            use std::sync::mpsc::sync_channel;
            // Audio first: the whole PCM is in memory after the demux; the decoder thread paces it
            // out per video frame like audioDecSendToIOP does (0x107100).
            let pcm: Vec<i16> = m.audio().map(|a| a.decode().into_iter().flat_map(|[l, r]| [l, r]).collect()).unwrap_or_default();
            let rate = m.audio().map(|a| a.header.rate).unwrap_or(48000) as f64;
            let (ring, stream) = open_pcm_out(&pcm, rate);
            let total = nf_formats::mpeg2::picture_counts(&m.demuxed.video).0.max(1);
            let per_frame = pcm.len().div_ceil(total);
            let es = m.demuxed.video.clone();
            let (tx, rx) = sync_channel::<Decoded>(2);
            let pcm_ring = ring.clone();
            std::thread::Builder::new().name(format!("movie-{code:08x}")).spawn(move || {
                let frames = match nf_formats::mpeg2::decode_video(&es) { Ok(f) => f, Err(e) => { eprintln!("movie: decode failed: {e}"); let _ = tx.send(Decoded::Eof); return; } };
                let mut at = 0usize;
                for f in &frames {
                    let rgb = nf_formats::mpeg2::yuv420_to_rgb8(f);
                    // The frame's audio slice goes out ahead of its picture (the game feeds the IOP
                    // ring from the same callback that produces the frame).
                    push_pcm(&pcm_ring, &pcm[at..(at + per_frame).min(pcm.len())]);
                    at = (at + per_frame).min(pcm.len());
                    if tx.send(Decoded::Frame(f.width as u32, f.height as u32, rgb)).is_err() { return; } // the receiver dropped = stop
                }
                let _ = tx.send(Decoded::Eof);
                // Hold the ring reference so the stream keeps draining any tail.
                let _ = &pcm_ring;
            }).ok();
            Ok(Player { code, rx: Some(rx), last: None, tick: 0, ended: false, _pcm: stream })
        }
    }

    pub fn code(&self) -> u32 { self.code }

    /// `psiMovieFinished__Fv` 0x001082e8: `BGFMVPlaying == 0` — true once the decoder hit the end of
    /// the file and the port has shown what it has. Without real playback (no `movies` feature, or
    /// a declined start) this is true immediately, exactly like the game's un-started movie.
    pub fn finished(&self) -> bool { self.ended }

    /// One render frame. Pulls a new decoded frame on every second call (the game's 30 fps pacing,
    /// main 0x00109080) and returns the frame to show as the full-screen plane, if any has arrived.
    pub fn present(&mut self) -> Option<(u32, u32, &[u8])> {
        self.tick = self.tick.wrapping_add(1);
        if self.tick % 2 == 0 || self.last.is_none() {
            if let Some(rx) = &self.rx {
                while let Ok(item) = rx.try_recv() {
                    match item {
                        Decoded::Frame(w, h, rgba) => self.last = Some((w, h, rgba)),
                        Decoded::Eof => self.ended = true,
                    }
                }
            }
        }
        self.last.as_ref().map(|(w, h, rgba)| (*w, *h, rgba.as_slice()))
    }

    /// `psiStopBackgroundMovie__Fv` 0x00108328 — drops the decoder channel and the audio stream.
    pub fn stop(self) {}
}

#[cfg(feature = "movies")]
/// The decoder-side half of the PCM ring.
fn push_pcm(ring: &std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<i16>>>, chunk: &[i16]) {
    if let Ok(mut q) = ring.lock() {
        // Cap ~0.25 s at 48 kHz stereo: the callback drains in real time, so a full ring means the
        // audio is far ahead of the picture — drop the oldest instead of growing.
        for &s in chunk {
            if q.len() >= 24_000 { q.pop_front(); }
            q.push_back(s);
        }
    }
}

#[cfg(feature = "movies")]
fn open_pcm_out(pcm: &[i16], rate: f64) -> (std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<i16>>>, Option<PcmOut>) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    let ring: Arc<Mutex<VecDeque<i16>>> = Arc::new(Mutex::new(pcm.iter().take(24_000).copied().collect()));
    let Some(device) = cpal::default_host().default_output_device() else { eprintln!("movie: no output device; running silent"); return (ring, None) };
    let Ok(cfg) = device.default_output_config() else { eprintln!("movie: no output config; running silent"); return (ring, None) };
    let dr = ring.clone();
    let dev_rate = cfg.sample_rate() as f64;
    let ch = cfg.channels() as usize;
    let pos = Arc::new(Mutex::new(0.0f64));
    let pos_cb = pos.clone();
    let err = |e: cpal::Error| eprintln!("movie audio: {e}");
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(cpal::StreamConfig::from(cfg.config()), move |d: &mut [f32], _| {
            let mut p = pos_cb.lock().unwrap();
            for out in d.iter_mut() {
                let step = dev_rate / rate / ch.max(1) as f64;
                let src = (*p * rate as f64) as usize / 2 * 2; // stay on a stereo pair
                let s = if let Ok(q) = dr.lock() { q.get(src % q.len().max(1)).copied().unwrap_or(0) } else { 0 };
                *out = s as f32 / 32768.0;
                *p += step;
            }
        }, err, None).ok(),
        _ => { eprintln!("movie audio: unsupported sample format; running silent"); None }
    };
    (ring.clone(), stream.map(move |s| PcmOut { _stream: s, ring }))
}

#[cfg(all(test, feature = "movies"))]
mod tests {
    use super::*;

    fn iso() -> Option<IsoImage> {
        let path = std::env::var_os("NF_MOVIE_ISO").or_else(|| std::env::var_os("NF_ISO"))?;
        Some(IsoImage::open(&path).unwrap())
    }

    /// The first briefing movie loads with the game's own geometry (512×448, 30 Hz, PCM 48 kHz stereo) and
    /// frame 0 encodes as a 512×448 PNG. Skips on images without movie data.
    #[test]
    fn briefing_movie_loads_and_frame0_shots() {
        let Some(iso) = iso() else { eprintln!("skip: NF_MOVIE_ISO/NF_ISO not set"); return };
        let Ok(m) = Movie::load(&iso, "07100001") else { eprintln!("skip: no movie data on this image"); return };
        assert_eq!(m.name, "07100001.PSS");
        assert_eq!(m.code, 0x0710_0001);
        assert_eq!((m.width, m.height, m.frame_rate_code), (512, 448, 5));
        let a = m.audio().unwrap();
        assert_eq!((a.header.kind, a.header.rate, a.header.channels), (1, 48000, 2));
        let png = m.shot(0).unwrap();
        assert!(png.len() > 50_000, "a real 512x448 frame is tens of KB, got {}", png.len());
        // Sister: the movie code 0x7100001 and the file name resolve to the same movie.
        let m2 = Movie::load(&iso, "0x7100001").unwrap();
        assert_eq!(m2.name, m.name);
    }
}
