//! `FILES.BIN` and its index, the `FileList` table in ACTION.ELF. Spec: docs/formats/files-bin.md.
//!
//! ```text
//! FileList (ACTION.ELF .data, symbol `FileList` 0x2446a0, 3,776 bytes = 118 × 32):
//!   0x00 char[16] name (NUL-padded, e.g. "07000001.bin", "TuningVars.txt")
//!   0x10 u32 LE   offset into FILES.BIN (2048-aligned)
//!   0x14 u32 LE   size
//!   0x18 u32 × 2  zero in every entry
//! ```
//! *Measured* (WORKLOG 0003); first described by NightfireResearch (*sourced*, facts only). The table is found through
//! the ELF's own `.symtab`, so no address is hard-coded.

use crate::buf::{invalid, Buf, Result};
use crate::elf::Elf;

pub const ENTRY_SIZE: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry { pub name: String, pub offset: u32, pub size: u32 }

/// Reads `FileList` out of ACTION.ELF via its symbol (value = address, size = table bytes).
pub fn from_action_elf(elf_bytes: &[u8]) -> Result<Vec<FileEntry>> {
    let elf = Elf::parse(elf_bytes)?;
    let syms = elf.symbols(elf_bytes)?;
    let Some(sym) = syms.iter().find(|s| s.name == "FileList") else { return invalid("no FileList symbol (not ACTION.ELF?)") };
    let (vaddr, seg, _) = elf.load_segments(elf_bytes)?.into_iter()
        .find(|(va, d, _)| sym.value >= *va && (sym.value - va) as usize + sym.size as usize <= d.len())
        .ok_or_else(|| crate::buf::FormatError::Invalid("FileList outside the loaded data".into()))?;
    parse(&seg[(sym.value - vaddr) as usize..][..sym.size as usize])
}

/// Parses a raw table (a whole number of 32-byte entries).
pub fn parse(table: &[u8]) -> Result<Vec<FileEntry>> {
    if table.len() % ENTRY_SIZE != 0 { return invalid(format!("FileList size {:#x} is not a multiple of 32", table.len())); }
    let b = Buf(table);
    let mut out = Vec::with_capacity(table.len() / ENTRY_SIZE);
    for i in 0..table.len() / ENTRY_SIZE {
        let o = i * ENTRY_SIZE;
        let raw = &table[o..o + 16];
        let n = raw.iter().position(|&c| c == 0).unwrap_or(16);
        if n == 0 || !raw[..n].iter().all(|c| c.is_ascii_graphic()) { return invalid(format!("FileList entry {i}: bad name")); }
        if b.u32(o + 24)? != 0 || b.u32(o + 28)? != 0 { return invalid(format!("FileList entry {i}: non-zero padding")); }
        out.push(FileEntry { name: String::from_utf8_lossy(&raw[..n]).into_owned(), offset: b.u32(o + 16)?, size: b.u32(o + 20)? });
    }
    Ok(out)
}

/// Checks the entries against the archive size: in bounds, 2048-aligned, no overlaps.
pub fn validate(entries: &[FileEntry], archive_size: u64) -> Result<()> {
    let mut sorted: Vec<&FileEntry> = entries.iter().collect();
    sorted.sort_by_key(|e| e.offset);
    for (i, e) in sorted.iter().enumerate() {
        if e.offset % 2048 != 0 { return invalid(format!("{}: offset {:#x} not 2048-aligned", e.name, e.offset)); }
        if e.offset as u64 + e.size as u64 > archive_size { return invalid(format!("{}: beyond FILES.BIN", e.name)); }
        if let Some(next) = sorted.get(i + 1) {
            if e.offset + e.size > next.offset { return invalid(format!("{} overlaps {}", e.name, next.name)); }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, offset: u32, size: u32) -> Vec<u8> {
        let mut e = vec![0u8; 32];
        e[..name.len()].copy_from_slice(name.as_bytes());
        e[16..20].copy_from_slice(&offset.to_le_bytes());
        e[20..24].copy_from_slice(&size.to_le_bytes());
        e
    }

    #[test]
    fn parses_and_validates_a_synthetic_table() {
        let t = [entry("a.dat", 0, 100), entry("07000001.bin", 2048, 4096)].concat();
        let es = parse(&t).unwrap();
        assert_eq!(es[1], FileEntry { name: "07000001.bin".into(), offset: 2048, size: 4096 });
        assert!(validate(&es, 6144).is_ok());
        assert!(validate(&es, 6143).is_err(), "out of bounds");
    }

    #[test]
    fn rejects_overlap_misalignment_and_bad_entries() {
        assert!(validate(&parse(&[entry("a", 0, 3000), entry("b", 2048, 10)].concat()).unwrap(), 1 << 20).is_err());
        assert!(validate(&parse(&entry("a", 100, 10)).unwrap(), 1 << 20).is_err());
        let mut bad = entry("a", 0, 1);
        bad[24] = 1;
        assert!(parse(&bad).is_err());
        assert!(parse(&entry("", 0, 1)).is_err());
        assert!(parse(&[0u8; 31]).is_err());
    }
}
