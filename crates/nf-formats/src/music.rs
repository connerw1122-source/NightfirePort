//! Music files and tables. Spec: docs/systems/music.md.
//!
//! `PS2/MFXINFO.MXI`: 150 × s32 track numbers; a track's file number n = its index + 1 (`FindMusicTrackNumber`), files
//! `PS2/MUSIC/MFX_<n >> 4>/MFX_<n>.{SMF,SSD}`. `.SMF`: 20-byte header {start count, marker count, start offset,
//! marker offset, base volume}, 52-byte section starts, 32-byte markers. `.SSD`: headerless SPU ADPCM, stereo,
//! 32 kHz, 0x100-byte blocks (0x80 left, 0x80 right).

use crate::buf::{invalid, Buf, Result};
use crate::elf::Elf;

/// `MFXINFO.MXI`: track number → file number (index + 1).
pub fn track_file(mxi: &[u8], track: i32) -> Result<Option<u32>> {
    let b = Buf(mxi);
    for k in 0..b.len() / 4 { if b.i32(4 * k)? == track { return Ok(Some(k as u32 + 1)); } }
    Ok(None)
}

pub fn music_path(n: u32, ext: &str) -> String { format!("PS2/MUSIC/MFX_{}/MFX_{n}.{ext}", n >> 4) }

/// `MusicMarkerData` (32 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marker {
    /// The section this marker belongs to.
    pub name: i32,
    /// Byte offset in the `.SSD` (interleaved); the driver uses it rounded down to 256.
    pub pos: i32,
    /// 0 bar, 10 section start (both jump points), 6 loop (jump point), 7 goto, 9 end, 5 pause.
    pub mtype: i32,
    pub loop_start: i32,
    /// For 6 / 7: the index of the marker playback continues from.
    pub loop_marker: i32,
}

impl Marker {
    /// A legal point for a deferred jump.
    pub fn eligible(&self) -> bool { matches!(self.mtype, 0 | 10 | 6) }
}

/// `MusicMarkerStartData` (52 bytes): a section's start marker copy, its first marker index, "always instant".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionStart { pub pos: i32, pub marker: i32, pub instant: bool }

#[derive(Clone, Debug, PartialEq)]
pub struct Smf { pub base_volume: i32, pub starts: Vec<SectionStart>, pub markers: Vec<Marker> }

impl Smf {
    pub fn parse(data: &[u8]) -> Result<Smf> {
        let b = Buf(data);
        let (ns, nm, so, mo, vol) = (b.i32(0)?, b.i32(4)?, b.i32(8)? as usize, b.i32(12)? as usize, b.i32(16)?);
        if ns < 0 || nm < 0 { return invalid("SMF counts"); }
        let starts = (0..ns as usize).map(|k| { let o = so + 52 * k; Ok(SectionStart { pos: b.i32(o + 4)?, marker: b.i32(o + 0x20)?, instant: b.i32(o + 0x24)? != 0 }) }).collect::<Result<_>>()?;
        let markers = (0..nm as usize).map(|k| {
            let o = mo + 32 * k;
            Ok(Marker { name: b.i32(o)?, pos: b.i32(o + 4)?, mtype: b.i32(o + 8)?, loop_start: b.i32(o + 0x14)?, loop_marker: b.i32(o + 0x1c)? })
        }).collect::<Result<_>>()?;
        Ok(Smf { base_volume: vol, starts, markers })
    }
}

/// A stereo SSD decoder: blocks are decoded in the order they are played, each channel keeping its ADPCM history
/// across reads, loops and jumps (the SPU voice decodes its ring continuously).
#[derive(Clone, Debug, Default)]
pub struct SsdDecoder { hist: [crate::vag::History; 2] }

impl SsdDecoder {
    /// Decodes `bytes` (a multiple of 0x100) into interleaved stereo samples (L, R), 224 frames per block.
    pub fn decode(&mut self, bytes: &[u8]) -> Vec<[i16; 2]> {
        let mut out = Vec::with_capacity(bytes.len() / 256 * 224);
        for blk in bytes.chunks_exact(256) {
            let mut ch: [Vec<i16>; 2] = [Vec::with_capacity(224), Vec::with_capacity(224)];
            for (c, half) in [&blk[..128], &blk[128..]].into_iter().enumerate() {
                for f in half.chunks_exact(16) { ch[c].extend(crate::vag::decode_frame(f, &mut self.hist[c], crate::vag::Variant::Rounded)); }
            }
            out.extend(ch[0].iter().zip(&ch[1]).map(|(&l, &r)| [l, r]));
        }
        out
    }
}

