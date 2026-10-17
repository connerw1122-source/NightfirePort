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

/// The movie PCM output: a cpal stream playing the whole track from memory (`open_pcm_out`), opened
/// with the first picture. Gain `MOVIE_GAIN` (the boot-default `MovieVolume`).
#[cfg(feature = "movies")]
struct PcmOut {
    _stream: cpal::Stream,
    /// The track position the device has played to, in source sample frames (an f64's bits) —
    /// the movie's master clock (`audio_clock`).
    played: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// The track length in source sample frames.
    frames: usize,
}

/// A playing movie, the port's `psiStartBackgroundMovie` (0x001081a8). fix-menu-movies2: this is
/// the ONE player/decoder path for every movie — the mission-flow briefing/outro (fix-movie-playback),
/// the front end's looping page backdrop and the P_INTRO pair (fix-menu-movies) all start a Player.
/// The decode runs on `MovieStream`'s closed-GOP streaming worker (the game decodes one MPEG frame
/// per tick on the IPU [M movies.md]; the whole-ES decode this replaced cost ~23–31 s before the
/// first frame), the PCM goes through cpal, and `present()` hands out one frame per second call
/// (the game decodes one MPEG frame per 2 vblanks, 30 fps, main 0x00109080's
/// `PS2FramesToSkip = (VBlankCount & ~1) - GameCount`).
///
/// Without the cargo feature `movies` the player declines real playback (`start` still runs the
/// whitelist + file checks, then reports done immediately) so the state machine is testable
/// everywhere.
pub struct Player {
    code: u32,
    /// The front end's backdrop loops [M `Menu_PlayMovie(aux, 0, 1, …)` — loop = 1,
    /// `Manager_SendMessage` 0x21c858:338; `psiDecodeBackgroundMovieFrame` 0x00108060 restarts the
    /// file]; the mission-flow movies play once [M GameFlow_Main 0x1c98c8 case 6 waits
    /// `psiMovieFinished`].
    looping: bool,
    last: Option<(u32, u32, Vec<u8>)>,
    /// Frames pulled since `started` (the pacing clock).
    tick: u32,
    started: Option<std::time::Instant>,
    ended: bool,
    /// The ONE decoder path: the closed-GOP streaming worker.
    #[cfg(feature = "movies")]
    stream: Option<MovieStream>,
    /// The movie's interleaved stereo PCM and its rate. The device stream opens with the FIRST
    /// picture (`present`), not at `start`: the mission flow starts the briefing during the map-load
    /// fade (decode-ahead), and the audio ran seconds ahead of a picture that wasn't showing yet.
    #[cfg(feature = "movies")]
    pcm: std::sync::Arc<Vec<i16>>,
    #[cfg(feature = "movies")]
    rate: f64,
    #[cfg(feature = "movies")]
    _pcm: Option<PcmOut>,
}

impl Player {
    /// `psiStartBackgroundMovie__FUiScUi` 0x001081a8 for one of the mission-flow codes: gate on the
    /// `UsingMovie` whitelist, open `cdrom0:\movies\30_FPS\%8.8x.pss;1`, start decoding + audio.
    /// (The game's `loop` and `volume` args are 0 / `SFXGetVolume()` at every mission-flow site
    /// [M]; the port plays once at fixed volume.)
    pub fn start(iso_path: &str, code: u32) -> Result<Player, StartError> {
        Self::open(iso_path, code, false)
    }

    /// The front end's start site: `Menu_PlayMovie(aux, 0, 1, 0, 0, 0)` — loop = 1 [M
    /// `Manager_SendMessage` 0x21c858:338] — for the page backdrop; the intro pair plays once
    /// (`P_INTRO_Handler` 0x209e88 waits `psiMovieFinished`), so it uses `start`. The same
    /// whitelist gates every code [M 0x107eb8].
    pub fn start_looping(iso_path: &str, code: u32) -> Result<Player, StartError> {
        Self::open(iso_path, code, true)
    }

    fn open(iso_path: &str, code: u32, looping: bool) -> Result<Player, StartError> {
        if !USING_MOVIE.contains(&code) { return Err(StartError::NotWhitelisted); }
        let iso = nf_formats::iso9660::IsoImage::open(iso_path).map_err(|e| StartError::NoMovieData(e.to_string()))?;
        let m = Movie::load(&iso, &format!("{code:08X}")).map_err(|e| StartError::NoMovieData(e.to_string()))?;
        #[cfg(not(feature = "movies"))]
        { let _ = (&m, looping); Ok(Player { code, looping, last: None, tick: 0, started: None, ended: true }) }
        #[cfg(feature = "movies")]
        {
            // Audio first: the whole PCM is in memory after the demux; each decoded frame's slice
            // is paced out at present() time, like audioDecSendToIOP does in the frame callback
            // (0x107100).
            let pcm: Vec<i16> = m.audio().map(|a| a.decode().into_iter().flat_map(|[l, r]| [l, r]).collect()).unwrap_or_default();
            let rate = m.audio().map(|a| a.header.rate).unwrap_or(48000) as f64;
            // The ONE decoder path: MovieStream's closed-GOP worker (fix-menu-movies'), now under
            // the mission-flow codes too.
            let stream = match MovieStream::start(&m, looping) {
                Ok(s) => Some(s),
                Err(e) => { eprintln!("movie: {code:08x}: {e}"); None }
            };
            Ok(Player { code, looping, last: None, tick: 0, started: None, ended: stream.is_none(), stream, pcm: std::sync::Arc::new(pcm), rate, _pcm: None })
        }
    }

