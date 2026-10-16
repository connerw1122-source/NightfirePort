//! movie-audio: the PSS PCM body (kind 1) is BLOCK-interleaved — `interleave` (512) bytes of the left
//! channel, then 512 of the right, in turn [D measured 2026-10-16 on 073B0048]. Read as L/R sample pairs,
//! each channel played every other sample of one channel (double speed, high pitched) with a channel
//! swap at every block (static): the user's playtest 1016. The decode must be the smooth reading.
//! Needs a retail image with movie data (`NF_ISO`).

#[cfg(feature = "movies")]
#[test]
fn movie_pcm_is_block_interleaved() {
    let Ok(p) = std::env::var("NF_ISO") else { return };
    let iso = nf_formats::iso9660::IsoImage::open(&p).unwrap();
    let Ok(m) = nf_view::movie::Movie::load(&iso, "073B0048") else { eprintln!("skip: no movie data"); return };
    let a = m.audio().unwrap();
    assert_eq!((a.header.kind, a.header.channels, a.header.interleave), (1, 2, 512), "PCM, stereo, 512-byte blocks [D]");
    let pcm = a.decode();
    let n = pcm.len().min(400_000);
    let rough = |f: &dyn Fn(usize) -> i64| (1..n).map(|i| (f(i) - f(i - 1)).abs()).sum::<i64>() / n as i64;
    let decoded_l = rough(&|i| pcm[i][0] as i64);
    // Sister: the old sample-pair reading of the same body (every 4 bytes = one "L/R pair").
    let b = &a.body;
    let pairwise_l = rough(&|i| i16::from_le_bytes([b[4 * i], b[4 * i + 1]]) as i64);
    assert!(decoded_l * 3 < pairwise_l * 2, "the decode is the smooth block reading: {decoded_l} vs the pair reading {pairwise_l}");
    assert_eq!(pcm.len(), b.len() / 4, "every sample is decoded (58.6 s at 48 kHz)");
}
