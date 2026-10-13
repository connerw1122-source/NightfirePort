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
