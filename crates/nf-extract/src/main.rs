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
        "level-export" =>level_export(&IsoImage::open(a(1)?)?, a(2)?, Path::new(a(3)?), args.get(4).is_some_and(|x| x == "all")),
        "gfx-boxes" =>gfx_boxes(&IsoImage::open(a(1)?)?),
        "gfx-export" =>gfx_export(&IsoImage::open(a(1)?)?, Path::new(a(2)?), args.get(3).map(String::as_str)),
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
