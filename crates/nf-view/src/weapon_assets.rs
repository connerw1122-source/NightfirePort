//! The disc-wide weapon asset index + loader (task 100a, `fix-weapon-assets`).
//!
//! A level archive ships only its own weapons' first-person skins (`weapon_data+0xDC`, kind-3
//! `05xxxxxx` entries), so a weapon handed over by `--give` or the dev menu on a level that does
//! not carry it had no view model (invisible/unanimated) and its view scripts could not resolve
//! (silent fire). The real game never hits this — a level only ever offers its own set — but the
//! port's dev tools hand Bond ANY weapon on ANY level on purpose.
//!
//! This module indexes the disc once per ISO (cached as a small text file in the user's temp dir,
//! keyed by the ISO's size and a SHA-1 over the FILES.BIN table, never game bytes and never in the
//! repo) and merges a weapon's assets from the archive that owns them into the scene's tables —
//! the way `merge_dev_view_models` (folded 190-fix-dev2) already did for `--dev`, generalized to a
//! wanted-weapon-id list so a non-dev load can pull exactly the given weapons. The merge is the
//! game's own session-accumulation mechanism (`AnimLoadFile` 0x1d8248 registers every archive's
//! skins/objects into the accumulating hashtable; nothing frees between levels [M]): skins, their
//! maps (meshes + textures live per map), skeletons, the paired `07F` animation-archive sequences
//! and the kind-5 view scripts join the level's tables first-wins, so every resolution the level
//! already had is unchanged.
//!
//! Everything the view-model build path (`Scene::build`'s first-person section) and the switch
//! machine consume then resolves for the merged weapons:
//! - the fp skin + its `main`/`extra` meshes (skinned parts + rigid parts),
//! - the sleeve (`AnimSleeveGetEntity(0)`'s set, `Scene::build`'s SLEEVES — usually the level's own),
//! - the skeleton and its `07F` sequences (draw +0xC0 / holster +0xC4 / idle +0xA0 / reload
//!   +0xA4/+0xA8/+0xAC / fire +0xB0/+0xB4 / variant +0xC8 / idle-alert +0xCC/+0xD0/+0xD4 scripts
//!   from `nf_formats::weapon` — the op-0 windows inside them name the sequence hashes),
//! - the muzzle-flash mesh (+0x48, datum 0, colour +0x56/55/54), the third-person mesh (+0x80,
//!   datum +0x50) and the projectile mesh (+0x5C) — the last three also ride `merge_weapon_models`
//!   for earlier story archives; this module covers the rest from the skin's owning archive.

use crate::{disc, Level};
use anyhow::{Context, Result};
use nf_formats::iso9660::IsoImage;
use nf_formats::{anim::Script, anim::Sequence, character, levelbin::LevelBin};
use std::collections::HashMap;
use std::path::PathBuf;

/// The four fists skins (`Player_Init` 0x1910f8 overrides record 1 +0xDC per level from
/// `PunchSkin.204`; the port's `apply_player_init_override` mirrors it). Bond always owns the
/// fists, so a missing fists skin always merges (`merge_dev_view_models`' set).
pub const PUNCH_SKINS: [u32; 4] = [0x0500_00ab, 0x0500_0099, 0x0500_003d, 0x0500_00a1];

/// The `weapon_data` view-script offsets (`nf_formats::weapon`'s accessors): idle, reload,
/// reload-start, reload-end, fire, fire-last, the 0xB8/0xBC pair, draw, holster, variant swap and
/// the idle-alert/holster-variant set.
pub const SCRIPT_OFFSETS: [usize; 14] =
    [0xa0, 0xa4, 0xa8, 0xac, 0xb0, 0xb4, 0xb8, 0xbc, 0xc0, 0xc4, 0xc8, 0xcc, 0xd0, 0xd4];

/// Which weapons' assets a load should pull in: none (the game's own behaviour), the `--give` /
/// dev-menu ids (surgical — only those weapons' skins, meshes, skeletons, sequences and scripts),
/// or everything with a first-person model (the `--dev` behaviour).
#[derive(Clone, Debug, PartialEq)]
pub enum Wanted {
    None,
    Ids(Vec<u8>),
    All,
}

