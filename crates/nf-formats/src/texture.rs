//! Map textures: blocks 0x10 (texture header), 0x0F (palette header), 0x16 (pixels) and 0x12 (palette), one 0x16 +
//! 0x12 pair per texture in table order. Spec: docs/formats/map.md §Textures.
//!
//! Texture header entry (12 bytes, *measured* via `psiCreateMapTextures` 0x11d200):
//! ```text
//! +0 u8 flags   bit 0: 256 colours / 8 bpp, else 16 colours / 4 bpp   (values 0x00 0x01 0x10 0x11 0x18 0x19)
//! +1 u8 ?
//! +2 u16 width − 1, +4 u16 height − 1
//! +6 u8 frames  (animated textures: frames stacked one after another)
//! +7 u8 rate    (the game uses 60 / rate)
//! +8 u32 hashcode on disc (0xFFFFFFFF or 03xxxxxx); replaced by the pixel pointer at load
//! ```
//! Palette: 16 or 256 entries × RGBA8888, alpha on the PS2 scale (0x80 = opaque; never above 0x80 on our disc).
//! 256-entry palettes are stored in the GS CSM1 order (entries 8–15 and 16–23 of every 32 swapped); 4-bpp pixels hold
//! the left pixel in the low nibble. Both orderings were decided by measuring neighbour smoothness over all textures
//! (WORKLOG 0008), and agree with NightfireResearch (*sourced*).

use crate::buf::{invalid, Buf, Result};
use crate::map::{id, MapFile};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    /// Position in the map's texture header (stable even when an earlier texture is skipped as short).
    pub index: usize,
    pub flags: u8,
    pub unknown1: u8,
    pub width: u16,
    pub height: u16,
    pub frames: u8,
    pub rate: u8,
    pub hashcode: u32,
    /// Palette in linear index order (CSM1 already undone), RGBA with PS2 alpha (0..=0x80).
    pub palette: Vec<[u8; 4]>,
    /// One byte per pixel, `width × height × frames`, palette indices.
    pub indices: Vec<u8>,
}

impl Texture {
    pub fn is_8bpp(&self) -> bool { self.flags & 1 != 0 }
    pub fn frame(&self, i: usize) -> &[u8] {
        let n = self.width as usize * self.height as usize;
        &self.indices[i * n..(i + 1) * n]
    }
    /// The palette with alpha mapped to 8 bits: `round(a × 255 / 128)`, so 0x80 → 255 (the port's convention).
    pub fn palette_rgba8(&self) -> Vec<[u8; 4]> {
        self.palette.iter().map(|c| [c[0], c[1], c[2], ps2_alpha_to_8bit(c[3])]).collect()
    }
}

pub fn ps2_alpha_to_8bit(a: u8) -> u8 { ((a as u32 * 255 + 64) / 128).min(255) as u8 }

/// Undoes the GS CSM1 CLUT order: within every 32 entries, entries 8..16 and 16..24 trade places.
pub fn unswizzle_csm1(pal: &[[u8; 4]]) -> Vec<[u8; 4]> {
    let mut out = pal.to_vec();
    for base in (0..pal.len()).step_by(32) {
        for j in 0..8 {
            if base + 24 <= pal.len() {
                out[base + 8 + j] = pal[base + 16 + j];
                out[base + 16 + j] = pal[base + 8 + j];
            }
        }
    }
    out
}

