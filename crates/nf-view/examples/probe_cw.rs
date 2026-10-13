fn main() {
    let iso = std::env::var("NF_ISO").unwrap();
    let img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    for e in entries.iter().filter(|e| e.name.starts_with("0700")) {
        let data = img.read_bytes(bin.lba as u64 * 2048 + e.offset as u64, e.size as u64).ok();
        let Some(data) = data else { continue };
        let Some(level) = nf_formats::level::Level::parse(&data).ok() else { continue };
        let n: Vec<_> = level.maps.iter().flat_map(|m| m.placements.iter().filter(|p| p.flags & 0xff == 0x3f)).collect();
        if !n.is_empty() {
            for p in n {
                println!("{} placement {} pos {:?} yaw {:.2} params {:?}", e.name, p.index, p.translation, p.rotation[1], p.params);
            }
        }
    }
}
