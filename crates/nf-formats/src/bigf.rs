//! EA `BIGF` archives: the `DRIVING/*.VIV`, `*.MUS` and `*EN.SPE` files. Spec: docs/formats/bigf.md.
//!
//! ```text
//! 0x00  "BIGF"
//! 0x04  u32 LE  archive size  (equals the file size in 20 of the 21 archives on our disc; MISC.VIV differs, UNKNOWNS U-004)
//! 0x08  u32 BE  entry count
//! 0x0c  u32 BE  header size   (bytes from 0 to the end of the entry table; measured, see the NF_ISO test)
//! 0x10  entries: u32 BE offset, u32 BE size, NUL-terminated name (`data\render\eaglrm.o`, backslashes)
//! ```
//! Offsets are from the start of the archive. All of this is *measured* on our disc (WORKLOG 0001); the BIGF layout
//! is also documented by the EA modding community (docs/research/driving-engine-ea-formats.md).

use crate::buf::{invalid, Buf, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BigfEntry { pub name: String, pub offset: u32, pub size: u32 }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bigf {
    /// The raw LE size field at 0x04 (not trusted; see the module doc).
    pub size_field: u32,
    pub header_size: u32,
    pub entries: Vec<BigfEntry>,
}

fn be32(b: &Buf, o: usize) -> Result<u32> { Ok(b.u32(o)?.swap_bytes()) }

impl Bigf {
    /// Parses the header and entry table. `data` needs only the first `header_size` bytes; `total` is the
    /// archive's real size, against which every entry is bounds-checked.
    pub fn parse(data: &[u8], total: u64) -> Result<Bigf> {
        let b = Buf(data);
        b.check(0, 16, "BIGF header")?;
        if &data[..4] != b"BIGF" { return invalid("not a BIGF archive"); }
        let (size_field, count, header_size) = (b.u32(4)?, be32(&b, 8)?, be32(&b, 12)?);
        if count > 1 << 20 { return invalid(format!("implausible BIGF entry count {count}")); }
        let mut pos = 16usize;
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let (offset, size) = (be32(&b, pos)?, be32(&b, pos + 4)?);
            let tail = b.tail(pos + 8, "BIGF entry name")?.bytes();
            let n = tail.iter().position(|&c| c == 0).ok_or_else(|| crate::buf::FormatError::Invalid("unterminated BIGF name".into()))?;
            let name = String::from_utf8_lossy(&tail[..n]).into_owned();
            pos += 8 + n + 1;
            if offset as u64 + size as u64 > total {
                return invalid(format!("BIGF entry {name} ({offset:#x}+{size:#x}) beyond the archive ({total:#x} bytes)"));
            }
            entries.push(BigfEntry { name, offset, size });
        }
        if pos as u32 > header_size { return invalid(format!("BIGF entry table ends at {pos:#x}, past header size {header_size:#x}")); }
        Ok(Bigf { size_field, header_size, entries })
    }

    /// End of the entry table as parsed (for checking what the header-size field means).
    pub fn table_end(&self) -> u32 { 16 + self.entries.iter().map(|e| 8 + e.name.len() as u32 + 1).sum::<u32>() }

    pub fn find(&self, name: &str) -> Option<&BigfEntry> { self.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)) }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn mini_bigf(files: &[(&str, &[u8])]) -> Vec<u8> {
        let table: usize = files.iter().map(|(n, _)| 8 + n.len() + 1).sum();
        let header = 16 + table;
        let mut offs = Vec::new();
        let mut pos = header.next_multiple_of(16);
        for (_, d) in files { offs.push(pos); pos = (pos + d.len()).next_multiple_of(16); }
        let mut out = vec![0u8; pos];
        out[..4].copy_from_slice(b"BIGF");
        out[4..8].copy_from_slice(&(pos as u32).to_le_bytes());
        out[8..12].copy_from_slice(&(files.len() as u32).to_be_bytes());
        out[12..16].copy_from_slice(&(header as u32).to_be_bytes());
        let mut p = 16;
        for ((n, d), o) in files.iter().zip(&offs) {
            out[p..p + 4].copy_from_slice(&(*o as u32).to_be_bytes());
            out[p + 4..p + 8].copy_from_slice(&(d.len() as u32).to_be_bytes());
            out[p + 8..p + 8 + n.len()].copy_from_slice(n.as_bytes());
            p += 8 + n.len() + 1;
            out[*o..*o + d.len()].copy_from_slice(d);
        }
        out
    }

    #[test]
    fn parses_a_synthetic_archive() {
        let a = mini_bigf(&[("data\\a.o", b"hello"), ("b.ssh", b"xyz")]);
        let g = Bigf::parse(&a, a.len() as u64).unwrap();
        assert_eq!(g.entries.len(), 2);
        assert_eq!(g.size_field as usize, a.len());
        assert_eq!(g.table_end(), g.header_size);
        let e = g.find("DATA\\A.O").unwrap();
        assert_eq!(&a[e.offset as usize..][..e.size as usize], b"hello");
        assert_eq!(g.entries[1].name, "b.ssh");
    }

    #[test]
    fn rejects_entries_past_the_end_and_bad_magic() {
        let a = mini_bigf(&[("x", b"0123456789")]);
        let e = &Bigf::parse(&a, a.len() as u64).unwrap().entries[0];
        let end = e.offset as u64 + e.size as u64;
        assert!(Bigf::parse(&a, end).is_ok());
        assert!(Bigf::parse(&a, end - 1).is_err());
        let mut b = a.clone();
        b[0] = b'C';
        assert!(Bigf::parse(&b, b.len() as u64).is_err());
    }
}
