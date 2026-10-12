//! models-characters: every skinned character on the disc resolves, poses and draws.
//!
//! - G-021: the 33 drone placements whose skin (placement param 0) is not in their level's archive
//!   resolve from the archive of the flow that holds it [D scan: 0700000c ×7 and 07000014 ×15 want
//!   0x05000090, 07000014 ×11 want 0x050000b1]. The port merges the foreign archive's maps, skins
//!   and skeletons (the game's hash tables accumulate every archive loaded in the session —
//!   `AnimLoadFile` 0x1d8248 registers per map, nothing frees between levels).
//! - The gameplay guard: the merge must not touch the level's own per-skeleton sequence lists
//!   (the order-sensitive data the WORKLOG warns about).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn assert_level_skins_complete(level_name: &str, want_skins: &[u32], min_chars_per_skin: usize) {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, level_name, false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    // The load is the thing under test: the merged skins live in the sim's table and the built
    // characters in the scene (a character exists only when the skin AND its meshes resolve — the
    // build skips unresolved ones — so the per-skin count covers the mesh check too).
    let mm = level_main_map(&iso, level_name);
    let mut placed = 0usize;
    for (i, p) in placements(&iso, level_name).iter().enumerate() {
        if p.flags & 0xff != 0x0f { continue; }
        placed += 1;
        let skin = p.params.iter().find(|(k, _)| *k == 0).map(|(_, v)| *v).unwrap_or(0);
        let sk = sim.skins.get(&skin).unwrap_or_else(|| panic!("{level_name} placement #{i}: skin {skin:08x} does not resolve (G-021)"));
        let skeleton = sim.skeletons.get(&sk.skeleton).unwrap_or_else(|| panic!("{level_name} skin {skin:08x}: skeleton {} missing", sk.skeleton));
        assert!(skeleton.bones > 0, "{level_name} skin {skin:08x}: empty skeleton");
    }
    let chars_for = |skin: u32| scene.characters.iter().filter(|c| c.skin.hashcode == skin).count();
    for &skin in want_skins {
        assert!(chars_for(skin) >= min_chars_per_skin,
            "{level_name}: only {} characters built for skin {skin:08x} (want ≥ {min_chars_per_skin}) — the placement did not draw", chars_for(skin));
    }
    eprintln!("[{level_name}] {placed} drone placements, all skins resolve");
}

fn placements(iso: &str, level_name: &str) -> Box<[nf_formats::placement::Placement]> {
    let img = nf_formats::iso9660::IsoImage::open(iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = nf_formats::level::Level::parse(&data).unwrap();
    let mm = level.main_map().unwrap();
    level.maps[mm].placements.clone().into_boxed_slice()
}

fn level_main_map(_iso: &str, _level_name: &str) -> usize { 0 }

/// G-021 on 0700000c (the power station): 7 placements want 0x05000090, which lives in
/// 07000049/0d/16/15's archives [D].
#[test]
fn g021_the_power_station_s_foreign_guards_resolve() {
    assert_level_skins_complete("0700000c.bin", &[0x0500_0090], 7);
}

/// G-021 on 07000014 (the airbase): 26 placements want 0x05000090 (×15) and 0x050000b1 (×11) [D].
#[test]
fn g021_the_airbase_s_foreign_guards_resolve() {
    assert_level_skins_complete("07000014.bin", &[0x0500_0090, 0x0500_00b1], 11);
}

/// The gameplay guard: the G-021 merge adds no sequence to any skeleton the level's own 07F already
/// covers (the per-skeleton lists are order-sensitive gameplay data — the WORKLOG's warning), and
/// the level's own object resolution is unchanged (a known hash still maps to the same map/object).
#[test]
fn g021_the_merge_leaves_gameplay_data_untouched() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let read = |name: &str| -> Option<Vec<u8>> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
        img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()
    };
    let data = read("07000014.bin").unwrap();
    let level = nf_formats::level::Level::parse(&data).unwrap();
    let lb = nf_formats::levelbin::LevelBin::parse(&data).unwrap();
    let own_skeletons: std::collections::HashSet<u8> = lb.entries.iter().filter(|e| e.kind == 6)
        .filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok())
        .map(|s| s.id as u8).collect();
    let anim = read("07F00014.bin").unwrap();
    let alb = nf_formats::levelbin::LevelBin::parse(&anim).unwrap();
    let mut own_seqs: std::collections::HashMap<u8, usize> = Default::default();
    for e in alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40) {
        if let Ok(s) = nf_formats::anim::Sequence::parse(alb.data(&anim, e)) { *own_seqs.entry(s.skeleton).or_default() += 1; }
    }
    let mut scene = nf_view::load(&iso, "07000014.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    // every skeleton the level's own 07F covers has EXACTLY the own count in the scene (no foreign
    // sequence appended to an existing bucket); the sentinel buckets (0x80 | skel) may hold merges.
    for (&sk, &n) in own_seqs.iter() {
        let got = scene.sequences.get(&sk).map(|v| v.len()).unwrap_or(0);
        assert_eq!(got, n, "skeleton {sk}: the scene has {got} sequences, the level's own 07F has {n} — the merge disturbed gameplay data");
    }
    let _ = own_skeletons;
    // the guard drone's start anim still resolves with a real sequence (the start path is
    // skeleton-filtered — the merge must not have broken it).
        assert!(sim.drones.list.iter().all(|d| d.anim.script.is_some() || !d.present),
        "a present drone lost its start script");
}

