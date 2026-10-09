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

#[allow(dead_code)]
mod png;

use anyhow::{bail, Context, Result};
use nf_formats::{disc, elf::Elf, iso9660::IsoImage, sha1, symfile};
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

fn symtab(path: &Path) -> Result<()> {
    let data = std::fs::read(path)?;
    let elf = Elf::parse(&data)?;
    println!("value\tsize\tkind\tbind\tshndx\tname");
    for s in elf.symbols(&data)?.iter().skip(1) {
        println!("{:08x}\t{}\t{}\t{}\t{}\t{}", s.value, s.size, s.kind(), s.binding(), s.shndx, s.name);
    }
    Ok(())
}
