//! The HUD's static tables in ACTION.ELF (docs/systems/hud.md): `SpriteInfo` arrays (`Sprite_Create2` 0x1de3d8,
//! 0x2C bytes), the bullet-strip table `BulletImg.189`, the fonts (`FONTSerpLight` + `FontDataSerpLight`, read
//! by `__Font_DrawText` 0x1ee598 / `Font_GetKernAdjust` 0x1ee298), and the language string tables
//! (`Txt_LoadLanguage` 0x1caad0 / `Txt_BindLabel` 0x1cad60 over the disc `*.dat` files, e.g. `USATxt.dat`).

use crate::buf::{Buf, FormatError, Result};
use crate::elf::Elf;

/// One `SpriteInfo`: colour 0xRRGGBBAA, shadow colour, layer (lower = on top), flags, x / y relative to the pane,
/// w / h (0 = texture size), texels u, v, du, dv, a text label (−1 = a texture sprite), the texture hash.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteInfo { pub colour: u32, pub colour2: u32, pub layer: i16, pub flags: u16, pub x: i16, pub y: i16, pub w: i16, pub h: i16, pub uv: [i16; 4], pub label: i32, pub texture: u32 }

/// A font glyph (0x18 bytes): texels, y offset, pre-gap (not for the first character), post-advance, its code and
/// kern pairs (next character, adjustment).
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph { pub u: i16, pub v: i16, pub w: i16, pub h: i16, pub yoff: i16, pub pre: i16, pub post: i16, pub code: u16, pub kerns: Vec<(u16, i8)> }

#[derive(Clone, Debug)]
pub struct Font { pub first: u16, pub last: u16, pub gap: f32, pub space: f32, pub glyphs: Vec<Glyph> }

/// A laid-out glyph: index into `Font::glyphs` and its top-left position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed { pub glyph: usize, pub x: f32, pub y: f32 }

impl Font {
    fn glyph(&self, c: u16) -> Option<usize> { self.glyphs.iter().position(|g| g.code == c) }

    /// `__Font_DrawText` at scale 1 for one line: per character the kern of the previous glyph's pair list, the pre-gap
    /// (not for the first), the quad at (pen, y + yoff − (glyph 0's h + yoff)), then pen += w + post-advance. A space
    /// advances by the font's space width × 1.25 (*suggested* from the research pass). Returns the glyphs and the width.
    pub fn layout(&self, text: &str, x: f32, y: f32) -> (Vec<Placed>, f32) {
        let top = self.glyphs.first().map_or(0.0, |g| (g.h + g.yoff) as f32);
        let (mut pen, mut out, mut prev): (f32, Vec<Placed>, Option<usize>) = (x, Vec::new(), None);
        for ch in text.bytes() {
            if ch == b' ' { pen += self.space * 1.25; prev = None; continue; }
            let Some(gi) = self.glyph(ch as u16) else { continue };
            let g = &self.glyphs[gi];
            if let Some(p) = prev {
                pen += self.glyphs[p].kerns.iter().find(|k| k.0 == ch as u16).map_or(0.0, |k| k.1 as f32);
                pen += g.pre as f32;
            }
            out.push(Placed { glyph: gi, x: pen, y: y + g.yoff as f32 - top });
            pen += (g.w + g.post) as f32;
            prev = Some(gi);
        }
        (out, pen - x)
    }
}

/// The HUD tables.
#[derive(Clone, Debug)]
pub struct HudTables {
    pub health: Vec<SpriteInfo>,
    pub ammo: Vec<SpriteInfo>,
    /// The air/oxygen pane's 3 sprites (`HUD_UpdateAirPane` 0x1a11c8, `AirSprInfo` 0x2bd490).
    pub air: Vec<SpriteInfo>,
    /// Per ammo type: u, v, width, pixels per round.
    pub bullets: Vec<[f32; 4]>,
    /// Font 2 (`FONTSerpLight`, the HUD's).
    pub font: Font,
}

