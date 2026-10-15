//! Track-mesh layer over the CARP container (`docs/formats/crp.md`; task `drive-track-mesh`).
//!
//! What this module decodes, all measured on our disc data (the 8 mission `.crp` tracks) and
//! matched to DRIVING.ELF code (EE = file offset + 0x107080):
//!
//! 1. **Articles** — the pool `Arti` records are the track's geometry/deco pieces. Each article's
//!    window holds: a `Base` record whose payload carries the piece's bounding box (float min at
//!    +0x10, max at +0x20 [D `S6_CustomLowRBnknRoad` measured −10.06/−7.5/−10.06 … 18.33/0.5/15.0;
//!    consumer semantics [S] Raikaru/nightfire-engine docs/driving.md]), a `Name` string record
//!    [M 0x2dc1b0], `as#N` records whose payload ends in the 4-byte word `{u16 sr_index, "sr"}`
//!    (little-endian) — the part's render reference [D measured: `as#0` tail `05 00 72 73` →
//!    `sr#5` = `EAGL::S6_CustomLowRBnknRoad::{as  0000}::Model[0].00`], `ca#0` (per-article
//!    collision: 256-7040 B float quads + packed words [D measured; layout next phase]), `in#0/#1`
//!    (64 B transforms, cnt=1) and `mp#0` (4 B).
//! 2. **The embedded module** — `ELFd` (head: ELF header + `.data`) + `ELFr` (continuation:
//!    section-name strings, `.strtab`, `.symtab`, `.rel.data`, section headers) form ONE MIPS32
//!    ET_REL image, split at the ELFd record's payload size [M 0x1a9600 lines 143-150 passes
//!    `(ELFd_ptr, ELFd_size, ELFr_ptr)` into `FUN_0029f820`; the resolver `FUN_0029ea00` 0x29ea00
//!    resolves every section `sh_addr` and `e_shoff` as
//!    `x < elfd_size ? ELFd+x : ELFr + (x - elfd_size)`]. Measured on paris (MIS01.VIV#4): 6
//!    sections; `e_shoff` 0x4642b0 lands exactly on the last 0xF0 bytes of the combined image;
//!    sections `.data` (0x3be7d0) + `.shstrtab` + `.strtab` + `.symtab` (3993 syms) + `.rel.data`
//!    (54,613 R_MIPS_32 relocs) [D].
//! 3. **Model symbols** — the `.symtab` carries `__Model:::<article>::{as  0000}::Model[0].NN`
//!    names (paris: 1,890) whose `st_value` is a flat offset into the combined image [D: the
//!    relocations targeting those slots use the `.data` section symbol (st_value 0), so the stored
//!    word IS the flat offset — verified by following `S12_LowRiverBnk_n_Road_00 …Model[0].1d`'s
//!    slot to a model-node object whose +0xC holds the segment index 0x1C and +0x18 a
//!    segment-list pointer into a shared array of `{value, 0xA000FFFF}` records referencing
//!    `__GPRenderMethod_<article>::…::Model[0].NN_<x>_name` strings]. Applying `.rel.data` (every
//!    entry `R_MIPS_32`: `word += symbol.st_value` [M `EAGL::DynamicLoader::Resolve` 0x2bb300;
//!    measured: 54,613/54,613 paris entries are type 2]) reconstructs the loaded image. The render
//!    geometry itself (the VIF chains the EAGL engine builds from these objects at load time) is
//!    the next phase's work — this module pins everything that walk needs.
//! 4. **Collision** — the pool `CDat` article is the collision world: window = `CGrd` (36 B grid
//!    header) + `ci#0` (63,616 B, count=994 items on paris) + ~1600 `cn#N` node records (38-88 B)
//!    [D measured]. `RNgp` is the road network, `Map ` the mission scene database (AICo/AIEl/AISp/
//!    Trgr/…/aq#N placement arrays) [D]. Their binary layouts are the next phase's work.