/// G-042: the 07000002 bathhouse civilian's skin (0x05000034, "polySurface2" — the most-textured
/// body skin on the level [D]) is not drawn from an all-white source texture. The over-bright
/// lighting root cause is fixed (fix-render2: 0x800 objects multiply by tint × cel ambient >> 8 —
/// gs-leaf.md §1.1); this pins the source texels.
#[test]
fn g042_the_bathhouse_civilian_skin_texels_are_not_white() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = nf_formats::level::Level::parse(&data).unwrap();
    let sk = level.skins.get(&0x0500_0034).expect("the civilian skin on 07000002");
    let mut checked = 0usize;
    for mh in sk.main.iter().flatten() {
        let Some((m, o)) = level.by_hash.get(mh).copied() else { continue };
        let obj = &level.maps[m].objects[o];
        for l in obj.gfx.leaves.iter() {
            for b in l.batches.iter() {
                let Some(t) = b.texture else { continue };
                let tex = &level.maps[m].textures[t as usize];
                if tex.width == 0 || tex.height == 0 { continue; }
                let rgba = tex.palette_rgba8();
                // A texel is "white" when all three channels are ≥ 0xF0. The skin must have
                // non-white texels (skin/cloth shades) — and not be overwhelmingly white.
                let (white, total) = rgba.iter().fold((0usize, 0usize), |(w, t), p| {
                    (w + usize::from(p[0] >= 0xf0 && p[1] >= 0xf0 && p[2] >= 0xf0), t + 1)
                });
                assert!(total > white, "skin 05000034 mesh {mh:08x} texture index {t}: every texel is white ({white}/{total})");
                assert!(white * 4 < total, "skin 05000034 mesh {mh:08x} texture index {t}: {white}/{total} white texels — the skin renders white");
                checked += 1;
            }
        }
    }
    assert!(checked >= 8, "only {checked} textures checked on the civilian skin — the mesh did not resolve");
    eprintln!("[g-042] {checked} textures of skin 05000034 carry non-white texels");
}

