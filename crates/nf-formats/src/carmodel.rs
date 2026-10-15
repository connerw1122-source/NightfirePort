//! The driving missions' car models — `data\car\model\<car>.crp` inside the `DRIVING/*.VIV`
//! archives (plus the shared `.ssh` texture sheet next to each `.crp`, named by the model's
//! `ns  ` record, e.g. `taxi.ssh`).
//!
//! Everything here is measured on the disc data and matched to DRIVING.ELF consumers
//! (EE = file offset + 0x107080, docs/atlas/driving.md build identity); evidence tags [M]/[D]
//! as in the other format modules. The container layer is `nf_formats::crp` (docs/formats/crp.md):
//! the decompressed `.crp` is a 16-byte record pool whose root is `CARP`; car containers hold a
//! pool of `Arti`×1 + `Shar`×1 and a directory of `ELFd + ELFr + Sect + ns#0`
//! [M golden/crp_golden.tsv, all 81 car/weapon files].
//!
//! The car **article** (the pool `Arti`) is the model. Its scan window holds these records
//! [M consumer FUN_001c7488 0x1c7488 / FUN_001c7658 0x1c7658 / FUN_001c7940 0x1c7940; measured
//! on `MISC.VIV#78` taxi.crp]:
//!
//! ```text
//! Base   a 0x20-byte-per-row table (taxi: 12 rows) of f32 4x4-ish transforms
//! Name   the article name ("taxi")
//! Skel   the part table — see `Skel`; the wheel/hub attachment points are parts here
//! an#N   animation refs (payload = inline 4-byte tag refs + f32s)
//! aq#N   animation refs (8-record windows)
//! as#N   the model-set variants — one record per game variant; the payload is an inline
//!        script of 4-byte (BE-char tag, u16 instance) refs (e.g. Base#1, ef#1, an#0, sr#34)
//!        mixed with f32s [M FUN_001c7658 looks the set up with FUN_001c6fb8(arti,'as  ')]
//! cs#N   empty payload in taxi (32 zero bytes)
//! ef#N   effect streams (1088 B for ef#1: f32 position rows + u16 fields)
//! el#N   element refs (payload = inline tag refs)
//! in#0   the instance list — see `Instances`
//! si#60  12-byte payload
//! sr#N   named resources — payload = NUL-terminated name, e.g.
//!        `GIOT::Cr_Dmg2`, `CARP::taxi::{ef  0001}::896`, `EAGL::taxi::{as  0004}::Model[0].00`
//! ns#0   (directory) the texture-sheet filename `taxi.ssh` [D]
//! ELFd/ELFr/Sect (directory) the embedded MIPS ET_REL module — see `CarModule`
//! ```
//!
//! # The part skeleton (`Skel`) [M 0x1c9910/0x1c9980/0x1c9a80]
//!
//! The consumers `FUN_001c9910` (part count = the record's `count` field) and `FUN_001c9980`
//! (part index by name, 1-based; compares `strcmp(name, payload + u16_at(entry+0x1c))`, stride
//! 0x20) read this record. The vehicle code resolves the wheel attachment points by these names
//! [M FUN_001d2648 0x1d2648, name-pointer table at EE 0x334070..0x334094]:
//! `leftfrontwheel`/`rightfrontwheel`/`leftrearwheel`/`rightrearwheel` (wheels),
//! `leftfronthub`/`rightfronthub`/`leftrearhub`/`rightrearhub` (hubs),
//! `leftfin`/`rightfin`/`leftrudder`/`rightrudder` (boats). Measured taxi parts:
//! `hood, leftfrontwheel, leftrearwheel, rightfrontwheel, rightrearwheel, trunk`.
//!
//! ```text
//! payload: N × 0x20-byte entries then a NUL-separated string pool
//! entry +0x0c  f32 (1.0 in every taxi entry)
//! entry +0x1c  u16 name offset into the payload (the string pool sits after the entries)
//! ```
//!
//! # The instances (`in  `) [M FUN_001a1bf0 0x1a1bf0]
//!
//! The record's `count` field is the instance count; the payload is `count × 0x40` bytes:
//! +0xc = a flags byte (bit 0x10 counted by the walker), +0x1c = a 4-byte inline reference
//! that the game relocates at load (in the file it reads as the short tag `sa#N`,
//! e.g. 0x61730000 = `sa` + instance 0).
//!
//! # The embedded module (`CarModule`)
//!
//! `ELFd` (148,640 B for taxi) + `ELFr` (13,904 B) glue into one 32-bit little-endian MIPS
//! ET_REL image: `.data` (WRITE+ALLOC) holds the whole model, `.symtab` names its structures,
//! `.rel.data` (all `R_MIPS_32`, section-relative) wires the pointers [M module loader
//! FUN_001a9600 0x1a9600 → FUN_0029f7c8/0x29f820; the section table sits after the glued
//! payload — measured `e_shoff` 0x27a00 = ELFd+ELFr end − 0xf0].
//!
//! Symbol vocabulary (taxi) [D]: `__Model:::taxi::{as  NNNN}::Model[L].00` — the model
//! descriptors, one per variant×LOD (values are .data offsets); `__EAGL::TAR:::{T0@hash}TEX0::$$00`
//! — texture references (resolved against the `.ssh` sheet by content hash); `MatCar*Packed`
//! — material names; `__EAGL::GeoPrimState:::STATE::...` — render-state strings
//! (`primtype=trilist/tristrip`, `modify=envmap/specmap/bumpmap/glaregen`, `texalpha`, `cull`);
//! `__LASTCHANGELIST:::4.06.01 ...` — the export tool version.
//!
//! # The model descriptor and its draw batches (measured, taxi Model[0])
//!
//! ```text
//! +0x00  ptr  texture-set name string ("$$00")
//! +0x04.. a 4x4 f32 matrix (identity in taxi), then a second matrix
//! +0x70     ASCII "metalbin in \n" (tool residue)
//! +0x9c  ptr  texture-set name string again
//! +0xac  ptr  the model-name string table ("taxi::{as  000}::Model[0].00\0…")
//! +0xb0  ptr  8 bytes (0,0)
//! +0xb4  ptr  texture list { u32 0, u32 count, ptr × count } → TAR texture objects
//! +0xbc  ptr  the texture list again
//! +0xc0  u32  0
//! +0xc4  ptr  { 0, 0x1fff5 }
//! +0xc8  u32  1
//! +0xcc  ptr  the batch table
//! ```
//!
//! The batch table is a sequence of groups: `[0xa0000000][0x00000000][count]` then
//! `count/2` × `[0xa000ffff][ptr]` (measured on all 13 taxi groups: 0x3a→29, 0xe→7, 0xa→5,
//! 4→2, …; the `ptr`s are .data offsets of the draw buffers).
//!
//! Each draw buffer starts with a 0x18-byte head (`[0x3f0][0][0x600000db][0][0][0]` for the
//! first taxi buffer; the same shape across buffers) and then is a **VIF command list**
//! [M `nf_formats::vif`]: `V4_32 NUM=1 addr=0` (a 4-word header quad), three `V4_8 NUM=1`
//! quads writing VU1 addresses 0x1f/0x20/0x21 (per-vertex color/UV-ish bytes), then
//! `V4_32 NUM=n addr=0x22` with n 16-byte (x,y,z,1.0) float quads — the vertices. The
//! EAGL GeoPrim consumer in DRIVING.ELF (the 0x262000–0x2a7000 render library range) is the
//! reader of these packets; the per-triangle index list is the remaining unknown (see
//! docs/formats/car-model.md).

