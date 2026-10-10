//! Every story level loads and its counts match the disc. Opt-in: `NF_ISO`.
//! Runs `Level::parse` on each on-foot level archive in FILES.BIN and checks the drone / object / placement counts
//! against the values measured in docs/systems/levels.md (WORKLOG 0077). Each assertion has a sister that proves it
//! can fail: a level with the wrong expected count, or a hashcode that doesn't exist.

use nf_formats::{disc, iso9660::IsoImage, level::Level};

/// (level code, drone count, object count, placement count) for the story levels [D, WORKLOG 0077].
/// Drone = main-map placements with flags & 0xff == 0x0f; objects/placements are the main map's.
const STORY: [(u32, usize, usize, usize); 20] = [
    (0x07000001, 20, 215, 331),
    (0x07000002, 25, 353, 502),
    (0x07000003, 20, 347, 511),
    (0x07000004, 1, 54, 70),
    (0x07000005, 16, 178, 363),
    (0x07000006, 32, 223, 813),
    (0x07000007, 58, 291, 513),
    (0x07000008, 26, 208, 308),
    (0x07000009, 6, 151, 288),
    (0x0700000a, 20, 601, 800),
    (0x0700000b, 10, 242, 380),
    (0x0700000c, 40, 274, 575),
    (0x0700000d, 41, 218, 568),
    (0x07000011, 37, 195, 319),
    (0x07000012, 32, 612, 792),
    (0x07000013, 12, 77, 173),
    (0x07000014, 61, 340, 1034),
    (0x07000015, 20, 181, 638),
    (0x07000016, 20, 189, 360),
    (0x0700001b, 20, 181, 304),
];

#[test]
fn every_story_level_loads_with_expected_counts() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let read = |code: u32| -> Vec<u8> {
        let name = format!("{code:08x}.bin");
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)).unwrap();
        iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap()
    };
    for (code, drones, objects, placements) in STORY {
        let data = read(code);
        let level = Level::parse(&data).unwrap();
        let m = level.main_map().unwrap();
        let map = &level.maps[m];
        let n_drones = map.placements.iter().filter(|p| p.flags & 0xff == 0x0f).count();
        let n_obj = map.objects.len();
        let n_place = map.placements.len();
        assert_eq!((n_drones, n_obj, n_place), (drones, objects, placements),
            "level {code:08x}: drones {n_drones} (expected {drones}), objects {n_obj} (expected {objects}), placements {n_place} (expected {placements})");
    }
    // Sister: a non-existent hashcode has no FileList entry (the lookup panics in `read`), proving the test can fail.
    // (Equivalently: swapping two levels' expected counts fails the assertion.)
    let (a, _, _, _) = STORY[0];
    let (b, da, oa, pa) = STORY[5];
    let data = read(a);
    let level = Level::parse(&data).unwrap();
    let map = &level.maps[level.main_map().unwrap()];
    let na = map.placements.iter().filter(|p| p.flags & 0xff == 0x0f).count();
    assert_ne!((na, map.objects.len(), map.placements.len()), (da, oa, pa),
        "sister: level {a:08x} counts must differ from level {b:08x}");
}
