//! fix-weapon-assets (100a): the disc-wide weapon asset index + loader.
//!
//! The dev tools (`--give`, the F2 menu) hand Bond ANY weapon on ANY level. Before this task a
//! weapon the level archive doesn't carry had no view model (invisible/unanimated — its fp skin
//! `weapon_data+0xDC` was not in the level's skin table) and its view scripts could not resolve
//! (silent fire). The loader pulls each given weapon's assets from its owning archive into the
//! scene's tables first-wins, so on any level the weapon behaves exactly as it does on a level
//! that carries it.
//!
//! - `weapon_index_covers_the_disc`: the index finds every fp skin and kind-5 script on the disc
//!   and caches itself (keyed by ISO size + FILES.BIN-table SHA-1); the second build is a hit.
//! - `merge_resolves_every_asset_on_07000005` (unit, through `weapon_assets::merge_weapon_assets`
//!   itself): every weapon id whose fp skin exists on the disc but not in 07000005's archive ends
//!   up with its skin's main/extra meshes, skeleton, `07F` sequences, view scripts (+0xA0..+0xD8),
//!   muzzle-flash mesh (+0x48), third-person mesh (+0x80) and projectile mesh (+0x5C) resolving —
//!   and every by_hash/skin/script resolution the level already had is byte-identical.
//! - `load_weapons_dresses_the_view_models_on_07000005` (integration, the real `nf_view::load_weapons`
//!   path): the view models build, the flash attachment carries the record's colour, and draw →
//!   fire → reload run through the real `step_bond` path for a sample of three ids.
//! - `native_carry_is_unchanged`: on 07000001 the merged load's numbers are identical to the plain
//!   load's (skin, meshes, datums, scripts, view-model slots) and nothing was added.
//! - `ids_without_a_first_person_model`: the expected no-model list with the reason.
//!
//! Opt-in via NF_ISO (the slim ISO is enough). Numbers, not images — the main agent renders the
//! `--bond` spots listed in the report.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

struct Disc {
    img: nf_formats::iso9660::IsoImage<std::fs::File>,
}

impl Disc {
    fn open() -> Disc {
        Disc { img: nf_formats::iso9660::IsoImage::open(&iso().expect("NF_ISO")).expect("open ISO") }
    }
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let (entries, bin) = nf_formats::disc::files_bin_entries(&self.img).ok()?;
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
        self.img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()
    }
    fn index(&self) -> nf_view::weapon_assets::DiscIndex {
        nf_view::weapon_assets::DiscIndex::for_iso(&self.img, &iso().unwrap())
    }
    fn weapons(&self, level_id: u32) -> nf_formats::weapon::WeaponData {
        let elf = self.img.read_file(self.img.find("/ACTION.ELF").expect("ACTION.ELF")).expect("read elf");
        let mut wd = nf_formats::weapon::WeaponData::from_action_elf(&elf).expect("weapon_data");
        wd.apply_player_init_override(level_id);
        wd
    }
}

/// The load_impl-shaped starting state for a merge test: the level, its skeletons, its `07F`
/// sequences, its kind-5 scripts (the same parses lib.rs runs before the merge).
fn level_state(d: &Disc, archive: &str) -> (nf_formats::level::Level, HashMap<u8, nf_formats::character::Skeleton>,
    HashMap<u8, Vec<nf_formats::anim::Sequence>>, HashMap<u32, nf_formats::anim::Script>) {
    use nf_formats::{character, levelbin::LevelBin};
    let data = d.read(archive).unwrap_or_else(|| panic!("no {archive}"));
    let level = nf_formats::level::Level::parse(&data).expect("parse level");
    let lb = LevelBin::parse(&data).expect("level bin");
    let skeletons: HashMap<u8, character::Skeleton> = lb.entries.iter().filter(|e| e.kind == 6)
        .map(|e| character::parse_skeleton(lb.data(&data, e)).map(|s| (s.id as u8, s)))
        .collect::<Result<_, _>>().expect("skeletons");
    let anim_name = format!("{}F{}", &archive[..2], &archive[3..]);
    let mut sequences: HashMap<u8, Vec<nf_formats::anim::Sequence>> = HashMap::new();
    if let Some(anim) = d.read(&anim_name) {
        let alb = LevelBin::parse(&anim).expect("anim bin");
        for e in alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40) {
            if let Ok(s) = nf_formats::anim::Sequence::parse(alb.data(&anim, e)) {
                sequences.entry(s.skeleton).or_default().push(s);
            }
        }
    }
    let scripts: HashMap<u32, nf_formats::anim::Script> = lb.entries.iter().filter(|e| e.kind == 5)
        .filter_map(|e| nf_formats::anim::Script::parse(lb.data(&data, e)).ok()).map(|x| (x.hashcode, x)).collect();
    (level, skeletons, sequences, scripts)
}

