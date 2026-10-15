//! The `.crp` CARP container — the driving side's core resource format
//! (`data\track\<level>.crp` inside every mission `.VIV`, `data\car\model\*.crp`,
//! `data\actors\weapons\*.crp`; docs/formats/crp.md).
//!
//! Every `.crp` is RefPack-compressed (`10fb`, `nf_formats::refpack`) or, for the five `MISC.VIV`
//! cars, plain; the decompressed bytes are a flat pool of **16-byte records** [M DRIVING.ELF,
//! EE = file offset + 0x107080]:
//!
//! ```text
//! +0x0  u32 tag      a big-endian 4-char code ("CARP", "Arti", "ELFd", …) — the byte order is
//!                    visible in the compare constants: "DBN " is 0x44424e20 [M 0x22f898],
//!                    "Sect" 0x53656374, "ns  " 0x736e2020 [M 0x1a9600]
//! +0x4  u32 flags    bit 0x01: short tag — the low 16 bits are an instance index and compares
//!                              normalize the low half to 0x2020 [M 0x1a9600 lines 87-95]
//!                    bit 0x02: the reference is relative to this record's own address —
//!                              **table references scale `dref` by 0x10 (record units)**
//!                              [M 0x2d6940: `sll $v0,$v0,4` = 0x00021100, then `addu`], while
//!                              **payload references use raw bytes** [M 0x1a9600/0x22f898 inline
//!                              `record + dref`]; bit clear: absolute file offset
//!                    bits 5+:  the record's scan-window skip, in records [M 0x2d5a10/0x2d5920]
//!                    bits 8+:  the payload size in bytes [M 0x1a9600: (flags >> 8) as the ELF size]
//! +0x8  i32 count    the record's scan-window length, in records [M 0x2d5a10]
//! +0xc  i32 dref     table/payload reference (see flags bit 0x02)
//! ```
//!
//! The **root record** is tagged `CARP`; its window is the chunk directory and the records before
//! the window are the pool. Consumers:
//! - the track loader `FUN_0022f898` 0x22f898 scans the root window for `"DBN "` and registers the
//!   DBN payload string (the track name) [M];
//! - the module loader `FUN_001a9600` 0x1a9600 walks `"ns  "` records (instance-indexed), loads
//!   `"ELFd"` as a 32-bit little-endian MIPS object and applies `"ELFr"` relocations
//!   (`FUN_0029f7c8`/`FUN_0029f820`), then `FUN_002daa18` consumes `"Sect"` [M];
//! - `FUN_002dc1b0` 0x2dc1b0 names the pool records: `"Arti"` articles carry a nested `"Name"`
//!   record; `"Shar"` = shared, `"CDat"` = collision, `"RNgp"` = road network, `"Map "` = map [M].
//!
//! This port mirrors the game's arithmetic with bounds checks; malformed data errors instead of
//! overrunning (the game trusts its data [M]).

use crate::buf::{invalid, FormatError, Result};

/// The root record's tag, `CARP` as a big-endian 4-char code (measured over all 89 `.crp` entries
/// of the 23 driving archives, docs/formats/crp.md).
pub const CARP: u32 = 0x4341_5250;

/// Tag constants as the game's compare constants spell them (big-endian 4-char codes) [M].
pub mod tag {
    /// `Arti` — an article [M 0x2dc1b0].
    pub const ARTI: u32 = 0x4172_7469;
    /// `Name` — an article's name record [M 0x2dc1b0 line 98].
    pub const NAME: u32 = 0x4e61_6d65;
    /// `Shar` — shared data [M 0x2dc1b0].
    pub const SHAR: u32 = 0x5368_6172;
    /// `CDat` — collision data [M 0x2dc1b0].
    pub const CDAT: u32 = 0x4344_6174;
    /// `RNgp` — road network [M 0x2dc1b0].
    pub const RNGP: u32 = 0x524e_6770;
    /// `Map ` — map data [M 0x2dc1b0].
    pub const MAP: u32 = 0x4d61_7020;
    /// `DBN ` — the track-name record the track loader scans for [M 0x22f898].
    pub const DBN: u32 = 0x4442_4e20;
    /// `ELFd` — the embedded ELF object [M 0x1a9600].
    pub const ELFD: u32 = 0x454c_4664;
    /// `ELFr` — the ELF's relocations [M 0x1a9600].
    pub const ELFR: u32 = 0x454c_4672;
    /// `MapN` — map name [M, present in the track root windows].
    pub const MAPN: u32 = 0x4d61_704e;
    /// `Sect` — sections, consumed after the module load [M 0x1a9600 → 0x2daa18].
    pub const SECT: u32 = 0x5365_6374;
    /// `ns  ` — instance-indexed sub-resources [M 0x1a9600].
    pub const NS: u32 = 0x736e_2020;
}

