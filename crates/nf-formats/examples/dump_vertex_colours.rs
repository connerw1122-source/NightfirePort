//! Dump vertex RGBA statistics for a level (Task A-C1).
//!
//! Run with: `cargo run --release --example dump_vertex_colours -- 07000001`
//! (needs NF_ISO env var pointing at the disc image).

use nf_formats::{disc, iso9660::IsoImage, level::Level};

fn main() -> anyhow::Result<()> {
    let archive = std::env::args().nth(1).unwrap_or_else(|| "07000005".into());
    let iso_path = std::env::var("NF_ISO").expect("NF_ISO must point at the disc image");
    let iso = IsoImage::open(&iso_path)?;
    let (entries, bin) = disc::files_bin_entries(&iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&archive))
        .ok_or_else(|| anyhow::anyhow!("no {archive} in FILES.BIN"))?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;

    let mut total = 0usize;
    let mut sum = [0u64; 4];
    let mut max = [0u8; 4];
    let mut min = [255u8; 4];
    let nbins = 16;
    let mut hist = [[0u64; 16]; 4];
    let mut low_rgb_high_a = 0usize;
    let mut fully_lit = 0usize;
    let mut white_opaque = 0usize;
    let mut ahist = [0u64; 256];
    let mut warm_pool = 0usize; // warm-orange (R>G>B), low sum, high A
    let mut sample = 0usize;
    let mut warm_bright: Vec<(usize, usize, usize, usize, u8, u8, u8, u8)> = Vec::new();

    let main = level.main_map().unwrap_or(0);
    // Count materials with and without textures.
    let mut with_tex = 0usize;
    let mut no_tex = 0usize;
    for obj in &level.maps[main].objects {
        for leaf in &obj.gfx.leaves {
            for b in &leaf.batches {
                if let Some(t) = b.texture {
                    if (t as usize) < level.maps[main].textures.len() { with_tex += 1; } else { no_tex += 1; }
                } else { no_tex += 1; }
            }
        }
    }
    println!("textures in main map: {}", level.maps[main].textures.len());
    println!("batches with valid texture: {with_tex}");
    println!("batches without texture:    {no_tex}");
    println!();
    for (oi, obj) in level.maps[main].objects.iter().enumerate() {
        for (li, leaf) in obj.gfx.leaves.iter().enumerate() {
            for (bi, b) in leaf.batches.iter().enumerate() {
                for (vi, v) in b.verts.iter().enumerate() {
                    total += 1;
                    for c in 0..4 {
                        sum[c] += v.rgba[c] as u64;
                        max[c] = max[c].max(v.rgba[c]);
                        min[c] = min[c].min(v.rgba[c]);
                        hist[c][(v.rgba[c] as usize) / (256 / nbins)] += 1;
                    }
                    ahist[v.rgba[3] as usize] += 1;
                    let (r, g, bl, a) = (v.rgba[0], v.rgba[1], v.rgba[2], v.rgba[3]);
                    if a >= 0x80 && (r as u16 + g as u16 + bl as u16) < 0x100 { low_rgb_high_a += 1; }
                    if (r as u16 + g as u16 + bl as u16) >= 0x180 && a == 0x80 { fully_lit += 1; }
                    if r == 0xff && g == 0xff && bl == 0xff && a == 0x80 { white_opaque += 1; }
                    // Warm pool: R > G > B with non-trivial brightness and high alpha (baked lamp glow).
                    if a >= 0x80 && r > g && g > bl && (r as u16) > 0x40 { warm_pool += 1; }
                    if vi < 5 && sample < 30 {
                        eprintln!("  obj={oi} leaf={li} batch={bi} v={vi}: rgba=({:#04x},{:#04x},{:#04x},{:#04x})", r, g, bl, a);
                        sample += 1;
                    }
                    // Track the brightest warm-orange vertices (lamp glow).
                    if r as u16 > 0xa0 && r > g && g > bl && warm_bright.len() < 40 {
                        warm_bright.push((oi, li, bi, vi, r, g, bl, a));
                    }
                }
            }
        }
    }

    println!("=== {archive} ===");
    println!("total vertices: {total}");
    let n = total.max(1) as f64;
    println!("mean RGBA: ({:.1},{:.1},{:.1},{:.1})",
        sum[0] as f64 / n, sum[1] as f64 / n, sum[2] as f64 / n, sum[3] as f64 / n);
    println!("min RGBA:  ({:#04x},{:#04x},{:#04x},{:#04x})", min[0], min[1], min[2], min[3]);
    println!("max RGBA:  ({:#04x},{:#04x},{:#04x},{:#04x})", max[0], max[1], max[2], max[3]);
    println!("low_rgb (sum<0x100) + high_a (>=0x80): {low_rgb_high_a} ({:.1}%)", 100.0 * low_rgb_high_a as f64 / n);
    println!("fully_lit (sum>=0x180, A=0x80):       {fully_lit} ({:.1}%)", 100.0 * fully_lit as f64 / n);
    println!("white_opaque (FF FF FF 80):           {white_opaque} ({:.1}%)", 100.0 * white_opaque as f64 / n);
    println!("warm_pool (R>G>B, R>0x40, A>=0x80):   {warm_pool} ({:.1}%)", 100.0 * warm_pool as f64 / n);
    println!("\nPer-channel histogram ({} bins, 0..256):", nbins);
    for c in 0..4 {
        let name = b"RGBA"[c] as char;
        print!("  {name}:");
        for b in 0..nbins {
            let pct = 100.0 * hist[c][b] as f64 / n;
            print!(" [{:>3}-{:<3}]={:>5.1}%", b * 16, b * 16 + 15, pct);
        }
        println!();
    }
    println!("\nAlpha distribution (top 16 values by count):");
    let mut alphas: Vec<(u8, u64)> = (0..256).map(|a| (a as u8, ahist[a])).collect();
    alphas.sort_by(|a, b| b.1.cmp(&a.1));
    for (a, c) in alphas.iter().take(16) {
        if *c == 0 { break; }
        println!("  A={:#04x} ({:3}): {} ({:.1}%)", a, a, c, 100.0 * *c as f64 / n);
    }
    println!("\nBrightest warm-orange vertices (lamp glow candidates, R>0xa0):");
    for (oi, li, bi, vi, r, g, bl, a) in &warm_bright {
        println!("  obj={oi} leaf={li} batch={bi} v={vi}: rgba=({:#04x},{:#04x},{:#04x},{:#04x})", r, g, bl, a);
    }
    Ok(())
}
