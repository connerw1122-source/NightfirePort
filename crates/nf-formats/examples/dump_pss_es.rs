//! Dump the demuxed video elementary stream of a PSS file (our demuxer, the port's real path):
//! `cargo run --release -p nf-formats --example dump_pss_es -- <in.pss> <out.es>`
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let pss = args.next().expect("usage: dump_pss_es <in.pss> <out.es>");
    let out = args.next().expect("missing out");
    let data = std::fs::read(&pss)?;
    let d = nf_formats::pss::demux(&data).map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("packs {}, video pkts {}, video {} bytes, audio {}",
        d.packs, d.video_packets, d.video.len(), d.audio.len());
    std::fs::write(&out, &d.video)?;
    Ok(())
}
