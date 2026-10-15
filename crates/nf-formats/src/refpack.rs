//! EA RefPack (QFS) decompressor — the `10 FB`-magic streams inside the driving archives
//! (`DRIVING/*.VIV`, docs/formats/driving-viv.md).
//!
//! Implemented from the game's own decompressor [M] (disassembly-verified; DRIVING.ELF, EE = file offset
//! + 0x107080):
//! - `0x00323bb0` — the entry gate: requires `stream[1] == 0xFB`, accepts flag bytes `0x10`/`0x90`
//!   (plus their bit-0 set twins `0x11`/`0x91` — the gate masks bit 0 off) and routes `0x1E`/`0x9E` to a
//!   pluggable decoder that is **never registered** in this build (the pointer at EE `0x3C8150` reads 0 and
//!   has only loads, 0x323c80/0x323e04/0x323e34) — those streams always fail;
//! - `0x00323e98` — the decompressor: header parse + the opcode loop;
//! - `0x00323cc8` — the size probe. It has a quirk the real decompressor does not share: it ignores the
//!   bit-0x01 compressed-size field and would read the compressed size as the decompressed size. This port
//!   follows the real decompressor's header code for both.
//!
//! Header (from `0x00323e98` lines 24-41):
//!
//! ```text
//! 0x00  u8   flags        bit 0x80: decompressed-size field is 4 bytes BE instead of 3 bytes BE
//!                         bit 0x01: a compressed-size field of the SAME width sits first and is skipped
//! 0x01  u8   0xFB         (magic; checked by 0x00323bb0)
//! 0x02  u8×3/4        [compressed size if flags bit 0x01, skipped — never used by the game]
//!       u8×3/4  decompressed size, big-endian
//! ```
//!
//! Opcode stream (from the disassembly at 0x323f50-0x3241b0; all offsets are +1 corrections, all copies
//! forward byte-wise so overlapping/RLE back-references behave identically):
//!
//! | op range   | stream                                   | literals first        | length                    | offset                                  |
//! |------------|------------------------------------------|-----------------------|---------------------------|-----------------------------------------|
//! | `00-7F`    | `[op][D1][lit × (op & 3)]`               | after D1              | `((op >> 2) & 7) + 3`     | `(D1 | ((op & 0x60) << 3)) + 1` (10-bit) |
//! | `80-BF`    | `[op][D1][D2][lit × (D1 >> 6)]`          | after D2              | `(op & 0x3F) + 4`         | `((D1 & 0x3F) << 8 | D2) + 1` (14-bit)   |
//! | `C0-DF`    | `[op][D1][D2][D3][lit × (op & 3)]`       | after D3              | `((op & 0x0C) << 6 | D3) + 5` | `(D2 | D1 << 8 | bit) + 1` (17-bit, `bit = 0x10000` if `op & 0x10`) |
//! | `E0-FB`    | `[op][lit × ((op & 0x1F) << 2) + 4]`     | —                     | —                         | —                                       |
//! | `FC-FF`    | stop; `[lit × (op & 3)]` tail then end   | —                     | —                         | —                                       |
//!
//! **Measured divergence from the published community spec** (EA RefPack/QFS as documented by the EA
//! modding community, docs/research/driving-engine-ea-formats.md; also Sims/SC4DbPF write-ups): the public
//! spec orders the `00-7F` command as `(op & 3) + 1` literals **first**, then D1. The game reads **D1
//! first**, then exactly `(op & 3)` literals (disasm: `lbu $a0,($a3)` for D1 at 0x323f60 precedes the
//! literal loop; decompile line order agrees). Both orderings were run over every RefPack entry of the
//! driving archives: the game's ordering decodes **1126/1126** streams to exactly their header-announced
//! size with the stream consumed to the last byte; the public ordering decodes **3/1126**
//! (crates/nf-formats/tests/driving_golden.rs, golden/driving_refpack.tsv). For this data the game wins.
//!
//! The game does not validate that the output length equals the header size (it just returns the header
//! value); this port returns the bytes actually produced and leaves that equality to callers/tests.

use crate::buf::{invalid, FormatError, Result};

/// True when the blob starts with the RefPack magic `10 FB` (measured classifier,
/// docs/formats/driving-viv.md; the same test the game's gate makes via `stream[1] == 0xFB`).
pub fn is_refpack(blob: &[u8]) -> bool { blob.len() >= 2 && blob[..2] == [0x10, 0xFB] }

