//! start-slot-0x2d: the second start-position set (type 0x2d) on the disc, through the port's real
//! registration (`nf_view::level_starts` → `nf_game::player::starts_from_placements`) and the real
//! load path (`nf_view::load`).
//!
//! Evidence: [M] `Player_AddNewStartPos` 0x195c00 stores the u16 kind at entry+0x22
//! (`parsemap_create_dynamic_objects` 0x1d04f0 case 0x24 → disasm 0x1d0a00 `addiu a2,$zero,1`;
//! case 0x2d → disasm 0x1d0e98 `move a2,$zero`); `Player_Start` 0x190fd0 picks kind-1 entries only
//! (disasm 0x191028/0x1910b0: both passes `bne v0, 1` on entry+0x22) — so the kind-0 (0x2d) records
//! are registered but never picked. [D] every per-level count and position below is read from the
//! disc's placements through the port's own parsers. Opt-in via NF_ISO.

use nf_formats::{disc, iso9660::IsoImage, level::Level};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(img: &IsoImage<std::fs::File>, name: &str) -> Vec<u8> {
    let (files, bin) = disc::files_bin_entries(img).unwrap();
    let f = files.iter().find(|f| f.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("{name}: not in FILES.BIN"));
    img.read_bytes(bin.lba as u64 * nf_formats::iso9660::SECTOR_SIZE as u64 + f.offset as u64, f.size as u64).unwrap()
}

/// The 07* level archives on the disc, sorted (the census universe).
fn archives(img: &IsoImage<std::fs::File>) -> Vec<String> {
    let (files, _) = disc::files_bin_entries(img).unwrap();
    let mut names: Vec<String> = files.iter()
        .map(|e| e.name.to_ascii_lowercase())
        .filter(|n| n.starts_with("0700") && n.ends_with(".bin"))
        .collect();
    names.sort();
    names.dedup();
    names
}

fn near(a: f32, b: f32, tol: f32) -> bool { (a - b).abs() < tol }

/// ── The disc sweep: every archive's main-map registration carries BOTH sets with the game's
/// kinds, in parse order. Totals [D]: 38 kind-1 (0x24) and 18 kind-0 (0x2d) on the main maps —
/// the census's 51 counts the disc's type-0x2d records including each archive's kind-0 shell-map
/// (0,0,0) dummy, which the main-map parse never reads [M `ResetMap_Load` 0x1d6360:150 parses the
/// loaded map's dynamic block; the port's `main_map` convention is pinned by the spawn playtests]. ──
#[test]
fn disc_sweep_registers_both_sets_with_the_game_kinds() {
    let Some(path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&path).unwrap();
    let mut total_k1 = 0usize;
    let mut total_k0 = 0usize;
    for name in archives(&img) {
        let Ok(level) = Level::parse(&read_archive(&img, &name)) else { continue };
        let starts = nf_view::level_starts(&level);
        let k1 = starts.iter().filter(|s| s.kind == 1).count();
        let k0 = starts.iter().filter(|s| s.kind == 0).count();
        total_k1 += k1;
        total_k0 += k0;
        // Observation [M 0x1d1c90]: the game skips a placement whose object lookup failed before
        // the dispatch runs, so a start whose celglist never resolves would not register. Check
        // every start's source placement resolves in the port's assembly (index into the map's
        // objects, or the hashcode in the archive's hash table). The packs and driving shells have
        // no main map — `level_starts` returns empty for them (no placements are parsed there).
        if let Some(m) = level.main_map() {
            let mut unresolved = 0usize;
            for p in level.maps[m].placements.iter()
                .filter(|p| p.flags & 0xa000 == 0 && matches!(p.flags & 0xff, 0x24 | 0x2d)) {
                let resolved = (p.index as usize) < level.maps[m].objects.len()
                    || (p.hashcode != 0 && level.by_hash.contains_key(&p.hashcode));
                if !resolved { unresolved += 1; }
            }
            if unresolved > 0 || k0 > 0 {
                eprintln!("[start-slot] {name}: kind-1 {k1}, kind-0 {k0}, unresolved {unresolved}");
            }
            assert_eq!(unresolved, 0, "{name}: every start placement resolves — the game's celglist gate [M 0x1d1c90] never bites on this disc");
        } else {
            assert!(starts.is_empty(), "{name}: no main map but starts registered");
        }
        if name == "07000007.bin" {
            assert_eq!((k1, k0), (2, 4), "07000007: two 0x24 + four main-map 0x2d [D placements]");
            let kinds: Vec<u16> = starts.iter().map(|s| s.kind).collect();
            assert_eq!(kinds, vec![1, 0, 0, 0, 0, 1], "parse order [D: 0x24 @33.40, four 0x2d, 0x24 @115.56]");
            assert!(near(starts[0].pos[0], 33.40, 0.02) && near(starts[0].pos[2], -70.63, 0.02), "start[0] = the party start [D]");
            assert!(near(starts[1].pos[0], 52.12, 0.02) && near(starts[1].pos[2], -50.67, 0.02), "the first 0x2d [D]");
            assert!(near(starts[2].pos[0], 91.59, 0.02) && near(starts[2].pos[2], -27.98, 0.02), "the fireplace-hall 0x2d [D]");
            assert!(near(starts[3].pos[0], 83.28, 0.02) && near(starts[3].pos[2], -53.29, 0.02), "the third 0x2d [D]");
            assert!(near(starts[4].pos[0], 95.49, 0.02) && near(starts[4].pos[1], 50.50, 0.02) && near(starts[4].pos[2], -85.24, 0.02), "the balcony 0x2d [D]");
            assert!(near(starts[5].pos[0], 115.56, 0.02), "start[5] = the second 0x24 [D]");
        }
        if name == "0700000b.bin" {
            assert_eq!((k1, k0), (1, 2), "0700000b: one 0x24 + two main-map 0x2d [D placements]");
            let kinds: Vec<u16> = starts.iter().map(|s| s.kind).collect();
            assert_eq!(kinds, vec![0, 1, 0], "the kind-0 entry comes FIRST in parse order [D]");
            assert!(near(starts[0].pos[0], 9.92, 0.02) && near(starts[0].pos[2], 21.48, 0.02), "the leading 0x2d [D]");
            assert!(near(starts[1].pos[0], 28.92, 0.02) && near(starts[1].pos[2], 28.67, 0.02), "the only kind-1 entry [D]");
            assert!(near(starts[2].pos[0], -0.42, 0.02) && near(starts[2].pos[1], 58.53, 0.02), "the upper 0x2d [D]");
        }
        if name == "0700000c.bin" {
            assert_eq!((k1, k0), (1, 3), "0700000c: the 0x24 twin pair [D placements]");
            assert!(near(starts[1].pos[0], 8.96, 0.02) && near(starts[0].pos[0], 8.98, 0.02),
                "the 0x2d twin sits 0.03 from its 0x24 — the pick must still be the 0x24 [M 0x190fd0]");
        }
    }
    assert_eq!(total_k1, 38, "kind-1 (0x24) starts on the disc's main maps [D]");
    assert_eq!(total_k0, 18, "kind-0 (0x2d) starts on the disc's main maps [D; the census's 51 = 18 + 33 shell-map dummies]");
}