impl HudTables {
    pub fn from_action_elf(data: &[u8]) -> Result<HudTables> {
        let elf = Elf::parse(data)?;
        let syms = elf.symbols(data)?;
        let segs = elf.load_segments(data)?;
        let at = |addr: u32| -> Result<&[u8]> {
            segs.iter().find_map(|(va, b, _)| (addr >= *va && ((addr - va) as usize) < b.len()).then(|| &b[(addr - va) as usize..]))
                .ok_or_else(|| FormatError::Invalid(format!("{addr:#x} outside the loaded data")))
        };
        let sym = |name: &str| -> Result<(u32, u32)> {
            syms.iter().find(|s| s.name == name).map(|s| (s.value, s.size))
                .ok_or_else(|| FormatError::Invalid(format!("ACTION.ELF has no symbol {name}")))
        };
        let sprites = |name: &str| -> Result<Vec<SpriteInfo>> {
            let (a, size) = sym(name)?;
            let b = Buf(at(a)?);
            (0..size as usize / 0x2c).map(|k| {
                let o = k * 0x2c;
                let s = |d: usize| b.u16(o + d).map(|v| v as i16);
                Ok(SpriteInfo { colour: b.u32(o)?, colour2: b.u32(o + 4)?, layer: s(8)?, flags: b.u16(o + 0xa)?, x: s(0xc)?, y: s(0xe)?, w: s(0x10)?, h: s(0x12)?,
                    uv: [s(0x14)?, s(0x16)?, s(0x18)?, s(0x1a)?], label: b.u32(o + 0x1c)? as i32, texture: b.u32(o + 0x24)? })
            }).collect()
        };
        let (ba, bsize) = sym("BulletImg.189")?;
        let bb = Buf(at(ba)?);
        let bullets = (0..bsize as usize / 16).map(|k| Ok([bb.f32(k * 16)?, bb.f32(k * 16 + 4)?, bb.f32(k * 16 + 8)?, bb.f32(k * 16 + 12)?])).collect::<Result<_>>()?;
        // Font header: +8 first / last char, +0xC glyph count, +0x10 glyph table, +0x14 line gap, +0x18 space advance.
        let (fa, _) = sym("FONTSerpLight")?;
        let fh = Buf(at(fa)?);
        let (first, last, count, table) = (fh.u16(8)?, fh.u16(10)?, fh.u32(0xc)? as usize, fh.u32(0x10)?);
        let gb = Buf(at(table)?);
        let glyphs = (0..count).map(|k| {
            let o = k * 0x18;
            let s = |d: usize| gb.u16(o + d).map(|v| v as i16);
            let (nk, kp) = (gb.u16(o + 0xe)? as usize, gb.u32(o + 0x10)?);
            // `Font_GetKernAdjust`: 6-byte pairs, the next character at +2, the adjustment (signed byte) at +4.
            let kerns = if nk == 0 || kp == 0 { Vec::new() } else {
                let kb = Buf(at(kp)?);
                (0..nk).map(|j| Ok((kb.u16(j * 6 + 2)?, kb.u8(j * 6 + 4)? as i8))).collect::<Result<_>>()?
            };
            Ok(Glyph { u: s(0)?, v: s(2)?, w: s(4)?, h: s(6)?, yoff: s(8)?, pre: s(0xa)?, post: s(0xc)?, code: gb.u16(o + 0x14)?, kerns })
        }).collect::<Result<_>>()?;
        let font = Font { first, last, gap: fh.f32(0x14)?, space: fh.f32(0x18)?, glyphs };
        let air = sprites("AirSprInfo")?;
        Ok(HudTables { health: sprites("HealthSprInfo")?, ammo: sprites("AmmoSprInfo")?, air, bullets, font })
    }
}

