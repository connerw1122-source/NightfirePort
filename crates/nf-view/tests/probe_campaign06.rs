//! probe: scan 07000006's placements for 0xE8 (LOAD_LEVEL) trigger wrappers across ALL maps
use nf_formats::level::Level;
use nf_formats::iso9660::IsoImage;
#[test]
fn probe_06_e8() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    let ii = IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&ii).expect("files.bin");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000006.bin")).expect("archive");
    let data = ii.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("data");
    let level = Level::parse(&data).expect("level");
    for (mi, m) in level.maps.iter().enumerate() {
        for (pi, p) in m.placements.iter().enumerate() {
            if (0xe8..=0xef).contains(&p.flags) {
                eprintln!("map {mi} placement {pi}: flags 0x{:x} params {:?}", p.flags, p.params.iter().take(6).collect::<Vec<_>>());
            }
        }
    }
    eprintln!("main map = {:?}", level.main_map());
}