use crate::buf::{invalid, FormatError, Result};
use crate::crp::{tag, Crp, Record};
use crate::sha1;

/// The `Skel` record: the named part table (the wheel/hub attachment points).
pub struct Skel {
    /// The record's `count` field [M 0x1c9910].
    pub count: i32,
    /// The part names, in record order [M 0x1c9980: `payload + u16_at(entry+0x1c)`, stride 0x20].
    pub parts: Vec<String>,
}

/// One `as  ` variant: the inline script refs (BE-char tag code + instance) of the model set.
pub struct Variant {
    pub instance: u16,
    /// The payload as raw bytes; the refs are 4-byte (tag u32 LE, where the high half is the
    /// BE-char tag and the low half the u16 instance) — kept raw because the payload also
    /// interleaves f32s (e.g. `sr#N` refs followed by floats).
    pub payload: Vec<u8>,
}

/// A parsed car-model `.crp`.
pub struct CarModel<'a> {
    data: &'a [u8],
    crp: Crp<'a>,
    /// The pool `Arti` (the model article) [M FUN_002d51f0(root,'Arti') = the pool scan].
    pub article: Record,
}

impl<'a> CarModel<'a> {
    /// Parses the decompressed `.crp` and locates the pool `Arti` (the car article)
    /// [M FUN_001c7488 0x1c7488: `FUN_002d51f0(handle+8, 'Arti')` then the name match walk].
    pub fn parse(data: &'a [u8]) -> Result<CarModel<'a>> {
        let crp = Crp::parse(data)?;
        let Some(article) = crp.pool()?.into_iter().find(|r| r.tag_code() == tag::ARTI) else {
            return invalid("no Arti in the car .crp pool");
        };
        Ok(CarModel { data, crp, article })
    }

    fn window_records(&self, r: &Record) -> Result<Vec<Record>> {
        let (start, end) = r.window(self.data)?;
        self.crp.window_records(start, end)
    }

    /// The article's name (the window `Name` record's payload) [M 0x1c7658 / 0x2dc1b0].
    pub fn name(&self) -> Result<Option<&'a str>> {
        self.crp.article_name(&self.article)
    }