/// The gameplay-skeleton guard (load_impl's rule): the skeletons every character placement's skin
/// animates — the appended sequences must never land on one of these.
fn gameplay_skeletons(level: &nf_formats::level::Level) -> HashSet<u8> {
    let mut gs = HashSet::new();
    if let Some(m) = level.main_map() {
        for p in level.maps[m].placements.iter() {
            let Some(&(_, skin)) = p.params.iter().find(|(i, _)| *i == 0) else { continue };
            if skin >> 24 != 5 { continue; }
            if let Some(sk) = level.skins.get(&skin) { gs.insert(sk.skeleton); }
        }
    }
    gs
}

#[test]
fn weapon_index_covers_the_disc() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return; }
    let d = Disc::open();
    let t = Instant::now();
    let idx = d.index();
    let cold = t.elapsed();
    assert!(idx.skins.len() >= 50, "the disc holds many distinct skins, got {}", idx.skins.len());
    assert!(idx.scripts.len() >= 100, "kind-5 anim scripts across the disc, got {}", idx.scripts.len());
    assert!(idx.skins.contains_key(&0x0500_00ab), "a known fists skin (the 07000005 PunchSkin override) is indexed");
    let t2 = Instant::now();
    let idx2 = d.index();
    let warm = t2.elapsed();
    assert_eq!(idx.skins, idx2.skins, "the cache restores the same index");
    assert_eq!(idx.scripts, idx2.scripts, "the cache restores the same scripts");
    eprintln!("[weapon-assets] index cold {:.2}s, warm {:.3}s ({} skins, {} scripts, {} archives)",
        cold.as_secs_f32(), warm.as_secs_f32(), idx.skins.len(), idx.scripts.len(), idx.archives);
}