    pub fn code(&self) -> u32 { self.code }

    /// Seconds since the first frame was presented (0 before any frame showed).
    pub fn shown_for(&self) -> f32 { self.started.map_or(0.0, |s| s.elapsed().as_secs_f32()) }

    /// True when this player loops its file (the front end's backdrop [M 0x21c858:338 loop = 1]);
    /// the mission-flow players never do.
    pub fn is_looping(&self) -> bool { self.looping }

    /// `psiMovieFinished__Fv` 0x001082e8: `BGFMVPlaying == 0` — true once the decoder hit the end of
    /// the file and the port has shown what it has. Without real playback (no `movies` feature, or
    /// a declined start) this is true immediately, exactly like the game's un-started movie.
    pub fn finished(&self) -> bool { self.ended }

    /// One render frame. Pulls the next decoded frame when one is due at the game's 30 fps (main 0x00109080 decodes
    /// on every second 60 Hz vblank) by the wall clock, and returns the frame to show as the full-screen plane, if any
    /// has arrived. (Pulling on every second CALL tied the movie to the render rate: ~66 frames a second on a fast
    /// display, measured 2026-10-15.)
    pub fn present(&mut self) -> Option<(u32, u32, &[u8])> {
        let now = std::time::Instant::now();
        let start = *self.started.get_or_insert(now);
        #[cfg(feature = "movies")]
        if let Some(t) = self.audio_clock() {
            return self.present_on_audio(now, t);
        }
        let mut due = ((now - start).as_secs_f64() * 30.0) as u32 + 1;
        // Never faster than real time: when the decoder falls behind (its warm-up on the first
        // frames, a slow GOP) the clock slips instead of the picture racing to catch up — the old
        // catch-up showed one movie frame per RENDER frame, so the intro's opening (the 007 white
        // dots) flashed by at 100+ fps (playtest 1015-2142). The audio rides the frames, so it
        // stays with the picture.
        if due > self.tick.wrapping_add(2) && self.last.is_some() {
            self.started = Some(now - std::time::Duration::from_secs_f64(self.tick as f64 / 30.0));
            due = self.tick.wrapping_add(1);
        }
        #[cfg(feature = "movies")]
        if self.tick < due || self.last.is_none() {
            if let Some(s) = self.stream.as_mut() {
                if s.advance() {
                    self.tick = self.tick.wrapping_add(1);
                    if let Some(f) = s.frame().cloned() {
                        // The frame's audio slice goes out with its picture (the game feeds the IOP
                        // ring from the same callback that produces the frame [M 0x107100]); a
                        // looping movie restarts the walk with the video [M 0x00108060].
                        // movie-audio: the audio is the device's own stream (`open_pcm_out` plays the
                        // whole track at its rate); per-picture pushes starved and dropped it.
                        if self.last.is_none() && self._pcm.is_none() {
                            self._pcm = open_pcm_out(self.pcm.clone(), self.rate, self.looping);
                        }
                        let rgba = nf_formats::mpeg2::yuv420_to_rgb8(&f);
                        self.last = Some((f.width as u32, f.height as u32, rgba));
                    }
                } else if s.finished && !self.looping {
                    // End of file [M psiMovieFinished 0x1082e8: BGFMVPlaying == 0]. A looping
                    // movie never gets here — its worker re-walks the chunks instead.
                    self.ended = true;
                }
            }
        }
        self.last.as_ref().map(|(w, h, rgba)| (*w, *h, rgba.as_slice()))
    }

    /// The audio device's play position in seconds while the movie's track is playing — the
    /// master clock for a once-through movie, as on the PS2, where the IOP streams the audio
    /// continuously and the picture skips frames to keep up [M audioDecSendToIOP 0x107100;
    /// PS2FramesToSkip]. None for a looping backdrop (the track wraps), with no device, or once
    /// the track has ended (the picture then finishes on the wall clock).
    #[cfg(feature = "movies")]
    fn audio_clock(&self) -> Option<f64> {
        let out = self._pcm.as_ref()?;
        if self.looping { return None; }
        let pos = f64::from_bits(out.played.load(std::sync::atomic::Ordering::Relaxed));
        if pos >= out.frames as f64 - 1.0 { return None; }
        Some(pos / self.rate)
    }

