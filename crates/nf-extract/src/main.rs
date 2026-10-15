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
//! nf-extract weapon-matrix <iso> [levels]  the weapon matrix (task 100d): every on-foot level ×
//!                                          every weapon — load, give, draw, fire ×3, reload,
//!                                          switch away/back; the mark table + aggregates + red
//!                                          cells. `levels` = comma list (e.g. 07000005,07000011)
//!                                          to run a subset. Needs no NF_ISO (the iso argument).
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
        // `iso-get <iso> <path on disc> <out file>`: one file off the image.
        "iso-get" => { let iso = IsoImage::open(a(1)?)?; let e = iso.find(a(2)?).context("no such file on the disc")?; std::fs::write(a(3)?, iso.read_file(e)?)?; Ok(()) }
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
        "datums" => datums(&IsoImage::open(a(1)?)?, a(2)?),
        "block-dump" => block_dump(&IsoImage::open(a(1)?)?, a(2)?, u8::from_str_radix(a(3)?.trim_start_matches("0x"), 16)?, Path::new(a(4)?)),
        "script-info" => script_info(&IsoImage::open(a(1)?)?, a(2)?, u32::from_str_radix(a(3)?, 16)?),
        "route" => { let f = |i: usize| -> Result<f32> { Ok(a(i)?.parse()?) }; route(&IsoImage::open(a(1)?)?, a(2)?, [f(3)?, f(4)?, f(5)?], [f(6)?, f(7)?, f(8)?]) }
        "covers" => covers(&IsoImage::open(a(1)?)?, a(2)?),
        "triggers" => triggers(&IsoImage::open(a(1)?)?, a(2)?),
        "nis" => nis(&IsoImage::open(a(1)?)?, a(2)?, args.get(3).map(|h| u32::from_str_radix(h, 16)).transpose()?),
        "placements" => placements(&IsoImage::open(a(1)?)?, a(2)?, u32::from_str_radix(a(3)?, 16)?),
        "sfx" => sfx(&IsoImage::open(a(1)?)?, &args[2..]),
        "coll-probe" => coll_probe(&IsoImage::open(a(1)?)?, a(2)?),
        "coll-check" => coll_check(&IsoImage::open(a(1)?)?),
        "coll-floor" => coll_floor(&IsoImage::open(a(1)?)?, a(2)?),
        "drone-props" => drone_props(&IsoImage::open(a(1)?)?, a(2)?),
        "ai-sim" => ai_sim(&IsoImage::open(a(1)?)?, a(2)?, [a(3)?.parse()?, a(4)?.parse()?, a(5)?.parse()?], args.get(6).map(|s| s.parse()).transpose()?.unwrap_or(600)),
        "seq-roots" => seq_roots(&IsoImage::open(a(1)?)?, a(2)?),
        "scripts" => scripts(&IsoImage::open(a(1)?)?),
        "pose-check" => pose_check(&IsoImage::open(a(1)?)?, a(2)?, a(3)?),
        "weapons" => weapons(&IsoImage::open(a(1)?)?, args.get(2).map(String::as_str)),
        "weapon-matrix" => {
            let levels: Option<Vec<String>> = args.get(2).map(|l| l.split(',').map(|s| {
                let s = s.trim();
                if s.contains('.') { s.to_string() } else { format!("{s}.bin") }
            }).collect());
            let mut quiet = |s: &str| eprintln!("{s}");
            let rows = nf_view::matrix::run(a(1)?, levels.as_deref(), &mut quiet)?;
            print!("{}", nf_view::matrix::mark_table(&rows));
            print!("\n{}", nf_view::matrix::aggregate_table(&rows));
            let red = nf_view::matrix::red_cells(&rows);
            if !red.is_empty() { print!("\nred cells (mark level weapon reason, the numbers behind them):\n{red}"); }
            Ok(())
        }
        "script-digest" => { print!("{}", disc::script_digest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "ai-path-digest" => { print!("{}", disc::ai_path_digest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "drone-digest" => { print!("{}", disc::drone_start_digest(&IsoImage::open(a(1)?)?)?); Ok(()) }
        "drones" => drones(&IsoImage::open(a(1)?)?, a(2)?),
        "gfx-skin-words" => gfx_skin_words(&IsoImage::open(a(1)?)?),
        "gfx-boxes" => gfx_boxes(&IsoImage::open(a(1)?)?),
        "gfx-export" => gfx_export(&IsoImage::open(a(1)?)?, Path::new(a(2)?), args.get(3).map(String::as_str)),
        "bigf-extract" => bigf_extract(&IsoImage::open(a(1)?)?, a(2)?, Path::new(a(3)?)),
        "crp-mesh" => crp_mesh(a(1)?, a(2)?, Path::new(a(3)?)),
        "crp-mesh-digest" => crp_mesh_digest(a(1)?),
        "symtab" => symtab(Path::new(a(1)?)),
        "drivingsym" => {
            let iso = IsoImage::open(a(1)?)?;
            let e = iso.find("/DRIVING/DRIVING.SYM").context("no /DRIVING/DRIVING.SYM")?;
            for s in symfile::parse(&iso.read_file(e)?)? { println!("{:08x}\t{}", s.addr, s.name); }
            Ok(())
        }
        "level-survey" => level_survey(&IsoImage::open(a(1)?)?),
        "level-mesh-audit" => level_mesh_audit(&IsoImage::open(a(1)?)?, a(2)?),
        "kind-census" => kind_census(&IsoImage::open(a(1)?)?),
        "char-textures" => char_textures(&IsoImage::open(a(1)?)?, a(2)?),
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

/// `crp-mesh <iso-or-DRIVING-dir> <viv> <out-dir>` — the driving track `.crp`'s mesh layer (task
/// drive-track-mesh): decompresses the archive's `data\track\*.crp`, parses the CARP pool,
/// relocates the embedded ELFd+ELFr module and writes per-article TSVs (`<track>-articles.tsv`:
/// name/bbox/parts/sr refs; `<track>-module.tsv`: sections/symbols/relocations/collision census).
/// Prints one summary row. The first argument is either a disc image (`<iso>`) or the extracted
/// `DRIVING/` folder (the slim test image ships without it; the tests use the folder form).
fn crp_mesh(source: &str, viv: &str, out: &Path) -> Result<()> {
    let (entry_name, row) = crp_mesh_row(source, viv, Some(out))?;
    println!("{row}");
    println!("# wrote TSVs for {entry_name} -> {}", out.display());
    Ok(())
}

/// `crp-mesh-digest <iso-or-DRIVING-dir>` — the canonical per-track digest rows (numbers only) for
/// all 8 mission tracks: the golden form (golden/crp_mesh_golden.tsv).
fn crp_mesh_digest(source: &str) -> Result<()> {
    for viv in MISSION_VIVS {
        println!("{}", crp_mesh_row(source, viv, None)?.1);
    }
    Ok(())
}

const MISSION_VIVS: [&str; 8] =
    ["MIS01.VIV", "MIS11.VIV", "MIS13A.VIV", "MIS13B.VIV", "MIS13C.VIV", "MIS3.VIV", "MIS4.VIV", "RACE.VIV"];

/// One archive's bytes: either from the disc image or straight from the DRIVING folder.
fn driving_archive_bytes(source: &str, viv: &str) -> Result<Vec<u8>> {
    let p = std::path::Path::new(source);
    if p.is_dir() {
        return Ok(std::fs::read(p.join(viv)).with_context(|| format!("reading {}", p.join(viv).display()))?);
    }
    let iso = IsoImage::open(p)?;
    let path = format!("/DRIVING/{viv}");
    let e = iso.find(&path).with_context(|| format!("no {path} on the disc"))?;
    Ok(iso.read_file(e)?)
}

/// Loads one mission VIV's track `.crp`, computes the mesh summary; with `out` set, writes the
/// per-article and module TSVs. Returns `(entry name, summary row)`.
fn crp_mesh_row(source: &str, viv: &str, out: Option<&Path>) -> Result<(String, String)> {
    use nf_formats::crp::Crp as CrpFile;
    use nf_formats::crp_mesh::TrackModule;

    let data = driving_archive_bytes(source, viv)?;
    let g = nf_formats::bigf::Bigf::parse(&data, data.len() as u64)?;
    let (idx, entry) = g.entries.iter().enumerate()
        .find(|(_, x)| {
            let n = x.name.to_ascii_lowercase();
            n.starts_with("data\\track\\") && n.ends_with(".crp")
        })
        .with_context(|| format!("{viv}: no data\\track\\*.crp entry"))?;
    let idx = idx;
    let entry_name = entry.name.clone();
    let blob = data[entry.offset as usize..(entry.offset + entry.size) as usize].to_vec();
    let dec = if nf_formats::refpack::is_refpack(&blob) {
        nf_formats::refpack::decompress(&blob)?
    } else {
        blob
    };
    let crp = CrpFile::parse(&dec)?;
    let track = crp.track_name()?.unwrap_or("-").to_owned();
    let articles = crp.track_articles()?;
    let module = TrackModule::load(&crp)?;
    let coll = crp.collision_census()?;
    let model_symbols = module.symbols.iter().filter(|s| s.name.starts_with("__Model:::")).count();
    let geoprims = module.geoprim_names().len();
    let parts: usize = articles.iter().map(|a| a.parts.len()).sum();
    let image_sha1: String = nf_formats::sha1::sha1(&module.image).iter().map(|b| format!("{b:02x}")).collect();
    let row = format!(
        "{viv}#{idx}\t{entry_name}\t{track}\t{}\t{parts}\t{model_symbols}\t{geoprims}\t{}\t{}\t{}\t{image_sha1}",
        articles.len(), module.relocations, coll.ci_total_items, coll.cn_records,
    );
    if let Some(out) = out {
        std::fs::create_dir_all(out)?;
        let stem = track.replace(['/', '\\'], "_");
        let mut art = String::from("name\tbbox_min\tbbox_max\tparts\n");
        for a in &articles {
            let fmt = |v: &[f32; 3]| format!("{:.4},{:.4},{:.4}", v[0], v[1], v[2]);
            let refs = if a.parts.is_empty() {
                "-".to_owned()
            } else {
                a.parts.iter()
                    .map(|p| match p.sr_index {
                        Some(sr) => format!("as#{}->sr#{}", p.as_instance, sr),
                        None => format!("as#{}->-", p.as_instance),
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            };
            art.push_str(&format!("{}\t{}\t{}\t{}\n", a.name, fmt(&a.bbox_min), fmt(&a.bbox_max), refs));
        }
        std::fs::write(out.join(format!("{stem}-articles.tsv")), art)?;
        let mut mods = String::from("section\ttype\toffset\tsize\tentsize\n");
        for s in &module.sections {
            mods.push_str(&format!("{}\t{:#x}\t{:#x}\t{}\t{:#x}\n", s.name, s.sh_type, s.offset, s.size, s.entsize));
        }
        mods.push_str(&format!("# symbols={}\n", module.symbols.len()));
        mods.push_str(&format!("# relocations={} (R_MIPS_32)\n", module.relocations));
        mods.push_str(&format!("# collision: grd={} ci={} ci_items={} cn={}\n", coll.grd_records, coll.ci_records, coll.ci_total_items, coll.cn_records));
        std::fs::write(out.join(format!("{stem}-module.tsv")), mods)?;
    }
    Ok((entry_name, row))
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
/// Compares our posing with the game's, from an EE RAM snapshot (`tools/trace/snapshot.py`): for each drone in the
/// list at 0x298e14 that plays exactly one sequence, the game's local pose buffer (`sAnimObject+0x3C`: translations
/// then quaternions per bone) and world palette (`sAnimObject+0x48`, 0x40 per bone) against `anim::local_pose` (root
/// translation zeroed, as `AnimSeqTick`) and `world_pose` under the game's object matrix (`obj+0x90`).
fn pose_check(iso: &IsoImage, level_name: &str, snapshot: &str) -> Result<()> {
    use nf_formats::{anim::{self, Sequence}, character, level::Level, levelbin::LevelBin};
    use std::collections::HashMap;
    let ram = std::fs::read(snapshot)?;
    let u32_at = |a: u32| -> u32 { let o = (a & 0x01ff_ffff) as usize; u32::from_le_bytes(ram[o..o + 4].try_into().unwrap()) };
    let f32_at = |a: u32| f32::from_bits(u32_at(a));
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let read = |name: &str| -> Result<Vec<u8>> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).with_context(|| format!("no {name}"))?;
        Ok(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?)
    };
    let data = read(level_name)?;
    let level = Level::parse(&data)?;
    let lb = LevelBin::parse(&data)?;
    let skels: HashMap<u8, character::Skeleton> = lb.entries.iter().filter(|e| e.kind == 6)
        .map(|e| character::parse_skeleton(lb.data(&data, e)).map(|s| (s.id as u8, s))).collect::<std::result::Result<_, _>>()?;
    let anim_data = read(&format!("{}F{}", &level_name[..2], &level_name[3..]))?;
    let alb = LevelBin::parse(&anim_data)?;
    let seqs: HashMap<u32, Sequence> = alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40)
        .filter_map(|e| Sequence::parse(alb.data(&anim_data, e)).ok()).map(|s| (s.hashcode, s)).collect();
    println!("level in snapshot: {:08x}", u32_at(0x2a3768 + 12));
    let (mut worst_local, mut worst_world) = (0f32, 0f32);
    let mut d = u32_at(0x298e14);
    let mut n = 0;
    while d != 0 && n < 128 {
        n += 1;
        let (drone, obj) = (d, u32_at(d + 0xc));
        d = u32_at(d + 4);
        let anim_ptr = u32_at(obj + 0xdc);
        let sa = anim_ptr + 0x70;
        let skin_hash = u32_at(u32_at(anim_ptr + 0x94));
        let mut scripts = Vec::new();
        let mut sc = u32_at(anim_ptr + 0x9c);
        while sc != 0 && scripts.len() < 8 {
            let seq = u32_at(sc + 0x50);
            if seq != 0 && u32_at(seq + 0x94) != 0 { scripts.push((u32_at(sc + 0x74), u32_at(seq + 0x94), f32_at(seq + 0x9c))); }
            sc = u32_at(sc + 0x48);
        }
        let label = format!("drone {drone:08x} skin {skin_hash:08x}");
        let [(script, seq_hash, frame)] = scripts[..] else { println!("{label}: {} playing sequences — skipped (blend)", scripts.len()); continue };
        let (Some(skin), Some(seq)) = (level.skins.get(&skin_hash), seqs.get(&seq_hash)) else { println!("{label}: skin or sequence {seq_hash:08x} not loaded"); continue };
        let Some(skel) = skels.get(&skin.skeleton) else { continue };
        if frame.fract() != 0.0 { println!("{label}: fractional frame {frame} — skipped"); continue; }
        let nb = skel.bones as usize;
        // Game local pose: translations (bones × 16 bytes) then quaternions.
        let buf = u32_at(sa + 0x3c);
        let mut report = String::new();
        let mut local = Vec::new();
        let mut dl = 0f32;
        for arith in [anim::Arith::Nearest, anim::Arith::Ps2] {
            let mut l = anim::local_pose_with(seq, skel, skin, frame as u32, arith);
            l[0].0 = [0.0; 3];
            let (mut worst, mut exact, mut total) = (0f32, 0, 0);
            let mut off_by = [0u32; 7]; // tx ty tz qx qy qz qw
            for (b, (t, q)) in l.iter().enumerate().filter(|(b, _)| skin.animated(*b)) {
                let vals = (0..3).map(|k| (f32_at(buf + 16 * b as u32 + 4 * k as u32), t[k]))
                    .chain((0..4).map(|k| (f32_at(buf + 16 * (nb + b) as u32 + 4 * k as u32), q[k])));
                for (i, (g, o)) in vals.enumerate() {
                    total += 1;
                    if g.to_bits() == o.to_bits() { exact += 1 } else { off_by[i] += 1 }
                    worst = worst.max((g - o).abs());
                }
            }
            report.push_str(&format!(" {arith:?}: max {worst:.2e}, {exact}/{total} bit-exact (differing tx ty tz qx qy qz qw: {off_by:?});"));
            if arith == anim::Arith::Ps2 { local = l; dl = worst; }
        }
        // World: ours × object matrix vs the palette.
        let objm: anim::Mat4 = std::array::from_fn(|r| std::array::from_fn(|c| f32_at(obj + 0x90 + 16 * r as u32 + 4 * c as u32)));
        let world = anim::world_pose(&local, skin);
        let pal = u32_at(sa + 0x48);
        let mut dw = 0f32;
        let mut bad = Vec::new();
        for (b, m) in world.iter().enumerate().filter(|(b, _)| skin.animated(*b)) {
            let ours = anim::mul(m, &objm);
            let mut db = 0f32;
            for r in 0..4 { for c in 0..3 { db = db.max((f32_at(pal + 0x40 * b as u32 + 16 * r as u32 + 4 * c as u32) - ours[r][c]).abs()); } }
            if db > 1e-5 { bad.push(format!("{b}:{db:.1e}")); }
            dw = dw.max(db);
        }
        let flags = u32_at(sa + 0x58);
        report.push_str(&format!(" sAnimObject+0x58 {flags:#x} (0x30 head track, 0x180 aim); bones > 1e-5: [{}];", bad.join(" ")));
        worst_local = worst_local.max(dl);
        worst_world = worst_world.max(dw);
        println!("{label}: script {script:08x} sequence {seq_hash:08x} frame {frame}: local{report} world (Ps2 local) max {dw:.2e}");
    }
    println!("worst local {worst_local:.2e}, worst world {worst_world:.2e}");
    Ok(())
}

/// A level's skins with their datums (id, bone, translation), and whether each weapon's third-person mesh resolves.
fn datums(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::{level::Level, weapon::WeaponData};
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF")?)?;
    let wd = WeaponData::from_action_elf(&elf)?;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;
    let mut skins: Vec<_> = level.skins.values().collect();
    skins.sort_by_key(|s| s.hashcode);
    for s in skins {
        let d: Vec<String> = s.datums.iter().map(|d| format!("{}@{}({:.2},{:.2},{:.2})", d.id, d.bone, d.t[0], d.t[1], d.t[2])).collect();
        println!("skin {:08x}: {}", s.hashcode, d.join(" "));
    }
    for w in [0x16u8, 0x24] {
        if let Some((mesh, datum)) = wd.third_person(w) {
            println!("weapon {w:#x}: mesh {mesh:08x} {} datum {datum}", if level.by_hash.contains_key(&mesh) { "found" } else { "NOT in by_hash" });
        }
    }
    Ok(())
}

/// Checks the collision decode on every 0x2E block of the disc: tree indices, leaf vertices inside their box, plane
/// normals against the triangles' winding, and which neighbouring object's render bounds the collision matches.
fn coll_check(iso: &IsoImage) -> Result<()> {
    use nf_formats::{collision::Collision, map::MapFile, ps2gfx::GfxObject};
    use std::collections::BTreeMap;
    let mut st: BTreeMap<String, u64> = BTreeMap::new();
    let mut add = |k: &str, n: u64| *st.entry(k.to_string()).or_default() += n;
    disc::for_each_map(iso, |_archive, _entry, _e, map| {
        let mf = MapFile::parse(map)?;
        // Render bounds of each PS2_GFX in order, and for each collision block the gfx index before/after it.
        let mut gfx_bounds: Vec<Option<([f32; 3], [f32; 3])>> = Vec::new();
        let mut colls: Vec<(usize, Collision)> = Vec::new();
        for blk in &mf.blocks {
            match blk.id {
                0x2d => {
                    let raw = &map[blk.offset as usize..][..blk.size as usize];
                    let bb = GfxObject::parse(raw).ok().and_then(|g| g.boxes.first().map(|b| (b.a, b.b)));
                    gfx_bounds.push(bb);
                }
                0x2e => match Collision::parse(blk.payload(map)) {
                    Ok(c) => colls.push((gfx_bounds.len(), c)),
                    Err(_) => add("parse error", 1),
                },
                _ => {}
            }
        }
        for (before, c) in &colls {
            add("blocks", 1);
            for &m in &c.materials { add(&format!("material {:02} (& 0x3f {})", m, m & 0x3f), 1); }
            add(&format!("version {}", c.version), 1);
            let n = c.boxes.len() as u16;
            if c.boxes.iter().any(|b| b.children.is_some_and(|(l, r)| l >= n || r >= n)) { add("child index out of range", 1); }
            let leaf_tris: usize = c.boxes.iter().map(|b| b.tris.len()).sum();
            add(if leaf_tris == c.tris.len() { "leaves cover all triangles exactly" } else { "leaf triangle count ≠ total" }, 1);
            for bx in c.boxes.iter().filter(|b| b.children.is_none()) {
                let base = bx.vblock as usize * 0x20;
                for t in &c.tris[bx.tris.start as usize..bx.tris.end.min(c.tris.len() as u16) as usize] {
                    let v = t.v.map(|i| { let w = |k: usize| c.words.get(base + i as usize + k).copied().unwrap_or(0) as f32;
                        [w(0) * bx.scale + bx.origin[0], w(1) * bx.scale + bx.origin[1], w(2) * bx.scale + bx.origin[2]] });
                    add("triangles", 1);
                    let inside = v.iter().all(|p| (0..3).all(|k| p[k] >= bx.min[k] - 0.02 && p[k] <= bx.max[k] + 0.02));
                    add(if inside { "vertices inside leaf box" } else { "vertex outside leaf box" }, 1);
                    let pw = |k: usize| c.words.get(base + t.plane as usize + k).copied().unwrap_or(0) as f32 / 16384.0;
                    let nrm = [pw(0), pw(1), pw(2)];
                    let (e1, e2) = ([v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]], [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]]);
                    let cr = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
                    let len = (cr[0] * cr[0] + cr[1] * cr[1] + cr[2] * cr[2]).sqrt();
                    if len < 1e-9 { add("degenerate triangle", 1); continue; }
                    let dot = (cr[0] * nrm[0] + cr[1] * nrm[1] + cr[2] * nrm[2]) / len;
                    let nlen = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt();
                    add(if (nlen - 1.0).abs() < 0.01 { "plane normal unit length" } else { "plane normal not unit" }, 1);
                    add(if dot > 0.99 { "normal = +cross(v1−v0, v2−v0)" } else if dot < -0.99 { "normal = −cross" } else { "normal disagrees with winding" }, 1);
                }
            }
            // Pairing: overlap of collision bounds with the object before / after.
            let (lo, hi) = c.boxes.first().map(|b| (b.min, b.max)).unwrap_or(([0.0; 3], [0.0; 3]));
            let close = |g: Option<&Option<([f32; 3], [f32; 3])>>| g.and_then(|x| *x).is_some_and(|(a, b)| (0..3).all(|k| (a[k] - lo[k]).abs() < 0.5 && (b[k] - hi[k]).abs() < 0.5));
            add(&format!("bounds match object before: {}", close(before.checked_sub(1).and_then(|i| gfx_bounds.get(i)))), 1);
            add(&format!("bounds match object after: {}", close(gfx_bounds.get(*before))), 1);
        }
        Ok(())
    })?;
    for (k, v) in st { println!("{v}\t{k}"); }
    Ok(())
}

/// Headless AI run: the level's drones (those present at start) against Bond standing still at a point, for
/// comparison with `tools/trace/ai_trace.py`. Prints state changes and shots of the drone nearest to Bond.
fn ai_sim(iso: &IsoImage, level_name: &str, bond: [f32; 3], frames: u32) -> Result<()> {
    use nf_formats::{anim::Script, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
    use nf_game::drone::{Data, Drones, Player, Rng};
    use std::collections::HashMap;
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF")?)?;
    let tables = drone::DroneAnimData::from_action_elf(&elf)?;
    // NF_CANDO=4c,40: per anim state its own danim, transition list and whether each anim set has a script for it.
    for x in std::env::var("NF_CANDO").unwrap_or_default().split(',').filter_map(|v| u16::from_str_radix(v.trim(), 16).ok()) {
        if let Some((own, list)) = tables.states.get(x as usize) {
            let sets: Vec<u16> = (0..12).filter(|&set| tables.script_for(*own, set).is_some()).collect();
            println!("state {x:#x}: own danim {own} sets with a script {sets:?}; transitions {:?}", list.iter().map(|t| (t.key, t.count, t.danim, t.b4, t.b5)).collect::<Vec<_>>());
        }
    }
    // NF_SCRIPTS=64-79: which anim sets have a script for each of those danims.
    if let Some((a0, a1)) = std::env::var("NF_SCRIPTS").ok().and_then(|v| { let (a, b) = v.split_once('-')?; Some((a.parse::<u16>().ok()?, b.parse::<u16>().ok()?)) }) {
        for x in a0..=a1 {
            let sets: Vec<u16> = (0..27).filter(|&set| tables.tables.get(x as usize).and_then(|r| r.get(set as usize)).is_some_and(|&v| v != 0)).collect();
            let info = tables.info.get(x as usize).copied().unwrap_or([0; 24]);
            println!("danim {x}: state {:#x} class {} follow-on {} own-set scripts {sets:?}", u16::from_le_bytes([info[8], info[9]]), info[12], u16::from_le_bytes([info[10], info[11]]));
        }
    }
    // NF_DANIM=265,244: print those danims' info rows (Drone_AnimInfo, 0x18 bytes) and their state's transition list.
    for x in std::env::var("NF_DANIM").unwrap_or_default().split(',').filter_map(|v| v.trim().parse::<usize>().ok()) {
        if let Some(r) = tables.info.get(x) { println!("danim {x}: info {:02x?}", r); }
    }
    let props = drone::PropertyTables::from_action_elf(&elf)?;
    let weapons = WeaponData::from_action_elf(&elf)?;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;
    let lb = LevelBin::parse(&data)?;
    let scripts: HashMap<u32, Script> = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
    let (cels, _) = level.instances(true);
    let world = nf_game::world::CollisionWorld::build(&level, &cels);
    let level_id = u32::from_str_radix(&level_name[..8], 16)?;
    let sequences = disc::level_sequences(iso, level_name)?;
    let skeletons: HashMap<u8, nf_formats::character::Skeleton> = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
    let m = level.main_map().context("no main map")?;
    let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap_or(&Default::default()), &world, level_id);
    let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
    let mut tuning = nf_formats::tuning::Tuning::from_action_elf(&elf)?;
    nf_game::tuning::read_tuning_vars(&mut tuning, level_id);
    let hitboxes = nf_formats::hitbox::CollisionBones::from_action_elf(&elf)?;
    let volumes = nf_game::aivolume::build(&level, level_id);
    let sfx_table = nf_formats::sfx::SfxTable::from_action_elf(&elf)?;
    let models = nf_game::pickup::models(&level);
    // Headless trace: no surface/voice sound tables (their paths draw no RNG — WORKLOG 0069 —
    // so the shared stream is unaffected; the speech/footstep sounds simply do not play).
    let d = Data { tables: &tables, props: &props, scripts: &scripts, weapons: &weapons, world: &world, level: level_id, sequences: &sequences, skins: &level.skins, skeletons: &skeletons, nav: &nav, covers: &covers, tuning: &tuning, hitboxes: &hitboxes, volumes: &volumes, sfx: Some(&sfx_table), models: &models, surfaces: None, voices: None, difficulty: 1 };
    let mut rng = Rng::new(std::env::var("NF_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(0x1234_5678));
    let mut lights = nf_game::light::LightList::default();
    let drones = nf_game::drone::level_drones(&d, &level, &mut rng, &mut lights);
    let mut sim = Drones::new(drones, rng);
    sim.attach_level(&level);
    // NF_SWITCHES=22,a4: switch channels on from the start (hex).
    for c in std::env::var("NF_SWITCHES").unwrap_or_default().split(',').filter_map(|c| u8::from_str_radix(c.trim(), 16).ok()) { sim.set_channel(c); }
    // NF_AIM_YAW: Bond's facing (radians; the game's obj+0x54), else −x.
    let aim = std::env::var("NF_AIM_YAW").ok().and_then(|v| v.parse::<f32>().ok()).map_or([-1.0, 0.0, 0.0], |y| [y.sin(), 0.0, y.cos()]);
    // The standing capsule per the player's stance (mirrors scene.rs's Player construction; the
    // trigger contact classification reads it [M Player_Collision 0x194df0]). Standing cc [M
    // player.rs ROOT_HEIGHT] — this tracer's sim carries no stance state.
    let capsule = ([bond[0], bond[1] + 0.55, bond[2]], [bond[0], bond[1] + 0.275 - nf_game::player::ROOT_HEIGHT, bond[2]]);
    let player = Player { pos: bond, target: [bond[0], bond[1] + 0.5, bond[2]], eye: [bond[0], bond[1] + 1.0, bond[2]], aim, sph: bond, hanging: false, capsule, cel: None };
    let near = (0..sim.list.len()).min_by(|&a, &b| {
        let da = |i: usize| { let p = sim.list[i].pos; (p[0] - bond[0]).powi(2) + (p[2] - bond[2]).powi(2) };
        da(a).total_cmp(&da(b))
    }).context("no drones")?;
    println!("{} drones; tracing drone {near} at {:?}", sim.list.len(), sim.list[near].pos);
    let mut last_danim = 0;
    let (mut hits, mut misses) = (Vec::new(), 0);
    // Per drone: path length and ticks without horizontal motion (patrol checks against live traces).
    let mut walked = vec![(0f32, 0u32); sim.list.len()];
    let mut steps_out = std::env::var_os("NF_STEPS").map(std::fs::File::create).transpose()?;
    // NF_SOUND=x,y,z,id,frame[;x,y,z,id,frame…]: sounds (SFXOutputData id, hex) made at those frames.
    let sounds: Vec<(f32, f32, f32, u32, u32)> = std::env::var("NF_SOUND").unwrap_or_default().split(';').filter_map(|v| {
        let p: Vec<&str> = v.split(',').collect();
        Some((p.first()?.parse().ok()?, p.get(1)?.parse().ok()?, p.get(2)?.parse().ok()?, u32::from_str_radix(p.get(3)?, 16).ok()?, p.get(4)?.parse().ok()?))
    }).collect();
    for _ in 0..frames {
        for &(x, y, z, id, at) in &sounds { if sim.frame + 1 == at { sim.play_sound([x, y, z], sfx_table.get(id)); } }
        // NF_SHOOT=f1,f2,…: Bond fires his P2K (weapon 2) at those frames from his eye at the traced drone's chest
        // (feet + NF_SHOOT_Y, default 1.3); NF_AIMING=1 holds aim mode.
        if std::env::var("NF_SHOOT").unwrap_or_default().split(',').filter_map(|v| v.trim().parse::<u32>().ok()).any(|f| f == sim.frame + 1) {
            let t = sim.list[near].pos;
            let y = std::env::var("NF_SHOOT_Y").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.3);
            let to = [t[0], t[1] - 1.0 + y, t[2]];
            let e = player.eye;
            let l = ((to[0] - e[0]).powi(2) + (to[1] - e[1]).powi(2) + (to[2] - e[2]).powi(2)).sqrt();
            sim.bond_aiming = std::env::var_os("NF_AIMING").is_some();
            sim.fire_player(&d, e, [(to[0] - e[0]) / l, (to[1] - e[1]) / l, (to[2] - e[2]) / l], 2, sfx_table.get(0x403));
        }
        let before: Vec<[f32; 3]> = sim.list.iter().map(|x| x.pos).collect();
        sim.tick(&d, &player);
        for ev in &sim.player_impacts {
            if let nf_game::drone::PlayerImpact::Drone { drone, part, .. } = ev {
                let x = &sim.list[*drone];
                println!("frame {:4}: Bond's bullet hit drone {drone} part {part}: health {:.3} state {:#x}", sim.frame, x.health, x.state);
            } else { println!("frame {:4}: Bond's bullet hit the world {:?}", sim.frame, ev); }
        }
        for (i, x) in sim.list.iter().enumerate() {
            let step = ((x.pos[0] - before[i][0]).powi(2) + (x.pos[2] - before[i][2]).powi(2)).sqrt();
            walked[i].0 += step;
            if step < 1e-6 && x.route.is_some() { walked[i].1 += 1 }
            if i == std::env::var("NF_STEPS_DRONE").ok().and_then(|v| v.parse().ok()).unwrap_or(1) {
                if let Some(f) = steps_out.as_mut() { use std::io::Write; writeln!(f, "{}\t{step}\t{}\t{}\t{}", sim.frame, x.anim.time, x.pos[0], x.pos[2])?; }
            }
        }
        for ev in &sim.impacts { if let nf_game::bullet::BulletEvent::Player { damage, .. } = ev { hits.push((sim.frame, *damage)); } else { misses += 1; } }
        if sim.frame % 100 == 0 {
            for (i, x) in sim.list.iter().enumerate().filter(|(_, x)| x.route.is_some()) {
                let r = x.route.as_ref().unwrap();
                println!("frame {:4}: patrol drone {i} state {:#x} at ({:.2}, {:.2}, {:.2}) route node {} (index {}) anim {:#x} danim {} script {:08x} t {} drawn {}", sim.frame, x.state, x.pos[0], x.pos[1], x.pos[2], r.nodes.get(r.index.max(0) as usize).copied().unwrap_or(0), r.index, x.anim.state, x.anim.danim, x.anim.script.unwrap_or(0), x.anim.time, x.drawn);
            }
        }
        let dr = &sim.list[near];
        if std::env::var_os("NF_TRACE_TICKS").is_some() {
            println!("tick {:4}: state {:#x} pos ({:.3}, {:.3}, {:.3}) anim {:#x} danim {} t {} call {:x?} reached {} from {:#x} blend {}", sim.frame, dr.state, dr.pos[0], dr.pos[1], dr.pos[2], dr.anim.state, dr.anim.danim, dr.anim.time, dr.anim.call, dr.anim.reached, dr.anim.call_from, dr.anim.blend_wait);
        }
        if sim.frame % 30 == 0 && !matches!(dr.state, 4 | 5 | 6 | 7) {
            let r = dr.croute.as_ref();
            println!("frame {:4}: state {:#x} at ({:.2}, {:.2}, {:.2}) yaw {:.2} off {:.2} dist {:.1} anim {:#x} route {:?} target {:?} seen {}", sim.frame, dr.state, dr.pos[0], dr.pos[1], dr.pos[2], dr.yaw, dr.yaw_offset, dr.dist, dr.anim.state,
                r.map(|r| (r.nodes.len(), r.index)), r.map(|r| r.target.map(|v| (v * 100.0).round() / 100.0)), dr.seen);
        }
        if dr.anim.danim != last_danim {
            last_danim = dr.anim.danim;
            println!("frame {:4}: danim {} script {:08x} anim state {:#x} class {}", sim.frame, dr.anim.danim, dr.anim.script.unwrap_or(0), dr.anim.state, dr.anim.class);
        }
    }
    if std::env::var_os("NF_STATES").is_some() {
        for x in &sim.list { println!("placement {:3} at ({:.1}, {:.1}) present {} state {:#x} set {} skin {:08x} health {} death channel {:#x} log {:x?}", x.placement, x.pos[0], x.pos[2], x.present, x.state, x.anim_set, x.skin, x.health, x.death_channel, &x.log[..x.log.len().min(16)]); }
    }
    for (i, _) in sim.list.iter().enumerate().filter(|(_, x)| x.route.is_some()) { println!("patrol drone {i}: walked {:.3} units in {frames} ticks, {} ticks without motion", walked[i].0, walked[i].1); }
    let dr = &sim.list[near];
    println!("state changes: {:?}", dr.log.iter().map(|(f, s)| format!("{f}:{s:#x}")).collect::<Vec<_>>());
    println!("shots: {:?}", dr.shots);
    println!("clip {} alert {} seen_ticks {} yaw {:.3} → to opp {:.3}", dr.clip, dr.alert, dr.seen_ticks, dr.yaw, dr.yaw_to_opp);
    println!("bullets: {} hit Bond {:?}, {misses} hit the world; Bond health {:.2}", hits.len(), hits.iter().take(12).collect::<Vec<_>>(), sim.health.health);
    for (i, x) in sim.list.iter().enumerate() { if let Some((f, _)) = x.log.iter().find(|(_, s)| *s == nf_game::drone::state::ATTACK) { println!("drone {i} at ({:.1}, {:.1}) attacked at tick {f}{}", x.pos[0], x.pos[2], if x.alerted_by_drone { " (alerted by another drone)" } else { "" }); } }
    Ok(())
}

/// The main map's placements of one type (flags word, hex): translation, rotation, the raw words +0x24..+0x47, the
/// parameters, and the referenced object's glist boxes.
fn placements(iso: &IsoImage, level_name: &str, kind: u32) -> Result<()> {
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = nf_formats::level::Level::parse(&data)?;
    let main = level.main_map().context("no main map")?;
    for m in 0..level.maps.len() {
    if m != main && std::env::var_os("NF_ALL_MAPS").is_none() { continue; }
    for (i, p) in level.maps[m].placements.iter().enumerate().filter(|(_, p)| p.flags == kind) {
        if m != main { print!("map {m} "); }
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((m, p.index as usize)) };
        let obj = target.and_then(|(om, oi)| level.maps[om].objects.get(oi));
        println!("placement {i}: t {:?} r {:?} extra {:08x?} scale {:?} params {:?} object {:?}", p.translation, p.rotation, p.extra, p.scale(), p.params, obj.map(|o| &o.name));
        if let Some(o) = obj {
            for (k, bx) in o.gfx.boxes.iter().enumerate() { println!("    box {k}: a {:?} b {:?}", bx.a, bx.b); }
            let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
            for v in o.gfx.leaves.iter().flat_map(|l| &l.batches).flat_map(|b| &b.verts) { for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); } }
            println!("    gfx verts {lo:?}..{hi:?}; flags {:#x}; collision {:?}", o.flags, o.collision.as_ref().map(|c| c.triangles().len()));
        }
    }
    }
    Ok(())
}