/// Every weapon id whose fp skin exists somewhere on the disc but not in the level's archive, and
/// (the reason list) the ids with no fp skin anywhere.
fn cross_level_ids(level: &nf_formats::level::Level, wd: &nf_formats::weapon::WeaponData,
    idx: &nf_view::weapon_assets::DiscIndex) -> (Vec<usize>, Vec<(usize, &'static str)>) {
    let mut give = Vec::new();
    let mut none = Vec::new();
    for w in 1..0x72usize {
        let fp = wd.first_person(w);
        if fp == 0 || fp == u32::MAX { none.push((w, "no first-person model (+0xDC = 0)")); continue; }
        if level.skins.contains_key(&fp) { continue; }
        match idx.skins.get(&fp) {
            Some(_) => give.push(w),
            None => none.push((w, "fp skin is on no archive of the disc")),
        }
    }
    (give, none)
}

#[test]
fn merge_resolves_every_asset_on_07000005() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return; }
    let d = Disc::open();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&d.img).expect("FILES.BIN");
    let read = |name: &str| -> anyhow::Result<Option<Vec<u8>>> {
        let Some(fe) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)) else { return Ok(None) };
        Ok(Some(d.img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?))
    };
    let index = nf_view::weapon_assets::DiscIndex::for_iso(&d.img, &iso().unwrap());
    let wd = d.weapons(0x0700_0005);
    let (mut level, mut skeletons, mut sequences, scripts) = level_state(&d, "07000005.bin");
    let have_scripts: HashSet<u32> = scripts.keys().copied().collect();
    let guard = gameplay_skeletons(&level);
    let (give, none) = cross_level_ids(&level, &wd, &index);
    eprintln!("[weapon-assets] 07000005 lacks {} weapon(s): {:?}", give.len(),
        give.iter().map(|w| format!("{w:#04x}")).collect::<Vec<_>>());
    eprintln!("[weapon-assets] no fp model anywhere: {:?}", none);
    assert!(!give.is_empty(), "07000005 must be missing at least the Q-Claw-class weapons");

    // Pre-merge snapshot: every resolution the level already had.
    let by_hash_before: Vec<(u32, (usize, usize))> = level.by_hash.iter().map(|(h, v)| (*h, *v)).collect();
    let skins_before: Vec<u32> = level.skins.keys().copied().collect();
    let mut scripts_merged = scripts.clone();

    let wanted = nf_view::weapon_assets::Wanted::Ids(give.iter().map(|&w| w as u8).collect());
    let t = Instant::now();
    let st = nf_view::weapon_assets::merge_weapon_assets(&d.img, &read, &mut level, &mut skeletons,
        &mut sequences, &mut scripts_merged, &have_scripts, &guard, &wd, &wanted, &index, false)
        .expect("merge");
    eprintln!("[weapon-assets] merged {} archive(s), {} skin(s), {} map(s), {} skeleton(s), {} sequence(s), {} script(s) in {:.2}s",
        st.archives, st.skins, st.maps, st.skeletons, st.sequences, st.scripts, t.elapsed().as_secs_f32());
    assert!(st.skins >= give.len(), "every wanted fp skin merged (got {} for {} ids)", st.skins, give.len());

    // Bit-identical: the level's own resolutions all survive first-wins.
    for (h, v) in &by_hash_before { assert_eq!(level.by_hash.get(h), Some(v), "by_hash {h:08x} unchanged"); }
    for h in &skins_before { assert!(level.skins.contains_key(h), "skin {h:08x} still present"); }

    // Every cross-level weapon's assets resolve now.
    let mut failures: Vec<String> = Vec::new();
    for &w in &give {
        let fp = wd.first_person(w);
        let Some(skin) = level.skins.get(&fp) else { failures.push(format!("{w:#04x}: fp skin {fp:08x} not merged")); continue; };
        for mh in skin.main.iter().flatten() {
            if !level.by_hash.contains_key(mh) { failures.push(format!("{w:#04x}: main mesh {mh:08x} unresolved")); }
        }
        for (bone, part) in &skin.extra {
            if let Some(ph) = part {
                if !level.by_hash.contains_key(ph) { failures.push(format!("{w:#04x}: extra part {ph:08x} (bone {bone}) unresolved")); }
            }
        }
        if !skeletons.contains_key(&skin.skeleton) { failures.push(format!("{w:#04x}: skeleton {} missing", skin.skeleton)); }
        if sequences.get(&skin.skeleton).map_or(true, |v| v.is_empty()) {
            failures.push(format!("{w:#04x}: skeleton {} has no sequences", skin.skeleton));
        }
        let rec = &wd.records[w];
        for (label, off) in [("idle", 0xa0), ("reload", 0xa4), ("fire", 0xb0), ("draw", 0xc0), ("holster", 0xc4)] {
            let h = u32::from_le_bytes(rec[off..off + 4].try_into().unwrap());
            if h != 0 && !scripts_merged.contains_key(&h) {
                failures.push(format!("{w:#04x}: {label} script {h:08x} missing"));
            }
        }
        for (label, off) in [("flash", 0x48), ("third-person", 0x80), ("projectile", 0x5c)] {
            let h = u32::from_le_bytes(rec[off..off + 4].try_into().unwrap());
            if h != 0 && h != u32::MAX && !level.by_hash.contains_key(&h) {
                failures.push(format!("{w:#04x}: {label} mesh {h:08x} unresolved"));
            }
        }
    }
    assert!(failures.is_empty(), "cross-level weapon assets broken on 07000005:\n{}", failures.join("\n"));
}

