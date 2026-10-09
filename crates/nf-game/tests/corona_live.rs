//! Level coronas against the live layout in `work/ee_start_05.bin` (WORKLOG 0073): the 13 type-0xFC
//! placements of 07000005, their objects' names and flags (the glist +0x44 word the game copies into
//! `obj+0xF0` — measured on the live objects), and the alpha-list bit (0x1) that puts them in
//! `View_AddObjects`' draw list 1. Opt-in: set `NF_ISO` to the disc image.

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};

/// The 13 live corona objects [L `coronas_snapshot.py` walk of `ee_start_05.bin`]: placement index,
/// object name, and the flags word the live `obj+0xF0` holds (== the object's own flags).
const GOLDEN: &[(usize, &str, u32)] = &[
    (14, "Corona_Glow2", 0x22d),
    (15, "Corona_Glow2", 0x22d),
    (16, "Corona_Glow2", 0x22d),
    (55, "Corona_Glow2", 0x22d),
    (231, "Window_corona", 0x229),
    (232, "Window_corona", 0x229),
    (233, "Window_corona", 0x229),
    (234, "Window_corona", 0x229),
    (235, "Window_corona", 0x229),
    (236, "Window_corona", 0x229),
    (237, "Window_corona", 0x229),
    (245, "Corona_test1", 0x22d),
    (260, "Corona_Glow2", 0x22d),
];

fn level_05() -> Option<Level> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Some(Level::parse(&data).unwrap())
}

#[test]
fn the_live_coronas_reproduce() {
    let Some(level) = level_05() else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let got = nf_game::corona::placements(ps);
    let expect: Vec<usize> = GOLDEN.iter().map(|&(i, _, _)| i).collect();
    assert_eq!(got, expect, "the 13 type-0xFC placements, in record order");
    // Names and the live flags word [L]: the game's `obj+0xF0` is the glist's +0x44, which for these
    // objects is the object's own flags — 0x22D (Glow2, test1) / 0x229 (Window_corona).
    for &(i, name, flags) in GOLDEN {
        let p = &ps[i];
        assert_eq!(p.flags & 0xff, 0xfc, "placement {i} is type 0xFC");
        let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(om, oi)| &level.maps[om].objects[oi]) }
            else { level.maps[m].objects.get(p.index as usize) };
        let obj = obj.unwrap_or_else(|| panic!("placement {i} resolves"));
        assert_eq!(obj.name.trim(), name, "placement {i}");
        assert_eq!(obj.flags, flags, "placement {i} flags (live obj+0xF0)");
        assert_ne!(obj.flags & 1, 0, "placement {i}: bit 0x1 — View_AddObjects draw list 1 (alpha list)");
    }
}

#[test]
fn the_filter_discriminates() {
    let Some(level) = level_05() else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let coronas = nf_game::corona::placements(ps);
    // Sister assertion: the neighbouring type 0xDB (doors) is a real, resolvable placement set that the
    // corona filter must not match (the name/flags goldens above would fail on Door_* objects).
    let doors: Vec<usize> = ps.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xdb && p.flags & 0xa000 == 0).map(|(i, _)| i).collect();
    assert!(!doors.is_empty(), "07000005 has door placements to confuse the filter with");
    assert!(coronas.iter().all(|c| !doors.contains(c)), "coronas and doors are disjoint");
    // And the +1 type: 0xFD (CamSubject) has no placement here — a filter off by one returns nothing.
    let fd: Vec<usize> = ps.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xfd).map(|(i, _)| i).collect();
    assert!(fd.is_empty());
    assert_eq!(coronas.len(), 13);
}