/// What a merge added (the tests assert on this; the load log reports it).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MergeStats {
    pub archives: usize,
    pub skins: usize,
    pub maps: usize,
    pub skeletons: usize,
    pub sequences: usize,
    pub scripts: usize,
}

/// skin hash → owning archive, kind-5 script hash → owning archive, and (weapon-model-coverage)
/// map-object hashcode → first archive on the disc that holds it (the first archive on the disc
/// that holds it — the same "first wins" the game's accumulating tables produce).
#[derive(Clone, Debug, Default)]
pub struct DiscIndex {
    /// The cache key the index was built for (the ISO's size; 0 = unknown/in-memory).
    pub cache_key: u64,
    pub skins: HashMap<u32, String>,
    pub scripts: HashMap<u32, String>,
    /// weapon-model-coverage: every map-object hashcode (the weapon records' +0x48/+0x4C/+0x5C/
    /// +0x7C/+0x80 meshes live here) → the first archive whose ENTITY_PARAMS names it.
    pub objects: HashMap<u32, String>,
    pub archives: usize,
}

impl DiscIndex {
    /// The index for this ISO: the cached copy when present and current, else one disc pass (every
    /// `.bin`'s directory + kind-3 heads and kind-5 scripts) and a cache write. The cache key is
    /// the ISO's size and a SHA-1 over the FILES.BIN table (name, offset, size per entry), so a
    /// different disc image can never hit a stale file.
    pub fn for_iso<R: std::io::Read + std::io::Seek>(iso: &IsoImage<R>, iso_path: &str) -> DiscIndex {
        let (key, cache_path) = match cache_path(iso, iso_path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("weapon assets: no cache key ({e:#}) — building the index in memory");
                return Self::scan(iso, 0);
            }
        };
        if let Some(idx) = std::fs::read_to_string(&cache_path).ok().and_then(|s| parse_cache(&s)) {
            if idx.cache_key == key { return idx; }
            eprintln!("weapon assets: cache {} is stale — rescanning", cache_path.display());
        }
        let t = std::time::Instant::now();
        let idx = Self::scan(iso, key);
        eprintln!("weapon assets: indexed {} archives ({} skins, {} scripts) in {:.2}s -> {}",
            idx.archives, idx.skins.len(), idx.scripts.len(), t.elapsed().as_secs_f32(), cache_path.display());
        if let Ok(body) = write_cache(&idx) {
            if let Some(dir) = cache_path.parent() { let _ = std::fs::create_dir_all(dir); }
            let tmp = cache_path.with_extension("tmp");
            if std::fs::write(&tmp, &body).and_then(|_| std::fs::rename(&tmp, &cache_path)).is_err() {
                eprintln!("weapon assets: could not write the cache {} (index stays in memory)", cache_path.display());
            }
        }
        idx
    }

    /// One pass over every level `.bin` on the disc. Kind-3 entries: the first u32 of the raw entry
    /// is the skin's hashcode (`Level::parse`'s skin rule; `parse_skin` itself needs the archive's
    /// skeleton bone counts, which the index pass does not owe). Kind-5 entries: `Script::parse`'s
    /// hashcode. Kind-0/1/2/11 entries (maps): every ENTITY_PARAMS (0x04) block's first u32 — the
    /// object hashcode `Level::parse` keys `by_hash` by [D census: 1,153 distinct on our disc].
    /// Only the `05`/`06` prefixes are tabulated for skins/scripts — everything else (map objects,
    /// NIS, menus) is not a weapon asset.
    fn scan<R: std::io::Read + std::io::Seek>(iso: &IsoImage<R>, cache_key: u64) -> DiscIndex {
        let mut idx = DiscIndex { cache_key, ..Default::default() };
        let Ok((entries, bin)) = disc::files_bin_entries(iso) else { return idx };
        let names: Vec<(String, u64, u64)> = entries.iter()
            .filter(|e| e.name.len() == 12 && e.name.ends_with(".bin"))
            .map(|e| (e.name.clone(), bin.lba as u64 * 2048 + e.offset as u64, e.size as u64)).collect();
        idx.archives = names.len();
        for (name, off, size) in &names {
            let fd = match iso.read_bytes(*off, *size) { Ok(d) => d, Err(_) => continue };
            let Ok(l) = LevelBin::parse(&fd) else { continue };
            for pe in l.entries.iter() {
                match pe.kind {
                    3 => {
                        let raw = l.data(&fd, pe);
                        if raw.len() < 4 { continue; }
                        let h = u32::from_le_bytes(raw[0..4].try_into().unwrap());
                        if (h & 0xff00_0000) == 0x0500_0000 { idx.skins.entry(h).or_insert_with(|| name.clone()); }
                    }
                    5 => {
                        if let Ok(sc) = Script::parse(l.data(&fd, pe)) {
                            if (sc.hashcode & 0xff00_0000) == 0x0600_0000 { idx.scripts.entry(sc.hashcode).or_insert_with(|| name.clone()); }
                        }
                    }
                    0 | 1 | 2 | 11 => {
                        // The map-object hashcodes (weapon-model-coverage). `MapFile::parse` is the
                        // same walk `Level::parse` runs; a non-map entry here (menu packs) just
                        // yields no 0x04 blocks — tolerated, the index only owes object names.
                        let Ok(mf) = nf_formats::map::MapFile::parse(l.data(&fd, pe)) else { continue };
                        for blk in mf.blocks.iter().filter(|b| b.id == nf_formats::map::id::ENTITY_PARAMS) {
                            let payload = blk.payload(l.data(&fd, pe));
                            if payload.len() < 4 { continue; }
                            let h = u32::from_le_bytes(payload[0..4].try_into().unwrap());
                            if h != u32::MAX { idx.objects.entry(h).or_insert_with(|| name.clone()); }
                        }
                    }
                    _ => {}
                }
            }
        }
        idx
    }
}