    /// `present` against the audio clock: the picture due at `t` seconds of audio is shown; when
    /// the decoder fell behind, the pictures in between are dropped (decoded, never converted or
    /// shown) so the picture lands back on the sound instead of slipping behind it. Ahead of the
    /// audio, the current picture holds.
    #[cfg(feature = "movies")]
    fn present_on_audio(&mut self, now: std::time::Instant, t: f64) -> Option<(u32, u32, &[u8])> {
        let due = (t * 30.0) as u32 + 1;
        let mut latest = None;
        if let Some(s) = self.stream.as_mut() {
            while self.tick < due {
                if !s.advance() {
                    if s.finished { self.ended = true; }
                    break;
                }
                self.tick = self.tick.wrapping_add(1);
                latest = s.frame().cloned();
            }
        }
        if let Some(f) = latest {
            let rgba = nf_formats::mpeg2::yuv420_to_rgb8(&f);
            self.last = Some((f.width as u32, f.height as u32, rgba));
        }
        // Keep the wall clock aligned for the hand-over when the track ends first.
        self.started = Some(now - std::time::Duration::from_secs_f64(self.tick as f64 / 30.0));
        self.last.as_ref().map(|(w, h, rgba)| (*w, *h, rgba.as_slice()))
    }

    /// `psiStopBackgroundMovie__Fv` 0x00108328 — stops the decoder worker and the audio stream
    /// (both drop with the player).
    pub fn stop(self) {}
}

/// movie-volume: the game's movie gain. `psiStartBackgroundMovie` stores `MovieVolume = SFXGetVolume()`
/// (= `SfxMasterVolume`, 100 at boot [L 0x30c95c]) and `audioDecStart` 0x107360:9 calls
/// `changeInputVolume(MovieVolume * 100)` → `sceSdSetParam` on both cores' input volume (0xf80 / 0x1080) [M
/// 0x1078c8]: 10000 of the SPU2's 0x7fff full-scale volume register [S: the 15-bit register range, the
/// SPU2 spec]. The port played the PCM at full scale — about 3x too loud (playtest 1016). The options'
/// SFX volume is not plumbed into the player yet; this is the boot default.
pub const MOVIE_GAIN: f32 = (100.0 * 100.0) / 32767.0;

#[cfg(feature = "movies")]
fn open_pcm_out(src: std::sync::Arc<Vec<i16>>, rate: f64, looping: bool) -> Option<PcmOut> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    if nf_audio::silent() || src.len() < 4 { return None; }
    let Some(device) = cpal::default_host().default_output_device() else { eprintln!("movie: no output device; running silent"); return None };
    let Ok(cfg) = device.default_output_config() else { eprintln!("movie: no output config; running silent"); return None };
    let dev_rate = cfg.sample_rate() as f64;
    let ch = cfg.channels() as usize;
    // movie-audio: the WHOLE track plays from memory at the device's own clock — the way a PS2
    // movie's audio is the master the IOP streams continuously [M audioDecSendToIOP 0x107100 keeps
    // the SPU2 ring full]. Feeding it one 33 ms slice per decoded picture (the old ring) starved it
    // whenever the picture slipped (gaps = static) and dropped samples when it caught up (pitch
    // jumps). Linear interpolation from the track's 48 kHz to the device rate.
    let frames = src.len() / 2;
    let step = rate / dev_rate;
    let mut pos = 0.0f64;
    let played = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0f64.to_bits()));
    let played2 = played.clone();
    let err = |e: cpal::Error| eprintln!("movie audio: {e}");
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(cpal::StreamConfig::from(cfg.config()), move |d: &mut [f32], _| {
            for frame in d.chunks_mut(ch.max(1)) {
                if pos >= frames as f64 - 1.0 {
                    if looping { pos = 0.0; } else { frame.fill(0.0); continue; }
                }
                let i = pos as usize;
                let t = (pos - i as f64) as f32;
                let at = |k: usize, c: usize| src[2 * k + c] as f32 / 32768.0 * MOVIE_GAIN;
                let l = at(i, 0) * (1.0 - t) + at(i + 1, 0) * t;
                let r = at(i, 1) * (1.0 - t) + at(i + 1, 1) * t;
                for (k, out) in frame.iter_mut().enumerate() { *out = if k % 2 == 0 { l } else { r }; }
                pos += step;
            }
            played2.store(pos.to_bits(), std::sync::atomic::Ordering::Relaxed);
        }, err, None).ok(),
        _ => { eprintln!("movie audio: unsupported sample format; running silent"); None }
    };
    if let Some(s) = &stream { if let Err(e) = s.play() { eprintln!("movie audio: {e}"); } }
    stream.map(|s| PcmOut { _stream: s, played, frames })
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
        // Sister: the movie code 0x7100001 and the file name resolve to the same movie. (The name
        // form needs all 8 digits: load() only treats a stem as a code when it is 8 chars —
        // "0x7100001" trimmed is 7 and looked up as the file 7100001.PSS, which no image has.)
        let m2 = Movie::load(&iso, "0x07100001").unwrap();
        assert_eq!(m2.name, m.name);
    }

    /// fix-menu-backdrop: the FIRST chunk carries the first GOP, not the bare sequence header. The
    /// old leading-open-GOP guard fired whenever the first GOP is CLOSED too (it is on every
    /// Nightfire movie: first GOP right after the 145-byte sequence header [L probe]), so chunk 0
    /// was (0, head_end) — zero GOPs, zero decoded frames — and `movie_still`'s chunks.first()
    /// found no frame at any N ("no frame N in the first chunk", every page's dark plate).
    #[test]
    fn the_first_gop_chunk_starts_at_the_first_gop() {
        let Some(iso) = iso() else { eprintln!("skip: NF_MOVIE_ISO/NF_ISO not set"); return };
        let Ok(m) = Movie::load(&iso, "07330048") else { eprintln!("skip: no movie data on this image"); return };
        let (chunks, head_end) = gop_chunks(&m.demuxed.video);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].0, head_end, "chunk 0 starts at the first GOP — the sequence header is prepended per chunk, never a chunk of its own");
        let mut part = m.demuxed.video[..head_end].to_vec();
        part.extend_from_slice(&m.demuxed.video[chunks[0].0..chunks[0].1]);
        let frames = nf_formats::mpeg2::decode_video(&part).expect("the first chunk decodes");
        assert!(!frames.is_empty(), "the first chunk decodes pictures (07330048: closed GOPs of 10 P-only pictures [L golden])");
        // Sister: the whole-ES picture count equals the chunks' frame sum (no pictures lost at the
        // boundaries; the same frames the streaming worker plays).
        let total: usize = chunks[1..].iter().map(|&(a, b)| {
            let mut p2 = m.demuxed.video[..head_end].to_vec();
            p2.extend_from_slice(&m.demuxed.video[a..b]);
            nf_formats::mpeg2::decode_video(&p2).map(|f| f.len()).unwrap_or(0)
        }).sum();
        let (pics, ..) = nf_formats::mpeg2::picture_counts(&m.demuxed.video);
        assert_eq!(frames.len() + total, pics, "the chunks cover every picture of the ES");
    }
}

