#[path = "common/walker.rs"]
mod walker;

#[test]
fn scan_07000016_ch24_writers() {
    let iso = std::env::var("NF_ISO").unwrap();
    let scene = nf_view::load(&iso, "07000016.bin", false).unwrap();
    let sim = scene.sim.as_ref().unwrap();
    for (i, t) in sim.drones.triggers.iter().enumerate() {
        if t.out == 0x24 || t.out == 0x34 || t.out == 0x27 || t.out == 0x28 {
            eprintln!("trig {i}: kind {:#x} out {:#x} mask {:#x} lock {:#x} placement {}", t.kind, t.out, t.mask, t.lock, t.placement);
        }
    }
    // Placement translations are in the parsed level.
    let iso2 = std::env::var("NF_ISO").unwrap();
    let img = nf_formats::iso9660::IsoImage::open(&iso2).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000016.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let lv = nf_formats::level::Level::parse(&data).unwrap();
    let mm = lv.main_map().unwrap();
    for pi in [250usize, 337, 338] {
        let p = &lv.maps[mm].placements[pi];
        eprintln!("placement {pi}: flags {:08x} trans {:?}", p.flags, p.translation);
    }
    drop(lv);
    // The walkway floor grid around the panel: from the corridor's west wall (x 7?) to the panel.
    let sim = scene.sim.as_ref().unwrap();
    for z in [-80.0f32, -84.0, -86.0, -100.0, -104.0, -106.0, -108.0, -112.0] {
        let mut row = String::new();
        for x in [2.0f32, 0.0, -2.0, -4.0, -6.0, -8.0, -10.0, -12.0, -14.0, -16.0] {
            let hit = sim.world.segment_hit([x, 9.0, z], [x, -6.0, z]);
            row.push_str(&format!(" x{x:.0}{}", hit.map(|h| format!("{:.2}", 9.0 - h.0 * 15.0)).unwrap_or("--".into())));
        }
        eprintln!("walk z{z:.1}:{row}");
    }

}