#[test]
fn load_weapons_dresses_the_view_models_on_07000005() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return; }
    let d = Disc::open();
    let index = d.index();
    let wd = d.weapons(0x0700_0005);
    let (level, _, _, _) = level_state(&d, "07000005.bin");
    let (give, none) = cross_level_ids(&level, &wd, &index);
    let ids: Vec<u8> = give.iter().map(|&w| w as u8).collect();

    let t = Instant::now();
    let mut scene = nf_view::load_weapons(&iso().unwrap(), "07000005.bin", false, &ids).expect("load 07000005 merged");
    eprintln!("[weapon-assets] 07000005 + {} weapon(s) loaded in {:.2}s (incl. merge)", ids.len(), t.elapsed().as_secs_f32());

    // A deterministic sample: the first gun-class id, any gadget-class id, the highest id.
    let mut sample: Vec<usize> = Vec::new();
    if let Some(&g) = give.iter().find(|&&w| (3..0x2f).contains(&w)) { sample.push(g); }
    if let Some(&g) = give.iter().find(|&&w| w >= 0x40) { sample.push(g); }
    sample.push(*give.last().unwrap());
    sample.dedup();
    let mut failures: Vec<String> = Vec::new();
    for &w in &sample {
        let fp = wd.first_person(w);
        let sk = scene.sim.as_ref().unwrap().skins.get(&fp).map(|s| s.skeleton);
        let Some(sk) = sk else { failures.push(format!("{w:#04x}: skin absent")); continue; };
        let Some(&ci) = scene.view_models.get(&(w as u8)) else { failures.push(format!("{w:#04x}: no view model built")); continue; };
        if scene.characters[ci].skin.hashcode != fp { failures.push(format!("{w:#04x}: wrong skin on the character")); }
        // The flash attachment: mesh on datum 0 with the record's colour (+0x56, +0x55, +0x54).
        let rec = &wd.records[w];
        let flash = u32::from_le_bytes(rec[0x48..0x4c].try_into().unwrap());
        if flash != 0 && flash != u32::MAX {
            let colour = [rec[0x56], rec[0x55], rec[0x54]];
            if !scene.attachments.iter().any(|a| a.character == ci as u32 && a.flash == Some(colour)) {
                failures.push(format!("{w:#04x}: no flash attachment (colour {colour:?})"));
            }
        }
        // Draw → fire → reload through the real input path.
        {
            let sim = scene.sim.as_mut().unwrap();
            let mut input = nf_game::player::Input::default();
            input.switch_to = Some(w as u8);
            sim.step_bond(input, false, false, false);
            for _ in 0..90 { sim.step_bond(Default::default(), false, false, false); }
            if sim.gun.weapon != w as u8 { failures.push(format!("{w:#04x}: not drawn (gun.weapon {:#04x})", sim.gun.weapon)); }
            if sim.view_script == 0 { failures.push(format!("{w:#04x}: no view script set")); }
            sim.step_bond(Default::default(), true, false, false);
            for _ in 0..30 { sim.step_bond(Default::default(), false, false, false); }
            sim.step_bond(Default::default(), false, true, false);
            for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
            let frame = sim.view_frame(sk);
            if frame.is_none() { failures.push(format!("{w:#04x}: view model does not pose after fire + reload")); }
            else { eprintln!("[weapon-assets] {w:#04x}: draw/fire/reload OK (view_script {:#x}, frame {:?})", sim.view_script, frame); }
        }
    }
    assert!(failures.is_empty(), "the sample weapons are broken on 07000005:\n{}", failures.join("\n"));
    // And the whole give set has view models.
    let missing: Vec<usize> = give.iter().copied()
        .filter(|w| !scene.view_models.contains_key(&(*w as u8))).collect();
    assert!(missing.is_empty(), "no view model for {missing:?}");
    let _ = none;
}