/// Per-archive census of main-map placement kinds (models-misc): `archive \t kind \t count \t resolved \t unresolved \t
/// with_gfx \t with_collision`. `resolved` = the record's object resolves (by hash or index); `with_gfx` = the object
/// has at least one non-empty batch (a visible mesh). The companion to `level_summary` (which counts only totals).
fn kind_census(iso: &IsoImage) -> Result<()> {
    let (entries, bin) = disc::files_bin_entries(iso)?;
    println!("archive\tkind\tcount\tresolved\tunresolved\twith_gfx\twith_collision");
    for fe in entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".bin")) {
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
        let level = nf_formats::level::Level::parse(&data)?;
        let Some(main) = level.main_map() else { continue };
        let mut kinds: std::collections::BTreeMap<u8, (u32, u32, u32, u32, u32)> = Default::default();
        for p in &level.maps[main].placements {
            let e = kinds.entry((p.flags & 0xff) as u8).or_default();
            e.0 += 1;
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < level.maps[main].objects.len() { Some((main, p.index as usize)) } else { None };
            let Some((om, oi)) = target else { e.2 += 1; continue };
            e.1 += 1;
            let o = &level.maps[om].objects[oi];
            if o.gfx.leaves.iter().any(|l| !l.batches.is_empty()) { e.3 += 1; }
            if o.collision.is_some() { e.4 += 1; }
        }
        for (k, (c, r, u, g, col)) in kinds { println!("{}\t0x{:02x}\t{c}\t{r}\t{u}\t{g}\t{col}", fe.name, k); }
    }
    Ok(())
}