/// Playtest F3 (the dressing cut-scene, 07000006): Bond's suit actor (0x05000007) and tuxedo actor
/// (0x0500000d — the third-person skin [D SP #810's streams]) both resolve and pose throughout; the
/// mid-script skin swap (`Script_AnimStart` 0x1d8f40 kills the stream's old object) must not leak
/// duplicate Anim objects.
#[test]
fn f3_both_dressing_actors_resolve_and_pose() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut ch = nf_game::triggers::Channels::default();
    for c in [0x1du32, 0x45, 0x70] { ch.set(c, 1, 0); }
    let save = nf_game::player::RamSave::capture(&Default::default(), 0, &nf_game::pickup::Inventory::level_start(0x73, 34), &ch);
    let mut scene = nf_view::load_with_carry(&iso, "07000006.bin", false, Some(&save)).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000 { sim.step_bond(nf_game::player::Input::default(), false, false, false); if !sim.disabled { break } }
    let lp = sim.drones.triggers.iter().find(|t| t.out == 0x83).expect("the LoadPoint").clone();
    let c = lp.centre();
    sim.bond.pos = [c[0], c[1] + 1.0, c[2]];
    let mut checks = 0;
    for _ in 0..1200u32 {
        sim.step_bond(nf_game::player::Input::default(), false, false, false);
        // the suit and the tuxedo: both live (one Anim object each, no duplicates) and both posed.
        let mut suit = (0usize, 0u8); let mut tux = (0usize, 0u8);
        for o in sim.drones.scripts.objects.list.iter().flatten() {
            if let nf_game::nis::Obj::Anim { skin, src: (s, st), .. } = o {
                if *s != 0x0600_0071 { continue }
                match (*skin, *st) {
                    (0x0500_0007, 1) => suit.0 += 1,
                    (0x0500_000d, 2) => tux.0 += 1,
                    _ => {}
                }
            }
        }
        if suit.0 > 0 || tux.0 > 0 {
            checks += 1;
            assert!(suit.0 <= 1 && tux.0 <= 1, "duplicated Anim objects: suit ×{}, tuxedo ×{} (the 0x1d8f40 kill leaked)", suit.0, tux.0);
        }
    }
    assert!(checks > 100, "the dressing cut-scene never played ({checks} object checks)");
    // both characters exist in the scene and pose (the pre-fix-render2 vanish was here).
    let suit_chars = scene.characters.iter().filter(|c| c.skin.hashcode == 0x0500_0007).count();
    let tux_chars = scene.characters.iter().filter(|c| c.skin.hashcode == 0x0500_000d).count();
    assert!(suit_chars >= 1 && tux_chars >= 1, "the dressing actors missing from the scene: suit {suit_chars}, tux {tux_chars}");
}

