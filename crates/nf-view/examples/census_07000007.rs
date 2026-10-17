//! Census dump for 07000007 (level-07-interactables): every placement grouped by its entity
//! type byte (placement flags & 0xff), with position, params and the object's name. The game
//! dispatches on the type byte at create time (`parsemap_create_dynamic_objects`); the docs'
//! proven bytes: 0xDB door, 0xEA TouchOnce, 0xEB Touch, 0xE8 LoadLevel, 0xE9 LoadPointScript
//! hook, 0x24 PlayerStart. Scratch print, not committed.

use nf_formats::level::Level;

fn main() {
    let Some(iso_path) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&iso_path).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).expect("FILES.BIN");
    let e = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000007.bin")).expect("07000007.bin in FILES.BIN");
    let data = iso.read_bytes(bin.lba as u64 * 2048 + e.offset as u64, e.size as u64).expect("read 07000007.bin");
    let level = Level::parse(&data).expect("parse 07000007");
    let m = level.main_map().expect("main map");
    let main = &level.maps[m];
    println!("maps: {} (main map #{m}: objects: {}, placements: {})", level.maps.len(), main.objects.len(), main.placements.len());
    let by_hash = |h: u32| {
        level.by_hash.get(&h).map(|&(map, object)| (map, object))
    };
    let mut by_type: std::collections::BTreeMap<u8, Vec<usize>> = std::collections::BTreeMap::new();
    for (i, p) in main.placements.iter().enumerate() {
        by_type.entry((p.flags & 0xff) as u8).or_default().push(i);
    }
    for (t, idxs) in &by_type {
        println!("\n=== type {t:#04x}: {} placements ===", idxs.len());
        for &i in idxs {
            let p = &main.placements[i];
            let (objname, hash) = if p.by_hash() {
                match by_hash(p.hashcode) {
                    Some((map, object)) => (level.maps[map].objects[object].name.as_str(), format!("{:08x}", p.hashcode)),
                    None => ("<unresolved>", format!("{:08x}", p.hashcode)),
                }
            } else {
                (main.objects.get(p.index as usize).map(|o| o.name.as_str()).unwrap_or("<obj-oob>"), format!("obj#{}", p.index))
            };
            let bounds = |p: &nf_formats::placement::Placement| -> ([f32;3],[f32;3]) {
                let (omap, oobj) = if p.by_hash() {
                    match by_hash(p.hashcode) { Some(x) => x, None => return ([0.0;3],[0.0;3]) }
                } else { (m, p.index as usize) };
                let o = &level.maps[omap].objects[oobj];
                let mx = nf_formats::placement::object_matrix(p.rotation, p.translation);
                let mut lo = [f32::INFINITY;3]; let mut hi = [f32::NEG_INFINITY;3];
                for x in [o.min[0], o.max[0]] { for y in [o.min[1], o.max[1]] { for z in [o.min[2], o.max[2]] {
                    let v = nf_formats::placement::transform(&mx, [x,y,z]);
                    for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); }
                } } }
                (lo, hi)
            };
            let (blo, bhi) = bounds(p);
            println!(
                "#{i} {hash} {objname:<30} pos ({:.1},{:.1},{:.1}) params {:?} world bounds ({:.1},{:.1},{:.1})..({:.1},{:.1},{:.1})",
                p.translation[0], p.translation[1], p.translation[2],
                p.params, blo[0], blo[1], blo[2], bhi[0], bhi[1], bhi[2]
            );
        }
    }
}
