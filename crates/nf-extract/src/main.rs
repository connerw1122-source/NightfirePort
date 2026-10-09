//! `nf-extract`: reads the user's own 007: Nightfire (PS2, NTSC-U) disc image. Nothing it writes is committed.
//!
//! ```text
//! nf-extract info <iso>                    build identity and sectors no directory entry covers
//! nf-extract manifest <iso>                path/lba/size/sha1 of every directory entry (TSV, stdout)
//! nf-extract elfs <iso> <out-dir>          SLUS_205.79, ACTION.ELF and DRIVING.ELF, SHA-1 checked
//! nf-extract files-manifest <iso>          FILES.BIN entries from ACTION.ELF's FileList (TSV, stdout)
//! nf-extract files-extract <iso> <out-dir> unpack FILES.BIN by FileList
//! nf-extract bin-list <iso>                every entry of every level .bin (TSV, stdout; ~44k rows)
//! nf-extract bin-digest <iso>              per-archive digest of bin-list (= golden/level_bins.tsv)
//! nf-extract tex-list <iso>                every map texture with index/palette SHA-1s (TSV, stdout)
//! nf-extract tex-digest <iso>              per-archive digest of tex-list (= golden/textures.tsv)
//! nf-extract tex-export <iso> <out> [pfx]  map textures as indexed PNGs (archives starting with pfx)
//! nf-extract bigf-manifest <iso>           every entry of every BIGF archive (TSV, stdout)
//! nf-extract bigf-extract <iso> <archive> <out-dir>   unpack one BIGF archive (e.g. /DRIVING/MISC.VIV)
//! nf-extract symtab <elf>                  an ELF's .symtab as TSV (stdout)
//! nf-extract drivingsym <iso>              DRIVING/DRIVING.SYM as TSV (stdout)
//! ```