/// One 16-byte record, parsed little-endian (the fields are `lw`-loaded; only the tag is
/// interpreted as a big-endian character code) [M].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub tag: u32,
    pub flags: u32,
    pub count: i32,
    pub dref: i32,
    /// The record's byte offset inside the file (the base for relative references).
    pub offset: usize,
}

impl Record {
    /// Parses one 16-byte record (public for the mesh layer's window walks, `crp_mesh`).
    pub fn parse(data: &[u8], offset: usize) -> Result<Record> {
        let b = data.get(offset..offset + 16)
            .ok_or_else(|| FormatError::Invalid(format!("record at {offset:#x} past the end ({})", data.len())))?;
        let le = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        Ok(Record { tag: le(0), flags: le(4), count: le(8) as i32, dref: le(12) as i32, offset })
    }

    /// The tag as the 4 bytes a big-endian reading spells (`"DBN "`, `"ELFd"`, …).
    pub fn tag_bytes(&self) -> [u8; 4] { self.tag.to_be_bytes() }

    /// The compare-normalized tag code: short tags (flags bit 0x01) have their low half replaced
    /// with 0x2020 (two spaces) [M 0x1a9600 lines 87-95].
    pub fn tag_code(&self) -> u32 {
        if self.flags & 1 != 0 { (self.tag & 0xffff_0000) | 0x2020 } else { self.tag }
    }

    /// The tag's instance index (flags bit 0x01) [M 0x1a9600: (u16)record[0]].
    pub fn instance(&self) -> Option<u16> {
        (self.flags & 1 != 0).then_some(self.tag as u16)
    }

    /// The record's scan-window skip in records [M 0x2d5a10: (flags >> 5) * 0x10].
    pub fn skip(&self) -> usize { (self.flags >> 5) as usize * 0x10 }

    /// The payload size in bytes [M 0x1a9600: (flags >> 8) as the ELF size].
    pub fn size(&self) -> usize { (self.flags >> 8) as usize }

    /// The table base the record points at — the game's `FUN_002d6940`: `record + dref*0x10` when
    /// flags bit 0x02 is set (dref counts 16-byte records), else the absolute `dref`
    /// [M 0x2d6940 disassembly: lw 4($a0); andi $v0,2; beq; lw 12($a0); `sll $v0,$v0,4`
    /// (0x00021100); addu $v0,$a0,$v0].
    pub fn table(&self, data: &[u8]) -> Result<usize> {
        let at = if self.flags & 2 != 0 {
            let signed = self.dref as isize;
            if signed < 0 {
                return invalid(format!("record at {:#x}: negative relative dref {}", self.offset, self.dref));
            }
            self.offset + (signed as usize) * 0x10
        } else {
            (self.dref as isize).max(0) as usize
        };
        if at > data.len() {
            return invalid(format!("record at {:#x}: table dref {:#x} -> {at:#x} past the end ({})",
                self.offset, self.dref, data.len()));
        }
        Ok(at)
    }

    /// The payload address — the game's inline resolution in the chunk consumers: `record + dref`
    /// in raw bytes when flags bit 0x02 is set, else the absolute `dref`
    /// [M 0x1a9600 `(int)puVar1 + puVar1[3]` / 0x22f898 `iVar2 + *(int *)(iVar2 + 0xc)`].
    pub fn data_addr(&self, data: &[u8]) -> Result<usize> {
        let at = if self.flags & 2 != 0 {
            let signed = self.dref as isize;
            if signed < 0 {
                return invalid(format!("record at {:#x}: negative relative dref {}", self.offset, self.dref));
            }
            self.offset + signed as usize
        } else {
            (self.dref as isize).max(0) as usize
        };
        if at > data.len() {
            return invalid(format!("record at {:#x}: data dref {:#x} -> {at:#x} past the end ({})",
                self.offset, self.dref, data.len()));
        }
        Ok(at)
    }

    /// The scan window this record points at: `[table + skip, table + skip + count*16)`
    /// [M 0x2d5a10/0x2d5920]. Returns `(start, end)` as byte offsets.
    pub fn window(&self, data: &[u8]) -> Result<(usize, usize)> {
        let base = self.table(data)?;
        let start = base + self.skip();
        let end = start + (self.count as usize) * 0x10;
        Ok((start, end))
    }
}

/// A parsed `.crp`: the file bytes plus the root `CARP` record.
pub struct Crp<'a> {
    pub data: &'a [u8],
    pub root: Record,
}