// ---------------------------------------------------------------------------
// Streaming playback (fix-menu-movies): the front end's background movies.
//
// The game decodes one MPEG frame per tick on the IPU in real time [M movies.md: DecodeBGFMVFrame
// 0x00108550 per tick, PS2FramesToSkip clamped to 2, main 0x00109080]. `decode_video` runs a whole
// ES per call (a 60 s menu loop takes ~30 s), so live playback decodes GOP chunks on a worker
// thread: every chunk starts at a CLOSED GOP (set_gop_closed_flag — measured closed on 07330048/
// 07350048/07370048; 07380048 mixes open GOPs, whose B-frames reference the previous GOP's
// anchors, so chunk boundaries sit only at closed-GOP starts and open GOPs ride their predecessor's
// chunk), with the stream's sequence header prepended — self-contained per ISO 13818-2 §6.4.
// ---------------------------------------------------------------------------

/// Decoded pictures the streaming worker may hold ahead of playback (~2 s at 30 fps).
#[cfg(feature = "movies")]
pub const STREAM_AHEAD: usize = 60;

/// Movie decode threads. One thread decodes these 512x448 MPEG-2 movies at ~34 fps on an idle
/// 24-thread desktop, and the heaviest GOPs of the 07300004 briefing run at 108 ms a picture —
/// a third of real time [L probe 2026-10-16]; the 2 s lead drained on those GOPs and the
/// picture stalled (playtest 1016: "starting and ending movies of missions are lagging"). The GOP
/// units are independent decodes (a closed GOP alone, an open one with its predecessor in front),
/// so they decode in parallel, round-robin, and come back out in order.
pub const DECODE_THREADS: usize = 4;