/// A language string table: one disc `*.dat` file (e.g. `USATxt.dat`, the NTSC-U default — `FNames[SelLang]`,
/// `Txt_LoadLanguage` 0x1caad0 sprintf's `"%s%s"` over it). Layout [M 0x1caad0, verified on the disc]:
/// `u32 dataBytes`, the data block, padded to `dataBytes + (4 - dataBytes % 4)` (the reader's advance is
/// `size + (4 - (size & 3))` — 4 even when aligned); `u32 NumEntries`; `u32 offsets[NumEntries - 1]`
/// (entry *i* ≥ 1 is the NUL-terminated string at `data[offsets[i-1]]`; entry 0 is the heap-string slot,
/// bound to an empty sentinel in the ELF); `u32 NumFixups`; `u32 fixups[NumFixups]`.
/// `Txt_BindLabel(id, _)` 0x1cad60: `index = fixups[id >> 24] + (id & 0xffffff)`; 0 → the heap (no static
/// string), `>= NumEntries` → "Invalid Text Label", else `Bank[index]`.
#[derive(Clone, Debug, Default)]
pub struct Lang {
    data: Vec<u8>,
    /// Entry *i*'s offset into `data` (index shifted by one: `offsets[i-1]` in the file).
    offsets: Vec<u32>,
    fixups: Vec<u32>,
}

impl Lang {
    pub fn parse(data: &[u8]) -> Result<Lang> {
        let b = Buf(data);
        if data.len() < 4 { return Err(FormatError::Invalid("text table: no header".into())); }
        let size = b.u32(0)? as usize;
        let mut p = 4 + size + (4 - (size & 3));
        if p + 4 > data.len() { return Err(FormatError::Invalid("text table: data past the file".into())); }
        let data_end = 4 + size;
        let n = b.u32(p)? as usize; p += 4;
        if n == 0 { return Ok(Lang { data: data[4..data_end].to_vec(), offsets: Vec::new(), fixups: Vec::new() }); }
        if p + (n - 1) * 4 > data.len() { return Err(FormatError::Invalid("text table: offsets past the file".into())); }
        let mut offsets = Vec::with_capacity(n - 1);
        for _ in 1..n { offsets.push(b.u32(p)?); p += 4; }
        if p + 4 > data.len() { return Err(FormatError::Invalid("text table: no fixup count".into())); }
        let nf = b.u32(p)? as usize; p += 4;
        if p + nf * 4 > data.len() { return Err(FormatError::Invalid("text table: fixups past the file".into())); }
        let mut fixups = Vec::with_capacity(nf);
        for _ in 0..nf { fixups.push(b.u32(p)?); p += 4; }
        Ok(Lang { data: data[4..data_end].to_vec(), offsets, fixups })
    }

    /// `Txt_BindLabel` (0x1cad60): the NUL-terminated string for a `0x2xxxxxx` label id, lossy-decoded.
    /// `None` = the game's empty cases: no fixup table, an out-of-range bank, index 0 (a heap string) or
    /// `>= NumEntries` ("Invalid Text Label").
    pub fn bind(&self, id: u32) -> Option<&str> {
        if self.fixups.is_empty() { return None; }
        let bank = (id >> 24) as usize;
        if bank >= self.fixups.len() { return None; }
        let index = self.fixups[bank] + (id & 0xffffff);
        if index == 0 || index as usize >= self.offsets.len() + 1 { return None; }
        let off = self.offsets[index as usize - 1] as usize;
        let end = self.data[off..].iter().position(|&c| c == 0).map_or(self.data.len(), |e| off + e);
        Some(std::str::from_utf8(&self.data[off..end]).unwrap_or(""))
    }

    /// `sprintf`-style substitution for the `%s` / `%d` forms the game builds with
    /// `sprintf(heap, Txt_BindLabel(id), args)` (`Pickup_Handler` 0x18e650). Only the pickup messages'
    /// forms are handled: `%s` (weapon name), `%d` (a count), `%%`.
    pub fn format(&self, id: u32, args: &[String]) -> String {
        let Some(tpl) = self.bind(id) else { return String::new() };
        let mut out = String::new();
        let mut next = 0usize;
        let mut chars = tpl.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' { out.push(c); continue; }
            match chars.next() {
                Some('%') => out.push('%'),
                Some('s') | Some('d') | Some('x') => {
                    if let Some(a) = args.get(next) { out.push_str(a); }
                    next += 1;
                }
                Some(other) => { out.push('%'); out.push(other); }
                None => out.push('%'),
            }
        }
        out
    }
}
