//! The Nightfire disc: build identity, the directory-tree manifest, and sectors no directory entry covers.
//! Spec: docs/formats/disc.md, docs/build-identity.md.
//!
//! Measured (WORKLOG 0001): the boot loader `SLUS_205.79` (byte-identical to `BASE.ELF`) loads `ACTION.ELF`
//! (Eurocom on-foot engine) or `DRIVING.ELF` (EA driving engine, LBA 1153); both are ordinary root-directory files.

use crate::bigf::Bigf;
use crate::buf::Buf;
use crate::iso9660::{IsoEntry, IsoImage, Result, SECTOR_SIZE};
use crate::sha1::{hex, Sha1};
use std::io::{Read, Seek};

pub const BOOT_ELF: &str = "/SLUS_205.79";
pub const BOOT_ELF_SHA1: &str = "d5a9bd3ee0e9f1faf519fbd27138a2cdf4e82efe";
pub const ACTION_ELF_SHA1: &str = "e7ffdcb38ccd309388046ab864ebde5e3414eaab";
pub const DRIVING_ELF_SHA1: &str = "d7b8a57f3b8e17e980a6d9228b2ad02ed8eebc5c";

/// A run of sectors no directory entry covers, as `(first_lba, end_lba)` (end exclusive).
pub fn unlisted_ranges<R: Read + Seek>(iso: &IsoImage<R>) -> Vec<(u32, u32)> {
    let mut spans: Vec<(u32, u32)> = iso.entries().iter()
        .map(|e| (e.lba, e.lba + (e.size as usize).div_ceil(SECTOR_SIZE).max(1) as u32)).collect();
    spans.sort();
    let mut out = Vec::new();
    let mut cur = 0u32;
    for (a, b) in spans {
        if a > cur { out.push((cur, a)); }
        cur = cur.max(b);
    }
    if iso.sector_count() > cur { out.push((cur, iso.sector_count())); }
    out
}

/// One manifest line per directory entry: `path \t lba \t size \t sha1` (directories: sha1 `-`). Numbers and hashes
/// only, so it can be committed (`golden/disc_files.tsv`).
pub fn manifest<R: Read + Seek>(iso: &IsoImage<R>) -> Result<String> {
    let mut s = String::from("path\tlba\tsize\tsha1\n");
    let mut entries: Vec<_> = iso.entries().iter().collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    for e in entries {
        let h = if e.is_directory { "-".to_string() } else { hash_extent(iso, e.lba, e.size as u64)? };
        s.push_str(&format!("{}\t{}\t{}\t{}\n", e.path, e.lba, e.size, h));
    }
    Ok(s)
}

/// SHA-1 of `size` bytes from `lba`.
pub fn hash_extent<R: Read + Seek>(iso: &IsoImage<R>, lba: u32, size: u64) -> Result<String> {
    hash_extent_at(iso, lba as u64 * SECTOR_SIZE as u64, size)
}

/// SHA-1 of `size` bytes from logical byte `offset`, read in 16 MiB chunks.
pub fn hash_extent_at<R: Read + Seek>(iso: &IsoImage<R>, offset: u64, size: u64) -> Result<String> {
    let mut h = Sha1::new();
    let mut done = 0u64;
    const CHUNK: u64 = 16 << 20;
    while done < size {
        let n = CHUNK.min(size - done);
        h.update(&iso.read_bytes(offset + done, n)?);
        done += n;
    }
    Ok(hex(&h.finish()))
}

/// Reads the header and entry table of the BIGF archive stored as directory entry `e`.
pub fn read_bigf<R: Read + Seek>(iso: &IsoImage<R>, e: &IsoEntry) -> Result<Bigf> {
    let base = e.lba as u64 * SECTOR_SIZE as u64;
    let head = iso.read_bytes(base, 16.min(e.size as u64))?;
    if head.len() < 16 { return crate::iso9660::bad(format!("{}: too small for a BIGF header", e.path)); }
    let hs = u32::from_be_bytes(head[12..16].try_into().unwrap()) as u64;
    Ok(Bigf::parse(&iso.read_bytes(base, hs.min(e.size as u64))?, e.size as u64)?)
}