    fn find_in_window(&self, code: u32) -> Result<Option<Record>> {
        for r in self.window_records(&self.article)? {
            if r.tag_code() == code {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }

    /// The `Skel` part table [M 0x1c9910/0x1c9980].
    pub fn skel(&self) -> Result<Option<Skel>> {
        let Some(rec) = self.find_in_window(0x536b_656c /* "Skel" */)? else {
            return Ok(None);
        };
        let payload = self.crp.payload(&rec)?;
        let count = rec.count;
        let mut parts = Vec::new();
        for i in 0..count.max(0) as usize {
            let at = i * 0x20 + 0x1c;
            let Some(b) = payload.get(at..at + 2) else {
                return invalid(format!("Skel entry {i} name offset past payload"));
            };
            let off = u16::from_le_bytes([b[0], b[1]]) as usize;
            let Some(rest) = payload.get(off..) else {
                return invalid(format!("Skel name offset {off:#x} past payload"));
            };
            let end = rest.iter().position(|&c| c == 0).unwrap_or(rest.len());
            parts.push(String::from_utf8_lossy(&rest[..end]).into_owned());
        }
        Ok(Some(Skel { count, parts }))
    }

    /// The `as  ` variant records with their raw payloads [M 0x1c7658].
    pub fn variants(&self) -> Result<Vec<Variant>> {
        let mut out = Vec::new();
        for r in self.window_records(&self.article)? {
            if r.tag_code() == 0x6173_2020 /* "as  " */ {
                out.push(Variant {
                    instance: r.instance().unwrap_or(0),
                    payload: self.crp.payload(&r)?.to_vec(),
                });
            }
        }
        Ok(out)
    }

    /// The named resources (`sr  ` records): payload = NUL-terminated name [D measured].
    pub fn resources(&self) -> Result<Vec<(u16, String)>> {
        let mut out = Vec::new();
        for r in self.window_records(&self.article)? {
            if r.tag_code() == 0x7372_2020 /* "sr  " */ {
                let p = self.crp.payload(&r)?;
                let end = p.iter().position(|&c| c == 0).unwrap_or(p.len());
                out.push((
                    r.instance().unwrap_or(0),
                    String::from_utf8_lossy(&p[..end]).into_owned(),
                ));
            }
        }
        Ok(out)
    }

    /// The instance count (the `in  ` record's count field) and its payload bytes
    /// (count × 0x40) [M 0x1c7658: count from the record; 0x1a1bf0 walks 0x40-byte entries].
    pub fn instances(&self) -> Result<Option<(i32, &'a [u8])>> {
        let Some(rec) = self.find_in_window(0x696e_2020 /* "in  " */)? else {
            return Ok(None);
        };
        let n = rec.count;
        let p = self.crp.payload(&rec)?;
        let want = n.max(0) as usize * 0x40;
        if p.len() < want {
            return invalid(format!("in  payload {} < count {n} × 0x40", p.len()));
        }
        Ok(Some((n, &p[..want])))
    }

    /// The texture-sheet filename from the directory `ns#0` record (`taxi.ssh`) [D measured].
    pub fn sheet_name(&self) -> Result<Option<&'a str>> {
        let Some(rec) = self.crp.find_instance(tag::NS, u32::MAX)? else {
            return Ok(None);
        };
        let p = self.crp.payload(&rec)?;
        let end = p.iter().position(|&c| c == 0).unwrap_or(p.len());
        Ok(Some(std::str::from_utf8(&p[..end]).map_err(|_| {
            FormatError::Invalid("ns payload is not UTF-8".into())
        })?))
    }

    /// The embedded module: the `ELFd` payload glued with the `ELFr` payload [M 0x1a9600;
    /// the section table sits inside the glued image, measured on taxi].
    pub fn module_bytes(&self) -> Result<Vec<u8>> {
        let Some(elfd) = self.crp.find(tag::ELFD)? else { return invalid("no ELFd") };
        let elfr = self.crp.find(tag::ELFR)?;
        let a = self.crp.payload(&elfd)?;
        let mut out = a.to_vec();
        if let Some(r) = elfr {
            out.extend_from_slice(self.crp.payload(&r)?);
        }
        Ok(out)
    }
}

/// One batch group: `count/2` draw buffers [measured; see the module doc].
pub struct BatchGroup {
    pub count: i32,
    /// .data offsets of the draw buffers.
    pub buffers: Vec<u32>,
}

/// A parsed model descriptor (the `__Model:::...` symbol target).
pub struct ModelDesc {
    /// The symbol name, e.g. `__Model:::taxi::{as  0000}::Model[0].00`.
    pub symbol: String,
    /// The `{as  NNNN}` variant index parsed from the symbol.
    pub variant: u32,
    /// The `Model[L]` LOD index parsed from the symbol.
    pub lod: u32,
    /// .data offset of the descriptor.
    pub offset: u32,
    /// The texture list (count + .data offsets of TAR texture objects).
    pub textures: Vec<u32>,
    pub groups: Vec<BatchGroup>,
}

/// The vehicle stats: for one draw buffer, the VIF unpack census.
/// The vertices are the `V4_32` quads with w = 1.0 (positions); the rest are census counts.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferCensus {
    pub v4_32_quads: u32,
    /// V4_32 quads whose 4th f32 is 1.0 — the (x,y,z,1.0) position quads [M measured].
    pub v4_32_pos_quads: u32,
    pub v4_8_quads: u32,
    pub v4_16_quads: u32,
    pub v3_32_quads: u32,
    pub other_quads: u32,
}

