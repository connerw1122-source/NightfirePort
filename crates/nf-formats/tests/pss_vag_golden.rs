//! G-008 golden tests for the disc's own PSS/VAG data (`NF_ISO`, numbers only — no game bytes in the repo).
//!
//! The slim on-foot image carries no movie data (all 22 `MOVIES/30_FPS/*.PSS` entries are size-0 stubs), but it
//! carries every VAG/SPU-ADPCM stream the game plays: the 25 sound banks (`.SBF`, mono, per-sample extents from
//! `.SHF`), the 16 music streams (`MFX_n.SSD`, stereo 32 kHz, 0x100-byte blocks of 0x80+0x80) and the speech
//! streams (`STREAMS.BIN` via `STREAMS.LUT`, 22 050 Hz). This file pins their structure and decode output to
//! `golden/pss_vag.tsv` (measured on this image) with sister assertions that corrupted headers fail. The movie
//! golden rows live in `golden/pss_movies.tsv` (measured on the retail image, `pss_movies_live.rs`); the PSS test
//! here checks the tree on any image and re-runs the demux golden on images with movie data.
//!
//! Independent cross-checks (2026-10-15): ffmpeg's `vag` demuxer (`adpcm_psx`) decodes the port's VAG frames
//! bit-exactly for the frame/nibble/shift/filter structure — its prediction is trunc-toward-zero of the summed
//! products (`(k0·h1 + k1·h2) / 64`), which matches neither of the port's two variants (Rounded drifted ≤ 27
//! over 16 884 bank samples, OpenGoal ≤ 78); DuckStation (master `spu.cpp`) shifts each product separately (the
//! OpenGoal form); vgmstream (`psx_decoder.c`) uses float coefficients. None of the three software decoders
//! implements the port's Rounded form, so the rounding attribution stays two-variant (`vag.rs` / `docs/formats/pss.md`
//! record this).

use nf_formats::golden::diff_tsv;
use nf_formats::iso9660::IsoImage;
use nf_formats::music::music_path;
use nf_formats::sha1::{hex, sha1, Sha1};
use nf_formats::soundbank::{bank_path, bank_slots, stream_data, stream_lut, Bank, Effect};

fn iso() -> Option<IsoImage> {
    let path = std::env::var_os("NF_ISO")?;
    Some(IsoImage::open(&path).unwrap_or_else(|e| panic!("iso {path:?}: {e}")))
}

fn get(iso: &IsoImage, p: &str) -> Vec<u8> {
    let e = iso.find(p).unwrap_or_else(|| panic!("{p} not in the image tree"));
    iso.read_file(&e).expect(p)
}