/// Every entry of every BIGF archive on the disc (found by magic): `archive#index \t name \t offset \t size \t sha1`.
/// Entry names are kept as identifiers (CLAUDE.md: identifiers allowed in goldens, content never).
pub fn bigf_manifest<R: Read + Seek>(iso: &IsoImage<R>) -> Result<String> {
    let mut s = String::from("key\tname\toffset\tsize\tsha1\n");
    let mut files: Vec<_> = iso.entries().iter().filter(|e| !e.is_directory && e.size >= 16).collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    for e in files {
        if iso.read_bytes(e.lba as u64 * SECTOR_SIZE as u64, 4)? != b"BIGF" { continue; }
        let g = read_bigf(iso, e)?;
        for (i, x) in g.entries.iter().enumerate() {
            let h = hash_extent_at(iso, e.lba as u64 * SECTOR_SIZE as u64 + x.offset as u64, x.size as u64)?;
            s.push_str(&format!("{}#{i}\t{}\t{}\t{}\t{h}\n", e.path, x.name, x.offset, x.size));
        }
    }
    Ok(s)
}

/// The `FileList` index read from the disc's ACTION.ELF, checked against FILES.BIN.
pub fn files_bin_entries<R: Read + Seek>(iso: &IsoImage<R>) -> Result<(Vec<crate::filelist::FileEntry>, &IsoEntry)> {
    let action = iso.find("/ACTION.ELF").ok_or_else(|| crate::buf::FormatError::Invalid("no ACTION.ELF".into()))?;
    let bin = iso.find("/FILES.BIN").ok_or_else(|| crate::buf::FormatError::Invalid("no FILES.BIN".into()))?;
    let entries = crate::filelist::from_action_elf(&iso.read_file(action)?)?;
    crate::filelist::validate(&entries, bin.size as u64)?;
    Ok((entries, bin))
}

/// `name \t offset \t size \t sha1` for every `FileList` entry (FILES.BIN contents hashed, never stored).
pub fn files_bin_manifest<R: Read + Seek>(iso: &IsoImage<R>) -> Result<String> {
    let (entries, bin) = files_bin_entries(iso)?;
    let mut s = String::from("name\toffset\tsize\tsha1\n");
    for e in &entries {
        let h = hash_extent_at(iso, bin.lba as u64 * SECTOR_SIZE as u64 + e.offset as u64, e.size as u64)?;
        s.push_str(&format!("{}\t{}\t{}\t{h}\n", e.name, e.offset, e.size));
    }
    Ok(s)
}

/// Every entry of every level `.bin` in FILES.BIN: `archive \t index \t type \t name \t offset \t length \t sha1`.
/// Too large to commit (≈44k rows); `level_bin_digest` condenses it per archive.
pub fn level_bin_entries<R: Read + Seek>(iso: &IsoImage<R>) -> Result<String> {
    let (entries, bin) = files_bin_entries(iso)?;
    let mut s = String::from("archive\tindex\ttype\tname\toffset\tlength\tsha1\n");
    for f in entries.iter().filter(|f| f.name.to_ascii_lowercase().ends_with(".bin")) {
        let data = iso.read_bytes(bin.lba as u64 * SECTOR_SIZE as u64 + f.offset as u64, f.size as u64)?;
        let lb = crate::levelbin::LevelBin::parse(&data)?;
        lb.validate(data.len())?;
        for (i, e) in lb.entries.iter().enumerate() {
            s.push_str(&format!("{}\t{i}\t{}\t{}\t{}\t{}\t{}\n", f.name, e.kind, e.name, e.offset, e.length,
                hex(&crate::sha1::sha1(lb.data(&data, e)))));
        }
    }
    Ok(s)
}

