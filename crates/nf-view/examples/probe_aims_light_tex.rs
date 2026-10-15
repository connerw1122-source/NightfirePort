//! fix-aims20-lights-render probe: dump the light mesh's texture (batch tex index 12 of the
//! "gunsight3d" map) — palette + average colour, to settle where the game's RED comes from.
//! Run: cargo run --release --example probe_aims_light_tex -- 07000005.bin

use nf_formats::level::Level;

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let level_name = std::env::args().nth(1).unwrap_or_else(|| "07000005.bin".into());
    let iso_img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso_img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&level_name)).expect("archive");
    let data = iso_img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    let (m, o) = level.by_hash.get(&0x0200_0132).copied().expect("mesh");
    let tex_idx = level.maps[m].objects[o].gfx.leaves[0].batches[0].texture.expect("tex");
    println!("mesh map={m} obj={o} tex_idx={tex_idx} textures={}", level.maps[m].textures.len());
    for (i, t) in level.maps[m].textures.iter().enumerate() {
        if i != tex_idx as usize { continue; }
        println!("  tex[{i}] (index {}): {}x{} 8bpp={} frames={} pal={}", i, t.width, t.height, t.is_8bpp(), t.frames, t.palette.len());
        let pal8 = t.palette_rgba8();
        let n = t.width as usize * t.height as usize;
        let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);
        for px in t.frame(0) {
            let c = pal8[*px as usize];
            r += c[0] as u64; g += c[1] as u64; b += c[2] as u64; a += c[3] as u64;
        }
        if n > 0 {
            println!("  frame0 avg rgba = ({},{},{},{})", r / n as u64, g / n as u64, b / n as u64, a / n as u64);
        }
        println!("  palette:");
        for (j, c) in pal8.iter().enumerate() { println!("    [{j}] = {:?}", c); }
    }
}
