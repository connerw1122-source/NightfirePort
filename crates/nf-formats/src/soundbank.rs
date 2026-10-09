//! Sound banks `PS2/<language>/SB_<slot / 8>/SB_<slot>.{SFX,SHF,SBF}` and `PS2/SBINFO.SBI`. Spec:
//! docs/formats/sound-bank.md.
//!
//! `SBINFO.SBI`: 150 × u32 bank hashcodes; a bank's slot is the index of its hash (`FindSoundBankSlot`, SFX.IRX
//! 0x2070). `.SHF`: u32 count, count × 36-byte sample headers. `.SBF`: SPU ADPCM, the samples back to back (the loader
//! sets each header's address to the running sum of sizes, `SFXLoadSoundBank` 0x25a4; on our disc the stored offsets
//! equal it). `.SFX`: u32 count, count × (u32 effect id, u32 offset of its parameter block in the file).

use crate::buf::{invalid, Buf, Result};

/// `SBINFO.SBI`: the bank hashcodes by slot.
pub fn bank_slots(sbi: &[u8]) -> Result<Vec<u32>> {
    let b = Buf(sbi);
    if b.len() % 4 != 0 { return invalid("SBINFO size is not a multiple of 4"); }
    (0..b.len() / 4).map(|k| b.u32(4 * k)).collect()
}

/// The disc path of a bank slot's file (`ext` = "SFX", "SHF" or "SBF").
pub fn bank_path(language: &str, slot: usize, ext: &str) -> String { format!("PS2/{language}/SB_{}/SB_{slot}.{ext}", slot >> 3) }

/// One `.SHF` sample header (`SampleHeaderData`, 36 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleHeader {
    /// +0: the sample loops (*suggested* meaning; 7 of 252 on bank 4).
    pub looped: u32,
    /// +4: byte offset in the `.SBF`; +8: size in bytes.
    pub offset: u32,
    pub size: u32,
    /// +0xC: SPU pitch (0x1000 = 48 kHz).
    pub pitch: u32,
    /// +0x10, +0x14 (channels, 1), +0x18 (4 = ADPCM bits), +0x1C, +0x20: kept raw.
    pub raw: [u32; 5],
}

/// `SFXParameters` (0x44 bytes, names from SFX.IRX's stabs) and its `SFXSamplePoolFiles` sub-tracks (28 bytes each;
/// one all-zero record always follows the last).
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub reverb_send: i32,
    /// 0 2-D, 1 distance only (centred), 2 full 3-D, 3 attached to the listener.
    pub tracking: i32,
    pub inner: i32,
    pub outer: i32,
    pub max_voices: i32,
    pub priority: i32,
    pub max_reject: i16,
    /// Duck amount (music ducks to 100 − this); also "is a ducker".
    pub ducker: i16,
    pub ducker_length: i32,
    pub master_volume: i32,
    pub min_delay: i32,
    pub max_delay: i32,
    pub multi_sample: i16,
    pub shuffled: i16,
    pub looped: i16,
    pub polyphonic: i16,
    pub outdoors: u8,
    pub pause_in_nis: u8,
    /// `SampleCount` records plus the trailing zero one (index `SampleCount`, what `SFXrnd(0)` = 0 picks when the
    /// count is 0).
    pub subs: Vec<SubTrack>,
    pub count: usize,
}

/// `SFXSamplePoolFiles`: sample index (< 0: a stream), pitch offset and its random range (0x3000 = +100 %), volume
/// 0..100 and its range, pan −100..100 and its range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SubTrack { pub file: i32, pub pitch: i32, pub pitch_rnd: i32, pub volume: i32, pub volume_rnd: i32, pub pan: i32, pub pan_rnd: i32 }

