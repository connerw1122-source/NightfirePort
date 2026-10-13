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

#[cfg(feature = "movies")]
pub struct MovieStream {
    /// Display-ordered frames ready to show, or the worker's end marker.
    rx: std::sync::mpsc::Receiver<WorkerMsg>,
    /// The newest frame `advance` handed out (kept so the caller can re-take it between ticks).
    current: Option<nf_formats::mpeg2::VideoFrame>,
    handle: Option<std::thread::JoinHandle<()>>,
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
    if bounds.first() != Some(&0) && head_end != 0 { bounds.insert(0, 0); } // a leading open GOP decodes alone from stream start
    bounds.push(es.len());
    let chunks = bounds.windows(2).map(|w| (w[0], w[1])).filter(|(a, b)| b > a).collect();
    (chunks, head_end)
}

#[cfg(feature = "movies")]
impl MovieStream {
    /// Starts the worker: decode chunk by chunk ahead of playback. `looping` restarts from the
    /// first chunk when the ES ends (the game's looping background movies).
    pub fn start(movie: &Movie, looping: bool) -> Result<MovieStream> {
        let (chunks, head_end) = gop_chunks(&movie.demuxed.video);
        anyhow::ensure!(!chunks.is_empty(), "no decodable GOP chunks in {}", movie.name);
        let head = movie.demuxed.video[..head_end].to_vec();
        let es = movie.demuxed.video.clone();
        let (tx, rx) = std::sync::mpsc::channel::<WorkerMsg>();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop2 = stop.clone();
        let pictures = nf_formats::mpeg2::picture_counts(&movie.demuxed.video).0;
        let code = movie.code;
        let handle = std::thread::Builder::new().name(format!("movie-{code:08x}")).spawn(move || {
            'movie: loop {
                for &(a, b) in &chunks {
                    if stop2.load(std::sync::atomic::Ordering::Relaxed) { break 'movie; }
                    // The chunk's own sequence header + its GOPs: a closed-GOP-aligned slice is
                    // self-contained, so `decode_video` on it equals the whole-stream decode's
                    // frames for the same pictures.
                    let mut part = head.clone();
                    part.extend_from_slice(&es[a..b]);
                    let Ok(frames) = nf_formats::mpeg2::decode_video(&part) else { break 'movie };
                    for f in frames {
                        if stop2.load(std::sync::atomic::Ordering::Relaxed) { break 'movie; }
                        if tx.send(WorkerMsg::Frame(f)).is_err() { break 'movie; }
                    }
                }
                if !looping { break; }
            }
            let _ = tx.send(WorkerMsg::Done);
        })?;
        Ok(MovieStream { rx, current: None, handle: Some(handle), stop, code, looping, pictures, finished: false })
    }

    /// Pops the next decoded frame into `current` (the display advance — the game decodes one frame
    /// per tick; the port's menus call this every other 60 Hz tick, the 30 fps movie rate). Returns
    /// false while the decoder is behind (the caller keeps the previous frame, like a skipped IPU
    /// frame [M PS2FramesToSkip]) or the stream ended.
    pub fn advance(&mut self) -> bool {
        match self.rx.try_recv() {
            Ok(WorkerMsg::Frame(f)) => { self.current = Some(f); true }
            Ok(WorkerMsg::Done) => { self.finished = true; false }
            Err(_) => false,
        }
    }

    /// The frame to display now.
    pub fn frame(&self) -> Option<&nf_formats::mpeg2::VideoFrame> { self.current.as_ref() }

    /// Stops the worker (page change / movie stop — `psiStopBackgroundMovie` frees instantly [M 0x00108328]).
    pub fn stop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = self.handle.take() { let _ = h.join(); }
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

/// The movie's frame `n` of the FIRST closed-GOP chunk as an RGBA `TextureImage` — the fast path
/// for stills (`--shot`): only that chunk decodes (≈ 10 pictures) instead of the whole ES.
#[cfg(feature = "movies")]
pub fn movie_still(iso_path: &str, code: u32, n: usize) -> Result<crate::scene::TextureImage> {
    let iso = IsoImage::open(iso_path)?;
    let m = Movie::load(&iso, &format!("{code:08X}"))?;
    let (chunks, head_end) = gop_chunks(&m.demuxed.video);
    let &(a, b) = chunks.first().context("no closed-GOP chunk in the movie")?;
    let mut part = m.demuxed.video[..head_end].to_vec();
    part.extend_from_slice(&m.demuxed.video[a..b]);
    let f = nf_formats::mpeg2::decode_video(&part)?.into_iter().nth(n)
        .context(format!("no frame {n} in the first chunk"))?;
    let rgba = nf_formats::mpeg2::yuv420_to_rgb8(&f);
    Ok(crate::scene::TextureImage { key: (usize::MAX, code), width: f.width as u32, height: f.height as u32, rgba, more: Vec::new(), rate: 1 })
}