fn bytewise(pcm: &[i16]) -> Vec<u8> {
    let mut b = Vec::with_capacity(pcm.len() * 2);
    for v in pcm {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

const GOLDEN: &str = include_str!("../golden/pss_vag.tsv");

fn golden_section(header: &str, prefix: &str) -> String {
    let rows: Vec<&str> = GOLDEN.lines().filter(|l| l.starts_with(prefix)).collect();
    format!("{header}\n{}", rows.join("\n"))
}

/// All 25 sound banks: `.SHF`/`.SFX`/`.SBF` parse, the sample sizes sum to the `.SBF`, every sample's extent
/// ends with the end flag (the port's `sample_extent`), and the looped counts, per-bank decoded-PCM SHA-1s and
/// pitch classes hold against the golden rows. Every looped sample's loop points are validated (frame
/// multiples, start < end, end = the sample's extent) and hashed into the per-bank `loop_sha1` column, with
/// the sum of all loop starts as a plain-number cross-check (`loop_starts_sum`).
#[test]
fn banks_decode_to_the_golden_numbers() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let slots = bank_slots(&get(&iso, "PS2/SBINFO.SBI")).unwrap();
    let used: Vec<usize> = (0..25).filter(|&s| slots[s] != 0xffffffff).collect();
    assert_eq!(used.len(), 25, "all 25 bank slots used");
    let mut rows = Vec::new();
    for &slot in &used {
        let shf = get(&iso, &bank_path("ENGLISH", slot, "SHF"));
        let sfx = get(&iso, &bank_path("ENGLISH", slot, "SFX"));
        let sbf = get(&iso, &bank_path("ENGLISH", slot, "SBF"));
        let bank = Bank::parse(&shf, &sfx).unwrap_or_else(|e| panic!("bank {slot}: {e}"));
        let sizes: u64 = bank.samples.iter().map(|s| s.size as u64).sum();
        assert_eq!(sizes, sbf.len() as u64, "bank {slot}: sample sizes fill the SBF");
        let mut looped = 0usize;
        let mut samples = 0usize;
        let mut pcm = Sha1::new();
        let mut loops = Sha1::new();
        let mut loop_start_sum = 0u64;
        for s in &bank.samples {
            let blob = &sbf[s.offset as usize..(s.offset + s.size) as usize];
            let ext = nf_formats::vag::sample_extent(blob, 0).unwrap_or_else(|e| panic!("bank {slot} sample at {:#x}: {e}", s.offset));
            if ext.looped {
                looped += 1;
                // G-008's loop-point check: the start is the frame marked FLAG_LOOP_START, the end is the
                // extent's last frame (the SPU repeats from the start marker to the end frame), both are
                // 28-sample frame multiples, and the region is inside the decoded sample.
                let (ls, le) = ext.loop_points().unwrap_or_else(|| panic!("bank {slot} sample at {:#x}: looped without loop points", s.offset));
                assert!(ls % 28 == 0 && le % 28 == 0, "bank {slot} sample at {:#x}: loop points {ls}..{le} are not frame multiples", s.offset);
                assert!(ls < le && le == ext.samples(), "bank {slot} sample at {:#x}: loop {ls}..{le} vs extent {}", s.offset, ext.samples());
                loops.update(&(ls as u64).to_le_bytes());
                loops.update(&(le as u64).to_le_bytes());
                loop_start_sum += ls as u64;
            }
            for v in nf_formats::vag::decode_extent(blob, &ext) {
                samples += 1;
                pcm.update(&v.to_le_bytes());
            }
        }
        let mut pitches: Vec<u32> = bank.samples.iter().map(|s| s.pitch).collect();
        pitches.sort();
        pitches.dedup();
        rows.push(format!("bank/{slot}\t{}\t{}\t{looped}\t{loop_start_sum}\t{}\t{}\t{samples}\t{pitches:?}", bank.samples.len(), sbf.len(), hex(&loops.finish()), hex(&pcm.finish())));
    }
    let actual = format!("kind\tid\theaders\tsbf_bytes\tlooped\tloop_starts_sum\tloop_sha1\tpcm_sha1\tpcm_samples\tpitches\n{}", rows.join("\n"));
    if std::env::var_os("PIN_GOLDEN").is_some() { println!("{actual}"); }
    let d = diff_tsv(&golden_section("kind\tid\theaders\tsbf_bytes\tlooped\tloop_starts_sum\tloop_sha1\tpcm_sha1\tpcm_samples\tpitches", "bank/"), &actual);
    assert!(d.is_empty(), "{} bank differences:\n{}", d.len(), d.join("\n"));
    // Sister A (corrupted .SHF header): bump the last sample's size → the sizes no longer fill the SBF and the
    // sample's extent runs past the file.
    let mut bad_shf = get(&iso, &bank_path("ENGLISH", 4, "SHF"));
    let last = bad_bank_last(&bad_shf);
    let at = 4 + 36 * last + 8;
    let size = u32::from_le_bytes(bad_shf[at..at + 4].try_into().unwrap());
    bad_shf[at..at + 4].copy_from_slice(&(size + 64).to_le_bytes());
    let bad_bank = Bank::parse(&bad_shf, &get(&iso, &bank_path("ENGLISH", 4, "SFX"))).unwrap();
    let bad_sizes: u64 = bad_bank.samples.iter().map(|s| s.size as u64).sum();
    assert_ne!(bad_sizes as usize, get(&iso, &bank_path("ENGLISH", 4, "SBF")).len(), "sister: a +64 size tamper must break the sizes check");
    // Sister B (corrupted ADPCM body): one flipped data byte changes the decoded PCM.
    let mut bad_sbf = get(&iso, &bank_path("ENGLISH", 4, "SBF"));
    let s0 = &bad_bank.samples[0];
    bad_sbf[s0.offset as usize + 4] ^= 0x10;
    let blob = &bad_sbf[s0.offset as usize..(s0.offset + s0.size) as usize];
    let ext = nf_formats::vag::sample_extent(blob, 0).unwrap();
    let good = get(&iso, &bank_path("ENGLISH", 4, "SBF"));
    let g0 = &good[s0.offset as usize..(s0.offset + s0.size) as usize];
    assert_ne!(
        nf_formats::vag::decode_extent(blob, &ext)[..64],
        nf_formats::vag::decode_extent(g0, &ext)[..64],
        "sister: a flipped ADPCM byte must change the decode"
    );
}

fn bad_bank_last(shf: &[u8]) -> usize { (u32::from_le_bytes(shf[..4].try_into().unwrap()) as usize).saturating_sub(1) }

/// The 16 music streams: sizes are whole 0x100 blocks, the de-interleaved per-channel frame streams hash to the
/// golden numbers, every `.SMF` marker byte position lands inside the stream, and three named tracks decode to
/// the golden stereo PCM SHA-1s.
#[test]
fn music_ssd_demuxes_and_decodes_to_the_golden_numbers() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let mut rows = Vec::new();
    for n in 1..=16u32 {
        let ssd = get(&iso, &music_path(n, "SSD"));
        assert_eq!(ssd.len() % 0x100, 0, "MFX_{n}: whole 0x100 blocks");
        let blocks = ssd.len() / 0x100;
        let mut l: Vec<u8> = Vec::with_capacity(ssd.len() / 2);
        let mut r: Vec<u8> = Vec::with_capacity(ssd.len() / 2);
        for blk in ssd.chunks_exact(0x100) {
            l.extend_from_slice(&blk[..0x80]);
            r.extend_from_slice(&blk[0x80..]);
        }
        let (lh, rh) = (hex(&sha1(&l)), hex(&sha1(&r)));
        let smf = nf_formats::music::Smf::parse(&get(&iso, &music_path(n, "SMF"))).unwrap_or_else(|e| panic!("MFX_{n}.SMF: {e}"));
        assert!(!smf.markers.is_empty(), "MFX_{n}: has markers");
        let maxpos = smf.markers.iter().map(|m| m.pos).max().unwrap();
        assert!((maxpos as usize) < ssd.len(), "MFX_{n}: max marker pos {maxpos} inside {}", ssd.len());
        let pcm = match n {
            1 | 11 | 16 => {
                let mut dec = nf_formats::music::SsdDecoder::default();
                let out = dec.decode(&ssd);
                let mut h = Sha1::new();
                for v in &out {
                    h.update(&v[0].to_le_bytes());
                    h.update(&v[1].to_le_bytes());
                }
                hex(&h.finish())
            }
            _ => "-".to_string(),
        };
        rows.push(format!("ssd/{n:02}\t{}\t{blocks}\t{}\t{lh}\t{rh}\t{pcm}", ssd.len(), blocks * 224));
    }
    let actual = format!("kind\tid\tbytes\tblocks\tsamples\tL_sha1\tR_sha1\tpcm_sha1\n{}", rows.join("\n"));
    let d = diff_tsv(&golden_section("kind\tid\tbytes\tblocks\tsamples\tL_sha1\tR_sha1\tpcm_sha1", "ssd/"), &actual);
    assert!(d.is_empty(), "{} ssd differences:\n{}", d.len(), d.join("\n"));
    // Sister: one flipped frame byte (left half of block 8) changes both the demux hash and the decode.
    let mut bad = get(&iso, &music_path(1, "SSD"));
    bad[8 * 0x100 + 0x20] ^= 0x40;
    let mut good_l: Vec<u8> = Vec::new();
    let mut bad_l: Vec<u8> = Vec::new();
    let good = get(&iso, &music_path(1, "SSD"));
    for (dst, src) in [(&mut good_l, &good), (&mut bad_l, &bad)] {
        for blk in src.chunks_exact(0x100) {
            dst.extend_from_slice(&blk[..0x80]);
        }
    }
    assert_ne!(hex(&sha1(&bad_l)), hex(&sha1(&good_l)), "sister: tampered left half changes the demux hash");
    let mut d1 = nf_formats::music::SsdDecoder::default();
    let mut d2 = nf_formats::music::SsdDecoder::default();
    assert_ne!(d1.decode(&bad)[..2304], d2.decode(&good)[..2304], "sister: tampered block 8 (samples 1792..2016) changes the decode");
}

