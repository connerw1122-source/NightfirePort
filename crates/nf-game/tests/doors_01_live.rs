//! Verify doors on 07000001 get the correct anim keys after the path_blocks fix. Opt-in: NF_ISO.
//! Also re-checks 07000005's doors still match the live snapshot (regression guard).
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};
use nf_game::door::Doors;
use nf_game::triggers::Channels;

fn load(code: u32) -> Option<Level> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let name = format!("{code:08x}.bin");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Some(Level::parse(&data).unwrap())
}

#[test]
fn doors_on_07000001_now_match_their_placements() {
    let Some(level) = load(0x07000001) else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let blocks = level.path_blocks();
    // The fix: path_blocks returns only the main map's blocks (24, not 25 with the sub-map's).
    assert_eq!(blocks.len(), 24, "main map has 24 0x19 blocks (the 25th was a kind=0 sub-map block)");
    let doors = Doors::create(ps, &blocks, |p| {
        let o = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(om, oi)| &level.maps[om].objects[oi]) }
                else { level.maps[m].objects.get(p.index as usize) };
        o.map(|o| (o.hashcode, o.centre, o.radius))
    });
    assert_eq!(doors.list.len(), 5);
    // Each door's first key position must match its placement's translation (the anim is in world space).
    for (k, d) in doors.list.iter().enumerate() {
        let p = &ps[d.placement];
        let keys = d.keys.as_ref().expect("each 07000001 door has a 2-key anim");
        let first = keys[0].pos;
        let t = p.translation;
        assert!((first[0] - t[0]).abs() < 1e-2 && (first[1] - t[1]).abs() < 1e-2 && (first[2] - t[2]).abs() < 1e-2,
            "door {k} (placement {}): first key {:?} must match translation {:?}",
            d.placement, first, t);
        // The closed matrix's translation is the first key's position (the door is rendered at its placement).
        assert!((d.matrix[3][0] - t[0]).abs() < 1e-2 && (d.matrix[3][1] - t[1]).abs() < 1e-2 && (d.matrix[3][2] - t[2]).abs() < 1e-2,
            "door {k} (placement {}): matrix translation {:?} must match {:?}",
            d.placement, [d.matrix[3][0], d.matrix[3][1], d.matrix[3][2]], t);
    }
    // The use key (E) now hits each door at its correct position, and the doors actually open.
    let mut doors = doors;
    let mut ch = Channels::default();
    let sphs: Vec<([f32; 3], f32)> = doors.list.iter().map(|d| d.sph).collect();
    let mut total_hits = 0;
    for (i, sph) in sphs.iter().enumerate() {
        let head = [sph.0[0], sph.0[1], sph.0[2] - 1.0];
        let look = [0.0, 0.0, 1.0];
        let (_, hits) = doors.player_use(head, look, &mut ch, 0);
        assert!(hits >= 1, "door {i} should be hit by the use sphere at its correct position");
        total_hits += hits;
    }
    println!("total use-key hits across 5 doors: {}", total_hits);
    // Run a few ticks: every door should transition CLOSED → OPENING and advance pos01.
    for f in 1..=5 {
        for i in 0..doors.list.len() {
            let sph = doors.list[i].sph;
            doors.update(i, &mut ch, &[(sph.0, 0.5f32, false)], f);
        }
        doors.calc();
    }
    for (i, d) in doors.list.iter().enumerate() {
        assert_eq!(d.state, 8, "door {i} should be OPENING after 5 ticks");
        assert!(d.pos01 > 0.0, "door {i} should have advanced past pos01=0");
    }
    println!("all 5 doors on 07000001 now open correctly via the E key");
}

#[test]
fn doors_on_07000005_still_match_the_live_snapshot() {
    let Some(level) = load(0x07000005) else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let blocks = level.path_blocks();
    // Regression guard: 07000005 has no sub-map 0x19 blocks, so path_blocks is unchanged (7 blocks).
    assert_eq!(blocks.len(), 7, "07000005 main map has 7 0x19 blocks (no sub-map blocks to filter)");
    let doors = Doors::create(ps, &blocks, |p| {
        let o = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(om, oi)| &level.maps[om].objects[oi]) }
                else { level.maps[m].objects.get(p.index as usize) };
        o.map(|o| (o.hashcode, o.centre, o.radius))
    });
    assert_eq!(doors.list.len(), 5, "5 doors on 07000005");
    // Door_UNLocked (placement 153) is the swing door with 3 keys at the world position [57.75, 26.00, 89.31].
    let swing = doors.list.iter().find(|d| d.placement == 153).unwrap();
    assert!(swing.keys.is_some(), "Door_UNLocked has a 3-key anim");
    let keys = swing.keys.as_ref().unwrap();
    assert_eq!(keys.len(), 3, "Door_UNLocked has 3 keys");
    for k in 0..3 {
        assert!((keys[k].pos[0] - 57.752_61).abs() < 1e-3, "key {k} x");
        assert!((keys[k].pos[1] - 26.000_368).abs() < 1e-3, "key {k} y");
        assert!((keys[k].pos[2] - 89.311_48).abs() < 1e-3, "key {k} z");
    }
    println!("07000005 Door_UNLocked still has the correct 3-key anim (regression check passes)");
}