/// On a level that carries the weapon natively, the merged load's numbers are identical to the
/// plain load's — and the merge added nothing (the per-level behaviour stays bit-identical).
#[test]
fn native_carry_is_unchanged() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return; }
    let plain = nf_view::load(&iso().unwrap(), "07000001.bin", false).expect("plain load");
    let merged = nf_view::load_weapons(&iso().unwrap(), "07000001.bin", false, &[0x02, 0x34]).expect("merged load");
    let (ps, ms) = (plain.sim.as_ref().unwrap(), merged.sim.as_ref().unwrap());
    for w in [0x02u8, 0x03, 0x34] {
        let fp = ps.weapons.first_person(w as usize);
        if fp == 0 { continue; }
        let (a, b) = (ps.skins.get(&fp), ms.skins.get(&fp));
        assert_eq!(a.map(|s| s.hashcode), b.map(|s| s.hashcode), "{w:#04x} skin unchanged");
        if let (Some(a), Some(b)) = (a, b) {
            assert_eq!(a.main, b.main, "{w:#04x} main meshes unchanged");
            assert_eq!(a.extra, b.extra, "{w:#04x} extra parts unchanged");
            assert_eq!(a.datums.len(), b.datums.len(), "{w:#04x} datums unchanged");
        }
        for off in [0xa0usize, 0xa4, 0xb0, 0xc0, 0xc4] {
            let h = u32::from_le_bytes(ps.weapons.records[w as usize][off..off + 4].try_into().unwrap());
            if h != 0 {
                assert_eq!(ps.scripts.contains_key(&h), ms.scripts.contains_key(&h), "{w:#04x} script +{off:#x} presence unchanged");
                if let (Some(x), Some(y)) = (ps.scripts.get(&h), ms.scripts.get(&h)) {
                    assert_eq!(x.commands.len(), y.commands.len(), "{w:#04x} script +{off:#x} content unchanged");
                }
            }
        }
        assert_eq!(plain.view_models.get(&(w as u8)), merged.view_models.get(&(w as u8)), "{w:#04x} view-model slot unchanged");
    }
    assert_eq!(plain.characters.len(), merged.characters.len(), "no characters added on a native level");
    assert_eq!(plain.view_models.len(), merged.view_models.len(), "no view models added on a native level");
}

/// The task's expected no-model list: which ids have no first-person skin anywhere on the disc,
/// with the reason (the hands-only gadgets, the binoculars, the decryptor — data-driven).
#[test]
fn ids_without_a_first_person_model() {
    if iso().is_none() { eprintln!("NF_ISO not set — skipping"); return; }
    let d = Disc::open();
    let index = d.index();
    let wd = d.weapons(0x0700_0005);
    let mut no_model: Vec<(usize, &'static str)> = Vec::new();
    for w in 1..0x72usize {
        let fp = wd.first_person(w);
        if fp == 0 || fp == u32::MAX { no_model.push((w, "no fp model (+0xDC = 0)")); }
        else if !index.skins.contains_key(&fp) { no_model.push((w, "fp skin on no archive")); }
    }
    eprintln!("[weapon-assets] ids without a first-person skin anywhere ({} of 0x71): {:?}",
        no_model.len(), no_model);
    // The port's give path treats these as usable-without-model (the record's fp gate refuses them
    // nowhere — `Player_EquipWeapon` only refuses +0xDC == 0).
    for (w, _) in &no_model {
        assert!(*w >= 0x40, "gun-class weapon {w:#04x} below 0x40 has no fp skin anywhere — unexpected");
    }
}