/// Per archive: `archive \t entries \t types (type:count,…) \t bytes \t sha1 of its rows in level_bin_entries`.
pub fn level_bin_digest(entries_tsv: &str) -> String {
    use std::collections::BTreeMap;
    let mut per: BTreeMap<&str, (u32, BTreeMap<u32, u32>, u64, Sha1)> = BTreeMap::new();
    for line in entries_tsv.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        let e = per.entry(c[0]).or_insert_with(|| (0, BTreeMap::new(), 0, Sha1::new()));
        e.0 += 1;
        *e.1.entry(c[2].parse().unwrap_or(u32::MAX)).or_default() += 1;
        e.2 += c[5].parse::<u64>().unwrap_or(0);
        e.3.update(line.as_bytes());
        e.3.update(b"\n");
    }
    let mut s = String::from("archive\tentries\ttypes\tbytes\trows_sha1\n");
    for (a, (n, types, bytes, h)) in per {
        let t: Vec<String> = types.iter().map(|(k, v)| format!("{k}:{v}")).collect();
        s.push_str(&format!("{a}\t{n}\t{}\t{bytes}\t{}\n", t.join(","), hex(&h.finish())));
    }
    s
}

/// Calls `f(archive, entry index, entry, entry bytes)` for every map entry (level `.bin` types 0/1/2/11).
pub fn for_each_map<R: Read + Seek>(iso: &IsoImage<R>, mut f: impl FnMut(&str, usize, &crate::levelbin::BinEntry, &[u8]) -> Result<()>) -> Result<()> {
    let (entries, bin) = files_bin_entries(iso)?;
    for fe in entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".bin")) {
        let data = iso.read_bytes(bin.lba as u64 * SECTOR_SIZE as u64 + fe.offset as u64, fe.size as u64)?;
        let lb = crate::levelbin::LevelBin::parse(&data)?;
        for (i, e) in lb.entries.iter().enumerate().filter(|(_, e)| matches!(e.kind, 0 | 1 | 2 | 11)) {
            f(&fe.name, i, e, lb.data(&data, e))?;
        }
    }
    Ok(())
}

/// Every map texture: `archive \t entry \t tex \t flags \t w \t h \t frames \t rate \t hashcode \t sha1(indices)
/// \t sha1(palette, linear order, PS2 alpha)`; textures the header over-states are listed with `short` hashes.
pub fn texture_entries<R: Read + Seek>(iso: &IsoImage<R>) -> Result<String> {
    let mut s = String::from("archive\tentry\ttex\tflags\tw\th\tframes\trate\thashcode\tindices_sha1\tpalette_sha1\n");
    for_each_map(iso, |archive, i, _e, map| {
        let mf = crate::map::MapFile::parse(map)?;
        let (texs, bad) = crate::texture::textures(map, &mf)?;
        let mut t = texs.iter();
        let n = texs.len() + bad.len();
        for k in 0..n {
            if let Some((_, msg)) = bad.iter().find(|(j, _)| *j == k) {
                s.push_str(&format!("{archive}\t{i}\t{k}\t-\t-\t-\t-\t-\t-\tshort\t{msg}\n"));
                continue;
            }
            let x = t.next().unwrap();
            let pal: Vec<u8> = x.palette.iter().flatten().copied().collect();
            s.push_str(&format!("{archive}\t{i}\t{k}\t{:#04x}\t{}\t{}\t{}\t{}\t{:08x}\t{}\t{}\n", x.flags, x.width, x.height,
                x.frames, x.rate, x.hashcode, hex(&crate::sha1::sha1(&x.indices)), hex(&crate::sha1::sha1(&pal))));
        }
        Ok(())
    })?;
    Ok(s)
}

/// Per archive digest of `texture_entries`: `archive \t textures \t 8bpp \t short \t rows_sha1`.
pub fn texture_digest(entries_tsv: &str) -> String {
    use std::collections::BTreeMap;
    let mut per: BTreeMap<&str, (u32, u32, u32, Sha1)> = BTreeMap::new();
    for line in entries_tsv.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        let e = per.entry(c[0]).or_insert_with(|| (0, 0, 0, Sha1::new()));
        e.0 += 1;
        if c[3] != "-" && u8::from_str_radix(c[3].trim_start_matches("0x"), 16).is_ok_and(|f| f & 1 != 0) { e.1 += 1; }
        if c[9] == "short" { e.2 += 1; }
        e.3.update(line.as_bytes());
        e.3.update(b"\n");
    }
    let mut s = String::from("archive\ttextures\teight_bpp\tshort\trows_sha1\n");
    for (a, (n, e8, short, h)) in per { s.push_str(&format!("{a}\t{n}\t{e8}\t{short}\t{}\n", hex(&h.finish()))); }
    s
}

