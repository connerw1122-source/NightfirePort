//! The Nightfire disc: build identity, the directory-tree manifest, and sectors no directory entry covers.
//! Spec: docs/formats/disc.md, docs/build-identity.md.
//!
//! Measured (WORKLOG 0001): the boot loader `SLUS_205.79` (byte-identical to `BASE.ELF`) loads `ACTION.ELF`
//! (Eurocom on-foot engine) or `DRIVING.ELF` (EA driving engine, LBA 1153); both are ordinary root-directory files.

use crate::bigf::Bigf;
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