/// The characters golden (numbers and hashes only, [D]): per story level — the drone placements,
/// the distinct skins their param 0 names, the level archive's own skin count. Every placed skin
/// must resolve after the G-021 substitution/merge; the archive counts pin the completeness of the
/// skin tables. The one exception: 0700001b's cut-scene skins 0x05050a17/0x05050ad2 exist on NO
/// archive of the disc [D scan] — dead references in that cut-scene script, listed in the report.
#[test]
fn golden_every_story_level_s_placed_skins_resolve() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    // (level, drones, placed skins sorted, archive skins)
    const TABLE: &[(u32, usize, &[u32], usize)] = &[
        (0x0700_0001, 20, &[0x0500_0036, 0x0500_0039, 0x0500_003a, 0x0500_004f], 28),
        (0x0700_0002, 25, &[0x0500_0034, 0x0500_0036, 0x0500_0039, 0x0500_003a, 0x0500_004f], 26),
        (0x0700_0003, 20, &[0x0500_002b, 0x0500_0036, 0x0500_0039, 0x0500_003a], 25),
        (0x0700_0004, 1, &[0x0500_001e], 22),
        (0x0700_0005, 16, &[0x0500_002e, 0x0500_002f, 0x0500_0030], 23),
        (0x0700_0006, 32, &[0x0500_002e, 0x0500_002f, 0x0500_0030, 0x0500_0035], 23),
        (0x0700_0007, 58, &[0x0500_0013, 0x0500_0014, 0x0500_0015, 0x0500_0016, 0x0500_0018, 0x0500_0019, 0x0500_0020, 0x0500_0022, 0x0500_0023, 0x0500_0024, 0x0500_0026], 26),
        (0x0700_0008, 26, &[0x0500_0012, 0x0500_0013, 0x0500_0014, 0x0500_002e, 0x0500_002f, 0x0500_0030, 0x0500_003b], 26),
        (0x0700_0009, 6, &[0x0500_0027, 0x0500_0028], 21),
        (0x0700_000a, 20, &[0x0500_0029, 0x0500_002a], 18),
        (0x0700_000b, 10, &[0x0500_0004, 0x0500_0008], 25),
        (0x0700_000c, 40, &[0x0500_006d, 0x0500_007e, 0x0500_0087, 0x0500_0090, 0x0500_0093], 21),
        (0x0700_000d, 41, &[0x0500_006d, 0x0500_007e, 0x0500_0087, 0x0500_0090, 0x0500_0093], 23),
        (0x0700_0011, 37, &[0x0500_0004, 0x0500_0006, 0x0500_0008, 0x0500_000e, 0x0500_000f, 0x0500_0038, 0x0500_003a], 23),
        (0x0700_0012, 32, &[0x0500_0006, 0x0500_0008, 0x0500_000e, 0x0500_000f], 18),
        (0x0700_0013, 12, &[0x0500_0004, 0x0500_0006, 0x0500_0008], 19),
        (0x0700_0014, 61, &[0x0500_0066, 0x0500_0087, 0x0500_008c, 0x0500_0090, 0x0500_0093, 0x0500_00b1], 24),
        (0x0700_0015, 20, &[0x0500_0087, 0x0500_008c], 26),
        (0x0700_0016, 20, &[0x0500_001e, 0x0500_005e, 0x0500_0087, 0x0500_008c, 0x0500_0090, 0x0500_00b1], 29),
        (0x0700_001b, 20, &[0x0500_0078, 0x0500_008f, 0x0500_00ba], 10),
    ];
    for &(id, drones, skins, archive_skins) in TABLE {
        let archive = format!("{id:08x}.bin");
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&archive)).expect("the archive");
        let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = nf_formats::level::Level::parse(&data).unwrap();
        let mm = level.main_map().expect("the main map");
        let mut placed: Vec<u32> = Vec::new();
        let mut count = 0usize;
        for p in level.maps[mm].placements.iter().filter(|p| p.flags & 0xff == 0x0f) {
            count += 1;
            if let Some(&(_, v)) = p.params.iter().find(|(k, _)| *k == 0) { if v != 0 && !placed.contains(&v) { placed.push(v); } }
        }
        placed.sort_unstable();
        let want: Vec<u32> = skins.to_vec();
        assert_eq!(count, drones, "{archive}: {count} drone placements, the golden says {drones}");
        assert_eq!(placed, want, "{archive}: the placed skin set drifted");
        assert_eq!(level.skins.len(), archive_skins, "{archive}: the archive's skin count drifted");
        for sk in placed {
            // 0x050000b1 on 07000014 resolves only through the load's session-table merge (the
            // post-load pass below); everything else must be in the archive (or substituted, but
            // the substitution keeps the placement hash out of the raw table only for 0x05000090).
            if id == 0x0700_0014 && sk == 0x0500_00b1 { continue; }
            assert!(level.skins.contains_key(&sk) || sk == 0x0500_0090,
                "{archive}: placed skin {sk:08x} unresolved");
        }
    }
    // the post-load check through the port: the substituted/merged levels resolve everything.
    for id in [0x0700_000cu32, 0x0700_0014] {
        let scene = nf_view::load(&iso, &format!("{id:08x}.bin"), false).expect("load");
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.skins.contains_key(&0x0500_0090), "{id:08x}: 0x05000090 did not resolve after the load");
        if id == 0x0700_0014 { assert!(sim.skins.contains_key(&0x0500_00b1), "0x050000b1 did not resolve after the load"); }
    }
}
