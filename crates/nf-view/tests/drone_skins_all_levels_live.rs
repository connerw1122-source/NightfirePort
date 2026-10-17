//! G-021 (drone-skins-g021): the game's drone-skin resolution, traced end to end and pinned
//! per placement.
//!
//! The chain, from the decompile:
//! 1. `LoaderLoad` 0x1d4a00 opens a hash-named .bin container and walks its sub-file directory;
//!    `LoaderProcess` 0x1d4880 case 3 hands each type-3 (skin) sub-file to `AnimLoadFile`
//!    0x1bdf40, which registers it first-wins (`hashtable_additem` only when
//!    `hashtable_getitem` 0x1ca5b0 misses). NOTHING frees skins: the only table deletions are
//!    `LoadableLoad`/`LoadableReload` 0x1d69e8/0x1d6ab0's hashtype-4 wipe (0x1ca520 compares the
//!    hash top byte == 0x04 — sequences) and `Script_FreeFromHashTable` 0x1daec0's single
//!    scripts. The table therefore holds the union of every container the session loaded: the
//!    front-end (boot, 0x07000048) + the story levels before the current one (mission::LEVELS
//!    order). `CommonAnimFile` (LoaderLoad type 4) is dead code (no writer, no type-4 call site,
//!    no such archive — fix-missing-drone-skins).
//! 2. `NDrone2_DefaultInit` 0x14b300:563-565 hands `AnimObjectNew` the placement param 0 as the
//!    captain swap (:536-555: kinds 1/2/3/6/7 by difficulty — property 9/5/8; the sim's
//!    `captain_setup`) left it in +0xe0. `AnimObjectNew` 0x1be910:33-43 substitutes 0x05000090 →
//!    0x05000087 on 0x0700000c, → 0x05000093 on 0x07000014 (GameState+0xC), then :44-46 does the
//!    hashtable get; a MISS returns 0 and the caller discards it — the drone keeps no skin
//!    record and no character meshes are created (skinless).
//! 3. [D scan of all 99 FILES.BIN containers]: 0x05000090 is carried by 07000049/0d/15/16, and
//!    0x050000b1 ONLY by 07000015/16 — both story-AFTER 07000014, so on a fresh story run the
//!    session table lacks both when 07000014 loads; the substitution resolves the 22 0x05000090
//!    placements, and the 0x050000b1 placements resolve ONLY as captains (kind 6 → 0x05000093,
//!    kind 7 → 0x050000b2 — both in 07000014's own archive).
//!
//! Opt-in via NF_ISO (the slim ISO is enough). Numbers/ids only.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn story_levels() -> Vec<u32> {
    nf_game::mission::LEVELS.iter().map(|l| l.level).collect()
}

fn open_archive(iso_path: &str, name: &str) -> Option<(Vec<u8>, nf_formats::level::Level)> {
    let img = nf_formats::iso9660::IsoImage::open(iso_path).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).unwrap();
    // 0700000e/0f/10 sit in mission::LEVELS but have no archive in FILES.BIN — the game never
    // loads them (campaign_flow_live's note).
    let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)) else { return None };
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = nf_formats::level::Level::parse(&data).unwrap();
    Some((data, level))
}

struct Row {
    /// Drone placements (flags & 0xff == 0x0f) whose param 0 is a 0x05 skin hashcode — the raw
    /// archive view (the G-021 count).
    placements: usize,
    /// Of those, the ones whose skin is not in the level's own archive.
    before_missing_skin: usize,
    /// The raw-missing param hashes (with multiplicity).
    missing_params: Vec<u32>,
    /// Drones the sim created at the boot difficulty (param 2 ≤ difficulty).
    drones: usize,
    /// Drones that resolve a skin: `sim.skins[dr.skin]` exists with a skeleton and
    /// `scene.characters` built a character for it. The rest are the game's skinless renders.
    resolved: usize,
    /// Resolved skin hashcodes (with multiplicity) — "the skin hash each placement resolves to".
    resolved_hashes: Vec<u32>,
    /// The skinless drones' (raw param, dr.skin) pairs.
    skinless: Vec<(u32, u32)>,
    /// Drones whose raw param is 0x050000b1 and whose captain swap replaced the request
    /// (dr.skin ≠ raw) — the game resolves those from the swapped skin.
    b1_captains: usize,
    /// Drones whose raw param is 0x050000b1 (the rest of the b1 placements sit above the load
    /// difficulty's `param 2` gate and create no drone — nothing to render).
    b1_drones: usize,
    /// For the two substitution levels: whether the sim's 0x05000090 entry carries the variant
    /// skin's data (skeleton + main list) — `AnimObjectNew` 0x1be910:33-43 — and the variant.
    sub_is_variant: Option<(bool, u32)>,
    /// Sleeve-flagged entries still raw in the loaded table (must be 0 — the sleeve rule).
    sleeve_left_after: usize,
}