/// The header's announced decompressed size, parsed exactly like the real decompressor's prologue
/// [M 0x00323e98 lines 24-41]. Returns the size and the offset where the opcode stream starts.
pub fn decompressed_size(stream: &[u8]) -> Result<(usize, usize)> {
    parse_header(stream).map(|(size, opcodes)| (size, opcodes))
}

fn parse_header(stream: &[u8]) -> Result<(usize, usize)> {
    if stream.len() < 2 {
        return invalid(format!("RefPack stream too short for a header ({})", stream.len()));
    }
    if stream[1] != 0xFB {
        return invalid(format!("RefPack magic: second byte {:#x} != 0xFB", stream[1]));
    }
    let flags = stream[0];
    let four_byte = flags & 0x80 != 0;
    let skip_compressed = flags & 0x01 != 0;
    let width = if four_byte { 4 } else { 3 };
    let mut pos = 2usize;
    if skip_compressed {
        pos += width; // the compressed-size field; the game skips it without reading [M]
    }
    let size_bytes = stream
        .get(pos..pos + width)
        .ok_or_else(|| FormatError::Invalid(format!("RefPack size field truncated at {pos}")))?;
    pos += width;
    let size = size_bytes.iter().fold(0usize, |acc, &b| (acc << 8) | b as usize);
    Ok((size, pos))
}

/// Decompresses a full RefPack stream (header + opcode stream) into a fresh `Vec<u8>`.
pub fn decompress(stream: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    decompress_into(stream, &mut out)?;
    Ok(out)
}

/// Decompresses into a caller-provided vector (cleared first). Returns the produced bytes.
///
/// Bounds-checked throughout — the game's loop trusts the data and has no length checks [M], which a port
/// cannot afford: malformed streams error instead of panicking or overrunning.
pub fn decompress_into(stream: &[u8], out: &mut Vec<u8>) -> Result<usize> {
    let (_announced, mut pos) = parse_header(stream)?;
    out.clear();
    let n = stream.len();
    // The game loops until it sees a 0xFC-0xFF opcode [M 0x32413c: sltiu $v0,$v1,0x71]; a stream that runs
    // out of bytes first is an error here (the game would read past its buffer).
    while pos < n {
        let op = stream[pos];
        pos += 1;
        if op < 0x80 {
            // 00-7F: [op][D1][lit × (op & 3)]; D1 is read BEFORE the literals [M 0x323f60].
            let Some(&d1) = stream.get(pos) else { return invalid("RefPack: truncated 00-7F command") };
            pos += 1;
            let lits = stream.get(pos..pos + (op & 3) as usize)
                .ok_or_else(|| FormatError::Invalid("RefPack: truncated 00-7F literals".into()))?;
            pos += (op & 3) as usize;
            out.extend_from_slice(lits);
            let length = ((op >> 2) & 7) as usize + 3;
            let offset = (d1 as usize | (((op & 0x60) as usize) << 3)) + 1;
            back_copy(out, offset, length)?;
        } else if op < 0xC0 {
            // 80-BF: [op][D1][D2][lit × (D1 >> 6)] [M 0x323ff8-0x32404c].
            let Some(&d1) = stream.get(pos) else { return invalid("RefPack: truncated 80-BF command") };
            let Some(&d2) = stream.get(pos + 1) else { return invalid("RefPack: truncated 80-BF command") };
            pos += 2;
            let nlit = (d1 >> 6) as usize;
            let lits = stream.get(pos..pos + nlit)
                .ok_or_else(|| FormatError::Invalid("RefPack: truncated 80-BF literals".into()))?;
            pos += nlit;
            out.extend_from_slice(lits);
            let length = (op & 0x3F) as usize + 4;
            let offset = (((d1 & 0x3F) as usize) << 8 | d2 as usize) + 1;
            back_copy(out, offset, length)?;
        } else if op < 0xE0 {
            // C0-DF: [op][D1][D2][D3][lit × (op & 3)]; 17-bit offset with the 0x10000 bit from op & 0x10
            // [M 0x32408c-0x324100].
            if pos + 3 > n {
                return invalid("RefPack: truncated C0-DF command");
            }
            let (d1, d2, d3) = (stream[pos], stream[pos + 1], stream[pos + 2]);
            pos += 3;
            let lits = stream.get(pos..pos + (op & 3) as usize)
                .ok_or_else(|| FormatError::Invalid("RefPack: truncated C0-DF literals".into()))?;
            pos += (op & 3) as usize;
            out.extend_from_slice(lits);
            let length = (((op & 0x0C) as usize) << 6 | d3 as usize) + 5;
            let high = if op & 0x10 != 0 { 0x10000 } else { 0 };
            let offset = (d2 as usize | (d1 as usize) << 8 | high) + 1;
            back_copy(out, offset, length)?;
        } else if op < 0xFC {
            // E0-FB: pure literal run of ((op & 0x1F) << 2) + 4 bytes [M 0x324134].
            let count = (((op & 0x1F) as usize) << 2) + 4;
            let lits = stream.get(pos..pos + count)
                .ok_or_else(|| FormatError::Invalid(format!("RefPack: literal run truncated ({count} wanted)")))?;
            pos += count;
            out.extend_from_slice(lits);
        } else {
            // FC-FF: stop; (op & 3) trailing literals then the stream ends [M 0x32417c-0x3241b0].
            let tail = (op & 3) as usize;
            let lits = stream.get(pos..pos + tail)
                .ok_or_else(|| FormatError::Invalid("RefPack: truncated stop literals".into()))?;
            out.extend_from_slice(lits);
            return Ok(out.len());
        }
    }
    invalid("RefPack: stream ended without a stop opcode (FC-FF)")
}

