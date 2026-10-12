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
    let scene: Vec<String> = lv.scene_instances(true).0.iter().filter(|i| i.placement.flags & 0xff == 0xe2).map(name).collect();
    eprintln!("[ss] 07000006 0xE2 in the scene: {scene:?}");
    assert!(scene.iter().filter(|n| n.starts_with("Script_Barrelroll")).count() >= 3, "the three trap barrels are drawn");
    assert!(scene.iter().any(|n| n.starts_with("Script_Barrelleg")), "and their sawhorse legs");
    // Sister: the cel-only pass alone (the old scene) has none of them.
    assert!(!lv.instances(true).0.iter().any(|i| i.placement.flags & 0xff == 0xe2), "0xE2 never creates a cel");
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