/// Behaviour properties of each drone placement (normal and alert blocks), decoded with the game's own tables.
fn drone_props(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::{drone, level::Level};
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF")?)?;
    let t = drone::PropertyTables::from_action_elf(&elf)?;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;
    let Some(m) = level.main_map() else { return Ok(()) };
    for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x0f) {
        let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
        let (nw, aw) = drone::behaviour_words(param);
        let set = |w: &[u32; 3]| (0..0x5b).filter(|&id| id != 0x20 && id != 0x30 && id != 0x44 && t.get(id, w) != 0)
            .map(|id| format!("{id:x}")).collect::<Vec<_>>().join(",");
        println!("({:.1}, {:.1}, {:.1}) k5 {:#x} ({}) k15 {} k16 {} | normal: alert {} [{}] | alert block: alert {} [{}]",
            p.translation[0], p.translation[1], p.translation[2], param(5), if param(5) < 0x24 { "OLD" } else { "NEW" }, param(15), param(16),
            drone::ALERTNESS_LEVELS[t.get(0x20, &nw) as usize & 3], set(&nw), drone::ALERTNESS_LEVELS[t.get(0x20, &aw) as usize & 3], set(&aw));
    }
    Ok(())
}

/// Collision floor under every drone placement of a level (nf-game's collision world built from cel instances).
fn coll_floor(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::level::Level;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;
    let (cels, _) = level.instances(true);
    let t = std::time::Instant::now();
    let world = nf_game::world::CollisionWorld::build(&level, &cels);
    println!("{} collision triangles from {} cel instances ({:?})", world.triangle_count(), cels.len(), t.elapsed());
    let Some(m) = level.main_map() else { return Ok(()) };
    for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x0f) {
        let f = world.floor_below(p.translation, 3.0);
        println!("drone ({:.3}, {:.3}, {:.3}): collision floor {:?} → placement − floor {:?}", p.translation[0], p.translation[1], p.translation[2], f, f.map(|f| p.translation[1] - f));
    }
    Ok(())
}

