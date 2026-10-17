//! Census v3 (task 750): on 07000005 — who sets channels 112/113/114 (0x70-0x72) and 127-129/133,
//! the inputs of the bond AND triggers; and every Switch (0x29) / Destroy (0xe1) / Lock / Sensor object
//! with its out channel. Opt-in NF_ISO. Scratch.
fn main() {
    let Some(iso_path) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&iso_path).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).expect("FILES.BIN");
    let read = |name: &str| -> Vec<u8> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap();
        iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap()
    };
    let data = read("07000005.bin");
    let lb = nf_formats::levelbin::LevelBin::parse(&data).unwrap();
    for se in lb.entries.iter().filter(|e| e.kind == 7) {
        let h = se.hash();
        let Ok(nis) = nf_formats::nis::Nis::parse(h, lb.data(&data, se)) else { continue };
        for (si, s) in nis.streams.iter().enumerate() {
            for (ci, cmd) in s.decode().iter().enumerate() {
                match cmd {
                    nf_formats::nis::Cmd::Event { kind, params } => {
                        for (pi, v) in params.iter().enumerate() {
                            if [112,113,114,127,128,129,133].contains(v) {
                                println!("script {h:08x} stream {si} cmd#{ci} EVENT {kind:#x} param[{pi}] = {v} (all {params:x?})");
                            }
                        }
                    }
                    nf_formats::nis::Cmd::Entity { params, obj_type, .. } => {
                        for (pi, v) in params.iter().enumerate() {
                            if [112,113,114,127,128,129,133].contains(v) {
                                println!("script {h:08x} stream {si} cmd#{ci} ENTITY {obj_type:#x} param[{pi}] = {v} (all {params:x?})");
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let lvl = nf_formats::level::Level::parse(&data).unwrap();
    for (mi, map) in lvl.maps.iter().enumerate() {
        for (i, p) in map.placements.iter().enumerate() {
            let t = p.flags & 0xff;
            // typed triggers: out = raw p0; switches/destroy/etc: dump all params for interesting types
            let p0 = p.params.iter().find(|(j, _)| *j == 0).map(|&(_, v)| v).unwrap_or(0);
            if matches!(t, 0xe8..=0xef | 0xdc) && [112,113,114,127,128,129,133].contains(&p0) {
                println!("map {mi} placement #{i} flags {:#x} TRIGGER out {p0} params {:?}", p.flags, p.params);
            }
            if matches!(t, 0x29 | 0xe1 | 0x2e | 0x2f) {
                println!("map {mi} placement #{i} flags {:#x} OBJ params {:?}", p.flags, p.params);
            }
        }
    }
}