/// The cache file's path: `<temp>/nf-weapon-index-<iso size>-<FILES.BIN table sha1[0..12]>.txt`.
fn cache_path<R: std::io::Read + std::io::Seek>(iso: &IsoImage<R>, iso_path: &str) -> Result<(u64, PathBuf)> {
    let size = std::fs::metadata(iso_path).with_context(|| format!("stat {iso_path}"))?.len();
    let (entries, _bin) = disc::files_bin_entries(iso).context("FILES.BIN")?;
    let mut h = nf_formats::sha1::Sha1::new();
    for e in &entries {
        h.update(e.name.as_bytes());
        h.update(&e.offset.to_le_bytes());
        h.update(&e.size.to_le_bytes());
    }
    let hex = nf_formats::sha1::hex(&h.finish());
    let name = format!("nf-weapon-index-{size:08x}-{}.txt", &hex[..12]);
    Ok((size, std::env::temp_dir().join(name)))
}

/// The cache body: the version/key header, then `S <hash> <archive>` (skin) / `C <hash>
/// <archive>` (script) / `O <hash> <archive>` (map object, weapon-model-coverage) rows — text, so
/// it stays greppable and holds no game bytes. V2 adds the object rows; a V1 file is stale.
fn write_cache(idx: &DiscIndex) -> Result<String> {
    let mut s = format!("#NF-WEAPON-INDEX-V2 {:016x}\n", idx.cache_key);
    for (h, a) in &idx.skins { s.push_str(&format!("S {h:08x} {a}\n")); }
    for (h, a) in &idx.scripts { s.push_str(&format!("C {h:08x} {a}\n")); }
    for (h, a) in &idx.objects { s.push_str(&format!("O {h:08x} {a}\n")); }
    Ok(s)
}

fn parse_cache(s: &str) -> Option<DiscIndex> {
    let mut idx = DiscIndex::default();
    let mut version = "";
    for line in s.lines() {
        let mut it = line.split_whitespace();
        match it.next()? {
            "#NF-WEAPON-INDEX-V2" => { idx.cache_key = u64::from_str_radix(it.next()?, 16).ok()?; version = "V2"; }
            "#NF-WEAPON-INDEX-V1" => { idx.cache_key = u64::from_str_radix(it.next()?, 16).ok()?; version = "V1"; }
            "S" => { let h = u32::from_str_radix(it.next()?, 16).ok()?; idx.skins.insert(h, it.next()?.to_string()); }
            "C" => { let h = u32::from_str_radix(it.next()?, 16).ok()?; idx.scripts.insert(h, it.next()?.to_string()); }
            "O" if version == "V2" => { let h = u32::from_str_radix(it.next()?, 16).ok()?; idx.objects.insert(h, it.next()?.to_string()); }
            _ => {}
        }
    }
    // A V1 cache has no object rows — the mesh top-up would find nothing; caller rescans.
    (version == "V2" && (!idx.skins.is_empty() || !idx.scripts.is_empty())).then_some(idx)
}

