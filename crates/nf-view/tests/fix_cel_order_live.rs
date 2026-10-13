//! fix-cel-order: the static draw order must be the game's cel-chain order.
//!
//! `View_AddCels` (0x1e6ba0, disassembled) hands the −90000 + 10·k list-0 keys to the **room cels**
//! over the +0x20 chain `Vision_Portal_Recurse` (0x1e86e8) threads per frame from the depth-sorted
//! `VisCelList` — `QuickSort` (0x1d30a8) comparator[3] @0x1d3430 orders it by the ascending u16
//! portal-recursion depth. The non-room 0x8000 placements (the 0x8019/0x8018 marker classes of
//! `parseentity_fixup_entity` 0x1d00c0: every baked shadow decal) hang on a room's +0x18 inner
//! chain and draw at ±dist² [L ee_start_05.bin: 19 rooms + 113 inner cels; 0c/07 disc probe: every
//! decal is a 0x8019 placement]. models-world's "chain ≈ placement-parse order" put 0700000c's
//! pylonshadow decal (chain 68) before the ground room under it (chain 88) — the decal darkened
//! the fog clear colour and its depth write rejected the ground: the user's pale lattice.

use nf_formats::disc;
use nf_formats::iso9660::IsoImage;
use nf_formats::level::Level;

fn iso_path() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The game's own `QuickSort` (0x1d30a8, comparator[3] 0x1d3430): ascending u16 depth at element+4,
/// unstable — pivot = element `count>>1` swapped to the front, Hoare partition, smaller side to the
/// stack. Hand-traced golden (the tie order feeds `View_AddCels`' −90000 + 10·k chain ordinals,
/// so a stable stand-in would diverge): input depths [1,0,1,0] sorts to [0,0,1,1] with the tie
/// order b, d | a, c — original indices [1,3,0,2].
#[test]
fn the_quick_sort_orders_viscel_by_depth_like_the_game() {
    let sort = |items: &[(usize, u16)]| {
        let mut v = items.to_vec();
        nf_view::scene::quicksort_viscel(&mut v);
        v
    };
    let got = sort(&[(10, 1), (11, 0), (12, 1), (13, 0)]);
    assert_eq!(got, vec![(11, 0), (13, 0), (10, 1), (12, 1)], "the hand-traced game quicksort order");
    // Sisters: always a permutation, always non-decreasing, and deterministic.
    let cases: [[(usize, u16); 7]; 3] = [
        [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0)],
        [(0, 5), (1, 4), (2, 3), (3, 2), (4, 1), (5, 0), (6, 5)],
        [(0, 1), (1, 2), (2, 1), (3, 2), (4, 1), (5, 2), (6, 1)],
    ];
    for case in cases {
        let mut a = case.to_vec();
        nf_view::scene::quicksort_viscel(&mut a);
        let mut b = case.to_vec();
        nf_view::scene::quicksort_viscel(&mut b);
        assert_eq!(a, b, "deterministic");
        let mut keys: Vec<u16> = a.iter().map(|x| x.1).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "non-decreasing by depth");
        keys.sort_unstable();
        let mut ids: Vec<usize> = a.iter().map(|x| x.0).collect();
        ids.sort_unstable();
        let mut want: Vec<usize> = case.iter().map(|x| x.0).collect();
        want.sort_unstable();
        assert_eq!(ids, want, "a permutation of the input");
    }
}

