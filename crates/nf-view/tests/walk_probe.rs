//! map-castle probe: per-level dump of starts, triggers, script players, script channel writes,
//! pickups and drones for 07000001..07000004. Scratch evidence for docs/levels/*.md — remove or
//! #[ignore] once the walkthrough tests stand.
//! Run: `NF_ISO=… cargo test -p nf-view --test walk_probe -- --nocapture --test-threads=1`

use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;
use nf_formats::levelbin::LevelBin;
use nf_formats::nis::Nis;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

fn param(p: &nf_formats::placement::Placement, k: u32) -> u32 {
    p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0)
}

fn dump_level(name: &str) {
    let iso = iso();
    let data = read_archive(&iso, name);
    let level = Level::parse(&data).expect("parse");
    let m = level.main_map().expect("main map");
    println!("\n================ {name} ================");
    // Starts (0x24) and exits/triggers.
    for (mi, mm) in level.maps.iter().enumerate() {
        for (pi, p) in mm.placements.iter().enumerate() {
            let f = p.flags & 0xff;
            match f {
                0x24 => println!("START map{mi} #{pi} at ({:.2},{:.2},{:.2}) rot y {:.3} params {:x?}",
                    p.translation[0], p.translation[1], p.translation[2], p.rotation[1], p.params),
                0xe8 | 0xea | 0xeb | 0xec | 0xed | 0xee | 0xef | 0xdc => {
                    let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                        else if (p.index as usize) < mm.objects.len() { Some((mi, p.index as usize)) } else { None };
                    let objname = obj.and_then(|(om, oi)| level.maps.get(om).and_then(|map| map.objects.get(oi)))
                        .map(|o| o.name.trim().to_string()).unwrap_or_default();
                    println!("TRIG map{mi} #{pi} kind {f:02x} at ({:.2},{:.2},{:.2}) obj {objname:?} p0 {} p1 {} p2 {} p3 {} p4 {} params {:x?}",
                        p.translation[0], p.translation[1], p.translation[2],
                        param(p, 0), param(p, 1), param(p, 2), param(p, 3), param(p, 4), p.params);
                }
                0xd9 => {
                    let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                        else if (p.index as usize) < mm.objects.len() { Some((mi, p.index as usize)) } else { None };
                    let objname = obj.and_then(|(om, oi)| level.maps.get(om).and_then(|map| map.objects.get(oi)))
                        .map(|o| o.name.trim().to_string()).unwrap_or_default();
                    println!("SP map{mi} #{pi} at ({:.2},{:.2},{:.2}) obj {objname:?} p(script) {:x} p1 {} p2(mode) {} p3 {} p4 {}",
                        p.translation[0], p.translation[1], p.translation[2],
                        param(p, 0), param(p, 1), param(p, 2), param(p, 3), param(p, 4));
                }
                0xf0 => {
                    println!("PICKUP map{mi} #{pi} at ({:.2},{:.2},{:.2}) p {} id {} rounds {} sw {} snd {} respawn {}",
                        p.translation[0], p.translation[1], p.translation[2],
                        param(p, 0), param(p, 1), param(p, 2) as u16, param(p, 3), param(p, 4), param(p, 5));
                }
                0xf => {
                    let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                        else if (p.index as usize) < mm.objects.len() { Some((mi, p.index as usize)) } else { None };
                    let objname = obj.and_then(|(om, oi)| level.maps.get(om).and_then(|map| map.objects.get(oi)))
                        .map(|o| o.name.trim().to_string()).unwrap_or_default();
                    println!("DRONE map{mi} #{pi} at ({:.2},{:.2},{:.2}) obj {objname:?} params {:x?}",
                        p.translation[0], p.translation[1], p.translation[2], p.params.iter().take(22).collect::<Vec<_>>());
                }
                _ => {}
            }
        }
    }
    // Scripts: kind-7 (NIS) in the archive + packs; kind-5 channel writes.
    let lb = LevelBin::parse(&data).expect("levelbin");
    let mut nis: Vec<Nis> = Vec::new();
    for e in lb.entries.iter().filter(|e| e.kind == 7) {
        if let Ok(n) = Nis::parse(e.hash(), lb.data(&data, e)) { nis.push(n); }
    }
    // Loadable packs via the level's type-0xC records.
    let mut packs: Vec<(u32, u8)> = Vec::new();
    for e in lb.entries.iter().filter(|e| e.kind == 0xC) {
        let r = lb.data(&data, e);
        if r.len() >= 5 {
            let (hash, flag) = (u32::from_le_bytes([r[0], r[1], r[2], r[3]]), r[4]);
            if (1..=3).contains(&flag) { packs.push((hash, flag)); }
        }
    }
    let level_id = u32::from_str_radix(&name[..8], 16).unwrap_or(0);
    for (hash, flag) in packs {
        let pack_name = format!("{:08x}.bin", (level_id & 0xff0f_ffff) | ((flag as u32 + 7) << 20));
        let img = IsoImage::open(&iso).expect("iso");
        let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
        if let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&pack_name)) {
            if let Ok(pack) = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64) {
                if let Ok(plb) = LevelBin::parse(&pack) {
                    for pe in plb.entries.iter().filter(|e| e.kind == 7) {
                        if let Ok(n) = Nis::parse(pe.hash(), plb.data(&pack, pe)) { nis.push(n); }
                    }
                }
            }
        }
    }
    println!("--- scripts: {} archive+pack NIS, {} type-5", nis.len(), lb.entries.iter().filter(|e| e.kind == 5).count());
    for n in nis.iter() {
        let mut ch_writes: Vec<String> = Vec::new();
        let mut has_load = false;
        for (si, st) in n.streams.iter().enumerate() {
            for cmd in st.decode() {
                match &cmd {
                    nf_formats::nis::Cmd::Event { kind, params } => {
                        if *kind == 0xd && params.len() >= 2 { ch_writes.push(format!("s{si}:0xD ch {:#x}={}", params[0], params[1])); }
                        if *kind == 0x11 && params.len() >= 4 { ch_writes.push(format!("s{si}:0x11 ch {:#x}={}", params[2], params[3])); }
                        if *kind == 0x12 { has_load = true; ch_writes.push(format!("s{si}:0x12 LOAD_LEVEL p={params:x?}")); }
                    }
                    _ => {}
                }
            }
        }
        if !ch_writes.is_empty() || has_load {
            println!("NIS {:08x}: {}", n.hashcode, ch_writes.join("; "));
        }
    }
    for e in lb.entries.iter().filter(|e| e.kind == 5) {
        if let Ok(s) = nf_formats::anim::Script::parse(lb.data(&data, e)) {
            let mut writes = Vec::new();
            for c in &s.commands {
                if c.op == 0xd && !c.args.is_empty() { writes.push(format!("0xD ch {:#x}={:?}", c.args[0], c.args.get(1))); }
                if c.op == 0x11 && c.args.len() >= 4 { writes.push(format!("0x11 ch {:#x}={}", c.args[2], c.args[3])); }
            }
            if !writes.is_empty() { println!("T5 {:08x}: {}", s.hashcode, writes.join("; ")); }
        }
    }
}