/// The fp skins a load still needs: every wanted weapon's `+0xDC` the level archive lacks (all
/// weapons for `Wanted::All`, exactly the listed ids otherwise) plus the missing fists skins.
pub fn wanted_skins(level: &Level, wd: &nf_formats::weapon::WeaponData, wanted: &Wanted) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let mut push = |h: u32, out: &mut Vec<u32>| {
        if h != 0 && h != u32::MAX && !level.skins.contains_key(&h) && !out.contains(&h) { out.push(h); }
    };
    match wanted {
        Wanted::None => {}
        Wanted::Ids(ids) => for &w in ids {
            if w < 0x72 { push(wd.first_person(w as usize), &mut out); }
        },
        Wanted::All => for w in 1..0x73usize { push(wd.first_person(w), &mut out); },
    }
    for &p in &PUNCH_SKINS { push(p, &mut out); }
    out
}

/// The kind-5 view-script hashes a weapon's switch/fire/reload machine consumes (the non-zero
/// `weapon_data` hashes at the `SCRIPT_OFFSETS`), for the wanted weapons only when surgical.
pub fn wanted_scripts(wd: &nf_formats::weapon::WeaponData, wanted: &Wanted) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let records: Vec<usize> = match wanted {
        Wanted::None => vec![],
        Wanted::Ids(ids) => ids.iter().filter(|&&w| w < 0x72).map(|&w| w as usize).collect(),
        Wanted::All => (1..0x73).collect(),
    };
    for w in records {
        for &off in &SCRIPT_OFFSETS {
            let h = u32::from_le_bytes(wd.records[w][off..off + 4].try_into().unwrap());
            if h != 0 && h != u32::MAX && !out.contains(&h) { out.push(h); }
        }
    }
    out
}

/// The weapon-model mesh hashes the wanted weapons still miss (weapon-model-coverage): the
/// records' +0x80/+0x4C/+0x48/+0x5C/+0x7C for exactly the wanted subset, plus — for `All` (the
/// dev panel, which can give anything) — the whole-table list `wanted_weapon_models` covers (the
/// sleeves and the casing meshes). A surgical `--give` stays surgical: only its own weapons'
/// meshes can open another archive, so a native-weapon give adds nothing (native_carry_is_unchanged).
pub fn meshes_for_wanted(level: &Level, wd: &nf_formats::weapon::WeaponData, wanted: &Wanted,
    casing_meshes: &[u32]) -> Vec<u32> {
    match wanted {
        Wanted::None => vec![],
        Wanted::All => crate::wanted_weapon_models(level, wd, casing_meshes)
            .into_iter().filter(|h| !level.by_hash.contains_key(h)).collect(),
        Wanted::Ids(ids) => {
            let mut out: Vec<u32> = Vec::new();
            for &w in ids {
                let Some(r) = wd.records.get(w as usize) else { continue };
                for off in [0x80usize, 0x4c, 0x48, 0x5c, 0x7c] {
                    let h = u32::from_le_bytes(r[off..off + 4].try_into().unwrap());
                    if h != 0 && h != u32::MAX && !level.by_hash.contains_key(&h) && !out.contains(&h) { out.push(h); }
                }
            }
            out
        }
    }
}