use anyhow::{bail, Context, Result};
use nf_formats::{disc, elf::Elf, iso9660::IsoImage, png, sha1, symfile};
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a = |i: usize| args.get(i).map(String::as_str).context("missing argument (see the module doc)");
    match a(0)? {
        "info" => info(&IsoImage::open(a(1)?)?),
        "manifest" => { print!("{}", disc::manifest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "elfs" => elfs(&IsoImage::open(a(1)?)?, Path::new(a(2)?)),
        "bigf-manifest" => { print!("{}", disc::bigf_manifest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "files-manifest" => { print!("{}", disc::files_bin_manifest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "files-extract" => files_extract(&IsoImage::open(a(1)?)?, Path::new(a(2)?)),
        "bin-list" => { print!("{}", disc::level_bin_entries(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "bin-digest" => { print!("{}", disc::level_bin_digest(&disc::level_bin_entries(&IsoImage::open(a(1)?)?)?)); Ok(()) }
        "tex-list" => { print!("{}", disc::texture_entries(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "tex-digest" => { print!("{}", disc::texture_digest(&disc::texture_entries(&IsoImage::open(a(1)?)?)?)); Ok(()) }
        "tex-export" => tex_export(&IsoImage::open(a(1)?)?, Path::new(a(2)?), args.get(3).map(String::as_str)),
        "gfx-list" => { print!("{}", disc::geometry_entries(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "gfx-digest" => { print!("{}", disc::geometry_digest(&disc::geometry_entries(&IsoImage::open(a(1)?)?)?)); Ok(()) }
        "level-summary" => { print!("{}", disc::level_summary(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "level-export" => level_export(&IsoImage::open(a(1)?)?, a(2)?, Path::new(a(3)?), args.get(4).is_some_and(|x| x == "all")),
        "seq-digest" => { print!("{}", disc::sequence_digest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "skins" => skins(&IsoImage::open(a(1)?)?),
        "seq-roots" => seq_roots(&IsoImage::open(a(1)?)?, a(2)?),
        "scripts" => scripts(&IsoImage::open(a(1)?)?),
        "script-digest" => { print!("{}", disc::script_digest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "drone-digest" => { print!("{}", disc::drone_start_digest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "drones" => drones(&IsoImage::open(a(1)?)?, a(2)?),
        "gfx-skin-words" => gfx_skin_words(&IsoImage::open(a(1)?)?),
        "gfx-boxes" => gfx_boxes(&IsoImage::open(a(1)?)?),
        "gfx-export" => gfx_export(&IsoImage::open(a(1)?)?, Path::new(a(2)?), args.get(3).map(String::as_str)),
        "bigf-extract" => bigf_extract(&IsoImage::open(a(1)?)?, a(2)?, Path::new(a(3)?)),
        "symtab" => symtab(Path::new(a(1)?)),
        "drivingsym" => {
            let iso = IsoImage::open(a(1)?)?;
            let e = iso.find("/DRIVING/DRIVING.SYM").context("no /DRIVING/DRIVING.SYM")?;
            for s in symfile::parse(&iso.read_file(e)?)? { println!("{:08x}\t{}", s.addr, s.name); }
            Ok(())
        }
        other => bail!("unknown command {other:?} (see the module doc)"),
    }
}

fn info(iso: &IsoImage) -> Result<()> {
    println!("volume id      {}", iso.volume_id());
    println!("sectors        {} (PVD says {})", iso.sector_count(), iso.volume_sectors());
    if let Some(cnf) = iso.find("/SYSTEM.CNF") { println!("SYSTEM.CNF     {:?}", String::from_utf8_lossy(&iso.read_file(cnf)?)); }
    let boot = iso.find(disc::BOOT_ELF).context("no SLUS_205.79: not the NTSC-U build this project targets")?;
    let h = disc::hash_extent(iso, boot.lba, boot.size as u64)?;
    println!("boot ELF sha1  {h} ({})", if h == disc::BOOT_ELF_SHA1 { "matches docs/build-identity.md" } else { "DIFFERENT BUILD" });
    for (a, b) in disc::unlisted_ranges(iso) { println!("unlisted       LBA {a}..{b} ({} sectors)", b - a); }
    Ok(())
}

fn elfs(iso: &IsoImage, out: &Path) -> Result<()> {
    std::fs::create_dir_all(out)?;
    let write = |name: &str, bytes: Vec<u8>, want: &str| -> Result<()> {
        let got = sha1::hex(&sha1::sha1(&bytes));
        if got != want { bail!("{name}: sha1 {got}, expected {want} (different build?)"); }
        std::fs::write(out.join(name), bytes)?;
        println!("{name}\t{got}");
        Ok(())
    };
    write("SLUS_205.79", iso.read_file(iso.find(disc::BOOT_ELF).context("no boot ELF")?)?, disc::BOOT_ELF_SHA1)?;
    write("ACTION.ELF", iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF")?)?, disc::ACTION_ELF_SHA1)?;
    write("DRIVING.ELF", iso.read_file(iso.find("/DRIVING.ELF").context("no DRIVING.ELF")?)?, disc::DRIVING_ELF_SHA1)
}

fn bigf_extract(iso: &IsoImage, archive: &str, out: &Path) -> Result<()> {
    let e = iso.find(archive).with_context(|| format!("no {archive} on the disc"))?;
    let g = disc::read_bigf(iso, e)?;
    for x in &g.entries {
        let rel: std::path::PathBuf = x.name.split(['\\', '/']).filter(|c| !c.is_empty() && *c != ".." && *c != ".").collect();
        let path = out.join(rel);
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(&path, iso.read_bytes(e.lba as u64 * 2048 + x.offset as u64, x.size as u64)?)?;
    }
    println!("{} entries -> {}", g.entries.len(), out.display());
    Ok(())
}

fn files_extract(iso: &IsoImage, out: &Path) -> Result<()> {
    std::fs::create_dir_all(out)?;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    for e in &entries {
        if e.name.contains(['/', '\\']) || e.name.starts_with('.') { bail!("unsafe FileList name {:?}", e.name); }
        std::fs::write(out.join(&e.name), iso.read_bytes(bin.lba as u64 * 2048 + e.offset as u64, e.size as u64)?)?;
    }
    println!("{} files -> {}", entries.len(), out.display());
    Ok(())
}

/// Writes `<out>/<archive>/<entry>_<tex>[_f<frame>].png` (indexed PNG; alpha via `round(a·255/128)`), optionally only
/// for archives whose name starts with `only`.
fn tex_export(iso: &IsoImage, out: &Path, only: Option<&str>) -> Result<()> {
    let mut written = 0usize;
    disc::for_each_map(iso, |archive, i, _e, map| {
        if only.is_some_and(|p| !archive.starts_with(p)) { return Ok(()); }
        let mf = nf_formats::map::MapFile::parse(map)?;
        let (texs, _bad) = nf_formats::texture::textures(map, &mf)?;
        let dir = out.join(archive.trim_end_matches(".bin"));
        std::fs::create_dir_all(&dir)?;
        for t in &texs {
            let (k, pal) = (t.index, t.palette_rgba8());
            for f in 0..t.frames as usize {
                let name = if t.frames > 1 { format!("{i:04}_{k:03}_f{f:02}.png") } else { format!("{i:04}_{k:03}.png") };
                std::fs::write(dir.join(name), png::encode_indexed(t.width as u32, t.height as u32, t.frame(f), &pal))?;
                written += 1;
            }
        }
        Ok(())
    })?;
    println!("{written} PNGs -> {}", out.display());
    Ok(())
}

/// Writes `<out>/<archive>/<entry>_<object>.obj` per static PS2_GFX object (positions and UVs printed with Rust's
/// shortest round-trip f32 formatting, so they parse back to the exact bits; `usemtl tex<N>` per batch).
fn gfx_export(iso: &IsoImage, out: &Path, only: Option<&str>) -> Result<()> {
    use std::fmt::Write as _;
    let mut written = 0usize;
    disc::for_each_gfx(iso, |archive, entry, n, hash, name, obj| {
        if only.is_some_and(|p| !archive.starts_with(p)) || obj.leaves.is_empty() { return Ok(()); }
        let dir = out.join(archive.trim_end_matches(".bin"));
        std::fs::create_dir_all(&dir)?;
        let mut s = format!("# {archive} entry {entry} object {n} hashcode {hash:08x} {name}\n");
        let mut base = 1usize;
        for (li, leaf) in obj.leaves.iter().enumerate() {
            for (bi, b) in leaf.batches.iter().enumerate() {
                let _ = writeln!(s, "g leaf{li}_batch{bi}\nusemtl tex{}", b.texture.map_or(-1, |t| t as i64));
                for v in &b.verts { let _ = writeln!(s, "v {} {} {}", v.pos[0], v.pos[1], v.pos[2]); }
                for v in &b.verts { let _ = writeln!(s, "vt {} {}", v.uv[0], v.uv[1]); }
                for t in b.triangles() {
                    let [a, b2, c] = t.map(|i| i as usize + base);
                    let _ = writeln!(s, "f {a}/{a} {b2}/{b2} {c}/{c}");
                }
                base += b.verts.len();
            }
        }
        std::fs::write(dir.join(format!("{entry:04}_{n:03}.obj")), s)?;
        written += 1;
        Ok(())
    })?;
    println!("{written} OBJ files -> {}", out.display());
    Ok(())
}

/// One world-space OBJ of a level archive's main-map placements (`all`: also dynamic entities), plus statistics.
fn level_export(iso: &IsoImage, archive: &str, out: &Path, all: bool) -> Result<()> {
    use nf_formats::{level::Level, placement::transform};
    use std::fmt::Write as _;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).with_context(|| format!("no {archive} in FILES.BIN"))?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;
    let (inst, missing) = level.instances(!all);
    let mut s = String::new();
    let (mut base, mut tris, mut skinned) = (1usize, 0usize, 0usize);
    for i in &inst {
        let obj = &level.maps[i.map].objects[i.object];
        if obj.gfx.is_skinned() { skinned += 1; }
        let _ = writeln!(s, "g {}_{}", level.maps[i.map].name, obj.name.replace(' ', "_"));
        for b in obj.gfx.leaves.iter().flat_map(|l| &l.batches) {
            for v in &b.verts { let p = transform(&i.matrix, v.pos); let _ = writeln!(s, "v {} {} {}", p[0], p[1], p[2]); }
            for t in b.triangles() { let [a, b2, c] = t.map(|k| k as usize + base); let _ = writeln!(s, "f {a} {b2} {c}"); tris += 1; }
            base += b.verts.len();
        }
    }
    std::fs::write(out, s)?;
    let main = level.main_map().map(|m| level.maps[m].name.clone()).unwrap_or_default();
    println!("{archive}: {} maps, main map {main}, {} instances ({} skinned, not drawn), {missing} unresolved, {tris} triangles -> {}",
        level.maps.len(), inst.len(), skinned, out.display());
    Ok(())
}

/// Measurement (WORKLOG 0013): every skin (type 3) parsed with its archive's skeletons; how its mesh hashcodes resolve;
/// which placements carry a skin hashcode in parameter 0 and what their objects are called.
/// Every animation script (type 5) on the disc: parse, command bytes within the entry, and whether each opcode-0
/// sequence exists in the matching animation archive (07F000NN for 070000NN), the archive itself, or nowhere.
fn scripts(iso: &IsoImage) -> Result<()> {
    use nf_formats::{anim::Script, levelbin::LevelBin};
    use std::collections::{BTreeMap, HashSet};
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let mut archives: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for fe in entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".bin")) {
        archives.insert(fe.name.to_ascii_lowercase(), iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?);
    }
    let hashes = |data: &[u8], kind: u8| -> Result<HashSet<u32>> {
        let lb = LevelBin::parse(data)?;
        Ok(lb.entries.iter().filter(|e| e.kind == kind).filter_map(|e| u32::from_str_radix(&e.name, 16).ok()).collect())
    };
    let mut st: BTreeMap<String, u64> = BTreeMap::new();
    for (name, data) in &archives {
        let lb = LevelBin::parse(data)?;
        let own_seq = hashes(data, 4)?;
        let anim_name = format!("{}f{}", &name[..2], &name[3..]);
        let anim_seq = match archives.get(&anim_name) { Some(a) => hashes(a, 4)?, None => HashSet::new() };
        for e in lb.entries.iter().filter(|e| e.kind == 5) {
            let d = lb.data(data, e);
            match Script::parse(d) {
                Ok(s) => {
                    *st.entry("scripts parsed".into()).or_default() += 1;
                    if s.looping { *st.entry("scripts looping".into()).or_default() += 1; }
                    let used = 8 + s.commands.iter().map(|c| 2 * (c.args.len() + 1)).sum::<usize>();
                    *st.entry(if used <= d.len() { "commands within entry" } else { "commands overrun entry" }.into()).or_default() += 1;
                    if s.hashcode != u32::from_str_radix(&e.name, 16).unwrap_or(0) { *st.entry("hash ≠ name".into()).or_default() += 1; }
                    for c in &s.commands { *st.entry(format!("opcode {}", c.op)).or_default() += 1; }
                    for (a, b, q) in s.sequences() {
                        let k = if anim_seq.contains(&q) { "sequence in 07F archive" } else if own_seq.contains(&q) { "sequence in same archive" } else { "sequence not found" };
                        *st.entry(k.into()).or_default() += 1;
                        if a < 1 || b > s.length || a > b { *st.entry("op-0 frame range outside the script".into()).or_default() += 1; }
                    }
                }
                Err(err) => { *st.entry(format!("parse error: {err}")).or_default() += 1; }
            }
        }
    }
    for (k, v) in st { println!("{v}\t{k}"); }
    Ok(())
}

/// Each drone placement of a level: skin, parameters 1/9, `drone_setup`, start danim, script and its sequence.
fn drones(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::{anim::{Script, Sequence}, drone, level::Level, levelbin::LevelBin};
    use std::collections::HashMap;
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF")?)?;
    let tables = drone::DroneAnimData::from_action_elf(&elf)?;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let read = |name: &str| -> Result<Vec<u8>> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).with_context(|| format!("no {name}"))?;
        Ok(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?)
    };
    let data = read(level_name)?;
    let level = Level::parse(&data)?;
    let lb = LevelBin::parse(&data)?;
    let scripts: HashMap<u32, Script> = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
    let anim = read(&format!("{}F{}", &level_name[..2], &level_name[3..]))?;
    let alb = LevelBin::parse(&anim)?;
    let seqs: HashMap<u32, Sequence> = alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40).filter_map(|e| Sequence::parse(alb.data(&anim, e)).ok()).map(|s| (s.hashcode, s)).collect();
    let level_id = u32::from_str_radix(&level_name[..8], 16)?;
    let Some(m) = level.main_map() else { return Ok(()) };
    println!("index\tskin\tparam1\tparam9\tkind\tset\tdanim\tstate\tscript\tscript_found\tlength\tloop\tsequence\tseq_frames");
    for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x0f) {
        let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
        let setup = drone::drone_setup(param(0), param(9), level_id);
        let start = drone::start_script(&tables, param, level_id);
        let (danim, set, custom) = match start {
            drone::StartScript::Table { danim, anim_set, .. } => (danim, anim_set, false),
            drone::StartScript::Custom(_) => (0, setup.anim_set, true),
        };
        let script = start.script();
        let s = script.and_then(|h| scripts.get(&h));
        let q = s.and_then(|s| s.sequences().next());
        println!("{}\t{:08x}\t{:08x}\t{:x}\t{:#x}\t{:#x}\t{:#x}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}", p.index, param(0), param(1), param(9), setup.kind, set, danim,
            tables.state_of(danim), script.map(|h| format!("{h:08x}{}", if custom { " (param 1)" } else { "" })).unwrap_or("-".into()), s.is_some(),
            s.map(|s| s.length).unwrap_or(0), s.map(|s| s.looping).unwrap_or(false),
            q.map(|q| format!("{:08x}", q.2)).unwrap_or("-".into()), q.and_then(|q| seqs.get(&q.2)).map(|x| x.frames.to_string()).unwrap_or("missing".into()));
    }
    Ok(())
}

/// Root-bone translation of every sequence of every skeleton a level's skins use (× the first such skin's scale),
/// at the first and last frame, next to the bind-pose root (−inverse bind) — the inputs to root motion.
fn seq_roots(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::{anim::Sequence, character, level::Level, levelbin::LevelBin};
    use std::collections::BTreeMap;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let read = |name: &str| -> Result<Vec<u8>> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).with_context(|| format!("no {name}"))?;
        Ok(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?)
    };
    let data = read(level_name)?;
    let level = Level::parse(&data)?;
    let lb = LevelBin::parse(&data)?;
    let skels: BTreeMap<u8, character::Skeleton> = lb.entries.iter().filter(|e| e.kind == 6)
        .map(|e| character::parse_skeleton(lb.data(&data, e)).map(|s| (s.id as u8, s))).collect::<std::result::Result<_, _>>()?;
    let anim = read(&format!("{}F{}", &level_name[..2], &level_name[3..]))?;
    let alb = LevelBin::parse(&anim)?;
    let mut skins: Vec<_> = level.skins.values().collect();
    skins.sort_by_key(|s| s.hashcode);
    for skin in skins {
        let Some(sk) = skels.get(&skin.skeleton) else { continue };
        let ib = skin.inverse_bind.first().map(|b| b.0).unwrap_or_default();
        // Lowest point of the bind-pose meshes (glist box minima equal the decoded bounds, U-019).
        let min_y = skin.main.iter().flatten().filter_map(|h| level.by_hash.get(h))
            .flat_map(|&(m, o)| level.maps[m].objects[o].gfx.boxes.iter().map(|b| b.a[1])).fold(f32::INFINITY, f32::min);
        println!("skin {:08x} skel {} scale {:?} bind root {:?} skel offset0×scale {:?} root has translation: {} bind mesh min y {min_y}", skin.hashcode, skin.skeleton,
            skin.scale, ib.map(|v| -v), [0, 1, 2].map(|k| sk.offsets[0][k] * skin.scale[k]), sk.has_translation(0));
    }
    for e in alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40) {
        let s = Sequence::parse(alb.data(&anim, e))?;
        let Some(sk) = skels.get(&s.skeleton) else { continue };
        if !sk.has_translation(0) { continue }
        let at = |f: u32| [0, 1, 2].map(|k| s.value(k, f));
        let mid = at(s.frames as u32 / 2 + 1);
        // Lowest joint relative to the root at frame 1, posed with the first level skin of this skeleton.
        let lowest = level.skins.values().filter(|k| k.skeleton == s.skeleton && s.flags & 8 == 0).min_by_key(|k| k.hashcode).map(|k| {
            let w = nf_formats::anim::world_pose(&nf_formats::anim::local_pose(&s, sk, k, 1), k);
            w.iter().enumerate().filter(|(b, _)| k.animated(*b)).map(|(_, m)| m[3][1]).fold(f32::INFINITY, f32::min) - w[0][3][1]
        });
        println!("{:08x}\tskel {}\tflags {:02x}\tframes {}\tfirst {:?}\tmid {:?}\tlast {:?}\tlowest joint − root (frame 1) {:?}", s.hashcode, s.skeleton, s.flags, s.frames, at(1), mid, at(s.frames as u32), lowest);
    }
    Ok(())
}

fn skins(iso: &IsoImage) -> Result<()> {
    use nf_formats::{character, level::Level, levelbin::LevelBin};
    use std::collections::{BTreeMap, HashMap};
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let mut st: BTreeMap<String, u64> = BTreeMap::new();
    let mut names: BTreeMap<String, u64> = BTreeMap::new();
    for fe in entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".bin")) {
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
        let lb = LevelBin::parse(&data)?;
        let skels: HashMap<u8, u8> = lb.entries.iter().filter(|e| e.kind == 6)
            .map(|e| character::parse_skeleton(lb.data(&data, e))).collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter().map(|s| (s.id as u8, s.bones)).collect();
        let level = Level::parse(&data)?;
        let mut skins = HashMap::new();
        for e in lb.entries.iter().filter(|e| e.kind == 3) {
            match character::parse_skin(lb.data(&data, e), |id| skels.get(&id).copied()) {
                Ok(s) => {
                    *st.entry("skins parsed".into()).or_default() += 1;
                    for h in s.main.iter().flatten() {
                        let k = match level.by_hash.get(h) {
                            Some(&(m, o)) if level.maps[m].objects[o].gfx.is_skinned() => "main mesh → skinned object",
                            Some(_) => "main mesh → static object",
                            None if character::Skin::is_sleeve(*h) => "main mesh → sleeve (substituted)",
                            None => "main mesh → unresolved",
                        };
                        *st.entry(k.into()).or_default() += 1;
                    }
                    skins.insert(s.hashcode, s);
                }
                Err(err) => { *st.entry(format!("skin parse error: {err}")).or_default() += 1; }
            }
        }
        if let Some(m) = level.main_map() {
            for p in &level.maps[m].placements {
                let Some(&(_, v)) = p.params.iter().find(|(i, _)| *i == 0) else { continue };
                if v >> 24 != 5 { continue; }
                let name = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(mm, o)| level.maps[mm].objects[o].name.clone()) } else { Some(level.maps[m].objects[p.index as usize].name.clone()) };
                *names.entry(format!("{} (skin {})", name.unwrap_or_default(), if skins.contains_key(&v) { "found" } else { "missing" })).or_default() += 1;
            }
        }
    }
    for (k, v) in st { println!("{v}\t{k}"); }
    println!("--- placements with a skin hashcode in parameter 0:");
    for (k, v) in names { println!("{v}\t{k}"); }
    Ok(())
}

/// Measurement for skinned batches: for each bit of the packed skinning word (vertex slot B+3+3k, w), how often it
/// is set at strip positions 0–1 vs ≥ 2, and the most common words (WORKLOG 0013).
fn gfx_skin_words(iso: &IsoImage) -> Result<()> {
    let mut first = [0u64; 32];
    let mut rest = [0u64; 32];
    let (mut n_first, mut n_rest) = (0u64, 0u64);
    let mut common: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
    disc::for_each_gfx(iso, |_a, _e, _n, _h, _name, obj| {
        for b in obj.leaves.iter().flat_map(|l| &l.batches).filter(|b| b.skinned) {
            for (i, v) in b.verts.iter().enumerate() {
                let w = v.w as u32;
                *common.entry(w).or_default() += 1;
                let (bits, n) = if i < 2 { (&mut first, &mut n_first) } else { (&mut rest, &mut n_rest) };
                *n += 1;
                for k in 0..32 { if w >> k & 1 != 0 { bits[k] += 1; } }
            }
        }
        Ok(())
    })?;
    println!("bit\tset_at_0_1 (of {n_first})\tset_at_2plus (of {n_rest})");
    for k in 0..32 { if first[k] + rest[k] > 0 { println!("{k}\t{}\t{}", first[k], rest[k]); } }
    let mut c: Vec<_> = common.into_iter().collect();
    c.sort_by(|a, b| b.1.cmp(&a.1));
    for (w, n) in c.iter().take(12) { println!("{w:08x}\t{n}"); }
    Ok(())
}

/// Per static leaf box: its two vectors and the decoded vertices' min/max (UNKNOWNS U-019), TSV on stdout.
fn gfx_boxes(iso: &IsoImage) -> Result<()> {
    println!("archive\tentry\tobject\tbox\ta\tb\tvmin\tvmax");
    disc::for_each_gfx(iso, |archive, entry, n, _hash, _name, obj| {
        for leaf in &obj.leaves {
            let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
            for v in leaf.batches.iter().flat_map(|b| &b.verts) {
                for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); }
            }
            let bx = &obj.boxes[leaf.box_index];
            let f = |v: &[f32; 3]| format!("{} {} {}", v[0], v[1], v[2]);
            println!("{archive}\t{entry}\t{n}\t{}\t{}\t{}\t{}\t{}", leaf.box_index, f(&bx.a), f(&bx.b), f(&lo), f(&hi));
        }
        Ok(())
    })?;
    Ok(())
}

fn symtab(path: &Path) -> Result<()> {
    let data = std::fs::read(path)?;
    let elf = Elf::parse(&data)?;
    println!("value\tsize\tkind\tbind\tshndx\tname");
    for s in elf.symbols(&data)?.iter().skip(1) {
        println!("{:08x}\t{}\t{}\t{}\t{}\t{}", s.value, s.size, s.kind(), s.binding(), s.shndx, s.name);
    }
    Ok(())
}