impl BufferCensus {
    /// Vertices = the V4_32 quads with w == 1.0 [M measured: `V4_32 NUM=n addr=0x22`
    /// with (x,y,z,1.0) f32 quads].
    pub fn vertices(&self) -> u32 {
        self.v4_32_pos_quads
    }
}

/// The parsed embedded module: the model descriptors and their buffers.
pub struct CarModule {
    /// The glued module image (ELFd || ELFr).
    pub image: Vec<u8>,
    /// The `.data` section's file offset inside `image`.
    pub data_offset: usize,
    /// The model descriptors, one per `__Model:::` symbol.
    pub models: Vec<ModelDesc>,
    /// Every `__EAGL::TAR::` texture symbol (name → .data offset).
    pub textures: Vec<(String, u32)>,
    /// The draw buffers: .data offset → (bytes, sha1, census).
    pub buffers: Vec<Buffer>,
}

pub struct Buffer {
    pub offset: u32,
    pub bytes: Vec<u8>,
    pub sha1: String,
    pub census: BufferCensus,
}

/// Walks one draw buffer as a VIF command list starting after the 0x18-byte head
/// [M measured head shape]. The VIF stream does not fill the whole [ptr, next ptr) range —
/// tool strings and the next DMA head follow it — so the walk stops cleanly at the first
/// word that is not a valid code (same packet arithmetic as `vif::parse_vif`).
pub fn census_buffer(bytes: &[u8]) -> Result<BufferCensus> {
    if bytes.len() < 0x18 {
        return invalid("draw buffer shorter than the 0x18-byte head");
    }
    let mut c = BufferCensus::default();
    let list = &bytes[0x18..];
    let mut pos = 0usize;
    while pos + 4 <= list.len() {
        let code = u32::from_le_bytes(list[pos..pos + 4].try_into().unwrap());
        let cmd = ((code >> 24) & 0x7f) as u8;
        let num = ((code >> 16) & 0xff) as u8;
        let imm = (code & 0xffff) as u16;
        let unpack = cmd & 0x60 == 0x60;
        let count = if unpack && num == 0 { 256 } else { num as u32 };
        let payload: usize = if unpack {
            let vn = ((cmd >> 2) & 3) as u32;
            let vl = (cmd & 3) as u32;
            (((count * ((32u32 >> vl) * (vn + 1)) / 8) + 3) & !3) as usize
        } else {
            match cmd {
                crate::vif::STMASK => 4,
                crate::vif::STROW | crate::vif::STCOL => 16,
                crate::vif::MPG => count as usize * 8,
                crate::vif::DIRECT | crate::vif::DIRECTHL => imm as usize * 16,
                _ => 0,
            }
        };
        if pos + 4 + payload > list.len() {
            break; // the stream ends inside the buffer range
        }
        match (cmd, unpack) {
            (_, true) => match (((cmd >> 2) & 3), (cmd & 3)) {
                (3, 0) => {
                    c.v4_32_quads += count;
                    // positions: quads with w == 1.0 [M measured: (x,y,z,1.0) f32s]
                    let data = &list[pos + 4..pos + 4 + payload];
                    for q in 0..(payload / 16) {
                        let w = f32::from_le_bytes(data[q * 16 + 12..q * 16 + 16].try_into().unwrap());
                        if w == 1.0 {
                            c.v4_32_pos_quads += 1;
                        }
                    }
                }
                (3, 2) => c.v4_8_quads += count,
                (3, 1) => c.v4_16_quads += count,
                (2, 0) => c.v3_32_quads += count,
                _ => c.other_quads += count,
            },
            _ => {}
        }
        pos += 4 + payload;
    }
    Ok(c)
}

