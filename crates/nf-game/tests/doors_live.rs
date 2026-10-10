//! The doors against the live layout in `work/ee_start_05.bin` (WORKLOG 0074): the 5 type-0xDB placements of
//! 07000005, their create-time `DOORINFO` state (measured on the live objects: kind, group, channels, sounds,
//! flags, speed, position, swing sign, frames) and the swing door's anim — block 3 of the level's 0x19 key
//! lists, whose three keys' quaternion (0, ±0.3826834, 0, −0.9238795 / 0, 0.9238795, 0, −0.3826834) pose the
//! door through `Spline_Interp`'s own matrix formula. Opt-in: set `NF_ISO` to the disc image.

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};

/// One door's live create state [L `doors_snapshot.py` walk of `ee_start_05.bin`]: placement index, object
/// name, kind (`obj+0xF6`), lock channel (`+0x68`), open sound (`+0x60`), close sound (`+0x62`), hit sound
/// (`+0x64`), flags (`+0x70`), speed (`+0x54`), position `+0x50`, swing sign `+0x5C`, frames `+0x6E`.
const GOLDEN: &[(usize, &str, i16, u16, u16, u16, u16, u16, f32, f32, f32, u16)] = &[
    // Door_Locked_3: a wide locked slider without an anim; the sphere is the gfx's own (r 3.948).
    (152, "Door_Locked_3", 2, 200, 0xffff, 0xffff, 320, 0x2, 1.0 / 30.0, 0.0, 0.0, 0xffff),
    // Door_Locked_4 ×3: the same door at the castle's interior.
    (153, "Door_UNLocked", 0, 0, 285, 284, 320, 0x3, 1.0 / 14.0, 0.5, -1.0, 2),
    (154, "Door_Locked_4", 2, 200, 0xffff, 0xffff, 320, 0x2, 1.0 / 30.0, 0.0, 0.0, 0xffff),
    (155, "Door_Locked_4", 2, 200, 0xffff, 0xffff, 320, 0x2, 1.0 / 30.0, 0.0, 0.0, 0xffff),
    (156, "Door_Locked_4", 2, 200, 0xffff, 0xffff, 320, 0x2, 1.0 / 30.0, 0.0, 0.0, 0xffff),
];

/// The swing door's three keys [L: the words at `m_paths[3]` = 0x009d1404]: position and quaternion.
const KEYS: &[([f32; 3], [f32; 4])] = &[
    ([57.752_61, 26.000_368, 89.311_48], [0.0, -0.382_683_43, 0.0, -0.923_879_56]),
    ([57.752_61, 26.000_368, 89.311_48], [0.0, 0.382_683_43, 0.0, -0.923_879_56]),
    ([57.752_61, 26.000_368, 89.311_48], [0.0, 0.923_879_56, 0.0, -0.382_683_43]),
];

fn level_05() -> Option<Level> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Some(Level::parse(&data).unwrap())
}

fn object<'a>(level: &'a Level, m: usize, p: &'a nf_formats::placement::Placement) -> &'a nf_formats::level::MapObject {
    if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(om, oi)| &level.maps[om].objects[oi]) }
        else { level.maps[m].objects.get(p.index as usize) }.unwrap()
}

