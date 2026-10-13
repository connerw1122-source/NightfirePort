//! MPEG-2 video: picture counting over the raw elementary stream (always available) and full decoding via the
//! pure-Rust [`oxideav_mpeg12video`] crate (behind the cargo feature `movies`; see docs/formats/pss.md).
//!
//! **Provenance:** Nightfire's `MOVIES/30_FPS/*.PSS` video is MPEG-2 Main Profile @ Main Level, 512×448,
//! interlaced (`progressive_sequence = 0`), 4:2:0, frame-rate code 5 (30 Hz), I/P/B pictures (measured on all
//! 22 movies, golden/pss_movies.tsv). The game itself decodes on the IPU inside the statically linked libmpeg
//! (`sceMpegGetPicture` 0x0022a3f8) into a 512×448 RGBA32 background (`DecodeBGFMVFrame` 0x00108550 centres the
//! frame in the 0x200×0x1c0 picture buffer from `AllocateMovieMemory` 0x00107d70: 512·448·4 bytes).

/// One MPEG-2 video frame, decoded (feature `movies`) or otherwise unavailable.
#[cfg(feature = "movies")]
#[derive(Clone, Debug)]
pub struct VideoFrame {
    pub width: usize,
    pub height: usize,
    /// Luminance plane, `width × height` samples.
    pub y: Vec<u8>,
    /// Cb plane, `width/2 × height/2` samples for 4:2:0.
    pub cb: Vec<u8>,
    /// Cr plane, `width/2 × height/2` samples for 4:2:0.
    pub cr: Vec<u8>,
    /// §6.3.10 `picture_coding_type`: 1 = I, 2 = P, 3 = B.
    pub coding_type: u8,
    /// §6.3.10 `temporal_reference` of the coded picture.
    pub temporal_reference: u16,
}

/// Counts the §6.2.3 picture headers (start codes `00 00 01 00`, picture start only — `01`–`AF` are slices) by
/// coding type: `(total, I, P, B)`.
pub fn picture_counts(es: &[u8]) -> (usize, usize, usize, usize) {
    let mut total = 0;
    let mut i = 0usize;
    let mut kinds = [0usize; 3];
    while i + 6 <= es.len() {
        let at = es[i..].windows(4).position(|w| w == [0, 0, 1, 0]);
        let Some(k) = at else { break };
        i += k;
        let tc = (u16::from_be_bytes([es[i + 4], es[i + 5]]) >> 3) as usize & 7;
        if (1..=3).contains(&tc) { kinds[tc - 1] += 1; total += 1; }
        i += 4;
    }
    (total, kinds[0], kinds[1], kinds[2])
}

/// Decodes a whole MPEG-2 video elementary stream in coded order's display order (the crate reorders B-frames
/// per §6.1.1.11). Slow (a movie's ES is tens of MB); tests use it on one movie at a time.
#[cfg(feature = "movies")]
pub fn decode_video(es: &[u8]) -> crate::buf::Result<Vec<VideoFrame>> {
    use crate::buf::invalid;
    use oxideav_mpeg12video::video_sequence::decode_video_sequence;
    let frames = decode_video_sequence(es).map_err(|e| invalid(format!("mpeg2: {e}")))?;
    Ok(frames
        .into_iter()
        .map(|f| VideoFrame {
            width: f.frame.width,
            height: f.frame.height,
            y: f.frame.y.samples().to_vec(),
            cb: f.frame.cb.samples().to_vec(),
            cr: f.frame.cr.samples().to_vec(),
            coding_type: f.picture_coding_type as u8,
            temporal_reference: f.temporal_reference,
        })
        .collect())
}

/// Packs a decoded 4:2:0 frame as Y then Cb then Cr (the ffmpeg `yuv420p` raw layout, so `ffmpeg -f rawvideo
/// -pix_fmt yuv420p` output of the same ES is byte-comparable, SHA-1 and all).
#[cfg(feature = "movies")]
pub fn pack_yuv420(f: &VideoFrame) -> Vec<u8> {
    let mut out = Vec::with_capacity(f.width * f.height * 3 / 2);
    out.extend_from_slice(&f.y);
    out.extend_from_slice(&f.cb);
    out.extend_from_slice(&f.cr);
    out
}

/// Upsamples a 4:2:0 frame to RGB888 (nearest neighbour, BT.601 limited-range like the PS2's CSC).
#[cfg(feature = "movies")]
pub fn yuv420_to_rgb8(f: &VideoFrame) -> Vec<u8> {
    let (w, h) = (f.width, f.height);
    let cw = w / 2;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let yp = f.y[y * w + x] as i32;
            let c = f.cb[(y / 2) * cw + x / 2] as i32 - 128;
            let r = f.cr[(y / 2) * cw + x / 2] as i32 - 128;
            let cl = |v: i32| v.clamp(0, 255) as u8;
            let (r8, g8, b8) = (
                cl(yp + ((91881 * r) >> 16)),
                cl(yp - ((22554 * c + 46802 * r) >> 16)),
                cl(yp + ((116130 * c) >> 16)),
            );
            let at = (y * w + x) * 4;
            out[at] = r8;
            out[at + 1] = g8;
            out[at + 2] = b8;
            out[at + 3] = 255;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Picture counting: a hand-built ES with I, P and B pictures (and slice start codes that must not count).
    #[test]
    fn picture_counts_by_type() {
        let mut es = Vec::new();
        let pic = |tc: u8| vec![0, 0, 1, 0, 0, (tc << 3) as u8, 0, 0, 1, 1, 0, 0, 1, 2];
        es.extend(pic(1)); // I
        es.extend(pic(2)); // P
        es.extend(pic(3)); // B
        es.extend(pic(3)); // B
        es.extend([0, 0, 1, 0x01, 0, 0, 1, 0x45]); // a slice, not a picture
        let (total, i, p, b) = picture_counts(&es);
        assert_eq!((total, i, p, b), (4, 1, 1, 2));
        // A tampered type byte changes the split (sister of the golden pictures columns).
        es[5] = 0x18; // second picture is B now
        let (total, i, p, b) = picture_counts(&es);
        assert_eq!((total, i, p, b), (4, 1, 0, 3));
    }
}
