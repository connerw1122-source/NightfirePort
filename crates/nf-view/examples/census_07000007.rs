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
            println!(
                "#{i} {hash} {objname:<30} pos ({:.1},{:.1},{:.1}) rot({:.2},{:.2},{:.2}) params {:?} extra[7]={:#x} extra[8]={:#x}",
                p.translation[0], p.translation[1], p.translation[2],
                p.rotation[0], p.rotation[1], p.rotation[2],
                p.params, p.extra[7], p.extra[8]
            );
        }
    }
}
