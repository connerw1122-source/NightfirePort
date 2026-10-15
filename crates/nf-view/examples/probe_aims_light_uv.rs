//! fix-aims20-lights-render probe: the light mesh's UVs + vertex layout — which texels do the
//! light quads sample? Run: cargo run --release --example probe_aims_light_uv -- 07000005.bin
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
    let obj = &level.maps[m].objects[o];
    let t = &level.maps[m].textures[12];
    let pal = t.palette_rgba8();
    println!("mesh {}x{} tex 64x64; verts:", obj.name, 0);
    for (i, b) in obj.gfx.leaves[0].batches[0].verts.iter().enumerate() {
        let u = (b.uv[0].clamp(0.0, 1.0) * 63.0) as usize;
        let v = (b.uv[1].clamp(0.0, 1.0) * 63.0) as usize;
        let texel = pal[t.frame(0)[v * 64 + u] as usize];
        println!("  v{i}: pos=({:.4},{:.4},{:.4}) uv=({:.3},{:.3}) -> texel{}={:?} rgba={:?} nrm={:?}",
            b.pos[0], b.pos[1], b.pos[2], b.uv[0], b.uv[1], v * 64 + u, texel, b.rgba, b.normal);
    }
    let tris: Vec<_> = obj.gfx.leaves[0].batches[0].triangles().into_iter().collect();
    println!("triangles: {:?}", tris);
}