/// Speech streams: the LUT's 1681 136-byte-header entries, the 828 whose header carries a type-9 end marker
/// (the rest are unreferenced stale takes — the LUT data sizes include them), the total played bytes, three
/// named decodes, and every stream `FileRef` in all 25 banks resolving to a valid entry (the game can only play
/// referenced entries, so a played entry without an end marker would hang its voice).
#[test]
fn speech_streams_decode_to_the_golden_numbers() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let lut = get(&iso, "PS2/ENGLISH/STREAMS/STREAMS.LUT");
    let bin = get(&iso, "PS2/ENGLISH/STREAMS/STREAMS.BIN");
    let entries = stream_lut(&lut).unwrap();
    assert_eq!(entries.len(), 1681, "LUT entries");
    assert!(entries.iter().all(|e| e[1] == 136), "every entry's header is 136 bytes");
    let mut valid = 0usize;
    let mut total = 0u64;
    for e in &entries {
        if let Ok(d) = stream_data(&bin, *e) {
            valid += 1;
            total += d.len() as u64;
            assert_eq!(d.len() % 16, 0, "whole ADPCM frames");
        }
    }
    assert_eq!(valid, 828, "entries with a type-9 end marker");
    assert_eq!(total, 29_322_752, "total played bytes");
    // the three named decodes (shortest, mid, longest by played bytes)
    let mut rows = Vec::new();
    for key in ["stream/532", "stream/738", "stream/316"] {
        let k: usize = key[7..].parse().unwrap();
        let d = stream_data(&bin, entries[k]).unwrap_or_else(|e| panic!("{key}: {e}"));
        let pcm = nf_formats::vag::decode(d);
        rows.push(format!("{key}\t{}\t{}\t{}", d.len(), pcm.len(), hex(&sha1(&bytewise(&pcm)))));
    }
    // every stream FileRef in all 25 banks resolves to a valid entry
    let slots = bank_slots(&get(&iso, "PS2/SBINFO.SBI")).unwrap();
    let mut refs = std::collections::BTreeSet::new();
    for &slot in &(0..25).filter(|&s| slots[s] != 0xffffffff).collect::<Vec<_>>() {
        let bank = Bank::parse(&get(&iso, &bank_path("ENGLISH", slot, "SHF")), &get(&iso, &bank_path("ENGLISH", slot, "SFX"))).unwrap();
        for (id, off) in &bank.effects {
            let eff = Effect::parse(&bank.sfx[*off as usize..]).unwrap_or_else(|e| panic!("slot {slot} effect {id:#x}: {e}"));
            for st in &eff.subs {
                if st.file < 0 {
                    let k = (-1 - st.file) as usize;
                    assert!(k < entries.len(), "slot {slot} effect {id:#x}: fileref {} out of range", st.file);
                    assert!(stream_data(&bin, entries[k]).is_ok(), "slot {slot} effect {id:#x}: fileref {} (entry {k}) has no end marker", st.file);
                    refs.insert(k);
                } else {
                    assert!((st.file as usize) < bank.samples.len(), "slot {slot} effect {id:#x}: sample ref {} out of range", st.file);
                }
            }
        }
    }
    assert_eq!(refs.len(), 675, "distinct referenced stream entries");
    rows.push(format!("streams/agg\t{valid}\t{total}\t{}", refs.len()));
    let actual = format!("kind\tid\tdata_bytes\tpcm_samples\tpcm_sha1\n{}", rows.join("\n"));
    let d = diff_tsv(&golden_section("kind\tid\tdata_bytes\tpcm_samples\tpcm_sha1", "stream"), &actual);
    assert!(d.is_empty(), "{} stream differences:\n{}", d.len(), d.join("\n"));
    // Sister: the type-9 end marker flipped to a non-end type makes stream_data fail.
    let e = entries[532];
    let mut bad_bin = bin.clone();
    let hdr = e[0] as usize;
    let (n, mo) = (
        u32::from_le_bytes(bad_bin[hdr + 4..hdr + 8].try_into().unwrap()) as usize,
        u32::from_le_bytes(bad_bin[hdr + 0xc..hdr + 0x10].try_into().unwrap()) as usize,
    );
    let mut done = false;
    for k in 0..n {
        let o = mo + 32 * k;
        if i32::from_le_bytes(bad_bin[hdr + o + 8..hdr + o + 12].try_into().unwrap()) == 9 {
            bad_bin[hdr + o + 8] = 5;
            done = true;
            break;
        }
    }
    assert!(done, "sister setup: entry 532 has a type-9 marker");
    assert!(stream_data(&bad_bin, e).is_err(), "sister: no end marker → stream_data fails");
}