#[test]
fn dump_castle_levels() {
    for name in ["07000001.bin", "07000002.bin", "07000003.bin", "07000004.bin"] {
        dump_level(name);
    }
}

#[test]
fn dump_all_nis_events() {
    let isov = iso();
    for name in ["07000001.bin", "07000002.bin", "07000003.bin", "07000004.bin"] {
        let data = read_archive(&isov, name);
        let lb = LevelBin::parse(&data).expect("levelbin");
        let level_id = u32::from_str_radix(&name[..8], 16).unwrap_or(0);
        println!("\n==== all NIS events {name} ====");
        let mut dump = |lb2: &LevelBin, d2: &[u8], tag: &str| {
            for e in lb2.entries.iter().filter(|e| e.kind == 7) {
                let Ok(n) = Nis::parse(e.hash(), lb2.data(d2, e)) else { continue };
                let mut evs: Vec<String> = Vec::new();
                for (si, st) in n.streams.iter().enumerate() {
                    for cmd in st.decode() {
                        if let nf_formats::nis::Cmd::Event { kind, params } = &cmd {
                            evs.push(format!("s{si}:e{kind:#x}({:x?})", params.iter().take(6).collect::<Vec<_>>()));
                        }
                    }
                }
                if !evs.is_empty() { println!("{tag} {:08x}: {}", n.hashcode, evs.join(" | ")); }
            }
        };
        dump(&lb, &data, "arc");
        let img = IsoImage::open(&isov).expect("iso");
        let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
        for e in lb.entries.iter().filter(|e| e.kind == 0xC) {
            let r = lb.data(&data, e);
            if r.len() < 5 { continue; }
            let (hash, flag) = (u32::from_le_bytes([r[0], r[1], r[2], r[3]]), r[4]);
            if !(1..=3).contains(&flag) { continue; }
            let pack_name = format!("{:08x}.bin", (level_id & 0xff0f_ffff) | ((flag as u32 + 7) << 20));
            if let Some(fe) = entries.iter().find(|en| en.name.eq_ignore_ascii_case(&pack_name)) {
                if let Ok(pack) = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64) {
                    if let Ok(plb) = LevelBin::parse(&pack) { dump(&plb, &pack, &format!("pack{flag}")); }
                }
            }
        }
    }
}
