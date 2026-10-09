//! 32-bit little-endian MIPS ELF reader: header, program headers, section headers and the `.symtab`.
//! Spec: docs/formats/elf.md.
//!
//! All three Nightfire executables are ET_EXEC with one PT_LOAD segment (measured, docs/build-identity.md).
//! `ACTION.ELF` keeps `.symtab`/`.strtab`; the boot loader and the driving ELF are stripped. The reader also
//! reports the byte extent of an ELF (the furthest of segment data and section headers); on our disc each ELF's
//! extent equals its directory-entry size (checked by the NF_ISO test).

use crate::buf::{invalid, Buf, Result};

pub const PT_LOAD: u32 = 1;
pub const SHT_SYMTAB: u32 = 2;
pub const SHT_NOBITS: u32 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramHeader { pub p_type: u32, pub offset: u32, pub vaddr: u32, pub filesz: u32, pub memsz: u32, pub flags: u32 }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section { pub name: String, pub sh_type: u32, pub addr: u32, pub offset: u32, pub size: u32, pub link: u32, pub entsize: u32 }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol { pub name: String, pub value: u32, pub size: u32, pub info: u8, pub shndx: u16 }

impl Symbol {
    /// STT_*: 0 notype, 1 object, 2 func, 3 section, 4 file.
    pub fn kind(&self) -> u8 { self.info & 0xf }
    /// STB_*: 0 local, 1 global, 2 weak.
    pub fn binding(&self) -> u8 { self.info >> 4 }
}

#[derive(Clone, Debug)]
pub struct Elf {
    pub entry: u32,
    pub phdrs: Vec<ProgramHeader>,
    pub sections: Vec<Section>,
    /// Bytes from the start of the file to the end of the furthest segment data or header table.
    pub extent: u32,
}

impl Elf {
    pub fn parse(data: &[u8]) -> Result<Elf> {
        let b = Buf(data);
        b.check(0, 52, "ELF header")?;
        if &data[..4] != b"\x7fELF" { return invalid("not an ELF (bad magic)"); }
        if data[4] != 1 || data[5] != 1 { return invalid("not a 32-bit little-endian ELF"); }
        if b.u16(18)? != 8 { return invalid(format!("e_machine {} is not MIPS (8)", b.u16(18)?)); }
        let (entry, phoff, shoff) = (b.u32(24)?, b.u32(28)? as usize, b.u32(32)? as usize);
        let (phentsize, phnum) = (b.u16(42)? as usize, b.u16(44)? as usize);
        let (shentsize, shnum, shstrndx) = (b.u16(46)? as usize, b.u16(48)? as usize, b.u16(50)? as usize);
        if phnum > 0 && phentsize != 32 { return invalid(format!("e_phentsize {phentsize}")); }
        if shnum > 0 && shentsize != 40 { return invalid(format!("e_shentsize {shentsize}")); }

        let mut extent = 52u64.max((phoff + phnum * 32) as u64);
        let mut phdrs = Vec::with_capacity(phnum);
        for i in 0..phnum {
            let o = phoff + i * 32;
            let ph = ProgramHeader { p_type: b.u32(o)?, offset: b.u32(o + 4)?, vaddr: b.u32(o + 8)?, filesz: b.u32(o + 16)?, memsz: b.u32(o + 20)?, flags: b.u32(o + 24)? };
            extent = extent.max(ph.offset as u64 + ph.filesz as u64);
            phdrs.push(ph);
        }
        let mut raw = Vec::with_capacity(shnum);
        for i in 0..shnum {
            let o = shoff + i * 40;
            raw.push((b.u32(o)?, b.u32(o + 4)?, b.u32(o + 12)?, b.u32(o + 16)?, b.u32(o + 20)?, b.u32(o + 24)?, b.u32(o + 36)?));
        }
        if shnum > 0 { extent = extent.max((shoff + shnum * 40) as u64); }
        let mut sections = Vec::with_capacity(shnum);
        let strtab = if shnum > 0 && shstrndx < shnum { Some(raw[shstrndx]) } else { None };
        for &(name, sh_type, addr, offset, size, link, entsize) in &raw {
            if sh_type != SHT_NOBITS { extent = extent.max(offset as u64 + size as u64); }
            let name = match strtab { Some(s) => cstr(&b, s.3 as usize + name as usize)?, None => String::new() };
            sections.push(Section { name, sh_type, addr, offset, size, link, entsize });
        }
        let extent = u32::try_from(extent).or_else(|_| invalid("ELF extent overflows"))?;
        if extent as usize > data.len() { return invalid(format!("ELF extent {extent:#x} beyond the {:#x} bytes given", data.len())); }
        Ok(Elf { entry, phdrs, sections, extent })
    }

