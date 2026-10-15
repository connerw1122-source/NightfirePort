//! fix-missing-drone-skins (110c, G-021): every story level's drone placements resolve a skin
//! through the real loader path (`nf_view::load`), and every skin's main-mesh list resolves the
//! way the game draws it.
//!
//! The game's resolution order, from the decompile:
//! 1. `AnimObjectNew` 0x1be910 (disasm 0x1be958-0x1be9b0) substitutes skin 0x05000090 before the
//!    hashtable get: level 0x0700000c → 0x05000087, 0x07000014 → 0x05000093 (the word compared is
//!    `GameState+0xC` @0x2a3774, the current level id). The port mirrors this at load.
//! 2. The session hashtable accumulates every archive loaded in the session — `AnimLoadFile`
//!    **0x1bdf40** registers each kind-3 entry first-wins and nothing frees between levels
//!    (0x1d8248 is `Script_Load`, the docs' old address). The port merges the holding archive.
//! 3. `AnimProcessSkinData` 0x1bd500:159-166: a MAIN-list entry whose hash byte 2 carries bit
//!    0x10 is replaced — before the celglist lookup — by `AnimSleeveGetEntity(0)` 0x1c6590: the
//!    first present entity of the fixed 8-entry `SleeveEnts` list @0x2c6ed8 (0x020009d0,
//!    0x02000900, 0x020001f7, 0x02000929, 0x020009e6, 0x020009ff, 0x02000a14, 0x02000a13),
//!    wrapping the list at most once; none present → 0 (nothing drawn for that entry).
//!    `CommonAnimFile` (LoaderLoad type 4, variable 0x30d860) is DEAD CODE in the retail build —
//!    no writer, no type-4 call site, 0 in every snapshot, and no such archive in FILES.BIN —
//!    so it is not a source (the loader comment in `nf_view::load` says the same).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn story_levels() -> Vec<u32> {
    nf_game::mission::LEVELS.iter().map(|l| l.level).collect()
}

fn open_archive(iso_path: &str, name: &str) -> (Vec<u8>, nf_formats::level::Level) {
    let img = nf_formats::iso9660::IsoImage::open(iso_path).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| panic!("{name}: not in FILES.BIN"));
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = nf_formats::level::Level::parse(&data).unwrap();
    (data, level)
}

/// One level's census row.
struct Row {
    /// Drone placements (flags & 0xff == 0x0f) whose param 0 is a 0x05 skin hashcode.
    placements: usize,
    /// Placements whose skin is NOT in the level's own archive — the before count (G-021's
    /// "N drones with a skin from another archive" line, pre-merge).
    before_missing_skin: usize,
    /// Placements whose skin still does not resolve (skin or skeleton missing, or the scene built
    /// no character for it) after `nf_view::load` — the after count; the fix makes this 0.
    after_unresolved: usize,
    /// Sleeve-flagged (byte-2 bit 0x10) main entries across the placed skins, as the raw archive
    /// parses them, and how many of the raw hashes exist in the archive itself.
    sleeve_entries: usize,
    sleeve_raw_present: usize,
    /// Sleeve-flagged entries that STILL sit in the loaded sim's skin table (the fix reroutes
    /// them at load, the way `AnimProcessSkinData` does at registration) — must be 0.
    sleeve_left_after: usize,
}

fn census(iso_path: &str, name: &str) -> Row {
    // BEFORE: the raw archive, before any of the loader's session merges.
    let (_, level) = open_archive(iso_path, name);
    let mut placements = 0usize;
    let mut before_missing_skin = 0usize;
    let mut skins_of_placements: Vec<u32> = Vec::new();
    if let Some(m) = level.main_map() {
        for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x0f) {
            let Some(&(_, skin)) = p.params.iter().find(|(i, _)| *i == 0) else { continue };
            if skin >> 24 != 5 || skin == 0 { continue; }
            placements += 1;
            if !skins_of_placements.contains(&skin) { skins_of_placements.push(skin); }
            if !level.skins.contains_key(&skin) { before_missing_skin += 1; }
        }
    }
    // Sleeve entries of the placed skins (the raw archive's own view).
    let (mut sleeve_entries, mut sleeve_raw_present) = (0usize, 0usize);
    for &sk in &skins_of_placements {
        if let Some(s) = level.skins.get(&sk) {
            for h in s.main.iter().flatten() {
                if nf_formats::character::Skin::is_sleeve(*h) {
                    sleeve_entries += 1;
                    if level.by_hash.contains_key(h) { sleeve_raw_present += 1; }
                }
            }
        }
    }
    // AFTER: the real loader path (substitution + session merge inside `nf_view::load`).
    let scene = nf_view::load(iso_path, name, false).unwrap_or_else(|e| panic!("{name}: load failed: {e}"));
    let sim = scene.sim.as_ref().unwrap_or_else(|| panic!("{name}: no sim"));
    let mut after_unresolved = 0usize;
    {
        let img = nf_formats::iso9660::IsoImage::open(iso_path).unwrap();
        let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap();
        let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let lvl = nf_formats::level::Level::parse(&data).unwrap();
        if let Some(m) = lvl.main_map() {
            for (i, p) in lvl.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x0f).enumerate() {
                let Some(&(_, skin)) = p.params.iter().find(|(i, _)| *i == 0) else { continue };
                if skin >> 24 != 5 || skin == 0 { continue; }
                let ok = sim.skins.get(&skin).map(|sk| sim.skeletons.get(&sk.skeleton).map(|s| s.bones > 0).unwrap_or(false)).unwrap_or(false)
                    && scene.characters.iter().any(|c| c.skin.hashcode == skin);
                if !ok { eprintln!("  [{name}] placement #{i}: skin {skin:08x} does not resolve after load"); after_unresolved += 1; }
            }
        }
    }
    // The game's runtime skin records never carry a 0x10-flagged main hash —
    // `AnimProcessSkinData` 0x1bd500:159-166 reroutes them at registration. The port must too.
    let mut sleeve_left_after = 0usize;
    for &sk in &skins_of_placements {
        if let Some(s) = sim.skins.get(&sk) {
            sleeve_left_after += s.main.iter().flatten().filter(|h| nf_formats::character::Skin::is_sleeve(**h)).count();
        }
    }
    Row { placements, before_missing_skin, after_unresolved, sleeve_entries, sleeve_raw_present, sleeve_left_after }
}

/// The per-level census. Run with `--nocapture` to see the table (the report's before/after).
#[test]
fn every_story_level_resolves_its_drone_skins() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for id in story_levels() {
        let name = format!("{id:08x}.bin");
        let r = census(&iso_path, &name);
        eprintln!("[{name}] placements {:3}, missing skin in archive {:2} (before), unresolved after load {:2}, sleeve entries {:3} (raw present {}, left after load {})",
            r.placements, r.before_missing_skin, r.after_unresolved, r.sleeve_entries, r.sleeve_raw_present, r.sleeve_left_after);
        assert_eq!(r.after_unresolved, 0, "{name}: {} of {} drone placements resolve no skin after load (G-021)", r.after_unresolved, r.placements);
        assert_eq!(r.sleeve_left_after, 0, "{name}: {} sleeve-flagged main entries the loaded skins still carry raw — `AnimProcessSkinData` 0x1bd500:159-166 reroutes them through `AnimSleeveGetEntity(0)` 0x1c6590", r.sleeve_left_after);
    }
}