/// ── THE sister, through the real load path: 0700000b's kind-0 entry is FIRST in parse order
/// [D], so a pick that ignored the kind would stand Bond at (9.92, 48.53, 21.48). The game picks
/// the kind-1 entry [M 0x190fd0 disasm 0x191028/0x1910b0] — the port must land there. ────────────
#[test]
fn load_0b_spawns_at_the_kind1_start_not_the_leading_0x2d() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "0700000b.bin", false).expect("load");
    let (p, yaw) = scene.start.expect("the load picked a start");
    assert!(near(p[0], 28.92, 0.02) && near(p[1], 13.28, 0.02) && near(p[2], 28.67, 0.02),
        "0700000b spawns at the kind-1 (0x24) start, got {p:?} — the leading kind-0 (9.92, 48.53, 21.48) must not win [M 0x190fd0]");
    assert!(near(yaw, -0.769, 0.02), "yaw = the placement's rotation[1] [D], got {yaw}");
    let b = &scene.sim.as_ref().unwrap().bond;
    assert!(near(b.pos[0], 28.92, 0.5) && near(b.pos[2], 28.67, 0.5),
        "Bond stands at the picked start, got {:?}", b.pos);
}

/// ── 07000007 with the second set registered: the cold load still opens at the party start
/// (33.40, 44.50, −70.63) — both 0x24 gates are 0, so pass 1 takes the first kind-1 entry
/// [M 0x190fd0; party07-start-rules pins the same spot from the 06 → 07 carry]. ───────────────────
#[test]
fn load_07_spawns_at_the_party_kind1_start_with_the_0x2d_set_registered() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let (p, yaw) = scene.start.expect("the load picked a start");
    assert!(near(p[0], 33.40, 0.02) && near(p[1], 44.50, 0.02) && near(p[2], -70.63, 0.02),
        "07000007 spawns at the party start plate #0, got {p:?} [D placements; M 0x190fd0]");
    assert!(near(yaw, 1.569, 0.02), "yaw = the placement's rotation[1] [D], got {yaw}");
}