impl<'a> Crp<'a> {
    /// Parses the root record; the tag must be `CARP`.
    pub fn parse(data: &'a [u8]) -> Result<Crp<'a>> {
        let root = Record::parse(data, 0)?;
        if root.tag != CARP {
            return invalid(format!("not a CARP container: tag {:#010x} ({:?})", root.tag,
                String::from_utf8_lossy(&root.tag_bytes())));
        }
        Ok(Crp { data, root })
    }

    /// The records of a window `[start, end)`, bounds-checked.
    pub fn window_records(&self, start: usize, end: usize) -> Result<Vec<Record>> {
        if start > end || end > self.data.len() {
            return invalid(format!("window [{start:#x}..{end:#x}) past the end ({})", self.data.len()));
        }
        (start..end).step_by(0x10).map(|o| Record::parse(self.data, o)).collect()
    }

    /// The pool: the records between the root's target and the root's window [M 0x2d5a10 — the
    /// window starts `skip` records in; everything before it in the same table is the pool].
    pub fn pool(&self) -> Result<Vec<Record>> {
        let base = self.root.table(self.data)?;
        let end = base + self.root.skip();
        if end > self.data.len() {
            return invalid(format!("pool [{base:#x}..{end:#x}) past the end ({})", self.data.len()));
        }
        (base..end).step_by(0x10).map(|o| Record::parse(self.data, o)).collect()
    }

    /// The root's chunk directory (the game's root scan window) [M 0x22f898/0x2d5a10].
    pub fn directory(&self) -> Result<Vec<Record>> {
        let (start, end) = self.root.window(self.data)?;
        self.window_records(start, end)
    }

    /// Exact-tag find over the root window — the game's `FUN_002d5a10` semantics: the raw tag
    /// word must equal `tag` [M 0x2d5a10].
    pub fn find(&self, tag: u32) -> Result<Option<Record>> {
        for r in self.directory()? {
            if r.tag == tag { return Ok(Some(r)); }
        }
        Ok(None)
    }

    /// Instance-indexed find — the game's `FUN_002d5920` semantics with an explicit index bound:
    /// the normalized code must match and the record's index must be ≤ `max_index` [M 0x2d5920].
    pub fn find_instance(&self, code: u32, max_index: u32) -> Result<Option<Record>> {
        for r in self.directory()? {
            let t = r.tag_code();
            if t == code && (r.tag as u32 & 0xffff) <= max_index { return Ok(Some(r)); }
        }
        Ok(None)
    }

    /// A record's payload: `size = flags >> 8` bytes at the resolved address [M 0x1a9600].
    pub fn payload(&self, rec: &Record) -> Result<&'a [u8]> {
        let at = rec.data_addr(self.data)?;
        let end = at + rec.size();
        if end > self.data.len() {
            return invalid(format!("record {:#x} ({:?}): payload [{at:#x}..{end:#x}) past the end ({})",
                rec.offset, String::from_utf8_lossy(&rec.tag_bytes()), self.data.len()));
        }
        Ok(&self.data[at..end])
    }