/// Calls `f(archive, entry, object index, entity hashcode, entity name, decoded object)` for every PS2_GFX block,
/// pairing it with the entity-params block (0x04) that *follows* it in parse order: `parsemap_block_PS2_GFX` stores
/// the gfx in the current cel, then `parsemap_block_entity_params` builds it (`psiCreateEntityGfx`) and advances to
/// the next cel (WORKLOG 0009). Every PS2_GFX must be followed by exactly one 0x04 before the next PS2_GFX.
pub fn for_each_gfx<R: Read + Seek>(iso: &IsoImage<R>,
    mut f: impl FnMut(&str, usize, usize, u32, &str, &crate::ps2gfx::GfxObject) -> Result<()>) -> Result<()> {
    use crate::map::{id, MapFile};
    for_each_map(iso, |archive, entry, _e, map| {
        let mf = MapFile::parse(map)?;
        let mut pending: Option<crate::ps2gfx::GfxObject> = None;
        let mut n = 0usize;
        for blk in &mf.blocks {
            match blk.id {
                id::PS2_GFX => {
                    if pending.is_some() { return crate::iso9660::bad(format!("{archive} entry {entry}: two PS2_GFX blocks without entity params")); }
                    let raw = &map[blk.offset as usize..][..blk.size as usize];
                    pending = Some(crate::ps2gfx::GfxObject::parse(raw)
                        .map_err(|e| crate::buf::FormatError::Invalid(format!("{archive} entry {entry} object {n}: {e}")))?);
                }
                id::ENTITY_PARAMS => {
                    let Some(obj) = pending.take() else { return crate::iso9660::bad(format!("{archive} entry {entry}: entity params without PS2_GFX")) };
                    let p = Buf(blk.payload(map));
                    let hash = p.u32(0)?;
                    let tail = p.tail(0x30, "entity name")?.bytes();
                    let name = String::from_utf8_lossy(&tail[..tail.iter().position(|&c| c == 0).unwrap_or(tail.len())]).into_owned();
                    f(archive, entry, n, hash, &name, &obj)?;
                    n += 1;
                }
                _ => {}
            }
        }
        if pending.is_some() { return crate::iso9660::bad(format!("{archive} entry {entry}: PS2_GFX at the end without entity params")); }
        Ok(())
    })
}

/// One row per PS2_GFX object: counts, the box vertex-count check, and a SHA-1 over the decoded geometry
/// (per batch: texture, prim, nloop; per vertex: position/uv f32 bits, rgba, normal, w).
pub fn geometry_entries<R: Read + Seek>(iso: &IsoImage<R>) -> Result<String> {
    let mut s = String::from("archive\tentry\tobject\thashcode\tname\tskinned\tboxes\tleaves\tbatches\tvertices\ttriangles\tkicks\tsetup_kicks\tbox_count_sum\tgeometry_sha1\n");
    for_each_gfx(iso, |archive, entry, n, hash, name, obj| {
        let mut h = Sha1::new();
        let (mut batches, mut verts, mut tris, mut kicks, mut setup, mut box_vsum) = (0, 0, 0, 0, 0, 0u64);
        for leaf in &obj.leaves {
            setup += leaf.setup_kicks;
            for b in &leaf.batches {
                batches += 1;
                verts += b.verts.len();
                tris += b.triangles().len();
                kicks += b.verts.iter().filter(|v| v.w == 0).count();
                h.update(&b.texture.unwrap_or(u32::MAX).to_le_bytes());
                h.update(&b.prim.to_le_bytes());
                h.update(&b.nloop.to_le_bytes());
                for v in &b.verts {
                    for x in v.pos.iter().chain(&v.uv) { h.update(&x.to_bits().to_le_bytes()); }
                    h.update(&v.rgba);
                    h.update(&v.normal.map(|c| c as u8));
                    h.update(&v.w.to_le_bytes());
                }
            }
            box_vsum += obj.boxes[leaf.box_index].vertex_count as u64;
        }
        let leaves = obj.boxes.iter().filter(|b| b.packet != 0).count();
        s.push_str(&format!("{archive}\t{entry}\t{n}\t{hash:08x}\t{name}\t{}\t{}\t{leaves}\t{batches}\t{verts}\t{tris}\t{kicks}\t{setup}\t{box_vsum}\t{}\n",
            obj.is_skinned() as u8, obj.boxes.len(), hex(&h.finish())));
        Ok(())
    })?;
    Ok(s)
}

