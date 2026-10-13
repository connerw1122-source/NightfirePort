//! fix-final-blockers probe 2b: every 0xdb-family placement on 07000016 (and 15) near the door[7]
//! threshold — including the ones `door_placements` excludes (flags & 0xa000 != 0) — with their
//! resolution and collision, to find the static seal at z −74.13. Diagnostic only.

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn dump_all_0xdb_placements() {
    use nf_formats::{disc, iso9660::IsoImage, level::Level};
    let iso_path = iso();
    for code in ["07000016"] {
        let iso = IsoImage::open(&iso_path).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&format!("{code}.bin"))).expect(code);
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let m = level.main_map().unwrap();
        eprintln!("=== {code}: every placement within 6 of (3.5, y, −74.1) ===");
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            let d2 = (p.translation[0] - 3.5).powi(2) + (p.translation[2] + 74.1).powi(2);
            if d2 > 36.0 { continue; }
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
            let has_collision = target.map(|(mm, oo)| level.maps[mm].objects[oo].collision.is_some()).unwrap_or(false);
            eprintln!(
                "plc {i} kind {:#04x} pos ({:.2},{:.2},{:.2}) flags {:#08x} create_cel {} by_hash {} hash {:#010x} index {} resolved {target:?} collision {has_collision}",
                p.flags & 0xff, p.translation[0], p.translation[1], p.translation[2], p.flags,
                p.flags & 0x8000 != 0, p.by_hash(), p.hashcode, p.index
            );
        }
        eprintln!("=== {code}: every 0xdb placement anywhere (all flag words) ===");
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            if p.flags & 0xff != 0xdb { continue; }
            eprintln!(
                "plc {i} pos ({:.2},{:.2},{:.2}) flags {:#08x} create_cel {}",
                p.translation[0], p.translation[1], p.translation[2], p.flags, p.flags & 0x8000 != 0
            );
        }
    }
}
