//! fix-aims20-lights-render probe: dump the AIMS-20 side-light mesh 0x2000132's material state —
//! the glist flags (celglist+0x44, the AnimObjectDraw 0x1c4490 tweak/0x800 bit + the light-selection
//! word), per-batch GS blend/test/zbuf and vertex colours. Scans the story archives for the hash.
//! Run: cargo run --release --example probe_aims_light_mesh -- [first-archive] [last-archive]

use nf_formats::level::Level;

fn main() {
    let iso = std::env::var("NF_ISO").expect("NF_ISO");
    let first: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let last: u32 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(16);
    let iso_img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso_img).unwrap();
    let read = |name: &str| -> Option<Vec<u8>> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
        iso_img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()
    };
    for n in first..=last {
        let name = format!("0700{n:04}.bin");
        let Some(data) = read(&name) else { continue };
        let Ok(level) = Level::parse(&data) else { continue };
        for hash in [0x0200_0132u32] {
            let Some(&(m, o)) = level.by_hash.get(&hash) else { continue };
            let obj = &level.maps[m].objects[o];
            println!("=== {hash:#010x} in {name} map={m} obj={o} name={:?} flags={:#010x} (lflags={:#06x}, 0x800={}) centre={:?} r={:.3}",
                obj.name, obj.flags, obj.flags & 0xffff, obj.flags & 0x800 != 0, obj.centre, obj.radius);
            let mut bi_total = 0usize;
            for (li, leaf) in obj.gfx.leaves.iter().enumerate() {
                let (mut alpha, mut test, mut zbuf) = (0x44u64, 0x5001bu64, 0x3100_0000u64);
                let mut gs_idx = 0usize;
                for (bi, b) in leaf.batches.iter().enumerate() {
                    while gs_idx < leaf.gs.len() && leaf.gs[gs_idx].2 <= bi {
                        let (r, v, _) = leaf.gs[gs_idx];
                        match r { 0x42 => alpha = v, 0x47 => test = v, 0x4e => zbuf = v, _ => {} }
                        gs_idx += 1;
                    }
                    let d = nf_view::scene::Draw::from_gs(5, 0, alpha, test, zbuf);
                    let n = b.verts.len();
                    let mut cmin = [255u32; 3]; let mut cmax = [0u32; 3]; let mut asum = [0u64; 4];
                    for v in &b.verts {
                        for k in 0..3 { cmin[k] = cmin[k].min(v.rgba[k] as u32); cmax[k] = cmax[k].max(v.rgba[k] as u32); }
                        for k in 0..4 { asum[k] += v.rgba[k] as u64; }
                    }
                    let avg = |k: usize| if n > 0 { asum[k] / n as u64 } else { 0 };
                    println!("  leaf{li} batch{bi}: n={n} blend={} zwrite={} atest={} fog={} tex={:?} rgba_avg=({},{},{},{}) r[{},{}] g[{},{}] b[{},{}] skinned={}",
                        d.blend, d.zwrite, d.atest, d.fog, b.texture, avg(0), avg(1), avg(2), avg(3),
                        cmin[0], cmax[0], cmin[1], cmax[1], cmin[2], cmax[2], b.skinned);
                    bi_total += 1;
                }
            }
            println!("  leaves={} batches={bi_total} textures_in_map={}", obj.gfx.leaves.len(), level.maps[m].textures.len());
        }
    }
}
