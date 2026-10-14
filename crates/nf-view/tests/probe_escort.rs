//! probe: 07000001 + 0700000a trigger placements + their referenced objects' collision
use nf_formats::level::Level;
use nf_formats::iso9660::IsoImage;
fn dump(iso: &str, name: &str) {
    let ii = IsoImage::open(iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&ii).expect("files.bin");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("no {name}"));
    let data = ii.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("data");
    let level = Level::parse(&data).expect("level");
    let mm = level.main_map().unwrap_or(0);
    eprintln!("=== {name}: main map {mm}, {} maps", level.maps.len());
    for (mi, m) in level.maps.iter().enumerate() {
        for (pi, p) in m.placements.iter().enumerate() {
            if p.flags == 0xe8 || p.flags == 0xeb || p.flags == 0xea {
                let params: Vec<String> = p.params.iter().take(12).map(|(k, v)| format!("{k}:{v}")).collect();
                let target = if p.by_hash() { format!("hash 0x{:x}", p.hashcode) } else { format!("obj {}", p.index) };
                let tris = if p.by_hash() {
                    level.by_hash.get(&p.hashcode).and_then(|&(om, oi)| level.maps[om].objects[oi].collision.as_ref()).map(|c| c.triangles().len()).unwrap_or(9999)
                } else if (p.index as usize) < m.objects.len() {
                    m.objects[p.index as usize].collision.as_ref().map(|c| c.triangles().len()).unwrap_or(9999)
                } else { 9998 };
                eprintln!("map {mi} pl {pi}: flags 0x{:x} {target} tri_count {tris} params [{}]", p.flags, params.join(", "));
                if p.by_hash() {
                    if let Some(&(om, oi)) = level.by_hash.get(&p.hashcode) {
                        let o = &level.maps[om].objects[oi];
                        eprintln!("   obj: maps[{om}][{oi}] kind? glist? name {}", Some(o.name.as_str()).unwrap_or("?"));
                    }
                }
            }
        }
    }
}
#[test]
fn probe_escort_triggers() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    dump(&iso, "07000001.bin");
    dump(&iso, "0700000a.bin");
}