impl Effect {
    pub fn parse(block: &[u8]) -> Result<Effect> {
        let b = Buf(block);
        let (i, h) = (|o: usize| b.i32(o), |o: usize| b.i16(o));
        let count = i(0x40)?.max(0) as usize;
        let subs = (0..=count).map(|k| {
            let o = 0x44 + 28 * k;
            Ok(SubTrack { file: i(o)?, pitch: i(o + 4)?, pitch_rnd: i(o + 8)?, volume: i(o + 12)?, volume_rnd: i(o + 16)?, pan: i(o + 20)?, pan_rnd: i(o + 24)? })
        }).collect::<Result<Vec<_>>>()?;
        Ok(Effect {
            reverb_send: i(0)?, tracking: i(4)?, inner: i(8)?, outer: i(0xc)?, max_voices: i(0x10)?, priority: i(0x14)?,
            max_reject: h(0x1c)?, ducker: h(0x22)?, ducker_length: i(0x24)?, master_volume: i(0x28)?, min_delay: i(0x2c)?,
            max_delay: i(0x30)?, multi_sample: h(0x34)?, shuffled: h(0x38)?, looped: h(0x3a)?, polyphonic: h(0x3c)?,
            outdoors: b.u8(0x3e)?, pause_in_nis: b.u8(0x3f)?, subs, count,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Bank {
    pub samples: Vec<SampleHeader>,
    /// `.SFX`: effect id → offset of its parameter block, in file order.
    pub effects: Vec<(u32, u32)>,
    /// The whole `.SFX` file (parameter blocks are read from it by offset).
    pub sfx: Vec<u8>,
}

impl Bank {
    pub fn parse(shf: &[u8], sfx: &[u8]) -> Result<Bank> {
        let b = Buf(shf);
        let n = b.u32(0)? as usize;
        if 4 + 36 * n != b.len() { return invalid(format!("SHF: {n} headers do not fill {} bytes", b.len())); }
        let samples = (0..n).map(|k| {
            let o = 4 + 36 * k;
            let w = |i: usize| b.u32(o + 4 * i);
            Ok(SampleHeader { looped: w(0)?, offset: w(1)?, size: w(2)?, pitch: w(3)?, raw: [w(4)?, w(5)?, w(6)?, w(7)?, w(8)?] })
        }).collect::<Result<Vec<_>>>()?;
        let s = Buf(sfx);
        let m = s.u32(0)? as usize;
        let effects = (0..m).map(|k| Ok((s.u32(4 + 8 * k)?, s.u32(8 + 8 * k)?))).collect::<Result<Vec<_>>>()?;
        if effects.iter().any(|&(_, o)| o as usize >= sfx.len()) { return invalid("SFX: a parameter block outside the file"); }
        Ok(Bank { samples, effects, sfx: sfx.to_vec() })
    }

    /// The parameter block of effect `id` (from its offset to the end of the file).
    pub fn effect(&self, id: u32) -> Option<&[u8]> {
        self.effects.iter().find(|e| e.0 == id).map(|&(_, o)| &self.sfx[o as usize..])
    }

    /// Sample `k` decoded as a voice plays it: PCM and the loop start (samples) when it repeats.
    pub fn decode(&self, sbf: &[u8], k: usize) -> Result<(Vec<i16>, Option<usize>)> {
        let h = self.samples.get(k).ok_or_else(|| crate::buf::FormatError::Invalid(format!("no sample {k}")))?;
        let end = (h.offset as usize).saturating_add(h.size as usize);
        if end > sbf.len() { return invalid(format!("sample {k} past the SBF")); }
        let blob = &sbf[h.offset as usize..end];
        let ext = crate::vag::sample_extent(blob, 0)?;
        Ok((crate::vag::decode_extent(blob, &ext), ext.loop_points().map(|l| l.0)))
    }
}

/// `STREAMS.LUT` (`PS2/<language>/STREAMS/`): 16-byte entries {header offset, header size (136), data offset
/// (header + 4096), data size} into `STREAMS.BIN`; a bank sub-track with FileRef < 0 plays entry −1 − FileRef.
pub fn stream_lut(lut: &[u8]) -> Result<Vec<[u32; 4]>> {
    let b = Buf(lut);
    (0..b.len() / 16).map(|k| Ok([b.u32(16 * k)?, b.u32(16 * k + 4)?, b.u32(16 * k + 8)?, b.u32(16 * k + 12)?])).collect()
}

/// A streamed effect's ADPCM in `STREAMS.BIN`: from the data offset, `end & !0xFF` bytes, where `end` is the position
/// of the header's type-9 marker (the IOP streams no further; the LUT's data size includes stale takes) [M, D].
pub fn stream_data<'a>(bin: &'a [u8], e: [u32; 4]) -> Result<&'a [u8]> {
    let h = Buf(bin).sub(e[0] as usize, e[1] as usize, "stream header")?;
    let (n, mo) = (h.u32(4)? as usize, h.u32(0xc)? as usize);
    let end = (0..n).filter_map(|k| { let o = mo + 32 * k; (h.i32(o + 8).ok()? == 9).then(|| h.u32(o + 4).ok()).flatten() }).next()
        .ok_or_else(|| crate::buf::FormatError::Invalid("stream without an end marker".into()))?;
    Ok(Buf(bin).sub(e[2] as usize, (end & !0xff) as usize, "stream data")?.bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headers_and_effects() {
        let mut shf = 2u32.to_le_bytes().to_vec();
        for k in 0..2u32 { for w in [0, k * 32, 32, 0x400, 0, 1, 4, 0, 0] { shf.extend(w.to_le_bytes()); } }
        let mut sfx = 1u32.to_le_bytes().to_vec();
        sfx.extend(139u32.to_le_bytes());
        sfx.extend(12u32.to_le_bytes());
        sfx.extend([7u8; 4]);
        let b = Bank::parse(&shf, &sfx).unwrap();
        assert_eq!((b.samples.len(), b.samples[1].offset, b.samples[1].pitch), (2, 32, 0x400));
        assert_eq!(b.effect(139), Some(&[7u8, 7, 7, 7][..]));
        assert_eq!(bank_path("ENGLISH", 4, "SHF"), "PS2/ENGLISH/SB_0/SB_4.SHF");
        // Sister: a header count that does not fill the file is refused; an unknown effect is absent.
        assert!(Bank::parse(&shf[..shf.len() - 1], &sfx).is_err());
        assert_eq!(b.effect(140), None);
        // An effect block: tracking 2, inner 2, outer 30, one sub-track (+ the zero record).
        let mut blk = vec![0u8; 0x44 + 28 * 2];
        for (o, v) in [(4usize, 2i32), (8, 2), (0xc, 30), (0x40, 1), (0x44, 185), (0x44 + 12, 80)] { blk[o..o + 4].copy_from_slice(&v.to_le_bytes()); }
        let e = Effect::parse(&blk).unwrap();
        assert_eq!((e.tracking, e.inner, e.outer, e.count, e.subs[0].file, e.subs[0].volume, e.subs[1]), (2, 2, 30, 1, 185, 80, SubTrack::default()));
        assert!(Effect::parse(&blk[..0x44 + 28]).is_err(), "the trailing zero record is required");
    }
}
