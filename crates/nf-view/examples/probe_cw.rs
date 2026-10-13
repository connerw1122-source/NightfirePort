fn main() {
    let iso = std::env::var("NF_ISO").unwrap();
    let disc = nf_formats::disc::Disc::open(&iso).unwrap();
    for e in disc.files.iter().filter(|e| e.name.starts_with("0700")) {
        let Ok(level) = (|| { let data = disc.read(&e.name).ok()?; nf_formats::level::Level::parse(&data).ok() })() else { continue };
        let n: Vec<_> = level.maps.iter().flat_map(|m| m.placements.iter().filter(|p| p.flags & 0xff == 0x3f)).collect();
        if !n.is_empty() {
            for p in n {
                println!("{} placement {} pos {:?} yaw {:.2} params {:?}", e.name, p.index, p.translation, p.rotation[1], p.params);
            }
        }
    }
}