    /// The ELF's total size, from the header alone (needs only the first `max(phoff+phnum*32, shoff+shnum*40)`
    /// bytes plus the section headers). Used to cut an ELF out of raw sectors.
    pub fn extent_of(data: &[u8]) -> Result<u32> { Ok(Elf::parse(data)?.extent) }

    pub fn section(&self, name: &str) -> Option<&Section> { self.sections.iter().find(|s| s.name == name) }

    /// The `$gp` value from `.reginfo` (offset 20), if present and non-zero.
    pub fn gp(&self, data: &[u8]) -> Result<Option<u32>> {
        let Some(s) = self.section(".reginfo") else { return Ok(None) };
        let gp = Buf(data).u32(s.offset as usize + 20)?;
        Ok((gp != 0).then_some(gp))
    }

    /// Every `.symtab` entry (index 0, the null symbol, included), names resolved through the linked string table.
    pub fn symbols(&self, data: &[u8]) -> Result<Vec<Symbol>> {
        let Some(st) = self.sections.iter().find(|s| s.sh_type == SHT_SYMTAB) else { return Ok(Vec::new()) };
        let Some(strs) = self.sections.get(st.link as usize) else { return invalid(".symtab sh_link out of range") };
        let b = Buf(data);
        let mut out = Vec::with_capacity(st.size as usize / 16);
        for i in 0..st.size as usize / 16 {
            let o = st.offset as usize + i * 16;
            let name = cstr(&b, strs.offset as usize + b.u32(o)? as usize)?;
            out.push(Symbol { name, value: b.u32(o + 4)?, size: b.u32(o + 8)?, info: b.u8(o + 12)?, shndx: b.u16(o + 14)? });
        }
        Ok(out)
    }

    /// The loaded image: PT_LOAD segments mapped to `(vaddr, file bytes, memsz)`.
    pub fn load_segments<'a>(&self, data: &'a [u8]) -> Result<Vec<(u32, &'a [u8], u32)>> {
        let b = Buf(data);
        self.phdrs.iter().filter(|p| p.p_type == PT_LOAD)
            .map(|p| Ok((p.vaddr, b.sub(p.offset as usize, p.filesz as usize, "PT_LOAD data")?.bytes(), p.memsz)))
            .collect()
    }
}