impl CarModule {
    /// Parses the glued module image (from `CarModel::module_bytes`).
    pub fn parse(image: Vec<u8>) -> Result<CarModule> {
        let elf = crate::elf::Elf::parse(&image)?;
        let Some(data) = elf.section(".data") else { return invalid("module has no .data") };
        let data_offset = data.offset as usize;
        let data_bytes = &image[data_offset..data_offset + data.size as usize];
        // .rel.data: all R_MIPS_32 against the .data section — the addend is the word in place,
        // so the relocated pointer value is the stored word [D measured: 1,174 relocs, all type 2].
        let mut relocs: Vec<u32> = Vec::new();
        if let Some(rel) = elf.section(".rel.data") {
            let base = rel.offset as usize;
            for i in 0..rel.size as usize / 8 {
                let (off, info) = (
                    u32::from_le_bytes(image[base + i * 8..base + i * 8 + 4].try_into().unwrap()),
                    u32::from_le_bytes(image[base + i * 8 + 4..base + i * 8 + 8].try_into().unwrap()),
                );
                if info & 0xff == 2 {
                    relocs.push(off);
                }
            }
        }
        relocs.sort_unstable();

        let symbols = elf.symbols(&image)?;
        let mut models = Vec::new();
        let mut textures = Vec::new();
        for s in &symbols {
            if s.name.starts_with("__Model:::") {
                // `__Model:::taxi::{as  0000}::Model[0].00` — variant and LOD from the name.
                let variant = s
                    .name
                    .split("{as")
                    .nth(1)
                    .and_then(|t| t.trim_start_matches(|c: char| !c.is_ascii_digit()).get(..4))
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(0);
                let lod = s
                    .name
                    .split("Model[")
                    .nth(1)
                    .and_then(|t| t.chars().next())
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(0);
                models.push(ModelDesc {
                    symbol: s.name.clone(),
                    variant,
                    lod,
                    offset: s.value,
                    textures: Vec::new(),
                    groups: Vec::new(),
                });
            } else if s.name.starts_with("__EAGL::TAR::") {
                textures.push((s.name.clone(), s.value));
            }
        }
        models.sort_by_key(|m| m.offset);
        let read_u32 = |off: u32| -> Option<u32> {
            let o = off as usize;
            if data_bytes.len() >= o + 4 {
                Some(u32::from_le_bytes(data_bytes[o..o + 4].try_into().unwrap()))
            } else {
                None
            }
        };
        for m in &mut models {
            // +0xb4: texture list { u32 0, u32 count, ptr × count } [D measured].
            if let Some(list) = read_u32(m.offset + 0xb4) {
                if let Some(count) = read_u32(list + 4) {
                    let count = count.min(64);
                    for i in 0..count {
                        if let Some(p) = read_u32(list + 8 + i * 4) {
                            m.textures.push(p);
                        }
                    }
                }
            }
            // +0xcc: the batch table [D measured: [0xa0000000][0][2N] then N × [0xa000ffff][ptr]].
            let mut table = match read_u32(m.offset + 0xcc) {
                Some(t) => t,
                None => continue,
            };
            for _ in 0..64 {
                let count = match read_u32(table + 8) {
                    Some(c) => c,
                    None => break,
                };
                let n = (count / 2).min(0x1000) as usize;
                let mut buffers = Vec::with_capacity(n);
                let mut bad = false;
                for i in 0..n {
                    match read_u32(table + 12 + i as u32 * 8 + 4) {
                        Some(p) => buffers.push(p),
                        None => {
                            bad = true;
                            break;
                        }
                    }
                }
                if bad {
                    break;
                }
                m.groups.push(BatchGroup { count: count as i32, buffers });
                // next group: after the entries; stop when the header shape no longer matches.
                match read_u32(table + 12 + n as u32 * 8) {
                    Some(0xa000_0000) => table += 12 + n as u32 * 8,
                    _ => break,
                }
            }
        }
        // The draw buffers: byte ranges + SHA-1 + census.
        let mut buffer_offsets: Vec<u32> = models
            .iter()
            .flat_map(|m| m.groups.iter().flat_map(|g| g.buffers.iter().copied()))
            .collect();
        buffer_offsets.sort_unstable();
        buffer_offsets.dedup();
        let mut buffers = Vec::new();
        for (i, &off) in buffer_offsets.iter().enumerate() {
            let end = buffer_offsets
                .get(i + 1)
                .copied()
                .unwrap_or(data.size) as usize;
            let end = end.min(data.size as usize);
            let o = off as usize;
            if o >= data.size as usize || end <= o {
                return invalid(format!("buffer at {off:#x} out of .data"));
            }
            let bytes = data_bytes[o..end].to_vec();
            let census = census_buffer(&bytes)?;
            buffers.push(Buffer {
                offset: off,
                sha1: sha1::hex(&sha1::sha1(&bytes)),
                bytes,
                census,
            });
        }
        Ok(CarModule { image, data_offset, models, textures, buffers })
    }

