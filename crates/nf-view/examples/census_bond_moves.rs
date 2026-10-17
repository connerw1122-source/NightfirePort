//! Census (task 750 bonus-007) v2: find EVERYTHING on the disc that touches switch channels
//! 0x65..=0x6E (the bond-move channels `Trigger_Calc` 0x1b1900:12-20 polls). Scans:
//!   1. every placement param (index, value) with value in range, any type;
//!   2. every script Entity command param with value in range;
//!   3. every script Event 0xD (set switch) / 0x11 (conditional set) with channel in range;
//!   4. SoundTrigger (type 0xF9) placements in full (their gate may be a bond channel).
//! Opt-in via NF_ISO. Scratch print, not committed.

fn is_bond(v: u32) -> bool { (0x65..=0x6e).contains(&v) }

fn main() {
    let Some(iso_path) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&iso_path).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).expect("FILES.BIN");
    let read = |name: &str| -> Vec<u8> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap();
        iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap()
    };
    let story: [u32; 20] = [
        0x07000001, 0x07000002, 0x07000003, 0x07000004, 0x07000005, 0x07000006, 0x07000007,
        0x07000008, 0x07000009, 0x0700000a, 0x0700000b, 0x0700000c, 0x0700000d, 0x07000011,
        0x07000012, 0x07000013, 0x07000014, 0x07000015, 0x07000016, 0x0700001b,
    ];
    for level in story {
        let name = format!("{level:08x}.bin");
        let data = read(&name);
        let Ok(lb) = nf_formats::levelbin::LevelBin::parse(&data) else { continue };
        let mut found: Vec<String> = Vec::new();
        // scripts
        for se in lb.entries.iter().filter(|e| e.kind == 7) {
            let h = se.hash();
            let Ok(nis) = nf_formats::nis::Nis::parse(h, lb.data(&data, se)) else { continue };
            for (si, s) in nis.streams.iter().enumerate() {
                for (ci, cmd) in s.decode().iter().enumerate() {
                    match cmd {
                        nf_formats::nis::Cmd::Event { kind, params } => {
                            for (pi, v) in params.iter().enumerate() {
                                if is_bond(*v) {
                                    found.push(format!("script {:08x} stream {si} cmd#{ci} EVENT kind {kind:#x} param[{pi}] = {v:#x} (all {params:x?})", h));
                                }
                            }
                        }
                        nf_formats::nis::Cmd::Entity { params, obj_type, .. } => {
                            for (pi, v) in params.iter().enumerate() {
                                if is_bond(*v) {
                                    found.push(format!("script {:08x} stream {si} cmd#{ci} ENTITY type {obj_type:#x} param[{pi}] = {v:#x} (all {params:x?})", h));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // placements
        if let Ok(lvl) = nf_formats::level::Level::parse(&data) {
            for (mi, map) in lvl.maps.iter().enumerate() {
                for (i, p) in map.placements.iter().enumerate() {
                    if p.flags & 0xff == 0xf9 {
                        found.push(format!("map {mi} placement #{i} SOUNDTRIGGER flags {:#x} params {:?} extra {:x?}", p.flags, p.params, p.extra));
                    }
                    for (j, v) in p.params.iter() {
                        if is_bond(*v) {
                            found.push(format!("map {mi} placement #{i} flags {:#x} param[{j}] = {v:#x} (all {:?})", p.flags, p.params));
                        }
                    }
                }
            }
        }
        println!("== {name} ==");
        if found.is_empty() { println!("  (none)"); }
        for f in found { println!("  {f}"); }
    }
}