/// Merges the wanted weapons' assets from their owning archives into the level's tables. First
/// wins everywhere (`by_hash` entries, skins, scripts), so every resolution the level already had
/// is bit-identical; appended maps never shift an existing (map, object) index; sequences append
/// to the fp skeletons' buckets (existing indices stay stable, duplicate hashes resolve to the
/// level's own first). After this, `Scene::build`'s first-person section and the switch machine's
/// script lookups resolve for the merged weapons exactly as they do on a level that carries them
/// natively. `dev` keeps `--dev`'s original lend-everything behaviour (an opened archive lends all
/// its missing skins); a surgical merge lends exactly the wanted skins.
#[allow(clippy::too_many_arguments)]
pub fn merge_weapon_assets<R: std::io::Read + std::io::Seek>(
    iso: &IsoImage<R>,
    read: &impl Fn(&str) -> Result<Option<Vec<u8>>>,
    level: &mut Level,
    skeletons: &mut HashMap<u8, character::Skeleton>,
    sequences: &mut HashMap<u8, Vec<Sequence>>,
    scripts: &mut HashMap<u32, Script>,
    have_scripts: &std::collections::HashSet<u32>,
    guard_skeletons: &std::collections::HashSet<u8>,
    wd: &nf_formats::weapon::WeaponData,
    wanted: &Wanted,
    index: &DiscIndex,
    dev: bool,
    // fix-weapon-fx-levels: the `ammo_data` casing meshes (`Bullet_init_casing` 0x12fde0's mesh
    // argument [M]) — wanted so a foreign-level give can draw the ejected casings.
    casing_meshes: &[u32],
) -> Result<MergeStats> {
    let _ = iso;
    let mut stats = MergeStats::default();
    if *wanted == Wanted::None { return Ok(stats); }
    let want_skins = wanted_skins(level, wd, wanted);
    let want_scripts = wanted_scripts(wd, wanted);
    // `--dev`'s original behaviour: an opened archive lends ALL its missing skins (opening it is
    // the expensive part). A surgical merge lends exactly the wanted skins.
    let lend = |h: u32| dev || want_skins.contains(&h);
    let mut merged: Vec<String> = Vec::new();
    for skin in &want_skins {
        let Some(archive_name) = index.skins.get(skin) else {
            eprintln!("weapon assets: skin {skin:08x} is on no archive of the disc"); continue;
        };
        if merged.contains(archive_name) { continue; }
        merged.push(archive_name.clone());
        let fd = match read(archive_name)? { Some(d) => d, None => continue };
        let Ok(foreign) = Level::parse(&fd) else { continue };
        // The skins the level lacks join the accumulating table ([M AnimLoadFile 0x1d8248]).
        // cross-process-determinism: walked in ascending hashcode, not the HashMap's per-process
        // random order — `new_skins`' order feeds the load pipeline's mesh/lend bookkeeping, and
        // the skins list is documented order-sensitive gameplay data.
        let mut new_skins: Vec<u32> = Vec::new();
        let mut foreign_skins: Vec<(u32, _)> = foreign.skins.clone().into_iter().collect();
        foreign_skins.sort_by_key(|&(h, _)| h);
        for (h, sk) in foreign_skins {
            if level.skins.contains_key(&h) || !lend(h) { continue; }
            level.skins.insert(h, sk);
            new_skins.push(h);
            stats.skins += 1;
        }
        // The foreign maps that hold the merged skins' meshes and the sleeves; the +0x80/+0x4C/
        // +0x5C hashes the level still misses ride along (`take_maps_holding` is first-wins).
        let mut hashes = crate::wanted_weapon_models(level, wd, casing_meshes);
        for h in &new_skins {
            if let Some(sk) = level.skins.get(h) {
                for mh in sk.main.iter().flatten() { hashes.push(*mh); }
                for (_, ph) in sk.extra.iter() { if let Some(x) = ph { hashes.push(*x); } }
            }
        }
        // The foreign skeletons the level lacks.
        let flb = LevelBin::parse(&fd)?;
        for e in flb.entries.iter().filter(|e| e.kind == 6) {
            let sk = character::parse_skeleton(flb.data(&fd, e))?;
            if skeletons.contains_key(&(sk.id as u8)) { continue; }
            skeletons.insert(sk.id as u8, sk);
            stats.skeletons += 1;
        }
        // The merged skins' skeletons mark the sequence buckets the view models look up; the
        // archive's paired 07F anim file appends its sequences for them — EXCEPT the skeletons a
        // drone or character placement animates (the per-skeleton lists are order-sensitive
        // gameplay data: `frame_of`'s `k % len` idle cycle and the skeleton-filtered drone
        // lookups; the loadable-pack path uses a `skeleton | 0x80` sentinel for the same reason).
        // The fp hands/gun skeletons never carry that data, so the view models still get theirs.
        let fp_skeletons: std::collections::HashSet<u8> = new_skins.iter()
            .filter_map(|h| level.skins.get(h)).map(|sk| sk.skeleton)
            .filter(|sk| !guard_skeletons.contains(sk)).collect();
        let anim_name = format!("{}F{}", &archive_name[..2], &archive_name[3..]);
        if let Some(anim) = read(&anim_name)? {
            let alb = LevelBin::parse(&anim)?;
            for pe in alb.entries.iter().filter(|pe| pe.kind == 4 && pe.length >= 0x40) {
                let s = match Sequence::parse(alb.data(&anim, pe)) { Ok(s) => s, Err(_) => continue };
                if fp_skeletons.contains(&s.skeleton) { sequences.entry(s.skeleton).or_default().push(s); stats.sequences += 1; }
            }
        }
        // The foreign kind-5 view scripts — the level's own stay (missing hashes only).
        for e in flb.entries.iter().filter(|e| e.kind == 5) {
            if let Ok(sc) = Script::parse(flb.data(&fd, e)) {
                if !scripts.contains_key(&sc.hashcode) && !have_scripts.contains(&sc.hashcode) {
                    scripts.insert(sc.hashcode, sc); stats.scripts += 1;
                }
            }
        }
        stats.maps += crate::take_maps_holding(level, foreign, &hashes);
        stats.archives += 1;
        eprintln!("weapon assets: merged {archive_name} ({} skins, {} maps so far)", new_skins.len(), stats.maps);
    }
    // The mesh top-up (weapon-model-coverage): a weapon-model hash the level archive misses and no
    // opened archive carried can still sit on an archive no wanted skin points at (weapon 0x63's
    // third-person mesh 0x02000312 lives on 07000004/16 [D census]; the session mesh merge only
    // sweeps EARLIER story archives, so 07000001-03 could never reach it). Pull it from the archive
    // the index names for it — the same session-accumulation mechanism, one map take per archive.
    let mut needed = meshes_for_wanted(level, wd, wanted, casing_meshes);
    needed.retain(|h| !level.by_hash.contains_key(h));
    let mut topup_archives: Vec<String> = Vec::new();
    for h in &needed {
        if let Some(a) = index.objects.get(h) { if !topup_archives.contains(a) { topup_archives.push(a.clone()); } }
    }
    for archive_name in &topup_archives {
        let fd = match read(archive_name)? { Some(d) => d, None => continue };
        let Ok(foreign) = Level::parse(&fd) else { continue };
        let moved = crate::take_maps_holding(level, foreign, &needed);
        if moved > 0 {
            needed.retain(|h| !level.by_hash.contains_key(h));
            stats.maps += moved;
            stats.archives += 1;
            eprintln!("weapon assets: {moved} map(s) of weapon-model meshes topped up from {archive_name}");
        }
    }
    // The top-up pass: a wanted view script no opened archive carried comes from the archive the
    // index names for it (a weapon's scripts can sit in a shared anim pack rather than beside its
    // skin). Parses only that archive's kind-5 entries.
    let missing: Vec<u32> = want_scripts.iter()
        .filter(|h| !scripts.contains_key(h) && !have_scripts.contains(h) && index.scripts.contains_key(h))
        .copied().collect();
    for archive_name in missing.iter().filter_map(|h| index.scripts.get(h)) {
        if merged.contains(archive_name) && stats.scripts == 0 { continue; }
        let fd = match read(archive_name)? { Some(d) => d, None => continue };
        let Ok(flb) = LevelBin::parse(&fd) else { continue };
        let before = stats.scripts;
        for e in flb.entries.iter().filter(|e| e.kind == 5) {
            if let Ok(sc) = Script::parse(flb.data(&fd, e)) {
                if !scripts.contains_key(&sc.hashcode) && !have_scripts.contains(&sc.hashcode) {
                    scripts.insert(sc.hashcode, sc); stats.scripts += 1;
                }
            }
        }
        if stats.scripts > before { eprintln!("weapon assets: {} script(s) topped up from {archive_name}", stats.scripts - before); }
    }
    Ok(stats)
}