/// Raw look at the collision blocks (0x2E) of a level's maps: header, first boxes, first triangle records.
fn coll_probe(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::{buf::Buf, levelbin::LevelBin, map::MapFile};
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let lb = LevelBin::parse(&data)?;
    let mut shown = 0;
    for e in lb.entries.iter().filter(|e| matches!(e.kind, 0 | 1 | 2 | 11)) {
        let Ok(map) = MapFile::parse(lb.data(&data, e)) else { continue };
        let raw = lb.data(&data, e);
        if shown == 0 {
            let seq: Vec<String> = map.blocks.iter().filter(|b| !matches!(b.id, 0x16 | 0x12)).take(60).map(|b| format!("{:02x}", b.id)).collect();
            println!("block order of {}: {}", e.name, seq.join(" "));
        }
        for blk in map.all(0x2e) {
            let p = blk.payload(raw);
            let b = Buf(p);
            let (ver, nbox, ntri, nvb) = (b.u32(0)?, b.u16(4)?, b.u16(6)?, b.u16(8)?);
            let base = if ver == 5 { 0x1c } else { 0xc };
            let boxes = base + nvb as usize * 0x40;
            let tris = boxes + nbox as usize * 0x30;
            let rest = tris + ntri as usize * 8;
            println!("map {} kind {} block size {:#x}: ver {ver} boxes {nbox} tris {ntri} vblocks {nvb} (×0x40 = {:#x}); boxes@{boxes:#x} tris@{tris:#x} end@{rest:#x} payload {:#x}",
                e.name, e.kind, blk.size, nvb as usize * 0x40, p.len());
            if shown < 2 {
                for k in 0..nbox.min(6) as usize {
                    let o = boxes + 0x30 * k;
                    let f = |i: usize| b.f32(o + 4 * i).unwrap_or(f32::NAN);
                    println!("  box {k}: min ({:.2} {:.2} {:.2}) c0 {} c1 {} | max ({:.2} {:.2} {:.2}) +1c {} +1e {} | +20 ({:.4} {:.4} {:.4} {:.6}) +2c {:#x}",
                        f(0), f(1), f(2), b.i16(o + 0xc)?, b.i16(o + 0xe)?, f(4), f(5), f(6), b.i16(o + 0x1c)?, b.u16(o + 0x1e)?, f(8), f(9), f(10), f(11), b.u32(o + 0x2c)?);
                }
                for k in 0..ntri.min(4) as usize {
                    let o = tris + 8 * k;
                    println!("  tri {k}: {} {} {} plane {} ", b.u16(o)?, b.u16(o + 2)?, b.u16(o + 4)?, b.u16(o + 6)?);
                }
                println!("  vertex block 0 s16: {:?}", (0..24).map(|i| b.i16(base + 2 * i).unwrap_or(0)).collect::<Vec<_>>());
                shown += 1;
            }
        }
    }
    Ok(())
}

