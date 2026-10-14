//! Scratch probe (task 289): dump 07000007's script 0x0600007A (the urn SP) — streams, commands.
use nf_formats::nis::Cmd;

fn main() {
    let iso_path = std::env::var("NF_ISO").expect("NF_ISO");
    let iso = nf_formats::iso9660::IsoImage::open(&iso_path).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000007.bin")).expect("07000007.bin");
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let lb = nf_formats::levelbin::LevelBin::parse(&data).unwrap();
    for se in lb.entries.iter().filter(|e| e.kind == 7) {
        if se.hash() != 0x0600_007a { continue; }
        let nis = nf_formats::nis::Nis::parse(se.hash(), lb.data(&data, se)).unwrap();
        println!("script {:08x} len {} streams {}", nis.hashcode, nis.length, nis.streams.len());
        for (si, s) in nis.streams.iter().enumerate() {
            println!(" stream {si}: keys {}..{}", s.first_key, s.last_key);
            for c in s.decode() {
                match c {
                    Cmd::Entity { hash, parent, spline, obj_type, params } =>
                        println!("   Entity glist {hash:08x} parent {parent:?} spline {spline} type {obj_type:02x} params {params:?}"),
                    Cmd::Anim { skin, script, start } => println!("   Anim skin {skin:08x} script {script:08x} start {start}"),
                    Cmd::Event { kind, params } => println!("   Event kind {kind:02x} params {params:?}"),
                    other => println!("   {other:?}"),
                }
            }
        }
        for (ki, k) in nis.keys.iter().enumerate() {
            if ki > 24 { break; }
            println!(" key {ki}: t {} pos {:?} scale {} f24 {}", k.time, k.pos, k.scale, k.f24());
        }
    }
    // Also every placement's flags/params summary for kinds 0x20/0xe1/0xd9 mask-2 SPs on 07000007.
    let level = nf_formats::level::Level::parse(&data).unwrap();
    let m = level.main_map().unwrap();
    for (i, p) in level.maps[m].placements.iter().enumerate() {
        let kind = p.flags & 0xff;
        if matches!(kind, 0x20 | 0xe1 | 0xd9) {
            let mask = p.params.iter().find(|(j, _)| *j == 5).map(|&(_, v)| v).unwrap_or(0);
            println!("placement #{i} kind {kind:02x} rot {:?} trans {:?} params {:?}", p.rotation, p.translation, p.params);
        }
    }
}
