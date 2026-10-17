//! Scratch probe (not committed): every placement of type 0x20 (Break_Create 0x12c230) on the
//! disc — per level archive: params (type/health/filter), resolved object bounds.

use std::collections::BTreeMap;

fn main() {
    let Some(iso_path) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&iso_path).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).expect("FILES.BIN");
    let mut total = 0usize;
    let mut per_kind: BTreeMap<u32, usize> = BTreeMap::new();
    let mut per_level: BTreeMap<String, usize> = BTreeMap::new();
    for e in entries.iter() {
        let name = e.name.clone();
        let lower = name.to_ascii_lowercase();
        if !(lower.len() == 12 && lower.ends_with(".bin") && lower[..8].bytes().all(|b| b.is_ascii_hexdigit())) { continue }
        let Ok(data) = iso.read_bytes(bin.lba as u64 * 2048 + e.offset as u64, e.size as u64) else { continue };
        let level = match nf_formats::level::Level::parse(&data) { Ok(l) => l, Err(_) => continue };
        let Some(m) = level.main_map() else { continue };
        for p in &level.maps[m].placements {
            if p.flags & 0xff != 0x20 { continue }
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0, |&(_, v)| v);
            let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(map, object)| &level.maps[map].objects[object]) }
                else { level.maps[m].objects.get(p.index as usize) };
            let (o_name, minmax) = obj.map_or(("<none>".to_string(), ([0.0f32; 3], [0.0f32; 3])), |o| {
                (o.name.clone(), (o.min, o.max))
            });
            let sane = [minmax.0, minmax.1].iter().all(|b| b.iter().all(|v| v.abs() < 1.0e6));
            total += 1;
            *per_kind.entry(param(0)).or_default() += 1;
            *per_level.entry(name[..8].to_string()).or_default() += 1;
            if matches!(name.as_str(), "07000005.bin" | "07000006.bin" | "07000008.bin") {
                let coll = obj.and_then(|o| o.collision.as_ref()).map(|c| {
                    let ms: Vec<u8> = c.materials.iter().copied().collect();
                    format!("ntri={} mats={:?}", c.triangles().len(), &ms[..ms.len().min(12)])
                }).unwrap_or_else(|| "no-collision".into());
                println!("{name} plc#{} obj={}({}) kind={} health={} filter={:#x} t=({:.1},{:.1},{:.1}) min=({:.1},{:.1},{:.1}) max=({:.1},{:.1},{:.1}) sane={} objflags={:#x} coll={coll}",
                    p.index, o_name, if p.by_hash() { format!("hash {:08x}", p.hashcode) } else { "index".to_string() },
                    param(0), f32::from_bits(param(1)), param(2), p.translation[0], p.translation[1], p.translation[2],
                    minmax.0[0], minmax.0[1], minmax.0[2], minmax.1[0], minmax.1[1], minmax.1[2], sane, obj.map_or(0, |o| o.flags));
            }
        }
    }
    println!("total 0x20 placements: {total}; by kind: {per_kind:?}");
    println!("per level: {per_level:?}");
}
// EOF marker not used