/// All textures of a map entry. Textures whose pixel block is shorter than the header implies are reported as
/// errors in the second vector (index, message) rather than failing the whole map (3 such cases, UNKNOWNS U-017).
pub fn textures(map: &[u8], f: &MapFile) -> Result<(Vec<Texture>, Vec<(usize, String)>)> {
    let Some(th) = f.first(id::TEXTURE_HEADER) else { return Ok((Vec::new(), Vec::new())) };
    let th = Buf(th.payload(map));
    let n = th.len() / 12;
    let data: Vec<_> = f.all(id::TEXTURE_DATA).collect();
    let pals: Vec<_> = f.all(id::PALETTE_DATA_PSX).collect();
    if data.len() != n || pals.len() != n {
        return invalid(format!("{n} texture headers but {} pixel and {} palette blocks", data.len(), pals.len()));
    }
    let mut out = Vec::with_capacity(n);
    let mut bad = Vec::new();
    for i in 0..n {
        let o = i * 12;
        let (flags, unknown1) = (th.u8(o)?, th.u8(o + 1)?);
        let (width, height) = (th.u16(o + 2)? + 1, th.u16(o + 4)? + 1);
        let (frames, rate, hashcode) = (th.u8(o + 6)?, th.u8(o + 7)?, th.u32(o + 8)?);
        let eight = flags & 1 != 0;
        let raw_pal = pals[i].payload(map);
        let colours = if eight { 256 } else { 16 };
        if raw_pal.len() < colours * 4 { return invalid(format!("texture {i}: palette {} bytes", raw_pal.len())); }
        let pal: Vec<[u8; 4]> = raw_pal[..colours * 4].chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect();
        let palette = if eight { unswizzle_csm1(&pal) } else { pal };
        let px = data[i].payload(map);
        let count = width as usize * height as usize * frames as usize;
        let need = if eight { count } else { count.div_ceil(2) };
        if px.len() < need {
            bad.push((i, format!("pixel block {} bytes, header needs {need}", px.len())));
            continue;
        }
        let indices = if eight { px[..count].to_vec() } else { (0..count).map(|k| (px[k / 2] >> (4 * (k & 1))) & 15).collect() };
        out.push(Texture { index: i, flags, unknown1, width, height, frames, rate, hashcode, palette, indices });
    }
    Ok((out, bad))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::tests::mini_map;

    #[test]
    fn csm1_swaps_the_middle_groups_of_each_32() {
        let pal: Vec<[u8; 4]> = (0..64u8).map(|i| [i, 0, 0, 0x80]).collect();
        let u = unswizzle_csm1(&pal);
        let order: Vec<u8> = u.iter().map(|c| c[0]).collect();
        let want: Vec<u8> = (0..64u8).map(|i| { let (b, r) = (i / 32 * 32, i % 32); b + match r { 8..=15 => r + 8, 16..=23 => r - 8, _ => r } }).collect();
        assert_eq!(order, want);
        assert_eq!(unswizzle_csm1(&u), pal, "the swap is its own inverse");
    }

    #[test]
    fn alpha_scale_endpoints() {
        assert_eq!((ps2_alpha_to_8bit(0), ps2_alpha_to_8bit(0x40), ps2_alpha_to_8bit(0x80), ps2_alpha_to_8bit(0xff)), (0, 128, 255, 255));
    }

    #[test]
    fn decodes_4bpp_low_nibble_first_and_8bpp_with_csm1() {
        // 4bpp 2×2 texture: bytes 0x21, 0x43 → indices 1,2,3,4.
        let mut hdr4 = vec![0u8; 12];
        hdr4[2..4].copy_from_slice(&1u16.to_le_bytes());
        hdr4[4..6].copy_from_slice(&1u16.to_le_bytes());
        hdr4[6] = 1;
        hdr4[8..12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        // 8bpp 1×1 texture with index 8, whose palette entry sits at stored position 16.
        let mut hdr8 = hdr4.clone();
        hdr8[0] = 1;
        hdr8[2..4].copy_from_slice(&0u16.to_le_bytes());
        hdr8[4..6].copy_from_slice(&0u16.to_le_bytes());
        let pal4: Vec<u8> = (0..16u8).flat_map(|i| [i, i, i, 0x80]).collect();
        let mut pal8 = vec![0u8; 1024];
        pal8[16 * 4..16 * 4 + 4].copy_from_slice(&[7, 8, 9, 0x40]);
        let m = mini_map(&[
            (id::TEXTURE_HEADER, &[hdr4.clone(), hdr8].concat()),
            (id::TEXTURE_DATA, &[0x21, 0x43]), (id::PALETTE_DATA_PSX, &pal4),
            (id::TEXTURE_DATA, &[8]), (id::PALETTE_DATA_PSX, &pal8),
            (id::END, &[]),
        ]);
        let f = MapFile::parse(&m).unwrap();
        let (t, bad) = textures(&m, &f).unwrap();
        assert!(bad.is_empty());
        assert_eq!((t[0].index, t[1].index), (0, 1));
        assert_eq!(t[0].indices, [1, 2, 3, 4]);
        assert_eq!(t[1].palette[t[1].indices[0] as usize], [7, 8, 9, 0x40]);
        assert_eq!(t[1].palette_rgba8()[8][3], 128);
    }
}