/// `weapon_data` built by running `_GLOBAL_$I$weapon_data` over ACTION.ELF; with a live dump (PCSX2 memory at
/// 0x2bf150), the differing bytes per record.
fn weapons(iso: &IsoImage, live: Option<&str>) -> Result<()> {
    use nf_formats::weapon::{WeaponData, RECORD};
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF")?)?;
    let t = std::time::Instant::now();
    let wd = WeaponData::from_action_elf(&elf)?;
    println!("{} records built in {:?}; SHA-1 {}", wd.records.len(), t.elapsed(), nf_formats::sha1::hex(&nf_formats::sha1::sha1(&wd.bytes())));
    for (w, _) in wd.records.iter().enumerate() {
        if let Some((mesh, datum)) = wd.third_person(w as u8) { println!("weapon {w:#04x}\tthird-person {mesh:08x}\tdatum {datum}"); }
    }
    if let Some(path) = live {
        let live = std::fs::read(path)?;
        let ours = wd.bytes();
        let mut total = 0;
        for (k, (a, b)) in ours.chunks(RECORD).zip(live.chunks(RECORD)).enumerate() {
            let diff: Vec<usize> = (0..RECORD.min(b.len())).filter(|&i| a[i] != b[i]).collect();
            if !diff.is_empty() { total += diff.len(); println!("record {k:#04x}: {} bytes differ at {:x?}", diff.len(), &diff[..diff.len().min(12)]); }
        }
        println!("live comparison: {total} differing bytes of {}", ours.len().min(live.len()));
    }
    Ok(())
}

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
    for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0xf1) {
        println!("spawner at ({:.3}, {:.3}, {:.3}) params {:?}", p.translation[0], p.translation[1], p.translation[2],
            p.params.iter().map(|(i, v)| format!("{i}={v:x}")).collect::<Vec<_>>());
    }
    println!("params (position)\tskin\tparam1\tparam9\tkind\tset\tdanim\tstate\tscript\tscript_found\tlength\tloop\tsequence\tseq_frames");
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
        println!("{}\t{:08x}\t{:08x}\t{:x}\t{:#x}\t{:#x}\t{:#x}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}", format!("{:?} ({:.3}, {:.3}, {:.3})", p.params.iter().map(|(i, v)| format!("{i}={v:x}")).collect::<Vec<_>>(), p.translation[0], p.translation[1], p.translation[2]), param(0), param(1), param(9), setup.kind, set, danim,
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
                    let nb = skels.get(&s.skeleton).copied().unwrap_or(0) as i32;
                    for dt in &s.datums {
                        *st.entry("datums".into()).or_default() += 1;
                        if dt.bone < -1 || dt.bone >= nb { *st.entry("datum bone out of range".into()).or_default() += 1; }
                        let n2: f32 = dt.q.iter().map(|v| v * v).sum();
                        if (n2 - 1.0).abs() > 1e-3 { *st.entry("datum quaternion not unit".into()).or_default() += 1; }
                        *st.entry(format!("datum id {}", dt.id)).or_default() += 1;
                    }
                    // Datums are the last field: their end padded to 32 must be the entry size.
                    let raw = lb.data(&data, e);
                    let skinned = if s.main.is_empty() { 0 } else { 0x1c * s.bones.len() };
                    let mut end = 0x15 + s.bones.len();
                    if !s.extra.is_empty() { end = (end + s.extra.len()).next_multiple_of(4) + 4 * s.extra.len(); }
                    if !s.main.is_empty() { end = end.next_multiple_of(4) + 4 * s.main.len(); }
                    end += skinned + 36 * s.datums.len();
                    *st.entry(if end.next_multiple_of(32) == raw.len() { "datums end at the padded entry end" } else { "datum end ≠ entry end" }.into()).or_default() += 1;
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

/// Dump one map block's payload (every occurrence, main map first) to `out_dir/<level>_<id>_<n>.bin` for study.
fn block_dump(iso: &IsoImage, level_name: &str, id: u8, out_dir: &Path) -> Result<()> {
    use nf_formats::{levelbin::LevelBin, map::MapFile};
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let lb = LevelBin::parse(&data)?;
    std::fs::create_dir_all(out_dir)?;
    let mut n = 0;
    for e in lb.entries.iter().filter(|e| matches!(e.kind, 0 | 1 | 2 | 11)) {
        let Ok(map) = MapFile::parse(lb.data(&data, e)) else { continue };
        for blk in map.all(id) {
            let p = blk.payload(lb.data(&data, e));
            let out = out_dir.join(format!("{}_{id:02x}_{n}.bin", level_name.trim_end_matches(".bin")));
            std::fs::write(&out, p)?;
            println!("{} kind {} -> {} ({} bytes)", e.name, e.kind, out.display(), p.len());
            n += 1;
        }
    }
    Ok(())
}

/// One script's commands, with each op-0 sequence's frame count and root translation at its ends.
/// Level scripts (type-7 entries, docs/formats/nis-script.md): header, streams (command bytes in hex) and, for one
/// script, its keys.
fn nis(iso: &IsoImage, level_name: &str, only: Option<u32>) -> Result<()> {
    use nf_formats::{levelbin::LevelBin, nis::Nis};
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let lb = LevelBin::parse(&data)?;
    for e in lb.entries.iter().filter(|e| e.kind == 7) {
        let h = u32::from_str_radix(&e.name, 16).unwrap_or(0);
        if only.is_some_and(|o| o != h) { continue; }
        match Nis::parse(h, lb.data(&data, e)) {
            Err(err) => println!("{h:08x}: {err}"),
            Ok(s) => {
                println!("{h:08x}: camera {} loop {} length {} streams {} keys {}", s.camera, s.loop_start, s.length, s.streams.len(), s.keys.len());
                for (i, st) in s.streams.iter().enumerate() {
                    println!("  stream {i}: keys {}..={} ({} bytes) {:x?}", st.first_key, st.last_key, st.commands.len(), st.decode());
                }
                if only.is_some() {
                    for (k, key) in s.keys.iter().enumerate() { println!("  key {k}: t {} pos {:?} q {:?} s {} w24 {:02x?} w28 {}", key.time, key.pos, key.quat, key.scale, key.w24, key.w28); }
                }
            }
        }
    }
    Ok(())
}

fn script_info(iso: &IsoImage, level_name: &str, hash: u32) -> Result<()> {
    use nf_formats::{anim::Script, levelbin::LevelBin};
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let lb = LevelBin::parse(&data)?;
    let s = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).find(|s| s.hashcode == hash).context("no such script")?;
    println!("script {hash:08x}: length {} looping {}", s.length, s.looping);
    for c in &s.commands { println!("  op {} args {:x?}", c.op, c.args); }
    let seqs = disc::level_sequences(iso, level_name)?;
    for (a, b, q) in s.sequences() {
        if let Some(sq) = seqs.get(&q) {
            let r = |f: u32| [sq.value(0, f), sq.value(1, f), sq.value(2, f)];
            println!("  frames {a}..{b} → sequence {q:08x}: {} frames, root first {:?} at {} {:?} last {:?}", sq.frames, r(1), b - a + 1, r((b - a + 1) as u32), r(sq.frames as u32));
            let path: f32 = (1..sq.frames as u32).map(|f| { let (u, v) = (r(f), r(f + 1)); ((v[0] - u[0]).powi(2) + (v[2] - u[2]).powi(2)).sqrt() }).sum();
            let wrap = { let (u, v) = (r(sq.frames as u32), r(1)); ((v[0] - u[0]).powi(2) + (v[2] - u[2]).powi(2)).sqrt() };
            println!("    horizontal root path over the frames {path:.4}, last → first {wrap:.4}");
        }
    }
    Ok(())
}