#[cfg(feature = "movies")]
pub struct MovieStream {
    /// One channel per decode thread: unit k's pictures arrive on `rx[k % DECODE_THREADS]`,
    /// followed by a `UnitEnd`; `next` is the channel whose turn it is.
    rx: Vec<std::sync::mpsc::Receiver<WorkerMsg>>,
    next: usize,
    /// The newest frame `advance` handed out (kept so the caller can re-take it between ticks).
    current: Option<nf_formats::mpeg2::VideoFrame>,
    handles: Vec<std::thread::JoinHandle<()>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The movie this stream plays (the host keys restarts on the code, like
    /// `Manager_SendMessage` 0x21c858:338 compares M_PAGE+0xC4 against the playing one).
    pub code: u32,
    /// Loop like `LoopingMovie` [M psiDecodeBackgroundMovieFrame 0x00108060 restarts the file].
    pub looping: bool,
    /// Total pictures the ES holds (the loop length in frames).
    pub pictures: usize,
    /// Set when the worker reported Done (a non-looping movie finished — `psiMovieFinished` [M 0x001082e8]).
    pub finished: bool,
}

#[cfg(feature = "movies")]
enum WorkerMsg {
    Frame(nf_formats::mpeg2::VideoFrame),
    /// The end of one GOP unit's pictures: the next unit is on the next thread's channel.
    UnitEnd,
    Done,
}

/// Splits the video ES at closed-GOP start codes (`00 00 01 B8` with closed_gop set); returns
/// (chunk byte ranges, the byte offset just past the leading sequence layer). Open GOPs extend
/// their predecessor's chunk.
#[cfg(feature = "movies")]
fn gop_chunks(es: &[u8]) -> (Vec<(usize, usize)>, usize) {
    let mut gops = Vec::new();
    let mut i = 0usize;
    while i + 4 <= es.len() {
        let Some(at) = es[i..].windows(4).position(|w| w == [0, 0, 1, 0xb8]) else { break };
        let at = i + at;
        let closed = (es[at + 7] >> 6) & 1 == 1; // temporal_reference is 25 bits (3 bytes + 1 bit), closed_gop the next
        gops.push((at, closed));
        i = at + 4;
    }
    // The sequence header ends where the first GOP begins (measured: one sequence header at the
    // stream start; `decode_video` needs it to establish the geometry).
    let head_end = gops.first().map(|(at, _)| *at).unwrap_or(es.len());
    let mut bounds: Vec<usize> = gops.iter().filter(|(_, closed)| *closed).map(|(at, _)| *at).collect();
    // A leading OPEN GOP (GOP bytes BEFORE the first closed one) decodes alone from stream start.
    // The old `bounds.first() != Some(&0)` also fired when the first GOP is CLOSED — which it is
    // on every Nightfire movie (first GOP right after the 145-byte sequence header [L probe:
    // 07100001/07330048/07350048 all gop[0] = (145, closed)]) — prepending a (0, head_end) chunk
    // that held only the sequence header and decoded to ZERO frames, so `movie_still`'s
    // chunks.first() found no frame at any N: "no frame N in the first chunk — the dark plate
    // stands in" [fix-menu-backdrop]. Prepend 0 only when open GOPs precede the first closed one
    // (first_closed_at > head_end), or when there is no closed GOP at all.
    if bounds.first().is_none_or(|&at| at > head_end) && head_end != 0 { bounds.insert(0, 0); }
    bounds.push(es.len());
    let chunks = bounds.windows(2).map(|w| (w[0], w[1])).filter(|(a, b)| b > a).collect();
    (chunks, head_end)
}

/// boot-flow: the streaming worker's decode units — one per GOP, so the first picture arrives after
/// one GOP's decode instead of a whole closed-GOP chunk (0x073B0048's first closed chunk held most of
/// the movie: the P_ESTHERO intro sat black ~20 s, then joined mid-way). Each unit is (decode from,
/// decode to, keep the last N frames): a closed GOP decodes alone (keep all); an OPEN GOP's leading
/// B pictures reference the previous GOP's last anchor [ISO 13818-2 §6.3.8 closed_gop], so it decodes
/// with the previous GOP in front and keeps only its own pictures (the last N of the output — robust
/// whether the decoder emits or drops the prefix's own unreferenced B pictures).
#[cfg(feature = "movies")]
fn gop_units(es: &[u8]) -> (Vec<(Option<(usize, usize)>, usize, usize, usize)>, usize) {
    let mut gops: Vec<(usize, bool)> = Vec::new();
    let mut i = 0usize;
    while i + 8 <= es.len() {
        let Some(at) = es[i..].windows(4).position(|w| w == [0, 0, 1, 0xb8]) else { break };
        let at = i + at;
        if at + 8 > es.len() { break; }
        gops.push((at, (es[at + 7] >> 6) & 1 == 1));
        i = at + 4;
    }
    let head_end = gops.first().map(|(at, _)| *at).unwrap_or(es.len());
    let pics = |a: usize, b: usize| es[a..b].windows(4).filter(|w| *w == [0, 0, 1, 0]).count();
    let mut units = Vec::new();
    for k in 0..gops.len() {
        let (at, closed) = gops[k];
        let end = gops.get(k + 1).map_or(es.len(), |g| g.0);
        let n = pics(at, end);
        if n == 0 { continue; }
        let prefix = if closed || k == 0 { None } else { Some((gops[k - 1].0, at)) };
        units.push((prefix, at, end, n));
    }
    (units, head_end)
}

/// boot-flow: a reference GOP without its own leading B pictures. An open GOP's leading Bs (coded right
/// after its I, before the next anchor) reference the GOP before it; as a prefix they have nothing in
/// front, and the decoder rejects them ("B-picture before two I/P anchors"). Their frames are dropped
/// anyway — only the prefix's anchors matter as references for the unit.
#[cfg(feature = "movies")]
fn strip_leading_b(gop: &[u8]) -> Vec<u8> {
    let starts: Vec<usize> = gop.windows(4).enumerate().filter(|(_, w)| *w == [0, 0, 1, 0]).map(|(i, _)| i).collect();
    let mut out = Vec::with_capacity(gop.len());
    out.extend_from_slice(&gop[..starts.first().copied().unwrap_or(gop.len())]);
    let mut seen_anchor = 0;
    for (k, &st) in starts.iter().enumerate() {
        let en = starts.get(k + 1).copied().unwrap_or(gop.len());
        let kind = gop.get(st + 5).map_or(0, |b| (b >> 3) & 7);
        if kind == 3 && seen_anchor < 2 { continue; } // a leading B (before the second anchor)
        if kind == 1 || kind == 2 { seen_anchor += 1; }
        out.extend_from_slice(&gop[st..en]);
    }
    out
}

#[cfg(feature = "movies")]
impl MovieStream {
    /// Starts the worker: decode GOP by GOP ahead of playback (`gop_units`). `looping` restarts from the
    /// first unit when the ES ends (the game's looping background movies).
    pub fn start(movie: &Movie, looping: bool) -> Result<MovieStream> {
        let (chunks, head_end) = gop_units(&movie.demuxed.video);
        anyhow::ensure!(!chunks.is_empty(), "no decodable GOP chunks in {}", movie.name);
        let head = movie.demuxed.video[..head_end].to_vec();
        let es = movie.demuxed.video.clone();
        // crash-fix: BOUNDED. The unbounded channel let the worker decode the whole movie ahead
        // of playback — a briefing started during the map-load fade (decode-ahead) piled every
        // picture in RAM (~350 KB each, thousands of them): freeze, then crash at the briefing's
        // end (playtest 1016). ~2 s of pictures is plenty of lead.
        // The lead is split over the decode threads' channels.
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let pictures = nf_formats::mpeg2::picture_counts(&movie.demuxed.video).0;
        let code = movie.code;
        let (chunks, head, es) = (std::sync::Arc::new(chunks), std::sync::Arc::new(head), std::sync::Arc::new(es));
        let mut rxs = Vec::new();
        let mut handles = Vec::new();
        for w in 0..DECODE_THREADS {
            let (tx, rx) = std::sync::mpsc::sync_channel::<WorkerMsg>((STREAM_AHEAD / DECODE_THREADS).max(1));
            rxs.push(rx);
            let (stop2, chunks, head, es) = (stop.clone(), chunks.clone(), head.clone(), es.clone());
            handles.push(std::thread::Builder::new().name(format!("movie-{code:08x}-{w}")).spawn(move || {
                // Thread w decodes the units at global positions w, w+N, w+2N, … (a looping movie
                // keeps counting past the end, so the round-robin order holds across the wrap).
                let n = chunks.len();
                let mut g = w;
                loop {
                    if !looping && g >= n { break; }
                    if stop2.load(std::sync::atomic::Ordering::Relaxed) { return; }
                    let (prefix, a, b, keep) = chunks[g % n];
                    // The unit's own sequence header + its GOP (an open GOP with its predecessor in
                    // front as the reference); only the unit's own pictures go out.
                    let mut part = (*head).clone();
                    if let Some((pa, pb)) = prefix { part.extend_from_slice(&strip_leading_b(&es[pa..pb])); }
                    part.extend_from_slice(&es[a..b]);
                    let mut frames = nf_formats::mpeg2::decode_video(&part).unwrap_or_default();
                    if frames.len() > keep { frames.drain(..frames.len() - keep); }
                    for f in frames {
                        if stop2.load(std::sync::atomic::Ordering::Relaxed) { return; }
                        if tx.send(WorkerMsg::Frame(f)).is_err() { return; }
                    }
                    if tx.send(WorkerMsg::UnitEnd).is_err() { return; }
                    g += DECODE_THREADS;
                }
                let _ = tx.send(WorkerMsg::Done);
            })?);
        }
        Ok(MovieStream { rx: rxs, next: 0, current: None, handles, stop, code, looping, pictures, finished: false })
    }