use crate::buf::{invalid, FormatError, Result};
use crate::crp::{tag, Crp, Record};

/// `Base` — an article's bounding-box record [D measured].
pub const TAG_BASE: u32 = 0x4261_7365;
/// `as  ` — a part reference record; short-tag instances carry the index in the low half [D].
pub const TAG_AS: u32 = 0x6173_2020;
/// `sr  ` — a named resource reference (payload = the EAGL symbol string) [D].
pub const TAG_SR: u32 = 0x7372_2020;
/// `CGrd` — the CDat collision-grid header record [D measured].
pub const TAG_CGRD: u32 = 0x4347_5264;
/// `ci  ` — CDat collision items [D measured].
pub const TAG_CI: u32 = 0x6369_2020;
/// `cn  ` — CDat collision nodes [D measured].
pub const TAG_CN: u32 = 0x636e_2020;

/// A parsed track article: name + bbox + the part references [D].
#[derive(Clone, Debug, PartialEq)]
pub struct ArticleMeta {
    /// The article's `Name` payload (`S6_CustomLowRBnknRoad`, `sky02`, …) [M 0x2dc1b0].
    pub name: String,
    /// `Base` payload floats: min at +0x10, max at +0x20 [D measured].
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    /// The article's `as#N` part references [D measured].
    pub parts: Vec<PartRef>,
}

/// One `as#N` record's render reference: the `sr#<index>` record in the same article window names
/// the EAGL resource (`EAGL::<article>::{as  0000}::Model[0].NN`) [D measured]. The tail word is
/// `{u16 sr_index, "sr"}` on the geometry pieces [D: `as#0`→`sr#5`, `as#1`→`sr#6` on
/// `S6_CustomLowRBnknRoad`]; other articles carry `as` records whose tail does not end in "sr"
/// [D measured: MIS01 `trafficlight` as#1 tail `00 02 05 00`] — those get `sr_index: None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartRef {
    /// The `as` record's instance index.
    pub as_instance: u16,
    /// The `sr` record's instance index inside the same article window, when the payload ends in
    /// the `{u16, "sr"}` reference form [D measured].
    pub sr_index: Option<u16>,
}

fn u32_at(data: &[u8], o: usize) -> Result<u32> {
    let b = data.get(o..o + 4).ok_or_else(|| {
        FormatError::Invalid(format!("u32 read at {o:#x} past the end ({})", data.len()))
    })?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn f32_at(data: &[u8], o: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(data, o)?))
}

fn payload_string(data: &[u8]) -> String {
    let s = data.split(|&b| b == 0).next().unwrap_or(&[]);
    String::from_utf8_lossy(s).into_owned()
}

impl<'a> Crp<'a> {
    /// Every pool article's mesh metadata: name, bounding box, part references. Errors only on
    /// malformed records (the game's data always carries `Base`+`Name` [D, all 8 tracks]).
    pub fn track_articles(&self) -> Result<Vec<ArticleMeta>> {
        let mut out = Vec::new();
        for arti in self.pool()? {
            if arti.tag != tag::ARTI {
                continue;
            }
            let (start, end) = arti.window(self.data)?;
            let mut meta = ArticleMeta {
                name: String::new(),
                bbox_min: [0.0; 3],
                bbox_max: [0.0; 3],
                parts: Vec::new(),
            };
            for o in (start..end).step_by(0x10) {
                let rec = Record::parse(self.data, o)?;
                if rec.tag == tag::NAME {
                    meta.name = payload_string(self.payload(&rec)?);
                } else if rec.tag == TAG_BASE {
                    let p = self.payload(&rec)?;
                    if p.len() < 0x2c {
                        return invalid(format!(
                            "article {}: Base payload {} < 0x2c bytes",
                            meta.name,
                            p.len()
                        ));
                    }
                    for i in 0..3 {
                        meta.bbox_min[i] = f32_at(p, 0x10 + i * 4)?;
                        meta.bbox_max[i] = f32_at(p, 0x20 + i * 4)?;
                    }
                } else if rec.tag_code() == TAG_AS {
                    let p = self.payload(&rec)?;
                    if p.len() < 4 {
                        return invalid(format!(
                            "article {}: as#{} payload {} < 4 bytes",
                            meta.name,
                            rec.instance().unwrap_or(0),
                            p.len()
                        ));
                    }
                    let tail = u32_at(p, p.len() - 4)?;
                    // The tail word is {u16 sr_index, "sr"} on the geometry pieces [D measured:
                    // 05 00 72 73]; other articles' `as` records carry other tails [D: MIS01
                    // `trafficlight` as#1 = 00 02 05 00] — recorded without the sr reference.
                    let sr_index = (tail >> 16 == 0x7372).then_some(tail as u16);
                    meta.parts.push(PartRef {
                        as_instance: rec.instance().unwrap_or(0),
                        sr_index,
                    });
                }
            }
            out.push(meta);
        }
        Ok(out)
    }