    /// The summed vertex count per (variant, lod).
    pub fn vertices_per_model(&self) -> Vec<(u32, u32, u32)> {
        self.models
            .iter()
            .map(|m| {
                let v: u32 = m
                    .groups
                    .iter()
                    .flat_map(|g| g.buffers.iter())
                    .filter_map(|b| self.buffers.iter().find(|x| x.offset == *b))
                    .map(|x| x.census.vertices())
                    .sum();
                (m.variant, m.lod, v)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal car .crp in the measured shape: root CARP, pool [Arti(with a window:
    /// Name + Skel)], directory [ELFd, ELFr, ns#0]. All offsets hand-laid, matching the game's
    /// record semantics (relative tables ×0x10, relative payloads raw).
    fn container() -> Vec<u8> {
        let mut d = vec![0u8; 0x400];
        let put = |d: &mut Vec<u8>, o: usize, tag: u32, flags: u32, count: i32, dref: i32| {
            d[o..o + 4].copy_from_slice(&tag.to_le_bytes());
            d[o + 4..o + 8].copy_from_slice(&flags.to_le_bytes());
            d[o + 8..o + 12].copy_from_slice(&(count as u32).to_le_bytes());
            d[o + 12..o + 16].copy_from_slice(&(dref as u32).to_le_bytes());
        };
        // root: table = 0x10 (relative dref 1), skip = 1 record (the Arti), window = 3 records.
        put(&mut d, 0x00, crate::crp::CARP, 2 | (2 << 5), 3, 1);
        // Arti at 0x10: window relative dref 6 → table 0x70, skip 0, 2 records (Name, Skel).
        put(&mut d, 0x10, tag::ARTI, 2, 2, 6);
        // directory at 0x30: ELFd, ELFr, ns#0 (all relative payloads: rec + dref raw bytes).
        d[0x30..0x34].copy_from_slice(&tag::ELFD.to_le_bytes());
        d[0x34..0x38].copy_from_slice(&((16u32 << 8) | 2).to_le_bytes()); // size 16
        d[0x3c..0x40].copy_from_slice(&0x2d0u32.to_le_bytes());           // payload 0x300
        d[0x40..0x44].copy_from_slice(&tag::ELFR.to_le_bytes());
        d[0x44..0x48].copy_from_slice(&((4u32 << 8) | 2).to_le_bytes()); // size 4
        d[0x4c..0x50].copy_from_slice(&0x2d0u32.to_le_bytes());          // payload 0x310
        d[0x50..0x54].copy_from_slice(&((tag::NS & 0xffff_0000) | 0).to_le_bytes());
        d[0x54..0x58].copy_from_slice(&((9u32 << 8) | 2 | 1).to_le_bytes());
        d[0x5c..0x60].copy_from_slice(&0x2d0u32.to_le_bytes()); // payload 0x320
        // Arti window at 0x70: Name + Skel.
        put(&mut d, 0x70, tag::NAME, (5 << 8) | 2, 0, 0x1a0); // payload 0x70+0x1a0=0x210 "taxi\0"
        // Skel at 0x80: 2 parts, payload relative raw dref 0x20 → 0xa0; size 0x60.
        put(&mut d, 0x80, 0x536b_656c, (0x60 << 8) | 2, 2, 0x20);
        // Skel payload at 0xa0: 2 entries of 0x20 + strings "hood\0leftfrontwheel\0" at +0x40.
        let strings_at = 0xa0 + 0x40;
        d[strings_at..strings_at + 5].copy_from_slice(b"hood\0");
        d[strings_at + 5..strings_at + 20].copy_from_slice(b"leftfrontwheel\0");
        d[0xa0 + 0x1c..0xa0 + 0x1e].copy_from_slice(&0x40u16.to_le_bytes());
        d[0xa0 + 0x0c..0xa0 + 0x10].copy_from_slice(&1.0f32.to_le_bytes());
        d[0xa0 + 0x20 + 0x1c..0xa0 + 0x20 + 0x1e].copy_from_slice(&0x45u16.to_le_bytes());
        d[0xa0 + 0x20 + 0x0c..0xa0 + 0x20 + 0x10].copy_from_slice(&1.0f32.to_le_bytes());
        d[0x210..0x215].copy_from_slice(b"taxi\0");
        d[0x300..0x304].copy_from_slice(b"\x7fELF");
        d[0x310..0x314].copy_from_slice(&[1, 2, 3, 4]);
        d[0x320..0x329].copy_from_slice(b"taxi.ssh\0");
        d
    }

    #[test]
    fn parses_article_skel_and_sheet() {
        let d = container();
        let m = CarModel::parse(&d).unwrap();
        assert_eq!(m.name().unwrap(), Some("taxi"));
        let sk = m.skel().unwrap().unwrap();
        assert_eq!(sk.count, 2);
        assert_eq!(sk.parts, vec!["hood".to_string(), "leftfrontwheel".to_string()]);
        assert_eq!(m.sheet_name().unwrap(), Some("taxi.ssh"));
        // module glue = ELFd || ELFr payloads.
        let modbytes = m.module_bytes().unwrap();
        assert_eq!(modbytes.len(), 16 + 4);
        assert_eq!(&modbytes[..4], b"\x7fELF");
        assert_eq!(&modbytes[16..], &[1, 2, 3, 4]);
        assert!(m.variants().unwrap().is_empty());
        assert!(m.instances().unwrap().is_none());
    }

    #[test]
    fn skel_rejects_short_payload() {
        let mut d = container();
        d[0x84..0x88].copy_from_slice(&((0x10u32 << 8) | 2).to_le_bytes()); // size 0x10 < 2 entries
        let m = CarModel::parse(&d).unwrap();
        assert!(m.skel().is_err());
    }
}
