//! 0xE2 placements (`SS_Create` 0x1a9c58, "SimpleScript"): the game creates them as drawn, room-linked objects that
//! sit at their placement until a channel activates them [M]. The port skipped them, so 07000006's barrel trap
//! (barrels on sawhorses by the courtyard canopy) was missing (user's PCSX2 screenshot). Opt-in: NF_ISO.

use nf_formats::{disc, iso9660::IsoImage, level::Level};

fn level(iso: &str, name: &str) -> Level {
    let img = IsoImage::open(iso).expect("iso");
    let (entries, bin) = disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).expect("archive");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    Level::parse(&data).expect("parse")
}

#[test]
fn simplescript_props_are_in_the_scene() {
    let Ok(iso) = std::env::var("NF_ISO") else { eprintln!("NF_ISO not set — skipping"); return };
    let lv = level(&iso, "07000006.bin");
    let name = |i: &nf_formats::level::Instance| lv.maps[i.map].objects[i.object].name.trim().to_string();
    // map-tower2: the ANIMATED 0xE2s (key count non-zero — `SS_Create` 0x1a9c58) moved from the
    // static scene pass to the posed ss pass (`SS_Update` 0x1a9948's KeyFrame_Interp at key 0 at
    // rest); the key-less ones stay static (the game creates nothing for them [S]).
    let statics: Vec<String> = lv.scene_instances(true).0.iter().filter(|i| i.placement.flags & 0xff == 0xe2).map(name).collect();
    let pb: Vec<&[u8]> = lv.path_blocks();
    let ss = nf_game::ss::SsList::create(&lv.maps[lv.main_map().unwrap()].placements, &pb, |_p| 0);
    let ss_names: Vec<String> = ss.list.iter().map(|s| {
        let p = &lv.maps[lv.main_map().unwrap()].placements[s.placement];
        let obj = if p.by_hash() { lv.by_hash.get(&p.hashcode).copied() } else { Some((lv.main_map().unwrap(), p.index as usize)) };
        obj.and_then(|(m, o)| lv.maps[m].objects.get(o)).map(|o| o.name.trim().to_string()).unwrap_or_default()
    }).collect();
    eprintln!("[ss] 07000006 animated 0xE2 (posed pass): {ss_names:?}");
    eprintln!("[ss] 07000006 key-less 0xE2 (static pass): {statics:?}");
    assert!(ss_names.iter().filter(|n| n.starts_with("Script_Barrelroll")).count() >= 3, "the three trap barrels are drawn (posed)");
    assert!(ss_names.iter().any(|n| n.starts_with("Script_Barrelleg")), "and their sawhorse legs");
    // Sister: none of the animated ones remains in the static pass (no double draw).
    assert!(!statics.iter().any(|n| n.starts_with("Script_Barrelroll")), "the animated barrels left the static pass");
}

/// Survey (prints only): every level's 0xE2 objects, to spot any that should stay hidden at rest.
#[test]
fn survey_simplescript_props() {
    let Ok(iso) = std::env::var("NF_ISO") else { return };
    if std::env::var("NF_SURVEY").is_err() { return }
    let img = IsoImage::open(&iso).expect("iso");
    let (entries, _) = disc::files_bin_entries(&img).expect("FILES.BIN");
    for e in entries.iter().filter(|e| e.name.to_ascii_lowercase().starts_with("070000") && e.name.to_ascii_lowercase().ends_with(".bin")) {
        let Ok(lv) = std::panic::catch_unwind(|| level(&iso, &e.name)) else { continue };
        let mut names: Vec<String> = lv.instances(false).0.iter().filter(|i| i.placement.flags & 0xff == 0xe2)
            .map(|i| lv.maps[i.map].objects[i.object].name.trim().to_string()).collect();
        names.sort(); names.dedup();
        if !names.is_empty() { eprintln!("{} {}: {names:?}", e.name, names.len()); }
    }
}