    /// An article window's `sr#<index>` payload string (`EAGL::<name>::{as  0000}::Model[0].NN`)
    /// — the EAGL symbol reference a `PartRef` points at [D measured].
    pub fn article_sr_string(&self, arti: &Record, index: u16) -> Result<Option<String>> {
        let (start, end) = arti.window(self.data)?;
        for o in (start..end).step_by(0x10) {
            let rec = Record::parse(self.data, o)?;
            if rec.tag_code() == TAG_SR && rec.instance() == Some(index) {
                return Ok(Some(payload_string(self.payload(&rec)?)));
            }
        }
        Ok(None)
    }

    /// The pool `CDat` article's record census — the collision world's top structure [D measured
    /// on paris: `CGrd` 36 B, `ci#0` 63,616 B with count=994 items, ~1600 `cn#N` nodes].
    pub fn collision_census(&self) -> Result<CollisionCensus> {
        let mut out = CollisionCensus {
            cdat_found: false,
            grd_records: 0,
            ci_records: 0,
            ci_total_items: 0,
            cn_records: 0,
        };
        for rec in self.pool()? {
            if rec.tag != tag::CDAT {
                continue;
            }
            out.cdat_found = true;
            let (start, end) = rec.window(self.data)?;
            for o in (start..end).step_by(0x10) {
                let r = Record::parse(self.data, o)?;
                match r.tag_code() {
                    TAG_CGRD => out.grd_records += 1,
                    TAG_CI => {
                        out.ci_records += 1;
                        out.ci_total_items += r.count.max(0) as usize;
                    }
                    TAG_CN => out.cn_records += 1,
                    _ => {}
                }
            }
        }
        Ok(out)
    }
}

/// The pool `CDat` article's record census [D measured].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CollisionCensus {
    pub cdat_found: bool,
    pub grd_records: usize,
    pub ci_records: usize,
    /// Sum of the `ci` records' `count` fields (paris: 994) [D measured].
    pub ci_total_items: usize,
    pub cn_records: usize,
}