/// The movies: all 22 `MOVIES/30_FPS/*.PSS` entries exist in the tree (on the slim on-foot image as size-0
/// stubs, counted and skipped; on an image with movie data each demuxes to its `pss_movies.tsv` golden row).
#[test]
fn pss_movies_tree_and_golden_when_data_exists() {
    let Some(iso) = iso() else { eprintln!("skip: NF_ISO not set"); return };
    let paths: Vec<String> = include_str!("../golden/disc_files.tsv")
        .lines()
        .skip(1)
        .map(|l| l.split('\t').next().unwrap().to_string())
        .filter(|p| p.starts_with("/MOVIES/30_FPS/") && p.ends_with(".PSS"))
        .collect();
    assert_eq!(paths.len(), 22, "22 movies on the disc");
    let mut with_data = Vec::new();
    for p in &paths {
        let e = iso.find(p).unwrap_or_else(|| panic!("{p} missing from the tree"));
        if e.size > 0 {
            with_data.push((p.clone(), e));
        }
    }
    if with_data.is_empty() {
        eprintln!("all 22 movies are stubs on this image (slim on-foot ISO) — tree check only");
        return;
    }
    eprintln!("movies with data: {}/{}", with_data.len(), paths.len());
    let golden = include_str!("../golden/pss_movies.tsv");
    for (p, e) in &with_data {
        let data = get(&iso, p);
        assert_eq!(data.len(), e.size as usize, "{p}: extent size");
        let d = nf_formats::pss::demux(&data).unwrap_or_else(|err| panic!("{p}: {err}"));
        let name = p.rsplit('/').next().unwrap();
        let line = golden.lines().find(|l| l.starts_with(name)).unwrap();
        let c: Vec<&str> = line.split('\t').collect();
        let expect = |i: usize| c[i].parse::<usize>().unwrap();
        assert_eq!(d.packs, expect(1), "{p}: packs");
        assert_eq!(d.video_packets, expect(2), "{p}: video packets");
        assert_eq!(d.audio_packets, expect(3), "{p}: audio packets");
        assert_eq!(d.video.len(), expect(6), "{p}: video ES bytes");
        assert_eq!(d.audio.values().map(|v| v.len()).sum::<usize>(), expect(7), "{p}: audio bytes");
        assert!(d.ended && d.trailing == 0, "{p}: end code reached, no trailing bytes");
        // the demuxed elementary stream hashes, printed for the golden (pin them from a data-image run)
        // Some movies carry no audio stream at all (07350048, the menu backdrop [D]) — no hash to print.
        eprintln!("{p}: ES sha1 video {} audio {}", hex(&sha1(&d.video)),
            d.audio.values().next().map_or_else(|| "none".to_string(), |a| hex(&sha1(a))));
        // Sister: a corrupted pack start code must fail the demux.
        let mut bad = data.clone();
        let first_pack = data.windows(4).position(|w| w == [0, 0, 1, 0xba]).unwrap();
        bad[first_pack] = 0x80;
        assert!(nf_formats::pss::demux(&bad).is_err(), "{p}: sister — corrupted pack header fails");
    }
}
