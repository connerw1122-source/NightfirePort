//! helicopter probe: every 0x2F (`Copter_Create` 0x17d340) placement across the disc's level
//! archives, with the raw record words (the u16 at +2 is the game's path-frame count, dropped by
//! the port's parse), plus every object name that smells of helicopter. Opt-in via NF_ISO.

use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

#[test]
fn probe_helicopters() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&iso).expect("open iso");
    let (entries, _bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let archives: Vec<String> = entries.iter()
        .filter_map(|e| {
            let n = e.name.to_ascii_uppercase();
            if n.starts_with("070000") && n.ends_with(".BIN") { Some(e.name.clone()) } else { None }
        })
        .collect();
    for archive in &archives {
        let data = read_archive(&iso, archive);
        let level = match nf_formats::level::Level::parse(&data) {
            Ok(l) => l,
            Err(e) => { eprintln!("{archive}: parse failed {e}"); continue }
        };
        // Raw placement payloads per map entry, for the record words the port's parse drops.
        let lb = nf_formats::levelbin::LevelBin::parse(&data).expect("levelbin");
        let mut raw_payloads: Vec<Option<Vec<u8>>> = vec![None; lb.entries.len()];
        for (entry, e) in lb.entries.iter().enumerate() {
            if !matches!(e.kind, 0 | 1 | 2 | 11) { continue }
            let mdata = lb.data(&data, e);
            let Ok(mf) = nf_formats::map::MapFile::parse(mdata) else { continue };
            for blk in &mf.blocks {
                if blk.id == 0x1a {
                    raw_payloads[entry] = Some(blk.payload(mdata).to_vec());
                }
            }
        }
        for (mi, map) in level.maps.iter().enumerate() {
            let raw = raw_payloads[map.entry].as_deref();
            let mut rec = 0usize;
            let mut off = 0usize;
            for (pi, p) in map.placements.iter().enumerate() {
                let kind = p.flags & 0xff;
                let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                    else if (p.index as usize) < map.objects.len() { Some((mi, p.index as usize)) } else { None };
                let name = target.map(|(m, o)| level.maps[m].objects[o].name.trim().to_string()).unwrap_or_default();
                let lower = name.to_lowercase();
                let heli_name = lower.contains("heli") || lower.contains("copter");
                if kind == 0x2f || heli_name {
                    let mut rawinfo = String::new();
                    if let Some(pl) = raw {
                        let n = u32::from_le_bytes(pl[off + 0x48..off + 0x4c].try_into().unwrap()) as usize;
                        if (flags_at(pl, off) & 0xff) == kind {
                            let w0 = u32::from_le_bytes(pl[off..off + 4].try_into().unwrap());
                            rawinfo = format!(" RAW w0={w0:#010x} (pathcount={})", w0 >> 16);
                        }
                        let _ = n;
                    }
                    eprintln!("{archive} map{mi} placement#{pi} kind={kind:#04x} index={} hash={:#x} flags={:#010x} params={:?} name=\"{name}\" pos={:?} rot={:?}{rawinfo}",
                        p.index, p.hashcode, p.flags, p.params, p.translation, p.rotation);
                }
                if let Some(pl) = raw {
                    let n = u32::from_le_bytes(pl[off + 0x48..off + 0x4c].try_into().unwrap()) as usize;
                    off += 0x4c + 8 * n;
                }
                rec += 1;
            }
        }
    }
}

fn flags_at(pl: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(pl[off + 8..off + 12].try_into().unwrap())
}
