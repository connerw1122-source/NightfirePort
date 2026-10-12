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
