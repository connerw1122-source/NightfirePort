//! Dump a raw MPEG-2 elementary stream as packed YUV420 frames (`pack_yuv420`, the ffmpeg `yuv420p`
//! layout) — the port half of the decoder-vs-reference PSNR comparison (task mpeg2-interlaced-decode).
//! Also prints one line per frame: index, picture type, temporal reference.
//!
//! Usage: cargo run --release -p nf-formats --features movies --example dump_mpeg2_yuv -- <in.es> <out.yuv> [max_frames]
#[cfg(feature = "movies")]
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let es_path = args.next().expect("usage: dump_mpeg2_yuv <in.es> <out.yuv> [max_frames]");
    let out_path = args.next().expect("missing out path");
    let max: Option<usize> = args.next().and_then(|s| s.parse().ok());

    let es = std::fs::read(&es_path)?;
    let frames = nf_formats::mpeg2::decode_video(&es)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let frames = match max { Some(n) => &frames[..frames.len().min(n)], None => &frames[..] };
    let mut out = Vec::new();
    for (i, f) in frames.iter().enumerate() {
        let kind = match f.coding_type { 1 => 'I', 2 => 'P', 3 => 'B', _ => '?' };
        println!("frame {i}: {kind} tr {}", f.temporal_reference);
        out.extend_from_slice(&nf_formats::mpeg2::pack_yuv420(f));
    }
    std::fs::write(&out_path, &out)?;
    eprintln!("{} frames -> {} ({} bytes)", frames.len(), out_path, out.len());
    Ok(())
}

#[cfg(not(feature = "movies"))]
fn main() { eprintln!("build with --features movies"); }