/// Route from one point to another through the level's AI network as a drone would (`NDrone2_MoveToObject`).
fn route(iso: &IsoImage, level_name: &str, from: [f32; 3], to: [f32; 3]) -> Result<()> {
    use nf_formats::level::Level;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;
    let m = level.main_map().context("no main map")?;
    let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
    let level_id = u32::from_str_radix(&level_name[..8], 16)?;
    let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().context("no AI network")?, &world, level_id);
    println!("{} paths, {} walls ({} passable, {} door, {} kick)", nav.paths.len(), nav.walls.len(),
        nav.walls.iter().filter(|w| w.flags & 0x400 != 0).count(), nav.walls.iter().filter(|w| w.flags & 0x800 != 0).count(), nav.walls.iter().filter(|w| w.flags & 0x1000 != 0).count());
    let path = nav.nav_path_for(from);
    println!("navigation path for the start: {:?}", path.map(|p| (p, &nav.paths[p].name)));
    println!("direct move test: {:?}", nav.move_test(&world, from, to, true, false));
    println!("  collision LOS at +1: {}, bounds test (near): {:?}, bounds (far): {:?}", world.line_of_sight([from[0], from[1] + 1.0, from[2]], [to[0], to[1] + 1.0, to[2]]),
        nav.bounds_test(from, to, true, false), nav.bounds_test(from, to, false, false));
    for (pi, p) in nav.paths.iter().enumerate().filter(|(_, p)| p.kind & 0xd == 0) {
        println!("  path {pi} {:?}: target nodes {:?}", p.name, nav.target_nodes(&world, pi, to, 36.0));
        let mut near: Vec<(f32, usize)> = p.nodes.iter().enumerate().map(|(i, n)| (nf_game::nav::dist2d(n.pos, to), i)).collect();
        near.sort_by(|x, y| x.0.total_cmp(&y.0));
        for &(dd, i) in near.iter().take(3) { let q = p.nodes[i].pos; println!("      near node {i}: ({:.2}, {:.2}, {:.2}) 2D {dd:.2}, move test node → target {:?}", q[0], q[1], q[2], nav.move_test(&world, q, to, false, false)); }
    }
    let Some(pi) = path else { return Ok(()) };
    // Posts near the straight line (2D distance < 0.6) and the post test at r 0.2 / 0.4.
    for w in &nav.walls {
        for q in [w.a, w.b] {
            let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
            let t = (((q[0] - from[0]) * dx + (q[2] - from[2]) * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
            let dd = ((from[0] + dx * t - q[0]).powi(2) + (from[2] + dz * t - q[2]).powi(2)).sqrt();
            if dd < 0.6 { println!("  post ({:.2}, {:.2}, {:.2}) h {:.2} at 2D {dd:.3} from the line (t {t:.2})", q[0], q[1], q[2], w.ha); }
        }
    }
    println!("  post test r 0.2: {}, r 0.4: {}", nav.bounds_node_test(0.2, from, to), nav.bounds_node_test(0.4, from, to));
    let goal = nav.target_nodes(&world, pi, to, 36.0);
    match nf_game::nav::Route::to_position(&nav, &world, Some(pi), from, to, &goal, &[]) {
        Ok((res, r)) => {
            println!("route result {res}: {} nodes {:?}", r.nodes.len(), r.nodes);
            for &n in &r.nodes { let q = nav.paths[pi].nodes[n as usize].pos; println!("    node {n}: ({:.2}, {:.2}, {:.2})", q[0], q[1], q[2]); }
        }
        Err(code) => println!("no route: code {code}"),
    }
    Ok(())
}

/// Cover nodes of a level (placements 0xE5 / 0xE6) with their emitter path and reachable node count.
fn covers(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::level::Level;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;
    let m = level.main_map().context("no main map")?;
    let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
    let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap_or(&Default::default()), &world, u32::from_str_radix(&level_name[..8], 16)?);
    for (i, c) in nf_game::cover::build(&level.maps[m].placements, &nav, &world).iter().enumerate() {
        println!("cover {i}: kind {} pos ({:.3}, {:.3}, {:.3}) yaw {:.4} flags {:#x} angle {:.3} depth {:.1} offset {:.1} on {:#x} off {:#x} path {:?} reachable {}/{}",
            c.kind, c.pos[0], c.pos[1], c.pos[2], c.yaw, c.flags, c.angle, c.depth, c.offset, c.on, c.off, c.path, c.dist.iter().filter(|&&x| x != 0xff).count(), c.dist.len());
    }
    Ok(())
}

/// Trigger objects of a level's main map (placements 0xE8, 0xEA–0xEF, 0xDC).
fn triggers(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::level::Level;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let level = Level::parse(&data)?;
    for t in nf_game::triggers::build(&level) {
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for tri in &t.tris { for v in tri { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } } }
        println!("placement {:3}: type {:#x} out {:#04x} mask {:#x} list {:x?} lock {:#x} hide {} tris {} box [{:.2}, {:.2}, {:.2}]-[{:.2}, {:.2}, {:.2}]",
            t.placement, t.kind, t.out, t.mask, t.list, t.lock, t.hide, t.tris.len(), lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
    }
    Ok(())
}

/// `SFXOutputData` hearing entries (inner / outer radius, weight, looped), optionally only the given ids (hex).
fn sfx(iso: &IsoImage, ids: &[String]) -> Result<()> {
    let elf = iso.read_file(iso.find("/ACTION.ELF").context("no ACTION.ELF")?)?;
    let t = nf_formats::sfx::SfxTable::from_action_elf(&elf)?;
    println!("{} entries", t.entries.len());
    let pick: Vec<usize> = if ids.is_empty() { (0..t.entries.len()).collect() } else { ids.iter().filter_map(|x| usize::from_str_radix(x.trim_start_matches("0x"), 16).ok()).collect() };
    for i in pick { if let Some(e) = t.entries.get(i) { println!("{i:#05x}: inner {} outer {} weight {} looped {}", e.inner, e.outer, e.weight, e.looped); } }
    Ok(())
}