/// `MapMusic` (ACTION.ELF 0x24f4d8, 35 × 76 bytes): per level the track, start section, the end stings and the
/// combat / stealth section sets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapMusic {
    pub level: u32,
    pub track: i32,
    pub start: i32,
    pub on_death: i32,
    pub on_fail: i32,
    pub end_combat: i32,
    pub end_calm: i32,
    pub alert_frames: i32,
    pub calm_frames: i32,
    pub combat: Vec<i32>,
    pub stealth: Vec<i32>,
}

/// `MapDroneData` (0x24f418, 12 × 16 bytes): the `MusicVars` of a level — attackers needed, recent-sight window
/// (seconds), distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicVars { pub level: u32, pub count: u16, pub floor: u16, pub window: u32, pub distance: f32 }

fn symbol<'a>(elf: &Elf, data: &'a [u8], segs: &[(u32, &'a [u8], u32)], name: &str) -> Result<&'a [u8]> {
    let syms = elf.symbols(data)?;
    let (addr, size) = syms.iter().find(|s| s.name == name).map(|s| (s.value, s.size as usize))
        .ok_or_else(|| crate::buf::FormatError::Invalid(format!("ACTION.ELF has no symbol {name}")))?;
    segs.iter().find_map(|(va, b, _)| (addr >= *va && (addr - va) as usize + size <= b.len()).then(|| &b[(addr - va) as usize..][..size]))
        .ok_or_else(|| crate::buf::FormatError::Invalid(format!("{name} outside the loaded data")))
}

/// Both tables from ACTION.ELF.
pub fn tables(data: &[u8]) -> Result<(Vec<MapMusic>, Vec<MusicVars>)> {
    let elf = Elf::parse(data)?;
    let segs = elf.load_segments(data)?;
    let mm = Buf(symbol(&elf, data, &segs, "MapMusic")?);
    let set = |o: usize| -> Result<Vec<i32>> { let mut v = Vec::new(); for k in 0..5 { let x = mm.i32(o + 4 * k)?; if x == -1 { break; } v.push(x); } Ok(v) };
    let music = (0..mm.len() / 76).map(|k| {
        let o = 76 * k;
        let i = |f: usize| mm.i32(o + f);
        Ok(MapMusic { level: mm.u32(o)?, track: i(4)?, start: i(8)?, on_death: i(0xc)?, on_fail: i(0x10)?, end_combat: i(0x14)?, end_calm: i(0x18)?,
            alert_frames: i(0x1c)?, calm_frames: i(0x20)?, combat: set(o + 0x24)?, stealth: set(o + 0x38)? })
    }).collect::<Result<Vec<_>>>()?;
    let md = Buf(symbol(&elf, data, &segs, "MapDroneData")?);
    let vars = (0..md.len() / 16).map(|k| { let o = 16 * k; Ok(MusicVars { level: md.u32(o)?, count: md.u16(o + 4)?, floor: md.u16(o + 6)?, window: md.u32(o + 8)?, distance: md.f32(o + 12)? }) }).collect::<Result<Vec<_>>>()?;
    Ok((music, vars))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smf_and_mxi() {
        let mut v = Vec::new();
        for w in [1i32, 2, 20, 72, 99] { v.extend(w.to_le_bytes()); }
        let mut start = vec![0u8; 52];
        start[4..8].copy_from_slice(&1024i32.to_le_bytes());
        start[0x20..0x24].copy_from_slice(&1i32.to_le_bytes());
        start[0x24..0x28].copy_from_slice(&(-1i32).to_le_bytes());
        v.extend(start);
        for m in [[0i32, 1024, 10, 2, 0, 0, 0, 0], [0, 4096, 6, 2, 0, 1024, 1, 0]] { for w in m { v.extend(w.to_le_bytes()); } }
        let s = Smf::parse(&v).unwrap();
        assert_eq!((s.base_volume, s.starts[0], s.markers[1].mtype, s.markers[1].loop_start), (99, SectionStart { pos: 1024, marker: 1, instant: true }, 6, 1024));
        assert!(s.markers[0].eligible() && s.markers[1].eligible());
        let mxi: Vec<u8> = [15i32, 16, -1].iter().flat_map(|x| x.to_le_bytes()).collect();
        assert_eq!(track_file(&mxi, 15).unwrap(), Some(1));
        assert_eq!(music_path(1, "SSD"), "PS2/MUSIC/MFX_0/MFX_1.SSD");
        // Sister: an unknown track has no file.
        assert_eq!(track_file(&mxi, 99).unwrap(), None);
    }
}
