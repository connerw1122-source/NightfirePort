//! fix-missing-objects probe 2: name search across the whole archive — every placement whose
//! object name matches the furniture/prop keywords, with classification; plus the DISTINCT names
//! of every not-drawn placement that has a mesh. Opt-in via NF_ISO.

use nf_formats::level::Level;
use nf_formats::levelbin::LevelBin;
use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

const KEYWORDS: &[&str] = &["fish", "fountain", "basin", "tier", "bed", "desk", "chair", "map", "clock",
    "locker", "fuse", "pylon", "wire", "cable", "stool", "table", "bench", "shelf", "cabinet", "sink",
    "toilet", "pot", "plant", "lamp", "barrel", "crate", "box", "sack"];

fn has_mesh(level: &Level, map: usize, object: usize) -> bool {
    level.maps[map].objects[object].gfx.leaves.iter().any(|l| !l.batches.is_empty())
}

#[test]
fn probe_name_search() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for archive in ["07000006.bin", "0700000c.bin"] {
        let data = read_archive(&iso, archive);
        let level = Level::parse(&data).expect("parse");
        let mm = level.main_map().unwrap();
        eprintln!("\n=== {archive}: placements matching keywords ===");
        for (pi, p) in level.maps[mm].placements.iter().enumerate() {
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < level.maps[mm].objects.len() { Some((mm, p.index as usize)) } else { None };
            let Some((map, object)) = target else { continue };
            let name = level.maps[map].objects[object].name.trim().to_lowercase();
            if !KEYWORDS.iter().any(|k| name.contains(k)) { continue; }
            let kind = p.flags & 0xff;
            let cel = p.creates_cel();
            let flags = level.maps[map].objects[object].flags;
            let mesh = has_mesh(&level, map, object);
            eprintln!("  #{pi:<4} kind {kind:#04x} cel {cel:>5} objf {flags:#06x} mesh {mesh:>5} at ({:.1},{:.1},{:.1}) \"{}\"",
                p.translation[0], p.translation[1], p.translation[2], level.maps[map].objects[object].name.trim());
        }
        eprintln!("=== {archive}: DISTINCT names of NOT-DRAWN placements WITH a mesh ===");
        let mut names: std::collections::BTreeMap<String, usize> = Default::default();
        for p in level.maps[mm].placements.iter() {
            let kind = p.flags & 0xff;
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < level.maps[mm].objects.len() { Some((mm, p.index as usize)) } else { None };
            let Some((map, object)) = target else { continue };
            let obj = &level.maps[map].objects[object];
            let mesh = has_mesh(&level, map, object);
            if !mesh { continue; }
            // The same skip logic as the scene build (paths that DO draw excluded):
            let drawn = match kind {
                0x2a | 0x22 | 0xd4 | 0x3e | 0xfc | 0xdb | 0xde | 0xf0 => true,
                0x3a | 0x2e | 0x30 | 0x34 | 0x35 | 0x3b | 0x71 | 0xd5 | 0xd8 | 0xe1 | 0xe3 | 0xda | 0xfa | 0xfd | 0xfe | 0x4a => true,
                0x0f => true,
                _ if obj.flags & 0x10 != 0 => false,               // cel pass skip
                _ if p.creates_cel() => true,                       // static
                _ if matches!(kind, 0x20 | 0xe2) => true,           // breakable/SS static ext
                _ => false,                                          // NO DRAW PATH
            };
            if !drawn {
                *names.entry(obj.name.trim().to_string()).or_default() += 1;
            }
        }
        for (name, n) in &names {
            eprintln!("  {n:3} × \"{name}\"");
        }
    }
}