/// The 0700000c pin: the baked `pylonshadow` decal (material 3380, texture 002d:003b) must draw
/// AFTER the ground room under it. models-world's parse-order key drew it first (chain 68 < the
/// ground's 88): it darkened the fog clear colour and its depth write rejected the ground — the
/// user's pale lattice (the real game shows a dark lattice). Under the cel-chain law the decal is
/// a non-room 0x8019 placement = a +0x18 inner node at ±dist², the ground room a −90000 + 10·k
/// chain entry — the decal always draws after every room.
#[test]
fn the_pylonshadow_decal_draws_after_the_ground_room() {
    let Some(path) = iso_path() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&path, "0700000c.bin", false).expect("load");
    // The decal's draws: the pylonshadow placement's object centre through its placement matrix —
    // the same world centre the build keys the draw groups with.
    let iso = IsoImage::open(&path).expect("open iso");
    let (entries, bin) = disc::files_bin_entries(&iso).expect("FILES.BIN");
    let base = bin.lba as u64 * 2048;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("0700000c.bin")).unwrap();
    let data = iso.read_bytes(base + fe.offset as u64, fe.size as u64).unwrap();
    let lvl = Level::parse(&data).unwrap();
    let main = lvl.main_map().unwrap();
    let world: Vec<(usize, &nf_view::scene::Material)> = scene.materials.iter().enumerate()
        .filter(|(_, m)| m.layer == nf_view::scene::Layer::World && m.draw.pass == 1).collect();
    let world_all: Vec<(usize, &nf_view::scene::Material)> = scene.materials.iter().enumerate()
        .filter(|(_, m)| m.layer == nf_view::scene::Layer::World).collect();
    assert!(world.iter().any(|(_, m)| m.chain != u32::MAX), "0c must have room-chain statics");

    let mut decal: Vec<(usize, &nf_view::scene::Material)> = Vec::new();
    let mut decal_seen = false;
    for p in &lvl.maps[main].placements {
        let target = if p.by_hash() { lvl.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < lvl.maps[main].objects.len() { Some((main, p.index as usize)) } else { None };
        let Some((om, oi)) = target else { continue };
        if lvl.maps[om].objects[oi].name.trim() != "pylonshadow" { continue; }
        decal_seen = true;
        let m = nf_formats::placement::object_matrix(p.rotation, p.translation);
        let centre = nf_formats::placement::transform(&m, lvl.maps[om].objects[oi].centre);
        let bits = centre.map(f32::to_bits);
        for (k, mat) in &world_all {
            if mat.centre.map(f32::to_bits) == bits { decal.push((*k, *mat)); }
        }
    }
    assert!(decal_seen, "0700000c must place the pylonshadow decal");
    assert!(!decal.is_empty(), "the pylonshadow decal must have pass-1 World draws");
    // The law: a non-room 0x8000 placement is a +0x18 inner node — distance-keyed (chain = MAX),
    // list by the glist flags bit 0x1 (`View_AddCels` 0x1e6ba0's inner push). models-world gave it
    // chain 68 and forced pass 1; both assertions fail before the fix.
    for (_, m) in &decal {
        assert_eq!(m.chain, u32::MAX, "the pylonshadow decal is an inner node, not a chain entry");
        assert!(matches!(m.draw.pass, 1 | 3), "an inner node draws in list 0 or list 1, not {:?}", m.draw.pass);
    }
    // The ground room under it: room-chain statics (chain != MAX) overlapping the decal's bounds.
    // Every one of them must sort BEFORE the decal (a negative synthetic key vs a dist² key).
    let mut ground = 0usize;
    for (_, d) in &decal {
        for (_, m) in &world {
            if m.chain == u32::MAX { continue; }
            let overlaps = (0..3).all(|ax| (m.centre[ax] - d.centre[ax]).abs() < m.radius + d.radius);
            if !overlaps { continue; }
            ground += 1;
            // A list-1 (pass 3) decal draws after the whole of list 0 — the ground law is structural.
            // A list-0 decal sorts by key: its dist² key is non-negative, the room's synthetic key negative.
            if d.draw.pass == 1 {
                let room_key = -90_000.0 + 10.0 * m.chain as f32;
                let decal_key = d.centre.iter().map(|c| c * c).sum::<f32>();
                assert!(room_key < decal_key, "the ground room at {:?} (chain {}) must draw before the decal at {:?}",
                    m.centre, m.chain, d.centre);
            }
        }
    }
    assert!(ground > 0, "the pylonshadow decal must overlap room-chain ground statics");
    // The −90000 block: every room-chain key is negative, every distance key non-negative, so the
    // rooms all draw before the inner nodes and dynamics (the renderer's pass-1 sort).
    let n_rooms = world.iter().filter(|(_, m)| m.chain != u32::MAX).count();
    let mut order: Vec<(f32, usize)> = world.iter().map(|(k, m)| {
        let d2 = m.centre.iter().map(|c| c * c).sum::<f32>();
        (if m.chain != u32::MAX { -90_000.0 + 10.0 * m.chain as f32 } else { d2 }, *k)
    }).collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    assert!(order[..n_rooms].iter().all(|(key, _)| *key < 0.0) && order[n_rooms..].iter().all(|(key, _)| *key >= 0.0),
        "the room chain must sort before every distance-keyed entry");
    // The rooms' fallback ordinals ascend through the draw order (the parse-rank fallback).
    let chains: Vec<u32> = order[..n_rooms].iter()
        .map(|(_, k)| world.iter().find(|(i, _)| i == k).unwrap().1.chain).collect();
    let mut sorted = chains.clone();
    sorted.sort_unstable();
    assert_eq!(chains, sorted, "the rooms must draw in their parse-rank order");
}