fn census(iso_path: &str, name: &str) -> Option<Row> {
    let (_, level) = open_archive(iso_path, name)?;
    let mut placements = 0usize;
    let mut before_missing_skin = 0usize;
    let mut missing_params: Vec<u32> = Vec::new();
    let mut raw_by_placement: std::collections::HashMap<usize, u32> = Default::default();
    if let Some(m) = level.main_map() {
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            if p.flags & 0xff != 0x0f { continue; }
            let Some(&(_, skin)) = p.params.iter().find(|(i, _)| *i == 0) else { continue };
            if skin >> 24 != 5 || skin == 0 { continue; }
            placements += 1;
            raw_by_placement.insert(i, skin);
            if !level.skins.contains_key(&skin) {
                before_missing_skin += 1;
                missing_params.push(skin);
            }
        }
    }
    // AFTER: the real loader path (the substitution + the session-gated merges live in
    // `nf_view::load`; the captains swap in the sim's `captain_setup`).
    let scene = nf_view::load(iso_path, name, false).unwrap_or_else(|e| panic!("{name}: load failed: {e}"));
    let sim = scene.sim.as_ref().unwrap_or_else(|| panic!("{name}: no sim"));
    let mut resolved = 0usize;
    let mut resolved_hashes: Vec<u32> = Vec::new();
    let mut skinless: Vec<(u32, u32)> = Vec::new();
    let (mut b1_captains, mut b1_drones) = (0usize, 0usize);
    for dr in &sim.drones.list {
        let raw = raw_by_placement.get(&dr.placement).copied().unwrap_or(0);
        if raw == 0x0500_00b1 { b1_drones += 1; if dr.skin != raw { b1_captains += 1; } }
        let ok = sim.skins.get(&dr.skin).map(|sk| sim.skeletons.get(&sk.skeleton).map(|s| s.bones > 0).unwrap_or(false)).unwrap_or(false)
            && scene.characters.iter().any(|c| c.skin.hashcode == dr.skin);
        if ok { resolved += 1; resolved_hashes.push(dr.skin); }
        else { skinless.push((raw, dr.skin)); }
    }
    // The substitution content pin, inside the loaded sim: the 0x05000090 entry must carry the
    // variant's skeleton + main list (`AnimObjectNew` 0x1be910:33-43 — the game builds the
    // object from the variant's record).
    let sub_is_variant = match id_of(name) {
        0x0700_000c => Some(sub_matches(&sim.skins, 0x0500_0087)),
        0x0700_0014 => Some(sub_matches(&sim.skins, 0x0500_0093)),
        _ => None,
    };
    let mut sleeve_left_after = 0usize;
    for s in sim.skins.values() {
        sleeve_left_after += s.main.iter().flatten().filter(|h| nf_formats::character::Skin::is_sleeve(**h)).count();
    }
    Some(Row { placements, before_missing_skin, missing_params, drones: sim.drones.list.len(),
        resolved, resolved_hashes, skinless, b1_captains, b1_drones, sub_is_variant, sleeve_left_after })
}

fn id_of(name: &str) -> u32 {
    u32::from_str_radix(name.get(..8).unwrap_or(""), 16).unwrap_or(0)
}

