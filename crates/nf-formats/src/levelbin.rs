//! Level / pack `.bin` archives inside FILES.BIN (`07000001.bin`, …). Spec: docs/formats/level-bin.md.
//!
//! ```text
//! 0x000 u32 dir_size, 0x004 u32 entry_count, zero to 0x800
//! 0x800 entry_count × { u32 length, u8 type, NUL-terminated name }, zero-padded to dir_size
//! 0x800 + dir_size: entry data back to back in directory order
//! ```
//! Mirrors `LoaderLoad` (ACTION.ELF 0x1d4a00): the game reads the directory as `round_up(dir_size, 2048)` bytes and
//! the entries as one sector-rounded batch; on our disc `dir_size` is always a multiple of 2048, so both views agree
//! (checked by `validate`).

use crate::buf::{invalid, Buf, Result};

pub const HEADER_SIZE: usize = 0x800;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinEntry {
    pub kind: u8,
    pub name: String,
    /// Absolute offset of the entry's data within the archive.
    pub offset: u32,
    pub length: u32,
}

impl BinEntry {
    /// `DirFileHash` as the loader computes it: up to 8 hex digits from the start of the name's last path component
    /// (after `\` or `/`), case-insensitive; stops at the first non-hex character. 0 when there are none.
    pub fn hash(&self) -> u32 {
        let last = self.name.rsplit(['\\', '/']).next().unwrap_or("");
        last.chars().take(8).map_while(|c| c.to_digit(16)).fold(0, |h, d| h << 4 | d)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelBin {
    pub dir_size: u32,
    pub entries: Vec<BinEntry>,
}

impl LevelBin {
    pub fn parse(data: &[u8]) -> Result<LevelBin> {
        let b = Buf(data);
        b.check(0, HEADER_SIZE, ".bin header")?;
        let (dir_size, count) = (b.u32(0)?, b.u32(4)?);
        if count == 0 { return Ok(LevelBin { dir_size, entries: Vec::new() }); }
        let dir = b.sub(HEADER_SIZE, dir_size as usize, ".bin directory")?;
        if count as usize > dir_size as usize / 6 { return invalid(format!("{count} entries cannot fit a {dir_size:#x}-byte directory")); }
        let mut pos = 0usize;
        let mut offset = (HEADER_SIZE + dir_size as usize) as u64;
        let mut entries = Vec::with_capacity(count as usize);
        for i in 0..count {
            let length = dir.u32(pos)?;
            let kind = dir.u8(pos + 4)?;
            let tail = dir.tail(pos + 5, ".bin entry name")?.bytes();
            let n = tail.iter().position(|&c| c == 0).ok_or_else(|| crate::buf::FormatError::Invalid(format!("entry {i}: unterminated name")))?;
            let name = String::from_utf8_lossy(&tail[..n]).into_owned();
            pos += 5 + n + 1;
            if offset + length as u64 > data.len() as u64 {
                return invalid(format!("entry {i} ({name}) {offset:#x}+{length:#x} beyond the {:#x}-byte archive", data.len()));
            }
            entries.push(BinEntry { kind, name, offset: offset as u32, length });
            offset += length as u64;
        }
        Ok(LevelBin { dir_size, entries })
    }

    /// The facts the reader relies on, as measured on our disc: directory size sector-aligned, and the entries filling
    /// the archive exactly.
    pub fn validate(&self, archive_len: usize) -> Result<()> {
        if self.entries.is_empty() { return Ok(()); }
        if self.dir_size % 2048 != 0 { return invalid(format!("dir_size {:#x} not sector-aligned", self.dir_size)); }
        let end = self.entries.last().map(|e| e.offset as usize + e.length as usize).unwrap_or(0);
        if end != archive_len { return invalid(format!("entries end at {end:#x}, archive is {archive_len:#x}")); }
        Ok(())
    }

    pub fn data<'a>(&self, archive: &'a [u8], e: &BinEntry) -> &'a [u8] { &archive[e.offset as usize..][..e.length as usize] }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn mini_bin(entries: &[(u8, &str, &[u8])]) -> Vec<u8> {
        let mut dir = Vec::new();
        for (k, n, d) in entries {
            dir.extend((d.len() as u32).to_le_bytes());
            dir.push(*k);
            dir.extend(n.as_bytes());
            dir.push(0);
        }
        let dir_size = dir.len().next_multiple_of(2048);
        dir.resize(dir_size, 0);
        let mut out = vec![0u8; HEADER_SIZE];
        out[..4].copy_from_slice(&(dir_size as u32).to_le_bytes());
        out[4..8].copy_from_slice(&(entries.len() as u32).to_le_bytes());
        out.extend(dir);
        for (_, _, d) in entries { out.extend(*d); }
        out
    }

    #[test]
    fn parses_a_synthetic_archive() {
        let a = mini_bin(&[(1, "01000002", b"map"), (6, "SKEL0002", b"skel!"), (12, "", &[0; 32])]);
        let bin = LevelBin::parse(&a).unwrap();
        bin.validate(a.len()).unwrap();
        assert_eq!(bin.entries.len(), 3);
        assert_eq!(bin.data(&a, &bin.entries[1]), b"skel!");
        assert_eq!(bin.entries[1].offset, 0x800 + 0x800 + 3);
        assert_eq!((bin.entries[0].hash(), bin.entries[1].hash(), bin.entries[2].hash()), (0x0100_0002, 0, 0));
    }

    #[test]
    fn hash_follows_the_loader() {
        let e = |n: &str| BinEntry { kind: 0, name: n.into(), offset: 0, length: 0 }.hash();
        assert_eq!(e("0400abcd"), 0x0400_abcd);
        assert_eq!(e("dir\\0400ABCD"), 0x0400_abcd);
        assert_eq!(e("a/b/12z"), 0x12);
        assert_eq!(e("123456789"), 0x1234_5678, "at most 8 digits");
    }

    #[test]
    fn rejects_truncation_and_trailing_bytes() {
        let a = mini_bin(&[(1, "01000002", b"map")]);
        assert!(LevelBin::parse(&a[..a.len() - 1]).is_err());
        let mut b = a.clone();
        b.push(0);
        assert!(LevelBin::parse(&b).unwrap().validate(b.len()).is_err());
        assert!(LevelBin::parse(&[0u8; 100]).is_err());
        let empty = vec![0u8; 2048];
        assert!(LevelBin::parse(&empty).unwrap().entries.is_empty());
    }
}