/// `level-survey`: loads every FILES.BIN level via the same `Level::parse` path nf-view uses, and reports — for each
/// level — the maps, objects, placements, drones, collision, scripts, skins and textures counts, the placement-type
/// histogram, the object-name histogram, and any parse error. Output is TSV on stdout. Used to write docs/systems/levels.md [M].
fn level_survey(iso: &IsoImage) -> Result<()> {
    use nf_formats::level::Level;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    println!("level\tname\tsize\tmaps\tmain\tobjects\tplacements\tdrones\tcollision\tscripts\tskins\ttextures\tplacement_types\tobject_names\tparse_ok\tparse_error");
    for fe in &entries {
        let name = &fe.name;
        if !name.ends_with(".bin") { continue; }
        // The on-foot levels: 0700xxxx (07 = level hashcode prefix); packs/animations are 07F0/0780/0790/07A0.
        if !name.starts_with("0700") { continue; }
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
        let lvl_id = u32::from_str_radix(&name[..8], 16).unwrap_or(0);
        let res = Level::parse(&data);
        let (ok, err) = match &res { Ok(_) => ("ok", String::new()), Err(e) => ("ERR", format!("{e}")) };
        let (n_maps, main, n_obj, n_place, n_coll, n_scripts, n_skins, n_tex, place_types, obj_names) = match &res {
            Ok(lvl) => {
                let main = lvl.main_map();
                let (n_obj, n_place, n_coll, n_scripts, n_tex, place_types, obj_names) = match main {
                    Some(m) => {
                        let map = &lvl.maps[m];
                        let mut pt = std::collections::BTreeMap::<u32, usize>::new();
                        let mut on = std::collections::BTreeMap::<String, usize>::new();
                        for p in &map.placements { *pt.entry(p.flags & 0xff).or_default() += 1; }
                        for o in &map.objects { *on.entry(o.name.trim().to_string()).or_default() += 1; }
                        (map.objects.len(), map.placements.len(),
                         map.objects.iter().filter(|o| o.collision.is_some()).count(),
                         map.placements.iter().filter(|p| p.flags == 0x07).count(),
                         map.textures.len(), pt, on)
                    }
                    None => (0, 0, 0, 0, 0, std::collections::BTreeMap::new(), std::collections::BTreeMap::new()),
                };
                (lvl.maps.len(), main.is_some(), n_obj, n_place, n_coll, n_scripts, lvl.skins.len(), n_tex, place_types, obj_names)
            }
            Err(_) => (0, false, 0, 0, 0, 0, 0, 0, std::collections::BTreeMap::new(), std::collections::BTreeMap::new()),
        };
        let drones = match &res { Ok(lvl) => main.then(|| lvl.maps.iter().flat_map(|m| m.placements.iter().filter(|p| p.flags & 0xff == 0x0f)).count()).unwrap_or(0), Err(_) => 0 };
        let pt: Vec<String> = place_types.iter().map(|(k, v)| format!("{k:#04x}:{v}")).collect();
        let on: Vec<String> = obj_names.iter().map(|(k, v)| format!("{k}:{v}")).collect();
        println!("{lvl_id:08x}\t{name}\t{}\t{n_maps}\t{}\t{n_obj}\t{n_place}\t{drones}\t{n_coll}\t{n_scripts}\t{n_skins}\t{n_tex}\t{}\t{}\t{ok}\t{err}",
            fe.size, if main { "yes" } else { "no" }, pt.join(","), on.join(","));
    }
    Ok(())
}

/// `level-mesh-audit <iso> <level.bin>`: reproduces `Scene::build`'s instance-skip logic in `nf-view/src/scene.rs`
/// and reports per-instance what survives and what is dropped, plus per-object mesh/flag stats. TSV on stdout.
/// Used to compare 07000001 vs 07000005 ("missing wall/roof meshes" investigation).
fn level_mesh_audit(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::level::Level;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level_name)).context("no such level")?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?;
    let lvl = Level::parse(&data)?;
    let m = lvl.main_map().context("no main map")?;
    let map = &lvl.maps[m];

    // Section 1: every main-map placement, its object, its draw decision.
    println!("section\tplacement_idx\tflags\ttype\tcreates_cel\tby_hash\tindex\tobject_name\thashcode\tglist_flags\tleaf_count\tnonempty_leaves\tbatch_count\ttotal_verts\tskip_reason");
    let (instances, _missing) = lvl.instances(true);
    let mut kept = 0usize; let mut skipped_cel = 0usize; let mut skipped_dynamic = 0usize;
    let mut skipped_empty_mesh = 0usize; let mut unresolved = 0usize;
    for (pi, p) in map.placements.iter().enumerate() {
        let ptype = p.flags & 0xff;
        let creates_cel = p.creates_cel();
        let target = if p.by_hash() { lvl.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
        let (obj_name, hashcode, glist_flags, leaf_count, nonempty_leaves, batch_count, total_verts) = match target {
            Some((om, oi)) => {
                let o = &lvl.maps[om].objects[oi];
                let leaves = &o.gfx.leaves;
                let nonempty = leaves.iter().filter(|l| !l.batches.is_empty()).count();
                let batches = leaves.iter().map(|l| l.batches.len()).sum::<usize>();
                let verts = leaves.iter().flat_map(|l| &l.batches).map(|b| b.verts.len()).sum::<usize>();
                (o.name.trim().to_string(), o.hashcode, o.flags, leaves.len(), nonempty, batches, verts)
            }
            None => ("<unresolved>".into(), 0, 0, 0, 0, 0, 0),
        };
        let skip_reason = if target.is_none() { unresolved += 1; "unresolved".to_string() }
            else if !creates_cel { "not_cel (dynamic-only placement)".to_string() }
            else {
                let o = &lvl.maps[target.unwrap().0].objects[target.unwrap().1];
                if o.skipped_by_cel_pass() {
                    skipped_cel += 1;
                    if matches!(ptype, 0xdb | 0xde | 0xfc) {
                        "cel_pass_skip_but_dynamic".to_string()
                    } else {
                        format!("cel_pass_skip (glist flag 0x10) glist_flags={:#x}", o.flags)
                    }
                } else if matches!(ptype, 0xdb) { skipped_dynamic += 1; "dynamic: door 0xdb".to_string() }
                else if matches!(ptype, 0xde) { skipped_dynamic += 1; "dynamic: searchlight 0xde".to_string() }
                else if matches!(ptype, 0xfc) { skipped_dynamic += 1; "dynamic: corona 0xfc".to_string() }
                else if matches!(ptype, 0x2a) { skipped_dynamic += 1; "dynamic: sky 0x2a".to_string() }
                else if obj_name != "<unresolved>" && batch_count == 0 {
                    skipped_empty_mesh += 1; format!("empty_mesh (leaves={leaf_count} nonempty=0 batches=0) glist_flags={:#x}", o.flags)
                } else { kept += 1; "kept".to_string() }
            };
        let creates_cel_str = if creates_cel { "yes" } else { "no" };
        let by_hash_str = if p.by_hash() { "yes" } else { "no" };
        let flags_hex = format!("{:#010x}", p.flags);
        let ptype_hex = format!("{:#04x}", ptype);
        println!("1\t{pi}\t{flags_hex}\t{ptype_hex}\t{creates_cel_str}\t{by_hash_str}\t{}\t{obj_name}\t{hashcode:#010x}\t{glist_flags:#x}\t{leaf_count}\t{nonempty_leaves}\t{batch_count}\t{total_verts}\t{skip_reason}",
            p.index);
    }
    eprintln!("---");
    eprintln!("placements: {}", map.placements.len());
    eprintln!("  kept (World pass):           {kept}");
    eprintln!("  skipped by cel pass (0x10):  {skipped_cel}");
    eprintln!("  skipped dynamic (0xdb/0xde/0xfc/0x2a): {skipped_dynamic}");
    eprintln!("  skipped empty mesh:          {skipped_empty_mesh}");
    eprintln!("  unresolved (no target obj):  {unresolved}");
    eprintln!("  not creates_cel (dynamic only): {}", map.placements.iter().filter(|p| !p.creates_cel()).count());
    eprintln!("  scene.rs build instances(true) returned: {}", instances.len());

    // Section 2: per-object histogram for the main map — every object's leaf/batch/flag stats.
    println!("section\tobject_idx\tobject_name\thashcode\tglist_flags\tleaf_count\tnonempty_leaves\tbatch_count\ttotal_verts\thas_collision");
    let mut empty_objects = 0usize;
    let mut skip_flag_objects = 0usize;
    let mut alpha_list_objects = 0usize;
    for (oi, o) in map.objects.iter().enumerate() {
        let leaves = &o.gfx.leaves;
        let nonempty = leaves.iter().filter(|l| !l.batches.is_empty()).count();
        let batches = leaves.iter().map(|l| l.batches.len()).sum::<usize>();
        let verts = leaves.iter().flat_map(|l| &l.batches).map(|b| b.verts.len()).sum::<usize>();
        if batches == 0 { empty_objects += 1; }
        if o.skipped_by_cel_pass() { skip_flag_objects += 1; }
        if o.flags & 1 != 0 { alpha_list_objects += 1; }
        let coll = if o.collision.is_some() { "yes" } else { "no" };
        println!("2\t{oi}\t{}\t{:#010x}\t{:#x}\t{}\t{nonempty}\t{batches}\t{verts}\t{coll}",
            o.name.trim(), o.hashcode, o.flags, leaves.len());
    }
    eprintln!("---");
    eprintln!("main-map objects: {}", map.objects.len());
    eprintln!("  objects with empty leaves (no batches): {empty_objects}");
    eprintln!("  objects with glist flag 0x10 (skipped_by_cel_pass): {skip_flag_objects}");
    eprintln!("  objects with glist flag 0x01 (alpha list): {alpha_list_objects}");

    // Section 3: which map-block IDs appear in the main map, and which fall through `_ => {}` in level.rs:124-142.
    // We have to re-parse the map data to get the block list (LevelMap drops it).
    let lb = nf_formats::levelbin::LevelBin::parse(&data)?;
    let me = &lb.entries[m];
    let map_data = lb.data(&data, me);
    let mf = nf_formats::map::MapFile::parse(map_data)?;
    println!("section\tblock_id\tblock_offset\tblock_size\thandled_in_level_rs");
    let mut block_id_counts: std::collections::BTreeMap<u8, usize> = Default::default();
    for blk in &mf.blocks {
        *block_id_counts.entry(blk.id).or_default() += 1;
        let handled = match blk.id {
            0x2e | nf_formats::map::id::PS2_GFX | nf_formats::map::id::ENTITY_PARAMS
                | nf_formats::map::id::MAP_DATA_STATIC | 0x05 | 0x28 | 0x19 => "yes",
            _ => "no (falls through to _ => {})",
        };
        println!("3\t{:#04x}\t{}\t{}\t{handled}", blk.id, blk.offset, blk.size);
    }
    eprintln!("---");
    eprintln!("main-map block IDs (count): ");
    for (id, n) in &block_id_counts {
        let handled = match *id {
            0x2e | nf_formats::map::id::PS2_GFX | nf_formats::map::id::ENTITY_PARAMS
                | nf_formats::map::id::MAP_DATA_STATIC | 0x05 | 0x28 | 0x19 => "handled",
            _ => "DROPPED",
        };
        eprintln!("  {:#04x}: {}  [{handled}]", id, n);
    }
    Ok(())
}