    /// Pops the next decoded frame into `current` (the display advance — the game decodes one frame
    /// per tick; the port's menus call this every other 60 Hz tick, the 30 fps movie rate). Returns
    /// false while the decoder is behind (the caller keeps the previous frame, like a skipped IPU
    /// frame [M PS2FramesToSkip]) or the stream ended.
    pub fn advance(&mut self) -> bool {
        if self.finished { return false; }
        // At most one full round of empty units (a GOP that decoded to nothing) per call.
        for _ in 0..=DECODE_THREADS {
            match self.rx[self.next].try_recv() {
                Ok(WorkerMsg::Frame(f)) => { self.current = Some(f); return true }
                Ok(WorkerMsg::UnitEnd) => { self.next = (self.next + 1) % self.rx.len(); }
                // The thread whose turn it is has no units left: the movie is over.
                Ok(WorkerMsg::Done) => { self.finished = true; return false }
                Err(_) => return false,
            }
        }
        false
    }

    /// The frame to display now.
    pub fn frame(&self) -> Option<&nf_formats::mpeg2::VideoFrame> { self.current.as_ref() }

    /// Stops the worker (page change / movie stop — `psiStopBackgroundMovie` frees instantly [M 0x00108328]).
    pub fn stop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        // A worker may sit in a full channel's send; drain until each sees the flag and exits
        // (a bare join here would deadlock against the bounded channels).
        while self.handles.iter().any(|h| !h.is_finished()) {
            for rx in &self.rx { while rx.try_recv().is_ok() {} }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        for h in self.handles.drain(..) { let _ = h.join(); }
        self.current = None;
    }
}

#[cfg(feature = "movies")]
impl Drop for MovieStream {
    fn drop(&mut self) { self.stop(); }
}

/// The movie's PCM as two mono SPU voices (left, right): the game's movie audio is one 48 kHz
/// stereo stream [M movies.md audio section; docs/formats/pss.md], and the port's SPU stand-in keys
/// per-voice mono `Pcm`s — L and R at pitch 0x1000 stay sample-aligned through the same mixer.
/// `looping` sets both voices' loop point to the stream start (the looping background movies
/// restart their file — and their audio — inside `psiDecodeBackgroundMovieFrame` [M 0x00108060]).
#[cfg(feature = "movies")]
pub fn pcm_voices(movie: &Movie, looping: bool) -> Result<(std::sync::Arc<nf_audio::Pcm>, std::sync::Arc<nf_audio::Pcm>)> {
    let a = movie.audio()?;
    anyhow::ensure!(a.header.kind == 1, "movie audio is not PCM (kind {})", a.header.kind);
    let pairs = a.decode();
    let (mut l, mut r) = (Vec::with_capacity(pairs.len()), Vec::with_capacity(pairs.len()));
    for p in &pairs { l.push(p[0]); r.push(p[1]); }
    let loop_start = looping.then_some(0);
    Ok((std::sync::Arc::new(nf_audio::Pcm { data: l, loop_start }),
        std::sync::Arc::new(nf_audio::Pcm { data: r, loop_start })))
}

/// The movie's frame `n` as an RGBA `TextureImage` — the fast path for stills (`--shot`): whole
/// closed-GOP chunks decode in order until `n` is covered (a chunk ≈ 10 pictures; frame 12 of the
/// menu loops rides chunk 1 [L golden/pss_movies.tsv: 180 I pictures / 1800 total]) instead of the
/// whole ES. Chunk boundaries sit at closed GOPs, so a chunk's frames equal the whole-ES decode's
/// frames for the same pictures [ISO 13818-2 §6.4.3; the `21d0739d…` frame-12 pin in
/// menu_movies::the_menu_loop_movie_decodes].
#[cfg(feature = "movies")]
pub fn movie_still(iso_path: &str, code: u32, n: usize) -> Result<crate::scene::TextureImage> {
    let iso = IsoImage::open(iso_path)?;
    let m = Movie::load(&iso, &format!("{code:08X}"))?;
    let (chunks, head_end) = gop_chunks(&m.demuxed.video);
    anyhow::ensure!(!chunks.is_empty(), "no closed-GOP chunk in the movie");
    let head = &m.demuxed.video[..head_end];
    let mut shown = 0usize;
    for &(a, b) in &chunks {
        let mut part = head.to_vec();
        part.extend_from_slice(&m.demuxed.video[a..b]);
        let frames = nf_formats::mpeg2::decode_video(&part)?;
        if n < shown + frames.len() {
            let f = &frames[n - shown];
            let rgba = nf_formats::mpeg2::yuv420_to_rgb8(f);
            return Ok(crate::scene::TextureImage { key: (usize::MAX, code), width: f.width as u32, height: f.height as u32, rgba, more: Vec::new(), rate: 1 });
        }
        shown += frames.len();
    }
    let (pics, ..) = nf_formats::mpeg2::picture_counts(&m.demuxed.video);
    bail!("no frame {n} in the first {} chunks (the movie has {pics} pictures)", chunks.len())
}

/// The FMV briefing's subtitle track (subtitles-data): `ResetMap_Load` (0x1d6360) phase 0 loads the
/// global pack `07000500.bin` from FILES.BIN and — only when the Subtitles option is on
/// (`DAT_002a37c8`) — creates one ScriptPlayer per movie code from its 14 kind-7 scripts (single
/// streams of op-0x1e `Script_TextStart` commands [D levels.md §5.4]) and plays it at once while
/// the briefing movie runs, so the script's tick axis is the movie's. The port builds the same
/// track from the pack's script: `nf_game::subtitle::intro_script_for(movie_code)` [M
/// 0x1d6360:40-99] picks it and `SubtitleTrack::from_script` walks its Wait + Text commands with
/// `Script_Run`'s clock. `Ok(None)` = the code has no intro row (e.g. 0x710004a — no briefing
/// subtitles by the game's own table).
pub fn briefing_subtitles(iso_path: &str, movie_code: u32, enabled: bool) -> Result<Option<nf_game::subtitle::SubtitleTrack>> {
    let Some(script) = nf_game::subtitle::intro_script_for(movie_code) else { return Ok(None) };
    let iso = IsoImage::open(iso_path)?;
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000500.bin"))
        .with_context(|| "no 07000500.bin in FILES.BIN (the global pack)")?;
    let pack = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let plb = nf_formats::levelbin::LevelBin::parse(&pack)?;
    for pe in plb.entries.iter().filter(|e| e.kind == 7) {
        if pe.hash() == script {
            let nis = nf_formats::nis::Nis::parse(script, plb.data(&pack, pe))?;
            return Ok(Some(nf_game::subtitle::SubtitleTrack::from_script(&nis, enabled)));
        }
    }
    bail!("script {script:08x} not in 07000500.bin")
}


/// playtest 1016: "starting and ending movies of missions are lagging … it throws the audio out of
/// sync". [L probe 2026-10-16: one decode thread ran the 07300004 briefing at ~34 fps on an idle
/// desktop, its heaviest GOPs at 108 ms a picture.] [S port engineering: decode throughput and the
/// A/V clock; the game's own rule is that the audio streams on and the picture skips frames to keep
/// up — PS2FramesToSkip, audioDecSendToIOP 0x107100.]
#[cfg(all(test, feature = "movies"))]
mod lag_tests {
    use super::*;

    fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

    /// The stream must outrun real time with margin across the briefing's heavy stretch, so a
    /// busy machine still keeps the 2 s lead. One thread managed ~34 fps here — the sister.
    #[test]
    fn the_briefing_decodes_well_ahead_of_real_time() {
        let Some(path) = iso() else { return };
        let iso = nf_formats::iso9660::IsoImage::open(&path).unwrap();
        let Ok(m) = Movie::load(&iso, "07300004") else { return };
        let mut s = MovieStream::start(&m, false).unwrap();
        let t0 = std::time::Instant::now();
        let mut n = 0usize;
        while n < 900 && t0.elapsed().as_secs() < 60 {
            if s.advance() { n += 1 } else if s.finished { break } else { std::thread::yield_now() }
        }
        let fps = n as f64 / t0.elapsed().as_secs_f64();
        eprintln!("07300004: {n} pictures at {fps:.1} fps with {DECODE_THREADS} threads");
        assert_eq!(n, 900, "the stream stalled or ended early");
        assert!(fps >= 50.0, "{fps:.1} fps — the decoder must run well ahead of the movie's 30");
    }

    /// The units come back in display order: the parallel stream's pictures match a plain
    /// in-order decode of the same GOPs, across several thread hand-overs.
    #[test]
    fn the_parallel_stream_keeps_the_picture_order() {
        let Some(path) = iso() else { return };
        let iso = nf_formats::iso9660::IsoImage::open(&path).unwrap();
        let Ok(m) = Movie::load(&iso, "07380048") else { return };
        let (units, head_end) = gop_units(&m.demuxed.video);
        let es = &m.demuxed.video;
        let mut want = Vec::new();
        for &(prefix, a, b, keep) in units.iter().take(DECODE_THREADS * 2 + 1) {
            let mut part = es[..head_end].to_vec();
            if let Some((pa, pb)) = prefix { part.extend_from_slice(&strip_leading_b(&es[pa..pb])); }
            part.extend_from_slice(&es[a..b]);
            let mut f = nf_formats::mpeg2::decode_video(&part).unwrap();
            if f.len() > keep { f.drain(..f.len() - keep); }
            want.extend(f.into_iter().map(|f| nf_formats::sha1::hex(&nf_formats::sha1::sha1(&f.y))));
        }
        let mut s = MovieStream::start(&m, false).unwrap();
        let mut got = Vec::new();
        let t0 = std::time::Instant::now();
        while got.len() < want.len() && t0.elapsed().as_secs() < 60 {
            if s.advance() { got.push(nf_formats::sha1::hex(&nf_formats::sha1::sha1(&s.frame().unwrap().y))) } else { std::thread::yield_now() }
        }
        assert_eq!(got.len(), want.len());
        assert!(got == want, "the parallel stream reordered pictures");
        // Sister: the comparison can fail (neighbouring pictures differ).
        assert!(want.windows(2).any(|w| w[0] != w[1]));
    }

    /// When the picture is late against the audio clock, it jumps to the picture the sound is
    /// at (dropping the ones in between) instead of playing them late and drifting behind.
    #[test]
    fn a_late_picture_catches_up_to_the_audio_clock() {
        let Some(path) = iso() else { return };
        let Ok(mut p) = Player::start(&path, 0x0730_0004) else { return };
        // Let the decoder build its lead, as during the map-load fade.
        std::thread::sleep(std::time::Duration::from_secs(2));
        let now = std::time::Instant::now();
        assert!(p.present_on_audio(now, 0.0).is_some(), "the first picture shows at audio 0");
        assert_eq!(p.tick, 1);
        // The audio is 1.5 s in (45 pictures): one present must land on picture 46.
        let mut ok = false;
        for _ in 0..200 {
            p.present_on_audio(std::time::Instant::now(), 1.5);
            if p.tick == 46 { ok = true; break }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(ok, "the picture stopped at {} instead of catching up to 46", p.tick);
        // Sister: with the audio clock standing still the picture holds (never runs ahead).
        for _ in 0..10 { p.present_on_audio(std::time::Instant::now(), 1.5); }
        assert_eq!(p.tick, 46, "the picture ran ahead of the audio");
    }
}