/// Whether the sim's 0x05000090 substitute entry carries the variant's skeleton + mains.
fn sub_matches(skins: &std::collections::HashMap<u32, nf_formats::character::Skin>, variant: u32) -> (bool, u32) {
    match (skins.get(&0x0500_0090), skins.get(&variant)) {
        (Some(sub), Some(var)) => (sub.skeleton == var.skeleton
            && sub.main.iter().flatten().collect::<Vec<_>>() == var.main.iter().flatten().collect::<Vec<_>>(), variant),
        _ => (false, variant),
    }
}

/// The per-level census. Run with `--nocapture` to see the table (the report's rows).
#[test]
fn every_story_level_resolves_its_drone_skins() {
    let Some(iso_path) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for id in story_levels() {
        let name = format!("{id:08x}.bin");
        let Some(r) = census(&iso_path, &name) else { eprintln!("[{name}] no archive in FILES.BIN — not a story level, skipped"); continue };
        let mut hashes = r.resolved_hashes.clone();
        hashes.sort_unstable();
        eprintln!("[{name}] placements {:3} (raw-missing {:2} {:?}), drones {:3}, resolved {:3} {:?}, skinless {:2} {:?}, b1-drones {} ({} captains), sleeve-left {}",
            r.placements, r.before_missing_skin, r.missing_params, r.drones, r.resolved, hashes, r.skinless.len(), r.skinless, r.b1_drones, r.b1_captains, r.sleeve_left_after);
        assert_eq!(r.sleeve_left_after, 0, "{name}: sleeve-flagged main entries the loaded skins still carry raw — `AnimProcessSkinData` 0x1bd500:159-166 reroutes them through `AnimSleeveGetEntity(0)` 0x1c6590");
        match id {
            0x0700_000c => {
                // The only raw-missing param is 0x05000090 [D scan]; the substitution
                // (`AnimObjectNew` 0x1be910:33-39) resolves every one of them.
                assert!(r.missing_params.iter().all(|&h| h == 0x0500_0090), "{name}: unexpected raw-missing params {:?}", r.missing_params);
                assert_eq!(r.sub_is_variant, Some((true, 0x0500_0087)), "{name}: the 0x05000090 substitute entry must carry 0x05000087's data");
                assert!(r.skinless.is_empty(), "{name}: {:?} render skinless — the substitution resolves every 0x05000090 placement from the own archive", r.skinless);
            }
            0x0700_0014 => {
                // Raw-missing params: 0x05000090 (substituted to 0x05000093's data) and
                // 0x050000b1 — carried ONLY by 07000015/16 [D scan], which the story loads after
                // 14, so the game's `hashtable_getitem` 0x1ca5b0 misses: skinless unless a
                // captain swap (0x14b300:536-555) replaced the request with an own-archive skin.
                assert!(r.missing_params.iter().all(|&h| h == 0x0500_0090 || h == 0x0500_00b1), "{name}: unexpected raw-missing params {:?}", r.missing_params);
                assert_eq!(r.sub_is_variant, Some((true, 0x0500_0093)), "{name}: the 0x05000090 substitute entry must carry 0x05000093's data");
                for &(raw, eff) in &r.skinless {
                    assert_eq!(raw, 0x0500_00b1, "{name}: a non-0x050000b1 placement ({raw:08x}) renders skinless");
                    assert_eq!(eff, 0x0500_00b1, "{name}: the captain swap replaced the request with {eff:08x} yet it still renders skinless");
                }
                let b1_raw = r.missing_params.iter().filter(|&&h| h == 0x0500_00b1).count();
                assert_eq!(r.skinless.len(), r.b1_drones - r.b1_captains,
                    "{name}: the skinless count must be the non-captain 0x050000b1 drones at the load difficulty ({} b1 drones, {} captain swaps; {} raw placements, the rest above the param-2 gate)", r.b1_drones, r.b1_captains, b1_raw);
                assert!(r.b1_drones <= b1_raw && r.b1_drones > 0, "{name}: the b1 drones ({}) must be a non-empty subset of the {} raw b1 placements", r.b1_drones, b1_raw);
            }
            _ => {
                assert!(r.missing_params.is_empty(), "{name}: unexpected raw-missing params {:?} [D scan: every other story archive carries its placements' skins]", r.missing_params);
                assert!(r.skinless.is_empty(), "{name}: {:?} render skinless — every requested skin is in the own archive", r.skinless);
            }
        }
    }
}