    /// The `"DBN "` payload as the track name — what the track loader registers [M 0x22f898:
    /// scans the root window for 0x44424e20, takes the payload, `strlen` + register].
    pub fn track_name(&self) -> Result<Option<&'a str>> {
        match self.find(tag::DBN)? {
            None => Ok(None),
            Some(rec) => {
                let p = self.payload(&rec)?;
                let s = p.split(|&b| b == 0).next().unwrap_or(&[]);
                Ok(Some(std::str::from_utf8(s).map_err(|_| {
                    FormatError::Invalid("DBN payload is not UTF-8".into())
                })?))
            }
        }
    }

    /// An article's name — the `"Name"` record inside the article's own window, payload string
    /// [M 0x2dc1b0 lines 97-108]. Special pool records (`Map `, `Shar`, `CDat`, `RNgp`) have no
    /// name record; the game labels them `<<Map>>`/`<<Shared>>`/`<<Collision>>`/`<<RoadNetwork>>`.
    pub fn article_name(&self, arti: &Record) -> Result<Option<&'a str>> {
        let (start, end) = arti.window(self.data)?;
        for o in (start..end).step_by(0x10) {
            let rec = Record::parse(self.data, o)?;
            if rec.tag == tag::NAME {
                let p = self.payload(&rec)?;
                let s = p.split(|&b| b == 0).next().unwrap_or(&[]);
                return std::str::from_utf8(s).map(Some).map_err(|_| {
                    FormatError::Invalid("Name payload is not UTF-8".into())
                });
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a little container: root + pool of 1 Arti (with a Name) + directory of
    /// [DBN, ns#3]. Layout (offsets in bytes):
    /// 0x00 root, 0x10 Arti, 0x20..0x40 directory [DBN, ns#3], 0x40 Name,
    /// 0x60 the DBN string payload, 0x70 the ns payload, 0x80 the Name string payload.
    fn container() -> Vec<u8> {
        let mut d = vec![0u8; 0x90];
        let put = |d: &mut Vec<u8>, o: usize, tag: u32, flags: u32, count: i32, dref: i32| {
            d[o..o + 4].copy_from_slice(&tag.to_le_bytes());
            d[o + 4..o + 8].copy_from_slice(&flags.to_le_bytes());
            d[o + 8..o + 12].copy_from_slice(&(count as u32).to_le_bytes());
            d[o + 12..o + 16].copy_from_slice(&(dref as u32).to_le_bytes());
        };
        // Root: table base = 0x10 via the RELATIVE path (dref = 1 record × 0x10 — the game's
        // FUN_002d6940 scaling); skip = 1 record (the Arti pool);
        // count = 2 directory records -> window [0x20..0x40).
        put(&mut d, 0x00, CARP, 2 | (1 << 5), 2, 1);
        // Arti at 0x10: absolute window at 0x40, 1 record ("Name").
        put(&mut d, 0x10, tag::ARTI, 0, 1, 0x40);
        // DBN at 0x20: payload = absolute 0x60, size = 6 ("paris\0").
        put(&mut d, 0x20, tag::DBN, 6 << 8, 0, 0x60);
        // ns#3 at 0x30: short tag (flags bit 1), payload relative to the record
        // (0x30 + 0x40 = 0x70), size 4.
        put(&mut d, 0x30, (tag::NS & 0xffff_0000) | 3, 1 | 2 | (4 << 8), 0, 0x40);
        // Name at 0x40: payload = absolute 0x80, size = 5 ("Bond\0").
        put(&mut d, 0x40, tag::NAME, 5 << 8, 0, 0x80);
        d[0x60..0x66].copy_from_slice(b"paris\0");
        d[0x70..0x74].copy_from_slice(b"NOIS");
        d[0x80..0x85].copy_from_slice(b"Bond\0");
        d
    }

    #[test]
    fn parses_root_pool_and_directory() {
        let d = container();
        let c = Crp::parse(&d).unwrap();
        assert_eq!(c.root.tag_code(), CARP);
        let pool = c.pool().unwrap();
        assert_eq!(pool.len(), 1);
        assert_eq!(pool[0].tag, tag::ARTI);
        let dir = c.directory().unwrap();
        assert_eq!(dir.len(), 2);
        assert_eq!(dir[0].tag, tag::DBN);
        assert_eq!(dir[0].offset, 0x20);
        assert_eq!(dir[1].tag_code(), tag::NS);
        assert_eq!(dir[1].instance(), Some(3));
    }

    #[test]
    fn resolves_absolute_and_relative_payloads() {
        let d = container();
        let c = Crp::parse(&d).unwrap();
        let dir = c.directory().unwrap();
        // DBN: absolute payload "paris".
        assert_eq!(c.payload(&dir[0]).unwrap(), b"paris\0");
        // ns: relative payload (record 0x30 + dref 0x40 = 0x70) "NOIS".
        assert_eq!(c.payload(&dir[1]).unwrap(), b"NOIS");
        // And the finders agree with the walk.
        assert_eq!(c.find(tag::DBN).unwrap().unwrap().offset, 0x20);
        assert_eq!(c.find_instance(tag::NS, u32::MAX).unwrap().unwrap().instance(), Some(3));
        assert!(c.find_instance(tag::NS, 2).unwrap().is_none(), "index 3 > bound 2 must not match");
        // track_name cuts at the NUL.
        assert_eq!(c.track_name().unwrap(), Some("paris"));
        // article_name resolves the nested Name record.
        let arti = &c.pool().unwrap()[0];
        assert_eq!(c.article_name(arti).unwrap(), Some("Bond"));
    }

    #[test]
    fn rejects_bad_magic_and_out_of_bounds() {
        let mut d = container();
        d[3] = b'X'; // no longer CARP
        assert!(Crp::parse(&d).is_err());
        // A payload that runs past the end must error, not panic.
        let mut bad = container();
        bad[0x25] = 0xff; // DBN size = 0xff06
        let c2 = Crp::parse(&bad).unwrap();
        let dir2 = c2.directory().unwrap();
        assert!(c2.payload(&dir2[0]).is_err());
        // A dref past the end must error.
        let mut bad2 = container();
        bad2[0x2c..0x30].copy_from_slice(&0x7fffffu32.to_le_bytes());
        let c3 = Crp::parse(&bad2).unwrap();
        assert!(c3.payload(&c3.directory().unwrap()[0]).is_err());
    }
}
