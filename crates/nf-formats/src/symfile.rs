//! `DRIVING/DRIVING.SYM`: an address → name table shipped on the disc. Spec: docs/formats/driving-sym.md.
//!
//! ```text
//! entries: (u32 LE address, u32 LE name offset) × n, sorted by address
//! names:   NUL-terminated strings; the first name offset equals 8·n (so n is derived from it)
//! ```
//! Measured: 12,719 entries, addresses 0 … 0xffffffff (linker symbols `_stack_size`, `_heap_size`, `_end` = 0x55cf04),
//! demangled C++ names and object paths (`d:\ToBurn\BondPS2\PS2_EE_Release\*.obj`).
//! **It does not describe the shipped driving ELF** (WORKLOG 0001: 4 % of its in-`.text` addresses land on a
//! function boundary, and its `_end` is 0x55cf04 against the ELF's 0x4279b4), so its names are hints only.

use crate::buf::{invalid, Buf, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymEntry { pub addr: u32, pub name: String }

pub fn parse(data: &[u8]) -> Result<Vec<SymEntry>> {
    let b = Buf(data);
    let first = b.u32(4)? as usize;
    if first % 8 != 0 || first == 0 || first > data.len() { return invalid(format!("first name offset {first:#x} is not a table size")); }
    let n = first / 8;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (addr, off) = (b.u32(i * 8)?, b.u32(i * 8 + 4)? as usize);
        if off < first { return invalid(format!("entry {i}: name offset {off:#x} inside the table")); }
        let tail = b.tail(off, "symbol name")?.bytes();
        let len = tail.iter().position(|&c| c == 0).ok_or_else(|| crate::buf::FormatError::Invalid("unterminated symbol name".into()))?;
        out.push(SymEntry { addr, name: String::from_utf8_lossy(&tail[..len]).into_owned() });
    }
    if out.windows(2).any(|w| w[0].addr > w[1].addr) { return invalid("symbol table not sorted by address"); }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mini(entries: &[(u32, &str)]) -> Vec<u8> {
        let table = entries.len() * 8;
        let mut names = Vec::new();
        let mut out = Vec::new();
        for (a, n) in entries {
            out.extend(a.to_le_bytes());
            out.extend(((table + names.len()) as u32).to_le_bytes());
            names.extend(n.as_bytes());
            names.push(0);
        }
        out.extend(names);
        out
    }

    #[test]
    fn parses_a_synthetic_table() {
        let d = mini(&[(0, "_umemory_size"), (0x107100, "main"), (0xffff_ffff, "_heap_size")]);
        let s = parse(&d).unwrap();
        assert_eq!(s.len(), 3);
        assert_eq!(s[1], SymEntry { addr: 0x107100, name: "main".into() });
    }

    #[test]
    fn rejects_unsorted_tables() {
        let d = mini(&[(0x200, "b"), (0x100, "a")]);
        assert!(parse(&d).is_err());
    }
}