/// Per archive: `archive \t objects \t skinned \t vertices \t triangles \t rows_sha1`.
pub fn geometry_digest(entries_tsv: &str) -> String {
    use std::collections::BTreeMap;
    let mut per: BTreeMap<&str, (u32, u32, u64, u64, Sha1)> = BTreeMap::new();
    for line in entries_tsv.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        let e = per.entry(c[0]).or_insert_with(|| (0, 0, 0, 0, Sha1::new()));
        e.0 += 1;
        e.1 += c[5].parse::<u32>().unwrap_or(0);
        e.2 += c[9].parse::<u64>().unwrap_or(0);
        e.3 += c[10].parse::<u64>().unwrap_or(0);
        e.4.update(line.as_bytes());
        e.4.update(b"\n");
    }
    let mut s = String::from("archive\tobjects\tskinned\tvertices\ttriangles\trows_sha1\n");
    for (a, (n, sk, v, t, h)) in per { s.push_str(&format!("{a}\t{n}\t{sk}\t{v}\t{t}\t{}\n", hex(&h.finish()))); }
    s
}

/// Per level archive with a main map: `archive \t maps \t main map \t placements \t cels \t resolved \t unresolved \t
/// triangles (all placements)`.
pub fn level_summary<R: Read + Seek>(iso: &IsoImage<R>) -> Result<String> {
    let (entries, bin) = files_bin_entries(iso)?;
    let mut s = String::from("archive\tmaps\tmain_map\tplacements\tcels\tresolved\tunresolved\ttriangles\n");
    for fe in entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".bin")) {
        let data = iso.read_bytes(bin.lba as u64 * SECTOR_SIZE as u64 + fe.offset as u64, fe.size as u64)?;
        let level = crate::level::Level::parse(&data)?;
        let Some(m) = level.main_map() else { continue };
        let (inst, missing) = level.instances(false);
        let tris: usize = inst.iter().map(|i| level.maps[i.map].objects[i.object].gfx.leaves.iter()
            .flat_map(|l| &l.batches).map(|b| b.triangles().len()).sum::<usize>()).sum();
        let pl = &level.maps[m].placements;
        s.push_str(&format!("{}\t{}\t{}\t{}\t{}\t{}\t{missing}\t{tris}\n", fe.name, level.maps.len(), level.maps[m].name,
            pl.len(), pl.iter().filter(|p| p.creates_cel()).count(), inst.len()));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso9660::tests::mini_iso;
    use std::io::Cursor;

    #[test]
    fn unlisted_ranges_are_the_gaps_between_entries() {
        // mini_iso: PVD at 16, root directory at 20 (the root is not itself an entry), entries at 21..=25 (the
        // 3000-byte ELF spans 23..=24). The system area, the PVD and the root's own sector are unlisted.
        let (img, _) = mini_iso(40);
        let iso = IsoImage::new(Cursor::new(img)).unwrap();
        assert_eq!(unlisted_ranges(&iso), [(0, 21), (26, 40)]);
    }

    #[test]
    fn manifest_lists_every_entry_with_hashes() {
        let (img, elf) = mini_iso(32);
        let iso = IsoImage::new(Cursor::new(img)).unwrap();
        let m = manifest(&iso).unwrap();
        let lines: Vec<&str> = m.lines().collect();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1], "/DATA\t21\t2048\t-");
        assert_eq!(lines[3], format!("/SCUS_971.99\t23\t3000\t{}", hex(&crate::sha1::sha1(&elf))));
    }
}