#[test]
fn the_live_doors_reproduce() {
    let Some(level) = level_05() else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let blocks = level.path_blocks();
    assert_eq!(blocks.len(), 7, "the level's m_paths table (7 blocks of 0x19)");
    let doors = nf_game::door::Doors::create(ps, &blocks, |p| {
        let o = object(&level, m, p);
        Some((o.hashcode, o.centre, o.radius))
    });
    let expect: Vec<usize> = GOLDEN.iter().map(|&(i, ..)| i).collect();
    assert_eq!(nf_game::door::door_placements(ps), expect, "the 5 type-0xDB placements, in record order");
    assert_eq!(doors.list.len(), 5);
    for (k, &(i, name, kind, ch_lock, snd_open, snd_close, snd_hit, flags, speed, pos01, swing, frames)) in GOLDEN.iter().enumerate() {
        let d = &doors.list[k];
        let p = &ps[i];
        assert_eq!(d.placement, i);
        assert_eq!(object(&level, m, p).name.trim(), name, "placement {i}");
        assert_eq!(d.kind, kind, "placement {i} kind (obj+0xF6)");
        assert_eq!(d.ch_open, 0, "placement {i} ch_open (+0x66): none on this level");
        assert_eq!(d.ch_lock, ch_lock, "placement {i} ch_lock (+0x68)");
        assert_eq!(d.snd_open, snd_open, "placement {i} entering-8 sound (+0x60)");
        assert_eq!(d.snd_close, snd_close, "placement {i} entering-0x10 sound (+0x62)");
        assert_eq!(d.snd_hit, snd_hit, "placement {i} hit sound (+0x64)");
        assert_eq!(d.flags, flags, "placement {i} DOORINFO flags (+0x70)");
        assert_eq!(d.state, 2, "placement {i} starts closed (obj+0xF4)");
        assert!((d.speed - speed).abs() < 1e-6, "placement {i} speed {:+e} (want {speed:+e})", d.speed);
        assert!((d.pos01 - pos01).abs() < 1e-6, "placement {i} pos01 (live {pos01})");
        assert!((d.swing - swing).abs() < 1e-6, "placement {i} swing sign (live {swing})");
        assert_eq!(d.frames, frames, "placement {i} frames (+0x6E)");
        // The anim: block 3 for the swing door, none for the sliders (their DOORINFO+0xC is NULL).
        if frames == 0xffff {
            assert!(d.keys.is_none(), "placement {i} has no anim [L]");
            assert_eq!(d.glist, object(&level, m, p).hashcode, "the collision shape is the object's own");
        } else {
            let keys = d.keys.as_ref().unwrap();
            assert_eq!(keys.len(), 3, "placement {i} has 3 keys");
            for (k, (pos, quat)) in KEYS.iter().enumerate() {
                for c in 0..3 { assert!((keys[k].pos[c] - pos[c]).abs() < 1e-4, "key {k} pos"); }
                for c in 0..4 { assert!((keys[k].quat[c] - quat[c]).abs() < 1e-6, "key {k} quat"); }
            }
            // The closed pose through `Spline_Interp`'s own quat→matrix formula (the live `obj+0x90` words
            // 3f3504f5 / 00000000 / bf3504f0 / 426702ac: m00 = 0.70711, m10 = 0, m20 = −0.70711, tx = 57.7526).
            let m90 = &d.matrix;
            assert!((m90[0][0] - 0.707_106_77).abs() < 1e-5, "closed m00 (live 0x3f3504f5)");
            assert!((m90[2][0] - -0.707_106_77).abs() < 1e-5, "closed m20 (live 0xbf3504f0)");
            assert!((m90[3][0] - 57.752_61).abs() < 1e-3, "closed tx (live 0x426702ac)");
            assert!((m90[0][3] - 0.0).abs() < 1e-6 && (m90[1][3] - 0.0).abs() < 1e-6 && (m90[3][3] - 1.0).abs() < 1e-6);
        }
    }
}

#[test]
fn the_key_lists_and_the_filter_discriminate() {
    let Some(level) = level_05() else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let blocks = level.path_blocks();
    // Sister assertion: a wrong block index gives different keys — block 2 is a 4-node path (56 bytes, 2 keys
    // of 28), not a door anim; and a NULL list parses to no keys at all.
    let wrong = nf_game::door::parse_keys(blocks[2]);
    assert_eq!(wrong.len(), 2, "block 2 is a path list, not the door's 3 keys");
    assert!(nf_game::door::parse_keys(&[]).is_empty());
    // The m_paths index lives in extra[8]'s low half and the count in extra[7]'s low half: the swing door's
    // record holds (3, rate 0, index 3); a door that misread them would pick the wrong block or none.
    let p = &ps[153];
    assert_eq!(p.extra[7] & 0xffff, 3, "record +0x40: the key count");
    assert_eq!(p.extra[8] & 0xffff, 3, "record +0x44: the m_paths index (block 3)");
    // And the filter against the neighbours: 0xDA (a Break placement type) must not match, 0xDC is triggers.
    let da: Vec<usize> = ps.iter().enumerate().filter(|(_, p)| p.flags & 0xff == 0xda).map(|(i, _)| i).collect();
    assert!(da.is_empty(), "07000005 has no 0xDA placements (an off-by-one filter returns them)");
    assert_eq!(nf_game::door::door_placements(ps).len(), 5);
}