/// One forward byte-wise back-reference copy. `offset` is from the CURRENT end of `out` (the game's
/// `param_1 + ~(offset - 1)` arithmetic [M]); the byte loop is load-bearing for overlap (RLE) copies —
/// `extend_from_within` would panic on ranges longer than the existing output.
fn back_copy(out: &mut Vec<u8>, offset: usize, length: usize) -> Result<()> {
    if offset > out.len() {
        return invalid(format!("RefPack: back-reference offset {offset} before start (out = {})", out.len()));
    }
    let src = out.len() - offset;
    for i in 0..length {
        let b = out[src + i];
        out.push(b);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wraps `payload` in a 0x10 header with a 3-byte announced size.
    fn stream(payload: &[u8], announced: usize) -> Vec<u8> {
        let mut s = vec![0x10, 0xFB, (announced >> 16) as u8, (announced >> 8) as u8, announced as u8];
        s.extend_from_slice(payload);
        s
    }

    #[test]
    fn header_variants_parse_like_the_game() {
        // 0x10: 3-byte size, opcodes at 5. [L real stream: paris_mis01.crp header 10 fb 68 29 00]
        let (size, at) = decompressed_size(&[0x10, 0xFB, 0x68, 0x29, 0x00]).unwrap();
        assert_eq!((size, at), (6_826_240, 5));
        // 0x90: 4-byte size, opcodes at 6.
        let (size, at) = decompressed_size(&[0x90, 0xFB, 0x00, 0x01, 0x00, 0x00]).unwrap();
        assert_eq!((size, at), (0x1_0000, 6));
        // 0x11: bit 0x01 — a 3-byte compressed-size field sits first and is skipped [M 0x323e98].
        let (size, at) = decompressed_size(&[0x11, 0xFB, 0xAB, 0xCD, 0xEF, 0x00, 0x10, 0x00]).unwrap();
        assert_eq!((size, at), (0x1000, 8));
        // 0x91: 4-byte size + skipped 4-byte compressed-size field.
        let (size, at) = decompressed_size(&[0x91, 0xFB, 1, 2, 3, 4, 0, 0, 0, 9]).unwrap();
        assert_eq!((size, at), (9, 10));
        // The gate rejects a wrong second byte [M 0x323bb0: ((byte*)param_1)[1] != 0xfb → 0].
        assert!(decompress(&[0x10, 0xFA, 0, 0, 1]).is_err());
        assert!(decompressed_size(&[0x10]).is_err());
    }

    #[test]
    fn pure_literal_runs_e0_fb() {
        // 0xE0 → 4 literals; 0xFB (0x1B << 2 | 4) → 112; stop 0xFC.
        let payload: Vec<u8> = {
            let mut p = vec![0xE0, b'P', b'Q', b'R', b'S'];
            p.push(0xE0 | 0x1B);
            p.extend(std::iter::repeat(b'x').take(112));
            p.push(0xFC);
            p
        };
        let out = decompress(&stream(&payload, 116)).unwrap();
        assert_eq!(out.len(), 116);
        assert_eq!(&out[..4], b"PQRS");
        assert!(out[4..].iter().all(|&b| b == b'x'));
    }

    #[test]
    fn short_command_00_7f_literals_after_d1_and_10_bit_offset() {
        // Emit "ABCDEFGH" via two 0xE0 runs (4 literals each), then a 0x03 command: literals (op&3)=0,
        // length ((0x03>>2)&7)+3 = 3, offset = D1+1 = 6 → src = 8-6 = 2 → copies out[2..5] = "CDE".
        let payload: Vec<u8> = [0xE0, b'A', b'B', b'C', b'D', 0xE0, b'E', b'F', b'G', b'H']
            .iter().copied().chain([0x00, 0x05, 0xFC]).collect(); // 0x00: 0 literals, length 3, offset 6
        let out = decompress(&stream(&payload, 11)).unwrap();
        assert_eq!(out, b"ABCDEFGHCDE");
        // With a literal in the 00-7F command: op&3 = 1 → ONE literal copied AFTER the offset byte (the
        // game's divergence from the public spec), then the match. op = 0x01: literals 1, offset bits
        // (op&0x60) clear, length ((0x01>>2)&7)+3 = 3, offset = D1+1 = 4 → out "xyzw!" + out[1..4] = "yzw".
        let p: Vec<u8> = [0xE0, b'x', b'y', b'z', b'w'].iter().copied().chain([0x01, 0x03, b'!', 0xFC]).collect();
        let out = decompress(&stream(&p, 8)).unwrap();
        assert_eq!(out, b"xyzw!yzw");
    }

    #[test]
    fn medium_command_80_bf_with_prefixed_literals() {
        // Two 0xE0 runs build "01234567"; then 0x8A: length (0x8A&0x3F)+4 = 14; D1 = 0x80 →
        // literals (D1>>6) = 2 ('8','9' appended after D2 — building "0123456789"), offset
        // ((0x80&0x3F)<<8 | D2)+1 = 6 → src = 4, overlap-cycles out[4..] = "456789" for 14 bytes.
        let mut payload = vec![0xE0, b'0', b'1', b'2', b'3', 0xE0, b'4', b'5', b'6', b'7'];
        payload.extend_from_slice(&[0x8A, 0x80, 0x05, b'8', b'9', 0xFC]);
        let out = decompress(&stream(&payload, 24)).unwrap();
        assert_eq!(&out[..10], b"0123456789");
        assert_eq!(out.len(), 24);
        assert_eq!(&out[10..], b"45678945678945"); // offset 6 → src 4: "456789" cycles
    }

    #[test]
    fn long_command_c0_df_17_bit_offset() {
        // The 0x10000 offset bit needs > 64 KiB of output first; build it with overlapping 0xC0-class
        // copies (length 1028, offset 1 doubles+ each time) and verify the 17-bit offset lands 64 KiB back.
        let mut payload = vec![0xE0, b'A', b'B', b'C', b'D']; // 4 bytes
        // Grow with 0xC0-class RLE: op = 0xC8 → length = ((0xC8&0x0C)<<6 | 0xFF)+5 = 772;
        // offset = (D2|D1<<8)+1 = 1 → each copy repeats the previous byte ('D') 772 times.
        for _ in 0..90 {
            payload.extend_from_slice(&[0xC8, 0x00, 0x00, 0xFF]);
        }
        // Output = 4 + 90*772 = 69,484 ≥ 65,537. Append the 17-bit command: op&0x10 set, D1 = D2 = 0,
        // D3 = 0x02: length = ((op&0x0C)<<6 | 0x02)+5 = 7; offset = 0x10000 + 1 = 65,537.
        // op bits: 0xC0 | 0x10 (offset bit) | 0x0C-class 0 | literals 0 = 0xD0.
        payload.extend_from_slice(&[0xD0, 0x00, 0x00, 0x02, 0xFC]);
        let out = decompress(&stream(&payload, 69_484 + 7)).unwrap();
        assert_eq!(out.len(), 69_484 + 7);
        // The 17-bit offset copies the bytes that sat 65,537 bytes back.
        let src = 69_484 - 65_537;
        assert_eq!(&out[69_484..], &out[src..src + 7]);
        // The RLE ran from the last literal: "ABCD" then 'D' repeats.
        assert_eq!(&out[..4], b"ABCD");
        assert_eq!(out[4], b'D');
        assert_eq!(out[69_491 - 1], b'D');
    }

    #[test]
    fn stop_variants_carry_trailing_literals() {
        // 0xFC: no tail. 0xFD: 1. 0xFE: 2. 0xFF: 3. [M 0x32417c: while (uVar5 = uVar5-1) != -1 copy]
        for (stop, tail) in [(0xFCu8, 0usize), (0xFD, 1), (0xFE, 2), (0xFF, 3)] {
            let mut payload = vec![0xE0, b'1', b'2', b'3', b'4', stop];
            payload.extend(std::iter::repeat(b'z').take(tail));
            let out = decompress(&stream(&payload, 4 + tail)).unwrap();
            assert_eq!(out.len(), 4 + tail);
            assert_eq!(&out[..4], b"1234");
            assert_eq!(&out[4..], &vec![b'z'; tail][..]);
        }
    }

    #[test]
    fn rle_overlap_runs_forward_like_the_game() {
        // One 'A' then 0x00-class: op = 0b0_11110_11 = 0x7B → literals 3 (garbage follows), length
        // ((0x7B>>2)&7)+3 = 10, offset (D1 | 0x300)+1 — with D1 = 0xFF → 0x3FF+1 = 1024. Too big.
        // Simpler classic RLE: emit 'A' (0xE0 run of 4? no—) use stop-tail? Build: literals "AB" via
        // 0xE1 (4), then 0x07: literals (7&3)=3, length ((7>>2)&7)+3 = 4, offset (D1|0)+1 = 1 →
        // copies the last byte 4 times (overlap RLE).
        let mut payload = vec![0xE0, b'A', b'B', b'C', b'D'];
        payload.extend_from_slice(&[0x07, 0x00, b'9', b'8', b'7', 0xFC]); // 3 literals '987', then 4×'7'
        let out = decompress(&stream(&payload, 11)).unwrap();
        assert_eq!(out, b"ABCD9877777");
    }

    #[test]
    fn malformed_streams_error_not_panic() {
        // No stop opcode → error (the game would run past its buffer).
        let mut s = stream(&[0xE0, 1, 2, 3, 4], 4);
        s.truncate(5); // literal run now truncated
        assert!(decompress(&s).is_err());
        // Offset before start.
        assert!(decompress(&stream(&[0x03, 0x7F, 0xFC], 0)).is_err());
        // Truncated stop tail.
        assert!(decompress(&stream(&[0xE0, 1, 2, 3, 4, 0xFF, 1], 5)).is_err());
        // Truncated 80-BF command.
        assert!(decompress(&stream(&[0x80, 0xFC], 0)).is_err());
        // Truncated C0-DF command.
        assert!(decompress(&stream(&[0xC0, 1, 2, 0xFC], 0)).is_err());
        // Truncated literal run inside a 00-7F command.
        assert!(decompress(&stream(&[0x02, 0x10, b'a', 0xFC], 1)).is_err());
    }

    #[test]
    fn hand_built_round_trip_matches_known_text() {
        // The measured default.tun keys (driving-viv.md) with a real repeat, encoded with literals + one
        // 0x80-class back-reference: the second "GRAVITY=-34\n" is a 12-byte copy from offset 24.
        let text = b"GRAVITY=-34\nSLEEP_VEL=5\nGRAVITY=-34\n";
        let mut payload = vec![0xE2]; // (2<<2)+4 = 12 literals
        payload.extend_from_slice(b"GRAVITY=-34\n");
        payload.extend_from_slice(&[0xE0, b'S', b'L', b'E', b'E']);
        payload.extend_from_slice(&[0xE0, b'P', b'_', b'V', b'E']);
        payload.extend_from_slice(&[0xE0, b'L', b'=', b'5', b'\n']); // out is now 24 bytes
        payload.extend_from_slice(&[0x88, 0x00, 23, 0xFC]); // 0x88: length (0x88&0x3F)+4 = 12,
        // offset ((0x00&0x3F)<<8 | 23)+1 = 24 → copies out[0..12] = "GRAVITY=-34\n".
        let out = decompress(&stream(&payload, 36)).unwrap();
        assert_eq!(out, text);
    }
}
