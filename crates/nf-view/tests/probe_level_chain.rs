//! probe: how do the no-exit levels chain? placement types, script events (esp. 0x12 LoadLevel),
//! 0xD9 script players and 0xE8 triggers per level — 06/09/0a/11/12/4a plus 05 as the contrast.
use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_formats::nis::Cmd;

#[test]
fn probe_level_chain() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let ii = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&ii).expect("files.bin");
    for code in [
        0x0700_0005u32, 0x0700_0006, 0x0700_0007, 0x0700_0009, 0x0700_000a, 0x0700_000b,
        0x0700_0011, 0x0700_0012, 0x0700_004a,
    ] {
        let name = format!("{code:08x}.bin");
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)) else {
            eprintln!("{name}: MISSING"); continue;
        };
        let data = ii.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("data");
        let level = Level::parse(&data).expect("level");
        eprintln!("=== {name}");
        // placement type histogram
        let mut types: Vec<(u32, usize)> = Vec::new();
        for m in &level.maps {
            for p in &m.placements {
                match types.iter_mut().find(|(t, _)| *t == p.flags) {
                    Some((_, n)) => *n += 1,
                    None => types.push((p.flags, 1)),
                }
            }
        }
        types.sort();
        let sum: String = types.iter().map(|(t, n)| format!("{t:x}:{n}")).collect::<Vec<_>>().join(" ");
        eprintln!("  placement types: {sum}");
        for (mi, m) in level.maps.iter().enumerate() {
            for p in &m.placements {
                if (0xd8..=0xef).contains(&p.flags) {
                    let ps: Vec<String> = p.params.iter().map(|(a, b)| format!("{a:x}={b:x}")).collect();
                    eprintln!("  map{mi} p{} flags {:x} params [{}]", p.index, p.flags, ps.join(" "));
                }
            }
        }
        // script events
        let mut hashes: Vec<&u32> = level.nis.keys().collect();
        hashes.sort();
        for hash in hashes {
            let nis = &level.nis[hash];
            for (si, s) in nis.streams.iter().enumerate() {
                for cmd in s.decode() {
                    if let Cmd::Event { kind, params } = cmd {
                        let ps: Vec<String> = params.iter().map(|p| format!("{p:08x}")).collect();
                        eprintln!("  script {hash:08x} stream{si} event kind {kind:#x} params [{}]", ps.join(" "));
                    }
                }
            }
        }
    }
}

#[test]
fn probe_level_chain2() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let ii = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&ii).expect("files.bin");
    // 1) main map vs the 0xE8s; 2) the loadable packs' events
    let read = |name: &str| -> Option<Vec<u8>> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
        Some(ii.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("data"))
    };
    for code in [0x0700_0006u32, 0x0700_0009, 0x0700_000a, 0x0700_0011, 0x0700_0012, 0x0700_004a, 0x0700_0005] {
        let name = format!("{code:08x}.bin");
        let Some(data) = read(&name) else { eprintln!("{name}: MISSING"); continue };
        let level = Level::parse(&data).expect("level");
        let main = level.main_map();
        eprintln!("=== {name}: {} maps, main = {:?} (kind1)", level.maps.len(), main);
        for (mi, m) in level.maps.iter().enumerate() {
            for p in &m.placements {
                if p.flags == 0xe8 { eprintln!("  0xE8 in map{mi} (main={}) p{} next={:x}", main == Some(mi), p.index, p.params.iter().find(|(j, _)| *j == 0).map(|(_, v)| v).unwrap_or(&0)); }
            }
        }
    }
    for pack in ["07800006.bin", "07900006.bin", "07a00006.bin", "07800009.bin", "07900009.bin", "07a00009.bin",
                 "0780000a.bin", "0790000a.bin", "07800011.bin", "07900011.bin", "07800012.bin", "07900012.bin",
                 "0780004a.bin", "0790004a.bin", "07a0004a.bin", "0780000b.bin", "0790000b.bin", "07a0000b.bin"] {
        let Some(data) = read(pack) else { eprintln!("{pack}: absent"); continue };
        let level = Level::parse(&data).expect("level");
        eprintln!("=== pack {pack}: {} scripts", level.nis.len());
        let mut hashes: Vec<&u32> = level.nis.keys().collect();
        hashes.sort();
        for hash in hashes {
            let nis = &level.nis[hash];
            for (si, s) in nis.streams.iter().enumerate() {
                for cmd in s.decode() {
                    if let Cmd::Event { kind, params } = cmd {
                        let ps: Vec<String> = params.iter().map(|p| format!("{p:08x}")).collect();
                        eprintln!("  script {hash:08x} stream{si} event kind {kind:#x} params [{}]", ps.join(" "));
                    }
                }
            }
        }
    }
}

#[test]
fn probe_level_chain3() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let ii = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&ii).expect("files.bin");
    for code in [0x0700_0006u32, 0x0700_0009, 0x0700_0011, 0x0700_004a] {
        let name = format!("{code:08x}.bin");
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)).unwrap();
        let data = ii.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let m = level.main_map().unwrap();
        eprintln!("=== {name} main map{m}");
        for p in &level.maps[m].placements {
            let q = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
            let inputs: Vec<String> = (2..=8).map(|k| format!("{:x}", q(k))).collect();
            match p.flags {
                0xea | 0xeb => eprintln!("  {:x} p{} TOUCH out={:x} mask={:x} in=[{}] lock={:x}", p.flags, p.index, q(0), q(1), inputs.join(","), q(7)),
                0xec | 0xed | 0xee | 0xef => eprintln!("  {:x} p{} MUX out={:x} mask={:x} in=[{}]", p.flags, p.index, q(0), q(9), inputs.join(",")),
                0xdc => eprintln!("  dc p{} KIND={:x} out={:x} in=[{}]", p.index, q(0), q(1), inputs.join(",")),
                _ => {}
            }
        }
    }
}

#[test]
fn probe_level_chain4() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let ii = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&ii).expect("files.bin");
    for code in [0x0700_0011u32] {
        let name = format!("{code:08x}.bin");
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)).unwrap();
        let data = ii.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        for (mi, m) in level.maps.iter().enumerate() {
            for p in &m.placements {
                let q = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
                let hit = matches!(p.flags, 0xea | 0xeb | 0xec | 0xed | 0xee | 0xef | 0xdc | 0xe8 | 0xd9)
                    && ((1..=8).any(|k| q(k) == 0x1f || q(k) == 0x1e) || q(0) == 0x1f || q(0) == 0x1e);
                if hit {
                    let ps: Vec<String> = p.params.iter().map(|(a, b)| format!("{a:x}={b:x}")).collect();
                    eprintln!("map{mi} p{} flags {:x} params [{}]", p.index, p.flags, ps.join(" "));
                }
            }
        }
        eprintln!("11: kinds of maps: {:?}", level.maps.iter().map(|m| m.kind).collect::<Vec<_>>());
    }
}

#[test]
fn probe_level_chain5() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let ii = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&ii).expect("files.bin");
    for code in [0x0700_0011u32, 0x0700_0009] {
        let name = format!("{code:08x}.bin");
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)).unwrap();
        let data = ii.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        eprintln!("=== {name}");
        for (hash, nis) in &level.nis {
            for (si, s) in nis.streams.iter().enumerate() {
                for cmd in s.decode() {
                    if let Cmd::Entity { obj_type, params, .. } = cmd {
                        let ps: Vec<String> = params.iter().map(|p| format!("{p:x}")).collect();
                        eprintln!("  script {hash:08x} stream{si} ENTITY type {obj_type:#x} params [{}]", ps.join(" "));
                    }
                }
            }
        }
    }
}