/// `char-textures <iso> <level.bin>`: per character placement (skin hashcode in parameter 0) of a level, dumps every
/// main mesh / rigid part / first-person sleeve and resolves each batch's texture index against the map the mesh was
/// loaded from. Reveals out-of-range texture indices (the white-fallback symptom) and missing skins / meshes.
fn char_textures(iso: &IsoImage, level_name: &str) -> Result<()> {
    use nf_formats::level::Level;
    use std::collections::HashMap;
    let (entries, bin) = disc::files_bin_entries(iso)?;
    let read = |name: &str| -> Result<Vec<u8>> {
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).with_context(|| format!("no {name}"))?;
        Ok(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64)?)
    };
    let data = read(level_name)?;
    let level = Level::parse(&data)?;
    let Some(m) = level.main_map() else { bail!("no main map"); };

    // Reverse-index map entries by hashcode → list of (map, object) so we can see ALL maps that declare a hashcode.
    let mut by_hash_all: HashMap<u32, Vec<(usize, usize)>> = HashMap::new();
    for (mi, map) in level.maps.iter().enumerate() {
        for (oi, obj) in map.objects.iter().enumerate() {
            if obj.hashcode != u32::MAX { by_hash_all.entry(obj.hashcode).or_default().push((mi, oi)); }
        }
    }

    let mut chars = 0u32;
    let mut total_batches = 0u32;
    let mut total_unresolved = 0u32;
    let mut total_oor = 0u32;
    let mut total_no_tex_ref = 0u32;
    let mut total_world_fallback = 0u32;

    println!("=== per-placement characters (skin hashcode in param 0) ===");
    for p in &level.maps[m].placements {
        if p.flags & 0xff != 0x0f { continue; }
        let skin_hash = p.params.iter().find(|(i, _)| *i == 0).map(|&(_, v)| v).unwrap_or(0);
        if skin_hash >> 24 != 5 { continue; }
        chars += 1;
        let Some(skin) = level.skins.get(&skin_hash) else {
            println!("\tplacement @ {:?}: skin {:08x} NOT IN level.skins (G-021: another archive)", p.translation, skin_hash);
            continue;
        };
        let inv_empty = skin.inverse_bind.is_empty();
        println!("\tplacement @ {:?}: skin {:08x} (skel {}): {} main, {} extra, inverse_bind empty={}",
            p.translation, skin_hash, skin.skeleton, skin.main.len(), skin.extra.len(), inv_empty);
        for (mi, mh) in skin.main.iter().enumerate() {
            let Some(mh) = mh else { continue };
            let hits = by_hash_all.get(&mh).cloned().unwrap_or_default();
            let first = level.by_hash.get(&mh).copied();
            println!("\t\tmain[{}] mesh {:08x}: by_hash_first={:?}, all_occurrences={:?} ({} maps declare this hashcode)",
                mi, mh, first, hits, hits.len());
            if hits.is_empty() { continue; }
            let (map_idx, obj_idx) = first.unwrap();
            let obj = &level.maps[map_idx].objects[obj_idx];
            let n_tex = level.maps[map_idx].textures.len();
            let skinned = obj.gfx.is_skinned();
            let world_fallback = !skinned || inv_empty;
            if world_fallback { total_world_fallback += 1; }
            println!("\t\t\tmap {} ({}) obj {} \"{}\": {} textures, skinned={}, world_fallback={}",
                map_idx, level.maps[map_idx].name, obj_idx, obj.name.trim(), n_tex, skinned, world_fallback);
            for (li, leaf) in obj.gfx.leaves.iter().enumerate() {
                let gs_summary: Vec<String> = leaf.gs.iter().map(|(r, v, _)| format!("{:#x}={:#x}", r, v)).collect();
                println!("\t\t\t\tleaf {}: gs=[{}] ({} batches)", li, gs_summary.join(","), leaf.batches.len());
                for (bi, b) in leaf.batches.iter().enumerate() {
                    total_batches += 1;
                    let tex_str = match b.texture {
                        None => { total_no_tex_ref += 1; "None (no REF tag → untextured)".to_string() }
                        Some(t) => {
                            if (t as usize) >= n_tex { total_oor += 1; format!("Some({}) OUT OF RANGE (map has {} textures)", t, n_tex) }
                            else { format!("Some({}) OK", t) }
                        }
                    };
                    println!("\t\t\t\t  batch {}: verts={} skinned={} tex={}", bi, b.verts.len(), b.skinned, tex_str);
                }
            }
            // Count how many of the OTHER maps declaring this hashcode have textures of the same count.
            for &(om, oo) in &hits {
                if (om, oo) == (map_idx, obj_idx) { continue; }
                let o = &level.maps[om].objects[oo];
                println!("\t\t\talso in map {} ({}) obj {} \"{}\": {} textures, skinned={}",
                    om, level.maps[om].name, oo, o.name.trim(), level.maps[om].textures.len(), o.gfx.is_skinned());
            }
        }
        for (ei, (bone, part)) in skin.extra.iter().enumerate() {
            let Some(part) = part else { continue };
            let first = level.by_hash.get(part).copied();
            let n_tex = first.map(|(mm, _)| level.maps[mm].textures.len());
            println!("\t\textra[{}] bone {} part {:08x}: by_hash_first={:?} (textures in that map: {:?})",
                ei, bone, part, first, n_tex);
        }
    }

    // First-person gun sleeves (SleeveEnts_324 + scene.rs SLEEVES constant).
    const SLEEVES: [u32; 8] = [0x0200_09d0, 0x0200_0900, 0x0200_01f7, 0x0200_0929, 0x0200_09e6, 0x0200_09ff, 0x0200_0a14, 0x0200_0a13];
    println!("=== first-person gun sleeves (Bond's hand/gun) ===");
    for &h in &SLEEVES {
        if let Some(&(mm, oo)) = level.by_hash.get(&h) {
            let o = &level.maps[mm].objects[oo];
            let n_tex = level.maps[mm].textures.len();
            let mut unresolved = 0;
            let mut oor = 0;
            let mut nb = 0;
            for leaf in &o.gfx.leaves {
                for b in &leaf.batches {
                    nb += 1;
                    match b.texture {
                        None => unresolved += 1,
                        Some(t) if (t as usize) >= n_tex => oor += 1,
                        _ => {}
                    }
                }
            }
            total_unresolved += unresolved;
            total_oor += oor;
            total_batches += nb;
            println!("\tsleeve {:08x} → map {} ({}) obj {} \"{}\": {} textures, {} batches, no_tex_ref={}, out_of_range={}",
                h, mm, level.maps[mm].name, oo, o.name.trim(), n_tex, nb, unresolved, oor);
        } else {
            println!("\tsleeve {:08x}: NOT FOUND in any map of this archive", h);
        }
    }

    println!("=== summary ===");
    println!("character placements: {}", chars);
    println!("total batches across character + sleeve meshes: {}", total_batches);
    println!("  batches with no texture REF:    {} (white fallback)", total_no_tex_ref);
    println!("  batches with out-of-range tex:  {} (white fallback)", total_oor);
    println!("  character meshes drawn as World layer (not Character): {} (because !is_skinned || inverse_bind empty)", total_world_fallback);

    // Show every map that declares a skin hashcode's main mesh, with its texture count, to see if there are duplicate
    // main-mesh hashcodes in maps with DIFFERENT texture counts.
    println!("=== duplicate main-mesh hashcodes across maps (texture count mismatch would explain oor) ===");
    let mut dupes = 0;
    for skin in level.skins.values() {
        for &mh in skin.main.iter().flatten() {
            let hits = by_hash_all.get(&mh).cloned().unwrap_or_default();
            if hits.len() < 2 { continue; }
            dupes += 1;
            let counts: Vec<usize> = hits.iter().map(|&(mm, oo)| {
                let o = &level.maps[mm].objects[oo];
                if o.gfx.is_skinned() { level.maps[mm].textures.len() } else { usize::MAX }
            }).collect();
            println!("\tskin {:08x} main mesh {:08x}: {} occurrences, texture counts {:?}",
                skin.hashcode, mh, hits.len(), counts);
        }
    }
    if dupes == 0 { println!("\t(no duplicate main-mesh hashcodes)"); }
    Ok(())
}
