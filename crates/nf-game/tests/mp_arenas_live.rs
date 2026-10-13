//! research-multiplayer: every multiplayer arena on the disc loads with the current parser and the
//! arena content is pinned (docs/atlas/multiplayer.md §1). The MP level codes are 0x07000021–0x29 +
//! 0x0700004c in the loading-image switch `ResetMap_LevelCode2Img` 0x1d5258 [M]; the on-foot ISO
//! ships eight of them (07000023..07000029, 0700004b — work/levels_new.tsv [D]; 0x21/0x22/0x4c are
//! absent). Arena markers in the placement flags byte [M, research-multiplayer]: type 0x25 = MP
//! spawn point (`MP_RegisterSpawnPoint` 0x184f20), type 0x26 = MP object (`MP_RegisterMPObject`
//! 0x185990), type 0xf0 = pickup, 0x0f = drone (none expected on an arena). Opt-in via NF_ISO.

use nf_formats::{disc, iso9660::IsoImage, level::Level};

/// (level code, placements, objects, drones, pickups, MP spawns (0x25), MP objects (0x26)).
/// Counts measured by this test's probe run and pinned [D survey; WORKLOG research-multiplayer].
type Counts = (u32, usize, usize, usize, usize, usize, usize);
const ARENAS: [Counts; 8] = [
    (0x07000023, 181, 78, 0, 20, 32, 27),
    (0x07000024, 266, 155, 0, 28, 32, 25),
    (0x07000025, 343, 236, 0, 24, 32, 18),
    (0x07000026, 124, 41, 0, 18, 30, 22),
    (0x07000027, 188, 89, 0, 19, 31, 28),
    (0x07000028, 138, 59, 0, 17, 31, 26),
    (0x07000029, 264, 86, 0, 20, 32, 24),
    (0x0700004b, 225, 113, 0, 28, 32, 28),
];

#[test]
fn every_mp_arena_loads_with_pinned_counts() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let read = |code: u32| -> Vec<u8> {
        let name = format!("{code:08x}.bin");
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name))
            .unwrap_or_else(|| panic!("arena {code:08x}.bin missing from FILES.BIN"));
        iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap()
    };
    for (code, places, objs, drones, pickups, spawns, mpobjs) in ARENAS {
        let data = read(code);
        let level = Level::parse(&data).unwrap();
        let m = level.main_map().unwrap();
        let map = &level.maps[m];
        let n_place = map.placements.len();
        let n_obj = map.objects.len();
        let n_drones = map.placements.iter().filter(|p| p.flags & 0xff == 0x0f).count();
        let n_pick = map.placements.iter().filter(|p| p.flags & 0xff == 0xf0).count();
        let n_spawn = map.placements.iter().filter(|p| p.flags & 0xff == 0x25).count();
        let n_mpobj = map.placements.iter().filter(|p| p.flags & 0xff == 0x26).count();
        println!("{code:08x}: places {n_place} objs {n_obj} drones {n_drones} pickups {n_pick} spawns {n_spawn} mpobjs {n_mpobj}");
        assert_eq!((n_place, n_obj, n_drones, n_pick, n_spawn, n_mpobj),
            (places, objs, drones, pickups, spawns, mpobjs),
            "arena {code:08x}: measured (places {n_place}, objs {n_obj}, drones {n_drones}, pickups {n_pick}, spawns {n_spawn}, mpobjs {n_mpobj})");
        // Every arena has spawn points and pickups; none has campaign drones [M 0x184f20/0x185990;
        // D survey; sister of docs/levels/0700004b.md's no-campaign-path survey].
        assert!(n_spawn > 0, "arena {code:08x}: no type-0x25 MP spawn placements");
        assert!(n_pick > 0, "arena {code:08x}: no pickups");
        assert_eq!(n_drones, 0, "arena {code:08x}: a campaign drone placement on a multiplayer arena");
    }
    // Sister: two arenas' content differs (swapping two rows of ARENAS fails), proving the pinned
    // table can fail.
    let (a, pa, ..) = ARENAS[0];
    let (b, pb, ..) = ARENAS[ARENAS.len() - 1];
    assert_ne!(pa, pb, "sister: arena {a:08x} and {b:08x} pinned placement counts must differ");
}