fn cstr(b: &Buf, o: usize) -> Result<String> {
    let tail = b.tail(o, "string table")?.bytes();
    let n = tail.iter().position(|&c| c == 0).ok_or_else(|| crate::buf::FormatError::Invalid("unterminated string".into()))?;
    Ok(String::from_utf8_lossy(&tail[..n]).into_owned())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A minimal ELF: one PT_LOAD of 8 bytes at 0x100000, sections: null, .text, .reginfo, .symtab, .strtab, .shstrtab.
    pub(crate) fn mini_elf() -> Vec<u8> {
        let mut d = vec![0u8; 0x200];
        d[..4].copy_from_slice(b"\x7fELF");
        d[4] = 1; d[5] = 1; d[6] = 1;
        let w16 = |d: &mut Vec<u8>, o: usize, v: u16| d[o..o + 2].copy_from_slice(&v.to_le_bytes());
        let w32 = |d: &mut Vec<u8>, o: usize, v: u32| d[o..o + 4].copy_from_slice(&v.to_le_bytes());
        w16(&mut d, 16, 2); w16(&mut d, 18, 8);
        w32(&mut d, 24, 0x100004); w32(&mut d, 28, 52); w32(&mut d, 32, 0x100);
        w16(&mut d, 42, 32); w16(&mut d, 44, 1); w16(&mut d, 46, 40); w16(&mut d, 48, 6); w16(&mut d, 50, 5);
        // PT_LOAD: offset 0x80, vaddr 0x100000, filesz 8, memsz 0x10
        for (i, v) in [1u32, 0x80, 0x100000, 0x100000, 8, 0x10, 7, 0x10].iter().enumerate() { w32(&mut d, 52 + i * 4, *v); }
        d[0x80..0x88].copy_from_slice(&[0x08, 0x00, 0xe0, 0x03, 0, 0, 0, 0]);   // jr ra; nop
        // .reginfo at 0x88 (24 bytes), gp at +20
        w32(&mut d, 0x88 + 20, 0x108000);
        // .symtab at 0xa0: null + one FUNC "f" at 0x100000 size 8
        w32(&mut d, 0xb0, 1); w32(&mut d, 0xb4, 0x100000); w32(&mut d, 0xb8, 8); d[0xbc] = 0x12; w16(&mut d, 0xbe, 1);
        // .strtab at 0xc0: "\0f\0"
        d[0xc1] = b'f';
        // .shstrtab at 0xd0
        let names = b"\0.text\0.reginfo\0.symtab\0.strtab\0.shstrtab\0";
        d[0xd0..0xd0 + names.len()].copy_from_slice(names);
        let sh = |d: &mut Vec<u8>, i: usize, f: [u32; 10]| for (k, v) in f.iter().enumerate() { d[0x100 + i * 40 + k * 4..0x100 + i * 40 + k * 4 + 4].copy_from_slice(&v.to_le_bytes()) };
        sh(&mut d, 1, [1, 1, 6, 0x100000, 0x80, 8, 0, 0, 4, 0]);
        sh(&mut d, 2, [7, 0x7000_0006, 2, 0, 0x88, 24, 0, 0, 4, 24]);
        sh(&mut d, 3, [16, 2, 0, 0, 0xa0, 32, 4, 1, 4, 16]);
        sh(&mut d, 4, [24, 3, 0, 0, 0xc0, 3, 0, 0, 1, 0]);
        sh(&mut d, 5, [32, 3, 0, 0, 0xd0, names.len() as u32, 0, 0, 1, 0]);
        d.truncate(0x100 + 6 * 40);
        d
    }

    #[test]
    fn parses_a_synthetic_elf() {
        let d = mini_elf();
        let e = Elf::parse(&d).unwrap();
        assert_eq!(e.entry, 0x100004);
        assert_eq!(e.extent as usize, d.len());
        assert_eq!(e.sections.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["", ".text", ".reginfo", ".symtab", ".strtab", ".shstrtab"]);
        assert_eq!(e.gp(&d).unwrap(), Some(0x108000));
        let syms = e.symbols(&d).unwrap();
        assert_eq!(syms.len(), 2);
        assert_eq!((syms[1].name.as_str(), syms[1].value, syms[1].kind(), syms[1].binding()), ("f", 0x100000, 2, 1));
        let segs = e.load_segments(&d).unwrap();
        assert_eq!(segs, [(0x100000, &d[0x80..0x88], 0x10)]);
    }

    #[test]
    fn extent_includes_trailing_bytes_check() {
        // Extra bytes after the ELF don't change the extent; a truncated ELF is rejected.
        let mut d = mini_elf();
        let n = d.len();
        d.extend([0xAA; 100]);
        assert_eq!(Elf::extent_of(&d).unwrap() as usize, n);
        assert!(Elf::parse(&d[..n - 1]).is_err());
    }

    #[test]
    fn rejects_non_mips_and_bad_magic() {
        let mut d = mini_elf();
        d[18] = 3;
        assert!(Elf::parse(&d).is_err());
        let mut d = mini_elf();
        d[0] = 0;
        assert!(Elf::parse(&d).is_err());
    }
}