/// The combined `ELFd`+`ELFr` MIPS module of one `.crp`, with the game's address resolution and
/// relocations applied [M 0x1a9600 → 0x29f820 → 0x29ea00].
pub struct TrackModule {
    /// The combined image bytes (ELFd payload, then ELFr payload), relocations applied.
    pub image: Vec<u8>,
    /// The ELFd payload size — the split the resolver uses [M 0x1a9600/0x29ea00].
    pub elfd_size: u32,
    /// Section headers in ELF order (offset = the game's resolved `sh_addr` [M 0x29ea00]).
    pub sections: Vec<ModuleSection>,
    /// Symbols in ELF order (name, flat st_value, size, info, shndx).
    pub symbols: Vec<ModuleSymbol>,
    /// Number of relocations applied.
    pub relocations: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSection {
    pub name: String,
    pub sh_type: u32,
    /// The game's resolved `sh_addr` — a runtime pointer in the combined-image space
    /// [M 0x29ea00 resolves `sh_addr`; the loader never uses `sh_offset` for addressing].
    pub offset: u32,
    /// The section's file offset (`sh_offset`) into the combined image — where the DATA lives
    /// [D measured: paris `.symtab` sh_offset 0x3e9e70 holds the symbol table the loader reads].
    pub data_offset: u32,
    pub size: u32,
    pub link: u32,
    pub entsize: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSymbol {
    pub name: String,
    /// Flat value in the combined-image space [D verified via relocation targets].
    pub value: u32,
    pub size: u32,
    pub info: u8,
    pub shndx: u16,
}

impl ModuleSymbol {
    /// Object-file binding/type nibbles.
    pub fn kind(&self) -> u8 { self.info & 0xf }
    pub fn binding(&self) -> u8 { self.info >> 4 }
}

impl TrackModule {
    /// Loads the module: `ELFd`+`ELFr` concatenated, section offsets resolved by the game's rule,
    /// symbols read, then every `.rel.data` entry applied (`R_MIPS_32`: `word += symbol value`)
    /// [M 0x29ea00 + 0x2bb300; measured: every entry of all 8 mission tracks is type 2].
    pub fn load(crp: &Crp<'_>) -> Result<TrackModule> {
        let dir = crp.directory()?;
        let elfd = dir
            .iter()
            .find(|r| r.tag == tag::ELFD)
            .ok_or_else(|| FormatError::Invalid("no ELFd record in the root window".into()))?;
        let elfd_size = elfd.size() as u32;
        let mut image = crp.payload(elfd)?.to_vec();
        let elfr = dir
            .iter()
            .find(|r| r.tag == tag::ELFR)
            .ok_or_else(|| FormatError::Invalid("no ELFr record in the root window".into()))?;
        image.extend_from_slice(crp.payload(elfr)?);

        if image.len() < 0x34 || &image[..4] != b"\x7fELF" {
            return invalid(format!("combined module image starts with {:?} ({})", &image[..4.min(image.len())], image.len()));
        }
        let e_shoff = u32_at(&image, 0x20)? as usize;
        let e_shentsize = u16::from_le_bytes([image[0x2e], image[0x2f]]) as usize;
        let e_shnum = u16::from_le_bytes([image[0x30], image[0x31]]) as usize;
        let shstrndx = u16::from_le_bytes([image[0x32], image[0x33]]) as usize;
        if e_shentsize != 0x28 {
            return invalid(format!("unexpected e_shentsize {e_shentsize:#x} (measured 0x28)"));
        }
        // The resolver: x < elfd_size ? ELFd+x : ELFr+(x-elfd_size) [M 0x29ea00].
        let resolve = |x: u32| -> Result<usize> {
            let at = if x < elfd_size { x as usize } else { elfd_size as usize + (x - elfd_size) as usize };
            if at > image.len() {
                return invalid(format!(
                    "resolved offset {at:#x} past the combined image ({})",
                    image.len()
                ));
            }
            Ok(at)
        };
        let sh_at = resolve(e_shoff as u32)?;
        if sh_at + e_shnum * 0x28 > image.len() {
            return invalid(format!(
                "section headers [{sh_at:#x}..{:#x}) past the combined image ({})",
                sh_at + e_shnum * 0x28,
                image.len()
            ));
        }
        let mut raw = Vec::with_capacity(e_shnum);
        for i in 0..e_shnum {
            let o = sh_at + i * 0x28;
            raw.push([
                u32_at(&image, o)?,
                u32_at(&image, o + 4)?,
                u32_at(&image, o + 8)?,
                u32_at(&image, o + 12)?,
                u32_at(&image, o + 16)?,
                u32_at(&image, o + 20)?,
                u32_at(&image, o + 24)?,
                u32_at(&image, o + 28)?,
                u32_at(&image, o + 32)?,
                u32_at(&image, o + 36)?,
            ]);
        }
        // raw = [name, type, flags, addr, offset, size, link, info, align, entsize]
        let mut sections: Vec<ModuleSection> = raw
            .iter()
            .map(|s| -> Result<ModuleSection> {
                Ok(ModuleSection {
                    name: String::new(),
                    sh_type: s[1],
                    offset: resolve(s[3])? as u32,
                    data_offset: s[4],
                    size: s[5],
                    link: s[6],
                    entsize: s[9],
                })
            })
            .collect::<Result<_>>()?;
        if shstrndx < sections.len() {
            let (st_off, st_size) = (sections[shstrndx].data_offset as usize, sections[shstrndx].size as usize);
            if st_off + st_size > image.len() {
                return invalid(format!(".shstrtab [{st_off:#x}..{:#x}) past the image", st_off + st_size));
            }
            for (i, s) in sections.iter_mut().enumerate() {
                let no = raw[i][0] as usize;
                let base = st_off.saturating_add(no);
                if base >= st_off + st_size {
                    continue;
                }
                let end = image[base..st_off + st_size].iter().position(|&b| b == 0).map(|p| base + p).unwrap_or(base);
                s.name = String::from_utf8_lossy(&image[base..end]).into_owned();
            }
        }
        // Symbols: the SYMTAB section (type 2), names via its sh_link [M 0x29ea00 resolves the
        // linked STRTAB the same way].
        let sym_idx = sections
            .iter()
            .position(|s| s.sh_type == 2)
            .ok_or_else(|| FormatError::Invalid("no SYMTAB section in the module".into()))?;
        let (sym_off, sym_size, sym_link) = {
            let s = &sections[sym_idx];
            (s.data_offset as usize, s.size as usize, s.link as usize)
        };
        let str_off = sections.get(sym_link)
            .map(|s| s.data_offset as usize)
            .ok_or_else(|| FormatError::Invalid("SYMTAB sh_link out of range".into()))?;
        let mut symbols = Vec::with_capacity(sym_size / 16);
        for i in 0..sym_size / 16 {
            let o = sym_off + i * 16;
            let nm = u32_at(&image, o)? as usize;
            let name = if nm == 0 {
                String::new()
            } else {
                let base = str_off + nm;
                if base >= image.len() {
                    return invalid(format!("symbol name at {base:#x} runs past the image"));
                }
                let end = image[base..].iter().position(|&b| b == 0).map(|p| base + p).unwrap_or(base);
                if end > image.len() {
                    return invalid(format!("symbol name at {base:#x} runs past the image"));
                }
                String::from_utf8_lossy(&image[base..end]).into_owned()
            };
            symbols.push(ModuleSymbol {
                name,
                value: u32_at(&image, o + 4)?,
                size: u32_at(&image, o + 8)?,
                info: *image.get(o + 12).ok_or_else(|| FormatError::Invalid("symbol past the image".into()))?,
                shndx: u16::from_le_bytes([
                    *image.get(o + 14).unwrap_or(&0),
                    *image.get(o + 15).unwrap_or(&0),
                ]),
            });
        }
        let mut module = TrackModule { image, elfd_size, sections, symbols, relocations: 0 };
        module.relocations = module.apply_relocations()?;
        Ok(module)
    }

    /// Applies every `.rel.data` entry: `R_MIPS_32` (`word += symbol value`) [M 0x2bb300;
    /// measured: every entry of all 8 mission tracks is type 2 — this loader handles no other
    /// type, matching the game's DynamicLoader (U-004)]. Bounds-checked; unknown types error.
    pub fn apply_relocations(&mut self) -> Result<usize> {
        let rel = self
            .sections
            .iter()
            .find(|s| s.sh_type == 9)
            .cloned()
            .ok_or_else(|| FormatError::Invalid("no .rel.data section in the module".into()))?;
        let ent = if rel.entsize == 0 { 8 } else { rel.entsize as usize };
        let n = rel.size as usize / ent;
        let rel_off = rel.data_offset as usize;
        for i in 0..n {
            let o = rel_off + i * ent;
            let off = u32_at(&self.image, o)? as usize;
            let info = u32_at(&self.image, o + 4)?;
            let sym_idx = (info >> 8) as usize;
            let typ = info & 0xff;
            if typ != 2 {
                return invalid(format!("relocation {i}: type {typ} is not R_MIPS_32"));
            }
            let val = self
                .symbols
                .get(sym_idx)
                .ok_or_else(|| FormatError::Invalid(format!("relocation {i}: symbol {sym_idx} out of range")))?
                .value as u64;
            if off.saturating_add(4) > self.image.len() {
                return invalid(format!("relocation {i} target {off:#x} runs past the image"));
            }
            let word = u32_at(&self.image, off)? as u64;
            let sum = (word + val) & 0xffff_ffff;
            self.image[off..off + 4].copy_from_slice(&(sum as u32).to_le_bytes());
        }
        Ok(n)
    }

    /// The `__Model:::<article>::{as  0000}::Model[0].NN` symbols of one article, sorted by the
    /// `NN` segment number [D measured: paris 1,890 Model symbols over 494 articles].
    pub fn model_segments(&self, article: &str) -> Vec<(u8, u32)> {
        let needle = format!("__Model:::{article}::{{as  0000}}::Model[0].");
        let mut out = Vec::new();
        for s in &self.symbols {
            if let Some(rest) = s.name.strip_prefix(&needle) {
                if rest.len() == 2 {
                    if let Ok(seg) = u8::from_str_radix(rest, 16) {
                        out.push((seg, s.value));
                    }
                }
            }
        }
        out.sort();
        out.dedup_by(|a, b| a.0 == b.0);
        out
    }

    /// The `__GPRenderMethod_<article>::{as  0000}::Model[0].NN_<x>_name` strings in the module
    /// image (the EAGL GeoPrim render-method names) [D measured at paris 0x180544]. Returns
    /// (article, segment, image offset) triples.
    pub fn geoprim_names(&self) -> Vec<(String, u8, u32)> {
        const NEEDLE: &[u8] = b"__GPRenderMethod_";
        let mut out = Vec::new();
        let mut i = 0;
        while i + NEEDLE.len() <= self.image.len() {
            if &self.image[i..i + NEEDLE.len()] != NEEDLE {
                i += 1;
                continue;
            }
            let start = i + NEEDLE.len();
            let end = self.image[start..]
                .iter()
                .position(|&b| b == 0)
                .map(|e| start + e)
                .unwrap_or(start);
            let s = String::from_utf8_lossy(&self.image[start..end]).into_owned();
            if let Some(art_end) = s.find("::{as  0000}::Model[0].") {
                let article = s[..art_end].to_owned();
                let rest = &s[art_end + 23..];
                let seg_end = rest.find('_').unwrap_or(rest.len());
                if let Ok(seg) = u8::from_str_radix(&rest[..seg_end], 16) {
                    out.push((article, seg, i as u32));
                }
            }
            i = end + 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clean hand-built article container (separate layout, no overlaps):
    /// 0x00 root, 0x10 Arti, 0x20 DBN (directory), 0x30..0x70 article window, 0x70 Base payload,
    /// 0xa0 Name payload, 0xb0 as payload, 0xc0 sr payload.
    #[test]
    fn article_metadata_roundtrip() {
        let mut d = vec![0u8; 0x100];
        let put = |d: &mut Vec<u8>, o: usize, tag: u32, flags: u32, count: i32, dref: i32| {
            d[o..o + 4].copy_from_slice(&tag.to_le_bytes());
            d[o + 4..o + 8].copy_from_slice(&flags.to_le_bytes());
            d[o + 8..o + 12].copy_from_slice(&(count as u32).to_le_bytes());
            d[o + 12..o + 16].copy_from_slice(&(dref as u32).to_le_bytes());
        };
        let (ARTI, DBN, NAME) = (
            crate::crp::tag::ARTI,
            crate::crp::tag::DBN,
            crate::crp::tag::NAME,
        );
        // Root: table base 0x10 (relative, dref=1 record), skip 1 record -> pool [0x10..0x20),
        // directory = 1 record [0x20..0x30).
        put(&mut d, 0x00, crate::crp::CARP, 2 | (1 << 5), 1, 1);
        put(&mut d, 0x10, ARTI, 0, 4, 0x30); // window [0x30..0x70)
        put(&mut d, 0x20, DBN, 6 << 8, 0, 0xf0); // "track" payload at 0xf0
        d[0xf0..0xf6].copy_from_slice(b"track\0");
        // Article window: Base, Name, as#0, sr#5.
        put(&mut d, 0x30, TAG_BASE, 0x2c << 8, 0, 0x70);
        put(&mut d, 0x40, NAME, 5 << 8, 0, 0xa0);
        put(&mut d, 0x50, (TAG_AS & 0xffff_0000) | 0, (4 << 8) | 2 | 1, 0, 0x60); // payload 0x50+0x60 = 0xb0
        put(&mut d, 0x60, (TAG_SR & 0xffff_0000) | 5, (0x2a << 8) | 1, 0, 0xc0);
        let f = |v: f32| v.to_le_bytes();
        d[0x80..0x84].copy_from_slice(&f(-10.0));
        d[0x84..0x88].copy_from_slice(&f(-7.5));
        d[0x88..0x8c].copy_from_slice(&f(-10.0));
        d[0x90..0x94].copy_from_slice(&f(18.0));
        d[0x94..0x98].copy_from_slice(&f(0.5));
        d[0x98..0x9c].copy_from_slice(&f(15.0));
        d[0xa0..0xa5].copy_from_slice(b"road\0");
        // as#0 payload: 4 bytes {u16 5, "sr"} = 05 00 72 73.
        d[0xb0..0xb4].copy_from_slice(&[0x05, 0x00, 0x72, 0x73]);
        // sr#5 payload: "EAGL::road::{as  0000}::Model[0].00" (35 bytes + NUL = 0x24).
        let sr = b"EAGL::road::{as  0000}::Model[0].00\0";
        d[0xc0..0xc0 + sr.len()].copy_from_slice(sr);
        let c = crate::crp::Crp::parse(&d).unwrap();
        assert_eq!(c.track_name().unwrap(), Some("track"));
        let arts = c.track_articles().unwrap();
        assert_eq!(arts.len(), 1);
        let a = &arts[0];
        assert_eq!(a.name, "road");
        assert_eq!(a.bbox_min, [-10.0, -7.5, -10.0]);
        assert_eq!(a.bbox_max, [18.0, 0.5, 15.0]);
        assert_eq!(a.parts, vec![PartRef { as_instance: 0, sr_index: Some(5) }]);
        // The sr lookup via the raw record (window walk from the pool).
        let arti = &c.pool().unwrap()[0];
        assert_eq!(
            c.article_sr_string(arti, 5).unwrap().as_deref(),
            Some("EAGL::road::{as  0000}::Model[0].00")
        );
        // A non-"sr" tail yields sr_index: None (the measured trafficlight form).
        d[0xb0..0xb4].copy_from_slice(&[0x00, 0x02, 0x05, 0x00]);
        let c2 = crate::crp::Crp::parse(&d).unwrap();
        assert_eq!(c2.track_articles().unwrap()[0].parts,
            vec![PartRef { as_instance: 0, sr_index: None }]);
        // Malformed: a Base payload too short must error, not panic.
        d[0x34..0x38].copy_from_slice(&(0x04u32 << 8).to_le_bytes()); // Base size 4
        let c3 = crate::crp::Crp::parse(&d).unwrap();
        assert!(c3.track_articles().is_err());
    }
}
